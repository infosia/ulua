//! Fixed native Math operations. No VM API runs on a Rust frame.
pub(crate) extern "C" fn dispatch(op: u32, a: f64, b: f64) -> f64 {
    match op {
        0 => libm::acos(a),
        1 => libm::acosh(a),
        2 => libm::asin(a),
        3 => libm::asinh(a),
        4 => libm::atan(a),
        5 => libm::atanh(a),
        6 => libm::atan2(a, b),
        7 => libm::cbrt(a),
        8 => libm::cos(a),
        9 => libm::cosh(a),
        10 => libm::exp(a),
        11 => libm::expm1(a),
        12 => libm::hypot(a, b),
        13 => libm::log(a),
        14 => libm::log1p(a),
        15 => libm::log10(a),
        16 => libm::log2(a),
        17 => {
            if b.is_nan() || (a.abs() == 1.0 && b.is_infinite()) {
                f64::NAN
            } else {
                libm::pow(a, b)
            }
        }
        18 => libm::sin(a),
        19 => libm::sinh(a),
        20 => libm::tan(a),
        21 => libm::tanh(a),
        _ => f64::NAN,
    }
}

/// Restart the instance Math.random sequence from a 64-bit seed.
/// # Safety
/// The instance must be live and its context must be serialized.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ulua_instance_seed_random(
    instance: *mut crate::Instance,
    seed: u64,
) -> crate::Status {
    if instance.is_null() {
        return crate::ULUA_INVALID_ARGUMENT;
    }
    // SAFETY: The caller supplies a live instance and serializes its context.
    unsafe {
        crate::objects::guard((*instance).context, std::ptr::null_mut(), || {
            crate::native::ulua_internal_shim_seed_random((*instance).vm, seed);
            crate::ULUA_OK
        })
    }
}
