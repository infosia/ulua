#ifndef ULUA_FAULT_API_H
#define ULUA_FAULT_API_H
extern "C" void ulua_internal_fault_arm(uint64_t index) noexcept { fault_next = index; }
extern "C" void ulua_internal_fault_last(ulua_fault_snapshot* out) noexcept { *out = fault_last; }
extern "C" void ulua_internal_fault_live(const ulua_vm* vm, ulua_fault_snapshot* out) noexcept {
    *out = vm->fault.snapshot;
    out->live_bytes = vm->used;
    out->records = 0;
    for (ulua_request* r = vm->requests; r; r = r->next) ++out->records;
    for (ulua_task* t = vm->tasks; t; t = t->next) ++out->records;
}
extern "C" bool ulua_internal_fault_seed(ulua_vm* vm) noexcept {
    if (fault_seed) return false;
    fault_seed = allocate(vm, nullptr, 0, 31);
    return fault_seed != nullptr;
}
extern "C" void ulua_internal_fault_release_seed() noexcept {
    std::free(fault_seed);
    fault_seed = nullptr;
}
#endif
