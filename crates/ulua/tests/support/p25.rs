//! Rejected programs use stock tsc with the handoff's exact command options.
use std::{collections::BTreeSet, path::PathBuf, process::Command};
use ulua_compiler::{Limits, compile};
pub fn rejected_fields(cases: &[(&str, &[u32])]) {
    rejected_references("fields", cases);
}
pub fn rejected_references(label: &str, cases: &[(&str, &[u32])]) {
    rejected_programs(label, "C11", cases);
}
pub fn rejected_programs(label: &str, code: &str, cases: &[(&str, &[u32])]) {
    let directory =
        std::env::temp_dir().join(format!("ulua-p25-d18-{label}-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let paths: Vec<_> = cases
        .iter()
        .enumerate()
        .map(|(i, (source, _))| {
            let diagnostic = compile(source.as_bytes(), Limits::default()).unwrap_err();
            assert_eq!(diagnostic.code, code, "{source}: {diagnostic:?}");
            let path = directory.join(format!("field-{i}.ts"));
            std::fs::write(&path, source).unwrap();
            path
        })
        .collect();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let output = Command::new("node")
        .current_dir(&root)
        .args([
            "node_modules/typescript/bin/tsc",
            "--noEmit",
            "--strict",
            "--target",
            "es2022",
            "--module",
            "esnext",
            "--lib",
            "es2022",
            "prelude/ulua.d.ts",
        ])
        .args(&paths)
        .output()
        .unwrap();
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let mut codes = vec![BTreeSet::new(); cases.len()];
    for line in text.lines().filter(|line| line.contains("error TS")) {
        let id = (0..cases.len())
            .find(|i| line.contains(&format!("field-{i}.ts(")))
            .unwrap_or_else(|| panic!("Unattributed tsc diagnostic: {line}"));
        let code = line
            .split("error TS")
            .nth(1)
            .unwrap()
            .split(':')
            .next()
            .unwrap()
            .parse::<u32>()
            .unwrap();
        codes[id].insert(code);
    }
    for (i, ((source, expected), actual)) in cases.iter().zip(&codes).enumerate() {
        assert_eq!(
            *actual,
            expected.iter().copied().collect(),
            "field-{i}: {source}\n{text}"
        );
    }
    assert_eq!(
        output.status.success(),
        codes.iter().all(BTreeSet::is_empty),
        "{text}"
    );
    eprintln!(
        "P2.5 {label} {code} tsc: {} fixtures; codes={codes:?}",
        cases.len()
    );
    std::fs::remove_dir_all(directory).unwrap();
}
