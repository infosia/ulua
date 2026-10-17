//! Unicode 17 default casing and the scalar-only UTF-8 constructor boundary.
fn convert(op: u32, a: f64, s: &str) -> Result<String, ()> {
    match op {
        10 => Ok(s.to_uppercase()),
        11 => Ok(s.to_lowercase()),
        12 | 13 => {
            let cp = if op == 13 {
                if !a.is_finite() || a == 0.0 {
                    0
                } else {
                    a.trunc().rem_euclid(65536.0) as u32
                }
            } else {
                if !a.is_finite() || a.fract() != 0.0 || !(0.0..=1114111.0).contains(&a) {
                    return Err(());
                }
                a as u32
            };
            char::from_u32(cp).map(|c| c.to_string()).ok_or(())
        }
        _ => Err(()),
    }
}
// SAFETY: C++ owns live UTF-8 input and an output buffer of at least 3*len+4 bytes.
pub(crate) unsafe fn dispatch(
    op: u32,
    a: f64,
    input: *const u8,
    len: usize,
    output: *mut u8,
) -> i32 {
    let s = unsafe { std::str::from_utf8_unchecked(std::slice::from_raw_parts(input, len)) };
    match convert(op, a, s) {
        Ok(result)
            if result.len() <= len.saturating_mul(3).saturating_add(4)
                && result.len() <= i32::MAX as usize =>
        {
            unsafe {
                std::ptr::copy_nonoverlapping(result.as_ptr(), output, result.len());
            }
            result.len() as i32
        }
        _ => -1,
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn hex(s: &str) -> String {
        s.as_bytes().iter().map(|b| format!("{b:02x}")).collect()
    }
    #[test]
    fn node_case_table_every_code_point_and_context() {
        assert_eq!(char::UNICODE_VERSION, (17, 0, 0));
        let mut rows = include_str!("../test-data/string-case-oracle.tsv")
            .lines()
            .filter(|s| !s.starts_with('#'))
            .peekable();
        let mut changed = 0;
        for cp in 0..=0x10ffff {
            let Some(c) = char::from_u32(cp) else {
                continue;
            };
            let input = c.to_string();
            let upper = convert(10, 0.0, &input).unwrap();
            let lower = convert(11, 0.0, &input).unwrap();
            // This independently pins the VM scratch bound for all scalar mappings.
            assert!(upper.len() <= 3 * input.len());
            assert!(lower.len() <= 3 * input.len());
            let key = format!("{cp:x}");
            if rows
                .peek()
                .is_some_and(|row| row.split('\t').next() == Some(&key))
            {
                let fields: Vec<_> = rows.next().unwrap().split('\t').collect();
                assert_eq!(hex(&upper), fields[1], "upper U+{key}");
                assert_eq!(hex(&lower), fields[2], "lower U+{key}");
                changed += 1;
            } else {
                assert_eq!(upper, input, "upper U+{key}");
                assert_eq!(lower, input, "lower U+{key}");
            }
        }
        assert_eq!(changed, 3037);
        for row in rows {
            let fields: Vec<_> = row.split('\t').collect();
            let bytes: Vec<_> = fields[0].as_bytes()[1..]
                .chunks_exact(2)
                .map(|c| u8::from_str_radix(std::str::from_utf8(c).unwrap(), 16).unwrap())
                .collect();
            let input = String::from_utf8(bytes).unwrap();
            assert_eq!(hex(&convert(10, 0.0, &input).unwrap()), fields[1]);
            assert_eq!(hex(&convert(11, 0.0, &input).unwrap()), fields[2]);
        }
    }
    #[test]
    fn scalar_and_uint16_edges() {
        for cp in [
            -1.0,
            1.5,
            f64::NAN,
            f64::INFINITY,
            f64::NEG_INFINITY,
            0xd800 as f64,
            0xdfff as f64,
            0x110000 as f64,
        ] {
            assert!(convert(12, cp, "").is_err(), "{cp}");
        }
        for cp in [0, 0xd7ff, 0xe000, 0xffff, 0x10000, 0x10ffff] {
            assert_eq!(
                convert(12, cp as f64, "").unwrap(),
                char::from_u32(cp).unwrap().to_string()
            );
        }
        for (x, cp) in [
            (f64::NAN, 0),
            (f64::INFINITY, 0),
            (f64::NEG_INFINITY, 0),
            (-0.0, 0),
            (-1.0, 65535),
            (65537.9, 1),
            (-65537.9, 65535),
            (1e100, 0),
        ] {
            assert_eq!(
                convert(13, x, "").unwrap(),
                char::from_u32(cp).unwrap().to_string()
            );
        }
        for x in [55296.0, 57343.0, 120832.0, -10240.0] {
            assert!(convert(13, x, "").is_err());
        }
    }
}

#[cfg(test)]
#[test]
fn case_charge_budget_edges() {
    #[link(name = "ulua_test_native", kind = "static")]
    unsafe extern "C" {
        fn ulua_internal_probe_native_bytes_per_step() -> u64;
        fn ulua_internal_probe_native_call_steps() -> u64;
        fn ulua_internal_probe_number_charge(
            budget: u64,
            mode: i32,
            dispatch: unsafe extern "C" fn(
                u32,
                f64,
                f64,
                i32,
                *const u8,
                usize,
                *mut u8,
                *mut f64,
            ) -> i32,
            counters: *mut u64,
            error: *mut std::ffi::c_char,
        ) -> i32;
    }
    // SAFETY: These getters have no pointer arguments or side effects.
    let charge = unsafe {
        ulua_internal_probe_native_call_steps() + 32 / ulua_internal_probe_native_bytes_per_step()
    };
    for (budget, status, completed) in [
        (1, 2, 0),
        (charge, 2, 0),
        (charge + 1, 2, 1),
        (3 * (charge + 1), 2, 3),
        (3 * (charge + 1) + 1, 0, 3),
    ] {
        let mut counters = [0; 4];
        let mut error = [0; 256];
        // SAFETY: The probe owns live input and output buffers and invokes the product dispatcher.
        let actual = unsafe {
            ulua_internal_probe_number_charge(
                budget,
                2,
                crate::number::dispatch,
                counters.as_mut_ptr(),
                error.as_mut_ptr(),
            )
        };
        assert_eq!(actual, status, "budget={budget}: {counters:?}");
        assert_eq!(counters[0], budget);
        assert_eq!(counters[3], completed);
    }
}
