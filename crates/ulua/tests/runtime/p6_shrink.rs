//! P6.S4 checks native equality and inline element checks through independent witnesses.
#[allow(dead_code)]
#[path = "../support/script.rs"]
mod support;
#[allow(dead_code)]
#[path = "../support/equality.rs"]
mod witness;
use std::time::Instant;

// Cost: one source through the ABI, node, tsc, and strict analyzer; Debug 0.40 seconds.
#[test]
fn native_equality_widens_singletons_and_string_refinements() {
    let start = Instant::now();
    let source = r#"
function narrow(s:string):boolean{if(s==="a"){return true;}return s==="a";}
export function main():void{print(`${true===false}:${false!=true}:${"a"=="b"}:${narrow("b")}`);}
"#;
    let mut suite = witness::Suite::named("P6 equality refinements");
    suite.source(source, Some("false:true:false:false\n"));
    suite.finish();
    eprintln!("P6 refinements cost={:.3}s", start.elapsed().as_secs_f64());
}

// Cost: one source through the ABI, node, tsc, and strict analyzer; Release about 0.38 seconds.
#[test]
fn native_equality_matches_primitive_and_reference_results() {
    let start = Instant::now();
    let source = r#"
function numbers(a:number,b:number):void{print(`${a===b}:${a!==b}:${a==b}:${a!=b}`);}
function strings(a:string,b:string):void{print(`${a===b}:${a!==b}:${a==b}:${a!=b}`);}
function booleans(a:boolean,b:boolean):void{print(`${a===b}:${a!==b}:${a==b}:${a!=b}`);}
type R={n:number};
function make():R|null{return {n:1};}
class Box{n:number;constructor(n:number){this.n=n;}}
export function main():void{
 numbers(0/0,0/0);numbers(-0,0);numbers(1/0,1/0);numbers(1/0,-1/0);
 strings("x","x");strings("x","y");booleans(false,false);booleans(false,true);
 let r:R|null=make();print(`${r===null}:${r!==null}:${null==r}:${null!=r}`);
 r=null;print(`${r===null}:${r!==null}:${null==r}:${null!=r}`);
 const a=[1];const same=a;const other=[1];print(`${a===same}:${a!==same}:${a==other}:${a!=other}`);
 const x:R={n:1};const y=x;const z:R={n:1};print(`${x===y}:${x!==y}:${x==z}:${x!=z}`);
 const b=new Box(1);const c=b;const d=new Box(1);print(`${b===c}:${b!==c}:${b==d}:${b!=d}`);
}
"#;
    let mut suite = witness::Suite::named("P6 native equality");
    let equal = "true:false:true:false\n";
    let unequal = "false:true:false:true\n";
    let identity = "true:false:false:true\n";
    let golden = [
        unequal, equal, equal, unequal, equal, unequal, equal, unequal, unequal, equal, identity,
        identity, identity,
    ]
    .concat();
    suite.source(source, Some(&golden));
    suite.finish();
    eprintln!("P6 equality cost={:.3}s", start.elapsed().as_secs_f64());
}

// Cost: one source through the ABI, node, tsc, and strict analyzer; Release about 0.38 seconds.
#[test]
fn inline_nil_checks_accept_non_optional_element_types() {
    let start = Instant::now();
    let source = r#"
type R={n:number};
function numberRead(a:number[],i:number):number{return a[i];}
function booleanRead(a:boolean[],i:number):boolean{return a[i];}
function stringRead(a:string[],i:number):string{return a[i];}
function recordRead(a:R[],i:number):R{return a[i];}
function write(a:number[],i:number):void{a[i]=30;}
export function main():void{
 const a=[10,20];write(a,2);
 print(`${numberRead(a,0)}:${numberRead(a,2)}:${booleanRead([false],0)}:${stringRead(["x"],0)}:${recordRead([{n:3}],0).n}`);
}
"#;
    let mut suite = witness::Suite::named("P6 inline nil checks");
    suite.source(source, Some("10:30:false:x:3\n"));
    suite.finish();
    eprintln!(
        "P6 inline strict cost={:.3}s",
        start.elapsed().as_secs_f64()
    );
}
