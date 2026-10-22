#ifndef ULUA_FAULTS_H
#define ULUA_FAULTS_H
// Only the feature build includes this header. Each test thread has an independent controller.
struct ulua_fault_snapshot {
    uint64_t allocations;
    uint64_t failures;
    size_t live_bytes;
    size_t host_bytes;
    size_t records;
};
struct ulua_fault_state {
    ulua_fault_snapshot snapshot;
    uint64_t fail_at;
    bool active;
};
static thread_local uint64_t fault_next = 0;
static thread_local ulua_fault_snapshot fault_last = {};
static void fault_begin(ulua_fault_state* fault) {
    fault->snapshot.allocations = 0;
    fault->snapshot.failures = 0;
    fault->fail_at = fault_next;
    fault_next = 0;
    fault->active = true;
    fault_last = fault->snapshot;
}
static bool fault_refuse(ulua_fault_state* fault) {
    if (!fault->active) return false;
    ++fault->snapshot.allocations;
    if (fault->snapshot.allocations != fault->fail_at) return false;
    ++fault->snapshot.failures;
    fault_last = fault->snapshot;
    return true;
}
static void fault_end(ulua_fault_state* fault) {
    fault->active = false;
    fault_last = fault->snapshot;
}
#endif
