//! C12.7 checks element receivers at the next access, after its key, or at its store.
//! Measured Debug cost: 0.55 seconds, including tsc, Node, the ABI, and luau-analyze.
#[allow(dead_code)]
#[path = "support/script.rs"]
mod support;
#[allow(dead_code)]
#[path = "support/equality.rs"]
mod witness;
use std::time::Instant;
use witness::Suite;
fn source(body: &str) -> String {
    format!(
        "type B={{f:number}};type W={{items:B[]}};type F={{f:(n:number)=>void}};export function main():void{{function h():number{{print('h');return 7;}}function i(n:number):number{{print(`${{n}}`);return n;}}print('start');{body}print('after');}}"
    )
}
#[test]
fn missing_store_receivers_match_node_output_and_trap_kind() {
    let start = Instant::now();
    let mut suite = Suite::named("P2.5 store traps");
    for (body, golden) in [
        ("const xs:B[]=[];xs[0].f=h();", "start\nh\n"),
        ("const xs:number[][]=[];xs[0][0]=h();", "start\nh\n"),
        ("const zs:B[][]=[[]];zs[0][1].f=h();", "start\nh\n"),
        (
            "const zs:number[][][]=[[[]]];zs[0][1][0]=h();",
            "start\nh\n",
        ),
        ("const zs:B[][]=[];zs[0][1].f=h();", "start\n"),
        ("const zs:number[][][]=[];zs[0][1][0]=h();", "start\n"),
        ("const xs:B[]=[];(xs[i(0)]).f=h();", "start\n0\nh\n"),
        (
            "const xs:number[][]=[];(xs[i(0)])[i(1)]=h();",
            "start\n0\n1\nh\n",
        ),
        (
            "const zs:B[][]=[[]];zs[i(0)][i(1)].f=h();",
            "start\n0\n1\nh\n",
        ),
        ("const zs:B[][]=[];zs[i(0)][i(1)].f=h();", "start\n0\n1\n"),
        // A read through a field needs its receiver before the right side.
        ("const xs:W[]=[];xs[0].items[0].f=h();", "start\n"),
        (
            "const xs:W[]=[{items:[]}];xs[0].items[0].f=h();",
            "start\nh\n",
        ),
        // Changing the array cannot repair the receiver value that was already read.
        (
            "const xs:B[]=[];function rhs():number{print('h');xs.push({f:0});return 7;}xs[0].f=rhs();",
            "start\nh\n",
        ),
        (
            "const xs:number[][]=[];function rhs():number{print('h');xs.push([]);return 7;}xs[0][0]=rhs();",
            "start\nh\n",
        ),
        // Compound assignments read the old value before h.
        ("const xs:B[]=[];xs[0].f+=h();", "start\n"),
        ("const xs:number[][]=[];xs[0][0]+=h();", "start\n"),
        ("const zs:B[][]=[[]];zs[0][1].f+=h();", "start\n"),
    ] {
        suite.source_trap(
            &source(body),
            golden,
            "trap:index-out-of-range",
            "TypeError",
        );
    }
    for (body, golden) in [
        (
            "const zs:number[][]=[];print(`${zs[i(0)][i(1)]}`);",
            "start\n0\n1\n",
        ),
        (
            "const zs:number[][]=[];zs[i(0)][i(1)]=h();",
            "start\n0\n1\nh\n",
        ),
        (
            "const zs:number[][]=[];zs[i(0)][i(1)]+=h();",
            "start\n0\n1\n",
        ),
        ("const ys:B[]=[];print(`${ys[i(0)].f}`);", "start\n0\n"),
        ("const ys:B[]=[];ys[i(0)].f+=h();", "start\n0\n"),
        ("const ys:F[]=[];ys[i(0)].f(h());", "start\n0\n"),
    ] {
        suite.source_trap(
            &source(body),
            golden,
            "trap:index-out-of-range",
            "TypeError",
        );
    }
    for index in ["-1", "0.5", "NaN", "Infinity"] {
        suite.source_trap(
            &source(&format!("const xs:B[]=[];xs[{index}].f=h();")),
            "start\nh\n",
            "trap:index-out-of-range",
            "TypeError",
        );
    }
    suite.source_trap(
        include_str!("../../../corpus/trap/t07-store-through-missing-element.ts"),
        "start\nh\n",
        "trap:index-out-of-range",
        "TypeError",
    );
    suite.finish();
    eprintln!("P2.5 store trap cost={:.3}s", start.elapsed().as_secs_f64());
}

#[test]
fn plain_element_reads_keep_the_c6_trap() {
    let mut script = support::Script::new(&source("const ys:B[]=[];const v=ys[0];"));
    assert_eq!(script.instance(ulua::RunLimits::default()).0, ulua::ULUA_OK);
    let (status, code, _) = script.call(script.index("main"), &[]);
    assert_eq!(
        (status, code.as_str()),
        (ulua::ULUA_FAILED, "trap:index-out-of-range")
    );
}

#[test]
fn present_store_receivers_and_compound_reads_match_node() {
    let start = Instant::now();
    let mut suite = Suite::named("P2.5 store controls");
    for body in [
        "const xs:B[]=[{f:1}];xs[0].f=h();print(`${xs[0].f}`);",
        "const xs:number[][]=[[1]];xs[0][0]=h();print(`${xs[0][0]}`);",
        "const zs:B[][]=[[{f:1},{f:2}]];zs[i(0)][i(1)].f=h();print(`${zs[0][1].f}`);",
        "const zs:number[][][]=[[[1],[2]]];zs[i(0)][i(1)][i(0)]=h();print(`${zs[0][1][0]}`);",
        "let xs:B[]=[{f:1}];const old=xs[0];function rhs():number{xs=[{f:9}];return h();}xs[0].f=rhs();print(`${old.f}:${xs[0].f}`);",
        "const xs:B[]=[{f:1}];const old=xs[0];function rhs():number{xs[0]={f:9};return h();}xs[0].f=rhs();print(`${old.f}:${xs[0].f}`);",
        "const xs:number[][]=[[1]];const old=xs[0];function rhs():number{xs[0]=[9];return h();}xs[0][0]=rhs();print(`${old[0]}:${xs[0][0]}`);",
        "const xs:B[]=[{f:1}];const old=xs[0];function rhs():number{xs[0]={f:9};return h();}xs[0].f+=rhs();print(`${old.f}:${xs[0].f}`);",
        "const xs:number[][]=[[1]];const old=xs[0];function rhs():number{xs[0]=[9];return h();}xs[0][0]+=rhs();print(`${old[0]}:${xs[0][0]}`);",
        "const xs:B[]=[{f:1}];function rhs():number{xs[0].f=99;return h();}xs[0].f+=rhs();print(`${xs[0].f}`);",
        "const xs:number[][]=[[1]];function rhs():number{xs[0][0]=99;return h();}xs[0][0]+=rhs();print(`${xs[0][0]}`);",
        "const xs:B[]=[{f:1}];function index():number{xs[0]={f:9};print('i');return 0;}xs[index()].f=h();print(`${xs[0].f}`);",
        "const xs:number[][]=[[1]];const old=xs[0];function index():number{xs[0]=[9];print('i');return 0;}xs[0][index()]=h();print(`${old[0]}:${xs[0][0]}`);",
        "const xs:B[]=[{f:1}];const old=xs[0];function rhs():number{xs.length=0;return h();}xs[0].f=rhs();print(`${old.f}:${xs.length}`);",
    ] {
        suite.source(&source(body), None);
    }
    suite.finish();
    eprintln!(
        "P2.5 store control cost={:.3}s",
        start.elapsed().as_secs_f64()
    );
}

#[test]
fn tsc_recursive_comparison_resource_limit_stays_accepted() {
    let mut source = String::new();
    for (prefix, length) in [("A", 25), ("B", 26)] {
        for i in 0..length {
            source.push_str(&format!(
                "type {prefix}{i}={{next:{prefix}{}|null}};",
                (i + 1) % length
            ));
        }
    }
    source.push_str("export function main():void{const a:A0={next:null};const b:B0=a;print(`${b.next==null}`);}");
    let mut suite = Suite::named("P2.5 tsc recursive resource");
    suite.source(&source, Some("true\n"));
    suite.finish();
}
