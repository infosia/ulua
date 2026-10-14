//! Source mutations of pipeline.md §9.2: bytes, tokens, subtrees, splices, nesting,
//! invalid UTF-8, lone surrogate escapes, and sizes up to the source ceiling.
use crate::{
    rng::Rng,
    seeds::Pool,
    tokens::{Kind, balanced, significant},
};

const DICTIONARY: &[&str] = &[
    "(",
    ")",
    "[",
    "]",
    "{",
    "}",
    ";",
    ",",
    ".",
    ":",
    "?",
    "?.",
    "??",
    "=",
    "==",
    "===",
    "!==",
    "=>",
    "+",
    "-",
    "*",
    "/",
    "%",
    "**",
    "!",
    "&&",
    "||",
    "<",
    ">",
    "...",
    "`",
    "${",
    "'",
    "\"",
    "let",
    "const",
    "function",
    "return",
    "if",
    "else",
    "for",
    "while",
    "of",
    "in",
    "new",
    "class",
    "this",
    "extends",
    "super",
    "try",
    "catch",
    "finally",
    "throw",
    "async",
    "await",
    "export",
    "import",
    "type",
    "interface",
    "null",
    "undefined",
    "any",
    "unknown",
    "never",
    "number",
    "string",
    "boolean",
    "void",
    "Promise",
    "Error",
    "typeof",
    "instanceof",
    "delete",
    "yield",
    "switch",
    "case",
    "break",
    "continue",
    "print",
    "Utf8",
    "hostAdd",
    "hostLater",
    "x",
    "0",
    "1",
    "-0",
    "1e308",
    "NaN",
    "Infinity",
    "0x7fffffff",
    "2147483648",
    "9007199254740993",
    "1e-324",
    "'\\u{10FFFF}'",
    "'\\0'",
    "[]",
    "{}",
    "()",
    "<number>",
    "as",
    "!",
    "#x",
    "@d",
];

const NUMBERS: &[&str] = &[
    "0",
    "-0",
    "1e308",
    "1e309",
    "5e-324",
    "0.1",
    "4294967296",
    "2147483647",
    "-2147483648",
    "9007199254740992",
    "1e21",
    "0b101",
    "0o17",
    "0xff",
    "1_000",
    ".5",
    "5.",
    "00",
    "08",
];

const INVALID_UTF8: &[&[u8]] = &[
    b"\xff",
    b"\x80",
    b"\xc0\xaf",
    b"\xe2\x82",
    b"\xed\xa0\x80",
    b"\xf4\x90\x80\x80",
    b"\xf0\x9f\x99",
    b"\xc3",
];

const SURROGATES: &[&str] = &[
    "\\uD800",
    "\\uDFFF",
    "\\uD83D",
    "\\u{D800}",
    "\\uDC00\\uD800",
    "\\uD83D\\uDE42",
];

fn insert(src: &mut Vec<u8>, at: usize, bytes: &[u8]) {
    let at = at.min(src.len());
    src.splice(at..at, bytes.iter().copied());
}

/// Apply one to four mutations to a seed program.
pub fn mutate(pool: &Pool, r: &mut Rng) -> Vec<u8> {
    let mut src = pool.base(r);
    // A quarter of the inputs keep the seed program, and half of the rest take one
    // mutation, so the checker and the emitter see accepted programs.
    let count = match r.below(8) {
        0 | 1 => 0,
        2..=4 => 1,
        _ => r.range(2, 4),
    };
    for _ in 0..count {
        one(pool, r, &mut src);
    }
    src
}

fn one(pool: &Pool, r: &mut Rng, src: &mut Vec<u8>) {
    let toks = significant(src);
    let at = r.below(src.len() + 1);
    match r.below(15) {
        0 if !src.is_empty() => {
            let i = r.below(src.len());
            src[i] ^= 1 << r.below(8);
        }
        1 => {
            let b = *r.pick(b"{}()[];,.:=<>+-*/!?&|'\"`$\\ \n\x00\x80\xff");
            insert(src, at, &[b]);
        }
        2 if !src.is_empty() => {
            let n = r.range(1, 8).min(src.len() - at.min(src.len()));
            let at = at.min(src.len());
            src.drain(at..at + n);
        }
        3 => {
            let word = if !toks.is_empty() && r.one_in(2) {
                src[r.pick(&toks).span.clone()].to_vec()
            } else {
                r.pick(DICTIONARY).as_bytes().to_vec()
            };
            let at = toks
                .get(r.below(toks.len() + 1))
                .map_or(src.len(), |t| t.span.start);
            let mut w = word;
            w.push(b' ');
            insert(src, at, &w);
        }
        4 if !toks.is_empty() => {
            let t = r.pick(&toks).span.clone();
            src.drain(t);
        }
        5 if toks.len() > 1 => {
            let mut a = r.pick(&toks).span.clone();
            let mut b = r.pick(&toks).span.clone();
            if a.start > b.start {
                std::mem::swap(&mut a, &mut b);
            }
            if a.end <= b.start {
                let (x, y) = (src[a.clone()].to_vec(), src[b.clone()].to_vec());
                src.splice(b, x);
                src.splice(a, y);
            }
        }
        6 => {
            // A subtree copied in place of another.
            let spans = balanced(src, &toks);
            if spans.len() > 1 {
                let from = src[r.pick(&spans).clone()].to_vec();
                let to = r.pick(&spans).clone();
                src.splice(to, from);
            }
        }
        7 => {
            // A splice: a prefix of this program and a suffix of another, at token starts.
            let other = pool.base(r);
            let other_toks = significant(&other);
            let cut = toks
                .get(r.below(toks.len() + 1))
                .map_or(src.len(), |t| t.span.start);
            let from = other_toks
                .get(r.below(other_toks.len() + 1))
                .map_or(other.len(), |t| t.span.start);
            src.truncate(cut);
            src.extend_from_slice(&other[from..]);
        }
        8 => nest(r, src, &toks),
        9 => insert(src, at, r.pick::<&[u8]>(INVALID_UTF8)),
        10 => {
            let texts: Vec<_> = toks.iter().filter(|t| t.kind == Kind::Text).collect();
            let escape = r.pick(SURROGATES).as_bytes();
            match texts.get(r.below(texts.len() + 1)) {
                Some(t) if t.span.len() >= 2 => insert(src, t.span.start + 1, escape),
                _ => {
                    let mut lit = b"print('".to_vec();
                    lit.extend_from_slice(escape);
                    lit.extend_from_slice(b"');");
                    let at = toks
                        .get(r.below(toks.len() + 1))
                        .map_or(src.len(), |t| t.span.start);
                    insert(src, at, &lit);
                }
            }
        }
        11 => grow(r, src, &toks),
        12 => {
            let nums: Vec<_> = toks.iter().filter(|t| t.kind == Kind::Number).collect();
            if !nums.is_empty() {
                let t = r.pick(&nums).span.clone();
                src.splice(t, r.pick(NUMBERS).bytes());
            }
        }
        13 => {
            let words: Vec<_> = toks.iter().filter(|t| t.kind == Kind::Word).collect();
            if !words.is_empty() {
                let t = r.pick(&words).span.clone();
                src.splice(t, r.pick(DICTIONARY).bytes());
            }
        }
        _ => {
            // A duplicate of one statement-sized span.
            let spans = balanced(src, &toks);
            if !spans.is_empty() {
                let s = src[r.pick(&spans).clone()].to_vec();
                insert(src, at, &s);
            }
        }
    }
}

/// Raise the nesting at one position near and past `DEPTH_CEILING`.
fn nest(r: &mut Rng, src: &mut Vec<u8>, toks: &[crate::tokens::Token]) {
    let ceiling = ulua_compiler::DEPTH_CEILING as usize;
    let random = r.range(1, ceiling + 16);
    let depth = *r.pick(&[
        ceiling - 2,
        ceiling - 1,
        ceiling,
        ceiling + 1,
        ceiling + 2,
        ceiling * 2,
        random,
        1000,
        5000,
    ]);
    let (open, close): (&[u8], &[u8]) = *r.pick(&[
        (b"(".as_slice(), b")".as_slice()),
        (b"[", b"]"),
        (b"{", b"}"),
        (b"!", b""),
        (b"-", b""),
        (b"(x:number):number=>", b""),
        (b"{a:", b"}"),
        (b"if(true){", b"}"),
        (b"try{", b"}finally{}"),
        (b"f(", b")"),
        (b"", b".x"),
        (b"", b"()"),
        (b"", b"[0]"),
        (b"1+", b""),
        (b"`${", b"}`"),
        (b"Promise<", b">"),
        (b"(", b")[]"),
    ]);
    let exprs: Vec<_> = toks
        .iter()
        .filter(|t| matches!(t.kind, Kind::Word | Kind::Number | Kind::Text))
        .collect();
    let target = match exprs.get(r.below(exprs.len() + 1)) {
        Some(t) => t.span.clone(),
        None => src.len()..src.len(),
    };
    let mut wrapped = open.repeat(depth);
    wrapped.extend_from_slice(&src[target.clone()]);
    wrapped.extend_from_slice(&close.repeat(depth));
    src.splice(target, wrapped);
}

/// Repeat one span up to a size near the source ceiling.
fn grow(r: &mut Rng, src: &mut Vec<u8>, toks: &[crate::tokens::Token]) {
    let ceiling = ulua_compiler::SOURCE_CEILING as usize;
    let (wide, small) = (r.range(ceiling / 2, ceiling + 256), r.range(1024, 8192));
    let target = *r.pick(&[ceiling - 1, ceiling, ceiling + 1, wide, small]);
    let spans = balanced(src, toks);
    let unit = match spans.get(r.below(spans.len() + 1)) {
        Some(s) if !s.is_empty() => src[s.clone()].to_vec(),
        _ => b"let v:number=1;".to_vec(),
    };
    let at = toks
        .get(r.below(toks.len() + 1))
        .map_or(src.len(), |t| t.span.start);
    let mut fill = vec![];
    while src.len() + fill.len() + unit.len() <= target {
        fill.extend_from_slice(&unit);
    }
    insert(src, at, &fill);
    if src.len() < target && r.one_in(2) {
        let pad = target - src.len();
        src.extend(std::iter::repeat_n(b' ', pad));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn same_seed_gives_same_mutants() {
        let pool = Pool::from_programs(vec![
            b"export function main(): void { print('a'); }".to_vec(),
            b"let x: number = [1, 2][0]; export function f(n: number): number { return n + x; }"
                .to_vec(),
        ]);
        let make = |seed| {
            let mut r = Rng::new(seed);
            (0..200).map(|_| mutate(&pool, &mut r)).collect::<Vec<_>>()
        };
        assert_eq!(make(9), make(9));
        assert_ne!(make(9), make(10));
        let all = make(11);
        assert!(all.iter().any(|s| std::str::from_utf8(s).is_err()));
        assert!(
            all.iter()
                .any(|s| s.len() >= ulua_compiler::SOURCE_CEILING as usize)
        );
    }
}
