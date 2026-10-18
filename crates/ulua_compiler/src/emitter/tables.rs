//! Opaque ordered tables. All storage belongs to the VM allocator; loops checkpoint every slot.
use crate::{ir::Method, tables::TableMethod};
use std::collections::BTreeSet;

pub(super) fn prelude(methods: &BTreeSet<(Method, usize)>) -> String {
    let kinds: BTreeSet<_> = methods
        .iter()
        .filter_map(|(m, kind)| matches!(m, Method::Table(m) if m.keyed()).then_some(*kind))
        .collect();
    if !methods.iter().any(|(m, _)| matches!(m, Method::Table(_))) {
        return String::new();
    }
    let mut out = String::from(COMMON);
    for kind in kinds {
        out.push_str(&keyed(kind));
    }
    out.push_str(EACH);
    out
}

fn keyed(kind: usize) -> String {
    let number = kind == TableMethod::key_kind(crate::types::Type::Number);
    // Only number keys can contain NaN or -0. Other key types need neither normalization nor a NaN slot.
    // Wide keys use bytes only in the index; the key array retains the script numbers.
    let normalize = if number {
        "    if k == 0 then k = 0 end\n"
    } else {
        ""
    };
    let convert_lookup = if number {
        r#"    if k < -2147483648 or k >= 2147483648 then
        k = string.pack("<d", k)
    end
"#
    } else {
        ""
    };
    let convert_set = if number {
        r#"    if k < -2147483648 or k >= 2147483648 then
        indexKey = string.pack("<d", k)
    end
"#
    } else {
        ""
    };
    let compact_index = if number {
        r#"            if k ~= k then nan = n
            elseif k < -2147483648 or k >= 2147483648 then
                index[string.pack("<d", k)] = n
            else index[k] = n end"#
    } else {
        "            index[k] = n"
    };
    let set_slot = if number {
        "if k ~= k then m.nan else m.index[indexKey]"
    } else {
        "m.index[indexKey]"
    };
    let lookup_slot = if number {
        "if k ~= k then m.nan else m.index[k]"
    } else {
        "m.index[k]"
    };
    let store_slot = if number {
        "        if k ~= k then m.nan = slot else m.index[indexKey] = slot end"
    } else {
        "        m.index[indexKey] = slot"
    };
    let has_slot = if number {
        "if k ~= k then m.nan ~= nil else m.index[k] ~= nil"
    } else {
        "m.index[k] ~= nil"
    };
    let delete_slot = if number {
        "    if k ~= k then m.nan = nil else m.index[k] = nil end"
    } else {
        "    m.index[k] = nil"
    };
    format!(
        r#"local function __ulua_table_compact_{kind}(m: __ulua_Table): ()
    local keys: {{any}} = {{}}
    local values: {{any}} = {{}}
    local index: {{[any]: number}} = {{}}
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
{compact_index}
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
local function __ulua_table_set_{kind}(m: __ulua_Table, k: any, v: any, onlyAbsent: boolean?): __ulua_Table
{normalize}    local indexKey = k
{convert_set}    local slot = {set_slot}
    if slot ~= nil and onlyAbsent then return m end
    local added = slot == nil
    if added then
        -- A trapped deletion can skip compaction. Keep slot keys inside the int range.
        if m.slots >= 2147483647 then __ulua_table_compact_{kind}(m) end
        m.slots += 1
        slot = m.slots
        __ulua_interrupt()
        m.live += 1
        __ulua_interrupt()
{store_slot}
        __ulua_interrupt()
        m.keys[slot] = k
    end
    __ulua_interrupt()
    m.values[slot :: number] = v
    if added then __ulua_table_changed(m) end
    return m
end
local function __ulua_table_add_{kind}(m: __ulua_Table, k: any): __ulua_Table
{normalize}    return __ulua_table_set_{kind}(m, k, k, true)
end
local function __ulua_table_get_{kind}(m: __ulua_Table, k: any): any
{convert_lookup}    local slot = {lookup_slot}
    if slot == nil then return nil end
    return m.values[slot]
end
local function __ulua_table_getOr_{kind}(m: __ulua_Table, k: any, fallback: any): any
{convert_lookup}    local slot = {lookup_slot}
    if slot == nil then return fallback end
    return m.values[slot]
end
local function __ulua_table_has_{kind}(m: __ulua_Table, k: any): boolean
{convert_lookup}    return {has_slot}
end
local function __ulua_table_delete_{kind}(m: __ulua_Table, k: any): boolean
{convert_lookup}    local slot = {lookup_slot}
    if slot == nil then return false end
{delete_slot}
    __ulua_interrupt()
    m.keys[slot] = nil
    __ulua_interrupt()
    m.values[slot] = nil
    m.live -= 1
    __ulua_table_changed(m)
    if m.live * 2 <= m.slots then __ulua_table_compact_{kind}(m) end
    return true
end
local function __ulua_table_new_{kind}(source: __ulua_Table?): __ulua_Table
    local m = {{}} :: __ulua_Table
    m.index = {{}}
    __ulua_interrupt()
    m.keys = {{}}
    m.values = {{}}
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
            if k ~= nil then __ulua_table_set_{kind}(m, k, source.values[i]) end
        end
    end
    return m
end
local function __ulua_table_fromArray_{kind}(source: {{any}}): __ulua_Table
    local m = __ulua_table_new_{kind}(nil)
    for i = 1, #source do
        __ulua_interrupt()
        __ulua_table_add_{kind}(m, source[i])
    end
    return m
end
"#
    )
}

const COMMON: &str = r#"type __ulua_Table = {index: {[any]: number}, keys: {any}, values: {any}, nan: number?, live: number, slots: number, active: number}
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
local function __ulua_table_changed(m: __ulua_Table): ()
    if m.active > 0 then __ulua_trap(5) end
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
"#;
const EACH: &str = r#"local function __ulua_table_each_0(m: __ulua_Table, f: any): ()
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

#[cfg(test)]
mod tests {
    use crate::{Limits, compile};

    // Cost: one compiler call per key type; no VM or external process.
    fn helpers(key: &str, declaration: &str) -> String {
        let source = format!(
            "{declaration} function main(k:{key}):void{{const m=new Map<{key},number>();m.set(k,1);m.get(k);m.getOr(k,0);m.has(k);m.delete(k);const c=new Map<{key},number>(m);const s=new Set<{key}>([k]);s.add(k);s.has(k);s.delete(k);const t=new Set<{key}>(s);}}"
        );
        let code = compile(source.as_bytes(), Limits::default()).unwrap().luau;
        let kind = usize::from(key != "number");
        for method in [
            "new",
            "fromArray",
            "set",
            "get",
            "getOr",
            "has",
            "delete",
            "add",
        ] {
            let helper = format!("__ulua_table_{method}_{kind}");
            assert!(code.matches(&format!("{helper}(")).count() >= 2, "{helper}");
        }
        let start = code.find("type __ulua_Table =").unwrap();
        let end = code[start..]
            .find("\nlocal function __ulua_table_each_0")
            .unwrap();
        code[start..start + end].to_owned()
    }

    #[test]
    fn number_key_helpers() {
        let code = helpers("number", "");
        for method in ["compact", "set", "get", "getOr", "has", "delete"] {
            let helper = format!("local function __ulua_table_{method}_0");
            let body = code
                .split(&helper)
                .nth(1)
                .unwrap()
                .split("\nend")
                .next()
                .unwrap();
            assert!(
                body.contains("k < -2147483648 or k >= 2147483648"),
                "{helper}"
            );
            assert_eq!(body.matches("string.pack").count(), 1, "{helper}");
        }
        assert!(!code.contains("typeof"));
    }

    fn direct_key_helpers(key: &str, declaration: &str) {
        let code = helpers(key, declaration);
        assert!(!code.contains("k < -2147483648"));
        assert!(!code.contains("string.pack"));
        assert!(!code.contains("typeof"));
        assert!(!code.contains("k ~= k"));
        assert!(!code.contains("k == 0"));
    }

    #[test]
    fn string_key_helpers() {
        direct_key_helpers("string", "");
    }
    #[test]
    fn boolean_key_helpers() {
        direct_key_helpers("boolean", "");
    }
    #[test]
    fn record_key_helpers() {
        direct_key_helpers("Key", "type Key={x:number};");
    }
    #[test]
    fn class_key_helpers() {
        direct_key_helpers("Key", "class Key { x:number=1; }");
    }
    #[test]
    fn array_key_helpers() {
        direct_key_helpers("number[]", "");
    }

    #[test]
    fn wide_add_converts_once() {
        let code = helpers("number", "");
        let add = code
            .split("local function __ulua_table_add_0")
            .nth(1)
            .unwrap()
            .split("\nend")
            .next()
            .unwrap();
        // A new key must reach only the set conversion, without a second slot conversion.
        assert!(!add.contains("__ulua_table_slot"));
        assert!(!add.contains("string.pack"));
        assert!(add.contains("__ulua_table_set_0"));
        let set = code
            .split("local function __ulua_table_set_0")
            .nth(1)
            .unwrap()
            .split("\nend")
            .next()
            .unwrap();
        assert_eq!(set.matches("string.pack").count(), 1);
    }
}
