//! TypeScript subset checks and typed Luau emission.
#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod checker;
mod depth;
mod emitter;
mod ir;

use oxc_allocator::Allocator;
use oxc_parser::{ParseOptions, Parser};
use oxc_span::SourceType;

/// Source ceiling in bytes.
pub const SOURCE_CEILING: u64 = 1024 * 1024; // provisional: S3 measures
/// AST and parser depth ceiling.
pub const DEPTH_CEILING: u32 = 128; // provisional: S3 measures
/// Stack size for the complete compiler pipeline.
pub const COMPILER_STACK_BYTES: usize = 8 * 1024 * 1024; // provisional: S3 measures

/// Host limits for the safe compiler stages.
#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub struct Limits {
    /// Maximum source length.
    pub max_source_bytes: u64,
    /// Maximum parser and AST depth.
    pub max_nesting_depth: u32,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            max_source_bytes: SOURCE_CEILING,
            max_nesting_depth: DEPTH_CEILING,
        }
    }
}

/// The first compiler violation.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct Diagnostic {
    /// Stable diagnostic code.
    pub code: &'static str,
    /// Source byte offset.
    pub offset: u64,
    /// One-based source line.
    pub line: u32,
    /// One-based byte column.
    pub column: u32,
    /// Human-readable message.
    pub message: String,
}
impl Diagnostic {
    pub(crate) fn at(
        source: &[u8],
        offset: usize,
        code: &'static str,
        message: impl Into<String>,
    ) -> Self {
        let offset = offset.min(source.len());
        let mut line = 1;
        let mut column = 1;
        let mut i = 0;
        while i < offset {
            match source[i] {
                b'\r' => {
                    line += 1;
                    column = 1;
                    if i + 1 < offset && source[i + 1] == b'\n' {
                        i += 1;
                    }
                }
                b'\n' => {
                    line += 1;
                    column = 1;
                }
                0xe2 if i + 2 < offset
                    && source[i + 1] == 0x80
                    && matches!(source[i + 2], 0xa8 | 0xa9) =>
                {
                    line += 1;
                    column = 1;
                    i += 2;
                }
                _ => {
                    column += 1;
                }
            }
            i += 1;
        }
        Self {
            code,
            offset: offset as u64,
            line,
            column,
            message: message.into(),
        }
    }
}

/// An export signature derived by the checker.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct Export {
    /// The TypeScript export name.
    pub name: String,
    /// ABI parameter type tags.
    pub parameters: Vec<u32>,
    /// ABI result type tag.
    pub result: u32,
}

/// Internal source and signatures for the runtime and corpus witness.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct Compiled {
    /// Strict, generated Luau source.
    pub luau: String,
    /// Export signatures in source order.
    pub exports: Vec<Export>,
}

/// Parse, bound, check, and emit one module.
pub fn compile(source: &[u8], limits: Limits) -> Result<Compiled, Diagnostic> {
    let max_source = limits.max_source_bytes.min(SOURCE_CEILING);
    if source.len() as u64 > max_source {
        return Err(Diagnostic::at(
            source,
            max_source as usize,
            "limit:source-bytes",
            "Source byte limit exceeded",
        ));
    }
    let text = std::str::from_utf8(source)
        .map_err(|e| Diagnostic::at(source, e.valid_up_to(), "syntax", "Source is not UTF-8"))?;
    let max_depth = limits.max_nesting_depth.min(DEPTH_CEILING);
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, text, SourceType::ts())
        .with_options(ParseOptions {
            max_nesting_depth: Some(max_depth as usize),
            ..ParseOptions::default()
        })
        .parse();
    if let Some(offset) = parsed.depth_exceeded {
        return Err(Diagnostic::at(
            source,
            offset as usize,
            "limit:nesting-depth",
            "Parser depth limit exceeded",
        ));
    }
    if let Some(error) = parsed.diagnostics.first() {
        let offset = error.labels.first().map_or(0, |label| label.offset());
        return Err(Diagnostic::at(
            source,
            offset as usize,
            "syntax",
            error.to_string(),
        ));
    }
    depth::check(&parsed.program, max_depth, source)?;
    let module = checker::check(&parsed.program, source)?;
    Ok(Compiled {
        luau: emitter::emit(&module),
        exports: module
            .functions
            .iter()
            .map(|f| Export {
                name: f.name.clone(),
                parameters: vec![],
                result: 0,
            })
            .collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    const HELLO: &[u8] = b"export function main(): void { print(\"hello\"); }";
    #[test]
    fn compile_a01_and_reject_other_shapes() {
        let result = compile(HELLO, Limits::default()).unwrap();
        assert!(result.luau.starts_with("--!strict\n"));
        assert_eq!(result.exports[0].name, "main");
        for source in [
            b"".as_slice(),
            b"export function main(): void {}",
            b"export function main(): void { print(1); }",
            b"export async function main(): void { print('x'); }",
            b"export function main<T>(): void { print('x'); }",
        ] {
            assert_eq!(
                compile(source, Limits::default()).unwrap_err().code,
                "unsupported"
            );
        }
    }
    #[test]
    fn limits_and_locations() {
        let e = compile(b"\n\xff", Limits::default()).unwrap_err();
        assert_eq!((e.code, e.offset, e.line, e.column), ("syntax", 1, 2, 1));
        let e = compile(
            HELLO,
            Limits {
                max_source_bytes: 0,
                ..Limits::default()
            },
        )
        .unwrap_err();
        assert_eq!(e.code, "limit:source-bytes");
        assert_eq!(
            compile(
                HELLO,
                Limits {
                    max_nesting_depth: 0,
                    ..Limits::default()
                }
            )
            .unwrap_err()
            .code,
            "limit:nesting-depth"
        );
        assert_eq!(
            compile(b"export function {", Limits::default())
                .unwrap_err()
                .code,
            "syntax"
        );
        for chain in [
            "1+".repeat(160) + "1",
            "f".to_string() + &"()".repeat(160),
            "a".to_string() + &".b".repeat(160),
        ] {
            let source = format!("export function main(): void {{ print({chain}); }}");
            assert_eq!(
                compile(source.as_bytes(), Limits::default())
                    .unwrap_err()
                    .code,
                "limit:nesting-depth"
            );
        }
        let source = format!(
            "export function main(): void {{ print({}'x'{}); }}",
            "(".repeat(160),
            ")".repeat(160)
        );
        assert_eq!(
            compile(source.as_bytes(), Limits::default())
                .unwrap_err()
                .code,
            "limit:nesting-depth"
        );
    }
    #[test]
    fn decoded_strings() {
        let result = compile(
            r#"export function main(): void { print("\x00\r\n\"\\é"); }"#.as_bytes(),
            Limits::default(),
        )
        .unwrap();
        assert!(result.luau.contains(r#""\000\013\010\034\092\195\169""#));
        for value in [r"\uD800", r"\u{D800}"] {
            let source = format!("export function main(): void {{ print(\"{value}\"); }}");
            assert_eq!(
                compile(source.as_bytes(), Limits::default())
                    .unwrap_err()
                    .code,
                "C5"
            );
        }
    }
}
