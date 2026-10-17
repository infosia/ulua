//! Closed Number surface and exact ECMAScript constants.
pub(crate) fn constant(name: &str) -> Option<f64> {
    Some(match name {
        "MAX_SAFE_INTEGER" => 9007199254740991.0,
        "MIN_SAFE_INTEGER" => -9007199254740991.0,
        "EPSILON" => f64::EPSILON,
        "MAX_VALUE" => f64::MAX,
        "MIN_VALUE" => f64::from_bits(1),
        "POSITIVE_INFINITY" => f64::INFINITY,
        "NEGATIVE_INFINITY" => f64::NEG_INFINITY,
        "NaN" => f64::NAN,
        _ => return None,
    })
}
pub(crate) fn member(name: &str) -> Option<(&'static str, usize, usize, bool)> {
    Some(match name {
        "isNaN" => ("__ulua_number_isNaN", 1, 1, false),
        "isFinite" => ("__ulua_number_isFinite", 1, 1, false),
        "isInteger" => ("__ulua_number_isInteger", 1, 1, false),
        "isSafeInteger" => ("__ulua_number_isSafeInteger", 1, 1, false),
        "parseInt" => ("__ulua_number_parseInt", 2, 2, true),
        "parseFloat" => ("__ulua_number_parseFloat", 1, 1, true),
        "toFixed" => ("__ulua_number_toFixed", 0, 1, false),
        "toExponential" => ("__ulua_number_toExponential", 0, 1, false),
        "toPrecision" => ("__ulua_number_toPrecision", 1, 1, false),
        "toString" => ("__ulua_number_toString", 1, 1, false),
        _ => return None,
    })
}
