#ifndef ULUA_MATH_H
#define ULUA_MATH_H
#include "allocator.h"
#include "lua.h"
#include "lualib.h"
#include "steps.h"
#include <cstring>
// P7.1 Rev 2 Debug Math shapes: N=1/2/4/8 gave maxima 2.747/1.748/1.664/1.719.
// Select the smallest candidate whose maximum is at most 5.
static constexpr uint64_t NATIVE_CALL_STEPS = 1;
static void seed_random(ulua_vm* vm, uint64_t seed) {
    for (auto& word : vm->random_state) {
        uint64_t z = (seed += UINT64_C(0x9E3779B97F4A7C15));
        z = (z ^ (z >> 30)) * UINT64_C(0xBF58476D1CE4E5B9);
        z = (z ^ (z >> 27)) * UINT64_C(0x94D049BB133111EB);
        word = z ^ (z >> 31);
    }
}
static uint64_t rotate(uint64_t x, int k) { return (x << k) | (x >> (64 - k)); }
static int math_random(lua_State* state) {
    auto* vm = static_cast<ulua_vm*>(lua_tolightuserdata(state, lua_upvalueindex(1)));
    charge_steps(state, vm, NATIVE_CALL_STEPS);
    auto* s = vm->random_state;
    const uint64_t x = rotate(s[0] + s[3], 23) + s[0];
    const uint64_t t = s[1] << 17;
    s[2] ^= s[0]; s[3] ^= s[1]; s[1] ^= s[2]; s[0] ^= s[3];
    s[2] ^= t; s[3] = rotate(s[3], 45);
    lua_pushnumber(state, static_cast<double>(x >> 11) * 0x1p-53);
    return 1;
}
static int math_native(lua_State* state) {
    const unsigned op = static_cast<unsigned>(lua_tointeger(state, lua_upvalueindex(1)));
    const double a = lua_tonumber(state, 1);
    const double b = lua_tonumber(state, 2);
    auto* vm = static_cast<ulua_vm*>(lua_tolightuserdata(state, lua_upvalueindex(2)));
    charge_steps(state, vm, NATIVE_CALL_STEPS);
    if (!vm->math_dispatch) luaL_error(state, "Missing Math dispatcher");
    const double result = vm->math_dispatch(op, a, b);
    // Rust returns before a VM allocation or raise.
    lua_pushnumber(state, result);
    return 1;
}
// P7.3 string cost is measured by number_parse_byte_cost.
static constexpr uint64_t NATIVE_BYTES_PER_STEP = 1;
static int number_native(lua_State* state) {
    const unsigned op = static_cast<unsigned>(lua_tointeger(state, lua_upvalueindex(1)));
    auto* vm = static_cast<ulua_vm*>(lua_tolightuserdata(state, lua_upvalueindex(2)));
    size_t length = 0;
    const char* input = op <= 1 ? lua_tolstring(state, 1, &length) : "";
    charge_steps(state, vm, NATIVE_CALL_STEPS + length / NATIVE_BYTES_PER_STEP);
    const double a = op <= 1 ? 0 : lua_tonumber(state, 1);
    const double b = lua_tonumber(state, 2);
    unsigned char output[2300];
    double number = 0;
    const int result = vm->number_dispatch(op, a, b, lua_gettop(state) >= 2,
        reinterpret_cast<const unsigned char*>(input), length, output, &number);
    if (result < 0) {
        vm->trap_kind = 4;
        luaL_error(state, "Invalid Number argument");
    }
    if (op <= 1) lua_pushnumber(state, number);
    else if (op >= 6) lua_pushboolean(state, number != 0);
    else {
        // Rust returns the closed format_steps formula in the numeric result slot.
        // Charge it before allocating the VM result string; no Rust frame is live.
        charge_steps(state, vm, static_cast<uint64_t>(number));
        lua_pushlstring(state, reinterpret_cast<const char*>(output), static_cast<size_t>(result));
    }
    return 1;
}
// The VM buffer bounds Rust's temporary result and is charged before conversion.
// Unicode default casing expands UTF-8 by at most three times (e.g. U+0390).
static int case_native(lua_State* state) {
    const unsigned op = static_cast<unsigned>(lua_tointeger(state, lua_upvalueindex(1)));
    auto* vm = static_cast<ulua_vm*>(lua_tolightuserdata(state, lua_upvalueindex(2)));
    size_t length = 0;
    const char* input = op <= 11 ? lua_tolstring(state, 1, &length) : "";
    charge_steps(state, vm, NATIVE_CALL_STEPS + length / NATIVE_BYTES_PER_STEP);
    if (length > (static_cast<size_t>(INT32_MAX) - 4) / 3) {
        record_stop(vm, SHIM_MEMORY);
        luaL_error(state, "String result exceeds memory limit");
    }
    unsigned char small[4];
    const size_t capacity = length * 3 + 4;
    size_t reserved = 0;
    if (op <= 11) {
        // Private operations 15/16 query Rust's bound without conversion or allocation.
        double unused_reservation = 0;
        const int reservation = vm->number_dispatch(op + 5, static_cast<double>(length), 0, 0,
            reinterpret_cast<const unsigned char*>(input), 0, small, &unused_reservation);
        if (reservation < 0) {
            record_stop(vm, SHIM_MEMORY);
            luaL_error(state, "String reservation exceeds memory limit");
        }
        reserved = static_cast<size_t>(reservation);
    }
    // The output buffer contains no physical reservation bytes.
    auto* output = op <= 11 ? static_cast<unsigned char*>(lua_newbuffer(state, capacity)) : small;
    if (reserved > vm->budget || vm->used > vm->budget - reserved) {
        record_stop(vm, SHIM_MEMORY);
        luaL_error(state, "String reservation exceeds memory limit");
    }
    // Reserve budget without allocation, allocation steps, or a fault-injection attempt.
    // This budget check applies even when allocation step charging is disabled.
    vm->used += reserved;
    const double a = op <= 11 ? 0 : lua_tonumber(state, 1);
    double unused = 0;
    const int result = vm->number_dispatch(op, a, 0, 0,
        reinterpret_cast<const unsigned char*>(input), length, output, &unused);
    // Rust dispatch neither calls Lua nor raises. No longjmp can skip this release.
    // Release before any Lua allocation or error, including a failed conversion.
    vm->used -= reserved;
    if (result < 0) {
        vm->trap_kind = 4;
        luaL_error(state, "Invalid String argument");
    }
    lua_pushlstring(state, reinterpret_cast<const char*>(output), static_cast<size_t>(result));
    return 1;
}
#include "json.h"
static void push_math(lua_State* state, ulua_vm* vm) {
    lua_createtable(state, 0, 23);
    lua_pushinteger(state, 0);
    lua_pushlightuserdata(state, vm);
    lua_pushcclosure(state, math_native, "acos", 2);
    lua_setfield(state, -2, "acos");
    lua_pushinteger(state, 1);
    lua_pushlightuserdata(state, vm);
    lua_pushcclosure(state, math_native, "acosh", 2);
    lua_setfield(state, -2, "acosh");
    lua_pushinteger(state, 2);
    lua_pushlightuserdata(state, vm);
    lua_pushcclosure(state, math_native, "asin", 2);
    lua_setfield(state, -2, "asin");
    lua_pushinteger(state, 3);
    lua_pushlightuserdata(state, vm);
    lua_pushcclosure(state, math_native, "asinh", 2);
    lua_setfield(state, -2, "asinh");
    lua_pushinteger(state, 4);
    lua_pushlightuserdata(state, vm);
    lua_pushcclosure(state, math_native, "atan", 2);
    lua_setfield(state, -2, "atan");
    lua_pushinteger(state, 5);
    lua_pushlightuserdata(state, vm);
    lua_pushcclosure(state, math_native, "atanh", 2);
    lua_setfield(state, -2, "atanh");
    lua_pushinteger(state, 6);
    lua_pushlightuserdata(state, vm);
    lua_pushcclosure(state, math_native, "atan2", 2);
    lua_setfield(state, -2, "atan2");
    lua_pushinteger(state, 7);
    lua_pushlightuserdata(state, vm);
    lua_pushcclosure(state, math_native, "cbrt", 2);
    lua_setfield(state, -2, "cbrt");
    lua_pushinteger(state, 8);
    lua_pushlightuserdata(state, vm);
    lua_pushcclosure(state, math_native, "cos", 2);
    lua_setfield(state, -2, "cos");
    lua_pushinteger(state, 9);
    lua_pushlightuserdata(state, vm);
    lua_pushcclosure(state, math_native, "cosh", 2);
    lua_setfield(state, -2, "cosh");
    lua_pushinteger(state, 10);
    lua_pushlightuserdata(state, vm);
    lua_pushcclosure(state, math_native, "exp", 2);
    lua_setfield(state, -2, "exp");
    lua_pushinteger(state, 11);
    lua_pushlightuserdata(state, vm);
    lua_pushcclosure(state, math_native, "expm1", 2);
    lua_setfield(state, -2, "expm1");
    lua_pushinteger(state, 12);
    lua_pushlightuserdata(state, vm);
    lua_pushcclosure(state, math_native, "hypot", 2);
    lua_setfield(state, -2, "hypot");
    lua_pushinteger(state, 13);
    lua_pushlightuserdata(state, vm);
    lua_pushcclosure(state, math_native, "log", 2);
    lua_setfield(state, -2, "log");
    lua_pushinteger(state, 14);
    lua_pushlightuserdata(state, vm);
    lua_pushcclosure(state, math_native, "log1p", 2);
    lua_setfield(state, -2, "log1p");
    lua_pushinteger(state, 15);
    lua_pushlightuserdata(state, vm);
    lua_pushcclosure(state, math_native, "log10", 2);
    lua_setfield(state, -2, "log10");
    lua_pushinteger(state, 16);
    lua_pushlightuserdata(state, vm);
    lua_pushcclosure(state, math_native, "log2", 2);
    lua_setfield(state, -2, "log2");
    lua_pushinteger(state, 17);
    lua_pushlightuserdata(state, vm);
    lua_pushcclosure(state, math_native, "pow", 2);
    lua_setfield(state, -2, "pow");
    lua_pushinteger(state, 18);
    lua_pushlightuserdata(state, vm);
    lua_pushcclosure(state, math_native, "sin", 2);
    lua_setfield(state, -2, "sin");
    lua_pushinteger(state, 19);
    lua_pushlightuserdata(state, vm);
    lua_pushcclosure(state, math_native, "sinh", 2);
    lua_setfield(state, -2, "sinh");
    lua_pushinteger(state, 20);
    lua_pushlightuserdata(state, vm);
    lua_pushcclosure(state, math_native, "tan", 2);
    lua_setfield(state, -2, "tan");
    lua_pushinteger(state, 21);
    lua_pushlightuserdata(state, vm);
    lua_pushcclosure(state, math_native, "tanh", 2);
    lua_setfield(state, -2, "tanh");
    lua_pushlightuserdata(state, vm);
    lua_pushcclosure(state, math_random, "random", 1);
    lua_setfield(state, -2, "random");
    if (vm->number_dispatch) {
        static const char* names[] = {"parseInt", "parseFloat", "toFixed", "toExponential", "toPrecision", "toString", "isNaN", "isFinite", "isInteger", "isSafeInteger"};
        for (unsigned op = 0; op < 10; ++op) {
            lua_pushinteger(state, op);
            lua_pushlightuserdata(state, vm);
            lua_pushcclosure(state, number_native, names[op], 2);
            char name[64] = "number_";
            std::strcat(name, names[op]);
            lua_setfield(state, -2, name);
        }
    }
    static const char* cases[] = {"case_upper", "case_lower", "case_fromCodePoint", "case_fromCharCode"};
    for (unsigned i = 0; i < 4; ++i) {
        lua_pushinteger(state, 10 + i);
        lua_pushlightuserdata(state, vm);
        lua_pushcclosure(state, case_native, cases[i], 2);
        lua_setfield(state, -2, cases[i]);
    }
    lua_pushlightuserdata(state, vm);
    lua_pushcclosure(state, json_parse, "json_parse", 1);
    lua_setfield(state, -2, "json_parse");
    lua_setreadonly(state, -1, true);
}
#endif
