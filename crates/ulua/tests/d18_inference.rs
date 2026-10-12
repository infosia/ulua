//! Direct contextual copies and seeded random copies after equality and truth tests.
//! Measured Debug cost: 1.18 s for 396 accepted sources and three batch witnesses.
#[allow(dead_code)]
#[path = "support/script.rs"]
mod support;
#[allow(dead_code)]
#[path = "support/equality.rs"]
mod witness;
use witness::Suite;

#[test]
fn inferred_copies_need_annotations_and_contextual_copies_pass_all_witnesses() {
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
                &format!("function take(xs:boolean[][]):void{{xs[0].push(false);print(xs[0].join());}}function takeArrow(f:()=>boolean):void{{print(`${{f()}}`);}}export function main():void{{const b=true;const c=false;{body}}}"),
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
    ] {
        suite.source(
            &format!("function number(n:number):number{{return n;}}export function main():void{{const n=2;const b=true;let copy={form};const xs=[{form}];const f=()=>{form};print(`${{copy}} ${{xs[0]}} ${{f()}}`);}}"),
            None,
        );
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
        let condition_index = (i / 9) % 4;
        let truth = if ty == "boolean" { "x" } else { "b" };
        let condition = match condition_index {
            0 => format!("x{}({compared})", rng.pick(&["===", "!==", "==", "!="])),
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
        let form = &choices[form_index];
        let position = (i / 3) % 3;
        let literal = match ty {
            "boolean" => "false",
            "string" => "'seed'",
            _ => "0",
        };
        let copy_prefix = match position {
            0 => "let copy=".into(),
            1 => format!("const xs=[[{literal}],["),
            _ => "let f=()=>".into(),
        };
        let inferred = match position {
            0 => format!("{copy_prefix}{form};copy={wider};"),
            1 => format!("{copy_prefix}{form}]];xs[1].push({wider});"),
            _ => format!("{copy_prefix}{form};f=()=>{wider};"),
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
            format!(
                "// D18 random\nfunction run({parameters}):void{{{binding}if({condition}){{{body}}}else{{print('else');}}}}export function main():void{{run({arguments});}}"
            )
        };
        let rejected_source = source(&inferred);
        let error =
            ulua_compiler::compile(rejected_source.as_bytes(), ulua_compiler::Limits::default())
                .unwrap_err();
        assert_eq!(error.code, "C11", "{rejected_source}");
        assert_eq!(
            error.offset as usize,
            rejected_source.find(&copy_prefix).unwrap() + copy_prefix.len(),
            "{rejected_source}"
        );
        assert_eq!(
            error.message,
            "An inferred type cannot come from a reference; add a type annotation"
        );
        rejected += 1;
        suite.source(&source(&annotated), None);
        positions[position] += 1;
        tests[condition_index] += 1;
        forms[form_index] += 1;
    }
    eprintln!(
        "D18 random seed=d18c7a35 accepted={} rejected={rejected} positions={positions:?} tests={tests:?} forms={forms:?}",
        suite.count - before
    );
}
