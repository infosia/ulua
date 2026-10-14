//! Test-only native allocation control. The feature is absent from production builds.
//! A controller belongs to the current thread. The host must serialize each instance on that thread.
use crate::Instance;
use std::ffi::c_void;

/// Allocation and lifetime observations from the native allocator.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
#[non_exhaustive]
pub struct Snapshot {
    /// Eligible allocations in the run. Free, shrink, and equal-size requests are excluded.
    pub allocations: u64,
    /// Allocations refused by the hook in the run.
    pub failures: u64,
    /// Live bytes charged to the instance budget.
    pub live_bytes: usize,
    /// Live completion string bytes outside the instance budget.
    pub host_bytes: usize,
    /// Native request and task records that remain linked.
    pub records: usize,
}
// SAFETY: These declarations match the feature-only native functions and their C layouts.
unsafe extern "C" {
    fn ulua_internal_fault_arm(index: u64);
    fn ulua_internal_fault_last(out: *mut Snapshot);
    fn ulua_internal_fault_live(vm: *const c_void, out: *mut Snapshot);
    fn ulua_internal_fault_seed(vm: *mut c_void) -> bool;
    fn ulua_internal_fault_release_seed();
}
/// Fail index `k` once in the next creation, call, completion, or settle on this thread.
/// Indices start at one. Zero counts allocations without a failure.
/// Each run resets the index and consumes this setting. A pump starts a run for each settle.
pub fn arm(k: u64) {
    // SAFETY: The native controller uses thread-local scalar state only.
    unsafe { ulua_internal_fault_arm(k) }
}
/// Return the last run snapshot, or the final snapshot after native instance destruction.
/// Destruction preserves the run count and records the remaining live bytes before it frees the VM descriptor.
pub fn last() -> Snapshot {
    let mut result = Snapshot::default();
    // SAFETY: The native function writes one initialized snapshot into valid storage.
    unsafe { ulua_internal_fault_last(&mut result) }
    result
}
/// Observe a live instance without a VM operation or allocation.
/// # Safety
/// The instance must be live and serialized on the current thread.
pub unsafe fn live(instance: *const Instance) -> Snapshot {
    let mut result = Snapshot::default();
    if !instance.is_null() {
        // SAFETY: The caller supplies a serialized live instance and valid snapshot storage.
        unsafe { ulua_internal_fault_live((*instance).vm, &mut result) }
    }
    result
}
/// Seed a 31-byte allocation that instance destruction does not release.
/// Call `release_seed` after destruction and the leak assertion to release the test's storage.
/// # Safety
/// The instance must be live and serialized on the current thread.
pub unsafe fn seed_leak(instance: *mut Instance) -> bool {
    if instance.is_null() {
        return false;
    }
    // SAFETY: The caller supplies a serialized live instance.
    unsafe { ulua_internal_fault_seed((*instance).vm) }
}
/// Release the seeded test allocation after instance destruction and the leak assertion.
/// # Safety
/// The instance that owned the seeded allocation must already be destroyed.
pub unsafe fn release_seed() {
    // SAFETY: The caller guarantees that the VM can no longer use or account for this allocation.
    unsafe { ulua_internal_fault_release_seed() }
}
