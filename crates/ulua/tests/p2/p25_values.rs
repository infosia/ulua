//! P2.5 direct and parameterized template programs use Node, tsc, Luau, and the ABI.
//! Single-worker Debug cost: 1.80 seconds for this test target.
#[path = "../support/p25.rs"]
mod field_witness;
#[allow(dead_code)]
#[path = "../support/script.rs"]
mod support;
#[allow(dead_code)]
#[path = "../support/equality.rs"]
mod witness;
use ulua_compiler::{Limits, compile};
use witness::Suite;
const TYPES: &str = "type Box={value:number};type Link={value:number;next:Link|null};type Wrap={box:Box;label:string|null};";
fn source(body: &str) -> String {
    format!("{TYPES}export function main():void{{{body}}}")
}
#[test]
fn records_null_tests_and_flow_match_all_witnesses() {
    let mut s = Suite::named("P2.5 direct");
    for body in [
        "const x:Box={value:1};const y=x;const z:Box={value:1};y.value=3;print(`${x.value}:${x===y}:${x!==z}`);",
        "const x:Box={value:1};const xs:Box[]=[x,{value:2}];xs[0].value+=4;const ys=xs.slice();ys[0].value*=2;print(`${x.value}:${xs[0]===ys[0]}:${xs[1]!==xs[0]}`);",
        "const x:Wrap={box:{value:1},label:null};x.label='set';x.box.value=4;const y=x.box;print(`${y.value}:${x.label??'none'}`);x.label=null;print(x.label??'none');",
        "const xs:Link[]=[{value:1,next:{value:2,next:null}}];const next=xs[0].next;if(next!=null){next.value=9;print(`${next.value}`);}print(`${xs[0].next?.value??0}`);",
        "let x:Box|null={value:2};const f=(x:Box|null):void=>{x=null;};if(x!=null){f(null);print(`${x.value}`);}",
        "let x:Box|null={value:2};const clear=():void=>{x=null;};if(x!=null){clear();print(`${x?.value??0}`);}",
        "let x:Box|null={value:2};const clear=():void=>{x=null;};if(x!=null){const y=x;clear();print(`${y?.value??0}`);}",
        "const x:Box|null={value:2};if(x!=null){print(`${x?.value??0}`);}",
        "let x:Box|null={value:2};if(x!=null){while(true){let x:Box|null=null;x=null;break;}print(`${x.value}`);}",
        "const x:Box|null={value:2};if(x!=null){const f=():number=>x.value;print(`${f()}`);}",
        "let x:Box|null={value:2};if(x!=null){x=null;}print(`${x?.value??0}`);",
        "let x:Box|null={value:2};if(x==null){return;}print(`${x.value}`);",
        "let x:Box|null={value:2};if(x!=null){}else{return;}print(`${x.value}`);",
        "const x:Box|null={value:2};const y:Box|null=null;print(`${x!=null&&x.value>0}:${y!=null&&y.value>0}:${x==null||x.value>0}`);",
        "const x:Box|null={value:2};print(`${!(x==null)?x.value:0}:${x==null?0:x.value}`);",
        "let x:Link|null={value:1,next:{value:2,next:null}};let sum=0;while(x!=null){sum+=x.value;x=x.next;}print(`${sum}:${x==null}`);",
        "const xs:Box[]=[{value:1},{value:2}];let sum=0;for(const x of xs){const y:Box|null=x;if(y==null){continue;}sum+=y.value;}print(`${sum}`);",
        "let x:Box|null={value:2};for(let i=0;i<2;i++){if(x==null){break;}print(`${x.value}`);x=null;}",
        "const x:Box|null={value:2};const a:Box|null=x;const f:(x:Box|null)=>number=(x:Box|null):number=>x?.value??0;print(`${f(a)}`);",
        "const x:number|null=0;const b:boolean|null=false;const t:string|null='';print(`${x??9}:${b??true}:${t??'fallback'}`);",
        "const x:Box|null=null;const b:Box|null={value:3};print(`${(x??b)?.value??0}`);",
        "const x:Box|null={value:2};let y:Box|null=x;let z:number=x?.value??0;y=null;print(`${z}:${y==null}`);",
        "const x:Box|null={value:2};if(false||x!=null){print(`${x.value}`);}if(true&&x!=null){print(`${x.value}`);}if(x==null||false){return;}print(`${x.value}`);",
        "let x:Box|null={value:2};while(true){if(x!=null){break;}x={value:3};}print(`${x.value}`);",
        "let x:Box|null={value:2};while(x==null){x={value:3};}print(`${x.value}`);",
        "let x:Box|null={value:2};for(;x==null;){x={value:3};}print(`${x.value}`);",
        "const x:Box|null={value:2};if(x!=null){while(true){let x:Box|null=null;x=null;break;}print(`${x.value}`);}",
    ] {
        s.source(&source(body), None);
    }
    for op in ["==", "!=", "===", "!=="] {
        for operand in ["x", "(x)", "get()", "wrap.box", "xs[0].next"] {
            for reverse in [false, true] {
                let test = if reverse {
                    format!("null{op}{operand}")
                } else {
                    format!("{operand}{op}null")
                };
                let program = format!(
                    "{TYPES}type Maybe={{box:Box|null}};function get():Box|null{{return {{value:3}};}}export function main():void{{const x:Box|null=get();const wrap:Maybe={{box:x}};const xs:Link[]=[{{value:1,next:null}}];print(`${{{test}}}`);if({test}){{print('yes');}}else{{print('no');}}print(`${{!({test})}}:${{{test}?1:2}}`);}}"
                );
                s.source(&program, None);
            }
        }
    }
    for program in [
        "type F={f:(n:number)=>number};const box:F={f:(n:number):number=>n+1};function arg():number{box.f=(n:number):number=>n+10;return 1;}export function main():void{print(`${box.f(arg())}:${box.f(1)}`);}",
        "type eval={v:number};type arguments={v:number};export function main():void{const a:eval={v:1};const b:arguments=a;print(`${b.v}`);}",
        "type F={new:(x:number)=>number;__eq:string;end:number};export function main():void{const x:F={new:(x:number):number=>x+1,__eq:'a',end:3};print(`${x.new(x.end)}:${x.__eq}`);}",
        "type A={x:number;y:string};type B={y:string;x:number};function take(a:A):A{return a;}export function main():void{const b:B={y:'ok',x:1};const a:A=take(b);const xs:A[]=[b];const ys:B[]=xs;print(`${a===b}:${ys[0]===b}`);}",
        "type A={v:B|null};type B={v:A|null};type C={v:C|null};function same(a:A,b:C):boolean{return a===b;}export function main():void{const a:A={v:null};const c:C=a;print(`${same(a,c)}`);}",
        "type A={next:A|null};type B={next:B|null};export function main():void{const a:A={next:null};const b:B=a;print(`${a===b}`);}",
        "type F={f:(n:number)=>void};let ticks=0;function arg():number{ticks+=1;return ticks;}function get(v:boolean):F|null{ticks+=1;return v?{f:(n:number):void=>{print(`${n}`);}}:null;}export function main():void{get(false)?.f(arg());get(true)?.f(arg());print(`${ticks}`);}",
        "let ticks=0;function make():number|null{ticks+=1;return 0;}function fallback():number{ticks+=10;return 9;}export function main():void{print(`${make()??fallback()}:${ticks}`);}",
        "type Box={value:number};const box:Box={value:1};let order='';function receiver():Box{order+='r';return box;}function value():number{order+='v';box.value=9;return 2;}export function main():void{receiver().value+=value();print(`${order}:${box.value}`);order='';receiver().value=value();print(`${order}:${box.value}`);}",
        "type Box={a:number;b:number};let order='';function value(n:number):number{order+=n;return n;}export function main():void{const box:Box={b:value(2),a:value(1)};print(`${order}:${box.a}:${box.b}`);}",
        "function get():string|null{return 'abc';}export function main():void{const s=get();print(`${s?.length??0}`);s?.slice(0);}",
        "function get():number[]|null{return [1];}export function main():void{const xs=get();xs?.push(2);xs?.reverse();print(`${xs?.length??0}`);}",
        "function get():((x:number)=>number)|null{return (x:number):number=>x+1;}export function main():void{const f=get();if(f!=null){print(`${f(3)}`);}const g:(x:number)=>number=f??((x:number):number=>x);print(`${g(1)}`);}",
    ] {
        s.source(program, None);
    }
    s.finish();
}
#[test]
fn narrowing_limits_and_closed_record_forms_reject() {
    let mut count = 0;
    for (body, code) in [
        (
            "let x:Box|null={value:1};if(x!=null){x=null;print(`${x.value}`);}",
            "C16",
        ),
        (
            "let x:Box|null={value:1};const f=():void=>{x=null;};if(x!=null){print(`${x.value}`);}",
            "C16",
        ),
        (
            "let x:Box|null={value:1};if(x!=null){const f=():number=>x.value;}",
            "C16",
        ),
        (
            "let x:Box|null={value:1};if(x!=null){function f():number{return x.value;}}",
            "C16",
        ),
        (
            "const w:Wrap={box:{value:1},label:'a'};if(w.label!=null){print(w.label+'b');}",
            "C16",
        ),
        (
            "const x:Link={value:1,next:null};const xs:Link[]=[x];if(xs[0].next!=null){print(`${xs[0].next.value}`);}",
            "C16",
        ),
        (
            "let x:Box|null={value:1};if(x!=null){}print(`${x.value}`);",
            "C16",
        ),
        (
            "let x:Box|null={value:1};if(x!=null){if(true){x=null;}print(`${x.value}`);}",
            "C16",
        ),
        (
            "let x:Box|null={value:1};if(x!=null){while(true){print(`${x.value}`);x=null;break;}}",
            "C16",
        ),
        ("const x:Box={value:1};print(`${x==null}`);", "C2"),
        ("const x:Box={value:1};print(`${x??x}`);", "C2"),
        (
            "const x:Box={value:1};const y={value:1,extra:2};const z:Box=y;",
            "C1",
        ),
        ("const x:Box={};", "C1"),
        ("const x:Box={value:1};print(`${x==={value:1}}`);", "C7"),
        ("const x:Box={value:1};const y:Box={value:true};", "C1"),
        (
            "const x:Box|null={value:1};if(x!=null){let y=x;y=null;}",
            "C11",
        ),
        (
            "const x:Box|null={value:1};if(x!=null){let y=x??{value:2};}",
            "C11",
        ),
        (
            "const x:Box|null={value:1};if(x!=null){const y=[x];}",
            "C11",
        ),
        (
            "const x:Box|null={value:1};if(x!=null){const y=()=>x;}",
            "C11",
        ),
        ("const x:Box|null={value:1};const y=x?.value;", "C2"),
        ("const x:Link|null=null;print(`${x?.next.value??0}`);", "C2"),
        ("const x=[null];", "C6"),
    ] {
        let program = source(body);
        assert_eq!(
            compile(program.as_bytes(), Limits::default())
                .unwrap_err()
                .code,
            code,
            "{program}"
        );
        count += 1;
    }
    for program in [
        "type Box={value:number};export function main():void{const x:Box|null={value:1};if(x!=null){function f():number{return x.value;}print(`${f()}`);}}",
        "type Box={value:number};let x:Box|null={value:1};export function main():void{if(x!=null){print(`${x.value}`);}}",
        "type Box={value:number};function f(x:Box|null):void{const clear=():void=>{x=null;};if(x!=null){print(`${x.value}`);}}export function main():void{}",
        "type Box={value:number};export function main():void{const x:Box|null=null;function read():number{return x.value;}print(`${read()}`);if(x==null){return;}}",
    ] {
        assert_eq!(
            compile(program.as_bytes(), Limits::default())
                .unwrap_err()
                .code,
            "C16",
            "{program}"
        );
        count += 1;
    }
    for name in [
        "number",
        "boolean",
        "string",
        "symbol",
        "bigint",
        "void",
        "any",
        "unknown",
        "never",
        "object",
        "undefined",
        "await",
        "yield",
        "let",
        "static",
        "implements",
        "interface",
        "package",
        "private",
        "protected",
        "public",
    ] {
        let program = format!("type {name}={{value:number}};export function main():void{{}}");
        let code = compile(program.as_bytes(), Limits::default())
            .err()
            .unwrap_or_else(|| panic!("Expected rejection: {program}"))
            .code;
        assert!(matches!(code, "syntax" | "C11" | "C2"), "{program}: {code}");
        count += 1;
    }
    let diagnostic = compile(
        source("const a:Box={value:1};const b:Link={value:1,next:null};print(`${a===b}`);")
            .as_bytes(),
        Limits::default(),
    )
    .unwrap_err();
    assert_eq!(diagnostic.code, "C7");
    assert!(diagnostic.message.contains("object types"));
    count += 1;
    eprintln!("P2.5 direct rejects={count}");
}
#[test]
fn template_records_and_nullable_values_match_node() {
    let mut s = Suite::named("P2.5 templates");
    let mut seed = 0x25c16u64;
    let mut rejected = 0;
    let mut reject_codes = std::collections::BTreeMap::new();
    for i in 0..240 {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        let n = (seed >> 32) % 31;
        let present = seed & 1 == 0;
        let value = if present {
            format!("{{value:{n}}}")
        } else {
            "null".into()
        };
        let op = ["==", "===", "!=", "!=="][(seed as usize >> 3) % 4];
        let test = if seed & 4 == 0 {
            format!("x{op}null")
        } else {
            format!("null{op}x")
        };
        let use_value = if op == "==" || op == "===" {
            format!("({test})?0:x.value")
        } else {
            format!("({test})?x.value:0")
        };
        let body = match i % 6 {
            0 => format!(
                "const x:Box|null={value};const y:Box={{value:{n}+1}};const z=x??y;z.value+=2;print(`${{{use_value}}}:${{z.value}}:${{z===y}}`);"
            ),
            1 => format!(
                "const x:Wrap={{box:{{value:{n}}},label:null}};const ys:Wrap[]=[x,x];ys[1].box.value+=3;const v=ys[0].label;if(v!=null){{print(v);}}else{{print(`${{x.box.value}}:${{ys[0]===ys[1]}}`);}}"
            ),
            2 => format!(
                "let x:Link|null={{value:{n},next:{{value:{n}+1,next:null}}}};let total=0;while(x!==null){{total+=x.value;x=x.next;}}print(`${{total}}:${{x===null}}`);"
            ),
            3 => format!(
                "const x:Box|null={value};const f=():number=>x?.value??{n}+1;print(`${{f()}}:${{{use_value}}}`);"
            ),
            4 => format!(
                "const x:number|null={};const b:boolean|null={};print(`${{x??{n}}}:${{b??true}}`);",
                if present { "0" } else { "null" },
                if present { "false" } else { "null" }
            ),
            _ => format!(
                "const x:Box|null={value};let y:Box|null=x;y=null;print(`${{x?.value??{n}+1}}:${{y==null}}`);"
            ),
        };
        s.source(&format!("// P2.5 template\n{}", source(&body)), None);
        let (invalid, code) = match i % 6 {
            0 => (
                format!(
                    "const x:Wrap={{box:{{value:{n}}},label:'x'}};if(x.label!=null){{print(x.label+'y');}}"
                ),
                "C16",
            ),
            1 => (
                format!(
                    "let x:Box|null={{value:{n}}};const clear=():void=>{{x=null;}};if(x!=null){{print(`${{x.value}}`);}}"
                ),
                "C16",
            ),
            2 => (
                format!("let x:Box|null={{value:{n}}};if(x!=null){{const f=():number=>x.value;}}"),
                "C16",
            ),
            3 => (
                format!("const x:Box|null={{value:{n}}};if(x!=null){{let y=x;y=null;}}"),
                "C11",
            ),
            4 => (format!("const x:Box={{value:{n},extra:1}};"), "C1"),
            _ => (format!("const xs=[{n},null];"), "C6"),
        };
        let invalid = source(&invalid);
        let result = compile(invalid.as_bytes(), Limits::default()).unwrap_err();
        assert_eq!(result.code, code, "{invalid}");
        *reject_codes.entry(result.code).or_insert(0usize) += 1;
        rejected += 1;
    }
    s.finish();
    eprintln!("P2.5 template reject codes: {reject_codes:?}");
    eprintln!(
        "P2.5 templates generated={} accepted={} rejected={rejected}",
        s.count + rejected,
        s.count
    );
}

#[test]
fn proto_fields_reject_in_types_and_literal_keys() {
    for key in [
        "__proto__",
        "\"__proto__\"",
        "'__proto__'",
        r"__pr\u006fto__",
        r"'__pr\u006fto__'",
    ] {
        for program in [
            format!("type T={{{key}:number}};export function main():void{{}}"),
            format!("export function main():void{{const t:{{{key}:number}}={{{key}:1}};}}"),
            format!("function read(t:{{{key}:number}}):void{{}}export function main():void{{}}"),
            format!("export function main():void{{const t={{{key}:1}};}}"),
        ] {
            let diagnostic = compile(program.as_bytes(), Limits::default()).unwrap_err();
            assert_eq!(diagnostic.code, "C1", "{program}: {diagnostic:?}");
            assert!(diagnostic.message.contains("__proto__"), "{diagnostic:?}");
        }
    }
    for program in [
        "export function main():void{const __proto__=1;const t={__proto__};}",
        r"export function main():void{const __proto__=1;const t={__pr\u006fto__};}",
        include_str!("../../../../corpus/reject/r102-proto-field.ts"),
    ] {
        assert_eq!(
            compile(program.as_bytes(), Limits::default())
                .unwrap_err()
                .code,
            "C1",
            "{program}"
        );
    }
}

#[test]
fn const_narrowing_continues_into_arrows_only() {
    let declaration =
        include_str!("../../../../corpus/reject/r101-const-narrowing-function-declaration.ts");
    assert_eq!(
        compile(declaration.as_bytes(), Limits::default())
            .unwrap_err()
            .code,
        "C16"
    );
    let arrow = declaration.replace(
        "function inner(): number {\n    return x.value;\n  }",
        "const inner = (): number => {\n    return x.value;\n  };",
    );
    assert_ne!(
        arrow, declaration,
        "The paired fixture must replace the declaration"
    );
    let mut suite = Suite::named("P2.5 const arrow");
    suite.source(&arrow, Some("3\n"));
    suite.finish();
}

#[test]
fn nullable_flow_joins_every_loop_edge() {
    assert_eq!(
        compile(
            include_bytes!("../../../../corpus/reject/r103-for-update-after-continue.ts"),
            Limits::default()
        )
        .unwrap_err()
        .code,
        "C16"
    );
    for body in [
        "const x:Box|null=null;let s=0;for(let i=0;i<2;s+=x.value){i+=1;if(x==null){continue;}}",
        "const x:Box|null=null;let s=0;for(let i=0;i<2;s+=x.value){i+=1;{if(x==null){if(i>0){continue;}else{return;}}}}",
        "const x:Box|null=null;let s=0;for(let i=0;i<2;s+=x.value){i+=1;if(x!=null){continue;}if(x==null){continue;}}",
        "let x:Box|null={value:1};let s=0;for(let i=0;i<2;s+=x.value){i+=1;if(x==null){return;}if(i==1){x=null;continue;}}",
        "const x:Box|null=null;let s=0;for(let i=0;i<2;s+=x.value){i+=1;if(x==null){continue;}else{break;}}",
        "const x:Box|null=null;let s=0;for(let i=0;i<2;s+=x.value){i+=1;if(x==null){continue;}else{return;}}",
        "let x:Box|null={value:1};if(x!=null){while(x.value>0){x=null;continue;}}",
        "let x:Box|null={value:1};if(x!=null){for(let i=0;x.value>0;i+=1){x=null;continue;}}",
        "const x:Box|null=null;for(let i=0;i<1;i+=1){if(x!=null){continue;}}print(`${x.value}`);",
        "const x:Box|null=null;while(false){if(x!=null){continue;}}print(`${x.value}`);",
        "const x:Box|null=null;for(const n of [1]){if(x!=null){continue;}}print(`${x.value}`);",
        "const x:Box|null=null;let s=0;for(let i=0;i<1;s+=x.value){i+=1;for(let j=0;j<1;j+=1){if(x==null){break;}}}",
        "const x:Box|null=null;const y:Box|null={value:1};if((x!=null?true:y!=null)){print(`${x.value}`);}",
        "const x:Box|null=null;const y:Box|null={value:1};if((x!=null&&true)||(y!=null&&true)){print(`${x.value}`);}",
        "let x:Box|null={value:1};let i=0;while(true){if(i==0){if(x==null){return;}break;}if(i==1){x=null;break;}if(x==null){return;}break;}print(`${x.value}`);",
        "let x:Box|null={value:1};let s=0;for(let i=0;i<3;s+=x.value){i+=1;if(i==1){if(x==null){return;}continue;}if(i==2){x=null;continue;}if(x==null){return;}}",
    ] {
        let program = source(body);
        assert_eq!(
            compile(program.as_bytes(), Limits::default())
                .unwrap_err()
                .code,
            "C16",
            "{program}"
        );
    }
    for body in [
        "label:for(let i=0;i<1;i+=1){continue label;}",
        "for(let i=0;i<1;i+=1){switch(i){case 0:continue;}}",
        "do{continue;}while(false);",
    ] {
        assert_eq!(
            compile(source(body).as_bytes(), Limits::default())
                .unwrap_err()
                .code,
            "C8",
            "{body}"
        );
    }
    let mut suite = Suite::named("P2.5 flow joins");
    for body in [
        "const x:Box|null={value:2};let s=0;for(let i=0;i<2;s+=x.value){i+=1;if(x==null){break;}if(i==1){continue;}}print(`${s}`);",
        "const x:Box|null={value:2};let s=0;for(let i=0;i<2;s+=x.value){i+=1;if(x==null){return;}if(i==1){continue;}}print(`${s}`);",
        "const x:Box|null={value:2};let s=0;for(let i=0;i<2;s+=x.value){i+=1;for(let j=0;j<1;j+=1){continue;}if(x==null){break;}}print(`${s}`);",
        "const x:Box|null={value:2};let s=0;for(let i=0;i<2;s+=x.value){i+=1;for(const j of [1]){continue;}if(x==null){return;}}print(`${s}`);",
        "const x:Box|null={value:2};let s=0;for(let i=0;i<2;s+=x.value){i+=1;if(x==null){break;}if(i==1){if(x!=null){continue;}}else{if(x!=null){continue;}}}print(`${s}`);",
        "const x:Box|null={value:2};let s=0;for(let i=0;i<2;s+=x.value){i+=1;if(x!=null){continue;}else{return;}}print(`${s}`);",
        "let x:Link|null={value:1,next:{value:2,next:null}};let sum=0;while(x!=null){sum+=x.value;x=x.next;continue;}print(`${sum}`);",
        "let x:Link|null={value:1,next:{value:2,next:null}};let sum=0;for(;x!=null;x=x.next){sum+=x.value;continue;}print(`${sum}`);",
        "const x:Box|null={value:2};while(true){if(x!=null){break;}continue;}print(`${x.value}`);",
        "const x:Box|null={value:2};for(;;){if(x!=null){break;}continue;}print(`${x.value}`);",
        "const x:Box|null={value:2};while(true){if(x==null){return;}break;}print(`${x.value}`);",
        "const x:Box|null={value:2};if(x==null){return;}let i=0;while(i<2&&x!=null){i+=1;continue;}print(`${x.value}`);",
        "const x:Box|null={value:2};if(x!=null){print(`${x.value}`);}else{return;}print(`${x.value}`);",
        "const x:Box|null={value:2};if(!(x===null)&&x.value>0){print(`${x.value}`);}if(x===null||x.value<0){return;}print(`${x.value}`);",
        "const x:Box|null={value:2};const b:boolean=true;if(!!((x!==null&&b)||(x!==null&&!b))){print(`${x.value}`);}",
        "const x:Box|null={value:2};const b:boolean=true;if(!((x===null||b)&&(x===null||!b))){print(`${x.value}`);}",
        "let calls=0;const yes=():boolean=>{calls+=1;return true;};const no=():boolean=>{calls+=10;return false;};print(`${(yes()||no())&&yes()}:${calls}`);",
    ] {
        suite.source(&source(body), None);
    }
    suite.finish();
}
#[test]
fn object_field_inference_requires_context_at_every_depth() {
    let programs = [
        "type Box={value:number};export function main():void{let y:Box|null={value:1};const o={b:y};o.b=null;}",
        "type Box={value:number};export function main():void{let y:Box|null={value:1};y=null;const o={b:y};o.b={value:2};}",
        "type Box={value:number};export function main():void{let y:Box|null={value:1};const o={b:{c:y}};o.b.c=null;}",
        "export function main():void{let n:number|null=1;const o={v:n};o.v=null;}",
        "function truth():boolean{return true;}export function main():void{const b=truth();if(b){const o={v:b};o.v=false;}}",
        "export function main():void{const xs:boolean[]=[true];if(xs[0]){const o={v:xs[0]};o.v=false;}}",
        "type W={b:boolean};export function main():void{const w:W={b:true};if(w.b){const o={v:w.b};o.v=false;}}",
        include_str!("../../../../corpus/reject/r105-narrowed-boolean-field.ts"),
        "type Box={value:number};function get():Box|null{return {value:1};}export function main():void{const y=get();if(y!=null){const f=()=>({b:y});const o=f();o.b=null;}}",
        "type Box={value:number};export function main():void{let y:Box|null={value:1};let o={b:y};o.b=null;}",
        "type Box={value:number};export function main():void{let y:Box|null={value:1};const os=[{b:y}];os[0].b=null;}",
        "function truth():boolean{return true;}export function main():void{const b=truth();if(b){const o={a:{v:(b&&true)}};o.a.v=false;}}",
        include_str!("../../../../corpus/reject/r104-narrowed-nullable-field.ts"),
    ];
    let cases: Vec<_> = programs
        .iter()
        .enumerate()
        .map(|(i, s)| (*s, if i == 11 { &[][..] } else { &[2322u32][..] }))
        .collect();
    field_witness::rejected_fields(&cases);
    let mut suite = Suite::named("P2.5 contextual fields");
    for body in [
        "let y:Box|null={value:1};const o:Hold={b:y};o.b=null;print(`${o.b?.value??0}`);",
        "let y:Box|null={value:1};let o:Hold={b:y};o.b=null;print(`${o.b?.value??0}`);",
        "let y:Box|null={value:1};const os:Hold[]=[{b:y}];os[0].b=null;print(`${os[0].b?.value??0}`);",
        "const y:Box|null={value:1};if(y!=null){const f:()=>Hold=()=>({b:y});const o=f();o.b=null;print(`${o.b?.value??0}`);}",
        "const y:Box|null={value:1};if(y!=null){const f=():Hold=>({b:y});const o=f();o.b=null;print(`${o.b?.value??0}`);}",
        "let y:Box|null={value:1};take({b:y});",
        "let y:Box|null={value:1};const o:Nested={b:{b:y}};o.b.b=null;print(`${o.b.b?.value??0}`);",
        "const b:boolean=true;const w:Flag={v:b};w.v=false;print(`${w.v}`);",
        "let n:number|null=1;const w:MaybeNumber={v:n};w.v=null;print(`${w.v??0}`);",
    ] {
        let program = format!(
            "type Hold={{b:Box|null}};type Nested={{b:Hold}};type Flag={{v:boolean}};type MaybeNumber={{v:number|null}};{TYPES}function take(o:Hold):void{{o.b=null;print(`${{o.b?.value??0}}`);}}export function main():void{{{body}}}"
        );
        suite.source(&program, None);
    }
    suite.finish();
}

#[test]
fn optional_and_parenthesized_references_require_context() {
    const SETUP: &str =
        "type B={s:string|null;flag:boolean};function get():B|null{return {s:'a',flag:true};}";
    let fixtures = [
        (
            "const o=get();if(o!=null&&o.s!=null){let y=o?.s??null;y=null;}",
            &[2322u32][..],
        ),
        (
            "const o=get();if(o!=null&&o.flag){const xs=[o?.flag??true];xs.push(false);}",
            &[2345][..],
        ),
        (
            "const o=get();if(o!=null&&o.s!=null){const f=()=>o?.s??null;}",
            &[][..],
        ),
        (
            "const o=get();if(o!=null&&o.s!=null){const out={a:o?.s??null};out.a=null;}",
            &[2322][..],
        ),
        (
            "const o:B={s:'a',flag:true};if(o.s!=null){let y=(o).s;y=null;}",
            &[2322][..],
        ),
        (
            "const o:B={s:'a',flag:true};if(o.flag){const xs=[(o).flag];xs.push(false);}",
            &[2345][..],
        ),
        (
            "const bs:B[]=[{s:'a',flag:true}];if(bs[0].flag){const xs=[(bs[0]).flag,(bs)[0].flag];xs.push(false);}",
            &[2345][..],
        ),
        (
            "const bs:B[]=[{s:'a',flag:true}];if(bs[0].flag){const xs=[(bs)[0].flag];xs.push(false);}",
            &[2345][..],
        ),
        (
            "const o=get();if(o!=null&&o.flag){const xs=[(o)?.flag??true];xs.push(false);}",
            &[2345][..],
        ),
        (
            "const o=get();if(o!=null&&o.flag){const out={a:(o)?.flag??true};out.a=false;}",
            &[2322][..],
        ),
        ("const o:B={s:'a',flag:true};const f=()=>(o).s;", &[][..]),
        (
            "const xs:boolean[]=[true];if(xs[0]){const ys=[(xs)[0]];ys.push(false);}",
            &[2345][..],
        ),
    ];
    let programs: Vec<_> = fixtures
        .iter()
        .map(|(body, _)| format!("{SETUP}export function main():void{{{body}}}"))
        .collect();
    let cases: Vec<_> = programs
        .iter()
        .zip(&fixtures)
        .map(|(source, (_, codes))| (source.as_str(), *codes))
        .chain([
            (
                include_str!("../../../../corpus/reject/r106-optional-chain-reference.ts"),
                &[2322][..],
            ),
            (
                include_str!("../../../../corpus/reject/r107-parenthesized-reference.ts"),
                &[2345][..],
            ),
        ])
        .collect();
    field_witness::rejected_references("optional-parenthesized", &cases);
    let mut suite = Suite::named("P2.5 reference contexts");
    for body in [
        "const o=get();if(o!=null){let y:string|null=o?.s??null;y=null;print(`${y==null}`);}",
        "const o=get();if(o!=null){const xs:boolean[]=[o?.flag??true];xs.push(false);print(`${xs.length}`);}",
        "const o=get();if(o!=null){const f=():string|null=>o?.s??null;print(f()??'none');}",
        "const o=get();if(o!=null){const out:Text={a:o?.s??null};out.a=null;print(`${out.a==null}`);}",
        "const o:B={s:'a',flag:true};let y:string|null=(o).s;y=null;print(`${y==null}`);",
        "const bs:B[]=[{s:'a',flag:true}];const xs:boolean[]=[(bs[0]).flag,(bs)[0].flag];xs.push(false);print(`${xs.length}`);",
        "const o=get();if(o!=null){take({a:o?.s??null});}",
        "const y:string|null=(get())?.s??null;print(y??'none');",
        "const bs:B[]=[{s:'a',flag:true}];const f=():string|null=>(bs[0]).s;print(f()??'none');",
        "let y=(get())?.s??null;y=null;print(`${y==null}`);",
        "const xs=[(get())?.flag??true];xs.push(false);print(`${xs.length}`);",
    ] {
        suite.source(&format!("{SETUP}type Text={{a:string|null}};function take(x:Text):void{{print(x.a??'none');}}export function main():void{{{body}}}"), None);
    }
    suite.finish();
}
