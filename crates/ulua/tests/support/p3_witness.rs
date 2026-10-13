//! ABI declaration text for host growth witnesses.
use std::ptr;
use ulua::*;
// External ABI descriptors are non-exhaustive and require field assignments.
#[allow(clippy::field_reassign_with_default)]
pub fn declarations(signatures: &[ulua_compiler::HostSignature]) -> String {
    unsafe extern "C" fn unused(
        _: *mut std::ffi::c_void,
        _: *const Value,
        _: usize,
        _: *mut Value,
    ) -> i32 {
        1
    }
    let mut c = ptr::null_mut();
    // SAFETY: This helper serializes a temporary context and owns all descriptor and output storage.
    unsafe {
        assert_eq!(ulua_context_new(&mut c), ULUA_OK);
        let mut grants = vec![];
        for s in signatures {
            let mut d = CapabilityDesc::default();
            d.name = s.name.as_ptr().cast();
            d.name_length = s.name.len();
            d.parameter_types = s.parameters.as_ptr();
            d.parameter_count = s.parameters.len();
            d.result_type = s.result;
            d.function = Some(unused);
            let mut cap = ptr::null_mut();
            assert_eq!(ulua_capability_new(c, &d, &mut cap), ULUA_OK);
            grants.push(cap);
        }
        let mut length = 0;
        assert_eq!(
            ulua_declarations(
                grants.as_ptr(),
                grants.len(),
                ptr::null_mut(),
                0,
                &mut length
            ),
            ULUA_INVALID_ARGUMENT
        );
        let mut bytes = vec![0; length];
        assert_eq!(
            ulua_declarations(
                grants.as_ptr(),
                grants.len(),
                bytes.as_mut_ptr().cast(),
                bytes.len(),
                &mut length
            ),
            ULUA_OK
        );
        assert_eq!(ulua_context_free(c), ULUA_OK);
        String::from_utf8(bytes).unwrap()
    }
}
