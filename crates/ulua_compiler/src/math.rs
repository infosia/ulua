//! Math names and ECMAScript constant bits.
pub(crate) fn member(name: &str) -> Option<(&'static str, usize)> {
    Some(match name {
        "abs" => ("__ulua_math_abs", 1),
        "ceil" => ("__ulua_ceil", 1),
        "floor" => ("__ulua_floor", 1),
        "sqrt" => ("__ulua_math_sqrt", 1),
        "round" => ("__ulua_math_round", 1),
        "sign" => ("__ulua_math_sign", 1),
        "trunc" => ("__ulua_math_trunc", 1),
        "max" => ("__ulua_math_max", 2),
        "min" => ("__ulua_math_min", 2),
        "clz32" => ("__ulua_math_clz32", 1),
        "imul" => ("__ulua_math_imul", 2),
        "fround" => ("__ulua_math_fround", 1),
        "acos" => ("__ulua_math_acos", 1),
        "acosh" => ("__ulua_math_acosh", 1),
        "asin" => ("__ulua_math_asin", 1),
        "asinh" => ("__ulua_math_asinh", 1),
        "atan" => ("__ulua_math_atan", 1),
        "atanh" => ("__ulua_math_atanh", 1),
        "atan2" => ("__ulua_math_atan2", 2),
        "cbrt" => ("__ulua_math_cbrt", 1),
        "cos" => ("__ulua_math_cos", 1),
        "cosh" => ("__ulua_math_cosh", 1),
        "exp" => ("__ulua_math_exp", 1),
        "expm1" => ("__ulua_math_expm1", 1),
        "hypot" => ("__ulua_math_hypot", 2),
        "log" => ("__ulua_math_log", 1),
        "log1p" => ("__ulua_math_log1p", 1),
        "log10" => ("__ulua_math_log10", 1),
        "log2" => ("__ulua_math_log2", 1),
        "pow" => ("__ulua_math_pow", 2),
        "sin" => ("__ulua_math_sin", 1),
        "sinh" => ("__ulua_math_sinh", 1),
        "tan" => ("__ulua_math_tan", 1),
        "tanh" => ("__ulua_math_tanh", 1),
        "random" => ("__ulua_math_random", 0),
        _ => return None,
    })
}
pub(crate) fn constant(name: &str) -> Option<f64> {
    Some(f64::from_bits(match name {
        "E" => 0x4005bf0a8b145769,
        "LN2" => 0x3fe62e42fefa39ef,
        "LN10" => 0x40026bb1bbb55516,
        "LOG2E" => 0x3ff71547652b82fe,
        "LOG10E" => 0x3fdbcb7b1526e50e,
        "PI" => 0x400921fb54442d18,
        "SQRT1_2" => 0x3fe6a09e667f3bcd,
        "SQRT2" => 0x3ff6a09e667f3bcd,
        _ => return None,
    }))
}

pub(crate) fn native(name: &str) -> Option<&str> {
    let member = name.strip_prefix("__ulua_math_")?;
    match member {
        "acos" | "acosh" | "asin" | "asinh" | "atan" | "atanh" | "atan2" | "cbrt" | "cos"
        | "cosh" | "exp" | "expm1" | "hypot" | "log" | "log1p" | "log10" | "log2" | "pow"
        | "sin" | "sinh" | "tan" | "tanh" | "random" => Some(member),
        _ => None,
    }
}
