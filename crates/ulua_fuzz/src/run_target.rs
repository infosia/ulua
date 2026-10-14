//! The `run` target: each accepted input is instantiated under small limits, with fixture
//! capabilities whose results the fuzzer chooses, and its exports are called. A request
//! model checks `ulua_instance_requests` after each call.
use crate::{
    check::{self, Report},
    compile_target::Compiled,
    host::{Events, HostState},
    rng::Rng,
};
use std::{
    cell::RefCell,
    collections::{HashSet, VecDeque},
    ptr,
};
use ulua::*;

/// The model of one instance: open requests, recorded completions, live tasks.
#[derive(Default)]
pub struct Model {
    /// Open requests: id and result type.
    pub open: Vec<(u64, u32)>,
    /// Recorded completions in record order: id and whether it is a failure.
    pub recorded: VecDeque<(u64, bool)>,
    /// Every request id that a start received.
    pub ids: HashSet<u64>,
    /// Live tasks: id and result type.
    pub tasks: Vec<(u64, u32)>,
    /// Every task id that a call start wrote.
    pub seen_tasks: HashSet<u64>,
    /// True when a grant of the instance is revoked.
    pub revoked: bool,
    /// The `max_open_requests` of the instance.
    pub max_open: u32,
    /// The grant names.
    pub names: Vec<String>,
}

/// The kind of a host-started run, for the codes that only some calls give.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RunCall {
    /// `ulua_instance_new`.
    Instance,
    /// `ulua_call`.
    Call,
    /// `ulua_call_start`.
    Start,
    /// `ulua_pump`; the flag is true when a settled record is a failure.
    Pump(bool),
}

impl Model {
    /// Apply the starts of one ABI call and check their ids.
    pub fn starts(&mut self, report: &mut Report, events: &Events) {
        for (id, ty) in &events.starts {
            if !self.ids.insert(*id) {
                report.fail("request:id-reused", format!("request id {id} given twice"));
            }
            self.open.push((*id, *ty));
        }
    }
    /// Check the counts of `ulua_instance_requests` against the model.
    pub fn counts(&self, report: &mut Report, instance: *mut Instance, after: &str) {
        let (mut open, mut recorded) = (u32::MAX, u32::MAX);
        // SAFETY: The instance is live; both outputs are local storage.
        let s = unsafe { ulua_instance_requests(instance, &mut open, &mut recorded) };
        if s != ULUA_OK
            || open as usize != self.open.len()
            || recorded as usize != self.recorded.len()
        {
            report.fail(
                "requests:count",
                format!(
                    "after {after}: status {s}, counts ({open}, {recorded}), model ({}, {})",
                    self.open.len(),
                    self.recorded.len()
                ),
            );
        }
    }
    /// Check the status of a host-started run against its events (abi.md §3, §6).
    pub fn run_status(
        &self,
        report: &mut Report,
        call: RunCall,
        status: Status,
        d: &Diagnostic,
        events: &Events,
    ) {
        let name = format!("{call:?}");
        check::note(format!("run:{name}:{}", check::status_name(status)));
        for v in &events.violations {
            report.fail(format!("host:violation:{name}"), v.clone());
        }
        if !events.after_stop.is_empty() {
            report.fail(
                format!("host:callback-after-stop:{name}"),
                format!("callbacks after a stopping return: {:?}", events.after_stop),
            );
        }
        let stop_capability = events.errors > 0 || events.start_fail > 0;
        let expected = if stop_capability {
            Some("host:capability")
        } else if events.bad > 0 {
            Some("host:result")
        } else if events.output_fail > 0 {
            Some("host:output")
        } else {
            None
        };
        if status == ULUA_OK {
            if let Some(code) = expected {
                report.fail(
                    format!("run:missed-stop:{name}:{code}"),
                    format!("{name} gave OK after a host return that stops with {code}"),
                );
            }
            return;
        }
        if !matches!(status, ULUA_FAILED | ULUA_HOST_ERROR) {
            report.fail(
                format!("status:{name}:{}", check::status_name(status)),
                format!("{name} gave status {status}"),
            );
            return;
        }
        let decoded = match check::decode(d) {
            Ok(x) if x.code != "unset" => x,
            Ok(_) => {
                report.fail(
                    format!("run:no-diagnostic:{name}"),
                    "the diagnostic is not written",
                );
                return;
            }
            Err(e) => {
                report.fail(format!("run:diagnostic-format:{name}"), e);
                return;
            }
        };
        let code = decoded.code.as_str();
        check::note(format!("run:{name}:{code}"));
        check::run_diagnostic(report, &name, &decoded);
        let known = if status == ULUA_FAILED {
            check::run_failed_code(code)
        } else {
            check::host_code(code)
        };
        let justified = match code {
            "host:capability" => stop_capability,
            "host:result" => events.bad > 0,
            "host:output" => events.output_fail > 0,
            "host:revoked" => self.revoked,
            "host:request" => matches!(call, RunCall::Pump(true)),
            "limit:requests" => self.open.len() >= self.max_open as usize,
            "async:pending" => call == RunCall::Call,
            _ => expected.is_none(),
        };
        if !known || !justified || expected.is_some_and(|e| e != code) {
            report.fail(
                format!("code:{name}:{}:{code}", check::status_name(status)),
                format!(
                    "{name}: {} {code} ({}); events: errors {} start_fail {} bad {} output_fail {}",
                    check::status_name(status),
                    decoded.message,
                    events.errors,
                    events.start_fail,
                    events.bad,
                    events.output_fail
                ),
            );
        }
        if matches!(code, "host:capability" | "host:revoked" | "host:result")
            && !self.names.contains(&decoded.message)
        {
            report.fail(
                format!("run:message:{code}"),
                format!("the message {:?} is not a grant name", decoded.message),
            );
        }
    }
}

fn arguments(aux: &mut Rng, types: &[u32], keep: &mut Vec<Vec<u8>>) -> (Vec<Value>, bool) {
    let mut valid = true;
    let mut out = vec![];
    for t in types {
        let mut v = Value::default();
        v.r#type = *t;
        match *t {
            ULUA_TYPE_NUMBER => v.number = aux.number(),
            ULUA_TYPE_BOOLEAN => v.boolean = u32::from(aux.one_in(2)),
            _ => {
                let bytes: &[u8] = aux.pick::<&[u8]>(&[
                    &b""[..],
                    b"text",
                    "é🙂".as_bytes(),
                    b"a\0b",
                    b"0123456789abcdef0123456789abcdef",
                ]);
                keep.push(bytes.to_vec());
                let last = keep.last().map_or(&[][..], |b| b.as_slice());
                v.data = last.as_ptr().cast();
                v.length = last.len();
            }
        }
        out.push(v);
    }
    if aux.one_in(12) {
        valid = false;
        match aux.below(4) {
            0 => out.push(Value::default()),
            1 if !out.is_empty() => {
                out.pop();
            }
            2 if !out.is_empty() => {
                let i = aux.below(out.len());
                out[i].r#type = (out[i].r#type % 3) + 1;
            }
            _ if !out.is_empty() => {
                let i = aux.below(out.len());
                out[i].boolean = 7;
            }
            _ => out.push(Value::default()),
        }
    }
    (out, valid)
}

/// True when a result value has the type and follows abi.md §4.
pub fn value_check(v: &Value, ty: u32) -> bool {
    if v.r#type != ty {
        return false;
    }
    match ty {
        ULUA_TYPE_STRING => {
            v.length == 0
                || (!v.data.is_null()
                    // SAFETY: A STRING result points to `length` bytes that the instance owns.
                    && std::str::from_utf8(unsafe {
                        std::slice::from_raw_parts(v.data.cast::<u8>(), v.length)
                    })
                    .is_ok())
        }
        ULUA_TYPE_BOOLEAN => v.boolean <= 1,
        _ => true,
    }
}

/// Instantiate an accepted module and drive its exports and requests.
pub fn run(c: &Compiled, aux: &mut Rng, state: &RefCell<HostState>, report: &mut Report) {
    {
        let mut s = state.borrow_mut();
        s.error_one_in = *aux.pick(&[0, 0, 4, 16, 64]);
        s.bad_one_in = *aux.pick(&[0, 0, 8, 32]);
        s.output_fail_one_in = *aux.pick(&[0, 0, 0, 16, 64]);
        s.events = Events::default();
    }
    let mut limits = RunLimits::default();
    limits.max_memory_bytes =
        *aux.pick(&[256 << 10, 1 << 20, 4 << 20, 16 << 20, 16 << 20, 64 << 20]);
    limits.max_steps = *aux.pick(&[0, 100, 20_000, 200_000, 200_000, 1_000_000]);
    limits.max_open_requests = *aux.pick(&[0, 1, 2, 4, 256]);
    let mut model = Model {
        max_open: limits.max_open_requests,
        names: c.signatures.iter().map(|s| s.name.clone()).collect(),
        ..Model::default()
    };
    let host = crate::host::host(state);
    let mut instance = ptr::null_mut();
    let mut d = check::diagnostic();
    // SAFETY: The context and module are live; descriptors and outputs are local storage.
    let s = unsafe {
        ulua_instance_new(
            c.owned.context,
            c.module,
            &host,
            &limits,
            &mut instance,
            &mut d,
        )
    };
    let events = state.borrow_mut().take();
    model.starts(report, &events);
    model.run_status(report, RunCall::Instance, s, &d, &events);
    if s != ULUA_OK {
        if !instance.is_null() {
            report.fail(
                "instance:non-null-on-failure",
                "a failed instance_new wrote *out",
            );
        }
        return;
    }
    model.counts(report, instance, "ulua_instance_new");
    let mut exports = vec![];
    // SAFETY: The module is live; outputs are local storage.
    unsafe {
        let mut count = 0;
        ulua_module_export_count(c.module, &mut count);
        for i in 0..count {
            let mut types = [0u32; 32];
            let (mut n, mut result) = (0usize, 0u32);
            ulua_module_export_signature(c.module, i, types.as_mut_ptr(), 32, &mut n, &mut result);
            exports.push((types[..n.min(32)].to_vec(), result));
        }
    }
    for step in 0..aux.range(4, 24) {
        if report.failures.len() > 8 {
            break;
        }
        // SAFETY: The instance and the grants stay live through each operation.
        unsafe { operation(c, instance, &exports, &mut model, aux, state, report) };
        model.counts(report, instance, &format!("operation {step}"));
    }
    // SAFETY: The instance is live and freed once.
    let s = unsafe { ulua_instance_free(instance) };
    if s != ULUA_OK {
        report.fail("status:ulua_instance_free", format!("status {s}"));
    }
}

unsafe fn operation(
    c: &Compiled,
    instance: *mut Instance,
    exports: &[(Vec<u32>, u32)],
    model: &mut Model,
    aux: &mut Rng,
    state: &RefCell<HostState>,
    report: &mut Report,
) {
    let mut d = check::diagnostic();
    let mut keep = vec![];
    // SAFETY: The caller keeps the instance live; every output is local storage.
    unsafe {
        match aux.below(10) {
            0..=3 if !exports.is_empty() || aux.one_in(4) => {
                let index = if aux.one_in(16) {
                    exports.len() as u32 + aux.below(3) as u32
                } else {
                    aux.below(exports.len()) as u32
                };
                let types = exports.get(index as usize).map_or(&[][..], |e| &e.0[..]);
                let result_type = exports.get(index as usize).map_or(0, |e| e.1);
                let (args, mut valid) = arguments(aux, types, &mut keep);
                valid &= (index as usize) < exports.len();
                let sync = aux.one_in(2);
                let mut task = u64::MAX;
                let mut value = Value::default();
                let s = if sync {
                    ulua_call(
                        instance,
                        index,
                        args.as_ptr(),
                        args.len(),
                        &mut value,
                        &mut d,
                    )
                } else {
                    ulua_call_start(
                        instance,
                        index,
                        args.as_ptr(),
                        args.len(),
                        &mut task,
                        &mut d,
                    )
                };
                let events = state.borrow_mut().take();
                if !valid {
                    if s != ULUA_INVALID_ARGUMENT
                        || events.calls > 0
                        || !events.starts.is_empty()
                        || !events.outputs.is_empty()
                    {
                        report.fail(
                            "call:invalid-arguments",
                            format!("status {s}, script ran: {}", events.calls),
                        );
                    }
                    return;
                }
                model.starts(report, &events);
                let call = if sync { RunCall::Call } else { RunCall::Start };
                model.run_status(report, call, s, &d, &events);
                if s == ULUA_OK && sync && !value_check(&value, result_type) {
                    report.fail(
                        "call:result-value",
                        format!("{value:?} for type {result_type}"),
                    );
                }
                if s == ULUA_OK && !sync {
                    if !model.seen_tasks.insert(task) {
                        report.fail("task:id-reused", format!("task id {task} given twice"));
                    }
                    model.tasks.push((task, result_type));
                }
            }
            4 => {
                let live = !model.tasks.is_empty() && !aux.one_in(6);
                let id = if live {
                    model.tasks[aux.below(model.tasks.len())].0
                } else {
                    let random = aux.next_u64();
                    *aux.pick(&[0, u64::MAX, random])
                };
                let position = model.tasks.iter().position(|t| t.0 == id);
                if aux.one_in(5) {
                    let s = ulua_task_discard(instance, id);
                    let want = if position.is_some() {
                        ULUA_OK
                    } else {
                        ULUA_INVALID_ARGUMENT
                    };
                    if s != want {
                        report.fail(
                            format!("task:discard:{}", check::status_name(s)),
                            format!("task {id}: status {s}, want {want}"),
                        );
                    }
                    if let Some(p) = position {
                        model.tasks.remove(p);
                    }
                    return;
                }
                let mut value = Value::default();
                let s = ulua_task_result(instance, id, &mut value, &mut d);
                let events = state.borrow_mut().take();
                if !events.outputs.is_empty() || events.calls > 0 || !events.starts.is_empty() {
                    report.fail("task:script-ran", "a task query ran script code");
                }
                let code = check::decode(&d).map(|x| x.code).unwrap_or_default();
                match position {
                    None if s != ULUA_INVALID_ARGUMENT => report.fail(
                        format!("task:unknown:{}", check::status_name(s)),
                        format!("unknown task {id}: status {s}"),
                    ),
                    None => {}
                    Some(p) => {
                        let ty = model.tasks[p].1;
                        let ok = match s {
                            ULUA_PENDING => true,
                            ULUA_OK => value_check(&value, ty),
                            ULUA_FAILED => code == "trap:uncaught-exception",
                            _ => false,
                        };
                        if !ok {
                            report.fail(
                                format!("task:result:{}:{code}", check::status_name(s)),
                                format!("task {id}: status {s} code {code} value {value:?}"),
                            );
                        }
                        if s != ULUA_PENDING {
                            model.tasks.remove(p);
                        }
                    }
                }
            }
            5 | 6 => {
                let pending: Vec<_> = model
                    .open
                    .iter()
                    .filter(|(id, _)| !model.recorded.iter().any(|r| r.0 == *id))
                    .copied()
                    .collect();
                let (id, ty) = if !pending.is_empty() && !aux.one_in(8) {
                    *aux.pick(&pending)
                } else {
                    let random = aux.next_u64() % 64;
                    (*aux.pick(&[0, 1, u64::MAX, random]), 1)
                };
                let open = pending.iter().any(|p| p.0 == id);
                let fail = aux.one_in(4);
                let (s, valid) = if fail {
                    (ulua_request_fail(instance, id), open)
                } else {
                    let mut v = crate::host::valid_value_for(aux, ty, &mut keep);
                    let mut valid = open;
                    if aux.one_in(8) {
                        v.r#type = (ty + 1) % 4;
                        valid = false;
                    }
                    (ulua_request_complete(instance, id, &v), valid)
                };
                let ok = if valid {
                    s == ULUA_OK || (!fail && s == ULUA_FAILED)
                } else {
                    s == ULUA_INVALID_ARGUMENT
                };
                if !ok {
                    report.fail(
                        format!("request:record:{}", check::status_name(s)),
                        format!("request {id} fail={fail} valid={valid}: status {s}"),
                    );
                }
                if s == ULUA_OK {
                    model.recorded.push_back((id, fail));
                }
            }
            7 | 8 => {
                let max = *aux.pick(&[0, 1, 1, 2, 5]);
                let mut settled = u32::MAX;
                let before: Vec<_> = model.recorded.iter().copied().collect();
                let s = ulua_pump(instance, max, &mut settled, &mut d);
                let events = state.borrow_mut().take();
                let k = settled as usize;
                let limit = if max == 0 {
                    before.len()
                } else {
                    (max as usize).min(before.len())
                };
                if k > limit || (s == ULUA_OK && k != limit) {
                    report.fail(
                        "pump:settled",
                        format!(
                            "status {s}, settled {settled}, recorded {}, max {max}",
                            before.len()
                        ),
                    );
                    return;
                }
                let failed = k > 0 && before[k - 1].1;
                for (id, _) in &before[..k] {
                    model.recorded.pop_front();
                    model.open.retain(|o| o.0 != *id);
                }
                model.starts(report, &events);
                model.run_status(report, RunCall::Pump(failed), s, &d, &events);
                // A failure record stops its settle with host:request, unless the step
                // charge of the settle stops it first (abi.md §6.4.7).
                if failed && !matches!(s, ULUA_HOST_ERROR | ULUA_FAILED) {
                    report.fail(
                        "pump:failure-settled",
                        format!("a failure record gave status {s}"),
                    );
                }
            }
            _ => {
                if !c.owned.grants.is_empty() {
                    let i = aux.below(c.owned.grants.len());
                    let s = ulua_capability_revoke(c.owned.grants[i]);
                    if s != ULUA_OK {
                        report.fail("status:ulua_capability_revoke", format!("status {s}"));
                    }
                    c.owned.bounds[i].revoked.set(true);
                    model.revoked = true;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compile_target::{Env, compile};
    fn diagnostic(code: &str) -> Diagnostic {
        let mut d = check::diagnostic();
        d.code.fill(0);
        for (o, b) in d.code.iter_mut().zip(code.bytes()) {
            *o = b as std::ffi::c_char;
        }
        d
    }
    #[test]
    fn run_statuses_outside_the_contract_are_failures() {
        let m = Model {
            names: vec!["hostAdd".into()],
            max_open: 4,
            ..Model::default()
        };
        let quiet = Events::default();
        let cases: [(Status, &str, Events, &str); 5] = [
            (
                ULUA_INTERNAL,
                "internal",
                Events::default(),
                "status:Call:INTERNAL",
            ),
            (
                ULUA_HOST_ERROR,
                "host:capability",
                Events::default(),
                "code:Call:HOST_ERROR:host:capability",
            ),
            (
                ULUA_FAILED,
                "async:pending",
                Events::default(),
                "code:Start:FAILED:async:pending",
            ),
            (
                ULUA_OK,
                "",
                Events {
                    errors: 1,
                    ..Events::default()
                },
                "run:missed-stop:Call:host:capability",
            ),
            (
                ULUA_FAILED,
                "limit:requests",
                Events::default(),
                "code:Call:FAILED:limit:requests",
            ),
        ];
        for (status, code, events, key) in cases {
            let mut r = Report::default();
            let call = if code == "async:pending" {
                RunCall::Start
            } else {
                RunCall::Call
            };
            m.run_status(&mut r, call, status, &diagnostic(code), &events);
            assert!(
                r.failures.iter().any(|f| f.key == key),
                "{key}: {:?}",
                r.failures
            );
        }
        let mut r = Report::default();
        m.run_status(
            &mut r,
            RunCall::Call,
            ULUA_FAILED,
            &diagnostic("trap:index-out-of-range"),
            &quiet,
        );
        assert!(r.clean(), "{:?}", r.failures);
    }
    #[test]
    fn a_request_count_that_differs_from_the_model_is_a_failure() {
        let env = Env {
            rules: check::rule_ids(include_str!("../../../specs/blocks/collisions.md")),
        };
        let state = RefCell::new(HostState::new(Rng::new(2)));
        let src = b"// grants: hostLater\nexport async function main(): Promise<void> { await hostLater(1); }";
        let mut r = Report::default();
        let c = compile(&env, src, &mut Rng::new(1), &state, &mut r).unwrap();
        let host = crate::host::host(&state);
        let mut instance = ptr::null_mut();
        // SAFETY: Live objects and local outputs.
        unsafe {
            assert_eq!(
                ulua_instance_new(
                    c.owned.context,
                    c.module,
                    &host,
                    ptr::null(),
                    &mut instance,
                    ptr::null_mut()
                ),
                ULUA_OK
            );
            let mut task = 0;
            assert_eq!(
                ulua_call_start(instance, 0, ptr::null(), 0, &mut task, ptr::null_mut()),
                ULUA_OK
            );
        }
        let mut m = Model::default();
        m.starts(&mut r, &state.borrow_mut().take());
        m.counts(&mut r, instance, "start");
        assert!(r.clean(), "{:?}", r.failures);
        // A seeded model defect: a recorded completion that the host never made.
        m.recorded.push_back((1, false));
        m.counts(&mut r, instance, "seeded");
        assert_eq!(r.failures[0].key, "requests:count");
    }
}
