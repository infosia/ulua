//! Counter bounds use the shared counted-loop eligibility test.
use super::tests::form;

#[test]
fn strict_literal_bound_refines_counter() {
    form("let r=0;for(let i=0;i<10;i++){r=(i+2)%3;}return r;", 1, 0);
}
#[test]
fn inclusive_literal_bound_refines_counter() {
    form("let r=0;for(let i=0;i<=10;i++){r=(i+2)%3;}return r;", 1, 0);
}
#[test]
fn binding_bound_reads_an_earlier_fact_component() {
    form(
        "const b=10;let r=0;for(let i=0;i<b;i++){r=(i+2)%3;}return r;",
        1,
        0,
    );
}
#[test]
fn unknown_bound_keeps_the_write_join() {
    form(
        "const b=10/2;let r=0;for(let i=0;i<b;i++){r=(i+2)%3;}return r;",
        0,
        1,
    );
}
#[test]
fn larger_step_refines_counter() {
    form("let r=0;for(let i=0;i<10;i+=3){r=(i+2)%3;}return r;", 1, 0);
}
#[test]
fn oversized_endpoint_keeps_the_write_join() {
    form(
        "let r=0;for(let i=0;i<=9007199254740992;i++){r=(i+2)%3;}return r;",
        0,
        1,
    );
}
#[test]
fn body_write_excludes_refinement() {
    form(
        "let r=0;for(let i=0;i<10;i++){i=3;r=(i+2)%3;}return r;",
        0,
        1,
    );
}
#[test]
fn closure_capture_excludes_refinement() {
    form(
        "let r=0;for(let i=0;i<10;i++){const f=():number=>i;r=(i+2)%3;}return r;",
        0,
        1,
    );
}
#[test]
fn continue_excludes_refinement() {
    form(
        "let r=0;for(let i=0;i<10;i++){r=(i+2)%3;continue;}return r;",
        0,
        1,
    );
}
#[test]
fn callbacks_counters_have_no_capture_after_inline_decisions() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let source =
        std::fs::read_to_string(root.join("crates/ulua_bench/workloads/ulua/callbacks.ts"))
            .unwrap();
    let mut module = super::tests::checked(&source);
    struct Inspect<'a>(&'a [bool], usize);
    impl super::Visitor for Inspect<'_> {
        fn expression(&mut self, _: &mut crate::ir::Expr) -> Result<(), crate::Diagnostic> {
            Ok(())
        }
        fn statement(&mut self, s: &mut crate::ir::Statement) -> Result<(), crate::Diagnostic> {
            if let crate::ir::Statement::For(init, _, _, _, _) = s
                && let [init] = init.as_slice()
                && let crate::ir::Statement::Declare(id, _) = init.form()
            {
                eprintln!("workload counter id={id} captured={}", self.0[*id]);
                self.1 += usize::from(self.0[*id]);
            }
            Ok(())
        }
    }
    let captured = module.captured.clone();
    let mut inspect = Inspect(&captured, 0);
    super::visit_module(&mut module, &mut inspect).unwrap();
    assert_eq!(inspect.1, 0);
    let code = crate::compile(source.as_bytes(), Default::default())
        .unwrap()
        .luau;
    assert!(!code.contains("while true do"), "{code}");
    assert_eq!(
        code.lines()
            .filter(|l| l.trim_start().starts_with("for __ulua_g"))
            .count(),
        5,
        "{code}"
    );
    assert_eq!(code.matches(" % ").count(), 5, "{code}");
    assert_eq!(code.matches("__ulua_fmod(").count(), 0, "{code}");
}

#[test]
fn inline_counter_has_no_iteration_copy_but_a_real_closure_keeps_it() {
    for real in [false, true] {
        let closure = if real { "const read=():number=>i;" } else { "" };
        let source = format!(
            "export function main():number{{const a=[8];for(let i=0;i<3;i++){{{closure}a.forEach((v:number):number=>(v+i+2)%3);}}return 0;}}"
        );
        let code = crate::compile(source.as_bytes(), Default::default())
            .unwrap()
            .luau;
        assert_eq!(code.contains("while true do"), real, "{code}");
        let numeric = code
            .lines()
            .filter(|l| l.trim_start().starts_with("for __ulua_g"))
            .count();
        assert_eq!(numeric, if real { 1 } else { 2 }, "{code}");
        let copies = code
            .lines()
            .filter(|l| {
                l.trim_start().starts_with("local __ulua_g1_") && l.contains(": number = (__ulua_b")
            })
            .count();
        assert_eq!(copies, usize::from(real), "{code}");
        assert_eq!(code.matches(" % ").count(), usize::from(!real), "{code}");
        assert_eq!(
            code.matches("__ulua_fmod(").count(),
            usize::from(real),
            "{code}"
        );
    }
}

pub(super) fn spilled_source() -> String {
    let declarations = (0..200).map(|i| format!("let v{i}=3;")).collect::<String>();
    format!(
        "export function main():number{{{declarations}let r=0;for(let i=0;i<v199;i++){{r=(i+2)%3;}}return r;}}"
    )
}
#[test]
fn spilled_bound_keeps_while_and_counter_fact() {
    let code = crate::compile(spilled_source().as_bytes(), Default::default())
        .unwrap()
        .luau;
    assert!(code.contains("while true do"), "{code}");
    assert!(
        !code
            .lines()
            .any(|l| l.trim_start().starts_with("for __ulua_g")),
        "{code}"
    );
    assert_eq!(code.matches(" % ").count(), 1, "{code}");
    assert_eq!(code.matches("__ulua_fmod(").count(), 0, "{code}");
}
#[test]
fn bound_in_the_counter_component_uses_component_rounds() {
    let body = "let b=3;let a=0;let r=0;for(let i=0;i<b;i++){a=i;r=(i+2)%3;}b=a;return r;";
    let mut module = super::tests::checked(&format!("export function main():number{{{body}}}"));
    // The bound edge closes b -> a -> i -> b. Dependency-separated joins finish in three rounds.
    assert_eq!(super::analyze(&mut module).unwrap().rounds, 5);
    form(body, 0, 1);
}
#[test]
fn empty_and_unproved_counter_ranges_keep_the_write_join() {
    form("let r=0;for(let i=10;i<0;i++){r=(i+2)%3;}return r;", 0, 1);
    form("let r=0;for(let i=-1;i<10;i++){r=i%3;}return r;", 0, 1);
    form("let r=0;for(let i=-0;i<10;i++){r=i%3;}return r;", 0, 1);
    form("let r=0;for(let i=0;i<0;i++){r=(i+2)%3;}return r;", 1, 0);
    form(
        "let r=0;for(let i=0;i<9007199254740992;i++){r=i%3;}return r;",
        1,
        0,
    );
}
#[test]
fn capture_summary_uses_only_emitted_function_owners() {
    let mut body = String::from("const a=[8];");
    for i in 0..12 {
        body += &format!("let x{i}={i};a.forEach((v:number):number=>x{i}+v);");
    }
    let source = format!("export function main():void{{{body}}}");
    let module = super::tests::checked(&source);
    let owner = module
        .functions
        .iter()
        .find(|f| f.exported)
        .unwrap()
        .binding
        + 1;
    for (id, b) in module
        .bindings
        .iter()
        .enumerate()
        .filter(|(_, b)| b.owner == owner && b.mutable)
    {
        assert!(!module.captured[id], "offset={}", b.start);
        assert_eq!(module.capture_counts[id], 0);
    }
    // The inline body's outer read belongs to its enclosing emitted closure.
    let module = super::tests::checked(
        "export function main():number{const a=[8];let x=1;const f=():void=>{a.forEach((v:number):number=>x+v);};const g=():number=>x;return g();}",
    );
    let x = module
        .bindings
        .iter()
        .position(|b| b.mutable && b.ty == Some(crate::ir::Type::Number))
        .unwrap();
    assert!(module.captured[x]);
    assert_eq!(module.capture_counts[x], 2);
}

#[test]
fn body_write_to_bound_excludes_refinement() {
    form(
        "let b=10;let r=0;for(let i=0;i<b;i++){b=5;r=(i+2)%3;}return r;",
        0,
        1,
    );
}

#[test]
fn closure_capture_of_bound_excludes_refinement() {
    form(
        "const b=10;let r=0;for(let i=0;i<b;i++){const read=():number=>b;r=(i+2)%3;}return r;",
        0,
        1,
    );
}
