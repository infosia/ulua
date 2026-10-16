//! Small ABI cases pin numeric bounds and evaluation order against hand values and Node.
//! One batched Node process and one analyzer process cover all finite cases.
#[path = "../support/analyzer.rs"]
mod analyzer;
#[allow(dead_code)]
#[path = "../support/script.rs"]
mod support;
use std::{
    ffi::{c_char, c_void},
    path::PathBuf,
    process::Command,
    time::Instant,
};
use ulua::*;

unsafe extern "C" fn output(data: *mut c_void, text: *const c_char, length: usize) -> i32 {
    // SAFETY: The test keeps the vector live. The ABI supplies readable UTF-8 bytes.
    unsafe {
        (*data.cast::<Vec<String>>()).push(
            String::from_utf8_lossy(std::slice::from_raw_parts(text.cast(), length)).into_owned(),
        );
    }
    0
}

#[test]
fn a67_counted_edges_match_the_golden() {
    let start = Instant::now();
    let source = include_str!("../../../../corpus/accept/a67-counted-loop-edges.ts");
    let code = ulua_compiler::compile(source.as_bytes(), Default::default())
        .unwrap()
        .luau;
    assert_eq!(code.matches("for __ulua_g").count(), 4);
    assert!(!code.contains("while true do"));
    let mut script = support::Script::new(source);
    let mut lines = Vec::<String>::new();
    let mut host = Host::default();
    host.userdata = std::ptr::from_mut(&mut lines).cast();
    host.output = Some(output);
    assert_eq!(
        script.instance_with_host(RunLimits::default(), &host).0,
        ULUA_OK
    );
    assert_eq!(script.call(script.index("main"), &[]).0, ULUA_OK);
    assert_eq!(
        lines.join("\n") + "\n",
        include_str!("../../../../corpus/accept/a67-counted-loop-edges.expected")
    );
    eprintln!(
        "a67 numeric edges cost={:.6}s",
        start.elapsed().as_secs_f64()
    );
}

#[test]
fn finite_loop_and_condition_cases_match_node() {
    let _measurement = crate::measurement_lock();
    let start = Instant::now();
    let directory = std::env::temp_dir().join(format!("ulua-p6s5-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let mut cases = vec![
        ("let b=6;let n=0;for(let i=0;i<b;i++){n+=i;b--;}return n;".to_owned(), 3.0, false),
        ("let n=0;let first=()=>0;let last=()=>0;for(let i=0;i<3;i++){if(i===0){first=()=>i;}last=()=>i;n+=i;}return n*100+first()*10+last();".to_owned(), 302.0, false),
        ("let n=0;for(let i=0;i<3;i++){const get=()=>1;n+=get();}return n;".to_owned(), 3.0, true),
        ("let n=0;for(let i=0;i<5;i++){if(i===3){break;}n+=i;}return n;".to_owned(), 3.0, true),
        ("let n=0;for(let i=0;i<3;i++){try{n+=i;}finally{n+=1;}}return n;".to_owned(), 6.0, true),
        ("let n=0;for(let i=0;i<3;i++){let j=0;while(j<2){j++;continue;}n+=j;}return n;".to_owned(), 6.0, true),
        ("let n=0;for(let i=0;i<-0;i++){n++;}return n;".to_owned(), 0.0, true),
        ("let n=0;const b=NaN;for(let i=10;i>=b;i--){n++;}return n;".to_owned(), 0.0, true),
        ("let n=0;const b=Infinity;for(let i=10;i>b;i--){n++;}return n;".to_owned(), 0.0, true),
        ("let n=0;const b=-Infinity;for(let i=0;i<=b;i++){n++;}return n;".to_owned(), 0.0, true),
        ("let n=0;for(let i=9007199254740996;i<9007199254740996;i++){n++;break;}return n;".to_owned(), 0.0, true),
        ("let n=0;const b=-9007199254740996;for(let i=-9007199254740996;i>b;i--){n++;break;}return n;".to_owned(), 0.0, true),
    ];
    for (test, result) in [
        ("false&&yes()", 0.0),
        ("true||yes()", 1.0),
        ("true&&yes()", 11.0),
        ("false||yes()", 11.0),
        ("yes()&&n===10", 11.0),
        ("yes()||no()", 11.0),
        ("(false||yes())&&no()", 110.0),
        ("true||(xs[2]&&yes())", 1.0),
        ("false&&(xs[2]||yes())", 0.0),
        ("xs[0]&&n===0&&yes()", 11.0),
        ("n===0&&yes()&&n===10", 11.0),
        ("n===1||yes()&&n===10", 11.0),
    ] {
        cases.push((format!("let n=0;const xs=[true];const yes=():boolean=>{{n+=10;return true;}};const no=():boolean=>{{n+=100;return false;}};let r=0;if({test}){{r=1;}}return n+r;"), result, false));
    }
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut paths = Vec::new();
    for (i, (body, expected, numeric)) in cases.iter().enumerate() {
        let source = format!("export function main():number{{{body}}}");
        let code = ulua_compiler::compile(source.as_bytes(), Default::default())
            .unwrap()
            .luau;
        assert_eq!(code.contains("for __ulua_g"), *numeric, "{source}\n{code}");
        let mut script = support::Script::new(&source);
        assert_eq!(script.instance(RunLimits::default()).0, ULUA_OK);
        let (status, diagnostic, value) = script.call(script.index("main"), &[]);
        assert_eq!((status, diagnostic.as_str()), (ULUA_OK, ""), "{source}");
        assert_eq!(value.number, *expected, "{source}");
        let path = directory.join(format!("case-{i}.luau"));
        std::fs::write(&path, code).unwrap();
        std::fs::write(path.with_extension("ts"), source).unwrap();
        std::fs::write(path.with_extension("expected"), expected.to_string()).unwrap();
        paths.push(path);
    }
    let analyzed = analyzer::output(root.join("target/luau-tools/luau-analyze"), &paths);
    assert!(
        analyzed.status.success(),
        "{}",
        analyzer::failure(&analyzed)
    );
    let runner = directory.join("node.cjs");
    std::fs::write(&runner, r#"const fs=require('fs'),path=require('path'),ts=require(process.argv[2]);
const dir=process.argv[3];for(const file of fs.readdirSync(dir).filter(f=>f.endsWith('.ts'))){
 const base=path.join(dir,file.slice(0,-3)),source=fs.readFileSync(base+'.ts','utf8');
 const js=ts.transpileModule(source,{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.CommonJS}}).outputText;
 const exports={};new Function('exports',js)(exports);
 const expected=Number(fs.readFileSync(base+'.expected','utf8')),actual=exports.main();
 if(actual!==expected)throw Error(file+': '+actual+' != '+expected);
}console.log('P6.S5 Node matches');"#).unwrap();
    let node = Command::new("node")
        .arg(runner)
        .arg(root.join("node_modules/typescript"))
        .arg(&directory)
        .output()
        .unwrap();
    assert!(node.status.success(), "{}", analyzer::failure(&node));
    std::fs::remove_dir_all(directory).unwrap();
    eprintln!(
        "P6.S5 finite cases={} analyzer+Node cost={:.3}s",
        cases.len(),
        start.elapsed().as_secs_f64()
    );
}

#[test]
fn numeric_backedges_and_nonfinite_bounds_reach_the_step_limit() {
    let start = Instant::now();
    for body in [
        "const b=Infinity;for(let i=0;i<b;i++){}",
        "const b=-Infinity;for(let i=0;i>b;i--){}",
        "for(let i=9007199254740992;i<=9007199254740992;i++){}",
    ] {
        let source = format!("export function main():void{{{body}}}");
        let code = ulua_compiler::compile(source.as_bytes(), Default::default())
            .unwrap()
            .luau;
        assert!(code.contains("for __ulua_g"));
        let mut script = support::Script::new(&source);
        let mut limits = RunLimits::default();
        limits.max_steps = 4096;
        assert_eq!(script.instance(limits).0, ULUA_OK);
        let (status, code, _) = script.call(script.index("main"), &[]);
        assert_eq!((status, code.as_str()), (ULUA_FAILED, "limit:steps"));
    }
    eprintln!(
        "numeric backedge step checks cost={:.6}s",
        start.elapsed().as_secs_f64()
    );
}

#[test]
fn condition_chains_and_numeric_exits_keep_instruction_bounds() {
    let _measurement = crate::measurement_lock();
    let start = Instant::now();
    let test = ["n<3"; (ulua_compiler::CONDITION_NODES + 1) / 4].join("&&");
    let source = format!(
        "export function main():number{{let n=0;for(let i=0;i<10;i++){{if({test}){{n++;}}else{{break;}}}}return n;}}"
    );
    let code = ulua_compiler::compile(source.as_bytes(), Default::default())
        .unwrap()
        .luau;
    super::p6_expressions::instructions(&code, 3.0);
    eprintln!(
        "condition/numeric VM instruction bounds cost={:.6}s",
        start.elapsed().as_secs_f64()
    );
}
