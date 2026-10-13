//! Capability ownership, descriptor checks, and the Rust callback boundary.
use crate::{
    abi::{bytes, value_valid},
    objects::guard,
    *,
};
use std::{
    cell::Cell,
    ffi::c_void,
    panic::{AssertUnwindSafe, catch_unwind},
    ptr,
    rc::Rc,
    slice,
};

/// Opaque host capability. Modules and instances retain its state after free.
#[non_exhaustive]
pub struct Capability {
    pub(crate) context: *mut Context,
    pub(crate) state: Rc<State>,
}
pub(crate) struct State {
    pub(crate) signature: ulua_compiler::HostSignature,
    pub(crate) revoked: Cell<bool>,
    function: HostFn,
    userdata: *mut c_void,
}
pub(crate) struct Binding {
    pub(crate) context: *mut Context,
    pub(crate) state: Rc<State>,
}
#[repr(C)]
pub(crate) struct NativeCapability {
    pub(crate) desc: CapabilityDesc,
    pub(crate) revoked: *const bool,
}
impl Binding {
    pub(crate) fn native(&mut self) -> NativeCapability {
        let userdata = ptr::from_mut(self).cast();
        let s = &self.state;
        let desc = CapabilityDesc {
            name: s.signature.name.as_ptr().cast(),
            name_length: s.signature.name.len(),
            parameter_types: s.signature.parameters.as_ptr(),
            parameter_count: s.signature.parameters.len(),
            result_type: s.signature.result,
            function: Some(invoke),
            userdata,
            ..CapabilityDesc::default()
        };
        NativeCapability {
            desc,
            revoked: self.state.revoked.as_ptr(),
        }
    }
}
fn name_valid(name: &str) -> bool {
    let mut bytes = name.bytes();
    if !(1..=64).contains(&name.len())
        || !bytes
            .next()
            .is_some_and(|b| b.is_ascii_alphabetic() || matches!(b, b'_' | b'$'))
        || !bytes.all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'$'))
    {
        return false;
    }
    crate::host_names::GLOBAL_NAMES
        .binary_search(&name)
        .is_err()
        && !matches!(
            name,
            "await"
                | "break"
                | "case"
                | "catch"
                | "class"
                | "const"
                | "continue"
                | "debugger"
                | "default"
                | "delete"
                | "do"
                | "else"
                | "enum"
                | "export"
                | "extends"
                | "false"
                | "finally"
                | "for"
                | "function"
                | "if"
                | "import"
                | "in"
                | "instanceof"
                | "new"
                | "null"
                | "return"
                | "super"
                | "switch"
                | "this"
                | "throw"
                | "true"
                | "try"
                | "typeof"
                | "var"
                | "void"
                | "while"
                | "with"
                | "yield"
                | "implements"
                | "interface"
                | "let"
                | "package"
                | "private"
                | "protected"
                | "public"
                | "static"
                | "eval"
                | "arguments"
        )
}
/// Create a capability and copy its name and signature.
/// # Safety
/// The context and descriptor storage must be live. Host data must outlive retained grants.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ulua_capability_new(
    context: *mut Context,
    desc: *const CapabilityDesc,
    out: *mut *mut Capability,
) -> Status {
    // SAFETY: The host supplies live objects and aligned descriptor storage.
    unsafe {
        if context.is_null() {
            return ULUA_INVALID_ARGUMENT;
        }
        guard(context, ptr::null_mut(), || {
            if desc.is_null() || out.is_null() {
                return ULUA_INVALID_ARGUMENT;
            }
            let d = &*desc;
            if d.struct_size != size_of::<CapabilityDesc>() as u32
                || d.reserved != 0
                || d.reserved2 != 0
                || !(1..=64).contains(&d.name_length)
                || d.parameter_count > ULUA_MAX_HOST_PARAMETERS
                || (d.parameter_types.is_null() && d.parameter_count != 0)
                || d.result_type > ULUA_TYPE_STRING
            {
                return ULUA_INVALID_ARGUMENT;
            }
            let Some(function) = d.function else {
                return ULUA_INVALID_ARGUMENT;
            };
            let Some(name) = bytes(d.name, d.name_length)
                .and_then(|b| std::str::from_utf8(b).ok())
                .filter(|n| name_valid(n))
            else {
                return ULUA_INVALID_ARGUMENT;
            };
            let parameters = if d.parameter_count == 0 {
                &[]
            } else {
                slice::from_raw_parts(d.parameter_types, d.parameter_count)
            };
            if !parameters
                .iter()
                .all(|t| matches!(*t, ULUA_TYPE_NUMBER | ULUA_TYPE_BOOLEAN | ULUA_TYPE_STRING))
            {
                return ULUA_INVALID_ARGUMENT;
            }
            let state = Rc::new(State {
                signature: ulua_compiler::HostSignature::new(
                    name,
                    parameters.to_vec(),
                    d.result_type,
                ),
                revoked: Cell::new(false),
                function,
                userdata: d.userdata,
            });
            let Ok(mut registry) = (*context).capabilities.try_borrow_mut() else {
                return ULUA_INTERNAL;
            };
            let capability = Box::into_raw(Box::new(Capability { context, state }));
            registry.push(capability);
            out.write(capability);
            ULUA_OK
        })
    }
}
/// Permanently revoke a live capability.
/// # Safety
/// The capability must be live and belong to a live context.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ulua_capability_revoke(capability: *mut Capability) -> Status {
    if capability.is_null() {
        return ULUA_INVALID_ARGUMENT;
    }
    // SAFETY: The host serializes the live capability and its context.
    unsafe {
        guard((*capability).context, ptr::null_mut(), || {
            let state = &(*capability).state;
            state.revoked.set(true);
            ULUA_OK
        })
    }
}
/// Revoke and release the host reference. Null succeeds.
/// # Safety
/// A non-null capability must be live. Its pointer becomes invalid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ulua_capability_free(capability: *mut Capability) -> Status {
    if capability.is_null() {
        return ULUA_OK;
    }
    // SAFETY: The host serializes this live object; remove its registry entry before destruction.
    unsafe {
        let context = (*capability).context;
        if (*context).busy.get() {
            return ULUA_BUSY;
        }
        let status = catch_unwind(AssertUnwindSafe(|| {
            let Ok(mut registry) = (*context).capabilities.try_borrow_mut() else {
                return ULUA_INTERNAL;
            };
            let state = &(*capability).state;
            state.revoked.set(true);
            registry.retain(|p| *p != capability);
            drop(Box::from_raw(capability));
            ULUA_OK
        }))
        .unwrap_or(ULUA_INTERNAL);
        if status == ULUA_INTERNAL {
            (*context).poisoned.set(true);
        }
        status
    }
}
pub(crate) unsafe fn grants(
    context: *mut Context,
    pointers: *const *mut Capability,
    count: usize,
) -> Option<Vec<Rc<State>>> {
    if (pointers.is_null() && count != 0)
        || count > isize::MAX as usize / size_of::<*mut Capability>()
    {
        return None;
    }
    // SAFETY: The caller supplies readable grant pointers and serializes their contexts.
    unsafe {
        let pointers = if count == 0 {
            &[]
        } else {
            slice::from_raw_parts(pointers, count)
        };
        let mut names = std::collections::HashSet::new();
        let mut states = Vec::with_capacity(count);
        for &p in pointers {
            if p.is_null() {
                return None;
            }
            let capability = &*p;
            if capability.context != context
                || capability.state.revoked.get()
                || !names.insert(capability.state.signature.name.as_str())
            {
                return None;
            }
            states.push(capability.state.clone());
        }
        Some(states)
    }
}
#[cfg(test)]
thread_local! { pub(crate) static PANIC_HOST: Cell<bool> = const { Cell::new(false) }; }
unsafe extern "C" fn invoke(
    userdata: *mut c_void,
    arguments: *const Value,
    count: usize,
    result: *mut Value,
) -> i32 {
    // SAFETY: The instance owns this stable binding; native storage covers arguments and result.
    unsafe {
        let binding = &*userdata.cast::<Binding>();
        let answer = catch_unwind(AssertUnwindSafe(|| {
            #[cfg(test)]
            if PANIC_HOST.with(|v| v.replace(false)) {
                panic!("test panic around host call");
            }
            let s = &binding.state;
            if (s.function)(s.userdata, arguments, count, result) != 0 {
                return 11;
            }
            if (*result).r#type != s.signature.result || !value_valid(&*result) {
                return 13;
            }
            0
        }));
        match answer {
            Ok(code) => code,
            Err(_) => {
                (*binding.context).poisoned.set(true);
                5
            }
        }
    }
}
