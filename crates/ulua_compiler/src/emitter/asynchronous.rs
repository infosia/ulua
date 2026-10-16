//! Native promise jobs. Only the private channel can suspend an activation.
pub(super) const LOCALS: usize = 30;
pub(super) const PRELUDE: &str = r#"type __ulua_Promise<T> = {
    state: number, value: T, reason: __ulua_Error?, handled: boolean,
    fulfilled: {(T) -> ()}, rejected: {(__ulua_Error) -> ()}
}
type __ulua_Rejection = {reason: __ulua_Error?, handled: boolean}
local __ulua_stop: (unknown) -> () = select(4, ...)
local __ulua_async_register: (any, any, any, any, any, any, any) -> () = select(5, ...)
local __ulua_promises: {[any]: boolean} = setmetatable({}, {__mode = "k"})
local __ulua_jobs: {() -> ()} = {}
local __ulua_head: number = 1
local __ulua_tail: number = 0
local __ulua_rejections: {__ulua_Rejection} = {}
local function __ulua_enqueue(job: () -> ()): ()
    __ulua_tail += 1
    __ulua_jobs[__ulua_tail] = job
end
local function __ulua_fulfill<T>(p: __ulua_Promise<T>, value: T): ()
    if p.state ~= 0 then return end
    p.state = 1
    p.value = value
    for _, f in p.fulfilled do __ulua_enqueue(function(): () f(value) end) end
    table.clear(p.fulfilled)
    table.clear(p.rejected)
end
local function __ulua_reject<T>(p: __ulua_Promise<T>, reason: __ulua_Error): ()
    if p.state ~= 0 then return end
    p.state = 2
    p.reason = reason
    __ulua_rejections[#__ulua_rejections + 1] = p
    for _, f in p.rejected do __ulua_enqueue(function(): () f(reason) end) end
    table.clear(p.fulfilled)
    table.clear(p.rejected)
end
local function __ulua_then<T>(p: __ulua_Promise<T>, f: (T) -> (), j: (__ulua_Error) -> ()): ()
    p.handled = true
    if p.state == 1 then __ulua_enqueue(function(): () f(p.value) end)
    elseif p.state == 2 then __ulua_enqueue(function(): () j(p.reason :: __ulua_Error) end)
    else table.insert(p.fulfilled, f); table.insert(p.rejected, j) end
end
local function __ulua_await<T>(p: __ulua_Promise<T>): T
    local ok: boolean, value: unknown = coroutine.yield(p)
    if not ok then error(value, 0) end
    -- The scheduler pairs this request with a fulfillment of the same promise.
    return value :: T
end
-- The outer promise keeps a nullable T intact during generic inference.
local function __ulua_new<T>(witness: __ulua_Promise<T>?): __ulua_Promise<T>
    local p: __ulua_Promise<T> = {state = 0, value = nil :: any, handled = false, fulfilled = {}, rejected = {}}
    __ulua_promises[p] = true
    return p
end
local function __ulua_new_void(): __ulua_Promise<nil>
    local p: __ulua_Promise<nil> = {state = 0, value = nil, handled = false, fulfilled = {}, rejected = {}}
    __ulua_promises[p] = true
    return p
end
local function __ulua_resolve<T>(result: __ulua_Promise<T>, value: any): ()
    if value == result then
        __ulua_reject(result, __ulua_error("TypeError", "Chaining cycle detected for promise #<Promise>"))
    -- A weak-table read keyed by a number outside the int range is undefined in the
    -- array fast path of the pinned VM; only a table can be a promise.
    elseif typeof(value) == "table" and __ulua_promises[value] then
        local adopted: __ulua_Promise<T> = value
        -- PromiseResolveThenableJob precedes the adoption reaction job.
        __ulua_enqueue(function(): ()
            __ulua_then(adopted,
                function(v: T): () __ulua_fulfill(result, v) end,
                function(e: __ulua_Error): () __ulua_reject(result, e) end)
        end)
    else __ulua_fulfill(result, value) end
end
local function __ulua_reaction<T>(result: __ulua_Promise<T>, body: () -> unknown): ()
    local ok, value = pcall(body)
    if ok then __ulua_resolve(result, value)
    else
        __ulua_stop(value)
        __ulua_reject(result, value :: __ulua_Error)
    end
end
local function __ulua_on_then1<T, U>(p: __ulua_Promise<T>, f: (T) -> unknown, result: __ulua_Promise<U>): __ulua_Promise<U>
    __ulua_then(p,
        function(v: T): () __ulua_reaction(result, function(): unknown return f(v) end) end,
        function(e: __ulua_Error): () __ulua_reject(result, e) end)
    return result
end
local function __ulua_on_then0<T, U>(p: __ulua_Promise<T>, f: () -> unknown, result: __ulua_Promise<U>): __ulua_Promise<U>
    return __ulua_on_then1(p, function(_: T): unknown return f() end, result)
end
local function __ulua_on_catch1<T>(p: __ulua_Promise<T>, f: (unknown) -> unknown, result: __ulua_Promise<T>): __ulua_Promise<T>
    __ulua_then(p,
        function(v: T): () __ulua_fulfill(result, v) end,
        function(e: __ulua_Error): () __ulua_reaction(result, function(): unknown return f(e) end) end)
    return result
end
local function __ulua_on_catch0<T>(p: __ulua_Promise<T>, f: () -> unknown, result: __ulua_Promise<T>): __ulua_Promise<T>
    return __ulua_on_catch1(p, function(_: unknown): unknown return f() end, result)
end
local function __ulua_on_finally<T>(p: __ulua_Promise<T>, f: () -> unknown, result: __ulua_Promise<T>): __ulua_Promise<T>
    local function complete(value: T, reason: __ulua_Error?): ()
        __ulua_reaction(result, function(): unknown
            local returned = f()
            local waited: __ulua_Promise<unknown>
            if typeof(returned) == "table" and __ulua_promises[returned] then waited = returned :: any
            else
                waited = __ulua_new(nil :: __ulua_Promise<unknown>?)
                __ulua_fulfill(waited, returned)
            end
            local bridge = __ulua_new(nil :: __ulua_Promise<T>?)
            -- ThenFinally returns PromiseResolve(callback()).then(valueThunk).
            __ulua_then(waited,
                function(_: unknown): ()
                    if reason ~= nil then __ulua_reject(bridge, reason)
                    else __ulua_fulfill(bridge, value) end
                end,
                function(e: __ulua_Error): () __ulua_reject(bridge, e) end)
            return bridge
        end)
    end
    __ulua_then(p,
        function(v: T): () complete(v, nil) end,
        function(e: __ulua_Error): () complete(nil :: any, e) end)
    return result
end
local function __ulua_on_then1v<T>(p: __ulua_Promise<T>, f: (T) -> (), result: __ulua_Promise<nil>): __ulua_Promise<nil>
    return __ulua_on_then1(p, function(v: T): unknown f(v); return nil end, result)
end
local function __ulua_on_then0v<T>(p: __ulua_Promise<T>, f: () -> (), result: __ulua_Promise<nil>): __ulua_Promise<nil>
    return __ulua_on_then0(p, function(): unknown f(); return nil end, result)
end
local function __ulua_on_catch1v(p: __ulua_Promise<nil>, f: (unknown) -> (), result: __ulua_Promise<nil>): __ulua_Promise<nil>
    return __ulua_on_catch1(p, function(e: unknown): unknown f(e); return nil end, result)
end
local function __ulua_on_catch0v(p: __ulua_Promise<nil>, f: () -> (), result: __ulua_Promise<nil>): __ulua_Promise<nil>
    return __ulua_on_catch0(p, function(): unknown f(); return nil end, result)
end
local function __ulua_on_finallyv<T>(p: __ulua_Promise<T>, f: () -> (), result: __ulua_Promise<T>): __ulua_Promise<T>
    return __ulua_on_finally(p, function(): unknown f(); return nil end, result)
end
local function __ulua_all<T>(values: {__ulua_Promise<T>}): __ulua_Promise<{T}>
    local result = __ulua_new(nil :: __ulua_Promise<{T}>?)
    local out: {T} = {}
    local remaining = #values
    if remaining == 0 then __ulua_fulfill(result, out) end
    for i, p in values do
        __ulua_then(p,
            function(v: T): ()
                out[i] = v
                remaining -= 1
                if remaining == 0 then __ulua_fulfill(result, out) end
            end,
            function(e: __ulua_Error): () __ulua_reject(result, e) end)
    end
    return result
end
local function __ulua_start<T>(result: __ulua_Promise<T>, body: () -> unknown): __ulua_Promise<T>
    local co: thread = coroutine.create(body)
    local resume: (boolean, unknown) -> ()
    resume = function(ok: boolean, value: unknown): ()
        local good, request = coroutine.resume(co, ok, value)
        if not good then
            -- This native check re-raises each stop before any promise operation.
            __ulua_stop(request)
            __ulua_reject(result, request)
        elseif coroutine.status(co) == "dead" then
            __ulua_resolve(result, request)
        else
            local awaited: __ulua_Promise<unknown> = request
            __ulua_then(awaited,
                function(v: unknown): () resume(true, v) end,
                function(e: __ulua_Error): () resume(false, e) end)
        end
    end
    resume(true, nil)
    return result
end
local function __ulua_reset(): ()
    table.clear(__ulua_jobs)
    table.clear(__ulua_rejections)
    __ulua_head = 1
    __ulua_tail = 0
end
local function __ulua_drain(): ()
    while __ulua_head <= __ulua_tail do
        local job = __ulua_jobs[__ulua_head]
        __ulua_jobs[__ulua_head] = nil
        __ulua_head += 1
        if __ulua_head > __ulua_tail then
            __ulua_head = 1
            __ulua_tail = 0
        elseif __ulua_head > 1024 then
            -- Copy in FIFO order. Destinations precede their sources, so overlaps preserve unread jobs.
            local remaining = __ulua_tail - __ulua_head + 1
            for i = 1, remaining do
                __ulua_jobs[i] = __ulua_jobs[__ulua_head + i - 1]
                __ulua_jobs[__ulua_head + i - 1] = nil
            end
            __ulua_head = 1
            __ulua_tail = remaining
        end
        -- Between jobs, head is at most 1024 and tail is at most live jobs + 1023.
        -- VM/src/ltable.cpp caps each table part at 2^26 slots. Queue keys remain below 2^31.
        job()
    end
    __ulua_head = 1
    __ulua_tail = 0
    local first: __ulua_Error? = nil
    for _, p in __ulua_rejections do
        if not p.handled and first == nil then first = p.reason end
    end
    table.clear(__ulua_rejections)
    if first ~= nil then error(first, 0) end
end
__ulua_async_register(__ulua_drain, __ulua_reset, __ulua_promises, __ulua_jobs, __ulua_rejections, __ulua_new, __ulua_fulfill)
"#;

// Keep helper temporaries outside the recursive expression frame in Debug builds.
impl super::Emitter<'_> {
    pub(super) fn promise_expression(
        &mut self,
        expr: &crate::ir::Expr,
        name: &str,
        arguments: &[crate::ir::Expr],
        dest: usize,
        depth: u32,
    ) -> Result<(), crate::Diagnostic> {
        use crate::ir::Type;
        self.capture(name);
        let mut slots = vec![];
        for arg in arguments {
            slots.push(self.expression(arg, depth)?);
        }
        let mut args: Vec<_> = slots.iter().map(|i| self.temp(*i)).collect();
        if name != "__ulua_all" {
            let value = self
                .module
                .types
                .promise_value(expr.ty)
                .ok_or_else(|| self.limit("internal", "Missing derived promise type"))?;
            let ty = if value == Type::Void {
                "nil".to_owned()
            } else {
                self.function_types.luau(&value)
            };
            if matches!(value, Type::Void | Type::Null) {
                self.capture("__ulua_new_void");
                args.push("__ulua_new_void()".to_owned());
            } else {
                self.capture("__ulua_new");
                args.push(format!("__ulua_new(nil :: __ulua_Promise<{ty}>?)"));
            }
        }
        self.write_slot(dest, format!("{name}({})", args.join(", ")));
        for slot in slots {
            self.release(slot);
        }
        Ok(())
    }
}
