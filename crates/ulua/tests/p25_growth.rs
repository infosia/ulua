//! P2.5 growth compares full ABI compilation at n and 4n, and checks ceiling output.
//! Single-worker Debug cost: 22.10 seconds for this test target.
//! Debug cost is measured and printed, including one batch analyzer run; it is not a gate bound.
//! Ratios above 6.5 use the minimum of two runs per size.
//! Round 12 isolated Debug cost: 9.164 seconds; largest ratio: 3.960.
//! Four concurrent copies, ten batches: 40/40 pass, with no analyzer failure.
//! Per-run Debug cost: 24.374 to 25.082 seconds; largest ratios: 4.010 to 4.510.
//! Round 13 Debug cost: 9.263 seconds; largest ratio: 4.044.
//! Equal alias links: 5.441/20.631 ms at 4/16 KiB, ratio 3.792; ceiling output factor: 12.095.
//! Round 10 measured Debug cost: 9.058 seconds. The largest ratio is 3.958.
//! Condition ratios: conditional 3.619, alternating logical 3.915, conditional with wrappers 3.660.
//! Logical with wrappers: 3.740. Nullable conditional: 3.684. Continue product: 3.863.
//! A conditional test gives no facts. The nullable shape uses a separate null guard in its body.
//! Round 2 ABI times compare 16 KiB and 64 KiB. Before uses the handoff HEAD.
//! Debug ybreak: 888.475/13581.536 ms -> 28.809/112.749 ms; ratios 15.286 -> 3.914.
//! Debug ycont: 678.899/10635.650 ms -> 27.229/106.332 ms; ratios 15.666 -> 3.905.
//! Debug distinct consts: 163.846/2246.953 ms -> 25.709/100.300 ms; ratios 13.714 -> 3.901.
//! Debug continue/update: 675.917/10637.277 ms -> 27.174/105.922 ms; ratios 15.738 -> 3.898.
//! Debug if/else ends: 39.578/158.546 ms -> 39.702/157.984 ms; ratios 4.006 -> 3.979.
//! Release ybreak: 94.294/991.532 ms -> 21.752/42.102 ms; ratios 10.515 -> 1.936.
//! Release ycont: 50.623/781.942 ms -> 7.733/25.709 ms; ratios 15.446 -> 3.324.
//! Release distinct consts: 15.706/189.527 ms -> 6.058/21.778 ms; ratios 12.067 -> 3.595.
//! Release continue/update: 51.067/792.868 ms -> 6.518/24.327 ms; ratios 15.526 -> 3.732.
//! Release if/else ends: 9.481/36.893 ms -> 9.500/35.942 ms; ratios 3.891 -> 3.783.
//! Stock tsc gives TS2563 for ceiling ybreak and if-end-writes; the other new shapes pass.
#[path = "support/analyzer.rs"]
mod analyzer;
#[allow(dead_code)]
#[path = "support/script.rs"]
mod support;
use std::{path::PathBuf, time::Instant};
use support::Script;
use ulua_compiler::{Limits, OUTPUT_FACTOR, OUTPUT_PRELUDE_BYTES, compile};
#[path = "support/p25_shapes.rs"]
mod shapes;
use shapes::{SHAPES, condition_depth, condition_source, make};

#[test]
fn condition_wrappers_reach_the_nesting_bound() {
    for shape in [
        "conditional-tests",
        "alternating-logical-tests",
        "conditional-wrapped-tests",
        "logical-wrapped-tests",
        "nullable-conditional-tests",
    ] {
        let depth = condition_depth(shape);
        compile(condition_source(shape, depth).as_bytes(), Limits::default()).unwrap();
        assert_eq!(
            compile(
                condition_source(shape, depth + 1).as_bytes(),
                Limits::default()
            )
            .unwrap_err()
            .code,
            "limit:nesting-depth",
            "{shape}"
        );
    }
}
#[test]
fn p25_full_pipeline_growth_and_output() {
    let start = Instant::now();
    let directory = std::env::temp_dir().join(format!("ulua-p25-growth-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let mut paths = vec![];
    let mut largest_ratio = 0.0f64;
    let mut largest_output = 0.0f64;
    for shape in SHAPES {
        let n = if matches!(
            *shape,
            "field-product"
                | "fact-product"
                | "break-product"
                | "continue-product"
                | "ybreak"
                | "ycont"
                | "distinct-const-break"
                | "continue-update-writes"
                | "if-end-writes"
        ) {
            16384
        } else {
            4096
        };
        let inputs = [make(shape, n), make(shape, 4 * n)];
        let measure = |source: &str| {
            let now = Instant::now();
            let script = Script::new(source);
            let elapsed = now.elapsed().as_secs_f64();
            drop(script);
            elapsed
        };
        let mut times = inputs.each_ref().map(|source| measure(source));
        if times[1] / times[0] > 6.5 {
            for (time, source) in times.iter_mut().zip(&inputs) {
                *time = time.min(measure(source));
            }
        }
        let ratio = times[1] / times[0];
        largest_ratio = largest_ratio.max(ratio);
        eprintln!(
            "P2.5 growth {shape}: bytes={n}/{} ms={:.3}/{:.3} ratio={ratio:.3}",
            4 * n,
            times[0] * 1000.0,
            times[1] * 1000.0
        );
        let ceiling = make(shape, 65536);
        match compile(ceiling.as_bytes(), Limits::default()) {
            Ok(c) => {
                assert!(c.luau.len() <= OUTPUT_FACTOR * ceiling.len() + OUTPUT_PRELUDE_BYTES);
                let output_ratio = (c.luau.len().saturating_sub(OUTPUT_PRELUDE_BYTES)) as f64
                    / ceiling.len() as f64;
                largest_output = largest_output.max(output_ratio);
                eprintln!(
                    "P2.5 output {shape}: source={} output={} factor={output_ratio:.3}",
                    ceiling.len(),
                    c.luau.len()
                );
                let path = directory.join(format!("{shape}.luau"));
                std::fs::write(&path, c.luau).unwrap();
                paths.push(path);
            }
            Err(d) => assert_eq!(d.code, "limit:output", "{shape}: {d:?}"),
        }
    }
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let result = analyzer::command(root.join("target/luau-tools/luau-analyze"))
        .args(&paths)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "luau-analyze outputs={}\n{}",
        directory.display(),
        analyzer::failure(&result)
    );
    std::fs::remove_dir_all(directory).unwrap();
    let cost = start.elapsed().as_secs_f64();
    eprintln!(
        "P2.5 growth max_ratio={largest_ratio:.3} max_output_factor={largest_output:.3} Debug_cost={cost:.3}s"
    );
    assert!(largest_ratio <= 8.0, "growth ratio {largest_ratio:.3}");
}
