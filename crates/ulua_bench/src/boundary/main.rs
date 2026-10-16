//! P6.2: measurement-only boundary prices and successive layer ablations.
#![warn(missing_docs)]
#[path = "../measurement.rs"]
mod measurement;
mod quantum;
#[path = "../record.rs"]
mod record;
mod report;
#[path = "../stats.rs"]
mod stats;
mod subjects;

use record::Metadata;
use stats::Measurement;
use std::{path::PathBuf, process::Command, time::Duration};

const DIRECTIONS: [&str; 2] = ["host-into-script", "script-into-host"];
const VARIANTS: [&str; 7] = [
    "floor",
    "accounting",
    "protected",
    "conversion",
    "trampoline",
    "shim",
    "ulua",
];
fn variants(script: bool) -> impl Iterator<Item = &'static str> {
    VARIANTS
        .into_iter()
        .filter(move |v| script || *v != "trampoline")
}
fn measure(script: bool, variant: &str) -> Result<String, String> {
    let quantum = quantum::observe().map_or(0, |d| d.as_nanos());
    let measurement = if variant == "ulua" {
        measurement::measure(
            &mut subjects::Product::new(script)?,
            &mut measurement::Monotonic,
        )?
    } else {
        let level = VARIANTS
            .iter()
            .position(|v| *v == variant)
            .ok_or("Unknown variant")?;
        measurement::measure(
            &mut subjects::Native::new(script, level as i32)?,
            &mut measurement::Monotonic,
        )?
    };
    Ok(format!("{quantum}\n{}", measurement.protocol()))
}
fn run() -> Result<(), String> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args == ["--help"] {
        println!(
            "usage: boundary [--stdout | --only <host-into-script|script-into-host>]\nDefault: record both boundaries from clean crates. --stdout and --only write no file."
        );
        return Ok(());
    }
    if cfg!(debug_assertions) {
        return Err("Use cargo run --release -p ulua_bench --bin boundary".into());
    }
    if let [flag, direction, variant] = args.as_slice()
        && flag == "--cell"
        && DIRECTIONS.contains(&direction.as_str())
        && variants(direction == DIRECTIONS[1]).any(|v| v == variant)
    {
        print!("{}", measure(direction == DIRECTIONS[1], variant)?);
        return Ok(());
    }
    let (only, stdout) = match args.as_slice() {
        [] => (None, false),
        [flag] if flag == "--stdout" => (None, true),
        [flag, id] if flag == "--only" && DIRECTIONS.contains(&id.as_str()) => {
            (Some(id.as_str()), true)
        }
        _ => return Err("usage: boundary [--stdout | --only <boundary-id>]".into()),
    };
    let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    if stdout {
        eprintln!("stdout mode: no file is written.");
    } else {
        record::require_committed_crates(&directory)?;
    }
    let metadata = Metadata::read(&directory.join("../.."))?;
    let executable = std::env::current_exe().map_err(|e| e.to_string())?;
    let mut rows = Vec::new();
    for direction in DIRECTIONS
        .into_iter()
        .filter(|id| only.is_none_or(|only| only == *id))
    {
        let mut cells = Vec::new();
        for variant in variants(direction == DIRECTIONS[1]) {
            eprintln!("Measure {direction} / {variant}");
            let output = Command::new(&executable)
                .args(["--cell", direction, variant])
                .current_dir(&directory)
                .output()
                .map_err(|e| e.to_string())?;
            if !output.status.success() {
                return Err(format!(
                    "{direction}/{variant}: {}",
                    String::from_utf8_lossy(&output.stderr)
                ));
            }
            let output = String::from_utf8(output.stdout).map_err(|e| e.to_string())?;
            let (quantum, protocol) = output.split_once('\n').ok_or("Missing quantum")?;
            let ns: u64 = quantum.parse().map_err(|_| "Invalid quantum")?;
            let measurement = Measurement::parse(protocol)?;
            let statistics = measurement.statistics()?;
            let cell = report::Cell {
                variant,
                measurement,
                statistics,
                quantum: (ns > 0).then(|| Duration::from_nanos(ns)),
            };
            eprintln!(
                "{direction}/{variant}: {:.3} ns/call; IQR {:.2}%; quantum {ns} ns; {}",
                cell.statistics.median * 1e6,
                cell.statistics.iqr / cell.statistics.median * 100.0,
                cell.status()
            );
            cells.push(cell);
        }
        rows.push(report::Row { direction, cells });
    }
    let (table, json) = report::render(&metadata, &rows);
    print!("{table}");
    if record::publish(
        &directory,
        stdout.then_some("stdout"),
        &json,
        &table,
        "boundary.json",
        "<!-- boundary:start -->",
        "<!-- boundary:end -->",
    )? {
        eprintln!("Full boundary run: wrote boundary.json and the README section.");
    } else {
        print!("\n{json}");
    }
    Ok(())
}
fn main() {
    if let Err(error) = run() {
        eprintln!("boundary: {error}");
        std::process::exit(1);
    }
}
