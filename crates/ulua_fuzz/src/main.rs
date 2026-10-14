//! `ulua-fuzz`: the in-house randomized robustness tool of pipeline.md §9 (D29).
//!
//! Usage:
//!
//! ```text
//! ulua-fuzz --target <compiler|run|abi|requests> [--seed N] [--inputs N | --seconds N]
//! ulua-fuzz --smoke
//! ulua-fuzz --target <name> --seed N --index I --emit-input <file>
//! ```
//!
//! The same seed and input count give the same inputs: input `i` of seed `s` comes from
//! a generator seeded with `mix(s, i)`. The supervisor runs the inputs in a worker process;
//! each failure goes to `target/fuzz/findings/` with its input, seed, and target, and the
//! first failure of each key is reduced (ddmin over bytes, then tokens).
//!
//! `--smoke` (the `sh tools/gate.sh sanitize` step) runs each target with the seed
//! [`SMOKE_SEED`] and the input counts of [`SMOKE`]: compiler 3,000, run 3,000, abi 2,000,
//! requests 1,500. Measured on the instrumented Debug build (macOS arm64): compiler 10 s,
//! run 10 s, abi 15 s, requests 20 s; each target stays below 60 s.
#![warn(missing_docs)]

mod check;
mod compile_target;
mod engine;
mod engine_calls;
mod engine_modules;
mod engine_nested;
#[cfg(test)]
mod engine_tests;
mod host;
mod model;
mod mutate;
mod rng;
mod run_target;
mod seeds;
mod shrink;
mod supervisor;
mod targets;
mod tokens;
mod witness;

use std::{
    io::Write,
    path::{Path, PathBuf},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use targets::Target;

/// The seed of `--smoke`.
pub const SMOKE_SEED: u64 = 0x5_2f0_22d;
/// The input count of each target under `--smoke`.
pub const SMOKE: [(Target, u64); 4] = [
    (Target::Compiler, 3000),
    (Target::Run, 3000),
    (Target::Abi, 2000),
    (Target::Requests, 1500),
];
/// The witness batch size of a worker.
const BATCH: usize = 16;
/// The replay budget of one reduction.
const SHRINK_BUDGET: usize = 1500;

#[derive(Default)]
struct Args {
    target: Option<Target>,
    seed: u64,
    inputs: Option<u64>,
    seconds: Option<u64>,
    smoke: bool,
    worker: bool,
    start: u64,
    end: u64,
    deadline_ms: Option<u128>,
    replay: Option<PathBuf>,
    index: u64,
    witness: bool,
    emit: Option<PathBuf>,
}

fn number(s: &str) -> Result<u64, String> {
    match s.strip_prefix("0x") {
        Some(h) => u64::from_str_radix(h, 16),
        None => s.parse(),
    }
    .map_err(|_| format!("not a number: {s}"))
}

fn parse() -> Result<Args, String> {
    let mut a = Args {
        seed: 1,
        end: u64::MAX,
        ..Args::default()
    };
    let mut it = std::env::args().skip(1);
    while let Some(arg) = it.next() {
        let mut value = || it.next().ok_or(format!("{arg} needs a value"));
        match arg.as_str() {
            "--target" => {
                let v = value()?;
                a.target = Some(Target::parse(&v).ok_or(format!("unknown target {v}"))?);
            }
            "--seed" => a.seed = number(&value()?)?,
            "--inputs" => a.inputs = Some(number(&value()?)?),
            "--seconds" => a.seconds = Some(number(&value()?)?),
            "--smoke" => a.smoke = true,
            "--worker" => a.worker = true,
            "--start" => a.start = number(&value()?)?,
            "--end" => a.end = number(&value()?)?,
            "--deadline-ms" => a.deadline_ms = Some(u128::from(number(&value()?)?)),
            "--replay" => a.replay = Some(PathBuf::from(value()?)),
            "--index" => a.index = number(&value()?)?,
            "--witness" => a.witness = true,
            "--emit-input" => a.emit = Some(PathBuf::from(value()?)),
            _ => return Err(format!("unknown argument {arg}")),
        }
    }
    Ok(a)
}

fn root() -> PathBuf {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    std::fs::canonicalize(&root).unwrap_or(root)
}

fn env(root: &Path) -> Result<compile_target::Env, String> {
    let text = std::fs::read_to_string(root.join("specs/blocks/collisions.md"))
        .map_err(|e| format!("collisions.md: {e}"))?;
    Ok(compile_target::Env {
        rules: check::rule_ids(&text),
    })
}

fn line(text: &str) -> String {
    text.replace('\n', "\\n")
}

fn emit(out: &mut impl Write, index: u64, f: &check::Failure) {
    let _ = writeln!(out, "F {index} {}\t{}", f.key, line(&f.detail));
}

/// Run one input in this process and print its failures (the protocol of a worker).
// The worker protocol names each part of one input run; a struct adds no meaning.
#[allow(clippy::too_many_arguments)]
fn one(
    out: &mut impl Write,
    target: Target,
    env: &compile_target::Env,
    input: &[u8],
    seed: u64,
    index: u64,
    witness: bool,
    samples: &mut Vec<witness::Sample>,
) {
    let _ = writeln!(out, "B {index}");
    let _ = out.flush();
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        targets::execute(target, env, input, seed, index, witness)
    }));
    match result {
        Ok(o) => {
            for f in &o.report.failures {
                emit(out, index, f);
            }
            if let Some(s) = o.sample {
                let _ = writeln!(out, "S {index}");
                samples.push(s);
            }
            let _ = writeln!(
                out,
                "R {index} {} {} {} {}",
                u8::from(o.accepted),
                o.nanos,
                input.len(),
                if target.source() { "c" } else { "-" }
            );
        }
        Err(_) => {
            let f = check::Failure {
                key: "panic".into(),
                detail: "a Rust panic in the input run".into(),
            };
            emit(out, index, &f);
            let _ = writeln!(out, "R {index} 0 0 {} -", input.len());
        }
    }
    let _ = out.flush();
}

fn flush_samples(out: &mut impl Write, root: &Path, samples: &mut Vec<witness::Sample>) -> bool {
    match witness::check(&witness::Tools::new(root), samples) {
        Ok(checked) => {
            for (index, f) in checked.failures {
                emit(out, index, &f);
            }
            for (index, detail) in checked.limits {
                let _ = writeln!(out, "L {index} {}", line(&detail));
            }
            samples.clear();
            true
        }
        Err(e) => {
            let _ = writeln!(out, "E {}", line(&e));
            false
        }
    }
}

fn worker(a: &Args) -> Result<(), String> {
    let root = root();
    let target = a.target.ok_or("--worker needs --target")?;
    let pool = seeds::Pool::load(&root)?;
    let env = env(&root)?;
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    let mut samples = vec![];
    let mut index = a.start;
    while index < a.end {
        if let Some(d) = a.deadline_ms {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_or(0, |t| t.as_millis());
            if now >= d {
                break;
            }
        }
        let input = targets::input(target, &pool, a.seed, index);
        let witness = target.source() && targets::sampled(a.seed, index);
        one(
            &mut out,
            target,
            &env,
            &input,
            a.seed,
            index,
            witness,
            &mut samples,
        );
        if samples.len() >= BATCH && !flush_samples(&mut out, &root, &mut samples) {
            return Ok(());
        }
        index += 1;
    }
    if flush_samples(&mut out, &root, &mut samples) {
        for (k, n) in check::take_stats() {
            let _ = writeln!(out, "C {n} {k}");
        }
        let _ = writeln!(out, "D");
    }
    Ok(())
}

fn replay(a: &Args) -> Result<(), String> {
    let root = root();
    let target = a.target.ok_or("--replay needs --target")?;
    let path = a.replay.as_ref().ok_or("--replay needs a file")?;
    let input = std::fs::read(path).map_err(|e| e.to_string())?;
    let env = env(&root)?;
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    let mut samples = vec![];
    one(
        &mut out,
        target,
        &env,
        &input,
        a.seed,
        a.index,
        a.witness,
        &mut samples,
    );
    if flush_samples(&mut out, &root, &mut samples) {
        let _ = writeln!(out, "D");
    }
    Ok(())
}

/// Run one target and print its summary line. Returns the failure count.
fn fuzz(
    target: Target,
    seed: u64,
    inputs: Option<u64>,
    seconds: Option<u64>,
) -> Result<usize, String> {
    let root = root();
    let started = Instant::now();
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let w = supervisor::Worker {
        program: exe,
        prefix: vec![],
    };
    let end = inputs.unwrap_or(if seconds.is_some() { u64::MAX } else { 1000 });
    let run = supervisor::Run {
        start: 0,
        end,
        seconds: seconds.map(Duration::from_secs),
        input_deadline: supervisor::INPUT_DEADLINE,
    };
    let pool = seeds::Pool::load(&root)?;
    let rec = supervisor::Recorder {
        root: &root,
        pool: &pool,
        target,
        seed,
    };
    let summary = supervisor::supervise(&w, &root, target, seed, &run, Some(&rec))?;
    // In a time box, reduction ends with the grace period after the box.
    let until = seconds.map(|s| started + Duration::from_secs(s) + supervisor::GRACE);
    let dirs = supervisor::reduce(&w, &rec, &summary, SHRINK_BUDGET, until)?;
    for (index, detail) in &summary.limits {
        eprintln!("ulua-fuzz: witness limit, input {index}: {detail}");
    }
    for (index, per_byte, median) in supervisor::slow(&summary.timings) {
        eprintln!(
            "ulua-fuzz: review: input {index} of {} seed {seed} compiles in {per_byte:.0} ns per byte, median {median:.0}",
            target.name()
        );
    }
    for (index, f) in &summary.failures {
        eprintln!(
            "ulua-fuzz: FAIL {} input {index}: {} {}",
            target.name(),
            f.key,
            f.detail.lines().next().unwrap_or("")
        );
    }
    for d in &dirs {
        eprintln!("ulua-fuzz: finding {}", d.display());
    }
    let stats: String = summary
        .stats
        .iter()
        .map(|(k, n)| format!("{n} {k}\n"))
        .collect();
    let stats_path = root.join(format!("target/fuzz/stats-{}-{seed}.txt", target.name()));
    std::fs::write(&stats_path, stats).map_err(|e| e.to_string())?;
    println!(
        "ulua-fuzz target={} seed={seed} inputs={} accepted={} witness_samples={} witness_limits={} failures={} observed={} cost={:.1}s",
        target.name(),
        summary.inputs,
        summary.accepted,
        summary.samples,
        summary.limits.len(),
        summary.failures.len(),
        summary.stats.len(),
        supervisor::cost(started)
    );
    Ok(summary.failures.len())
}

fn run() -> Result<bool, String> {
    let a = parse()?;
    if a.worker {
        worker(&a)?;
        return Ok(true);
    }
    if let Some(path) = &a.emit {
        // Write input `--index` of a seed, for review of a reported input.
        let target = a.target.ok_or("--emit-input needs --target")?;
        let pool = seeds::Pool::load(&root())?;
        let input = targets::input(target, &pool, a.seed, a.index);
        std::fs::write(path, input).map_err(|e| e.to_string())?;
        return Ok(true);
    }
    if a.replay.is_some() {
        replay(&a)?;
        return Ok(true);
    }
    if a.smoke {
        let mut failures = 0;
        for (target, count) in SMOKE {
            failures += fuzz(target, SMOKE_SEED, Some(count), None)?;
        }
        return Ok(failures == 0);
    }
    let target = a.target.ok_or(
        "usage: ulua-fuzz --target <compiler|run|abi|requests> [--seed N] [--inputs N | --seconds N] | --smoke",
    )?;
    Ok(fuzz(target, a.seed, a.inputs, a.seconds)? == 0)
}

fn main() {
    match run() {
        Ok(true) => {}
        Ok(false) => std::process::exit(1),
        Err(e) => {
            eprintln!("ulua-fuzz: {e}");
            std::process::exit(2);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn a_panic_in_an_input_is_a_failure() {
        let env = env(&root()).unwrap();
        let mut out = vec![];
        targets::INJECT_PANIC.with(|c| c.set(true));
        one(
            &mut out,
            Target::Compiler,
            &env,
            b"x",
            1,
            7,
            false,
            &mut vec![],
        );
        let text = String::from_utf8(out).unwrap();
        assert!(text.contains("F 7 panic\t"), "{text}");
        let mut out = vec![];
        one(
            &mut out,
            Target::Compiler,
            &env,
            b"export function main(): void {}",
            1,
            8,
            false,
            &mut vec![],
        );
        let text = String::from_utf8(out).unwrap();
        assert!(!text.contains("F 8"), "{text}");
        assert!(text.contains("R 8 1 "), "{text}");
    }
    #[test]
    fn the_same_seed_gives_the_same_inputs() {
        let pool = seeds::Pool::load(&root()).unwrap();
        for t in Target::ALL {
            let a: Vec<_> = (0..40).map(|i| targets::input(t, &pool, 9, i)).collect();
            let b: Vec<_> = (0..40).map(|i| targets::input(t, &pool, 9, i)).collect();
            let c: Vec<_> = (0..40).map(|i| targets::input(t, &pool, 10, i)).collect();
            assert_eq!(a, b);
            assert_ne!(a, c);
        }
    }
}
