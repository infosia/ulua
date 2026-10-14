//! The supervisor runs the inputs in a worker process, so a sanitizer report, a signal,
//! an abort, or a hang fails the input that was running and the run continues after it.
//! It writes each finding when it occurs, then reduces each one with replay processes.
use crate::{
    check::Failure,
    seeds::Pool,
    shrink::{Budget, shrink},
    targets::{self, Target},
    witness::run_with_deadline,
};
use std::{
    collections::BTreeMap,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::mpsc::{RecvTimeoutError, channel},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

/// The deadline of one input (pipeline.md §9.3).
pub const INPUT_DEADLINE: Duration = Duration::from_secs(30);
/// The time after the end of a time box that a worker gets for its last witness batch.
pub const GRACE: Duration = Duration::from_secs(90);

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
    /// Witness resource limits: input index and description.
    pub limits: Vec<(u64, String)>,
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

/// Writes each finding record when it occurs.
pub struct Recorder<'a> {
    /// The repository root.
    pub root: &'a Path,
    /// The seed programs, to regenerate an input.
    pub pool: &'a Pool,
    /// The target.
    pub target: Target,
    /// The seed.
    pub seed: u64,
}

fn sanitize(key: &str) -> String {
    key.chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .take(48)
        .collect()
}

impl Recorder<'_> {
    /// The record directory of one input and key.
    pub fn dir(&self, index: u64, key: &str) -> PathBuf {
        let at = if index == u64::MAX {
            "none".into()
        } else {
            index.to_string()
        };
        self.root.join("target/fuzz/findings").join(format!(
            "{}-{}-{at}-{}",
            self.target.name(),
            self.seed,
            sanitize(key)
        ))
    }
    /// Write the input, seed, target, and failure of one finding (pipeline.md §9.4).
    pub fn failure(&self, index: u64, f: &Failure) -> Result<PathBuf, String> {
        self.write(index, &f.key, &f.detail, "failure")
    }
    /// Write a witness resource limit with its input; it is not a failure (§9.3).
    pub fn limit(&self, index: u64, detail: &str) -> Result<PathBuf, String> {
        self.write(index, "witness-limit", detail, "limit")
    }
    fn write(&self, index: u64, key: &str, detail: &str, kind: &str) -> Result<PathBuf, String> {
        let dir = self.dir(index, key);
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let meta = format!(
            "kind={kind}\ntarget={}\nseed={}\nindex={index}\nkey={key}\n",
            self.target.name(),
            self.seed
        );
        std::fs::write(dir.join("meta.txt"), meta).map_err(|e| e.to_string())?;
        std::fs::write(dir.join(format!("{kind}.txt")), detail).map_err(|e| e.to_string())?;
        if index != u64::MAX {
            let input = targets::input(self.target, self.pool, self.seed, index);
            std::fs::write(dir.join("input.bin"), input).map_err(|e| e.to_string())?;
        }
        Ok(dir)
    }
}

fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_millis())
}

/// The key of a process that stopped abnormally: a sanitizer report first, then the
/// signal, then a panic message, then the exit code.
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
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        if let Some(sig) = status.signal() {
            return format!("crash:signal:{sig}");
        }
    }
    if stderr.contains("panicked at") {
        return "crash:panic".into();
    }
    format!("crash:exit:{}", status.code().unwrap_or(-1))
}

fn tail(text: &str) -> String {
    let lines: Vec<_> = text.lines().collect();
    lines[lines.len().saturating_sub(40)..].join("\n")
}

/// The limits of one supervised run.
pub struct Run {
    /// The first input.
    pub start: u64,
    /// The end of the inputs, exclusive.
    pub end: u64,
    /// The time box, if any.
    pub seconds: Option<Duration>,
    /// The deadline of each input.
    pub input_deadline: Duration,
}

/// Run the inputs of a target until done or past the time box, restarting the worker
/// after each crash or hang. Each failure and limit goes to `record` when it occurs.
pub fn supervise(
    worker: &Worker,
    root: &Path,
    target: Target,
    seed: u64,
    run: &Run,
    record: Option<&Recorder>,
) -> Result<Summary, String> {
    let deadline_ms = run.seconds.map(|d| now_ms() + d.as_millis());
    let tmp = root.join("target/fuzz/tmp");
    std::fs::create_dir_all(&tmp).map_err(|e| e.to_string())?;
    let mut summary = Summary::default();
    let add = |summary: &mut Summary, index: u64, f: Failure| {
        if let Some(r) = record {
            let _ = r.failure(index, &f);
        }
        summary.failures.push((index, f));
    };
    let mut next = run.start;
    while next < run.end {
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
            &run.end.to_string(),
        ]);
        if let Some(d) = deadline_ms {
            cmd.args(["--deadline-ms", &d.to_string()]);
        }
        let mut child = cmd
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(err_file)
            .spawn()
            .map_err(|e| format!("worker: {e}"))?;
        let stdout = child.stdout.take().ok_or("worker stdout")?;
        let (tx, rx) = channel();
        // The reader thread ends with the pipe; the supervisor never blocks on it.
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                if line.is_err() || tx.send(line.unwrap_or_default()).is_err() {
                    break;
                }
            }
        });
        let mut current = None;
        let mut began = Instant::now();
        let mut done = false;
        let mut hung = false;
        let mut overrun = false;
        loop {
            let line = match rx.recv_timeout(Duration::from_millis(50)) {
                Ok(line) => line,
                Err(RecvTimeoutError::Timeout) => {
                    if current.is_some() && began.elapsed() >= run.input_deadline {
                        hung = true;
                    } else if deadline_ms.is_some_and(|d| now_ms() >= d + GRACE.as_millis()) {
                        overrun = true;
                    } else {
                        continue;
                    }
                    let _ = child.kill();
                    break;
                }
                Err(RecvTimeoutError::Disconnected) => break,
            };
            let mut parts = line.splitn(2, ' ');
            let tag = parts.next().unwrap_or("");
            let rest = parts.next().unwrap_or("");
            match tag {
                "B" => {
                    current = rest.parse::<u64>().ok();
                    began = Instant::now();
                }
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
                    let f = Failure {
                        key: key.to_owned(),
                        detail: detail.replace("\\n", "\n"),
                    };
                    add(&mut summary, index.parse().unwrap_or(u64::MAX), f);
                }
                "L" => {
                    let (index, detail) = rest.split_once(' ').unwrap_or((rest, ""));
                    let index = index.parse().unwrap_or(u64::MAX);
                    if let Some(r) = record {
                        let _ = r.limit(index, detail);
                    }
                    summary.limits.push((index, detail.to_owned()));
                }
                "E" => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(format!("worker: {rest}"));
                }
                "D" => done = true,
                _ => {}
            }
        }
        let status = child.wait().map_err(|e| e.to_string())?;
        let stderr = std::fs::read_to_string(&err_path).unwrap_or_default();
        let _ = std::fs::remove_file(&err_path);
        if overrun {
            eprintln!("ulua-fuzz: the worker passed the end of the time box; the run ends");
            break;
        }
        let report = stderr.contains("Sanitizer") || stderr.contains("runtime error:");
        if done && status.success() && !report {
            break;
        }
        let key = if hung {
            "hang".to_owned()
        } else {
            crash_key(&status, &stderr)
        };
        let detail = if hung {
            format!("the input passed its deadline of {:?}", run.input_deadline)
        } else {
            tail(&stderr)
        };
        match current {
            Some(i) => {
                summary.inputs += 1;
                add(&mut summary, i, Failure { key, detail });
                next = i + 1;
            }
            None if done || report => {
                // A report after the last input: the worker cannot name the input.
                add(&mut summary, u64::MAX, Failure { key, detail });
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

/// Reduce the first failure of each key (pipeline.md §9.4). The record states whether the
/// reduction reached a 1-minimal input or stopped at its budget or at the time box.
pub fn reduce(
    worker: &Worker,
    rec: &Recorder,
    summary: &Summary,
    budget: usize,
    until: Option<Instant>,
) -> Result<Vec<PathBuf>, String> {
    let mut first: BTreeMap<String, ()> = BTreeMap::new();
    let mut out = vec![];
    for (index, failure) in &summary.failures {
        let dir = rec.dir(*index, &failure.key);
        out.push(dir.clone());
        if *index == u64::MAX || first.insert(failure.key.clone(), ()).is_some() {
            continue;
        }
        let input = targets::input(rec.target, rec.pool, rec.seed, *index);
        let witness = failure.key.starts_with("witness:");
        let probe = dir.join("candidate.bin");
        let mut timed_out = false;
        let mut test = |candidate: &[u8]| {
            if until.is_some_and(|u| Instant::now() >= u) {
                timed_out = true;
                return false;
            }
            std::fs::write(&probe, candidate).is_ok()
                && replay(worker, rec.target, rec.seed, *index, &probe, witness)
                    .is_ok_and(|keys| keys.contains(&failure.key))
        };
        if !test(&input) {
            let why = if timed_out {
                "reduction=not-run (the time box ended)\n"
            } else {
                "reduction=not-reproduced (the replay of the input passes)\n"
            };
            std::fs::write(dir.join("reduction.txt"), why).map_err(|e| e.to_string())?;
            continue;
        }
        let mut b = Budget(budget);
        let small = shrink(&input, &mut test, &mut b);
        let _ = std::fs::remove_file(&probe);
        let state = if timed_out {
            "reduction=stopped (the time box ended)".to_owned()
        } else if b.0 == 0 {
            format!("reduction=stopped (the budget of {budget} replays)")
        } else {
            "reduction=1-minimal (bytes and tokens)".to_owned()
        };
        std::fs::write(dir.join("shrunk.bin"), &small).map_err(|e| e.to_string())?;
        std::fs::write(
            dir.join("reduction.txt"),
            format!("{state}\nbytes={} to {}\n", input.len(), small.len()),
        )
        .map_err(|e| e.to_string())?;
        eprintln!(
            "ulua-fuzz: finding {} reduced from {} to {} bytes, {state}: {}",
            failure.key,
            input.len(),
            small.len(),
            dir.display()
        );
    }
    Ok(out)
}

/// Replay one input file in a fresh process. Returns the failure keys, a crash or a hang
/// included.
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
    // A witness replay can run two witness deadlines after the input deadline.
    let deadline = INPUT_DEADLINE
        + if witness {
            4 * crate::witness::DEADLINE
        } else {
            Duration::ZERO
        };
    let Some(out) = run_with_deadline(&mut cmd, deadline)? else {
        return Ok(vec!["hang".into()]);
    };
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
    fn run(start: u64, end: u64) -> Run {
        Run {
            start,
            end,
            seconds: None,
            input_deadline: INPUT_DEADLINE,
        }
    }
    #[test]
    fn an_input_that_never_ends_is_a_hang_and_the_run_continues() {
        // A seeded hang: the stub worker starts input 7 and never ends it.
        let w = sh("echo 'B 7'; exec sleep 60");
        let mut r = run(7, 8);
        r.input_deadline = Duration::from_millis(500);
        let s = supervise(&w, &root(), Target::Compiler, 1, &r, None).unwrap();
        assert_eq!(s.failures.len(), 1);
        assert_eq!((s.failures[0].0, s.failures[0].1.key.as_str()), (7, "hang"));
    }
    #[test]
    fn an_abort_with_a_panic_message_uses_the_platform_crash_key() {
        let w = sh("echo 'B 1'; echo \"thread 'main' panicked at x\" >&2; kill -ABRT $$");
        let s = supervise(&w, &root(), Target::Compiler, 1, &run(1, 2), None).unwrap();
        // Windows exposes an exit code, so the panic message precedes the exit fallback.
        let expected = if cfg!(windows) {
            "crash:panic"
        } else {
            "crash:signal:6"
        };
        assert_eq!(s.failures.len(), 1);
        assert_eq!(
            (s.failures[0].0, s.failures[0].1.key.as_str()),
            (1, expected)
        );
    }
    #[test]
    fn a_worker_signal_fails_the_running_input() {
        // A seeded crash: the fake worker starts input 3, then aborts.
        let w = sh("echo 'R 2 1 10 5'; echo 'B 3'; kill -ABRT $$");
        let s = supervise(&w, &root(), Target::Compiler, 1, &run(2, 4), None);
        let s = s.unwrap();
        assert_eq!(s.failures.len(), 1);
        assert_eq!(s.failures[0].0, 3);
        // Git Bash reports SIGABRT (6) as Windows exit code 6 << 8, without a signal.
        if cfg!(windows) {
            assert_eq!(s.failures[0].1.key, "crash:exit:1536");
        } else {
            assert_eq!(s.failures[0].1.key, "crash:signal:6");
        }
    }
    #[test]
    fn a_sanitizer_report_fails_the_running_input() {
        let w = sh(
            "echo 'B 5'; echo '==1==ERROR: AddressSanitizer: heap-buffer-overflow on address' >&2; exit 1",
        );
        let s = supervise(&w, &root(), Target::Compiler, 1, &run(5, 6), None).unwrap();
        assert_eq!(s.failures.len(), 1);
        assert_eq!(s.failures[0].1.key, "crash:asan:heap-buffer-overflow");
    }
    #[test]
    fn a_reported_failure_is_kept() {
        let w =
            sh("echo 'B 0'; echo 'F 0 internal:ulua_compile\tdetail'; echo 'R 0 0 1 1'; echo D");
        let s = supervise(&w, &root(), Target::Compiler, 1, &run(0, 1), None).unwrap();
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
