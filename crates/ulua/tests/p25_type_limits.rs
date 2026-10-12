//! C15 type-link depth: 48 passes the analyzer; 49 gives limit:type-depth.
//! Recursive alias components contribute one step. Shared branches use cached depths.
//! Measured Debug cost: 1.979 seconds, including ABI execution, tsc, and fifteen analyzer witnesses.
//! Ceiling groups: direct 26, branching 20, nullable 48, array 48, function 41, anonymous 58, array values 63.
#[allow(dead_code)]
#[path = "support/script.rs"]
mod support;
use std::{
    path::{Path, PathBuf},
    process::Command,
    time::Instant,
};
use support::Script;
use ulua::{CompileLimits, RunLimits, ULUA_OK};
use ulua_compiler::{Limits, SOURCE_CEILING, TYPE_DEPTH_BOUND, compile};

fn chain(shape: &str, depth: usize, group: usize) -> (String, String) {
    let name = |i: usize| format!("A{group}_{i}");
    let value = |i: usize| format!("c{group}_{i}");
    if shape == "anonymous" || shape == "array-values" {
        let mut body = format!(
            "const {}={};",
            value(0),
            if shape == "anonymous" { "{v:0}" } else { "0" }
        );
        let base = usize::from(shape == "anonymous");
        for i in 1..=depth - base {
            let child = value(i - 1);
            let literal = if shape == "anonymous" {
                format!("{{a:{child}}}")
            } else {
                format!("[{child}]")
            };
            body.push_str(&format!("const {}={literal};", value(i)));
        }
        return (String::new(), body);
    }
    let step = if matches!(shape, "direct" | "branching") {
        1
    } else {
        2
    };
    let levels = (depth - 1) / step;
    let mut aliases = format!("type {}={{v:number}};", name(0));
    let mut body = format!("const {}:{}={{v:0}};", value(0), name(0));
    for i in 1..=levels {
        let previous = name(i - 1);
        let child = value(i - 1);
        let (field, entry) = match shape {
            "nullable" => (format!("a:{previous}|null"), format!("a:{child}")),
            "array" => (format!("a:{previous}[]"), format!("a:[{child}]")),
            "function-result" => (
                format!("a:()=>{previous}"),
                format!("a:():{previous}=>{child}"),
            ),
            "function-parameter" => (
                format!("a:(x:{previous})=>number"),
                format!("a:(x:{previous}):number=>0"),
            ),
            "branching" => (
                format!("a:{previous};b:{previous}"),
                format!("a:{child},b:{child}"),
            ),
            _ => (format!("a:{previous}"), format!("a:{child}")),
        };
        aliases.push_str(&format!("type {}={{{field}}};", name(i)));
        body.push_str(&format!("const {}:{}={{{entry}}};", value(i), name(i)));
    }
    if 1 + step * levels < depth {
        let previous = name(levels);
        let child = value(levels);
        body.push_str(&match shape {
            "nullable" => format!("const last{group}:{previous}|null={child};"),
            "array" => format!("const last{group}:{previous}[]=[{child}];"),
            "function-result" => {
                format!("const last{group}:()=>{previous}=():{previous}=>{child};")
            }
            "function-parameter" => {
                format!("const last{group}:(x:{previous})=>number=(x:{previous}):number=>0;")
            }
            _ => unreachable!(),
        });
    }
    (aliases, body)
}
fn source(shape: &str, depth: usize, groups: usize) -> String {
    let mut aliases = String::new();
    let mut body = String::new();
    for group in 0..groups {
        let (a, b) = chain(shape, depth, group);
        aliases.push_str(&a);
        body.push_str(&b);
    }
    format!("{aliases}export function main():void{{{body}}}")
}
fn ring(above: usize) -> String {
    let mut aliases = String::new();
    for i in 0..100 {
        aliases.push_str(&format!("type R{i}={{n:R{}|null}};", (i + 1) % 100));
    }
    let mut body = "const c0:R0={n:null};".to_owned();
    for i in 1..=above {
        aliases.push_str(&format!(
            "type A{i}={{a:{}}};",
            if i == 1 {
                "R0".to_owned()
            } else {
                format!("A{}", i - 1)
            }
        ));
        body.push_str(&format!("const c{i}:A{i}={{a:c{}}};", i - 1));
    }
    format!("{aliases}export function main():void{{{body}}}")
}
fn check(directory: &Path, name: &str, source: &str) -> PathBuf {
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
fn type_depth_bound_and_ceiling_combinations() {
    let start = Instant::now();
    assert_eq!(TYPE_DEPTH_BOUND, 48);
    let directory =
        std::env::temp_dir().join(format!("ulua-p25-type-limits-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let mut paths = vec![];
    for shape in [
        "direct",
        "nullable",
        "array",
        "function-result",
        "function-parameter",
        "anonymous",
        "array-values",
    ] {
        paths.push(check(&directory, shape, &source(shape, 48, 1)));
        let past = source(shape, 49, 1);
        let diagnostic = compile(past.as_bytes(), Limits::default()).unwrap_err();
        assert_eq!(diagnostic.code, "limit:type-depth", "{shape}");
        assert_eq!(diagnostic.line, 1);
        assert_eq!(diagnostic.column as u64, diagnostic.offset + 1);
        let form = if shape == "anonymous" {
            "{a:c0_47}"
        } else if shape == "array-values" {
            "[c0_48]"
        } else if matches!(shape, "direct" | "branching") {
            "type A0_48="
        } else {
            "type A0_24="
        };
        assert_eq!(
            diagnostic.offset as usize,
            past.find(form).unwrap(),
            "{shape}"
        );
        Script::compile(&past, CompileLimits::default(), "limit:type-depth");
    }
    paths.push(check(&directory, "recursive-ring-100-plus-47", &ring(47)));
    assert_eq!(
        compile(ring(48).as_bytes(), Limits::default())
            .unwrap_err()
            .code,
        "limit:type-depth"
    );
    // Signature syntax reaches the nesting limit first. Value chains still reach the type bound.
    let mut maximum = 0;
    for levels in 1..=49 {
        let source = format!(
            "function f(x:number{}):void{{}}export function main():void{{}}",
            "[]".repeat(levels)
        );
        match compile(source.as_bytes(), Limits::default()) {
            Ok(_) => maximum = levels,
            Err(d) => {
                assert_eq!(d.code, "limit:nesting-depth");
                break;
            }
        }
    }
    assert!(maximum < TYPE_DEPTH_BOUND);
    eprintln!(
        "P2.5 array type syntax: maximum={maximum}, next=limit:nesting-depth; value chains reach 48"
    );
    for shape in [
        "direct",
        "branching",
        "nullable",
        "array",
        "function-result",
        "anonymous",
        "array-values",
    ] {
        let mut groups = 1;
        while source(shape, 48, groups + 1).len() <= SOURCE_CEILING as usize {
            groups += 1;
        }
        let mut source = source(shape, 48, groups);
        let bytes = source.len();
        source.extend(std::iter::repeat_n(' ', SOURCE_CEILING as usize - bytes));
        paths.push(check(&directory, &format!("ceiling-{shape}"), &source));
        eprintln!(
            "P2.5 combined depth {shape}: depth=48 groups={groups} content_bytes={bytes} source_bytes={}",
            source.len()
        );
    }
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let now = Instant::now();
    let output = Command::new(root.join("target/luau-tools/luau-analyze"))
        .args(paths.iter().map(|p| p.with_extension("luau")))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "Bound shape exceeds the analyzer budget:\n{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    eprintln!(
        "P2.5 type depth analyzer: programs={} cost={:.3}s",
        paths.len(),
        now.elapsed().as_secs_f64()
    );
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
    assert!(
        output.status.success()
            || (!text.is_empty()
                && text
                    .lines()
                    .all(|line| line.contains("error TS2321:") || line.contains("error TS2563:"))),
        "{text}"
    );
    eprintln!("P2.5 type depth tsc: {text}");
    std::fs::remove_dir_all(directory).unwrap();
    eprintln!("P2.5 type depth cost={:.3}s", start.elapsed().as_secs_f64());
}
