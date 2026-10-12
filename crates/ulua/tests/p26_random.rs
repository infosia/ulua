//! Fixed-seed class programs exercise the accepted surface against three external witnesses.
//! The test prints fixture and witness costs. No assertion reads elapsed time.
#[allow(dead_code)]
#[path = "support/script.rs"]
mod support;
#[allow(dead_code)]
#[path = "support/equality.rs"]
mod witness;
use std::time::Instant;

struct Random(u64);
impl Random {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn number(&mut self) -> u64 {
        self.next() % 19
    }
}

#[test]
fn random_class_surface_matches_node() {
    let start = Instant::now();
    let mut random = Random(0x26c1_13a5_7e91);
    let mut suite = witness::Suite::named("P2.6 random");
    for i in 0..160 {
        let n = random.number();
        let step = random.number();
        let branch = random.next().is_multiple_of(2);
        let initializer = random.next().is_multiple_of(2);
        let field = if initializer {
            format!("v:number={n};")
        } else {
            "v:number;".into()
        };
        let constructor = if initializer {
            "constructor(n:number){this.v+=n;}".to_owned()
        } else {
            format!("constructor(n:number){{if({branch}){{this.v=n;}}else{{this.v=n+1;}}}}")
        };
        let update = if random.next().is_multiple_of(2) {
            "this.v+=n;"
        } else {
            "this.v=this.v+n;"
        };
        let private = if random.next().is_multiple_of(2) {
            "private "
        } else {
            ""
        };
        let declarations = format!(
            r#"class C{{{private}{field}{constructor}
 flag:boolean={branch};maybe:C|null=null;label:string='s{i}';
 add(n:number={step}):number{{{update}return this.v;}}
 self():C{{return this;}}
 arrow():()=>()=>number{{return ():()=>number=>():number=>this.add();}}
 recurse(n:number):number{{if(n==0){{return this.v;}}return this.recurse(n-1);}}
 other(o:C):number{{const f=():number=>o.v;return f();}}
 get():number{{return this.v;}}
}}
type R={{item:C;f:(c:C)=>C}};
function identity(c:C):C{{return c;}}
"#
        );
        let body = match random.next() % 8 {
            0 => format!("const c=new C({n});print(`${{c.add({step})}} ${{c.self()===c}}`);"),
            1 => format!(
                "const c=new C({n});const f=c.arrow();const g=f();print(`${{g()}} ${{g()}}`);"
            ),
            2 => format!(
                "const xs:C[]=[new C({n}),new C({step})];const ys=xs.map((c:C):C=>c.self());ys.forEach((c:C):void=>{{c.add({step});}});print(`${{ys[0].get()}} ${{xs[1].get()}} ${{xs[0]===ys[0]}}`);"
            ),
            3 => format!(
                "const c=new C({n});let x:C|null={};x?.add({step});x=c;x?.add();print(`${{c.get()}}`);x=null;x?.add(100);",
                if branch { "null" } else { "c" }
            ),
            4 => format!(
                "const c=new C({n});const r:R={{item:c,f:identity}};print(`${{r.f(r.item).add({step})}} ${{r.item===c}}`);"
            ),
            5 => format!(
                "const c=new C({n});c.maybe=new C({step});const d:C=c.maybe??c;print(`${{c.other(d)}} ${{c.recurse({step})}} ${{c===d}}`);"
            ),
            6 => format!(
                "const c=new C({n});const flags:boolean[]=[c.flag];const f=():boolean=>c.flag;c.flag=!c.flag;print(`${{flags[0]}} ${{f()}} ${{c.label}}`);"
            ),
            _ => format!(
                "const c=new C({n});const f=(x:C):C=>x.self();const d:C=f(c);for(let k=0;k<3;k++){{d.add({step});}}print(`${{c.get()}} ${{c===d}}`);"
            ),
        };
        suite.source(
            &format!("{declarations}export function main():void{{{body}}}"),
            None,
        );
    }
    let fixtures = start.elapsed().as_secs_f64();
    suite.finish();
    eprintln!(
        "P2.6 random seed=0x26c113a57e91 generated=160 accepted={} fixtures={fixtures:.3}s witnesses={:.3}s total={:.3}s",
        suite.count,
        start.elapsed().as_secs_f64() - fixtures,
        start.elapsed().as_secs_f64()
    );
}
