//! Direct D18 forms, source offsets, and contextual types.
use crate::{Limits, compile};

const PREFIX: &str = "function take(xs:boolean[][]):void{}function truth():boolean{return true;}function number():number{return 1;}export function main():void{const b=true;const c=false;const n=1;const s='a';";
const MESSAGE: &str = "An inferred type cannot come from a reference; add a type annotation";

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
fn each_reference_form_needs_a_type_at_each_inference_position() {
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
    for form in ["n", "s", "NaN", "Infinity"] {
        reject("let copy=", form, ";");
        reject("const xs=[", form, "];");
        reject("const f=()=>", form, ";");
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
fn narrowed_references_need_annotations_after_equalities_and_truth_tests() {
    for condition in ["b", "!b", "b===true", "b!==false", "s==='b'", "n===2"] {
        for (name, ty, wider) in [
            ("b", "boolean", "false"),
            ("s", "string", "'c'"),
            ("n", "number", "3"),
        ] {
            reject(&format!("if({condition}){{let copy="), name, ";}");
            reject(&format!("if({condition}){{const xs=["), name, "];}");
            reject(&format!("if({condition}){{const f=()=>"), name, ";}");
            accept(&format!(
                "if({condition}){{let copy:{ty}={name};copy={wider};const xs:{ty}[]=[{name}];xs.push({wider});const f=():{ty}=>{name};}}"
            ));
        }
    }
}
