//! Contract checks use the product ABI and existing fixtures. No nested Cargo build runs.
#[path = "emit_tests.rs"]
mod emit_tests;
use super::*;
use std::{
    path::{Path, PathBuf},
    time::Instant,
};
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn invoke(args: &[OsString]) -> (i32, String, String) {
    let mut out = Vec::new();
    let mut err = Vec::new();
    let code = execute(args.iter().cloned(), &mut out, &mut err);
    (
        code,
        String::from_utf8(out).unwrap(),
        String::from_utf8(err).unwrap(),
    )
}
fn call(args: &[&str]) -> (i32, String, String) {
    invoke(&args.iter().map(OsString::from).collect::<Vec<_>>())
}
fn scripts(directory: &Path) -> Vec<PathBuf> {
    let mut paths: Vec<_> = std::fs::read_dir(directory)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "ts"))
        .collect();
    paths.sort();
    paths
}
struct Scratch(PathBuf);
impl Scratch {
    fn new(label: &str) -> Self {
        let path = std::env::temp_dir().join(format!("ulua-cli-{label}-{}", std::process::id()));
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }
    fn script(&self, name: &str, text: &str) -> OsString {
        let path = self.0.join(name);
        std::fs::write(&path, text).unwrap();
        path.into_os_string()
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
#[test]
fn usage_and_io() {
    for args in [vec![], vec!["help"], vec!["--help"]] {
        let (code, out, err) = call(&args);
        assert_eq!(code, 0);
        assert_eq!(out, USAGE);
        assert!(err.is_empty());
    }
    for args in [
        vec!["emit"],
        vec!["check"],
        vec!["check", "x", "y"],
        vec!["run", "--watch"],
        vec!["build"],
        vec!["link-flags", "--cc", "other"],
        vec!["help", "x"],
    ] {
        let (code, out, err) = call(&args);
        assert_eq!(code, 2, "{args:?}");
        assert!(out.is_empty());
        assert!(!err.is_empty());
    }
    assert_eq!(call(&["check", "missing-cli-source.ts"]).0, 2);
    struct Broken;
    impl Write for Broken {
        fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
            Err(std::io::Error::other("broken"))
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    assert_eq!(execute(["help"], &mut Broken, &mut Vec::new()), 2);
}
#[test]
fn corpus_checks() {
    let start = Instant::now();
    let mut count = 0;
    for directory in ["accept", "reject"] {
        for path in scripts(&root().join("corpus").join(directory)) {
            let source = std::fs::read_to_string(&path).unwrap();
            if source.lines().any(|line| line.starts_with("// grants:")) {
                continue;
            }
            let (code, out, err) = invoke(&["check".into(), path.clone().into_os_string()]);
            assert!(out.is_empty(), "{}", path.display());
            if directory == "accept" {
                assert_eq!(code, 0, "{}: {err}", path.display());
                assert_eq!(err, format!("check: {}: no errors\n", path.display()));
            } else {
                let expected = source
                    .lines()
                    .find_map(|l| l.strip_prefix("// expected-error: "))
                    .unwrap();
                let (expected_code, _) = expected.split_once(": ").unwrap();
                assert_eq!(code, 1, "{}", path.display());
                let product = compile_diagnostic(source.as_bytes());
                assert!(
                    err.starts_with(&format!("error[{}]: {}\n", product.0, product.1)),
                    "{}: {err}",
                    path.display()
                );
                assert!(
                    err.starts_with(&format!("error[{expected_code}]: ")),
                    "{}: {err}",
                    path.display()
                );
            }
            count += 1;
        }
    }
    eprintln!(
        "cli check corpus entries={count} cost={:.3}s",
        start.elapsed().as_secs_f64()
    );
}
#[test]
fn example_runs_and_traps() {
    let start = Instant::now();
    let mut count = 0;
    for directory in ["examples/typescript", "examples/run"] {
        for path in scripts(&root().join(directory)) {
            let stem = path.file_stem().unwrap().to_str().unwrap();
            let expected_path = if directory == "examples/run" && stem == "hello" {
                path.with_file_name("expected.txt")
            } else {
                path.with_file_name(format!("expected-{stem}.txt"))
            };
            let expected = std::fs::read_to_string(expected_path).unwrap();
            let mut lines: Vec<_> = expected.lines().collect();
            let stop = lines
                .last()
                .filter(|line| line.starts_with("error ") || line.starts_with("stopped "))
                .copied();
            let expected_code = stop.map(|line| {
                line.split_whitespace()
                    .nth(1)
                    .unwrap()
                    .trim_end_matches(':')
                    .to_string()
            });
            if stop.is_some() {
                lines.pop();
            }
            let wanted = lines
                .iter()
                .map(|line| format!("{line}\n"))
                .collect::<String>();
            let (code, out, err) = invoke(&["run".into(), path.clone().into_os_string()]);
            assert_eq!(
                code,
                i32::from(expected_code.is_some()),
                "{}: {err}",
                path.display()
            );
            assert_eq!(out, wanted, "{}", path.display());
            if let Some(expected_code) = expected_code {
                assert!(
                    err.lines()
                        .next()
                        .unwrap()
                        .starts_with(&format!("error[{expected_code}]: ")),
                    "{}: {err}",
                    path.display()
                );
            }
            count += 1;
        }
    }
    for path in scripts(&root().join("corpus/trap")) {
        let source = std::fs::read_to_string(&path).unwrap();
        let kind = source
            .lines()
            .find_map(|l| l.strip_prefix("// trap: "))
            .unwrap();
        let expected = std::fs::read_to_string(path.with_extension("expected")).unwrap();
        let (code, out, err) = invoke(&["run".into(), path.clone().into_os_string()]);
        assert_eq!(code, 1, "{}: {err}", path.display());
        assert!(
            err.contains(&format!("error[trap:{kind}]")),
            "{}: {err}",
            path.display()
        );
        let wanted = expected
            .lines()
            .filter(|line| !line.starts_with("trap: "))
            .map(|line| format!("{line}\n"))
            .collect::<String>();
        assert_eq!(out, wanted, "{}", path.display());
        count += 1;
    }
    eprintln!(
        "cli runs={count} cost={:.3}s",
        start.elapsed().as_secs_f64()
    );
}
#[test]
fn run_forms_and_output_failure() {
    let scratch = Scratch::new("forms");
    let file = scratch.script(
        "main.ts",
        "export function main(n:number):number{return n;}",
    );
    assert_eq!(invoke(&["run".into(), file]).0, 2);
    let file = scratch.script(
        "none.ts",
        "export function setup():number{print('init');return 1;} const value:number=setup();",
    );
    let (code, out, err) = invoke(&["run".into(), file.clone()]);
    assert_eq!(code, 0, "{err}");
    assert_eq!(out, "init\n");
    assert_eq!(err, format!("run: {}: no export main\n", file.display()));
    let file = scratch.script("result.ts", "export function main():number{return 42;}");
    assert_eq!(
        invoke(&["run".into(), file]),
        (0, String::new(), String::new())
    );
    let file = scratch.script("host.ts", "export function main():void{hostLog('hello');}");
    let (code, _, err) = invoke(&["run".into(), file]);
    assert_eq!(code, 1);
    assert!(err.starts_with("error[C18]:"));
    struct Broken;
    impl Write for Broken {
        fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
            Err(std::io::Error::other("broken output"))
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let file = scratch.script(
        "output.ts",
        "export function main():void{print('one');print('two');}",
    );
    let mut err = Vec::new();
    assert_eq!(
        execute([OsString::from("run"), file], &mut Broken, &mut err),
        1
    );
    assert!(
        String::from_utf8(err)
            .unwrap()
            .starts_with("error[host:output]:")
    );
}
#[test]
fn rendered_diagnostics() {
    use crate::diagnostic::render;
    assert_eq!(
        render(
            "C6",
            "bad",
            "test.ts",
            "one\ntwo\nconst value = values.at(0);",
            3,
            22
        ),
        "error[C6]: bad\n --> test.ts:3:22\n  |\n3 | const value = values.at(0);\n  |                      ^\n  = rule: C6 Arrays\n"
    );
    assert!(render("syntax", "bad", "x", "\téx!", 1, 5).contains("  | \t  ^"));
    for source in ["a\r\nb", "a\rb", "a\u{2028}b", "a\u{2029}b"] {
        assert!(render("syntax", "bad", "x", source, 2, 1).contains("2 | b\n"));
    }
    for line in [0, 100] {
        assert_eq!(
            render("C6", "bad", "x", "x", line, 1),
            format!(
                "error[C6]: bad\n{}= rule: C6 Arrays\n",
                " ".repeat(line.to_string().len() + 1)
            )
        );
    }
    assert_eq!(
        render("unknown", "bad", "x", "", 0, 0),
        "error[unknown]: bad\n"
    );
    assert!(
        render("C6", "bad", "x", "a\nb\nc\nd\ne\nf\ng\nh\ni\nj", 10, 1).contains("10 | j\n   | ^")
    );
    let headings = std::fs::read_to_string(root().join("specs/blocks/collisions.md")).unwrap();
    let headings: Vec<_> = headings
        .lines()
        .filter_map(|l| l.strip_prefix("## C"))
        .map(|l| {
            let (n, title) = l.split_once(". ").unwrap();
            (format!("C{n}"), title.to_string())
        })
        .collect();
    let titles: Vec<_> = diagnostic::RULES
        .iter()
        .filter(|(c, _)| c.starts_with('C'))
        .map(|(c, t)| (c.to_string(), t.to_string()))
        .collect();
    assert_eq!(headings, titles);
    for path in scripts(&root().join("corpus/reject")) {
        let text = std::fs::read_to_string(path).unwrap();
        let header = text
            .lines()
            .find_map(|l| l.strip_prefix("// expected-error: "))
            .unwrap();
        let code = header.split_once(": ").unwrap().0;
        assert!(
            render(code, "bad", "", "", 0, 0).contains("= rule:"),
            "{code}"
        );
    }
    let spec = std::fs::read_to_string(root().join("specs/blocks/corpus.md")).unwrap();
    let section = spec
        .split("## 6. Trap kinds")
        .nth(1)
        .unwrap()
        .split("## 7.")
        .next()
        .unwrap();
    for line in section.lines().filter(|l| l.starts_with("| `")) {
        let cells: Vec<_> = line.split('|').map(str::trim).collect();
        let kind = cells[1].trim_matches('`');
        let rule = cells[2]
            .strip_prefix("collisions.md ")
            .unwrap()
            .split(" (")
            .next()
            .unwrap();
        let rendered = render(&format!("trap:{kind}"), "bad", "", "", 0, 0);
        assert!(
            rendered
                .lines()
                .last()
                .unwrap()
                .ends_with(&format!("({rule})")),
            "{rendered}"
        );
    }
    let mut texts = std::collections::HashSet::new();
    for (code, text) in diagnostic::RULES {
        assert!(texts.insert(text), "duplicate rule text for {code}: {text}");
    }
}
#[test]
fn path_precedence() {
    use native::select;
    let default = Some(PathBuf::from("repo"));
    assert_eq!(
        select(Some("option".into()), Some("env".into()), default.clone()),
        Some("option".into())
    );
    assert_eq!(
        select(None, Some("env".into()), default.clone()),
        Some("env".into())
    );
    assert_eq!(select(None, None, default), Some("repo".into()));
    assert_eq!(select(None, None, None), None);
    let (code, out, err) = call(&["link-flags", "--lib", "missing-cli-library.a"]);
    assert_eq!(code, 2);
    assert!(out.is_empty());
    for text in [
        "--lib",
        "ULUA_LIB",
        "target/release",
        "cargo build --release -p ulua",
    ] {
        assert!(err.contains(text));
    }
}
// Compare CLI messages with an independent product call. Corpus headers describe rules.
fn compile_diagnostic(source: &[u8]) -> (String, String) {
    use ulua::*;
    let mut context = std::ptr::null_mut();
    let mut module = std::ptr::null_mut();
    let mut diagnostic = Diagnostic::default();
    // SAFETY: The source is live. Calls are serialized and both owned objects are released.
    unsafe {
        assert_eq!(ulua_context_new(&mut context), ULUA_OK);
        assert_ne!(
            ulua_compile(
                context,
                source.as_ptr().cast(),
                source.len(),
                std::ptr::null(),
                0,
                std::ptr::null(),
                &mut module,
                &mut diagnostic
            ),
            ULUA_OK
        );
        ulua_module_free(module);
        ulua_context_free(context);
    }
    (
        crate::diagnostic::cstring(&diagnostic.code),
        crate::diagnostic::cstring(&diagnostic.message),
    )
}

#[test]
fn bounded_source_and_diagnostics() {
    use std::io::Read;
    // An infinite reader proves that the command does not request the rest of a file.
    let bytes = script::read_source(std::io::repeat(b'x'), 16).unwrap();
    assert_eq!(bytes, vec![b'x'; 17]);
    let mut limits = ulua::CompileLimits::default();
    // SAFETY: The descriptor is live, initialized, and writable.
    unsafe {
        assert_eq!(
            ulua::ulua_default_compile_limits(&mut limits),
            ulua::ULUA_OK
        );
    }
    let scratch = Scratch::new("bounded");
    let path = scratch.0.join("large.ts");
    let mut file = std::fs::File::create(&path).unwrap();
    std::io::copy(
        &mut std::io::repeat(b'x').take(limits.max_source_bytes + 4096),
        &mut file,
    )
    .unwrap();
    let (code, out, err) = invoke(&["check".into(), path.into_os_string()]);
    assert_eq!(code, 1);
    assert!(out.is_empty());
    assert!(err.starts_with("error[limit:source-bytes]: "), "{err}");
    assert!(err.len() < 2048, "{} diagnostic bytes", err.len());
    let source = format!("const x: number = 0;{}?", " ".repeat(999_980));
    let file = scratch.script("one-line.ts", &source);
    let (code, _, err) = invoke(&["check".into(), file]);
    assert_eq!(code, 1);
    assert!(err.len() < 2048, "{} diagnostic bytes", err.len());
}
#[test]
fn diagnostic_windows_controls_and_gutters() {
    use crate::diagnostic::render;
    let source = "a".repeat(300);
    for (column, shown, caret) in [
        (1, format!("{}...", "a".repeat(120)), 0),
        (151, format!("...{}...", "a".repeat(120)), 63),
        (300, format!("...{}", "a".repeat(120)), 122),
    ] {
        let rendered = render("C6", "bad", "x", &source, 1, column);
        assert!(
            rendered.contains(&format!("1 | {shown}\n  | {}^\n", " ".repeat(caret))),
            "{rendered}"
        );
    }
    let unicode = "é".repeat(300);
    assert!(
        render("C6", "bad", "x", &unicode, 1, 301).contains(&format!(
            "1 | ...{}...\n  | {}^",
            "é".repeat(120),
            " ".repeat(63)
        ))
    );
    let rendered = render("C4", "bad", "x", "\n\n\n\n\n\n\n\n\n\n\nabc", 12, 2);
    assert_eq!(
        rendered,
        "error[C4]: bad\n  --> x:12:2\n   |\n12 | abc\n   |  ^\n   = rule: C4 Numbers\n"
    );
    let controls = "\0\x1b\x7f\u{85}\u{9f}";
    let rendered = render(
        "syntax",
        &format!("x\n\t{controls}"),
        "x",
        format!("\t{controls}z"),
        1,
        9,
    );
    assert!(
        rendered.starts_with("error[syntax]: x\\u{0a}\\u{09}\\u{00}\\u{1b}\\u{7f}\\u{85}\\u{9f}\n"),
        "{rendered}"
    );
    assert!(
        rendered.contains(
            "1 | \t\\u{00}\\u{1b}\\u{7f}\\u{85}\\u{9f}z\n  | \t                              ^"
        ),
        "{rendered}"
    );
    let scratch = Scratch::new("forged");
    let file = scratch.script(
        "throw.ts",
        r#"export function main():void{throw new Error("x\n  = rule: fake\nerror[C1]: forged");}"#,
    );
    let (code, out, err) = invoke(&["run".into(), file]);
    assert_eq!(code, 1);
    assert!(out.is_empty());
    assert_eq!(err.lines().count(), 2, "{err}");
    assert!(
        err.contains("x\\u{0a}  = rule: fake\\u{0a}error[C1]: forged"),
        "{err}"
    );
}
#[test]
fn usage_reasons_compiler_words_and_statuses() {
    for (args, reason) in [
        (vec!["wrong"], "wrong"),
        (vec!["check"], "<file.ts>"),
        (vec!["run", "x", "extra"], "extra"),
        (vec!["build"], "--host"),
        (vec!["build", "--host"], "--host"),
        (vec!["build", "--host", "x.rs"], "x.rs"),
        (vec!["link-flags", "--cc", "bad"], "bad"),
        (vec!["link-flags", "--lib"], "--lib"),
        (vec!["build", "--oops"], "--oops"),
    ] {
        let (code, out, err) = call(&args);
        assert_eq!(code, 2);
        assert!(out.is_empty());
        assert!(err.starts_with("ulua: "), "{err}");
        assert!(err.lines().next().unwrap().contains(reason), "{err}");
        assert!(err.ends_with(USAGE));
    }
    let cmd = native::compiler_value(
        std::ffi::OsStr::new(" \tcc -DVALUE=1\r\n-O0\x0b-fno-builtin\x0c"),
        "CC",
    )
    .unwrap();
    assert_eq!(cmd.get_program(), "cc");
    assert_eq!(
        cmd.get_args().collect::<Vec<_>>(),
        ["-DVALUE=1", "-O0", "-fno-builtin"]
    );
    assert!(
        native::compiler_value(std::ffi::OsStr::new(" \t"), "CXX")
            .unwrap_err()
            .1
            .contains("CXX")
    );
    for (status, name) in [
        (0, "ULUA_OK"),
        (1, "ULUA_INVALID_ARGUMENT"),
        (2, "ULUA_REJECTED"),
        (3, "ULUA_FAILED"),
        (4, "ULUA_HOST_ERROR"),
        (5, "ULUA_BUSY"),
        (6, "ULUA_INTERNAL"),
        (7, "ULUA_POISONED"),
        (8, "ULUA_PENDING"),
        (99, "ULUA_UNKNOWN"),
    ] {
        let failure = script::status_failure(
            status,
            &ulua::Diagnostic::default(),
            std::ffi::OsStr::new("x"),
            b"",
        );
        assert_eq!(
            failure,
            (1, format!("error[abi:status]: {name} ({status})"))
        );
    }
}

#[test]
fn raw_byte_carets_and_source_character_windows() {
    use crate::diagnostic::render;
    // Invalid sequences consume one byte per replacement, while a valid é consumes two bytes.
    let rendered = render("syntax", "bad", "x", b"\xff\r\n\xc3\xa9\xff\xe2\x82Z", 2, 6);
    assert!(rendered.contains("2 | é���Z\n  |     ^\n"), "{rendered}");
    let mut bytes = vec![0xff; 70_000];
    bytes[65_536] = b'X';
    let rendered = render("syntax", "bad", "ff.ts", &bytes, 1, 65_537);
    assert!(rendered.contains(" --> ff.ts:1:65537\n"));
    assert!(
        rendered.contains(&format!(
            "1 | ...{}X{}...\n  | {}^\n",
            "�".repeat(60),
            "�".repeat(59),
            " ".repeat(63)
        )),
        "{rendered}"
    );
    // The window holds 120 SOURCE characters before escaping, even when each prints six characters.
    let rendered = render("syntax", "bad", "x", "\x1b".repeat(300), 1, 151);
    assert!(
        rendered.contains(&format!(
            "1 | ...{}...\n  | {}^\n",
            "\\u{1b}".repeat(120),
            " ".repeat(3 + 60 * 6)
        )),
        "{rendered}"
    );
    let scratch = Scratch::new("invalid-bytes");
    let file = scratch.0.join("ff.ts");
    std::fs::write(&file, vec![0xff; 70_000]).unwrap();
    let (code, out, err) = invoke(&["check".into(), file.into_os_string()]);
    assert_eq!(code, 1);
    assert!(out.is_empty());
    assert!(err.starts_with("error[limit:source-bytes]: "), "{err}");
    assert!(err.contains(":1:65537\n"), "{err}");
    assert!(
        err.contains(&format!(
            "1 | ...{}\n  | {}^\n",
            "�".repeat(120),
            " ".repeat(122)
        )),
        "{err}"
    );
    assert!(err.len() < 2048, "{} bytes", err.len());
}
#[test]
fn unicode_format_ranges_and_escaping() {
    use crate::diagnostic::{escape, render};
    // Independently chosen Unicode 17.0 Cf range endpoints include BMP and supplementary characters.
    for point in [
        0xad, 0x600, 0x605, 0x61c, 0x890, 0x891, 0x200b, 0x200f, 0x202a, 0x202e, 0x2060, 0x2064,
        0x2066, 0x2069, 0x206f, 0xfeff, 0xfff9, 0xfffb, 0x13430, 0x1343f, 0x1bca0, 0x1bca3,
        0x1d173, 0x1d17a, 0xe0001, 0xe0020, 0xe007f,
    ] {
        let text = char::from_u32(point).unwrap().to_string();
        assert_eq!(escape(&text, false), format!("\\u{{{point:02x}}}"));
        assert_eq!(escape(&text, true), format!("\\u{{{point:02x}}}"));
    }
    for point in [
        0xac, 0xae, 0x5ff, 0x606, 0x88f, 0x892, 0x200a, 0x2010, 0x2029, 0x202f, 0x2065, 0x2070,
        0xfe00, 0xfff8, 0xfffc, 0x1342f, 0x13440, 0x1d172, 0x1d17b, 0xe0002, 0xe001f, 0xe0080,
    ] {
        let text = char::from_u32(point).unwrap().to_string();
        assert_eq!(escape(&text, false), text, "U+{point:04X}");
    }
    let rendered = render(
        "syntax",
        "x\u{202e}\u{2066}\u{2069}",
        "x",
        "a\u{200b}b",
        1,
        5,
    );
    assert!(
        rendered.starts_with("error[syntax]: x\\u{202e}\\u{2066}\\u{2069}\n"),
        "{rendered}"
    );
    assert!(
        rendered.contains("1 | a\\u{200b}b\n  |          ^\n"),
        "{rendered}"
    );
}
#[test]
fn filenames_escape_on_success_errors_usage_and_native_answers() {
    let scratch = Scratch::new("filenames");
    // Cf is valid in file names on each supported OS. LF is also tested on Unix.
    let part = if cfg!(unix) {
        "\u{202e}\u{200b}\t\n"
    } else {
        "\u{202e}\u{200b}"
    };
    let filename = format!("script{part}.ts");
    let file = scratch.script(&filename, "export function setup():void{}");
    let shown = crate::diagnostic::filename(&file);
    assert!(shown.contains("\\u{202e}\\u{200b}"));
    assert_eq!(
        invoke(&["check".into(), file.clone()]),
        (0, String::new(), format!("check: {shown}: no errors\n"))
    );
    assert_eq!(
        invoke(&["run".into(), file.clone()]),
        (0, String::new(), format!("run: {shown}: no export main\n"))
    );
    std::fs::write(&file, "export function main(n:number):void{print(`${n}`);}").unwrap();
    let (code, _, err) = invoke(&["run".into(), file.clone()]);
    assert_eq!(code, 2);
    assert!(
        err.starts_with(&format!(
            "ulua: run: {shown}: export main must have no parameters\n"
        )),
        "{err}"
    );
    std::fs::write(&file, "const x:any=1;").unwrap();
    let (code, _, err) = invoke(&["check".into(), file.clone()]);
    assert_eq!(code, 1);
    assert!(err.contains(&format!("--> {shown}:1:")), "{err}");
    std::fs::remove_file(&file).unwrap();
    let (code, _, err) = invoke(&["check".into(), file.clone()]);
    assert_eq!(code, 2);
    assert!(err.starts_with(&format!("{shown}: ")), "{err}");
    assert_eq!(err.lines().count(), 1);
    let (code, _, err) = invoke(&["check".into(), "extra".into(), file.clone()]);
    assert_eq!(code, 2);
    assert!(
        err.starts_with(&format!("ulua: check: unexpected argument {shown}\n")),
        "{err}"
    );
    let library = scratch.0.join(format!("lib{part}.a"));
    std::fs::write(&library, b"flags only").unwrap();
    let include = scratch.0.join(format!("include{part}"));
    std::fs::create_dir(&include).unwrap();
    std::fs::write(include.join("ulua.h"), b"").unwrap();
    let args = vec![
        "link-flags".into(),
        "--cc".into(),
        "unix".into(),
        "--lib".into(),
        library.clone().into_os_string(),
        "--include".into(),
        include.clone().into_os_string(),
    ];
    let (code, out, err) = invoke(&args);
    assert_eq!(code, 0, "{err}");
    assert!(err.is_empty());
    assert!(out.starts_with(&format!(
        "-I{}\n{}\n",
        crate::diagnostic::filename(std::fs::canonicalize(&include).unwrap().as_os_str()),
        crate::diagnostic::filename(std::fs::canonicalize(&library).unwrap().as_os_str())
    )));
    std::fs::remove_file(&library).unwrap();
    let (code, _, err) = invoke(&args);
    assert_eq!(code, 2);
    assert!(
        err.starts_with(&crate::diagnostic::filename(library.as_os_str())),
        "{err}"
    );
    assert_eq!(err.lines().count(), 1);
    let (code, _, err) = invoke(&["build".into(), "--host".into(), file]);
    assert_eq!(code, 2);
    assert_eq!(
        err.lines().next().unwrap().matches("\\u{202e}").count(),
        1,
        "{err}"
    );
}
