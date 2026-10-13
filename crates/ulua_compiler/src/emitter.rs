//! Strict Luau emission with ordered values and a per-function local budget.
mod accessors;
mod arrays;
mod asynchronous;
mod constants;
mod exceptions;
mod expressions;
mod functions;
mod joins;
pub(crate) mod methods;
mod output;
mod prelude;
mod statements;
mod storage;
mod strings;
mod types;
use crate::{
    Diagnostic, LOCAL_BUDGET, TEMPORARY_BOUND,
    ir::{self, Type},
};
use std::{collections::HashMap, fmt::Write};

const SPILL_FIELDS: usize = 128;

pub(crate) use prelude::LIBRARIES;
const LIBRARY_LOCALS: usize = 14;

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
    match *ty {
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
#[derive(Clone)]
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
    statement_offset: usize,
    output: Option<&'a output::Output>,
    credited: usize,
    copies: HashMap<usize, String>,
    generated_count: usize,
    generated_live: usize,
    generated_peak: usize,
    overhead: usize,
    storage_offset: usize,
    loops: Vec<Loop>,
    protected: Vec<Protection>,
    protected_slots: Vec<Vec<usize>>,
    joins: joins::Joins,
    context: Option<&'a functions::Emission>,
    upvalues: std::collections::HashSet<String>,
    async_void: bool,
    activation_parameters: std::collections::HashSet<String>,
    activation_upvalues: std::collections::HashSet<String>,
}
#[derive(Clone)]
struct Loop {
    tail: Option<String>,
}
struct Protection {
    loop_base: usize,
    locals: std::collections::HashSet<String>,
    upvalues: std::collections::HashSet<String>,
    slots: std::collections::BTreeSet<usize>,
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
            statement_offset: 0,
            output: None,
            credited: 0,
            copies: HashMap::new(),
            generated_count: 0,
            generated_live: 0,
            generated_peak: 0,
            overhead: if owner == 0 {
                LIBRARY_LOCALS + if module.exceptions { 4 } else { 0 }
            } else {
                0
            },
            storage_offset: 0,
            loops: vec![],
            protected: vec![],
            protected_slots: vec![],
            joins: joins::Joins::default(),
            context: None,
            upvalues: std::collections::HashSet::new(),
            async_void: false,
            activation_parameters: Default::default(),
            activation_upvalues: Default::default(),
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
        // The storage probe needs slots and joins, but it discards emitted text.
        if self.context.is_none() {
            return;
        }
        let text = text.as_ref();
        let bytes = text.len() + if self.indent == 0 { 1 } else { 5 };
        let credited = std::mem::take(&mut self.credited);
        if let Some(output) = self.output {
            output.charge(bytes.saturating_sub(credited), self.statement_offset);
            if output.exceeded() {
                return;
            }
        }
        let _ = writeln!(
            self.out,
            "{}{}",
            if self.indent == 0 { "" } else { "    " },
            text
        );
    }
    fn output_check(&self) -> Result<(), Diagnostic> {
        self.output
            .map_or(Ok(()), |output| output.check(self.module))
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
        if self.uses_accessor(id) {
            self.joins.clear(id);
            let name = accessors::name(id);
            self.capture(&name);
            let ty = self.module.bindings[id]
                .ty
                .ok_or_else(|| self.limit("internal", "Missing accessor type"))?;
            return Ok(format!(
                "{name}(false, ({}) :: {})",
                initial(&ty),
                self.function_types.luau(&ty)
            ));
        }
        let location = self.location(id)?;
        self.capture(&location);
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
        if self.uses_accessor(id) {
            let name = accessors::name(id);
            self.capture(&name);
            self.line(format!("{name}(true, ({}) :: {ty})", value.as_ref()));
            return Ok(());
        }
        let location = self.location(id)?;
        self.capture(&location);
        self.write(&location, value, &ty);
        Ok(())
    }
    fn write_binding_slot(&mut self, id: usize, slot: usize) -> Result<(), Diagnostic> {
        let ty = self.module.bindings[id]
            .ty
            .ok_or_else(|| self.limit("internal", "Missing checked binding type"))?;
        if ty == self.slots[slot].ty {
            self.joins.clear(id);
            if self.uses_accessor(id) {
                let name = accessors::name(id);
                self.capture(&name);
                self.line(format!("{name}(true, {})", self.temp(slot)));
                return Ok(());
            }
            let location = self.location(id)?;
            self.capture(&location);
            // Both locals have this declared type. A redundant cast expands large alias graphs.
            self.line(format!("{location} = {}", self.temp(slot)));
            Ok(())
        } else {
            self.write_binding(id, self.temp(slot))
        }
    }
    fn capture(&mut self, location: &str) {
        if self.context.is_none() {
            return;
        }
        let name = location.split('.').next().unwrap_or(location);
        if self.activation_parameters.contains(name) {
            self.activation_upvalues.insert(name.to_owned());
        }
        // A nested closure also requires each intermediate closure to capture the name.
        for frame in self.protected.iter_mut().rev() {
            if frame.locals.contains(name) {
                return;
            }
            frame.upvalues.insert(name.to_owned());
        }
        if name
            .strip_prefix("__ulua_caught")
            .and_then(|id| id.parse::<usize>().ok())
            .and_then(|id| self.module.bindings.get(id))
            .is_some_and(|b| b.owner == self.owner)
        {
            return;
        }
        if let Some(context) = self.context {
            if context.locals[self.owner].contains(name)
                || name.starts_with(&format!("__ulua_g{}_", self.owner))
                || name.starts_with(&format!("__ulua_t{}_", self.owner))
            {
                return;
            }
        } else {
            return;
        }
        self.upvalues.insert(name.to_owned());
    }
    fn slot(&mut self, ty: &Type) -> Result<usize, Diagnostic> {
        let ty = &self.module.types.identity(*ty);
        if let Some(i) = self.slots.iter().position(|s| !s.busy && s.ty == *ty) {
            self.slots[i].busy = true;
            if let Some(frame) = self.protected.last_mut() {
                frame.slots.insert(i);
            }
            self.capture(&self.temp(i));
            Ok(i)
        } else {
            self.reserve_temporary()?;
            self.storage_offset = self.offset;
            let i = self.slots.len();
            self.slots.push(Slot {
                ty: *ty,
                busy: true,
            });
            if let Some(frame) = self.protected.last_mut() {
                frame.slots.insert(i);
            }
            self.capture(&self.temp(i));
            Ok(i)
        }
    }
    fn reserve_temporary(&self) -> Result<(), Diagnostic> {
        if self.slots.len() + self.generated_live + self.overhead >= TEMPORARY_BOUND {
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
    fn generated(&mut self) -> Result<String, Diagnostic> {
        self.reserve_temporary()?;
        self.storage_offset = self.offset;
        let name = format!("__ulua_g{}_{}", self.owner, self.generated_count);
        self.generated_live += 1;
        self.generated_peak = self.generated_peak.max(self.generated_live);
        self.generated_count += 1;
        if let Some(frame) = self.protected.last_mut() {
            frame.locals.insert(name.clone());
        }
        Ok(name)
    }
    fn headers(&self, declarations: &str) -> String {
        declarations
            .lines()
            .map(|s| format!("{}{s}\n", if self.indent == 0 { "" } else { "    " }))
            .collect()
    }
}

pub(crate) fn emit(module: &ir::Module, factor: usize) -> Result<String, Diagnostic> {
    let bound = crate::OUTPUT_PRELUDE_BYTES
        .saturating_add(if module.exceptions {
            prelude::EXCEPTIONS.len()
        } else {
            0
        })
        .saturating_add(if module.asynchronous {
            asynchronous::PRELUDE.len()
        } else {
            0
        })
        .saturating_add(methods::prelude(&module.methods)?.0.len())
        .saturating_add(factor.saturating_mul(module.source.len()));
    functions::emit(module, bound)
}

#[cfg(test)]
mod output_tests {
    use super::*;

    #[test]
    fn the_budget_counts_nested_output_exactly_once() {
        for source in [
            "\nexport function main():void{print('a');}",
            "\nexport async function main():Promise<void>{}",
            "\nexport async function main():Promise<void>{const f=async():Promise<number>=>{return 1;};print(`${await f()}`);}",
            "\nexport function main():void{function f(x:number=1):number{return x;}let g=():number=>f();print(`${g()}`);}",
            "\nexport function main():void{for(let i=0;i<2;i++){let g=():number=>i;print(`${g()}`);}}",
        ] {
            let allocator = oxc_allocator::Allocator::default();
            let parsed =
                oxc_parser::Parser::new(&allocator, source, oxc_span::SourceType::ts()).parse();
            let module =
                crate::checker::check(&parsed.program, source.as_bytes(), crate::DEPTH_CEILING)
                    .unwrap();
            let generated = emit(&module, crate::OUTPUT_FACTOR).unwrap();
            assert_eq!(
                functions::emit(&module, generated.len()).unwrap(),
                generated
            );
            let error = functions::emit(&module, generated.len() - 1).unwrap_err();
            assert_eq!(
                (error.code, error.offset, error.line, error.column),
                ("limit:output", 1, 2, 1)
            );
        }
    }

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
        let output = output::Output::new(bound);
        output.charge(bound, 0);
        assert!(output.check(&module).is_ok());
        output.charge(1, source.find("print").unwrap());
        output.charge(1, 0);
        let crossing = output.check(&module).unwrap_err();
        assert_eq!(
            (
                crossing.code,
                crossing.offset,
                crossing.line,
                crossing.column
            ),
            ("limit:output", 28, 1, 29)
        );
        let statement = module.functions[0]
            .body
            .iter_mut()
            .find(|s| matches!(s.form(), ir::Statement::Eval(_)))
            .unwrap();
        let ir::Statement::Located(_, form) = statement else {
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
        let error = emit(&module, crate::OUTPUT_FACTOR).unwrap_err();
        assert_eq!(error.code, "limit:output");
        assert_eq!(error.offset, source.find("print").unwrap() as u64);
        assert_eq!((error.line, error.column), (1, 29));
        assert_eq!(error.message, "Generated source exceeds the output bound");
    }

    #[test]
    fn final_output_check_rejects_an_independent_counter_mismatch() {
        let source = "export function main():void{}";
        let allocator = oxc_allocator::Allocator::default();
        let parsed =
            oxc_parser::Parser::new(&allocator, source, oxc_span::SourceType::ts()).parse();
        let module =
            crate::checker::check(&parsed.program, source.as_bytes(), crate::DEPTH_CEILING)
                .unwrap();
        let output = output::Output::new(100);
        output.charge(10, 0);
        output.finish(&module, 10).unwrap();
        for written in [9, 11] {
            assert_eq!(
                output.finish(&module, written).unwrap_err().code,
                "internal"
            );
        }
    }
}
