//! Emit stream, artifact, bounded-input, and rejection checks use the real entry point.
use super::*;

#[test]
fn stdout_file_and_usage() {
    let scratch = Scratch::new("emit-streams");
    let part = if cfg!(unix) { "\u{202e}\n" } else { "\u{202e}" };
    let file = scratch.script(
        &format!("input{part}.ts"),
        "export function main():void{throw new Error('not executed');}",
    );
    let output = scratch
        .0
        .join(format!("output{part}.luau"))
        .into_os_string();
    let (code, luau, err) = invoke(&["emit".into(), file.clone()]);
    assert_eq!(code, 0, "{err}");
    assert!(err.is_empty());
    assert!(luau.starts_with("--!strict\n"));
    // This script traps under run. Emission must still succeed without executing main.
    let ran = invoke(&["run".into(), file.clone()]);
    assert_eq!(ran.0, 1);
    assert!(ran.2.contains("not executed"));
    let (code, out, err) = invoke(&["emit".into(), file.clone(), "-o".into(), output.clone()]);
    assert_eq!(code, 0, "{err}");
    assert!(out.is_empty());
    assert_eq!(std::fs::read(&output).unwrap(), luau.as_bytes());
    // Options can precede the input file, with the same artifact and status line.
    assert_eq!(
        invoke(&["emit".into(), "-o".into(), output.clone(), file.clone()]),
        (code, out.clone(), err.clone())
    );
    // An emitted artifact is not a second input language for the command.
    assert_eq!(invoke(&["check".into(), output.clone()]).0, 1);
    assert_eq!(
        err,
        format!(
            "emit: {}: {}\n",
            diagnostic::filename(&file),
            diagnostic::filename(&output)
        )
    );
    for args in [
        vec!["emit"],
        vec!["emit", "-o", "out"],
        vec!["emit", "x", "-o"],
        vec!["emit", "x", "--bad"],
        vec!["emit", "x", "-o", "out", "extra"],
    ] {
        let (code, out, err) = call(&args);
        assert_eq!(code, 2, "{args:?}");
        assert!(out.is_empty());
        assert!(err.starts_with("ulua: emit:"));
        assert!(err.ends_with(USAGE));
    }
}
#[test]
fn reject_corpus_matches_check_and_preserves_outputs() {
    let start = Instant::now();
    let scratch = Scratch::new("emit-rejects");
    let output = scratch.0.join("keep.luau");
    std::fs::write(&output, b"existing artifact").unwrap();
    let mut count = 0;
    // Compare every rejection, including grants fixtures: both commands always pass an empty grant list.
    for file in scripts(&root().join("corpus/reject")) {
        let file = file.into_os_string();
        let checked = invoke(&["check".into(), file.clone()]);
        assert_eq!(checked.0, 1, "{}", file.display());
        assert!(checked.1.is_empty());
        assert_eq!(
            invoke(&["emit".into(), file.clone()]),
            checked,
            "{}",
            file.display()
        );
        assert_eq!(
            invoke(&[
                "emit".into(),
                file,
                "-o".into(),
                output.clone().into_os_string()
            ]),
            checked
        );
        assert_eq!(std::fs::read(&output).unwrap(), b"existing artifact");
        count += 1;
    }
    let rejected = scratch.script("reject.ts", "const x:any=1;");
    let absent = scratch.0.join("missing-parent/output.luau");
    let checked = invoke(&["check".into(), rejected.clone()]);
    assert_eq!(
        invoke(&[
            "emit".into(),
            rejected,
            "-o".into(),
            absent.clone().into_os_string()
        ]),
        checked
    );
    assert!(!absent.exists());
    eprintln!(
        "emit reject entries={count} cost={:.3}s",
        start.elapsed().as_secs_f64()
    );
}
#[test]
fn bounds_invalid_bytes_and_io_errors() {
    let scratch = Scratch::new("emit-errors");
    let file = scratch.0.join("invalid.ts");
    for source in [
        vec![0xff; 70_000],
        b"\xff\r\nconst x:any=1;".to_vec(),
        b"export function main():void{hostLog('x');}".to_vec(),
    ] {
        std::fs::write(&file, source).unwrap();
        let checked = invoke(&["check".into(), file.clone().into_os_string()]);
        assert_eq!(checked.0, 1);
        assert_eq!(
            invoke(&["emit".into(), file.clone().into_os_string()]),
            checked
        );
    }
    let file = scratch.script("valid.ts", "export function main():void{}");
    let output = scratch.0.join("missing/output.luau");
    let (code, out, err) = invoke(&[
        "emit".into(),
        file.clone(),
        "-o".into(),
        output.clone().into_os_string(),
    ]);
    assert_eq!(code, 2);
    assert!(out.is_empty());
    assert!(err.contains(&output.display().to_string()));
    let missing = scratch.0.join("absent.ts").into_os_string();
    assert_eq!(
        invoke(&["emit".into(), missing.clone()]),
        invoke(&["check".into(), missing])
    );
    struct Broken;
    impl Write for Broken {
        fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
            Err(std::io::Error::other("broken"))
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut err = Vec::new();
    assert_eq!(
        execute([OsString::from("emit"), file], &mut Broken, &mut err),
        2
    );
    assert_eq!(String::from_utf8(err).unwrap(), "stdout: broken\n");
}

#[test]
fn input_alias_and_rename_failure_preserve_files() {
    let scratch = Scratch::new("emit-atomic");
    let source = "export function main():void{}";
    let file = scratch.script("input.ts", source);
    let alias = scratch.0.join(".").join("input.ts").into_os_string();
    let outputs = vec![file.clone(), alias];
    #[cfg(windows)]
    let outputs = {
        let mut outputs = outputs;
        outputs.push(std::fs::canonicalize(&file).unwrap().into_os_string());
        outputs
    };
    for output in outputs {
        let (code, out, err) = invoke(&["emit".into(), file.clone(), "-o".into(), output]);
        assert_eq!(code, 2);
        assert!(out.is_empty());
        assert!(err.starts_with("ulua: emit: output "));
        assert!(err.contains(" names input "));
        assert!(!err.contains(r"\\?\"));
        assert_eq!(std::fs::read(&file).unwrap(), source.as_bytes());
    }
    #[cfg(unix)]
    {
        let alias = scratch.0.join("alias.ts");
        std::os::unix::fs::symlink(&file, &alias).unwrap();
        assert_eq!(
            invoke(&[
                "emit".into(),
                file.clone(),
                "-o".into(),
                alias.into_os_string()
            ])
            .0,
            2
        );
        assert_eq!(std::fs::read(&file).unwrap(), source.as_bytes());
    }
    // Rename cannot replace this directory. It must remove the staged artifact.
    let output = scratch.0.join("old-output");
    std::fs::create_dir(&output).unwrap();
    std::fs::write(output.join("keep"), b"old output").unwrap();
    let before = std::fs::read_dir(&scratch.0).unwrap().count();
    let (code, out, err) = invoke(&[
        "emit".into(),
        file,
        "-o".into(),
        output.clone().into_os_string(),
    ]);
    assert_eq!(code, 2);
    assert!(out.is_empty());
    assert!(err.contains(&output.display().to_string()));
    assert_eq!(std::fs::read(output.join("keep")).unwrap(), b"old output");
    assert_eq!(std::fs::read_dir(&scratch.0).unwrap().count(), before);
}
