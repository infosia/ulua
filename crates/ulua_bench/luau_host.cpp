// This host is private to the benchmark crate. It shares the product's pinned VM.
#include "lua.h"
#include "lualib.h"
#include "luacode.h"
#include <cstdlib>
#include <cstring>

static void message(char* out, const char* text) noexcept {
    if (!text) text = "Unknown Luau error";
    const size_t length = std::strlen(text);
    const size_t count = length < 255 ? length : 255;
    std::memcpy(out, text, count);
    out[count] = 0;
}

extern "C" lua_State* ulua_bench_luau_new(const char* source, size_t length, char* error) noexcept {
    lua_State* state = nullptr;
    char* bytecode = nullptr;
    try {
        size_t size = 0;
        // nullptr selects the production defaults: O1, debug level 1, type info level 0.
        bytecode = luau_compile(source, length, nullptr, &size);
        if (!bytecode) { message(error, "No bytecode"); return nullptr; }
        state = luaL_newstate();
        if (!state) { std::free(bytecode); message(error, "No VM"); return nullptr; }
        luaL_openlibs(state);
        luaL_sandbox(state);
        luaL_sandboxthread(state);
        int status = luau_load(state, "benchmark", bytecode, size, 0);
        std::free(bytecode);
        bytecode = nullptr;
        if (!status) status = lua_pcall(state, 0, 1, 0);
        if (status || !lua_isfunction(state, -1)) {
            message(error, status ? lua_tostring(state, -1) : "Workload must return a function");
            lua_close(state);
            return nullptr;
        }
        // Stack slot 1 holds the workload. No interrupt callback is installed.
        return state;
    } catch (...) {
        std::free(bytecode);
        if (state) lua_close(state);
        message(error, "Luau host exception");
        return nullptr;
    }
}

extern "C" int ulua_bench_luau_result(lua_State* state, int status, double* result, char* error) noexcept {
    if (status || lua_type(state, -1) != LUA_TNUMBER) {
        message(error, status ? lua_tostring(state, -1) : "Checksum must be a number");
        return 1;
    }
    *result = lua_tonumber(state, -1);
    lua_pop(state, 1);
    return 0;
}

// The C ABI wrappers isolate the pinned VM's C++ linkage from Rust.
extern "C" void ulua_bench_luau_prepare(lua_State* state) noexcept {
    lua_pushvalue(state, 1);
}
extern "C" int ulua_bench_luau_call(lua_State* state) noexcept {
    return lua_pcall(state, 0, 1, 0);
}
extern "C" void ulua_bench_luau_free(lua_State* state) noexcept {
    lua_close(state);
}
