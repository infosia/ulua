//! Signed differences price successive probes. Invalid endpoints withhold their layer.
use crate::{
    quantum,
    record::{Metadata, quote},
    stats::{Measurement, Statistics, checksums_match},
    subjects::{MAX_MEMORY_BYTES, MAX_STEPS},
};
use std::{fmt::Write, time::Duration};

pub(super) struct Cell {
    pub variant: &'static str,
    pub measurement: Measurement,
    pub statistics: Statistics,
    pub quantum: Option<Duration>,
}
impl Cell {
    pub(super) fn status(&self) -> &'static str {
        if !quantum::acceptable(self.quantum, self.statistics.median) {
            "clock-quantum"
        } else if self.statistics.noisy {
            "noise"
        } else {
            "ok"
        }
    }
    fn ns(&self) -> f64 {
        self.statistics.median * 1e6
    }
}
pub(super) struct Row {
    pub direction: &'static str,
    pub cells: Vec<Cell>,
}
fn layer(variant: &str) -> &'static str {
    match variant {
        "floor" => "VM floor",
        "accounting" => "step interrupts and charging allocator",
        "protected" => "protected wrapper",
        "conversion" => "numeric ABI conversion and direct typed callback (script only)",
        "trampoline" => "host trampoline, callback dispatch and call charge",
        "shim" => "remaining native shim",
        "ulua" => "rest: Rust ABI, guards and product setup/helpers",
        _ => unreachable!(),
    }
}
fn layers(row: &Row) -> Vec<(&'static str, Option<f64>, Option<f64>)> {
    let matched = checksums_match(
        &row.cells
            .iter()
            .map(|c| c.measurement.clone())
            .collect::<Vec<_>>(),
    );
    let total = row
        .cells
        .last()
        .filter(|c| c.variant == "ulua" && c.status() == "ok")
        .map(Cell::ns);
    row.cells
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let previous = i.checked_sub(1).map(|i| &row.cells[i]);
            let delta =
                if matched && c.status() == "ok" && previous.is_none_or(|c| c.status() == "ok") {
                    Some(c.ns() - previous.map_or(0.0, Cell::ns))
                } else {
                    None
                };
            (
                layer(c.variant),
                delta,
                delta.zip(total).map(|(d, total)| 100.0 * d / total),
            )
        })
        .collect()
}
fn candidate(row: &Row) -> (String, String) {
    let layers = layers(row);
    if layers.iter().any(|(_, _, share)| share.is_none()) {
        return (
            "shrink candidate withheld; a required variant failed the measurement policy.".into(),
            "null".into(),
        );
    }
    if let Some((name, _, Some(share))) = layers
        .iter()
        .skip(1)
        .filter(|(_, _, share)| share.is_some_and(|s| s > 0.0))
        .max_by(|a, b| a.2.unwrap_or_default().total_cmp(&b.2.unwrap_or_default()))
    {
        (
            format!(
                "shrink candidate: {name}, {share:.2}% of the product call. This is a probe estimate."
            ),
            format!("{{\"name\":{},\"share_percent\":{share}}}", quote(name)),
        )
    } else {
        (
            "the call is the floor at this measurement resolution.".into(),
            "{\"name\":\"the floor\"}".into(),
        )
    }
}
pub(super) fn render(metadata: &Metadata, rows: &[Row]) -> (String, String) {
    let mut table = format!(
        "Session: {}. Host ID: `{}`.\n\nMachine: {}. CPU: {}. Power: {}.\n\nLuau: `{}`. Ulua: `{}`; worktree dirty: {}.\n\nRust: {}. Release; O1; max_steps = {MAX_STEPS}; max_memory_bytes = {MAX_MEMORY_BYTES}; max_open_requests = 256.\n\nEach region has 1,000 boundary calls. Clock: std::time::Instant. Quantum: minimum positive span of 100,000 consecutive clock pairs.\n\n| Boundary | Variant | ns/call | Quantum ns | IQR/median | Status |\n|---|---|---:|---:|---:|---|\n",
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
    let mut json_rows = Vec::new();
    for row in rows {
        let matched = checksums_match(
            &row.cells
                .iter()
                .map(|c| c.measurement.clone())
                .collect::<Vec<_>>(),
        );
        let mut json_cells = Vec::new();
        for cell in &row.cells {
            let m = &cell.measurement;
            let s = &cell.statistics;
            let status = if matched {
                cell.status()
            } else {
                "checksum-mismatch"
            };
            let published = if status == "ok" {
                Some(cell.ns())
            } else {
                None
            };
            let quantum = cell.quantum.map(|q| q.as_nanos() as f64);
            let _ = writeln!(
                table,
                "| {} | {} | {} | {} | {:.2}% | {} |",
                row.direction,
                cell.variant,
                published.map_or("-".into(), |n| format!("{n:.3}")),
                optional(quantum),
                s.iqr / s.median * 100.0,
                status
            );
            let samples = m
                .samples
                .iter()
                .map(|sample| {
                    format!(
                        "{{\"checksum\":{},\"seconds\":{}}}",
                        sample.checksum, sample.seconds
                    )
                })
                .collect::<Vec<_>>()
                .join(",");
            json_cells.push(format!("{{\"variant\":{},\"status\":{},\"ns_per_call\":{},\"quantum_ns\":{},\"warmup\":{{\"iterations\":{},\"seconds\":{},\"checksum\":{},\"consistent\":{}}},\"observed\":{{\"min_seconds\":{},\"median_seconds\":{},\"max_seconds\":{},\"q1_seconds\":{},\"q3_seconds\":{},\"iqr_seconds\":{}}},\"samples\":[{samples}]}}", quote(cell.variant), quote(status), optional(published), optional(quantum), m.warmup_iterations, m.warmup_seconds, m.warmup_checksum, m.warmup_consistent, s.min, s.median, s.max, s.q1, s.q3, s.iqr));
        }
        json_rows.push(format!(
            "{{\"boundary\":{},\"checksums_match\":{matched},\"variants\":[{}],\"layers\":[{}],\"shrink_candidate\":{}}}",
            quote(row.direction),
            json_cells.join(",\n"),
            layers(row)
                .iter()
                .map(|(name, delta, share)| format!(
                    "{{\"name\":{},\"ns_per_call\":{},\"share_percent\":{}}}",
                    quote(name),
                    optional(*delta),
                    optional(*share)
                ))
                .collect::<Vec<_>>()
                .join(","),
            candidate(row).1
        ));
    }
    table.push_str("\nLayer prices use signed adjacent differences. Shares use the full product median as denominator. Negative prices retain process noise and interactions.\n\n| Boundary | Layer | ns/call | Product share |\n|---|---|---:|---:|\n");
    for row in rows {
        for (name, delta, share) in layers(row) {
            let _ = writeln!(
                table,
                "| {} | {name} | {} | {} |",
                row.direction,
                delta.map_or("-".into(), |n| format!("{n:.3}")),
                share.map_or("-".into(), |n| format!("{n:.2}%"))
            );
        }
    }
    for row in rows {
        let _ = writeln!(table, "\n{}: {}", row.direction, candidate(row).0);
        for c in &row.cells {
            let _ = writeln!(
                table,
                "- {} / {}: warm-up {:.3} ms, {} iterations; observed median {:.3} ns/call; {}.",
                row.direction,
                c.variant,
                c.measurement.warmup_seconds * 1e3,
                c.measurement.warmup_iterations,
                c.ns(),
                c.status()
            );
        }
    }
    let json = format!(
        "{{\"schema\":1,\"metadata\":{},\"policy\":{{\"clock\":\"std::time::Instant\",\"calls_per_region\":1000,\"clock_pairs\":100000,\"max_quantum_over_median\":0.01,\"samples\":11,\"warmup_min_seconds\":0.2,\"warmup_min_iterations\":3,\"iqr_limit\":0.15,\"quartile\":\"linear interpolation at (n-1)*p\",\"profile\":\"release\",\"luau_optimization_level\":1}},\"ulua_run_limits\":{{\"max_steps\":{MAX_STEPS},\"max_memory_bytes\":{MAX_MEMORY_BYTES},\"max_open_requests\":256}},\"boundaries\":[{}]}}\n",
        metadata.json(),
        json_rows.join(",\n")
    );
    (table, json)
}
fn optional(n: Option<f64>) -> String {
    n.map_or("null".into(), |n| n.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stats::Sample;

    fn cell(variant: &'static str, seconds: f64) -> Cell {
        let measurement = Measurement {
            warmup_iterations: 3,
            warmup_seconds: 0.2,
            warmup_checksum: 500500,
            warmup_consistent: true,
            samples: vec![
                Sample {
                    checksum: 500500,
                    seconds
                };
                11
            ],
        };
        Cell {
            variant,
            statistics: measurement.statistics().unwrap(),
            measurement,
            quantum: Some(Duration::from_nanos(1)),
        }
    }

    // Cost: 0.000106 s, Debug, measured locally; synthetic observations only, no process or boundary execution.
    #[test]
    fn layers_withhold_invalid_endpoints() {
        let start = std::time::Instant::now();
        let mut row = Row {
            direction: "host-into-script",
            cells: vec![
                cell("floor", 0.001),
                cell("accounting", 0.002),
                cell("ulua", 0.004),
            ],
        };
        let prices = layers(&row);
        assert_eq!(prices[1].1, Some(1000.0));
        assert_eq!(prices[1].2, Some(25.0));
        assert_eq!(prices.iter().map(|p| p.2.unwrap()).sum::<f64>(), 100.0);
        assert!(candidate(&row).1.contains("50"));
        row.cells[1].quantum = Some(Duration::from_micros(21));
        assert_eq!(row.cells[1].status(), "clock-quantum");
        assert!(layers(&row)[1..].iter().all(|p| p.1.is_none()));
        assert_eq!(candidate(&row).1, "null");
        row.cells[1].quantum = Some(Duration::from_nanos(1));
        row.cells[1].statistics.noisy = true;
        assert_eq!(row.cells[1].status(), "noise");
        assert!(layers(&row)[1..].iter().all(|p| p.1.is_none()));
        row.cells[1].statistics.noisy = false;
        row.cells[1].measurement.samples[3].checksum += 1;
        assert!(layers(&row).iter().all(|p| p.1.is_none()));
        eprintln!(
            "layers_withhold_invalid_endpoints cost={:.6}s",
            start.elapsed().as_secs_f64()
        );
    }
}
