//! Fixed-seed completion programs compare the ABI with node, tsc, and single-worker analysis.
//! Measured Debug target cost: 1.75 seconds.
//! The target reports witness cost. No assertion reads time.
#[allow(dead_code)]
#[path = "support/script.rs"]
mod support;
#[allow(dead_code)]
#[path = "support/equality.rs"]
mod witnesses;
use std::time::Instant;
struct Random(u64);
impl Random {
    fn next(&mut self) -> usize {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0 as usize
    }
}
#[test]
fn random_exceptions_match_three_witnesses() {
    let start = Instant::now();
    let mut r = Random(0x27c1_13a5_7e91);
    let mut suite = witnesses::Suite::named("P2.7 random");
    let kinds = [
        "Error",
        "TypeError",
        "RangeError",
        "SyntaxError",
        "ReferenceError",
        "EvalError",
        "URIError",
    ];
    for i in 0..240 {
        let kind = kinds[r.next() % 7];
        let tested = kinds[r.next() % 7];
        let n = r.next() % 9;
        let mut pending = match r.next() % 5 {
            0 => format!("return {n};"),
            1 => "throw err;".into(),
            2 => "break;".into(),
            3 => "continue;".into(),
            _ => "score++;".into(),
        };
        let mut caught = match r.next() % 5 {
            0 => format!("return {};", n + 10),
            1 => "throw e;".into(),
            2 => "break;".into(),
            3 => "continue;".into(),
            _ => "score+=2;".into(),
        };
        if r.next().is_multiple_of(2) {
            let jump = if r.next().is_multiple_of(2) {
                "break"
            } else {
                "continue"
            };
            pending = format!("if(k=={}){{{jump};}}return {n};", r.next() % 3);
        }
        if r.next().is_multiple_of(2) {
            let jump = if r.next().is_multiple_of(2) {
                "break"
            } else {
                "continue"
            };
            caught = format!("if(k=={}){{{jump};}}return {};", r.next() % 3, n + 10);
        }
        let finalizer = match r.next() % 5 {
            0 => format!("return {};", n + 20),
            1 => "throw new URIError('final');".into(),
            2 => "break;".into(),
            3 => "continue;".into(),
            _ => "score+=3;".into(),
        };
        let inner = format!(
            "try{{print(`try${{k}}`);{pending}}}catch(e){{if(e instanceof {tested}){{print(e.message);}}{caught}}}finally{{print('final');{finalizer}}}"
        );
        let body = if r.next().is_multiple_of(2) {
            format!("try{{{inner}}}finally{{print('outer');}}")
        } else {
            inner
        };
        let source = format!(
            "let score:number=0;function f():number{{const err=new {kind}('m{i}');err.name='custom';for(let k=0;k<3;k++){{{body}score+=5;}}return score;}}export function main():void{{try{{print(`${{f()}}`);}}catch(e){{if(e instanceof Error){{print(`${{e.name}}: ${{e.message}}`);}}else{{throw e;}}}}print(`${{score}}`);}}"
        );
        suite.source(&source, None);
    }
    let fixtures = start.elapsed().as_secs_f64();
    suite.finish();
    eprintln!(
        "P2.7 random accepted={} fixtures_cost={fixtures:.3}s total_cost={:.3}s",
        suite.count,
        start.elapsed().as_secs_f64()
    );
}
