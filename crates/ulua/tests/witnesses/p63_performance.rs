//! P6.3: ten reduced benchmark pairs, with the §6 sample policy.
//! Measured cost: 3.273 seconds in one Windows x86_64 run; ten workload pairs.
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
}
macro_rules! case {
    ($id:literal, $sizes:expr) => {
        Case {
            id: $id,
            ulua: include_str!(concat!("../../../ulua_bench/workloads/ulua/", $id, ".ts")),
            luau: include_str!(concat!("../../../ulua_bench/workloads/luau/", $id, ".luau")),
            sizes: $sizes,
        }
    };
}

// These runs are records, not bounds.
// macOS aarch64 runs use the P6.3 close. Windows x86_64 runs use c8cb050.
// Linux x86_64 runs use gcc 15.2.0 at ebf2fed.
//                   macOS aarch64 runs                   Windows x86_64 runs                  Linux x86_64 runs
// Workload          Run 1        Run 2        Run 3        Run 1        Run 2        Run 3        Run 1        Run 2        Run 3
// fib-recursive     1.728378618  1.738812740  1.717784320  1.659988261  1.582882869  1.579282793  1.826952615  1.662845176  1.742185191
// fib-loop          2.275175843  2.195648634  2.214573412  1.300668321  1.293014555  1.361322191  1.437005553  1.281734102  1.437856561
// mandelbrot        1.151970196  1.160009387  1.157890324  1.584819080  1.644375399  1.563581773  1.744657769  1.758056129  1.759964421
// primes            2.888844444  2.881059780  2.763153279  1.419674361  1.405705536  1.398252115  1.113386371  1.109517976  1.110056318
// sort              3.590058200  3.627846468  3.488790685  3.400581665  3.351272454  3.142357608  3.060473754  3.073539125  3.051075709
// tree              2.300722816  2.293444286  2.290509119  1.926847325  2.039634577  2.083177011  1.987333898  1.941778542  1.944999618
// queen             3.200331157  3.206669378  3.071503879  4.121234637  4.157434579  4.126208141  3.179031336  3.039910299  3.180478671
// particles         3.533115665  3.386335171  3.335365588  2.176150681  2.194046320  2.182662449  1.855363983  1.852111222  1.858323582
// callbacks         3.174364506  3.176350313  3.132888022  1.613859767  1.540546223  1.668515022  1.391641617  1.395207009  1.392529356
// collect           2.077057366  2.063421737  2.081703582  1.695375193  1.712946816  1.650939518  1.801986902  1.813718175  1.801475730

// Each mapping applies to both sources, including the inclusive Luau loop bounds.
const CASES: [Case; 10] = [
    case!("fib-recursive", &[("31", "27")]),
    case!("fib-loop", &[("1000000", "100000"), ("999999", "99999")]),
    case!("mandelbrot", &[("400", "60"), ("399", "59")]),
    case!("primes", &[("200000", "50000")]),
    case!("sort", &[("100000", "20000"), ("99999", "19999")]),
    case!("tree", &[("14", "12")]),
    case!("queen", &[("11", "9"), ("10", "8")]),
    case!("particles", &[("20000", "4000"), ("19999", "3999")]),
    case!("callbacks", &[("100000", "10000"), ("99999", "9999")]),
    case!("collect", &[("40000", "4000"), ("39999", "3999")]),
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
#[ignore = "Measurement only: benchmarks.md section 6 and D38"]
fn ten_workload_performance_ratios() {
    let _measurement = crate::measurement_lock();
    let start = Instant::now();
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
            "performance {}: luau_ms={:.6} ulua_ms={:.6} ratio={ratio:.9} checksum={} warmup_runs={runs} warmup_ms={:.3}/{:.3}",
            case.id,
            times[0][3] * 1000.0,
            times[1][3] * 1000.0,
            checksum.unwrap(),
            warmup[0] * 1000.0,
            warmup[1] * 1000.0,
        );
    }
    eprintln!(
        "performance test cost={:.3}s",
        start.elapsed().as_secs_f64()
    );
}
