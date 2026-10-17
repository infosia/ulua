//! Bounded JSON grammar cursor. The caller allocates all scratch and values in the VM.
const HEADER: usize = 144;
fn space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\r' | b'\n')
}
fn hex(bytes: &[u8]) -> Result<u32, ()> {
    let mut n = 0;
    if bytes.len() != 4 {
        return Err(());
    }
    for &b in bytes {
        n = n * 16
            + match b {
                b'0'..=b'9' => u32::from(b - b'0'),
                b'a'..=b'f' => u32::from(b - b'a' + 10),
                b'A'..=b'F' => u32::from(b - b'A' + 10),
                _ => return Err(()),
            };
    }
    Ok(n)
}
fn string(input: &[u8], cursor: &mut usize, out: &mut [u8]) -> Result<usize, ()> {
    *cursor += 1;
    let mut len = 0;
    loop {
        let b = *input.get(*cursor).ok_or(())?;
        *cursor += 1;
        if b == b'"' {
            std::str::from_utf8(out.get(..len).ok_or(())?).map_err(|_| ())?;
            return Ok(len);
        }
        if b < 32 {
            return Err(());
        }
        if b != b'\\' {
            *out.get_mut(len).ok_or(())? = b;
            len += 1;
            continue;
        }
        let b = *input.get(*cursor).ok_or(())?;
        *cursor += 1;
        let cp = match b {
            b'"' | b'\\' | b'/' => u32::from(b),
            b'b' => 8,
            b'f' => 12,
            b'n' => 10,
            b'r' => 13,
            b't' => 9,
            b'u' => {
                let mut cp = hex(input.get(*cursor..*cursor + 4).ok_or(())?)?;
                *cursor += 4;
                if (0xd800..=0xdbff).contains(&cp) {
                    if input.get(*cursor..*cursor + 2) != Some(b"\\u") {
                        return Err(());
                    }
                    *cursor += 2;
                    let low = hex(input.get(*cursor..*cursor + 4).ok_or(())?)?;
                    *cursor += 4;
                    if !(0xdc00..=0xdfff).contains(&low) {
                        return Err(());
                    }
                    cp = 0x10000 + ((cp - 0xd800) << 10) + low - 0xdc00;
                }
                cp
            }
            _ => return Err(()),
        };
        let c = char::from_u32(cp).ok_or(())?;
        let mut bytes = [0; 4];
        let encoded = c.encode_utf8(&mut bytes).as_bytes();
        out.get_mut(len..len + encoded.len())
            .ok_or(())?
            .copy_from_slice(encoded);
        len += encoded.len();
    }
}
fn number(input: &[u8], i: &mut usize) -> Result<f64, ()> {
    let start = *i;
    if input.get(*i) == Some(&b'-') {
        *i += 1;
    }
    match input.get(*i) {
        Some(b'0') => *i += 1,
        Some(b'1'..=b'9') => {
            *i += 1;
            while input.get(*i).is_some_and(u8::is_ascii_digit) {
                *i += 1;
            }
        }
        _ => return Err(()),
    }
    if input.get(*i) == Some(&b'.') {
        *i += 1;
        let first = *i;
        while input.get(*i).is_some_and(u8::is_ascii_digit) {
            *i += 1;
        }
        if *i == first {
            return Err(());
        }
    }
    if matches!(input.get(*i), Some(b'e' | b'E')) {
        *i += 1;
        if matches!(input.get(*i), Some(b'+' | b'-')) {
            *i += 1;
        }
        let first = *i;
        while input.get(*i).is_some_and(u8::is_ascii_digit) {
            *i += 1;
        }
        if *i == first {
            return Err(());
        }
    }
    std::str::from_utf8(input.get(start..*i).ok_or(())?)
        .map_err(|_| ())?
        .parse()
        .map_err(|_| ())
}
// Events: null, number, string, boolean, array, object, key, end-container, end-text.
// States 0/1: root value/end; 2/3/4: array first/value/end; 5..9: object key/colon/value/end.
fn next(input: &[u8], i: &mut usize, out: &mut [u8]) -> Result<i32, ()> {
    loop {
        while input.get(*i).is_some_and(|&b| space(b)) {
            *i += 1;
        }
        let top = usize::from(*out.get(129).ok_or(())?);
        if top > 128 {
            return Err(());
        }
        let state = *out.get(top).ok_or(())?;
        let b = input.get(*i).copied();
        match state {
            1 => return if b.is_none() { Ok(8) } else { Err(()) },
            4 | 9 => {
                if b == Some(if state == 4 { b']' } else { b'}' }) {
                    *i += 1;
                    out[129] -= 1;
                    return Ok(7);
                }
                if b != Some(b',') {
                    return Err(());
                }
                *i += 1;
                out[top] = if state == 4 { 3 } else { 6 };
                continue;
            }
            5 | 6 => {
                if state == 5 && b == Some(b'}') {
                    *i += 1;
                    out[129] -= 1;
                    return Ok(7);
                }
                if b != Some(b'"') {
                    return Err(());
                }
                let len = string(input, i, out.get_mut(HEADER..).ok_or(())?)?;
                out[136..144].copy_from_slice(&(len as u64).to_le_bytes());
                out[top] = 7;
                return Ok(6);
            }
            7 => {
                if b != Some(b':') {
                    return Err(());
                }
                *i += 1;
                out[top] = 8;
                continue;
            }
            2 if b == Some(b']') => {
                *i += 1;
                out[129] -= 1;
                return Ok(7);
            }
            0 | 2 | 3 | 8 => {}
            _ => return Err(()),
        }
        out[top] = match state {
            0 => 1,
            2 | 3 => 4,
            _ => 9,
        };
        let event = match b {
            Some(b'[' | b'{') => {
                if top == 128 {
                    return Err(());
                }
                out[129] += 1;
                out[top + 1] = if b == Some(b'[') { 2 } else { 5 };
                *i += 1;
                if b == Some(b'[') { 4 } else { 5 }
            }
            Some(b'"') => {
                let len = string(input, i, out.get_mut(HEADER..).ok_or(())?)?;
                out[136..144].copy_from_slice(&(len as u64).to_le_bytes());
                2
            }
            Some(b'n' | b't' | b'f') => {
                let literal: &[u8] = match b {
                    Some(b'n') => b"null",
                    Some(b't') => b"true",
                    _ => b"false",
                };
                if input.get(*i..*i + literal.len()) != Some(literal) {
                    return Err(());
                }
                *i += literal.len();
                out[136..144]
                    .copy_from_slice(&(if b == Some(b't') { 1u64 } else { 0 }).to_le_bytes());
                if b == Some(b'n') { 0 } else { 3 }
            }
            Some(b'-' | b'0'..=b'9') => {
                let n = number(input, i)?;
                // Finish the grammar before reporting a nonfinite number as a type error.
                if !n.is_finite() {
                    out[130] = 1;
                }
                out[136..144].copy_from_slice(&n.to_bits().to_le_bytes());
                1
            }
            _ => return Err(()),
        };
        return Ok(event);
    }
}
// SAFETY: The shim provides input[len] and a VM buffer[len+144]. No VM call enters Rust.
pub(crate) unsafe fn dispatch(
    input: *const u8,
    len: usize,
    cursor: f64,
    output: *mut u8,
    consumed: *mut f64,
) -> i32 {
    // SAFETY: The private dispatcher supplies these live buffers for the call.
    let (input, out) = unsafe {
        (
            std::slice::from_raw_parts(input, len),
            std::slice::from_raw_parts_mut(output, len + HEADER),
        )
    };
    let mut i = cursor as usize;
    let event = next(input, &mut i, out).unwrap_or(-1);
    // SAFETY: The dispatcher provides one writable numeric result.
    unsafe {
        consumed.write(i as f64);
    }
    event
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn grammar_events_and_surrogate_validation() {
        for s in [
            r#"{"x":[null,true,false,-0,1.25e2,"\ud83d\ude00"]}"#,
            "[]",
            "{}",
            " 42 \n",
        ] {
            let mut out = vec![0; s.len() + HEADER];
            let mut i = 0;
            while next(s.as_bytes(), &mut i, &mut out).unwrap() != 8 {}
        }
        for s in [
            "",
            "[1,]",
            "{\"x\":}",
            "01",
            "1.",
            "1e+",
            "true x",
            r#""\ud800""#,
            r#""\udc00""#,
            r#""\ud800\u0041""#,
        ] {
            let mut out = vec![0; s.len() + HEADER];
            let mut i = 0;
            loop {
                match next(s.as_bytes(), &mut i, &mut out) {
                    Err(()) => break,
                    Ok(8) => panic!("accepted {s}"),
                    _ => {}
                }
            }
        }
    }
}
