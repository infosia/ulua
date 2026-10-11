// Test-only raw Luau reaches library limits that the P1 surface cannot express.
#include "../shim/runtime_errors.h"
#include "lualib.h"
#include "ldo.h"
#include "lstate.h"
#include <cstdio>
#include <cstring>

struct ErrorCase {
    const char* source;
    int code;
    const char* message;
};
static int hard_lua_depth(lua_State* state) {
    luaD_reallocCI(state, LUAI_MAXCALLS + (LUAI_MAXCALLS >> 3));
    luaD_growCI(state);
    return 0;
}
static int hard_c_depth(lua_State* state) {
    state->nCcalls = LUAI_MAXCCALLS + (LUAI_MAXCCALLS >> 3);
    luaD_checkCstack(state);
    return 0;
}
extern "C" int ulua_internal_probe_runtime_errors() noexcept {
    // Reach both unlabelled LUA_ERRERR depth sites without an error handler.
    // lua_cpcall restores the native call count and shrinks the call-info allocation.
    lua_State* state = luaL_newstate();
    if (!state) return -1;
    int lua_status = lua_cpcall(state, hard_lua_depth, nullptr);
    int lua_code = run_error_code(lua_status, lua_tostring(state, -1));
    lua_settop(state, 0);
    int c_status = lua_cpcall(state, hard_c_depth, nullptr);
    int c_code = run_error_code(c_status, lua_tostring(state, -1));
    lua_close(state);
    if (lua_status != LUA_ERRERR || c_status != LUA_ERRERR || lua_code != SHIM_STACK || c_code != SHIM_STACK) return -2;
    const ErrorCase cases[] = {
        {"return {main=function() string.rep('ab',1073741824) end}", SHIM_MEMORY, "resulting string too large"},
        {"return {main=function() table.create(67108865) end}", SHIM_MEMORY, "table overflow"},
        {"return {main=function() string.packsize('c1073741825') end}", SHIM_MEMORY, "size specifier is too large"},
        {"return {main=function() string.packsize('c1073741824c1') end}", SHIM_MEMORY,
            "invalid argument #1 to 'packsize' (format result too large)"},
        {"return {main=function() table.move({},-2147483647,2147483647,1) end}", SHIM_MEMORY,
            "invalid argument #3 to 'move' (too many elements to move)"},
        {"return {main=function() table.unpack({},1,2147483647) end}", SHIM_STACK, "too many results to unpack"},
        {"return {main=function() string.byte(string.rep('a',8001),1,8001) end}", SHIM_STACK,
            "stack overflow (string slice too long)"},
        {"return {main=function() string.match(string.rep('a',33),string.rep('(a)',33)) end}", SHIM_MEMORY,
            "too many captures"},
        {"return {main=function() string.match(string.rep('a',300),string.rep('a?',300)) end}", SHIM_STACK, "pattern too complex"},
        {"local t={} setmetatable(t,{__index=t}) return {main=function() local x=t.x end}", SHIM_STACK,
            "'__index' chain too long; possible loop"},
        {"local t={} setmetatable(t,{__newindex=t}) return {main=function() t.x=1 end}", SHIM_STACK,
            "'__newindex' chain too long; possible loop"},
        {"local function f() return string.gsub('x','x',f) end return {main=f}", SHIM_STACK, "C stack overflow"},
        // Similar words and payload colons cannot turn an unrelated error into a limit.
        {"return {main=function() error('kept:123: stack overflow detail') end}", SHIM_INTERNAL,
            "kept:123: stack overflow detail"},
        {"return {main=function() error('error in error handling') end}", SHIM_INTERNAL, "error in error handling"},
        {"return {main=function() error('stack overflow (unrelated)') end}", SHIM_INTERNAL, "stack overflow (unrelated)"},
        {"return {main=function() local t={} t() end}", SHIM_INTERNAL, "attempt to call a table value"},
        {"return {main=function() string.packsize('i17') end}", SHIM_INTERNAL, "integral size (17) out of limits [1,16]"},
    };
    ulua_host host = {};
    host.struct_size = sizeof(host);
    ulua_run_limits limits = {};
    limits.struct_size = sizeof(limits);
    limits.max_memory_bytes = 64 * 1024 * 1024;
    limits.max_steps = 10000;
    int index = 0;
    for (const ErrorCase& test : cases) {
        ++index;
        char error[256] = {};
        char* bytecode = nullptr;
        size_t length = 0;
        int code = ulua_internal_shim_compile(test.source, std::strlen(test.source), &bytecode, &length, error);
        ulua_vm* vm = nullptr;
        if (!code) code = ulua_internal_shim_new(bytecode, length, &host, &limits, &vm, error);
        ulua_internal_shim_bytecode_free(bytecode);
        if (code) { std::fprintf(stderr, "runtime fixture %d setup: %d %s\n", index, code, error); return index; }
        ulua_value result = {};
        code = ulua_internal_shim_call(vm, "main", nullptr, 0, ULUA_TYPE_VOID, &result, error);
        ulua_internal_shim_free(vm);
        if (code != test.code || std::strcmp(error, test.message) != 0) {
            std::fprintf(stderr, "runtime fixture %d: %d %s\n", index, code, error);
            return index;
        }
    }
    return 0;
}
