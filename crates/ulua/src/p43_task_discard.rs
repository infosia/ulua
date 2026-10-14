//! Task discard releases roots after a stopped settle and accepts each unconsumed task state.
use super::*;

// SAFETY: The private probe reads the allocation total of a live fixture VM.
unsafe extern "C" {
    fn ulua_internal_test_async_bytes(vm: *const c_void) -> usize;
}

#[test]
fn ulua_task_discard_releases_pending_task_after_stop_and_collection() {
    let now = Instant::now();
    let mut r = Run::new(
        "export async function work():Promise<number>{return await host1(1,true,'s');}export function main():void{}",
        &[1],
        RunLimits::default(),
    );
    let mut tasks = vec![];
    // Multiple promise roots occupy complete allocator pages. Collection can then release those pages.
    for _ in 0..256 {
        let started = r.start("work");
        assert_eq!(started.0, ULUA_OK);
        tasks.push(started.2);
        let id = r.data.requests.pop_front().unwrap().0;
        // SAFETY: The fixture owns the live instance and serializes the failure record.
        unsafe { assert_eq!(ulua_request_fail(r.i, id), ULUA_OK) };
        let stopped = r.pump(1);
        assert_eq!(stopped.0, ULUA_HOST_ERROR);
        assert_eq!(code(&stopped.1), "host:request");
        assert_eq!(r.result(started.2).0, ULUA_PENDING);
    }
    let task = tasks[0];
    assert_eq!(r.counts(), (0, 0));
    let before = start_stops::stats(&r);
    assert_eq!((before.1, before.2), (256, 0));
    // SAFETY: The fixture owns this unconsumed task and checks null and unknown ids separately.
    unsafe {
        assert_eq!(
            ulua_task_discard(ptr::null_mut(), task),
            ULUA_INVALID_ARGUMENT
        );
        assert_eq!(ulua_task_discard(r.i, 0), ULUA_INVALID_ARGUMENT);
        assert_eq!(ulua_task_discard(r.i, u64::MAX), ULUA_INVALID_ARGUMENT);
        for &task in &tasks {
            assert_eq!(ulua_task_discard(r.i, task), ULUA_OK);
        }
        assert_eq!(ulua_task_discard(r.i, task), ULUA_INVALID_ARGUMENT);
    }
    for task in tasks {
        assert_eq!(r.result(task).0, ULUA_INVALID_ARGUMENT);
    }
    // SAFETY: The fixture owns the VM; this read requests no collection or allocation.
    let released = unsafe { ulua_internal_test_async_bytes((*r.i).vm) };
    let after = start_stops::stats(&r);
    assert_eq!((after.1, after.2), (0, 0));
    assert!(
        after.0 < released && released < before.0,
        "bytes before={} released={released} collected={}",
        before.0,
        after.0
    );
    assert_eq!(r.counts(), (0, 0));
    eprintln!(
        "P4.3 task discard pending promises=256/0 bytes={}/{released}/{} cost={:.3}s",
        before.0,
        after.0,
        now.elapsed().as_secs_f64()
    );
}

#[test]
fn ulua_task_discard_accepts_sync_fulfilled_rejected_and_live_pending_tasks() {
    let mut r = Run::new(
        "export function sync():string{return 'retained';}export async function fulfilled():Promise<number>{return 42;}export async function rejected():Promise<number>{throw new TypeError('no');}export async function pending():Promise<number>{return await host1(1,true,'s');}export function main():void{}",
        &[1],
        RunLimits::default(),
    );
    for name in ["sync", "fulfilled", "rejected", "pending"] {
        let started = r.start(name);
        assert_eq!(started.0, ULUA_OK, "{name} {}", code(&started.1));
        let before = r.counts();
        // SAFETY: The fixture owns each task; discard releases the task but keeps host requests.
        unsafe { assert_eq!(ulua_task_discard(r.i, started.2), ULUA_OK) };
        assert_eq!(r.result(started.2).0, ULUA_INVALID_ARGUMENT);
        assert_eq!(r.counts(), before);
    }
    let consumed = r.start("sync").2;
    assert_eq!(r.result(consumed).0, ULUA_OK);
    // SAFETY: The fixture owns the live instance. A terminal query consumed this id.
    unsafe { assert_eq!(ulua_task_discard(r.i, consumed), ULUA_INVALID_ARGUMENT) };
    let id = r.data.requests.pop_front().unwrap().0;
    assert_eq!(r.complete(id, number(7.0)), ULUA_OK);
    assert_eq!(r.pump(1).0, ULUA_OK);
    assert_eq!(r.counts(), (0, 0));
}
