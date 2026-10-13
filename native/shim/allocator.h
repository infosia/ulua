#ifndef ULUA_ALLOCATOR_H
#define ULUA_ALLOCATOR_H
#include "ulua_shim.h"
#include <cstdlib>
struct lua_State;
struct ulua_request;
struct ulua_task;
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
    int errors;
    int drain;
    int new_promise;
    int fulfill;
    uint64_t next_request;
    uint64_t next_task;
    uint32_t max_open_requests;
    uint32_t open_requests;
    uint32_t recorded_requests;
    ulua_request* requests;
    ulua_request* recorded_head;
    ulua_request* recorded_tail;
    ulua_task* tasks;
    int reset;
    int promises;
    int jobs;
    int rejections;
    int collection;
    bool async_failed;
    bool request_starting;
    int result_ref;
    char capability_name[65];
    char exception[256];
    bool charging;
#ifdef ULUA_MEASUREMENTS
    uint64_t allocation_bytes;
    uint64_t allocation_steps;
    uint64_t interrupts;
    uint64_t allocation_calls;
    uint64_t resume_checks;
    uint64_t resume_allocations;
    uint64_t resume_snapshot;
    bool resume_unwind;
#endif
    ulua_host host;
};
static void* allocate(void* userdata, void* pointer, size_t old_size, size_t new_size) {
    ulua_vm* vm = static_cast<ulua_vm*>(userdata);
    size_t old = pointer ? old_size : 0;
#ifdef ULUA_MEASUREMENTS
    if (new_size) ++vm->allocation_calls;
#endif
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
            if (!vm->flag) vm->flag = SHIM_STEPS;
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
