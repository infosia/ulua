//! Fuzzer host callbacks: output, sync host functions, and async starts. The fuzzer
//! chooses each result. Each callback records what it did, so a status can be checked
//! against the event that justifies it.
use crate::rng::Rng;
use std::{
    cell::RefCell,
    collections::VecDeque,
    ffi::{c_char, c_void},
};
use ulua::*;

/// What one callback does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// Return 0 with a valid result.
    Ok,
    /// Return a non-zero status.
    Error,
    /// Return 0 with a result that breaks abi.md §6.2.4.
    Bad,
    /// Run one nested ABI call, then return 0 with a valid result.
    Nested,
}

/// A nested ABI call: its name, its status, and the statuses that abi.md allows.
pub type Nested = Box<dyn FnMut() -> (String, Status, Vec<Status>)>;

/// The events of one ABI call.
#[derive(Default, Debug)]
pub struct Events {
    /// Sync host calls.
    pub calls: u32,
    /// Sync host calls that returned non-zero.
    pub errors: u32,
    /// Sync host results that break the rule.
    pub bad: u32,
    /// Output calls that returned non-zero.
    pub output_fail: u32,
    /// Accepted starts: request id and result type.
    pub starts: Vec<(u64, u32)>,
    /// Starts that returned non-zero.
    pub start_fail: u32,
    /// A callback after a stopping return in the same ABI call.
    pub after_stop: Vec<String>,
    /// Each violation that a callback sees (an argument outside the signature,
    /// a call of a revoked capability).
    pub violations: Vec<String>,
    /// Output texts.
    pub outputs: Vec<Vec<u8>>,
    /// Arguments of sync host calls and starts, per call.
    pub arguments: Vec<Vec<f64>>,
    /// Nested calls: name, status, allowed statuses.
    pub nested: Vec<(String, Status, Vec<Status>)>,
    /// True after a stopping return.
    pub stopped: bool,
}

/// The state that the callbacks share. The fuzzer reads and resets `events` per ABI call.
pub struct HostState {
    /// The choice source when no queued mode applies.
    pub rng: Rng,
    /// One in `n` random callbacks gives `Error` (0: never).
    pub error_one_in: usize,
    /// One in `n` random sync results gives `Bad` (0: never).
    pub bad_one_in: usize,
    /// One in `n` output calls returns non-zero (0: never).
    pub output_fail_one_in: usize,
    /// Queued modes for the next host calls and starts; a queued mode wins over the
    /// random choice.
    pub modes: VecDeque<Mode>,
    /// Queued modes for the next output calls.
    pub output_modes: VecDeque<Mode>,
    /// Queued nested calls for `Mode::Nested`.
    pub nested: VecDeque<Nested>,
    /// The value that the next sync result of each type carries, when set.
    pub result: Option<Value>,
    /// The events of the current ABI call.
    pub events: Events,
    buffer: Vec<u8>,
}

/// One capability binding. The descriptor userdata points to it.
pub struct Bound {
    /// The capability name.
    pub name: String,
    /// Parameter types.
    pub parameters: Vec<u32>,
    /// Result type.
    pub result: u32,
    /// True for an async capability.
    pub asynchronous: bool,
    /// True after the fuzzer revokes or frees the capability.
    pub revoked: std::cell::Cell<bool>,
    /// The shared state.
    pub state: *const RefCell<HostState>,
}

impl HostState {
    /// A state with no failure modes.
    pub fn new(rng: Rng) -> Self {
        Self {
            rng,
            error_one_in: 0,
            bad_one_in: 0,
            output_fail_one_in: 0,
            modes: VecDeque::new(),
            output_modes: VecDeque::new(),
            nested: VecDeque::new(),
            result: None,
            events: Events::default(),
            buffer: vec![],
        }
    }
    /// Take the events of the finished ABI call.
    pub fn take(&mut self) -> Events {
        std::mem::take(&mut self.events)
    }
    fn mode(&mut self, sync_result: bool) -> Mode {
        if let Some(m) = self.modes.pop_front() {
            return m;
        }
        if self.error_one_in > 0 && self.rng.one_in(self.error_one_in) {
            Mode::Error
        } else if sync_result && self.bad_one_in > 0 && self.rng.one_in(self.bad_one_in) {
            Mode::Bad
        } else {
            Mode::Ok
        }
    }
    fn nested(&mut self) {
        if let Some(mut call) = self.nested.pop_front() {
            let outcome = call();
            self.events.nested.push(outcome);
        }
    }
    fn begin(&mut self, what: &str) {
        if self.events.stopped {
            self.events.after_stop.push(what.to_owned());
        }
    }
}

/// A capability descriptor for a binding, with the fuzzer callbacks.
pub fn descriptor(b: &Bound) -> CapabilityDesc {
    let mut d = CapabilityDesc::default();
    d.name = b.name.as_ptr().cast();
    d.name_length = b.name.len();
    d.parameter_types = b.parameters.as_ptr();
    d.parameter_count = b.parameters.len();
    d.result_type = b.result;
    if b.asynchronous {
        d.start = Some(start);
    } else {
        d.function = Some(function);
    }
    d.userdata = std::ptr::from_ref(b).cast_mut().cast();
    d
}

const STRINGS: &[&str] = &["", "é", "🙂", "x\0y", "host", "\u{10FFFF}"];

fn valid_value(state: &mut HostState, ty: u32) -> Value {
    let mut v = Value::default();
    v.r#type = ty;
    if let Some(r) = state.result
        && r.r#type == ty
    {
        return r;
    }
    match ty {
        ULUA_TYPE_NUMBER => v.number = state.rng.number(),
        ULUA_TYPE_BOOLEAN => v.boolean = u32::from(state.rng.one_in(2)),
        ULUA_TYPE_STRING => {
            state.buffer.clear();
            if state.rng.one_in(8) {
                state
                    .buffer
                    .extend(std::iter::repeat_n(b'w', state.rng.range(64, 4096)));
            } else {
                state
                    .buffer
                    .extend_from_slice(state.rng.pick(STRINGS).as_bytes());
            }
            v.data = state.buffer.as_ptr().cast();
            v.length = state.buffer.len();
        }
        _ => {}
    }
    v
}

fn bad_value(state: &mut HostState, ty: u32) -> Value {
    let mut v = valid_value(state, ty);
    match state.rng.below(4) {
        0 => v.r#type = if ty == ULUA_TYPE_STRING { 1 } else { ty + 1 },
        1 => v.r#type = 9,
        2 if ty == ULUA_TYPE_STRING => {
            state.buffer.clear();
            state.buffer.extend_from_slice(b"a\xffb");
            v.data = state.buffer.as_ptr().cast();
            v.length = 3;
        }
        2 if ty == ULUA_TYPE_BOOLEAN => v.boolean = 2,
        _ => {
            if ty == ULUA_TYPE_NUMBER || ty == ULUA_TYPE_VOID {
                v.boolean = 1;
            } else {
                v.number = 1.0;
            }
        }
    }
    v
}

/// A valid value of a type for an ABI argument. `keep` owns the string bytes.
pub fn valid_value_for(rng: &mut Rng, ty: u32, keep: &mut Vec<Vec<u8>>) -> Value {
    let mut v = Value::default();
    v.r#type = ty;
    match ty {
        ULUA_TYPE_NUMBER => v.number = rng.number(),
        ULUA_TYPE_BOOLEAN => v.boolean = u32::from(rng.one_in(2)),
        ULUA_TYPE_STRING => {
            keep.push(rng.pick(STRINGS).as_bytes().to_vec());
            let b = keep.last().map_or(&[][..], |b| b.as_slice());
            v.data = b.as_ptr().cast();
            v.length = b.len();
        }
        _ => {}
    }
    v
}

/// Check the arguments of a host call against the signature (abi.md §4, §6.2.3).
unsafe fn arguments(b: &Bound, state: &mut HostState, args: *const Value, count: usize) {
    if count != b.parameters.len() || (count > 0 && args.is_null()) {
        state
            .events
            .violations
            .push(format!("{}: {count} arguments", b.name));
        return;
    }
    let mut numbers = vec![];
    for (i, ty) in b.parameters.iter().enumerate() {
        // SAFETY: The runtime passes `count` readable values for the duration of the call.
        let v = unsafe { *args.add(i) };
        let text_ok = v.r#type != ULUA_TYPE_STRING || {
            // SAFETY: A STRING argument points to `length` readable bytes during the call.
            let bytes = if v.length == 0 {
                Some(&[][..])
            } else if v.data.is_null() {
                None
            } else {
                Some(unsafe { std::slice::from_raw_parts(v.data.cast::<u8>(), v.length) })
            };
            bytes.is_some_and(|b| std::str::from_utf8(b).is_ok())
        };
        if v.r#type != *ty || !text_ok {
            state
                .events
                .violations
                .push(format!("{}: argument {i} breaks the signature", b.name));
        }
        numbers.push(v.number);
    }
    state.events.arguments.push(numbers);
}

unsafe extern "C" fn function(
    userdata: *mut c_void,
    args: *const Value,
    count: usize,
    result: *mut Value,
) -> i32 {
    // SAFETY: The descriptor userdata is a live Bound; its state outlives every run.
    let b = unsafe { &*userdata.cast::<Bound>() };
    // SAFETY: The state pointer stays valid while any capability that names it lives.
    let Ok(mut state) = (unsafe { &*b.state }).try_borrow_mut() else {
        return 1;
    };
    state.begin(&b.name);
    state.events.calls += 1;
    if b.revoked.get() {
        state
            .events
            .violations
            .push(format!("{}: called after revocation", b.name));
    }
    // SAFETY: The runtime passes `count` readable arguments.
    unsafe { arguments(b, &mut state, args, count) };
    let mode = state.mode(true);
    if mode == Mode::Nested {
        // A nested call can run another callback; the state stays borrowed meanwhile,
        // and a callback that finds it borrowed returns 1.
        state.nested();
    }
    match mode {
        Mode::Error => {
            state.events.errors += 1;
            state.events.stopped = true;
            1
        }
        Mode::Bad => {
            state.events.bad += 1;
            state.events.stopped = true;
            let v = bad_value(&mut state, b.result);
            // SAFETY: The runtime passes writable result storage.
            unsafe { result.write(v) };
            0
        }
        _ => {
            let v = valid_value(&mut state, b.result);
            // SAFETY: The runtime passes writable result storage.
            unsafe { result.write(v) };
            0
        }
    }
}

unsafe extern "C" fn start(
    userdata: *mut c_void,
    args: *const Value,
    count: usize,
    request: u64,
) -> i32 {
    // SAFETY: The descriptor userdata is a live Bound; its state outlives every run.
    let b = unsafe { &*userdata.cast::<Bound>() };
    // SAFETY: The state pointer stays valid while any capability that names it lives.
    let Ok(mut state) = (unsafe { &*b.state }).try_borrow_mut() else {
        return 1;
    };
    state.begin(&b.name);
    if b.revoked.get() {
        state
            .events
            .violations
            .push(format!("{}: started after revocation", b.name));
    }
    // SAFETY: The runtime passes `count` readable arguments.
    unsafe { arguments(b, &mut state, args, count) };
    let mode = state.mode(false);
    if mode == Mode::Nested {
        state.nested();
    }
    if mode == Mode::Error {
        state.events.start_fail += 1;
        state.events.stopped = true;
        1
    } else {
        state.events.starts.push((request, b.result));
        0
    }
}

/// The output callback. Its userdata is the shared state.
pub unsafe extern "C" fn output(userdata: *mut c_void, text: *const c_char, length: usize) -> i32 {
    // SAFETY: The host userdata is the live shared state of this run.
    let cell = unsafe { &*userdata.cast::<RefCell<HostState>>() };
    let Ok(mut state) = cell.try_borrow_mut() else {
        return 1;
    };
    state.begin("print");
    let bytes = if length == 0 {
        vec![]
    } else {
        // SAFETY: The text is readable for `length` bytes until this callback returns.
        unsafe { std::slice::from_raw_parts(text.cast::<u8>(), length) }.to_vec()
    };
    if std::str::from_utf8(&bytes).is_err() {
        state
            .events
            .violations
            .push("print: output is not UTF-8".into());
    }
    state.events.outputs.push(bytes);
    let mode = match state.output_modes.pop_front() {
        Some(m) => m,
        None if state.output_fail_one_in > 0 && {
            let n = state.output_fail_one_in;
            state.rng.one_in(n)
        } =>
        {
            Mode::Error
        }
        None => Mode::Ok,
    };
    if mode == Mode::Nested {
        state.nested();
    }
    if mode == Mode::Error {
        state.events.output_fail += 1;
        state.events.stopped = true;
        1
    } else {
        0
    }
}

/// A host descriptor whose userdata is the shared state.
pub fn host(state: &RefCell<HostState>) -> Host {
    let mut h = Host::default();
    h.userdata = std::ptr::from_ref(state).cast_mut().cast();
    h.output = Some(output);
    h
}
