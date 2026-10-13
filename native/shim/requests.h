#ifndef ULUA_REQUESTS_H
#define ULUA_REQUESTS_H
// Included inside the protected shim. Records use the instance allocator.
struct ulua_request {
    uint64_t id;
    uint32_t type;
    int promise;
    int value;
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
static void begin_run(ulua_vm* vm) {
    vm->steps = 0;
    vm->flag = 0;
    vm->trap_kind = 0;
    vm->charging = true;
#ifdef ULUA_MEASUREMENTS
    vm->allocation_bytes = 0;
    vm->allocation_steps = 0;
    vm->interrupts = 0;
#endif
}
static void reset_jobs(lua_State* state, ulua_vm* vm) {
    if (vm->async_failed && vm->reset) {
        lua_getref(state, vm->reset);
        if (lua_pcall(state, 0, 0, 0) != LUA_OK) raise_call_error(state, vm);
        vm->async_failed = false;
    }
}
static void invalidate_result(ulua_vm* vm) {
    if (vm->result_ref) {
        lua_unref(vm->thread, vm->result_ref);
        vm->result_ref = 0;
    }
}
static void push_value(lua_State* state, const ulua_value* value) {
    if (value->type == ULUA_TYPE_NUMBER) lua_pushnumber(state, value->number);
    else if (value->type == ULUA_TYPE_BOOLEAN) lua_pushboolean(state, value->boolean);
    else if (value->type == ULUA_TYPE_STRING) lua_pushlstring(state, value->data ? value->data : "", value->length);
    else lua_pushnil(state);
}
static int start_request(lua_State* state, ulua_vm* vm, const ulua_internal_capability* cap,
    const ulua_value* arguments) {
    if (vm->open_requests >= vm->max_open_requests) {
        vm->flag = SHIM_REQUEST_LIMIT;
        luaL_error(state, "Request limit exceeded");
    }
    if (vm->next_request == UINT64_MAX) luaL_error(state, "Request ids exhausted");
    // Allocate the promise first. No record exists if that operation stops.
    lua_getref(state, vm->new_promise);
    lua_pushnil(state);
    if (lua_pcall(state, 1, 1, 0) != LUA_OK) raise_call_error(state, vm);
    const int promise = lua_ref(state, -1);
    ulua_request* request = static_cast<ulua_request*>(allocate(vm, nullptr, 0, sizeof(ulua_request)));
    if (!request) { lua_unref(state, promise); luaL_error(state, "not enough memory"); }
    std::memset(request, 0, sizeof(*request));
    request->id = ++vm->next_request;
    request->type = cap->desc.result_type;
    request->next = vm->requests;
    vm->requests = request;
    ++vm->open_requests;
    request->promise = promise;
    int code = cap->desc.start(cap->desc.userdata, arguments, cap->desc.parameter_count, request->id);
    if (code) {
        vm->flag = code;
        std::memcpy(vm->capability_name, cap->desc.name, cap->desc.name_length);
        vm->capability_name[cap->desc.name_length] = 0;
        luaL_error(state, "Host start stopped");
    }
    return 1;
}
static ulua_request* find_request(ulua_vm* vm, uint64_t id) {
    for (ulua_request* r = vm->requests; r; r = r->next) if (r->id == id) return r;
    return nullptr;
}
static void unlink_request(ulua_vm* vm, ulua_request* r) {
    ulua_request** cursor = &vm->requests;
    while (*cursor && *cursor != r) cursor = &(*cursor)->next;
    if (*cursor) *cursor = r->next;
    --vm->open_requests;
}
static void release_request(ulua_vm* vm, ulua_request* r) {
    if (r->promise) lua_unref(vm->thread, r->promise);
    if (r->value) lua_unref(vm->thread, r->value);
    allocate(vm, r, sizeof(*r), 0);
}
static void discard_records(ulua_vm* vm) {
    // lua_close frees all references. Only native record allocations need a separate release.
    while (vm->requests) {
        ulua_request* r = vm->requests;
        vm->requests = r->next;
        allocate(vm, r, sizeof(*r), 0);
    }
    while (vm->tasks) {
        ulua_task* t = vm->tasks;
        vm->tasks = t->next;
        allocate(vm, t, sizeof(*t), 0);
    }
}
struct Complete { ulua_vm* vm; ulua_request* request; const ulua_value* value; };
static int complete(lua_State* state) {
    Complete* args = static_cast<Complete*>(lua_tolightuserdata(state, 1));
    if (args->value) {
        push_value(state, args->value);
        args->request->value = lua_ref(state, -1);
    }
    return 0;
}
extern "C" int ulua_internal_shim_complete(ulua_vm* vm, uint64_t id, const ulua_value* value, char* error) noexcept {
    try {
        invalidate_result(vm);
        ulua_request* r = find_request(vm, id);
        if (!r || r->recorded || (value && value->type != r->type)) return SHIM_ARGUMENT;
        Complete args = {vm, r, value};
        vm->charging = false;
        const int status = lua_cpcall(vm->thread, complete, &args);
        if (status != LUA_OK) {
            // A failed copy records nothing and does not alter the scheduler.
            message(error, "Completion copy failed");
            lua_settop(vm->thread, 0);
            return status == LUA_ERRMEM ? SHIM_MEMORY : SHIM_INTERNAL;
        }
        r->recorded = true;
        r->failed = !value;
        if (vm->recorded_tail) vm->recorded_tail->completion_next = r;
        else vm->recorded_head = r;
        vm->recorded_tail = r;
        ++vm->recorded_requests;
        lua_settop(vm->thread, 0);
        return SHIM_OK;
    } catch (...) { message(error, "Completion raised a C++ exception"); return SHIM_INTERNAL; }
}
struct Settle { ulua_vm* vm; ulua_request* request; };
static int settle(lua_State* state) {
    Settle* args = static_cast<Settle*>(lua_tolightuserdata(state, 1));
    ulua_vm* vm = args->vm;
    if (vm->steps >= vm->max_steps) { vm->flag = SHIM_STEPS; luaL_error(state, "Step limit exceeded"); }
    ++vm->steps;
    if (args->request->failed) { vm->flag = SHIM_REQUEST; luaL_error(state, "Host request failed"); }
    reset_jobs(state, vm);
    lua_getref(state, vm->fulfill);
    lua_getref(state, args->request->promise);
    if (args->request->value) lua_getref(state, args->request->value);
    else lua_pushnil(state);
    if (lua_pcall(state, 2, 0, 0) != LUA_OK) raise_call_error(state, vm);
    lua_getref(state, vm->drain);
    if (lua_pcall(state, 0, 0, 0) != LUA_OK) raise_call_error(state, vm);
    return 0;
}
extern "C" int ulua_internal_shim_pump(ulua_vm* vm, uint32_t maximum, uint32_t* settled, char* error) noexcept {
    *settled = 0;
    try {
        invalidate_result(vm);
        while (vm->recorded_head && (!maximum || *settled < maximum)) {
            ulua_request* r = vm->recorded_head;
            vm->recorded_head = r->completion_next;
            if (!vm->recorded_head) vm->recorded_tail = nullptr;
            --vm->recorded_requests;
            unlink_request(vm, r);
            begin_run(vm);
            Settle args = {vm, r};
            int code = finish(vm, vm->thread, lua_cpcall(vm->thread, settle, &args), error);
            lua_settop(vm->thread, 0);
            release_request(vm, r);
            ++*settled;
            if (code) return code;
        }
        return SHIM_OK;
    } catch (...) { message(error, "Pump raised a C++ exception"); return SHIM_INTERNAL; }
}
extern "C" void ulua_internal_shim_counts(const ulua_vm* vm, uint32_t* open, uint32_t* recorded) noexcept {
    *open = vm->open_requests;
    *recorded = vm->recorded_requests;
}
struct TaskResult { ulua_vm* vm; ulua_task* task; ulua_value* result; bool pending; };
static int task_result(lua_State* state) {
    TaskResult* args = static_cast<TaskResult*>(lua_tolightuserdata(state, 1));
    lua_getref(state, args->task->reference);
    if (args->task->promise) {
        lua_rawgetfield(state, -1, "state");
        int status = lua_tointeger(state, -1);
        lua_pop(state, 1);
        if (!status) { args->pending = true; return 0; }
        if (status == 2) {
            lua_rawgetfield(state, -1, "reason");
            raise_call_error(state, args->vm);
        }
        lua_rawgetfield(state, -1, "value");
    }
    ulua_value value = {};
    value.type = args->task->type;
    if (value.type == ULUA_TYPE_NUMBER) value.number = lua_tonumber(state, -1);
    else if (value.type == ULUA_TYPE_BOOLEAN) value.boolean = lua_toboolean(state, -1);
    else if (value.type == ULUA_TYPE_STRING) {
        value.data = lua_tolstring(state, -1, &value.length);
        args->vm->result_ref = lua_ref(state, -1);
    }
    *args->result = value;
    return 0;
}
extern "C" int ulua_internal_shim_task_result(ulua_vm* vm, uint64_t id, ulua_value* result, char* error) noexcept {
    try {
        invalidate_result(vm);
        ulua_task** cursor = &vm->tasks;
        while (*cursor && (*cursor)->id != id) cursor = &(*cursor)->next;
        if (!*cursor) return SHIM_ARGUMENT;
        ulua_task* t = *cursor;
        TaskResult args = {vm, t, result, false};
        vm->charging = false;
        int status = lua_cpcall(vm->thread, task_result, &args);
        // Pending queries are observations, not failed runs. Preserve scheduler state.
        if (status == LUA_OK && args.pending) { lua_settop(vm->thread, 0); return SHIM_TASK_PENDING; }
        const bool needs_reset = vm->async_failed;
        int code = finish(vm, vm->thread, status, error);
        // A successful observation cannot clear the scheduler reset after an earlier stop.
        if (status == LUA_OK) vm->async_failed = needs_reset;
        lua_settop(vm->thread, 0);
        *cursor = t->next;
        lua_unref(vm->thread, t->reference);
        allocate(vm, t, sizeof(*t), 0);
        return code;
    } catch (...) { message(error, "Task query raised a C++ exception"); return SHIM_INTERNAL; }
}
#endif
