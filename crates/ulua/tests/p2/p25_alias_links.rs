//! Equal aliases retain one structural identity through every composite link.
//! Each source passes stock tsc, Node, the ABI, and luau-analyze. The test prints its Debug cost.
//! Single-worker Debug cost: 0.60 seconds for this test target.
#[allow(dead_code)]
#[path = "../support/script.rs"]
mod support;
// Shared witnesses include constructors for other test targets.
#[allow(dead_code)]
#[path = "../support/equality.rs"]
mod witness;
use std::time::Instant;
use witness::Suite;

#[test]
fn equal_aliases_relate_through_links_before_and_after_declarations() {
    let start = Instant::now();
    let mut suite = Suite::named("P2.5 alias links");
    for (field, value, target, body, golden) in [
        ("Q[]", "[{v:1}]", "P[]", "print(`${ys[0].v}`);", "1\n"),
        (
            "Q|null",
            "{v:1}",
            "P|null",
            "print(`${ys!=null?ys.v:0}`);",
            "1\n",
        ),
        (
            "Q[][]",
            "[[{v:1}]]",
            "P[][]",
            "print(`${ys[0][0].v}`);",
            "1\n",
        ),
        (
            "Q[]|null",
            "[{v:1}]",
            "P[]|null",
            "print(`${ys!=null?ys[0].v:0}`);",
            "1\n",
        ),
        (
            "(a:P)=>Q",
            "(a:P):Q=>a",
            "(a:Q)=>P",
            "print(`${ys({v:1}).v}`);",
            "1\n",
        ),
        (
            "(a:P,b:Q)=>number",
            "(a:P,b:Q):number=>a.v+b.v",
            "(a:Q,b:P)=>number",
            "print(`${ys({v:1},{v:2})}`);",
            "3\n",
        ),
        (
            "((a:P)=>Q)[]",
            "[(a:P):Q=>a]",
            "((a:Q)=>P)[]",
            "const f:(a:Q)=>P=ys[0];print(`${f({v:1}).v}`);",
            "1\n",
        ),
        (
            "((a:P)=>Q)|null",
            "(a:P):Q=>a",
            "((a:Q)=>P)|null",
            "print(`${ys!=null?ys({v:1}).v:0}`);",
            "1\n",
        ),
        (
            "(a:(p:P)=>Q)=>(b:Q)=>P",
            "(a:(p:P)=>Q):((b:Q)=>P)=>a",
            "(a:(p:Q)=>P)=>(b:P)=>Q",
            "const f:(b:P)=>Q=ys((a:Q):P=>a);print(`${f({v:1}).v}`);",
            "1\n",
        ),
        (
            "(a:P|null)=>Q|null",
            "(a:P|null):Q|null=>a",
            "(a:Q|null)=>P|null",
            "const y:P|null=ys({v:1});print(`${y!=null?y.v:0}`);",
            "1\n",
        ),
        (
            "(a:P[])=>Q[]",
            "(a:P[]):Q[]=>a",
            "(a:Q[])=>P[]",
            "const xs:P[]=ys([{v:1}]);print(`${xs[0].v}`);",
            "1\n",
        ),
        (
            "WQ",
            "{inner:[{v:1}]}",
            "WP",
            "print(`${ys.inner[0].v}`);",
            "1\n",
        ),
    ] {
        for reverse in [false, true] {
            let aliases = if reverse {
                "type Q={v:number};type P={v:number};"
            } else {
                "type P={v:number};type Q={v:number};"
            };
            for before in [false, true] {
                let nested = if field == "WQ" {
                    "type WP={inner:P[]};type WQ={inner:Q[]};"
                } else {
                    ""
                };
                let declarations = format!("{aliases}{nested}type R={{xs:{field}}};");
                let reader = format!("function read(r:R):void{{const ys:{target}=r.xs;{body}}}");
                let main =
                    format!("export function main():void{{const r:R={{xs:{value}}};read(r);}}");
                let source = if before {
                    format!("{reader}{main}{declarations}")
                } else {
                    format!("{declarations}{reader}{main}")
                };
                suite.source(&source, Some(golden));
            }
        }
    }
    // An unrelated field must not change the relation of the array field.
    suite.source("type P={v:number};type Q={v:number};type R={xs:Q[];ys:P[]};export function main():void{const r:R={xs:[{v:1}],ys:[]};const ys:P[]=r.xs;print(`${ys[0].v}`);}", Some("1\n"));
    for (a, b) in [(2, 3), (3, 2), (3, 4)] {
        for reverse in [false, true] {
            let mut aliases = String::new();
            let rings = if reverse {
                [("B", b), ("A", a)]
            } else {
                [("A", a), ("B", b)]
            };
            for (prefix, count) in rings {
                for i in 0..count {
                    let next = (i + 1) % count;
                    aliases.push_str(&format!(
                        "type {prefix}{i}={{v:number;next:(p:{prefix}{next})=>{prefix}{next}}};"
                    ));
                }
            }
            aliases.push_str("type R={xs:B0[];n:B0|null;f:(p:A0)=>B0};");
            suite.source(&format!("{aliases}export function main():void{{const a:A0={{v:1,next:(p:A1):A1=>p}};const b:B0=a;const r:R={{xs:[b],n:b,f:(p:A0):B0=>p}};const xs:A0[]=r.xs;const n:A0|null=r.n;const f:(p:B0)=>A0=r.f;const g:(p:B1)=>A1=b.next;print(`${{xs[0].v}}:${{n!=null?n.v:0}}:${{f(b).v}}:${{g(b).v}}`);}}"), Some("1:1:1:1\n"));
        }
    }
    suite.source(
        include_str!("../../../../corpus/accept/a49-equal-aliases-through-links.ts"),
        Some("1 2 3 3 7 5\n"),
    );
    suite.finish();
    eprintln!("P2.5 alias link cost={:.3}s", start.elapsed().as_secs_f64());
}
