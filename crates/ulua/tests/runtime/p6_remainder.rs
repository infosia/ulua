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
#[cfg(feature = "remainder-shadow")]
use std::path::PathBuf;
use std::time::Instant;
#[cfg(feature = "remainder-shadow")]
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}
#[cfg(feature = "remainder-shadow")]
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
// Cost: 0.406 s, Debug; 104 corpus entries, with one extra compile for index counts.
#[test]
#[cfg(feature = "remainder-shadow")]
fn shadow_corpus_accept_and_trap() {
    shadow();
    let start = Instant::now();
    let mut count = 0;
    let mut index_checks = 0;
    let mut runtime_stops = 0;
    for directory in ["accept", "trap"] {
        let mut paths: Vec<_> = std::fs::read_dir(root().join("corpus").join(directory))
            .unwrap()
            .map(|e| e.unwrap().path())
            .filter(|p| p.extension().is_some_and(|e| e == "ts"))
            .collect();
        paths.sort();
        for path in paths {
            let source = std::fs::read_to_string(&path).unwrap();
            let luau = ulua_compiler::compile_with_grants(
                source.as_bytes(),
                Default::default(),
                &requests::signatures(&source),
            )
            .unwrap()
            .luau;
            index_checks += luau
                .lines()
                .filter(|line| line.contains("then error(\"proven index shadow mismatch\") end"))
                .count();
            let expected = std::fs::read_to_string(path.with_extension("expected")).unwrap();
            let (status, code, output, _) = requests::run(&source, Default::default());
            runtime_stops += usize::from(code == "internal");
            assert_eq!(runtime_stops, 0, "{}: {code}", path.display());
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
    assert!(index_checks > 0);
    eprintln!(
        "shadow corpus entries={count} index_checks={index_checks} runtime_stops={runtime_stops} cost={:.3}s",
        start.elapsed().as_secs_f64()
    );
}

#[test]
fn native_remainder_keeps_statement_instruction_bounds() {
    // The shadow helper has test-only calls outside the production statement count.
    #[cfg(feature = "remainder-shadow")]
    if std::env::var_os("ULUA_REMAINDER_SHADOW").is_some() {
        eprintln!("skip instruction bounds: test-only remainder shadow helper is active");
        return;
    }
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

// Cost: 0.08 s, Debug; two VM traces of 128 read/write groups.
#[test]
fn proven_indices_keep_statement_instruction_bounds() {
    #[cfg(feature = "remainder-shadow")]
    if std::env::var_os("ULUA_REMAINDER_SHADOW").is_some() {
        return;
    }
    let _measurement = crate::measurement_lock();
    let mut totals = Vec::new();
    for (setup, index, checked) in [("", "0", 0), ("let p=b?0:0;", "p", 384)] {
        let body = format!("a[{index}]+=1;a[1]=a[{index}];s+=a[1];").repeat(128);
        let source = format!(
            "export function main(b:boolean):number{{{setup}let a=[0,0];let s=0;{body}return s;}}"
        );
        let code = ulua_compiler::compile(source.as_bytes(), Default::default())
            .unwrap()
            .luau;
        let tests: Vec<_> = code
            .lines()
            .filter(|line| {
                (line.contains(" < #__ulua_") || line.contains(" <= #__ulua_"))
                    && (line.contains("then __ulua_trap(1) end")
                        || line.contains(": boolean = not ("))
            })
            .collect();
        assert_eq!(tests.len(), 640);
        assert_eq!(
            tests
                .iter()
                .filter(|line| line.contains("__ulua_floor"))
                .count(),
            checked
        );
        assert_eq!(
            tests.iter().filter(|line| line.contains(">= 0")).count(),
            checked
        );
        totals.push(super::p6_expressions::instructions(&code, 8256.0));
    }
    assert!(
        totals[0] < totals[1],
        "proven={} checked={}",
        totals[0],
        totals[1]
    );
    eprintln!(
        "proven index instructions={} checked index instructions={}",
        totals[0], totals[1]
    );
}

// Cost: two short ABI runs, each compared with its committed golden.
#[test]
fn proven_index_corpus_edges_match_goldens() {
    for (source, expected) in [
        (
            include_str!("../../../../corpus/accept/a87-proven-index-edges.ts"),
            include_str!("../../../../corpus/accept/a87-proven-index-edges.expected"),
        ),
        (
            include_str!("../../../../corpus/trap/t17-proven-index-at-length.ts"),
            include_str!("../../../../corpus/trap/t17-proven-index-at-length.expected"),
        ),
    ] {
        let (status, code, output, _) = requests::run(source, Default::default());
        if let Some((output_before_trap, trap)) = expected.rsplit_once("trap:") {
            assert_eq!(status, ulua::ULUA_FAILED);
            assert_eq!(code, format!("trap:{}", trap.trim()));
            assert_eq!(output, output_before_trap);
        } else {
            assert_eq!((status, code.as_str()), (ulua::ULUA_OK, ""));
            assert_eq!(output, expected);
        }
    }
}
