//! Host bindings spill at the local budget and contribute to closure upvalues.
//! This target reports compiler, ABI, and analyzer costs without time assertions.
// External ABI descriptors are non-exhaustive and require field assignments.
#![allow(clippy::field_reassign_with_default)]
#[path = "support/analyzer.rs"]
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
unsafe extern "C" fn result(_: *mut c_void, _: *const Value, _: usize, out: *mut Value) -> i32 {
    // SAFETY: The runtime supplies writable result storage for this number signature.
    unsafe {
        (*out).r#type = 1;
        (*out).number = 1.0;
    }
    0
}
fn run(source: &str, signatures: &[HostSignature]) {
    let mut c = ptr::null_mut();
    let mut m = ptr::null_mut();
    let mut i = ptr::null_mut();
    let mut d = Diagnostic::default();
    // SAFETY: The test serializes its objects and owns all descriptor, source, and output storage.
    unsafe {
        assert_eq!(ulua_context_new(&mut c), ULUA_OK);
        let mut grants = vec![];
        for s in signatures {
            let mut desc = CapabilityDesc::default();
            desc.name = s.name.as_ptr().cast();
            desc.name_length = s.name.len();
            desc.parameter_types = s.parameters.as_ptr();
            desc.parameter_count = s.parameters.len();
            desc.result_type = s.result;
            desc.function = Some(result);
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
