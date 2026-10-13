//! Fixed corpus capabilities mirror the Node witness fixtures.
// External ABI descriptors are non-exhaustive and require field assignments.
#![allow(clippy::field_reassign_with_default)]

use std::{collections::VecDeque, ffi::c_void, ptr, slice};
use ulua::*;

pub struct Fixtures {
    pub output: Vec<String>,
    buffer: String,
    requests: VecDeque<(u64, f64)>,
}
impl Fixtures {
    pub fn new() -> Self {
        Self {
            output: vec![],
            buffer: String::new(),
            requests: VecDeque::new(),
        }
    }
    pub unsafe fn settle(
        &mut self,
        instance: *mut Instance,
        task: &mut Option<u64>,
        diagnostic: &mut Diagnostic,
    ) -> Status {
        // SAFETY: The harness serializes its live instance and keeps fixture values live.
        unsafe {
            while let Some((request, n)) = self.requests.pop_front() {
                let mut value = Value::default();
                value.r#type = ULUA_TYPE_NUMBER;
                value.number = n;
                let status = ulua_request_complete(instance, request, &value);
                if status != ULUA_OK {
                    return status;
                }
                let mut count = 0;
                let status = ulua_pump(instance, 1, &mut count, diagnostic);
                if status != ULUA_OK {
                    // C19.7 gives the main rejection precedence over an unhandled rejection.
                    if status == ULUA_FAILED
                        && std::ffi::CStr::from_ptr(diagnostic.code.as_ptr()).to_bytes()
                            == b"trap:uncaught-exception"
                        && let Some(id) = *task
                    {
                        let original = *diagnostic;
                        let observed = ulua_task_result(instance, id, &mut value, diagnostic);
                        if observed == ULUA_FAILED
                            && std::ffi::CStr::from_ptr(diagnostic.code.as_ptr()).to_bytes()
                                == b"trap:uncaught-exception"
                        {
                            return observed;
                        }
                        *diagnostic = original;
                    }
                    return status;
                }
                if let Some(id) = *task {
                    let status = ulua_task_result(instance, id, &mut value, diagnostic);
                    if status == ULUA_OK {
                        *task = None;
                    } else if status != ULUA_PENDING {
                        return status;
                    }
                }
            }
            ULUA_OK
        }
    }
    pub fn signatures(source: &[u8]) -> Result<Vec<ulua_compiler::HostSignature>, String> {
        let text = std::str::from_utf8(source).map_err(|e| e.to_string())?;
        let header = text
            .lines()
            .take_while(|l| l.trim().is_empty() || l.starts_with("//"))
            .find_map(|l| l.strip_prefix("// grants:"));
        header
            .into_iter()
            .flat_map(|h| h.split(','))
            .map(|n| signature(n.trim()))
            .collect()
    }
    pub unsafe fn grants(
        &mut self,
        context: *mut Context,
        signatures: &[ulua_compiler::HostSignature],
    ) -> Result<Vec<*mut Capability>, String> {
        let mut grants = vec![];
        for s in signatures {
            let mut d = CapabilityDesc::default();
            d.name = s.name.as_ptr().cast();
            d.name_length = s.name.len();
            d.parameter_types = s.parameters.as_ptr();
            d.parameter_count = s.parameters.len();
            d.result_type = s.result;
            if s.asynchronous {
                d.start = Some(later);
            } else {
                d.function = Some(match s.name.as_str() {
                    "hostAdd" => add,
                    "hostConcat" => concat,
                    "hostNot" => not,
                    "hostLog" => log,
                    _ => return Err("Unknown fixture".into()),
                });
            }
            d.userdata = ptr::from_mut(self).cast();
            let mut cap = ptr::null_mut();
            // SAFETY: Descriptors and signatures cover their lengths; the fixture outlives all runs.
            if unsafe { ulua_capability_new(context, &d, &mut cap) } != ULUA_OK {
                return Err("Fixture capability failed".into());
            }
            grants.push(cap);
        }
        Ok(grants)
    }
}
fn signature(name: &str) -> Result<ulua_compiler::HostSignature, String> {
    let (p, r) = match name {
        "hostAdd" => (vec![1, 1], 1),
        "hostConcat" => (vec![3, 3], 3),
        "hostNot" => (vec![2], 2),
        "hostLog" => (vec![3], 0),
        "hostLater" => (vec![1], 1),
        _ => return Err(format!("Unknown grant fixture: {name}")),
    };
    let mut signature = ulua_compiler::HostSignature::new(name, p, r);
    signature.asynchronous = name == "hostLater";
    Ok(signature)
}
unsafe fn text<'a>(v: &Value) -> Option<&'a str> {
    // SAFETY: The runtime guarantees readable UTF-8 argument bytes through this host call.
    let b = if v.length == 0 {
        &[]
    } else {
        unsafe { slice::from_raw_parts(v.data.cast(), v.length) }
    };
    std::str::from_utf8(b).ok()
}
unsafe extern "C" fn add(_: *mut c_void, a: *const Value, _: usize, out: *mut Value) -> i32 {
    // SAFETY: The registered signature guarantees two numbers and writable result storage.
    unsafe {
        (*out).r#type = 1;
        (*out).number = (*a).number + (*a.add(1)).number;
    }
    0
}
unsafe extern "C" fn not(_: *mut c_void, a: *const Value, _: usize, out: *mut Value) -> i32 {
    // SAFETY: The registered signature guarantees a boolean and writable result storage.
    unsafe {
        (*out).r#type = 2;
        (*out).boolean = u32::from((*a).boolean == 0);
    }
    0
}
unsafe extern "C" fn concat(data: *mut c_void, a: *const Value, _: usize, out: *mut Value) -> i32 {
    // SAFETY: Fixture data stays live; the signature guarantees two readable strings and writable result storage.
    unsafe {
        let f = &mut *data.cast::<Fixtures>();
        let (Some(left), Some(right)) = (text(&*a), text(&*a.add(1))) else {
            return 1;
        };
        f.buffer.clear();
        f.buffer.push_str(left);
        f.buffer.push_str(right);
        (*out).r#type = 3;
        (*out).data = f.buffer.as_ptr().cast();
        (*out).length = f.buffer.len();
    }
    0
}
unsafe extern "C" fn log(data: *mut c_void, a: *const Value, _: usize, _: *mut Value) -> i32 {
    // SAFETY: Fixture data stays live; the signature guarantees one readable string.
    unsafe {
        let Some(text) = text(&*a) else {
            return 1;
        };
        (*data.cast::<Fixtures>())
            .output
            .push(format!("log:{text}"));
    }
    0
}

unsafe extern "C" fn later(data: *mut c_void, a: *const Value, _: usize, request: u64) -> i32 {
    // SAFETY: The registered signature supplies a number; the harness owns the live fixture.
    unsafe {
        (*data.cast::<Fixtures>())
            .requests
            .push_back((request, (*a).number));
    }
    0
}
