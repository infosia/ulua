//! The reference model of the `abi` and `requests` targets: fixed programs whose exports
//! have known results, the capability names and their validity, and the documented
//! status of each call (abi.md). The model derives each expectation from the contract,
//! apart from the library.
use ulua::*;

/// The behaviour of one export of a fixed program.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// `print('main')`, then `counter += 1`.
    Main,
    /// Returns nothing and runs no host code.
    Nop,
    /// `a + b`.
    Add,
    /// Returns its string.
    Echo,
    /// `!b`.
    Flip,
    /// Returns `counter`.
    Count,
    /// An index out of range.
    Boom,
    /// `while (true) {}`.
    Spin,
    /// `hostAdd(a, 1)`.
    UseAdd,
    /// `hostLog(s)`.
    UseLog,
    /// `total += await hostLater(n)`, returns the value.
    Wait,
    /// Two requests `n` and `n + 1`, returns their sum.
    Pair,
    /// `n` sequential requests `0..n`, returns their sum.
    Chain,
    /// Returns `total`.
    Sum,
    /// An async export that throws.
    Fails,
    /// An async export that returns `n`.
    Quick,
}

/// One export: name, parameter types, result type, behaviour.
pub struct Export {
    /// The export name.
    pub name: &'static str,
    /// Parameter type tags.
    pub params: &'static [u32],
    /// Result type tag.
    pub result: u32,
    /// The behaviour.
    pub kind: Kind,
}

/// The compile outcome of a program whose grants are valid and complete.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Compiles {
    /// `ULUA_OK`.
    Yes,
    /// `ULUA_REJECTED` with this code.
    No(&'static str),
}

/// The instance outcome of a compiled program under the default limits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Init {
    /// `ULUA_OK`.
    Ok,
    /// A module-level trap.
    Trap(&'static str),
    /// A module-level `print('init')`.
    Print,
}

/// A fixed program.
pub struct Program {
    /// The source.
    pub source: &'static str,
    /// The grants that the script calls, by name.
    pub needs: &'static [&'static str],
    /// The compile outcome.
    pub compiles: Compiles,
    /// The instance outcome.
    pub init: Init,
    /// The exports in source order.
    pub exports: &'static [Export],
}

const N: u32 = ULUA_TYPE_NUMBER;
const B: u32 = ULUA_TYPE_BOOLEAN;
const S: u32 = ULUA_TYPE_STRING;
const V: u32 = ULUA_TYPE_VOID;

macro_rules! export {
    ($name:literal, [$($p:expr),*], $r:expr, $k:ident) => {
        Export { name: $name, params: &[$($p),*], result: $r, kind: Kind::$k }
    };
}

/// The fixed programs. Index 2 is the async program of the `requests` target.
pub const PROGRAMS: &[Program] = &[
    Program {
        source: "let counter: number = 0;\nexport function main(): void { print('main'); counter = counter + 1; }\nexport function add(a: number, b: number): number { return a + b; }\nexport function echo(s: string): string { return s; }\nexport function flip(b: boolean): boolean { return !b; }\nexport function count(): number { return counter; }\nexport function boom(): number { const xs: number[] = []; return xs[3]; }\nexport function spin(): void { while (true) {} }\n",
        needs: &[],
        compiles: Compiles::Yes,
        init: Init::Ok,
        exports: &[
            export!("main", [], V, Main),
            export!("add", [N, N], N, Add),
            export!("echo", [S], S, Echo),
            export!("flip", [B], B, Flip),
            export!("count", [], N, Count),
            export!("boom", [], N, Boom),
            export!("spin", [], V, Spin),
        ],
    },
    Program {
        source: "export function useAdd(a: number): number { return hostAdd(a, 1); }\nexport function useLog(s: string): void { hostLog(s); }\nexport function main(): void {}\n",
        needs: &["hostAdd", "hostLog"],
        compiles: Compiles::Yes,
        init: Init::Ok,
        exports: &[
            export!("useAdd", [N], N, UseAdd),
            export!("useLog", [S], V, UseLog),
            export!("main", [], V, Nop),
        ],
    },
    Program {
        source: "let total: number = 0;\nexport async function wait(n: number): Promise<number> { const v = await hostLater(n); total = total + v; return v; }\nexport async function pair(n: number): Promise<number> { const a = hostLater(n); const b = hostLater(n + 1); return (await a) + (await b); }\nexport async function chain(n: number): Promise<number> { let s: number = 0; for (let i = 0; i < n; i++) { s = s + await hostLater(i); } return s; }\nexport function sum(): number { return total; }\nexport async function fails(): Promise<number> { throw new RangeError('x'); }\nexport async function quick(n: number): Promise<number> { return n; }\nexport function main(): void {}\n",
        needs: &["hostLater"],
        compiles: Compiles::Yes,
        init: Init::Ok,
        exports: &[
            export!("wait", [N], N, Wait),
            export!("pair", [N], N, Pair),
            export!("chain", [N], N, Chain),
            export!("sum", [], N, Sum),
            export!("fails", [], N, Fails),
            export!("quick", [N], N, Quick),
            export!("main", [], V, Nop),
        ],
    },
    Program {
        source: "export function (\n",
        needs: &[],
        compiles: Compiles::No("syntax"),
        init: Init::Ok,
        exports: &[],
    },
    Program {
        source: "export function arr(): number[] { return []; }\nexport function main(): void {}\n",
        needs: &[],
        compiles: Compiles::No("unsupported"),
        init: Init::Ok,
        exports: &[],
    },
    Program {
        source: "const xs: number[] = [];\nconst y: number = xs[1];\nexport function main(): void {}\n",
        needs: &[],
        compiles: Compiles::Yes,
        init: Init::Trap("trap:index-out-of-range"),
        exports: &[export!("main", [], V, Nop)],
    },
    Program {
        source: "function f(): number { print('init'); return 1; }\nconst z: number = f();\nexport function main(): void {}\n",
        needs: &[],
        compiles: Compiles::Yes,
        init: Init::Print,
        exports: &[export!("main", [], V, Nop)],
    },
];

/// A capability signature: parameter types, result type, async flag.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Sig {
    /// Parameter type tags.
    pub params: Vec<u32>,
    /// Result type tag.
    pub result: u32,
    /// True for an async capability.
    pub asynchronous: bool,
}

/// The signature that a program needs for a grant name.
pub fn need(name: &str) -> Option<Sig> {
    let (params, result, asynchronous) = match name {
        "hostAdd" => (vec![N, N], N, false),
        "hostLog" => (vec![S], V, false),
        "hostLater" => (vec![N], N, true),
        _ => return None,
    };
    Some(Sig {
        params,
        result,
        asynchronous,
    })
}

/// Capability names with their validity under abi.md §6.2.1, derived from the rule text:
/// 1 to 64 bytes, `[A-Za-z_$][A-Za-z0-9_$]*`, not an ES2022 reserved word, not `eval` or
/// `arguments`, not a global value name of the ES2022 library or the prelude.
pub const NAMES: &[(&[u8], bool)] = &[
    (b"hostAdd", true),
    (b"hostLog", true),
    (b"hostLater", true),
    (b"hostNot", true),
    (b"h$_9", true),
    (b"_", true),
    (b"$", true),
    (
        b"abcdefghijabcdefghijabcdefghijabcdefghijabcdefghijabcdefghijabcd",
        true,
    ),
    (
        b"abcdefghijabcdefghijabcdefghijabcdefghijabcdefghijabcdefghijabcde",
        false,
    ),
    (b"", false),
    (b"1x", false),
    (b"a-b", false),
    (b"print", false),
    (b"Utf8", false),
    (b"Number", false),
    (b"Math", false),
    (b"Promise", false),
    (b"await", false),
    (b"class", false),
    (b"yield", false),
    (b"eval", false),
    (b"arguments", false),
    ("é".as_bytes(), false),
    (b"a\xff", false),
    (b"a b", false),
];

/// The expected outcome of a run: a status and, for a stop, its code.
#[derive(Clone, Debug, PartialEq)]
pub enum Expect {
    /// `ULUA_OK`.
    Ok,
    /// A stop with a status and a code.
    Stop(Status, &'static str),
}

/// The value of a task.
#[derive(Clone, Debug, PartialEq)]
pub enum TaskState {
    /// The promise is pending and waits on a request.
    Pending,
    /// The promise is pending for good (its request failed, or a run stopped).
    Stuck,
    /// The promise is fulfilled with a number.
    Number(f64),
    /// The promise is rejected.
    Rejected,
    /// A sync export: its result has this type.
    Typed(u32),
}

/// What a settle of a request resumes.
#[derive(Clone, Debug)]
pub enum Waiter {
    /// `wait`: the task, if one observes it.
    Wait(Option<u64>),
    /// `pair`: the task, the two ids, and the values so far.
    Pair(Option<u64>, [u64; 2], [Option<f64>; 2]),
    /// `chain`: the task, the next index, the count, the sum so far.
    Chain(Option<u64>, u32, u32, f64),
}

/// Equality of two numbers for a value check: the same bits, or both NaN.
pub fn same(a: f64, b: f64) -> bool {
    a.to_bits() == b.to_bits() || (a.is_nan() && b.is_nan())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn programs_compile_as_the_model_states() {
        for (i, p) in PROGRAMS.iter().enumerate() {
            let grants: Vec<_> = p
                .needs
                .iter()
                .map(|n| {
                    let s = need(n).unwrap();
                    let mut g = ulua_compiler::HostSignature::new(*n, s.params, s.result);
                    g.asynchronous = s.asynchronous;
                    g
                })
                .collect();
            let r = ulua_compiler::compile_with_grants(
                p.source.as_bytes(),
                ulua_compiler::Limits::default(),
                &grants,
            );
            match (p.compiles, r) {
                (Compiles::Yes, Ok(c)) => {
                    let names: Vec<_> = c.exports.iter().map(|e| e.name.as_str()).collect();
                    let want: Vec<_> = p.exports.iter().map(|e| e.name).collect();
                    assert_eq!(names, want, "program {i}");
                    for (e, w) in c.exports.iter().zip(p.exports) {
                        assert_eq!((&e.parameters[..], e.result), (w.params, w.result));
                    }
                }
                (Compiles::No(code), Err(d)) => assert_eq!(d.code, code, "program {i}"),
                (c, r) => panic!("program {i}: model {c:?}, compiler {:?}", r.err()),
            }
            if !p.needs.is_empty() {
                let r = ulua_compiler::compile(p.source.as_bytes(), Default::default());
                assert!(r.is_err(), "program {i} compiles without its grants");
            }
        }
    }
}
