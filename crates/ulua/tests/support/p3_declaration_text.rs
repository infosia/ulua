//! Copy declaration bytes through the public ABI.
use std::ptr;
use ulua::*;

pub unsafe fn text(grants: &[*mut Capability]) -> Result<String, String> {
    let mut length = 0;
    // SAFETY: The caller owns and serializes live grants; the null query writes only length.
    unsafe {
        if ulua_declarations(
            grants.as_ptr(),
            grants.len(),
            ptr::null_mut(),
            0,
            &mut length,
        ) != ULUA_INVALID_ARGUMENT
            || length == 0
        {
            return Err("Declaration length query failed".into());
        }
        let mut bytes = vec![0; length];
        if ulua_declarations(
            grants.as_ptr(),
            grants.len(),
            bytes.as_mut_ptr().cast(),
            bytes.len(),
            &mut length,
        ) != ULUA_OK
        {
            return Err("Declarations failed".into());
        }
        String::from_utf8(bytes).map_err(|e| e.to_string())
    }
}
