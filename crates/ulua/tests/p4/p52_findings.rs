//! P5.2 fuzz findings on the async helpers. Each case failed under the sanitizer build
//! (pipeline.md §2.6) before its fix: a weak-table lookup keyed by a number outside the
//! `int` range runs the float-to-int conversion of the Luau array fast path
//! (`VM/src/lvmexecute.cpp` of the pin), which UBSan reports. The normal build cannot see it.
#[allow(dead_code)]
#[path = "../support/requests.rs"]
mod requests;
use ulua::*;

fn run(source: &str) -> (Status, String, String) {
    let (status, code, output, _) = requests::run(source, RunLimits::default());
    (status, code, output)
}

#[test]
fn async_results_outside_the_int_range_resolve() {
    // `return x` resolves the promise of `f` with each value.
    let source = "async function f(x: number): Promise<number> { return x; }\nexport async function main(): Promise<void> { print(`${await f(0 / 0)} ${await f(1e300)} ${await f(-1e300)} ${await f(1099511627776)}`); }";
    assert_eq!(
        run(source),
        (
            ULUA_OK,
            String::new(),
            "NaN 1e+300 -1e+300 1099511627776\n".into()
        )
    );
}

#[test]
fn host_completions_outside_the_int_range_resolve() {
    // The fuzz input: a request completed with NaN, returned from an async export.
    let source = "// grants: hostLater\nasync function wait(n: number): Promise<number> { const v = await hostLater(n); return v; }\nexport async function main(): Promise<void> { print(`${await wait(0 / 0)} ${await wait(1e300)}`); }";
    assert_eq!(run(source), (ULUA_OK, String::new(), "NaN 1e+300\n".into()));
}

#[test]
fn async_results_of_infinities_and_negative_zero_resolve() {
    let source = "async function f(x: number): Promise<number> { return x; }\nfunction negativeZero(x: number): boolean { return x === 0 && 1 / x < 0; }\nexport async function main(): Promise<void> { const z = await f(-0); print(`${await f(1 / 0)} ${await f(-1 / 0)} ${z} ${negativeZero(z)}`); }";
    assert_eq!(
        run(source),
        (ULUA_OK, String::new(), "Infinity -Infinity 0 true\n".into())
    );
}

#[test]
fn host_completions_of_infinities_and_negative_zero_resolve() {
    let source = "// grants: hostLater\nasync function wait(n: number): Promise<number> { const v = await hostLater(n); return v; }\nexport async function main(): Promise<void> { const z = await wait(-0); print(`${await wait(1 / 0)} ${await wait(-1 / 0)} ${1 / z}`); }";
    assert_eq!(
        run(source),
        (
            ULUA_OK,
            String::new(),
            "Infinity -Infinity -Infinity\n".into()
        )
    );
}
