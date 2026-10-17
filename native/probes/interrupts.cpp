// Count executed VM instructions with production bytecode, independent of emitter counters.
#include "../shim/ulua_shim.h"
#include "lua.h"
#include "lualib.h"
#include "lstate.h"
#include "../shim/math.h"
#include <algorithm>
#include <cstdint>
#include <unordered_map>
#include <vector>
#include <string>
#include <limits>

struct Statement {
    int line = -1;
    const Proto* function = nullptr;
    const Instruction* pc = nullptr;
    uint64_t instructions = 0;
};
struct Gap {
    uint64_t current = 0;
    uint64_t largest = 0;
    uint64_t interrupts = 0;
    uint64_t total = 0;
    uint64_t statement = 0;
    int statement_line = -1;
    int drain = 0;
    uint64_t run = 0;
    uint64_t largest_run = 0;
    std::vector<bool> execution;
    std::vector<int> statement_lines;
    std::unordered_map<lua_State*, std::vector<Statement>> frames;
};
static void step(lua_State* state, lua_Debug* debug) {
    auto* gap = static_cast<Gap*>(lua_callbacks(state)->userdata);
    ++gap->current;
    ++gap->total;
    // Keep the caller's statement across calls. Callee instructions belong to its own lines.
    auto& frames = gap->frames[state];
    frames.resize(static_cast<size_t>(lua_stackdepth(state)));
    auto& frame = frames.back();
    const int line = debug->currentline > 0 && static_cast<size_t>(debug->currentline) < gap->statement_lines.size()
        ? gap->statement_lines[debug->currentline] : debug->currentline;
    const auto* function = clvalue(state->ci->func)->l.p;
    const auto* pc = state->ci->savedpc;
    if (frame.line != line || frame.function != function || pc <= frame.pc) {
        if (debug->currentline > 0 && static_cast<size_t>(debug->currentline) < gap->execution.size()
            && gap->execution[debug->currentline]) {
            ++gap->run;
            gap->largest_run = std::max(gap->largest_run, gap->run);
        }
        frame = {line, function, pc, 0};
    }
    frame.pc = pc;
    ++frame.instructions;
    if (frame.instructions > gap->statement) {
        gap->statement = frame.instructions;
        gap->statement_line = frame.line;
    }
}
static void interrupt(lua_State* state, int gc) {
    if (gc >= 0) return;
    auto* gap = static_cast<Gap*>(lua_callbacks(state)->userdata);
    gap->largest = std::max(gap->largest, gap->current);
    gap->current = 0;
    gap->run = 0;
    ++gap->interrupts;
}
// Instruction counting excludes native bodies; preserve the string for the next call.
static int case_identity(unsigned op, double cursor, double, int, const unsigned char* input, size_t len, unsigned char* output, double* consumed) {
    // Native work is outside this instruction measurement. The parse shape reads the number 1.
    if (op == 14) {
        if (cursor != 0) return 8;
        const double value = 1;
        std::memcpy(output + 136, &value, sizeof(value));
        *consumed = 1;
        return 1;
    }
    std::memcpy(output, input, len);
    return static_cast<int>(len);
}
static int unused(lua_State*) { return 0; }
static int register_async(lua_State* state) {
    auto* gap = static_cast<Gap*>(lua_callbacks(state)->userdata);
    gap->drain = lua_ref(state, 1);
    return 0;
}
static int drain(lua_State* state, Gap& gap) {
    if (!gap.drain) return 0;
    lua_getref(state, gap.drain);
    return lua_pcall(state, 0, 0, 0);
}
extern "C" int ulua_internal_probe_interrupts(const char* source, size_t length, int branch,
    uint64_t* largest, uint64_t* interrupts, uint64_t* statement,
    int* statement_line, uint64_t* total, uint64_t* run, double* result, char* error, ulua_math_fn math_dispatch) {
    *result = std::numeric_limits<double>::quiet_NaN();
    char* bytecode = nullptr;
    size_t size = 0;
    int status = ulua_internal_shim_compile(source, length, &bytecode, &size, error);
    if (status) return status;
    lua_State* state = luaL_newstate();
    if (!state) { ulua_internal_shim_bytecode_free(bytecode); return SHIM_MEMORY; }
    luaL_openlibs(state);
    ulua_vm math_vm = {};
    math_vm.max_steps = UINT64_MAX;
    math_vm.math_dispatch = math_dispatch;
    math_vm.number_dispatch = case_identity;
    seed_random(&math_vm, UINT64_C(0x5355425352454144));
    push_math(state, &math_vm);
    const int math_ref = lua_ref(state, -1);
    lua_pop(state, 1);
    luaL_sandbox(state);
    luaL_sandboxthread(state);
    status = luau_load(state, "interrupt-probe", bytecode, size, 0);
    ulua_internal_shim_bytecode_free(bytecode);
    Gap gap;
    // Source lines identify statements independently of the emitter's path counters.
    gap.execution.push_back(false);
    gap.statement_lines.push_back(0);
    std::string text(source, length);
    for (size_t begin = 0; begin < text.size();) {
        size_t end = text.find('\n', begin);
        if (end == std::string::npos) end = text.size();
        std::string line = text.substr(begin, end - begin);
        size_t first = line.find_first_not_of(" \t");
        line = first == std::string::npos ? "" : line.substr(first);
        bool execution = !line.empty() && line.rfind("--", 0) != 0 && line.rfind("type ", 0) != 0
            && line != "else" && line.rfind("end", 0) != 0 && line != "do" && line != "repeat"
            && line != "while true do" && line != "until true" && line != "__ulua_interrupt()" && line != "nil"
            && line != "," && line != ")" && line != ")), 0)" && line != "}";
        const int source_line = static_cast<int>(gap.statement_lines.size());
        // Fixed prelude boolean continuations belong to the same return statement.
        const bool continuation = line.rfind("or ", 0) == 0 || line.rfind("and ", 0) == 0;
        gap.statement_lines.push_back(continuation ? gap.statement_lines.back() : source_line);
        gap.execution.push_back(execution && !continuation);
        begin = end + 1;
    }
    lua_callbacks(state)->userdata = &gap;
    lua_callbacks(state)->debugstep = step;
    lua_callbacks(state)->interrupt = interrupt;
    lua_singlestep(state, 1);
    if (!status) {
        lua_pushcfunction(state, unused, "trap");
        lua_pushnil(state);
        lua_pushcfunction(state, unused, "register");
        lua_pushcfunction(state, unused, "stop");
        lua_pushcfunction(state, register_async, "async_register");
        lua_getref(state, math_ref);
        status = lua_pcall(state, 6, 1, 0);
        if (!status) status = drain(state, gap);
    }
    if (!status) {
        lua_getfield(state, -1, "main");
        lua_pushboolean(state, branch);
        status = lua_pcall(state, 1, 1, 0);
        if (!status) status = drain(state, gap);
        if (!status && lua_istable(state, -1)) {
            // The probe observes the export promise after the production drain boundary.
            lua_getfield(state, -1, "state");
            if (lua_tointeger(state, -1) != 1) status = SHIM_INTERNAL;
            lua_pop(state, 1);
            lua_getfield(state, -1, "value");
        }
        if (!status && lua_isnumber(state, -1)) *result = lua_tonumber(state, -1);
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
    *statement = gap.statement;
    *statement_line = gap.statement_line;
    *total = gap.total;
    *run = gap.largest_run;
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

// Measure one production entry, then a short batch that amortizes the host clock cost.
// Compilation and module initialization stay outside the call measurements.
#include "../shim/allocator.h"
#include <chrono>
extern "C" int ulua_internal_probe_entry(const char* bytecode, size_t length,
    const ulua_run_limits* limits, uint64_t* steps, double* seconds, char* error) {
    ulua_host host = {};
    host.struct_size = sizeof(host);
    ulua_vm* vm = nullptr;
    int status = ulua_internal_shim_new(bytecode, length, &host, nullptr, 0, limits, &vm, error);
    if (!status) {
        ulua_value result = {};
        status = ulua_internal_shim_call(vm, "main", nullptr, 0, ULUA_TYPE_NUMBER, &result, error);
        *steps = vm->steps;
        const auto begin = std::chrono::steady_clock::now();
        constexpr int samples = 100;
        for (int i = 0; !status && i < samples; ++i)
            status = ulua_internal_shim_call(vm, "main", nullptr, 0, ULUA_TYPE_NUMBER, &result, error);
        *seconds = std::chrono::duration<double>(std::chrono::steady_clock::now() - begin).count() / samples;
    }
    ulua_internal_shim_free(vm);
    return status;
}
