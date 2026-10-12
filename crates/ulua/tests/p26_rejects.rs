//! Rejection witnesses pin C13 path shapes and the class surface boundaries.
//! Measured Debug cost: 1.169 seconds; 143 rejects and one accept control in four parallel tests.
//! One tsc batch checks each diagnostic set. The test prints its cost without a time assertion.
#[allow(dead_code)]
#[path = "support/script.rs"]
mod support;
#[allow(dead_code)]
#[path = "support/p25.rs"]
mod witness;
#[allow(dead_code)]
#[path = "support/equality.rs"]
mod witnesses;
use std::time::Instant;

#[test]
fn constructor_paths_and_receiver_uses() {
    let start = Instant::now();
    let cases = [
        ("", &[2564][..]),
        ("constructor(b:boolean){if(b){this.v=1;}}", &[2564]),
        (
            "constructor(b:boolean){if(b){this.v=1;}else{print('x');}}",
            &[2564],
        ),
        ("constructor(b:boolean){while(b){this.v=1;break;}}", &[2564]),
        ("constructor(){for(let i=0;i<1;i++){this.v=1;}}", &[2564]),
        ("constructor(){for(const n of [1]){this.v=n;}}", &[2564]),
        ("constructor(b:boolean){if(b){return;}this.v=1;}", &[2564]),
        (
            "constructor(b:boolean){if(b){this.v=1;return;}else{return;}}",
            &[2564],
        ),
        ("constructor(){print(`${this.v}`);this.v=1;}", &[2565]),
        ("constructor(){this.read();this.v=1;}", &[]),
        ("constructor(){const self:C=this;this.v=1;}", &[]),
        ("constructor(){const xs:C[]=[this];this.v=1;}", &[]),
        ("constructor(){this.v=this.read();}", &[]),
        ("constructor(){this.v+=1;}", &[2564, 2565]),
        ("constructor(){this.v++;}", &[2564, 2565]),
        ("constructor(){const f=():number=>this.v;this.v=1;}", &[]),
        (
            "constructor(){const f=():()=>number=>():number=>this.v;this.v=1;}",
            &[],
        ),
        ("constructor(){const f=():void=>{this.v=1;};f();}", &[2564]),
        (
            "constructor(){while(true){if(true){break;}this.v=1;}}",
            &[2564],
        ),
        (
            "constructor(b:boolean){for(;b;this.v=1){continue;}}",
            &[2564],
        ),
    ];
    let programs: Vec<_> = cases.iter().map(|(member,codes)| (
        format!("class C{{v:number;{member}read():number{{return this.v;}}}}export function main():void{{}}"), *codes)).collect();
    let references: Vec<_> = programs
        .iter()
        .map(|(source, codes)| (source.as_str(), *codes))
        .collect();
    witness::rejected_programs("P2.6-construction", "C13", &references);
    let mut accepted = witnesses::Suite::named("P2.6 construction control");
    accepted.source(
        "class C{v:number;constructor(){while(true){this.v=1;break;}const f=():number=>this.v;}read():number{return this.v;}}export function main():void{print(`${new C().read()}`);}",
        Some("1\n"),
    );
    accepted.finish();
    for (field, codes) in [
        ("v:number=this.read();", &[][..]),
        ("v:()=>number=():number=>this.read();", &[][..]),
    ] {
        let source =
            format!("class C{{{field}read():number{{return 1;}}}}export function main():void{{}}");
        witness::rejected_programs("P2.6-initializer", "C13", &[(source.as_str(), codes)]);
    }
    eprintln!(
        "P2.6 C13 rejects={} accepted=1 cost={:.3}s",
        programs.len() + 2,
        start.elapsed().as_secs_f64()
    );
}

#[test]
fn private_members_and_closed_class_surface() {
    let start = Instant::now();
    let mut cases = Vec::new();
    for effect in [
        "print(`${c.v}`);",
        "c.v=2;",
        "c.v++;",
        "c.m();",
        "c.m=():number=>2;",
        "const f=c.m;",
        "n?.m();",
        "print(`${n?.v??0}`);",
    ] {
        cases.push((format!("class C{{private v:number=1;private m():number{{return this.v;}}}}export function main():void{{const c=new C();const n:C|null=c;{effect}}}"), vec![2341]));
    }
    for member in [
        "private constructor(){}",
        "public constructor(){}",
        "protected constructor(){}",
    ] {
        cases.push((
            format!("class C{{{member}}}export function main():void{{}}"),
            vec![],
        ));
    }
    for returned in ["this", "new C()", "1"] {
        cases.push((
            format!(
                "class C{{constructor(){{return {returned};}}}}export function main():void{{}}"
            ),
            vec![],
        ));
    }
    cases.push(("class C{v:number=1;m():number{function f():number{return this.v;}return f();}}export function main():void{}".into(),vec![2683]));
    for member in [
        "static v:number=1;",
        "readonly v:number=1;",
        "public v:number=1;",
        "get v():number{return 1;}",
        "set v(n:number){}",
        "#v:number=1;",
        "v?:number;",
        "v!:number;",
    ] {
        cases.push((
            format!("class C{{{member}}}export function main():void{{}}"),
            vec![],
        ));
    }
    let references: Vec<_> = cases
        .iter()
        .map(|(source, codes)| (source.as_str(), codes.as_slice()))
        .collect();
    witness::rejected_programs("P2.6-surface", "C11", &references);
    let type_names: Vec<_> = [
        "number", "boolean", "string", "symbol", "bigint", "any", "unknown", "never", "object",
    ]
    .iter()
    .map(|name| format!("class {name}{{}}export function main():void{{}}"))
    .collect();
    let type_refs: Vec<_> = type_names
        .iter()
        .map(|s| (s.as_str(), &[2414][..]))
        .collect();
    witness::rejected_programs("P2.6-type-names", "syntax", &type_refs);
    let proto = [
        (
            "class C{__proto__:number=1;}export function main():void{}",
            &[][..],
        ),
        (
            "class C{__proto__():number{return 1;}}export function main():void{}",
            &[][..],
        ),
    ];
    witness::rejected_programs("P2.6-proto", "C1", &proto);
    let mut inferred_cases = vec![];
    let mut nullable_arrays = vec![];
    for field in ["b:boolean=true;", "n:number|null=1;"] {
        let name = if field.starts_with('b') { "b" } else { "n" };
        for inferred in [
            format!("let v=this.{name};"),
            format!("const xs=[this.{name}];"),
            format!("const f=()=>this.{name};"),
            format!("const r={{v:this.{name}}};"),
        ] {
            let source =
                format!("class C{{{field}m():void{{{inferred}}}}}export function main():void{{}}");
            assert_eq!(
                ulua_compiler::compile(source.as_bytes(), ulua_compiler::Limits::default())
                    .unwrap_err()
                    .code,
                if name == "n" && inferred.starts_with("const xs=") {
                    "C6"
                } else {
                    "C11"
                },
                "{source}"
            );
            if name == "n" && inferred.starts_with("const xs=") {
                nullable_arrays.push(source);
            } else {
                inferred_cases.push(source);
            }
        }
    }
    let refs: Vec<_> = inferred_cases
        .iter()
        .map(|s| (s.as_str(), &[][..]))
        .collect();
    witness::rejected_programs("P2.6-this-inference", "C11", &refs);
    let refs: Vec<_> = nullable_arrays
        .iter()
        .map(|s| (s.as_str(), &[][..]))
        .collect();
    witness::rejected_programs("P2.6-this-array", "C6", &refs);
    let fields = [(
        "class C{v:number|null=1;m():number{if(this.v!=null){return this.v;}return 0;}}export function main():void{}",
        &[][..],
    )];
    witness::rejected_programs("P2.6-field-narrowing", "C16", &fields);
    eprintln!(
        "P2.6 surface rejects={} cost={:.3}s",
        cases.len() + type_names.len() + 11,
        start.elapsed().as_secs_f64()
    );
}

#[test]
fn inferred_receivers_reject_polymorphic_this() {
    let start = Instant::now();
    let reviewer = [
        ("const u=this;let t=u;t=new A();", &[2322][..]),
        ("let t=this;t=new A();", &[2322][..]),
        ("const xs=[this];xs.push(new A());", &[2345]),
        ("const f=()=>this;let g=f;g=()=>new A();", &[2322]),
        ("let t=this;if(o.n>0){t=o;}", &[2322]),
        ("let xs=[this,this];xs=[new A()];", &[2322]),
        ("[this].indexOf(new A());", &[2345]),
        ("const g=()=>this;const xs=[g];", &[]),
    ];
    let mut programs: Vec<_> = reviewer
        .iter()
        .map(|(body, codes)| {
            (
                format!(
                    "class A{{n:number=0;m(o:A):void{{{body}}}}}export function main():void{{}}"
                ),
                *codes,
            )
        })
        .collect();
    for form in [
        "this",
        "(this)",
        "c?this:o",
        "c?o:this",
        "maybe??this",
        "!this",
        "this&&o",
        "o||this",
    ] {
        for statement in [
            format!("const u={form};"),
            format!("let t={form};"),
            format!("const xs=[{form}];"),
            format!("const f=()=>{form};"),
            format!("const r={{v:{form}}};"),
        ] {
            programs.push((format!("class A{{n:number=0;m(o:A,c:boolean,maybe:A|null):void{{{statement}}}}}export function main():void{{}}"),&[]));
        }
    }
    let refs: Vec<_> = programs
        .iter()
        .map(|(source, codes)| (source.as_str(), *codes))
        .collect();
    witness::rejected_programs("P2.6-review-this", "C11", &refs);
    eprintln!(
        "P2.6 review this rejects={} cost={:.3}s",
        programs.len(),
        start.elapsed().as_secs_f64()
    );
}

#[test]
fn optional_calls_are_not_arrow_expression_bodies() {
    let start = Instant::now();
    let mut programs = vec![];
    for setup in [
        "class D{m():number{return 1;}}type R={d:D|null};const r:R={d:new D()};",
        "type D={m:()=>number};type R={d:D|null};const r:R={d:{m:():number=>1}};",
        "class D{m():void{}}type R={d:D|null};const r:R={d:new D()};",
        "type D={m:()=>void};type R={d:D|null};const r:R={d:{m:():void=>{}}};",
        "type O={f:(n:number)=>number};const o:O|null={f:(n:number):number=>n};",
        "type O={f:(n:number)=>void};const o:O|null={f:(n:number):void=>{}};",
    ] {
        for body in [
            "const f=():void=>r.d?.m();",
            "run(():void=>r.d?.m());",
            "let f=()=>r.d?.m();f=noop;",
            "const f=()=>r.d?.m();const xs=[f];xs.push(():void=>{});",
            "const f=():void=>((r.d?.m()));",
        ] {
            let codes: &[u32] = if setup.contains("m():number")
                || setup.contains("m:()=>number")
                || setup.contains("f:(n:number)=>number")
            {
                if body.contains("xs.push") {
                    &[2345]
                } else {
                    &[2322]
                }
            } else {
                &[]
            };
            let body = if setup.starts_with("type O=") {
                body.replace("r.d?.m()", "o?.f(1)")
            } else {
                body.to_owned()
            };
            programs.push((format!("{setup}function run(f:()=>void):void{{f();}}function noop():void{{}}export function main():void{{{body}}}"),codes));
        }
    }
    let refs: Vec<_> = programs
        .iter()
        .map(|(source, codes)| (source.as_str(), *codes))
        .collect();
    witness::rejected_programs("P2.6-review-optional", "C2", &refs);
    eprintln!(
        "P2.6 review optional rejects={} cost={:.3}s",
        programs.len(),
        start.elapsed().as_secs_f64()
    );
}
