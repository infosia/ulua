#[allow(dead_code)]
#[path = "../support/script.rs"]
mod support;
use ulua::{RunLimits, ULUA_FAILED, ULUA_OK};
#[test]
fn number_trap_edges_are_invalid_argument() {
    for expression in [
        "(1).toFixed(101)",
        "(1).toFixed(-1)",
        "(1).toExponential(101)",
        "(1).toPrecision(0)",
        "(1).toPrecision(101)",
        "(1).toString(1)",
        "(1).toString(37)",
        "parseInt('1',0)",
        "Number.parseInt('1',37)",
        "(1).toFixed(Infinity)",
        "(1).toPrecision(-Infinity)",
    ] {
        let mut s = support::Script::new(&format!(
            "const x={expression};export function main():void{{}}"
        ));
        assert_eq!(
            s.instance(RunLimits::default()),
            (ULUA_FAILED, "trap:invalid-argument".into()),
            "{expression}"
        );
    }
    let mut s = support::Script::new("export function main():string{return (2.5).toFixed(100);}");
    assert_eq!(s.instance(RunLimits::default()).0, ULUA_OK);
    assert_eq!(s.call(s.index("main"), &[]).0, ULUA_OK);
}
#[test]
#[ignore = "explicit P7.3 byte cost measurement"]
fn number_parse_byte_cost() {
    let _lock = crate::measurement_lock();
    number_cost::parse_verify(5.0);
}
#[path = "../support/number_cost.rs"]
mod number_cost;

#[test]
#[ignore = "explicit P7.3 product-VM format sweep"]
fn number_format_sweep() {
    let _lock = crate::measurement_lock();
    number_cost::format_verify(5.0);
}
