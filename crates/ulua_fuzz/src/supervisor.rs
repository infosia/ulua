//! The supervisor runs the inputs in a worker process, so a sanitizer report, a signal,
//! or an abort fails the input that was running and the run continues after it. It then
//! writes the findings and reduces each one with replay processes.
use crate::{
    check::Failure,
    seeds::Pool,
    shrink::{Budget, shrink},
    targets::{self, Target},
};
use std::{
    collections::BTreeMap,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

/// The summary of one supervised run.
#[derive(Default, Debug)]
pub struct Summary {
    /// Inputs run.
    pub inputs: u64,
    /// Inputs with an accepted compile.
    pub accepted: u64,
    /// Inputs checked by the witnesses.
    pub samples: u64,
    /// Failures: input index and failure.
    pub failures: Vec<(u64, Failure)>,
    /// Compile time and source bytes of each input, for the slow-compile report.
    pub timings: Vec<(u64, f64, usize)>,
    /// Observation counts: a call and its status or code.
    pub stats: BTreeMap<String, u64>,
}

/// A worker command line: the binary and its leading arguments.
pub struct Worker {
    /// The program.
    pub program: PathBuf,
    /// Arguments before the worker arguments.
    pub prefix: Vec<String>,
}

fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_millis())
}

/// The key of a process that stopped abnormally, from its status and stderr.
pub fn crash_key(status: &std::process::ExitStatus, stderr: &str) -> String {
    if let Some(line) = stderr
        .lines()
        .find(|l| l.contains("ERROR: AddressSanitizer:"))
    {
        let kind = line
            .split("AddressSanitizer:")
            .nth(1)
            .and_then(|r| r.split_whitespace().next())
            .unwrap_or("report");
        return format!("crash:asan:{kind}");
    }
    if stderr.contains("runtime error:") {
        return "crash:ubsan".into();
    }
    if stderr.contains("panicked at") {
        return "crash:panic".into();
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        if let Some(sig) = status.signal() {
            return format!("crash:signal:{sig}");
        }
    }
    format!("crash:exit:{}", status.code().unwrap_or(-1))
}

fn tail(text: &str) -> String {
    let lines: Vec<_> = text.lines().collect();
    lines[lines.len().saturating_sub(40)..].join("\n")
}

/// Run inputs `start..end` of a target until done or past the deadline, restarting the
/// worker after each crash.
pub fn supervise(
    worker: &Worker,
    root: &Path,
    target: Target,
    seed: u64,
    start: u64,
    end: u64,
    deadline: Option<Duration>,
) -> Result<Summary, String> {
    let deadline_ms = deadline.map(|d| now_ms() + d.as_millis());
    let tmp = root.join("target/fuzz/tmp");
    std::fs::create_dir_all(&tmp).map_err(|e| e.to_string())?;
    let mut summary = Summary::default();
    let mut next = start;
    while next < end {
        if deadline_ms.is_some_and(|d| now_ms() >= d) {
            break;
        }
        static FILES: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let n = FILES.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let err_path = tmp.join(format!("worker-{}-{n}.stderr", std::process::id()));
        let err_file = std::fs::File::create(&err_path).map_err(|e| e.to_string())?;
        let mut cmd = Command::new(&worker.program);
        cmd.args(&worker.prefix).args([
            "--worker",
            "--target",
            target.name(),
            "--seed",
            &seed.to_string(),
            "--start",
            &next.to_string(),
            "--end",
            &end.to_string(),
        ]);
        if let Some(d) = deadline_ms {
            cmd.args(["--deadline-ms", &d.to_string()]);
        }
        let mut child = cmd
            .stdout(Stdio::piped())
            .stderr(err_file)
            .spawn()
            .map_err(|e| format!("worker: {e}"))?;
        let stdout = child.stdout.take().ok_or("worker stdout")?;
        let mut current = None;
        let mut done = false;
        for line in BufReader::new(stdout).lines() {
            let line = line.map_err(|e| e.to_string())?;
            let mut parts = line.splitn(2, ' ');
            let tag = parts.next().unwrap_or("");
            let rest = parts.next().unwrap_or("");
            match tag {
                "B" => current = rest.parse::<u64>().ok(),
                "R" => {
                    let f: Vec<_> = rest.split(' ').collect();
                    let index: u64 = f.first().and_then(|x| x.parse().ok()).unwrap_or(0);
                    summary.inputs += 1;
                    summary.accepted += u64::from(f.get(1) == Some(&"1"));
                    let nanos: f64 = f.get(2).and_then(|x| x.parse().ok()).unwrap_or(0.0);
                    let bytes: f64 = f.get(3).and_then(|x| x.parse().ok()).unwrap_or(0.0);
                    if f.get(1) == Some(&"1") || f.get(4) == Some(&"c") {
                        summary.timings.push((index, nanos, bytes as usize));
                    }
                    current = None;
                }
                "S" => summary.samples += 1,
                "C" => {
                    let (n, key) = rest.split_once(' ').unwrap_or(("0", rest));
                    *summary.stats.entry(key.to_owned()).or_default() +=
                        n.parse::<u64>().unwrap_or(0);
                }
                "F" => {
                    let (index, failure) = rest.split_once(' ').unwrap_or((rest, ""));
                    let (key, detail) = failure.split_once('\t').unwrap_or((failure, ""));
                    summary.failures.push((
                        index.parse().unwrap_or(u64::MAX),
                        Failure {
                            key: key.to_owned(),
                            detail: detail.replace("\\n", "\n"),
                        },
                    ));
                }
                "E" => return Err(format!("worker: {rest}")),
                "D" => done = true,
                _ => {}
            }
        }
        let status = child.wait().map_err(|e| e.to_string())?;
        let stderr = std::fs::read_to_string(&err_path).unwrap_or_default();
        let _ = std::fs::remove_file(&err_path);
        let report = stderr.contains("Sanitizer") || stderr.contains("runtime error:");
        if done && status.success() && !report {
            break;
        }
        let key = crash_key(&status, &stderr);
        match current {
            Some(i) => {
                summary.inputs += 1;
                summary.failures.push((
                    i,
                    Failure {
                        key,
                        detail: tail(&stderr),
                    },
                ));
                next = i + 1;
            }
            None if done || report => {
                // A report after the last input: the worker cannot name the input.
                summary.failures.push((
                    u64::MAX,
                    Failure {
                        key,
                        detail: tail(&stderr),
                    },
                ));
                break;
            }
            None => {
                return Err(format!(
                    "worker stopped outside an input: {}",
                    tail(&stderr)
                ));
            }
        }
    }
    Ok(summary)
}

/// Write each failure under `target/fuzz/findings/` and reduce the first failure of each
/// key. Returns the finding directories.
pub fn findings(
    worker: &Worker,
    root: &Path,
    pool: &Pool,
    target: Target,
    seed: u64,
    summary: &Summary,
    budget: usize,
) -> Result<Vec<PathBuf>, String> {
    let base = root.join("target/fuzz/findings");
    let mut first: BTreeMap<String, ()> = BTreeMap::new();
    let mut out = vec![];
    for (index, failure) in &summary.failures {
        let dir = base.join(format!(
            "{}-{seed}-{}-{}",
            target.name(),
            if *index == u64::MAX {
                "none".into()
            } else {
                index.to_string()
            },
            sanitize(&failure.key)
        ));
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let meta = format!(
            "target={}\nseed={seed}\nindex={index}\nkey={}\n",
            target.name(),
            failure.key
        );
        std::fs::write(dir.join("meta.txt"), meta).map_err(|e| e.to_string())?;
        std::fs::write(dir.join("failure.txt"), &failure.detail).map_err(|e| e.to_string())?;
        out.push(dir.clone());
        if *index == u64::MAX {
            continue;
        }
        let input = targets::input(target, pool, seed, *index);
        let path = dir.join("input.bin");
        std::fs::write(&path, &input).map_err(|e| e.to_string())?;
        if first.insert(failure.key.clone(), ()).is_some() {
            continue;
        }
        let witness = failure.key.starts_with("witness:");
        let probe = dir.join("candidate.bin");
        let mut test = |candidate: &[u8]| {
            std::fs::write(&probe, candidate).is_ok()
                && replay(worker, target, seed, *index, &probe, witness)
                    .is_ok_and(|keys| keys.contains(&failure.key))
        };
        if !test(&input) {
            std::fs::write(
                dir.join("not-reproduced.txt"),
                "the replay of the input passes",
            )
            .map_err(|e| e.to_string())?;
            continue;
        }
        let small = shrink(&input, &mut test, &mut Budget(budget));
        let _ = std::fs::remove_file(&probe);
        std::fs::write(dir.join("shrunk.bin"), &small).map_err(|e| e.to_string())?;
        eprintln!(
            "ulua-fuzz: finding {} reduced from {} to {} bytes: {}",
            failure.key,
            input.len(),
            small.len(),
            dir.display()
        );
    }
    Ok(out)
}

fn sanitize(key: &str) -> String {
    key.chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .take(48)
        .collect()
}

/// Replay one input file in a fresh process. Returns the failure keys, a crash included.
pub fn replay(
    worker: &Worker,
    target: Target,
    seed: u64,
    index: u64,
    path: &Path,
    witness: bool,
) -> Result<Vec<String>, String> {
    let mut cmd = Command::new(&worker.program);
    cmd.args(&worker.prefix).args([
        "--replay",
        &path.display().to_string(),
        "--target",
        target.name(),
        "--seed",
        &seed.to_string(),
        "--index",
        &index.to_string(),
    ]);
    if witness {
        cmd.arg("--witness");
    }
    let out = cmd.output().map_err(|e| e.to_string())?;
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    let mut keys: Vec<String> = stdout
        .lines()
        .filter_map(|l| l.strip_prefix("F "))
        .filter_map(|l| l.split_once(' '))
        .map(|(_, f)| f.split('\t').next().unwrap_or("").to_owned())
        .collect();
    let done = stdout.lines().any(|l| l == "D");
    if !done || !out.status.success() || stderr.contains("Sanitizer") {
        keys.push(crash_key(&out.status, &stderr));
    }
    Ok(keys)
}

/// The smallest source of the slow-compile report (pipeline.md §9.3).
pub const SLOW_MIN_BYTES: usize = 1024;

/// The sources of at least [`SLOW_MIN_BYTES`] whose compile time per source byte exceeds
/// 1,000 times the median per byte of the inputs of at least that size in the same run
/// (pipeline.md §9.3): a report for review, not a failure. Smaller sources are outside
/// the metric. Each entry: index, time per byte, median.
pub fn slow(timings: &[(u64, f64, usize)]) -> Vec<(u64, f64, f64)> {
    let large: Vec<(u64, f64)> = timings
        .iter()
        .filter(|t| t.2 >= SLOW_MIN_BYTES)
        .map(|t| (t.0, t.1 / t.2 as f64))
        .collect();
    if large.is_empty() {
        return vec![];
    }
    let mut v: Vec<f64> = large.iter().map(|t| t.1).collect();
    v.sort_by(f64::total_cmp);
    let median = v[v.len() / 2];
    large
        .iter()
        .filter(|t| median > 0.0 && t.1 > 1000.0 * median)
        .map(|t| (t.0, t.1, median))
        .collect()
}

/// Seconds since `start`, for the summary line.
pub fn cost(start: Instant) -> f64 {
    start.elapsed().as_secs_f64()
}

#[cfg(test)]
mod tests {
    use super::*;
    fn sh(script: &str) -> Worker {
        Worker {
            program: "sh".into(),
            prefix: vec!["-c".into(), script.into(), "worker".into()],
        }
    }
    fn root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
    }
    #[test]
    fn a_worker_signal_fails_the_running_input() {
        // A seeded crash: the fake worker starts input 3, then aborts.
        let w = sh("echo 'R 2 1 10 5'; echo 'B 3'; kill -ABRT $$");
        let s = supervise(&w, &root(), Target::Compiler, 1, 2, 4, None);
        let s = s.unwrap();
        assert!(
            s.failures
                .iter()
                .any(|(i, f)| *i == 3 && f.key.starts_with("crash:signal"))
        );
    }
    #[test]
    fn a_sanitizer_report_fails_the_running_input() {
        let w = sh(
            "echo 'B 5'; echo '==1==ERROR: AddressSanitizer: heap-buffer-overflow on address' >&2; exit 1",
        );
        let s = supervise(&w, &root(), Target::Compiler, 1, 5, 6, None).unwrap();
        assert_eq!(s.failures.len(), 1);
        assert_eq!(s.failures[0].1.key, "crash:asan:heap-buffer-overflow");
    }
    #[test]
    fn a_reported_failure_is_kept() {
        let w =
            sh("echo 'B 0'; echo 'F 0 internal:ulua_compile\tdetail'; echo 'R 0 0 1 1'; echo D");
        let s = supervise(&w, &root(), Target::Compiler, 1, 0, 1, None).unwrap();
        assert_eq!(s.failures[0].1.key, "internal:ulua_compile");
        assert_eq!(s.inputs, 1);
    }
    #[test]
    fn slow_compiles_are_reported() {
        // 100 inputs of 2 KiB at 1 ns per byte, then seeded slow inputs.
        let mut t: Vec<_> = (0..100).map(|i| (i, 2048.0, 2048)).collect();
        t.push((100, 2048.0 * 5000.0, 2048));
        t.push((101, 1024.0 * 2000.0, 1024));
        // Outside the metric: the empty source and a slow source of 100 bytes.
        t.push((102, 1.0e6, 0));
        t.push((103, 100.0 * 1.0e6, 100));
        assert_eq!(slow(&t), vec![(100, 5000.0, 1.0), (101, 2000.0, 1.0)]);
        assert!(slow(&t[..100]).is_empty());
        assert!(slow(&t[102..]).is_empty());
    }
}
