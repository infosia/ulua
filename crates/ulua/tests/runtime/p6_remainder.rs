//! Golden corpus checks and one correctness run of every workload under the test-only shadow option.
#[allow(dead_code)]
#[path = "../../../ulua_bench/src/measurement.rs"]
mod measurement;
#[allow(dead_code)]
#[path = "../support/requests.rs"]
mod requests;
#[allow(dead_code)]
#[path = "../../../ulua_bench/src/stats.rs"]
mod stats;
use std::{path::PathBuf, time::Instant};
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn shadow() {
    assert!(
        std::env::var_os("ULUA_REMAINDER_SHADOW").is_some(),
        "Set the test-only shadow option"
    );
    let code = ulua_compiler::compile(
        b"export function main():number{return 7%3;}",
        Default::default(),
    )
    .unwrap()
    .luau;
    assert!(
        code.contains("return __ulua_native_remainder(")
            || code.matches("__ulua_native_remainder(").count() >= 2,
        "{code}"
    );
}
#[test]
fn a68_remainder_edges_match_the_golden() {
    let start = Instant::now();
    let source = include_str!("../../../../corpus/accept/a68-native-remainder-edges.ts");
    let (status, code, output, _) = requests::run(source, Default::default());
    assert_eq!((status, code.as_str()), (ulua::ULUA_OK, ""));
    assert_eq!(
        output,
        include_str!("../../../../corpus/accept/a68-native-remainder-edges.expected")
    );
    eprintln!(
        "a68 remainder edges cost={:.6}s",
        start.elapsed().as_secs_f64()
    );
}
#[test]
#[ignore = "Run with ULUA_REMAINDER_SHADOW=1; this option exists only in test builds"]
fn shadow_corpus_accept_and_trap() {
    shadow();
    let start = Instant::now();
    let mut count = 0;
    for directory in ["accept", "trap"] {
        let mut paths: Vec<_> = std::fs::read_dir(root().join("corpus").join(directory))
            .unwrap()
            .map(|e| e.unwrap().path())
            .filter(|p| p.extension().is_some_and(|e| e == "ts"))
            .collect();
        paths.sort();
        for path in paths {
            let source = std::fs::read_to_string(&path).unwrap();
            let expected = std::fs::read_to_string(path.with_extension("expected")).unwrap();
            let (status, code, output, _) = requests::run(&source, Default::default());
            if directory == "accept" {
                assert_eq!(
                    (status, code.as_str()),
                    (ulua::ULUA_OK, ""),
                    "{}",
                    path.display()
                );
                assert_eq!(output, expected, "{}", path.display());
            } else {
                let (golden, trap) = expected.rsplit_once("trap:").unwrap();
                assert_eq!(status, ulua::ULUA_FAILED, "{}", path.display());
                assert_eq!(code, format!("trap:{}", trap.trim()), "{}", path.display());
                assert_eq!(output, golden, "{}", path.display());
            }
            count += 1;
        }
    }
    eprintln!(
        "shadow corpus entries={count} stops=0 cost={:.3}s",
        start.elapsed().as_secs_f64()
    );
}

#[test]
fn native_remainder_keeps_statement_instruction_bounds() {
    let _measurement = crate::measurement_lock();
    let start = Instant::now();
    let body = "x=(x*1664525+1013904223)%4294967296;x%=4294967296;".repeat(128);
    let source = format!("export function main():number{{let x=1;{body}return x;}}");
    let code = ulua_compiler::compile(source.as_bytes(), Default::default())
        .unwrap()
        .luau;
    assert_eq!(code.matches(" % ").count(), 256);
    let mut expected = 1u64;
    for _ in 0..128 {
        expected = (expected * 1664525 + 1013904223) % 4294967296;
    }
    super::p6_expressions::instructions(&code, expected as f64);
    eprintln!(
        "native remainder instruction bounds cost={:.6}s",
        start.elapsed().as_secs_f64()
    );
}
