#ifndef ULUA_ALLOCATOR_H
#define ULUA_ALLOCATOR_H
#include "ulua_shim.h"
#include <cstdlib>
struct lua_State;
struct ulua_vm {
    lua_State* state;
    lua_State* thread;
    size_t budget;
    size_t used;
    uint64_t max_steps;
    uint64_t steps;
    int flag;
    int trap_kind;
    int exports;
    bool charging;
#ifdef ULUA_MEASUREMENTS
    uint64_t allocation_bytes;
    uint64_t allocation_steps;
    uint64_t interrupts;
#endif
    ulua_host host;
};
static void* allocate(void* userdata, void* pointer, size_t old_size, size_t new_size) {
    ulua_vm* vm = static_cast<ulua_vm*>(userdata);
    size_t old = pointer ? old_size : 0;
    if (!new_size) {
        std::free(pointer);
        vm->used -= old;
        return nullptr;
    }
    if (pointer && new_size <= old_size) {
        // Luau can shrink outside a protection boundary after a stack overflow.
        // If the system refuses a shrink, keep the original storage.
        void* result = new_size == old_size ? pointer : std::realloc(pointer, new_size);
        vm->used -= old_size - new_size;
        return result ? result : pointer;
    }
    if (vm->charging) {
        const uint64_t charge = new_size / ALLOCATION_STEP_BYTES + (new_size % ALLOCATION_STEP_BYTES != 0);
        if (vm->steps > vm->max_steps || charge > vm->max_steps - vm->steps) {
            vm->flag = SHIM_STEPS;
            return nullptr;
        }
        vm->steps += charge;
#ifdef ULUA_MEASUREMENTS
        vm->allocation_bytes += new_size;
        vm->allocation_steps += charge;
#endif
    }
    size_t base = vm->used - old;
    if (new_size > vm->budget || base > vm->budget - new_size) return nullptr;
    void* result = std::realloc(pointer, new_size);
    if (result) vm->used = base + new_size;
    return result;
}
#endif
