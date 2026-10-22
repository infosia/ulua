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
extern "C" void ulua_internal_probe_stack_start() noexcept {
    unsigned char marker;
    auto top = static_cast<unsigned char*>(pthread_get_stackaddr_np(pthread_self()));
    bottom = reinterpret_cast<unsigned char*>(reinterpret_cast<uintptr_t>(top) - pthread_get_stacksize_np(pthread_self()) + 65536);
    painted_end = reinterpret_cast<unsigned char*>(reinterpret_cast<uintptr_t>(&marker) - 4096);
    std::memset(bottom, 0xa5, size_t(painted_end - bottom));
}
extern "C" size_t ulua_internal_probe_stack_end() noexcept {
    auto top = static_cast<unsigned char*>(pthread_get_stackaddr_np(pthread_self()));
    auto p = bottom;
    while (p < painted_end && *p == 0xa5) ++p;
    return size_t(top - p);
}
extern "C" size_t ulua_internal_probe_heap() noexcept {
    malloc_statistics_t stats = {};
    malloc_zone_statistics(nullptr, &stats);
    return stats.size_in_use;
}
#elif defined(__linux__)
#include <pthread.h>
#include <sys/syscall.h>
#include <unistd.h>
static thread_local unsigned char* bottom;
static thread_local unsigned char* painted_end;
static thread_local unsigned char* stack_top;
extern "C" void ulua_internal_probe_stack_start() noexcept {
    bottom = nullptr;
    painted_end = nullptr;
    stack_top = nullptr;
    // glibc derives main-thread bounds from rlimit; they do not exclude the kernel guard gap, so the probe stays unstarted and the end returns 0.
    if (syscall(SYS_gettid) == getpid()) return;
    pthread_attr_t attr;
    if (pthread_getattr_np(pthread_self(), &attr) != 0) return;
    void* base = nullptr;
    size_t size = 0;
    const int status = pthread_attr_getstack(&attr, &base, &size);
    pthread_attr_destroy(&attr);
    if (status != 0 || size <= 65536) return;
    unsigned char marker;
    const uintptr_t low = reinterpret_cast<uintptr_t>(base) + 65536;
    const uintptr_t high = reinterpret_cast<uintptr_t>(base) + size;
    const uintptr_t end = reinterpret_cast<uintptr_t>(&marker) - 4096;
    if (end <= low || end >= high) return;
    bottom = reinterpret_cast<unsigned char*>(low);
    painted_end = reinterpret_cast<unsigned char*>(end);
    stack_top = reinterpret_cast<unsigned char*>(high);
    std::memset(bottom, 0xa5, size_t(end - low));
}
extern "C" size_t ulua_internal_probe_stack_end() noexcept {
    if (!bottom) return 0;
    auto p = bottom;
    while (p < painted_end && *p == 0xa5) ++p;
    return size_t(stack_top - p);
}
extern "C" size_t ulua_internal_probe_heap() noexcept { return 0; }
#elif defined(_WIN32)
#define WIN32_LEAN_AND_MEAN
#include <windows.h>
#include <malloc.h>
static thread_local unsigned char* bottom;
static thread_local unsigned char* painted_end;
static thread_local unsigned char* stack_top;
extern "C" void ulua_internal_probe_stack_start() noexcept {
    unsigned char marker;
    MEMORY_BASIC_INFORMATION region = {};
    VirtualQuery(&marker, &region, sizeof(region));
    bottom = static_cast<unsigned char*>(region.AllocationBase) + 65536;
    ULONG_PTR low, high;
    GetCurrentThreadStackLimits(&low, &high);
    stack_top = reinterpret_cast<unsigned char*>(high);
    painted_end = reinterpret_cast<unsigned char*>(reinterpret_cast<uintptr_t>(&marker) - 4096);
    const size_t length = size_t(painted_end - bottom);
    // __chkstk grows the committed stack through its guard pages normally.
    // Leave 64 KiB at the reservation bottom and 4 KiB below the live probe frame.
    bottom = static_cast<unsigned char*>(_alloca(length));
    painted_end = bottom + length;
    std::memset(bottom, 0xa5, length);
}
extern "C" size_t ulua_internal_probe_stack_end() noexcept {
    auto p = bottom;
    while (p < painted_end && *p == 0xa5) ++p;
    return size_t(stack_top - p);
}
extern "C" size_t ulua_internal_probe_heap() noexcept { return 0; }
#else
extern "C" void ulua_internal_probe_stack_start() noexcept {}
extern "C" size_t ulua_internal_probe_stack_end() noexcept { return 0; }
extern "C" size_t ulua_internal_probe_heap() noexcept { return 0; }
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
extern "C" int ulua_internal_probe_ccalls(int count) noexcept { return ccalls(count, recurse); }
extern "C" int ulua_internal_probe_ccalls_protected(int count) noexcept { return ccalls(count, recurse_protected); }
