//! Join inputs use distinct references and shared edge prefixes.
pub const SHAPES: &[&str] = &["join-deep", "join-wide", "join-nestloops"];

pub fn source(shape: &str, bytes: usize) -> String {
    let mut source = "export function main():void{}function f(c:boolean):void{".to_owned();
    // Every wide exit follows four new tests. No exit carries only the entry fact.
    let (prefix, suffix) = match shape {
        "join-deep" => ("if(c){".repeat(12), "}else{return;}".repeat(12)),
        "join-wide" => ("while(true){".into(), "}".into()),
        "join-nestloops" => ("for(;c;){".repeat(10), "}".repeat(10)),
        _ => unreachable!("known join shape"),
    };
    source.push_str(&prefix);
    let mut i = 0;
    loop {
        let mut unit = String::new();
        if shape == "join-wide" {
            unit.push_str("if(c){");
        }
        for j in 0..if shape == "join-wide" { 4 } else { 1 } {
            let id = i * 4 + j;
            unit.push_str(&format!("let a{id}=0;if(a{id}!==1){{return;}}"));
        }
        match shape {
            "join-wide" => unit.push_str("break;}"),
            "join-nestloops" => unit.push_str("if(c){continue;}if(c){continue;}"),
            _ => {}
        }
        if source.len() + unit.len() + suffix.len() + 1 > bytes {
            break;
        }
        source.push_str(&unit);
        i += 1;
    }
    source.push_str(&suffix);
    source.push('}');
    source.push_str(&" ".repeat(bytes - source.len()));
    source
}
