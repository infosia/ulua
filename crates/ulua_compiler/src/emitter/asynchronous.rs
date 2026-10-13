//! Native promise jobs. Only the private channel can suspend an activation.
pub(super) const LOCALS: usize = 15;
pub(super) const PRELUDE: &str = r#"type __ulua_Promise<T> = {
    state: number, value: T, reason: __ulua_Error?, handled: boolean,
    fulfilled: {(T) -> ()}, rejected: {(__ulua_Error) -> ()}
}
type __ulua_Rejection = {reason: __ulua_Error?, handled: boolean}
local __ulua_stop: (unknown) -> () = select(4, ...)
local __ulua_async_register: (any, any, any, any) -> () = select(5, ...)
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
            if request == result then
                __ulua_reject(result, __ulua_error("TypeError", "Chaining cycle detected for promise #<Promise>"))
            elseif __ulua_promises[request] then
                local adopted: __ulua_Promise<T> = request
                -- PromiseResolveThenableJob precedes the adoption reaction job.
                __ulua_enqueue(function(): ()
                    __ulua_then(adopted,
                        function(v: T): () __ulua_fulfill(result, v) end,
                        function(e: __ulua_Error): () __ulua_reject(result, e) end)
                end)
            else __ulua_fulfill(result, request) end
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
__ulua_async_register(__ulua_drain, __ulua_reset, __ulua_promises, __ulua_jobs)
"#;
