// Isolate native charges from CALL interrupts in the product Math and Number closures.
#include "../shim/ulua_math.h"
#include "../shim/runtime_errors.h"
#include "lualib.h"
#include <cstring>

struct MathBudget {
    ulua_vm vm = {};
    const char* bytecode;
    size_t size;
    uint64_t budget;
    uint64_t interrupts = 0;
    uint64_t interrupt_stops = 0;
    uint64_t completed = 0;
    int marks = 0;
    bool armed = false;
};
static void budget_interrupt(lua_State* state, int gc) {
    if (gc >= 0) return;
    auto* probe = static_cast<MathBudget*>(lua_callbacks(state)->userdata);
    if (probe->armed) {
        ++probe->interrupts;
        if (probe->vm.steps >= probe->vm.max_steps) ++probe->interrupt_stops;
    }
    charge_steps(state, &probe->vm, 1);
}
static int arm_budget(lua_State* state) {
    auto* probe = static_cast<MathBudget*>(lua_callbacks(state)->userdata);
    probe->vm.steps = 0;
    probe->vm.max_steps = probe->budget;
    probe->armed = true;
    return 0;
}
static int run_budget(lua_State* state) {
    auto* probe = static_cast<MathBudget*>(lua_tolightuserdata(state, 1));
    lua_callbacks(state)->userdata = probe;
    lua_callbacks(state)->interrupt = budget_interrupt;
    if (luau_load(state, "=script", probe->bytecode, probe->size, 0)) lua_error(state);
    lua_pushcfunction(state, arm_budget, "arm_budget");
    push_math(state, &probe->vm);
    lua_createtable(state, 3, 0);
    probe->marks = lua_ref(state, -1);
    const int status = lua_pcall(state, 3, 1, 0);
    lua_getref(state, probe->marks);
    for (int i = 1; i <= 3; ++i) {
        lua_rawgeti(state, -1, i);
        if (!lua_isnil(state, -1)) ++probe->completed;
        lua_pop(state, 1);
    }
    lua_pop(state, 1);
    if (status) lua_error(state);
    return 0;
}
extern "C" uint64_t ulua_internal_probe_native_call_steps() noexcept {
    return NATIVE_CALL_STEPS;
}
extern "C" int ulua_internal_probe_math_charge(uint64_t budget, int random,
    ulua_math_fn dispatch, uint64_t* counters, char* error) noexcept {
    const char* source = random
        ? "local arm,m,marks=...\narm()\nmarks[1]=m.random()\nmarks[2]=m.random()\nmarks[3]=m.random()\n"
        : "local arm,m,marks=...\narm()\nmarks[1]=m.sin(1)\nmarks[2]=m.sin(1)\nmarks[3]=m.sin(1)\n";
    char* bytecode = nullptr;
    size_t size = 0;
    int code = ulua_internal_shim_compile(source, std::strlen(source), &bytecode, &size, error);
    if (code) return code;
    lua_State* state = luaL_newstate();
    if (!state) { ulua_internal_shim_bytecode_free(bytecode); return SHIM_MEMORY; }
    MathBudget probe;
    probe.bytecode = bytecode;
    probe.size = size;
    probe.budget = budget;
    probe.vm.max_steps = UINT64_MAX;
    probe.vm.math_dispatch = dispatch;
    seed_random(&probe.vm, UINT64_C(0x5355425352454144));
    const int status = lua_cpcall(state, run_budget, &probe);
    code = probe.vm.flag ? probe.vm.flag : (status ? SHIM_INTERNAL : SHIM_OK);
    if (status) {
        const char* text = lua_tostring(state, -1);
        std::strncpy(error, text ? run_message(text) : "No Math probe diagnostic", 255);
        error[255] = 0;
    }
    counters[0] = probe.vm.steps;
    counters[1] = probe.interrupts;
    counters[2] = probe.interrupt_stops;
    counters[3] = probe.completed;
    lua_close(state);
    ulua_internal_shim_bytecode_free(bytecode);
    return code;
}

extern "C" uint64_t ulua_internal_probe_native_bytes_per_step() noexcept {
    return NATIVE_BYTES_PER_STEP;
}
// Exercise the real product closure with exactly known native charges.
extern "C" int ulua_internal_probe_number_charge(uint64_t budget, int formatting,
    ulua_number_fn dispatch, uint64_t* counters, char* error) noexcept {
    const char* source = formatting == 2
        ? "local arm,m,marks=...\nlocal s='aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa'\narm()\nmarks[1]=m.case_upper(s)\nmarks[2]=m.case_upper(s)\nmarks[3]=m.case_upper(s)\n"
        : formatting
        ? "local arm,m,marks=...\narm()\nmarks[1]=m.number_toFixed(2.5,100)\nmarks[2]=m.number_toFixed(2.5,100)\nmarks[3]=m.number_toFixed(2.5,100)\n"
        : "local arm,m,marks=...\nlocal s='                               1'\narm()\nmarks[1]=m.number_parseInt(s,10)\nmarks[2]=m.number_parseInt(s,10)\nmarks[3]=m.number_parseInt(s,10)\n";
    char* bytecode = nullptr;
    size_t size = 0;
    int code = ulua_internal_shim_compile(source, std::strlen(source), &bytecode, &size, error);
    if (code) return code;
    lua_State* state = luaL_newstate();
    if (!state) { ulua_internal_shim_bytecode_free(bytecode); return SHIM_MEMORY; }
    MathBudget probe;
    probe.bytecode = bytecode;
    probe.size = size;
    probe.budget = budget;
    probe.vm.max_steps = UINT64_MAX;
    probe.vm.number_dispatch = dispatch;
    lua_pushcfunction(state, run_budget, "run_budget");
    lua_pushlightuserdata(state, &probe);
    const int status = lua_pcall(state, 1, 0, 0);
    code = status ? (probe.vm.flag ? probe.vm.flag : SHIM_INTERNAL) : 0;
    if (status) {
        const char* text = lua_tostring(state, -1);
        if (text) { std::strncpy(error, text, 255); error[255] = 0; }
    }
    counters[0] = probe.vm.steps;
    counters[1] = probe.interrupts;
    counters[2] = probe.interrupt_stops;
    counters[3] = probe.completed;
    lua_close(state);
    ulua_internal_shim_bytecode_free(bytecode);
    return code;
}
