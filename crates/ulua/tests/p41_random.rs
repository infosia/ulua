//! Fixed-seed async tasks compare their full job order with three witnesses.
//! Each run awaits async main and records unhandledRejection in the node witness.
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
fn random_async_programs_match_three_witnesses() {
    let start = Instant::now();
    let mut r = Random(0x0041_c124_0026);
    let mut suite = witnesses::Suite::named("P4.1 random");
    for i in 0..240 {
        let mut functions = String::from(
            "async function value(n:number):Promise<number>{return n;}async function failure(n:number):Promise<number>{await value(n);throw new RangeError('bad');}",
        );
        for task in 0..3 {
            let count = 1 + r.next() % 4;
            let fail = r.next().is_multiple_of(3);
            let pending = match r.next() % 4 {
                0 => "return await value(sum);",
                1 => "return value(sum);",
                2 => "throw new TypeError('return');",
                _ => "sum+=await value(1);",
            };
            let caught = match r.next() % 4 {
                0 => "return await value(9);",
                1 => "throw e;",
                2 => "return value(11);",
                _ => "sum+=await value(3);",
            };
            let finalizer = match r.next() % 4 {
                0 => "return await value(13);",
                1 => "return value(17);",
                2 => "throw new URIError('final');",
                _ => "sum+=await value(5);",
            };
            functions.push_str(&format!("async function task{task}():Promise<number>{{let sum:number=0;print('t{task} start');for(let k=0;k<{count};k++){{try{{sum+=await {}(k);print(`t{task} value ${{sum}}`);{pending}}}catch(e){{print('t{task} catch');if(e instanceof Error){{print(e.name);}}{caught}}}finally{{await value(k);print('t{task} finally');{finalizer}}}}}return sum;}}",if fail {"failure"} else {"value"}));
        }
        let order = if r.next().is_multiple_of(2) {
            [0, 1, 2]
        } else {
            [2, 0, 1]
        };
        let mut body =
            String::from("const p0=task0();const p1=task1();const p2=task2();print('started');");
        for task in order {
            body.push_str(&format!("try{{print(`p{task} ${{await p{task}}} ${{await p{task}}}`);}}catch(e){{if(e instanceof Error){{print(`p{task} ${{e.name}}`);}}}}"));
        }
        body.push_str(&format!("print('end{i}');"));
        if i % 24 == 0 {
            body.push_str("const lost=failure(1);");
            suite.source_async_trap(
                &format!("{functions}export async function main():Promise<void>{{{body}}}"),
                "RangeError",
            );
        } else {
            suite.source(
                &format!("{functions}export async function main():Promise<void>{{{body}}}"),
                None,
            );
        }
    }
    suite.finish();
    eprintln!(
        "P4.1 random seed=0x41c1240026 programs={} unhandled=10 cost={:.3}s",
        suite.count,
        start.elapsed().as_secs_f64()
    );
}
