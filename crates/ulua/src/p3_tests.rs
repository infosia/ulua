//! Direct host ABI tests. Costs are printed; no assertion uses elapsed time.
use crate::*;
use std::{
    ffi::{CStr, c_void},
    ptr, slice,
    time::Instant,
};
struct Run {
    c: *mut Context,
    m: *mut Module,
    i: *mut Instance,
    cap: *mut Capability,
    calls: usize,
    mode: u32,
    result: Value,
    text: Vec<u8>,
    observed: Vec<Value>,
}
impl Run {
    fn new() -> Box<Self> {
        let mut run = Box::new(Self {
            c: ptr::null_mut(),
            m: ptr::null_mut(),
            i: ptr::null_mut(),
            cap: ptr::null_mut(),
            calls: 0,
            mode: 0,
            result: Value::default(),
            text: vec![],
            observed: vec![],
        });
        // SAFETY: The fixture owns writable pointer storage and serializes the context.
        unsafe {
            assert_eq!(ulua_context_new(&mut run.c), ULUA_OK);
        }
        run
    }
    fn descriptor(&mut self, parameters: &[u32], result: u32) -> CapabilityDesc {
        CapabilityDesc {
            name: c"host".as_ptr(),
            name_length: 4,
            parameter_types: parameters.as_ptr(),
            parameter_count: parameters.len(),
            result_type: result,
            function: Some(callback),
            userdata: ptr::from_mut(self).cast(),
            ..CapabilityDesc::default()
        }
    }
    fn capability(&mut self, parameters: &[u32], result: u32) {
        let d = self.descriptor(parameters, result);
        // SAFETY: Descriptor pointers and fixture userdata remain valid through all callback calls.
        unsafe {
            assert_eq!(ulua_capability_new(self.c, &d, &mut self.cap), ULUA_OK);
        }
    }
    fn compile(&mut self, source: &str, grants: &[*mut Capability]) -> (Status, String) {
        let mut d = Diagnostic::default();
        // SAFETY: The fixture owns source, grants, context, and all output storage.
        let status = unsafe {
            ulua_compile(
                self.c,
                source.as_ptr().cast(),
                source.len(),
                grants.as_ptr(),
                grants.len(),
                ptr::null(),
                &mut self.m,
                &mut d,
            )
        };
        (status, code(&d))
    }
    fn instance(&mut self, limits: RunLimits) -> (Status, String) {
        let mut d = Diagnostic::default();
        // SAFETY: The fixture owns both objects and descriptors.
        let status = unsafe {
            ulua_instance_new(
                self.c,
                self.m,
                &Host::default(),
                &limits,
                &mut self.i,
                &mut d,
            )
        };
        (status, code(&d))
    }
    fn call(&mut self, name: &str, args: &[Value]) -> (Status, Diagnostic, Value) {
        let mut index = 0;
        let mut d = Diagnostic::default();
        let mut v = Value::default();
        // SAFETY: The fixture owns its objects and argument and output storage.
        let status = unsafe {
            assert_eq!(
                ulua_module_find_export(self.m, name.as_ptr().cast(), name.len(), &mut index),
                ULUA_OK
            );
            ulua_call(self.i, index, args.as_ptr(), args.len(), &mut v, &mut d)
        };
        (status, d, v)
    }
    fn ready(&mut self, _declaration: &str, body: &str) {
        let source = format!(
            " export function f():void{{{body}}} export function main():void{{}} export function ok():number{{return 42;}}"
        );
        assert_eq!(self.compile(&source, &[self.cap]), (ULUA_OK, String::new()));
        assert_eq!(self.instance(RunLimits::default()).0, ULUA_OK);
    }
}
impl Drop for Run {
    fn drop(&mut self) {
        // SAFETY: Context ownership releases all live child objects once.
        unsafe {
            assert_eq!(ulua_context_free(self.c), ULUA_OK);
        }
    }
}
fn code(d: &Diagnostic) -> String {
    // SAFETY: The ABI terminates the diagnostic arrays.
    unsafe {
        CStr::from_ptr(d.code.as_ptr())
            .to_string_lossy()
            .into_owned()
    }
}
fn message(d: &Diagnostic) -> String {
    // SAFETY: The ABI terminates the diagnostic arrays.
    unsafe {
        CStr::from_ptr(d.message.as_ptr())
            .to_string_lossy()
            .into_owned()
    }
}
unsafe extern "C" fn callback(
    data: *mut c_void,
    args: *const Value,
    count: usize,
    result: *mut Value,
) -> i32 {
    // SAFETY: Registered userdata is the stable fixture; native argument and result storage covers its declared size.
    unsafe {
        let r = &mut *data.cast::<Run>();
        r.calls += 1;
        assert_eq!((*result).r#type, ULUA_TYPE_VOID);
        assert_eq!((*result).number, 0.0);
        assert_eq!((*result).boolean, 0);
        assert_eq!((*result).length, 0);
        assert!((*result).data.is_null());
        r.observed = if count == 0 {
            vec![]
        } else {
            slice::from_raw_parts(args, count).to_vec()
        };
        if r.mode == 5 || r.mode == 6 {
            ulua_internal_test_exhaust_steps((*r.i).vm);
            if r.mode == 5 {
                return 9;
            }
        }
        if r.mode == 1 {
            return 9;
        }
        if r.mode == 2 {
            reenter(r);
        }
        if r.mode == 3 {
            *result = *args;
            return 0;
        }
        *result = r.result;
        if (*result).r#type == ULUA_TYPE_STRING && r.mode != 4 {
            (*result).data = r.text.as_ptr().cast();
            (*result).length = r.text.len();
        }
        0
    }
}
unsafe fn reenter(r: &mut Run) {
    let mut m = ptr::null_mut();
    let mut i = ptr::null_mut();
    let mut c = ptr::null_mut();
    let mut n = 0;
    let mut length = 0;
    let mut name = ptr::null();
    let mut d = Diagnostic::default();
    let mut v = Value::default();
    let desc = r.descriptor(&[], 0);
    // SAFETY: Each call uses live objects and valid storage. BUSY prevents all mutations.
    unsafe {
        assert_eq!(ulua_context_free(r.c), ULUA_BUSY);
        assert_eq!(
            ulua_compile(
                r.c,
                c"".as_ptr(),
                0,
                ptr::null(),
                0,
                ptr::null(),
                &mut m,
                &mut d
            ),
            ULUA_BUSY
        );
        assert_eq!(ulua_module_free(r.m), ULUA_BUSY);
        assert_eq!(ulua_module_export_count(r.m, &mut n), ULUA_BUSY);
        assert_eq!(
            ulua_module_export_name(r.m, 0, &mut name, &mut length),
            ULUA_BUSY
        );
        assert_eq!(
            ulua_module_export_signature(r.m, 0, ptr::null_mut(), 0, &mut length, &mut n),
            ULUA_BUSY
        );
        assert_eq!(
            ulua_module_find_export(r.m, c"f".as_ptr(), 1, &mut n),
            ULUA_BUSY
        );
        assert_eq!(
            ulua_instance_new(r.c, r.m, &Host::default(), ptr::null(), &mut i, &mut d),
            ULUA_BUSY
        );
        assert_eq!(ulua_instance_free(r.i), ULUA_BUSY);
        assert_eq!(ulua_call(r.i, 0, ptr::null(), 0, &mut v, &mut d), ULUA_BUSY);
        assert_eq!(ulua_capability_new(r.c, &desc, &mut c), ULUA_BUSY);
        assert_eq!(
            ulua_declarations(&r.cap, 1, ptr::null_mut(), 0, &mut length),
            ULUA_BUSY
        );
        assert_eq!(ulua_capability_revoke(r.cap), ULUA_BUSY);
        assert_eq!(ulua_capability_free(r.cap), ULUA_BUSY);
    }
}
#[test]
fn capability_descriptors_and_null_frees() {
    let start = Instant::now();
    let mut r = Run::new();
    let base = r.descriptor(&[], 0);
    let mut cases = vec![];
    let mut d = base;
    d.struct_size -= 1;
    cases.push(d);
    let mut d = base;
    d.reserved = 1;
    cases.push(d);
    let mut d = base;
    d.reserved2 = 1;
    cases.push(d);
    let mut d = base;
    d.function = None;
    cases.push(d);
    let mut d = base;
    d.result_type = 4;
    cases.push(d);
    let mut d = base;
    d.parameter_count = 17;
    cases.push(d);
    let mut d = base;
    d.parameter_types = ptr::null();
    d.parameter_count = 1;
    cases.push(d);
    let invalid = [0, 4, u32::MAX];
    for t in &invalid {
        let mut d = base;
        d.parameter_types = t;
        d.parameter_count = 1;
        cases.push(d);
    }
    // SAFETY: All descriptor cases have valid storage; invalid lengths are rejected before dereference.
    unsafe {
        for d in cases {
            let mut cap = ptr::null_mut();
            assert_eq!(
                ulua_capability_new(r.c, &d, &mut cap),
                ULUA_INVALID_ARGUMENT
            );
            assert!(cap.is_null());
        }
        for name in [
            "",
            "a-b",
            "0host",
            "é",
            "print",
            "Utf8",
            "Number",
            "Math",
            "JSON",
            "String",
            "Infinity",
            "NaN",
            "undefined",
            "Error",
            "return",
            "await",
            "eval",
            "arguments",
            "yield",
            "true",
            "null",
        ] {
            let mut d = base;
            d.name = name.as_ptr().cast();
            d.name_length = name.len();
            let mut cap = ptr::null_mut();
            assert_eq!(
                ulua_capability_new(r.c, &d, &mut cap),
                ULUA_INVALID_ARGUMENT,
                "{name}"
            );
        }
        let long = "a".repeat(65);
        let mut d = base;
        d.name = long.as_ptr().cast();
        d.name_length = long.len();
        let mut cap = ptr::null_mut();
        assert_eq!(
            ulua_capability_new(r.c, &d, &mut cap),
            ULUA_INVALID_ARGUMENT
        );
        d.name = ptr::null();
        d.name_length = 1;
        assert_eq!(
            ulua_capability_new(r.c, &d, &mut cap),
            ULUA_INVALID_ARGUMENT
        );
        assert_eq!(
            ulua_capability_new(r.c, ptr::null(), &mut cap),
            ULUA_INVALID_ARGUMENT
        );
        assert_eq!(
            ulua_capability_new(r.c, &base, ptr::null_mut()),
            ULUA_INVALID_ARGUMENT
        );
        assert_eq!(
            ulua_capability_new(ptr::null_mut(), &base, &mut cap),
            ULUA_INVALID_ARGUMENT
        );
        assert_eq!(
            ulua_capability_revoke(ptr::null_mut()),
            ULUA_INVALID_ARGUMENT
        );
        assert_eq!(ulua_capability_free(ptr::null_mut()), ULUA_OK);
        for name in [
            "$host",
            "_host",
            "a1",
            &"a".repeat(64),
            "MathHost",
            "NumberHost",
        ] {
            let mut d = base;
            d.name = name.as_ptr().cast();
            d.name_length = name.len();
            assert_eq!(ulua_capability_new(r.c, &d, &mut cap), ULUA_OK);
            assert_eq!(ulua_capability_free(cap), ULUA_OK);
        }
    }
    r.capability(&[1; 16], 0);
    eprintln!("P3 descriptor cost={:.3}s", start.elapsed().as_secs_f64());
}
#[test]
fn compile_grants_duplicates_revoked_unused_foreign_and_unknown() {
    let mut r = Run::new();
    r.capability(&[1, 2, 3], 1);
    let main = "export function main():void{}";
    assert_eq!(r.compile(main, &[r.cap]).0, ULUA_OK);
    assert_eq!(
        r.compile(
            "export function main():void{const n=host(1,true,'x');}",
            &[]
        ),
        (ULUA_REJECTED, "C18".into())
    );
    assert_eq!(
        r.compile(
            "export function main():void{const n=host(1,true,'x');}",
            &[r.cap]
        )
        .0,
        ULUA_OK
    );
    assert_eq!(
        r.compile(
            "const host=1;export function main():void{print(`${host}`);}",
            &[r.cap]
        )
        .0,
        ULUA_OK
    );
    // SAFETY: An unused or fully shadowed grant produces no retained capability in the module.
    unsafe {
        assert!((*r.m).grants.is_empty());
    }
    assert_eq!(r.compile(main, &[r.cap, r.cap]).0, ULUA_INVALID_ARGUMENT);
    let mut other = Run::new();
    other.capability(&[], 0);
    assert_eq!(r.compile(main, &[other.cap]).0, ULUA_INVALID_ARGUMENT);
    assert_eq!(r.compile(main, &[ptr::null_mut()]).0, ULUA_INVALID_ARGUMENT);
    // SAFETY: Both operations use the fixture's live serialized capability.
    unsafe {
        assert_eq!(ulua_capability_revoke(r.cap), ULUA_OK);
        assert_eq!(ulua_capability_revoke(r.cap), ULUA_OK);
    }
    assert_eq!(r.compile(main, &[r.cap]).0, ULUA_INVALID_ARGUMENT);
}
#[test]
fn values_strings_nan_negative_zero_and_result_copy() {
    let start = Instant::now();
    for (tag, ty) in [(1, "number"), (2, "boolean"), (3, "string")] {
        let mut r = Run::new();
        r.capability(&[tag], tag);
        r.mode = 3;
        let source = format!(
            " export function f(a:{ty}):{ty}{{return host(a);}} export function main():void{{}}"
        );
        assert_eq!(r.compile(&source, &[r.cap]).0, ULUA_OK);
        assert_eq!(r.instance(RunLimits::default()).0, ULUA_OK);
        let texts = ["", "é🙂\0x"];
        let mut values = vec![];
        if tag == 1 {
            for n in [1.5, f64::NAN, -0.0, f64::INFINITY, f64::NEG_INFINITY] {
                let v = Value {
                    r#type: tag,
                    number: n,
                    ..Value::default()
                };
                values.push(v);
            }
        }
        if tag == 2 {
            for b in [0, 1] {
                let v = Value {
                    r#type: tag,
                    boolean: b,
                    ..Value::default()
                };
                values.push(v);
            }
        }
        if tag == 3 {
            for s in texts {
                let v = Value {
                    r#type: tag,
                    data: s.as_ptr().cast(),
                    length: s.len(),
                    ..Value::default()
                };
                values.push(v);
            }
        }
        for v in values {
            let (status, d, result) = r.call("f", &[v]);
            assert_eq!(status, ULUA_OK, "{}", code(&d));
            if tag == 1 {
                assert_eq!(result.number.to_bits(), v.number.to_bits());
                assert_eq!(r.observed[0].number.to_bits(), v.number.to_bits());
            }
            if tag == 2 {
                assert_eq!(result.boolean, v.boolean);
                assert_eq!(r.observed[0].boolean, v.boolean);
            }
            if tag == 3 {
                // SAFETY: The ABI result remains live until the next context operation; both byte ranges cover their lengths.
                unsafe {
                    assert_eq!(
                        slice::from_raw_parts(result.data.cast::<u8>(), result.length),
                        slice::from_raw_parts(v.data.cast::<u8>(), v.length)
                    );
                }
                let mut count = 0;
                // SAFETY: Module queries preserve the result lifetime and write to valid storage.
                unsafe {
                    assert_eq!(ulua_module_export_count(r.m, &mut count), ULUA_OK);
                    assert_eq!(
                        slice::from_raw_parts(result.data.cast::<u8>(), result.length),
                        slice::from_raw_parts(v.data.cast::<u8>(), v.length)
                    );
                }
            }
        }
        if tag == 3 {
            let bytes = [0xff];
            let v = Value {
                r#type: 3,
                data: bytes.as_ptr().cast(),
                length: 1,
                ..Value::default()
            };
            let before = r.calls;
            assert_eq!(r.call("f", &[v]).0, ULUA_INVALID_ARGUMENT);
            assert_eq!(r.calls, before);
        }
    }
    let mut r = Run::new();
    r.capability(&[], 3);
    r.text = vec![b'x'; 1024 * 1024];
    r.result.r#type = 3;
    let source =
        "export function f():string{const a=host();host();return a;}export function main():void{}";
    assert_eq!(r.compile(source, &[r.cap]).0, ULUA_OK);
    assert_eq!(r.instance(RunLimits::default()).0, ULUA_OK);
    let (status, _, v) = r.call("f", &[]);
    assert_eq!(status, ULUA_OK);
    assert_eq!(v.length, 1024 * 1024);
    // SAFETY: The returned string is instance-owned and live through this read.
    unsafe {
        assert!(
            slice::from_raw_parts(v.data.cast::<u8>(), v.length)
                .iter()
                .all(|b| *b == b'x')
        );
    }
    eprintln!("P3 value cost={:.3}s", start.elapsed().as_secs_f64());
}
#[test]
fn stops_skip_catch_finally_and_preserve_instance() {
    for position in [
        "try{host();}catch(e){host();}finally{host();}",
        "try{throw new Error('e');}catch(e){host();}finally{host();}",
        "try{}finally{host();host();}",
    ] {
        let mut r = Run::new();
        r.capability(&[], 0);
        r.mode = 1;
        r.ready("", position);
        let (s, d, _) = r.call("f", &[]);
        assert_eq!(s, ULUA_HOST_ERROR);
        assert_eq!(code(&d), "host:capability");
        assert_eq!(message(&d), "host");
        assert_eq!(r.calls, 1);
        assert_eq!(r.call("ok", &[]).2.number, 42.0);
    }
}
#[test]
fn malformed_results_have_named_stops_and_recovery() {
    let bad_bytes = [0xff];
    let mut malformed = vec![(1, Value::default())];
    let v = Value {
        r#type: 1,
        boolean: 1,
        ..Value::default()
    };
    malformed.push((1, v));
    let v = Value {
        r#type: 2,
        boolean: 2,
        ..Value::default()
    };
    malformed.push((2, v));
    let v = Value {
        r#type: 3,
        data: bad_bytes.as_ptr().cast(),
        length: 1,
        ..Value::default()
    };
    malformed.push((3, v));
    let v = Value {
        r#type: 3,
        length: 1,
        ..Value::default()
    };
    malformed.push((3, v));
    for tag in [0, 1, 2, 3] {
        for field in 0..4 {
            let mut v = Value {
                r#type: tag,
                ..Value::default()
            };
            match field {
                0 if tag != 2 => v.boolean = 1,
                1 if tag != 1 => v.number = 1.0,
                2 if tag != 3 => v.data = c"x".as_ptr(),
                3 if tag != 3 => v.length = 1,
                _ => continue,
            }
            malformed.push((tag, v));
        }
    }
    for (tag, v) in malformed {
        let mut r = Run::new();
        r.capability(&[], tag);
        r.mode = 4;
        r.result = v;
        let body = if tag == 0 {
            "host();"
        } else {
            "const a=host();"
        };
        r.ready("", body);
        let (s, d, _) = r.call("f", &[]);
        assert_eq!(s, ULUA_HOST_ERROR);
        assert_eq!(code(&d), "host:result");
        assert_eq!(message(&d), "host");
        assert_eq!(r.call("ok", &[]).2.number, 42.0);
    }
}

#[test]
fn unused_number_requires_positive_zero_bits() {
    let start = Instant::now();
    for tag in [ULUA_TYPE_VOID, ULUA_TYPE_BOOLEAN, ULUA_TYPE_STRING] {
        let mut r = Run::new();
        r.capability(&[], tag);
        r.result.r#type = tag;
        r.ready("", "host();");
        assert_eq!(r.call("f", &[]).0, ULUA_OK);
        r.result.number = -0.0;
        let (status, diagnostic, _) = r.call("f", &[]);
        assert_eq!(status, ULUA_HOST_ERROR);
        assert_eq!(code(&diagnostic), "host:result");
        assert_eq!(message(&diagnostic), "host");
        r.result.number = 0.0;
        assert_eq!(r.call("f", &[]).0, ULUA_OK);
    }
    eprintln!(
        "P3 unused number positive/negative zero tags=3 cost={:.3}s",
        start.elapsed().as_secs_f64()
    );
}
#[test]
fn revoke_and_free_keep_module_and_instance_references() {
    for free in [false, true] {
        let mut r = Run::new();
        r.capability(&[], 0);
        r.ready("", "host();");
        // SAFETY: The capability is live; free invalidates only its host pointer.
        unsafe {
            assert_eq!(
                if free {
                    ulua_capability_free(r.cap)
                } else {
                    ulua_capability_revoke(r.cap)
                },
                ULUA_OK
            );
        }
        r.cap = ptr::null_mut();
        let (s, d, _) = r.call("f", &[]);
        assert_eq!(s, ULUA_HOST_ERROR);
        assert_eq!(code(&d), "host:revoked");
        assert_eq!(message(&d), "host");
        assert_eq!(r.calls, 0);
        assert_eq!(r.call("ok", &[]).2.number, 42.0);
        let mut second = ptr::null_mut();
        let mut d = Diagnostic::default();
        // SAFETY: The retained module remains live and the new instance output is writable.
        unsafe {
            assert_eq!(
                ulua_instance_new(r.c, r.m, &Host::default(), ptr::null(), &mut second, &mut d),
                ULUA_OK
            );
            assert_eq!(ulua_instance_free(second), ULUA_OK);
        }
    }
}
#[test]
fn callback_busy_and_library_panic_poisoning() {
    let mut r = Run::new();
    r.capability(&[], 0);
    r.mode = 2;
    r.ready("", "host();");
    assert_eq!(r.call("f", &[]).0, ULUA_OK);
    assert_eq!(r.calls, 1);
    crate::capabilities::PANIC_HOST.with(|v| v.set(true));
    let (s, d, _) = r.call("f", &[]);
    assert_eq!(s, ULUA_INTERNAL);
    assert_eq!(code(&d), "internal");
    let mut v = Value::default();
    // SAFETY: The poisoned instance remains live; all outputs have valid storage.
    unsafe {
        assert_eq!(
            ulua_call(r.i, 0, ptr::null(), 0, &mut v, &mut d.clone()),
            ULUA_POISONED
        );
    }
    // SAFETY: Free functions remain valid after poisoning.
    unsafe {
        assert_eq!(ulua_capability_revoke(r.cap), ULUA_POISONED);
        assert_eq!(ulua_capability_free(r.cap), ULUA_OK);
        r.cap = ptr::null_mut();
    }
}
#[test]
fn host_string_memory_stop_and_recovery() {
    let mut r = Run::new();
    r.capability(&[], 3);
    r.text = vec![b'x'; 1024 * 1024];
    r.result.r#type = 3;
    r.ready("", "const a=host();");
    // SAFETY: The fixture frees its instance before replacing it.
    unsafe {
        assert_eq!(ulua_instance_free(r.i), ULUA_OK);
        r.i = ptr::null_mut();
    }
    let limits = RunLimits {
        max_memory_bytes: 512 * 1024,
        ..RunLimits::default()
    };
    assert_eq!(r.instance(limits).0, ULUA_OK);
    let (s, d, _) = r.call("f", &[]);
    assert_eq!(s, ULUA_FAILED);
    assert_eq!(code(&d), "limit:memory");
    assert_eq!(r.call("ok", &[]).2.number, 42.0);
}
#[test]
fn exactly_one_step_per_host_call() {
    #[link(name = "ulua_test_native", kind = "static")]
    // SAFETY: The test-only native archive reads the live VM step counter.
    unsafe extern "C" {
        fn ulua_internal_test_host_steps(vm: *const c_void) -> u64;
        fn ulua_internal_test_no_interrupt(vm: *mut c_void);
    }
    let mut r = Run::new();
    r.capability(&[], 0);
    let source = "export function one():void{host();}export function two():void{host();host();}export function three():void{host();host();host();}export function main():void{}";
    assert_eq!(r.compile(source, &[r.cap]).0, ULUA_OK);
    assert_eq!(r.instance(RunLimits::default()).0, ULUA_OK);
    // SAFETY: The test owns the live VM; this test-only hook isolates native host charges.
    unsafe {
        ulua_internal_test_no_interrupt((*r.i).vm);
    }
    let mut charges = vec![];
    for (name, count) in [("one", 1), ("two", 2), ("three", 3)] {
        r.call(name, &[]);
        let before = r.calls;
        assert_eq!(r.call(name, &[]).0, ULUA_OK);
        assert_eq!(r.calls - before, count);
        // SAFETY: The fixture keeps its instance and VM live through the native query.
        charges.push(unsafe { ulua_internal_test_host_steps((*r.i).vm) });
    }
    assert_eq!(charges[1] - charges[0], 1);
    assert_eq!(charges[2] - charges[1], 1);
    eprintln!("P3 host charges calls=1/2/3 total_steps={charges:?}");
}

#[test]
fn host_steps_include_the_vm_call_interrupt() {
    let start = Instant::now();
    let mut r = Run::new();
    r.capability(&[], ULUA_TYPE_VOID);
    let mut source = String::from("export function main():void{}");
    for (name, calls) in [("zero", 0), ("one", 1), ("two", 2)] {
        source.push_str(&format!(
            "export function {name}(n:number):number{{let i=0;while(i<n){{{}i++;}}return i;}}",
            "host();".repeat(calls)
        ));
    }
    assert_eq!(r.compile(&source, &[r.cap]).0, ULUA_OK);
    // These loops cost more steps than instance setup, so setup cannot set the measured threshold.
    for iterations in [4096, 8192] {
        let mut costs = vec![];
        for (name, calls) in [("zero", 0), ("one", 1), ("two", 2)] {
            let mut low = 0;
            let mut high = DEFAULT_STEPS;
            while high - low > 1 {
                let budget = low + (high - low) / 2;
                let limits = RunLimits {
                    max_steps: budget,
                    ..RunLimits::default()
                };
                assert_eq!(r.instance(limits).0, ULUA_OK);
                let before = r.calls;
                let (status, diagnostic, result) = r.call(
                    name,
                    &[Value {
                        r#type: ULUA_TYPE_NUMBER,
                        number: iterations as f64,
                        ..Value::default()
                    }],
                );
                if status == ULUA_OK {
                    assert_eq!(result.number, iterations as f64);
                    assert_eq!(r.calls - before, iterations * calls);
                    high = budget;
                } else {
                    assert_eq!(status, ULUA_FAILED);
                    assert_eq!(code(&diagnostic), "limit:steps");
                    low = budget;
                }
                // SAFETY: The fixture releases each live instance after its budget probe.
                unsafe {
                    assert_eq!(ulua_instance_free(r.i), ULUA_OK);
                }
                r.i = ptr::null_mut();
            }
            costs.push(high);
        }
        assert_eq!(costs[1] - costs[0], 2 * iterations as u64);
        assert_eq!(costs[2] - costs[1], 2 * iterations as u64);
        eprintln!(
            "P3 interrupt-on iterations={iterations} calls=0/1/2 minimum_steps={costs:?} host_cost=2"
        );
    }
    eprintln!(
        "P3 interrupt-on bisection cost={:.3}s",
        start.elapsed().as_secs_f64()
    );
}
#[test]
fn descriptor_storage_is_copied_and_sixteen_arguments_work() {
    let mut r = Run::new();
    let mut name = b"host".to_vec();
    let mut types = vec![1; 16];
    let mut d = r.descriptor(&types, 0);
    d.name = name.as_ptr().cast();
    // SAFETY: The descriptor and output cover their lengths; the library copies borrowed fields.
    unsafe {
        assert_eq!(ulua_capability_new(r.c, &d, &mut r.cap), ULUA_OK);
    }
    name.fill(b'x');
    types.fill(3);
    let arguments = (0..16).map(|i| i.to_string()).collect::<Vec<_>>().join(",");
    r.ready("", &format!("host({arguments});"));
    assert_eq!(r.call("f", &[]).0, ULUA_OK);
    assert_eq!(r.observed.len(), 16);
    for (i, v) in r.observed.iter().enumerate() {
        assert_eq!(v.number, i as f64);
    }
}
unsafe extern "C" fn output_reentry(
    data: *mut c_void,
    _: *const std::ffi::c_char,
    _: usize,
) -> i32 {
    // SAFETY: The host descriptor carries a stable Run allocation; reentry uses its live objects.
    unsafe {
        reenter(&mut *data.cast::<Run>());
    }
    0
}
#[test]
fn output_callback_has_the_same_busy_rules() {
    let mut r = Run::new();
    r.capability(&[], 0);
    assert_eq!(
        r.compile("export function main():void{print('x');}", &[r.cap])
            .0,
        ULUA_OK
    );
    let host = Host {
        userdata: ptr::from_mut(&mut *r).cast(),
        output: Some(output_reentry),
        ..Host::default()
    };
    let mut d = Diagnostic::default();
    // SAFETY: The fixture owns all objects and the host callback data remains live.
    unsafe {
        assert_eq!(
            ulua_instance_new(r.c, r.m, &host, ptr::null(), &mut r.i, &mut d),
            ULUA_OK
        );
    }
    assert_eq!(r.call("main", &[]).0, ULUA_OK);
}

#[link(name = "ulua_test_native", kind = "static")]
// SAFETY: The test-only native functions change limits of the fixture's live VM.
unsafe extern "C" {
    fn ulua_internal_test_exhaust_steps(vm: *mut c_void);
    fn ulua_internal_test_restore_steps(vm: *mut c_void, steps: u64);
}
#[test]
fn recorded_host_stop_survives_unwind_at_the_step_boundary() {
    for (mode, tag, expected) in [(5, 0, "host:capability"), (6, 1, "host:result")] {
        let mut r = Run::new();
        r.capability(&[], tag);
        r.mode = mode;
        r.ready("", "try{host();}catch(e){host();}finally{host();}");
        let (status, d, _) = r.call("f", &[]);
        assert_eq!(status, ULUA_HOST_ERROR, "{}", code(&d));
        assert_eq!(code(&d), expected);
        assert_eq!(message(&d), "host");
        assert_eq!(r.calls, 1);
        // SAFETY: The fixture owns a live instance. The hook restores the production budget for recovery.
        unsafe {
            ulua_internal_test_restore_steps((*r.i).vm, DEFAULT_STEPS);
        }
        assert_eq!(r.call("ok", &[]).2.number, 42.0);
    }
}

#[test]
fn resumed_host_stops_skip_handlers_finalizers_and_jobs() {
    // SAFETY: The native probes match these declarations and receive a live serialized VM.
    #[cfg(feature = "measurements")]
    unsafe extern "C" {
        fn ulua_internal_test_no_interrupt(vm: *mut c_void);
        fn ulua_internal_test_resume_stats(
            vm: *mut c_void,
            checks: *mut u64,
            allocations: *mut u64,
        );
    }
    let start = Instant::now();
    for (mode, revoked, expected) in [
        (1, false, "host:capability"),
        (0, true, "host:revoked"),
        (4, false, "host:result"),
    ] {
        for form in ["control", "await", "then", "catch", "finally", "map"] {
            let mut r = Run::new();
            r.capability(&[], ULUA_TYPE_NUMBER);
            r.mode = mode;
            let action = "try{return host();}catch{cleanup++;return 0;}finally{cleanup++;}";
            let callback = match form {
            "control" => "const n=host();".to_owned(),
            "then" => format!("const p=tick().then(():number=>{{{action}}});const q=later();await p;await q;"),
            "catch" => format!("const p=fail().catch((e:unknown):number=>{{{action}}});const q=later();await p;await q;"),
            "finally" => "const p=tick().finally(():void=>{try{host();}catch{cleanup++;}finally{cleanup++;}});const q=later();await p;await q;".to_owned(),
            "map" => format!("const q=later();const ps=[1].map(async(n:number):Promise<number>=>{{{action}}});await Promise.all(ps);await q;"),
            _ => "const p=task();const q=later();await p;await q;".to_owned(),
        };
            let callback = callback.replace(";const q=later();", ".catch(():number=>{cleanup++;return 0;}).finally(():void=>{cleanup++;});const q=later();");
            let source = format!(
                "let cleanup:number=0;async function tick():Promise<number>{{return 1;}}async function fail():Promise<number>{{throw new Error();}}async function task():Promise<number>{{await tick();{action}}}function later():Promise<number>{{return tick().then(():number=>{{cleanup++;return 0;}});}}export async function f():Promise<void>{{{callback}}}export function read():number{{return cleanup;}}export function ok():number{{return 42;}}export function main():void{{}}"
            );
            assert_eq!(r.compile(&source, &[r.cap]).0, ULUA_OK);
            assert_eq!(r.instance(RunLimits::default()).0, ULUA_OK);
            if revoked {
                // SAFETY: The fixture owns this live capability and serializes its revocation.
                unsafe {
                    assert_eq!(ulua_capability_revoke(r.cap), ULUA_OK);
                }
            }
            #[cfg(feature = "measurements")]
            // SAFETY: Disable interrupts to expose the scheduler's recorded-stop check directly.
            unsafe {
                ulua_internal_test_no_interrupt((*r.i).vm);
            }
            let (status, d, _) = r.call("f", &[]);
            #[cfg(feature = "measurements")]
            {
                let mut checks = 0;
                let mut allocations = 0;
                // SAFETY: The fixture owns the VM and writable counter storage.
                unsafe {
                    ulua_internal_test_resume_stats((*r.i).vm, &mut checks, &mut allocations);
                }
                assert!(checks > 0);
                assert_eq!(allocations, 0);
                eprintln!(
                    "P4.1/P4.2 host stop {expected} form={form} checks={checks} allocations={allocations}"
                );
            }
            assert_eq!(status, ULUA_HOST_ERROR);
            assert_eq!(code(&d), expected);
            assert_eq!(r.calls, usize::from(!revoked));
            assert_eq!(r.call("read", &[]).2.number, 0.0);
            assert_eq!(r.call("ok", &[]).2.number, 42.0);
        }
    }
    eprintln!(
        "P4.1/P4.2 host stops controls=3 protected=15 cost={:.3}s",
        start.elapsed().as_secs_f64()
    );
}
