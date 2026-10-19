//! Deterministic diagnostics with byte-based source columns.
use std::ffi::c_char;

/// Collision titles and descriptions for all product diagnostic codes.
pub(crate) const RULES: &[(&str, &str)] = &[
    ("C1", "Value representation"),
    ("C2", "Absence — `T | null` only"),
    ("C3", "Truth tests are `boolean`"),
    ("C4", "Numbers"),
    ("C5", "Strings"),
    ("C6", "Arrays"),
    ("C7", "Equality"),
    ("C8", "Forms outside the subset"),
    ("C9", "Loop bindings"),
    ("C10", "Declaration order"),
    ("C11", "The accepted surface is closed"),
    ("C12", "Evaluation order"),
    ("C13", "Construction"),
    ("C14", "No metatables; literal values"),
    ("C15", "Luau compiler limits"),
    ("C16", "Narrowing"),
    ("C17", "Exceptions"),
    ("C18", "Host functions"),
    ("C19", "Async functions and promises"),
    ("C20", "JSON"),
    (
        "syntax",
        "the source is not valid UTF-8 or valid TypeScript syntax",
    ),
    ("parse", "a JSON string does not follow the JSON grammar"),
    (
        "unsupported",
        "the source uses a form outside the accepted subset",
    ),
    ("internal", "the library detected an implementation defect"),
    (
        "async:pending",
        "the main promise has no settled result after the job drain",
    ),
    ("limit:arguments", "a call has too many arguments"),
    ("limit:fields", "a record or class has too many fields"),
    (
        "limit:json-depth",
        "a JSON value exceeds the permitted container depth",
    ),
    (
        "limit:memory",
        "the instance exhausted its permitted heap bytes",
    ),
    (
        "limit:nesting-depth",
        "the source exceeds the permitted syntax nesting depth",
    ),
    (
        "limit:output",
        "the generated module exceeds its permitted output size",
    ),
    ("limit:parameters", "a function has too many parameters"),
    ("limit:requests", "too many host requests remain open"),
    (
        "limit:source-bytes",
        "the source file exceeds the permitted byte count",
    ),
    (
        "limit:stack",
        "the script exhausted the permitted call stack",
    ),
    ("limit:steps", "the script exhausted its instruction budget"),
    (
        "limit:template-parts",
        "a template literal has too many parts",
    ),
    (
        "limit:temporaries",
        "an expression needs too many temporary registers",
    ),
    (
        "limit:type-depth",
        "a type exceeds the permitted structural depth",
    ),
    ("limit:upvalues", "a closure captures too many bindings"),
    (
        "trap:array-modified",
        "an array changed during traversal (C6.9)",
    ),
    ("trap:index-out-of-range", "an index outside the array (C6)"),
    (
        "trap:invalid-argument",
        "a string operation received an invalid argument (C5)",
    ),
    (
        "trap:json-number",
        "a JSON conversion encountered a non-finite number (C20)",
    ),
    (
        "trap:map-modified",
        "a map or set changed during traversal (C6.10)",
    ),
    (
        "trap:string-boundary",
        "a string position splits a UTF-8 scalar (C5)",
    ),
    (
        "trap:uncaught-exception",
        "an exception or promise rejection escaped its handler (C17.8, C19.7)",
    ),
    (
        "host:capability",
        "a granted host function reported failure",
    ),
    (
        "host:output",
        "the host output callback could not write a line",
    ),
    (
        "host:request",
        "an asynchronous host request reported failure",
    ),
    (
        "host:resources",
        "the OS refused a compiler thread or its resources",
    ),
    (
        "host:result",
        "a host function returned an invalid ABI value",
    ),
    (
        "host:revoked",
        "a script called a capability that the host revoked",
    ),
];
/// Read the bytes before the first NUL in a diagnostic field.
pub(crate) fn cstring(bytes: &[c_char]) -> String {
    String::from_utf8_lossy(
        &bytes
            .iter()
            .take_while(|b| **b != 0)
            .map(|b| *b as u8)
            .collect::<Vec<_>>(),
    )
    .into_owned()
}
// Unicode 17.0.0, General_Category=Cf (21 ranges, 170 code points).
// Source: https://www.unicode.org/Public/17.0.0/ucd/extracted/DerivedGeneralCategory.txt
const FORMAT_RANGES: &[(u32, u32)] = &[
    (0x00ad, 0x00ad),
    (0x0600, 0x0605),
    (0x061c, 0x061c),
    (0x06dd, 0x06dd),
    (0x070f, 0x070f),
    (0x0890, 0x0891),
    (0x08e2, 0x08e2),
    (0x180e, 0x180e),
    (0x200b, 0x200f),
    (0x202a, 0x202e),
    (0x2060, 0x2064),
    (0x2066, 0x206f),
    (0xfeff, 0xfeff),
    (0xfff9, 0xfffb),
    (0x110bd, 0x110bd),
    (0x110cd, 0x110cd),
    (0x13430, 0x1343f),
    (0x1bca0, 0x1bca3),
    (0x1d173, 0x1d17a),
    (0xe0001, 0xe0001),
    (0xe0020, 0xe007f),
];
/// Escape terminal controls and format characters, with source tabs as an exception.
pub(crate) fn escape(text: &str, source: bool) -> String {
    let mut out = String::new();
    for c in text.chars() {
        if (c.is_control()
            || FORMAT_RANGES
                .iter()
                .any(|&(first, last)| (first..=last).contains(&(c as u32))))
            && !(source && c == '\t')
        {
            use std::fmt::Write;
            let _ = write!(out, "\\u{{{:02x}}}", c as u32);
        } else {
            out.push(c);
        }
    }
    out
}
/// Render an operating-system file name with the diagnostic escape rules.
pub(crate) fn filename(name: &std::ffi::OsStr) -> String {
    escape(&name.to_string_lossy(), false)
}

// Keep original byte offsets. Each invalid byte is one replacement character, even within one invalid chunk.
fn source_chars(source: &[u8]) -> impl Iterator<Item = (usize, char)> + '_ {
    let mut offset = 0;
    source.utf8_chunks().flat_map(move |chunk| {
        let start = offset;
        let valid = chunk.valid();
        let invalid_start = start + valid.len();
        offset = invalid_start + chunk.invalid().len();
        valid
            .char_indices()
            .map(move |(index, c)| (start + index, c))
            .chain((0..chunk.invalid().len()).map(move |index| (invalid_start + index, '\u{fffd}')))
    })
}
fn source_line(source: &[u8], wanted: u32) -> Option<&[u8]> {
    if wanted == 0 {
        return None;
    }
    let mut line = 1;
    let mut start = 0;
    let mut chars = source_chars(source).peekable();
    while let Some((index, c)) = chars.next() {
        if matches!(c, '\n' | '\r' | '\u{2028}' | '\u{2029}') {
            if line == wanted {
                return Some(&source[start..index]);
            }
            start = index + c.len_utf8();
            if c == '\r' && chars.peek().is_some_and(|(_, next)| *next == '\n') {
                chars.next();
                start += 1;
            }
            line += 1;
        }
    }
    (line == wanted).then_some(&source[start..])
}
fn window(text: &[u8], column: u32) -> (String, String) {
    let count = source_chars(text).count();
    let caret = source_chars(text)
        .take_while(|(i, _)| *i < column.saturating_sub(1) as usize)
        .count();
    // Select at most 120 source characters BEFORE escapes. Escapes can expand the printed width.
    let start = caret.saturating_sub(60).min(count.saturating_sub(120));
    let end = (start + 120).min(count);
    let mut shown = if start > 0 {
        "...".to_string()
    } else {
        String::new()
    };
    let mut prefix = " ".repeat(shown.len());
    for (index, (_, c)) in source_chars(text).enumerate().skip(start).take(end - start) {
        let escaped = escape(&c.to_string(), true);
        if index < caret {
            if c == '\t' {
                prefix.push('\t');
            } else {
                prefix.push_str(&" ".repeat(escaped.chars().count()));
            }
        }
        shown.push_str(&escaped);
    }
    if end < count {
        shown.push_str("...");
    }
    (shown, prefix)
}
/// Render a diagnostic from raw source bytes with a bounded window and escaped script text.
pub(crate) fn render(
    code: &str,
    message: &str,
    file: &str,
    source: impl AsRef<[u8]>,
    line: u32,
    column: u32,
) -> String {
    let mut out = format!("error[{code}]: {}\n", escape(message, false));
    let width = line.to_string().len();
    if let Some(text) = source_line(source.as_ref(), line) {
        let (text, prefix) = window(text, column);
        out.push_str(&format!("{:width$}--> {}:{line}:{column}\n {:width$}|\n{line:>width$} | {text}\n {:width$}| {prefix}^\n", "", escape(file, false), "", ""));
    }
    if let Some((_, title)) = RULES.iter().find(|(key, _)| *key == code) {
        let rule = if code.starts_with('C') {
            format!("{code} {title}")
        } else {
            title.to_string()
        };
        out.push_str(&format!(" {:width$}= rule: {rule}\n", ""));
    }
    out
}
