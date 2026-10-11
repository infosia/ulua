// Native allocator fixtures link only into tests and measurement executables.
#include "../shim/allocator.h"
#include <cstring>

#ifdef ULUA_MEASUREMENTS
// These probes exist only in the opt-in measurement build.
extern "C" void ulua_internal_probe_run_stats(ulua_vm* vm, uint64_t* bytes, uint64_t* charges, uint64_t* interrupts) noexcept {
    *bytes = vm->allocation_bytes;
    *charges = vm->allocation_steps;
    *interrupts = vm->interrupts;
}
extern "C" int ulua_internal_probe_allocation(size_t bytes, uint64_t steps) noexcept {
    ulua_vm vm = {};
    vm.budget = SIZE_MAX;
    vm.max_steps = steps;
    vm.charging = true;
    void* value = allocate(&vm, nullptr, 0, bytes);
    int result = value ? SHIM_OK : vm.flag;
    allocate(&vm, value, bytes, 0);
    return result;
}
#endif

extern "C" int ulua_internal_probe_shrink() noexcept {
    ulua_vm vm = {};
    vm.budget = SIZE_MAX;
    vm.max_steps = 1;
    vm.charging = true;
    void* value = allocate(&vm, nullptr, 0, ALLOCATION_STEP_BYTES);
    if (!value) return 1;
    std::memset(value, 0x5a, ALLOCATION_STEP_BYTES);
    // Even a exhausted memory budget cannot refuse an equal allocation or shrink.
    vm.budget = 0;
    void* equal = allocate(&vm, value, ALLOCATION_STEP_BYTES, ALLOCATION_STEP_BYTES);
    if (!equal) { std::free(value); return 2; }
    void* smaller = allocate(&vm, equal, ALLOCATION_STEP_BYTES, ALLOCATION_STEP_BYTES / 2);
    if (!smaller) { std::free(equal); return 3; }
    int result = vm.steps == 1 && vm.flag == 0 && vm.used == ALLOCATION_STEP_BYTES / 2
        && static_cast<unsigned char*>(smaller)[0] == 0x5a ? 0 : 4;
    allocate(&vm, smaller, ALLOCATION_STEP_BYTES / 2, 0);
    return vm.used == 0 ? result : 5;
}
