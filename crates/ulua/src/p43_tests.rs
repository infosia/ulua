//! Direct P4.3 ABI cases. Report cost; no assertion uses elapsed time.
use crate::*;
use std::{
    collections::VecDeque,
    ffi::{CStr, c_char, c_void},
    ptr,
    time::Instant,
};
#[derive(Default)]
struct Data {
    requests: VecDeque<(u64, Vec<Value>)>,
    output: Vec<String>,
    strings: Vec<String>,
    context: *mut Context,
    instance: *mut Instance,
    busy: Vec<Status>,
    fail: bool,
}
unsafe extern "C" fn start(data: *mut c_void, args: *const Value, count: usize, id: u64) -> i32 {
    // SAFETY: The fixture owns the data and native arguments through the callback.
    unsafe {
        let d = &mut *data.cast::<Data>();
        let args = if count == 0 {
            &[]
        } else {
            std::slice::from_raw_parts(args, count)
        };
        // String snapshots retain metadata only. String bytes are copied below.
        for a in args.iter().filter(|a| a.r#type == ULUA_TYPE_STRING) {
            let bytes = std::slice::from_raw_parts(a.data.cast(), a.length);
            d.strings
                .push(std::str::from_utf8(bytes).unwrap().to_owned());
        }
        d.requests.push_back((id, args.to_vec()));
        if !d.instance.is_null() {
            let mut result = Value::default();
            let mut task = 0;
            let mut n = 0;
            d.busy.extend([
                ulua_request_complete(d.instance, id, &result),
                ulua_request_fail(d.instance, id),
                ulua_pump(d.instance, 0, &mut n, ptr::null_mut()),
                ulua_instance_requests(d.instance, &mut n, &mut 0),
                ulua_call_start(d.instance, 0, ptr::null(), 0, &mut task, ptr::null_mut()),
                ulua_task_result(d.instance, task, &mut result, ptr::null_mut()),
                ulua_instance_free(d.instance),
                ulua_context_free(d.context),
            ]);
        }
        i32::from(d.fail)
    }
}
unsafe extern "C" fn sync(_: *mut c_void, _: *const Value, _: usize, _: *mut Value) -> i32 {
    0
}
unsafe extern "C" fn output(data: *mut c_void, text: *const c_char, len: usize) -> i32 {
    // SAFETY: Fixture data stays live; native text covers len bytes through this callback.
    unsafe {
        let bytes = std::slice::from_raw_parts(text.cast(), len);
        (*data.cast::<Data>())
            .output
            .push(std::str::from_utf8(bytes).unwrap().to_owned());
    }
    0
}
fn code(d: &Diagnostic) -> String {
    // SAFETY: ABI diagnostics terminate both strings.
    unsafe {
        CStr::from_ptr(d.code.as_ptr())
            .to_string_lossy()
            .into_owned()
    }
}
struct Run {
    c: *mut Context,
    m: *mut Module,
    i: *mut Instance,
    caps: Vec<*mut Capability>,
    data: Box<Data>,
}
impl Run {
    fn new(source: &str, tags: &[u32], limits: RunLimits) -> Self {
        let source = if source.contains("main():Promise<number>")
            || source.contains("main():Promise<string>")
        {
            source.replace("function main()", "function work()") + "export function main():void{}"
        } else {
            source.to_owned()
        };
        let mut r = Self {
            c: ptr::null_mut(),
            m: ptr::null_mut(),
            i: ptr::null_mut(),
            caps: vec![],
            data: Box::default(),
        };
        // SAFETY: The fixture owns and serializes all objects and descriptor storage.
        unsafe {
            assert_eq!(ulua_context_new(&mut r.c), ULUA_OK);
            r.data.context = r.c;
            let parameters = [1, 2, 3];
            for &tag in tags {
                let name = format!("host{tag}");
                let desc = CapabilityDesc {
                    name: name.as_ptr().cast(),
                    name_length: name.len(),
                    parameter_types: parameters.as_ptr(),
                    parameter_count: 3,
                    result_type: tag,
                    start: Some(start),
                    userdata: ptr::from_mut(&mut *r.data).cast(),
                    ..CapabilityDesc::default()
                };
                let mut cap = ptr::null_mut();
                assert_eq!(ulua_capability_new(r.c, &desc, &mut cap), ULUA_OK);
                r.caps.push(cap);
            }
            let mut d = Diagnostic::default();
            assert_eq!(
                ulua_compile(
                    r.c,
                    source.as_ptr().cast(),
                    source.len(),
                    r.caps.as_ptr(),
                    r.caps.len(),
                    ptr::null(),
                    &mut r.m,
                    &mut d
                ),
                ULUA_OK,
                "{} {source}",
                code(&d)
            );
            let host = Host {
                userdata: ptr::from_mut(&mut *r.data).cast(),
                output: Some(output),
                ..Host::default()
            };
            assert_eq!(
                ulua_instance_new(r.c, r.m, &host, &limits, &mut r.i, &mut d),
                ULUA_OK,
                "{}",
                code(&d)
            );
            r.data.instance = r.i;
        }
        r
    }
    fn index(&self, name: &str) -> u32 {
        let name = if name == "main" {
            // SAFETY: The fixture owns the module and output storage.
            let mut work = 0;
            if unsafe { ulua_module_find_export(self.m, c"work".as_ptr(), 4, &mut work) } == ULUA_OK
            {
                return work;
            }
            name
        } else {
            name
        };
        let mut index = 0;
        // SAFETY: The fixture owns the module and name bytes.
        unsafe {
            assert_eq!(
                ulua_module_find_export(self.m, name.as_ptr().cast(), name.len(), &mut index),
                ULUA_OK
            );
        }
        index
    }
    fn start(&self, name: &str) -> (Status, Diagnostic, u64) {
        let mut d = Diagnostic::default();
        let mut task = u64::MAX;
        // SAFETY: The fixture supplies live objects and local output storage.
        let status =
            unsafe { ulua_call_start(self.i, self.index(name), ptr::null(), 0, &mut task, &mut d) };
        (status, d, task)
    }
    fn result(&self, task: u64) -> (Status, Diagnostic, Value) {
        let mut d = Diagnostic::default();
        let mut value = Value::default();
        // SAFETY: The fixture supplies live objects and local output storage.
        let status = unsafe { ulua_task_result(self.i, task, &mut value, &mut d) };
        (status, d, value)
    }
    fn counts(&self) -> (u32, u32) {
        let (mut open, mut recorded) = (0, 0);
        // SAFETY: The fixture supplies a live instance and local output storage.
        unsafe {
            assert_eq!(
                ulua_instance_requests(self.i, &mut open, &mut recorded),
                ULUA_OK
            );
        }
        (open, recorded)
    }
    fn complete(&self, id: u64, value: Value) -> Status {
        // SAFETY: The instance and value bytes stay live through this serialized call.
        unsafe { ulua_request_complete(self.i, id, &value) }
    }
    fn pump(&self, maximum: u32) -> (Status, Diagnostic, u32) {
        let mut d = Diagnostic::default();
        let mut settled = u32::MAX;
        // SAFETY: The fixture supplies a live instance and local output storage.
        let status = unsafe { ulua_pump(self.i, maximum, &mut settled, &mut d) };
        (status, d, settled)
    }
}
impl Drop for Run {
    fn drop(&mut self) {
        // SAFETY: The fixture releases its live context and all remaining children exactly once.
        unsafe {
            assert_eq!(ulua_context_free(self.c), ULUA_OK);
        }
    }
}
fn number(n: f64) -> Value {
    Value {
        r#type: 1,
        number: n,
        ..Value::default()
    }
}

#[test]
fn async_descriptor_declarations_and_limits() {
    let now = Instant::now();
    let r = Run::new(
        "export function main():void{}",
        &[0, 1, 2, 3],
        RunLimits::default(),
    );
    // SAFETY: Test-local descriptors and outputs satisfy the ABI preconditions.
    unsafe {
        let mut cap = ptr::null_mut();
        let mut desc = CapabilityDesc {
            name: c"later".as_ptr(),
            name_length: 5,
            ..CapabilityDesc::default()
        };
        for (function, callback) in [
            (None, None),
            (Some(sync as HostFn), Some(start as HostStartFn)),
        ] {
            desc.function = function;
            desc.start = callback;
            assert_eq!(
                ulua_capability_new(r.c, &desc, &mut cap),
                ULUA_INVALID_ARGUMENT
            );
            assert!(cap.is_null());
        }
        desc.function = None;
        desc.start = Some(start);
        desc.struct_size -= 1;
        assert_eq!(
            ulua_capability_new(r.c, &desc, &mut cap),
            ULUA_INVALID_ARGUMENT
        );
        let mut limits = RunLimits::default();
        assert_eq!(ulua_default_run_limits(&mut limits), ULUA_OK);
        assert_eq!(limits.max_open_requests, 256);
        limits.reserved2 = 1;
        let mut i = ptr::null_mut();
        assert_eq!(
            ulua_instance_new(r.c, r.m, &Host::default(), &limits, &mut i, ptr::null_mut()),
            ULUA_INVALID_ARGUMENT
        );
        let mut text = [0u8; 1024];
        let mut length = 0;
        assert_eq!(
            ulua_declarations(
                r.caps.as_ptr(),
                4,
                text.as_mut_ptr().cast(),
                text.len(),
                &mut length
            ),
            ULUA_OK
        );
        let expected = "// Ulua host declarations. Generated by ulua_declarations.\ndeclare function host0(p0: number, p1: boolean, p2: string): Promise<void>;\ndeclare function host1(p0: number, p1: boolean, p2: string): Promise<number>;\ndeclare function host2(p0: number, p1: boolean, p2: string): Promise<boolean>;\ndeclare function host3(p0: number, p1: boolean, p2: string): Promise<string>;\n";
        assert_eq!(&text[..length], expected.as_bytes());
        assert_eq!(ulua_capability_revoke(r.caps[0]), ULUA_OK);
        assert_eq!(
            ulua_declarations(
                r.caps.as_ptr(),
                4,
                text.as_mut_ptr().cast(),
                text.len(),
                &mut length
            ),
            ULUA_OK
        );
        assert_eq!(&text[..length], expected.as_bytes());
        text.fill(42);
        assert_eq!(
            ulua_declarations(r.caps.as_ptr(), 4, text.as_mut_ptr().cast(), 1, &mut length),
            ULUA_INVALID_ARGUMENT
        );
        assert_eq!(length, expected.len());
        assert!(text.iter().all(|b| *b == 42));
    }
    eprintln!(
        "P4.3 descriptors declarations cases=8 cost={:.3}s",
        now.elapsed().as_secs_f64()
    );
}
#[test]
fn ulua_request_complete_types_validation_copy_and_busy() {
    let now = Instant::now();
    let mut r = Run::new(
        "export async function main():Promise<string>{await host0(2.5,true,'ok');print(`${await host1(2.5,true,'ok')}`);print(`${await host2(2.5,true,'ok')}`);return await host3(2.5,true,'ok');}",
        &[0, 1, 2, 3],
        RunLimits::default(),
    );
    let task = r.start("main").2;
    let mut string = "é🦀\0".as_bytes().to_vec();
    let values = [
        Value::default(),
        number(f64::NAN),
        Value {
            r#type: 2,
            boolean: 1,
            ..Value::default()
        },
        Value {
            r#type: 3,
            data: string.as_ptr().cast(),
            length: string.len(),
            ..Value::default()
        },
    ];
    for value in values {
        assert_eq!(r.result(task).0, ULUA_PENDING);
        let (id, args) = r.data.requests.pop_front().unwrap();
        assert_eq!(args[0].number, 2.5);
        assert_eq!(args[1].boolean, 1);
        assert_eq!(args[2].length, 2);
        assert_eq!(r.counts(), (1, 0));
        assert_eq!(r.complete(u64::MAX, value), ULUA_INVALID_ARGUMENT);
        let wrong = if value.r#type == 0 {
            number(0.0)
        } else {
            Value::default()
        };
        assert_eq!(r.complete(id, wrong), ULUA_INVALID_ARGUMENT);
        assert_eq!(r.counts(), (1, 0));
        assert_eq!(r.complete(id, value), ULUA_OK);
        if value.r#type == 3 {
            string.fill(b'x');
        }
        assert_eq!(r.complete(id, value), ULUA_INVALID_ARGUMENT);
        // SAFETY: The fixture keeps the instance live through failure validation.
        unsafe {
            assert_eq!(ulua_request_fail(r.i, id), ULUA_INVALID_ARGUMENT);
        }
        assert_eq!(r.counts(), (1, 1));
        assert_eq!(r.pump(1).0, ULUA_OK);
        assert_eq!(r.complete(id, value), ULUA_INVALID_ARGUMENT);
    }
    let result = r.result(task);
    assert_eq!(result.0, ULUA_OK);
    // SAFETY: The instance owns the string until the next ABI call.
    unsafe {
        assert_eq!(
            std::slice::from_raw_parts(result.2.data.cast::<u8>(), result.2.length),
            "é🦀\0".as_bytes()
        );
    }
    assert_eq!(r.data.output, ["NaN", "true"]);
    assert_eq!(r.data.strings, ["ok", "ok", "ok", "ok"]);
    assert_eq!(r.counts(), (0, 0));
    assert_eq!(r.result(task).0, ULUA_INVALID_ARGUMENT);
    assert_eq!(r.data.busy.len(), 32);
    assert!(r.data.busy.iter().all(|s| *s == ULUA_BUSY));
    eprintln!(
        "P4.3 completion types=4 callback_busy=32 cost={:.3}s",
        now.elapsed().as_secs_f64()
    );
}
#[test]
fn complete_rejects_invalid_value_fields_and_utf8() {
    let mut r = Run::new(
        "export function main():void{const p=host3(1,true,'s');}",
        &[3],
        RunLimits::default(),
    );
    assert_eq!(r.start("main").0, ULUA_OK);
    let id = r.data.requests.pop_front().unwrap().0;
    let invalid_utf8 = [0xffu8];
    for value in [
        Value {
            r#type: 3,
            data: invalid_utf8.as_ptr().cast(),
            length: 1,
            ..Value::default()
        },
        Value {
            r#type: 3,
            length: 1,
            ..Value::default()
        },
        Value {
            r#type: 3,
            boolean: 1,
            ..Value::default()
        },
        Value {
            r#type: 3,
            number: -0.0,
            ..Value::default()
        },
        Value {
            r#type: 1,
            length: 1,
            ..Value::default()
        },
    ] {
        assert_eq!(r.complete(id, value), ULUA_INVALID_ARGUMENT);
        assert_eq!(r.counts(), (1, 0));
    }
    assert_eq!(
        r.complete(
            id,
            Value {
                r#type: 3,
                ..Value::default()
            }
        ),
        ULUA_OK
    );
    assert_eq!(r.pump(0).0, ULUA_OK);
}
#[test]
fn ulua_request_fail_stops_without_script_and_preserves_later_records() {
    let mut r = Run::new(
        "async function wait():Promise<void>{try{await host1(1,true,'s');print('after');}catch{print('catch');}finally{print('finally');}}export function main():void{const a=wait();const b=host1(2,true,'s').then((n:number):void=>print(`${n}`));}export function good():number{return 42;}",
        &[1],
        RunLimits::default(),
    );
    assert_eq!(r.start("main").0, ULUA_OK);
    let first = r.data.requests.pop_front().unwrap().0;
    let second = r.data.requests.pop_front().unwrap().0;
    // SAFETY: The fixture owns the instance and request ids.
    unsafe {
        assert_eq!(ulua_request_fail(r.i, u64::MAX), ULUA_INVALID_ARGUMENT);
        assert_eq!(ulua_request_fail(r.i, first), ULUA_OK);
        assert_eq!(ulua_request_fail(r.i, first), ULUA_INVALID_ARGUMENT);
    }
    assert_eq!(r.complete(second, number(2.0)), ULUA_OK);
    let stopped = r.pump(0);
    assert_eq!(stopped.0, ULUA_HOST_ERROR);
    assert_eq!(code(&stopped.1), "host:request");
    assert_eq!(stopped.2, 1);
    assert!(r.data.output.is_empty());
    assert_eq!(r.counts(), (1, 1));
    assert_eq!(r.pump(0).0, ULUA_OK);
    assert_eq!(r.data.output, ["2"]);
    assert_eq!(r.counts(), (0, 0));
    let task = r.start("good").2;
    assert_eq!(r.result(task).2.number, 42.0);
}
#[test]
fn ulua_pump_record_order_maximum_and_empty() {
    let mut r = Run::new(
        "export function main():void{for(let i=0;i<3;i++){const p=host1(i,true,'s').then((n:number):void=>print(`${n}`));}}",
        &[1],
        RunLimits::default(),
    );
    assert_eq!(r.start("main").0, ULUA_OK);
    let ids: Vec<_> = r.data.requests.drain(..).map(|r| r.0).collect();
    assert!(ids.windows(2).all(|w| w[0] < w[1]));
    assert_eq!(r.pump(0).2, 0);
    assert_eq!(r.counts(), (3, 0));
    for index in [2, 0, 1] {
        assert_eq!(r.complete(ids[index], number(index as f64)), ULUA_OK);
    }
    assert!(r.data.output.is_empty());
    assert_eq!(r.counts(), (3, 3));
    assert_eq!(r.pump(1).2, 1);
    assert_eq!(r.data.output, ["2"]);
    assert_eq!(r.counts(), (2, 2));
    assert_eq!(r.pump(0).2, 2);
    assert_eq!(r.data.output, ["2", "0", "1"]);
    assert_eq!(r.pump(5).2, 0);
    assert_eq!(r.counts(), (0, 0));
    assert_eq!(r.start("main").0, ULUA_OK);
    assert!(r.data.requests[0].0 > *ids.last().unwrap());
}
#[test]
fn ulua_call_start_and_ulua_task_result_sync_pending_fulfilled_rejected_consumed() {
    let mut r = Run::new(
        "export function sync():string{return 'ok';}export async function main():Promise<number>{return await host1(1,true,'s');}export async function reject():Promise<number>{await host1(2,true,'s');throw new TypeError('bad');}export async function immediate():Promise<boolean>{return true;}",
        &[1],
        RunLimits::default(),
    );
    let task = r.start("sync").2;
    let result = r.result(task);
    assert_eq!(result.0, ULUA_OK);
    assert_eq!(result.2.length, 2);
    assert_eq!(r.result(task).0, ULUA_INVALID_ARGUMENT);
    let immediate = r.start("immediate").2;
    assert_ne!(task, immediate);
    assert_eq!(r.result(immediate).2.boolean, 1);
    let task = r.start("main").2;
    let failed = r.start("reject").2;
    assert_ne!(task, failed);
    assert_eq!(r.result(task).0, ULUA_PENDING);
    assert_eq!(r.result(task).0, ULUA_PENDING);
    assert_eq!(r.result(u64::MAX).0, ULUA_INVALID_ARGUMENT);
    for (id, args) in r.data.requests.drain(..).collect::<Vec<_>>() {
        assert_eq!(r.complete(id, args[0]), ULUA_OK);
    }
    assert_eq!(r.pump(0).0, ULUA_OK);
    assert_eq!(r.result(task).2.number, 1.0);
    assert_eq!(r.result(task).0, ULUA_INVALID_ARGUMENT);
    let result = r.result(failed);
    assert_eq!(result.0, ULUA_FAILED);
    assert_eq!(code(&result.1), "trap:uncaught-exception");
    assert_eq!(r.result(failed).0, ULUA_INVALID_ARGUMENT);
    let mut value = Value::default();
    let mut d = Diagnostic::default();
    // SAFETY: The fixture owns live instance and result storage.
    unsafe {
        assert_eq!(
            ulua_call(r.i, r.index("main"), ptr::null(), 0, &mut value, &mut d),
            ULUA_FAILED
        );
    }
    assert_eq!(code(&d), "async:pending");
}
#[test]
fn ulua_instance_requests_limit_bound_and_reentrant_successor() {
    let now = Instant::now();
    let source = "export function main():void{const a=host1(1,true,'s');const b=host1(2,true,'s');}export async function serial():Promise<void>{for(let i=0;i<10;i++){await host1(i,true,'s');}}";
    for cap in [0, 1, 2] {
        let limits = RunLimits {
            max_open_requests: cap,
            ..RunLimits::default()
        };
        let r = Run::new(source, &[1], limits);
        let result = r.start("main");
        assert_eq!(r.counts(), (cap, 0));
        if cap < 2 {
            assert_eq!(result.0, ULUA_FAILED);
            assert_eq!(code(&result.1), "limit:requests");
        } else {
            assert_eq!(result.0, ULUA_OK);
        }
    }
    let mut r = Run::new(
        source,
        &[1],
        RunLimits {
            max_open_requests: 1,
            ..RunLimits::default()
        },
    );
    let task = r.start("serial").2;
    for n in 0..10 {
        let id = r.data.requests.pop_front().unwrap().0;
        assert_eq!(r.complete(id, number(n as f64)), ULUA_OK);
        assert_eq!(r.pump(1).0, ULUA_OK);
    }
    assert_eq!(r.result(task).0, ULUA_OK);
    assert_eq!(r.counts(), (0, 0));
    eprintln!(
        "P4.3 request bounds=0/1/2 serial_settles=10 cost={:.3}s",
        now.elapsed().as_secs_f64()
    );
}
#[test]
fn revocation_preserves_open_requests_and_free_discards_requests_tasks() {
    let mut r = Run::new(
        "export async function main():Promise<number>{return await host1(1,true,'s');}",
        &[1],
        RunLimits::default(),
    );
    let task = r.start("main").2;
    let id = r.data.requests.pop_front().unwrap().0;
    // SAFETY: The fixture owns its live capability.
    unsafe {
        assert_eq!(ulua_capability_revoke(r.caps[0]), ULUA_OK);
    }
    assert_eq!(r.complete(id, number(3.0)), ULUA_OK);
    assert_eq!(r.pump(0).0, ULUA_OK);
    assert_eq!(r.result(task).2.number, 3.0);
    let result = r.start("main");
    assert_eq!(result.0, ULUA_HOST_ERROR);
    assert_eq!(code(&result.1), "host:revoked");
    assert!(r.data.requests.is_empty());
    let mut r = Run::new(
        "export async function main():Promise<number>{return await host1(1,true,'s');}",
        &[1],
        RunLimits::default(),
    );
    let task = r.start("main").2;
    let id = r.data.requests.pop_front().unwrap().0;
    assert_eq!(r.complete(id, number(1.0)), ULUA_OK);
    // SAFETY: The fixture frees the live instance; it never uses that pointer again.
    unsafe {
        assert_eq!(ulua_instance_free(r.i), ULUA_OK);
        r.i = ptr::null_mut();
    }
    let host = Host::default();
    // SAFETY: The module and context remain live; the new instance has separate id ownership.
    unsafe {
        assert_eq!(
            ulua_instance_new(r.c, r.m, &host, ptr::null(), &mut r.i, ptr::null_mut()),
            ULUA_OK
        );
    }
    assert_eq!(r.complete(id, number(1.0)), ULUA_INVALID_ARGUMENT);
    assert_eq!(r.result(task).0, ULUA_INVALID_ARGUMENT);
    assert_eq!(r.counts(), (0, 0));
}
#[test]
fn async_start_error_stops_and_no_later_callback() {
    let mut r = Run::new(
        "export async function main():Promise<void>{try{await host1(1,true,'s');}catch{print('catch');}finally{print('finally');}await host1(2,true,'s');}",
        &[1],
        RunLimits::default(),
    );
    r.data.fail = true;
    let result = r.start("main");
    assert_eq!(result.0, ULUA_HOST_ERROR);
    assert_eq!(code(&result.1), "host:capability");
    assert_eq!(r.data.requests.len(), 1);
    assert!(r.data.output.is_empty());
}
#[test]
fn new_public_functions_validate_nulls_without_outputs() {
    let r = Run::new("export function main():void{}", &[], RunLimits::default());
    let mut n = 42;
    let mut task = 42;
    let mut value = number(42.0);
    let mut diagnostic = Diagnostic {
        struct_size: 0,
        ..Diagnostic::default()
    };
    // SAFETY: Invalid null pointers need no storage; all non-null objects and outputs remain live.
    unsafe {
        assert_eq!(
            ulua_call_start(
                ptr::null_mut(),
                0,
                ptr::null(),
                0,
                &mut task,
                ptr::null_mut()
            ),
            ULUA_INVALID_ARGUMENT
        );
        assert_eq!(
            ulua_call_start(r.i, 0, ptr::null(), 0, ptr::null_mut(), ptr::null_mut()),
            ULUA_INVALID_ARGUMENT
        );
        assert_eq!(
            ulua_call_start(r.i, 0, ptr::null(), 0, &mut task, &mut diagnostic),
            ULUA_INVALID_ARGUMENT
        );
        assert_eq!(task, 42);
        assert_eq!(
            ulua_task_result(ptr::null_mut(), 0, &mut value, ptr::null_mut()),
            ULUA_INVALID_ARGUMENT
        );
        assert_eq!(
            ulua_task_result(r.i, 0, ptr::null_mut(), ptr::null_mut()),
            ULUA_INVALID_ARGUMENT
        );
        assert_eq!(
            ulua_request_complete(ptr::null_mut(), 0, &value),
            ULUA_INVALID_ARGUMENT
        );
        assert_eq!(
            ulua_request_complete(r.i, 0, ptr::null()),
            ULUA_INVALID_ARGUMENT
        );
        assert_eq!(ulua_request_fail(ptr::null_mut(), 0), ULUA_INVALID_ARGUMENT);
        assert_eq!(
            ulua_pump(ptr::null_mut(), 0, &mut n, ptr::null_mut()),
            ULUA_INVALID_ARGUMENT
        );
        assert_eq!(n, 42);
        assert_eq!(
            ulua_pump(r.i, 0, ptr::null_mut(), ptr::null_mut()),
            ULUA_INVALID_ARGUMENT
        );
        assert_eq!(
            ulua_instance_requests(ptr::null(), &mut n, &mut 0),
            ULUA_INVALID_ARGUMENT
        );
        assert_eq!(n, 42);
        assert_eq!(
            ulua_instance_requests(r.i, ptr::null_mut(), &mut n),
            ULUA_INVALID_ARGUMENT
        );
        assert_eq!(
            ulua_instance_requests(r.i, &mut n, ptr::null_mut()),
            ULUA_INVALID_ARGUMENT
        );
    }
}

#[test]
fn pump_steps_reset_per_settle_and_stop_keeps_records_and_clears_jobs() {
    #[link(name = "ulua_test_native", kind = "static")]
    // SAFETY: Test-only probes observe and adjust limits of this live VM.
    unsafe extern "C" {
        fn ulua_internal_test_host_steps(vm: *const c_void) -> u64;
        fn ulua_internal_test_restore_steps(vm: *mut c_void, steps: u64);
    }
    let now = Instant::now();
    let source = "function work(n:number):number{let sum=0;for(let i=0;i<1000;i++){sum+=i;}return n+sum;}export function main():void{for(let i=0;i<3;i++){const pending=host1(i,true,'s');const p=pending.then(work).then((n:number):void=>print(`${n}`));const other=pending.then((n:number):void=>{});}}export function good():number{return 42;}";
    let mut r = Run::new(source, &[1], RunLimits::default());
    assert_eq!(r.start("main").0, ULUA_OK);
    let ids: Vec<_> = r.data.requests.drain(..).map(|r| r.0).collect();
    for id in &ids {
        assert_eq!(r.complete(*id, number(1.0)), ULUA_OK);
    }
    assert_eq!(r.pump(1).0, ULUA_OK);
    // SAFETY: The test owns its live VM and measures the first successful settle.
    let steps = unsafe { ulua_internal_test_host_steps((*r.i).vm) };
    // Equal limits for each remaining settle succeed even though the sum exceeds this budget.
    unsafe {
        ulua_internal_test_restore_steps((*r.i).vm, steps + 32);
    }
    assert_eq!(r.pump(0).0, ULUA_OK);
    assert_eq!(r.data.output.len(), 3);
    let mut r = Run::new(source, &[1], RunLimits::default());
    let ready = r.start("good").2;
    assert_eq!(r.start("main").0, ULUA_OK);
    let ids: Vec<_> = r.data.requests.drain(..).map(|r| r.0).collect();
    for id in &ids {
        assert_eq!(r.complete(*id, number(1.0)), ULUA_OK);
    }
    // SAFETY: The fixture owns the VM. The low budget stops inside the first reaction.
    unsafe {
        ulua_internal_test_restore_steps((*r.i).vm, 100);
    }
    let result = r.pump(0);
    assert_eq!(result.0, ULUA_FAILED);
    assert_eq!(code(&result.1), "limit:steps");
    assert_eq!(result.2, 1);
    assert_eq!(r.counts(), (2, 2));
    assert!(r.data.output.is_empty());
    assert_eq!(r.result(ready).2.number, 42.0);
    // SAFETY: Restore the same live VM for the two later recorded completions.
    unsafe {
        ulua_internal_test_restore_steps((*r.i).vm, DEFAULT_STEPS);
    }
    assert_eq!(r.pump(0).0, ULUA_OK);
    assert_eq!(r.data.output.len(), 2);
    assert_eq!(r.counts(), (0, 0));
    let task = r.start("good").2;
    assert_eq!(r.result(task).2.number, 42.0);
    eprintln!(
        "P4.3 per-settle step_limit={steps}+32 settles=3 stop_records=2 cost={:.3}s",
        now.elapsed().as_secs_f64()
    );
}
#[cfg(feature = "measurements")]
#[test]
fn each_start_and_settle_charges_exactly_one_explicit_step() {
    #[link(name = "ulua_test_native", kind = "static")]
    // SAFETY: The test-only archive computes independent totals from this VM.
    unsafe extern "C" {
        fn ulua_internal_test_explicit_steps(vm: *const c_void) -> u64;
    }
    let mut r = Run::new(
        "export function main():void{const a=host1(1,true,'s');const b=host1(2,true,'s');const c=host1(3,true,'s');}",
        &[1],
        RunLimits::default(),
    );
    assert_eq!(r.start("main").0, ULUA_OK);
    // SAFETY: The fixture owns this live VM and the probe reads counters only.
    assert_eq!(unsafe { ulua_internal_test_explicit_steps((*r.i).vm) }, 3);
    for (id, args) in r.data.requests.drain(..).collect::<Vec<_>>() {
        assert_eq!(r.complete(id, args[0]), ULUA_OK);
        assert_eq!(r.pump(1).0, ULUA_OK);
        // SAFETY: The fixture keeps the VM live through this independent count query.
        assert_eq!(unsafe { ulua_internal_test_explicit_steps((*r.i).vm) }, 1);
    }
    eprintln!("P4.3 explicit charges starts=3 settles=1/1/1");
}

#[path = "p43_limits.rs"]
mod limits;
