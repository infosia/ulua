#ifndef ULUA_ALLOCATOR_H
#define ULUA_ALLOCATOR_H
#include "ulua_shim.h"
#include <cstdlib>
#ifdef ULUA_FAULT_INJECTION
#include "faults.h"
#endif
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
    int tables;
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
#ifdef ULUA_FAULT_INJECTION
    ulua_fault_state fault;
#endif
    uint64_t random_state[4];
    int math;
    ulua_math_fn math_dispatch;
    ulua_number_fn number_dispatch;
    ulua_host host;
};
static void record_stop(ulua_vm* vm, int code) {
    if (!vm->flag) vm->flag = code;
}
static void* memory_stop(ulua_vm* vm) {
    // GC resize paths can catch LUA_ERRMEM. Retain the refusal until the run boundary.
    record_stop(vm, SHIM_MEMORY);
    return nullptr;
}
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
#ifdef ULUA_FAULT_INJECTION
    if (fault_refuse(&vm->fault)) return memory_stop(vm);
#endif
    if (vm->charging) {
        const uint64_t charge = new_size / ALLOCATION_STEP_BYTES + (new_size % ALLOCATION_STEP_BYTES != 0);
        if (vm->steps > vm->max_steps || charge > vm->max_steps - vm->steps) {
            record_stop(vm, SHIM_STEPS);
            return nullptr;
        }
        vm->steps += charge;
#ifdef ULUA_MEASUREMENTS
        vm->allocation_bytes += new_size;
        vm->allocation_steps += charge;
#endif
    }
    size_t base = vm->used - old;
    if (new_size > vm->budget || base > vm->budget - new_size) return memory_stop(vm);
    void* result = std::realloc(pointer, new_size);
    if (!result) return memory_stop(vm);
    vm->used = base + new_size;
    return result;
}
static void* allocate_record(ulua_vm* vm, void* pointer, size_t old_size, size_t new_size) {
    void* result = allocate(vm, pointer, old_size, new_size);
#ifdef ULUA_FAULT_INJECTION
    // Count owned allocations independently of the request and task lists.
    if (!pointer && result) ++vm->fault.snapshot.records;
    if (pointer && !new_size) --vm->fault.snapshot.records;
#endif
    return result;
}
#endif
