//! Direct D18 forms, source offsets, and contextual types.
use crate::{Limits, compile};

const PREFIX: &str = "function take(xs:boolean[][]):void{}function truth():boolean{return true;}function number():number{return 1;}export function main():void{const b=true;const c=false;const n=1;const s='a';const bs:boolean[]=[true];const bss:boolean[][]=[[true]];const ns:number[]=[1];const ss:string[]=['a'];const fs:(()=>number)[]=[number];";
const MESSAGE: &str =
    "An inferred type cannot come from a boolean reference; add a type annotation";

fn accept(body: &str) {
    let source = format!("{PREFIX}{body}}}");
    assert!(
        compile(source.as_bytes(), Limits::default()).is_ok(),
        "{source}"
    );
}

fn reject(before: &str, form: &str, after: &str) {
    let source = format!("{PREFIX}{before}{form}{after}}}");
    let error = compile(source.as_bytes(), Limits::default()).unwrap_err();
    assert_eq!(error.code, "C11", "{source}: {error:?}");
    assert_eq!(error.message, MESSAGE, "{source}");
    assert_eq!(
        error.offset as usize,
        PREFIX.len() + before.len(),
        "{source}"
    );
}

#[test]
fn each_boolean_reference_form_needs_a_type_at_each_inference_position() {
    for form in [
        "b",
        "(b)",
        "((b))",
        "!b",
        "!((b))",
        "c?b:false",
        "c?false:b",
        "b&&true",
        "true&&b",
        "b||false",
        "false||b",
        "c?(!b&&true):(false||b)",
        "bs[0]",
        "bss[0][0]",
        "(bs[0])",
        "!bs[0]",
        "c?bs[0]:false",
        "c?false:bss[0][0]",
        "bs[0]&&true",
        "true&&bss[0][0]",
        "bs[0]||false",
        "false||bss[0][0]",
        "c?(!bs[0]&&true):(false||bss[0][0])",
    ] {
        reject("let copy=", form, ";");
        reject("const xs=[", form, "];");
        reject("const xs=[false,", form, "];");
        reject("const xs=[[false],[", form, "]];");
        reject("const f=()=>", form, ";");
        // A const copy does not infer a mutable type.
        accept(&format!("const copy={form};"));
        for body in [
            format!("let copy:boolean={form};copy=false;"),
            format!("const xs:boolean[]=[{form}];xs.push(false);"),
            format!("const xs:boolean[][]=[[false],[{form}]];"),
            format!("take([[{form}]]);"),
            format!("const xs:boolean[][]=[];xs.push([{form}]);"),
            format!("let xs:boolean[][]=[];xs=[[{form}]];"),
            format!("const xs:boolean[][]=[[]];xs[0]=[{form}];"),
            format!("const f=():boolean=>{form};"),
            format!("const f=():boolean[]=>[{form}];"),
            format!("const f:()=>boolean=()=>{form};"),
            format!("const f:()=>boolean[][]=()=>[[{form}]];"),
        ] {
            accept(&body);
        }
    }
}

#[test]
fn nonboolean_references_keep_inference_at_each_position() {
    for form in [
        "n",
        "s",
        "NaN",
        "Infinity",
        "ns",
        "ss",
        "bs",
        "bss[0]",
        "ns[0]",
        "ss[0]",
        "ns.length",
        "bss[0].length",
        "(n)",
        "c?n:2",
        "c?s:'b'",
    ] {
        accept(&format!(
            "let copy={form};const xs=[{form}];const f=()=>{form};"
        ));
    }
    for form in ["number", "fs[0]", "fs"] {
        accept(&format!("let copy={form};const xs=[{form}];"));
    }
    // The separate function-result rule still requires an arrow result annotation.
    accept("const f=():()=>number=>number;const g=():(()=>number)[]=>fs;");
    for form in ["number", "fs[0]", "fs"] {
        let source = format!("{PREFIX}const f=()=>{form};}}");
        let error = compile(source.as_bytes(), Limits::default()).unwrap_err();
        assert_eq!(error.code, "C11", "{source}");
        assert_eq!(
            error.message, "Function result needs an annotation",
            "{source}"
        );
    }
}

#[test]
fn an_inferred_sibling_type_is_not_a_contextual_type() {
    reject("const xs=[[false],[true,", "b", "]];");
    reject("const xs=[():boolean=>true,()=>", "b", "];");
    reject("const xs=true?[false]:[", "b", "];");
    accept("const xs:boolean[][]=[[false],[true,b]];");
}

#[test]
fn nonreference_forms_keep_inference() {
    for form in [
        "1",
        "'a'",
        "true",
        "(false)",
        "n+1",
        "-n",
        "number()",
        "`${s}`",
        "`text`",
        "c?1:2",
        "c?true:false",
        "!true",
        "true&&false",
        "true||false",
        "n===2",
        "b!==false",
    ] {
        // D16 separately restricts boolean let initializers.
        if !matches!(
            form,
            "c?true:false" | "!true" | "true&&false" | "true||false"
        ) {
            accept(&format!("let copy={form};"));
        }
        accept(&format!("const xs=[{form}];const f=()=>{form};"));
    }
    accept("const xs=[truth()];const f=()=>truth();");
    accept("const xs=[()=>1];const f=()=>[[1]];");
    // Only the conditional arms define a reference form.
    accept("let copy=b?1:2;const xs=[b?true:false];const f=()=>b?1:2;");
    // References inside an arithmetic, call, or template do not define its form.
    accept("const xs=[number()+n];const f=()=>`${b}`;");
}

#[test]
fn only_boolean_references_need_annotations_after_equalities_and_truth_tests() {
    for condition in ["b", "!b", "b===true", "b!==false", "s==='b'", "n===2"] {
        for (name, ty, wider) in [
            ("b", "boolean", "false"),
            ("s", "string", "'c'"),
            ("n", "number", "3"),
        ] {
            if ty == "boolean" {
                reject(&format!("if({condition}){{let copy="), name, ";}");
                reject(&format!("if({condition}){{const xs=["), name, "];}");
                reject(&format!("if({condition}){{const f=()=>"), name, ";}");
            } else {
                accept(&format!(
                    "if({condition}){{let copy={name};copy={wider};const xs=[{name}];xs.push({wider});const f=()=>{name};}}"
                ));
            }
            accept(&format!(
                "if({condition}){{let copy:{ty}={name};copy={wider};const xs:{ty}[]=[{name}];xs.push({wider});const f=():{ty}=>{name};}}"
            ));
        }
    }
}
