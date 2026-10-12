//! P2.5 growth compares full ABI compilation at n and 4n, and checks ceiling output.
//! The Debug budget is twelve seconds, including one batch analyzer run.
//! Round 6 measured Debug cost: 8.860 seconds. The largest ratio is 4.022.
//! Condition ratios: conditional 3.650, alternating logical 3.786, conditional with wrappers 3.601.
//! Logical with wrappers: 3.722. Nullable conditional: 3.711. Continue product: 3.876.
//! A conditional test gives no facts. The nullable shape uses a separate null guard in its body.
//! Round 2 ABI times compare 16 KiB and 64 KiB. Before uses the handoff HEAD.
//! Debug ybreak: 888.475/13581.536 ms -> 28.809/112.749 ms; ratios 15.286 -> 3.914.
//! Debug ycont: 678.899/10635.650 ms -> 27.229/106.332 ms; ratios 15.666 -> 3.905.
//! Debug distinct consts: 163.846/2246.953 ms -> 25.709/100.300 ms; ratios 13.714 -> 3.901.
//! Debug continue/update: 675.917/10637.277 ms -> 27.174/105.922 ms; ratios 15.738 -> 3.898.
//! Debug if/else ends: 39.578/158.546 ms -> 39.702/157.984 ms; ratios 4.006 -> 3.979.
//! Release ybreak: 94.294/991.532 ms -> 21.752/42.102 ms; ratios 10.515 -> 1.936.
//! Release ycont: 50.623/781.942 ms -> 7.733/25.709 ms; ratios 15.446 -> 3.324.
//! Release distinct consts: 15.706/189.527 ms -> 6.058/21.778 ms; ratios 12.067 -> 3.595.
//! Release continue/update: 51.067/792.868 ms -> 6.518/24.327 ms; ratios 15.526 -> 3.732.
//! Release if/else ends: 9.481/36.893 ms -> 9.500/35.942 ms; ratios 3.891 -> 3.783.
//! Stock tsc gives TS2563 for ceiling ybreak and if-end-writes; the other new shapes pass.
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
    "conditional-tests",
    "alternating-logical-tests",
    "conditional-wrapped-tests",
    "logical-wrapped-tests",
    "nullable-conditional-tests",
    "continue-product",
    "ybreak",
    "ycont",
    "distinct-const-break",
    "continue-update-writes",
    "if-end-writes",
];
fn join_source(shape: &str, bytes: usize) -> String {
    let mut out = "function g():string|null{return null;}export function main():void{const c:boolean=false;let y:string|null=g();".to_owned();
    let tail = "}}";
    out.push_str(match shape {
        "ycont" => "for(let i=0;i<1;i++){",
        "continue-update-writes" => "for(let i=0;i<1;y=g()){i+=1;",
        "distinct-const-break" => "while(c){if(y==null){return;}",
        _ => "while(c){",
    });
    let mut i = 0;
    loop {
        let unit = match shape {
            "ybreak" => "if(y==null)break;y=g();".to_owned(),
            "ycont" | "continue-update-writes" => "if(y==null)continue;y=g();".to_owned(),
            "distinct-const-break" => {
                format!("const x{i}:string|null=g();if(x{i}==null){{y=g();break;}}")
            }
            _ => "if(y==null){y=g();}else{y=g();}y=g();".to_owned(),
        };
        if out.len() + unit.len() + tail.len() > bytes {
            break;
        }
        out.push_str(&unit);
        i += 1;
    }
    out.push_str(tail);
    out.push_str(&" ".repeat(bytes - out.len()));
    out
}
fn condition_depth(shape: &str) -> usize {
    match shape {
        "conditional-wrapped-tests" | "logical-wrapped-tests" => 8,
        "nullable-conditional-tests" => 20,
        _ => 21,
    }
}
fn condition(shape: &str, depth: usize) -> String {
    let mut test = if shape == "nullable-conditional-tests" {
        "x!=null".to_owned()
    } else {
        "b".to_owned()
    };
    for level in 0..depth {
        test = match shape {
            "alternating-logical-tests" => {
                format!("({test}{}b)", if level % 2 == 0 { "&&" } else { "||" })
            }
            "conditional-wrapped-tests" => format!("(!(({test}?true:false)))"),
            "logical-wrapped-tests" => {
                format!("(!(({test}{}b)))", if level % 2 == 0 { "&&" } else { "||" })
            }
            _ => format!("({test}?true:false)"),
        };
    }
    test
}
fn condition_source(shape: &str, depth: usize) -> String {
    let test = condition(shape, depth);
    let body = if shape == "nullable-conditional-tests" {
        "if(x!=null){x.v+=1;}"
    } else {
        ""
    };
    format!(
        "type B={{v:number}};export function main():void{{const b:boolean=true;const x:B|null={{v:0}};if({test}){{{body}}}}}"
    )
}
#[test]
fn condition_wrappers_reach_the_nesting_bound() {
    for shape in [
        "conditional-tests",
        "alternating-logical-tests",
        "conditional-wrapped-tests",
        "logical-wrapped-tests",
        "nullable-conditional-tests",
    ] {
        let depth = condition_depth(shape);
        compile(condition_source(shape, depth).as_bytes(), Limits::default()).unwrap();
        assert_eq!(
            compile(
                condition_source(shape, depth + 1).as_bytes(),
                Limits::default()
            )
            .unwrap_err()
            .code,
            "limit:nesting-depth",
            "{shape}"
        );
    }
}
fn make(shape: &str, bytes: usize) -> String {
    if matches!(
        shape,
        "ybreak" | "ycont" | "distinct-const-break" | "continue-update-writes" | "if-end-writes"
    ) {
        return join_source(shape, bytes);
    }
    let mut declarations = "type B={v:number};".to_owned();
    let mut prefix = String::new();
    let unit: String;
    let mut tail = "}";
    match shape {
        "conditional-tests"
        | "alternating-logical-tests"
        | "conditional-wrapped-tests"
        | "logical-wrapped-tests"
        | "nullable-conditional-tests" => {
            prefix = "const b:boolean=true;const x:B|null={v:0};".into();
            let test = condition(shape, condition_depth(shape));
            let body = if shape == "nullable-conditional-tests" {
                "if(x!=null){x.v+=1;}"
            } else {
                ""
            };
            unit = format!("if({test}){{{body}}}");
        }
        "continue-product" => {
            let count = bytes / 190;
            for i in 0..count {
                prefix.push_str(&format!("const x{i}:B|null={{v:0}};"));
            }
            prefix.push_str("for(let i=0;i<2;i+=x0.v){");
            for i in 0..count {
                prefix.push_str(&format!("if(x{i}==null){{return;}}"));
            }
            unit = "if(x0.v==1){continue;}".into();
            tail = "i+=1;}}";
        }
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
        let n = if matches!(
            *shape,
            "field-product"
                | "fact-product"
                | "break-product"
                | "continue-product"
                | "ybreak"
                | "ycont"
                | "distinct-const-break"
                | "continue-update-writes"
                | "if-end-writes"
        ) {
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
