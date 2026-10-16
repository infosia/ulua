//! Fixed library calls have constant work or work bounded by their output bytes.
use crate::INTERRUPT_STATEMENTS;

/// Bound module initialization across fixed helper declarations.
/// Function bodies use VM calls and loop back edges; each top-level closure creation is one statement.
/// Fixed preludes put each top-level executable statement on its own line without indentation.
pub(super) fn checkpoint(text: &str) -> String {
    if text.is_empty() {
        return String::new();
    }
    let mut out = String::new();
    let mut count = 0;
    for line in text.lines() {
        if line == "__ulua_interrupt()" {
            count = 0;
        } else if line.starts_with("local ") || line.starts_with("__ulua_") {
            if count == INTERRUPT_STATEMENTS {
                out.push_str("__ulua_interrupt()\n");
                count = 0;
            }
            count += 1;
            // Fixed top-level calls fire a VM interrupt, including the native async registration.
            if line.starts_with("__ulua_") && line.contains('(') {
                count = 0;
            }
        }
        out.push_str(line);
        out.push('\n');
    }
    // The following generated code starts with a fresh forward-path count.
    if count != 0 {
        out.push_str("__ulua_interrupt()\n");
    }
    out
}

pub(crate) const LIBRARIES: &str = r#"local __ulua_trap: (number) -> () = (...) :: (number) -> ()
local function __ulua_interrupt(): () end
local function __ulua_literal_set<T>(a: {T}, i: number, value: T): ()
    a[i] = value
end
local function __ulua_equal(a: unknown, b: unknown): boolean return a == b end
local function __ulua_unequal(a: unknown, b: unknown): boolean return a ~= b end
local function __ulua_concat(parts: {string}, separator: string?): string
    local nonempty: {string} = {}
    for i = 1, #parts do
        if i > 1 and separator ~= nil and #separator > 0 then
            nonempty[#nonempty + 1] = separator
        end
        local part = parts[i]
        if #part > 0 then nonempty[#nonempty + 1] = part end
    end
    -- Each retained entry contributes at least one output byte.
    return table.concat(nonempty)
end
local __ulua_print: typeof(print) = print
local __ulua_fmod: typeof(math.fmod) = math.fmod
local __ulua_floor: typeof(math.floor) = math.floor
local __ulua_ceil: typeof(math.ceil) = math.ceil
local __ulua_tostring: typeof(tostring) = tostring
local __ulua_gsub: typeof(string.gsub) = string.gsub
local __ulua_huge: typeof(math.huge) = math.huge
local function __ulua_format(__ulua_n: number): string
    if __ulua_n ~= __ulua_n then return "NaN" end
    if __ulua_n == __ulua_huge then return "Infinity" end
    if __ulua_n == -__ulua_huge then return "-Infinity" end
    if __ulua_n == 0 then return "0" end
    local __ulua_s: string = (__ulua_gsub(__ulua_tostring(__ulua_n), "e([+-])0*(%d)", "e%1%2")) :: string
    return __ulua_s
end
local function __ulua_get<T>(a: {T}, i: number, deferred: boolean?): T
    if not (i >= 0 and i < #a and __ulua_floor(i) == i) then
        if deferred then return nil :: any end
        __ulua_trap(1)
    end
    return a[i + 1]
end
__ulua_interrupt()
local function __ulua_receiver(value: unknown): ()
    if value == nil then __ulua_trap(1) end
end
local function __ulua_set<T>(a: {T}, i: number, v: T): ()
    if not (i >= 0 and i <= #a and __ulua_floor(i) == i) then __ulua_trap(1) end
    a[i + 1] = v
end
__ulua_interrupt()
"#;

pub(super) const EXCEPTIONS: &str = r#"local __ulua_register: (any) -> () = select(3, ...)
type __ulua_Error = {name: string, message: string}
local __ulua_errors: {[__ulua_Error]: string} = setmetatable({}, {__mode = "k"})
__ulua_register(__ulua_errors)
local function __ulua_error(kind: string, message: string): __ulua_Error
    local value: __ulua_Error = {name = kind, message = message}
    __ulua_errors[value] = kind
    return value
end
local function __ulua_instance(value: unknown, kind: string): boolean
    if typeof(value) ~= "table" then return false end
    local actual = __ulua_errors[value :: __ulua_Error]
    return actual ~= nil and (kind == "Error" or actual == kind)
end
type __ulua_completion_body = (unknown) -> (number, any, number)
local function __ulua_protect(body: __ulua_completion_body, caught: unknown): (number, any, number)
    local ok, tag, value, target = pcall(body, caught)
    if not ok then
        if not __ulua_instance(tag, "Error") then error(tag, 0) end
        return 2, tag, 0
    end
    return tag, value, target
end
local function __ulua_try(body: __ulua_completion_body, handler: __ulua_completion_body?, finalizer: __ulua_completion_body?): (number, any, number)
    local tag, value, target = __ulua_protect(body, nil)
    if tag == 2 and handler ~= nil then
        tag, value, target = __ulua_protect(handler, value)
    end
    if finalizer ~= nil then
        local finalTag, finalValue, finalTarget = __ulua_protect(finalizer, nil)
        if finalTag ~= 0 then tag, value, target = finalTag, finalValue, finalTarget end
    end
    if tag == 2 then error(value, 0) end
    return tag, value, target
end
"#;
