//! Opaque ordered tables. All storage belongs to the VM allocator; loops checkpoint every slot.
pub(super) const PRELUDE: &str = r#"type __ulua_Table = {index: {[any]: number}, keys: {any}, values: {any}, nan: number?, live: number, slots: number, active: number}
local __ulua_table_register: (any, boolean?) -> () = select(3, ...)
local __ulua_table_active: {[__ulua_Table]: boolean} = {}
__ulua_table_register(__ulua_table_active, true)
local function __ulua_table_begin_0(m: __ulua_Table): ()
    m.active += 1
    __ulua_table_active[m] = true
end
local function __ulua_table_end_0(m: __ulua_Table): ()
    m.active -= 1
    __ulua_interrupt()
    if m.active == 0 then __ulua_table_active[m] = nil end
end
-- Wide number keys use bytes only in the index; keys retain the script numbers.
local function __ulua_table_slot(m: __ulua_Table, k: any): number?
    if k ~= k then return m.nan end
    if typeof(k) == "number" and (k < -2147483648 or k >= 2147483648) then
        return m.index[string.pack("<d", k)]
    end
    return m.index[k]
end
local function __ulua_table_changed(m: __ulua_Table): ()
    if m.active > 0 then __ulua_trap(5) end
end
local function __ulua_table_compact(m: __ulua_Table): ()
    local keys: {any} = {}
    local values: {any} = {}
    local index: {[any]: number} = {}
    local n = 0
    local nan: number? = nil
    for i = 1, m.slots do
        __ulua_interrupt()
        local k = m.keys[i]
        if k ~= nil then
            n += 1
            keys[n] = k
            __ulua_interrupt()
            values[n] = m.values[i]
            __ulua_interrupt()
            if k ~= k then nan = n
            elseif typeof(k) == "number" and (k < -2147483648 or k >= 2147483648) then
                index[string.pack("<d", k)] = n
            else index[k] = n end
        end
    end
    __ulua_interrupt()
    m.keys = keys
    m.values = values
    __ulua_interrupt()
    m.index = index
    m.nan = nan
    __ulua_interrupt()
    m.slots = n
end
local function __ulua_table_set_0(m: __ulua_Table, k: any, v: any): __ulua_Table
    if k == 0 then k = 0 end
    local indexKey = k
    if typeof(k) == "number" and (k < -2147483648 or k >= 2147483648) then
        indexKey = string.pack("<d", k)
    end
    local slot = if k ~= k then m.nan else m.index[indexKey]
    local added = slot == nil
    if added then
        -- A trapped deletion can skip compaction. Keep slot keys inside the int range.
        if m.slots >= 2147483647 then __ulua_table_compact(m) end
        m.slots += 1
        slot = m.slots
        __ulua_interrupt()
        m.live += 1
        __ulua_interrupt()
        if k ~= k then m.nan = slot else m.index[indexKey] = slot end
        __ulua_interrupt()
        m.keys[slot] = k
    end
    __ulua_interrupt()
    m.values[slot :: number] = v
    if added then __ulua_table_changed(m) end
    return m
end
local function __ulua_table_add_0(m: __ulua_Table, k: any): __ulua_Table
    if k == 0 then k = 0 end
    if __ulua_table_slot(m, k) == nil then __ulua_table_set_0(m, k, k) end
    return m
end
local function __ulua_table_get_0(m: __ulua_Table, k: any): any
    if typeof(k) == "number" and (k < -2147483648 or k >= 2147483648) then
        k = string.pack("<d", k)
    end
    local slot = if k ~= k then m.nan else m.index[k]
    if slot == nil then return nil end
    return m.values[slot]
end
local function __ulua_table_getOr_0(m: __ulua_Table, k: any, fallback: any): any
    if typeof(k) == "number" and (k < -2147483648 or k >= 2147483648) then
        k = string.pack("<d", k)
    end
    local slot = if k ~= k then m.nan else m.index[k]
    if slot == nil then return fallback end
    return m.values[slot]
end
local function __ulua_table_has_0(m: __ulua_Table, k: any): boolean
    if typeof(k) == "number" and (k < -2147483648 or k >= 2147483648) then
        k = string.pack("<d", k)
    end
    return if k ~= k then m.nan ~= nil else m.index[k] ~= nil
end
local function __ulua_table_delete_0(m: __ulua_Table, k: any): boolean
    if typeof(k) == "number" and (k < -2147483648 or k >= 2147483648) then
        k = string.pack("<d", k)
    end
    local slot = if k ~= k then m.nan else m.index[k]
    if slot == nil then return false end
    if k ~= k then m.nan = nil else m.index[k] = nil end
    __ulua_interrupt()
    m.keys[slot] = nil
    __ulua_interrupt()
    m.values[slot] = nil
    m.live -= 1
    __ulua_table_changed(m)
    if m.live * 2 <= m.slots then __ulua_table_compact(m) end
    return true
end
local function __ulua_table_clear_0(m: __ulua_Table): ()
    local changed = m.live > 0
    __ulua_interrupt()
    m.index = {}
    m.keys = {}
    __ulua_interrupt()
    m.values = {}
    m.nan = nil
    __ulua_interrupt()
    m.live = 0
    m.slots = 0
    __ulua_interrupt()
    if changed then __ulua_table_changed(m) end
end
local function __ulua_table_new_0(source: __ulua_Table?): __ulua_Table
    local m = {} :: __ulua_Table
    m.index = {}
    __ulua_interrupt()
    m.keys = {}
    m.values = {}
    __ulua_interrupt()
    m.nan = nil
    m.live = 0
    __ulua_interrupt()
    m.slots = 0
    m.active = 0
    __ulua_interrupt()
    if source ~= nil then
        for i = 1, source.slots do
            __ulua_interrupt()
            local k = source.keys[i]
            if k ~= nil then __ulua_table_set_0(m, k, source.values[i]) end
        end
    end
    return m
end
local function __ulua_table_fromArray_0(source: {any}): __ulua_Table
    local m = __ulua_table_new_0(nil)
    for i = 1, #source do
        __ulua_interrupt()
        __ulua_table_add_0(m, source[i])
    end
    return m
end
local function __ulua_table_each_0(m: __ulua_Table, f: any): ()
    __ulua_table_begin_0(m)
    local ok, failure = pcall(function(): ()
        for i = 1, m.slots do
            __ulua_interrupt()
            local k = m.keys[i]
            if k ~= nil then f(m.values[i], k) end
        end
    end)
    __ulua_table_end_0(m)
    if not ok then error(failure, 0) end
end
"#;
