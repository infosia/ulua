#[path = "../support/number_cost.rs"]
mod cost;
#[test]
fn number_interrupt_time_ratios() {
    let _lock = crate::measurement_lock();
    // §5.19 gates at 6; candidate selection and the explicit measurement target 5.
    cost::parse_verify(6.0);
    cost::format_verify(6.0);
}
