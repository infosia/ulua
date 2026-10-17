#ifndef ULUA_STEPS_H
#define ULUA_STEPS_H
#include "allocator.h"
#include "lua.h"
#include "lualib.h"

// Consume positive steps as repeated interrupts do. The last available step can complete.
static inline void charge_steps(lua_State* state, ulua_vm* vm, uint64_t charge) {
    // A protected unwind must preserve the first recorded native stop.
    if (vm->flag || vm->trap_kind) luaL_error(state, "Run stopped");
    // A one-step call retains the interrupt's single comparison.
    if (vm->steps >= vm->max_steps || (charge > 1 && charge > vm->max_steps - vm->steps)) {
        vm->steps = vm->max_steps;
        record_stop(vm, SHIM_STEPS);
        luaL_error(state, "Step limit exceeded");
    }
    vm->steps += charge;
}
#endif
