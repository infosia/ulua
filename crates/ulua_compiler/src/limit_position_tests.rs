//! Each compiler limit reports its causing form before a later statement.
use crate::{Diagnostic, Limits, compile};

fn position(source: &str, marker: &str, code: &str, error: Diagnostic) {
    let offset = source.find(marker).unwrap();
    let prefix = &source[..offset];
    let line = prefix.bytes().filter(|b| *b == b'\n').count() + 1;
    let column = prefix.rsplit('\n').next().unwrap().len() + 1;
    assert_eq!(
        (error.code, error.offset, error.line, error.column),
        (code, offset as u64, line as u32, column as u32),
        "{error:?}"
    );
}

#[test]
fn temporaries_point_at_the_crossing_form_with_module_and_default_overhead() {
    let head = (0..32)
        .map(|i| format!("i{i}=0"))
        .collect::<Vec<_>>()
        .join(",");
    for (prefix, heads, tails, suffix) in [
        ("export function main():void{".into(), 4, 2, "}"),
        (
            format!(
                "function f({}):void{{",
                (0..32)
                    .map(|i| format!("p{i}:number=1"))
                    .collect::<Vec<_>>()
                    .join(",")
            ),
            3,
            3,
            "}export function main():void{}",
        ),
    ] {
        let source = format!(
            "{prefix}{}{}for(;false;){{print('later');}}{}{suffix}",
            format!("for(let {head};false;){{").repeat(heads),
            "for(;false;){".repeat(tails),
            "}".repeat(heads + tails)
        );
        position(
            &source,
            "for(;false;){print",
            "limit:temporaries",
            compile(source.as_bytes(), Limits::default()).unwrap_err(),
        );
    }
    let params = (0..32)
        .map(|i| format!("p{i}:number"))
        .collect::<Vec<_>>()
        .join(",");
    let mut args = vec!["1"; 32];
    // Four fixed helpers reduce the module's remaining temporary capacity by four.
    args[20] = "987654321";
    let mut value = format!("f({})", args.join(","));
    for _ in 0..3 {
        value = format!("f({},{value})", vec!["1"; 31].join(","));
    }
    let source = format!(
        "function f({params}):number{{return p0;}}\nconst xs={value};\nexport function main():void{{print('later');}}"
    );
    position(
        &source,
        "987654321",
        "limit:temporaries",
        compile(source.as_bytes(), Limits::default()).unwrap_err(),
    );
}

#[test]
fn upvalues_point_at_the_closure_before_its_later_statement() {
    let mut bindings = String::new();
    let mut scopes = 0;
    for i in 0usize..193 {
        if i > 0 && i.is_multiple_of(crate::CAPTURE_LOCALS) {
            bindings.push_str(&format!("function scope{scopes}():number{{"));
            scopes += 1;
        }
        bindings.push_str(&format!("let x{i}={i};"));
    }
    let reads = (0..193)
        .map(|i| format!("total+=x{i};"))
        .collect::<String>();
    let mut source = format!(
        "export function score():number{{{bindings}\nconst f=():number=>{{let total=0;{reads}return total;}};print('later');return f();"
    );
    for scope in (0..scopes).rev() {
        source.push_str(&format!("}}return scope{scope}();"));
    }
    source.push_str("}export function main():void{}");
    position(
        &source,
        "():number=>",
        "limit:upvalues",
        compile(source.as_bytes(), Limits::default()).unwrap_err(),
    );
}

#[test]
fn parameters_point_at_the_parameter_form_before_the_body() {
    for arrow in [false, true] {
        let params = (0..33)
            .map(|i| format!("p{i}:number"))
            .collect::<Vec<_>>()
            .join(",");
        let source = if arrow {
            format!(
                "export function main():void{{\n  const f=({params}):void=>{{}};\n  print('later');}}"
            )
        } else {
            format!(
                "\nfunction f({params}):void{{print('body');}}\nexport function main():void{{print('later');}}"
            )
        };
        position(
            &source,
            "(p0",
            "limit:parameters",
            compile(source.as_bytes(), Limits::default()).unwrap_err(),
        );
    }
}

#[test]
fn arguments_point_at_the_call_before_the_later_print() {
    let source = format!(
        "function f():void{{}}\nexport function main():void{{\n  f({});\n  print('later');}}",
        vec!["1"; 33].join(",")
    );
    position(
        &source,
        "f(1",
        "limit:arguments",
        compile(source.as_bytes(), Limits::default()).unwrap_err(),
    );
}

#[test]
fn template_parts_point_at_the_template_before_the_later_print() {
    let source = format!(
        "export function main():void{{\n  const text=`{}`;\n  print('later');}}",
        "${1}".repeat(513)
    );
    position(
        &source,
        "`",
        "limit:template-parts",
        compile(source.as_bytes(), Limits::default()).unwrap_err(),
    );
}

#[test]
fn nesting_depth_points_at_the_crossing_ast_form() {
    let source = "export function main():void{\n  const xs=[1];\n  print('later');}";
    let allocator = oxc_allocator::Allocator::default();
    let parsed = oxc_parser::Parser::new(&allocator, source, oxc_span::SourceType::ts()).parse();
    position(
        source,
        "[1]",
        "limit:nesting-depth",
        crate::depth::check(&parsed.program, 5, source.as_bytes()).unwrap_err(),
    );
}

#[test]
fn output_points_at_the_crossing_statement_before_the_later_print() {
    let source = "export function main():void{\n  print('early');\n  print('later');}";
    let allocator = oxc_allocator::Allocator::default();
    let parsed = oxc_parser::Parser::new(&allocator, source, oxc_span::SourceType::ts()).parse();
    let mut module =
        crate::checker::check(&parsed.program, source.as_bytes(), crate::DEPTH_CEILING).unwrap();
    let statement = module.functions[0]
        .body
        .iter_mut()
        .find(|s| matches!(s.form(), crate::ir::Statement::Eval(_)))
        .unwrap();
    let crate::ir::Statement::Located(_, s) = statement else {
        panic!("located statement")
    };
    let crate::ir::Statement::Eval(e) = s.as_mut() else {
        panic!("call statement")
    };
    let crate::ir::ExprKind::Call(_, args) = &mut e.kind else {
        panic!("call")
    };
    args[0].kind = crate::ir::ExprKind::String("x".repeat(10_000));
    position(
        source,
        "print('early')",
        "limit:output",
        crate::emitter::emit(&module, crate::OUTPUT_FACTOR).unwrap_err(),
    );
}
