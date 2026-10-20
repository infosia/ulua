//! Compare the normal CLI process with the timed ABI in an independent test graph.
//! Run with ULUA_CLI_EMIT_BINARY set to the existing binary and a separate Cargo target directory.
//! Use `cargo test --manifest-path crates/ulua_cli/tests/emit_comparison/Cargo.toml --offline`.
//! This target alone enables ulua/timing. It does not depend on ulua_cli.
#![warn(missing_docs)]
use std::{path::Path, process::Command, ptr, sync::Mutex, time::Instant};
use ulua::{
    compile_measurement::{CompileEvent, Observer, replace_observer},
    *,
};

static RUST_TEXT: Mutex<Option<String>> = Mutex::new(None);
fn observe(event: CompileEvent<'_>) {
    if let CompileEvent::Rust(text) = event
        && let Ok(mut capture) = RUST_TEXT.lock()
    {
        *capture = Some(text.to_owned());
    }
}
struct ObserverOwner(Option<Observer>);
impl Drop for ObserverOwner {
    fn drop(&mut self) {
        replace_observer(self.0);
    }
}
struct ContextOwner(*mut Context);
impl Drop for ContextOwner {
    fn drop(&mut self) {
        // SAFETY: This context is live, serialized, and freed once after all ABI calls finish.
        unsafe {
            ulua_context_free(self.0);
        }
    }
}
#[test]
fn accept_corpus_matches_abi_rust_observation() {
    let start = Instant::now();
    let binary = std::env::var_os("ULUA_CLI_EMIT_BINARY")
        .expect("set ULUA_CLI_EMIT_BINARY to the already built CLI");
    assert!(Path::new(&binary).is_file());
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../..");
    let mut paths: Vec<_> = std::fs::read_dir(root.join("corpus/accept"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "ts"))
        .collect();
    paths.sort();
    // Install on this caller thread. ulua_compile forwards the observer to its own worker thread.
    let _observer = ObserverOwner(replace_observer(Some(observe)));
    let mut count = 0;
    for path in paths {
        let source = std::fs::read(&path).unwrap();
        if source
            .split(|byte| *byte == b'\n')
            .any(|line| line.starts_with(b"// grants:"))
        {
            continue;
        }
        *RUST_TEXT.lock().unwrap() = None;
        let mut context = ptr::null_mut();
        let mut module = ptr::null_mut();
        let mut diagnostic = Diagnostic::default();
        // SAFETY: Source and output storage remain live. No grants or custom limits are supplied.
        unsafe {
            assert_eq!(ulua_context_new(&mut context), ULUA_OK);
            let _context = ContextOwner(context);
            assert_eq!(
                ulua_compile(
                    context,
                    source.as_ptr().cast(),
                    source.len(),
                    ptr::null(),
                    0,
                    ptr::null(),
                    &mut module,
                    &mut diagnostic
                ),
                ULUA_OK,
                "{}",
                path.display()
            );
            assert_eq!(ulua_module_free(module), ULUA_OK);
        }
        let observed = RUST_TEXT
            .lock()
            .unwrap()
            .take()
            .expect("ABI must emit one Rust observation");
        let emitted = Command::new(&binary)
            .arg("emit")
            .arg(&path)
            .output()
            .unwrap();
        assert!(
            emitted.status.success(),
            "{}: {}",
            path.display(),
            String::from_utf8_lossy(&emitted.stderr)
        );
        assert!(emitted.stderr.is_empty());
        assert_eq!(emitted.stdout, observed.as_bytes(), "{}", path.display());
        count += 1;
    }
    assert!(count > 0);
    eprintln!(
        "emit ABI comparison entries={count} cost={:.3}s",
        start.elapsed().as_secs_f64()
    );
}
