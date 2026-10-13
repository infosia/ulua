//! Mixed jump and return blocks retain their loop target through catch and finally.
//! Measured Debug cost: 0.50 seconds for the mixed completion witnesses.
//! The catch fixture checks 195 and 256 real parameters, with node and both type checkers.
//! This target reports cost. No assertion reads a clock.
#[allow(dead_code)]
#[path = "../support/script.rs"]
mod support;
#[allow(dead_code)]
#[path = "../support/equality.rs"]
mod witnesses;
use std::time::Instant;
#[test]
fn mixed_completions_keep_for_while_and_nested_targets() {
    let start = Instant::now();
    let mut suite = witnesses::Suite::named("P2.7 mixed jumps");
    for loop_kind in ["for", "while", "nested"] {
        for block in ["try", "catch"] {
            for jump in ["break", "continue"] {
                for finally in [false, true] {
                    let mixed = format!("if(i<2){{{jump};}}return i;");
                    let body = if block == "try" {
                        format!("try{{{mixed}}}catch{{return 90;}}")
                    } else {
                        format!("try{{throw new Error();}}catch{{{mixed}}}")
                    };
                    let body = if finally {
                        format!("{body}finally{{print(`final${{i}}`);}}")
                    } else {
                        body
                    };
                    let loop_body = match loop_kind {
                        "for" => format!("for(let i=0;i<3;i++){{{body}}}"),
                        "while" => format!("let i:number=-1;while(i<2){{i++;{body}}}"),
                        _ => format!(
                            "for(let j=0;j<2;j++){{for(let i=0;i<3;i++){{try{{{body}}}finally{{print(`outer${{j}}`);}}}}}}"
                        ),
                    };
                    let source = format!(
                        "function f():number{{{loop_body}return 10;}}export function main():void{{print(`${{f()}}`);print('usable');}}"
                    );
                    suite.source(&source, None);
                }
            }
        }
    }
    suite.source("function f():number{let x:number=0;try{for(let i=0;i<3;i++){try{if(i==0){continue;}if(i==1){break;}return 7;}finally{x++;}}return x;}finally{print('outer');}}export function main():void{print(`${f()}`);}",Some("outer\n2\n"));
    suite.source("function f():number{let n:number=0;while(true){try{n++;if(n<3){continue;}return n;}catch{throw new Error();}finally{print(`f${n}`);}}}export function main():void{print(`${f()}`);}",Some("f1\nf2\nf3\n3\n"));
    suite.finish();
    eprintln!(
        "P2.7 mixed jumps count={} cost={:.3}s",
        suite.count,
        start.elapsed().as_secs_f64()
    );
}

#[test]
fn catch_parameters_are_not_parent_upvalues() {
    let start = Instant::now();
    let mut suite = witnesses::Suite::named("P2.7 catch locals");
    let unit = "try{throw new Error('x');}catch(e){if(e instanceof Error){n++;}}";
    for count in [195, 256] {
        let source = format!(
            "export function main():void{{let n:number=0;{}print(`${{n}}`);}}",
            unit.repeat(count)
        );
        suite.source(&source, Some(&format!("{count}\n")));
    }
    suite.finish();
    eprintln!(
        "P2.7 catch locals count={} cost={:.3}s",
        suite.count,
        start.elapsed().as_secs_f64()
    );
}
