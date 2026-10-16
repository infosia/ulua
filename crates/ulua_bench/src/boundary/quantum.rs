//! Clock resolution check. Preserve fractional medians at the one-percent boundary.
use std::time::{Duration, Instant};

pub(super) const PAIRS: usize = 100_000;

pub(super) fn smallest_positive(differences: impl Iterator<Item = Duration>) -> Option<Duration> {
    differences.filter(|d| !d.is_zero()).min()
}
pub(super) fn observe() -> Option<Duration> {
    smallest_positive((0..PAIRS).map(|_| {
        let first = Instant::now();
        Instant::now().duration_since(first)
    }))
}
pub(super) fn acceptable(quantum: Option<Duration>, median_seconds: f64) -> bool {
    let Some(quantum) = quantum else {
        return false;
    };
    // A median can contain half a nanosecond after interpolation. Do not truncate it.
    !quantum.is_zero()
        && median_seconds.is_finite()
        && median_seconds > 0.0
        && quantum.as_nanos() as f64 * 100.0 <= median_seconds * 1e9
}

#[cfg(test)]
mod tests {
    use super::*;

    // Cost: 0.000024 s, Debug, measured locally; fixed arithmetic vectors, no clock probe or workload.
    #[test]
    fn quantum_arithmetic() {
        let start = Instant::now();
        let ns = Duration::from_nanos;
        assert_eq!(
            smallest_positive([ns(0), ns(30), ns(10), ns(20), ns(0)].into_iter()),
            Some(ns(10))
        );
        assert_eq!(smallest_positive([ns(0); 4].into_iter()), None);
        assert_eq!(smallest_positive([].into_iter()), None);
        assert!(acceptable(Some(ns(10)), 1000.0 / 1e9));
        assert!(!acceptable(Some(ns(10)), 999.5 / 1e9));
        assert!(acceptable(Some(ns(10)), 1000.5 / 1e9));
        assert!(!acceptable(None, 1.0));
        assert!(!acceptable(Some(ns(0)), 1.0));
        for median in [0.0, -1.0, f64::NAN, f64::INFINITY] {
            assert!(!acceptable(Some(ns(1)), median));
        }
        // Multiplication must not overflow Duration or u64 at large spans.
        assert!(!acceptable(Some(Duration::MAX), 1.0));
        assert!(acceptable(Some(Duration::from_secs(1)), 100.0));
        eprintln!(
            "quantum_arithmetic cost={:.6}s",
            start.elapsed().as_secs_f64()
        );
    }
}
