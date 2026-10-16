//! Preserve raw observations and withhold published cells that fail the policy.
use crate::{
    stats::{Measurement, Statistics, checksums_match},
    subjects::{MAX_MEMORY_BYTES, MAX_STEPS},
};
use std::{fmt::Write, path::Path, process::Command};

pub(crate) struct Cell {
    pub subject: &'static str,
    pub measurement: Measurement,
    pub statistics: Statistics,
}
pub(crate) struct Row {
    pub id: String,
    pub cells: Vec<Cell>,
}
impl Row {
    fn matched(&self) -> bool {
        checksums_match(
            &self
                .cells
                .iter()
                .map(|c| c.measurement.clone())
                .collect::<Vec<_>>(),
        )
    }
    fn status(&self, cell: &Cell) -> &'static str {
        if !self.matched() {
            "checksum-mismatch"
        } else if cell.statistics.noisy {
            "noise"
        } else {
            "ok"
        }
    }
    fn ratio(&self) -> Option<f64> {
        if self.cells.len() != 2 || self.cells.iter().any(|c| self.status(c) != "ok") {
            return None;
        }
        Some(self.cells[1].statistics.median / self.cells[0].statistics.median)
    }
    fn json(&self) -> String {
        let cells = self.cells.iter().map(|c| {
            let m = &c.measurement;
            let s = &c.statistics;
            let status = self.status(c);
            let published = if status == "ok" { s.median.to_string() } else { "null".into() };
            let samples = m.samples.iter().map(|s| format!("{{\"checksum\":{},\"seconds\":{}}}", s.checksum, s.seconds)).collect::<Vec<_>>().join(",");
            format!("{{\"subject\":{},\"status\":{},\"median_seconds\":{published},\"warmup\":{{\"iterations\":{},\"seconds\":{},\"checksum\":{},\"consistent\":{}}},\"observed\":{{\"min_seconds\":{},\"median_seconds\":{},\"max_seconds\":{},\"q1_seconds\":{},\"q3_seconds\":{},\"iqr_seconds\":{},\"iqr_over_median\":{}}},\"samples\":[{samples}]}}",
                quote(c.subject), quote(status), m.warmup_iterations, m.warmup_seconds, m.warmup_checksum, m.warmup_consistent,
                s.min, s.median, s.max, s.q1, s.q3, s.iqr, s.iqr / s.median)
        }).collect::<Vec<_>>().join(",\n");
        let ratio = self.ratio().map_or("null".into(), |r| r.to_string());
        format!(
            "{{\"workload\":{},\"checksums_match\":{},\"ulua_over_luau\":{ratio},\"cells\":[\n{cells}\n]}}",
            quote(&self.id),
            self.matched()
        )
    }
}
pub(crate) fn quote(value: &str) -> String {
    let mut out = String::from("\"");
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            ch if ch < ' ' => {
                let _ = write!(out, "\\u{:04x}", ch as u32);
            }
            ch => out.push(ch),
        }
    }
    out.push('"');
    out
}
fn command(program: &str, arguments: &[&str], root: &Path) -> Result<String, String> {
    let output = Command::new(program)
        .args(arguments)
        .current_dir(root)
        .output()
        .map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err(format!("Command failed: {program}"));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}
// FNV-1a over the hostname bytes gives the same identity across Rust releases.
fn host_identity(hostname: &[u8]) -> String {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in hostname {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("host-{hash:016x}")
}

pub(crate) fn require_committed_crates(directory: &Path) -> Result<(), String> {
    let status = command(
        "git",
        &[
            "status",
            "--porcelain=v1",
            "-z",
            "--untracked-files=all",
            "--",
            ":(top)crates/",
        ],
        directory,
    )?;
    if !status.is_empty() {
        return Err("Changes under crates/: no file is written. Commit the measured code before a full run.".into());
    }
    Ok(())
}
pub(crate) struct Metadata {
    pub timestamp: String,
    pub host: String,
    pub machine: String,
    pub cpu: String,
    pub power: String,
    pub luau: String,
    pub ulua: String,
    pub dirty: bool,
    pub rustc: String,
}
impl Metadata {
    pub fn read(root: &Path) -> Result<Self, String> {
        // Keep a stable host identity without recording a user or device name.
        let host = host_identity(command("hostname", &[], root)?.as_bytes());
        let power = command("pmset", &["-g", "batt"], root)?
            .lines()
            .next()
            .ok_or("No power source")?
            .to_owned();
        let hardware = command("system_profiler", &["SPHardwareDataType"], root)?;
        let field = |key: &str| -> Option<String> {
            hardware.lines().find_map(|line| {
                line.trim()
                    .strip_prefix(key)
                    .map(str::trim)
                    .map(str::to_owned)
            })
        };
        let cpu = field("Chip:")
            .or_else(|| field("Processor Name:"))
            .ok_or("No CPU description")?;
        let model = field("Model Name:").ok_or("No machine model")?;
        let cores = field("Total Number of Cores:").unwrap_or_else(|| "unknown cores".into());
        Ok(Self {
            timestamp: command("date", &["-u", "+%Y-%m-%dT%H:%M:%SZ"], root)?,
            host,
            machine: format!("{}; {model}", command("uname", &["-srm"], root)?),
            cpu: format!("{cpu}; {cores} cores"),
            power,
            luau: std::fs::read_to_string(root.join("luau-revision.txt"))
                .map_err(|e| e.to_string())?
                .trim()
                .to_owned(),
            ulua: command("git", &["rev-parse", "HEAD"], root)?,
            dirty: !command("git", &["status", "--porcelain"], root)?.is_empty(),
            rustc: command("rustc", &["--version"], root)?,
        })
    }
    fn json(&self) -> String {
        format!(
            "{{\"timestamp_utc\":{},\"host_id\":{},\"host_hash\":\"fnv1a-64\",\"machine\":{},\"cpu\":{},\"power\":{},\"luau_revision\":{},\"ulua_commit\":{},\"worktree_dirty\":{},\"rustc\":{}}}",
            quote(&self.timestamp),
            quote(&self.host),
            quote(&self.machine),
            quote(&self.cpu),
            quote(&self.power),
            quote(&self.luau),
            quote(&self.ulua),
            self.dirty,
            quote(&self.rustc)
        )
    }
}
pub(crate) fn json(metadata: &Metadata, rows: &[Row]) -> String {
    let rows = rows.iter().map(Row::json).collect::<Vec<_>>().join(",\n");
    format!(
        "{{\n\"schema\":1,\"metadata\":{},\n\"policy\":{{\"clock\":\"std::time::Instant\",\"samples\":11,\"warmup_min_seconds\":0.2,\"warmup_min_iterations\":3,\"iqr_limit\":0.15,\"quartile\":\"linear interpolation at (n-1)*p\",\"profile\":\"release\",\"luau_optimization_level\":1,\"luau_interrupt_callback\":false,\"ulua_interrupt_callback\":true}},\n\"ulua_run_limits\":{{\"max_steps\":{MAX_STEPS},\"max_memory_bytes\":{MAX_MEMORY_BYTES},\"max_open_requests\":256}},\n\"workloads\":[\n{rows}\n]\n}}\n",
        metadata.json()
    )
}
pub(crate) fn table(metadata: &Metadata, rows: &[Row]) -> String {
    let mut out = format!(
        "Session: {}. Host ID: `{}` (FNV-1a 64-bit hostname hash).\n\nMachine: {}. CPU: {}. Power: {}.\n\nLuau: `{}`. Ulua: `{}`; worktree dirty: {}.\n\nRust: {}. Profile: Release. Luau: O1, no interrupt callback.\n\nUlua limits: max_steps = {MAX_STEPS}; max_memory_bytes = {MAX_MEMORY_BYTES}; max_open_requests = 256.\n\n| Workload | Checksum | luau ms (baseline) | ulua ms | ulua / luau |\n|---|---:|---:|---:|---:|\n",
        metadata.timestamp,
        metadata.host,
        metadata.machine,
        metadata.cpu,
        metadata.power,
        metadata.luau,
        metadata.ulua,
        metadata.dirty,
        metadata.rustc
    );
    for row in rows {
        let cells = row
            .cells
            .iter()
            .map(|c| {
                if row.status(c) == "ok" {
                    format!(
                        "{:.3}{}",
                        c.statistics.median * 1000.0,
                        if c.subject == "luau" { " (1.00×)" } else { "" }
                    )
                } else {
                    "-".into()
                }
            })
            .collect::<Vec<_>>();
        let ratio = row.ratio().map_or("-".into(), |r| format!("{r:.3}×"));
        let checksum = if row.matched() {
            row.cells[0].measurement.warmup_checksum.to_string()
        } else {
            "mismatch".into()
        };
        let label = if row.id == "callbacks" {
            "callbacks (Ulua methods; Luau loops)"
        } else {
            &row.id
        };
        let _ = writeln!(
            out,
            "| {label} | {checksum} | {} | {} | {ratio} |",
            cells[0], cells[1]
        );
    }
    out.push_str(
        "\nWarm-up and noise (IQR / median; a withheld cell has no published timing):\n\n",
    );
    for row in rows {
        for cell in &row.cells {
            let _ = writeln!(
                out,
                "- {} / {}: warm-up {:.3} ms, {} iterations; IQR {:.3} ms ({:.2}%); {}.",
                row.id,
                cell.subject,
                cell.measurement.warmup_seconds * 1000.0,
                cell.measurement.warmup_iterations,
                cell.statistics.iqr * 1000.0,
                cell.statistics.iqr / cell.statistics.median * 100.0,
                row.status(cell)
            );
        }
    }
    out
}

const START: &str = "<!-- results:start -->";
const END: &str = "<!-- results:end -->";
pub(crate) fn publish(
    directory: &Path,
    only: Option<&str>,
    json: &str,
    table: &str,
) -> Result<bool, String> {
    if only.is_some() {
        return Ok(false);
    }
    // Check again at publication, so changes during a measurement also prevent a record.
    require_committed_crates(directory)?;
    let readme = directory.join("README.md");
    let original = std::fs::read_to_string(&readme).map_err(|e| e.to_string())?;
    let start = original
        .find(START)
        .ok_or("README results start marker is absent")?
        + START.len();
    let end = original[start..]
        .find(END)
        .ok_or("README results end marker is absent")?
        + start;
    let updated = format!("{}\n\n{}\n{}", &original[..start], table, &original[end..]);
    std::fs::write(directory.join("results.json"), json).map_err(|e| e.to_string())?;
    std::fs::write(readme, updated).map_err(|e| e.to_string())?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stats::Sample;

    // Cost: 0.000150 s, Debug, measured locally; two fixed FNV-1a vectors, no machine metadata commands.
    #[test]
    fn host_identity_uses_fixed_fnv1a() {
        let start = std::time::Instant::now();
        assert_eq!(host_identity(b""), "host-cbf29ce484222325");
        assert_eq!(host_identity(b"hello"), "host-a430d84680aabd0b");
        eprintln!(
            "host_identity_uses_fixed_fnv1a cost={:.6}s",
            start.elapsed().as_secs_f64()
        );
    }
    // Cost: 0.000653 s, Debug, measured locally; synthetic observations, no benchmark execution.
    #[test]
    fn report_withholds_noisy_cell_and_mismatched_row() {
        let start = std::time::Instant::now();
        let m = Measurement {
            warmup_iterations: 3,
            warmup_seconds: 0.2,
            warmup_checksum: 7,
            warmup_consistent: true,
            samples: vec![
                Sample {
                    checksum: 7,
                    seconds: 1.0
                };
                11
            ],
        };
        let a = Cell {
            subject: "luau",
            statistics: m.statistics().unwrap(),
            measurement: m.clone(),
        };
        let b = Cell {
            subject: "ulua",
            statistics: m.statistics().unwrap(),
            measurement: m,
        };
        let mut row = Row {
            id: "seeded".into(),
            cells: vec![a, b],
        };
        assert_eq!(row.ratio(), Some(1.0));
        row.cells[1].statistics.noisy = true;
        assert_eq!(row.status(&row.cells[0]), "ok");
        assert_eq!(row.status(&row.cells[1]), "noise");
        assert!(row.ratio().is_none());
        row.cells[1].measurement.samples[5].checksum = 8;
        assert!(
            row.cells
                .iter()
                .all(|c| row.status(c) == "checksum-mismatch")
        );
        assert_eq!(row.json().matches("\"median_seconds\":null").count(), 2);
        assert_eq!(quote("\"\\\n"), "\"\\\"\\\\\\u000a\"");
        eprintln!(
            "report_withholds_noisy_cell_and_mismatched_row cost={:.6}s",
            start.elapsed().as_secs_f64()
        );
    }
}
