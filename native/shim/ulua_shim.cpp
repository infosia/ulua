// Every raising VM call stays inside a native protected call.
// Protected callbacks contain only trivial objects: longjmp skips no destructor.
#include "ulua_shim.h"
#include "lua.h"
#include "lualib.h"
#include "luacode.h"
#include <cstdlib>
#include <cstring>
#include <limits>

struct ulua_vm {
    lua_State* state;
    lua_State* thread;
    size_t budget;
    size_t used;
    uint64_t max_steps;
    uint64_t steps;
    int flag;
    int exports;
    bool charging;
#ifdef ULUA_MEASUREMENTS
    uint64_t allocation_bytes;
    uint64_t allocation_steps;
    uint64_t interrupts;
#endif
    ulua_host host;
};
static void message_bytes(char* out, const char* text, size_t length) {
    if (!out) return;
    size_t n = length;
    if (n > 255) n = 255;
    // Preserve UTF-8 boundaries if truncation reaches a continuation byte.
    while (n && n < length && (static_cast<unsigned char>(text[n]) & 0xc0) == 0x80) --n;
    std::memcpy(out, text, n);
    out[n] = 0;
}
static void message(char* out, const char* text) { message_bytes(out, text, std::strlen(text)); }
static void* allocate(void* userdata, void* pointer, size_t old_size, size_t new_size) {
    ulua_vm* vm = static_cast<ulua_vm*>(userdata);
    size_t old = pointer ? old_size : 0;
    if (!new_size) {
        std::free(pointer);
        vm->used -= old;
        return nullptr;
    }
    if (vm->charging) {
        const uint64_t charge = new_size / ALLOCATION_STEP_BYTES + (new_size % ALLOCATION_STEP_BYTES != 0);
        if (vm->steps > vm->max_steps || charge > vm->max_steps - vm->steps) {
            vm->flag = SHIM_STEPS;
            return nullptr;
        }
        vm->steps += charge;
#ifdef ULUA_MEASUREMENTS
        vm->allocation_bytes += new_size;
        vm->allocation_steps += charge;
#endif
    }
    size_t base = vm->used - old;
    if (new_size > vm->budget || base > vm->budget - new_size) return nullptr;
    void* result = std::realloc(pointer, new_size);
    if (result) vm->used = base + new_size;
    return result;
}
static void interrupt(lua_State* state, int gc) {
    if (gc >= 0) return;
    ulua_vm* vm = static_cast<ulua_vm*>(lua_callbacks(state)->userdata);
    if (vm->steps >= vm->max_steps) {
        vm->flag = SHIM_STEPS;
        luaL_error(state, "Step limit exceeded");
    }
    ++vm->steps;
#ifdef ULUA_MEASUREMENTS
    ++vm->interrupts;
#endif
}
static int output(lua_State* state) {
    ulua_vm* vm = static_cast<ulua_vm*>(lua_callbacks(state)->userdata);
    size_t length = 0;
    const char* text = luaL_checklstring(state, 1, &length);
    int status = vm->host.output ? vm->host.output(vm->host.userdata, text, length) : 0;
    if (status) {
        vm->flag = SHIM_HOST;
        // The host and Rust frames have returned before this raise.
        luaL_error(state, "Output callback failed");
    }
    return 0;
}
static int setup(lua_State* state) {
    ulua_vm* vm = static_cast<ulua_vm*>(lua_tolightuserdata(state, 1));
    luaL_openlibs(state);
    lua_pushcfunction(state, output, "print");
    lua_setglobal(state, "print");
    luaL_sandbox(state);
    vm->thread = lua_newthread(state);
    lua_ref(state, -1);
    return 0;
}
static int sandbox(lua_State* state) {
    luaL_sandboxthread(state);
    return 0;
}
static int collect(lua_State* state) {
    lua_gc(state, LUA_GCCOLLECT, 0);
    return 0;
}
static int finish(ulua_vm* vm, lua_State* state, int status, char* error) {
    vm->charging = false;
    if (status == LUA_OK) return SHIM_OK;
    const char* text = lua_type(state, -1) == LUA_TSTRING ? lua_tostring(state, -1) : "VM returned a non-string error";
    message(error, text);
    int code = SHIM_INTERNAL;
    // A failed error-handler allocation can replace LUA_ERRMEM with LUA_ERRERR.
    if (status == LUA_ERRMEM || std::strcmp(text, "not enough memory") == 0) code = SHIM_MEMORY;
    else if (std::strstr(text, "stack overflow")) code = SHIM_STACK;
    if (vm->flag) code = vm->flag;
    vm->flag = 0;
    lua_settop(state, 0);
    // Collection also has a native protection boundary.
    if (code == SHIM_MEMORY || code == SHIM_STEPS) {
        lua_cpcall(state, collect, nullptr);
        lua_settop(state, 0);
    }
    return code;
}
struct Load {
    ulua_vm* vm;
    const char* bytecode;
    size_t length;
};
static int load(lua_State* state) {
    Load* args = static_cast<Load*>(lua_tolightuserdata(state, 1));
    if (luau_load(state, "=script", args->bytecode, args->length, 0)) lua_error(state);
    lua_call(state, 0, 1);
    if (!lua_istable(state, -1)) luaL_error(state, "Generated chunk returned no export table");
    args->vm->exports = lua_ref(state, -1);
    return 0;
}
struct Call {
    ulua_vm* vm;
    const char* name;
    const ulua_value* arguments;
    size_t count;
    uint32_t result_type;
    ulua_value* result;
};
static int call(lua_State* state) {
    Call* args = static_cast<Call*>(lua_tolightuserdata(state, 1));
    lua_getref(state, args->vm->exports);
    lua_getfield(state, -1, args->name);
    if (!lua_isfunction(state, -1)) luaL_error(state, "Generated export is not a function");
    for (size_t i = 0; i < args->count; ++i) {
        const ulua_value* value = &args->arguments[i];
        if (value->type == ULUA_TYPE_NUMBER) lua_pushnumber(state, value->number);
        else lua_pushboolean(state, value->boolean);
    }
    lua_call(state, static_cast<int>(args->count), args->result_type == ULUA_TYPE_VOID ? 0 : 1);
    ulua_value value = {};
    value.type = args->result_type;
    if (value.type == ULUA_TYPE_NUMBER) {
        if (lua_type(state, -1) != LUA_TNUMBER) luaL_error(state, "Generated result has the wrong type");
        value.number = lua_tonumber(state, -1);
    } else if (value.type == ULUA_TYPE_BOOLEAN) {
        if (lua_type(state, -1) != LUA_TBOOLEAN) luaL_error(state, "Generated result has the wrong type");
        value.boolean = lua_toboolean(state, -1);
    }
    *args->result = value;
    return 0;
}
extern "C" int ulua_shim_compile(const char* source, size_t length, char** out, size_t* out_length, char* error) noexcept {
    char* bytecode = nullptr;
    try {
        size_t size = 0;
        bytecode = luau_compile(source, length, nullptr, &size);
        if (!bytecode || !size) {
            std::free(bytecode);
            message(error, "Luau compiler returned no bytecode");
            return SHIM_INTERNAL;
        }
        if (bytecode[0] == 0) {
            message_bytes(error, bytecode + 1, size - 1);
            std::free(bytecode);
            return SHIM_INTERNAL;
        }
        *out = bytecode;
        *out_length = size;
        return SHIM_OK;
    } catch (...) {
        std::free(bytecode);
        message(error, "Luau compiler raised a C++ exception");
        return SHIM_INTERNAL;
    }
}
extern "C" void ulua_shim_bytecode_free(char* bytecode) noexcept { std::free(bytecode); }
extern "C" void ulua_shim_free(ulua_vm* vm) noexcept {
    if (!vm) return;
    try { if (vm->state) lua_close(vm->state); } catch (...) { }
    std::free(vm);
}
extern "C" int ulua_shim_new(const char* bytecode, size_t length, const ulua_host* host,
    const ulua_run_limits* limits, ulua_vm** out, char* error) noexcept {
    ulua_vm* vm = nullptr;
    try {
        vm = static_cast<ulua_vm*>(std::calloc(1, sizeof(ulua_vm)));
        if (!vm) { message(error, "Instance allocation failed"); return SHIM_MEMORY; }
        vm->budget = limits->max_memory_bytes > SIZE_MAX ? SIZE_MAX : static_cast<size_t>(limits->max_memory_bytes);
        vm->max_steps = limits->max_steps;
        vm->host = *host;
        vm->charging = true;
        vm->state = lua_newstate(allocate, vm);
        if (!vm->state) {
            message(error, "Memory limit exceeded at VM creation");
            int code = vm->flag == SHIM_STEPS ? SHIM_STEPS : SHIM_MEMORY;
            ulua_shim_free(vm);
            return code;
        }
        lua_callbacks(vm->state)->userdata = vm;
        int status = lua_cpcall(vm->state, setup, vm);
        int code = finish(vm, vm->state, status, error);
        if (!code) {
            vm->charging = true;
            code = finish(vm, vm->thread, lua_cpcall(vm->thread, sandbox, nullptr), error);
        }
        if (!code) {
            lua_callbacks(vm->state)->interrupt = interrupt;
            Load args = {vm, bytecode, length};
            vm->charging = true;
            code = finish(vm, vm->thread, lua_cpcall(vm->thread, load, &args), error);
        }
        if (code) { ulua_shim_free(vm); return code; }
        lua_settop(vm->thread, 0);
        *out = vm;
        return SHIM_OK;
    } catch (...) {
        ulua_shim_free(vm);
        message(error, "VM creation raised a C++ exception");
        return SHIM_INTERNAL;
    }
}
extern "C" int ulua_shim_call(ulua_vm* vm, const char* name, const ulua_value* arguments,
    size_t count, uint32_t result_type, ulua_value* result, char* error) noexcept {
    try {
        vm->steps = 0;
        vm->flag = 0;
        vm->charging = true;
#ifdef ULUA_MEASUREMENTS
        vm->allocation_bytes = 0;
        vm->allocation_steps = 0;
        vm->interrupts = 0;
#endif
        Call args = {vm, name, arguments, count, result_type, result};
        int code = finish(vm, vm->thread, lua_cpcall(vm->thread, call, &args), error);
        lua_settop(vm->thread, 0);
        return code;
    } catch (...) {
        message(error, "VM call raised a C++ exception");
        return SHIM_INTERNAL;
    }
}

#ifdef ULUA_MEASUREMENTS
// These probes exist only in the opt-in measurement build.
extern "C" void ulua_probe_run_stats(ulua_vm* vm, uint64_t* bytes, uint64_t* charges, uint64_t* interrupts) noexcept {
    *bytes = vm->allocation_bytes;
    *charges = vm->allocation_steps;
    *interrupts = vm->interrupts;
}
extern "C" int ulua_probe_allocation(size_t bytes, uint64_t steps) noexcept {
    ulua_vm vm = {};
    vm.budget = SIZE_MAX;
    vm.max_steps = steps;
    vm.charging = true;
    void* value = allocate(&vm, nullptr, 0, bytes);
    int result = value ? SHIM_OK : vm.flag;
    allocate(&vm, value, bytes, 0);
    return result;
}
#endif
