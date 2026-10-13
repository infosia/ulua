//! Five start stops retain no extra records, promises, or bytes after one and 10,000 attempts.
use super::*;

#[link(name = "ulua_test_native", kind = "static")]
// SAFETY: Private probes use the live fixture VM and writable local output storage.
unsafe extern "C" {
    fn ulua_internal_test_async_headroom(vm: *mut c_void, bytes: usize);
    fn ulua_internal_test_request_memory_stop(vm: *mut c_void, stage: i32) -> i32;
    fn ulua_internal_test_request_memory_stage(vm: *mut c_void) -> i32;
    fn ulua_internal_test_request_stats(
        vm: *mut c_void,
        bytes: *mut usize,
        promises: *mut usize,
        records: *mut usize,
        next: *mut u64,
    ) -> i32;
}

fn stats(r: &Run) -> (usize, usize, usize, u64) {
    let (mut bytes, mut promises, mut records, mut next) = (0, 0, 0, 0);
    // SAFETY: Collection stays protected; the fixture owns the VM and all outputs.
    unsafe {
        assert_eq!(
            ulua_internal_test_request_stats(
                (*r.i).vm,
                &mut bytes,
                &mut promises,
                &mut records,
                &mut next,
            ),
            0
        );
    }
    (bytes, promises, records, next)
}

#[test]
fn every_start_stop_discards_only_its_request_and_has_no_memory_growth() {
    let now = Instant::now();
    for asynchronous in [false, true] {
        let work = if asynchronous {
            "export async function work():Promise<number>{return await host1(2,true,'s');}"
        } else {
            "export function work():void{const p=host1(2,true,'s');}"
        };
        let source = format!(
            "export function main():void{{}}export function good():number{{return 42;}}export function seed():void{{const p=host1(1,true,'s');}}{work}"
        );
        for path in [
            "nonzero",
            "revoked",
            "promise-memory",
            "record-memory",
            "requests",
        ] {
            let sample_start = Instant::now();
            let mut r = Run::new(
                &source,
                &[1],
                RunLimits {
                    max_open_requests: if path == "requests" { 1 } else { 4_000_000_000 },
                    ..RunLimits::default()
                },
            );
            // The separate BUSY tests cover callback reentry. This probe retains no callback history.
            r.data.instance = ptr::null_mut();
            let seed_task = r.start("seed");
            assert_eq!(seed_task.0, ULUA_OK);
            assert_eq!(r.result(seed_task.2).0, ULUA_OK);
            let seed = r.data.requests.pop_front().unwrap().0;
            assert_eq!(r.complete(seed, number(7.0)), ULUA_OK);
            let expected_code = match path {
                "nonzero" => {
                    r.data.fail = true;
                    "host:capability"
                }
                "revoked" => {
                    // SAFETY: The fixture owns this live grant; its recorded request remains valid.
                    unsafe { assert_eq!(ulua_capability_revoke(r.caps[0]), ULUA_OK) };
                    "host:revoked"
                }
                "promise-memory" | "record-memory" => {
                    // SAFETY: The probe replaces only this fixture's factory with a protected test wrapper.
                    unsafe {
                        assert_eq!(
                            ulua_internal_test_request_memory_stop(
                                (*r.i).vm,
                                if path == "promise-memory" { 1 } else { 2 },
                            ),
                            0
                        );
                    }
                    "limit:memory"
                }
                _ => "limit:requests",
            };
            let before = r.counts();
            assert_eq!(before, (1, 1));
            let baseline = stats(&r);
            assert_eq!((baseline.1, baseline.2), (1, 1));
            let mut candidate = baseline.3 + 1;
            let mut first_bytes = 0;
            let mut final_bytes = 0;
            for attempt in 1..=10_000 {
                let result = r.start("work");
                assert_eq!(
                    result.0,
                    if expected_code.starts_with("host:") {
                        ULUA_HOST_ERROR
                    } else {
                        ULUA_FAILED
                    },
                    "{path} attempt={attempt} {}",
                    code(&result.1)
                );
                assert_eq!(code(&result.1), expected_code, "{path} attempt={attempt}");
                assert_eq!(result.2, u64::MAX);
                assert_eq!(r.counts(), before, "{path} attempt={attempt}");
                if path == "nonzero" {
                    let id = r.data.requests.pop_front().unwrap().0;
                    assert_eq!(id, candidate);
                    candidate = id + 1;
                    assert_eq!(r.complete(id, number(2.0)), ULUA_INVALID_ARGUMENT);
                    // SAFETY: The fixture owns the instance; the failed start's id is discarded.
                    unsafe { assert_eq!(ulua_request_fail(r.i, id), ULUA_INVALID_ARGUMENT) };
                } else {
                    assert!(r.data.requests.is_empty());
                    assert_eq!(r.complete(candidate, number(2.0)), ULUA_INVALID_ARGUMENT);
                    // SAFETY: No callback received this id; the live instance rejects it.
                    unsafe { assert_eq!(ulua_request_fail(r.i, candidate), ULUA_INVALID_ARGUMENT) };
                }
                // SAFETY: Restore budget after the test wrapper sets the allocation boundary.
                unsafe { ulua_internal_test_async_headroom((*r.i).vm, 8 * 1024 * 1024) };
                if path.ends_with("memory") {
                    // SAFETY: The protected probe observes the last factory stage in this live VM.
                    let stage = unsafe { ulua_internal_test_request_memory_stage((*r.i).vm) };
                    assert_eq!(stage, if path == "promise-memory" { 1 } else { 2 });
                }
                if attempt == 1 || attempt == 10_000 {
                    let sample = stats(&r);
                    assert_eq!((sample.1, sample.2), (baseline.1, baseline.2), "{path}");
                    if attempt == 1 {
                        first_bytes = sample.0;
                    } else {
                        final_bytes = sample.0;
                        assert!(
                            final_bytes <= first_bytes,
                            "{path} async={asynchronous}: {final_bytes} > {first_bytes}"
                        );
                    }
                }
            }
            assert_eq!(r.pump(0).0, ULUA_OK);
            assert_eq!(r.counts(), (0, 0));
            assert!(r.data.output.is_empty());
            let good = r.start("good");
            assert_eq!(good.0, ULUA_OK);
            assert_eq!(r.result(good.2).2.number, 42.0);
            if path == "nonzero" || path == "requests" {
                r.data.fail = false;
                let recovery = r.start("work");
                assert_eq!(recovery.0, ULUA_OK);
                let id = r.data.requests.pop_front().unwrap().0;
                assert!(id >= candidate);
                assert_eq!(r.complete(id, number(2.0)), ULUA_OK);
                assert_eq!(r.pump(0).0, ULUA_OK);
                assert_eq!(r.result(recovery.2).0, ULUA_OK);
                assert_eq!(r.counts(), (0, 0));
            }
            eprintln!(
                "P4.3 start-stop path={path} async={asynchronous} attempts=10000 bytes_1={first_bytes} bytes_10000={final_bytes} cost={:.3}s",
                sample_start.elapsed().as_secs_f64()
            );
        }
    }
    eprintln!(
        "P4.3 start-stop attempts=100000 cost={:.3}s",
        now.elapsed().as_secs_f64()
    );
}
