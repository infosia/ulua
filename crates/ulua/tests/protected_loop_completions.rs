//! Source exits retain their completion order through protected calls.
//! Each accepted source uses the ABI, node, tsc, and the single-worker analyzer.
#[allow(dead_code)]
#[path = "support/script.rs"]
mod support;
#[allow(dead_code)]
#[path = "support/equality.rs"]
mod witnesses;
use std::time::Instant;

#[test]
fn protected_loop_completion_order_matches_node() {
    let start = Instant::now();
    let mut suite = witnesses::Suite::named("protected-loop-completions");
    for asynchronous in [false, true] {
        let (prefix, number, void, call, result) = if asynchronous {
            (
                "async ",
                "Promise<number>",
                "Promise<void>",
                "await t()",
                "await f()",
            )
        } else {
            ("", "number", "void", "t()", "f()")
        };
        for jump in ["break", "continue", "return 7"] {
            for location in ["try", "catch", "finally"] {
                for nested in [false, true] {
                    let action = format!("r+={call};if(i==0){{{jump};}}r+=10;");
                    let body = match location {
                        "catch" => format!(
                            "try{{throw new Error('caught');}}catch(e){{if(e instanceof Error){{print(e.message);}}{action}}}finally{{r+=100;print('final');}}"
                        ),
                        "finally" => format!("try{{r++;}}finally{{{action}print('final');}}"),
                        _ => format!("try{{{action}}}finally{{r+=100;print('final');}}"),
                    };
                    let body = if nested {
                        format!("try{{{body}}}finally{{r+=1000;print('outer');}}")
                    } else {
                        body
                    };
                    let source = format!(
                        "{prefix}function t():{number}{{return 1;}}{prefix}function f():{number}{{let r:number=0;for(let i=0;i<2;i++){{{body}}}return r;}}export {prefix}function main():{void}{{print(`${{{result}}}`);}}"
                    );
                    suite.source(&source, None);
                }
            }
        }
        // A finally return overrides a saved return or jump. An inner loop keeps its own jump target.
        suite.source(&format!("{prefix}function t():{number}{{return 1;}}{prefix}function f():{number}{{let r:number=0;for(let i=0;i<2;i++){{try{{for(let j=0;j<2;j++){{try{{r+={call};continue;}}finally{{r+=10;}}}}return r;}}finally{{return 9;}}}}return 0;}}export {prefix}function main():{void}{{print(`${{{result}}}`);}}"), Some("9\n"));
        suite.source(&format!("{prefix}function t():{number}{{return 1;}}{prefix}function f():{number}{{let r:number=0;for(let i=0;i<2;i++){{try{{try{{r+={call};break;}}finally{{r+=10;}}}}finally{{r+=100;}}}}return r;}}export {prefix}function main():{void}{{print(`${{{result}}}`);}}"), Some("111\n"));
    }
    suite.finish();
    eprintln!(
        "protected loop completion programs={} cost={:.3}s",
        suite.count,
        start.elapsed().as_secs_f64()
    );
}
