//! Direct contextual copies and seeded random copies after equality and truth tests.
//! Measured Debug cost: 2.03 s for 712 accepted sources and three batch witnesses.
#[allow(dead_code)]
#[path = "support/script.rs"]
mod support;
#[allow(dead_code)]
#[path = "support/equality.rs"]
mod witness;
use witness::Suite;

#[test]
fn boolean_copies_need_context_and_other_copies_pass_all_witnesses() {
    let mut suite = Suite::named("D18");
    direct(&mut suite);
    random(&mut suite);
    suite.finish();
}

fn direct(suite: &mut Suite) {
    for form in [
        "b",
        "(b)",
        "!b",
        "c?b:false",
        "c?false:b",
        "b&&true",
        "true&&b",
        "b||false",
        "false||b",
        "c?(!b&&true):(false||b)",
        "bs[0]",
        "bss[0][0]",
        "(bs[0])",
        "!bss[0][0]",
        "c?bs[0]:false",
        "c?false:bss[0][0]",
        "bs[0]&&true",
        "true&&bss[0][0]",
        "bs[0]||false",
        "false||bss[0][0]",
        "c?(!bs[0]&&true):(false||bss[0][0])",
    ] {
        for body in [
            format!("let copy:boolean={form};copy=false;print(`${{copy}}`);"),
            format!("const xs:boolean[]=[{form}];xs.push(false);print(xs.join());"),
            format!("const xs:boolean[][]=[[{form}]];xs[0].push(false);print(xs[0].join());"),
            format!("take([[{form}]]);"),
            format!("const xs:boolean[][]=[];xs.push([{form}]);print(xs[0].join());"),
            format!("let xs:boolean[][]=[];xs=[[{form}]];print(xs[0].join());"),
            format!("const f=():boolean=>{form};print(`${{f()}}`);"),
            format!("const f:()=>boolean=()=>{form};print(`${{f()}}`);"),
            format!("const f=():boolean[][]=>[[{form}]];print(f()[0].join());"),
            format!("takeArrow(()=>{form});"),
        ] {
            suite.source(
                &format!("function take(xs:boolean[][]):void{{xs[0].push(false);print(xs[0].join());}}function takeArrow(f:()=>boolean):void{{print(`${{f()}}`);}}export function main():void{{const b=true;const c=false;const bs:boolean[]=[true];const bss:boolean[][]=[[true]];{body}}}"),
                None,
            );
        }
    }
    for form in [
        "1",
        "n+1",
        "-n",
        "number(n)",
        "`${n}`",
        "'a'",
        "true",
        "b?1:2",
        "n",
        "s",
        "NaN",
        "Infinity",
        "ns[0]",
        "ss[0]",
        "ns.length",
        "nss[0].length",
    ] {
        suite.source(
            &format!("function number(n:number):number{{return n;}}export function main():void{{const n=2;const s='a';const ns=[2];const ss=['a'];const nss=[[2]];const b=true;let copy={form};const xs=[{form}];const f=()=>{form};print(`${{copy}} ${{xs[0]}} ${{f()}}`);}}"),
            None,
        );
    }
    for form in ["ns", "nss[0]", "(ns)", "b?ns:ns"] {
        suite.source(&format!("export function main():void{{const ns=[2];const nss=[[2]];const b=true;let copy={form};const xs=[{form}];const f=()=>{form};copy.push(3);print(`${{copy.join()}} ${{xs[0].join()}} ${{f().join()}}`);}}"), None);
    }
    for form in ["number", "fs[0]"] {
        suite.source(&format!("function number():number{{return 2;}}export function main():void{{const fs:(()=>number)[]=[number];let copy={form};const xs=[{form}];const f=():()=>number=>{form};const returned=f();print(`${{copy()}} ${{returned()}} ${{xs.length}}`);}}"), None);
    }
}

struct Random(u32);
impl Random {
    fn next(&mut self) -> u32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        self.0
    }
    fn pick<'a>(&mut self, values: &'a [&str]) -> &'a str {
        values[self.next() as usize % values.len()]
    }
}

fn random(suite: &mut Suite) {
    let mut rng = Random(0xd18c_7a35);
    let before = suite.count;
    let mut positions = [0; 3];
    let mut tests = [0; 4];
    let mut forms = [0; 10];
    let mut accesses = [0; 3];
    let mut rejected = 0;
    for i in 0..288 {
        let (ty, values): (&str, &[&str]) = match i % 3 {
            0 => ("boolean", &["true", "false"]),
            1 => ("string", &["'a'", "'b'", "''"]),
            _ => ("number", &["-1", "0", "1", "2", "NaN", "Infinity"]),
        };
        let initial = rng.pick(values);
        let compared = rng.pick(values);
        let wider = rng.pick(values);
        let access = (i / 27) % 3;
        let reference = ["x", "items[0]", "nested[0][0]"][access];
        let condition_index = (i / 9) % 4;
        let truth = if ty == "boolean" { reference } else { "b" };
        let condition = match condition_index {
            0 => format!(
                "{reference}{}({compared})",
                rng.pick(&["===", "!==", "==", "!="])
            ),
            1 => truth.into(),
            2 => format!("!{truth}"),
            _ => format!("{truth}{}true", rng.pick(&["===", "!==", "==", "!="])),
        };
        let boolean = rng.pick(&["true", "false"]);
        let choices = if ty == "boolean" {
            vec![
                "x".into(),
                "(x)".into(),
                "!x".into(),
                "b?x:false".into(),
                "b?false:x".into(),
                "x&&true".into(),
                "true&&x".into(),
                "x||false".into(),
                "false||x".into(),
                "b?(!x&&true):(false||x)".into(),
            ]
        } else {
            vec![
                "x".into(),
                "(x)".into(),
                "((x))".into(),
                format!("b?x:{initial}"),
                format!("b?{initial}:x"),
            ]
        };
        let form_index = rng.next() as usize % choices.len();
        let form = choices[form_index].replace('x', reference);
        let position = (i / 3) % 3;
        let copy_prefix = match position {
            0 => "let copy=".into(),
            1 => String::from("const xs=["),
            _ => "let f=()=>".into(),
        };
        let inferred = match position {
            0 => format!("{copy_prefix}{form};copy={wider};print(`${{copy}}`);"),
            1 => format!("{copy_prefix}{form}];xs.push({wider});print(xs.join());"),
            _ => format!("{copy_prefix}{form};print(`${{f()}}`);f=()=>{wider};print(`${{f()}}`);"),
        };
        let annotated = match position {
            0 => format!("let copy:{ty}={form};copy={wider};print(`${{copy}}`);"),
            1 => format!(
                "const xs:{ty}[][]=[[{initial}],[{form}]];xs[1].push({wider});print(xs[1].join());"
            ),
            _ => format!(
                "let f=():{ty}=>{form};print(`${{f()}}`);f=():{ty}=>{wider};print(`${{f()}}`);"
            ),
        };
        let source = |body: &str| {
            let (parameters, binding, arguments) = if i % 2 == 0 {
                (
                    "b:boolean".into(),
                    format!("const x={initial};"),
                    boolean.into(),
                )
            } else {
                (
                    format!("x:{ty},b:boolean"),
                    String::new(),
                    format!("{initial},{boolean}"),
                )
            };
            let compound = if ty == "number" {
                "let k=x;k*=2;k+=1;items[0]*=2;print(`${k} ${items[0]}`);"
            } else {
                ""
            };
            format!(
                "// D18 random\nfunction run({parameters}):void{{{binding}const items:{ty}[]=[x];const nested:{ty}[][]=[[x]];if({condition}){{{body}{compound}}}else{{print('else');}}}}export function main():void{{run({arguments});}}"
            )
        };
        let inferred_source = source(&inferred);
        if ty == "boolean" {
            let error = ulua_compiler::compile(
                inferred_source.as_bytes(),
                ulua_compiler::Limits::default(),
            )
            .unwrap_err();
            assert_eq!(error.code, "C11", "{inferred_source}");
            assert_eq!(
                error.offset as usize,
                inferred_source.find(&copy_prefix).unwrap() + copy_prefix.len(),
                "{inferred_source}"
            );
            assert_eq!(
                error.message,
                "An inferred type cannot come from a boolean reference; add a type annotation"
            );
            rejected += 1;
        } else {
            suite.source(&inferred_source, None);
        }
        suite.source(&source(&annotated), None);
        positions[position] += 1;
        tests[condition_index] += 1;
        forms[form_index] += 1;
        accesses[access] += 1;
    }
    eprintln!(
        "D18 random seed=d18c7a35 accepted={} rejected={rejected} positions={positions:?} tests={tests:?} forms={forms:?} accesses={accesses:?}",
        suite.count - before
    );
}
