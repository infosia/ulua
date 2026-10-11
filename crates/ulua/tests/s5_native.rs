//! Default-build native checks exercise the production allocator and VM recovery path.
//! Measured Debug cost: 0.12 seconds (round 8 review).
// SAFETY: These fixture entries own all native state and return only a status.
#[link(name = "ulua_test_native", kind = "static")]
unsafe extern "C" {
    fn ulua_internal_probe_shrink() -> i32;
    fn ulua_internal_probe_stack_shrink() -> i32;
}

#[test]
fn shrink_and_equal_size_succeed_at_the_step_boundary() {
    assert_eq!(ulua::ulua_abi_version(), 1);
    // SAFETY: The fixture takes no pointers and frees its allocation.
    assert_eq!(unsafe { ulua_internal_probe_shrink() }, 0);
}

#[test]
fn overflow_recovery_shrinks_outside_protection_at_the_step_boundary() {
    assert_eq!(ulua::ulua_abi_version(), 1);
    // SAFETY: The fixture protects the overflow and owns the VM until recovery completes.
    assert_eq!(unsafe { ulua_internal_probe_stack_shrink() }, 0);
}

// This fixture uses the compile operation only.
#[allow(dead_code)]
#[path = "support/script.rs"]
mod support;

#[test]
fn spill_tables_share_the_local_budget_with_parameters_and_loop_storage() {
    let parameters = (0..32)
        .map(|i| format!("p{i}:number=0"))
        .collect::<Vec<_>>()
        .join(",");
    let mut source = format!("function f({parameters}):void{{");
    for i in 0..5000 {
        source.push_str(&format!("let a{i}=0;"));
    }
    source.push_str(&"for(;false;){}".repeat(102));
    source.push_str("}export function main():void{}");
    let generated = std::thread::Builder::new()
        .stack_size(ulua_compiler::COMPILER_STACK_BYTES)
        .spawn({
            let source = source.clone();
            move || {
                ulua_compiler::compile(source.as_bytes(), ulua_compiler::Limits::default())
                    .unwrap()
                    .luau
            }
        })
        .unwrap()
        .join()
        .unwrap();
    // Derive the count from emitted declarations and the source parameter count.
    let body = generated
        .split_once(" = (function(")
        .unwrap()
        .1
        .split("\nend) ::")
        .next()
        .unwrap();
    let locals = body
        .lines()
        .filter(|line| line.starts_with("    local "))
        .count()
        + 32;
    assert_eq!(locals, ulua_compiler::LOCAL_BUDGET);
    assert_eq!(generated.matches("local __ulua_spill").count(), 8);
    let _script = support::Script::new(&source);
}
