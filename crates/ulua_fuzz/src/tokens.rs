//! A total lexer over arbitrary bytes. It gives token spans for mutation and shrinking.
use std::ops::Range;

/// The kind of one token span.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// Spaces, tabs, and line ends.
    Space,
    /// A line or block comment.
    Comment,
    /// An identifier or a keyword.
    Word,
    /// A number literal.
    Number,
    /// A string or template literal.
    Text,
    /// A punctuator or any other byte.
    Punct,
}

/// One token: a kind and a byte range of the input.
#[derive(Clone, Debug)]
pub struct Token {
    /// The token kind.
    pub kind: Kind,
    /// The byte range.
    pub span: Range<usize>,
}

fn word_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_' || b == b'$' || b >= 0x80
}

/// Split bytes into tokens. The spans cover the input without a gap.
pub fn lex(src: &[u8]) -> Vec<Token> {
    let mut out = vec![];
    let mut i = 0;
    while i < src.len() {
        let start = i;
        let b = src[i];
        let kind = if b.is_ascii_whitespace() {
            while i < src.len() && src[i].is_ascii_whitespace() {
                i += 1;
            }
            Kind::Space
        } else if src[i..].starts_with(b"//") {
            while i < src.len() && src[i] != b'\n' {
                i += 1;
            }
            Kind::Comment
        } else if src[i..].starts_with(b"/*") {
            i += 2;
            while i < src.len() && !src[i..].starts_with(b"*/") {
                i += 1;
            }
            i = (i + 2).min(src.len());
            Kind::Comment
        } else if b.is_ascii_digit() {
            while i < src.len() && (src[i].is_ascii_alphanumeric() || src[i] == b'.') {
                i += 1;
            }
            Kind::Number
        } else if word_byte(b) {
            while i < src.len() && word_byte(src[i]) {
                i += 1;
            }
            Kind::Word
        } else if matches!(b, b'\'' | b'"' | b'`') {
            i += 1;
            while i < src.len() && src[i] != b {
                if src[i] == b'\\' {
                    i += 1;
                }
                if b != b'`' && src.get(i) == Some(&b'\n') {
                    break;
                }
                i += 1;
            }
            i = (i + 1).min(src.len());
            Kind::Text
        } else {
            let three = [b"===", b"!==", b"...", b"**=", b"&&=", b"||=", b"??="];
            let two = [
                b"=>", b"==", b"!=", b"<=", b">=", b"&&", b"||", b"??", b"?.", b"++", b"--", b"+=",
                b"-=", b"*=", b"/=", b"%=", b"**",
            ];
            if three.iter().any(|t| src[i..].starts_with(*t)) {
                i += 3;
            } else if two.iter().any(|t| src[i..].starts_with(*t)) {
                i += 2;
            } else {
                i += 1;
            }
            Kind::Punct
        };
        out.push(Token {
            kind,
            span: start..i,
        });
    }
    out
}

/// The tokens that are not space or comment.
pub fn significant(src: &[u8]) -> Vec<Token> {
    lex(src)
        .into_iter()
        .filter(|t| !matches!(t.kind, Kind::Space | Kind::Comment))
        .collect()
}

/// Each balanced bracket span `(..)`, `[..]`, `{..}` among the tokens.
pub fn balanced(src: &[u8], tokens: &[Token]) -> Vec<Range<usize>> {
    let mut stack: Vec<(u8, usize)> = vec![];
    let mut out = vec![];
    for t in tokens {
        if t.kind != Kind::Punct || t.span.len() != 1 {
            continue;
        }
        let b = src[t.span.start];
        match b {
            b'(' | b'[' | b'{' => stack.push((b, t.span.start)),
            b')' | b']' | b'}' => {
                let open = match b {
                    b')' => b'(',
                    b']' => b'[',
                    _ => b'{',
                };
                if let Some(&(o, start)) = stack.last()
                    && o == open
                {
                    stack.pop();
                    out.push(start..t.span.end);
                }
            }
            _ => {}
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn spans_cover_input_and_brackets_balance() {
        let src = b"let x = f('a\\'b', [1, 2]) /* c */ // d\n=> \xff";
        let tokens = lex(src);
        let mut at = 0;
        for t in &tokens {
            assert_eq!(t.span.start, at);
            at = t.span.end;
        }
        assert_eq!(at, src.len());
        let s = significant(src);
        assert!(
            s.iter()
                .any(|t| t.kind == Kind::Text && &src[t.span.clone()] == b"'a\\'b'")
        );
        let spans = balanced(src, &s);
        assert!(spans.iter().any(|r| &src[r.clone()] == b"[1, 2]"));
        assert!(
            spans
                .iter()
                .any(|r| &src[r.clone()] == b"('a\\'b', [1, 2])")
        );
    }
}
