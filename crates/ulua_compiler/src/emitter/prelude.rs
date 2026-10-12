//! Fixed library calls have constant work or work bounded by their output bytes.
pub(crate) const LIBRARIES: &str = r#"local __ulua_trap: (number) -> () = (...) :: (number) -> ()
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
local __ulua_print: typeof(print) = (print) :: typeof(print)
local __ulua_fmod: typeof(math.fmod) = (math.fmod) :: typeof(math.fmod)
local __ulua_tostring: typeof(tostring) = (tostring) :: typeof(tostring)
local __ulua_gsub: typeof(string.gsub) = (string.gsub) :: typeof(string.gsub)
local __ulua_huge: typeof(math.huge) = (math.huge) :: typeof(math.huge)
local function __ulua_format(__ulua_n: number): string
    if __ulua_n ~= __ulua_n then return "NaN" end
    if __ulua_n == __ulua_huge then return "Infinity" end
    if __ulua_n == -__ulua_huge then return "-Infinity" end
    if __ulua_n == 0 then return "0" end
    local __ulua_s: string = (__ulua_gsub(__ulua_tostring(__ulua_n), "e([+-])0*(%d)", "e%1%2")) :: string
    return __ulua_s
end
local function __ulua_get<T>(a: {T}, i: number, deferred: boolean?): T
    if not (i >= 0 and i < #a and __ulua_fmod(i, 1) == 0) then
        if deferred then return nil :: any end
        __ulua_trap(1)
    end
    return a[i + 1]
end
local function __ulua_set<T>(a: {T}, i: number, v: T): ()
    if not (i >= 0 and i <= #a and __ulua_fmod(i, 1) == 0) then __ulua_trap(1) end
    a[i + 1] = v
end
"#;
