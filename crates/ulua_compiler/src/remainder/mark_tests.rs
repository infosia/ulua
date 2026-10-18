//! Integral rules exercise emitted guards independently of the interval facts.
use super::{Diagnostic, E, Expr, Module, S, Visitor};
use crate::ir::IndexForm;

fn index_forms(module: &mut Module) -> Vec<IndexForm> {
    struct Forms(Vec<IndexForm>);
    impl Visitor for Forms {
        fn expression(&mut self, e: &mut Expr) -> Result<(), Diagnostic> {
            if let E::Index(_, i) = &e.kind {
                self.0.push(i.index_form);
            }
            Ok(())
        }
        fn statement(&mut self, s: &mut S) -> Result<(), Diagnostic> {
            if let S::Element(_, i, _, _) = s {
                self.0.push(i.index_form);
            }
            Ok(())
        }
    }
    let mut forms = Forms(vec![]);
    super::visit_module(module, &mut forms).unwrap();
    forms.0
}

fn check(source: &str, integral: bool) {
    let mut module = super::tests::checked(source);
    super::analyze(&mut module).unwrap();
    let expected = if integral {
        IndexForm::Integral
    } else {
        IndexForm::Checked
    };
    assert!(
        index_forms(&mut module).iter().all(|f| *f == expected),
        "{source}"
    );
    let code = crate::emitter::emit(&module, crate::OUTPUT_FACTOR).unwrap();
    let tests: Vec<_> = code
        .lines()
        .filter(|line| {
            (line.contains(" < #__ulua_") || line.contains(" <= #__ulua_"))
                && (line.contains("then __ulua_trap(1) end") || line.contains(": boolean = not ("))
        })
        .collect();
    assert!(!tests.is_empty(), "{code}");
    for test in tests {
        assert!(test.contains(">= 0"), "{test}");
        assert_eq!(
            test.contains("__ulua_floor("),
            !integral,
            "{source}\n{test}"
        );
    }
}
fn form(body: &str, integral: bool) {
    check(
        &format!("export function main(p:number):number{{const a=[1,2];{body}}}"),
        integral,
    );
}
macro_rules! forms {
    ($($name:ident: $body:literal, $integral:literal;)*) => {$(
        #[test] fn $name() { form($body, $integral); }
    )*};
}
// Cost: small compiler-only cases; no VM or external process.
forms! {
    integer_double: "return a[9007199254740994];", true;
    infinity: "return a[Infinity];", true;
    nan: "return a[NaN];", true;
    negative_zero: "return a[-0];", true;
    binding: "let j=-1;return a[j];", true;
    addition: "return a[-1+2];", true;
    subtraction: "return a[0-1];", true;
    multiplication: "return a[-1*2];", true;
    unary_minus: "return a[-a.length];", true;
    unary_minus_unmarked: "return a[-p];", false;
    remainder: "return a[-1%2];", true;
    floor: "return a[Math.floor(p)];", true;
    ceil: "return a[Math.ceil(p)];", true;
    trunc: "return a[Math.trunc(p)];", true;
    round: "return a[Math.round(p)];", true;
    abs: "return a[Math.abs(-1)];", true;
    array_length: "return a[a.length];", true;
    string_length: "return a['hi'.length];", true;
    map_size: "const m=new Map<number,number>();return a[m.size];", true;
    set_size: "const m=new Set<number>();return a[m.size];", true;
    utf8_byte_length: "return a[Utf8.byteLength('hé')];", true;
    parentheses: "return a[((-1))];", true;
    cycle: "let j=-1;let k=j;j=k;return a[k];", true;
    update: "let j=0;j--;return a[j];", true;
    compound_write: "let j=-1;a[j]+=2;return 0;", true;
    unmarked_write: "let j=-1;j=p;return a[j];", false;
    closure_write: "let j=-1;const f=():void=>{j=p;};f();return a[j];", false;
    division: "return a[2/1];", false;
    field: "const r={v:1};return a[r.v];", false;
    element: "const b=[p];return a[b[p]-1];", false;
    call_result: "const f=():number=>1;return a[f()];", false;
    for_of: "for(const j of a){return a[j];}return 0;", false;
    abs_unmarked: "return a[Math.abs(p)];", false;
    conditional: "return a[p>0?1:2];", false;
    fractional: "return a[0.5];", false;
}
#[test]
fn binding_mark_from_element_fact() {
    let mut module = super::tests::checked(
        "export function main():number{const a=[1,2];const b:number[]=[1];let j=b[0];return a[j-2];}",
    );
    super::analyze(&mut module).unwrap();
    // The element has a fact, while j-2 has only a mark because its range is negative.
    assert_eq!(
        index_forms(&mut module),
        [IndexForm::Proven, IndexForm::Integral]
    );
    let code = crate::emitter::emit(&module, crate::OUTPUT_FACTOR).unwrap();
    let test = code
        .lines()
        .rfind(|line| line.contains(" < #__ulua_") && line.contains("then __ulua_trap(1) end"))
        .unwrap();
    assert!(test.contains(">= 0"), "{test}");
    assert!(!test.contains("__ulua_floor("), "{test}");
}
#[test]
fn parameter() {
    check(
        "function f(j:number):number{const a=[1];return a[j];}export function main():number{return f(-1);}",
        true,
    );
}

#[test]
fn excluded_parameters() {
    for (prefix, parameter, calls) in [
        ("export ", "j:number", "return f(-1);"),
        ("", "j:number=0", "return f(-1);"),
        ("", "j:number", "const g=f;return g(-1);"),
        ("", "j:number", "return 0;"),
    ] {
        check(
            &format!(
                "{prefix}function f({parameter}):number{{const a=[1];j=-1;return a[j];}}export function main():number{{{calls}}}"
            ),
            false,
        );
    }
    check(
        "async function f(j:number):Promise<number>{const a=[1];return a[j];}export async function main():Promise<number>{return await f(-1);}",
        false,
    );
    check(
        "export function main():number{const f=(j:number):number=>{const a=[1];return a[j];};return f(-1);}",
        false,
    );
}

#[test]
fn parameter_writes_and_recursive_cycle() {
    check(
        "function f(j:number):number{const a=[1];if(j>0){f(j-1);}j--;return a[j];}export function main():number{return f(-1);}",
        true,
    );
    check(
        "function f(j:number):number{const a=[1];return a[j];}export function main(p:number):number{f(-1);return f(p);}",
        false,
    );
    check(
        "function f(j:number):number{const a=[1];j/=2;return a[j];}export function main():number{return f(-1);}",
        false,
    );
}

#[test]
fn long_cycle_and_removal_chain() {
    for unmarked in [false, true] {
        let mut body = "let x0=-1;".to_owned();
        for i in 1..64 {
            body.push_str(&format!("let x{i}=x{};", i - 1));
        }
        body.push_str("x0=x63;");
        if unmarked {
            body.push_str("x0=p;");
        }
        body.push_str("return a[x63];");
        form(&body, !unmarked);
    }
}

#[test]
fn seedless_cycle_keeps_mark_but_missing_initializer_does_not() {
    struct Rewrite(bool);
    impl Visitor for Rewrite {
        fn expression(&mut self, _: &mut Expr) -> Result<(), Diagnostic> {
            Ok(())
        }
        fn statement(&mut self, s: &mut S) -> Result<(), Diagnostic> {
            if let S::Declare(id, e) = s
                && e.ty == crate::ir::Type::Number
            {
                if self.0 {
                    e.kind = E::Read(*id);
                } else {
                    let id = *id;
                    let value = std::mem::replace(
                        e,
                        Expr {
                            flow: None,
                            bare_true: false,
                            index_form: IndexForm::Checked,
                            start: 0,
                            ty: crate::ir::Type::Number,
                            kind: E::Number(0.0),
                        },
                    );
                    *s = S::Assign(id, None, value);
                }
            }
            Ok(())
        }
    }
    // These IR fixtures isolate eligibility and the greatest fixed point from source initialization checks.
    for seedless in [true, false] {
        let mut module = super::tests::checked(
            "export function main():number{const a=[1];let j=0;return a[j];}",
        );
        super::visit_module(&mut module, &mut Rewrite(seedless)).unwrap();
        assert!(super::analyze(&mut module).unwrap().index_sites.is_empty());
        assert_eq!(
            index_forms(&mut module),
            [if seedless {
                IndexForm::Integral
            } else {
                IndexForm::Checked
            }]
        );
    }
}

#[test]
fn mark_growth_shape_contains_surviving_and_removed_marks() {
    for bytes in [4096, 16384, 65536] {
        let mut module =
            super::tests::checked(&super::tests::growth::source("integral-marks", bytes));
        super::analyze(&mut module).unwrap();
        let forms = index_forms(&mut module);
        let integral = forms.iter().filter(|f| **f == IndexForm::Integral).count();
        let checked = forms.iter().filter(|f| **f == IndexForm::Checked).count();
        assert!(integral >= bytes / 150);
        assert_eq!(integral, checked);
    }
}

// Forced forms verify the omitted-test guard, including NaN's exemption.
#[cfg(feature = "remainder-shadow")]
#[test]
#[ignore = "Run with ULUA_REMAINDER_SHADOW=1 to test false integral forms"]
fn shadow_rejects_false_integral_forms() {
    struct Force;
    impl Visitor for Force {
        fn expression(&mut self, e: &mut Expr) -> Result<(), Diagnostic> {
            if let E::Index(_, i) = &mut e.kind {
                i.index_form = IndexForm::Integral;
            }
            Ok(())
        }
        fn statement(&mut self, s: &mut S) -> Result<(), Diagnostic> {
            if let S::Element(_, i, _, _) = s {
                i.index_form = IndexForm::Integral;
            }
            Ok(())
        }
    }
    assert!(std::env::var_os("ULUA_REMAINDER_SHADOW").is_some());
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let directory =
        std::env::temp_dir().join(format!("ulua-integral-false-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    for index in ["0.5", "-0.5", "NaN", "Infinity", "-Infinity", "-0"] {
        for body in [
            format!("return a[{index}];"),
            format!("a[{index}]=2;return 0;"),
            format!("a[{index}]+=2;return 0;"),
        ] {
            let mut module = super::tests::checked(&format!(
                "export function main():number{{const a=[1];{body}}}"
            ));
            super::visit_module(&mut module, &mut Force).unwrap();
            let code = crate::emitter::emit(&module, crate::OUTPUT_FACTOR).unwrap();
            let path = directory.join("case.luau");
            std::fs::write(&path, format!("local function module(...)\n{code}\nend\nmodule(function() error(\"index trap\") end).main()\n")).unwrap();
            let result = std::process::Command::new(root.join("target/luau-tools/luau"))
                .arg(path)
                .output()
                .unwrap();
            let error = String::from_utf8_lossy(&result.stderr);
            let fraction = index.contains("0.5");
            assert_eq!(
                error.contains("integral index shadow mismatch"),
                fraction,
                "{body}: {error}"
            );
            assert_eq!(result.status.success(), index == "-0", "{body}: {error}");
            if !fraction && index != "-0" {
                assert!(error.contains("index trap"), "{error}");
            }
        }
    }
    std::fs::remove_dir_all(directory).unwrap();
}
