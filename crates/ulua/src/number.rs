//! Native Number operations. Rust frames never call the VM or survive a VM unwind.
const DIGITS: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyz";
fn whitespace(c: char) -> bool {
    matches!(c, '\u{0009}'..='\u{000d}' | ' ' | '\u{00a0}' | '\u{1680}' | '\u{2000}'..='\u{200a}' | '\u{2028}' | '\u{2029}' | '\u{202f}' | '\u{205f}' | '\u{3000}' | '\u{feff}')
}
fn integer(x: f64) -> f64 {
    if x.is_nan() { 0.0 } else { x.trunc() }
}
fn int32(x: f64) -> i32 {
    if !x.is_finite() || x == 0.0 {
        return 0;
    }
    let r = x.trunc() % 4294967296.0;
    (if r < 0.0 { r + 4294967296.0 } else { r } as u32) as i32
}
pub(crate) fn parse_int(s: &str, radix: f64) -> Result<f64, ()> {
    let radix = int32(radix);
    if !(2..=36).contains(&radix) {
        return Err(());
    }
    let s = s.trim_start_matches(whitespace);
    let (negative, s) = if let Some(s) = s.strip_prefix('-') {
        (true, s)
    } else {
        (false, s.strip_prefix('+').unwrap_or(s))
    };
    let s = if radix == 16 && (s.starts_with("0x") || s.starts_with("0X")) {
        &s[2..]
    } else {
        s
    };
    // An integer beyond 1120 bits always rounds to infinity. Keep consuming the
    // prefix without growing scratch memory; work is bounded per input byte.
    let mut limbs = [0u32; 35];
    let mut overflow = false;
    let mut used = 1;
    let mut any = false;
    for c in s.bytes() {
        let d = match c {
            b'0'..=b'9' => c - b'0',
            b'a'..=b'z' => c - b'a' + 10,
            b'A'..=b'Z' => c - b'A' + 10,
            _ => break,
        };
        if d as i32 >= radix {
            break;
        }
        any = true;
        if overflow {
            continue;
        }
        let mut carry = d as u64;
        for word in &mut limbs[..used] {
            let n = *word as u64 * radix as u64 + carry;
            *word = n as u32;
            carry = n >> 32;
        }
        if carry != 0 {
            if used == limbs.len() {
                overflow = true;
            } else {
                limbs[used] = carry as u32;
                used += 1;
            }
        }
    }
    if !any {
        return Ok(f64::NAN);
    }
    let bits = limbs
        .iter()
        .rposition(|&w| w != 0)
        .map_or(0, |i| i * 32 + 32 - limbs[i].leading_zeros() as usize);
    let value = if overflow || bits > 1024 {
        f64::INFINITY
    } else if bits == 0 {
        0.0
    } else {
        let bit = |i: usize| (limbs[i / 32] >> (i % 32)) & 1;
        let shift = bits.saturating_sub(53);
        let mut top = 0u64;
        for i in (shift..bits).rev() {
            top = top * 2 + bit(i) as u64;
        }
        if shift > 0 && bit(shift - 1) != 0 && (top & 1 != 0 || (0..shift - 1).any(|i| bit(i) != 0))
        {
            top += 1;
        }
        (top as f64) * 2.0f64.powi(shift as i32)
    };
    Ok(if negative { -value } else { value })
}
pub(crate) fn parse_float(s: &str) -> f64 {
    let s = s.trim_start_matches(whitespace);
    let b = s.as_bytes();
    let mut i = usize::from(matches!(b.first(), Some(b'+' | b'-')));
    if s[i..].starts_with("Infinity") {
        return if b.first() == Some(&b'-') {
            f64::NEG_INFINITY
        } else {
            f64::INFINITY
        };
    }
    let start = i;
    while i < b.len() && b[i].is_ascii_digit() {
        i += 1;
    }
    let mut any = i > start;
    if b.get(i) == Some(&b'.') {
        i += 1;
        let start = i;
        while i < b.len() && b[i].is_ascii_digit() {
            i += 1;
        }
        any |= i > start;
    }
    if !any {
        return f64::NAN;
    }
    if matches!(b.get(i), Some(b'e' | b'E')) {
        let end = i;
        i += 1;
        if matches!(b.get(i), Some(b'+' | b'-')) {
            i += 1;
        }
        let start = i;
        while i < b.len() && b[i].is_ascii_digit() {
            i += 1;
        }
        if i == start {
            i = end;
        }
    }
    s[..i].parse().unwrap_or(f64::NAN)
}
fn special(x: f64) -> Option<String> {
    if x.is_nan() {
        Some("NaN".into())
    } else if x == f64::INFINITY {
        Some("Infinity".into())
    } else if x == f64::NEG_INFINITY {
        Some("-Infinity".into())
    } else {
        None
    }
}
fn shortest(x: f64) -> (String, i32) {
    let s = format!("{:e}", x.abs());
    let (m, e) = s.split_once('e').unwrap();
    let e: i32 = e.parse().unwrap();
    let mut m = m.to_owned();
    let fraction_digits = m.split_once('.').map_or(0, |(_, f)| f.len());
    // Rust's shortest dtoa picks the larger decimal on a tie. ECMAScript
    // shortest dtoa picks the even significand among equally close choices.
    if tie_down(x, fraction_digits as i32 - e) && m.as_bytes().last().unwrap() % 2 == 1 {
        let last = m.pop().unwrap() as u8;
        m.push((last - 1) as char);
    }
    (m, e)
}
fn fixed_layout(m: &str, e: i32) -> String {
    let digits = m.replace('.', "");
    let point = e + 1;
    if point <= 0 {
        format!("0.{}{digits}", "0".repeat(-point as usize))
    } else if point as usize >= digits.len() {
        format!("{digits}{}", "0".repeat(point as usize - digits.len()))
    } else {
        format!(
            "{}.{}",
            &digits[..point as usize],
            &digits[point as usize..]
        )
    }
}
pub(crate) fn decimal(x: f64) -> String {
    if let Some(s) = special(x) {
        return s;
    }
    if x == 0.0 {
        return "0".into();
    }
    let sign = if x < 0.0 { "-" } else { "" };
    let (m, e) = shortest(x);
    if (-6..21).contains(&e) {
        format!("{sign}{}", fixed_layout(&m, e))
    } else {
        format!("{sign}{m}e{e:+}")
    }
}
// Rust rounds decimal ties to even. Determine an exact halfway case using
// x = m * 2^e; multiplying by 10^k leaves denominator 2 exactly here.
fn tie_down(x: f64, k: i32) -> bool {
    if x == 0.0 {
        return false;
    }
    let bits = x.abs().to_bits();
    let raw = ((bits >> 52) & 2047) as i32;
    let mut m = (bits & ((1u64 << 52) - 1)) | if raw == 0 { 0 } else { 1u64 << 52 };
    let e = if raw == 0 { -1074 } else { raw - 1075 };
    let zeros = m.trailing_zeros() as i32;
    m >>= zeros;
    if e + zeros + k != -1 {
        return false;
    }
    if k < 0 {
        for _ in 0..-k {
            if !m.is_multiple_of(5) {
                return false;
            }
            m /= 5;
        }
    }
    m % 4 == 1 // 5^k is 1 modulo 4, including its inverse.
}
fn increment(s: &mut String) {
    let mut bytes = s.as_bytes().to_vec();
    for c in bytes.iter_mut().rev() {
        if *c == b'.' {
            continue;
        }
        if *c == b'9' {
            *c = b'0';
        } else {
            *c += 1;
            *s = String::from_utf8(bytes).unwrap();
            return;
        }
    }
    bytes.insert(0, b'1');
    *s = String::from_utf8(bytes).unwrap();
}
fn scientific(x: f64, digits: usize) -> (String, i32) {
    let s = format!("{:.*e}", digits, x.abs());
    let (mantissa, exponent) = s.split_once('e').unwrap();
    let mut e: i32 = exponent.parse().unwrap();
    let mut m = mantissa.to_owned();
    if tie_down(x, digits as i32 - e) {
        increment(&mut m);
        if m.starts_with("10") {
            m = if digits == 0 {
                "1".into()
            } else {
                format!("1.{}", "0".repeat(digits))
            };
            e += 1;
        }
    }
    (m, e)
}
pub(crate) fn format_number(op: u32, x: f64, arg: Option<f64>) -> Result<String, ()> {
    let d = integer(arg.unwrap_or(0.0));
    if op == 5 {
        if !(2.0..=36.0).contains(&d) {
            return Err(());
        }
        return Ok(if d == 10.0 {
            decimal(x)
        } else {
            radix_string(x, d as u32)
        });
    }
    if !(if op == 4 { 1.0 } else { 0.0 }..=100.0).contains(&d) {
        return Err(());
    }
    if let Some(s) = special(x) {
        return Ok(s);
    }
    let sign = if x < 0.0 { "-" } else { "" };
    let n = d as usize;
    if op == 2 {
        if x.abs() >= 1e21 {
            return Ok(decimal(x));
        }
        let mut s = format!("{:.*}", n, x.abs());
        if tie_down(x, n as i32) {
            increment(&mut s);
        }
        return Ok(format!("{sign}{s}"));
    }
    if op == 3 && arg.is_none() {
        let (m, e) = shortest(x);
        return Ok(format!("{sign}{m}e{e:+}"));
    }
    let (m, e) = scientific(x, if op == 4 { n - 1 } else { n });
    if op == 3 || e < -6 || e >= n as i32 {
        return Ok(format!("{sign}{m}e{e:+}"));
    }
    Ok(format!("{sign}{}", fixed_layout(&m, e)))
}
// V8's precision-bounded radix conversion, including its integer zero fill.
// Algorithm reference: v8/src/numbers/conversions.cc DoubleToRadixStringView.
pub(crate) fn radix_string(x: f64, radix: u32) -> String {
    if let Some(s) = special(x) {
        return s;
    }
    if x == 0.0 {
        return "0".into();
    }
    let value = x.abs();
    let r = radix as f64;
    let mut whole = value.floor();
    let mut fraction = value - whole;
    let mut delta = ((f64::from_bits(value.to_bits() + 1) - value) * 0.5).max(f64::from_bits(1));
    let mut tail: Vec<u8> = vec![];
    if fraction >= delta {
        loop {
            fraction *= r;
            delta *= r;
            let digit = fraction as u32;
            tail.push(digit as u8);
            fraction -= digit as f64;
            if (fraction > 0.5 || (fraction == 0.5 && digit & 1 != 0)) && fraction + delta > 1.0 {
                loop {
                    match tail.pop() {
                        Some(d) if d + 1 < radix as u8 => {
                            tail.push(d + 1);
                            break;
                        }
                        Some(_) => (),
                        None => {
                            whole += 1.0;
                            break;
                        }
                    }
                }
                break;
            }
            if fraction < delta {
                break;
            }
        }
    }
    let mut head = vec![];
    while whole / r >= 2.0f64.powi(53) {
        whole /= r;
        head.push(b'0');
    }
    loop {
        let rem = whole % r;
        head.push(DIGITS[rem as usize]);
        whole = (whole - rem) / r;
        if whole <= 0.0 {
            break;
        }
    }
    if x < 0.0 {
        head.push(b'-');
    }
    head.reverse();
    if !tail.is_empty() {
        head.push(b'.');
        head.extend(tail.into_iter().map(|d| DIGITS[d as usize]));
    }
    String::from_utf8(head).unwrap()
}
/// Rev 6 cost model: 32 + 3 * output bytes + ceil(abs(floor(log2(abs(x)))) / 2).
/// Zero and nonfinite values use exponent 0. Digits are represented by the
/// actual output length. Calibrated with the product-VM number_format_sweep
/// (same Script/run and baseline as p55_ratio), including subnormals, near 1,
/// near 2^53 and near MAX_VALUE, digits 0/20/100 and radixes 2/3/10/36.
/// 225 shapes: maximum 3.769 (2^53-1 in radix 2); the full measurement is
/// test-data/number-format-sweep.tsv. Ordinary (1).toFixed(0) costs 36 steps
/// including the native base, instead of a fixed 512. This replaces
/// the raw-Luau calibration which understated ratios (fixed 512 reached 7.162).
fn format_steps(x: f64, _digits: Option<f64>, output: &str) -> u64 {
    let magnitude = if x == 0.0 || !x.is_finite() {
        0
    } else {
        let bits = x.abs().to_bits();
        let raw = ((bits >> 52) & 2047) as i32;
        let e = if raw == 0 {
            -1074 + (63 - bits.leading_zeros()) as i32
        } else {
            raw - 1023
        };
        e.unsigned_abs() as u64
    };
    32 + 3 * output.len() as u64 + magnitude.div_ceil(2)
}

/// Output buffer is owned by C++; every allocation and raise happens after return.
pub(crate) unsafe extern "C" fn dispatch(
    op: u32,
    a: f64,
    b: f64,
    has_b: i32,
    input: *const u8,
    len: usize,
    output: *mut u8,
    number: *mut f64,
) -> i32 {
    if op == 14 {
        // SAFETY: The JSON shim provides its private state and bounded scratch buffer.
        return unsafe { crate::json::dispatch(input, len, a, output, number) };
    }
    if op >= 10 {
        // SAFETY: The shim supplies the input and a 3*len (or four byte) output buffer.
        return unsafe { crate::string_case::dispatch(op, a, input, len, output) };
    }
    if op <= 1 {
        // SAFETY: The shim supplies the live UTF-8 Luau string for this call.
        let s = unsafe { std::str::from_utf8_unchecked(std::slice::from_raw_parts(input, len)) };
        let value = if op == 0 {
            parse_int(s, b)
        } else {
            Ok(parse_float(s))
        };
        return match value {
            Ok(n) => {
                unsafe {
                    *number = n;
                }
                0
            }
            Err(()) => -1,
        };
    }
    if op >= 6 {
        let yes = match op {
            6 => a.is_nan(),
            7 => a.is_finite(),
            8 => a.is_finite() && a.trunc() == a,
            _ => a.is_finite() && a.trunc() == a && a.abs() <= 9007199254740991.0,
        };
        unsafe {
            *number = f64::from(yes);
        }
        return 0;
    }
    match format_number(op, a, (has_b != 0).then_some(b)) {
        Ok(s) => {
            assert!(s.len() <= 2300);
            unsafe {
                *number = format_steps(a, (has_b != 0).then_some(b), &s) as f64;
                std::ptr::copy_nonoverlapping(s.as_ptr(), output, s.len());
            }
            s.len() as i32
        }
        Err(()) => -1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn unhex(s: &str) -> String {
        String::from_utf8(
            (0..s.len())
                .step_by(2)
                .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
                .collect(),
        )
        .unwrap()
    }
    fn same(a: f64, b: f64) -> bool {
        (a.is_nan() && b.is_nan()) || a.to_bits() == b.to_bits()
    }
    #[test]
    fn node_number_tables() {
        let mut counts = [0; 10];
        let mut differences = [0; 10];
        let mut radix_counts = [0; 37];
        let mut radix_differences = [0; 37];
        let mut max_ulp = 0;
        let mut failures = vec![];
        for line in include_str!("../test-data/number-oracle.tsv")
            .lines()
            .filter(|l| !l.starts_with('#'))
        {
            let f: Vec<_> = line.split('\t').collect();
            let op: usize = f[0].parse().unwrap();
            counts[op] += 1;
            let mismatch = if op <= 1 {
                let s = unhex(f[1]);
                let expected = f64::from_bits(u64::from_str_radix(f[3], 16).unwrap());
                let actual = if op == 0 {
                    parse_int(&s, f[2].parse().unwrap()).unwrap()
                } else {
                    parse_float(&s)
                };
                if op == 0 {
                    let exact = f64::from_bits(u64::from_str_radix(f[4], 16).unwrap());
                    assert!(
                        same(actual, exact),
                        "exact integer: {line}; actual={actual:?}"
                    );
                }
                if op == 0 {
                    let radix: usize = f[2].parse().unwrap();
                    radix_counts[radix] += 1;
                    if !same(actual, expected) {
                        radix_differences[radix] += 1;
                        max_ulp = max_ulp.max(actual.to_bits().abs_diff(expected.to_bits()));
                    }
                }
                !same(actual, expected)
            } else {
                let x = f64::from_bits(u64::from_str_radix(f[1], 16).unwrap());
                let actual = if op < 6 {
                    format_number(
                        op as u32,
                        x,
                        if f[2] == "_" {
                            None
                        } else {
                            Some(f[2].parse().unwrap())
                        },
                    )
                    .unwrap()
                } else {
                    let mut value = 0.0;
                    let mut output = [0u8; 2300];
                    // SAFETY: Predicate dispatch writes only the live numeric result; all buffers cover the callback contract.
                    assert_eq!(
                        unsafe {
                            dispatch(
                                op as u32,
                                x,
                                0.0,
                                0,
                                b"".as_ptr(),
                                0,
                                output.as_mut_ptr(),
                                &mut value,
                            )
                        },
                        0
                    );
                    (value != 0.0).to_string()
                };
                let expected = unhex(f[3]);
                if actual != expected && failures.len() < 12 {
                    failures.push(format!(
                        "op {op} x={x:?} arg={} actual={actual} expected={expected}",
                        f[2]
                    ));
                }
                actual != expected
            };
            if mismatch {
                differences[op] += 1;
            }
        }
        eprintln!("Number Node tables counts={counts:?}; differences={differences:?}");
        eprintln!(
            "parseInt by radix counts={radix_counts:?}; differences={radix_differences:?}; maximum ulp={max_ulp}"
        );
        for radix in [2, 4, 8, 10, 16, 32] {
            assert_eq!(radix_differences[radix], 0);
        }
        assert!(counts.iter().all(|&c| c >= 10000));
        assert_eq!(&differences[1..], &[0; 9], "{}", failures.join("\n"));
    }
    #[test]
    fn number_range_and_prefix_edges() {
        for op in 2..=5 {
            for d in [f64::INFINITY, f64::NEG_INFINITY, -1.0, 101.0] {
                assert!(format_number(op, 1.0, Some(d)).is_err(), "{op} {d}");
            }
        }
        assert!(format_number(4, 1.0, Some(0.0)).is_err());
        assert_eq!(format_number(2, -0.0, None).unwrap(), "0");
        assert_eq!(format_number(2, 2.5, Some(0.0)).unwrap(), "3");
        assert_eq!(format_number(2, 1.25, Some(1.9)).unwrap(), "1.3");
        assert_eq!(format_number(2, 1.0, Some(f64::NAN)).unwrap(), "1");
        for r in [0.0, 1.0, 37.0, f64::NAN, f64::INFINITY] {
            assert!(parse_int("1", r).is_err());
        }
        assert_eq!(parse_int("\u{feff}-0xFFz", 4294967312.9).unwrap(), -255.0);
        assert!(parse_float("\u{85}1").is_nan());
        for (s, x) in [
            ("1e+", 1.0),
            ("-.5e-2x", -0.005),
            ("+Infinityx", f64::INFINITY),
            ("0x10", 0.0),
        ] {
            assert!(same(parse_float(s), x));
        }
    }
}

#[cfg(test)]
#[link(name = "ulua_test_native", kind = "static")]
unsafe extern "C" {
    fn ulua_internal_probe_native_bytes_per_step() -> u64;
    fn ulua_internal_probe_native_call_steps() -> u64;
    fn ulua_internal_probe_number_charge(
        budget: u64,
        formatting: i32,
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
#[cfg(test)]
#[test]
fn number_native_and_format_charge_budget_edges() {
    use std::sync::atomic::{AtomicU64, Ordering};
    static CALLS: AtomicU64 = AtomicU64::new(0);
    unsafe extern "C" fn counted(
        op: u32,
        a: f64,
        b: f64,
        has_b: i32,
        input: *const u8,
        len: usize,
        out: *mut u8,
        num: *mut f64,
    ) -> i32 {
        CALLS.fetch_add(1, Ordering::Relaxed);
        // SAFETY: Forward the native probe's live buffers without altering their lengths.
        unsafe { dispatch(op, a, b, has_b, input, len, out, num) }
    }
    // SAFETY: The getters have no pointer arguments or side effects.
    let parse_charge = unsafe {
        ulua_internal_probe_native_call_steps() + 32 / ulua_internal_probe_native_bytes_per_step()
    };
    // Native base plus the measured output/exponent formula for 2.5.toFixed(100).
    let format_charge = unsafe { ulua_internal_probe_native_call_steps() }
        + format_steps(
            2.5,
            Some(100.0),
            &format_number(2, 2.5, Some(100.0)).unwrap(),
        );
    for (formatting, charge) in [(0, parse_charge), (1, format_charge)] {
        for (budget, status, completed) in [
            (1, 2, 0),
            (charge, 2, 0),
            (charge + 1, 2, 1),
            (3 * (charge + 1), 2, 3),
            (3 * (charge + 1) + 1, 0, 3),
        ] {
            CALLS.store(0, Ordering::Relaxed);
            let mut counters = [0; 4];
            let mut error = [0; 256];
            // SAFETY: Arrays cover the fixed outputs; counted has the native callback signature.
            let actual = unsafe {
                ulua_internal_probe_number_charge(
                    budget,
                    formatting,
                    counted,
                    counters.as_mut_ptr(),
                    error.as_mut_ptr(),
                )
            };
            assert_eq!(
                actual, status,
                "format={formatting} budget={budget}: {counters:?}"
            );
            let expected_dispatches = if formatting != 0 && budget == charge {
                1
            } else {
                completed
            };
            assert_eq!(
                CALLS.load(Ordering::Relaxed),
                expected_dispatches,
                "{budget}"
            );
            assert_eq!(counters[3], completed);
            assert_eq!(counters[0], budget);
        }
    }
}

#[cfg(test)]
#[test]
fn number_format_charge_model_edges() {
    assert_eq!(format_steps(1.0, Some(0.0), "1"), 35);
    assert_eq!(format_steps(-0.0, None, "0"), 35);
    assert_eq!(format_steps(f64::INFINITY, Some(20.0), "Infinity"), 56);
    let binary = format_number(5, f64::from_bits(1), Some(2.0)).unwrap();
    assert_eq!(binary.len(), 1076);
    assert_eq!(format_steps(f64::from_bits(1), Some(2.0), &binary), 3797);
    let fixed = format_number(2, 2.5, Some(100.0)).unwrap();
    assert_eq!(format_steps(2.5, Some(100.0), &fixed), 339);
}
