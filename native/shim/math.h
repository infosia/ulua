#ifndef ULUA_MATH_H
#define ULUA_MATH_H
#include "allocator.h"
#include "lua.h"
#include "lualib.h"
#include "steps.h"
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
    lua_setreadonly(state, -1, true);
}
#endif
