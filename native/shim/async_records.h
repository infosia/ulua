#ifndef ULUA_ASYNC_RECORDS_H
#define ULUA_ASYNC_RECORDS_H
#include "ulua.h"
#include <cstdint>
// Native records use the instance allocator. Completion string bytes use host memory.
struct ulua_request {
    uint64_t id;
    uint32_t type;
    int promise;
    ulua_value value;
    bool recorded;
    bool failed;
    ulua_request* next;
    ulua_request* completion_next;
};
struct ulua_task {
    uint64_t id;
    uint32_t type;
    int reference;
    bool promise;
    ulua_task* next;
};
#endif
