// Cost: one native build and fourteen protected calls. No assertion uses elapsed time.
// The allocator counts accepted requests independently of the VM used count.
#include "ulua_math.h"
#include <cstdio>
#include <climits>
#include <initializer_list>

static ulua_vm vm;
static size_t physical;
static size_t before;
static size_t requested[8];
static size_t allocations;
static size_t reservation = 32768;
static unsigned conversions;
static bool observed;
static bool invalid_result;
static constexpr size_t length = 8192;

static void* tracked_allocate(void* userdata, void* pointer, size_t old_size, size_t new_size) {
    const size_t old = pointer ? old_size : 0;
    void* result = allocate(userdata, pointer, old_size, new_size);
    if (!new_size) physical -= old;
    else if (result) {
        physical = physical - old + new_size;
        if (allocations < 8) requested[allocations] = new_size;
        ++allocations;
    }
    return result;
}
static int dispatch(unsigned op, double, double, int, const unsigned char*, size_t n,
    unsigned char* output, double*) {
    if (op == 15 || op == 16) return static_cast<int>(reservation);
    ++conversions;
    observed = vm.used == physical + reservation && allocations == 1;
    if (invalid_result) return -1;
    std::memset(output, op == 10 ? 'B' : 'c', n);
    return static_cast<int>(n);
}
#define CHECK(condition) do { if (!(condition)) { \
    std::fprintf(stderr, "line %d: %s (used=%zu physical=%zu before=%zu allocations=%zu conversions=%u)\n", \
        __LINE__, #condition, vm.used, physical, before, allocations, conversions); return 1; } } while (0)

// Modes 0..5: success, reservation refusal, result refusal, negative result, exact budget, charging disabled.
// Mode 6 refuses the reservation with allocation step charging disabled.
static int run(unsigned op, int mode, size_t output_charge) {
    vm = {};
    vm.budget = SIZE_MAX;
    vm.max_steps = UINT64_MAX;
    vm.number_dispatch = dispatch;
    physical = 0;
    conversions = 0;
    observed = false;
    invalid_result = mode == 3;
    reservation = mode == 2 ? 4096 : 32768;
    lua_State* state = lua_newstate(tracked_allocate, &vm);
    CHECK(state);
    lua_gc(state, LUA_GCSTOP, 0);
    lua_checkstack(state, 128);
    lua_pushinteger(state, static_cast<int>(op));
    lua_pushlightuserdata(state, &vm);
    lua_pushcclosure(state, case_native, "case-budget", 2);
    static char input[length];
    std::memset(input, 'A', length);
    lua_pushlstring(state, input, length);
    before = vm.used;
    CHECK(before == physical);
    allocations = 0;
    vm.charging = mode != 5 && mode != 6;
    vm.steps = 0;
    vm.allocation_calls = 0;
    vm.allocation_bytes = 0;
    vm.allocation_steps = 0;
    fault_begin(&vm.fault);
    if (mode == 1 || mode == 6) vm.budget = before + output_charge + reservation - 1;
    if (mode == 2 || mode == 4) vm.budget = before + output_charge + reservation;
    const int status = lua_pcall(state, 1, 1, 0);
    // This equality detects both a leaked reservation and physical reservation bytes.
    CHECK(vm.used == physical);
    if (mode == 0 || mode == 4 || mode == 5) {
        CHECK(status == 0 && conversions == 1 && observed);
        CHECK(allocations == 2); // Only the output buffer and the result string allocate.
        CHECK(vm.used == before + requested[0] + requested[1]);
        CHECK(lua_objlen(state, -1) == length);
        CHECK(vm.fault.snapshot.allocations == 2);
        CHECK(vm.allocation_calls == 2);
        const uint64_t bytes = requested[0] + requested[1];
        const uint64_t steps = (requested[0] + ALLOCATION_STEP_BYTES - 1) / ALLOCATION_STEP_BYTES
            + (requested[1] + ALLOCATION_STEP_BYTES - 1) / ALLOCATION_STEP_BYTES;
        CHECK(vm.allocation_bytes == (vm.charging ? bytes : 0));
        CHECK(vm.allocation_steps == (vm.charging ? steps : 0));
        CHECK(vm.steps == NATIVE_CALL_STEPS + length / NATIVE_BYTES_PER_STEP + (vm.charging ? steps : 0));
        if (mode == 0) output_charge = requested[0];
    } else {
        CHECK(status != 0);
        CHECK(mode == 1 || mode == 6 ? conversions == 0 : conversions == 1 && observed);
        CHECK(mode == 3 ? vm.trap_kind == 4 : vm.flag == SHIM_MEMORY);
    }
    std::printf("op=%u mode=%d before=%zu used=%zu physical=%zu output=%zu result=%zu\n",
        op, mode, before, vm.used, physical, requested[0], allocations > 1 ? requested[1] : 0);
    fault_end(&vm.fault);
    lua_close(state);
    CHECK(vm.used == 0 && physical == 0);
    return mode == 0 ? static_cast<int>(output_charge) : 0;
}
int main() {
    for (unsigned op : {10u, 11u}) {
        const int output_charge = run(op, 0, 0);
        if (output_charge <= 1) return 1;
        for (int mode = 1; mode <= 6; ++mode) {
            if (run(op, mode, static_cast<size_t>(output_charge))) return 1;
        }
    }
    return 0;
}
