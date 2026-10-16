//! Shared region timing. Preparation and validation stay outside the clock.
use crate::stats::{Measurement, SAMPLES, Sample, WARMUP_ITERATIONS, WARMUP_SECONDS};
use std::time::Instant;

pub(crate) trait Workload {
    fn prepare(&mut self);
    fn call(&mut self) -> i32;
    fn result(&mut self, status: i32) -> Result<u64, String>;
}

pub(crate) trait Clock {
    type Mark;
    fn start(&mut self) -> Self::Mark;
    fn elapsed(&mut self, start: Self::Mark) -> f64;
}
pub(crate) struct Monotonic;
impl Clock for Monotonic {
    type Mark = Instant;
    fn start(&mut self) -> Instant {
        Instant::now()
    }
    fn elapsed(&mut self, start: Instant) -> f64 {
        start.elapsed().as_secs_f64()
    }
}
fn iteration(subject: &mut impl Workload, clock: &mut impl Clock) -> Result<Sample, String> {
    subject.prepare();
    let start = clock.start();
    let status = subject.call();
    let seconds = clock.elapsed(start);
    let checksum = subject.result(status)?;
    Ok(Sample { checksum, seconds })
}
pub(crate) fn measure(
    subject: &mut impl Workload,
    clock: &mut impl Clock,
) -> Result<Measurement, String> {
    let first = iteration(subject, clock)?;
    let mut measurement = Measurement {
        warmup_iterations: 1,
        warmup_seconds: first.seconds,
        warmup_checksum: first.checksum,
        warmup_consistent: true,
        samples: Vec::with_capacity(SAMPLES),
    };
    while measurement.warmup_iterations < WARMUP_ITERATIONS
        || measurement.warmup_seconds < WARMUP_SECONDS
    {
        let sample = iteration(subject, clock)?;
        measurement.warmup_consistent &= sample.checksum == first.checksum;
        measurement.warmup_iterations += 1;
        measurement.warmup_seconds += sample.seconds;
    }
    for _ in 0..SAMPLES {
        measurement.samples.push(iteration(subject, clock)?);
    }
    measurement.validate()?;
    Ok(measurement)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{cell::Cell, rc::Rc};

    struct FakeWorkload {
        phase: Rc<Cell<u8>>,
        calls: usize,
    }
    impl Workload for FakeWorkload {
        fn prepare(&mut self) {
            assert_eq!(self.phase.replace(1), 0);
        }
        fn call(&mut self) -> i32 {
            assert_eq!(self.phase.get(), 2);
            self.calls += 1;
            0
        }
        fn result(&mut self, status: i32) -> Result<u64, String> {
            assert_eq!(status, 0);
            assert_eq!(self.phase.replace(0), 3);
            Ok(17)
        }
    }
    struct FakeClock {
        phase: Rc<Cell<u8>>,
        duration: f64,
    }
    impl Clock for FakeClock {
        type Mark = ();
        fn start(&mut self) {
            assert_eq!(self.phase.replace(2), 1);
        }
        fn elapsed(&mut self, (): ()) -> f64 {
            assert_eq!(self.phase.replace(3), 2);
            self.duration
        }
    }

    // Cost: 0.000008 s, Debug, measured locally; fake execution and time, no sleep or benchmark workload.
    #[test]
    fn measure_enforces_warmup_and_sample_counts() {
        let start = Instant::now();
        // 1/8 second reaches the floor before three iterations; 1/16 needs four iterations.
        for (duration, warmup_iterations) in [(0.125, 3), (0.0625, 4)] {
            let phase = Rc::new(Cell::new(0));
            let mut subject = FakeWorkload {
                phase: phase.clone(),
                calls: 0,
            };
            let mut clock = FakeClock { phase, duration };
            let measurement = measure(&mut subject, &mut clock).unwrap();
            assert_eq!(measurement.warmup_iterations, warmup_iterations);
            assert_eq!(
                measurement.warmup_seconds,
                duration * warmup_iterations as f64
            );
            assert_eq!(measurement.samples.len(), 11);
            assert_eq!(subject.calls, warmup_iterations as usize + 11);
            assert!(measurement.warmup_consistent);
            assert_eq!(measurement.warmup_checksum, 17);
            assert!(
                measurement
                    .samples
                    .iter()
                    .all(|s| s.checksum == 17 && s.seconds == duration)
            );
        }
        eprintln!(
            "measure_enforces_warmup_and_sample_counts cost={:.6}s",
            start.elapsed().as_secs_f64()
        );
    }
}
