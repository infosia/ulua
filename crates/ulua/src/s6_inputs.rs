//! Round 3 inputs expose pending joins and literal expansion at the source ceiling.
pub const SHAPES: &[&str] = &[
    "pending-if",
    "pending-loops",
    "spilled-and",
    "large-numbers",
    "spilled-mod-36",
    "spilled-mod-37",
    "spilled-mod-38",
    "spilled-mod-39",
    "spilled-mod-40",
    "spilled-mod-41",
    "spilled-mod-42",
];

pub fn source(shape: &str, bytes: usize) -> String {
    if let Some(terms) = shape.strip_prefix("spilled-mod-") {
        let terms: usize = terms.parse().unwrap();
        let mut source = (0..if bytes < 8192 { 0 } else { 120 })
            .map(|i| format!("function f{i}():void{{}}"))
            .collect::<String>();
        source.push_str("export function main():void{");
        for i in 0..200 {
            source.push_str(&format!("let a{i}=0;"));
        }
        source.push_str("let x=1;");
        let unit = format!("x={};", vec!["x"; terms].join("%"));
        while source.len() + unit.len() < bytes {
            source.push_str(&unit);
        }
        source.push('}');
        source.push_str(&" ".repeat(bytes - source.len()));
        return source;
    }
    let mut source = "export function main():void{let c=true;".to_owned();
    if shape == "large-numbers" {
        let mut i = 0;
        loop {
            let unit = format!("let a{i}={};", vec!["1e308"; 42].join("+"));
            if source.len() + unit.len() + 1 > bytes {
                break;
            }
            source.push_str(&unit);
            i += 1;
        }
    } else {
        let alphabet = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";
        let names: Vec<_> = (0..)
            .map(|i| {
                format!(
                    "{}{}",
                    char::from(alphabet[i / 52]),
                    char::from(alphabet[i % 52])
                )
            })
            .filter(|n| !["do", "if", "in"].contains(&n.as_str()))
            .take(if shape == "spilled-and" {
                200
            } else {
                bytes * 4 / 100
            })
            .collect();
        source.push_str("let ");
        source.push_str(
            &names
                .iter()
                .map(|n| format!("{n}={}", if shape == "spilled-and" { "true" } else { "0" }))
                .collect::<Vec<_>>()
                .join(","),
        );
        source.push(';');
        if shape == "spilled-and" {
            source.push_str("let b=true;");
            let chain = format!("b={};", vec!["b"; 38].join("&&"));
            let unit = format!("if(c){{{}}}", chain.repeat(8));
            while source.len() + unit.len() < bytes {
                source.push_str(&unit);
            }
        } else {
            source.push_str("if(c){");
            for n in &names {
                source.push_str(&format!("{n}=1;"));
            }
            source.push('}');
            if shape == "pending-loops" {
                source.push_str("while(c){for(;c;){while(c){aa=aa+1;break;}break;}break;}for(;c;){aa=aa+1;break;}");
            }
            let unit = if shape == "pending-if" {
                "if(c){}"
            } else {
                "while(c){while(c){break;}break;}"
            };
            while source.len() + unit.len() < bytes {
                source.push_str(unit);
            }
        }
    }
    source.push('}');
    source.push_str(&" ".repeat(bytes - source.len()));
    source
}
