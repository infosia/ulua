//! The `compiler` target: source bytes into `ulua_compile` through the C ABI.
use crate::{
    check::{self, Report},
    host::{Bound, HostState, descriptor},
    rng::Rng,
};
use std::{cell::RefCell, ptr, time::Instant};
use ulua::*;

/// A context that frees itself, with the bindings that its capabilities point to.
/// The context drops first, then the bindings.
pub struct Owned {
    /// The context.
    pub context: *mut Context,
    /// The capabilities of the grants, in grant order.
    pub grants: Vec<*mut Capability>,
    /// The bindings of the grants. Each is boxed: a descriptor userdata points to it,
    /// so it keeps its address when the vector grows.
    #[allow(clippy::vec_box)]
    pub bounds: Vec<Box<Bound>>,
}

impl Drop for Owned {
    fn drop(&mut self) {
        // SAFETY: The owner holds the only reference to a live context or null.
        unsafe {
            ulua_context_free(self.context);
        }
    }
}

/// The result of one compile.
pub struct Compiled {
    /// The context and grants.
    pub owned: Owned,
    /// The module, or null when the compile did not give `ULUA_OK`.
    pub module: *mut Module,
    /// The grant signatures.
    pub signatures: Vec<ulua_compiler::HostSignature>,
    /// The limits of the compile.
    pub limits: ulua_compiler::Limits,
    /// The time of `ulua_compile`.
    pub nanos: u128,
}

/// The fixed facts of a run: the rule ids of collisions.md.
pub struct Env {
    /// The rule ids.
    pub rules: Vec<String>,
}

fn limits(aux: &mut Rng, len: usize) -> Option<CompileLimits> {
    if aux.below(8) != 0 {
        return None;
    }
    let mut l = CompileLimits::default();
    if aux.one_in(2) {
        let random = aux.below(70_000) as u64;
        l.max_source_bytes = *aux.pick(&[
            0,
            1,
            len.saturating_sub(1) as u64,
            len as u64,
            len as u64 + 1,
            u64::MAX,
            random,
        ]);
    }
    if aux.one_in(2) {
        l.max_nesting_depth = *aux.pick(&[0, 1, 2, 5, 16, 47, 48, 49, 1000, u32::MAX]);
    }
    Some(l)
}

/// Compile `source` with its grants and check the status and the diagnostic.
pub fn compile(
    env: &Env,
    source: &[u8],
    aux: &mut Rng,
    state: &RefCell<HostState>,
    report: &mut Report,
) -> Option<Compiled> {
    let signatures = crate::seeds::grants(source);
    let mut owned = Owned {
        context: ptr::null_mut(),
        grants: vec![],
        bounds: vec![],
    };
    // SAFETY: Each pointer is local storage or an object of this owned context.
    unsafe {
        let s = ulua_context_new(&mut owned.context);
        if s != ULUA_OK {
            report.fail(
                format!("status:ulua_context_new:{}", check::status_name(s)),
                "context creation failed",
            );
            return None;
        }
        for sig in &signatures {
            let b = Box::new(Bound {
                name: sig.name.clone(),
                parameters: sig.parameters.clone(),
                result: sig.result,
                asynchronous: sig.asynchronous,
                revoked: std::cell::Cell::new(false),
                state,
            });
            let d = descriptor(&b);
            let mut cap = ptr::null_mut();
            let s = ulua_capability_new(owned.context, &d, &mut cap);
            if s != ULUA_OK {
                report.fail(
                    format!("status:ulua_capability_new:{}", check::status_name(s)),
                    format!("fixture {} refused", sig.name),
                );
                return None;
            }
            owned.grants.push(cap);
            owned.bounds.push(b);
        }
        let chosen = limits(aux, source.len());
        let mut effective = ulua_compiler::Limits::default();
        if let Some(l) = chosen {
            effective.max_source_bytes = l.max_source_bytes.min(ulua_compiler::SOURCE_CEILING);
            effective.max_nesting_depth = l.max_nesting_depth.min(ulua_compiler::DEPTH_CEILING);
        }
        let mut module = ptr::null_mut();
        let mut d = check::diagnostic();
        let started = Instant::now();
        let s = ulua_compile(
            owned.context,
            source.as_ptr().cast(),
            source.len(),
            owned.grants.as_ptr(),
            owned.grants.len(),
            chosen.as_ref().map_or(ptr::null(), ptr::from_ref),
            &mut module,
            &mut d,
        );
        let nanos = started.elapsed().as_nanos();
        check::note(format!("ulua_compile:{}", check::status_name(s)));
        if s == ULUA_REJECTED
            && let Ok(x) = check::decode(&d)
        {
            check::note(format!("ulua_compile:{}", x.code));
        }
        let len = source.len() as u64;
        let over = len > effective.max_source_bytes;
        match s {
            ULUA_OK => {
                if over {
                    report.fail(
                        "compile:source-bytes-accepted",
                        format!("{len} bytes over a limit of {}", effective.max_source_bytes),
                    );
                }
                if std::str::from_utf8(source).is_err() {
                    report.fail(
                        "compile:invalid-utf8-accepted",
                        "a non-UTF-8 source compiled",
                    );
                }
                if module.is_null() {
                    report.fail("compile:null-module", "ULUA_OK wrote a null module");
                    return None;
                }
                exports(module, report);
            }
            ULUA_REJECTED | ULUA_FAILED | ULUA_INTERNAL => {
                let decoded = match check::decode(&d) {
                    Ok(x) if x.code != "unset" => x,
                    Ok(_) => {
                        report.fail(
                            format!("compile:no-diagnostic:{}", check::status_name(s)),
                            "the diagnostic is not written",
                        );
                        return None;
                    }
                    Err(e) => {
                        report.fail("compile:diagnostic-format", e);
                        return None;
                    }
                };
                rejected(env, source, s, &decoded, over, &effective, report);
            }
            other => report.fail(
                format!("status:ulua_compile:{}", check::status_name(other)),
                format!("ulua_compile gave status {other}"),
            ),
        }
        Some(Compiled {
            owned,
            module: if s == ULUA_OK {
                module
            } else {
                ptr::null_mut()
            },
            signatures,
            limits: effective,
            nanos,
        })
    }
}

fn rejected(
    env: &Env,
    source: &[u8],
    s: Status,
    d: &check::Decoded,
    over: bool,
    limits: &ulua_compiler::Limits,
    report: &mut Report,
) {
    if s == ULUA_INTERNAL || d.code == "internal" {
        report.fail(
            "internal:ulua_compile",
            format!("{} {}: {}", check::status_name(s), d.code, d.message),
        );
        return;
    }
    if s == ULUA_FAILED {
        if d.code != "host:resources" {
            report.fail(
                format!("code:ulua_compile:FAILED:{}", d.code),
                d.message.clone(),
            );
        }
        return;
    }
    if !check::compile_code(&d.code, &env.rules) {
        report.fail(
            format!("code:ulua_compile:{}", d.code),
            format!("{}: {}", d.code, d.message),
        );
    }
    if over != (d.code == "limit:source-bytes") {
        report.fail(
            format!("compile:source-bytes:{}", d.code),
            format!(
                "{} bytes, limit {}, code {}",
                source.len(),
                limits.max_source_bytes,
                d.code
            ),
        );
    }
    if !over && let Err(e) = std::str::from_utf8(source) {
        let at = e.valid_up_to() as u64;
        if d.code != "syntax" || d.offset != at {
            report.fail(
                "compile:invalid-utf8-position",
                format!("first invalid byte {at}, diagnostic {d:?}"),
            );
        }
    }
    if d.offset > source.len() as u64 {
        report.fail(
            "compile:offset-past-end",
            format!("{d:?} for {} bytes", source.len()),
        );
    } else if d.code != "limit:source-bytes" {
        let (line, column) = check::line_column(source, d.offset as usize);
        if (line, column) != (d.line, d.column) {
            report.fail(
                format!("compile:line-column:{}", d.code),
                format!(
                    "offset {} gives {line}:{column}, diagnostic {d:?}",
                    d.offset
                ),
            );
        }
    }
}

/// Check the export queries of a compiled module (abi.md §4.3, §7.3).
unsafe fn exports(module: *mut Module, report: &mut Report) {
    // SAFETY: The module is live; every output is local storage.
    unsafe {
        let mut count = 0;
        if ulua_module_export_count(module, &mut count) != ULUA_OK {
            report.fail("module:export-count", "the export count query failed");
            return;
        }
        for i in 0..count {
            let (mut name, mut len) = (ptr::null(), 0);
            let s = ulua_module_export_name(module, i, &mut name, &mut len);
            let ok = s == ULUA_OK
                && !name.is_null()
                && std::str::from_utf8(std::slice::from_raw_parts(name.cast::<u8>(), len)).is_ok();
            if !ok {
                report.fail("module:export-name", format!("export {i}: status {s}"));
            }
            let mut types = [9u32; 32];
            let (mut n, mut result) = (0usize, 9u32);
            let s = ulua_module_export_signature(
                module,
                i,
                types.as_mut_ptr(),
                types.len(),
                &mut n,
                &mut result,
            );
            if s != ULUA_OK
                || n > types.len()
                || result > ULUA_TYPE_STRING
                || !types[..n.min(32)].iter().all(|t| (1..=3).contains(t))
            {
                report.fail(
                    "module:export-signature",
                    format!("export {i}: status {s}, {n} parameters, result {result}"),
                );
            }
        }
    }
}

/// The generated Luau of an accepted source, from the Rust API (as `ulua-corpus
/// --emit-luau` does). A disagreement with the ABI result is a failure.
pub fn luau(c: &Compiled, source: &[u8], report: &mut Report) -> Option<String> {
    let limits = c.limits;
    let signatures = &c.signatures;
    // Windows MSVC Debug witness replay overflowed the 1 MiB main stack.
    // /STACK:1638400 failed; /STACK:1654784 passed for smoke input 1277.
    // Match the C ABI compiler stack without enlarging the ABI run stack.
    let result = std::thread::scope(|scope| {
        std::thread::Builder::new()
            .name("ulua-fuzz-compiler".into())
            .stack_size(ulua_compiler::COMPILER_STACK_BYTES)
            .spawn_scoped(scope, || {
                ulua_compiler::compile_with_grants(source, limits, signatures)
            })
            .map(|thread| thread.join())
    });
    let result = match result {
        Ok(result) => result,
        Err(e) => {
            report.fail("harness:compiler-thread", e.to_string());
            return None;
        }
    };
    match result {
        Ok(Ok(compiled)) => Some(compiled.luau),
        Ok(Err(d)) => {
            report.fail(
                "compile:api-disagrees",
                format!("the ABI accepts, the Rust API gives {}", d.code),
            );
            None
        }
        Err(_) => {
            report.fail(
                "panic:compile_with_grants",
                "the Rust compiler API panicked",
            );
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn env() -> Env {
        Env {
            rules: check::rule_ids(include_str!("../../../specs/blocks/collisions.md")),
        }
    }
    #[test]
    fn a_wrong_status_or_code_is_a_failure() {
        let env = env();
        let limits = ulua_compiler::Limits::default();
        let d = |code: &str| check::Decoded {
            code: code.into(),
            message: "m".into(),
            offset: 0,
            line: 1,
            column: 1,
        };
        let mut r = Report::default();
        rejected(
            &env,
            b"x",
            ULUA_INTERNAL,
            &d("internal"),
            false,
            &limits,
            &mut r,
        );
        assert_eq!(r.failures[0].key, "internal:ulua_compile");
        let mut r = Report::default();
        rejected(
            &env,
            b"x",
            ULUA_REJECTED,
            &d("trap:x"),
            false,
            &limits,
            &mut r,
        );
        assert_eq!(r.failures[0].key, "code:ulua_compile:trap:x");
        let mut r = Report::default();
        rejected(
            &env,
            b"x",
            ULUA_REJECTED,
            &d("limit:source-bytes"),
            false,
            &limits,
            &mut r,
        );
        assert!(r.failures[0].key.starts_with("compile:source-bytes"));
        let mut r = Report::default();
        let mut at = d("syntax");
        at.line = 2;
        rejected(&env, b"x", ULUA_REJECTED, &at, false, &limits, &mut r);
        assert_eq!(r.failures[0].key, "compile:line-column:syntax");
        let mut r = Report::default();
        rejected(&env, b"x", ULUA_REJECTED, &d("C1"), false, &limits, &mut r);
        assert!(r.clean());
    }
    #[test]
    fn real_compiles_are_clean() {
        let env = env();
        let state = RefCell::new(HostState::new(Rng::new(1)));
        for src in [
            &b"export function main(): void { print('a'); }"[..],
            b"export function main(): void { print(\xff); }",
            b"// grants: hostAdd\nexport function f(): number { return hostAdd(1, 2); }",
            b"let x: any = 1;",
        ] {
            let mut r = Report::default();
            let mut aux = Rng::new(3);
            let c = compile(&env, src, &mut aux, &state, &mut r);
            assert!(c.is_some() && r.clean(), "{:?}", r.failures);
        }
    }
}
