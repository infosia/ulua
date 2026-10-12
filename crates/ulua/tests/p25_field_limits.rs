//! C15 counts nested anonymous types at each position. Alias names end the count.
//! Measured Debug cost: 1.579 seconds, including ABI execution, tsc, and luau-analyze.
#[allow(dead_code)]
#[path = "support/script.rs"]
mod support;
use std::{path::PathBuf, process::Command, time::Instant};
use support::Script;
use ulua::{CompileLimits, RunLimits, ULUA_OK};
use ulua_compiler::{FIELD_BOUND, Limits, SOURCE_CEILING, compile};

fn key(mut n: usize) -> String {
    let mut digits = vec![];
    loop {
        digits.push(b"0123456789abcdefghijklmnopqrstuvwxyz"[n % 36] as char);
        n /= 36;
        if n == 0 {
            break;
        }
    }
    format!("f{}", digits.into_iter().rev().collect::<String>())
}
fn fields(group: usize, count: usize, last: &str, typed: bool) -> String {
    (0..count)
        .map(|i| {
            let value = if i + 1 == count {
                last
            } else if typed {
                "number"
            } else {
                "0"
            };
            format!(
                "{}:{value}{}",
                key(group * FIELD_BOUND + i),
                if typed { ";" } else { "," }
            )
        })
        .collect()
}
fn boundary(alias: bool, count: usize) -> String {
    if alias {
        format!(
            "type W={{{}}};function read(w:W):number{{return w.f0;}}export function main():void{{}}",
            fields(0, count, "number", true)
        )
    } else {
        format!(
            "export function main():void{{const w={{{}}};print(`${{w.f0}}`);}}",
            fields(0, count, "0", false)
        )
    }
}
fn nested_boundary(count: usize, wrapper: &str) -> String {
    let child = format!("{{{}}}", fields(1, FIELD_BOUND / 2, "0", false));
    let value = match wrapper {
        "array" => format!("[[{child}]]"),
        "nullable" => format!("true?{child}:null"),
        "function" => format!("()=>({child})"),
        "mixed" => format!("()=>true?[{child}]:null"),
        _ => child,
    };
    format!(
        "export function main():void{{const w={{{}}};print(`${{w.f0}}`);}}",
        fields(0, count - FIELD_BOUND / 2, &value, false)
    )
}
fn nested_value(group: usize) -> String {
    let width = FIELD_BOUND / 4;
    let mut value = "0".to_owned();
    for level in (0..4).rev() {
        let start = group * FIELD_BOUND + level * width;
        let mut entries = (start..start + width - 1)
            .map(|i| format!("{}:0,", key(i)))
            .collect::<String>();
        entries.push_str(&format!("{}:{value},", key(start + width - 1)));
        value = format!("{{{entries}}}");
    }
    value
}
fn doubling(levels: usize) -> String {
    let mut body = "const c0={v:0};".to_owned();
    for i in 1..=levels {
        body.push_str(&format!("const c{i}={{a:c{},b:c{}}};", i - 1, i - 1));
    }
    format!("export function main():void{{{body}}}")
}
fn combined(shape: &str, count: usize) -> String {
    let mut aliases = String::new();
    if shape.contains("alias") || matches!(shape, "recursive-literals" | "nested-named-literals") {
        for group in 0..count {
            let last = if shape.starts_with("recursive") {
                format!("W{}|null", (group + 1) % count)
            } else if matches!(shape, "nested-aliases" | "nested-named-literals")
                && group + 1 < count
            {
                format!("W{}", group + 1)
            } else {
                "number".into()
            };
            aliases.push_str(&format!(
                "type W{group}={{{}}};",
                fields(group, FIELD_BOUND, &last, true)
            ));
            aliases.push_str(&format!("function use{group}(w:W{group}):void{{}}"));
        }
    }
    let mut body = String::new();
    if shape == "nested-named-literals" {
        let mut value = "0".to_owned();
        for group in (0..count).rev() {
            value = format!("{{{}}}", fields(group, FIELD_BOUND, &value, false));
        }
        body = format!("const w:W0={value};");
    } else if shape == "nested-literals" {
        for group in 0..count {
            body.push_str(&format!("const w{group}={};", nested_value(group)));
        }
    } else if shape.contains("literal") {
        for group in 0..count {
            let (annotation, last) = if shape == "recursive-literals" {
                (format!(":W{group}"), "null")
            } else {
                (String::new(), "0")
            };
            body.push_str(&format!(
                "const w{group}{annotation}={{{}}};",
                fields(group, FIELD_BOUND, last, false)
            ));
        }
        if shape == "recursive-literals" {
            for group in 0..count {
                body.push_str(&format!(
                    "w{group}.{}=w{};",
                    key((group + 1) * FIELD_BOUND - 1),
                    (group + 1) % count
                ));
            }
        }
    }
    format!("{aliases}export function main():void{{{body}}}")
}
fn check(directory: &std::path::Path, name: &str, source: &str) -> PathBuf {
    let compiled =
        compile(source.as_bytes(), Limits::default()).unwrap_or_else(|d| panic!("{name}: {d:?}"));
    let mut script = Script::new(source);
    assert_eq!(script.instance(RunLimits::default()).0, ULUA_OK, "{name}");
    assert_eq!(script.call(script.index("main"), &[]).0, ULUA_OK, "{name}");
    let path = directory.join(format!("{name}.ts"));
    std::fs::write(&path, source).unwrap();
    std::fs::write(path.with_extension("luau"), compiled.luau).unwrap();
    path
}
#[test]
fn field_bound_and_combined_ceiling_shapes_pass_the_analyzer() {
    let start = Instant::now();
    assert_eq!(FIELD_BOUND, 1024);
    let directory =
        std::env::temp_dir().join(format!("ulua-p25-field-limits-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let mut paths = vec![];
    for alias in [false, true] {
        let source = boundary(alias, FIELD_BOUND);
        paths.push(check(
            &directory,
            if alias {
                "bound-alias"
            } else {
                "bound-literal"
            },
            &source,
        ));
        let source = boundary(alias, FIELD_BOUND + 1);
        let diagnostic = compile(source.as_bytes(), Limits::default()).unwrap_err();
        assert_eq!(diagnostic.code, "limit:fields");
        assert_eq!(
            diagnostic.offset as usize,
            source.find(&format!("{}:", key(FIELD_BOUND))).unwrap()
        );
        assert_eq!(diagnostic.line, 1);
        assert_eq!(diagnostic.column as u64, diagnostic.offset + 1);
        Script::compile(&source, CompileLimits::default(), "limit:fields");
    }
    let past = format!(
        "export function main():void{{const w={{child:{{{}}}}};}}",
        fields(0, FIELD_BOUND + 1, "0", false)
    );
    assert_eq!(
        compile(past.as_bytes(), Limits::default())
            .unwrap_err()
            .code,
        "limit:fields"
    );
    for wrapper in ["direct", "array", "nullable", "function", "mixed"] {
        paths.push(check(
            &directory,
            &format!("bound-nested-{wrapper}"),
            &nested_boundary(FIELD_BOUND, wrapper),
        ));
        let past = nested_boundary(FIELD_BOUND + 1, wrapper);
        let diagnostic = compile(past.as_bytes(), Limits::default()).unwrap_err();
        assert_eq!(diagnostic.code, "limit:fields", "{wrapper}");
        assert_eq!(
            diagnostic.offset as usize,
            past.find("const w=").unwrap() + "const w=".len()
        );
        Script::compile(&past, CompileLimits::default(), "limit:fields");
    }
    paths.push(check(&directory, "doubling-8", &doubling(8)));
    let past = doubling(9);
    assert_eq!(
        compile(past.as_bytes(), Limits::default())
            .unwrap_err()
            .code,
        "limit:fields"
    );
    Script::compile(&past, CompileLimits::default(), "limit:fields");
    let child = format!("{{{}}}", fields(1, FIELD_BOUND / 2, "0", false));
    let alias = format!("type A={{{}}};", fields(1, FIELD_BOUND / 2, "number", true));
    let named = format!(
        "{alias}export function main():void{{const c:A={child};const w={{{}}};}}",
        fields(0, FIELD_BOUND, "c", false)
    );
    paths.push(check(&directory, "named-boundary", &named));
    let anonymous = format!(
        "{alias}export function main():void{{const c={child};const w={{{}}};}}",
        fields(0, FIELD_BOUND / 2 + 1, "c", false)
    );
    assert_eq!(
        compile(anonymous.as_bytes(), Limits::default())
            .unwrap_err()
            .code,
        "limit:fields"
    );
    Script::compile(&anonymous, CompileLimits::default(), "limit:fields");
    for access in [
        "const h={a:c};const v=h.a;",
        "const xs=[c];const v=xs[0];",
        "const make=()=>c;const v=make();",
    ] {
        let past = format!(
            "{alias}export function main():void{{const c={child};{access}const w={{{}}};}}",
            fields(0, FIELD_BOUND / 2 + 1, "v", false)
        );
        assert_eq!(
            compile(past.as_bytes(), Limits::default())
                .unwrap_err()
                .code,
            "limit:fields"
        );
    }
    for shape in [
        "flat-aliases",
        "flat-literals",
        "nested-aliases",
        "nested-literals",
        "nested-named-literals",
        "recursive-aliases",
        "recursive-literals",
    ] {
        let mut count = 1;
        while combined(shape, count + 1).len() <= SOURCE_CEILING as usize {
            count += 1;
        }
        let mut source = combined(shape, count);
        let bytes = source.len();
        assert!(bytes <= SOURCE_CEILING as usize);
        source.extend(std::iter::repeat_n(' ', SOURCE_CEILING as usize - bytes));
        paths.push(check(&directory, shape, &source));
        eprintln!(
            "P2.5 combined fields {shape}: roots={count} fields_per_root={FIELD_BOUND} content_bytes={bytes} source_bytes={}",
            source.len()
        );
    }
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let result = Command::new(root.join("target/luau-tools/luau-analyze"))
        .args(paths.iter().map(|p| p.with_extension("luau")))
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "Combined field count exceeds the analyzer budget:\n{}{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
    let result = Command::new("node")
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
    assert!(
        result.status.success(),
        "{}{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
    std::fs::remove_dir_all(directory).unwrap();
    eprintln!(
        "P2.5 field bound: 1024 passes; 1025 gives limit:fields; combined luau-analyze={} tsc={} cost={:.3}s",
        paths.len(),
        paths.len(),
        start.elapsed().as_secs_f64()
    );
}
