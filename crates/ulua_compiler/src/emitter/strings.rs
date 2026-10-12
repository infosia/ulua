//! Byte helpers use KMP searches and checked UTF-8 boundaries.
use crate::{ir::Method, strings::StringMethod as M};
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn prelude(methods: &BTreeSet<(Method, usize)>) -> (String, usize) {
    let mut selected = BTreeMap::new();
    for &(method, _) in methods {
        if let Method::String(m) = method {
            let (params, result, body) = member(m);
            selected.insert(
                super::methods::name(method, 0),
                format!(
                    "local function {}({params}): {result}\n{body}end\n",
                    super::methods::name(method, 0)
                ),
            );
        }
    }
    let mut utilities = BTreeMap::new();
    // Resolve helper dependencies from fixed compiler text, never from script text.
    loop {
        let mut added = false;
        for &(name, source) in UTILITIES {
            if !utilities.contains_key(name)
                && selected
                    .values()
                    .chain(utilities.values())
                    .any(|s: &String| {
                        // Check name boundaries without repeated full token scans.
                        s.match_indices(name).any(|(start, _)| {
                            let identifier = |b: &u8| b.is_ascii_alphanumeric() || *b == b'_';
                            let before = start.checked_sub(1).and_then(|i| s.as_bytes().get(i));
                            let after = s.as_bytes().get(start + name.len());
                            !before.is_some_and(identifier) && !after.is_some_and(identifier)
                        })
                    })
            {
                utilities.insert(name, source.to_owned());
                added = true;
            }
        }
        if !added {
            break;
        }
    }
    // The utility table has dependency order. Members depend only on utilities.
    let mut out = String::new();
    for &(name, _) in UTILITIES {
        if let Some(source) = utilities.get(name) {
            out.push_str(source);
        }
    }
    for source in selected.values() {
        out.push_str(source);
    }
    (out, utilities.len())
}
const UTILITIES: &[(&str, &str)] = &[
    (
        "__ulua_si",
        r#"local function __ulua_si(x: number): number
    if x ~= x or x == 0 then return 0 end
    if x == __ulua_huge or x == -__ulua_huge then return x end
    return x - __ulua_fmod(x, 1)
end
"#,
    ),
    (
        "__ulua_sq",
        r#"local function __ulua_sq(s: string, i: number): boolean
    local b = string.byte(s, i + 1)
    return b == nil or b < 128 or b > 191
end
"#,
    ),
    (
        "__ulua_sb",
        r#"local function __ulua_sb(s: string, i: number): ()
    if not __ulua_sq(s, i) then __ulua_trap(3) end
end
"#,
    ),
    (
        "__ulua_sn",
        r#"local function __ulua_sn(s: string, i: number): number
    local b = string.byte(s, i + 1) :: number
    if b < 128 then return i + 1 end
    if b < 224 then return i + 2 end
    if b < 240 then return i + 3 end
    return i + 4
end
"#,
    ),
    (
        "__ulua_sc",
        r#"local function __ulua_sc(x: number, n: number): number
    return math.clamp(__ulua_si(x), 0, n)
end
"#,
    ),
    (
        "__ulua_sp",
        r#"local function __ulua_sp(x: number, n: number): number
    local i = __ulua_si(x)
    if i < 0 then i += n end
    return math.clamp(i, 0, n)
end
"#,
    ),
    (
        "__ulua_su",
        r#"local function __ulua_su(s: string, first: number, last: number?): string
    local finish = if last == nil then #s else last
    if first > finish then return "" end
    if first == 1 and finish == #s then return s end
    -- Fixed chunks bound work when Luau reuses an interned string.
    if finish - first < 256 then return string.sub(s, first, finish) end
    local pieces: {string} = {}
    while first <= finish do
        pieces[#pieces + 1] = string.sub(s, first, math.min(first + 255, finish))
        first += 256
    end
    return __ulua_concat(pieces)
end
"#,
    ),
    (
        "__ulua_se",
        r#"local function __ulua_se(s: string, needle: string, a: number): boolean
    for i = 1, #needle do
        if string.byte(s, a + i) ~= string.byte(needle, i) then return false end
    end
    return true
end
"#,
    ),
    (
        "__ulua_ss",
        r#"local function __ulua_ss(s: string, a: number, b: number): string
    __ulua_sb(s, a)
    __ulua_sb(s, b)
    return __ulua_su(s, a + 1, b)
end
"#,
    ),
    (
        "__ulua_sf",
        r#"local function __ulua_sf(s: string, needle: string, p: number, backward: boolean): number
    __ulua_sb(s, p)
    local m = #needle
    if m == 0 then return p end
    if m > #s or (not backward and m > #s - p) then return -1 end
    local direction = if backward then -1 else 1
    local origin = if backward then m else 1
    -- KMP prefix lengths use the same byte order as the search.
    local prefix: {number} = {0}
    local matched = 0
    for i = 2, m do
        local b = string.byte(needle, origin + (i - 1) * direction)
        while matched > 0 and b ~= string.byte(needle, origin + matched * direction) do
            matched = prefix[matched]
        end
        if b == string.byte(needle, origin + matched * direction) then matched += 1 end
        prefix[i] = matched
    end
    matched = 0
    local i = if backward then math.min(p, #s - m) + m - 1 else p
    while i >= 0 and i < #s do
        local b = string.byte(s, i + 1)
        while matched > 0 and b ~= string.byte(needle, origin + matched * direction) do
            matched = prefix[matched]
        end
        if b == string.byte(needle, origin + matched * direction) then matched += 1 end
        if matched == m then
            local a = if backward then i else i - m + 1
            if __ulua_sq(s, a) and __ulua_sq(s, a + m) then return a end
            matched = prefix[matched]
        end
        i += direction
    end
    return -1
end
"#,
    ),
    (
        "__ulua_sr",
        r#"local function __ulua_sr(s: string, n: number): string
    local result = ""
    local block = s
    while n > 0 do
        if __ulua_fmod(n, 2) == 1 then result ..= block end
        n = math.floor(n / 2)
        if n > 0 then block ..= block end
    end
    return result
end
"#,
    ),
    (
        "__ulua_sw",
        r#"local function __ulua_sw(s: string, i: number): boolean
    local cp = utf8.codepoint(s, i + 1)
    return (cp >= 9 and cp <= 13) or cp == 32 or cp == 160 or cp == 5760
        or (cp >= 8192 and cp <= 8202) or cp == 8232 or cp == 8233
        or cp == 8239 or cp == 8287 or cp == 12288 or cp == 65279
end
"#,
    ),
    (
        "__ulua_st",
        r#"local function __ulua_st(s: string, left: boolean, right: boolean): string
    local a = 0
    local b = #s
    if left then
        while a < b and __ulua_sw(s, a) do a = __ulua_sn(s, a) end
    end
    if right then
        while b > a do
            local i = b - 1
            while (string.byte(s, i + 1) :: number) >= 128 and (string.byte(s, i + 1) :: number) <= 191 do i -= 1 end
            if not __ulua_sw(s, i) then break end
            b = i
        end
    end
    return __ulua_su(s, a + 1, b)
end
"#,
    ),
    (
        "__ulua_sd",
        r#"local function __ulua_sd(s: string, len: number, pad: string?, left: boolean): string
    local n = math.clamp(__ulua_si(len), 0, 9007199254740991)
    if n <= #s then return s end
    local p = if pad == nil then " " else pad
    if #p == 0 then return s end
    local count = n - #s
    local full = math.floor(count / #p)
    local rest = __ulua_fmod(count, #p)
    __ulua_sb(p, rest)
    local fill = __ulua_sr(p, full) .. __ulua_su(p, 1, rest)
    return if left then fill .. s else s .. fill
end
"#,
    ),
    (
        "__ulua_sx",
        r#"local function __ulua_sx(s: string, a: number, b: number, replacement: string): string
    local pieces: {string} = {}
    local i = 1
    local start = 1
    while i <= #replacement do
        if string.byte(replacement, i) == 36 and i < #replacement then
            local c = string.sub(replacement, i + 1, i + 1)
            local value: string? = nil
            if c == "$" then value = "$"
            elseif c == "&" then value = __ulua_su(s, a + 1, b)
            elseif c == "`" then value = __ulua_su(s, 1, a)
            elseif c == "'" then value = __ulua_su(s, b + 1) end
            if value ~= nil then
                table.insert(pieces, __ulua_su(replacement, start, i - 1))
                table.insert(pieces, value)
                i += 2
                start = i
            else i += 1 end
        else i += 1 end
    end
    table.insert(pieces, __ulua_su(replacement, start))
    return __ulua_concat(pieces)
end
"#,
    ),
    (
        "__ulua_sv",
        r#"local function __ulua_sv(s: string, pattern: string, replacement: string, all: boolean): string
    local pieces: {string} = {}
    local cursor = 0
    local search = 0
    while search <= #s do
        local a = __ulua_sf(s, pattern, search, false)
        if a < 0 then break end
        local b = a + #pattern
        table.insert(pieces, __ulua_su(s, cursor + 1, a))
        table.insert(pieces, __ulua_sx(s, a, b, replacement))
        cursor = b
        if not all then break end
        if #pattern == 0 then
            if a == #s then break end
            search = __ulua_sn(s, a)
        else search = b end
    end
    table.insert(pieces, __ulua_su(s, cursor + 1))
    return __ulua_concat(pieces)
end
"#,
    ),
];
fn member(m: M) -> (&'static str, &'static str, &'static str) {
    match m {
        M::Slice => (
            "s: string, start: number?, finish: number?",
            "string",
            r#"    local a = if start == nil then 0 else __ulua_sp(start, #s)
    local b = if finish == nil then #s else __ulua_sp(finish, #s)
    __ulua_sb(s, b)
    return __ulua_ss(s, a, math.max(a, b))
"#,
        ),
        M::Substring => (
            "s: string, start: number, finish: number?",
            "string",
            r#"    local a = __ulua_sc(start, #s)
    local b = if finish == nil then #s else __ulua_sc(finish, #s)
    return __ulua_ss(s, math.min(a, b), math.max(a, b))
"#,
        ),
        M::Substr => (
            "s: string, start: number, count: number?",
            "string",
            r#"    local a = __ulua_sp(start, #s)
    local n = if count == nil then #s - a else __ulua_sc(count, #s - a)
    return __ulua_ss(s, a, a + n)
"#,
        ),
        M::IndexOf => (
            "s: string, needle: string, from: number?",
            "number",
            "    return __ulua_sf(s, needle, if from == nil then 0 else __ulua_sc(from, #s), false)\n",
        ),
        M::LastIndexOf => (
            "s: string, needle: string, position: number?",
            "number",
            r#"    local p = if position == nil or position ~= position then #s else __ulua_sc(position, #s)
    return __ulua_sf(s, needle, p, true)
"#,
        ),
        M::Includes => (
            "s: string, needle: string, from: number?",
            "boolean",
            "    return __ulua_sf(s, needle, if from == nil then 0 else __ulua_sc(from, #s), false) >= 0\n",
        ),
        M::StartsWith => (
            "s: string, needle: string, position: number?",
            "boolean",
            r#"    local p = if position == nil then 0 else __ulua_sc(position, #s)
    __ulua_sb(s, p)
    local b = p + #needle
    if b > #s or not __ulua_sq(s, b) then return false end
    return __ulua_se(s, needle, p)
"#,
        ),
        M::EndsWith => (
            "s: string, needle: string, position: number?",
            "boolean",
            r#"    local p = if position == nil then #s else __ulua_sc(position, #s)
    __ulua_sb(s, p)
    local a = p - #needle
    if a < 0 or not __ulua_sq(s, a) then return false end
    return __ulua_se(s, needle, a)
"#,
        ),
        M::CharAt | M::At => (
            "s: string, position: number",
            "string",
            if m == M::At {
                r#"    local i = __ulua_si(position)
    if i < 0 then i += #s end
    if i < 0 or i >= #s then __ulua_trap(1) end
    __ulua_sb(s, i)
    return string.sub(s, i + 1, __ulua_sn(s, i))
"#
            } else {
                r#"    local i = __ulua_si(position)
    if i < 0 or i >= #s then return "" end
    __ulua_sb(s, i)
    return string.sub(s, i + 1, __ulua_sn(s, i))
"#
            },
        ),
        M::CharCodeAt | M::CodePointAt => (
            "s: string, position: number",
            "number",
            if m == M::CharCodeAt {
                r#"    local i = __ulua_si(position)
    if i < 0 or i >= #s then __ulua_trap(1) end
    __ulua_sb(s, i)
    return string.byte(s, i + 1) :: number
"#
            } else {
                r#"    local i = __ulua_si(position)
    if i < 0 or i >= #s then __ulua_trap(1) end
    __ulua_sb(s, i)
    return (utf8.codepoint(s, i + 1)) :: number
"#
            },
        ),
        M::Split => (
            "s: string, separator: string, limit: number?",
            "{string}",
            r#"    local n = 4294967295
    if limit ~= nil then
        local v = __ulua_si(limit)
        n = if v == __ulua_huge or v == -__ulua_huge then 0 else v % 4294967296
    end
    local pieces: {string} = {}
    if n == 0 then return pieces end
    if #separator == 0 then
        local i = 0
        while i < #s and #pieces < n do
            local b = __ulua_sn(s, i)
            table.insert(pieces, string.sub(s, i + 1, b))
            i = b
        end
        return pieces
    end
    local i = 0
    while #pieces < n do
        local a = __ulua_sf(s, separator, i, false)
        if a < 0 then break end
        table.insert(pieces, __ulua_su(s, i + 1, a))
        i = a + #separator
    end
    if #pieces < n then table.insert(pieces, __ulua_su(s, i + 1)) end
    return pieces
"#,
        ),
        M::Trim => (
            "s: string",
            "string",
            "    return __ulua_st(s, true, true)\n",
        ),
        M::TrimStart => (
            "s: string",
            "string",
            "    return __ulua_st(s, true, false)\n",
        ),
        M::TrimEnd => (
            "s: string",
            "string",
            "    return __ulua_st(s, false, true)\n",
        ),
        M::Repeat => (
            "s: string, count: number",
            "string",
            r#"    local n = __ulua_si(count)
    if n < 0 or n == __ulua_huge then __ulua_trap(4) end
    if #s == 0 then return s end
    return __ulua_sr(s, n)
"#,
        ),
        M::PadStart => (
            "s: string, len: number, pad: string?",
            "string",
            "    return __ulua_sd(s, len, pad, true)\n",
        ),
        M::PadEnd => (
            "s: string, len: number, pad: string?",
            "string",
            "    return __ulua_sd(s, len, pad, false)\n",
        ),
        M::Replace => (
            "s: string, pattern: string, replacement: string",
            "string",
            "    return __ulua_sv(s, pattern, replacement, false)\n",
        ),
        M::ReplaceAll => (
            "s: string, pattern: string, replacement: string",
            "string",
            "    return __ulua_sv(s, pattern, replacement, true)\n",
        ),
        M::Concat => (
            "s: string, other: string",
            "string",
            "    return s .. other\n",
        ),
        M::Next => (
            "s: string, i: number",
            "number",
            "    return __ulua_sn(s, i)\n",
        ),
    }
}
