//! P5.3: native allocation sweeps. Baselines count each creation, call, completion, and settle.
//! Each selected index uses a fresh instance. No external witness or measurement process runs.
//! Previous sanitizer sweep: 26 programs, 182 baseline runs, 1,257 injection trials; measured cost 0.37 seconds.
//! That sweep used 3,967 ABI runs with replay and recovery. Each source adds a recovery export and handler markers.
//! Normal Debug with both spills: 28 programs, 192 baseline runs, 1,352 allocation indices and injection trials, 0.140 seconds.
//! This spill sweep uses 4,262 ABI runs, including replay and recovery.
//! One rejected example has no native runtime allocation. The total ABI run count includes replay and recovery.
//! Programs above 20,000 indices use stride ceil(n/20,000), plus 64 indices beside every phase boundary.
//! The summary prints baseline and injection runs, eligible allocations, and cost. No time assertion applies.
mod fixture;
mod programs;
use fixture::{Fixture, MARKER, TRY_MARKER, assert_clean, code};
use std::{ptr, time::Instant};
use ulua::{fault_injection as fault, *};

#[derive(Clone, Copy, Debug)]
enum Phase {
    New,
    Call(u32, bool),
    Complete,
    Settle,
}
impl Phase {
    fn run(self, f: &mut Fixture) -> Status {
        match self {
            Self::New => f.create(),
            Self::Call(i, start) => f.call(i, start),
            Self::Complete => f.complete(),
            Self::Settle => f.settle(),
        }
    }
}
#[derive(Clone, Debug)]
struct Run {
    phase: Phase,
    allocations: u64,
    status: Status,
    code: String,
}
fn observe(f: &mut Fixture, phase: Phase) -> Run {
    fault::arm(0);
    let status = phase.run(f);
    Run {
        phase,
        allocations: fault::last().allocations,
        status,
        code: code(&f.diagnostic).into(),
    }
}
fn baseline(f: &mut Fixture, export: Option<(u32, bool)>) -> Vec<Run> {
    f.reset();
    let first = observe(f, Phase::New);
    let ok = first.status == ULUA_OK;
    let mut runs = vec![first];
    if !ok {
        assert_clean(fault::last()).unwrap();
        return runs;
    }
    if let Some((index, start)) = export {
        let call = observe(f, Phase::Call(index, start));
        let proceed = call.status == ULUA_OK || call.code == "async:pending";
        runs.push(call);
        if proceed {
            while !f.host.requests.is_empty() {
                assert!(runs.len() < 1000, "unexpected request loop");
                let complete = observe(f, Phase::Complete);
                assert_eq!(complete.status, ULUA_OK);
                runs.push(complete);
                let settle = observe(f, Phase::Settle);
                let ok = settle.status == ULUA_OK;
                runs.push(settle);
                if !ok {
                    break;
                }
            }
            if start && f.task != 0 {
                let mut value = Value::default();
                let mut d = Diagnostic::default();
                // SAFETY: The fixture owns the task and writable result storage.
                unsafe {
                    assert_eq!(
                        ulua_task_result(f.instance, f.task, &mut value, &mut d),
                        ULUA_OK,
                        "{}",
                        code(&d)
                    );
                }
            }
        }
    }
    assert_clean(f.free()).unwrap();
    runs
}
fn selected(index: u64, total: u64, boundaries: &[u64]) -> bool {
    total <= 20_000
        || index.is_multiple_of(total.div_ceil(20_000))
        || boundaries.iter().any(|&b| index.abs_diff(b) <= 64)
}
fn inject(f: &mut Fixture, runs: &[Run], target: usize, k: u64, label: &str) -> bool {
    f.reset();
    let mut protected_failure = false;
    for (i, run) in runs.iter().enumerate().take(target + 1) {
        fault::arm(if i == target { k } else { 0 });
        let status = run.phase.run(f);
        if i != target {
            assert_eq!(status, run.status, "prefix {label} phase={i}");
            assert_eq!(code(&f.diagnostic), run.code, "prefix {label} phase={i}");
            assert_eq!(
                fault::last().allocations,
                run.allocations,
                "prefix {label} phase={i} allocation count"
            );
            continue;
        }
        let snapshot = fault::last();
        assert_eq!(
            snapshot.failures, 1,
            "{label} phase={:?} k={k} {snapshot:?}",
            run.phase
        );
        assert_eq!(
            status,
            ULUA_FAILED,
            "{label} phase={:?} k={k} status={status} code={} {snapshot:?}",
            run.phase,
            code(&f.diagnostic)
        );
        if matches!(run.phase, Phase::Complete) {
            let mut open = 0;
            let mut recorded = 0;
            // SAFETY: The fixture owns the live instance and count storage.
            unsafe {
                assert_eq!(
                    ulua_instance_requests(f.instance, &mut open, &mut recorded),
                    ULUA_OK
                );
            }
            assert!(open > 0);
            assert_eq!(recorded, 0);
            let (id, value) = f.completion.unwrap();
            fault::arm(0);
            // SAFETY: A refused copy leaves this live request open; its bytes remain stable.
            unsafe {
                assert_eq!(ulua_request_complete(f.instance, id, &value), ULUA_OK);
            }
        } else {
            assert_eq!(
                code(&f.diagnostic),
                "limit:memory",
                "{label} phase={:?} k={k}",
                run.phase
            );
        }
        assert!(
            !f.host.handler_after_failure,
            "handler ran after refusal: {label} phase={:?} k={k}",
            run.phase
        );
        if label == "protected-allocation" && f.host.output.iter().any(|s| s == TRY_MARKER) {
            assert!(!f.host.output.iter().any(|s| s == MARKER));
            protected_failure = true;
        }
        if !f.instance.is_null() {
            f.small();
            assert_clean(f.free()).unwrap();
        } else {
            assert_clean(snapshot).unwrap();
            // A failed constructor has no instance. Retry construction if its baseline can succeed.
            if runs[0].status == ULUA_OK {
                f.small();
                assert_clean(f.free()).unwrap();
            } else {
                f.small_in_context();
            }
        }
    }
    protected_failure
}

// Cost: two compiler calls, two injected calls, and two recovery calls.
#[test]
fn refused_gc_allocations_cannot_return_pending_or_steps() {
    let start = Instant::now();
    for (source, grants, k) in [
        (
            include_str!("../../../../corpus/accept/a64-host-requests.ts"),
            &["hostLater", "hostLog"][..],
            5,
        ),
        (
            include_str!("../../../../examples/limits/limits.ts"),
            &[][..],
            1,
        ),
    ] {
        let mut f = Fixture::new(source, grants);
        fault::arm(0);
        assert_eq!(f.create(), ULUA_OK);
        // These indices reach GC stack resize during the call and its post-stop collection.
        fault::arm(k);
        assert_eq!(f.call(0, false), ULUA_FAILED);
        assert_eq!(fault::last().failures, 1);
        assert_eq!(code(&f.diagnostic), "limit:memory", "k={k}");
        assert!(!f.host.handler_after_failure);
        f.small();
        assert_clean(f.free()).unwrap();
    }
    eprintln!(
        "GC refused allocations call_k=5 cleanup_k=1 recovery=ok cost={:.6}s",
        start.elapsed().as_secs_f64()
    );
}

#[test]
fn allocation_failure_sweep() {
    let start = Instant::now();
    let mut injections = 0;
    let mut baselines = 0;
    let mut allocations = 0;
    let mut program_count = 0;
    let mut abi_runs = 0;
    for program in programs::programs() {
        if program.rejected {
            let mut context = ptr::null_mut();
            let mut module = ptr::null_mut();
            let mut d = Diagnostic::default();
            // SAFETY: The test owns serialized context and writable compile outputs.
            unsafe {
                assert_eq!(ulua_context_new(&mut context), ULUA_OK);
                assert_eq!(
                    ulua_compile(
                        context,
                        program.source.as_ptr().cast(),
                        program.source.len(),
                        ptr::null(),
                        0,
                        ptr::null(),
                        &mut module,
                        &mut d
                    ),
                    ULUA_REJECTED
                );
                assert!(module.is_null());
                assert_eq!(ulua_context_free(context), ULUA_OK);
            }
            continue;
        }
        let mut f = Fixture::new(&program.source, &program.grants);
        let new = baseline(&mut f, None);
        if program.name == "examples/run/init-trap.ts" {
            assert_eq!(new[0].status, ULUA_FAILED);
            assert_eq!(new[0].code, "trap:index-out-of-range");
        } else {
            assert_eq!(new[0].status, ULUA_OK, "{}: {}", program.name, new[0].code);
        }
        baselines += new.len();
        let mut plans = vec![new];
        if plans[0][0].status == ULUA_OK {
            for export in 0..f.exports {
                if export == f.small {
                    continue;
                }
                for start in [false, true] {
                    let plan = baseline(&mut f, Some((export, start)));
                    for run in plan.iter().skip(1) {
                        if run.status != ULUA_OK {
                            assert_eq!(run.status, ULUA_FAILED, "{} {run:?}", program.name);
                            assert!(
                                run.code == "async:pending"
                                    || (program.name == "examples/limits/limits.ts"
                                        && matches!(
                                            run.code.as_str(),
                                            "limit:steps"
                                                | "limit:memory"
                                                | "trap:index-out-of-range"
                                        ))
                                    || (program.name == "examples/run/trap.ts"
                                        && run.code == "trap:index-out-of-range"),
                                "unexpected baseline stop: {} {run:?}",
                                program.name
                            );
                        }
                    }
                    baselines += plan.len();
                    plans.push(plan);
                }
            }
        }
        // Constructor indices are swept once per program; each export gets both ABI call paths.
        let total: u64 = plans
            .iter()
            .enumerate()
            .map(|(i, plan)| {
                plan.iter()
                    .skip(usize::from(i != 0))
                    .map(|r| r.allocations)
                    .sum::<u64>()
            })
            .sum();
        let mut offset = 0;
        let mut boundaries = vec![1];
        for (i, plan) in plans.iter().enumerate() {
            for run in plan.iter().skip(usize::from(i != 0)) {
                boundaries.push(offset + 1);
                offset += run.allocations;
            }
        }
        boundaries.push(total);
        let before = injections;
        let mut protected_failures = 0;
        offset = 0;
        for (i, plan) in plans.iter().enumerate() {
            for (phase, run) in plan.iter().enumerate().skip(usize::from(i != 0)) {
                for k in 1..=run.allocations {
                    if selected(offset + k, total, &boundaries) {
                        protected_failures +=
                            usize::from(inject(&mut f, plan, phase, k, program.name));
                        injections += 1;
                    }
                }
                offset += run.allocations;
            }
        }
        if program.name == "protected-allocation" {
            assert!(
                protected_failures > 0,
                "no allocation refusal inside the try body"
            );
            eprintln!("fault protected-allocation try_failures={protected_failures}");
        }
        abi_runs += f.runs;
        allocations += total;
        program_count += 1;
        eprintln!(
            "fault program={} eligible={total} injections={}",
            program.name,
            injections - before
        );
    }
    eprintln!(
        "fault sweep programs={program_count} baseline_runs={baselines} injection_runs={injections} abi_runs={abi_runs} eligible={allocations} cost={:.3}s",
        start.elapsed().as_secs_f64()
    );
}

#[test]
fn seeded_leak_fails_the_same_lifetime_check() {
    let mut f = Fixture::new("export function main(): number { return 1; }", &[]);
    assert_eq!(f.create(), ULUA_OK);
    // SAFETY: The fixture owns a live instance; the test releases the seed after VM destruction.
    unsafe {
        let before = fault::live(f.instance).live_bytes;
        assert!(fault::seed_leak(f.instance));
        assert_eq!(fault::live(f.instance).live_bytes, before + 31);
        let snapshot = f.free();
        let result = assert_clean(snapshot);
        fault::release_seed();
        assert_eq!(snapshot.live_bytes, 31);
        assert!(result.is_err());
    }
}
#[test]
fn sampling_keeps_stride_and_both_sides_of_phase_boundaries() {
    let n: u64 = 100_001;
    let boundaries = [1, 30_000, 80_000, n];
    assert_eq!(n.div_ceil(20_000), 6);
    for k in 1..=n {
        let expected = k % 6 == 0 || boundaries.iter().any(|&b| k.abs_diff(b) <= 64);
        assert_eq!(selected(k, n, &boundaries), expected);
    }
    for k in 1..=20_000 {
        assert!(selected(k, 20_000, &[]));
    }
    assert!(!selected(30_067, n, &boundaries));
}
