//! Direct ABI tests and an independent C header layout check.
use super::*;
use std::{
    ffi::{c_char, c_void},
    mem::offset_of,
    ptr,
};
const HELLO: &[u8] = b"export function main(): void { print(\"hello\"); }";

struct Fixture {
    context: *mut Context,
    module: *mut Module,
}
impl Fixture {
    fn new() -> Self {
        Self::source(HELLO)
    }
    fn source(source: &[u8]) -> Self {
        let mut context = ptr::null_mut();
        let mut module = ptr::null_mut();
        // SAFETY: Test-local output storage and source bytes satisfy the ABI preconditions.
        unsafe {
            assert_eq!(ulua_context_new(&mut context), ULUA_OK);
            assert_eq!(
                ulua_compile(
                    context,
                    source.as_ptr().cast(),
                    source.len(),
                    ptr::null(),
                    &mut module,
                    ptr::null_mut()
                ),
                ULUA_OK
            );
        }
        Self { context, module }
    }
    fn instance(&self, host: &Host) -> *mut Instance {
        let mut instance = ptr::null_mut();
        // SAFETY: The fixture owns live objects and the host descriptor covers its size.
        unsafe {
            assert_eq!(
                ulua_instance_new(
                    self.context,
                    self.module,
                    host,
                    ptr::null(),
                    &mut instance,
                    ptr::null_mut()
                ),
                ULUA_OK
            );
        }
        instance
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        // SAFETY: The fixture owns the live context and frees all remaining children once.
        unsafe {
            assert_eq!(ulua_context_free(self.context), ULUA_OK);
        }
    }
}
fn code(d: &Diagnostic) -> String {
    d.code
        .iter()
        .take_while(|c| **c != 0)
        .map(|c| char::from(*c as u8))
        .collect()
}

#[test]
fn abi_version() {
    assert_eq!(ulua_abi_version(), 1);
}
#[test]
fn default_compile_limits() {
    let mut limits = CompileLimits {
        max_source_bytes: 0,
        ..CompileLimits::default()
    };
    // SAFETY: Test descriptors and outputs satisfy the ABI preconditions.
    unsafe {
        assert_eq!(ulua_default_compile_limits(&mut limits), ULUA_OK);
        assert!(limits.max_source_bytes > 0);
        limits.struct_size = 0;
        assert_eq!(
            ulua_default_compile_limits(&mut limits),
            ULUA_INVALID_ARGUMENT
        );
        assert_eq!(
            ulua_default_compile_limits(ptr::null_mut()),
            ULUA_INVALID_ARGUMENT
        );
    }
}
#[test]
fn default_run_limits() {
    let mut limits = RunLimits {
        max_steps: 0,
        ..RunLimits::default()
    };
    // SAFETY: Test descriptors and outputs satisfy the ABI preconditions.
    unsafe {
        assert_eq!(ulua_default_run_limits(&mut limits), ULUA_OK);
        assert!(limits.max_steps > 0);
        limits.struct_size = 0;
        assert_eq!(ulua_default_run_limits(&mut limits), ULUA_INVALID_ARGUMENT);
        assert_eq!(
            ulua_default_run_limits(ptr::null_mut()),
            ULUA_INVALID_ARGUMENT
        );
    }
}
#[test]
fn context_new_and_free() {
    let mut context = ptr::null_mut();
    // SAFETY: The test supplies pointer storage and frees the context once.
    unsafe {
        assert_eq!(ulua_context_new(ptr::null_mut()), ULUA_INVALID_ARGUMENT);
        assert_eq!(ulua_context_new(&mut context), ULUA_OK);
        assert!(!context.is_null());
        assert_eq!(ulua_context_free(context), ULUA_OK);
        assert_eq!(ulua_context_free(ptr::null_mut()), ULUA_OK);
    }
}
#[test]
fn compile_rejects_and_reports_locations() {
    let fixture = Fixture::new();
    let mut d = Diagnostic::default();
    let mut module = fixture.module;
    // SAFETY: The fixture objects, source buffers, and outputs are live.
    unsafe {
        let invalid = b"\n\xff";
        assert_eq!(
            ulua_compile(
                fixture.context,
                invalid.as_ptr().cast(),
                invalid.len(),
                ptr::null(),
                &mut module,
                &mut d
            ),
            ULUA_REJECTED
        );
        assert!(module.is_null());
        assert_eq!(code(&d), "syntax");
        assert_eq!((d.offset, d.line, d.column), (1, 2, 1));
        let mut limits = CompileLimits {
            max_source_bytes: 0,
            ..CompileLimits::default()
        };
        assert_eq!(
            ulua_compile(
                fixture.context,
                HELLO.as_ptr().cast(),
                HELLO.len(),
                &limits,
                &mut module,
                &mut d
            ),
            ULUA_REJECTED
        );
        assert_eq!(code(&d), "limit:source-bytes");
        limits = CompileLimits::default();
        limits.max_nesting_depth = 0;
        assert_eq!(
            ulua_compile(
                fixture.context,
                HELLO.as_ptr().cast(),
                HELLO.len(),
                &limits,
                &mut module,
                &mut d
            ),
            ULUA_REJECTED
        );
        assert_eq!(code(&d), "limit:nesting-depth");
        module = fixture.module;
        assert_eq!(
            ulua_compile(
                fixture.context,
                ptr::null(),
                1,
                ptr::null(),
                &mut module,
                &mut d
            ),
            ULUA_INVALID_ARGUMENT
        );
        assert_eq!(module, fixture.module);
        limits.struct_size = 0;
        assert_eq!(
            ulua_compile(
                fixture.context,
                HELLO.as_ptr().cast(),
                HELLO.len(),
                &limits,
                &mut module,
                &mut d
            ),
            ULUA_INVALID_ARGUMENT
        );
        assert_eq!(module, fixture.module);
    }
}
#[test]
fn module_export_count() {
    let f = Fixture::new();
    let mut count = 42;
    // SAFETY: The fixture owns a live module and the output is writable.
    unsafe {
        assert_eq!(ulua_module_export_count(f.module, &mut count), ULUA_OK);
        assert_eq!(count, 1);
        assert_eq!(
            ulua_module_export_count(f.module, ptr::null_mut()),
            ULUA_INVALID_ARGUMENT
        );
    }
}
#[test]
fn module_export_name() {
    let f = Fixture::new();
    let mut name = ptr::null();
    let mut length = 0;
    // SAFETY: The fixture owns a live module; name bytes remain valid through this test.
    unsafe {
        assert_eq!(
            ulua_module_export_name(f.module, 0, &mut name, &mut length),
            ULUA_OK
        );
        assert_eq!(
            std::slice::from_raw_parts(name.cast::<u8>(), length),
            b"main"
        );
        assert_eq!(
            ulua_module_export_name(f.module, 1, &mut name, &mut length),
            ULUA_INVALID_ARGUMENT
        );
        assert_eq!(length, 4);
    }
}
#[test]
fn module_export_signature() {
    let f = Fixture::new();
    let mut count = 42;
    let mut result = 42;
    // SAFETY: The fixture owns a live module; all outputs cover their capacities.
    unsafe {
        assert_eq!(
            ulua_module_export_signature(f.module, 0, ptr::null_mut(), 0, &mut count, &mut result),
            ULUA_OK
        );
        assert_eq!((count, result), (0, ULUA_TYPE_VOID));
        // Construct a future typed signature independently to test the ABI capacity rule.
        (&mut (*f.module).exports)[0].parameters = vec![ULUA_TYPE_NUMBER, ULUA_TYPE_BOOLEAN];
        let mut parameter = 99;
        assert_eq!(
            ulua_module_export_signature(f.module, 0, &mut parameter, 1, &mut count, &mut result),
            ULUA_INVALID_ARGUMENT
        );
        assert_eq!((count, result, parameter), (2, ULUA_TYPE_VOID, 99));
        let mut parameters = [99; 2];
        assert_eq!(
            ulua_module_export_signature(
                f.module,
                0,
                parameters.as_mut_ptr(),
                2,
                &mut count,
                &mut result
            ),
            ULUA_OK
        );
        assert_eq!(parameters, [ULUA_TYPE_NUMBER, ULUA_TYPE_BOOLEAN]);
    }
}
#[test]
fn module_find_export() {
    let f = Fixture::new();
    let mut index = 42;
    // SAFETY: The fixture owns a live module; name bytes and output storage are valid.
    unsafe {
        assert_eq!(
            ulua_module_find_export(f.module, c"main".as_ptr(), 4, &mut index),
            ULUA_OK
        );
        assert_eq!(index, 0);
        assert_eq!(
            ulua_module_find_export(f.module, c"other".as_ptr(), 5, &mut index),
            ULUA_INVALID_ARGUMENT
        );
        assert_eq!(index, u32::MAX);
        index = 42;
        assert_eq!(
            ulua_module_find_export(f.module, ptr::null(), 1, &mut index),
            ULUA_INVALID_ARGUMENT
        );
        assert_eq!(index, 42);
    }
}
#[test]
fn instance_new_limits_and_context_ownership() {
    let f = Fixture::new();
    let other = Fixture::new();
    let host = Host::default();
    let mut instance = ptr::null_mut();
    let mut d = Diagnostic::default();
    let mut limits = RunLimits {
        max_memory_bytes: 0,
        ..RunLimits::default()
    };
    // SAFETY: Both fixtures and all test descriptors and outputs remain live.
    unsafe {
        assert_eq!(
            ulua_instance_new(
                f.context,
                other.module,
                &host,
                ptr::null(),
                &mut instance,
                &mut d
            ),
            ULUA_INVALID_ARGUMENT
        );
        assert_eq!(
            ulua_instance_new(f.context, f.module, &host, &limits, &mut instance, &mut d),
            ULUA_FAILED
        );
        assert!(instance.is_null());
        assert_eq!(code(&d), "limit:memory");
        assert_eq!(d.offset, u64::MAX);
        limits = RunLimits::default();
        limits.max_steps = 0;
        let status = ulua_instance_new(f.context, f.module, &host, &limits, &mut instance, &mut d);
        // A zero interrupt budget fails at the first interrupt, not before VM setup.
        if status == ULUA_OK {
            let mut result = Value::default();
            assert_eq!(
                ulua_call(instance, 0, ptr::null(), 0, &mut result, &mut d),
                ULUA_FAILED
            );
        } else {
            assert_eq!(status, ULUA_FAILED);
        }
        assert_eq!(code(&d), "limit:steps");
    }
}
#[test]
fn instance_free() {
    let f = Fixture::new();
    let instance = f.instance(&Host::default());
    // SAFETY: The fixture owns a live instance; explicit free removes it from the context.
    unsafe {
        assert_eq!(ulua_instance_free(instance), ULUA_OK);
        assert_eq!(ulua_instance_free(ptr::null_mut()), ULUA_OK);
    }
}
#[test]
fn module_free_preserves_instances() {
    let f = Fixture::new();
    let instance = f.instance(&Host::default());
    let mut result = Value::default();
    // SAFETY: The instance owns its VM and export signatures independently of the module.
    unsafe {
        assert_eq!(ulua_module_free(f.module), ULUA_OK);
        assert_eq!(ulua_module_free(ptr::null_mut()), ULUA_OK);
        assert_eq!(
            ulua_call(instance, 0, ptr::null(), 0, &mut result, ptr::null_mut()),
            ULUA_OK
        );
        assert_eq!(result.r#type, ULUA_TYPE_VOID);
    }
}
struct Output {
    lines: Vec<String>,
    context: *mut Context,
    module: *mut Module,
    fail: bool,
    busy: bool,
}
unsafe extern "C" fn collect(data: *mut c_void, text: *const c_char, length: usize) -> i32 {
    // SAFETY: The test passes a live Output and the shim provides readable text during this callback.
    unsafe {
        let out = &mut *data.cast::<Output>();
        out.lines.push(
            String::from_utf8_lossy(std::slice::from_raw_parts(text.cast::<u8>(), length))
                .into_owned(),
        );
        let mut count = 42;
        out.busy = ulua_module_export_count(out.module, &mut count) == ULUA_BUSY
            && count == 42
            && ulua_context_free(out.context) == ULUA_BUSY;
        i32::from(out.fail)
    }
}
#[test]
fn call_output_host_failure_and_reentry() {
    let f = Fixture::new();
    let mut output = Output {
        lines: vec![],
        context: f.context,
        module: f.module,
        fail: true,
        busy: false,
    };
    let host = Host {
        output: Some(collect),
        userdata: ptr::from_mut(&mut output).cast(),
        ..Host::default()
    };
    let instance = f.instance(&host);
    let mut result = Value::default();
    let mut d = Diagnostic::default();
    // SAFETY: The fixture, callback state, and argument and result buffers remain live.
    unsafe {
        assert_eq!(
            ulua_call(instance, 0, ptr::null(), 0, &mut result, &mut d),
            ULUA_HOST_ERROR
        );
        assert_eq!(output.lines, ["hello"]);
        assert!(output.busy);
        assert_eq!(d.offset, u64::MAX);
        output.fail = false;
        assert_eq!(
            ulua_call(instance, 0, ptr::null(), 0, &mut result, &mut d),
            ULUA_OK
        );
        assert_eq!(output.lines, ["hello", "hello"]);
        assert_eq!(result.r#type, ULUA_TYPE_VOID);
        let value = Value::default();
        assert_eq!(
            ulua_call(instance, 0, &value, 1, &mut result, &mut d),
            ULUA_INVALID_ARGUMENT
        );
        assert_eq!(
            ulua_call(instance, 1, ptr::null(), 0, &mut result, &mut d),
            ULUA_INVALID_ARGUMENT
        );
        assert_eq!(output.lines.len(), 2);
        // A malformed typed argument must not call the VM.
        (&mut (*instance).exports)[0].parameters = vec![ULUA_TYPE_NUMBER];
        let value = Value {
            r#type: ULUA_TYPE_NUMBER,
            boolean: 1,
            ..Value::default()
        };
        assert_eq!(
            ulua_call(instance, 0, &value, 1, &mut result, &mut d),
            ULUA_INVALID_ARGUMENT
        );
        assert_eq!(output.lines.len(), 2);
    }
}
#[test]
fn internal_poison_and_free() {
    let f = Fixture::new();
    let mut d = Diagnostic::default();
    // SAFETY: The test injects a caught internal status into a live context.
    unsafe {
        assert_eq!(
            objects::guard(f.context, &mut d, || ULUA_INTERNAL),
            ULUA_INTERNAL
        );
        let mut count = 42;
        assert_eq!(
            ulua_module_export_count(f.module, &mut count),
            ULUA_POISONED
        );
        assert_eq!(count, 42);
        assert_eq!(ulua_module_free(f.module), ULUA_OK);
    }
}
#[test]
fn header_layout() {
    // SAFETY: The linked C probe has this signature and returns only static layout values.
    unsafe extern "C" {
        fn ulua_layout_probe(index: usize) -> usize;
    }
    let expected = [
        size_of::<Diagnostic>(),
        offset_of!(Diagnostic, struct_size),
        offset_of!(Diagnostic, reserved),
        offset_of!(Diagnostic, code),
        offset_of!(Diagnostic, message),
        offset_of!(Diagnostic, offset),
        offset_of!(Diagnostic, line),
        offset_of!(Diagnostic, column),
        size_of::<Value>(),
        offset_of!(Value, r#type),
        offset_of!(Value, boolean),
        offset_of!(Value, number),
        offset_of!(Value, data),
        offset_of!(Value, length),
        size_of::<CompileLimits>(),
        offset_of!(CompileLimits, struct_size),
        offset_of!(CompileLimits, reserved),
        offset_of!(CompileLimits, max_source_bytes),
        offset_of!(CompileLimits, max_nesting_depth),
        offset_of!(CompileLimits, reserved2),
        size_of::<RunLimits>(),
        offset_of!(RunLimits, struct_size),
        offset_of!(RunLimits, reserved),
        offset_of!(RunLimits, max_memory_bytes),
        offset_of!(RunLimits, max_steps),
        size_of::<Host>(),
        offset_of!(Host, struct_size),
        offset_of!(Host, reserved),
        offset_of!(Host, userdata),
        offset_of!(Host, output),
        ULUA_OK as usize,
        ULUA_INVALID_ARGUMENT as usize,
        ULUA_REJECTED as usize,
        ULUA_FAILED as usize,
        ULUA_HOST_ERROR as usize,
        ULUA_BUSY as usize,
        ULUA_INTERNAL as usize,
        ULUA_POISONED as usize,
        ULUA_TYPE_VOID as usize,
        ULUA_TYPE_NUMBER as usize,
        ULUA_TYPE_BOOLEAN as usize,
        ULUA_TYPE_STRING as usize,
        ULUA_MIN_RUN_STACK,
    ];
    for (index, expected) in expected.into_iter().enumerate() {
        // SAFETY: The probe accepts every index, and the C compiler derives these values from the public header.
        assert_eq!(
            unsafe { ulua_layout_probe(index) },
            expected,
            "layout index {index}"
        );
    }
}

fn p1_output(source: &str) -> Vec<String> {
    let f = Fixture::source(source.as_bytes());
    let mut output = Output {
        lines: vec![],
        context: f.context,
        module: f.module,
        fail: false,
        busy: false,
    };
    let host = Host {
        output: Some(collect),
        userdata: ptr::from_mut(&mut output).cast(),
        ..Host::default()
    };
    let instance = f.instance(&host);
    let mut main = 0;
    let mut result = Value::default();
    let mut diagnostic = Diagnostic::default();
    // SAFETY: The fixture and callback buffers remain live until the synchronous call returns.
    unsafe {
        assert_eq!(
            ulua_module_find_export(f.module, c"main".as_ptr(), 4, &mut main),
            ULUA_OK
        );
        assert_eq!(
            ulua_call(instance, main, ptr::null(), 0, &mut result, &mut diagnostic),
            ULUA_OK,
            "{}",
            code(&diagnostic)
        );
    }
    output.lines
}

#[test]
fn p1_evaluation_order_defaults_short_circuit_and_compound_assignment() {
    let source = r#"
        let n = 0;
        function inc(): number { n += 1; return n; }
        function pair(a: number, b: number): number { return a * 10 + b; }
        function flag(): boolean { n += 1; return true; }
        function defaults(a: number = inc(), b: number = a + inc()): number { return a * 10 + b; }
        export function main(): void {
            n = 1; print(`${n + inc()} ${n}`);
            n = 1; print(`${pair(n, inc())} ${n}`);
            n = 1; n += inc(); print(`${n}`);
            n = 1; n %= inc(); print(`${n}`);
            n = 0; print(`${false && flag()} ${true || flag()} ${n}`);
            n = 0; print(`${true ? inc() : inc()} ${false ? inc() : inc()} ${n}`);
            n = 0; print(`${defaults()} ${n}`);
            n = 0; print(`${defaults(10, 20)} ${n}`);
            n = 0; print(`${n}${inc()}${n}${inc()}${n}`);
        }
    "#;
    assert_eq!(
        p1_output(source),
        [
            "3 2",
            "12 2",
            "3",
            "1",
            "false true 0",
            "1 2 2",
            "13 2",
            "120 0",
            "01122"
        ]
    );
}

#[test]
fn p1_loop_copy_back_continue_break_nested_loops_and_parameters() {
    let source = r#"
        function f(n: number): number { n++; n -= 1; return n; }
        export function main(): void {
            let total = 0;
            for (let i = 0; i < 6; ++i) {
                i += 1;
                if (i === 1) { continue; }
                if (i === 5) { break; }
                total += i;
            }
            print(`${total}`);
            let sum = 0;
            for (let i = 0, j = 10; i < 3; i++) {
                j += 2;
                for (let k = 0; k < 3; k++) {
                    if (k === 1) { continue; }
                    sum += i + j;
                }
            }
            print(`${sum}`);
            let count = 0;
            while (count < 3) { count++; if (count === 2) { continue; } print(`${count}`); }
            for (;;) { break; }
            print(`${f(10)}`);
        }
    "#;
    assert_eq!(p1_output(source), ["3", "90", "1", "3", "10"]);
}

#[test]
fn p1_spills_remain_typed_and_reachable_across_nested_scopes() {
    let mut source = String::from("function id(n: number): number { return n; }");
    for i in 0..220 {
        source.push_str(&format!("let m{i} = id({i});"));
    }
    source.push_str("function read(): number { return m219; } export function main(): void {");
    for i in 0..180 {
        source.push_str(&format!("const x{i} = id({i});"));
    }
    for i in 0..30 {
        source.push_str(&format!(
            "{{ let n{i} = 0; for (let j{i} = 0; j{i} < 3; j{i}++) {{ n{i} += j{i}; }} }}"
        ));
    }
    source.push_str("m219 += 1; print(`${x179} ${read()}`); }");
    assert_eq!(p1_output(&source), ["179 220"]);
}

#[test]
fn string_exports_reject_before_host_calls() {
    let f = Fixture::new();
    for source in [
        "export function g():string{return \"x\";} export function main():void {}",
        "export function f(s:string):void{print(s);} export function main():void {}",
    ] {
        let mut module = ptr::null_mut();
        let mut d = Diagnostic::default();
        // SAFETY: The fixture owns the context and all source and output storage covers its size.
        unsafe {
            assert_eq!(
                ulua_compile(
                    f.context,
                    source.as_ptr().cast(),
                    source.len(),
                    ptr::null(),
                    &mut module,
                    &mut d
                ),
                ULUA_REJECTED
            );
        }
        assert_eq!(code(&d), "unsupported");
        assert!(module.is_null());
    }
}

#[test]
fn refused_compiler_thread_keeps_context_usable() {
    let f = Fixture::new();
    let mut module = ptr::null_mut();
    let mut d = Diagnostic::default();
    crate::abi::REFUSE_COMPILER_THREAD.with(|flag| flag.set(true));
    // SAFETY: The fixture owns the context and all source and output storage covers its size.
    unsafe {
        assert_eq!(
            ulua_compile(
                f.context,
                HELLO.as_ptr().cast(),
                HELLO.len(),
                ptr::null(),
                &mut module,
                &mut d
            ),
            ULUA_FAILED
        );
        assert!(module.is_null());
        assert_eq!(code(&d), "host:resources");
        assert_eq!(
            ulua_compile(
                f.context,
                HELLO.as_ptr().cast(),
                HELLO.len(),
                ptr::null(),
                &mut module,
                &mut d
            ),
            ULUA_OK
        );
    }
}

#[test]
fn templates_freeze_each_read_and_conversion_in_order() {
    for count in [7, 8, 256] {
        let source = format!(
            "let n=1;function inc():number {{n++;return n;}} export function main():void {{print(`{}${{n}}${{inc()}}${{n}}`);}}",
            "x".repeat(count)
                .chars()
                .map(|_| "${n}")
                .collect::<String>()
        );
        let expected = format!("{}122", "1".repeat(count));
        assert_eq!(p1_output(&source), [expected]);
    }
}
