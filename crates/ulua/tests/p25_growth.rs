//! P2.5 growth compares full ABI compilation at n and 4n, and checks ceiling output.
//! The Debug budget is twelve seconds, including one batch analyzer run.
#[allow(dead_code)]
#[path = "support/script.rs"]
mod support;
use std::{path::PathBuf, process::Command, time::Instant};
use support::Script;
use ulua_compiler::{Limits, OUTPUT_FACTOR, OUTPUT_PRELUDE_BYTES, compile};
const SHAPES: &[&str] = &[
    "fields",
    "literals",
    "nested",
    "tests",
    "joins",
    "functions",
    "field-product",
    "fact-product",
    "record-functions",
    "break-product",
    "recursive-product",
];
fn make(shape: &str, bytes: usize) -> String {
    let mut declarations = "type B={v:number};".to_owned();
    let mut prefix = String::new();
    let unit: String;
    let mut tail = "}";
    match shape {
        "fields" | "field-product" => {
            let fields = if shape == "fields" {
                bytes / 48
            } else {
                bytes / 160
            };
            declarations = format!(
                "type R={{{}}};",
                (0..fields)
                    .map(|i| format!("f{i}:number;"))
                    .collect::<String>()
            );
            prefix = format!(
                "const r:R={{{}}};",
                (0..fields).map(|i| format!("f{i}:0,")).collect::<String>()
            );
            unit = "r.f0=r.f0;".into();
        }
        "literals" => {
            prefix = "const xs:B[]=[];".into();
            unit = "xs.push({v:0});".into();
        }
        "nested" => {
            // Fixed alias depth isolates the cost of repeated nested literals.
            declarations = "type R0={v:number};".into();
            for i in 1..12 {
                declarations.push_str(&format!("type R{i}={{v:R{}}};", i - 1));
            }
            let mut literal = "{v:0}".to_owned();
            for _ in 1..12 {
                literal = format!("{{v:{literal}}}");
            }
            prefix = "const xs:R11[]=[];".into();
            unit = format!("xs.push({literal});");
        }
        "tests" => {
            prefix = "const x:B|null={v:1};".into();
            unit = "if(x!=null){print(`${x.v}`);}".into();
        }
        "joins" => {
            prefix = "let x:B|null={v:1};".into();
            unit = "if(x!=null){if(x.v==0){x=null;}else{x={v:0};}}x?.v??0;"
                .replace("x?.v??0;", "print(`${x?.v??0}`);");
        }
        "functions" => {
            prefix = "const x:B|null={v:1};".into();
            unit = "if(x!=null){const f=():()=>number=>():number=>x.v;const g=f();g();}".into();
        }
        "recursive-product" => {
            let count = bytes / 72;
            declarations.clear();
            for (prefix, count) in [("A", count), ("C", count + 1)] {
                for i in 0..count {
                    declarations.push_str(&format!(
                        "type {prefix}{i}={{v:{prefix}{}}};",
                        (i + 1) % count
                    ));
                }
            }
            declarations.push_str("function same(a:A0,b:C0):boolean{return a===b;}");
            unit = "".into();
        }
        "break-product" => {
            let count = bytes / 160;
            for i in 0..count {
                prefix.push_str(&format!("const x{i}:B|null={{v:0}};"));
            }
            prefix.push_str("while(true){");
            for i in 0..count {
                prefix.push_str(&format!("if(x{i}==null){{return;}}"));
            }
            unit = "if(x0.v==1){break;}".into();
            tail = "break;}}";
        }
        "fact-product" => {
            for i in 0..bytes / 180 {
                prefix.push_str(&format!(
                    "const x{i}:B|null={{v:0}};if(x{i}==null){{return;}}"
                ));
            }
            unit = "if(x0!=null){const f=():number=>x0.v;f();}".into();
        }
        _ => {
            declarations = "type R={f:(n:number)=>number};".into();
            prefix = "const r:R={f:(n:number):number=>n};".into();
            unit = "r.f(0);".into();
        }
    }
    let mut out = format!("{declarations}export function main():void{{{prefix}");
    while !unit.is_empty() && out.len() + unit.len() + tail.len() <= bytes {
        out.push_str(&unit);
    }
    out.push_str(tail);
    assert!(out.len() <= bytes, "{shape}");
    out.push_str(&" ".repeat(bytes - out.len()));
    out
}
#[test]
fn p25_full_pipeline_growth_and_output() {
    let start = Instant::now();
    let directory = std::env::temp_dir().join(format!("ulua-p25-growth-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let mut paths = vec![];
    let mut largest_ratio = 0.0f64;
    let mut largest_output = 0.0f64;
    for shape in SHAPES {
        let n = if matches!(*shape, "field-product" | "fact-product" | "break-product") {
            16384
        } else {
            4096
        };
        let inputs = [make(shape, n), make(shape, 4 * n)];
        let measure = |source: &str| {
            let now = Instant::now();
            let script = Script::new(source);
            let elapsed = now.elapsed().as_secs_f64();
            drop(script);
            elapsed
        };
        let mut times = inputs.each_ref().map(|source| measure(source));
        if times[1] / times[0] > 6.5 {
            for (time, source) in times.iter_mut().zip(&inputs) {
                *time = time.min(measure(source));
            }
        }
        let ratio = times[1] / times[0];
        largest_ratio = largest_ratio.max(ratio);
        eprintln!(
            "P2.5 growth {shape}: bytes={n}/{} ms={:.3}/{:.3} ratio={ratio:.3}",
            4 * n,
            times[0] * 1000.0,
            times[1] * 1000.0
        );
        let ceiling = make(shape, 65536);
        match compile(ceiling.as_bytes(), Limits::default()) {
            Ok(c) => {
                assert!(c.luau.len() <= OUTPUT_FACTOR * ceiling.len() + OUTPUT_PRELUDE_BYTES);
                let output_ratio = (c.luau.len().saturating_sub(OUTPUT_PRELUDE_BYTES)) as f64
                    / ceiling.len() as f64;
                largest_output = largest_output.max(output_ratio);
                eprintln!(
                    "P2.5 output {shape}: source={} output={} factor={output_ratio:.3}",
                    ceiling.len(),
                    c.luau.len()
                );
                let path = directory.join(format!("{shape}.luau"));
                std::fs::write(&path, c.luau).unwrap();
                paths.push(path);
            }
            Err(d) => assert_eq!(d.code, "limit:output", "{shape}: {d:?}"),
        }
    }
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let result = Command::new(root.join("target/luau-tools/luau-analyze"))
        .args(&paths)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
    std::fs::remove_dir_all(directory).unwrap();
    let cost = start.elapsed().as_secs_f64();
    eprintln!(
        "P2.5 growth max_ratio={largest_ratio:.3} max_output_factor={largest_output:.3} Debug_cost={cost:.3}s"
    );
    assert!(largest_ratio <= 8.0);
    assert!(cost <= 12.0, "P2.5 Debug budget: {cost:.3}s");
}
