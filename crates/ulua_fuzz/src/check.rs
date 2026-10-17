//! Failures, diagnostic decoding, and the documented statuses and codes of abi.md.
use std::ffi::c_char;
use ulua::*;

/// One property failure of one input. `key` names the property and the observed fact;
/// a reduced input keeps the failure when it gives the same key.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Failure {
    /// A short stable name: the property, the call, and the observed status or code.
    pub key: String,
    /// The full observation.
    pub detail: String,
}

/// The failures that one input gives.
#[derive(Default, Debug)]
pub struct Report {
    /// Each failure, in observation order.
    pub failures: Vec<Failure>,
}

impl Report {
    /// Record one failure.
    pub fn fail(&mut self, key: impl Into<String>, detail: impl Into<String>) {
        self.failures.push(Failure {
            key: key.into(),
            detail: detail.into(),
        });
    }
    /// True when no failure is recorded.
    #[cfg(test)]
    pub fn clean(&self) -> bool {
        self.failures.is_empty()
    }
}

thread_local! {
    static STATS: std::cell::RefCell<std::collections::BTreeMap<String, u64>> =
        const { std::cell::RefCell::new(std::collections::BTreeMap::new()) };
}

/// Count one observation (a call and its status), so a clean run shows what it covered.
pub fn note(key: impl Into<String>) {
    STATS.with(|s| *s.borrow_mut().entry(key.into()).or_default() += 1);
}

/// Take the observation counts of this thread.
pub fn take_stats() -> std::collections::BTreeMap<String, u64> {
    STATS.with(|s| std::mem::take(&mut *s.borrow_mut()))
}

/// The name of a status.
pub fn status_name(s: Status) -> &'static str {
    match s {
        ULUA_OK => "OK",
        ULUA_INVALID_ARGUMENT => "INVALID_ARGUMENT",
        ULUA_REJECTED => "REJECTED",
        ULUA_FAILED => "FAILED",
        ULUA_HOST_ERROR => "HOST_ERROR",
        ULUA_BUSY => "BUSY",
        ULUA_INTERNAL => "INTERNAL",
        ULUA_POISONED => "POISONED",
        ULUA_PENDING => "PENDING",
        _ => "UNKNOWN",
    }
}

/// A diagnostic with a sentinel code, so a write of the library is visible.
pub fn diagnostic() -> Diagnostic {
    let mut d = Diagnostic::default();
    for (o, b) in d.code.iter_mut().zip(b"unset") {
        *o = *b as c_char;
    }
    d
}

/// The decoded fields of a diagnostic.
#[derive(Clone, Debug)]
pub struct Decoded {
    /// The code text.
    pub code: String,
    /// The message text.
    pub message: String,
    /// The source offset or `u64::MAX`.
    pub offset: u64,
    /// The line or 0.
    pub line: u32,
    /// The column or 0.
    pub column: u32,
}

/// Decode a diagnostic. A missing terminator or a byte outside the format is an error.
pub fn decode(d: &Diagnostic) -> Result<Decoded, String> {
    let code: Vec<u8> = d.code.iter().map(|c| *c as u8).collect();
    let message: Vec<u8> = d.message.iter().map(|c| *c as u8).collect();
    let end = code
        .iter()
        .position(|b| *b == 0)
        .ok_or("code has no terminator")?;
    let code = &code[..end];
    if code.is_empty() || !code.iter().all(|b| b.is_ascii_graphic()) {
        return Err(format!("code is not ASCII text: {code:?}"));
    }
    let mend = message
        .iter()
        .position(|b| *b == 0)
        .ok_or("message has no terminator")?;
    let message =
        std::str::from_utf8(&message[..mend]).map_err(|e| format!("message is not UTF-8: {e}"))?;
    Ok(Decoded {
        code: String::from_utf8_lossy(code).into_owned(),
        message: message.to_owned(),
        offset: d.offset,
        line: d.line,
        column: d.column,
    })
}

/// The trap kinds of corpus.md §6.
pub const TRAPS: &[&str] = &[
    "trap:index-out-of-range",
    "trap:array-modified",
    "trap:map-modified",
    "trap:string-boundary",
    "trap:invalid-argument",
    "trap:uncaught-exception",
];

/// The C15 bounds of pipeline.md §6 and the two input limits.
pub const COMPILE_LIMITS: &[&str] = &[
    "limit:source-bytes",
    "limit:nesting-depth",
    "limit:parameters",
    "limit:arguments",
    "limit:template-parts",
    "limit:temporaries",
    "limit:upvalues",
    "limit:fields",
    "limit:type-depth",
    "limit:output",
];

/// The rule ids that `specs/blocks/collisions.md` defines.
pub fn rule_ids(collisions: &str) -> Vec<String> {
    collisions
        .lines()
        .filter_map(|l| l.strip_prefix("## C"))
        .filter_map(|l| l.split_once('.'))
        .filter(|(n, _)| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
        .map(|(n, _)| format!("C{n}"))
        .collect()
}

/// A compile diagnostic code of pipeline.md §3.7.
pub fn compile_code(code: &str, rules: &[String]) -> bool {
    matches!(code, "syntax" | "unsupported")
        || COMPILE_LIMITS.contains(&code)
        || rules.iter().any(|r| r == code)
}

/// The `ULUA_FAILED` codes of a run (abi.md §3, §4.7, §6.4).
pub fn run_failed_code(code: &str) -> bool {
    TRAPS.contains(&code)
        || matches!(
            code,
            "limit:memory" | "limit:steps" | "limit:stack" | "limit:requests" | "async:pending"
        )
}

/// The `ULUA_HOST_ERROR` codes (abi.md §3, §6).
pub fn host_code(code: &str) -> bool {
    matches!(
        code,
        "host:output" | "host:capability" | "host:revoked" | "host:result" | "host:request"
    )
}

/// The line and column of a byte offset, with the ECMAScript line terminators
/// (LF, CR, CRLF, U+2028, U+2029). Derived here, apart from the compiler.
pub fn line_column(source: &[u8], offset: usize) -> (u32, u32) {
    let (mut line, mut start, mut i) = (1u32, 0usize, 0usize);
    while i < offset.min(source.len()) {
        let b = source[i];
        let width = if b == b'\r' && source.get(i + 1) == Some(&b'\n') && i + 1 < offset {
            2
        } else if b == b'\n' || b == b'\r' {
            1
        } else if source[i..].starts_with("\u{2028}".as_bytes())
            || source[i..].starts_with("\u{2029}".as_bytes())
        {
            3
        } else {
            0
        };
        if width > 0 && i + width <= offset {
            line += 1;
            i += width;
            start = i;
        } else {
            i += 1;
        }
    }
    (line, (offset - start + 1) as u32)
}

/// Check the diagnostic fields that every written diagnostic carries. A run diagnostic
/// has offset `UINT64_MAX` (abi.md §3) and names no chunk of the generated Luau.
pub fn run_diagnostic(report: &mut Report, call: &str, d: &Decoded) {
    if d.offset != u64::MAX || d.line != 0 || d.column != 0 {
        report.fail(
            format!("run-position:{call}:{}", d.code),
            format!("{call}: run diagnostic {d:?} has a source position"),
        );
    }
    if d.code != "trap:uncaught-exception" && d.message.contains("[string") {
        report.fail(
            format!("run-message:{call}:{}", d.code),
            format!("{call}: run diagnostic names a chunk: {d:?}"),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn decode_rejects_unterminated_and_non_ascii_codes() {
        let mut d = diagnostic();
        assert_eq!(decode(&d).unwrap().code, "unset");
        d.code.fill(b'a' as c_char);
        assert!(decode(&d).is_err());
        let mut d = diagnostic();
        d.code[0] = 0xc3u8 as c_char;
        assert!(decode(&d).is_err());
        let mut d = diagnostic();
        d.message[0] = 0xffu8 as c_char;
        assert!(decode(&d).is_err());
    }
    #[test]
    fn code_sets_follow_the_contract() {
        let rules = rule_ids("## C1. A\n## C19. B\n## Other\n");
        assert_eq!(rules, ["C1", "C19"]);
        assert!(compile_code("C19", &rules) && compile_code("limit:output", &rules));
        assert!(!compile_code("internal", &rules) && !compile_code("C20", &rules));
        assert!(run_failed_code("trap:array-modified") && !run_failed_code("internal"));
        assert!(host_code("host:request") && !host_code("host:resources"));
    }
    #[test]
    fn line_column_counts_each_terminator() {
        let s = "a\nb\r\nc\rd\u{2028}e".as_bytes();
        assert_eq!(line_column(s, 0), (1, 1));
        assert_eq!(line_column(s, 2), (2, 1));
        assert_eq!(line_column(s, 5), (3, 1));
        assert_eq!(line_column(s, 7), (4, 1));
        assert_eq!(line_column(s, s.len() - 1), (5, 1));
        assert_eq!(line_column(b"abc", 2), (1, 3));
    }
    #[test]
    fn a_run_diagnostic_with_a_position_is_a_failure() {
        let mut r = Report::default();
        let d = Decoded {
            code: "limit:steps".into(),
            message: "[string \"x\"]:1: Step limit exceeded".into(),
            offset: 3,
            line: 1,
            column: 4,
        };
        run_diagnostic(&mut r, "ulua_call", &d);
        assert_eq!(r.failures.len(), 2);
    }
}
