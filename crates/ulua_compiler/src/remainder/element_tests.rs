//! Element and callback facts require confinement and integer writes.
use super::tests::form;
#[test]
fn allowed_element_uses() {
    for usage in [
        "",
        "const n=a.length;",
        "a.pop();",
        "a.push(4);",
        "a[0]=4;",
        "a[0]%=3;",
        "for(const v of a){}",
        "a.forEach((v:number):number=>v%3);",
        "const b=a.map((v:number):number=>v%3);",
        "const b=a.filter((v:number):boolean=>v%3===0);",
        "const b=a.reduce((s:number,v:number):number=>(s+v)%3,0);",
    ] {
        let count = usage.matches('%').count();
        form(&format!("const a=[8];{usage}return a[0]%3;"), count + 1, 0);
    }
}
#[test]
fn escapes_remove_element_facts() {
    for usage in [
        "const f=(x:number[]):void=>{};f(a);",
        "const b=a;",
        "const b={v:a};",
        "const f=():number[]=>a;",
        "const b=a===a;",
        "a.join(',');",
        "a.forEach((v:number):void=>{});",
        "const f=(v:number):number=>v; a.map(f);",
    ] {
        form(&format!("const a=[8];{usage}return a[0]%3;"), 0, 1);
    }
}
#[test]
fn element_writes() {
    for write in [
        "a.push(2.5);",
        "a[0]=-0;",
        "a[0]/=2;",
        "const f=():void=>{a.push(-1);};f();",
        "a=[1.5];",
    ] {
        form(&format!("let a=[8];{write}return a[0]%3;"), 0, 1);
    }
    form(
        "let a=[8];a=[4];a.push(3);a[0]=5;a[0]%=3;return a[0]%2;",
        2,
        0,
    );
    form(
        "const a=[8];const b=a.map((v:number):number=>v%3);const c=b.filter((v:number):boolean=>v%2===0);return c[0]%2;",
        3,
        0,
    );
    form(
        "const a=[8];const b=a.map((v:number):number=>v/2);return b[0]%2;",
        0,
        1,
    );
    form(
        "const a=[1.5];const b=a.filter((v:number):boolean=>true);return b[0]%2;",
        0,
        1,
    );
}
#[test]
fn callback_and_reduce_facts() {
    form(
        "const a=[8];return a.reduce((s:number,v:number,i:number):number=>(s+v+i)%100,0)%3;",
        2,
        0,
    );
    form(
        "const a=[8];return a.reduce((s:number,v:number):number=>s%3+v,0.5)%3;",
        0,
        2,
    );
    form(
        "const a=[8];return a.reduce((s:number,v:number):number=>(s-v)%3,0)%3;",
        0,
        2,
    );
    form(
        "const a=[1.5];a.forEach((v:number,i:number):number=>i%3+v%3);return 0;",
        1,
        1,
    );
    form(
        "const a=[8];let r=0;for(const v of a){r=v%3;}return r;",
        0,
        1,
    );
}
#[test]
fn parameter_array_has_no_element_fact() {
    let source = "function f(a:number[]):number{return a[0]%3+a.reduce((s:number,v:number,i:number):number=>s%3+v%3+i%3,0);}export function main():number{return f([8]);}";
    let code = crate::compile(source.as_bytes(), Default::default())
        .unwrap()
        .luau;
    assert_eq!(code.matches(" % ").count(), 1, "{code}");
    assert_eq!(code.matches("__ulua_fmod(").count(), 3, "{code}");
}

#[test]
fn element_components_reach_round_limit() {
    for bytes in [4096, 16384, 65536] {
        let mut module =
            super::tests::checked(&super::tests::growth::source("element-fact-rounds", bytes));
        let report = super::analyze(&mut module).unwrap();
        assert_eq!(report.rounds, super::FACT_ROUNDS);
        assert_eq!(report.components_at_limit, report.sites.len());
        assert!(report.components_at_limit >= bytes / 160);
    }
}

#[test]
fn contract_arithmetic_limits_keep_fmod() {
    form(
        "const a:number[]=[];let s=7;for(let i=0;i<50;i++){s=(s*1103515245+12345)%2147483648;a.push(s);}return a[0]%3;",
        0,
        2,
    );
    form(
        "const a=[8];for(let k=0;k<20;k++){a.forEach((v:number,i:number):number=>(v+i+k)%4294967296);}return 0;",
        1,
        0,
    );
    form(
        "const a=[9007199254740991,9007199254740992];a.forEach((v:number,i:number):number=>(v+i)%1000);return a[0]%7;",
        1,
        1,
    );
    form("const a=[Infinity];return a[0]%5;", 0, 1);
}
#[test]
fn confinement_covers_closures_and_binding_writes() {
    form("const a=[8];const f=():number=>a[0]%3;return f();", 1, 0);
    form(
        "const a=[8];const f=():void=>{const alias=a;};return a[0]%3;",
        0,
        1,
    );
    form("const a=[8];const o={a:[1]};o.a=a;return a[0]%3;", 0, 1);
    form("let a=[8];const b=[4];a=b;return a[0]%3+b[0]%3;", 0, 2);
    form("const a=[8];const holder=[a];return a[0]%3;", 0, 1);
    form(
        "let a=[8];a=a.map((v:number):number=>v%3);return a[0]%2;",
        2,
        0,
    );
    form(
        "let a=[8];a=a.filter((v:number):boolean=>v%3===0);return a[0]%2;",
        2,
        0,
    );
    form(
        "const a=[8];const b=a.map((v:number):number=>v%3);b.push(-1);return b[0]%2;",
        1,
        1,
    );
    form(
        "const a=[8];return a.reduce((s:number,v:number):number=>4,0)%3;",
        1,
        0,
    );
}
#[test]
fn failed_element_component_preserves_independent_index() {
    let mut body = String::from("const a=[1];");
    for (next, prev) in [
        ("b", "a"),
        ("c", "b"),
        ("d", "c"),
        ("e", "d"),
        ("f", "e"),
        ("g", "f"),
        ("h", "g"),
    ] {
        body += &format!("const {next}=[{prev}[0]];");
    }
    body += "a[0]=h[0];a.forEach((v:number,i:number):number=>v%3+i%3);return h[0]%3;";
    form(&body, 1, 2);
}

#[test]
fn module_inline_reduce_includes_the_body_write() {
    for (body, native, fmod) in [("s%3", 2, 0), ("-1", 0, 1)] {
        let source = format!(
            "const a=[1];const r=a.reduce((s:number,v:number):number=>{body},0)%3;export function main():number{{return r;}}"
        );
        let code = crate::compile(source.as_bytes(), Default::default())
            .unwrap()
            .luau;
        assert_eq!(code.matches(" % ").count(), native, "{code}");
        assert_eq!(code.matches("__ulua_fmod(").count(), fmod, "{code}");
    }
}

#[test]
fn unset_element_member_invalidates_its_component() {
    use super::*;
    let mut module = super::tests::checked(
        "export function main():number{const a=[7];const b=[1];const c=[1];a[0]=b[0];return a[0]%3+7%a[0];}",
    );
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
    super::visit_module(&mut module, &mut declarations).unwrap();
    let ids = declarations.0;
    let [a, b, c] = ids[..] else {
        panic!("Expected three arrays")
    };
    struct Replace {
        a: usize,
        b: usize,
        c: usize,
    }
    impl Visitor for Replace {
        fn expression(&mut self, _: &mut Expr) -> Result<(), Diagnostic> {
            Ok(())
        }
        fn statement(&mut self, s: &mut S) -> Result<(), Diagnostic> {
            if let S::Declare(id, e) = s {
                let E::Array(entries) = &mut e.kind else {
                    return Ok(());
                };
                let item = &mut entries[0];
                let element = |id| Expr {
                    kind: E::Index(
                        Box::new(Expr {
                            kind: E::Read(id),
                            ty: e.ty,
                            flow: None,
                            bare_true: false,
                            index_form: crate::ir::IndexForm::Checked,
                            start: 0,
                        }),
                        Box::new(Expr {
                            kind: E::Number(0.0),
                            ty: crate::ir::Type::Number,
                            flow: None,
                            bare_true: false,
                            index_form: crate::ir::IndexForm::Checked,
                            start: 0,
                        }),
                    ),
                    ty: crate::ir::Type::Number,
                    flow: None,
                    bare_true: false,
                    index_form: crate::ir::IndexForm::Checked,
                    start: 0,
                };
                if *id == self.b {
                    *item = element(self.c);
                }
                if *id == self.c {
                    item.kind =
                        E::Binary("+", Box::new(element(self.a)), Box::new(element(self.b)));
                }
            }
            Ok(())
        }
    }
    // The b/c writes stay unset. The write a[0]=b[0] never joins.
    super::visit_module(&mut module, &mut Replace { a, b, c }).unwrap();
    assert!(super::analyze(&mut module).unwrap().sites.is_empty());
    let code = crate::emitter::emit(&module, crate::OUTPUT_FACTOR).unwrap();
    assert_eq!(code.matches("__ulua_fmod(").count(), 2, "{code}");
}

#[test]
fn callback_index_uses_the_pinned_table_bound() {
    form(
        "const a=[1.5];a.forEach((v:number,i:number):number=>(i+9007199120523265)%3);return 0;",
        1,
        0,
    );
    form(
        "const a=[1.5];a.forEach((v:number,i:number):number=>(i+9007199120523266)%3);return 0;",
        0,
        1,
    );
    form(
        "const a=[8];a.forEach((v:number,i:number):number=>7%i);return 0;",
        0,
        1,
    );
    form("const a=[8];a[0]*=0;return a[0]%3;", 1, 0);
}

#[test]
fn extra_inline_parameters_have_no_fact_in_checked_ir() {
    use super::*;
    use crate::ir::{Binding, Type};
    for method in ["forEach", "map", "filter", "reduce"] {
        let (parameters, body, init, native, fmod) = match method {
            "reduce" => ("s:number,v:number,i:number", "s%3+v%3+i%3+v%3", ",0", 2, 2),
            "filter" => ("v:number,i:number", "v%3+i%3+v%3!==0", "", 2, 1),
            _ => ("v:number,i:number", "v%3+i%3+v%3", "", 2, 1),
        };
        let mut module = super::tests::checked(&format!(
            "export function main():number{{const a=[8];a.{method}(({parameters})=>{body}{init});return 0;}}"
        ));
        let function = module
            .functions
            .iter_mut()
            .find(|f| f.inline_callback.is_some())
            .unwrap();
        let original = function.parameters[usize::from(method == "reduce")].0;
        let extra = module.bindings.len();
        function.parameters.push((extra, None));
        module.bindings.push(Binding {
            ty: Some(Type::Number),
            owner: function.binding + 1,
            start: 0,
            end: 0,
            mutable: true,
            declaration: false,
        });
        module.captured.push(false);
        module.capture_counts.push(0);
        struct Replace {
            original: usize,
            extra: usize,
            reads: usize,
        }
        impl Visitor for Replace {
            fn expression(&mut self, e: &mut Expr) -> Result<(), Diagnostic> {
                if let E::Read(id) = &mut e.kind
                    && *id == self.original
                {
                    self.reads += 1;
                    if self.reads == 2 {
                        *id = self.extra;
                    }
                }
                Ok(())
            }
        }
        // C6 rejects the extra source parameter. The IR appends it after the index parameter.
        let mut replace = Replace {
            original,
            extra,
            reads: 0,
        };
        visit_module(&mut module, &mut replace).unwrap();
        assert_eq!(replace.reads, 2, "{method}");
        analyze(&mut module).unwrap();
        let code = crate::emitter::emit(&module, crate::OUTPUT_FACTOR).unwrap();
        assert_eq!(code.matches(" % ").count(), native, "{method}\n{code}");
        assert_eq!(
            code.matches("__ulua_fmod(").count(),
            fmod,
            "{method}\n{code}"
        );
    }
}

#[test]
fn filter_requires_a_confined_source_for_destination_elements() {
    for (escape, native, fmod) in [("", 4, 0), ("const alias=a;", 1, 3)] {
        form(
            &format!(
                "const a=[8];{escape}const b=a.filter((v:number,i:number):boolean=>v%3+i%3!==0);return a[0]%3+b[0]%3;"
            ),
            native,
            fmod,
        );
    }
}

#[test]
fn optional_array_uses_remove_confinement() {
    use super::*;
    struct Optional {
        changed: usize,
        length_type: crate::ir::Type,
    }
    impl Visitor for Optional {
        fn expression(&mut self, e: &mut Expr) -> Result<(), Diagnostic> {
            e.kind = match std::mem::replace(&mut e.kind, E::Null) {
                E::Push(a, b) => {
                    self.changed += 1;
                    E::OptionalPush(a, b)
                }
                E::Length(a) => {
                    self.changed += 1;
                    e.ty = self.length_type;
                    E::OptionalLength(a)
                }
                other => other,
            };
            Ok(())
        }
    }
    for usage in [
        "a.push(4);",
        "const use=(n:number|null):void=>{};use(a.length);",
    ] {
        let body = format!("const a=[8];{usage}return a[0]%3;");
        form(&body, 1, 0);
        let mut module = super::tests::checked(&format!("export function main():number{{{body}}}"));
        // C2 rejects optional access on this non-nullable binding. Build both optional IR forms directly.
        let mut optional = Optional {
            changed: 0,
            length_type: module.types.nullable(crate::ir::Type::Number),
        };
        visit_module(&mut module, &mut optional).unwrap();
        assert_eq!(optional.changed, 1);
        analyze(&mut module).unwrap();
        let code = crate::emitter::emit(&module, crate::OUTPUT_FACTOR).unwrap();
        assert_eq!(code.matches(" % ").count(), 0, "{code}");
        assert_eq!(code.matches("__ulua_fmod(").count(), 1, "{code}");
    }
}

#[test]
fn only_reduce_callbacks_store_an_accumulator() {
    let mut module = super::tests::checked(
        "export function main():number{const a=[8];a.forEach((v:number):number=>v%3);a.map((v:number):number=>v%3);a.filter((v:number):boolean=>v%3!==0);return a.reduce((s:number,v:number,i:number):number=>(s+v+i)%100,0);}",
    );
    let elements = super::elements::Elements::collect(&mut module).unwrap();
    assert_eq!(elements.callbacks.len(), 4);
    for f in module
        .functions
        .iter()
        .filter(|f| f.inline_callback.is_some())
    {
        let expected = if f.parameters.len() == 3 {
            Some(f.parameters[0].0)
        } else {
            None
        };
        assert_eq!(elements.callbacks[&f.binding].accumulator, expected);
    }
}
