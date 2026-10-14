//! The capability, compile, module, and default-limit operations of the engine.
use crate::{
    check,
    engine::{Engine, MMod, Slot},
    model::{self, Compiles, PROGRAMS},
};
use std::{collections::HashSet, ptr};
use ulua::*;

impl Engine<'_> {
    pub(crate) fn signature(&self, c: usize) -> ulua_compiler::HostSignature {
        let cap = &self.caps[c];
        let mut s =
            ulua_compiler::HostSignature::new(&cap.name, cap.sig.params.clone(), cap.sig.result);
        s.asynchronous = cap.sig.asynchronous;
        s
    }

    /// A grant list over live capabilities, with a chosen defect. Returns the pointers,
    /// the pool indices of the non-null entries, and the defect.
    pub(crate) fn grant_list(
        &mut self,
        ctx: Option<usize>,
    ) -> (Vec<*mut Capability>, Vec<usize>, Option<&'static str>) {
        let mut idx = vec![];
        for _ in 0..self.rd.below(4) {
            if let Some(c) = self.live_cap(ctx) {
                idx.push(c);
            }
        }
        let mut ptrs: Vec<_> = idx.iter().map(|i| self.caps[*i].ptr).collect();
        match self.rd.below(14) {
            0 => {
                ptrs.insert(self.rd.below(ptrs.len() + 1), ptr::null_mut());
            }
            1 => {
                if let Some(other) = self.live_cap(None) {
                    idx.push(other);
                    ptrs.push(self.caps[other].ptr);
                }
            }
            2 if !idx.is_empty() => {
                let d = idx[self.rd.below(idx.len())];
                idx.push(d);
                ptrs.push(self.caps[d].ptr);
            }
            _ => {}
        }
        let null = ptrs.iter().any(|p| p.is_null());
        let mut names = HashSet::new();
        let dup = idx
            .iter()
            .any(|i| !names.insert(self.caps[*i].name.clone()));
        let defect = if null {
            Some("a null grant")
        } else if dup {
            Some("two grants with one name")
        } else {
            None
        };
        (ptrs, idx, defect)
    }

    pub(crate) fn declarations(&mut self) {
        let (ptrs, idx, defect) = self.grant_list(None);
        let sigs: Vec<_> = idx.iter().map(|i| self.signature(*i)).collect();
        let text = crate::witness::declarations(&sigs);
        let mut buffer = vec![0u8; text.len() + 8];
        let mut length = usize::MAX;
        let variant = self.rd.below(6);
        let (buf, cap, len_ptr): (*mut u8, usize, *mut usize) = match variant {
            0 => (ptr::null_mut(), 0, &mut length),
            1 => (
                buffer.as_mut_ptr(),
                text.len().saturating_sub(1),
                &mut length,
            ),
            2 => (buffer.as_mut_ptr(), buffer.len(), ptr::null_mut()),
            3 => (ptr::null_mut(), 4, &mut length),
            4 => (buffer.as_mut_ptr(), buffer.len(), &mut length),
            _ => (buffer.as_mut_ptr(), text.len(), &mut length),
        };
        // SAFETY: The grants are live pool capabilities or null; the buffer covers `cap`.
        let s = unsafe { ulua_declarations(ptrs.as_ptr(), ptrs.len(), buf.cast(), cap, len_ptr) };
        let (want, why) = match (defect, variant) {
            (Some(d), _) => (ULUA_INVALID_ARGUMENT, d),
            (None, 0 | 1) => (ULUA_INVALID_ARGUMENT, "a capacity below the length"),
            (None, 2) => (ULUA_INVALID_ARGUMENT, "a null length"),
            (None, 3) => (ULUA_INVALID_ARGUMENT, "a null buffer with a capacity"),
            _ => (ULUA_OK, "valid"),
        };
        if !self.expect("ulua_declarations", s, &[want], why) {
            return;
        }
        if defect.is_none() && matches!(variant, 0 | 1 | 4 | 5) && length != text.len() {
            self.report.fail(
                "declarations:length",
                format!("length {length}, model {}", text.len()),
            );
        }
        if s == ULUA_OK && &buffer[..text.len()] != text.as_bytes() {
            self.report.fail(
                "declarations:text",
                format!(
                    "{:?} != {text:?}",
                    String::from_utf8_lossy(&buffer[..text.len()])
                ),
            );
        }
        if s != ULUA_OK && buffer.iter().any(|b| *b != 0) {
            self.report
                .fail("declarations:wrote-on-failure", "a failed call wrote bytes");
        }
    }

    /// Compile a program in a context with a grant list of pool indices.
    pub(crate) fn compile_with(
        &mut self,
        ctx: usize,
        program: usize,
        grants: Vec<usize>,
        defect: Option<&'static str>,
    ) {
        let ptrs: Vec<_> = grants.iter().map(|i| self.caps[*i].ptr).collect();
        self.compile_ptrs(
            ctx,
            program,
            ptrs,
            grants,
            defect,
            Slot::Valid,
            Slot::Valid,
            false,
        );
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn compile_ptrs(
        &mut self,
        ctx: usize,
        program: usize,
        ptrs: Vec<*mut Capability>,
        grants: Vec<usize>,
        mut defect: Option<&'static str>,
        diag: Slot,
        limits: Slot,
        null_out: bool,
    ) {
        let p = &PROGRAMS[program];
        let other = grants.iter().any(|g| self.caps[*g].ctx != ctx);
        let revoked = grants.iter().any(|g| self.caps[*g].bound.revoked.get());
        if defect.is_none() && other {
            defect = Some("a grant of another context");
        }
        if defect.is_none() && revoked {
            defect = Some("a revoked grant");
        }
        if matches!(diag, Slot::Size | Slot::Reserved) {
            defect = Some("an invalid diagnostic");
        }
        if matches!(limits, Slot::Size | Slot::Reserved) {
            defect = Some("invalid limits");
        }
        if null_out {
            defect = Some("a null out");
        }
        let mut held = vec![];
        let mut satisfied = true;
        for n in p.needs {
            let want = model::need(n);
            match grants.iter().find(|g| self.caps[**g].name == *n) {
                Some(g) if Some(&self.caps[*g].sig) == want.as_ref() => held.push(*g),
                _ => satisfied = false,
            }
        }
        let mut d = check::diagnostic();
        let dp = Self::diagnostic(diag, &mut d);
        let mut l = CompileLimits::default();
        match limits {
            Slot::Size => l.struct_size += 1,
            Slot::Reserved => l.reserved2 = 1,
            _ => {}
        }
        let lp = if limits == Slot::Null {
            ptr::null()
        } else {
            ptr::from_ref(&l)
        };
        let mut out = ptr::null_mut();
        let outp = if null_out {
            ptr::null_mut()
        } else {
            ptr::from_mut(&mut out)
        };
        // SAFETY: The source is static; grants are live pool pointers or null; outputs are local.
        let s = unsafe {
            ulua_compile(
                self.ctxs[ctx].ptr,
                p.source.as_ptr().cast(),
                p.source.len(),
                ptrs.as_ptr(),
                ptrs.len(),
                lp,
                outp,
                dp,
            )
        };
        let (want, code) = match (defect, p.compiles) {
            (Some(_), _) => (ULUA_INVALID_ARGUMENT, None),
            (None, Compiles::No(code)) => (ULUA_REJECTED, Some(code)),
            (None, Compiles::Yes) if !satisfied => (ULUA_REJECTED, Some("*")),
            _ => (ULUA_OK, None),
        };
        let why = format!("program {program}, defect {defect:?}, grants satisfied {satisfied}");
        if !self.expect("ulua_compile", s, &[want], &why) {
            return;
        }
        if let (Some(code), false) = (code, dp.is_null()) {
            self.written("ulua_compile", &d, code, false);
        }
        if s == ULUA_OK {
            self.compiled = true;
            self.mods.push(MMod {
                ptr: out,
                ctx,
                program,
                held,
                live: true,
            });
        } else if !out.is_null() {
            self.report
                .fail("compile:out-on-failure", "a failed compile wrote a module");
        }
    }

    pub(crate) fn compile(&mut self) {
        let Some(ctx) = self.live_ctx() else {
            return;
        };
        if self.rd.one_in(32) {
            let mut out = ptr::null_mut();
            let which = self.rd.below(2);
            // SAFETY: A null context or a null source with a non-zero length is refused first.
            let s = unsafe {
                if which == 0 {
                    ulua_compile(
                        ptr::null_mut(),
                        c"x".as_ptr(),
                        1,
                        ptr::null(),
                        0,
                        ptr::null(),
                        &mut out,
                        ptr::null_mut(),
                    )
                } else {
                    ulua_compile(
                        self.ctxs[ctx].ptr,
                        ptr::null(),
                        3,
                        ptr::null(),
                        0,
                        ptr::null(),
                        &mut out,
                        ptr::null_mut(),
                    )
                }
            };
            self.expect(
                "ulua_compile",
                s,
                &[ULUA_INVALID_ARGUMENT],
                "a null pointer",
            );
            return;
        }
        let program = if self.rd.one_in(3) {
            self.rd.below(PROGRAMS.len())
        } else {
            self.rd.below(3)
        };
        // Grants mostly come from this context, and include the needs of the program.
        let (mut ptrs, mut idx, defect) = self.grant_list(Some(ctx));
        if self.rd.below(3) != 0 {
            for n in PROGRAMS[program].needs {
                if !idx.iter().any(|i| self.caps[*i].name == *n) {
                    let found = (0..self.caps.len()).find(|i| {
                        let c = &self.caps[*i];
                        c.live && c.ctx == ctx && c.name == *n && !c.bound.revoked.get()
                    });
                    if let Some(f) = found {
                        idx.push(f);
                        ptrs.push(self.caps[f].ptr);
                    }
                }
            }
        }
        let diag = self.slot();
        let limits = match self.rd.below(16) {
            0 => Slot::Size,
            1 => Slot::Reserved,
            2..=7 => Slot::Valid,
            _ => Slot::Null,
        };
        let null_out = self.rd.one_in(40);
        let mut names = HashSet::new();
        let dup = idx
            .iter()
            .any(|i| !names.insert(self.caps[*i].name.clone()));
        let defect = defect.or(dup.then_some("two grants with one name"));
        self.compile_ptrs(ctx, program, ptrs, idx, defect, diag, limits, null_out);
    }

    pub(crate) fn module_free(&mut self) {
        let Some(m) = self.live_mod(None).filter(|_| !self.rd.one_in(10)) else {
            // SAFETY: Null is valid for a free function.
            let s = unsafe { ulua_module_free(ptr::null_mut()) };
            self.expect("ulua_module_free", s, &[ULUA_OK], "null");
            return;
        };
        // SAFETY: A live pool module, freed once. Its instances stay valid (abi.md §7.2).
        let s = unsafe { ulua_module_free(self.mods[m].ptr) };
        self.expect("ulua_module_free", s, &[ULUA_OK], "a live module");
        self.mods[m].live = false;
    }

    pub(crate) fn queries(&mut self) {
        let Some(m) = self.live_mod(None) else {
            return;
        };
        let module = self.mods[m].ptr;
        let exports = PROGRAMS[self.mods[m].program].exports;
        let n = exports.len() as u32;
        let index = self.rd.below(exports.len() + 2) as u32;
        // SAFETY: The module is live; every output is local storage or null.
        unsafe {
            match self.rd.below(4) {
                0 => {
                    let mut count = u32::MAX;
                    let null = self.rd.one_in(6);
                    let s = ulua_module_export_count(
                        module,
                        if null { ptr::null_mut() } else { &mut count },
                    );
                    let want = if null { ULUA_INVALID_ARGUMENT } else { ULUA_OK };
                    if self.expect("ulua_module_export_count", s, &[want], "count")
                        && s == ULUA_OK
                        && count != n
                    {
                        self.report
                            .fail("module:count", format!("count {count}, model {n}"));
                    }
                }
                1 => {
                    let (mut name, mut len) = (ptr::null(), usize::MAX);
                    let s = ulua_module_export_name(module, index, &mut name, &mut len);
                    let want = if index < n {
                        ULUA_OK
                    } else {
                        ULUA_INVALID_ARGUMENT
                    };
                    if self.expect("ulua_module_export_name", s, &[want], "index") && s == ULUA_OK {
                        let got = std::slice::from_raw_parts(name.cast::<u8>(), len);
                        if got != exports[index as usize].name.as_bytes() {
                            self.report
                                .fail("module:name", format!("{:?}", String::from_utf8_lossy(got)));
                        }
                    }
                }
                2 => {
                    let mut types = [99u32; 4];
                    let (mut count, mut result) = (usize::MAX, u32::MAX);
                    let capacity = self.rd.below(4);
                    let s = ulua_module_export_signature(
                        module,
                        index,
                        types.as_mut_ptr(),
                        capacity,
                        &mut count,
                        &mut result,
                    );
                    let Some(e) = exports.get(index as usize) else {
                        self.expect(
                            "ulua_module_export_signature",
                            s,
                            &[ULUA_INVALID_ARGUMENT],
                            "index",
                        );
                        return;
                    };
                    let small = capacity < e.params.len();
                    let want = if small {
                        ULUA_INVALID_ARGUMENT
                    } else {
                        ULUA_OK
                    };
                    if self.expect("ulua_module_export_signature", s, &[want], "capacity") {
                        if count != e.params.len() || result != e.result {
                            self.report
                                .fail("module:signature", format!("{count} {result}"));
                        }
                        if s == ULUA_OK && types[..count] != *e.params {
                            self.report
                                .fail("module:signature-types", format!("{types:?}"));
                        }
                        if small && types.iter().any(|t| *t != 99) {
                            self.report
                                .fail("module:signature-wrote-types", format!("{types:?}"));
                        }
                    }
                }
                _ => {
                    let mut out = 7u32;
                    let choice = self.rd.below(4);
                    let (name, len): (*const u8, usize) = match choice {
                        0 if index < n => (
                            exports[index as usize].name.as_ptr(),
                            exports[index as usize].name.len(),
                        ),
                        1 => (b"nope".as_ptr(), 4),
                        2 => (b"a\xff".as_ptr(), 2),
                        _ => (ptr::null(), 5),
                    };
                    let s = ulua_module_find_export(module, name.cast(), len, &mut out);
                    let found = choice == 0 && index < n;
                    let want = if found {
                        ULUA_OK
                    } else {
                        ULUA_INVALID_ARGUMENT
                    };
                    if self.expect("ulua_module_find_export", s, &[want], "name") {
                        let ok = match choice {
                            _ if found => out == index,
                            1 => out == u32::MAX,
                            2 => out == u32::MAX || out == 7,
                            _ if name.is_null() => out == 7,
                            _ => out == u32::MAX,
                        };
                        if !ok {
                            self.report.fail(
                                "module:find-export-output",
                                format!("choice {choice}: index {out}"),
                            );
                        }
                    }
                }
            }
        }
    }

    pub(crate) fn defaults(&mut self) {
        let variant = self.rd.below(3);
        let mut c = CompileLimits::default();
        let mut r = RunLimits::default();
        c.max_source_bytes = 1;
        r.max_steps = 1;
        if variant == 1 {
            c.struct_size += 1;
            r.struct_size += 1;
        }
        // SAFETY: Each output is local storage or null.
        let (a, b) = unsafe {
            if variant == 2 {
                (
                    ulua_default_compile_limits(ptr::null_mut()),
                    ulua_default_run_limits(ptr::null_mut()),
                )
            } else {
                (
                    ulua_default_compile_limits(&mut c),
                    ulua_default_run_limits(&mut r),
                )
            }
        };
        let want = if variant == 0 {
            ULUA_OK
        } else {
            ULUA_INVALID_ARGUMENT
        };
        self.expect("ulua_default_compile_limits", a, &[want], "descriptor");
        self.expect("ulua_default_run_limits", b, &[want], "descriptor");
        if variant == 0 {
            let compile = (
                c.max_source_bytes,
                c.max_nesting_depth,
                c.reserved,
                c.reserved2,
            );
            let run = (
                r.max_memory_bytes,
                r.max_steps,
                r.max_open_requests,
                r.reserved,
                r.reserved2,
            );
            if compile != (65_536, 48, 0, 0) || run != (64 << 20, 1_000_000, 256, 0, 0) {
                self.report
                    .fail("defaults:values", format!("{compile:?} {run:?}"));
            }
        }
    }
}
