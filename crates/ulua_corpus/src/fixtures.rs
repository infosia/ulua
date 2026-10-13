//! Fixed corpus capabilities mirror the Node witness fixtures.
// External ABI descriptors are non-exhaustive and require field assignments.
#![allow(clippy::field_reassign_with_default)]

use std::{ffi::c_void, ptr, slice};
use ulua::*;

pub struct Fixtures {
    pub output: Vec<String>,
    buffer: String,
}
impl Fixtures {
    pub fn new() -> Self {
        Self {
            output: vec![],
            buffer: String::new(),
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
            d.function = Some(match s.name.as_str() {
                "hostAdd" => add,
                "hostConcat" => concat,
                "hostNot" => not,
                "hostLog" => log,
                _ => return Err("Unknown fixture".into()),
            });
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
        _ => return Err(format!("Unknown grant fixture: {name}")),
    };
    Ok(ulua_compiler::HostSignature::new(name, p, r))
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
