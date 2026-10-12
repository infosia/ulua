//! Direct Node, tsc, analyzer, and ABI witnesses for the array method contract.
//! Measured Debug cost: 3.07 s in the P2.3 phase review.
#[allow(dead_code)]
#[path = "support/script.rs"]
mod support;
#[path = "support/p23.rs"]
mod witness;
use witness::Suite;

#[test]
fn array_methods_match_node_and_the_named_traps() {
    let mut s = Suite::new();
    positions(&mut s);
    callbacks(&mut s);
    loops(&mut s);
    random(&mut s);
    s.finish();
}
fn positions(s: &mut Suite) {
    for position in [
        "-Infinity",
        "-99",
        "-3.9",
        "-1",
        "-0.9",
        "-0",
        "0",
        "0.9",
        "1.9",
        "3",
        "99",
        "NaN",
        "Infinity",
    ] {
        for array in ["[NaN,-0,2,NaN,0]", "[]"] {
            for method in ["indexOf", "lastIndexOf", "includes"] {
                for value in ["NaN", "-0", "2", "99"] {
                    s.case(&format!(
                        "const xs:number[]={array};return `${{xs.{method}({value},{position})}}`;"
                    ));
                }
            }
        }
        for array in ["[1,2,3,4,5]", "[]"] {
            s.case(&format!(
                "const xs:number[]={array};return xs.slice({position}).join();"
            ));
            s.case(&format!("const xs:number[]={array};const removed=xs.splice({position});return `${{removed.join()}}/${{xs.join()}}`;"));
            for end in ["-Infinity", "-2.9", "NaN", "2.9", "99", "Infinity"] {
                s.case(&format!(
                    "const xs:number[]={array};return xs.slice({position},{end}).join();"
                ));
                s.case(&format!(
                    "const xs:number[]={array};return xs.fill(9,{position},{end}).join();"
                ));
                s.case(&format!("const xs:number[]={array};const removed=xs.splice({position},{end});return `${{removed.join()}}/${{xs.join()}}`;"));
                s.case(&format!(
                    "const xs:number[]={array};return xs.copyWithin({position},0,{end}).join();"
                ));
                s.case(&format!(
                    "const xs:number[]={array};return xs.copyWithin(0,{position},{end}).join();"
                ));
            }
        }
        let body = format!("const xs=[1,2,3];const v=xs.at({position});");
        if matches!(
            position,
            "-3.9" | "-1" | "-0.9" | "-0" | "0" | "0.9" | "1.9" | "NaN"
        ) {
            s.case(&format!("{body}return `${{v}}`;"));
        } else {
            s.trap(&body, "trap:index-out-of-range");
        }
        s.trap(
            &format!("const xs:number[]=[];xs.at({position});"),
            "trap:index-out-of-range",
        );
    }
    for method in ["indexOf", "lastIndexOf", "includes"] {
        for array in ["[NaN,-0,2,NaN,0]", "[]"] {
            s.case(&format!(
                "const xs:number[]={array};return `${{xs.{method}(NaN)}}:${{xs.{method}(-0)}}`;"
            ));
        }
        s.case(&format!(
            "const xs=['a','b','a'];return `${{xs.{method}('a',1)}}`;"
        ));
        s.case(&format!(
            "const xs=[true,false,true];return `${{xs.{method}(false)}}`;"
        ));
        s.case(&format!(
            "const x=[1];const xs:number[][]=[x,[1]];return `${{xs.{method}(x)}}`;"
        ));
    }
    for array in [
        "[NaN,-0,Infinity,-Infinity,0.1,1e21,1e-7,1e20,1e-6]",
        "[true,false]",
        "['','a','😀','é']",
        "[]",
    ] {
        let annotation = if array == "[]" || array.starts_with("[NaN") {
            ":number[]"
        } else {
            ""
        };
        for args in ["", "''", "'|'", "'😀'"] {
            s.case(&format!(
                "const xs{annotation}={array};return xs.join({args});"
            ));
        }
    }
    for array in ["[1,2,3,4]", "[]"] {
        s.case(&format!(
            "const xs:number[]={array};return `${{xs.unshift(9)}}:${{xs.join()}}`;"
        ));
        s.case(&format!(
            "const xs:number[]={array};return xs.reverse().join();"
        ));
        s.case(&format!(
            "const xs:number[]={array};return xs.concat([]).concat([9]).join();"
        ));
        s.case(&format!(
            "const xs:number[]={array};return xs.slice().join();"
        ));
        s.case(&format!(
            "const xs:number[]={array};return xs.fill(9).join();"
        ));
        s.case(&format!(
            "const xs:number[]={array};return xs.copyWithin(1,0).join();"
        ));
        s.case(&format!(
            "const xs:number[]={array};return xs.copyWithin(0,1).join();"
        ));
        s.case(&format!(
            "const xs:number[]={array};xs.length=0;return `${{xs.length}}`;"
        ));
    }
    for method in ["pop", "shift"] {
        s.trap(
            &format!("const xs:number[]=[];xs.{method}();"),
            "trap:index-out-of-range",
        );
        s.case(&format!(
            "const xs=[-0,2,3];const v=xs.{method}();return `${{1/v}}:${{xs.join()}}`;"
        ));
        s.case(&format!(
            "const xs=['a'];return `${{xs.{method}()}}:${{xs.length}}`;"
        ));
    }
    s.case("const xs=[1,2,3];const original=xs;function arg():number{xs[0]=9;xs.push(4);return -1;}return original.slice(arg()).join();");
    s.case("let xs=[1,2];const old=xs;function arg():number{xs=[9];old.push(3);return 0;}return xs.slice(arg()).join();");
    s.case("const xs=[1,2,3];let order=0;function receiver():number[]{order=order*10+1;return xs;}function target():number{order=order*10+2;return 0;}function start():number{order=order*10+3;return 1;}function end():number{order=order*10+4;return 3;}const ys=receiver().copyWithin(target(),start(),end());return `${order}:${ys.join()}`;");
}
fn callbacks(s: &mut Suite) {
    for array in ["[3,1,2]", "[]"] {
        s.case(&format!("const xs:number[]={array};let n=0;xs.forEach((v:number,i:number):void=>{{n=n*10+v+i;}});return `${{n}}`;"));
        s.case(&format!("const xs:number[]={array};return xs.map((v:number,i:number):number=>v+i).filter((v:number):boolean=>v>1).join();"));
        s.case(&format!(
            "const xs:number[]={array};return xs.flatMap((v:number):number[]=>v>1?[v,v]:[]).join();"
        ));
        for method in ["some", "every", "findIndex", "findLastIndex"] {
            for predicate in ["v>1", "v>9", "v>0"] {
                s.case(&format!("const xs:number[]={array};return `${{xs.{method}((v:number,i:number):boolean=>{predicate}&&i<2)}}`;"));
            }
        }
        for method in ["reduce", "reduceRight"] {
            s.case(&format!(
                "const xs:number[]={array};return `${{xs.{method}((a:number,v:number,i:number):number=>a*2+v+i,10)}}`;"
            ));
            s.case(&format!("const xs:number[]={array};return xs.{method}((a:string,v:number):string=>`${{a}}${{v}}`,'x');"));
            s.case(&format!("const xs:number[]={array};let order=0;const value=xs.{method}((a:number,v:number):number=>{{order=order*10+v;return a-v;}},99);return `${{value}}:${{order}}`;"));
        }
    }
    for comparator in ["a[0]-b[0]", "(a[0]-b[0])/10", "NaN", "-0"] {
        s.case(&format!("const xs=[[1,0],[0,1],[1,2],[0,3],[1,4],[0,5]];xs.sort((a:number[],b:number[]):number=>{comparator});return xs.map((v:number[]):number=>v[1]).join();"));
    }
    for comparator in ["0", "NaN", "-0"] {
        s.case(&format!(
            "const xs=[3,1,2];xs.sort((a:number,b:number):number=>{comparator});return xs.join();"
        ));
    }
    s.case("const xs:number[]=[];xs.sort((a:number,b:number):number=>a-b);return xs.join();");
    s.case("const xs=[1];xs.sort((a:number,b:number):number=>a-b);return xs.join();");
    s.case("const xs=[1,2,3];return xs.map((v:number):number=>xs.filter((w:number):boolean=>w<v).reduce((a:number,w:number):number=>a+w,0)).join();");
    s.case(
        "const xs=[1,2,3];return xs.map((v:number,i:number):number=>{xs[2]=9;return v+i;}).join();",
    );
    s.case(
        "const xs=[1,2];return xs.map((v:number):number=>{xs.push(3);xs.pop();return v;}).join();",
    );
    s.case("const xs=[1,2];xs.forEach((v:number):number=>v);return xs.join();");
    s.case("let xs=[1];const old=xs;function callback():(v:number)=>number{xs=[9];old.push(2);return (v:number):number=>v;}return xs.map(callback()).join();");
    for method in [
        "forEach",
        "map",
        "filter",
        "some",
        "every",
        "findIndex",
        "findLastIndex",
        "flatMap",
        "reduce",
        "reduceRight",
        "sort",
    ] {
        for mutation in [
            "xs.push(9);",
            "xs.pop();",
            "xs.shift();",
            "xs.splice(0,1);",
            "xs.length=0;",
            "xs.map((x:number):number=>{xs.push(x);return x;});",
            "const missing=xs[99];",
        ] {
            let kind = if mutation == "const missing=xs[99];" {
                "trap:index-out-of-range"
            } else {
                "trap:array-modified"
            };
            let (params, result, value, tail) = match method {
                "forEach" => ("v:number", "void", "", ""),
                "flatMap" => ("v:number", "number[]", "return [v];", ""),
                "map" => ("v:number", "number", "return v;", ""),
                "reduce" | "reduceRight" => ("a:number,v:number", "number", "return a+v;", ",0"),
                "sort" => ("a:number,v:number", "number", "return a-v;", ""),
                "every" => ("v:number", "boolean", "return true;", ""),
                _ => ("v:number", "boolean", "return false;", ""),
            };
            let condition = if method == "sort" { "calls==2" } else { "v==2" };
            s.trap(&format!("const xs=[1,2,3];let calls=0;xs.{method}(({params}):{result}=>{{calls++;if({condition}){{{mutation}}}{value}}}{tail});"), kind);
        }
    }
}
fn loops(s: &mut Suite) {
    let loops = "for(let i=0;i<n;i++){total+=i;}".repeat(200);
    s.case(&format!(
        "const n=3;let total=0;{loops}return `${{total}}`;"
    ));
    s.case("const fs:(()=>number)[]=[];for(let i=0;i<3;i++){fs.push(()=>i);}for(let i=0;i<3;i++){fs.push(()=>i);}return fs.map((f:()=>number):number=>f()).join();");
    let head = (0..32)
        .map(|i| format!("i{i}=0"))
        .collect::<Vec<_>>()
        .join(",");
    for tails in [0, 1] {
        let body = format!(
            "{}{}{}",
            format!("for(let {head};false;){{").repeat(4),
            "for(;false;){".repeat(tails),
            "}".repeat(4 + tails)
        );
        s.case(&format!("{body}return 'ok';"));
    }
    for body in [
        "const xs=[1];xs.sort();",
        "const xs=[1];xs.reduce((a:number,v:number):number=>a+v);",
        "const xs=[1];xs.map((v:number,i:number,a:number[]):number=>v);",
        "const xs=[()=>1];xs.indexOf(()=>1);",
        "const xs=[1];xs.flat();",
    ] {
        let error = ulua_compiler::compile(
            format!("export function main():void{{{body}}}").as_bytes(),
            ulua_compiler::Limits::default(),
        )
        .unwrap_err();
        assert_eq!(error.code, "C6", "{body}");
    }
}
fn random(s: &mut Suite) {
    let before = (s.count, s.comparable, s.traps, s.rejects);
    let mut state = 0x23a3390_u64;
    for i in 0..320 {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
        let length = ((state >> 32) % 8) as usize;
        let entries = (0..length)
            .map(|j| ((state >> (j * 5)) % 9).to_string())
            .collect::<Vec<_>>()
            .join(",");
        let position = ["NaN", "Infinity", "-Infinity", "-2.8", "0.9", "3.1"][(state % 6) as usize];
        let body = match i%8 {
            0 => format!("return xs.map((v:number,i:number):number=>v+i).filter((v:number):boolean=>v>3).slice({position}).join();"),
            1 => format!("xs.sort((a:number,b:number):number=>(a-b)/3);return `${{xs.includes(4,{position})}}:${{xs.join()}}`;"),
            2 => "return xs.flatMap((v:number):number[]=>v>4?[v]:[]).reduceRight((a:string,v:number):string=>`${a}${v}`,'');".into(),
            3 => format!("const ys=xs.splice({position},2.9);xs.unshift(7);xs.copyWithin(1,0);return `${{ys.join()}}/${{xs.join()}}`;"),
            4 => "return `${xs.every((v:number):boolean=>v>=0)}:${xs.findLastIndex((v:number):boolean=>v<4)}`;".into(),
            5 => "xs.map((v:number):number=>{xs.push(v);return v;});return xs.join();".into(),
            6 => "xs.filter((v:number):boolean=>{const missing=xs[99];return true;});return xs.join();".into(),
            _ => "return xs.map((v:number,i:number,a:number[]):number=>v).join();".into(),
        };
        let trap = match (i % 8, length) {
            (5, 1..) => Some("trap:array-modified"),
            (6, 1..) => Some("trap:index-out-of-range"),
            _ => None,
        };
        s.source(&format!("function score():string{{const xs:number[]=[{entries}];{body}}}export function main():void{{print(score());}}"),trap);
    }
    eprintln!(
        "P2.3 random: programs=320 accepted={} node={} traps={} rejected={}",
        s.count - before.0,
        s.comparable - before.1,
        s.traps - before.2,
        s.rejects - before.3
    );
}

#[test]
fn helper_loops_use_steps_and_callback_traps_leave_the_instance_usable() {
    use ulua::*;
    let entries = vec!["1"; 1000].join(",");
    let source = format!(
        "const xs=[{entries}];export function run():number{{xs.sort((a:number,b:number):number=>a-b);return xs.length;}}export function good():number{{return 7;}}export function main():void{{}}"
    );
    let mut script = support::Script::new(&source);
    let mut limits = RunLimits::default();
    limits.max_steps = 5000;
    assert_eq!(script.instance(limits).0, ULUA_OK);
    let (status, code, _) = script.call(script.index("run"), &[]);
    assert_eq!((status, code.as_str()), (ULUA_FAILED, "limit:steps"));
    let (status, code, value) = script.call(script.index("good"), &[]);
    assert_eq!((status, code.as_str(), value.number), (ULUA_OK, "", 7.0));
    let source = "let count=0;export function bad():void{const xs=[1,2,3];xs.map((v:number):number=>{xs.push(v);count=7;return v;});}export function good():number{return count;}export function main():void{}";
    let mut script = support::Script::new(source);
    assert_eq!(script.instance(RunLimits::default()).0, ULUA_OK);
    for _ in 0..2 {
        let (status, code, _) = script.call(script.index("bad"), &[]);
        assert_eq!(
            (status, code.as_str()),
            (ULUA_FAILED, "trap:array-modified")
        );
        let (status, code, value) = script.call(script.index("good"), &[]);
        assert_eq!((status, code.as_str(), value.number), (ULUA_OK, "", 7.0));
    }
    let source = "const xs=[1,2];const ys=xs.map((v:number):number=>{xs.length=0;return v;});export function main():void{}";
    let mut script = support::Script::new(source);
    assert_eq!(
        script.instance(RunLimits::default()),
        (ULUA_FAILED, "trap:array-modified".into())
    );
}
