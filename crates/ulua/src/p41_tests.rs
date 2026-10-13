//! Native observations verify queue retention and allocation-free stop propagation.
use crate::*;
use std::{ffi::c_void, ptr, time::Instant};
// SAFETY: These private probes match their C++ definitions and receive a live VM.
unsafe extern "C" {
    fn ulua_internal_test_async_stats(vm: *mut c_void, bytes: *mut usize, jobs: *mut usize) -> i32;
    #[cfg(feature = "measurements")]
    fn ulua_internal_test_no_interrupt(vm: *mut c_void);
    #[cfg(feature = "measurements")]
    fn ulua_internal_test_async_headroom(vm: *mut c_void, bytes: usize);
    #[cfg(feature = "measurements")]
    fn ulua_internal_test_resume_stats(vm: *mut c_void, checks: *mut u64, allocations: *mut u64);
}
struct Run {
    context: *mut Context,
    instance: *mut Instance,
    module: *mut Module,
}
impl Run {
    fn new(source: &str, limits: &RunLimits) -> Self {
        Self::with_host(source, limits, &Host::default())
    }
    fn with_host(source: &str, limits: &RunLimits, host: &Host) -> Self {
        let mut r = Self {
            context: ptr::null_mut(),
            instance: ptr::null_mut(),
            module: ptr::null_mut(),
        };
        let mut d = Diagnostic::default();
        // SAFETY: The fixture owns all descriptors and pointers and serializes object lifetimes.
        unsafe {
            assert_eq!(ulua_context_new(&mut r.context), ULUA_OK);
            assert_eq!(
                ulua_compile(
                    r.context,
                    source.as_ptr().cast(),
                    source.len(),
                    ptr::null(),
                    0,
                    ptr::null(),
                    &mut r.module,
                    &mut d
                ),
                ULUA_OK,
                "{:?}",
                d.code
            );
            assert_eq!(
                ulua_instance_new(r.context, r.module, host, limits, &mut r.instance, &mut d),
                ULUA_OK,
                "{:?}",
                d.code
            );
        }
        r
    }
    fn call(&self, name: &str, arguments: &[Value]) -> Status {
        let mut index = 0;
        let mut d = Diagnostic::default();
        let mut value = Value::default();
        // SAFETY: The fixture keeps the module and instance live and all storage covers its length.
        unsafe {
            assert_eq!(
                ulua_module_find_export(self.module, name.as_ptr().cast(), name.len(), &mut index),
                ULUA_OK
            );
            ulua_call(
                self.instance,
                index,
                arguments.as_ptr(),
                arguments.len(),
                &mut value,
                &mut d,
            )
        }
    }
    fn stats(&self) -> (usize, usize) {
        let mut bytes = 0;
        let mut jobs = 0;
        // SAFETY: The native probe protects collection and receives writable output storage.
        unsafe {
            assert_eq!(
                ulua_internal_test_async_stats((*self.instance).vm, &mut bytes, &mut jobs),
                0
            );
        }
        (bytes, jobs)
    }
}
impl Drop for Run {
    fn drop(&mut self) {
        // SAFETY: The owned context releases every live child exactly once.
        unsafe {
            assert_eq!(ulua_context_free(self.context), ULUA_OK);
        }
    }
}
#[test]
fn ten_thousand_serial_awaits_retain_no_job() {
    let start = Instant::now();
    let r = Run::new(
        "async function v():Promise<number>{return 1;}export async function serial(n:number):Promise<number>{const p=v();let a:number=0;for(let i=0;i<n;i++){a+=await p;}return a;}export function main():void{}",
        &RunLimits::default(),
    );
    let mut counts = vec![];
    for n in [1000, 10000] {
        let value = Value {
            r#type: ULUA_TYPE_NUMBER,
            number: n as f64,
            ..Value::default()
        };
        assert_eq!(r.call("serial", &[value]), ULUA_OK);
        let (bytes, jobs) = r.stats();
        assert_eq!(jobs, 0);
        counts.push(bytes);
    }
    assert!(counts[1] <= counts[0] * 2, "retained bytes: {counts:?}");
    eprintln!(
        "P4.1 serial await retained 1000/10000 bytes={}/{} jobs=0 cost={:.3}s",
        counts[0],
        counts[1],
        start.elapsed().as_secs_f64()
    );
}
#[cfg(feature = "measurements")]
#[test]
fn resume_stop_reraise_requests_no_allocation() {
    let start = Instant::now();
    for body in [
        "let s:string='ab';while(true){s=s+s;}",
        "recurse();",
        "const xs:number[]=[];print(`${xs[0]}`);",
        "const xs:number[]=[1];xs.forEach((n:number):void=>{xs.push(n);});",
        "print('é'.slice(1));",
        "print('x'.repeat(-1));",
        "print('x'.repeat(1000000000));",
        "print('stop');",
        "const p=nest(100);await p;",
    ] {
        let source = format!(
            "async function tick():Promise<number>{{return 1;}}function recurse():number{{return recurse();}}async function nest(n:number):Promise<number>{{if(n===0){{return 1;}}return nest(n-1);}}export async function fail():Promise<number>{{await tick();{body}return 0;}}export function main():void{{}}"
        );
        let limits = RunLimits {
            max_memory_bytes: 8 * 1024 * 1024,
            ..RunLimits::default()
        };
        let host = Host {
            output: Some(stop_output),
            ..Host::default()
        };
        let r = Run::with_host(&source, &limits, &host);
        // SAFETY: This probe disables interrupts to expose the scheduler's own stop check.
        unsafe {
            ulua_internal_test_no_interrupt((*r.instance).vm);
            if body.contains("nest") {
                ulua_internal_test_async_headroom((*r.instance).vm, 64 * 1024);
            }
        }
        assert_eq!(
            r.call("fail", &[]),
            if body == "print('stop');" {
                ULUA_HOST_ERROR
            } else {
                ULUA_FAILED
            }
        );
        let mut checks = 0;
        let mut allocations = 0;
        // SAFETY: The serialized fixture owns a live VM and writable counters.
        unsafe {
            ulua_internal_test_resume_stats((*r.instance).vm, &mut checks, &mut allocations);
        }
        assert!(checks > 0);
        assert_eq!(allocations, 0);
        eprintln!("P4.1 stop reraise checks={checks} allocations={allocations}");
    }
    eprintln!(
        "P4.1 stop allocation probe cost={:.3}s",
        start.elapsed().as_secs_f64()
    );
}

#[cfg(feature = "measurements")]
unsafe extern "C" fn stop_output(_: *mut c_void, _: *const std::ffi::c_char, _: usize) -> i32 {
    1
}
