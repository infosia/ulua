//! Direct tests of the engine properties: each seeded violation is a failure.
use crate::{
    check::Report,
    engine::{Engine, run},
    host::Events,
    rng::Rng,
};
use ulua::*;

fn inputs(n: usize) -> Vec<Vec<u8>> {
    let mut r = Rng::new(77);
    (0..n)
        .map(|_| (0..r.range(64, 900)).map(|_| r.next_u64() as u8).collect())
        .collect()
}

#[test]
fn random_sequences_are_clean() {
    for requests in [false, true] {
        for input in inputs(40) {
            let mut report = Report::default();
            run(requests, &input, &mut report);
            assert!(report.clean(), "requests={requests}: {:?}", report.failures);
        }
    }
}

#[test]
fn a_status_outside_the_model_is_a_failure() {
    let mut report = Report::default();
    let mut e = Engine::new(false, &[], &mut report);
    assert!(!e.expect("ulua_call", ULUA_INTERNAL, &[ULUA_OK], "seeded"));
    drop(e);
    assert_eq!(report.failures[0].key, "status:ulua_call:INTERNAL");
}

#[test]
fn a_request_count_that_differs_from_the_model_is_a_failure() {
    let mut report = Report::default();
    let mut e = Engine::new(true, &[1, 2, 3], &mut report);
    assert!(
        e.insts.iter().any(|i| i.live),
        "the requests setup gives an instance"
    );
    e.counts();
    assert!(e.report.clean(), "{:?}", e.report.failures);
    // A seeded model defect: one open request that the library never started.
    let i = e.insts.iter().position(|i| i.live).unwrap();
    e.insts[i].open.push(99);
    e.rd = crate::engine::Reader::from(&[1, 1, 1][..]);
    e.counts();
    drop(e);
    assert!(
        report.failures.iter().any(|f| f.key == "requests:count"),
        "{:?}",
        report.failures
    );
}

#[test]
fn a_nested_status_other_than_busy_is_a_failure() {
    let mut report = Report::default();
    let mut e = Engine::new(false, &[], &mut report);
    let events = Events {
        nested: vec![("ulua_module_free".into(), ULUA_OK, vec![ULUA_BUSY])],
        violations: vec!["hostAdd: called after revocation".into()],
        ..Events::default()
    };
    e.nested_results("ulua_call", &events);
    drop(e);
    let keys: Vec<_> = report.failures.iter().map(|f| f.key.as_str()).collect();
    assert_eq!(
        keys,
        ["nested:ulua_module_free:OK", "host:violation:ulua_call"]
    );
}

#[test]
fn a_wrong_diagnostic_code_is_a_failure() {
    let mut report = Report::default();
    let mut e = Engine::new(false, &[], &mut report);
    let mut d = crate::check::diagnostic();
    for (o, b) in d.code.iter_mut().zip(b"limit:steps\0") {
        *o = *b as std::ffi::c_char;
    }
    e.written("ulua_call", &d, "trap:index-out-of-range", true);
    drop(e);
    assert_eq!(report.failures[0].key, "code:ulua_call:limit:steps");
}
