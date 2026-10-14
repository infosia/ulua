//! The four targets of pipeline.md §9.2 and the input of each index.
use crate::{
    check::Report,
    compile_target::{self, Env},
    host::HostState,
    rng::{self, Rng},
    seeds::Pool,
    witness::Sample,
};
use std::cell::RefCell;

/// A fuzz target.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Target {
    /// Source bytes into `ulua_compile`.
    Compiler,
    /// Accepted inputs instantiated and called.
    Run,
    /// Random ABI call sequences over a pool of objects.
    Abi,
    /// Random async request sequences.
    Requests,
}

impl Target {
    /// Every target, in smoke order.
    pub const ALL: [Target; 4] = [Target::Compiler, Target::Run, Target::Abi, Target::Requests];
    /// The command-line name.
    pub fn name(self) -> &'static str {
        match self {
            Target::Compiler => "compiler",
            Target::Run => "run",
            Target::Abi => "abi",
            Target::Requests => "requests",
        }
    }
    /// Parse a command-line name.
    pub fn parse(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|t| t.name() == s)
    }
    /// True for the targets whose input is a source program.
    pub fn source(self) -> bool {
        matches!(self, Target::Compiler | Target::Run)
    }
}

/// The input of one index. The same seed and index give the same bytes.
pub fn input(target: Target, pool: &Pool, seed: u64, index: u64) -> Vec<u8> {
    let mut r = Rng::new(rng::mix(seed, index, rng::STREAM_INPUT));
    if target.source() {
        crate::mutate::mutate(pool, &mut r)
    } else {
        let n = r.range(16, 1200);
        (0..n).map(|_| r.next_u64() as u8).collect()
    }
}

/// True when the index is in the one-in-64 witness sample.
pub fn sampled(seed: u64, index: u64) -> bool {
    rng::mix(seed, index, rng::STREAM_WITNESS).is_multiple_of(64)
}

#[cfg(test)]
thread_local! {
    /// A test hook: the next input run panics.
    pub static INJECT_PANIC: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// The outcome of one input.
#[derive(Default)]
pub struct Outcome {
    /// The failures.
    pub report: Report,
    /// True when the compile gave `ULUA_OK`.
    pub accepted: bool,
    /// The compile time.
    pub nanos: u128,
    /// The witness sample, when the input is sampled and accepted.
    pub sample: Option<Sample>,
}

/// Run one input of a target. `witness` selects the sample for this input.
pub fn execute(
    target: Target,
    env: &Env,
    input: &[u8],
    seed: u64,
    index: u64,
    witness: bool,
) -> Outcome {
    let mut out = Outcome::default();
    #[cfg(test)]
    if INJECT_PANIC.with(|c| c.replace(false)) {
        panic!("injected panic");
    }
    let mut aux = Rng::new(rng::mix(seed, index, rng::STREAM_AUX));
    match target {
        Target::Compiler | Target::Run => {
            let state = RefCell::new(HostState::new(aux.clone()));
            let compiled = compile_target::compile(env, input, &mut aux, &state, &mut out.report);
            if let Some(c) = compiled {
                out.nanos = c.nanos;
                out.accepted = !c.module.is_null();
                if out.accepted
                    && witness
                    && let Some(luau) = compile_target::luau(&c, input, &mut out.report)
                {
                    out.sample = Some(Sample {
                        index,
                        source: input.to_vec(),
                        signatures: c.signatures.clone(),
                        luau,
                    });
                }
                if out.accepted && target == Target::Run {
                    crate::run_target::run(&c, &mut aux, &state, &mut out.report);
                }
            }
        }
        Target::Abi | Target::Requests => {
            out.accepted = crate::engine::run(target == Target::Requests, input, &mut out.report);
        }
    }
    out
}
