//! One storage pass per function; one emission pass over the closure tree.
use super::*;
use crate::ir::Statement as S;

// An export statement stores one field. The single-export return is also one field.
const EXPORT_FIELDS: usize = 1;
const EXPORT_TABLE_PREFIX: &str = "return {\n";
const EXPORT_TABLE_SUFFIX: &str = "}\n";
const EXPORT_LOOP_PREFIX: &str = "local __ulua_exports: {[string]: any} = {\n";
#[cfg(feature = "test-output-limit")]
pub(super) const EXPORT_LOOP_OVERHEAD: usize =
    "while true do\nend\n".len() + EXPORT_LOOP_PREFIX.len() - EXPORT_TABLE_PREFIX.len();
pub(super) const EMPTY_EXPORT_TABLE_BYTES: usize =
    EXPORT_TABLE_PREFIX.len() + EXPORT_TABLE_SUFFIX.len();

pub(super) struct Layout {
    pub(super) headers: String,
    pub(super) entries: HashMap<usize, String>,
    pub(super) entry_locals: HashMap<usize, std::collections::HashSet<String>>,
    pub(super) slots: Vec<Slot>,
    pub(super) protected_slots: Vec<Vec<usize>>,
    pub(super) overhead: usize,
}
pub(super) struct Emission {
    pub(super) layouts: Vec<Option<Layout>>,
    functions: Vec<Option<usize>>,
    pub(super) locals: Vec<std::collections::HashSet<String>>,
    pub(super) accessors: Vec<bool>,
    numeric_arrows: constants::NumericArrows,
}

pub(super) fn emit(module: &ir::Module, bound: usize) -> Result<String, Diagnostic> {
    emit_with_exports(module, bound, false)
}
#[cfg(feature = "test-output-limit")]
pub(super) fn emit_export_loop(module: &ir::Module, bound: usize) -> Result<String, Diagnostic> {
    emit_with_exports(module, bound, true)
}
fn emit_with_exports(
    module: &ir::Module,
    bound: usize,
    repeat_exports: bool,
) -> Result<String, Diagnostic> {
    let types = types::FunctionTypes::new(module)?;
    let numeric_arrows = constants::numeric_arrows(module, &types)?;
    let paged_exports = module.functions.iter().filter(|f| f.exported).count() > EXPORT_FIELDS;
    let library_locals = LIBRARY_LOCALS
        + usize::from(paged_exports || repeat_exports)
        + usize::from(!module.hosts.is_empty())
        + numeric_arrows.locals
        + if module.asynchronous {
            asynchronous::LOCALS
        } else {
            0
        }
        + if module.exceptions { 6 } else { 0 };
    let (method_prelude, method_locals) = methods::prelude(&module.methods)?;
    let size = module.bindings.len() + 1;
    let mut owners = vec![vec![]; size];
    let mut groups = vec![0; size];
    let mut context = Emission {
        layouts: (0..size).map(|_| None).collect(),
        functions: vec![None; size],
        locals: vec![std::collections::HashSet::new(); size],
        accessors: accessors::select(module),
        numeric_arrows,
    };
    for (i, f) in module.functions.iter().enumerate() {
        context.functions[f.binding] = Some(i);
    }
    for (id, b) in module.bindings.iter().enumerate() {
        if context.functions[id].is_some_and(|i| module.functions[i].name.is_empty()) {
            continue;
        }
        owners[b.owner].push(id);
    }
    for ids in &mut owners {
        ids.sort_by_key(|id| module.bindings[*id].start);
    }
    let mut catch_ids = std::collections::HashSet::new();
    let mut work: Vec<_> = module.body.iter().collect();
    for f in &module.functions {
        work.extend(&f.initializers);
        work.extend(&f.body);
    }
    while let Some(s) = work.pop() {
        match s.form() {
            S::Entry(ids) => {
                let key = ids.iter().min().map_or(0, |id| id + 1);
                for &id in ids {
                    groups[id] = key;
                }
            }
            S::Try(a, b, f) => {
                work.push(a);
                if let Some((id, b)) = b {
                    catch_ids.extend(id.iter().copied());
                    work.push(b);
                }
                work.extend(f.iter().map(|s| s.as_ref()));
            }
            S::Block(b) => work.extend(b),
            S::If(_, a, b) => {
                work.push(a);
                work.extend(b.iter().map(|b| b.as_ref()));
            }
            S::While(_, b) | S::ForOf(_, _, b) => work.push(b),
            S::For(init, _, update, b, _) => {
                work.extend(init);
                work.extend(update.iter().map(|s| s.as_ref()));
                work.push(b);
            }
            _ => {}
        }
    }
    let captured_catches: std::collections::HashSet<_> = catch_ids
        .iter()
        .filter(|id| module.captured[**id])
        .map(|id| module.bindings[*id].owner)
        .collect();
    for (owner, ids) in owners.iter_mut().enumerate() {
        ids.retain(|id| !catch_ids.contains(id));
        let mut captured = 0;
        for &id in ids.iter() {
            if module.captured[id] {
                captured += 1;
                if captured > crate::CAPTURE_LOCALS - usize::from(captured_catches.contains(&owner))
                {
                    // Spill fields do not carry mutable-local versions in the Luau analyzer.
                    context.accessors[id] = false;
                }
            }
        }
    }
    let mut locations = HashMap::new();
    let empty = HashMap::new();
    for owner in std::iter::once(0).chain(module.functions.iter().map(|f| f.binding + 1)) {
        let accessor_count = owners[owner]
            .iter()
            .filter(|id| context.accessors[**id])
            .count();
        let mut probe = Emitter::new(module, owner, &empty, &owners[owner], &types);
        probe.overhead = accessor_count
            + if owner == 0 {
                library_locals + method_locals
            } else {
                0
            };
        let (parameters, overhead) = if owner == 0 {
            probe.statements(&module.body, 0)?;
            (vec![], accessor_count + library_locals + method_locals)
        } else {
            let i = context.functions[owner - 1]
                .ok_or_else(|| probe.limit("internal", "Missing function identity"))?;
            let f = &module.functions[i];
            probe.overhead =
                accessor_count + f.parameters.iter().filter(|(_, d)| d.is_some()).count();
            probe.statements(&f.initializers, 0)?;
            probe.defaults(f)?;
            probe.statements(&f.body, 0)?;
            (
                f.parameters.iter().map(|(id, _)| *id).collect(),
                accessor_count + f.parameters.iter().filter(|(_, d)| d.is_some()).count(),
            )
        };
        // A protected catch closure can capture its own parameter beside these block bindings.
        let reserve = usize::from(captured_catches.contains(&owner));
        let mut layout = probe.plan(&parameters, overhead, &groups, reserve)?;
        for id in owners[owner]
            .iter()
            .copied()
            .filter(|id| context.accessors[*id])
        {
            let binding = &module.bindings[id];
            let location = probe.location(id)?;
            let ty = binding
                .ty
                .ok_or_else(|| probe.limit("internal", "Missing accessor type"))?;
            let key = if module.captured[id] { groups[id] } else { 0 };
            let header = if key == 0 {
                &mut layout.headers
            } else {
                layout.entries.entry(key).or_default()
            };
            header.push_str(&accessors::declaration(id, &location, &types.luau(&ty)));
            layout
                .entry_locals
                .entry(key)
                .or_default()
                .insert(accessors::name(id));
            context.locals[owner].insert(accessors::name(id));
        }
        locations.extend(probe.locations);
        context.layouts[owner] = Some(layout);
    }
    for (&id, location) in &locations {
        context.locals[module.bindings[id].owner]
            .insert(location.split('.').next().unwrap_or(location).to_owned());
    }
    context.locals[0].extend(context.numeric_arrows.names.values().cloned());
    // Catch parameters belong to protected closures of this owner, not to its upvalues.
    for id in catch_ids {
        context.locals[module.bindings[id].owner].insert(format!("__ulua_b{id}"));
    }
    for f in &module.functions {
        for (id, _) in &f.parameters {
            context.locals[f.binding + 1].insert(format!("__ulua_b{id}"));
        }
    }
    for name in [
        "__ulua_new",
        "__ulua_new_void",
        "__ulua_on_then0",
        "__ulua_on_then1",
        "__ulua_on_catch0",
        "__ulua_on_catch1",
        "__ulua_on_finally",
        "__ulua_on_then0v",
        "__ulua_on_then1v",
        "__ulua_on_catch0v",
        "__ulua_on_catch1v",
        "__ulua_on_finallyv",
        "__ulua_all",
        "__ulua_start",
        "__ulua_promises",
        "__ulua_await",
        "__ulua_register",
        "__ulua_errors",
        "__ulua_error",
        "__ulua_instance",
        "__ulua_protect",
        "__ulua_try",
        "__ulua_concat",
        "__ulua_print",
        "__ulua_fmod",
        "__ulua_tostring",
        "__ulua_gsub",
        "__ulua_huge",
        "__ulua_format",
        "__ulua_trap",
        "__ulua_get",
        "__ulua_set",
        "__ulua_literal_set",
        "__ulua_interrupt",
        "__ulua_equal",
        "__ulua_unequal",
        "__ulua_receiver",
    ] {
        context.locals[0].insert(name.into());
    }
    for &(method, arity) in &module.methods {
        context.locals[0].insert(methods::name(method, arity));
    }
    let mut chunk = Emitter::new(module, 0, &locations, &owners[0], &types);
    chunk.context = Some(&context);
    chunk.overhead = context.layouts[0].as_ref().map_or(0, |l| l.overhead);
    chunk.indent = 0;
    chunk.slots = context.layouts[0]
        .as_ref()
        .map(|l| l.slots.clone())
        .unwrap_or_default();
    for s in &mut chunk.slots {
        s.busy = false;
    }
    let mut out = String::from("--!strict\n");
    out.push_str(&prelude::checkpoint(LIBRARIES));
    if module.exceptions {
        out.push_str(&prelude::checkpoint(prelude::EXCEPTIONS));
    }
    if module.asynchronous {
        out.push_str(&prelude::checkpoint(asynchronous::PRELUDE));
    }
    out.push_str(&prelude::checkpoint(&method_prelude));
    out.push_str(&types.declarations);
    out.push_str(&chunk.headers(&context.numeric_arrows.declarations));
    if let Some(layout) = &context.layouts[0] {
        out.push_str(&chunk.headers(&layout.headers));
    }
    let output = output::Output::new(bound);
    let first_offset = module
        .body
        .iter()
        .find_map(|s| match s {
            S::Located(offset, _) => Some(*offset as usize),
            _ => None,
        })
        .unwrap_or(0);
    output.charge(out.len(), first_offset);
    output.check(module)?;
    chunk.output = Some(&output);
    if let Some(entry) = module.body.first() {
        chunk.statements(std::slice::from_ref(entry), 0)?;
    }
    if !module.hosts.is_empty() {
        chunk.line("local __ulua_grants: {any} = select(2, ...)");
    }
    for (i, (id, _)) in module.hosts.iter().enumerate() {
        let ty = module.bindings[*id]
            .ty
            .ok_or_else(|| chunk.limit("internal", "Missing host type"))?;
        chunk.write_binding(
            *id,
            format!("__ulua_grants[{}] :: {}", i + 1, types.luau(&ty)),
        )?;
    }
    chunk.statements(&module.body[1..], 0)?;
    let last_offset = module
        .body
        .iter()
        .rev()
        .find_map(|s| match s {
            S::Located(offset, _) => Some(*offset as usize),
            _ => None,
        })
        .unwrap_or(first_offset);
    chunk.statement_offset = last_offset;
    if repeat_exports {
        chunk.line("while true do");
    }
    chunk.exports(paged_exports, !repeat_exports)?;
    if repeat_exports {
        chunk.line("end");
    }
    out.push_str(&chunk.out);
    output.finish(module, out.len())?;
    Ok(out)
}

impl Emitter<'_> {
    fn exports(&mut self, paged_exports: bool, terminal: bool) -> Result<(), Diagnostic> {
        if paged_exports {
            self.line("local __ulua_exports: {[string]: any} = {}");
            for f in self.module.functions.iter().filter(|f| f.exported) {
                self.line(format!(
                    "__ulua_exports[{}] = {}",
                    quote(&f.name),
                    self.location(f.binding)?
                ));
            }
            if terminal {
                self.line("return __ulua_exports");
            }
        } else {
            let mut trailer = String::from(if terminal {
                EXPORT_TABLE_PREFIX
            } else {
                EXPORT_LOOP_PREFIX
            });
            for f in self.module.functions.iter().filter(|f| f.exported) {
                let _ = writeln!(
                    trailer,
                    "    [{}] = {},",
                    quote(&f.name),
                    self.location(f.binding)?
                );
            }
            trailer.push_str(EXPORT_TABLE_SUFFIX.trim_end());
            self.line(trailer);
        }
        Ok(())
    }
    pub(super) fn closure(&mut self, id: usize, depth: u32) -> Result<String, Diagnostic> {
        let Some(context) = self.context else {
            return Ok("nil :: any".into());
        };
        if let Some(name) = context.numeric_arrows.names.get(&id) {
            self.capture(name);
            return Ok(name.clone());
        }
        let i = context.functions[id]
            .ok_or_else(|| self.limit("internal", "Missing closure identity"))?;
        let f = &self.module.functions[i];
        let layout = context.layouts[id + 1]
            .as_ref()
            .ok_or_else(|| self.limit("internal", "Missing closure layout"))?;
        let mut child = Emitter::new(
            self.module,
            id + 1,
            self.module_locations,
            &[],
            self.function_types,
        );
        child.context = Some(context);
        child.async_void =
            f.asynchronous && self.module.types.promise_value(f.result) == Some(Type::Void);
        if f.asynchronous {
            child.activation_parameters = f
                .parameters
                .iter()
                .filter(|(_, d)| d.is_none())
                .map(|(id, _)| format!("__ulua_b{id}"))
                .collect();
            child.activation_upvalues.extend(
                f.parameters
                    .iter()
                    .filter(|(_, d)| d.is_some())
                    .map(|(id, _)| format!("__ulua_arg{id}")),
            );
        }
        child.overhead = layout.overhead;
        child.copies = self.copies.clone();
        child.slots = layout.slots.clone();
        for s in &mut child.slots {
            s.busy = false;
        }
        let mut parameters = vec![];
        let mut parameter_locals = String::new();
        for (id, default) in &f.parameters {
            let ty = self.module.bindings[*id]
                .ty
                .as_ref()
                .ok_or_else(|| self.limit("internal", "Missing parameter type"))?;
            let ty = self.function_types.luau(ty);
            if default.is_some() {
                parameters.push(format!("__ulua_arg{id}: {ty}?"));
                if child.location(*id)? == format!("__ulua_b{id}") {
                    let _ = writeln!(
                        parameter_locals,
                        "local __ulua_b{id}: {ty} = ({}) :: {ty}",
                        initial(
                            self.module.bindings[*id]
                                .ty
                                .as_ref()
                                .ok_or_else(|| self.limit("internal", "Missing default type"))?
                        )
                    );
                }
            } else {
                parameters.push(format!("__ulua_b{id}: {ty}"));
            }
        }
        let value_type = if f.asynchronous {
            self.module.types.promise_value(f.result)
        } else {
            None
        };
        let body_type = value_type.map(|ty| {
            if ty == Type::Void {
                "nil".to_owned()
            } else {
                self.function_types.luau(&ty)
            }
        });
        let async_prefix = body_type.as_ref().map_or(String::new(), |ty| {
            format!("    local __ulua_p: __ulua_Promise<{ty}> = {{state = 0, value = nil :: any, handled = false, fulfilled = {{}}, rejected = {{}}}}\n    __ulua_promises[__ulua_p] = true\n    return __ulua_start(__ulua_p, function(): ({ty} | __ulua_Promise<{ty}>)\n")
        });
        let prefix = format!(
            "function({}): {}\n{}{}{}",
            parameters.join(", "),
            self.function_types.luau(&f.result),
            async_prefix,
            child.headers(&parameter_locals),
            child.headers(&layout.headers)
        );
        child.output = self.output;
        child.statement_offset = self.statement_offset;
        if let Some(output) = self.output {
            output.charge(prefix.len(), self.statement_offset);
            output.check(self.module)?;
        }
        child.statements(&f.initializers, depth)?;
        child.defaults(f)?;
        child.statements(&f.body, depth)?;
        let activation_captures = child.upvalues.len() + child.activation_upvalues.len();
        if f.asynchronous {
            child.capture("__ulua_start");
            child.capture("__ulua_promises");
        }
        if activation_captures > crate::UPVALUE_BUDGET
            || child.upvalues.len() > crate::UPVALUE_BUDGET
        {
            return Err(Diagnostic::at(
                &self.module.source,
                self.module.bindings[id].start as usize,
                "limit:upvalues",
                "Closure upvalue limit exceeded",
            ));
        }
        for name in &child.upvalues {
            self.capture(name);
        }
        if let Some(output) = self.output {
            output.charge(3, self.statement_offset);
            output.check(self.module)?;
        }
        let suffix = if f.asynchronous {
            let tail =
                if value_type == Some(Type::Void) && !f.body.iter().any(|s| ir::abrupt(s, false)) {
                    "    return nil\n"
                } else {
                    ""
                };
            format!("{tail}    end)\nend")
        } else {
            "end".to_owned()
        };
        if let Some(output) = self.output {
            output.charge(suffix.len().saturating_sub(3), self.statement_offset);
        }
        let value = format!("{prefix}{}{suffix}", child.out);
        // Child writes already use the module budget. The enclosing line counts only its own bytes.
        self.credited += value.len();
        Ok(value)
    }
}
