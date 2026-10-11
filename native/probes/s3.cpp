// Opt-in measurements. No probe enters the product shim or public C header.
#include "lua.h"
#include "lualib.h"
#include <cstdint>
#include <cstring>
#if defined(__APPLE__)
#include <pthread.h>
#include <malloc/malloc.h>
static thread_local unsigned char* bottom;
static thread_local unsigned char* painted_end;
extern "C" void ulua_probe_stack_start() noexcept {
    unsigned char marker;
    auto top = static_cast<unsigned char*>(pthread_get_stackaddr_np(pthread_self()));
    bottom = reinterpret_cast<unsigned char*>(reinterpret_cast<uintptr_t>(top) - pthread_get_stacksize_np(pthread_self()) + 65536);
    painted_end = reinterpret_cast<unsigned char*>(reinterpret_cast<uintptr_t>(&marker) - 4096);
    std::memset(bottom, 0xa5, size_t(painted_end - bottom));
}
extern "C" size_t ulua_probe_stack_end() noexcept {
    auto top = static_cast<unsigned char*>(pthread_get_stackaddr_np(pthread_self()));
    auto p = bottom;
    while (p < painted_end && *p == 0xa5) ++p;
    return size_t(top - p);
}
extern "C" size_t ulua_probe_heap() noexcept {
    malloc_statistics_t stats = {};
    malloc_zone_statistics(nullptr, &stats);
    return stats.size_in_use;
}
#else
extern "C" void ulua_probe_stack_start() noexcept {}
extern "C" size_t ulua_probe_stack_end() noexcept { return 0; }
extern "C" size_t ulua_probe_heap() noexcept { return 0; }
#endif
static int recurse(lua_State* state) {
    int remaining = int(lua_tointeger(state, 1));
    if (remaining) {
        lua_pushcfunction(state, recurse, "probe");
        lua_pushinteger(state, remaining - 1);
        lua_call(state, 1, 0);
    }
    return 0;
}
static int recurse_protected(lua_State* state) {
    int remaining = int(lua_tointeger(state, 1));
    if (remaining) {
        lua_pushcfunction(state, recurse_protected, "probe");
        lua_pushinteger(state, remaining - 1);
        if (lua_pcall(state, 1, 0, 0) != LUA_OK) lua_error(state);
    }
    return 0;
}
static int ccalls(int count, lua_CFunction function) {
    lua_State* state = luaL_newstate();
    if (!state) return -1;
    lua_pushcfunction(state, function, "probe");
    lua_pushinteger(state, count);
    int status = lua_pcall(state, 1, 0, 0);
    lua_close(state);
    return status;
}
extern "C" int ulua_probe_ccalls(int count) noexcept { return ccalls(count, recurse); }
extern "C" int ulua_probe_ccalls_protected(int count) noexcept { return ccalls(count, recurse_protected); }
