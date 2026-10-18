// Measurement-only ablations. No product source or object is replaced.
#include "../../native/shim/allocator.h"
#include "lua.h"
#include "lualib.h"
#include "luacode.h"
#include <cstring>
#include <new>

// Levels: floor, accounting, protected, conversion, trampoline, native shim.
struct Boundary {
    lua_State* state = nullptr;
    ulua_vm charges = {};
    ulua_vm* shim = nullptr;
    int function_ref = 0;
    int direction = 0;
    int level = 0;
    ulua_host_fn callback = nullptr;
    ulua_internal_capability cap = {};
    uint8_t revoked = 0;
    uint32_t types[2] = {ULUA_TYPE_NUMBER, ULUA_TYPE_NUMBER};
    double sum = 0;
    char error[256] = {};
};
static void interrupt(lua_State* state, int gc) {
    if (gc >= 0) return;
    auto* vm = static_cast<ulua_vm*>(lua_callbacks(state)->userdata);
    if (vm->flag || vm->trap_kind) luaL_error(state, "Run stopped");
    if (vm->steps >= vm->max_steps) luaL_error(state, "Step limit exceeded");
    ++vm->steps;
}
static int add(lua_State* state) {
    lua_pushnumber(state, luaL_checknumber(state, 1) + luaL_checknumber(state, 2));
    return 1;
}
extern "C" int32_t ulua_boundary_add(void*, const ulua_value*, size_t, ulua_value*);
static int converted(lua_State* state) {
    ulua_value args[2] = {};
    for (int i = 0; i < 2; ++i) {
        args[i].type = ULUA_TYPE_NUMBER;
        args[i].number = luaL_checknumber(state, i + 1);
    }
    ulua_value result = {};
    // The direct cross-language call keeps the converted ABI values observable.
    if (ulua_boundary_add(nullptr, args, 2, &result)) luaL_error(state, "Host result failed");
    lua_pushnumber(state, result.number);
    return 1;
}
static int trampoline(lua_State* state) {
    auto* b = static_cast<Boundary*>(lua_tolightuserdata(state, lua_upvalueindex(1)));
    if (b->revoked || b->charges.steps >= b->charges.max_steps) luaL_error(state, "Host stopped");
    ++b->charges.steps;
    // Match the shim's fixed argument array and descriptor dispatch.
    ulua_value args[ULUA_MAX_HOST_PARAMETERS] = {};
    for (size_t i = 0; i < b->cap.desc.parameter_count; ++i) {
        args[i].type = b->cap.desc.parameter_types[i];
        args[i].number = luaL_checknumber(state, static_cast<int>(i + 1));
    }
    ulua_value result = {};
    int code = b->cap.desc.function(b->cap.desc.userdata, args, b->cap.desc.parameter_count, &result);
    if (code || result.type != ULUA_TYPE_NUMBER) luaL_error(state, "Host result failed");
    lua_pushnumber(state, result.number);
    return 1;
}
static const char* source(int direction, int level) {
    if (direction == 0)
        return "local function f(a: number, b: number): number return a + b end return {f=f}";
    // The shim passes grants as chunk arguments. All loops capture the same local binding.
    if (level == 5)
        return "local _, hosts = ... local hostAdd = hosts[1] local function f(): number local s = 0 local i = 0 while i < 1000 do s = s + hostAdd(i, 1) i = i + 1 end return s end return {f=f}";
    return "local hostAdd = hostAdd local function f(): number local s = 0 local i = 0 while i < 1000 do s = s + hostAdd(i, 1) i = i + 1 end return s end return {f=f}";
}
struct Call { Boundary* b; double a; double sum; ulua_value args[2]; ulua_value result; };
static int protected_call(lua_State* state) {
    auto* c = static_cast<Call*>(lua_tolightuserdata(state, 1));
    Boundary* b = c->b;
    lua_getref(state, b->function_ref);
    if (b->direction == 0) {
        if (b->level >= 3) {
            for (int i = 0; i < 2; ++i) {
                const ulua_value* v = &c->args[i];
                if (v->type == ULUA_TYPE_NUMBER) lua_pushnumber(state, v->number);
                else if (v->type == ULUA_TYPE_BOOLEAN) lua_pushboolean(state, v->boolean);
                else lua_pushlstring(state, v->data ? v->data : "", v->length);
            }
        } else {
            lua_pushnumber(state, c->a);
            lua_pushnumber(state, 1);
        }
    }
    if (lua_pcall(state, b->direction == 0 ? 2 : 0, 1, 0)) lua_error(state);
    if (b->level >= 3) {
        if (lua_type(state, -1) != LUA_TNUMBER) luaL_error(state, "Result type");
        c->result.type = ULUA_TYPE_NUMBER;
        c->result.number = lua_tonumber(state, -1);
        c->sum = c->result.number;
    } else c->sum = lua_tonumber(state, -1);
    return 0;
}
extern "C" void ulua_boundary_free(Boundary* b) noexcept {
    if (!b) return;
    if (b->shim) ulua_internal_shim_free(b->shim);
    if (b->state) lua_close(b->state);
    delete b;
}
extern "C" Boundary* ulua_boundary_new(int direction, int level, ulua_host_fn callback, char* error) noexcept {
    // The shared allocator also defines the record helper. This probe allocates VM storage only.
    (void)&allocate_record;
    Boundary* b = new (std::nothrow) Boundary;
    if (!b) { std::strcpy(error, "No host storage"); return nullptr; }
    char* bytecode = nullptr;
    try {
        b->direction = direction;
        b->level = level;
        b->callback = callback;
        b->charges.budget = 1073741824;
        b->charges.max_steps = 10000000000ULL;
        b->charges.charging = true;
        b->cap.desc.struct_size = sizeof(ulua_capability_desc);
        b->cap.desc.name = "hostAdd";
        b->cap.desc.name_length = 7;
        b->cap.desc.parameter_types = b->types;
        b->cap.desc.parameter_count = 2;
        b->cap.desc.result_type = ULUA_TYPE_NUMBER;
        b->cap.desc.function = callback;
        b->cap.revoked = &b->revoked;
        size_t length = 0;
        const char* text = source(direction, level);
        bytecode = luau_compile(text, std::strlen(text), nullptr, &length);
        if (!bytecode) { std::strcpy(error, "No bytecode"); ulua_boundary_free(b); return nullptr; }
        if (level == 5) {
            ulua_host host = {};
            host.struct_size = sizeof(host);
            ulua_run_limits limits = {};
            limits.struct_size = sizeof(limits);
            limits.max_steps = b->charges.max_steps;
            limits.max_memory_bytes = b->charges.budget;
            limits.max_open_requests = 256;
            const char* const export_names[] = {"f"};
            int code = ulua_internal_shim_new(bytecode, length, &host, &b->cap, direction ? 1 : 0, export_names, 1, &limits, &b->shim, error);
            std::free(bytecode);
            bytecode = nullptr;
            if (code) { ulua_boundary_free(b); return nullptr; }
            return b;
        }
        b->state = level >= 1 ? lua_newstate(allocate, &b->charges) : luaL_newstate();
        if (!b->state) { std::free(bytecode); std::strcpy(error, "No VM"); ulua_boundary_free(b); return nullptr; }
        lua_State* state = b->state;
        lua_callbacks(state)->userdata = &b->charges;
        luaL_openlibs(state);
        if (direction) {
            if (level >= 4) {
                lua_pushlightuserdata(state, b);
                lua_pushcclosure(state, trampoline, "hostAdd", 1);
            } else lua_pushcfunction(state, level >= 3 ? converted : add, "hostAdd");
            lua_setglobal(state, "hostAdd");
        }
        luaL_sandbox(state);
        luaL_sandboxthread(state);
        int code = luau_load(state, "boundary", bytecode, length, 0);
        std::free(bytecode);
        bytecode = nullptr;
        if (!code) code = lua_pcall(state, 0, 1, 0);
        if (code) {
            const char* message = lua_tostring(state, -1);
            std::strncpy(error, message ? message : "Load failed", 255);
            ulua_boundary_free(b);
            return nullptr;
        }
        b->charges.exports = lua_ref(state, 1);
        lua_getfield(state, 1, "f");
        if (!lua_isfunction(state, -1)) {
            std::strcpy(error, "Missing export");
            ulua_boundary_free(b);
            return nullptr;
        }
        b->function_ref = lua_ref(state, -1);
        // Slot 1 is exports; slot 2 is the function, for the direct floor.
        if (level >= 1) lua_callbacks(state)->interrupt = interrupt;
        return b;
    } catch (...) {
        std::free(bytecode);
        ulua_boundary_free(b);
        std::strcpy(error, "Native host exception");
        return nullptr;
    }
}
extern "C" int ulua_boundary_region(Boundary* b) noexcept {
    b->sum = 0;
    b->charges.steps = 0;
    const int count = b->direction == 0 ? 1000 : 1;
    // The floor pays no ABI value construction or protected-wrapper bookkeeping.
    if (b->level < 2) {
        for (int i = 0; i < count; ++i) {
            lua_pushvalue(b->state, 2);
            if (b->direction == 0) {
                lua_pushnumber(b->state, i);
                lua_pushnumber(b->state, 1);
            }
            int code = lua_pcall(b->state, b->direction == 0 ? 2 : 0, 1, 0);
            double result = lua_tonumber(b->state, -1);
            lua_settop(b->state, 2);
            if (code) return code;
            b->sum += result;
        }
        return 0;
    }
    for (int i = 0; i < count; ++i) {
        Call c = {};
        c.b = b;
        c.a = i;
        if (b->level >= 3) {
            c.args[0].type = c.args[1].type = ULUA_TYPE_NUMBER;
            c.args[0].number = i;
            c.args[1].number = 1;
        }
        int code;
        if (b->level == 5) {
            code = ulua_internal_shim_call(b->shim, 0, c.args, b->direction == 0 ? 2 : 0,
                ULUA_TYPE_NUMBER, &c.result, b->error);
            c.sum = c.result.number;
        } else {
            code = lua_cpcall(b->state, protected_call, &c);
            lua_settop(b->state, 2);
        }
        if (code) return code;
        b->sum += c.sum;
    }
    return 0;
}
extern "C" double ulua_boundary_result(Boundary* b) noexcept { return b->sum; }

extern "C" const char* ulua_boundary_error(Boundary* b) noexcept { return b->error; }
