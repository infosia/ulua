//! Request and task allocations obey the instance budget. Report bounded probe costs.
use super::*;
#[link(name = "ulua_test_native", kind = "static")]
// SAFETY: The test-only probe adjusts this fixture's allocator budget.
unsafe extern "C" {
    fn ulua_internal_test_async_headroom(vm: *mut c_void, bytes: usize);
}
#[test]
fn completion_copy_obeys_memory_limit_and_failed_copy_records_nothing() {
    let now = Instant::now();
    let mut r = Run::new(
        "export async function work():Promise<string>{return await host3(1,true,'s');}export function main():void{}",
        &[3],
        RunLimits::default(),
    );
    let task = r.start("work").2;
    let id = r.data.requests.pop_front().unwrap().0;
    // SAFETY: This fixture owns its live VM. Headroom is derived from allocator usage.
    unsafe {
        ulua_internal_test_async_headroom((*r.i).vm, 4096);
    }
    let bytes = vec![b'x'; 65536];
    let value = Value {
        r#type: 3,
        data: bytes.as_ptr().cast(),
        length: bytes.len(),
        ..Value::default()
    };
    assert_eq!(r.complete(id, value), ULUA_FAILED);
    assert_eq!(r.counts(), (1, 0));
    assert_eq!(r.result(task).0, ULUA_PENDING);
    assert_eq!(
        r.complete(
            id,
            Value {
                r#type: 3,
                data: c"ok".as_ptr(),
                length: 2,
                ..Value::default()
            }
        ),
        ULUA_OK
    );
    assert_eq!(r.pump(1).0, ULUA_OK);
    assert_eq!(r.result(task).2.length, 2);
    eprintln!(
        "P4.3 completion memory copy=65536 headroom=4096 failed_records=0 cost={:.3}s",
        now.elapsed().as_secs_f64()
    );
}
#[test]
fn settle_memory_stop_preserves_later_completion_and_instance() {
    let mut r = Run::new(
        "async function wait():Promise<void>{try{await host1(1,true,'s');let s='x';for(let i=0;i<24;i++){s+=s;}print(s);}catch{print('catch');}finally{print('finally');}}export function main():void{const a=wait();const b=host1(2,true,'s').then((n:number):void=>print(`${n}`));}export function good():number{return 42;}",
        &[1],
        RunLimits::default(),
    );
    assert_eq!(r.start("main").0, ULUA_OK);
    for (id, args) in r.data.requests.drain(..).collect::<Vec<_>>() {
        assert_eq!(r.complete(id, args[0]), ULUA_OK);
    }
    // SAFETY: This fixture owns the VM. Limit the reaction allocation, not module setup.
    unsafe {
        ulua_internal_test_async_headroom((*r.i).vm, 65536);
    }
    let result = r.pump(0);
    assert_eq!(result.0, ULUA_FAILED);
    assert_eq!(code(&result.1), "limit:memory");
    assert_eq!(result.2, 1);
    assert_eq!(r.counts(), (1, 1));
    assert!(r.data.output.is_empty());
    assert_eq!(r.pump(1).0, ULUA_OK);
    assert_eq!(r.data.output, ["2"]);
    assert_eq!(r.counts(), (0, 0));
    let task = r.start("good").2;
    assert_eq!(r.result(task).2.number, 42.0);
}
#[test]
fn number_negative_zero_and_boolean_unused_fields_follow_value_contract() {
    for (tag, good, bad) in [
        (
            1,
            number(-0.0),
            Value {
                r#type: 1,
                boolean: 1,
                ..Value::default()
            },
        ),
        (
            2,
            Value {
                r#type: 2,
                boolean: 0,
                ..Value::default()
            },
            Value {
                r#type: 2,
                boolean: 2,
                ..Value::default()
            },
        ),
        (
            0,
            Value::default(),
            Value {
                number: -0.0,
                ..Value::default()
            },
        ),
    ] {
        let mut r = Run::new(
            &format!("export function main():void{{const p=host{tag}(1,true,'s');}}"),
            &[tag],
            RunLimits::default(),
        );
        assert_eq!(r.start("main").0, ULUA_OK);
        let id = r.data.requests.pop_front().unwrap().0;
        assert_eq!(r.complete(id, bad), ULUA_INVALID_ARGUMENT);
        assert_eq!(r.complete(id, good), ULUA_OK);
        assert_eq!(r.pump(0).0, ULUA_OK);
    }
}
