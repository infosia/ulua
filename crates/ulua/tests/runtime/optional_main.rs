//! D27 modules use the public ABI without a required main export.
//! These tests cover module setup, export metadata, sync calls, and async tasks.
// External ABI descriptors are non-exhaustive and require field assignments.
#![allow(clippy::field_reassign_with_default)]
use std::{ffi::c_void, ptr};
use ulua::*;

#[derive(Default)]
struct Capture {
    bytes: Vec<u8>,
}

fn diagnostic_text(diagnostic: &Diagnostic) -> String {
    // SAFETY: The ABI terminates both fixed diagnostic arrays with a NUL byte.
    unsafe {
        format!(
            "{}: {}",
            std::ffi::CStr::from_ptr(diagnostic.code.as_ptr()).to_string_lossy(),
            std::ffi::CStr::from_ptr(diagnostic.message.as_ptr()).to_string_lossy()
        )
    }
}

unsafe extern "C" fn output(
    userdata: *mut c_void,
    text: *const std::ffi::c_char,
    length: usize,
) -> i32 {
    // SAFETY: The fixture owns this byte buffer. The ABI supplies valid bytes during the callback.
    unsafe {
        let capture = &mut *userdata.cast::<Capture>();
        capture
            .bytes
            .extend_from_slice(std::slice::from_raw_parts(text.cast::<u8>(), length));
        capture.bytes.push(b'\n');
    }
    0
}

struct Fixture {
    context: *mut Context,
    module: *mut Module,
    instance: *mut Instance,
    capture: Box<Capture>,
}

impl Fixture {
    fn new(source: &str, names: &[&str]) -> Self {
        let mut fixture = Self {
            context: ptr::null_mut(),
            module: ptr::null_mut(),
            instance: ptr::null_mut(),
            capture: Box::default(),
        };
        let mut diagnostic = Diagnostic::default();
        let mut host = Host::default();
        // The box keeps the callback state at one address when the fixture moves.
        host.userdata = (&mut *fixture.capture as *mut Capture).cast();
        host.output = Some(output);
        // SAFETY: All input and output storage is live. The fixture serializes its context and children.
        unsafe {
            assert_eq!(ulua_context_new(&mut fixture.context), ULUA_OK);
            assert_eq!(
                ulua_compile(
                    fixture.context,
                    source.as_ptr().cast(),
                    source.len(),
                    ptr::null(),
                    0,
                    ptr::null(),
                    &mut fixture.module,
                    &mut diagnostic
                ),
                ULUA_OK,
                "{}",
                diagnostic_text(&diagnostic)
            );
            let mut count = 0;
            assert_eq!(
                ulua_module_export_count(fixture.module, &mut count),
                ULUA_OK
            );
            assert_eq!(count as usize, names.len());
            for (index, expected) in names.iter().enumerate() {
                let mut name = ptr::null();
                let mut length = 0;
                assert_eq!(
                    ulua_module_export_name(fixture.module, index as u32, &mut name, &mut length),
                    ULUA_OK
                );
                assert_eq!(
                    std::slice::from_raw_parts(name.cast::<u8>(), length),
                    expected.as_bytes()
                );
            }
            assert_eq!(
                ulua_instance_new(
                    fixture.context,
                    fixture.module,
                    &host,
                    ptr::null(),
                    &mut fixture.instance,
                    &mut diagnostic
                ),
                ULUA_OK,
                "{}",
                diagnostic_text(&diagnostic)
            );
        }
        fixture
    }

    fn index(&self, name: &str) -> u32 {
        let mut index = u32::MAX;
        // SAFETY: The fixture owns the live module and the name covers its byte length.
        unsafe {
            assert_eq!(
                ulua_module_find_export(self.module, name.as_ptr().cast(), name.len(), &mut index),
                ULUA_OK
            );
        }
        index
    }

    fn call(&self, name: &str, arguments: &[Value]) -> Value {
        let mut result = Value::default();
        let mut diagnostic = Diagnostic::default();
        // SAFETY: The fixture owns the instance. The arguments and output cover their ABI lengths.
        unsafe {
            assert_eq!(
                ulua_call(
                    self.instance,
                    self.index(name),
                    arguments.as_ptr(),
                    arguments.len(),
                    &mut result,
                    &mut diagnostic
                ),
                ULUA_OK,
                "{}",
                diagnostic_text(&diagnostic)
            );
        }
        result
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        // SAFETY: The fixture owns the context. It frees all children before the callback buffer drops.
        unsafe {
            assert_eq!(ulua_context_free(self.context), ULUA_OK);
        }
    }
}

#[test]
fn no_exports_still_run_module_initializers() {
    let empty = Fixture::new("// A module without exports.\n", &[]);
    assert!(empty.capture.bytes.is_empty());
    let fixture = Fixture::new(
        "function setup():number{print('module ready');return 7;}const value:number=setup();",
        &[],
    );
    assert_eq!(&fixture.capture.bytes, b"module ready\n");
    let mut index = 0;
    // SAFETY: The fixture owns the live module and writable index storage.
    unsafe {
        assert_eq!(
            ulua_module_find_export(fixture.module, b"main".as_ptr().cast(), 4, &mut index),
            ULUA_INVALID_ARGUMENT
        );
    }
    assert_eq!(index, u32::MAX);
}

#[test]
fn other_exports_run_sync_calls_and_async_tasks_without_main() {
    let fixture = Fixture::new(
        "export function add(a:number,b:number):number{return a+b;}\
         export async function answer():Promise<number>{return 42;}",
        &["add", "answer"],
    );
    let mut argument = Value::default();
    argument.r#type = ULUA_TYPE_NUMBER;
    argument.number = 4.0;
    let result = fixture.call("add", &[argument, argument]);
    assert_eq!((result.r#type, result.number), (ULUA_TYPE_NUMBER, 8.0));
    let mut task = 0;
    let mut result = Value::default();
    let mut diagnostic = Diagnostic::default();
    let mut settled = 0;
    // SAFETY: The fixture serializes the instance. All task and result storage is writable and live.
    unsafe {
        assert_eq!(
            ulua_call_start(
                fixture.instance,
                fixture.index("answer"),
                ptr::null(),
                0,
                &mut task,
                &mut diagnostic
            ),
            ULUA_OK
        );
        assert_eq!(
            ulua_pump(fixture.instance, 0, &mut settled, &mut diagnostic),
            ULUA_OK
        );
        assert_eq!(
            ulua_task_result(fixture.instance, task, &mut result, &mut diagnostic),
            ULUA_OK
        );
    }
    assert_eq!(settled, 0);
    assert_eq!((result.r#type, result.number), (ULUA_TYPE_NUMBER, 42.0));
}

#[test]
fn main_and_other_exports_share_instance_state() {
    let fixture = Fixture::new(
        "let count:number=0;export function main():void{count++;}\
         export function current():number{return count;}",
        &["main", "current"],
    );
    assert_eq!(fixture.call("current", &[]).number, 0.0);
    assert_eq!(fixture.call("main", &[]).r#type, ULUA_TYPE_VOID);
    assert_eq!(fixture.call("current", &[]).number, 1.0);
}

#[test]
fn main_has_the_same_signature_rules_as_other_exports() {
    let fixture = Fixture::new(
        "export function main(n:number):number{return n+1;}",
        &["main"],
    );
    let mut argument = Value::default();
    argument.r#type = ULUA_TYPE_NUMBER;
    argument.number = 9.0;
    assert_eq!(fixture.call("main", &[argument]).number, 10.0);
}
