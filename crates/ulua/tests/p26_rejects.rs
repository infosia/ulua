//! Rejection witnesses pin C13 path shapes and the class surface boundaries.
//! One tsc batch checks each diagnostic set. The test prints its cost without a time assertion.
#[allow(dead_code)]
#[path = "support/p25.rs"]
mod witness;
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
        ("constructor(){const self=this;this.v=1;}", &[]),
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
        (
            "constructor(){while(true){this.v=1;break;}const f=():number=>this.v;}",
            &[],
        ),
    ];
    // The last shape is an accept control, separate from the C13 rejects.
    let programs: Vec<_> = cases[..cases.len()-1].iter().map(|(member,codes)| (
        format!("class C{{v:number;{member}read():number{{return this.v;}}}}export function main():void{{}}"), *codes)).collect();
    let references: Vec<_> = programs
        .iter()
        .map(|(source, codes)| (source.as_str(), *codes))
        .collect();
    witness::rejected_programs("P2.6-construction", "C13", &references);
    for (field, codes) in [
        ("v:number=this.read();", &[][..]),
        ("v:()=>number=():number=>this.read();", &[][..]),
    ] {
        let source =
            format!("class C{{{field}read():number{{return 1;}}}}export function main():void{{}}");
        witness::rejected_programs("P2.6-initializer", "C13", &[(source.as_str(), codes)]);
    }
    eprintln!(
        "P2.6 C13 rejects={} cost={:.3}s",
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
