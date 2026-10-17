//! Each source builds the fact boundary that its emitted-form assertion checks.
use super::*;
use crate::{Limits, compile};
fn form(body: &str, native: usize, fmod: usize) {
    let source = format!("export function main():number{{{body}}}");
    let code = compile(source.as_bytes(), Limits::default()).unwrap().luau;
    assert_eq!(code.matches(" % ").count(), native, "{source}\n{code}");
    assert_eq!(
        code.matches("__ulua_fmod(").count(),
        fmod,
        "{source}\n{code}"
    );
}
macro_rules! forms {
    ($($name:ident: $body:literal, $native:literal, $fmod:literal;)*) => {$ (
        #[test] fn $name() { form($body, $native, $fmod); }
    )*};
}
forms! {
    literal_max: "return 9007199254740992%3;", 1, 0;
    literal_above_max: "return 9007199254740994%3;", 0, 1;
    literal_zero: "return 0%3;", 1, 0;
    noninteger_literal: "return 1.5%3;", 0, 1;
    negative_literal: "return -1%3;", 0, 1;
    negative_zero: "return -0%3;", 0, 1;
    noninteger_divisor: "return 7%0.1;", 0, 1;
    infinite_divisor: "return 7%Infinity;", 0, 1;
    divisor_zero: "return 7%0;", 0, 1;
    divisor_lower_zero: "let d=1;d=0;return 7%d;", 0, 1;
    increment_saturation: "let x=9007199254740992;x++;return x%3;", 1, 0;
    left_increment_saturation: "let x=9007199254740992;x=1+x;return x%3;", 1, 0;
    addition_boundary: "let a=9007199254740990;return (a+2)%3;", 1, 0;
    addition_overflow: "let a=9007199254740992;return (a+2)%3;", 0, 1;
    binding_one_is_not_literal_one: "let a=9007199254740992;let b=1;return (a+b)%3;", 0, 1;
    subtraction: "let a=8;let b=3;return (a-b)%3;", 1, 0;
    subtraction_equal: "let a=3;return (a-a)%3;", 1, 0;
    subtraction_lower_bound: "let a=3;a=8;let b=4;return (a-b)%3;", 0, 1;
    decrement_negative: "let d=1;d--;return 8%d;", 0, 1;
    product_boundary: "return (4503599627370496*2)%3;", 1, 0;
    product_overflow: "return (9007199254740992*2)%3;", 0, 1;
    product_negative_zero: "let z=0;return (z * -1)%3;", 0, 1;
    remainder_chain: "let x=8;let y=x%3;return y%2;", 2, 0;
    parentheses: "let x=8;return (((x)))%((3));", 1, 0;
    division: "return (8/2)%3;", 0, 1;
    unary_minus: "let x=8;return (-x)%3;", 0, 1;
    field: "const x={v:8};return x.v%3;", 0, 1;
    element: "const x=[8];return x[0]%3;", 0, 1;
    call_result: "const f=()=>8;return f()%3;", 0, 1;
    conditional: "const b=true;return (b?8:4)%3;", 0, 1;
    closure_invalid_write: "let c=8;const f=():void=>{c=-4;};f();return c%3;", 0, 1;
    closure_valid_write: "let c=8;const f=():void=>{c=4;};f();return c%3;", 1, 0;
    closure_increment: "let c=8;const f=():void=>{c++;};f();return c%3;", 1, 0;
    compound_remainder: "let x=8;x%=3;return x;", 1, 0;
    compound_remainder_invalid: "let x=-8;x%=3;return x;", 0, 1;
    compound_addition: "let x=8;x+=2;return x%3;", 0, 1;
    compound_multiplication: "let x=8;x*=2;return x%3;", 0, 1;
    for_of: "let r=0;for(const x of [8]){r=x%3;}return r;", 0, 1;
    trial_division: "let r=0;for(let n=2;n<=100;n++){let d=2;while(d*d<=n){r=n%d;d++;}}return r;", 1, 0;
    lcg: "let x=1;for(let i=0;i<10;i++){x=(x*1664525+1013904223)%4294967296;}return x;", 1, 0;
    counter: "let c=0;for(let i=0;i<10;i++){c=(c+1)%4294967296;}return c;", 1, 0;
}
#[test]
fn parameter_and_module_binding_have_no_fact() {
    for source in [
        "export function main(x:number):number{return x%3;}",
        "export function main(x:number=8):number{return x%3;}",
        "let x=8;export function main():number{return x%3;}",
    ] {
        let code = compile(source.as_bytes(), Limits::default()).unwrap().luau;
        assert_eq!(code.matches("__ulua_fmod(").count(), 1, "{code}");
    }
}
pub(super) fn checked(source: &str) -> Module {
    let allocator = oxc_allocator::Allocator::default();
    let parsed = oxc_parser::Parser::new(
        &allocator,
        source,
        oxc_span::SourceType::ts().with_module(true),
    )
    .parse();
    assert!(parsed.diagnostics.is_empty());
    crate::checker::check_with_grants(&parsed.program, source.as_bytes(), 128, &[]).unwrap()
}
fn chain(count: usize) -> String {
    let mut body = "let x0=1;".to_owned();
    for i in 1..count {
        body += &format!("let x{i}=x{};", i - 1);
    }
    body + &format!("return x{}%3;", count - 1)
}
fn cycle(count: usize) -> String {
    chain(count).replace(
        &format!("return x{}%3;", count - 1),
        &format!("x0=x{};", count - 1),
    )
}
#[test]
fn component_round_limit_preserves_independent_counter() {
    let body = format!("{}return x6%3;", cycle(7));
    let mut module = checked(&format!("export function main():number{{{body}}}"));
    assert_eq!(analyze(&mut module).unwrap().rounds, FACT_ROUNDS);
    form(&body, 1, 0);
    let body = format!("{}let i=0;i++;return x7%3+i%40;", cycle(8));
    form(&body, 1, 1);
}
#[test]
fn fib_component_does_not_invalidate_loop_counter() {
    form(
        "let result=0;for(let i=0;i<10;i++){let a=0;let b=1;const m=i%40;for(let j=0;j<m;j++){const next=a+b;a=b;b=next;result=next%3;}result=result+a%3+b%3;}return result;",
        1,
        3,
    );
}
#[test]
fn dependent_component_reads_failed_component_as_no_fact() {
    form(
        &format!("{}let dependent=x7;return dependent%3;", cycle(8)),
        0,
        1,
    );
}
#[test]
fn cycle_across_closure_write_shares_component_limit() {
    for (count, native, fmod) in [(7, 1, 0), (8, 0, 1)] {
        let body = chain(count).replace(
            &format!("return x{}%3;", count - 1),
            &format!(
                "const f=():void=>{{x0=x{};}};f();return x{}%3;",
                count - 1,
                count - 1
            ),
        );
        form(&body, native, fmod);
    }
}
#[test]
fn acyclic_chain_has_no_component_round_limit() {
    let body = chain(32);
    let mut module = checked(&format!("export function main():number{{{body}}}"));
    assert_eq!(analyze(&mut module).unwrap().rounds, 2);
    form(&body, 1, 0);
}
#[test]
fn widening_thresholds_exclude_other_bindings() {
    let mut module =
        checked("export function main():number{let x=0;x++;const other=7%3;return x;}");
    assert_eq!(analyze(&mut module).unwrap().rounds, 3);
}
#[test]
fn nested_independent_component_keeps_its_fact() {
    let body = format!(
        "{}const f=():number=>{{let n=7;return n%3;}};return x7%3+f();",
        cycle(8)
    );
    form(&body, 1, 1);
}
#[test]
fn report_corpus_and_workload_facts() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut largest = 0;
    let mut corpus_sites = 0;
    let mut corpus_checks = 0;
    let mut corpus_compounds = 0;
    let mut corpus_shadow_tests = 0;
    for directory in [
        "corpus/accept",
        "corpus/trap",
        "crates/ulua_bench/workloads/ulua",
    ] {
        let mut directory_largest = 0;
        let mut paths: Vec<_> = std::fs::read_dir(root.join(directory))
            .unwrap()
            .map(|e| e.unwrap().path())
            .filter(|p| p.extension().is_some_and(|e| e == "ts"))
            .collect();
        paths.sort();
        for path in paths {
            let source = std::fs::read_to_string(&path).unwrap();
            let allocator = oxc_allocator::Allocator::default();
            let parsed = oxc_parser::Parser::new(
                &allocator,
                &source,
                oxc_span::SourceType::ts().with_module(true),
            )
            .parse();
            // Host fixtures need their grants; the runtime corpus run covers them separately.
            let grants: Vec<_> = source
                .lines()
                .filter_map(|l| l.strip_prefix("// grants:"))
                .flat_map(|h| h.split(','))
                .map(|name| {
                    let name = name.trim();
                    let (parameters, result) = match name {
                        "hostAdd" => (vec![1, 1], 1),
                        "hostConcat" => (vec![3, 3], 3),
                        "hostNot" => (vec![2], 2),
                        "hostLog" => (vec![3], 0),
                        "hostLater" => (vec![1], 1),
                        _ => panic!("Unknown grant fixture: {name}"),
                    };
                    let mut signature = crate::HostSignature::new(name, parameters, result);
                    signature.asynchronous = name == "hostLater";
                    signature
                })
                .collect();
            let mut module =
                crate::checker::check_with_grants(&parsed.program, source.as_bytes(), 128, &grants)
                    .unwrap();
            let report = analyze(&mut module).unwrap();
            let (index_sites, compound_writes) = super::index_tests::index_counts(&mut module);
            assert_eq!(index_sites, report.index_sites.len(), "{}", path.display());
            let code = crate::emitter::emit(&module, crate::OUTPUT_FACTOR).unwrap();
            let checks = code
                .lines()
                .filter(|line| {
                    (line.contains(" < #__ulua_") || line.contains(" <= #__ulua_"))
                        && !line.contains("__ulua_floor")
                        && (line.contains("then __ulua_trap(1) end")
                            || line.contains(": boolean = not ("))
                })
                .count();
            assert_eq!(checks, index_sites + compound_writes, "{}", path.display());
            let shadow_tests = code
                .lines()
                .filter(|line| line.contains("then error(\"proven index shadow mismatch\") end"))
                .count();
            #[cfg(feature = "remainder-shadow")]
            if std::env::var_os("ULUA_REMAINDER_SHADOW").is_some() {
                assert_eq!(shadow_tests, checks, "{}", path.display());
            }
            if directory.starts_with("corpus") {
                corpus_sites += report.index_sites.len();
                corpus_checks += checks;
                corpus_compounds += compound_writes;
                corpus_shadow_tests += shadow_tests;
            } else {
                let id = path.file_stem().unwrap().to_str().unwrap();
                let expected = match id {
                    "queen" => 3,
                    "particles" | "collect" => 2,
                    _ => 0,
                };
                assert_eq!(checks, expected, "{id}");
                #[cfg(feature = "remainder-shadow")]
                if expected > 0 && std::env::var_os("ULUA_REMAINDER_SHADOW").is_some() {
                    assert!(shadow_tests > 0, "{id}");
                }
                eprintln!(
                    "proven workload={} ir_sites={} compound_writes={compound_writes} emitted_checks={checks} shadow_tests={shadow_tests}",
                    path.file_stem().unwrap().to_str().unwrap(),
                    report.index_sites.len()
                );
            }
            largest = largest.max(report.rounds);
            directory_largest = directory_largest.max(report.rounds);
            let sites: Vec<_> = report
                .sites
                .iter()
                .map(|start| {
                    let prefix = &source[..*start as usize];
                    let line = prefix.bytes().filter(|b| *b == b'\n').count() + 1;
                    format!("{line}:{}", source.lines().nth(line - 1).unwrap().trim())
                })
                .collect();
            eprintln!(
                "facts {directory}/{} rounds={} native={sites:?}",
                path.file_name().unwrap().to_string_lossy(),
                report.rounds
            );
        }
        eprintln!("largest fact round count for {directory}={directory_largest}");
    }
    assert_eq!(corpus_checks, corpus_sites + corpus_compounds);
    eprintln!(
        "proven corpus ir_sites={corpus_sites} compound_writes={corpus_compounds} emitted_checks={corpus_checks} shadow_tests={corpus_shadow_tests}"
    );
    eprintln!("largest fact round count={largest}");
    assert!(largest <= FACT_ROUNDS);
}

#[path = "../../../ulua/tests/support/remainder_growth.rs"]
mod growth;
#[test]
fn growth_shape_reaches_eight_rounds() {
    for bytes in [4096, 16384, 65536] {
        let mut module = checked(&growth::source("fact-rounds", bytes));
        let report = analyze(&mut module).unwrap();
        assert_eq!(report.rounds, FACT_ROUNDS);
        assert!(!report.sites.is_empty());
        assert_eq!(report.components_at_limit, report.sites.len());
        assert!(report.components_at_limit >= bytes / 100);
    }
}

#[cfg(feature = "remainder-shadow")]
#[test]
#[ignore = "Run with ULUA_REMAINDER_SHADOW=1 to test false native flags"]
fn shadow_rejects_false_native_flags() {
    assert!(std::env::var_os("ULUA_REMAINDER_SHADOW").is_some());
    struct Force;
    impl Visitor for Force {
        fn expression(&mut self, e: &mut Expr) -> Result<(), Diagnostic> {
            if let E::Binary(op @ "%", _, _) = &mut e.kind {
                *op = NATIVE_REMAINDER;
            }
            Ok(())
        }
    }
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let directory =
        std::env::temp_dir().join(format!("ulua-remainder-false-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    for (i, expression) in ["-0%5", "7%0.1", "1%Infinity"].iter().enumerate() {
        let mut module = checked(&format!(
            "export function main():number{{return {expression};}}"
        ));
        visit_module(&mut module, &mut Force).unwrap();
        let code = crate::emitter::emit(&module, crate::OUTPUT_FACTOR).unwrap();
        let path = directory.join(format!("case-{i}.luau"));
        std::fs::write(
            &path,
            format!("local function module(...)\n{code}\nend\nmodule(function() end).main()\n"),
        )
        .unwrap();
        let result = std::process::Command::new(root.join("target/luau-tools/luau"))
            .arg(path)
            .output()
            .unwrap();
        assert!(!result.status.success());
        assert!(
            String::from_utf8_lossy(&result.stderr).contains("native remainder shadow mismatch"),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
    }
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn unset_member_invalidates_the_whole_component() {
    let mut module =
        checked("export function main():number{let x=7;let y=1;let z=1;x=y;return x%3+7%x;}");
    struct Declarations(Vec<usize>);
    impl Visitor for Declarations {
        fn expression(&mut self, _: &mut Expr) -> Result<(), Diagnostic> {
            Ok(())
        }
        fn statement(&mut self, s: &mut S) -> Result<(), Diagnostic> {
            if let S::Declare(id, _) = s {
                self.0.push(*id);
            }
            Ok(())
        }
    }
    let mut declarations = Declarations(vec![]);
    visit_module(&mut module, &mut declarations).unwrap();
    assert_eq!(declarations.0.len(), 3);
    let [x, y, z] = declarations.0[..] else {
        unreachable!();
    };
    struct Initializers {
        x: usize,
        y: usize,
        z: usize,
    }
    impl Visitor for Initializers {
        fn expression(&mut self, _: &mut Expr) -> Result<(), Diagnostic> {
            Ok(())
        }
        fn statement(&mut self, s: &mut S) -> Result<(), Diagnostic> {
            if let S::Declare(id, e) = s {
                if *id == self.y {
                    e.kind = E::Read(self.z);
                } else if *id == self.z {
                    let read = |id| Expr {
                        flow: None,
                        bare_true: false,
                        start: e.start,
                        ty: e.ty,
                        kind: E::Read(id),
                    };
                    e.kind = E::Binary("+", Box::new(read(self.x)), Box::new(read(self.y)));
                }
            }
            Ok(())
        }
    }
    // Build x=7; y=z; z=x+y; x=y. The y/z writes stay unset and x=y never joins.
    visit_module(&mut module, &mut Initializers { x, y, z }).unwrap();
    let report = analyze(&mut module).unwrap();
    assert_eq!(report.rounds, 2);
    let code = crate::emitter::emit(&module, crate::OUTPUT_FACTOR).unwrap();
    assert_eq!(code.matches("__ulua_fmod(").count(), 2, "{code}");
    assert!(!code.contains(" % "), "{code}");
    assert!(report.sites.is_empty());
}

#[test]
fn missing_initializer_and_unset_cycles_have_no_binding_fact() {
    struct RemoveInitializer;
    impl Visitor for RemoveInitializer {
        fn expression(&mut self, _: &mut Expr) -> Result<(), Diagnostic> {
            Ok(())
        }
        fn statement(&mut self, s: &mut S) -> Result<(), Diagnostic> {
            if let S::Declare(id, e) = s {
                let id = *id;
                let replacement = Expr {
                    flow: None,
                    bare_true: false,
                    start: e.start,
                    ty: e.ty,
                    kind: E::Number(8.0),
                };
                let value = std::mem::replace(e, replacement);
                *s = S::Assign(id, None, value);
            }
            Ok(())
        }
    }
    let mut module = checked("export function main():number{let x=8;return x%3;}");
    visit_module(&mut module, &mut RemoveInitializer).unwrap();
    assert!(analyze(&mut module).unwrap().sites.is_empty());
    let mut module = checked("export function main():number{let x=8;return x%3;}");
    let mut id = 0;
    struct Cycle<'a>(&'a mut usize);
    impl Visitor for Cycle<'_> {
        fn expression(&mut self, _: &mut Expr) -> Result<(), Diagnostic> {
            Ok(())
        }
        fn statement(&mut self, s: &mut S) -> Result<(), Diagnostic> {
            if let S::Declare(id, e) = s {
                *self.0 = *id;
                e.kind = E::Read(*id);
            }
            Ok(())
        }
    }
    visit_module(&mut module, &mut Cycle(&mut id)).unwrap();
    let report = analyze(&mut module).unwrap();
    assert_eq!(report.rounds, 1);
    assert!(report.sites.is_empty());
}
forms! {
    literal_one_saturates_only_addition: "return (9007199254740992*1)%3;", 1, 0;
    integer_exponent_and_hex: "return (0x10+1e2)%3;", 1, 0;
    full_expression_slot_path: "return ((2*3)+(4*5))%7;", 1, 0;
    stable_compound_multiply: "let x=8;x*=0;return x%3;", 1, 0;
    stable_compound_subtract: "let x=8;x-=0;return x%3;", 1, 0;
    compound_division_invalidates: "let x=8;x/=2;return x%3;", 0, 1;
    field_compound_keeps_fmod: "let x={v:8};x.v%=3;return x.v;", 0, 1;
    element_compound_keeps_fmod: "let x=[8];x[0]%=3;return x[0];", 0, 1;
}

#[test]
fn native_remainder_reserves_the_fmod_checkpoint_weight() {
    for unit in ["x=x%3;", "x%=3;"] {
        let body = unit.repeat(128);
        let native = format!("export function main():number{{let x=1;{body}return x;}}");
        let fmod = format!("export function main(x:number):number{{{body}return x;}}");
        let native = compile(native.as_bytes(), Limits::default()).unwrap().luau;
        let fmod = compile(fmod.as_bytes(), Limits::default()).unwrap().luau;
        assert_eq!(native.matches(" % ").count(), 128);
        assert_eq!(fmod.matches("__ulua_fmod(").count(), 128);
        assert!(
            native.matches("__ulua_interrupt()").count()
                >= fmod.matches("__ulua_interrupt()").count(),
            "{unit}: native checkpoints={} fmod checkpoints={}",
            native.matches("__ulua_interrupt()").count(),
            fmod.matches("__ulua_interrupt()").count()
        );
    }
}

#[test]
fn unrelated_remainders_do_not_delay_counter_widening() {
    form(
        "let s=0;for(let i=0;i<100;i++){s=s+i%2+i%3+i%4+i%5+i%6+i%7+i%8;}return s;",
        7,
        0,
    );
}

#[test]
fn missing_expression_node_returns_internal() {
    let mut graph = Graph::default();
    let mut eligible = [];
    let mut build = Build {
        source: &[],
        graph: &mut graph,
        indices: HashMap::new(),
        starts: HashMap::new(),
        stack: vec![],
        owners: &[],
        eligible: &mut eligible,
    };
    let mut expression = Expr {
        flow: None,
        bare_true: false,
        start: 0,
        ty: crate::ir::Type::Number,
        kind: E::Number(1.0),
    };
    let child = Expr {
        kind: E::Number(2.0),
        ..expression
    };
    expression.kind = E::Binary(
        "%",
        Box::new(child),
        Box::new(Expr {
            kind: E::Number(3.0),
            ..expression
        }),
    );
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        build.expression(&mut expression)
    }));
    assert_eq!(result.unwrap().unwrap_err().code, "internal");
    assert_eq!(build.index(&expression).unwrap_err().code, "internal");
    assert_eq!(
        build.lookup(&build.starts, &expression).unwrap_err().code,
        "internal"
    );
}

#[test]
fn deep_ir_expression_returns_nesting_depth() {
    let mut module = checked("export function main():number{return 7%3;}");
    let expression = || Expr {
        flow: None,
        bare_true: false,
        start: 37,
        ty: crate::ir::Type::Number,
        kind: E::Number(1.0),
    };
    let mut value = expression();
    for _ in 0..module.depth {
        value = Expr {
            kind: E::Binary("+", Box::new(value), Box::new(expression())),
            ..expression()
        };
    }
    module.functions[0].body = vec![S::Return(Some(value))];
    let error = analyze(&mut module).err().unwrap();
    assert_eq!(error.code, "limit:nesting-depth");
    assert_eq!(error.offset, 37);
}

#[test]
fn deep_ir_statement_returns_nesting_depth() {
    let mut module = checked("export function main():number{return 7%3;}");
    let mut statement = S::Return(None);
    for _ in 0..=module.depth {
        statement = S::Block(vec![S::Located(37, Box::new(statement))]);
    }
    module.functions[0].body = vec![statement];
    let error = analyze(&mut module).err().unwrap();
    assert_eq!(error.code, "limit:nesting-depth");
    assert_eq!(error.offset, 37);
}

#[test]
fn removed_write_keys_return_internal() {
    let mut graph = Graph::default();
    let mut eligible = [false];
    let mut build = Build {
        source: &[],
        graph: &mut graph,
        indices: HashMap::new(),
        starts: HashMap::new(),
        stack: vec![],
        owners: &[1],
        eligible: &mut eligible,
    };
    let mut statement = S::Assign(
        0,
        None,
        Expr {
            flow: None,
            bare_true: false,
            start: 0,
            ty: crate::ir::Type::Number,
            kind: E::Number(1.0),
        },
    );
    let S::Assign(_, _, e) = &mut statement else {
        unreachable!()
    };
    build.enter_expression();
    build.expression(e).unwrap();
    let key = address(e);
    let start = build.starts.remove(&key).unwrap();
    assert_eq!(
        build.statement(&mut statement).unwrap_err().code,
        "internal"
    );
    build.starts.insert(key, start);
    build.indices.remove(&key);
    assert_eq!(
        build.statement(&mut statement).unwrap_err().code,
        "internal"
    );
}

#[test]
fn nested_for_of_storage_blocks_preserve_checked_depth() {
    form(
        &format!(
            "const a=[1];let x=0;{}x++;return x%3;",
            "for(const t of a)".repeat(38)
        ),
        1,
        0,
    );
}
