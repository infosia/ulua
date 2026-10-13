// Independent native observations of the retained FIFO and allocator requests.
#include "../shim/allocator.h"
#include "lua.h"
struct AsyncStats { ulua_vm* vm; size_t bytes; size_t jobs; };
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
extern "C" void ulua_internal_test_resume_stats(const ulua_vm* vm, uint64_t* checks, uint64_t* allocations) {
    *checks = vm->resume_checks;
    *allocations = vm->resume_allocations;
}
#endif
// Reserve a small measured headroom after setup, without a machine-specific budget guess.
extern "C" void ulua_internal_test_async_headroom(ulua_vm* vm, size_t bytes) {
    vm->budget = vm->used + bytes;
}
