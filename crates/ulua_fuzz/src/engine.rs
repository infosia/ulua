//! The `abi` and `requests` targets: an input is a byte string that chooses a sequence
//! of ABI calls over a pool of live objects (and objects whose parent is freed), with
//! valid and invalid arguments. A model gives the documented status of each call.
//! Freed pointers are never passed: abi.md §7.1 makes them invalid (a host precondition).
use crate::{
    check::{self, Report},
    host::{Bound, HostState, descriptor},
    model::{self, Init, NAMES, PROGRAMS, Sig, TaskState, Waiter},
    rng::Rng,
};
use std::{
    cell::RefCell,
    collections::{HashMap, HashSet, VecDeque},
    ptr,
};
use ulua::*;

/// The choice source: the input bytes, then zeros.
pub struct Reader<'a> {
    data: &'a [u8],
    at: usize,
}

impl<'a> From<&'a [u8]> for Reader<'a> {
    fn from(data: &'a [u8]) -> Self {
        Self { data, at: 0 }
    }
}

impl Reader<'_> {
    /// The next byte.
    pub fn byte(&mut self) -> u8 {
        let b = self.data.get(self.at).copied().unwrap_or(0);
        self.at += 1;
        b
    }
    /// A value in `0..n`.
    pub fn below(&mut self, n: usize) -> usize {
        match n {
            0 => 0,
            1..=256 => self.byte() as usize % n,
            _ => (self.byte() as usize | (self.byte() as usize) << 8) % n,
        }
    }
    /// True with probability about `1 / n`.
    pub fn one_in(&mut self, n: usize) -> bool {
        self.below(n) == 0
    }
    /// True after the last byte.
    pub fn done(&self) -> bool {
        self.at >= self.data.len()
    }
}

/// A context of the pool.
pub struct MCtx {
    /// The pointer.
    pub ptr: *mut Context,
    /// False after `ulua_context_free`.
    pub live: bool,
}

/// A capability of the pool.
pub struct MCap {
    /// The pointer.
    pub ptr: *mut Capability,
    /// The context index.
    pub ctx: usize,
    /// The name.
    pub name: String,
    /// The signature.
    pub sig: Sig,
    /// The binding that the descriptor userdata points to.
    pub bound: Box<Bound>,
    /// False after `ulua_capability_free` or the free of its context.
    pub live: bool,
}

/// A module of the pool.
pub struct MMod {
    /// The pointer.
    pub ptr: *mut Module,
    /// The context index.
    pub ctx: usize,
    /// The program index.
    pub program: usize,
    /// The capabilities that the script calls, by pool index.
    pub held: Vec<usize>,
    /// False after a free.
    pub live: bool,
}

/// An instance of the pool and its model state.
pub struct MInst {
    /// The pointer.
    pub ptr: *mut Instance,
    /// The context index.
    pub ctx: usize,
    /// The program index.
    pub program: usize,
    /// The capabilities that the script calls.
    pub held: Vec<usize>,
    /// `counter` of program 0.
    pub counter: f64,
    /// `total` of program 2.
    pub total: f64,
    /// Open request ids.
    pub open: Vec<u64>,
    /// Recorded completions in record order; `None` is a failure.
    pub recorded: VecDeque<(u64, Option<f64>)>,
    /// What each open request resumes.
    pub waiters: HashMap<u64, Waiter>,
    /// Live tasks.
    pub tasks: HashMap<u64, TaskState>,
    /// Every task id given.
    pub seen_tasks: HashSet<u64>,
    /// Every request id given.
    pub seen_requests: HashSet<u64>,
    /// The `max_open_requests` of the instance.
    pub max_open: u32,
    /// False after a free.
    pub live: bool,
}

/// The engine of one input.
pub struct Engine<'r> {
    /// The choice source.
    pub rd: Reader<'r>,
    /// The callback state.
    pub state: Box<RefCell<HostState>>,
    /// Contexts.
    pub ctxs: Vec<MCtx>,
    /// Capabilities.
    pub caps: Vec<MCap>,
    /// Modules.
    pub mods: Vec<MMod>,
    /// Instances.
    pub insts: Vec<MInst>,
    /// The failures.
    pub report: &'r mut Report,
    /// True for the `requests` profile.
    pub requests: bool,
    /// True when a compile gave `ULUA_OK`.
    pub compiled: bool,
}

impl Drop for Engine<'_> {
    fn drop(&mut self) {
        for c in &mut self.ctxs {
            if c.live {
                // SAFETY: A live pool context is freed once; its children go with it.
                unsafe { ulua_context_free(c.ptr) };
                c.live = false;
            }
        }
    }
}

/// Run one input. Returns true when a compile gave `ULUA_OK`.
pub fn run(requests: bool, input: &[u8], report: &mut Report) -> bool {
    let mut e = Engine::new(requests, input, report);
    let mut steps = 0;
    while !e.rd.done() && e.report.failures.is_empty() && steps < 600 {
        e.step();
        steps += 1;
    }
    e.compiled
}

/// The validity variant of a descriptor pointer argument.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Slot {
    /// A null pointer.
    Null,
    /// A valid descriptor.
    Valid,
    /// A wrong `struct_size`.
    Size,
    /// A non-zero reserved field.
    Reserved,
}

impl<'r> Engine<'r> {
    /// An engine over one input, after the setup of its profile.
    pub fn new(requests: bool, input: &'r [u8], report: &'r mut Report) -> Self {
        let seed = input
            .iter()
            .fold(0u64, |h, b| h.rotate_left(5) ^ u64::from(*b));
        let mut e = Engine {
            rd: Reader { data: input, at: 0 },
            state: Box::new(RefCell::new(HostState::new(Rng::new(seed)))),
            ctxs: vec![],
            caps: vec![],
            mods: vec![],
            insts: vec![],
            report,
            requests,
            compiled: false,
        };
        e.setup();
        e
    }
    /// Record a failure when the status is not in the allowed set.
    pub fn expect(&mut self, call: &str, got: Status, allowed: &[Status], why: &str) -> bool {
        check::note(format!("{call}:{}", check::status_name(got)));
        if allowed.contains(&got) {
            return true;
        }
        let want: Vec<_> = allowed.iter().map(|s| check::status_name(*s)).collect();
        self.report.fail(
            format!("status:{call}:{}", check::status_name(got)),
            format!(
                "{call}: status {}, the model allows {want:?}: {why}",
                check::status_name(got)
            ),
        );
        false
    }
    /// A diagnostic slot choice. Rare invalid descriptors keep most calls valid.
    pub fn slot(&mut self) -> Slot {
        match self.rd.below(24) {
            0 => Slot::Size,
            1 => Slot::Reserved,
            2..=5 => Slot::Null,
            _ => Slot::Valid,
        }
    }
    /// A diagnostic for a slot choice. Null is valid for a diagnostic (abi.md §3).
    pub fn diagnostic(slot: Slot, d: &mut Diagnostic) -> *mut Diagnostic {
        match slot {
            Slot::Null => ptr::null_mut(),
            Slot::Size => {
                d.struct_size += 1;
                d
            }
            Slot::Reserved => {
                d.reserved = 1;
                d
            }
            Slot::Valid => d,
        }
    }
    /// Check a written diagnostic: its code, and the fields of a run diagnostic.
    pub fn written(&mut self, call: &str, d: &Diagnostic, code: &str, run: bool) {
        match check::decode(d) {
            Ok(x) if x.code == code || (code == "*" && x.code != "unset") => {
                check::note(format!("{call}:{}", x.code));
                if run {
                    check::run_diagnostic(self.report, call, &x);
                }
            }
            Ok(x) => self.report.fail(
                format!("code:{call}:{}", x.code),
                format!(
                    "{call}: code {} ({}), the model expects {code}",
                    x.code, x.message
                ),
            ),
            Err(e) => self.report.fail(format!("diagnostic:{call}"), e),
        }
    }
    /// A live context.
    pub fn live_ctx(&mut self) -> Option<usize> {
        let live: Vec<_> = (0..self.ctxs.len())
            .filter(|i| self.ctxs[*i].live)
            .collect();
        (!live.is_empty()).then(|| live[self.rd.below(live.len())])
    }
    /// A live capability, optionally of one context.
    pub fn live_cap(&mut self, ctx: Option<usize>) -> Option<usize> {
        let live: Vec<_> = (0..self.caps.len())
            .filter(|i| self.caps[*i].live && ctx.is_none_or(|c| self.caps[*i].ctx == c))
            .collect();
        (!live.is_empty()).then(|| live[self.rd.below(live.len())])
    }
    /// A live module, optionally of one context.
    pub fn live_mod(&mut self, ctx: Option<usize>) -> Option<usize> {
        let live: Vec<_> = (0..self.mods.len())
            .filter(|i| self.mods[*i].live && ctx.is_none_or(|c| self.mods[*i].ctx == c))
            .collect();
        (!live.is_empty()).then(|| live[self.rd.below(live.len())])
    }
    /// A live instance.
    pub fn live_inst(&mut self) -> Option<usize> {
        let live: Vec<_> = (0..self.insts.len())
            .filter(|i| self.insts[*i].live)
            .collect();
        (!live.is_empty()).then(|| live[self.rd.below(live.len())])
    }

    fn setup(&mut self) {
        self.context_new(false);
        if self.requests {
            self.setup_async();
        }
    }

    /// A fresh `hostLater` capability, program 2 compiled with it, and an instance.
    fn setup_async(&mut self) {
        let Some(ctx) = self.live_ctx() else {
            return self.context_new(false);
        };
        self.capability(ctx, 2, None, false);
        let cap = self.caps.len() - 1;
        if !self.caps[cap].live || self.caps[cap].ctx != ctx {
            return;
        }
        self.compile_with(ctx, 2, vec![cap], None);
        let m = self.mods.len().checked_sub(1);
        if let Some(m) = m.filter(|m| self.mods[*m].live && self.mods[*m].held == [cap]) {
            self.instance_with(ctx, m, Slot::Valid, 0, true);
        }
    }

    fn step(&mut self) {
        let op = if self.requests {
            // Async starts, completions, failures, pumps, queries, discards, revocations, frees.
            match self.rd.below(64) {
                0..=19 => 20,
                20..=27 => 21,
                28..=37 => 22,
                38..=41 => 23,
                42..=51 => 24,
                52..=53 => 25,
                54 => 4,
                55 => 5,
                56 => 14,
                57 => 11,
                58 => 6,
                59..=60 => 28,
                _ => 26,
            }
        } else {
            self.rd.below(27)
        };
        match op {
            0 => self.context_new(true),
            1 => self.context_free(),
            2..=3 => self.capability_new(),
            4 => self.capability_revoke(),
            5 => self.capability_free(),
            6 => self.declarations(),
            7..=9 => self.compile(),
            10 => self.module_free(),
            11 => self.queries(),
            12..=13 => self.instance_new(),
            14 => self.instance_free(),
            15 => self.defaults(),
            16..=20 => self.call(),
            21 => self.task_query(),
            22 => self.record(),
            23 => self.task_discard(),
            24 => self.pump(),
            25 => self.counts(),
            28 => self.setup_async(),
            _ => self.record(),
        }
    }

    fn context_new(&mut self, variants: bool) {
        if self.ctxs.iter().filter(|c| c.live).count() >= 3 {
            return;
        }
        let null = variants && self.rd.one_in(8);
        let mut out = ptr::null_mut();
        // SAFETY: The output is local storage or null.
        let s = unsafe { ulua_context_new(if null { ptr::null_mut() } else { &mut out }) };
        let want = if null { ULUA_INVALID_ARGUMENT } else { ULUA_OK };
        if self.expect("ulua_context_new", s, &[want], "out pointer") && s == ULUA_OK {
            self.ctxs.push(MCtx {
                ptr: out,
                live: true,
            });
        }
    }

    fn context_free(&mut self) {
        let Some(c) = self.live_ctx().filter(|_| !self.rd.one_in(8)) else {
            // SAFETY: Null is valid for a free function (abi.md §1.9).
            let s = unsafe { ulua_context_free(ptr::null_mut()) };
            self.expect("ulua_context_free", s, &[ULUA_OK], "null");
            return;
        };
        // SAFETY: A live pool context.
        let s = unsafe { ulua_context_free(self.ctxs[c].ptr) };
        self.expect("ulua_context_free", s, &[ULUA_OK], "a live context");
        self.ctxs[c].live = false;
        for cap in self.caps.iter_mut().filter(|x| x.ctx == c) {
            cap.live = false;
            cap.bound.revoked.set(true);
        }
        for m in self.mods.iter_mut().filter(|x| x.ctx == c) {
            m.live = false;
        }
        for i in self.insts.iter_mut().filter(|x| x.ctx == c) {
            i.live = false;
        }
    }

    /// Create a capability with a name and a signature; `bad` breaks one descriptor field.
    fn capability(&mut self, ctx: usize, name: usize, sig: Option<Sig>, bad: bool) {
        let (bytes, valid_name) = NAMES[name];
        let text = String::from_utf8_lossy(bytes).into_owned();
        let sig = sig.or_else(|| model::need(&text)).unwrap_or(Sig {
            params: vec![ULUA_TYPE_NUMBER],
            result: ULUA_TYPE_NUMBER,
            asynchronous: false,
        });
        let bound = Box::new(Bound {
            name: text.clone(),
            parameters: sig.params.clone(),
            result: sig.result,
            asynchronous: sig.asynchronous,
            revoked: std::cell::Cell::new(false),
            state: &*self.state,
        });
        let mut d = descriptor(&bound);
        d.name = bytes.as_ptr().cast();
        d.name_length = bytes.len();
        let mut valid = valid_name;
        let mut why = format!("name {text:?} valid={valid_name}");
        if bad {
            valid = false;
            let choice = self.rd.below(11);
            why = format!("descriptor variant {choice}");
            match choice {
                0 => d.struct_size -= 1,
                1 => d.reserved = 1,
                2 => d.reserved2 = 1,
                3 => {
                    d.parameter_count = ULUA_MAX_HOST_PARAMETERS + 1;
                    d.parameter_types = ZEROS.as_ptr();
                }
                4 => d.result_type = 4,
                5 => {
                    d.function = Some(noop);
                    d.start = Some(noop_start);
                }
                6 => {
                    d.function = None;
                    d.start = None;
                }
                7 => d.name = ptr::null(),
                8 if d.parameter_count > 0 => d.parameter_types = ptr::null(),
                9 if d.parameter_count > 0 => d.parameter_types = ZEROS.as_ptr(),
                _ => d.name_length = 0,
            }
        }
        let mut out = ptr::null_mut();
        // SAFETY: The descriptor storage and the static name are live through the call.
        let s = unsafe { ulua_capability_new(self.ctxs[ctx].ptr, &d, &mut out) };
        let want = if valid {
            ULUA_OK
        } else {
            ULUA_INVALID_ARGUMENT
        };
        if self.expect("ulua_capability_new", s, &[want], &why) && s == ULUA_OK {
            self.caps.push(MCap {
                ptr: out,
                ctx,
                name: text,
                sig,
                bound,
                live: true,
            });
        }
    }

    fn capability_new(&mut self) {
        let Some(ctx) = self.live_ctx() else {
            return;
        };
        if self.rd.one_in(24) {
            let d = CapabilityDesc::default();
            let mut out = ptr::null_mut();
            let which = self.rd.below(3);
            // SAFETY: Each pointer is null or local storage.
            let s = unsafe {
                match which {
                    0 => ulua_capability_new(ptr::null_mut(), &d, &mut out),
                    1 => ulua_capability_new(self.ctxs[ctx].ptr, ptr::null(), &mut out),
                    _ => ulua_capability_new(self.ctxs[ctx].ptr, &d, ptr::null_mut()),
                }
            };
            self.expect(
                "ulua_capability_new",
                s,
                &[ULUA_INVALID_ARGUMENT],
                "a null pointer",
            );
            return;
        }
        let name = if self.rd.one_in(2) {
            self.rd.below(4)
        } else {
            self.rd.below(NAMES.len())
        };
        let need = model::need(&String::from_utf8_lossy(NAMES[name].0));
        let sig = if let Some(mut wrong) = need.clone().filter(|_| self.rd.one_in(6)) {
            // A needed name gets a first parameter of the other type: the call of the
            // program then has no lowering, whatever the checker does with the rest.
            wrong.params[0] = if wrong.params[0] == ULUA_TYPE_STRING {
                ULUA_TYPE_NUMBER
            } else {
                ULUA_TYPE_STRING
            };
            Some(wrong)
        } else if self.rd.one_in(6) && need.is_none() {
            let n = self.rd.below(4);
            let params = (0..n).map(|_| 1 + self.rd.below(3) as u32).collect();
            Some(Sig {
                params,
                result: self.rd.below(4) as u32,
                asynchronous: self.rd.one_in(2),
            })
        } else {
            None
        };
        let bad = self.rd.one_in(8);
        self.capability(ctx, name, sig, bad);
    }

    fn capability_revoke(&mut self) {
        let Some(c) = self.live_cap(None).filter(|_| !self.rd.one_in(10)) else {
            // SAFETY: Null is checked before any read.
            let s = unsafe { ulua_capability_revoke(ptr::null_mut()) };
            self.expect(
                "ulua_capability_revoke",
                s,
                &[ULUA_INVALID_ARGUMENT],
                "null",
            );
            return;
        };
        // SAFETY: A live pool capability.
        let s = unsafe { ulua_capability_revoke(self.caps[c].ptr) };
        if self.expect("ulua_capability_revoke", s, &[ULUA_OK], "a live capability") {
            self.caps[c].bound.revoked.set(true);
        }
    }

    fn capability_free(&mut self) {
        let Some(c) = self.live_cap(None).filter(|_| !self.rd.one_in(10)) else {
            // SAFETY: Null is valid for a free function.
            let s = unsafe { ulua_capability_free(ptr::null_mut()) };
            self.expect("ulua_capability_free", s, &[ULUA_OK], "null");
            return;
        };
        // SAFETY: A live pool capability, freed once.
        let s = unsafe { ulua_capability_free(self.caps[c].ptr) };
        self.expect("ulua_capability_free", s, &[ULUA_OK], "a live capability");
        self.caps[c].live = false;
        self.caps[c].bound.revoked.set(true);
    }

    /// Create an instance of a live module. `memory` 0 selects the default limits.
    pub fn instance_with(
        &mut self,
        ctx: usize,
        m: usize,
        host_slot: Slot,
        memory: u64,
        clean: bool,
    ) -> Option<usize> {
        let program = self.mods[m].program;
        let mut h = crate::host::host(&self.state);
        match host_slot {
            Slot::Size => h.struct_size += 1,
            Slot::Reserved => h.reserved = 1,
            _ => {}
        }
        let mut l = RunLimits::default();
        if memory > 0 {
            l.max_memory_bytes = memory;
        }
        l.max_steps = if self.rd.one_in(2) {
            5_000
        } else {
            DEFAULT_STEPS
        };
        l.max_open_requests = *[256, 1, 2, 0].get(self.rd.below(8)).unwrap_or(&256);
        let limit_slot = match self.rd.below(20) {
            0 if !clean => Slot::Size,
            1 if !clean => Slot::Reserved,
            _ => Slot::Valid,
        };
        match limit_slot {
            Slot::Size => l.struct_size += 1,
            Slot::Reserved => l.reserved2 = 1,
            _ => {}
        }
        let diag = if clean { Slot::Valid } else { self.slot() };
        let mut d = check::diagnostic();
        let dp = Self::diagnostic(diag, &mut d);
        let output = self.output_mode(ctx, None);
        let other = self.mods[m].ctx != ctx;
        let defect = if other {
            Some("a module of another context")
        } else if host_slot != Slot::Valid {
            Some("an invalid host")
        } else if limit_slot != Slot::Valid {
            Some("invalid limits")
        } else if matches!(diag, Slot::Size | Slot::Reserved) {
            Some("an invalid diagnostic")
        } else {
            None
        };
        let mut out = ptr::null_mut();
        let hp = if host_slot == Slot::Null {
            ptr::null()
        } else {
            ptr::from_ref(&h)
        };
        // SAFETY: The module and context are live; descriptors and outputs are local.
        let s = unsafe {
            ulua_instance_new(self.ctxs[ctx].ptr, self.mods[m].ptr, hp, &l, &mut out, dp)
        };
        let events = self.take_events();
        let init = PROGRAMS[program].init;
        let expect = match (defect, init) {
            (Some(_), _) => model::Expect::Stop(ULUA_INVALID_ARGUMENT, ""),
            _ if memory > 0 && memory < 300_000 => model::Expect::Stop(ULUA_FAILED, "limit:memory"),
            (None, Init::Trap(code)) => model::Expect::Stop(ULUA_FAILED, code),
            (None, Init::Print) if output == crate::host::Mode::Error => {
                model::Expect::Stop(ULUA_HOST_ERROR, "host:output")
            }
            _ => model::Expect::Ok,
        };
        let why =
            format!("program {program}, defect {defect:?}, memory {memory}, output {output:?}");
        self.nested_results("ulua_instance_new", &events);
        if !self.outcome("ulua_instance_new", s, &d, dp.is_null(), &expect, &why) {
            return None;
        }
        if s != ULUA_OK {
            if !out.is_null() {
                self.report
                    .fail("instance:out-on-failure", "*out is not null");
            }
            return None;
        }
        let held = self.mods[m].held.clone();
        self.insts.push(MInst {
            ptr: out,
            ctx,
            program,
            held,
            counter: 0.0,
            total: 0.0,
            open: vec![],
            recorded: VecDeque::new(),
            waiters: HashMap::new(),
            tasks: HashMap::new(),
            seen_tasks: HashSet::new(),
            seen_requests: HashSet::new(),
            max_open: l.max_open_requests,
            live: true,
        });
        Some(self.insts.len() - 1)
    }

    /// Compare a status and its diagnostic with an expectation.
    pub fn outcome(
        &mut self,
        call: &str,
        s: Status,
        d: &Diagnostic,
        null: bool,
        e: &model::Expect,
        why: &str,
    ) -> bool {
        let (want, code) = match e {
            model::Expect::Ok => (ULUA_OK, ""),
            model::Expect::Stop(st, code) => (*st, *code),
        };
        if !self.expect(call, s, &[want], why) {
            return false;
        }
        if !null && matches!(s, ULUA_FAILED | ULUA_HOST_ERROR) {
            self.written(call, d, code, true);
        }
        self.report.failures.is_empty()
    }

    fn instance_new(&mut self) {
        let Some(ctx) = self.live_ctx() else {
            return;
        };
        let m = if self.rd.one_in(12) {
            self.live_mod(None)
        } else {
            self.live_mod(Some(ctx))
        };
        let Some(m) = m else {
            return;
        };
        if self.rd.one_in(40) {
            let mut out = ptr::null_mut();
            // SAFETY: A null context or out pointer is refused before any read of them.
            let s = unsafe {
                if self.rd.one_in(2) {
                    ulua_instance_new(
                        ptr::null_mut(),
                        self.mods[m].ptr,
                        &Host::default(),
                        ptr::null(),
                        &mut out,
                        ptr::null_mut(),
                    )
                } else {
                    ulua_instance_new(
                        self.ctxs[ctx].ptr,
                        self.mods[m].ptr,
                        &Host::default(),
                        ptr::null(),
                        ptr::null_mut(),
                        ptr::null_mut(),
                    )
                }
            };
            self.expect(
                "ulua_instance_new",
                s,
                &[ULUA_INVALID_ARGUMENT],
                "a null pointer",
            );
            return;
        }
        let host = match self.rd.below(30) {
            0 => Slot::Null,
            1 => Slot::Size,
            2 => Slot::Reserved,
            _ => Slot::Valid,
        };
        let memory = match self.rd.below(16) {
            0 => 100_000,
            1..=3 => 8 << 20,
            _ => 0,
        };
        self.instance_with(ctx, m, host, memory, false);
    }

    fn instance_free(&mut self) {
        let Some(i) = self.live_inst().filter(|_| !self.rd.one_in(10)) else {
            // SAFETY: Null is valid for a free function.
            let s = unsafe { ulua_instance_free(ptr::null_mut()) };
            self.expect("ulua_instance_free", s, &[ULUA_OK], "null");
            return;
        };
        // SAFETY: A live pool instance, freed once.
        let s = unsafe { ulua_instance_free(self.insts[i].ptr) };
        self.expect("ulua_instance_free", s, &[ULUA_OK], "a live instance");
        self.insts[i].live = false;
    }
}

/// Parameter types that are all invalid, longer than any count that the engine passes.
static ZEROS: [u32; ULUA_MAX_HOST_PARAMETERS + 1] = [0; ULUA_MAX_HOST_PARAMETERS + 1];

unsafe extern "C" fn noop(
    _: *mut std::ffi::c_void,
    _: *const Value,
    _: usize,
    _: *mut Value,
) -> i32 {
    0
}
unsafe extern "C" fn noop_start(
    _: *mut std::ffi::c_void,
    _: *const Value,
    _: usize,
    _: u64,
) -> i32 {
    0
}
