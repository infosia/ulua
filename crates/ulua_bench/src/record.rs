//! Shared machine metadata and clean-crates publication policy.
use std::{fmt::Write, path::Path, process::Command};

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
    pub(crate) fn json(&self) -> String {
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
pub(crate) fn publish(
    directory: &Path,
    only: Option<&str>,
    json: &str,
    table: &str,
    filename: &str,
    start_marker: &str,
    end_marker: &str,
) -> Result<bool, String> {
    if only.is_some() {
        return Ok(false);
    }
    // Check again at publication, so changes during a measurement also prevent a record.
    require_committed_crates(directory)?;
    let readme = directory.join("README.md");
    let original = std::fs::read_to_string(&readme).map_err(|e| e.to_string())?;
    let start = original
        .find(start_marker)
        .ok_or("README results start marker is absent")?
        + start_marker.len();
    let end = original[start..]
        .find(end_marker)
        .ok_or("README results end marker is absent")?
        + start;
    let updated = format!("{}\n\n{}\n{}", &original[..start], table, &original[end..]);
    std::fs::write(directory.join(filename), json).map_err(|e| e.to_string())?;
    std::fs::write(readme, updated).map_err(|e| e.to_string())?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
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
}
