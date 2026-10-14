//! The nested ABI calls that a host callback runs on its own context (abi.md §1.7).
use crate::{engine::Engine, host::Nested};
use std::ptr;
use ulua::*;

const SOURCE: &str = "export function main(): void {}";

impl Engine<'_> {
    /// A nested ABI call on a context whose callback runs (abi.md §1.7).
    pub(crate) fn nested(&mut self, ctx: usize, inst: Option<usize>) -> Nested {
        let c = self.ctxs[ctx].ptr;
        let m = self.live_mod(Some(ctx)).map(|m| self.mods[m].ptr);
        let cap = self.live_cap(Some(ctx)).map(|x| self.caps[x].ptr);
        let ip = inst.map(|i| self.insts[i].ptr);
        let op = self.rd.below(18);
        let busy = vec![ULUA_BUSY];
        Box::new(move || {
            let mut d = Diagnostic::default();
            let mut out = ptr::null_mut();
            let mut v = Value::default();
            v.r#type = ULUA_TYPE_NUMBER;
            // SAFETY: Each pointer is a live object of the busy context, or local storage.
            unsafe {
                let s = |name: &str, s: Status| (name.to_owned(), s, busy.clone());
                match (op, m, cap, ip) {
                    (1, Some(m), _, _) => s("ulua_module_free", ulua_module_free(m)),
                    (2, _, _, Some(i)) => s("ulua_instance_free", ulua_instance_free(i)),
                    (3, ..) => s("ulua_context_free", ulua_context_free(c)),
                    (4, _, Some(k), _) => s("ulua_capability_free", ulua_capability_free(k)),
                    (5, _, Some(k), _) => s("ulua_capability_revoke", ulua_capability_revoke(k)),
                    (6, _, _, Some(i)) => {
                        s("ulua_call", ulua_call(i, 0, ptr::null(), 0, &mut v, &mut d))
                    }
                    (7, Some(m), _, _) => {
                        let mut n = 0;
                        s(
                            "ulua_module_export_count",
                            ulua_module_export_count(m, &mut n),
                        )
                    }
                    (8, _, _, Some(i)) => {
                        s("ulua_request_complete", ulua_request_complete(i, 1, &v))
                    }
                    (9, _, _, Some(i)) => {
                        let mut n = 0;
                        s("ulua_pump", ulua_pump(i, 0, &mut n, &mut d))
                    }
                    (10, _, _, Some(i)) => {
                        let (mut a, mut b) = (0, 0);
                        s(
                            "ulua_instance_requests",
                            ulua_instance_requests(i, &mut a, &mut b),
                        )
                    }
                    (11, _, Some(k), _) => {
                        let mut len = 0;
                        s(
                            "ulua_declarations",
                            ulua_declarations(&k, 1, ptr::null_mut(), 0, &mut len),
                        )
                    }
                    (12, ..) => {
                        let name = b"nestedCap";
                        let params = [ULUA_TYPE_NUMBER];
                        let mut desc = CapabilityDesc::default();
                        desc.name = name.as_ptr().cast();
                        desc.name_length = name.len();
                        desc.parameter_types = params.as_ptr();
                        desc.parameter_count = 1;
                        desc.result_type = ULUA_TYPE_NUMBER;
                        desc.function = Some(never);
                        let mut k = ptr::null_mut();
                        s("ulua_capability_new", ulua_capability_new(c, &desc, &mut k))
                    }
                    (13, ..) => {
                        // Another context is not busy: both calls succeed.
                        let mut other = ptr::null_mut();
                        let a = ulua_context_new(&mut other);
                        let b = ulua_context_free(other);
                        let st = if a == ULUA_OK && b == ULUA_OK {
                            ULUA_OK
                        } else {
                            a.max(b)
                        };
                        ("ulua_context_new+free".to_owned(), st, vec![ULUA_OK])
                    }
                    (14, _, _, Some(i)) => {
                        s("ulua_task_result", ulua_task_result(i, 1, &mut v, &mut d))
                    }
                    (15, _, _, Some(i)) => s("ulua_task_discard", ulua_task_discard(i, 1)),
                    (16, _, _, Some(i)) => {
                        let mut t = 0;
                        s(
                            "ulua_call_start",
                            ulua_call_start(i, 0, ptr::null(), 0, &mut t, &mut d),
                        )
                    }
                    (17, Some(m), _, _) => {
                        let mut i = ptr::null_mut();
                        s(
                            "ulua_instance_new",
                            ulua_instance_new(c, m, &Host::default(), ptr::null(), &mut i, &mut d),
                        )
                    }
                    _ => s(
                        "ulua_compile",
                        ulua_compile(
                            c,
                            SOURCE.as_ptr().cast(),
                            SOURCE.len(),
                            ptr::null(),
                            0,
                            ptr::null(),
                            &mut out,
                            &mut d,
                        ),
                    ),
                }
            }
        })
    }
}

unsafe extern "C" fn never(
    _: *mut std::ffi::c_void,
    _: *const Value,
    _: usize,
    _: *mut Value,
) -> i32 {
    1
}
