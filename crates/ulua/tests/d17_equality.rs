//! Direct results, operand effects, and seeded random programs for D17.
//! Single-worker Debug cost: 1.65 seconds for this test target.
#[allow(dead_code)]
#[path = "support/script.rs"]
mod support;
#[path = "support/equality.rs"]
mod witness;
use witness::Suite;

#[test]
fn equality_matches_node_with_only_the_documented_tsc_exceptions() {
    let mut suite = Suite::new();
    direct(&mut suite);
    effects(&mut suite);
    random(&mut suite);
    suite.finish();
}

fn direct(s: &mut Suite) {
    for op in ["===", "!==", "==", "!="] {
        let same = if op.contains('!') { "false" } else { "true" };
        let different = if op.contains('!') { "true" } else { "false" };
        for (a, b, equal) in [
            ("1", "1", true),
            ("1", "2", false),
            ("-0", "0", true),
            ("NaN", "NaN", false),
            ("NaN", "1", false),
            ("1", "NaN", false),
            ("Infinity", "Infinity", true),
            ("'a'", "'a'", true),
            ("'a'", "'b'", false),
            ("true", "true", true),
            ("true", "false", false),
            ("false", "true", false),
        ] {
            s.source(
                &format!("export function main():void{{print(`${{({a}){op}({b})}}`);}}"),
                Some(&format!("{}\n", if equal { same } else { different })),
            );
        }
        for (ty, literal) in [
            ("number[]", "[1]"),
            ("string[]", "['a']"),
            ("boolean[]", "[true]"),
            ("number[][]", "[[1]]"),
            ("(()=>number)[]", "[():number=>1]"),
        ] {
            s.source(
                &format!("export function main():void{{const a:{ty}={literal};const b=a;const c:{ty}={literal};print(`${{a{op}b}} ${{a{op}c}}`);}}"),
                Some(&format!("{same} {different}\n")),
            );
        }
    }
}

fn effects(s: &mut Suite) {
    for op in ["===", "!==", "==", "!="] {
        let different = if op.contains('!') { "true" } else { "false" };
        for (ty, left, right) in [
            ("number", "1", "2"),
            ("string", "'a'", "'b'"),
            ("boolean", "true", "false"),
            ("number[]", "[1]", "[1]"),
        ] {
            // A trace of 12 and two calls pins order and one evaluation per operand.
            s.source(
                &format!("let trace=0;let calls=0;function left():{ty}{{trace=trace*10+1;calls++;return {left};}}function right():{ty}{{trace=trace*10+2;calls++;return {right};}}export function main():void{{const result=left(){op}right();print(`${{result}} ${{trace}} ${{calls}}`);}}"),
                Some(&format!("{different} 12 2\n")),
            );
        }
        // The left read must precede the right call that mutates its binding.
        s.source(
            &format!("let value=1;let calls=0;function change():number{{value=2;calls++;return 1;}}export function main():void{{print(`${{value{op}change()}} ${{value}} ${{calls}}`);}}"),
            Some(&format!("{} 2 1\n", if op.contains('!') { "false" } else { "true" })),
        );
        // Short circuits and conditional arms must preserve the operand effects.
        s.source(
            &format!("let trace=0;function mark(n:number):boolean{{trace=trace*10+n;return n===1;}}export function main():void{{const result=(false&&mark(9)||mark(1)){op}(true?mark(2):mark(8));print(`${{result}} ${{trace}}`);}}"),
            Some(&format!("{different} 12\n")),
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
    fn pick<'a>(&mut self, choices: &[&'a str]) -> &'a str {
        choices[self.next() as usize % choices.len()]
    }
}

fn random(s: &mut Suite) {
    let mut rng = Random(0xd17c_7a35);
    let mut shapes = [0; 8];
    let before = s.count;
    for i in 0..320 {
        let op = rng.pick(&["===", "!==", "==", "!="]);
        let other_op = rng.pick(&["===", "!==", "==", "!="]);
        let (ty, literals): (&str, &[&str]) = match rng.next() % 3 {
            0 => ("number", &["-1", "0", "1", "2", "NaN", "-0", "Infinity"]),
            1 => ("string", &["'a'", "'b'", "''", "`x`"]),
            _ => ("boolean", &["true", "false", "!false", "(true&&false)"]),
        };
        let literal = rng.pick(literals);
        let second = rng.pick(literals);
        let args = (rng.pick(literals), rng.pick(literals));
        let c = rng.pick(&["true", "false"]);
        let lhs = rng.pick(&["x", "(x)", "identity(x)", "(c?x:y)"]);
        let rhs = rng.pick(&["y", literal, "identity(y)", "(c?y:x)"]);
        let eq = format!("({lhs}){op}({rhs})");
        let next = format!("x{other_op}({second})");
        let body = match i % 8 {
            0 => format!("if({eq}){{return {next};}}else{{return !({next});}}"),
            1 => format!(
                "let result:boolean=false;for(let i=0;i!==4;i++){{if(i===1){{continue;}}result={eq};if(result{other_op}c){{break;}}x=y;}}return result;"
            ),
            2 => format!(
                "if({eq}){{const f=():boolean=>{next};return f();}}function f():boolean{{return {eq};}}return f();"
            ),
            3 => format!(
                "const a={eq};const b=!a;const d=b||c;let e:boolean=d;if(e){{x=y;e={next};}}return e{other_op}a;"
            ),
            4 => format!(
                "let i=0;let result:boolean=false;while(i!=4){{i++;const a={eq};if(a){{result={next};continue;}}result=!a;if(i==3){{break;}}}}return result;"
            ),
            5 => format!(
                "const a={eq};if(a){{return (a{op}false){other_op}(c===true);}}return (true{op}false){other_op}a;"
            ),
            6 => format!("const a={eq};const b=NaN{other_op}NaN;return a{op}b;"),
            _ => format!(
                "const a:{ty}[]=[x];const b=a;const d:{ty}[]=[y];let result:boolean=a{op}b;for(const value of a){{const alias=value{other_op}y;const f=():boolean=>(a{op}d){other_op}alias;result=f();}}return result;"
            ),
        };
        shapes[i % 8] += 1;
        let source = format!(
            "function identity(x:{ty}):{ty}{{return x;}}function run(x:{ty},y:{ty},c:boolean):boolean{{{body}}}export function main():void{{print(`${{run({},{},{c})}}`);}}",
            args.0, args.1
        );
        s.source(&source, None);
    }
    let mut rejected = 0;
    for _ in 0..48 {
        let op = rng.pick(&["===", "!==", "==", "!="]);
        let operands = [
            ("1", "'1'"),
            ("'a'", "true"),
            ("false", "0"),
            ("a", "b"),
            ("f", "f"),
            ("a", "[1]"),
        ];
        let (a, b) = operands[rng.next() as usize % operands.len()];
        let source = format!(
            "function f():number{{return 1;}}export function main():void{{const a=[1];const b=['a'];const result={a}{op}{b};}}"
        );
        assert_eq!(
            ulua_compiler::compile(source.as_bytes(), ulua_compiler::Limits::default())
                .unwrap_err()
                .code,
            "C7"
        );
        rejected += 1;
    }
    eprintln!(
        "D17 random seed=d17c7a35 accepted={} rejected={rejected} shapes={shapes:?}",
        s.count - before
    );
}
