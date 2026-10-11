//! Strict Luau emission with ordered values and a per-function local budget.
mod expressions;
mod statements;
use crate::{
    Diagnostic, LOCAL_BUDGET, TEMPORARY_BOUND,
    ir::{self, Type},
};
use std::{collections::HashMap, fmt::Write};

const LIBRARIES: &str = "local __ulua_print = print\nlocal __ulua_fmod = math.fmod\nlocal __ulua_tostring = tostring\nlocal __ulua_gsub = string.gsub\nlocal __ulua_huge = math.huge\nlocal function __ulua_format(__ulua_n: number): string\n    if __ulua_n ~= __ulua_n then return \"NaN\" end\n    if __ulua_n == __ulua_huge then return \"Infinity\" end\n    if __ulua_n == -__ulua_huge then return \"-Infinity\" end\n    if __ulua_n == 0 then return \"0\" end\n    local __ulua_s = __ulua_gsub(__ulua_tostring(__ulua_n), \"e([+-])0*(%d)\", \"e%1%2\")\n    return __ulua_s\nend\n";

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
        n.to_string()
    }
}
struct Slot {
    ty: Type,
    busy: bool,
}
struct Emitter<'a> {
    module: &'a ir::Module,
    owner: usize,
    out: String,
    indent: usize,
    slots: Vec<Slot>,
    locations: HashMap<usize, String>,
    copies: HashMap<usize, String>,
    generated: Vec<(String, String, String, bool)>,
    loops: Vec<Loop>,
}
#[derive(Clone)]
struct Loop {
    tail: Option<String>,
}
impl<'a> Emitter<'a> {
    fn new(module: &'a ir::Module, owner: usize, locations: HashMap<usize, String>) -> Self {
        Self {
            module,
            owner,
            out: String::new(),
            indent: 1,
            slots: vec![],
            locations,
            copies: HashMap::new(),
            generated: vec![],
            loops: vec![],
        }
    }
    fn guard(&self, depth: u32) -> Result<u32, Diagnostic> {
        let next = depth + 1;
        if next >= self.module.depth {
            return Err(self.limit("limit:nesting-depth", "Emitter depth limit exceeded"));
        }
        Ok(next)
    }
    fn limit(&self, code: &'static str, message: &str) -> Diagnostic {
        Diagnostic::at(&[], 0, code, message)
    }
    fn line(&mut self, text: impl AsRef<str>) {
        let _ = writeln!(self.out, "{}{}", "    ".repeat(self.indent), text.as_ref());
    }
    fn location(&self, id: usize) -> Result<String, Diagnostic> {
        self.copies
            .get(&id)
            .or_else(|| self.locations.get(&id))
            .cloned()
            .ok_or_else(|| self.limit("C11", "Missing checked binding"))
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
        let mut ids: Vec<_> = self
            .module
            .bindings
            .iter()
            .enumerate()
            .filter(|(i, b)| b.owner == self.owner && !parameter_ids.contains(i))
            .map(|(i, _)| i)
            .collect();
        ids.sort_by_key(|id| self.module.bindings[*id].start);
        let spill = fixed + ids.len() > LOCAL_BUDGET;
        let available = LOCAL_BUDGET - fixed - usize::from(spill);
        let mut declarations = String::new();
        let mut fields = vec![];
        let mut values = vec![];
        for (index, id) in ids.into_iter().enumerate() {
            let ty = self.module.bindings[id]
                .ty
                .as_ref()
                .ok_or_else(|| self.limit("C11", "Missing checked type"))?;
            let name = format!("__ulua_b{id}");
            if index < available {
                let _ = writeln!(
                    declarations,
                    "local {name}: {} = {}",
                    ty.luau(),
                    initial(ty)
                );
                self.locations.insert(id, name);
            } else {
                fields.push(format!("{name}: {}", ty.luau()));
                values.push(format!("{name} = {}", initial(ty)));
                self.locations
                    .insert(id, format!("__ulua_spill{}.{name}", self.owner));
            }
        }
        if spill {
            let _ = writeln!(
                declarations,
                "local __ulua_spill{}: {{ {} }} = {{ {} }}",
                self.owner,
                fields.join(", "),
                values.join(", ")
            );
        }
        for (i, slot) in self.slots.iter().enumerate() {
            let _ = writeln!(
                declarations,
                "local {}: {} = {}",
                self.temp(i),
                slot.ty.luau(),
                initial(&slot.ty)
            );
        }
        for (name, ty, value, entry) in &self.generated {
            if !entry {
                continue;
            }
            let _ = writeln!(declarations, "local {name}: {ty} = {value}");
        }
        Ok(declarations)
    }
    fn headers(&self, declarations: &str) -> String {
        declarations
            .lines()
            .map(|s| format!("{}{s}\n", "    ".repeat(self.indent)))
            .collect()
    }
}

pub(crate) fn emit(module: &ir::Module) -> Result<String, Diagnostic> {
    // The first pass counts reusable temporaries and generated loop storage.
    let raw: HashMap<_, _> = module
        .bindings
        .iter()
        .enumerate()
        .map(|(i, _)| (i, format!("__ulua_b{i}")))
        .collect();
    let mut chunk = Emitter::new(module, 0, raw.clone());
    chunk.statements(&module.body, 0)?;
    let chunk_declarations = chunk.plan(&[], 6)?;
    let module_locations = chunk.locations.clone();
    let mut out = String::from("--!strict\n");
    out.push_str(LIBRARIES);
    out.push_str(&chunk_declarations);
    for f in &module.functions {
        let mut probe = Emitter::new(module, f.binding + 1, raw.clone());
        probe.defaults(f)?;
        probe.statements(&f.body, 0)?;
        let params: Vec<_> = f.parameters.iter().map(|(id, _)| *id).collect();
        let default_count = f.parameters.iter().filter(|(_, d)| d.is_some()).count();
        let declarations = probe.plan(&params, default_count)?;
        let mut locations = module_locations.clone();
        locations.extend(
            probe
                .locations
                .iter()
                .filter(|(id, _)| module.bindings[**id].owner == f.binding + 1)
                .map(|(id, s)| (*id, s.clone())),
        );
        let mut e = Emitter::new(module, f.binding + 1, locations);
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
                parameters.push(format!("__ulua_arg{id}: {}?", ty.luau()));
                let _ = writeln!(
                    parameter_locals,
                    "    local __ulua_b{id}: {} = {}",
                    ty.luau(),
                    initial(ty)
                );
            } else {
                parameters.push(format!("__ulua_b{id}: {}", ty.luau()));
            }
        }
        let _ = writeln!(
            out,
            "{} = function({}): {}",
            module_locations[&f.binding],
            parameters.join(", "),
            f.result.luau()
        );
        out.push_str(&parameter_locals);
        out.push_str(&e.headers(&declarations));
        out.push_str(&e.out);
        out.push_str("end\n");
    }
    let mut body = Emitter::new(module, 0, module_locations.clone());
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
    Ok(out)
}
