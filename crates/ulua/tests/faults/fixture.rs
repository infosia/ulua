//! Owned ABI objects and host fixtures for allocation sweeps.
#![allow(clippy::field_reassign_with_default)]
use std::{
    collections::VecDeque,
    ffi::{CStr, c_char, c_void},
    ptr,
};
use ulua::{fault_injection as fault, *};

pub const SMALL: &str = "__fault_small";
pub const MARKER: &str = "__fault_handler";
pub const TRY_MARKER: &str = "__fault_try";

#[derive(Default)]
pub struct HostState {
    pub requests: VecDeque<(u64, Value)>,
    pub output: Vec<String>,
    buffer: String,
    pub handler_after_failure: bool,
    pub started_requests: u32,
}
unsafe extern "C" fn output(data: *mut c_void, bytes: *const c_char, length: usize) -> i32 {
    // SAFETY: The fixture owns the callback data; the shim supplies readable UTF-8 bytes.
    unsafe {
        let state = &mut *data.cast::<HostState>();
        let text = std::str::from_utf8(std::slice::from_raw_parts(bytes.cast(), length)).unwrap();
        if text == MARKER && fault::last().failures != 0 {
            state.handler_after_failure = true;
        }
        state.output.push(text.into());
    }
    0
}
unsafe extern "C" fn later(data: *mut c_void, args: *const Value, _: usize, id: u64) -> i32 {
    // SAFETY: The descriptor guarantees one argument and the fixture owns the callback data.
    unsafe {
        (*data.cast::<HostState>()).requests.push_back((id, *args));
        (*data.cast::<HostState>()).started_requests += 1;
    }
    0
}
unsafe extern "C" fn later_text(data: *mut c_void, _: *const Value, _: usize, id: u64) -> i32 {
    let mut value = Value::default();
    value.r#type = ULUA_TYPE_STRING;
    value.data = c"copied host completion: unique text".as_ptr();
    value.length = 35;
    // SAFETY: The descriptor guarantees that the fixture owns the live callback data.
    unsafe {
        (*data.cast::<HostState>()).requests.push_back((id, value));
        (*data.cast::<HostState>()).started_requests += 1;
    }
    0
}
unsafe extern "C" fn add(_: *mut c_void, args: *const Value, _: usize, out: *mut Value) -> i32 {
    // SAFETY: The descriptor supplies two numbers and writable result storage.
    unsafe {
        (*out).r#type = ULUA_TYPE_NUMBER;
        (*out).number = (*args).number + (*args.add(1)).number;
    }
    0
}
unsafe extern "C" fn damage(_: *mut c_void, args: *const Value, _: usize, out: *mut Value) -> i32 {
    // SAFETY: The descriptor supplies one number and writable result storage.
    unsafe {
        (*out).r#type = ULUA_TYPE_NUMBER;
        (*out).number = (*args).number * 2.0;
    }
    0
}
unsafe extern "C" fn not(_: *mut c_void, args: *const Value, _: usize, out: *mut Value) -> i32 {
    // SAFETY: The descriptor supplies one boolean and writable result storage.
    unsafe {
        (*out).r#type = ULUA_TYPE_BOOLEAN;
        (*out).boolean = u32::from((*args).boolean == 0);
    }
    0
}
unsafe extern "C" fn concat(
    data: *mut c_void,
    args: *const Value,
    _: usize,
    out: *mut Value,
) -> i32 {
    // SAFETY: The descriptor supplies two strings; the buffer outlives the result copy.
    unsafe {
        let state = &mut *data.cast::<HostState>();
        state.buffer.clear();
        for j in 0..2 {
            let value = &*args.add(j);
            if value.length != 0 {
                state.buffer.push_str(
                    std::str::from_utf8(std::slice::from_raw_parts(
                        value.data.cast(),
                        value.length,
                    ))
                    .unwrap(),
                );
            }
        }
        (*out).r#type = ULUA_TYPE_STRING;
        (*out).data = state.buffer.as_ptr().cast();
        (*out).length = state.buffer.len();
    }
    0
}
unsafe extern "C" fn log(data: *mut c_void, args: *const Value, _: usize, _: *mut Value) -> i32 {
    // SAFETY: The descriptor supplies one string and live callback data.
    unsafe { output(data, (*args).data, (*args).length) }
}

pub fn code(d: &Diagnostic) -> &str {
    // SAFETY: The ABI terminates its fixed diagnostic array.
    unsafe { CStr::from_ptr(d.code.as_ptr()).to_str().unwrap() }
}
pub fn assert_clean(s: fault::Snapshot) -> Result<(), String> {
    if s.live_bytes == 0 && s.host_bytes == 0 && s.records == 0 {
        Ok(())
    } else {
        Err(format!("native lifetime leak: {s:?}"))
    }
}

pub struct Fixture {
    pub context: *mut Context,
    pub module: *mut Module,
    pub instance: *mut Instance,
    pub host: Box<HostState>,
    pub exports: u32,
    pub small: u32,
    pub diagnostic: Diagnostic,
    pub task: u64,
    pub completion: Option<(u64, Value)>,
    pub runs: usize,
    settled_requests: u32,
}
impl Fixture {
    pub fn new(source: &str, names: &[&str]) -> Self {
        let mut f = Self {
            context: ptr::null_mut(),
            module: ptr::null_mut(),
            instance: ptr::null_mut(),
            host: Box::default(),
            exports: 0,
            small: 0,
            diagnostic: Diagnostic::default(),
            task: 0,
            completion: None,
            runs: 0,
            settled_requests: 0,
        };
        // SAFETY: This fixture owns and serializes every ABI object and callback buffer.
        unsafe {
            assert_eq!(ulua_context_new(&mut f.context), ULUA_OK);
            let mut grants = vec![];
            for &name in names {
                let (types, result, function, start): (
                    Vec<u32>,
                    u32,
                    Option<HostFn>,
                    Option<HostStartFn>,
                ) = match name {
                    "hostAdd" => (vec![1, 1], 1, Some(add), None),
                    "hostConcat" => (vec![3, 3], 3, Some(concat), None),
                    "hostNot" => (vec![2], 2, Some(not), None),
                    "hostLog" | "log" => (vec![3], 0, Some(log), None),
                    "damage" => (vec![1], 1, Some(damage), None),
                    "hostLater" | "fetchScore" => (vec![1], 1, None, Some(later)),
                    "hostText" => (vec![], 3, None, Some(later_text)),
                    _ => panic!("Unknown test grant {name}"),
                };
                let mut d = CapabilityDesc::default();
                d.name = name.as_ptr().cast();
                d.name_length = name.len();
                d.parameter_types = types.as_ptr();
                d.parameter_count = types.len();
                d.result_type = result;
                d.function = function;
                d.start = start;
                d.userdata = ptr::from_mut(&mut *f.host).cast();
                let mut cap = ptr::null_mut();
                assert_eq!(ulua_capability_new(f.context, &d, &mut cap), ULUA_OK);
                grants.push(cap);
            }
            let source =
                handlers(source) + "\nexport function __fault_small(): number { return 73; }\n";
            assert_eq!(
                ulua_compile(
                    f.context,
                    source.as_ptr().cast(),
                    source.len(),
                    grants.as_ptr(),
                    grants.len(),
                    ptr::null(),
                    &mut f.module,
                    &mut f.diagnostic
                ),
                ULUA_OK,
                "{}",
                code(&f.diagnostic)
            );
            assert_eq!(ulua_module_export_count(f.module, &mut f.exports), ULUA_OK);
            assert_eq!(
                ulua_module_find_export(f.module, SMALL.as_ptr().cast(), SMALL.len(), &mut f.small),
                ULUA_OK
            );
        }
        f
    }
    pub fn create(&mut self) -> Status {
        self.runs += 1;
        self.host.requests.clear();
        self.host.output.clear();
        self.host.handler_after_failure = false;
        self.host.started_requests = 0;
        self.settled_requests = 0;
        self.task = 0;
        self.completion = None;
        let mut host = Host::default();
        host.output = Some(output);
        host.userdata = ptr::from_mut(&mut *self.host).cast();
        let mut limits = RunLimits::default();
        limits.max_memory_bytes = 1024 * 1024;
        limits.max_steps = 100_000;
        self.diagnostic = Diagnostic::default();
        // SAFETY: All descriptors and outputs remain live; the fixture serializes this context.
        unsafe {
            ulua_instance_new(
                self.context,
                self.module,
                &host,
                &limits,
                &mut self.instance,
                &mut self.diagnostic,
            )
        }
    }
    pub fn call(&mut self, export: u32, start: bool) -> Status {
        self.runs += 1;
        let mut types = [0; 32];
        let mut count = 0;
        let mut result = 0;
        // SAFETY: The fixture owns the module and writable signature storage.
        unsafe {
            assert_eq!(
                ulua_module_export_signature(
                    self.module,
                    export,
                    types.as_mut_ptr(),
                    types.len(),
                    &mut count,
                    &mut result
                ),
                ULUA_OK
            );
        }
        let args: Vec<_> = types[..count]
            .iter()
            .map(|&t| {
                let mut v = Value::default();
                v.r#type = t;
                match t {
                    1 => v.number = 2.0,
                    2 => v.boolean = 1,
                    3 => {
                        v.data = c"input string".as_ptr();
                        v.length = 12;
                    }
                    _ => panic!("test argument type"),
                }
                v
            })
            .collect();
        let mut value = Value::default();
        self.diagnostic = Diagnostic::default();
        // SAFETY: The instance, arguments, and output storage remain live and serialized.
        unsafe {
            if start {
                ulua_call_start(
                    self.instance,
                    export,
                    args.as_ptr(),
                    args.len(),
                    &mut self.task,
                    &mut self.diagnostic,
                )
            } else {
                ulua_call(
                    self.instance,
                    export,
                    args.as_ptr(),
                    args.len(),
                    &mut value,
                    &mut self.diagnostic,
                )
            }
        }
    }
    pub fn complete(&mut self) -> Status {
        self.runs += 1;
        let (id, value) = self.host.requests.pop_front().unwrap();
        self.completion = Some((id, value));
        // SAFETY: The fixture owns a live instance and stable completion bytes.
        unsafe { ulua_request_complete(self.instance, id, &value) }
    }
    pub fn settle(&mut self) -> Status {
        self.runs += 1;
        let mut settled = 0;
        self.diagnostic = Diagnostic::default();
        // SAFETY: The fixture owns a serialized instance and writable outputs.
        let status = unsafe { ulua_pump(self.instance, 1, &mut settled, &mut self.diagnostic) };
        assert_eq!(settled, 1);
        self.settled_requests += settled;
        status
    }
    pub fn small(&mut self) {
        self.runs += 1;
        fault::arm(0);
        if self.instance.is_null() {
            assert_eq!(self.create(), ULUA_OK, "{}", code(&self.diagnostic));
        }
        let mut value = Value::default();
        let mut d = Diagnostic::default();
        // SAFETY: The fixture owns the instance and writable result storage.
        unsafe {
            assert_eq!(
                ulua_call(
                    self.instance,
                    self.small,
                    ptr::null(),
                    0,
                    &mut value,
                    &mut d
                ),
                ULUA_OK,
                "{}",
                code(&d)
            );
        }
        assert_eq!(value.number, 73.0);
    }
    pub fn small_in_context(&mut self) {
        // A module that always traps during initialization cannot create an instance on retry.
        // Compile a small module in the same context to check that failed construction leaves it usable.
        let source = "export function __fault_small(): number { return 73; }";
        let original = self.module;
        let mut module = ptr::null_mut();
        let mut d = Diagnostic::default();
        // SAFETY: The fixture owns the context, source bytes, and output storage.
        unsafe {
            assert_eq!(
                ulua_compile(
                    self.context,
                    source.as_ptr().cast(),
                    source.len(),
                    ptr::null(),
                    0,
                    ptr::null(),
                    &mut module,
                    &mut d
                ),
                ULUA_OK
            );
        }
        self.module = module;
        let original_small = self.small;
        self.small = 0;
        self.small();
        assert_clean(self.free()).unwrap();
        self.small = original_small;
        self.module = original;
        // SAFETY: The small module has no remaining instances and belongs to this context.
        unsafe {
            assert_eq!(ulua_module_free(module), ULUA_OK);
        }
    }
    pub fn free(&mut self) -> fault::Snapshot {
        // SAFETY: The fixture releases its instance once and invalidates its pointer.
        unsafe {
            if !self.instance.is_null() {
                let mut open = 0;
                let mut recorded = 0;
                assert_eq!(
                    ulua_instance_requests(self.instance, &mut open, &mut recorded),
                    ULUA_OK
                );
                assert_eq!(open, self.host.started_requests - self.settled_requests);
            }
            assert_eq!(ulua_instance_free(self.instance), ULUA_OK);
        }
        self.instance = ptr::null_mut();
        fault::last()
    }
    pub fn reset(&mut self) {
        if !self.instance.is_null() {
            assert_clean(self.free()).unwrap();
        }
        fault::arm(0);
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        // SAFETY: Context destruction owns and releases all remaining fixture objects.
        unsafe {
            assert_eq!(ulua_context_free(self.context), ULUA_OK);
        }
    }
}
fn handlers(source: &str) -> String {
    // Mark catch bodies with or without a binding, and finally bodies. Each marker uses a loaded string constant.
    let mut result = source.to_owned();
    let mut insertions = vec![];
    for (start, _) in source
        .match_indices("catch (")
        .chain(source.match_indices("catch {"))
        .chain(source.match_indices("finally {"))
    {
        let brace = source[start..].find('{').unwrap() + start + 1;
        insertions.push(brace);
    }
    insertions.sort_unstable();
    for offset in insertions.into_iter().rev() {
        result.insert_str(offset, " print(\"__fault_handler\"); ");
    }
    result
}
