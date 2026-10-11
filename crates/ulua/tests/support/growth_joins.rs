//! Join and alias inputs use distinct references and shared prefixes.
pub const SHAPES: &[&str] = &[
    "join-deep",
    "join-wide",
    "join-nestloops",
    "alias-chain",
    "alias-chain-eight",
];

pub fn source(shape: &str, bytes: usize) -> String {
    if shape.starts_with("alias-chain") {
        return alias_chain(bytes, shape == "alias-chain-eight");
    }
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

// The chain occupies 40% of the bytes. Each later declaration reads its last alias 40 times.
fn alias_chain(bytes: usize, distinct: bool) -> String {
    let count = if distinct { 8 } else { 1 };
    let mut source = "export function main():void{}function f():void{".to_owned();
    for i in 0..count {
        source.push_str(&format!("let x{i}=0;"));
    }
    source.push_str("const a0=x0===1;");
    let mut last = 0;
    loop {
        let unit = format!("const a{}=a{last}||x{}===1;", last + 1, last % count);
        if source.len() + unit.len() > bytes * 2 / 5 {
            break;
        }
        source.push_str(&unit);
        last += 1;
    }
    let terms = vec![format!("a{last}"); 40].join("||");
    let start = source.len();
    let mut i = 0;
    loop {
        let unit = format!("const z{i}={terms};");
        if source.len() + unit.len() - start > bytes * 2 / 5 {
            break;
        }
        source.push_str(&unit);
        i += 1;
    }
    source.push('}');
    source.push_str(&" ".repeat(bytes - source.len()));
    source
}
