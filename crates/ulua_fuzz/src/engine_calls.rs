//! The call, task, request, and pump operations of the engine, the nested calls from
//! inside host callbacks, and the simulation of the fixed programs (model.rs).
use crate::{
    check,
    engine::{Engine, Slot},
    host::{Events, Mode},
    model::{Expect, Kind, PROGRAMS, TaskState, Waiter, same},
};
use std::{collections::VecDeque, ptr};
use ulua::*;

/// The simulation of one host-started run: queued modes, observed starts, and counts.
struct Sim {
    modes: VecDeque<Mode>,
    starts: VecDeque<u64>,
    calls: u32,
    started: u32,
}

impl Sim {
    fn mode(&mut self) -> Mode {
        self.modes.pop_front().unwrap_or(Mode::Ok)
    }
}

impl Engine<'_> {
    /// A host mode from the input; a nested mode queues one nested call.
    fn host_mode(&mut self, ctx: usize, inst: Option<usize>) -> Mode {
        let m = match self.rd.below(16) {
            0 => Mode::Error,
            1 => Mode::Bad,
            2 => Mode::Nested,
            _ => Mode::Ok,
        };
        if m == Mode::Nested {
            let n = self.nested(ctx, inst);
            self.state.borrow_mut().nested.push_back(n);
        }
        m
    }

    /// Queue the mode of the next output call and return it.
    pub fn output_mode(&mut self, ctx: usize, inst: Option<usize>) -> Mode {
        let m = match self.host_mode(ctx, inst) {
            Mode::Bad => Mode::Ok,
            m => m,
        };
        self.state.borrow_mut().output_modes.push_back(m);
        m
    }

    /// Check the nested calls and the callback violations of one ABI call.
    pub fn nested_results(&mut self, call: &str, events: &Events) {
        for (name, got, allowed) in &events.nested {
            check::note(format!("nested:{name}:{}", check::status_name(*got)));
            if !allowed.contains(got) {
                self.report.fail(
                    format!("nested:{name}:{}", check::status_name(*got)),
                    format!(
                        "{name} inside a callback of {call}: status {got}, allowed {allowed:?}"
                    ),
                );
            }
        }
        for v in &events.violations {
            self.report
                .fail(format!("host:violation:{call}"), v.clone());
        }
        if !events.after_stop.is_empty() {
            self.report.fail(
                format!("host:callback-after-stop:{call}"),
                format!("{:?}", events.after_stop),
            );
        }
    }

    /// Take the events of the finished call and drop each queued mode and nested call,
    /// so none of them runs in a later call.
    pub fn take_events(&mut self) -> Events {
        let mut s = self.state.borrow_mut();
        s.modes.clear();
        s.output_modes.clear();
        s.nested.clear();
        s.result = None;
        s.take()
    }

    /// One start of `hostLater` in the simulation.
    fn start(&mut self, i: usize, sim: &mut Sim) -> Result<u64, Expect> {
        let held = self.insts[i].held.first().copied();
        if held.is_some_and(|c| self.caps[c].bound.revoked.get()) {
            return Err(Expect::Stop(ULUA_HOST_ERROR, "host:revoked"));
        }
        if self.insts[i].open.len() >= self.insts[i].max_open as usize {
            return Err(Expect::Stop(ULUA_FAILED, "limit:requests"));
        }
        if sim.mode() == Mode::Error {
            return Err(Expect::Stop(ULUA_HOST_ERROR, "host:capability"));
        }
        sim.started += 1;
        let Some(id) = sim.starts.pop_front() else {
            self.report.fail(
                "requests:start-missing",
                "the model expects a start the host did not see",
            );
            return Err(Expect::Ok);
        };
        if !self.insts[i].seen_requests.insert(id) {
            self.report
                .fail("request:id-reused", format!("request id {id} given twice"));
        }
        self.insts[i].open.push(id);
        Ok(id)
    }

    /// One sync host call in the simulation.
    fn host(&mut self, i: usize, cap: usize, sim: &mut Sim) -> Result<(), Expect> {
        let held = self.insts[i].held.get(cap).copied();
        if held.is_some_and(|c| self.caps[c].bound.revoked.get()) {
            return Err(Expect::Stop(ULUA_HOST_ERROR, "host:revoked"));
        }
        sim.calls += 1;
        match sim.mode() {
            Mode::Error => Err(Expect::Stop(ULUA_HOST_ERROR, "host:capability")),
            Mode::Bad => Err(Expect::Stop(ULUA_HOST_ERROR, "host:result")),
            _ => Ok(()),
        }
    }

    /// The call operation: `ulua_call` or `ulua_call_start` of one export.
    pub fn call(&mut self) {
        let Some(i) = self.live_inst() else {
            return;
        };
        let (ctx, program) = (self.insts[i].ctx, self.insts[i].program);
        let exports = PROGRAMS[program].exports;
        let index = if exports.is_empty() || self.rd.one_in(16) {
            exports.len() + self.rd.below(2)
        } else {
            self.rd.below(exports.len())
        };
        let sync = if self.requests {
            self.rd.one_in(3)
        } else {
            self.rd.one_in(2)
        };
        let e = exports.get(index);
        let mut keep: Vec<Vec<u8>> = vec![];
        let mut args = vec![];
        let mut numbers = vec![];
        for t in e.map_or(&[][..], |e| e.params) {
            let mut v = Value::default();
            v.r#type = *t;
            match *t {
                ULUA_TYPE_NUMBER => {
                    v.number = match e.map(|e| e.kind) {
                        Some(Kind::Chain) => self.rd.below(4) as f64,
                        Some(Kind::Add | Kind::UseAdd) if self.rd.one_in(4) => {
                            f64::from_bits(u64::from_le_bytes([
                                0,
                                0,
                                0,
                                0,
                                self.rd.byte(),
                                self.rd.byte(),
                                self.rd.byte(),
                                self.rd.byte(),
                            ]))
                        }
                        _ => self.rd.below(100) as f64,
                    };
                    numbers.push(v.number);
                }
                ULUA_TYPE_BOOLEAN => v.boolean = u32::from(self.rd.one_in(2)),
                _ => {
                    let s: &[u8] = [&b""[..], b"text", "é🙂".as_bytes(), b"a\0b"][self.rd.below(4)];
                    keep.push(s.to_vec());
                }
            }
            args.push(v);
        }
        for (v, k) in args
            .iter_mut()
            .filter(|v| v.r#type == ULUA_TYPE_STRING)
            .zip(&keep)
        {
            v.data = k.as_ptr().cast();
            v.length = k.len();
        }
        // An invalid argument gives INVALID_ARGUMENT before any script runs (abi.md §4, §7.7).
        let mut defect = e.is_none().then_some("an export index out of range");
        let mut null_args = false;
        let mut null_out = false;
        if self.rd.one_in(12) {
            let choice = self.rd.below(6);
            defect = Some("an invalid argument");
            match choice {
                0 => args.push(Value::default()),
                1 if !args.is_empty() => {
                    args.pop();
                }
                2 if !args.is_empty() => args[0].r#type = (args[0].r#type % 3) + 1,
                3 if !args.is_empty() => args[0].boolean = 2,
                4 => null_args = true,
                5 => null_out = true,
                _ => {
                    keep.push(b"\xff".to_vec());
                    let mut v = Value::default();
                    v.r#type = ULUA_TYPE_STRING;
                    v.data = keep[keep.len() - 1].as_ptr().cast();
                    v.length = 1;
                    args.push(v);
                }
            }
            if null_args && args.is_empty() {
                // A null pointer with count 0 is valid.
                args.push(Value::default());
            }
        }
        let diag = self.slot();
        if matches!(diag, Slot::Size | Slot::Reserved) {
            defect = defect.or(Some("an invalid diagnostic"));
        }
        let kind = e.map(|e| e.kind);
        let mut queued = VecDeque::new();
        let host_calls = match kind {
            Some(Kind::UseAdd | Kind::UseLog | Kind::Wait | Kind::Chain) => 1,
            Some(Kind::Pair) => 2,
            _ => 0,
        };
        for _ in 0..host_calls {
            queued.push_back(self.host_mode(ctx, Some(i)));
        }
        if kind == Some(Kind::Main) {
            self.output_mode(ctx, Some(i));
        }
        let host_value = self.rd.below(1000) as f64 - 500.0;
        {
            let mut s = self.state.borrow_mut();
            s.modes = queued.clone();
            let mut r = Value::default();
            r.r#type = ULUA_TYPE_NUMBER;
            r.number = host_value;
            s.result = Some(r);
        }
        let output_mode = self.state.borrow().output_modes.front().copied();
        let mut d = check::diagnostic();
        let dp = Self::diagnostic(diag, &mut d);
        let mut value = Value::default();
        let mut task = u64::MAX;
        let ap = if null_args {
            ptr::null()
        } else {
            args.as_ptr()
        };
        let inst = self.insts[i].ptr;
        // SAFETY: The instance is live; arguments and outputs are local storage or null.
        let s = unsafe {
            if sync {
                ulua_call(
                    inst,
                    index as u32,
                    ap,
                    args.len(),
                    if null_out {
                        ptr::null_mut()
                    } else {
                        &mut value
                    },
                    dp,
                )
            } else {
                ulua_call_start(
                    inst,
                    index as u32,
                    ap,
                    args.len(),
                    if null_out { ptr::null_mut() } else { &mut task },
                    dp,
                )
            }
        };
        let events = self.take_events();
        let call = if sync { "ulua_call" } else { "ulua_call_start" };
        self.nested_results(call, &events);
        if let Some(why) = defect {
            if self.expect(call, s, &[ULUA_INVALID_ARGUMENT], why)
                && (events.calls > 0 || !events.starts.is_empty() || !events.outputs.is_empty())
            {
                self.report
                    .fail("call:script-ran-on-invalid", format!("{call}: {why}"));
            }
            return;
        }
        let Some(e) = e else {
            return;
        };
        let mut sim = Sim {
            modes: queued,
            starts: events.starts.iter().map(|x| x.0).collect(),
            calls: 0,
            started: 0,
        };
        let task_id = (!sync).then_some(task);
        let (expect, state) = self.simulate(
            i,
            e.kind,
            &numbers,
            host_value,
            &mut sim,
            output_mode,
            task_id,
        );
        let sent = match e.kind {
            Kind::UseAdd => vec![vec![numbers[0], 1.0]],
            Kind::Wait => vec![vec![numbers[0]]],
            Kind::Pair => vec![vec![numbers[0]], vec![numbers[0] + 1.0]],
            Kind::Chain => vec![vec![0.0]],
            _ => vec![],
        };
        let received = &events.arguments;
        if received
            .iter()
            .zip(&sent)
            .any(|(r, s)| r.len() != s.len() || r.iter().zip(s).any(|(a, b)| !same(*a, *b)))
        {
            self.report.fail(
                format!("host:arguments:{}", e.name),
                format!("received {received:?}, sent {sent:?}"),
            );
        }
        let why = format!("export {} kind {:?} sync {sync}", e.name, e.kind);
        if sim.calls != events.calls || sim.started as usize != events.starts.len() {
            self.report.fail(
                format!("events:{call}:{}", e.name),
                format!(
                    "{why}: host calls {} starts {}, model {} {}",
                    events.calls,
                    events.starts.len(),
                    sim.calls,
                    sim.started
                ),
            );
            return;
        }
        if !self.outcome(call, s, &d, dp.is_null(), &expect, &why) {
            return;
        }
        if s != ULUA_OK {
            return;
        }
        if sync {
            let ok = match &state {
                TaskState::Number(n) => value.r#type == ULUA_TYPE_NUMBER && same(value.number, *n),
                TaskState::Typed(t) => crate::run_target::value_check(&value, *t),
                _ => false,
            };
            let echo = e.kind == Kind::Echo && {
                // SAFETY: A STRING result points to `length` bytes that the instance owns.
                let got = if value.length == 0 {
                    &[][..]
                } else {
                    unsafe { std::slice::from_raw_parts(value.data.cast::<u8>(), value.length) }
                };
                got != keep.first().map_or(&[][..], |k| k.as_slice())
            };
            let flip = e.kind == Kind::Flip && value.boolean == args[0].boolean;
            if !ok || echo || flip {
                self.report.fail(
                    format!("call:value:{}", e.name),
                    format!("{why}: {value:?}, model {state:?}"),
                );
            }
        } else {
            if !self.insts[i].seen_tasks.insert(task) {
                self.report
                    .fail("task:id-reused", format!("task id {task} given twice"));
            }
            self.insts[i].tasks.insert(task, state);
        }
    }

    /// The expectation of one export call and the state of its promise.
    #[allow(clippy::too_many_arguments)]
    fn simulate(
        &mut self,
        i: usize,
        kind: Kind,
        numbers: &[f64],
        host_value: f64,
        sim: &mut Sim,
        output: Option<Mode>,
        task: Option<u64>,
    ) -> (Expect, TaskState) {
        let n = numbers.first().copied().unwrap_or(0.0);
        let pending = || match task {
            None => Expect::Stop(ULUA_FAILED, "async:pending"),
            Some(_) => Expect::Ok,
        };
        let value = |v: f64| (Expect::Ok, TaskState::Number(v));
        match kind {
            Kind::Main => {
                if output == Some(Mode::Error) {
                    return (
                        Expect::Stop(ULUA_HOST_ERROR, "host:output"),
                        TaskState::Stuck,
                    );
                }
                self.insts[i].counter += 1.0;
                (Expect::Ok, TaskState::Typed(ULUA_TYPE_VOID))
            }
            Kind::Nop => (Expect::Ok, TaskState::Typed(ULUA_TYPE_VOID)),
            Kind::Add => value(numbers[0] + numbers[1]),
            Kind::Echo => (Expect::Ok, TaskState::Typed(ULUA_TYPE_STRING)),
            Kind::Flip => (Expect::Ok, TaskState::Typed(ULUA_TYPE_BOOLEAN)),
            Kind::Count => value(self.insts[i].counter),
            Kind::Sum => value(self.insts[i].total),
            Kind::Boom => (
                Expect::Stop(ULUA_FAILED, "trap:index-out-of-range"),
                TaskState::Stuck,
            ),
            Kind::Spin => (Expect::Stop(ULUA_FAILED, "limit:steps"), TaskState::Stuck),
            Kind::UseAdd => match self.host(i, 0, sim) {
                Ok(()) => value(host_value),
                Err(e) => (e, TaskState::Stuck),
            },
            Kind::UseLog => match self.host(i, 1, sim) {
                Ok(()) => (Expect::Ok, TaskState::Typed(ULUA_TYPE_VOID)),
                Err(e) => (e, TaskState::Stuck),
            },
            Kind::Quick => match task {
                None => value(n),
                Some(_) => (Expect::Ok, TaskState::Number(n)),
            },
            Kind::Fails => match task {
                None => (
                    Expect::Stop(ULUA_FAILED, "trap:uncaught-exception"),
                    TaskState::Stuck,
                ),
                Some(_) => (Expect::Ok, TaskState::Rejected),
            },
            Kind::Wait => match self.start(i, sim) {
                Ok(id) => {
                    self.insts[i].waiters.insert(id, Waiter::Wait(task));
                    (pending(), TaskState::Pending)
                }
                Err(e) => (e, TaskState::Stuck),
            },
            Kind::Pair => {
                let a = match self.start(i, sim) {
                    Ok(a) => a,
                    Err(e) => return (e, TaskState::Stuck),
                };
                match self.start(i, sim) {
                    Ok(b) => {
                        let w = Waiter::Pair(task, [a, b], [None, None]);
                        self.insts[i].waiters.insert(a, w.clone());
                        self.insts[i].waiters.insert(b, w);
                        (pending(), TaskState::Pending)
                    }
                    Err(e) => (e, TaskState::Stuck),
                }
            }
            Kind::Chain => {
                let count = n as u32;
                if count == 0 {
                    return match task {
                        None => value(0.0),
                        Some(_) => (Expect::Ok, TaskState::Number(0.0)),
                    };
                }
                match self.start(i, sim) {
                    Ok(id) => {
                        self.insts[i]
                            .waiters
                            .insert(id, Waiter::Chain(task, 0, count, 0.0));
                        (pending(), TaskState::Pending)
                    }
                    Err(e) => (e, TaskState::Stuck),
                }
            }
        }
    }

    /// The task query operation.
    pub fn task_query(&mut self) {
        let Some(i) = self.live_inst() else {
            return;
        };
        let id = self.task_id(i);
        let null = self.rd.one_in(20);
        let diag = self.slot();
        let mut d = check::diagnostic();
        let dp = Self::diagnostic(diag, &mut d);
        let mut v = Value::default();
        // SAFETY: The instance is live; outputs are local storage or null.
        let s = unsafe {
            ulua_task_result(
                self.insts[i].ptr,
                id,
                if null { ptr::null_mut() } else { &mut v },
                dp,
            )
        };
        let events = self.take_events();
        if events.calls > 0 || !events.outputs.is_empty() || !events.starts.is_empty() {
            self.report
                .fail("task:script-ran", "a task query ran script code");
        }
        if null || matches!(diag, Slot::Size | Slot::Reserved) {
            self.expect(
                "ulua_task_result",
                s,
                &[ULUA_INVALID_ARGUMENT],
                "an invalid output",
            );
            return;
        }
        let Some(state) = self.insts[i].tasks.get(&id).cloned() else {
            self.expect(
                "ulua_task_result",
                s,
                &[ULUA_INVALID_ARGUMENT],
                "an unknown or consumed task",
            );
            return;
        };
        let (expect, done) = match state {
            TaskState::Pending | TaskState::Stuck => (Expect::Stop(ULUA_PENDING, ""), false),
            TaskState::Rejected => (Expect::Stop(ULUA_FAILED, "trap:uncaught-exception"), true),
            _ => (Expect::Ok, true),
        };
        let why = format!("task {id} state {state:?}");
        if s == ULUA_PENDING && expect == Expect::Stop(ULUA_PENDING, "") {
        } else if !self.outcome("ulua_task_result", s, &d, dp.is_null(), &expect, &why) {
            return;
        }
        let ok = match state {
            TaskState::Number(n) => v.r#type == ULUA_TYPE_NUMBER && same(v.number, n),
            TaskState::Typed(t) => crate::run_target::value_check(&v, t),
            _ => true,
        };
        if s == ULUA_OK && !ok {
            self.report.fail("task:value", format!("{why}: {v:?}"));
        }
        if done {
            self.insts[i].tasks.remove(&id);
        }
    }

    fn task_id(&mut self, i: usize) -> u64 {
        let live: Vec<u64> = self.insts[i].tasks.keys().copied().collect();
        if !live.is_empty() && !self.rd.one_in(5) {
            let mut sorted = live;
            sorted.sort_unstable();
            return sorted[self.rd.below(sorted.len())];
        }
        let seen: Vec<u64> = self.insts[i].seen_tasks.iter().copied().collect();
        match self.rd.below(4) {
            0 if !seen.is_empty() => {
                let mut s = seen;
                s.sort_unstable();
                s[self.rd.below(s.len())]
            }
            1 => u64::MAX,
            2 => 0,
            _ => self.rd.below(64) as u64,
        }
    }

    /// The task discard operation.
    pub fn task_discard(&mut self) {
        let Some(i) = self.live_inst() else {
            return;
        };
        let id = self.task_id(i);
        // SAFETY: The instance is live.
        let s = unsafe { ulua_task_discard(self.insts[i].ptr, id) };
        let live = self.insts[i].tasks.remove(&id).is_some();
        let want = if live { ULUA_OK } else { ULUA_INVALID_ARGUMENT };
        self.expect(
            "ulua_task_discard",
            s,
            &[want],
            &format!("task {id} live {live}"),
        );
    }

    fn request_id(&mut self, i: usize) -> u64 {
        let inst = &self.insts[i];
        let unrecorded: Vec<u64> = inst
            .open
            .iter()
            .copied()
            .filter(|id| !inst.recorded.iter().any(|r| r.0 == *id))
            .collect();
        if !unrecorded.is_empty() && self.rd.below(6) != 0 {
            return unrecorded[self.rd.below(unrecorded.len())];
        }
        let seen: Vec<u64> = {
            let mut s: Vec<u64> = inst.seen_requests.iter().copied().collect();
            s.sort_unstable();
            s
        };
        match self.rd.below(4) {
            0 if !seen.is_empty() => seen[self.rd.below(seen.len())],
            1 => u64::MAX,
            2 => 0,
            _ => seen.last().map_or(1, |m| m + 1),
        }
    }

    /// The completion operation: `ulua_request_complete` or `ulua_request_fail`.
    pub fn record(&mut self) {
        let Some(i) = self.live_inst() else {
            return;
        };
        let id = self.request_id(i);
        let inst = &self.insts[i];
        let open = inst.open.contains(&id) && !inst.recorded.iter().any(|r| r.0 == id);
        let ptr_i = inst.ptr;
        let fail = self.rd.one_in(4);
        let number = match self.rd.below(10) {
            0 => f64::NAN,
            1 => -0.0,
            2 => 1e300,
            3 => f64::NEG_INFINITY,
            _ => self.rd.below(50) as f64,
        };
        let mut v = Value::default();
        v.r#type = ULUA_TYPE_NUMBER;
        v.number = number;
        let mut valid = open;
        let mut null = false;
        match self.rd.below(12) {
            0 => {
                v.r#type = ULUA_TYPE_STRING;
                v.data = b"x".as_ptr().cast();
                v.length = 1;
                valid = false;
            }
            1 => {
                v.boolean = 1;
                valid = false;
            }
            2 => {
                null = true;
                valid = false;
            }
            _ => {}
        }
        // SAFETY: The instance is live; the value is local storage or null.
        let s = unsafe {
            if fail {
                ulua_request_fail(ptr_i, id)
            } else {
                ulua_request_complete(ptr_i, id, if null { ptr::null() } else { &v })
            }
        };
        let valid = if fail { open } else { valid };
        let call = if fail {
            "ulua_request_fail"
        } else {
            "ulua_request_complete"
        };
        let want = if valid {
            ULUA_OK
        } else {
            ULUA_INVALID_ARGUMENT
        };
        if self.expect(
            call,
            s,
            &[want],
            &format!("request {id} open {open} valid {valid}"),
        ) && s == ULUA_OK
        {
            self.insts[i]
                .recorded
                .push_back((id, (!fail).then_some(number)));
        }
    }

    /// The pump operation.
    pub fn pump(&mut self) {
        let Some(i) = self.live_inst() else {
            return;
        };
        let ctx = self.insts[i].ctx;
        let max = [0u32, 1, 1, 2, 3][self.rd.below(5)];
        let null = self.rd.one_in(24);
        let diag = self.slot();
        let mut queued = VecDeque::new();
        for _ in 0..4 {
            queued.push_back(self.host_mode(ctx, Some(i)));
        }
        self.state.borrow_mut().modes = queued.clone();
        let mut d = check::diagnostic();
        let dp = Self::diagnostic(diag, &mut d);
        let mut settled = u32::MAX;
        // SAFETY: The instance is live; outputs are local storage or null.
        let s = unsafe {
            ulua_pump(
                self.insts[i].ptr,
                max,
                if null { ptr::null_mut() } else { &mut settled },
                dp,
            )
        };
        let events = self.take_events();
        self.nested_results("ulua_pump", &events);
        if null || matches!(diag, Slot::Size | Slot::Reserved) {
            self.expect(
                "ulua_pump",
                s,
                &[ULUA_INVALID_ARGUMENT],
                "an invalid output",
            );
            return;
        }
        let mut sim = Sim {
            modes: queued,
            starts: events.starts.iter().map(|x| x.0).collect(),
            calls: 0,
            started: 0,
        };
        let mut count = 0u32;
        let mut expect = Expect::Ok;
        while max == 0 || count < max {
            let Some((id, value)) = self.insts[i].recorded.pop_front() else {
                break;
            };
            count += 1;
            self.insts[i].open.retain(|o| *o != id);
            let waiter = self.insts[i].waiters.remove(&id);
            let Some(v) = value else {
                if let Some(w) = waiter {
                    self.stuck(i, &w);
                }
                expect = Expect::Stop(ULUA_HOST_ERROR, "host:request");
                break;
            };
            if let Err(e) = self.settle(i, id, v, waiter, &mut sim) {
                expect = e;
                break;
            }
        }
        let why = format!("max {max}, model settles {count}");
        if sim.started as usize != events.starts.len() || events.calls > 0 {
            self.report.fail(
                "events:ulua_pump",
                format!(
                    "{why}: starts {}, model {}",
                    events.starts.len(),
                    sim.started
                ),
            );
            return;
        }
        if !self.outcome("ulua_pump", s, &d, dp.is_null(), &expect, &why) {
            return;
        }
        if settled != count {
            self.report
                .fail("pump:settled", format!("{why}: settled {settled}"));
        }
    }

    fn stuck(&mut self, i: usize, w: &Waiter) {
        let task = match w {
            Waiter::Wait(t) | Waiter::Chain(t, ..) => *t,
            Waiter::Pair(t, ids, _) => {
                for id in ids {
                    self.insts[i].waiters.remove(id);
                }
                *t
            }
        };
        if let Some(t) = task
            && let Some(state) = self.insts[i].tasks.get_mut(&t)
        {
            *state = TaskState::Stuck;
        }
    }

    fn fulfil(&mut self, i: usize, task: Option<u64>, v: f64) {
        if let Some(t) = task
            && let Some(state) = self.insts[i].tasks.get_mut(&t)
        {
            *state = TaskState::Number(v);
        }
    }

    /// One settle of a value in the simulation.
    fn settle(
        &mut self,
        i: usize,
        id: u64,
        v: f64,
        waiter: Option<Waiter>,
        sim: &mut Sim,
    ) -> Result<(), Expect> {
        match waiter {
            None => Ok(()),
            Some(Waiter::Wait(task)) => {
                self.insts[i].total += v;
                self.fulfil(i, task, v);
                Ok(())
            }
            Some(Waiter::Pair(task, ids, mut got)) => {
                let slot = usize::from(ids[1] == id);
                got[slot] = Some(v);
                let other = ids[1 - slot];
                match got {
                    [Some(a), Some(b)] => self.fulfil(i, task, a + b),
                    _ => {
                        self.insts[i]
                            .waiters
                            .insert(other, Waiter::Pair(task, ids, got));
                    }
                }
                Ok(())
            }
            Some(Waiter::Chain(task, index, count, sum)) => {
                let sum = sum + v;
                if index + 1 >= count {
                    self.fulfil(i, task, sum);
                    return Ok(());
                }
                match self.start(i, sim) {
                    Ok(next) => {
                        self.insts[i]
                            .waiters
                            .insert(next, Waiter::Chain(task, index + 1, count, sum));
                        Ok(())
                    }
                    Err(e) => {
                        self.stuck(i, &Waiter::Chain(task, index, count, sum));
                        Err(e)
                    }
                }
            }
        }
    }

    /// The count operation: `ulua_instance_requests` against the model.
    pub fn counts(&mut self) {
        let Some(i) = self.live_inst() else {
            return;
        };
        let null = self.rd.one_in(16);
        let (mut open, mut recorded) = (u32::MAX, u32::MAX);
        // SAFETY: The instance is live; outputs are local storage or null.
        let s = unsafe {
            ulua_instance_requests(
                self.insts[i].ptr,
                if null { ptr::null_mut() } else { &mut open },
                &mut recorded,
            )
        };
        let want = if null { ULUA_INVALID_ARGUMENT } else { ULUA_OK };
        if !self.expect("ulua_instance_requests", s, &[want], "count outputs") || null {
            return;
        }
        let inst = &self.insts[i];
        if open as usize != inst.open.len() || recorded as usize != inst.recorded.len() {
            let detail = format!(
                "counts ({open}, {recorded}), model ({}, {})",
                inst.open.len(),
                inst.recorded.len()
            );
            self.report.fail("requests:count", detail);
        }
    }
}
