// Reach luaD_pcall -> restore_stack_limit with a step budget exhausted after overflow.
#include "../shim/allocator.h"
#include "lua.h"
#include "ldo.h"
#include "lstate.h"
#include <cstring>

struct Fixture {
    ulua_vm vm;
    int overflow_size;
    int boundary_shrinks;
    uint64_t boundary_steps;
};
static void* tracked_allocate(void* userdata, void* pointer, size_t old_size, size_t new_size) {
    Fixture* fixture = static_cast<Fixture*>(userdata);
    if (pointer && new_size && new_size < old_size
        && fixture->vm.charging && fixture->vm.steps == fixture->vm.max_steps)
        ++fixture->boundary_shrinks;
    return allocate(&fixture->vm, pointer, old_size, new_size);
}
static void step_boundary(lua_State* state) {
    Fixture* fixture = static_cast<Fixture*>(lua_callbacks(state)->userdata);
    fixture->overflow_size = state->size_ci;
    fixture->boundary_steps = fixture->vm.steps;
    fixture->vm.max_steps = fixture->vm.steps;
    fixture->vm.charging = true;
}
static void overflow(lua_State* state, void*) {
    // Grow once past the normal call-depth limit, then raise the VM stack overflow.
    luaD_growCI(state);
}
static int prepare(lua_State* state) {
    luaD_reallocCI(state, LUAI_MAXCALLS);
    return 0;
}
extern "C" int ulua_internal_probe_stack_shrink() noexcept {
    Fixture fixture = {};
    fixture.vm.budget = SIZE_MAX;
    lua_State* state = lua_newstate(tracked_allocate, &fixture);
    if (!state) return 1;
    if (lua_cpcall(state, prepare, nullptr) != LUA_OK) { lua_close(state); return 2; }
    lua_callbacks(state)->userdata = &fixture;
    lua_callbacks(state)->debugprotectederror = step_boundary;
    int status = luaD_pcall(state, overflow, nullptr, 0, 0);
    bool valid = status == LUA_ERRRUN && fixture.overflow_size > LUAI_MAXCALLS
        && state->size_ci == LUAI_MAXCALLS && fixture.boundary_shrinks == 1
        && fixture.vm.steps == fixture.boundary_steps && fixture.vm.flag == 0;
    fixture.vm.charging = false;
    lua_close(state);
    return valid ? 0 : 3;
}
