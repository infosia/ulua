//! Index consumers use the existing expression facts.
use crate::{Limits, compile};

fn form(body: &str, proven: bool, checks: usize) {
    let source = format!(
        "export function main(p:number):number{{let a=[1,2,3,4,5];let k=0;let h=2;let r={{i:0}};{body}}}"
    );
    let code = compile(source.as_bytes(), Limits::default()).unwrap().luau;
    let tests: Vec<_> = code
        .lines()
        .filter(|line| {
            (line.contains(" < #__ulua_") || line.contains(" <= #__ulua_"))
                && !line.contains("function")
        })
        .collect();
    assert_eq!(tests.len(), checks, "{source}\n{code}");
    for test in tests {
        assert_eq!(test.contains(">= 0"), !proven, "{source}\n{test}");
        assert_eq!(test.contains("__ulua_floor"), !proven, "{source}\n{test}");
    }
}

macro_rules! forms {
    ($($name:ident: $body:literal, $proven:literal, $checks:literal;)*) => {$ (
        #[test] fn $name() { form($body, $proven, $checks); }
    )*};
}
// Cost: 20 small compiles, 0.01 s in Debug; no VM or external process.
forms! {
    read: "return a[k];", true, 1;
    binding_read: "let value=a[k];return value;", true, 1;
    write: "a[k]=2;return 0;", true, 1;
    compound_write: "a[k]+=2;return 0;", true, 2;
    compound_string_write: "let s=['a'];s[k]+='b';return 0;", true, 2;
    literal: "return a[0];", true, 1;
    counter: "for(let j=0;j<4;j++){a[j]=j;}return 0;", true, 1;
    sum: "return a[k+1];", true, 1;
    remainder: "return a[k%3];", true, 1;
    product: "return a[2*2];", true, 1;
    parameter: "return a[p];", false, 1;
    division: "return a[h/2];", false, 1;
    field: "return a[r.i];", false, 1;
    parameter_write: "a[p]=2;return 0;", false, 1;
    parameter_compound_write: "a[p]+=2;return 0;", false, 2;
    negative_zero: "return a[-0];", false, 1;
    fractional: "return a[h-0.5];", false, 1;
    closure_counter: "const f=():void=>{k++;};f();return a[k];", true, 1;
    closure_invalidates: "const f=():void=>{k=-1;};f();return a[k];", false, 1;
    saturation: "let n=9007199254740992;n++;return a[n];", true, 1;
}

// Cost: 12 short CLI runs; both shadow rejection tests take 0.06 s in Debug.
#[cfg(feature = "remainder-shadow")]
#[test]
#[ignore = "Run with ULUA_REMAINDER_SHADOW=1 to test false index forms"]
fn shadow_rejects_false_index_forms() {
    use super::{Diagnostic, E, Expr, S, Visitor};
    struct Force;
    impl Visitor for Force {
        fn expression(&mut self, e: &mut Expr) -> Result<(), Diagnostic> {
            if let E::Index(_, i) = &mut e.kind {
                i.index_form = crate::ir::IndexForm::Proven;
            }
            Ok(())
        }
        fn statement(&mut self, s: &mut S) -> Result<(), Diagnostic> {
            if let S::Element(_, i, _, _) = s {
                i.index_form = crate::ir::IndexForm::Proven;
            }
            Ok(())
        }
    }
    assert!(std::env::var_os("ULUA_REMAINDER_SHADOW").is_some());
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let directory = std::env::temp_dir().join(format!("ulua-index-false-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    for index in ["-1", "0.5", "NaN", "-Infinity"] {
        for body in [
            format!("return a[{index}];"),
            format!("a[{index}]=2;return 0;"),
            format!("a[{index}]+=2;return 0;"),
        ] {
            let mut module = super::tests::checked(&format!(
                "export function main():number{{let a=[1];{body}}}"
            ));
            super::visit_module(&mut module, &mut Force).unwrap();
            let code = crate::emitter::emit(&module, crate::OUTPUT_FACTOR).unwrap();
            let path = directory.join("case.luau");
            std::fs::write(
                &path,
                format!("local function module(...)\n{code}\nend\nmodule(function() error(\"index trap\") end).main()\n"),
            )
            .unwrap();
            let result = std::process::Command::new(root.join("target/luau-tools/luau"))
                .arg(path)
                .output()
                .unwrap();
            assert!(!result.status.success(), "{body}");
            assert!(
                String::from_utf8_lossy(&result.stderr).contains("proven index shadow mismatch"),
                "{body}: {}",
                String::from_utf8_lossy(&result.stderr)
            );
        }
    }
    std::fs::remove_dir_all(directory).unwrap();
}

// A compound write has one IR site and emits both a read test and a write test.
pub(super) fn index_counts(module: &mut super::Module) -> (usize, usize) {
    use super::{Diagnostic, E, Expr, S, Visitor};
    use crate::ir::IndexForm::Proven;
    #[derive(Default)]
    struct Counts {
        sites: usize,
        compound: usize,
    }
    impl Visitor for Counts {
        fn expression(&mut self, e: &mut Expr) -> Result<(), Diagnostic> {
            if let E::Index(_, i) = &e.kind {
                self.sites += usize::from(i.index_form == Proven);
            }
            Ok(())
        }
        fn statement(&mut self, s: &mut S) -> Result<(), Diagnostic> {
            if let S::Element(_, i, op, _) = s
                && i.index_form == Proven
            {
                self.sites += 1;
                self.compound += usize::from(op.is_some());
            }
            Ok(())
        }
    }
    let mut counts = Counts::default();
    super::visit_module(module, &mut counts).unwrap();
    (counts.sites, counts.compound)
}

fn context_form(source: &str, proven: usize, checked: usize, tests: usize) -> String {
    use super::{Diagnostic, E, Expr, Visitor};
    use crate::ir::IndexForm;
    struct Forms(Vec<IndexForm>);
    impl Visitor for Forms {
        fn expression(&mut self, e: &mut Expr) -> Result<(), Diagnostic> {
            if let E::Index(_, i) = &e.kind {
                self.0.push(i.index_form);
            }
            Ok(())
        }
    }
    let mut module = super::tests::checked(source);
    let report = super::analyze(&mut module).unwrap();
    let mut forms = Forms(vec![]);
    super::visit_module(&mut module, &mut forms).unwrap();
    assert_eq!(
        forms.0.iter().filter(|f| **f == IndexForm::Proven).count(),
        proven
    );
    assert_eq!(
        forms.0.iter().filter(|f| **f == IndexForm::Checked).count(),
        checked
    );
    assert_eq!(report.index_sites.len(), index_counts(&mut module).0);
    let code = crate::emitter::emit(&module, crate::OUTPUT_FACTOR).unwrap();
    let emitted = code
        .lines()
        .filter(|line| {
            (line.contains(" < #__ulua_") || line.contains(" <= #__ulua_"))
                && !line.contains("__ulua_floor")
                && (line.contains("then __ulua_trap(1) end") || line.contains(": boolean = not ("))
        })
        .count();
    assert_eq!(emitted, tests, "{code}");
    code
}

fn nested_form(source: &str, proven: usize, checked: usize, tests: usize) {
    let code = context_form(source, proven, checked, tests);
    assert!(
        code.lines()
            .any(|line| line.contains("__ulua_get(") && line.contains(", true)")),
        "{code}"
    );
}

// Cost: ten small compiles; the IR forms and the emitted tests are checked separately.
#[test]
fn module_body_index_reads_use_expression_facts() {
    context_form(
        "const a=[1];const n=a[1-1];export function main():number{return n;}",
        1,
        0,
        1,
    );
    context_form(
        "const a=[1];const k=0;const n=a[k+1];export function main():number{return n;}",
        0,
        1,
        0,
    );
}
#[test]
fn closure_body_index_read_uses_outer_fact() {
    context_form(
        "export function main():number{const a=[1];let k=0;const f=():number=>a[k];return f();}",
        1,
        0,
        1,
    );
}
#[test]
fn parameter_default_index_uses_literal_expression_fact() {
    context_form(
        "const a=[1];export function main(p:number=a[1-1]):number{return p;}",
        1,
        0,
        1,
    );
    context_form(
        "const a=[1];const k=0;export function main(p:number=a[k]):number{return p;}",
        0,
        1,
        0,
    );
}
#[test]
fn nested_field_receiver_stays_checked() {
    nested_form(
        "export function main():number{const a=[{v:1}];return a[0].v;}",
        0,
        1,
        0,
    );
}
#[test]
fn nested_element_receiver_stays_checked() {
    nested_form(
        "export function main():number{const a=[[1]];return a[0][0];}",
        1,
        1,
        1,
    );
}
#[test]
fn nested_write_receivers_stay_checked() {
    nested_form(
        "export function main():number{const a=[{v:1}];a[0].v=2;return 0;}",
        0,
        1,
        0,
    );
    nested_form(
        "export function main():number{const a=[[1]];a[0][0]+=2;return 0;}",
        0,
        1,
        2,
    );
}

#[test]
fn nested_receiver_index_operand_keeps_its_own_fact() {
    nested_form(
        "export function main():number{const a=[{v:1}];const b=[0];let k=0;return a[b[k]].v;}",
        1,
        1,
        1,
    );
}
