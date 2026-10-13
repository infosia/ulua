//! Protected closure budgets include owner locals, outer upvalues, and helpers.
//! Bound witnesses use 192 binding upvalues. Temporary shadows are closure locals; the next count is 193.
//! Measured single-worker Debug costs: budgets 1.258 seconds; shared values 0.401 seconds; target 1.66 seconds.
//! This target reports its measured Debug cost without a time assertion.
#[allow(dead_code)]
#[path = "support/script.rs"]
mod support;
#[allow(dead_code)]
#[path = "support/equality.rs"]
mod witnesses;
use std::time::Instant;

fn declarations(prefix: &str, count: usize) -> String {
    (0..count)
        .map(|i| format!("let {prefix}{i}:number=1;"))
        .collect()
}

fn source(kind: &str, position: &str, outer: usize, own: usize) -> String {
    let mut writes = String::new();
    for i in 0..outer.max(own) {
        writes.push_str(&format!("v{}=v{}+w{};", i % outer, i % outer, i % own));
    }
    let block = match position {
        "catch" => format!("try{{throw new Error();}}catch{{{writes}}}finally{{}}"),
        "finally" => format!("try{{}}finally{{{writes}}}"),
        _ => format!("try{{{writes}}}finally{{}}"),
    };
    let body = format!("{}{block}return w1+v1;", declarations("w", own));
    match kind {
        "method" => format!(
            "{}class C{{g():number{{{body}}}}}export function main():void{{print(`${{new C().g()}}`);}}",
            declarations("v", outer)
        ),
        "arrow" => format!(
            "export function main():void{{{}const g=():number=>{{{body}}};print(`${{g()}}`);}}",
            declarations("v", outer)
        ),
        _ => format!(
            "{}function g():number{{{body}}}export function main():void{{print(`${{g()}}`);}}",
            declarations("v", outer)
        ),
    }
}

#[test]
fn protected_closures_obey_the_bound_and_the_next_count() {
    let start = Instant::now();
    let mut suite = witnesses::Suite::named("P2.7 protected upvalues");
    for kind in ["function", "method", "arrow"] {
        for position in ["try", "catch", "finally"] {
            let accepted = source(kind, position, 96, 96);
            suite.source(&accepted, Some("3\n"));
            for (outer, own) in [(96, 97), (98, 98), (99, 99), (120, 120), (100, 100)] {
                let rejected = source(kind, position, outer, own);
                let diagnostic =
                    ulua_compiler::compile(rejected.as_bytes(), Default::default()).unwrap_err();
                assert_eq!(
                    diagnostic.code, "limit:upvalues",
                    "{kind}/{position}: {diagnostic:?}"
                );
                assert_eq!(diagnostic.offset, rejected.find("try").unwrap() as u64);
                support::Script::compile(&rejected, Default::default(), "limit:upvalues");
            }
        }
    }
    // Spilled binding fields share one captured table. Hundreds of reads do not count separately.
    suite.source(&source("function", "try", 8, 256), Some("34\n"));
    suite.source("export function main():void{try{let x:number=1;const f=():number=>x;const g=():number=>{const old=x;x++;return old;};print(`${f()} ${g()} ${f()}`);}catch(e){if(e instanceof Error){throw e;}}}",Some("1 1 2\n"));
    suite.finish();
    eprintln!(
        "P2.7 protected upvalues accepted={} rejected=45 cost={:.3}s",
        suite.count,
        start.elapsed().as_secs_f64()
    );
}

#[test]
fn typed_accessors_preserve_shared_native_values() {
    let start = Instant::now();
    let mut suite = witnesses::Suite::named("P2.7 shared captures");
    suite.source("export function main():void{let n:number=0;const f=():number=>{n++;return n;};const g=():number=>{try{n+=2;}finally{n++;}return n;};print(`${f()} ${g()} ${f()} ${n}`);}",Some("1 4 5 5\n"));
    suite.source("export function main():void{let s:string|null='x';const f=():void=>{s=null;};const g=():void=>{s='y';};print(s??'nil');f();print(s??'nil');g();print(s??'nil');}",Some("x\nnil\ny\n"));
    suite.source("export function main():void{let e:Error=new Error('x');const f=():void=>{e=new TypeError('y');};const g=():string=>e.message;print(g());f();print(g());}",Some("x\ny\n"));
    suite.source("export function main():void{let f:(n:number)=>number=(n:number):number=>n;const g=():number=>f(1);const h=():number=>f(2);print(`${g()} ${h()}`);f=(n:number):number=>n+10;print(`${g()} ${h()}`);}",Some("1 2\n11 12\n"));
    suite.source("export function main():void{const fs:(()=>number)[]=[];for(let i=0;i<3;i++){fs.push(():number=>{i++;return i;});fs.push(():number=>i);}const f0=fs[0],f1=fs[1],f2=fs[2],f3=fs[3];print(`${f0()} ${f1()} ${f2()} ${f3()}`);}",None);
    suite.source("export function main():void{let x:number=1;const f=():boolean=>{print('test');x=2;return false;};print(`${f()?x:x}`);const b:boolean=false,c:boolean=true;print(`${b?x:c?x:x}`);}",Some("test\n2\n2\n"));
    suite.source("export function main():void{const f=():number=>-0;const g=():number=>-0;const h=():number=>1e309;const j=():number=>1e309;print(`${1/f()} ${1/g()} ${h()} ${j()}`);}",Some("-Infinity -Infinity Infinity Infinity\n"));
    suite.source("export function main():void{const f=():number=>1;const g=():number|null=>1;const h=():number=>1;const j=():number|null=>1;print(`${f()} ${g()??0} ${h()} ${j()??0}`);}",Some("1 1 1 1\n"));
    suite.source("export function main():void{let n:number=0;const f=():number=>{n++;print(`${n}`);return n;};const xs=[[f()],[f()]];print(`${xs[0][0]} ${xs[1][0]}`);}",Some("1\n2\n1 2\n"));
    suite.finish();
    eprintln!(
        "P2.7 shared captures count={} cost={:.3}s",
        suite.count,
        start.elapsed().as_secs_f64()
    );
}
