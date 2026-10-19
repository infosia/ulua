//! P6.S13 review witnesses. Each fixture isolates one receiver or length condition.
use crate::{Limits, compile};
use std::time::Instant;

fn code(source: &str) -> String {
    compile(source.as_bytes(), Limits::default())
        .unwrap_or_else(|error| panic!("{source}\n{error:?}"))
        .luau
}
fn snapshot(luau: &str, id: usize) -> bool {
    luau.lines().any(|line| {
        line.trim_start().starts_with("__ulua_t")
            && (line.contains(&format!("= (__ulua_b{id}) ::"))
                || line.contains(&format!("= ((__ulua_b{id} ::")))
    })
}

// Cost: one emission per accept/trap entry; no VM or external witness.
#[test]
fn corpus_lines_never_start_with_parenthesis() {
    let now = Instant::now();
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../corpus");
    let mut failures = vec![];
    let mut entries = 0;
    for directory in ["accept", "trap"] {
        let mut paths: Vec<_> = std::fs::read_dir(root.join(directory))
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .filter(|path| path.extension().is_some_and(|extension| extension == "ts"))
            .collect();
        paths.sort();
        for path in paths {
            let source = std::fs::read_to_string(&path).unwrap();
            let grants: Vec<_> = source
                .lines()
                .filter_map(|line| line.strip_prefix("// grants:"))
                .flat_map(|line| line.split(','))
                .map(|name| {
                    let name = name.trim();
                    let (parameters, result) = match name {
                        "hostAdd" | "hostLater" => {
                            (vec![1; if name == "hostAdd" { 2 } else { 1 }], 1)
                        }
                        "hostConcat" => (vec![3, 3], 3),
                        "hostNot" => (vec![2], 2),
                        "hostLog" => (vec![3], 0),
                        _ => panic!("unknown grant {name}"),
                    };
                    let mut signature = crate::HostSignature::new(name, parameters, result);
                    signature.asynchronous = name == "hostLater";
                    signature
                })
                .collect();
            let luau = crate::compile_with_grants(source.as_bytes(), Limits::default(), &grants)
                .unwrap_or_else(|error| panic!("{}: {error:?}", path.display()))
                .luau;
            entries += 1;
            for (line, text) in luau.lines().enumerate() {
                if text.trim_start().starts_with('(') {
                    failures.push(format!("{}:{}: {text}", path.display(), line + 1));
                }
            }
        }
    }
    eprintln!(
        "receiver class check entries={entries} cost={:.6}s",
        now.elapsed().as_secs_f64()
    );
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

macro_rules! narrowed {
    ($name:ident, $ty:expr, $initial:expr, $access:expr) => {
        #[test]
        fn $name() {
            let luau = code(&format!("type R={{x:number}};export function main():void{{let a:{}|null={};if(a!==null){{{}}}}}", $ty, $initial, $access));
            assert!(snapshot(&luau, 1), "{luau}");
            assert!(!luau.lines().any(|line| line.trim_start().starts_with('(')), "{luau}");
        }
    };
}
narrowed!(narrowed_field_read, "R", "{x:1}", "const x=a.x;");
narrowed!(narrowed_field_write, "R", "{x:1}", "a.x=2;");
narrowed!(narrowed_field_compound, "R", "{x:1}", "a.x+=2;");
narrowed!(narrowed_element_read, "number[]", "[1]", "const x=a[0];");
narrowed!(narrowed_element_write, "number[]", "[1]", "a[0]=2;");
narrowed!(narrowed_element_compound, "number[]", "[1]", "a[0]+=2;");

fn captured_index_call(op: &str) {
    let luau = code(&format!(
        "export function main():void{{let a=[1];const swap=():number=>{{a=[9];return 0;}};a[swap()]{op}5;}}"
    ));
    assert!(snapshot(&luau, 1), "{luau}");
    assert!(
        luau.find("= (__ulua_b1) ::").unwrap() < luau.find("__ulua_b2()").unwrap(),
        "{luau}"
    );
}
#[test]
fn captured_element_write_index_call_keeps_snapshot() {
    captured_index_call("=");
}
#[test]
fn captured_element_compound_index_call_keeps_snapshot() {
    captured_index_call("+=");
}
#[test]
fn nested_receiver_index_call_keeps_snapshot() {
    let luau = code(
        "export function main():void{let a=[[1]];const f=():number=>{a=[[2]];return 0;};a[f()][0]=5;}",
    );
    assert!(snapshot(&luau, 1), "{luau}");
    assert!(
        luau.find("= (__ulua_b1) ::").unwrap() < luau.find("__ulua_b2()").unwrap(),
        "{luau}"
    );
    assert!(luau.contains("__ulua_get("), "{luau}");
}

fn body_length(prefix: &str, body: &str) {
    let luau = code(&format!(
        "type R={{f:()=>number}};export function main():void{{{prefix}for(let i=0;i<1;i++){{{body}}}}}"
    ));
    let head = luau.find("for __ulua_g").expect("counted loop");
    assert!(!luau.contains("number = #"), "{luau}");
    assert!(luau[head..].contains(" < #"), "{luau}");
}
#[test]
fn length_for_of_binding_write() {
    body_length(
        "const grid=[[1]];let s=0;",
        "for(const row of grid){s+=row[i];}",
    );
}
#[test]
fn length_optional_call_exclusion() {
    body_length(
        "const a=[1];const b:R|null={f:()=>0};",
        "const x=a[i];b?.f();",
    );
}
#[test]
fn length_optional_method_exclusion() {
    body_length(
        "const a=[1];const b:number[]|null=[2];",
        "const x=a[i];b?.slice();",
    );
}
#[test]
fn length_optional_push_exclusion() {
    body_length(
        "const a=[1];const b:number[]|null=[2];",
        "const x=a[i];b?.push(1);",
    );
}
#[test]
fn length_json_exclusion() {
    body_length("const a=[1];", "const x=a[i];const r=JSON.stringify(a);");
}
#[test]
fn length_while_body_recursion() {
    body_length("let a=[1];", "const x=a[i];while(false){a=[2];}");
}
#[test]
fn length_try_body_recursion() {
    body_length("let a=[1];", "const x=a[i];try{a=[2];}catch(e){}");
}
#[test]
fn length_if_body_recursion() {
    body_length("let a=[1];", "const x=a[i];if(i===0){a=[2];}");
}

#[test]
fn length_try_catch_body_recursion() {
    body_length("let a=[1];", "const x=a[i];try{}catch(e){a=[2];}");
}
#[test]
fn length_try_finally_body_recursion() {
    body_length("let a=[1];", "const x=a[i];try{}finally{a=[2];}");
}
#[test]
fn length_if_else_body_recursion() {
    body_length("let a=[1];", "const x=a[i];if(i===0){}else{a=[2];}");
}
