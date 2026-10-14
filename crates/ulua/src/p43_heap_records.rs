//! Full-heap records use the review sweep. Five paths cover each value tag and failure.
use super::*;

#[cfg(feature = "measurements")]
// SAFETY: The test archive observes the allocator counter of the live fixture VM.
unsafe extern "C" {
    fn ulua_internal_test_allocation_calls(vm: *const c_void) -> u64;
}

#[test]
fn full_heap_accepts_every_completion_and_failure_and_pump_removes_every_record() {
    let now = Instant::now();
    let mut instances = 0;
    let mut records = 0;
    let mut stops = 0;
    for path in 0..=4 {
        let tag = if path == 4 { 1 } else { path };
        let source = format!(
            "class Node{{next:Node|null;constructor(next:Node|null){{this.next=next;}}}}let head:Node|null=null;export function main():void{{for(let i=0;i<256;i++){{const p=host{tag}(i,true,'s');}}while(true){{head=new Node(head);}}}}"
        );
        for budget in (2_000_000..=2_010_000).step_by(37) {
            let mut r = Run::new(
                &source,
                &[tag],
                RunLimits {
                    max_memory_bytes: budget,
                    max_steps: 10_000_000,
                    ..RunLimits::default()
                },
            );
            // Callback reentry has a separate direct test. Keep the sweep's host storage bounded.
            r.data.instance = ptr::null_mut();
            let stopped = r.start("main");
            assert_eq!(stopped.0, ULUA_FAILED, "path={path} budget={budget}");
            assert_eq!(code(&stopped.1), "limit:memory");
            assert_eq!(r.data.requests.len(), 256);
            assert_eq!(r.counts(), (256, 0));
            let value = match tag {
                0 => Value::default(),
                1 => number(7.0),
                2 => Value {
                    r#type: 2,
                    boolean: 1,
                    ..Value::default()
                },
                _ => Value {
                    r#type: 3,
                    data: c"host bytes".as_ptr(),
                    length: 10,
                    ..Value::default()
                },
            };
            #[cfg(feature = "measurements")]
            // SAFETY: The fixture owns this live VM; the probe reads a counter only.
            let allocations = unsafe { ulua_internal_test_allocation_calls((*r.i).vm) };
            for (id, _) in &r.data.requests {
                let status = if path == 4 {
                    // SAFETY: The fixture owns the live instance and serializes each record call.
                    unsafe { ulua_request_fail(r.i, *id) }
                } else {
                    r.complete(*id, value)
                };
                assert_eq!(status, ULUA_OK, "path={path} budget={budget} id={id}");
                assert_eq!(r.complete(*id, value), ULUA_INVALID_ARGUMENT);
                // SAFETY: This recorded id stays invalid for another failure record.
                unsafe { assert_eq!(ulua_request_fail(r.i, *id), ULUA_INVALID_ARGUMENT) };
                records += 1;
            }
            assert_eq!(r.complete(u64::MAX, value), ULUA_INVALID_ARGUMENT);
            // SAFETY: The fixture owns the instance; this unknown id records nothing.
            unsafe { assert_eq!(ulua_request_fail(r.i, u64::MAX), ULUA_INVALID_ARGUMENT) };
            #[cfg(feature = "measurements")]
            // SAFETY: The same independent counter covers success and validation failure paths.
            assert_eq!(
                unsafe { ulua_internal_test_allocation_calls((*r.i).vm) },
                allocations
            );
            assert_eq!(r.counts(), (256, 256));
            for remaining in (0..256).rev() {
                let settled = r.pump(1);
                assert_eq!(settled.2, 1, "path={path} budget={budget}");
                match settled.0 {
                    ULUA_OK => assert_ne!(path, 4),
                    ULUA_FAILED => {
                        assert_eq!(code(&settled.1), "limit:memory");
                        stops += 1;
                    }
                    ULUA_HOST_ERROR => {
                        assert_eq!(path, 4);
                        assert_eq!(code(&settled.1), "host:request");
                        stops += 1;
                    }
                    status => panic!("Unexpected settle status {status}"),
                }
                assert_eq!(r.counts(), (remaining, remaining));
            }
            for (id, _) in &r.data.requests {
                assert_eq!(r.complete(*id, value), ULUA_INVALID_ARGUMENT);
                // SAFETY: The fixture owns the instance. The pump removed each id.
                unsafe { assert_eq!(ulua_request_fail(r.i, *id), ULUA_INVALID_ARGUMENT) };
            }
            assert_eq!(r.pump(0).2, 0);
            instances += 1;
        }
    }
    eprintln!(
        "P4.3 full-heap sweep instances={instances} records={records} stops={stops} counts=0/0 cost={:.3}s",
        now.elapsed().as_secs_f64()
    );
}
