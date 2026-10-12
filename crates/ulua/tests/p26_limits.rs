//! Class field and method bounds, nominal value-flow rejection, and class type-link limits.
//! Single-worker Debug cost: 0.64 seconds for this test target.
//! Four boundaries use external witnesses. Two initializer boundaries use the native compiler and VM.
//! Costs are reports, never assertions.
#[path = "support/analyzer.rs"]
mod analyzer;
#[allow(dead_code)]
#[path = "support/script.rs"]
mod support;
use std::{path::PathBuf, process::Command, time::Instant};
use ulua_compiler::{Limits, compile};

fn fields(n: usize, methods: bool) -> String {
    let mut out = "class C{".to_owned();
    for i in 0..n {
        out.push_str(&if methods {
            format!("m{i}():number{{return 1;}}")
        } else {
            format!("v{i}:number=1;")
        });
    }
    out.push_str("}export function main():void{new C();}");
    out
}
fn chain(n: usize, nullable: bool) -> String {
    let mut out = "class C0{v:number=1;}".to_owned();
    for i in 1..n {
        let prev = i - 1;
        out.push_str(&if nullable {
            format!("class C{i}{{v:C{prev}|null=null;}}")
        } else {
            format!("class C{i}{{v:C{prev}=new C{prev}();}}")
        });
    }
    out.push_str("export function main():void{}");
    out
}
#[test]
fn class_boundaries_and_nominal_flows() {
    let start = Instant::now();
    let dir = std::env::temp_dir().join(format!("ulua-P2.6-limits-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let mut paths = vec![];
    for (name, source) in [
        ("fields", fields(1024, false)),
        ("methods", fields(1024, true)),
        ("direct-depth", chain(48, false)),
        ("nullable-depth", chain(24, true)),
    ] {
        let compiled = compile(source.as_bytes(), Limits::default())
            .unwrap_or_else(|d| panic!("{name}: {d:?}"));
        let mut script = support::Script::new(&source);
        assert_eq!(script.instance(ulua::RunLimits::default()).0, ulua::ULUA_OK);
        assert_eq!(script.call(script.index("main"), &[]).0, ulua::ULUA_OK);
        let path = dir.join(format!("{name}.luau"));
        std::fs::write(&path, compiled.luau).unwrap();
        paths.push(path);
        std::fs::write(dir.join(format!("{name}.ts")), source).unwrap();
    }
    for expression in [
        format!("{}0{}", "(".repeat(43), ")".repeat(43)),
        format!("{}0", "- ".repeat(43)),
    ] {
        // The ABI passes accepted boundary output to the native luau_compile function.
        let source =
            format!("class C{{v:number={expression};}}export function main():void{{new C();}}");
        let mut script = support::Script::new(&source);
        assert_eq!(script.instance(ulua::RunLimits::default()).0, ulua::ULUA_OK);
        assert_eq!(script.call(script.index("main"), &[]).0, ulua::ULUA_OK);
    }
    for source in [fields(1025, false), fields(1025, true)] {
        assert_eq!(
            compile(source.as_bytes(), Limits::default())
                .unwrap_err()
                .code,
            "limit:fields"
        );
    }
    let mixed = format!(
        "class C{{{}m():void{{}}}}export function main():void{{}}",
        (0..1024)
            .map(|i| format!("f{i}:number=0;"))
            .collect::<String>()
    );
    assert_eq!(
        compile(mixed.as_bytes(), Limits::default())
            .unwrap_err()
            .code,
        "limit:fields"
    );
    for source in [chain(49, false), chain(25, true)] {
        assert_eq!(
            compile(source.as_bytes(), Limits::default())
                .unwrap_err()
                .code,
            "limit:type-depth"
        );
    }
    let prefix = "class A{v:number=1;}class B{v:number=1;}type R={v:number};";
    for body in [
        "const b:B=new A();",
        "const a:A={v:1};",
        "const r:R=new A();",
        "const bs:B[]=[new A()];",
        "const f:(a:A)=>B=(a:A):A=>a;",
        "const b:B|null=new A();",
        "const r:R[]=[new A()];",
    ] {
        let source = format!("{prefix}export function main():void{{{body}}}");
        assert_eq!(
            compile(source.as_bytes(), Limits::default())
                .unwrap_err()
                .code,
            "C1",
            "{source}"
        );
    }
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let now = Instant::now();
    let out = analyzer::command(root.join("target/luau-tools/luau-analyze"))
        .args(&paths)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "luau-analyze {}",
        analyzer::failure(&out)
    );
    let analyze = now.elapsed().as_secs_f64();
    let out = Command::new("node")
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
        .args(paths.iter().map(|p| p.with_extension("ts")))
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let runner = dir.join("node.cjs");
    std::fs::write(&runner,r#"const fs=require('fs'),ts=require(process.argv[2]);
for(const file of process.argv.slice(3)){
 const source=fs.readFileSync(file,'utf8');
 const js=ts.transpileModule(source,{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.CommonJS}}).outputText;
 const exports={},lines=[];new Function('exports','print',js)(exports,s=>lines.push(s));
 if(exports.main()!==undefined||lines.length!==0)throw Error(file);
}
console.log('P2.6 boundary node matches: '+process.argv.slice(3).length);"#).unwrap();
    let out = Command::new("node")
        .arg(&runner)
        .arg(root.join("node_modules/typescript"))
        .args(paths.iter().map(|p| p.with_extension("ts")))
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    eprint!("{}", String::from_utf8_lossy(&out.stdout));
    eprintln!(
        "P2.6 limits accepted=6 external_witnesses=4 rejected=12 analyzer={analyze:.3}s total_cost={:.3}s",
        start.elapsed().as_secs_f64()
    );
    std::fs::remove_dir_all(dir).unwrap();
}
