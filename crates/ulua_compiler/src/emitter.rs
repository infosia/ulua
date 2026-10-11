//! Strict Luau emission with ordered values and a per-function local budget.
mod expressions;
mod joins;
mod statements;
mod types;
use crate::{
    Diagnostic, LOCAL_BUDGET, TEMPORARY_BOUND,
    ir::{self, Type},
};
use std::{collections::HashMap, fmt::Write};

const SPILL_FIELDS: usize = 128;

pub(crate) const LIBRARIES: &str = "local __ulua_concat: typeof(table.concat) = (table.concat) :: typeof(table.concat)\nlocal __ulua_print: typeof(print) = (print) :: typeof(print)\nlocal __ulua_fmod: typeof(math.fmod) = (math.fmod) :: typeof(math.fmod)\nlocal __ulua_tostring: typeof(tostring) = (tostring) :: typeof(tostring)\nlocal __ulua_gsub: typeof(string.gsub) = (string.gsub) :: typeof(string.gsub)\nlocal __ulua_huge: typeof(math.huge) = (math.huge) :: typeof(math.huge)\nlocal function __ulua_format(__ulua_n: number): string\n    if __ulua_n ~= __ulua_n then return \"NaN\" end\n    if __ulua_n == __ulua_huge then return \"Infinity\" end\n    if __ulua_n == -__ulua_huge then return \"-Infinity\" end\n    if __ulua_n == 0 then return \"0\" end\n    local __ulua_s: string = (__ulua_gsub(__ulua_tostring(__ulua_n), \"e([+-])0*(%d)\", \"e%1%2\")) :: string\n    return __ulua_s\nend\n";

pub(crate) fn quote(value: &str) -> String {
    let mut out = String::from("\"");
    for byte in value.bytes() {
        if (0x20..=0x7e).contains(&byte) && byte != b'"' && byte != b'\\' {
            out.push(char::from(byte));
        } else {
            let _ = write!(out, "\\{byte:03}");
        }
    }
    out.push('"');
    out
}
fn initial(ty: &Type) -> &'static str {
    match ty {
        Type::Number => "0",
        Type::Boolean => "false",
        Type::String => "\"\"",
        _ => "nil :: any",
    }
}
fn number(n: f64) -> String {
    if n.is_nan() {
        "(0/0)".into()
    } else if n == f64::INFINITY {
        "__ulua_huge".into()
    } else if n == f64::NEG_INFINITY {
        "(-__ulua_huge)".into()
    } else if n == 0.0 && n.is_sign_negative() {
        "(-0)".into()
    } else {
        format!("{n:?}")
    }
}
struct Slot {
    ty: Type,
    busy: bool,
}
struct Emitter<'a> {
    module: &'a ir::Module,
    function_types: &'a types::FunctionTypes,
    owner: usize,
    out: String,
    indent: usize,
    slots: Vec<Slot>,
    locations: HashMap<usize, String>,
    module_locations: &'a HashMap<usize, String>,
    binding_ids: &'a [usize],
    offset: usize,
    copies: HashMap<usize, String>,
    generated: Vec<(String, String, String, bool)>,
    loops: Vec<Loop>,
    joins: joins::Joins,
}
#[derive(Clone)]
struct Loop {
    tail: Option<String>,
}
impl<'a> Emitter<'a> {
    fn new(
        module: &'a ir::Module,
        owner: usize,
        module_locations: &'a HashMap<usize, String>,
        binding_ids: &'a [usize],
        function_types: &'a types::FunctionTypes,
    ) -> Self {
        Self {
            module,
            function_types,
            owner,
            out: String::new(),
            indent: 1,
            slots: vec![],
            locations: HashMap::new(),
            module_locations,
            binding_ids,
            offset: 0,
            copies: HashMap::new(),
            generated: vec![],
            loops: vec![],
            joins: joins::Joins::default(),
        }
    }
    fn guard(&self, depth: u32) -> Result<u32, Diagnostic> {
        let next = depth + 1;
        if next >= self.module.depth {
            return Err(self.limit(
                "internal",
                "Emitter depth limit exceeded after checked input",
            ));
        }
        Ok(next)
    }
    fn limit(&self, code: &'static str, message: &str) -> Diagnostic {
        Diagnostic::at(&self.module.source, self.offset, code, message)
    }
    fn line(&mut self, text: impl AsRef<str>) {
        let _ = writeln!(
            self.out,
            "{}{}",
            if self.indent == 0 { "" } else { "    " },
            text.as_ref()
        );
    }
    fn location(&self, id: usize) -> Result<String, Diagnostic> {
        self.copies
            .get(&id)
            .or_else(|| self.locations.get(&id))
            .or_else(|| self.module_locations.get(&id))
            .cloned()
            .map_or_else(|| Ok(format!("__ulua_b{id}")), Ok)
    }
    fn write(&mut self, name: &str, value: impl AsRef<str>, ty: &str) {
        self.line(format!("{name} = ({}) :: {ty}", value.as_ref()));
    }
    fn write_slot(&mut self, slot: usize, value: impl AsRef<str>) {
        let ty = self.function_types.luau(&self.slots[slot].ty);
        self.write(&self.temp(slot), value, &ty);
    }
    fn read_binding(&mut self, id: usize) -> Result<String, Diagnostic> {
        let location = self.location(id)?;
        // Delay the join until a read proves that the binding is live.
        // Each read emits at most one join, independent of enclosing branch count.
        if self.joins.clear(id) {
            self.write_binding(id, &location)?;
        }
        Ok(location)
    }
    fn write_binding(&mut self, id: usize, value: impl AsRef<str>) -> Result<(), Diagnostic> {
        let ty = self.function_types.luau(
            self.module.bindings[id]
                .ty
                .as_ref()
                .ok_or_else(|| self.limit("internal", "Missing checked binding type"))?,
        );
        self.joins.clear(id);
        self.write(&self.location(id)?, value, &ty);
        Ok(())
    }
    fn slot(&mut self, ty: &Type) -> Result<usize, Diagnostic> {
        if let Some(i) = self.slots.iter().position(|s| !s.busy && s.ty == *ty) {
            self.slots[i].busy = true;
            Ok(i)
        } else {
            self.reserve_temporary()?;
            let i = self.slots.len();
            self.slots.push(Slot {
                ty: ty.clone(),
                busy: true,
            });
            Ok(i)
        }
    }
    fn reserve_temporary(&self) -> Result<(), Diagnostic> {
        if self.slots.len() + self.generated.len() >= TEMPORARY_BOUND {
            return Err(self.limit("limit:temporaries", "Non-spill local limit exceeded"));
        }
        Ok(())
    }
    fn temp(&self, i: usize) -> String {
        format!("__ulua_t{}_{}", self.owner, i)
    }
    fn release(&mut self, i: usize) {
        if let Some(s) = self.slots.get_mut(i) {
            s.busy = false;
        }
    }
    fn generated(&mut self, ty: &str, value: &str) -> Result<String, Diagnostic> {
        self.reserve_temporary()?;
        let name = format!("__ulua_g{}_{}", self.owner, self.generated.len());
        self.generated
            .push((name.clone(), ty.into(), value.into(), true));
        Ok(name)
    }
    fn plan(&mut self, parameter_ids: &[usize], overhead: usize) -> Result<String, Diagnostic> {
        let fixed = parameter_ids.len() + self.slots.len() + self.generated.len() + overhead;
        if self.slots.len() + self.generated.len() + overhead > TEMPORARY_BOUND {
            return Err(self.limit("limit:temporaries", "Non-spill local limit exceeded"));
        }
        let ids: Vec<_> = self
            .binding_ids
            .iter()
            .copied()
            .filter(|id| !parameter_ids.contains(id))
            .collect();
        let spill = fixed + ids.len() > LOCAL_BUDGET;
        let capacity = LOCAL_BUDGET - fixed;
        // Reserve every table before assigning any binding to a local.
        let mut table_count = usize::from(spill);
        if spill {
            loop {
                let needed = (ids.len() - capacity + table_count).div_ceil(SPILL_FIELDS);
                if needed == table_count {
                    break;
                }
                if needed > capacity {
                    table_count = capacity;
                    break;
                }
                table_count = needed;
            }
        }
        let available = capacity - table_count;
        let fields_per_table = if spill {
            (ids.len() - available).div_ceil(table_count)
        } else {
            0
        };
        let mut declarations = String::new();
        let mut tables = vec![Vec::new(); table_count];
        for (index, id) in ids.into_iter().enumerate() {
            let ty = self.module.bindings[id]
                .ty
                .as_ref()
                .ok_or_else(|| self.limit("C11", "Missing checked type"))?;
            let name = format!("__ulua_b{id}");
            if index < available {
                let _ = writeln!(
                    declarations,
                    "local {name}: {0} = ({1}) :: {0}",
                    self.function_types.luau(ty),
                    initial(ty)
                );
                self.locations.insert(id, name);
            } else {
                let table = (index - available) / fields_per_table;
                tables[table].push((name.clone(), self.function_types.luau(ty), initial(ty)));
                let page = (index - available) % fields_per_table / SPILL_FIELDS;
                let location = if fields_per_table > SPILL_FIELDS {
                    format!("{}.__ulua_page{page}.{name}", self.spill_name(table))
                } else {
                    format!("{}.{name}", self.spill_name(table))
                };
                self.locations.insert(id, location);
            }
        }
        for (table, fields) in tables.into_iter().enumerate() {
            if fields.is_empty() {
                continue;
            }
            let (ty, value) = types::spill_constructor(&fields);
            let _ = writeln!(
                declarations,
                "local {}: {ty} = ({value}) :: {ty}",
                self.spill_name(table)
            );
        }
        for (i, slot) in self.slots.iter().enumerate() {
            let _ = writeln!(
                declarations,
                "local {0}: {1} = ({2}) :: {1}",
                self.temp(i),
                self.function_types.luau(&slot.ty),
                initial(&slot.ty)
            );
        }
        for (name, ty, value, entry) in &self.generated {
            if !entry {
                continue;
            }
            let _ = writeln!(declarations, "local {name}: {ty} = ({value}) :: {ty}");
        }
        Ok(declarations)
    }
    fn spill_name(&self, table: usize) -> String {
        if table == 0 {
            format!("__ulua_spill{}", self.owner)
        } else {
            format!("__ulua_spill{}_{table}", self.owner)
        }
    }
    fn headers(&self, declarations: &str) -> String {
        declarations
            .lines()
            .map(|s| format!("{}{s}\n", if self.indent == 0 { "" } else { "    " }))
            .collect()
    }
}

pub(crate) fn emit(module: &ir::Module) -> Result<String, Diagnostic> {
    // The first pass counts reusable temporaries and generated loop storage.
    let mut owners = vec![vec![]; module.bindings.len() + 1];
    for (id, binding) in module.bindings.iter().enumerate() {
        owners[binding.owner].push(id);
    }
    for ids in &mut owners {
        ids.sort_by_key(|id| module.bindings[*id].start);
    }
    let function_types = types::FunctionTypes::new(module);
    let empty = HashMap::new();
    let mut chunk = Emitter::new(module, 0, &empty, &owners[0], &function_types);
    chunk.statements(&module.body, 0)?;
    let chunk_declarations = chunk.plan(&[], 7)?;
    let module_locations = chunk.locations;
    let mut out = String::from("--!strict\n");
    out.push_str(LIBRARIES);
    out.push_str(&function_types.declarations);
    out.push_str(&chunk_declarations);
    for f in &module.functions {
        let mut probe = Emitter::new(
            module,
            f.binding + 1,
            &module_locations,
            &owners[f.binding + 1],
            &function_types,
        );
        probe.defaults(f)?;
        probe.statements(&f.body, 0)?;
        let params: Vec<_> = f.parameters.iter().map(|(id, _)| *id).collect();
        let default_count = f.parameters.iter().filter(|(_, d)| d.is_some()).count();
        let declarations = probe.plan(&params, default_count)?;
        let mut e = Emitter::new(
            module,
            f.binding + 1,
            &module_locations,
            &owners[f.binding + 1],
            &function_types,
        );
        e.locations = probe.locations;
        e.slots = probe.slots;
        for slot in &mut e.slots {
            slot.busy = false;
        }
        e.defaults(f)?;
        e.statements(&f.body, 0)?;
        let mut parameters = vec![];
        let mut parameter_locals = String::new();
        for (id, default) in &f.parameters {
            let ty = module.bindings[*id]
                .ty
                .as_ref()
                .ok_or_else(|| e.limit("C11", "Missing parameter type"))?;
            if default.is_some() {
                parameters.push(format!("__ulua_arg{id}: {}?", function_types.luau(ty)));
                let _ = writeln!(
                    parameter_locals,
                    "    local __ulua_b{id}: {0} = ({1}) :: {0}",
                    function_types.luau(ty),
                    initial(ty)
                );
            } else {
                parameters.push(format!("__ulua_b{id}: {}", function_types.luau(ty)));
            }
        }
        let _ = writeln!(
            out,
            "{} = (function({}): {}",
            module_locations[&f.binding],
            parameters.join(", "),
            function_types.luau(&f.result)
        );
        out.push_str(&parameter_locals);
        out.push_str(&e.headers(&declarations));
        out.push_str(&e.out);
        let _ = writeln!(
            out,
            "end) :: {}",
            function_types.luau(
                module.bindings[f.binding]
                    .ty
                    .as_ref()
                    .ok_or_else(|| e.limit("internal", "Missing function type"))?
            )
        );
    }
    let mut body = Emitter::new(module, 0, &module_locations, &owners[0], &function_types);
    body.indent = 0;
    body.statements(&module.body, 0)?;
    out.push_str(&body.out);
    out.push_str("return {\n");
    for f in module.functions.iter().filter(|f| f.exported) {
        let _ = writeln!(
            out,
            "    [{}] = {},",
            quote(&f.name),
            module_locations[&f.binding]
        );
    }
    out.push_str("}\n");
    check_output(module, out.len())?;
    Ok(out)
}

fn check_output(module: &ir::Module, generated: usize) -> Result<(), Diagnostic> {
    let bound = crate::OUTPUT_PRELUDE_BYTES
        .saturating_add(crate::OUTPUT_FACTOR.saturating_mul(module.source.len()));
    if generated > bound {
        return Err(Diagnostic::at(
            &module.source,
            0,
            "internal",
            "Generated source exceeds the output bound",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod output_tests {
    use super::*;

    #[test]
    fn independently_oversized_ir_reaches_the_output_check() {
        let source = "export function main():void{print('x');}";
        let allocator = oxc_allocator::Allocator::default();
        let parsed =
            oxc_parser::Parser::new(&allocator, source, oxc_span::SourceType::ts()).parse();
        let mut module =
            crate::checker::check(&parsed.program, source.as_bytes(), crate::DEPTH_CEILING)
                .unwrap();
        let bound = crate::OUTPUT_PRELUDE_BYTES + crate::OUTPUT_FACTOR * source.len();
        assert!(check_output(&module, bound).is_ok());
        assert_eq!(
            check_output(&module, bound + 1).unwrap_err().code,
            "internal"
        );
        let ir::Statement::Located(_, form) = &mut module.functions[0].body[0] else {
            panic!("located statement");
        };
        let ir::Statement::Eval(expr) = form.as_mut() else {
            panic!("call statement");
        };
        let ir::ExprKind::Call(_, arguments) = &mut expr.kind else {
            panic!("call expression");
        };
        arguments[0].kind = ir::ExprKind::String("x".repeat(bound + 1));
        // The original source size remains intact. The generated text independently exceeds its bound.
        let error = emit(&module).unwrap_err();
        assert_eq!(error.code, "internal");
        assert_eq!(error.message, "Generated source exceeds the output bound");
    }
}
