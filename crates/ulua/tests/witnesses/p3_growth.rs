//! Host growth compiles and runs every shape at 4/16/64 KiB through the C ABI.
//! The compiler ratio is at most eight for four times the source. Measured Debug cost: 10.700 seconds.
// External ABI descriptors are non-exhaustive and require field assignments.
#![allow(clippy::field_reassign_with_default)]
#[path = "../support/p3_shapes.rs"]
mod shapes;
use std::{ffi::c_void, ptr, time::Instant};
use ulua::*;
unsafe extern "C" fn host(data: *mut c_void, _: *const Value, _: usize, result: *mut Value) -> i32 {
    // SAFETY: Userdata is a live type tag and the runtime supplies writable result storage.
    unsafe {
        (*result).r#type = *data.cast::<u32>();
    }
    0
}
#[test]
fn host_pipeline_growth_and_ceiling() {
    let _measurement = crate::measurement_lock();
    let start = Instant::now();
    for shape in shapes::SHAPES {
        let mut times = vec![];
        for bytes in [4096, 16384, 65536] {
            let source = shapes::source(shape, bytes);
            let signatures = shapes::grants(&source);
            let compiled = ulua_compiler::compile_with_grants(
                source.as_bytes(),
                ulua_compiler::Limits::default(),
                &signatures,
            )
            .unwrap();
            assert!(
                compiled.luau.len()
                    <= ulua_compiler::OUTPUT_PRELUDE_BYTES + bytes * ulua_compiler::OUTPUT_FACTOR
            );
            let mut c = ptr::null_mut();
            let mut m = ptr::null_mut();
            let mut i = ptr::null_mut();
            let mut d = Diagnostic::default();
            // SAFETY: The test owns every source, descriptor, and output; signature storage outlives every host call.
            unsafe {
                assert_eq!(ulua_context_new(&mut c), ULUA_OK);
                let mut grants = vec![];
                for s in &signatures {
                    let mut desc = CapabilityDesc::default();
                    desc.name = s.name.as_ptr().cast();
                    desc.name_length = s.name.len();
                    desc.parameter_types = s.parameters.as_ptr();
                    desc.parameter_count = s.parameters.len();
                    desc.result_type = s.result;
                    desc.function = Some(host);
                    desc.userdata = ptr::from_ref(&s.result).cast_mut().cast();
                    let mut cap = ptr::null_mut();
                    assert_eq!(ulua_capability_new(c, &desc, &mut cap), ULUA_OK);
                    grants.push(cap);
                }
                let mut minimum = f64::INFINITY;
                for _ in 0..3 {
                    let now = Instant::now();
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
                        ULUA_OK,
                        "{:?}",
                        d.code
                    );
                    minimum = minimum.min(now.elapsed().as_secs_f64());
                    assert_eq!(ulua_module_free(m), ULUA_OK);
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
                    ULUA_OK,
                    "{:?}",
                    d.code
                );
                assert_eq!(ulua_context_free(c), ULUA_OK);
                times.push(minimum);
                eprintln!(
                    "P3 pipeline {shape} bytes={bytes} grants={} generated={} ratio_output={:.3} seconds={minimum:.6}",
                    signatures.len(),
                    compiled.luau.len(),
                    compiled.luau.len() as f64 / bytes as f64
                );
            }
        }
        let ratio = times[1] / times[0];
        assert!(ratio <= 8.0, "{shape} compiler ratio={ratio:.3}");
        eprintln!(
            "P3 pipeline {shape} ratio_4_to_16={ratio:.3} ratio_16_to_64={:.3}",
            times[2] / times[1]
        );
    }
    eprintln!(
        "P3 growth shapes={} cost={:.3}s",
        shapes::SHAPES.len(),
        start.elapsed().as_secs_f64()
    );
}
