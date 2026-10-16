//! P6.1: the product C ABI against the pinned native Luau VM.
#![warn(missing_docs)]
mod report;
mod stats;
mod subjects;
#[cfg(test)]
mod tests;

use report::{Cell, Metadata, Row};
use stats::Measurement;
use std::{
    path::{Path, PathBuf},
    process::Command,
};

pub(crate) const IDS: [&str; 10] = [
    "fib-recursive",
    "fib-loop",
    "mandelbrot",
    "primes",
    "sort",
    "tree",
    "queen",
    "particles",
    "callbacks",
    "collect",
];
const SUBJECTS: [&str; 2] = ["luau", "ulua"];

#[derive(Debug, PartialEq)]
pub(crate) enum Mode {
    Full,
    Only(String),
    Cell(String, String),
    Help,
}
impl Mode {
    pub fn parse(args: &[String]) -> Result<Self, String> {
        match args {
            [] => Ok(Self::Full),
            [help] if help == "--help" => Ok(Self::Help),
            [flag, id] if flag == "--only" && IDS.contains(&id.as_str()) => {
                Ok(Self::Only(id.clone()))
            }
            [flag, id, subject]
                if flag == "--cell"
                    && IDS.contains(&id.as_str())
                    && SUBJECTS.contains(&subject.as_str()) =>
            {
                Ok(Self::Cell(id.clone(), subject.clone()))
            }
            _ => Err("usage: bench [--only <workload-id>]".into()),
        }
    }
    fn only(&self) -> Option<&str> {
        match self {
            Self::Only(id) => Some(id),
            _ => None,
        }
    }
}
fn directory() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn cell(directory: &Path, id: &str, subject: &str) -> Result<Measurement, String> {
    let (folder, extension) = if subject == "ulua" {
        ("ulua", "ts")
    } else {
        ("luau", "luau")
    };
    let source = std::fs::read(
        directory
            .join("workloads")
            .join(folder)
            .join(format!("{id}.{extension}")),
    )
    .map_err(|e| e.to_string())?;
    match subject {
        "ulua" => subjects::measure(&mut subjects::Ulua::new(&source)?, &mut subjects::Monotonic),
        "luau" => subjects::measure(&mut subjects::Luau::new(&source)?, &mut subjects::Monotonic),
        _ => Err("Unknown subject".into()),
    }
}
fn child(
    executable: &Path,
    directory: &Path,
    id: &str,
    subject: &'static str,
) -> Result<Cell, String> {
    let output = Command::new(executable)
        .args(["--cell", id, subject])
        .current_dir(directory)
        .output()
        .map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err(format!(
            "{id} / {subject}: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let measurement =
        Measurement::parse(&String::from_utf8(output.stdout).map_err(|e| e.to_string())?)?;
    let statistics = measurement.statistics()?;
    eprintln!(
        "{id} / {subject}: observed median {:.3} ms, IQR {:.2}%; warm-up {:.3} ms / {} iterations",
        statistics.median * 1000.0,
        statistics.iqr / statistics.median * 100.0,
        measurement.warmup_seconds * 1000.0,
        measurement.warmup_iterations
    );
    Ok(Cell {
        subject,
        measurement,
        statistics,
    })
}
fn run() -> Result<(), String> {
    let mode = Mode::parse(&std::env::args().skip(1).collect::<Vec<_>>())?;
    if mode == Mode::Help {
        println!(
            "usage: bench [--only <workload-id>]\nWorkloads: {}",
            IDS.join(", ")
        );
        return Ok(());
    }
    if cfg!(debug_assertions) {
        return Err("Use the Release build: cargo run --release -p ulua_bench --bin bench".into());
    }
    let directory = directory();
    if let Mode::Cell(id, subject) = &mode {
        print!("{}", cell(&directory, id, subject)?.protocol());
        return Ok(());
    }
    if mode.only().is_some() {
        eprintln!("--only: report goes to stdout; no file is written.");
    } else {
        // Reject changed inputs before the full measurement, then check again before publication.
        report::require_committed_crates(&directory)?;
    }
    let metadata = Metadata::read(&directory.join("../.."))?;
    let executable = std::env::current_exe().map_err(|e| e.to_string())?;
    let ids: Vec<_> = mode.only().map_or_else(|| IDS.to_vec(), |id| vec![id]);
    let mut rows = Vec::with_capacity(ids.len());
    let mut mismatch = false;
    for id in ids {
        let mut cells = Vec::with_capacity(SUBJECTS.len());
        for subject in SUBJECTS {
            eprintln!("Measure {id} / {subject}");
            cells.push(child(&executable, &directory, id, subject)?);
        }
        mismatch |= !stats::checksums_match(
            &cells
                .iter()
                .map(|c| c.measurement.clone())
                .collect::<Vec<_>>(),
        );
        rows.push(Row {
            id: id.into(),
            cells,
        });
    }
    let json = report::json(&metadata, &rows);
    let table = report::table(&metadata, &rows);
    print!("{table}");
    if report::publish(&directory, mode.only(), &json, &table)? {
        eprintln!("Full suite: wrote results.json and the README table.");
    } else {
        print!("\n{json}");
    }
    if mismatch {
        return Err(
            "Checksum mismatch: all timings for each affected workload are withheld".into(),
        );
    }
    Ok(())
}
fn main() {
    if let Err(error) = run() {
        eprintln!("bench: {error}");
        std::process::exit(1);
    }
}
