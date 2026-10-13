//! Cyclic class aliases retain every name that a descendant uses before the root definition.
//! Single-worker Debug cost: 0.38 seconds for this test target.
//! Four cycle shapes use the ABI, Node, tsc, and single-worker Luau analysis.
//! The test prints its cost without a wall-clock assertion.
#[allow(dead_code)]
#[path = "../support/script.rs"]
mod support;
#[allow(dead_code)]
#[path = "../support/equality.rs"]
mod witnesses;
use std::time::Instant;

#[test]
fn cyclic_class_aliases_pass_all_witnesses() {
    let start = Instant::now();
    let mut suite = witnesses::Suite::named("P2.6 class cycles");
    suite.source(
        r#"
class X { d: D; f: F | null = null; constructor(d: D) { this.d = d; } }
class D { x: X | null = null; e: X | null = null; }
class F { x: X | null = null; e: X | null = null; }
export function main(): void { const d = new D(); const x = new X(d);
    d.x = x; x.f = new F(); print(`${x.d === d}`); }
"#,
        Some("true\n"),
    );
    suite.source(
        r#"
class A { b: B | null = null; }
class B { c: C | null = null; }
class C { a: A | null = null; }
export function main(): void {
    const a = new A(); const b = new B(); const c = new C();
    a.b = b; b.c = c; c.a = a;
    const ab: B | null = a.b; const bc: C | null = b.c; const ca: A | null = c.a;
    if (ab != null && bc != null && ca != null) {
        print(`${ab === b} ${bc === c} ${ca === a}`);
    }
}
"#,
        Some("true true true\n"),
    );
    suite.source(
        r#"
type R = { c: C | null };
class C { r: R; constructor(r: R) { this.r = r; } }
export function main(): void {
    const r: R = { c: null }; const c = new C(r); r.c = c;
    const rc: C | null = r.c;
    if (rc != null) { print(`${c.r === r} ${rc === c}`); }
}
"#,
        Some("true true\n"),
    );
    suite.source(
        r#"
class A { v: A | null = null; other: B | null = null; }
class B { v: A | null = null; other: B | null = null; }
export function main(): void {
    const a = new A(); const b = new B();
    a.v = a; a.other = b; b.v = a; b.other = b;
    const other: B | null = a.other; const value: A | null = b.v;
    if (other != null && value != null) { print(`${other === b} ${value === a}`); }
}
"#,
        Some("true true\n"),
    );
    suite.finish();
    eprintln!(
        "P2.6 cycles accepted={} cost={:.3}s",
        suite.count,
        start.elapsed().as_secs_f64()
    );
}
