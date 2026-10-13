// Every raising VM call stays inside a native protected call.
// Protected callbacks contain only trivial objects: longjmp skips no destructor.
#include "allocator.h"
#include "runtime_errors.h"
#include "lua.h"
#include "lualib.h"
#include "luacode.h"
#include <cstdlib>
#include <cstring>
#include <limits>

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
static void interrupt(lua_State* state, int gc) {
    if (gc >= 0) return;
    ulua_vm* vm = static_cast<ulua_vm*>(lua_callbacks(state)->userdata);
    // A protected unwind must preserve the first recorded native stop.
    if (vm->flag || vm->trap_kind) luaL_error(state, "Run stopped");
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
static int start_request(lua_State*, ulua_vm*, const ulua_internal_capability*, const ulua_value*);
static int host_call(lua_State* state) {
    ulua_vm* vm = static_cast<ulua_vm*>(lua_callbacks(state)->userdata);
    const ulua_internal_capability* cap = static_cast<const ulua_internal_capability*>(lua_touserdata(state, lua_upvalueindex(1)));
    if (cap->desc.start) vm->request_starting = true;
    int code = 0;
    if (*cap->revoked) code = 12;
    else if (vm->steps >= vm->max_steps) code = SHIM_STEPS;
    else {
        ++vm->steps;
        ulua_value arguments[ULUA_MAX_HOST_PARAMETERS] = {};
        for (size_t i = 0; i < cap->desc.parameter_count; ++i) {
            ulua_value* a = &arguments[i];
            a->type = cap->desc.parameter_types[i];
            const int index = static_cast<int>(i + 1);
            if (a->type == ULUA_TYPE_NUMBER) a->number = luaL_checknumber(state, index);
            else if (a->type == ULUA_TYPE_BOOLEAN) a->boolean = lua_toboolean(state, index);
            else a->data = luaL_checklstring(state, index, &a->length);
        }
        if (cap->desc.start) return start_request(state, vm, cap, arguments);
        ulua_value result = {};
        code = cap->desc.function(cap->desc.userdata, arguments, cap->desc.parameter_count, &result);
        if (!code) {
            if (result.type == ULUA_TYPE_VOID) return 0;
            if (result.type == ULUA_TYPE_NUMBER) lua_pushnumber(state, result.number);
            else if (result.type == ULUA_TYPE_BOOLEAN) lua_pushboolean(state, result.boolean);
            else lua_pushlstring(state, result.data ? result.data : "", result.length);
            return 1;
        }
    }
    vm->flag = code;
    std::memcpy(vm->capability_name, cap->desc.name, cap->desc.name_length);
    vm->capability_name[cap->desc.name_length] = 0;
    // Rust and host frames return before this stop. Protected closures cannot catch it.
    luaL_error(state, "Host call stopped");
}
static int trap(lua_State* state) {
    ulua_vm* vm = static_cast<ulua_vm*>(lua_callbacks(state)->userdata);
    vm->trap_kind = luaL_checkinteger(state, 1);
    luaL_error(state, "Script trap");
}
static int register_errors(lua_State* state) {
    ulua_vm* vm = static_cast<ulua_vm*>(lua_callbacks(state)->userdata);
    vm->errors = lua_ref(state, 1);
    return 0;
}
static int check_resume(lua_State* state) {
    ulua_vm* vm = static_cast<ulua_vm*>(lua_callbacks(state)->userdata);
    // Classify the recorded stop and native class before the Error identity check.
    if (!vm->flag && !vm->trap_kind && lua_type(state, 1) == LUA_TSTRING)
        vm->flag = run_error_code(LUA_ERRRUN, run_message(lua_tostring(state, 1)));
    // Preserve the original error. This path creates no error object or location string.
    if (!vm->flag && !vm->trap_kind && lua_istable(state, 1) && vm->errors) {
        lua_getref(state, vm->errors);
        lua_pushvalue(state, 1);
        lua_rawget(state, -2);
        if (!lua_isnil(state, -1)) return 0;
    }
    lua_settop(state, 1);
#ifdef ULUA_MEASUREMENTS
    ++vm->resume_checks;
    if (!vm->resume_unwind) {
        vm->resume_snapshot = vm->allocation_calls;
        vm->resume_unwind = true;
    }
#endif
    lua_error(state);
}
static int register_async(lua_State* state) {
    ulua_vm* vm = static_cast<ulua_vm*>(lua_callbacks(state)->userdata);
    vm->drain = lua_ref(state, 1);
    vm->reset = lua_ref(state, 2);
    vm->promises = lua_ref(state, 3);
    vm->jobs = lua_ref(state, 4);
    vm->rejections = lua_ref(state, 5);
    vm->new_promise = lua_ref(state, 6);
    vm->fulfill = lua_ref(state, 7);
    return 0;
}
static void raise_call_error(lua_State* state, ulua_vm* vm) {
    // This function runs inside load or call, below the native protection boundary.
    const int error_index = lua_gettop(state);
    if (!vm->flag && !vm->trap_kind && lua_istable(state, error_index) && vm->errors) {
        lua_getref(state, vm->errors);
        lua_pushvalue(state, error_index);
        lua_rawget(state, -2);
        if (!lua_isnil(state, -1)) {
            lua_rawgetfield(state, error_index, "name");
            size_t name_size = 0;
            const char* name = lua_tolstring(state, -1, &name_size);
            lua_rawgetfield(state, error_index, "message");
            size_t text_size = 0;
            const char* text = lua_tolstring(state, -1, &text_size);
            if (name && text) {
                size_t n = name_size > 255 ? 255 : name_size;
                while (n && n < name_size && (static_cast<unsigned char>(name[n]) & 0xc0) == 0x80) --n;
                std::memcpy(vm->exception, name, n);
                if (n < 255) vm->exception[n++] = ':';
                if (n < 255) vm->exception[n++] = ' ';
                size_t m = text_size > 255 - n ? 255 - n : text_size;
                while (m && m < text_size && (static_cast<unsigned char>(text[m]) & 0xc0) == 0x80) --m;
                std::memcpy(vm->exception + n, text, m);
                vm->exception[n + m] = 0;
                vm->flag = SHIM_UNCAUGHT;
            }
        }
        lua_settop(state, error_index);
    }
    lua_error(state);
}
static int collect(lua_State* state);
static int setup(lua_State* state) {
    ulua_vm* vm = static_cast<ulua_vm*>(lua_tolightuserdata(state, 1));
    luaL_openlibs(state);
    lua_pushcfunction(state, output, "print");
    lua_setglobal(state, "print");
    // Reserve the collector closure before any run can exhaust the memory budget.
    lua_pushcfunction(state, collect, "collect");
    vm->collection = lua_ref(state, -1);
    lua_pop(state, 1);
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
#ifdef ULUA_MEASUREMENTS
    if (vm->resume_unwind) {
        vm->resume_allocations += vm->allocation_calls - vm->resume_snapshot;
        vm->resume_unwind = false;
    }
#endif
    vm->charging = false;
    const bool request_starting = vm->request_starting;
    vm->request_starting = false;
    vm->async_failed = status != LUA_OK;
    if (status == LUA_OK) return SHIM_OK;
    const char* text = run_message(lua_type(state, -1) == LUA_TSTRING ? lua_tostring(state, -1) : "VM returned a non-string error");
    message(error, text);
    int code = run_error_code(status, text);
    if (vm->flag) code = vm->flag;
    if (code == SHIM_STEPS) message(error, "Step limit exceeded");
    if (!vm->flag && vm->trap_kind == 1) {
        code = SHIM_INDEX;
        message(error, "Index out of range");
    }
    if (!vm->flag && vm->trap_kind == 2) {
        code = SHIM_ARRAY_MODIFIED;
        message(error, "Array modified");
    }
    if (!vm->flag && vm->trap_kind == 3) {
        code = SHIM_STRING_BOUNDARY;
        message(error, "String boundary");
    }
    if (!vm->flag && vm->trap_kind == 4) {
        code = SHIM_INVALID_ARGUMENT;
        message(error, "Invalid argument");
    }
    if (code == SHIM_UNCAUGHT) message(error, vm->exception);

    if (code >= 11 && code <= 13) message(error, vm->capability_name);
    vm->trap_kind = 0;
    vm->flag = 0;
    lua_settop(state, 0);
    // These operations do not allocate. Release queue roots before the collection.
    if (vm->jobs) {
        lua_getref(state, vm->jobs);
        lua_cleartable(state, -1);
        lua_pop(state, 1);
    }
    if (vm->rejections) {
        lua_getref(state, vm->rejections);
        lua_cleartable(state, -1);
        lua_pop(state, 1);
    }
    // Collection also has a native protection boundary.
    // Reclaim discarded factory objects before another failed start can enlarge the weak promise table.
    if (request_starting || code == SHIM_MEMORY || code == SHIM_STEPS) {
        lua_getref(state, vm->collection);
        lua_pcall(state, 0, 0, 0);
        lua_settop(state, 0);
    }
    return code;
}
#include "requests.h"
struct Load {
    ulua_vm* vm;
    const char* bytecode;
    size_t length;
    const ulua_internal_capability* grants;
    size_t grant_count;
};
static int load(lua_State* state) {
    Load* args = static_cast<Load*>(lua_tolightuserdata(state, 1));
    if (luau_load(state, "=script", args->bytecode, args->length, 0)) lua_error(state);
    if (!lua_checkstack(state, 7)) luaL_error(state, "stack limit");
    lua_pushcfunction(state, trap, "__ulua_trap");
    lua_createtable(state, static_cast<int>(args->grant_count), 0);
    for (size_t i = 0; i < args->grant_count; ++i) {
        void* storage = lua_newuserdata(state, sizeof(ulua_internal_capability));
        std::memcpy(storage, &args->grants[i], sizeof(ulua_internal_capability));
        lua_pushcclosure(state, host_call, "host", 1);
        lua_rawseti(state, -2, static_cast<int>(i + 1));
    }
    lua_pushcfunction(state, register_errors, "register_errors");
    lua_pushcfunction(state, check_resume, "check_resume");
    lua_pushcfunction(state, register_async, "register_async");
    if (lua_pcall(state, 5, 1, 0) != LUA_OK) raise_call_error(state, args->vm);
    if (args->vm->drain) {
        lua_getref(state, args->vm->drain);
        if (lua_pcall(state, 0, 0, 0) != LUA_OK) raise_call_error(state, args->vm);
    }
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
    ulua_task* task;
};
static int call(lua_State* state) {
    Call* args = static_cast<Call*>(lua_tolightuserdata(state, 1));
    if (args->vm->async_failed && args->vm->reset) {
        lua_getref(state, args->vm->reset);
        if (lua_pcall(state, 0, 0, 0) != LUA_OK) raise_call_error(state, args->vm);
        args->vm->async_failed = false;
    }
    if (args->vm->result_ref) { lua_unref(state, args->vm->result_ref); args->vm->result_ref = 0; }
    lua_getref(state, args->vm->exports);
    lua_getfield(state, -1, args->name);
    if (!lua_isfunction(state, -1)) luaL_error(state, "Generated export is not a function");
    for (size_t i = 0; i < args->count; ++i) {
        const ulua_value* value = &args->arguments[i];
        if (value->type == ULUA_TYPE_NUMBER) lua_pushnumber(state, value->number);
        else if (value->type == ULUA_TYPE_BOOLEAN) lua_pushboolean(state, value->boolean);
        else lua_pushlstring(state, value->data ? value->data : "", value->length);
    }
    if (lua_pcall(state, static_cast<int>(args->count), 1, 0) != LUA_OK)
        raise_call_error(state, args->vm);
    const int result_index = lua_gettop(state);
    bool promise = false;
    if (args->vm->promises && lua_istable(state, result_index)) {
        lua_getref(state, args->vm->promises);
        lua_pushvalue(state, result_index);
        lua_rawget(state, -2);
        promise = !lua_isnil(state, -1);
        lua_settop(state, result_index);
        if (promise) {
            // The host observes the returned promise, as an await of async main does.
            lua_pushboolean(state, true);
            lua_rawsetfield(state, result_index, "handled");
        }
    }
    if (args->vm->drain) {
        lua_getref(state, args->vm->drain);
        const int drained = lua_pcall(state, 0, 0, 0);
        if (drained != LUA_OK) {
            // An export rejection takes precedence only over an unhandled rejection, never a stop.
            const int error_index = lua_gettop(state);
            if (promise && drained == LUA_ERRRUN && !args->vm->flag && !args->vm->trap_kind
                && lua_istable(state, error_index) && args->vm->errors) {
                lua_getref(state, args->vm->errors);
                lua_pushvalue(state, error_index);
                lua_rawget(state, -2);
                const bool exception = !lua_isnil(state, -1);
                lua_settop(state, error_index);
                if (exception) {
                    lua_rawgetfield(state, result_index, "state");
                    const bool rejected = lua_tointeger(state, -1) == 2;
                    lua_pop(state, 1);
                    if (rejected) {
                        lua_rawgetfield(state, result_index, "reason");
                        lua_replace(state, error_index);
                    }
                }
            }
            raise_call_error(state, args->vm);
        }
    }
    if (args->task) {
        args->task->promise = promise;
        args->task->reference = lua_ref(state, result_index);
        return 0;
    }
    if (promise) {
        lua_rawgetfield(state, result_index, "state");
        const int settled = lua_tointeger(state, -1);
        lua_pop(state, 1);
        if (settled == 0) { args->vm->flag = SHIM_PENDING; luaL_error(state, "Async export is pending"); }
        if (settled == 2) {
            lua_rawgetfield(state, result_index, "reason");
            raise_call_error(state, args->vm);
        }
        lua_rawgetfield(state, result_index, "value");
    }
    ulua_value value = {};
    value.type = args->result_type;
    if (value.type == ULUA_TYPE_NUMBER) {
        if (lua_type(state, -1) != LUA_TNUMBER) luaL_error(state, "Generated result has the wrong type");
        value.number = lua_tonumber(state, -1);
    } else if (value.type == ULUA_TYPE_BOOLEAN) {
        if (lua_type(state, -1) != LUA_TBOOLEAN) luaL_error(state, "Generated result has the wrong type");
        value.boolean = lua_toboolean(state, -1);
    }
    if (value.type == ULUA_TYPE_STRING) {
        if (lua_type(state, -1) != LUA_TSTRING) luaL_error(state, "Generated result has the wrong type");
        value.data = lua_tolstring(state, -1, &value.length);
        args->vm->result_ref = lua_ref(state, -1);
    }
    *args->result = value;
    return 0;
}
extern "C" int ulua_internal_shim_compile(const char* source, size_t length, char** out, size_t* out_length, char* error) noexcept {
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
extern "C" void ulua_internal_shim_bytecode_free(char* bytecode) noexcept { std::free(bytecode); }
extern "C" void ulua_internal_shim_free(ulua_vm* vm) noexcept {
    if (!vm) return;
    discard_records(vm);
    try { if (vm->state) lua_close(vm->state); } catch (...) { }
    std::free(vm);
}
extern "C" int ulua_internal_shim_new(const char* bytecode, size_t length, const ulua_host* host,
    const ulua_internal_capability* grants, size_t grant_count,
    const ulua_run_limits* limits, ulua_vm** out, char* error) noexcept {
    ulua_vm* vm = nullptr;
    try {
        vm = static_cast<ulua_vm*>(std::calloc(1, sizeof(ulua_vm)));
        if (!vm) { message(error, "Instance allocation failed"); return SHIM_MEMORY; }
        vm->budget = limits->max_memory_bytes > SIZE_MAX ? SIZE_MAX : static_cast<size_t>(limits->max_memory_bytes);
        vm->max_steps = limits->max_steps;
        vm->max_open_requests = limits->max_open_requests;
        vm->host = *host;
        vm->charging = true;
        vm->state = lua_newstate(allocate, vm);
        if (!vm->state) {
            message(error, "Memory limit exceeded at VM creation");
            int code = vm->flag == SHIM_STEPS ? SHIM_STEPS : SHIM_MEMORY;
            if (code == SHIM_STEPS) message(error, "Step limit exceeded");
            ulua_internal_shim_free(vm);
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
            Load args = {vm, bytecode, length, grants, grant_count};
            vm->charging = true;
            code = finish(vm, vm->thread, lua_cpcall(vm->thread, load, &args), error);
        }
        if (code) { ulua_internal_shim_free(vm); return code; }
        lua_settop(vm->thread, 0);
        *out = vm;
        return SHIM_OK;
    } catch (...) {
        ulua_internal_shim_free(vm);
        message(error, "VM creation raised a C++ exception");
        return SHIM_INTERNAL;
    }
}
extern "C" int ulua_internal_shim_call(ulua_vm* vm, const char* name, const ulua_value* arguments,
    size_t count, uint32_t result_type, ulua_value* result, char* error) noexcept {
    try {
        vm->steps = 0;
        vm->flag = 0;
        vm->trap_kind = 0;
        vm->charging = true;
#ifdef ULUA_MEASUREMENTS
        vm->allocation_bytes = 0;
        vm->allocation_steps = 0;
        vm->interrupts = 0;
#endif
        Call args = {vm, name, arguments, count, result_type, result, nullptr};
        int code = finish(vm, vm->thread, lua_cpcall(vm->thread, call, &args), error);
        lua_settop(vm->thread, 0);
        return code;
    } catch (...) {
        message(error, "VM call raised a C++ exception");
        return SHIM_INTERNAL;
    }
}

static int call_start(lua_State* state) {
    Call* args = static_cast<Call*>(lua_tolightuserdata(state, 1));
    ulua_vm* vm = args->vm;
    if (vm->next_task == UINT64_MAX) luaL_error(state, "Task ids exhausted");
    args->task = static_cast<ulua_task*>(allocate(vm, nullptr, 0, sizeof(ulua_task)));
    if (!args->task) luaL_error(state, "not enough memory");
    std::memset(args->task, 0, sizeof(ulua_task));
    args->task->id = ++vm->next_task;
    args->task->type = args->result_type;
    return call(state);
}
extern "C" int ulua_internal_shim_call_start(ulua_vm* vm, const char* name, const ulua_value* arguments,
    size_t count, uint32_t result_type, uint64_t* task, char* error) noexcept {
    try {
        begin_run(vm);
        Call args = {vm, name, arguments, count, result_type, nullptr, nullptr};
        int code = finish(vm, vm->thread, lua_cpcall(vm->thread, call_start, &args), error);
        if (code) {
            if (args.task) {
                if (args.task->reference) lua_unref(vm->thread, args.task->reference);
                allocate(vm, args.task, sizeof(ulua_task), 0);
            }
        } else {
            args.task->next = vm->tasks;
            vm->tasks = args.task;
            *task = args.task->id;
        }
        lua_settop(vm->thread, 0);
        return code;
    } catch (...) { message(error, "Task start raised a C++ exception"); return SHIM_INTERNAL; }
}
