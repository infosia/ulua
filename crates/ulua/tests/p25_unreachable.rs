//! C16.8 checks declared types at unreachable points against stock tsc.
//! Measured Debug cost: 1.30 seconds, including tsc, Node, and Luau analysis.
#[allow(dead_code)]
#[path = "support/p25.rs"]
mod oracle;
#[allow(dead_code)]
#[path = "support/script.rs"]
mod support;
#[allow(dead_code)]
#[path = "support/equality.rs"]
mod witness;
use std::time::Instant;
use witness::Suite;

fn source(body: &str, present: bool) -> String {
    let value = if present { "{v:2}" } else { "null" };
    format!(
        "type B={{v:number}};function mk():B|null{{return {value};}}export function main():void{{let out=0;let a:B|null=mk();const k:B|null=mk();{body}}}"
    )
}

#[test]
fn unreachable_reads_have_no_narrowing() {
    let start = Instant::now();
    let bodies = [
        "if(k!=null){return;out+=k.v;}",
        "if(k!=null){let i=0;while(i<3){i++;continue;out+=k.v;}}",
        "if(a!=null){let i=0;while(i<3){i++;break;out+=a.v;}}",
        "if(k==null){return;}if(out>0){return;}else{return;}out+=k.v;",
        "if(k==null){return;}while(true){out+=1;if(out>3){return;}}out+=k.v;",
        "if(k==null){return;}for(;;){return;}out+=k.v;",
        "if(k!=null){const f=():number=>{return 1;out+=k.v;};out+=f();}",
        "if(k==null){return;}for(let i=0;i<3;out+=k.v){i++;break;}",
        // Unreachable scopes and expression edges cannot inherit the last path either.
        "if(k!=null){return;const f=():number=>k.v;}",
        "if(k!=null){out+=false?k.v:0;}",
        "if(k!=null){out+=true?0:k.v;}",
        "if(k!=null){if(false){out+=k.v;}}",
        "if(k!=null){print(`${false&&k.v>0}`);}",
        "if(k!=null){print(`${true||k.v>0}`);}",
        "if(k!=null){return;if(k!=null){out+=k.v;}}",
        "if(k!=null){while(true){if(out>0){break;}else{continue;}out+=k.v;}}",
        "if(k==null){return;}for(let i=0;i<3;out+=k.v){i++;return;}",
        "if(k!=null){for(const n of [1]){continue;out+=k.v;}}",
    ];
    let programs: Vec<_> = bodies.iter().map(|body| source(body, false)).collect();
    let cases: Vec<_> = programs
        .iter()
        .map(|s| (s.as_str(), &[18047u32][..]))
        .chain([(
            include_str!("../../../corpus/reject/r108-unreachable-narrowed-read.ts"),
            &[18047][..],
        )])
        .collect();
    oracle::rejected_programs("unreachable-reads", "C16", &cases);
    eprintln!(
        "P2.5 unreachable reads cost={:.3}s",
        start.elapsed().as_secs_f64()
    );
}

#[test]
fn unreachable_operations_use_declared_types() {
    let start = Instant::now();
    for (code, bodies, codes) in [
        (
            "C16",
            &[
                "if(k!=null){return;k.v=1;}",
                "if(k!=null){return;k.v+=1;}",
                "if(a!=null){while(out<3){out++;continue;a.v=1;}}",
            ][..],
            &[18047u32][..],
        ),
        (
            "C16",
            &[
                "if(k!=null){return;let b:B=k;}",
                "let b:B={v:0};if(k!=null){return;b=k;}",
            ][..],
            &[2322][..],
        ),
        (
            "C11",
            &[
                "if(k!=null){return;let y=k;y=null;}",
                "if(k!=null){return;const o={a:k};o.a=null;}",
            ][..],
            &[][..],
        ),
        (
            "C6",
            &["if(k!=null){return;const xs=[k];xs.push(null);}"][..],
            &[][..],
        ),
        (
            "C7",
            &[
                "const b:B={v:0};if(k!=null){return;print(`${k===b}`);}",
                "if(k!=null){return;print(`${k===k}`);}",
            ][..],
            &[][..],
        ),
    ] {
        let programs: Vec<_> = bodies.iter().map(|body| source(body, false)).collect();
        let cases: Vec<_> = programs.iter().map(|s| (s.as_str(), codes)).collect();
        oracle::rejected_programs(
            &format!(
                "unreachable-operations-{code}-{}",
                codes.first().copied().unwrap_or(0)
            ),
            code,
            &cases,
        );
    }
    let mut suite = Suite::named("P2.5 unreachable declared types");
    for body in [
        // Null tests, annotations, optional reads, and coalescing use the declared nullable type.
        "print('before');if(k!=null){return;print(`${k==null}:${k!=null}:${k===null}:${null!==k}`);}",
        "print('before');if(k!=null){return;const y:B|null=k;print(`${y?.v??0}`);}",
        "print('before');if(k!=null){return;print(`${k?.v??0}:${(k??{v:0}).v}`);}",
        "print('before');if(a!=null){return;a=null;const y:B|null=a;print(`${y?.v??0}`);}",
        "print('before');if(k!=null){return;const f=():number=>{if(k==null){return 0;}return k.v;};out+=f();}",
        "print('before');if(k==null){return;}for(let i=0;i<3;out+=k?.v??0){return;}",
    ] {
        suite.source(&source(body, true), Some("before\n"));
    }
    suite.finish();
    eprintln!(
        "P2.5 unreachable operations cost={:.3}s",
        start.elapsed().as_secs_f64()
    );
}

#[test]
fn reachable_controls_keep_their_narrowing() {
    let start = Instant::now();
    let mut suite = Suite::named("P2.5 reachable controls");
    for body in [
        "if(k!=null){out+=k.v;print(`${out}`);return;}",
        "if(k!=null){let i=0;while(i<3){i++;out+=k.v;continue;}}print(`${out}`);",
        "if(a!=null){let i=0;while(i<3){i++;out+=a.v;break;}}print(`${out}`);",
        "if(k==null){return;}if(out>0){return;}else{out+=k.v;}print(`${out}`);",
        "if(k==null){return;}while(true){out+=1;if(out>3){break;}}out+=k.v;print(`${out}`);",
        "if(k==null){return;}for(;;){break;}out+=k.v;print(`${out}`);",
        "if(k!=null){const f=():number=>{out+=k.v;return 1;};const n=f();out+=n;}print(`${out}`);",
        "if(k==null){return;}for(let i=0;i<2;out+=k.v){i++;}print(`${out}`);",
        "if(k!=null){const f=():number=>k.v;out+=f();}print(`${out}`);",
        "if(k!=null){out+=true?k.v:0;out+=false?0:k.v;}print(`${out}`);",
        "if(k!=null){print(`${true&&k.v>0}:${false||k.v>0}`);}",
        "if(k!=null){if(false){return;}out+=k.v;print(`${out}`);}",
        "if(a==null){a=mk();return;}out+=a.v;print(`${out}`);",
        "if(k!=null){k.v=3;k.v+=1;print(`${k.v}`);}",
    ] {
        suite.source(&source(body, true), None);
    }
    suite.finish();
    eprintln!(
        "P2.5 reachable controls cost={:.3}s",
        start.elapsed().as_secs_f64()
    );
}
