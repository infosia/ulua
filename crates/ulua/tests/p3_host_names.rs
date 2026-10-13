//! Compare the fixed name lists with the ES2022 library and prelude at test time.
// External ABI descriptors are non-exhaustive and require field assignments.
#![allow(clippy::field_reassign_with_default)]
#[path = "../../ulua_compiler/src/host_names.rs"]
mod names;
use std::{ffi::c_void, path::PathBuf, process::Command, ptr, time::Instant};
use ulua::*;
unsafe extern "C" fn callback(_: *mut c_void, _: *const Value, _: usize, _: *mut Value) -> i32 {
    0
}
#[test]
fn global_names_match_tsc_and_have_direct_abi_rejections() {
    let now = Instant::now();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let node = Command::new("node")
        .current_dir(&root)
        .args([
            "--input-type=module",
            "-e",
            include_str!("support/p3_global_names.mjs"),
        ])
        .output()
        .unwrap();
    assert!(
        node.status.success(),
        "{}",
        String::from_utf8_lossy(&node.stderr)
    );
    let measured = String::from_utf8(node.stdout).unwrap();
    let (values, types) = measured.split_once("\n\n").unwrap();
    assert_eq!(names::GLOBAL_NAMES, values.lines().collect::<Vec<_>>());
    assert_eq!(names::GLOBAL_TYPE_NAMES, types.lines().collect::<Vec<_>>());
    let mut c = ptr::null_mut();
    // SAFETY: This fixture owns the context, descriptors, and outputs and serializes every call.
    unsafe {
        assert_eq!(ulua_context_new(&mut c), ULUA_OK);
        for name in names::GLOBAL_NAMES.iter().copied().chain(["arguments"]) {
            let mut desc = CapabilityDesc::default();
            desc.name = name.as_ptr().cast();
            desc.name_length = name.len();
            desc.function = Some(callback);
            let mut cap = ptr::null_mut();
            assert_eq!(
                ulua_capability_new(c, &desc, &mut cap),
                ULUA_INVALID_ARGUMENT,
                "{name}"
            );
            assert!(cap.is_null());
        }
        for name in [
            "NumberHost",
            "MathHost",
            "JSONHost",
            "number",
            "math",
            "$print",
            "hostGlobalThis",
        ] {
            let mut desc = CapabilityDesc::default();
            desc.name = name.as_ptr().cast();
            desc.name_length = name.len();
            desc.function = Some(callback);
            let mut cap = ptr::null_mut();
            assert_eq!(ulua_capability_new(c, &desc, &mut cap), ULUA_OK, "{name}");
            assert_eq!(ulua_capability_free(cap), ULUA_OK);
        }
        assert_eq!(ulua_context_free(c), ULUA_OK);
    }
    eprintln!(
        "P3 global value names={} type_only={} direct_invalid={} valid_near_misses=7 cost={:.3}s",
        names::GLOBAL_NAMES.len(),
        names::GLOBAL_TYPE_NAMES.len(),
        names::GLOBAL_NAMES.len() + 1,
        now.elapsed().as_secs_f64()
    );
}
