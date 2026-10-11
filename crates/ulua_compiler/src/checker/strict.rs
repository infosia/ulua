//! Strict module errors that the pinned parser does not report.

pub(super) fn binding(name: &str) -> bool {
    matches!(
        name,
        "eval"
            | "arguments"
            | "await"
            | "implements"
            | "interface"
            | "let"
            | "package"
            | "private"
            | "protected"
            | "public"
            | "static"
            | "yield"
    )
}

pub(super) fn legacy_number(raw: &[u8]) -> bool {
    raw.first() == Some(&b'0') && raw.get(1).is_some_and(u8::is_ascii_digit)
}

pub(super) fn legacy_escape(raw: &[u8]) -> bool {
    let mut i = 0;
    while i < raw.len() {
        if raw[i] == b'\\' {
            i += 1;
            if let Some(&escaped) = raw.get(i) {
                if matches!(escaped, b'1'..=b'9')
                    || (escaped == b'0' && raw.get(i + 1).is_some_and(u8::is_ascii_digit))
                {
                    return true;
                }
                // An escaped backslash cannot start another escape.
                i += 1;
            }
        } else {
            i += 1;
        }
    }
    false
}
