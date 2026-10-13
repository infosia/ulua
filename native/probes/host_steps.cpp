#include "../shim/allocator.h"
#include "lua.h"
// Independent total VM charges after an operation, including its native calls.
extern "C" uint64_t ulua_internal_test_host_steps(const ulua_vm* vm) { return vm->steps; }

// Remove VM instruction charges to isolate the explicit native host charge.
extern "C" void ulua_internal_test_no_interrupt(ulua_vm* vm) { lua_callbacks(vm->state)->interrupt = nullptr; }
// Place the budget boundary after the host starts, then restore it for recovery checks.
extern "C" void ulua_internal_test_exhaust_steps(ulua_vm* vm) { vm->max_steps = vm->steps; }
extern "C" void ulua_internal_test_restore_steps(ulua_vm* vm, uint64_t steps) { vm->max_steps = steps; }
