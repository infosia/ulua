// Observe the production allocator and GC without a runtime policy change.
#include "../shim/allocator.h"
#include "lua.h"
#include "lstate.h"
#include "lgc.h"
#include <algorithm>
#include <cstdint>
static const char* const export_names[] = {"main"};

struct Observation {
    ulua_vm* vm;
    lua_Alloc allocator;
    void (*interrupt)(lua_State*, int);
    uint64_t* values;
    double last_collection;
};
// Slots 0..2: initial allocator bytes, initial GC bytes, peak allocator bytes.
// Slots 3..7: post-call allocator bytes, GC bytes, threshold, steps, completed cycles.
// Slots 8..11: first refused size, allocator bytes, GC bytes, threshold.
// Slots 12..16: post-collection allocator bytes, GC bytes, post-close bytes, refused GC state, GC callbacks.
static thread_local Observation* observation = nullptr;

static void* observed_allocate(void* userdata, void* pointer, size_t old_size, size_t new_size) {
    auto& sample = *observation;
    void* result = sample.allocator(userdata, pointer, old_size, new_size);
    sample.values[2] = std::max(sample.values[2], static_cast<uint64_t>(sample.vm->used));
    if (!result && new_size && !sample.values[8]) {
        sample.values[8] = new_size;
        sample.values[9] = sample.vm->used;
        sample.values[10] = sample.vm->state->global->totalbytes;
        sample.values[11] = sample.vm->state->global->GCthreshold;
        sample.values[15] = sample.vm->state->global->gcstate;
    }
    return result;
}

static void observed_interrupt(lua_State* state, int gc) {
    auto& sample = *observation;
    const double completed = state->global->gcstats.endtimestamp;
    if (completed != sample.last_collection) ++sample.values[7];
    sample.last_collection = completed;
    if (gc >= 0) ++sample.values[16];
    sample.interrupt(state, gc);
}

static int collect(lua_State* state) {
    lua_gc(state, LUA_GCCOLLECT, 0);
    return 0;
}

extern "C" int ulua_internal_probe_capture_memory(const char* bytecode, size_t length,
    const ulua_run_limits* limits, int goal, uint64_t* values, char* error) {
    ulua_host host = {};
    host.struct_size = sizeof(host);
    ulua_vm* vm = nullptr;
    int status = ulua_internal_shim_new(bytecode, length, &host, nullptr, 0, export_names, 1, limits, &vm, error);
    if (status) return status;
    auto* global = vm->state->global;
    values[0] = vm->used;
    values[1] = global->totalbytes;
    values[2] = vm->used;
    Observation sample = {vm, global->frealloc, lua_callbacks(vm->state)->interrupt, values, global->gcstats.endtimestamp};
    observation = &sample;
    global->frealloc = observed_allocate;
    lua_callbacks(vm->state)->interrupt = observed_interrupt;
    if (goal > 0) lua_gc(vm->state, LUA_GCSETGOAL, goal);
    if (goal < 0) lua_gc(vm->state, LUA_GCSETSTEPMUL, -goal);
    ulua_value result = {};
    status = ulua_internal_shim_call(vm, 0, nullptr, 0, ULUA_TYPE_NUMBER, &result, error);
    values[3] = vm->used;
    values[4] = global->totalbytes;
    values[5] = global->GCthreshold;
    values[6] = vm->steps;
    // The call unwinds its frame. A protected full collection measures the retained heap.
    vm->charging = false;
    lua_callbacks(vm->state)->interrupt = nullptr;
    const int collected = lua_cpcall(vm->state, collect, nullptr);
    values[12] = vm->used;
    values[13] = global->totalbytes;
    global->frealloc = sample.allocator;
    release_export_refs(vm);
    lua_close(vm->state);
    vm->state = nullptr;
    values[14] = vm->used;
    observation = nullptr;
    ulua_internal_shim_free(vm);
    return collected ? SHIM_INTERNAL : status;
}
