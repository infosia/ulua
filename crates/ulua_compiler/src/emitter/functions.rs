//! One storage pass per function; one emission pass over the closure tree.
use super::*;
use crate::ir::Statement as S;

pub(super) struct Layout {
    pub(super) headers: String,
    pub(super) entries: HashMap<usize, String>,
    pub(super) slots: Vec<Slot>,
}
pub(super) struct Emission {
    pub(super) layouts: Vec<Option<Layout>>,
    functions: Vec<Option<usize>>,
    pub(super) locals: Vec<std::collections::HashSet<String>>,
}

pub(super) fn emit(module: &ir::Module) -> Result<String, Diagnostic> {
    let size = module.bindings.len() + 1;
    let mut owners = vec![vec![]; size];
    let mut groups = vec![0; size];
    let mut context = Emission {
        layouts: (0..size).map(|_| None).collect(),
        functions: vec![None; size],
        locals: vec![std::collections::HashSet::new(); size],
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
    let mut work: Vec<_> = module.body.iter().collect();
    for f in &module.functions {
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
    let types = types::FunctionTypes::new(module)?;
    let mut locations = HashMap::new();
    let empty = HashMap::new();
    for owner in std::iter::once(0).chain(module.functions.iter().map(|f| f.binding + 1)) {
        let mut probe = Emitter::new(module, owner, &empty, &owners[owner], &types);
        let (parameters, overhead) = if owner == 0 {
            probe.statements(&module.body, 0)?;
            (vec![], 10)
        } else {
            let i = context.functions[owner - 1]
                .ok_or_else(|| probe.limit("internal", "Missing function identity"))?;
            let f = &module.functions[i];
            probe.defaults(f)?;
            probe.statements(&f.body, 0)?;
            (
                f.parameters.iter().map(|(id, _)| *id).collect(),
                f.parameters.iter().filter(|(_, d)| d.is_some()).count(),
            )
        };
        let layout = probe.plan(&parameters, overhead, &groups)?;
        locations.extend(probe.locations);
        context.layouts[owner] = Some(layout);
    }
    for (&id, location) in &locations {
        context.locals[module.bindings[id].owner]
            .insert(location.split('.').next().unwrap_or(location).to_owned());
    }
    for f in &module.functions {
        for (id, _) in &f.parameters {
            context.locals[f.binding + 1].insert(format!("__ulua_b{id}"));
        }
    }
    for name in [
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
    ] {
        context.locals[0].insert(name.into());
    }
    let mut chunk = Emitter::new(module, 0, &locations, &owners[0], &types);
    chunk.context = Some(&context);
    chunk.indent = 0;
    chunk.slots = context.layouts[0]
        .as_ref()
        .map(|l| l.slots.clone())
        .unwrap_or_default();
    for s in &mut chunk.slots {
        s.busy = false;
    }
    chunk.statements(&module.body, 0)?;
    let mut out = String::from("--!strict\n");
    out.push_str(LIBRARIES);
    out.push_str(&types.declarations);
    if let Some(layout) = &context.layouts[0] {
        out.push_str(&layout.headers);
    }
    out.push_str(&chunk.out);
    out.push_str("return {\n");
    for f in module.functions.iter().filter(|f| f.exported) {
        let _ = writeln!(
            out,
            "    [{}] = {},",
            quote(&f.name),
            chunk.location(f.binding)?
        );
    }
    out.push_str("}\n");
    check_output(module, out.len())?;
    Ok(out)
}

impl Emitter<'_> {
    pub(super) fn closure(&mut self, id: usize, depth: u32) -> Result<String, Diagnostic> {
        let Some(context) = self.context else {
            return Ok("nil :: any".into());
        };
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
        child.copies = self.copies.clone();
        child.slots = layout.slots.clone();
        for s in &mut child.slots {
            s.busy = false;
        }
        child.defaults(f)?;
        child.statements(&f.body, depth)?;
        if child.upvalues.len() > crate::UPVALUE_BUDGET {
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
                let _ = writeln!(
                    parameter_locals,
                    "    local __ulua_b{id}: {ty} = ({}) :: {ty}",
                    initial(
                        self.module.bindings[*id]
                            .ty
                            .as_ref()
                            .ok_or_else(|| self.limit("internal", "Missing default type"))?
                    )
                );
            } else {
                parameters.push(format!("__ulua_b{id}: {ty}"));
            }
        }
        Ok(format!(
            "function({}): {}\n{}{}{}end",
            parameters.join(", "),
            self.function_types.luau(&f.result),
            parameter_locals,
            child.headers(&layout.headers),
            child.out
        ))
    }
}
