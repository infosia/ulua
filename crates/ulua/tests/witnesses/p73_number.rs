#[path = "../support/number_cost.rs"]
mod cost;
use super::p55_ratio::TIME_RATIO_BOUND;
#[test]
fn number_interrupt_time_ratios() {
    let _lock = crate::measurement_lock();
    // Section 5.19 uses the shared bound; candidate selection and the explicit measurement target 5.
    cost::parse_verify(TIME_RATIO_BOUND);
    cost::format_verify(TIME_RATIO_BOUND);
}
