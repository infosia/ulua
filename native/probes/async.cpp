// Independent native observations of the retained FIFO and allocator requests.
#include "../shim/allocator.h"
#include "../shim/async_records.h"
#include "lua.h"
#include "lualib.h"
struct AsyncStats { ulua_vm* vm; size_t bytes; size_t jobs; };
extern "C" size_t ulua_internal_test_async_bytes(const ulua_vm* vm) {
    return vm->used;
}
static int async_stats(lua_State* state) {
    AsyncStats* stats = static_cast<AsyncStats*>(lua_tolightuserdata(state, 1));
    lua_gc(state, LUA_GCCOLLECT, 0);
    stats->bytes = stats->vm->used;
    lua_getref(state, stats->vm->jobs);
    lua_pushnil(state);
    while (lua_next(state, -2)) { ++stats->jobs; lua_pop(state, 1); }
    return 0;
}
extern "C" int ulua_internal_test_async_stats(ulua_vm* vm, size_t* bytes, size_t* jobs) {
    AsyncStats stats = {vm, 0, 0};
    const int status = lua_cpcall(vm->thread, async_stats, &stats);
    lua_settop(vm->thread, 0);
    *bytes = stats.bytes;
    *jobs = stats.jobs;
    return status;
}
#ifdef ULUA_MEASUREMENTS
extern "C" uint64_t ulua_internal_test_allocation_calls(const ulua_vm* vm) {
    return vm->allocation_calls;
}
extern "C" void ulua_internal_test_resume_stats(const ulua_vm* vm, uint64_t* checks, uint64_t* allocations) {
    *checks = vm->resume_checks;
    *allocations = vm->resume_allocations;
}
#endif
// Reserve a small measured headroom after setup, without a machine-specific budget guess.
extern "C" void ulua_internal_test_async_headroom(ulua_vm* vm, size_t bytes) {
    vm->budget = vm->used + bytes;
}

// Stop the real request factory at either allocation boundary. The wrapper stays test-only.
static int request_memory_stop(lua_State* state) {
    ulua_vm* vm = static_cast<ulua_vm*>(lua_callbacks(state)->userdata);
    const int stage = lua_tointeger(state, lua_upvalueindex(2));
    if (stage == 1) {
        // Reserve stack space before the budget stop. Retain each result until the real factory fails.
        luaL_checkstack(state, 512, "request factory probe");
        vm->budget = vm->used;
        for (int i = 0; i < 256; ++i) {
            lua_pushinteger(state, 1);
            lua_replace(state, lua_upvalueindex(3));
            lua_pushvalue(state, lua_upvalueindex(1));
            lua_pushnil(state);
            lua_call(state, 1, 1);
            lua_pushinteger(state, 2);
            lua_replace(state, lua_upvalueindex(3));
        }
        luaL_error(state, "Request factory probe did not stop");
    }
    lua_pushvalue(state, lua_upvalueindex(1));
    lua_pushnil(state);
    lua_call(state, 1, 1);
    if (stage == 2) {
        // Reserve a reusable reference slot. The next refused allocation is the native record.
        const int reference = lua_ref(state, -1);
        lua_unref(state, reference);
        vm->budget = vm->used;
        lua_pushinteger(state, 2);
        lua_replace(state, lua_upvalueindex(3));
    }
    return 1;
}
struct RequestStop { ulua_vm* vm; int stage; };
static int request_memory_setup(lua_State* state) {
    RequestStop* args = static_cast<RequestStop*>(lua_tolightuserdata(state, 1));
    lua_getref(state, args->vm->new_promise);
    lua_pushinteger(state, args->stage);
    lua_pushinteger(state, 0);
    lua_pushcclosure(state, request_memory_stop, "request_memory_stop", 3);
    const int reference = lua_ref(state, -1);
    lua_unref(state, args->vm->new_promise);
    args->vm->new_promise = reference;
    return 0;
}
extern "C" int ulua_internal_test_request_memory_stop(ulua_vm* vm, int stage) {
    RequestStop args = {vm, stage};
    const int status = lua_cpcall(vm->thread, request_memory_setup, &args);
    lua_settop(vm->thread, 0);
    return status;
}
static int request_memory_stage(lua_State* state) {
    RequestStop* args = static_cast<RequestStop*>(lua_tolightuserdata(state, 1));
    lua_getref(state, args->vm->new_promise);
    lua_getupvalue(state, -1, 3);
    args->stage = lua_tointeger(state, -1);
    return 0;
}
extern "C" int ulua_internal_test_request_memory_stage(ulua_vm* vm) {
    RequestStop args = {vm, 0};
    const int status = lua_cpcall(vm->thread, request_memory_stage, &args);
    lua_settop(vm->thread, 0);
    return status == LUA_OK ? args.stage : -1;
}
struct PromiseStats { ulua_vm* vm; size_t promises; };
static int promise_stats(lua_State* state) {
    PromiseStats* stats = static_cast<PromiseStats*>(lua_tolightuserdata(state, 1));
    lua_gc(state, LUA_GCCOLLECT, 0);
    lua_getref(state, stats->vm->promises);
    lua_pushnil(state);
    while (lua_next(state, -2)) { ++stats->promises; lua_pop(state, 1); }
    return 0;
}
extern "C" int ulua_internal_test_request_stats(ulua_vm* vm, size_t* bytes, size_t* promises,
    size_t* records, uint64_t* next) {
    PromiseStats stats = {vm, 0};
    const int status = lua_cpcall(vm->thread, promise_stats, &stats);
    lua_settop(vm->thread, 0);
    *bytes = vm->used;
    *promises = stats.promises;
    *records = 0;
    for (const ulua_request* r = vm->requests; r; r = r->next) ++*records;
    *next = vm->next_request;
    return status;
}
