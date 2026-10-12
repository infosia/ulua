//! P2.6 direct witnesses cover construction, receiver capture, nominal flow, and identity.
//! Single-worker Debug cost: 0.46 seconds for this test target.
//! The test prints its total cost. No assertion reads wall-clock time.
#[allow(dead_code)]
#[path = "support/script.rs"]
mod support;
#[allow(dead_code)]
#[path = "support/equality.rs"]
mod witnesses;
use std::time::Instant;

#[test]
fn classes_match_node_and_both_type_checkers() {
    let start = Instant::now();
    let mut suite = witnesses::Suite::named("P2.6 direct");
    suite.source(r#"
function trace(s:string,n:number):number{print(s);return n;}
class C {
 a:number=trace('a',1); b:number; c:number=trace('c',3);
 constructor(x:number=trace('default',5),y:number=trace('second default',6)) {
  print('body'); if(x==0){this.b=y;return;}else{this.b=x+y;}
 }
 sum():number{return this.a+this.b+this.c;}
}
class Plain {n:number=trace('plain',7);}
export function main():void{
 const c=new C(trace('x',10),trace('y',20));print(`${c.sum()}`);
 print(`${new C().sum()}`);print(`${new C(0).sum()}`);
 print(`${new Plain().n+new Plain().n}`);new Plain();
}
"#, Some("x\ny\na\nc\nbody\n34\na\nc\ndefault\nsecond default\nbody\n15\na\nc\nsecond default\nbody\n10\nplain\nplain\n14\nplain\n"));
    suite.source(
        r#"
class Acc {
 private value:number; private factor:number=2;
 constructor(n:number){this.value=n;}
 private add(n:number=1):number{this.value+=n;return this.value;}
 apply(n:number):Acc{this.add(n*this.factor);return this;}
 nested():()=>()=>number{return ():()=>number=>():number=>this.add();}
 store(xs:Acc[]):void{xs.push(this);}
 pass(f:(a:Acc)=>Acc):Acc{return f(this);}
 recurse(n:number):number{if(n==0){return this.value;}return this.recurse(n-1);}
}
type Holder={a:Acc;f:(a:Acc)=>Acc};
function same(a:Acc,b:Acc):boolean{return a===b;}
export function main():void{
 const a=new Acc(1);const xs:Acc[]=[];a.store(xs);
 const h:Holder={a:a,f:(x:Acc):Acc=>x};const b=a.pass(h.f).apply(3);
 const f=a.nested();const g=f();print(`${g()} ${a.recurse(5)}`);
 print(`${same(a,b)} ${a===xs[0]} ${a===h.a} ${a!==new Acc(8)}`);
 let n:Acc|null=a;n?.apply(1);n=null;n?.apply(100);
 n=a;if(n!=null){print(`${n.recurse(0)}`);}
}
"#,
        Some("8 8\ntrue true true true\n10\n"),
    );
    suite.source(
        r#"
class K {v:number;constructor(v:number){this.v=v;}m(n:number):number{return this.v+n;}}
let k:K=new K(10);
function replace():number{k=new K(20);return 1;}
function receiver():K{print('receiver');return k;}
function argument(n:number):number{print('argument');return n;}
export function main():void{
 print(`${k.m(replace())}`);print(`${receiver().m(argument(2))}`);
 let x:K|null=null;x?.m(argument(9));x=k;x?.m(argument(3));
}
"#,
        Some("11\nreceiver\nargument\n22\nargument\n"),
    );
    suite.source(
        r#"
class Keys {new:number|null=null;end:number=1;__index:number=2;
 __eq(other:Keys):boolean{return this.end==other.end;}
 get():number{return this.new??5;}
}
export function main():void{
 const a=new Keys();const b=new Keys();a.end++;a.end-=1;a.new=3;
 print(`${a===b} ${a.__eq(b)} ${a.get()} ${a.__index}`);
}
"#,
        Some("false true 3 2\n"),
    );
    suite.source(
        r#"
class Loop {v:number;constructor(n:number){while(true){if(n>0){this.v=n;break;}this.v=0;break;}}
 read():number{return this.v;}}
class Arrow {v:number;read:()=>number=():number=>7;
 constructor(n:number){this.v=n;const a=():()=>number=>():number=>this.v;this.read=a();}
 bump():void{const f=(n:number):void=>this.set(n);f(this.v+1);}
 set(n:number):void{this.v=n;}}
export function main():void{const a=new Arrow(new Loop(5).read());a.bump();print(`${a.read()}`);}
"#,
        Some("6\n"),
    );
    suite.source(
        r#"
class Flags {flag:boolean=true;n:number|null=1;
 read():number{const b:boolean=this.flag;const xs:boolean[]=[this.flag];
 const f=():boolean=>this.flag;const n:number|null=this.n;
 if(this.n!=null){print(`${this.n??0}`);}return b&&xs[0]&&f()?n??0:0;}}
export function main():void{print(`${new Flags().read()}`);}
"#,
        Some("1\n1\n"),
    );
    suite.source(
        r#"
class Private {
 private v:number=1;private f:()=>number=():number=>3;
 private m():number{return this.v;}
 other(o:Private):number{function inner(p:Private):number{p.v+=1;return p.m();}
  const arrow=():number=>o.m();const n:Private|null=o;n?.m();n?.f();return inner(o)+arrow();}
}
export function main():void{const a=new Private();const b=new Private();print(`${a.other(b)}`);}
"#,
        Some("4\n"),
    );
    suite.source(
        r#"
class Paths {a:number;b:number;constructor(n:number){if(n>0){this.a=n;this.b=1;return;}
 for(;;){this.a=0;this.b=2;break;}}sum():number{return this.a+this.b;}}
export function main():void{print(`${new Paths(3).sum()} ${new Paths(0).sum()}`);}
"#,
        Some("4 2\n"),
    );
    suite.source(r#"
type Holder={value:A};
class A{n:number=0;
 check(o:A):void{const u:A=this;let t:A=this;t=o;
  const xs:A[]=[this];xs.push(o);const f=():A=>this;
  const contextual:()=>A=()=>this;const r:Holder={value:this};
  print(`${u===this} ${t===o} ${xs[0]===this} ${f()===this} ${contextual()===this} ${r.value===this} ${this===o}`);
 }
 m():number{this.n++;return this.n;}}
type Optional={f:()=>number};
export function main():void{const a=new A();a.check(new A());
 const d:A|null=a;const f=():void=>{d?.m();};f();
 const o:Optional|null={f:():number=>2};const g=():void=>{o?.f();};g();
 const h=():void=>a.check(a);h();print(`${a.n}`);}
"#,Some("true true true true true true false\ntrue true true true true true true\n1\n"));
    suite.finish();
    eprintln!(
        "P2.6 direct accepted={} cost={:.3}s",
        suite.count,
        start.elapsed().as_secs_f64()
    );
}
