//! Host bindings spill at the local budget and contribute to closure upvalues.
//! This target reports compiler, ABI, and analyzer costs without time assertions.
// External ABI descriptors are non-exhaustive and require field assignments.
#![allow(clippy::field_reassign_with_default)]
#[path = "../support/analyzer.rs"]
mod analyzer;

use std::{ffi::c_void, path::PathBuf, ptr, time::Instant};
use ulua::*;
use ulua_compiler::{HostSignature, Limits, compile_with_grants};
fn declarations(count: usize) -> (String, Vec<HostSignature>) {
    let s = String::new();
    let mut g = vec![];
    for i in 0..count {
        g.push(HostSignature::new(format!("host{i}"), vec![], 1));
    }
    (s, g)
}
struct Observation {
    index: usize,
    calls: *mut Vec<usize>,
}
unsafe extern "C" fn void_result(
    data: *mut c_void,
    _: *const Value,
    _: usize,
    _: *mut Value,
) -> i32 {
    // SAFETY: Each descriptor holds a live observation and the serialized fixture owns its call sequence.
    unsafe {
        let observation = &*data.cast::<Observation>();
        (*observation.calls).push(observation.index);
    }
    0
}
unsafe extern "C" fn result(
    data: *mut c_void,
    args: *const Value,
    count: usize,
    out: *mut Value,
) -> i32 {
    // SAFETY: The runtime supplies writable result storage for this number signature.
    unsafe {
        void_result(data, args, count, out);
        (*out).r#type = 1;
        (*out).number = 1.0;
    }
    0
}
fn run(source: &str, signatures: &[HostSignature]) -> Vec<usize> {
    let mut c = ptr::null_mut();
    let mut m = ptr::null_mut();
    let mut i = ptr::null_mut();
    let mut d = Diagnostic::default();
    let mut calls = vec![];
    let mut observations: Vec<_> = (0..signatures.len())
        .map(|index| Observation {
            index,
            calls: ptr::from_mut(&mut calls),
        })
        .collect();
    // SAFETY: The test serializes its objects and owns all descriptor, source, and output storage.
    unsafe {
        assert_eq!(ulua_context_new(&mut c), ULUA_OK);
        let mut grants = vec![];
        for (s, observation) in signatures.iter().zip(&mut observations) {
            let mut desc = CapabilityDesc::default();
            desc.name = s.name.as_ptr().cast();
            desc.name_length = s.name.len();
            desc.parameter_types = s.parameters.as_ptr();
            desc.parameter_count = s.parameters.len();
            desc.result_type = s.result;
            desc.function = Some(if s.result == ULUA_TYPE_VOID {
                void_result
            } else {
                result
            });
            desc.userdata = ptr::from_mut(observation).cast();
            let mut cap = ptr::null_mut();
            assert_eq!(ulua_capability_new(c, &desc, &mut cap), ULUA_OK);
            grants.push(cap);
        }
        assert_eq!(
            ulua_compile(
                c,
                source.as_ptr().cast(),
                source.len(),
                grants.as_ptr(),
                grants.len(),
                ptr::null(),
                &mut m,
                &mut d
            ),
            ULUA_OK
        );
        assert_eq!(
            ulua_instance_new(c, m, &Host::default(), ptr::null(), &mut i, &mut d),
            ULUA_OK,
            "{:?}",
            d.code
        );
        let mut index = 0;
        assert_eq!(
            ulua_module_find_export(m, c"main".as_ptr(), 4, &mut index),
            ULUA_OK
        );
        assert_eq!(
            ulua_call(i, index, ptr::null(), 0, &mut Value::default(), &mut d),
            ULUA_OK
        );
        assert_eq!(ulua_context_free(c), ULUA_OK);
    }
    calls
}
#[test]
fn local_spill_and_upvalue_edges() {
    let start = Instant::now();
    let dir = std::env::temp_dir().join(format!("ulua-p3-limits-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let mut files = vec![];
    for count in [159, 160, 161, 162, 176, 177, 512, 1200] {
        let (mut source, grants) = declarations(count);
        source.push_str("export function main():void{let n=0;");
        for i in 0..count {
            source.push_str(&format!("n+=host{i}();"));
        }
        source.push('}');
        let compiled = compile_with_grants(source.as_bytes(), Limits::default(), &grants).unwrap();
        if count >= 161 {
            assert!(compiled.luau.contains("__ulua_spill0"));
        }
        run(&source, &grants);
        let file = dir.join(format!("locals-{count}.luau"));
        std::fs::write(&file, &compiled.luau).unwrap();
        files.push(file);
        eprintln!(
            "P3 local declarations={count} source={} output={} spill={}",
            source.len(),
            compiled.luau.len(),
            compiled.luau.contains("__ulua_spill0")
        );
    }
    for captures in [91, 92, 93] {
        let (mut source, grants) = declarations(100);
        source.push_str("function outer():void{");
        for i in 0..captures {
            source.push_str(&format!("const a{i}=1;"));
        }
        source.push_str("function inner():number{let n=0;");
        for i in 0..100 {
            source.push_str(&format!("n+=host{i}();"));
        }
        for i in 0..captures {
            source.push_str(&format!("n+=a{i};"));
        }
        source.push_str("return n;}inner();}export function main():void{outer();}");
        let compiled = compile_with_grants(source.as_bytes(), Limits::default(), &grants);
        if captures == 93 {
            assert_eq!(compiled.unwrap_err().code, "limit:upvalues");
        } else {
            let compiled = compiled.unwrap();
            run(&source, &grants);
            let file = dir
                .join(format!("upvalues-{}", 100 + captures))
                .with_extension("luau");
            std::fs::write(&file, compiled.luau).unwrap();
            files.push(file);
        }
        eprintln!("P3 upvalues={} accepted={}", 100 + captures, captures < 93);
    }
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let out = analyzer::command(root.join("target/luau-tools/luau-analyze"))
        .args(files)
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", analyzer::failure(&out));
    std::fs::remove_dir_all(dir).unwrap();
    eprintln!("P3 limits cost={:.3}s", start.elapsed().as_secs_f64());
}

#[test]
fn one_grant_table_reaches_the_source_ceiling() {
    let start = Instant::now();
    let mut ceiling = String::from("export function main(): void {");
    let mut signatures = vec![];
    loop {
        let name = format!("g{}", signatures.len());
        let call = format!("{name}();");
        if ceiling.len() + call.len() + 2 > ulua_compiler::SOURCE_CEILING as usize {
            break;
        }
        ceiling.push_str(&call);
        signatures.push(HostSignature::new(name, vec![], ULUA_TYPE_VOID));
    }
    ceiling.push_str("\n}");
    assert_eq!(signatures.len(), 8326);
    assert_eq!(ceiling.len(), 65530);
    let beyond = ceiling.replace("\n}", &format!("g{}();\n}}", signatures.len()));
    assert_eq!(
        compile_with_grants(beyond.as_bytes(), Limits::default(), &signatures)
            .unwrap_err()
            .code,
        "limit:source-bytes"
    );
    let dir = std::env::temp_dir().join(format!("ulua-p3-grant-table-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    for count in [8100, signatures.len()] {
        let source = if count == signatures.len() {
            ceiling.clone()
        } else {
            let mut source = String::from("export function main(): void {");
            for s in &signatures[..count] {
                source.push_str(&format!("{}();", s.name));
            }
            source.push_str("\n}");
            source
        };
        let mut grants = signatures[..count].to_vec();
        grants.reverse();
        let compiled = compile_with_grants(source.as_bytes(), Limits::default(), &grants).unwrap();
        assert_eq!(
            compiled.hosts,
            grants.iter().map(|g| g.name.clone()).collect::<Vec<_>>()
        );
        assert!(compiled.luau.contains("__ulua_spill0"));
        assert_eq!(run(&source, &grants), (0..count).rev().collect::<Vec<_>>());
        let path = dir.join(format!("grants-{count}.luau"));
        std::fs::write(&path, &compiled.luau).unwrap();
        let now = Instant::now();
        let output = analyzer::command(root.join("target/luau-tools/luau-analyze"))
            .arg(path)
            .output()
            .unwrap();
        assert!(output.status.success(), "{}", analyzer::failure(&output));
        eprintln!(
            "P3 grant table count={count} source={} output={} analyzer={:.3}s",
            source.len(),
            compiled.luau.len(),
            now.elapsed().as_secs_f64()
        );
    }
    std::fs::remove_dir_all(dir).unwrap();
    eprintln!("P3 grant table cost={:.3}s", start.elapsed().as_secs_f64());
}
