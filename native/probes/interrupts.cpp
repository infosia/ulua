// Count executed VM instructions with production bytecode, independent of emitter counters.
#include "../shim/ulua_shim.h"
#include "lua.h"
#include "lualib.h"
#include <algorithm>
#include <cstdint>

struct Gap {
    uint64_t current = 0;
    uint64_t largest = 0;
    uint64_t interrupts = 0;
};
static void step(lua_State* state, lua_Debug*) {
    auto* gap = static_cast<Gap*>(lua_callbacks(state)->userdata);
    ++gap->current;
}
static void interrupt(lua_State* state, int gc) {
    if (gc >= 0) return;
    auto* gap = static_cast<Gap*>(lua_callbacks(state)->userdata);
    gap->largest = std::max(gap->largest, gap->current);
    gap->current = 0;
    ++gap->interrupts;
}
static int unused(lua_State*) { return 0; }
extern "C" int ulua_internal_probe_interrupts(const char* source, size_t length, int branch,
    uint64_t* largest, uint64_t* interrupts, char* error) {
    char* bytecode = nullptr;
    size_t size = 0;
    int status = ulua_internal_shim_compile(source, length, &bytecode, &size, error);
    if (status) return status;
    lua_State* state = luaL_newstate();
    if (!state) { ulua_internal_shim_bytecode_free(bytecode); return SHIM_MEMORY; }
    luaL_openlibs(state);
    luaL_sandbox(state);
    luaL_sandboxthread(state);
    status = luau_load(state, "interrupt-probe", bytecode, size, 0);
    ulua_internal_shim_bytecode_free(bytecode);
    Gap gap;
    lua_callbacks(state)->userdata = &gap;
    lua_callbacks(state)->debugstep = step;
    lua_callbacks(state)->interrupt = interrupt;
    lua_singlestep(state, 1);
    if (!status) {
        lua_pushcfunction(state, unused, "trap");
        lua_pushnil(state);
        lua_pushcfunction(state, unused, "register");
        status = lua_pcall(state, 3, 1, 0);
    }
    if (!status) {
        lua_getfield(state, -1, "main");
        lua_pushboolean(state, branch);
        status = lua_pcall(state, 1, 0, 0);
    }
    if (status) {
        const char* message = lua_tostring(state, -1);
        if (message) {
            size_t i = 0;
            for (; message[i] && i < 255; ++i) error[i] = message[i];
            error[i] = 0;
        }
    }
    *largest = std::max(gap.largest, gap.current);
    *interrupts = gap.interrupts;
    lua_close(state);
    return status;
}

// Repeat generated module work with the production compiler, allocator, and default run limits.
extern "C" int ulua_internal_probe_step_loop(const char* source, size_t length,
    const ulua_run_limits* limits, char* error) {
    ulua_host host = {};
    host.struct_size = sizeof(host);
    ulua_vm* vm = nullptr;
    int status = ulua_internal_shim_new(source, length, &host, nullptr, 0, limits, &vm, error);
    ulua_internal_shim_free(vm);
    return status;
}
