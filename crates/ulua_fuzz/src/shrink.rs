//! Delta debugging (ddmin): remove units while the failure stays, first over bytes, then over tokens.
use std::ops::Range;

/// The unit of one reduction pass.
#[derive(Clone, Copy, Debug)]
pub enum Unit {
    /// One byte.
    Bytes,
    /// One token of `tokens::lex`.
    Tokens,
}

fn units(input: &[u8], unit: Unit) -> Vec<Range<usize>> {
    match unit {
        Unit::Bytes => (0..input.len()).map(|i| i..i + 1).collect(),
        Unit::Tokens => crate::tokens::lex(input)
            .into_iter()
            .map(|t| t.span)
            .collect(),
    }
}

fn without(input: &[u8], spans: &[Range<usize>]) -> Vec<u8> {
    let mut out = Vec::with_capacity(input.len());
    let mut at = 0;
    for s in spans {
        out.extend_from_slice(&input[at..s.start]);
        at = s.end;
    }
    out.extend_from_slice(&input[at..]);
    out
}

/// A test budget. Each candidate run spends one.
pub struct Budget(pub usize);

fn ddmin(
    mut current: Vec<u8>,
    unit: Unit,
    test: &mut dyn FnMut(&[u8]) -> bool,
    budget: &mut Budget,
) -> Vec<u8> {
    let mut n = 2usize;
    loop {
        let us = units(&current, unit);
        if us.is_empty() || budget.0 == 0 {
            return current;
        }
        n = n.min(us.len()).max(1);
        let size = us.len().div_ceil(n);
        let mut reduced = false;
        for chunk in us.chunks(size) {
            if budget.0 == 0 {
                return current;
            }
            budget.0 -= 1;
            let candidate = without(&current, chunk);
            if test(&candidate) {
                current = candidate;
                n = n.saturating_sub(1).max(2);
                reduced = true;
                break;
            }
        }
        if !reduced {
            if n >= us.len() {
                return current;
            }
            n = (n * 2).min(us.len());
        }
    }
}

/// Reduce `input` to a smaller input for which `test` stays true. The result is 1-minimal
/// in bytes and in tokens when the budget allows.
pub fn shrink(input: &[u8], test: &mut dyn FnMut(&[u8]) -> bool, budget: &mut Budget) -> Vec<u8> {
    let mut current = input.to_vec();
    loop {
        let before = current.len();
        current = ddmin(current, Unit::Bytes, test, budget);
        current = ddmin(current, Unit::Tokens, test, budget);
        if current.len() == before || budget.0 == 0 {
            return current;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn shrinks_to_the_failing_core() {
        let input = b"let a = 1; let bad = f(xyz); let c = 3; export function main(): void {}";
        let mut test = |c: &[u8]| c.windows(3).any(|w| w == b"xyz") && c.contains(&b'(');
        let out = shrink(input, &mut test, &mut Budget(10_000));
        assert!(test(&out));
        assert!(out.len() <= 5, "{}", String::from_utf8_lossy(&out));
        // A one-unit removal of the result never keeps the failure: the result is 1-minimal.
        for i in 0..out.len() {
            let mut c = out.clone();
            c.remove(i);
            assert!(!test(&c));
        }
    }
    #[test]
    fn budget_bounds_the_test_count() {
        let mut calls = 0;
        let mut test = |_: &[u8]| {
            calls += 1;
            false
        };
        let _ = shrink(&[7; 4096], &mut test, &mut Budget(50));
        assert!(calls <= 50);
    }
}
