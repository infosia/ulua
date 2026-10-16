//! P6.3: ten reduced benchmark pairs, with the §6 sample policy.
//! Measured cost: 4.9 to 5.1 seconds in the timing gate profile; ten workload pairs.
//! Profile: Debug Rust tests; native shim and pinned VM use O3 and NDEBUG.
use std::time::Instant;

// Reuse the benchmark hosts, region clock, and sources. No workload algorithm lives here.
#[path = "../../../ulua_bench/src/measurement.rs"]
#[allow(dead_code)]
mod measurement;
#[path = "../../../ulua_bench/src/stats.rs"]
#[allow(dead_code)]
mod stats;
#[path = "../../../ulua_bench/src/subjects.rs"]
#[allow(dead_code, unused_imports)]
mod subjects;

// SAFETY: The benchmark host uses the same pinned VM as this native test archive.
#[link(name = "ulua_test_native", kind = "static")]
unsafe extern "C" {}

struct Case {
    id: &'static str,
    ulua: &'static str,
    luau: &'static str,
    sizes: &'static [(&'static str, &'static str)],
    bound: f64,
}
macro_rules! case {
    ($id:literal, $sizes:expr, $bound:expr) => {
        Case {
            id: $id,
            ulua: include_str!(concat!("../../../ulua_bench/workloads/ulua/", $id, ".ts")),
            luau: include_str!(concat!("../../../ulua_bench/workloads/luau/", $id, ".luau")),
            sizes: $sizes,
            bound: $bound,
        }
    };
}

// Close measurements in this profile. Bound = ceil(max(runs) * 1.25 * 100) / 100.
// Workload          Run 1        Run 2        Run 3        Bound
// fib-recursive     1.728378618  1.738812740  1.717784320  2.18
// fib-loop          2.275175843  2.195648634  2.214573412  2.85
// mandelbrot        1.151970196  1.160009387  1.157890324  1.46
// primes            2.888844444  2.881059780  2.763153279  3.62
// sort              3.590058200  3.627846468  3.488790685  4.54
// tree              2.300722816  2.293444286  2.290509119  2.88
// queen             3.200331157  3.206669378  3.071503879  4.01
// particles         3.533115665  3.386335171  3.335365588  4.42
// callbacks         3.174364506  3.176350313  3.132888022  3.98
// collect           2.077057366  2.063421737  2.081703582  2.61

// Each mapping applies to both sources, including the inclusive Luau loop bounds.
const CASES: [Case; 10] = [
    case!("fib-recursive", &[("31", "27")], 2.18),
    case!(
        "fib-loop",
        &[("1000000", "100000"), ("999999", "99999")],
        2.85
    ),
    case!("mandelbrot", &[("400", "60"), ("399", "59")], 1.46),
    case!("primes", &[("200000", "50000")], 3.62),
    case!("sort", &[("100000", "20000"), ("99999", "19999")], 4.54),
    case!("tree", &[("14", "12")], 2.88),
    case!("queen", &[("11", "9"), ("10", "8")], 4.01),
    case!("particles", &[("20000", "4000"), ("19999", "3999")], 4.42),
    case!("callbacks", &[("100000", "10000"), ("99999", "9999")], 3.98),
    case!("collect", &[("40000", "4000"), ("39999", "3999")], 2.61),
];

fn reduced(source: &str, sizes: &[(&str, &str)]) -> String {
    // Split at non-digits, so a size never replaces part of another numeric literal.
    source
        .split_inclusive(|c: char| !c.is_ascii_digit())
        .map(|part| {
            let digits = part.trim_end_matches(|c: char| !c.is_ascii_digit());
            match sizes.iter().find(|(old, _)| *old == digits) {
                Some((_, new)) => format!("{new}{}", &part[digits.len()..]),
                None => part.to_owned(),
            }
        })
        .collect()
}

fn sample(subject: &mut impl measurement::Workload, expected: &mut Option<u64>, id: &str) -> f64 {
    let sample = measurement::iteration(subject, &mut measurement::Monotonic)
        .unwrap_or_else(|error| panic!("{id}: {error}"));
    assert_eq!(
        sample.checksum,
        *expected.get_or_insert(sample.checksum),
        "{id} checksum"
    );
    sample.seconds
}

#[test]
fn ten_workload_performance_ratios() {
    let _measurement = crate::measurement_lock();
    let start = Instant::now();
    let mut failures = Vec::new();
    for case in CASES {
        let mut luau = subjects::Luau::new(reduced(case.luau, case.sizes).as_bytes()).unwrap();
        let mut ulua = subjects::Ulua::new(reduced(case.ulua, case.sizes).as_bytes()).unwrap();
        let mut checksum = None;
        let mut warmup = [0.0; 2];
        let mut runs = 0;
        while runs < 3 || warmup.iter().any(|seconds| *seconds < 0.050) {
            warmup[0] += sample(&mut luau, &mut checksum, case.id);
            warmup[1] += sample(&mut ulua, &mut checksum, case.id);
            runs += 1;
        }
        let mut times = [[0.0; 7]; 2];
        let [luau_times, ulua_times] = &mut times;
        for (luau_time, ulua_time) in luau_times.iter_mut().zip(ulua_times) {
            *luau_time = sample(&mut luau, &mut checksum, case.id);
            *ulua_time = sample(&mut ulua, &mut checksum, case.id);
        }
        for samples in &mut times {
            samples.sort_by(f64::total_cmp);
        }
        let ratio = times[1][3] / times[0][3];
        eprintln!(
            "performance {}: luau_ms={:.6} ulua_ms={:.6} ratio={ratio:.9} bound={:.2} checksum={} warmup_runs={runs} warmup_ms={:.3}/{:.3}",
            case.id,
            times[0][3] * 1000.0,
            times[1][3] * 1000.0,
            case.bound,
            checksum.unwrap(),
            warmup[0] * 1000.0,
            warmup[1] * 1000.0,
        );
        if ratio > case.bound {
            failures.push((case.id, ratio, case.bound));
        }
    }
    eprintln!(
        "performance test cost={:.3}s",
        start.elapsed().as_secs_f64()
    );
    assert!(
        failures.is_empty(),
        "performance bounds exceeded: {failures:?}"
    );
}
