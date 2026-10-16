//! One sample policy for both subjects. All durations use seconds.
use std::fmt::Write;

pub(crate) const SAMPLES: usize = 11;
pub(crate) const WARMUP_SECONDS: f64 = 0.2;
pub(crate) const WARMUP_ITERATIONS: u64 = 3;
pub(crate) const NOISE_LIMIT: f64 = 0.15;

#[derive(Clone, Debug)]
pub(crate) struct Sample {
    pub checksum: u64,
    pub seconds: f64,
}
#[derive(Clone, Debug)]
pub(crate) struct Measurement {
    pub warmup_iterations: u64,
    pub warmup_seconds: f64,
    pub warmup_checksum: u64,
    pub warmup_consistent: bool,
    pub samples: Vec<Sample>,
}
#[derive(Clone, Debug)]
pub(crate) struct Statistics {
    pub min: f64,
    pub median: f64,
    pub max: f64,
    pub q1: f64,
    pub q3: f64,
    pub iqr: f64,
    pub noisy: bool,
}

fn quartile(sorted: &[f64], p: f64) -> f64 {
    let position = (sorted.len() - 1) as f64 * p;
    let lower = position.floor() as usize;
    let upper = position.ceil() as usize;
    sorted[lower] + (sorted[upper] - sorted[lower]) * (position - lower as f64)
}

impl Measurement {
    pub fn validate(&self) -> Result<(), String> {
        if self.warmup_iterations < WARMUP_ITERATIONS
            || !self.warmup_seconds.is_finite()
            || self.warmup_seconds < WARMUP_SECONDS
        {
            return Err("Warm-up is below 200 ms or three iterations".into());
        }
        if self.samples.len() < SAMPLES {
            return Err("Fewer than 11 timed samples".into());
        }
        if self.warmup_checksum >= 4_294_967_296
            || self
                .samples
                .iter()
                .any(|s| s.checksum >= 4_294_967_296 || !s.seconds.is_finite() || s.seconds <= 0.0)
        {
            return Err("Invalid checksum or duration".into());
        }
        Ok(())
    }
    pub fn statistics(&self) -> Result<Statistics, String> {
        self.validate()?;
        let mut sorted: Vec<_> = self.samples.iter().map(|s| s.seconds).collect();
        sorted.sort_by(f64::total_cmp);
        let median = quartile(&sorted, 0.5);
        let q1 = quartile(&sorted, 0.25);
        let q3 = quartile(&sorted, 0.75);
        let iqr = q3 - q1;
        Ok(Statistics {
            min: sorted[0],
            median,
            max: sorted[sorted.len() - 1],
            q1,
            q3,
            iqr,
            noisy: iqr > NOISE_LIMIT * median,
        })
    }
    pub fn protocol(&self) -> String {
        let mut out = format!(
            "{} {:.17} {} {}\n",
            self.warmup_iterations,
            self.warmup_seconds,
            self.warmup_checksum,
            u8::from(self.warmup_consistent)
        );
        for s in &self.samples {
            let _ = writeln!(out, "{} {:.17}", s.checksum, s.seconds);
        }
        out
    }
    pub fn parse(text: &str) -> Result<Self, String> {
        let mut words = text.split_whitespace();
        fn next<T: std::str::FromStr>(
            words: &mut std::str::SplitWhitespace<'_>,
        ) -> Result<T, String> {
            words
                .next()
                .ok_or("Truncated measurement")?
                .parse()
                .map_err(|_| "Invalid measurement".into())
        }
        let warmup_iterations = next(&mut words)?;
        let warmup_seconds = next(&mut words)?;
        let warmup_checksum = next(&mut words)?;
        let consistent: u8 = next(&mut words)?;
        if consistent > 1 {
            return Err("Invalid checksum flag".into());
        }
        let mut samples = Vec::new();
        while words.clone().next().is_some() {
            samples.push(Sample {
                checksum: next(&mut words)?,
                seconds: next(&mut words)?,
            });
        }
        let measurement = Self {
            warmup_iterations,
            warmup_seconds,
            warmup_checksum,
            warmup_consistent: consistent == 1,
            samples,
        };
        measurement.validate()?;
        Ok(measurement)
    }
}

// A mismatch in either warm-up or any timed sample withholds the whole workload.
pub(crate) fn checksums_match(measurements: &[Measurement]) -> bool {
    let Some(first) = measurements.first() else {
        return false;
    };
    let expected = first.warmup_checksum;
    measurements.iter().all(|m| {
        m.warmup_consistent
            && m.warmup_checksum == expected
            && m.samples.iter().all(|s| s.checksum == expected)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    fn measurement(times: &[f64]) -> Measurement {
        Measurement {
            warmup_iterations: 3,
            warmup_seconds: 0.2,
            warmup_checksum: 17,
            warmup_consistent: true,
            samples: times
                .iter()
                .map(|&seconds| Sample {
                    checksum: 17,
                    seconds,
                })
                .collect(),
        }
    }
    // Cost: 0.000004 s, Debug, measured locally; arithmetic only, no workload execution.
    #[test]
    fn quartiles_and_noise_gate() {
        let start = Instant::now();
        assert_eq!(quartile(&[0.0, 10.0, 20.0, 30.0], 0.25), 7.5);
        assert_eq!(quartile(&[0.0, 10.0, 20.0, 30.0], 0.75), 22.5);
        assert_eq!(quartile(&[7.0], 0.75), 7.0);
        let mut times = vec![10.0; 11];
        times[..3].fill(9.0);
        times[8..].fill(12.0);
        let at_limit = measurement(&times).statistics().unwrap();
        assert_eq!(
            (at_limit.min, at_limit.median, at_limit.max),
            (9.0, 10.0, 12.0)
        );
        assert_eq!((at_limit.q1, at_limit.q3, at_limit.iqr), (9.5, 11.0, 1.5));
        assert!(!at_limit.noisy);
        times[8..].fill(12.01);
        assert!(measurement(&times).statistics().unwrap().noisy);
        eprintln!(
            "quartiles_and_noise_gate cost={:.6}s",
            start.elapsed().as_secs_f64()
        );
    }
    // Cost: 0.000012 s, Debug, measured locally; seeded mismatches cover every sample position.
    #[test]
    fn mismatch_withholds_whole_workload() {
        let start = Instant::now();
        let clean = measurement(&[1.0; 11]);
        assert!(checksums_match(&[clean.clone(), clean.clone()]));
        for index in 0..11 {
            let mut bad = clean.clone();
            bad.samples[index].checksum += 1;
            assert!(!checksums_match(&[clean.clone(), bad]));
        }
        let mut bad = clean.clone();
        bad.warmup_checksum += 1;
        assert!(!checksums_match(&[clean.clone(), bad]));
        let mut bad = clean.clone();
        bad.warmup_consistent = false;
        assert!(!checksums_match(&[bad, clean]));
        assert!(!checksums_match(&[]));
        eprintln!(
            "mismatch_withholds_whole_workload cost={:.6}s",
            start.elapsed().as_secs_f64()
        );
    }
    // Cost: 0.000028 s, Debug, measured locally; protocol and floor checks only.
    #[test]
    fn rejects_short_warmup_and_invalid_samples() {
        let start = Instant::now();
        let clean = measurement(&[1.0; 11]);
        assert_eq!(
            Measurement::parse(&clean.protocol()).unwrap().samples.len(),
            11
        );
        let mut bad = clean.clone();
        bad.warmup_seconds = 0.199999;
        assert!(bad.validate().is_err());
        let mut bad = clean.clone();
        bad.warmup_iterations = 2;
        assert!(bad.validate().is_err());
        let mut bad = clean.clone();
        bad.samples.pop();
        assert!(bad.validate().is_err());
        for seconds in [0.0, -1.0, f64::NAN, f64::INFINITY] {
            let mut bad = clean.clone();
            bad.samples[0].seconds = seconds;
            assert!(bad.validate().is_err());
        }
        assert!(Measurement::parse("3 0.2 17 1\n17").is_err());
        eprintln!(
            "rejects_short_warmup_and_invalid_samples cost={:.6}s",
            start.elapsed().as_secs_f64()
        );
    }
}
