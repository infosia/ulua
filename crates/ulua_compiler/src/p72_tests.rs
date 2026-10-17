//! P7.2 absent results retain their own flow rules while sharing nil lowering.
use crate::{Limits, compile};

fn check(body: &str) -> Result<crate::Compiled, crate::Diagnostic> {
    compile(
        format!(
            "type Box={{n:number|null}};export function main():void{{const xs=[1,2,3];{body}}}"
        )
        .as_bytes(),
        Limits::default(),
    )
}

#[test]
fn find_absent_accepted_flows() {
    for method in ["find", "findLast"] {
        for body in [
            "const x=CALL;",
            "let x=CALL;x=CALL;x=2;",
            "const x=CALL;const y=x??2;",
            "const x=CALL;const y=x??CALL;const z=y??2;",
            "const x=CALL;const y=CALL;const z=x??y;const n=z??2;",
            "const x=CALL;if(x==null){}else{const y:number=x;}",
            "const x=CALL;if(x!=null){const y:number=x;}",
            "const x=CALL;if(x===null){}else{const y:number=x;}",
            "const x=CALL;if(x!==null){const y:number=x;}",
            "const x=CALL;if(null!=x){const y:number=x;}",
            "const x=CALL;const y=x;",
        ] {
            let body = body.replace(
                "CALL",
                &format!("xs.{method}((v:number,i:number):boolean=>i===1)"),
            );
            assert!(check(&body).is_ok(), "{body}: {:?}", check(&body).err());
        }
        let source = format!(
            "type Item={{n:number;m:(v:number)=>void}};export function main():void{{const xs:Item[]=[{{n:1,m:(v:number):void=>{{}}}}];const x=xs.{method}((v:Item):boolean=>true);const n=x?.n??0;x?.m(n);}}"
        );
        compile(source.as_bytes(), Limits::default()).unwrap();
    }
}

#[test]
fn find_absent_rejected_flows() {
    let prefix = "const x=xs.find((v:number):boolean=>true);";
    for (body, code, message) in [
        (
            "const y:number|null=x;",
            "C2",
            "a find result is held only through an inferred type",
        ),
        (
            "let y=xs.find((v:number):boolean=>true);y=null;",
            "C2",
            "null is not assignable to a find result",
        ),
        (
            "function f(v:number|null):void{}f(x);",
            "C2",
            "a find result is held only through an inferred type",
        ),
        (
            "function f():number|null{return x;}",
            "C2",
            "a find result is held only through an inferred type",
        ),
        (
            "const f=()=>xs.find((v:number):boolean=>true);",
            "C2",
            "a find result is held only through an inferred type",
        ),
        (
            "const o={n:x};",
            "C2",
            "a find result is held only through an inferred type",
        ),
        (
            "const o:Box={n:x};",
            "C2",
            "a find result is held only through an inferred type",
        ),
        (
            "const o:Box={n:0};o.n=x;",
            "C2",
            "a find result is held only through an inferred type",
        ),
        (
            "const a=[x];",
            "C2",
            "a find result is held only through an inferred type",
        ),
        (
            "xs[0]=x;",
            "C2",
            "a find result is held only through an inferred type",
        ),
        (
            "xs.push(x);",
            "C2",
            "a find result is held only through an inferred type",
        ),
        (
            "const s=`${x}`;",
            "C2",
            "a find result is not a template substitution",
        ),
        (
            "const b=x===2;",
            "C7",
            "Equality requires one primitive type, identical array types, or identical object types",
        ),
        (
            "let y=x;",
            "C11",
            "An inferred type cannot come from a narrowable reference; add a type annotation",
        ),
        (
            "if(x!=null){let y=x;}",
            "C11",
            "An inferred type cannot come from a narrowable reference; add a type annotation",
        ),
        (
            "if(x!=null){const a=[x];}",
            "C11",
            "An inferred type cannot come from a narrowable reference; add a type annotation",
        ),
        (
            "if(x!=null){const o={n:x};}",
            "C11",
            "An inferred type cannot come from a narrowable reference; add a type annotation",
        ),
        (
            "if(x!=null){const f=()=>x;}",
            "C11",
            "An inferred type cannot come from a narrowable reference; add a type annotation",
        ),
    ] {
        let error = check(&format!("{prefix}{body}")).expect_err(body);
        assert_eq!(
            (error.code, error.message.as_str()),
            (code, message),
            "{body}"
        );
    }
}

#[test]
fn find_callbacks_keep_c69_signature_rules() {
    for method in ["find", "findLast"] {
        for callback in [
            "()=>true",
            "(v:number,i:number,a:number[]):boolean=>true",
            "(v:string):boolean=>true",
            "(v:number):number=>v",
        ] {
            assert!(check(&format!("xs.{method}({callback});")).is_err());
        }
        let compiled = check(&format!("xs.{method}((v:number):boolean=>true);")).unwrap();
        assert!(compiled.luau.contains("if #a ~= n then __ulua_trap(2) end"));
        assert!(compiled.luau.contains("if value then return v end"));
        assert!(compiled.luau.contains("return nil"));
    }
}
