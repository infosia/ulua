//! Selected, generic array helpers. Loops use the VM interrupt budget.
use crate::{Diagnostic, ir::Method as M};
use std::collections::BTreeSet;

pub(super) fn name(method: M, arity: usize) -> String {
    format!("__ulua_{}_{arity}", method.name())
}

const INTEGER: &str = r#"local function __ulua_integer(x: number): number
    if x ~= x or x == 0 then return 0 end
    if x == __ulua_huge or x == -__ulua_huge then return x end
    return x - __ulua_fmod(x, 1)
end
"#;
const POSITION: &str = r#"local function __ulua_position(x: number?, n: number, default: number): number
    if x == nil then return default end
    local i = __ulua_integer(x)
    if i < 0 then i += n end
    if i < 0 then return 0 end
    if i > n then return n end
    return i
end
"#;

fn positions(method: M) -> bool {
    matches!(
        method,
        M::IndexOf | M::Includes | M::Slice | M::Fill | M::Splice | M::CopyWithin
    )
}

pub(crate) fn prelude(methods: &BTreeSet<(M, usize)>) -> Result<(String, usize), Diagnostic> {
    let mut out = String::new();
    let position = methods.iter().any(|(m, _)| positions(*m));
    let integer = position
        || methods
            .iter()
            .any(|(m, _)| matches!(*m, M::At | M::LastIndexOf));
    if integer {
        out.push_str(INTEGER);
    }
    if position {
        out.push_str(POSITION);
    }
    let utilities = usize::from(integer) + usize::from(position);
    for &(method, arity) in methods {
        if !method.arities().contains(&arity) {
            return Err(invalid());
        }
        let (generic, parameters, result, body) = match method {
            M::IndexOf
            | M::LastIndexOf
            | M::Includes
            | M::Join
            | M::Slice
            | M::Concat
            | M::Fill
            | M::Reverse
            | M::Splice
            | M::Unshift
            | M::CopyWithin
            | M::At
            | M::Pop
            | M::Shift
            | M::Clear => {
                if arity != 0 {
                    return Err(invalid());
                }
                plain(method)?
            }
            M::ForEach
            | M::Map
            | M::Filter
            | M::Some
            | M::Every
            | M::FindIndex
            | M::FindLastIndex
            | M::FlatMap
            | M::Reduce
            | M::ReduceRight
            | M::Sort => callback(method, arity)?,
        };
        out.push_str(&format!(
            "local function {}{generic}({parameters}): {result}\n{body}end\n",
            name(method, arity)
        ));
    }
    Ok((out, methods.len() + utilities))
}

fn invalid() -> Diagnostic {
    Diagnostic::at(
        &[],
        0,
        "internal",
        "Invalid array helper kind or callback arity",
    )
}

type Helper = (String, String, String, String);
fn form(generic: &str, parameters: &str, result: &str, body: &str) -> Helper {
    (
        generic.into(),
        parameters.into(),
        result.into(),
        body.into(),
    )
}

fn plain(method: M) -> Result<Helper, Diagnostic> {
    let (parameters, result, body) = match method {
        M::IndexOf | M::Includes => {
            let equality = if method == M::Includes {
                "a[i + 1] == v or (a[i + 1] ~= a[i + 1] and v ~= v)"
            } else {
                "a[i + 1] == v"
            };
            let yes = if method == M::Includes { "true" } else { "i" };
            let no = if method == M::Includes { "false" } else { "-1" };
            return Ok(form(
                "<T>",
                "a: {T}, v: T, from: number?",
                if method == M::Includes {
                    "boolean"
                } else {
                    "number"
                },
                &format!(
                    r#"    local n = #a
    local i = __ulua_position(from, n, 0)
    while i < n do
        if {equality} then return {yes} end
        i += 1
    end
    return {no}
"#
                ),
            ));
        }
        M::LastIndexOf => (
            "a: {T}, v: T, from: number?",
            "number",
            r#"    local n = #a
    local i = if from == nil then n - 1 else __ulua_integer(from)
    if i < 0 then i += n end
    if i >= n then i = n - 1 end
    while i >= 0 do
        if a[i + 1] == v then return i end
        i -= 1
    end
    return -1
"#,
        ),
        M::Join => (
            "a: {T}, separator: string?",
            "string",
            r#"    local parts: {string} = {}
    for i = 1, #a do
        local v = a[i]
        if typeof(v) == "number" then
            parts[i] = __ulua_format(v)
        elseif typeof(v) == "boolean" then
            parts[i] = if v then "true" else "false"
        elseif typeof(v) == "string" then
            parts[i] = v
        end
    end
    return __ulua_concat(parts, if separator == nil then "," else separator)
"#,
        ),
        M::Slice => (
            "a: {T}, first: number?, last: number?",
            "{T}",
            r#"    local n = #a
    local start = __ulua_position(first, n, 0)
    local finish = __ulua_position(last, n, n)
    local out: {T} = {}
    for i = start + 1, finish do out[#out + 1] = a[i] end
    return out
"#,
        ),
        M::Concat => (
            "a: {T}, b: {T}",
            "{T}",
            r#"    local out: {T} = {}
    for i = 1, #a do out[#out + 1] = a[i] end
    for i = 1, #b do out[#out + 1] = b[i] end
    return out
"#,
        ),
        M::Fill => (
            "a: {T}, v: T, first: number?, last: number?",
            "{T}",
            r#"    local n = #a
    local start = __ulua_position(first, n, 0)
    local finish = __ulua_position(last, n, n)
    for i = start + 1, finish do a[i] = v end
    return a
"#,
        ),
        M::Reverse => (
            "a: {T}",
            "{T}",
            r#"    local n = #a
    local i = 1
    while i <= n / 2 do
        local v = a[i]
        a[i] = a[n + 1 - i]
        a[n + 1 - i] = v
        i += 1
    end
    return a
"#,
        ),
        M::Splice => (
            "a: {T}, first: number, count: number?",
            "{T}",
            r#"    local n = #a
    local start = __ulua_position(first, n, 0)
    local size = if count == nil then n - start else __ulua_integer(count)
    if size < 0 then size = 0 end
    if size > n - start then size = n - start end
    local out: {T} = {}
    for i = 1, size do out[i] = a[start + i] end
    for i = start + 1, n - size do a[i] = a[i + size] end
    for i = n, n - size + 1, -1 do a[i] = nil end
    return out
"#,
        ),
        M::Unshift => (
            "a: {T}, v: T",
            "number",
            r#"    local n = #a
    for i = n, 1, -1 do a[i + 1] = a[i] end
    a[1] = v
    return n + 1
"#,
        ),
        M::CopyWithin => (
            "a: {T}, target: number, first: number, last: number?",
            "{T}",
            r#"    local n = #a
    local to = __ulua_position(target, n, 0)
    local start = __ulua_position(first, n, 0)
    local finish = __ulua_position(last, n, n)
    local count = finish - start
    if count > n - to then count = n - to end
    if start < to and to < start + count then
        for i = count, 1, -1 do a[to + i] = a[start + i] end
    else
        for i = 1, count do a[to + i] = a[start + i] end
    end
    return a
"#,
        ),
        M::At => (
            "a: {T}, position: number",
            "T",
            r#"    local i = __ulua_integer(position)
    if i < 0 then i += #a end
    return __ulua_get(a, i)
"#,
        ),
        M::Pop => (
            "a: {T}",
            "T",
            r#"    local n = #a
    if n == 0 then __ulua_trap(1) end
    local v = a[n]
    a[n] = nil
    return v
"#,
        ),
        M::Shift => (
            "a: {T}",
            "T",
            r#"    local n = #a
    if n == 0 then __ulua_trap(1) end
    local v = a[1]
    for i = 1, n - 1 do a[i] = a[i + 1] end
    a[n] = nil
    return v
"#,
        ),
        M::Clear => (
            "a: {T}",
            "()",
            r#"    for i = #a, 1, -1 do a[i] = nil end
"#,
        ),
        M::ForEach
        | M::Map
        | M::Filter
        | M::Some
        | M::Every
        | M::FindIndex
        | M::FindLastIndex
        | M::FlatMap
        | M::Reduce
        | M::ReduceRight
        | M::Sort => return Err(invalid()),
    };
    Ok(form("<T>", parameters, result, body))
}

fn callback(method: M, arity: usize) -> Result<Helper, Diagnostic> {
    if method == M::Sort {
        return match arity {
            2 => Ok(form(
                "<T>",
                "a: {T}, compare: (T, T) -> number",
                "{T}",
                SORT,
            )),
            _ => Err(invalid()),
        };
    }
    if matches!(method, M::Reduce | M::ReduceRight) {
        let range = if method == M::Reduce {
            "1, n"
        } else {
            "n, 1, -1"
        };
        let (input, arguments) = match arity {
            2 => ("U, T", "accumulator, a[i]"),
            3 => ("U, T, number", "accumulator, a[i], i - 1"),
            _ => return Err(invalid()),
        };
        return Ok(form(
            "<T, U>",
            &format!("a: {{T}}, f: ({input}) -> U, initial: U"),
            "U",
            &format!(
                r#"    local n = #a
    local accumulator = initial
    for i = {range} do
        accumulator = f({arguments})
        if #a ~= n then __ulua_trap(2) end
    end
    return accumulator
"#
            ),
        ));
    }
    let (input, arguments) = match arity {
        1 => ("T", "v"),
        2 => ("T, number", "v, i - 1"),
        _ => return Err(invalid()),
    };
    let (generic, callback_result, result, initial, step, finish) = match method {
        M::ForEach => ("<T, U...>", "U...", "()", "", "", ""),
        M::Map => (
            "<T, U>",
            "U",
            "{U}",
            "    local out: {U} = {}\n",
            "        out[i] = value\n",
            "    return out\n",
        ),
        M::FlatMap => (
            "<T, U>",
            "{U}",
            "{U}",
            "    local out: {U} = {}\n",
            "        for j = 1, #value do out[#out + 1] = value[j] end\n",
            "    return out\n",
        ),
        M::Filter => (
            "<T>",
            "boolean",
            "{T}",
            "    local out: {T} = {}\n",
            "        if value then out[#out + 1] = v end\n",
            "    return out\n",
        ),
        M::Some => (
            "<T>",
            "boolean",
            "boolean",
            "",
            "        if value then return true end\n",
            "    return false\n",
        ),
        M::Every => (
            "<T>",
            "boolean",
            "boolean",
            "",
            "        if not value then return false end\n",
            "    return true\n",
        ),
        M::FindIndex | M::FindLastIndex => (
            "<T>",
            "boolean",
            "number",
            "",
            "        if value then return i - 1 end\n",
            "    return -1\n",
        ),
        M::IndexOf
        | M::LastIndexOf
        | M::Includes
        | M::Join
        | M::Slice
        | M::Concat
        | M::Fill
        | M::Reverse
        | M::Splice
        | M::Unshift
        | M::CopyWithin
        | M::At
        | M::Pop
        | M::Shift
        | M::Clear
        | M::Reduce
        | M::ReduceRight
        | M::Sort => return Err(invalid()),
    };
    let range = if method == M::FindLastIndex {
        "n, 1, -1"
    } else {
        "1, n"
    };
    let call = if method == M::ForEach {
        format!("        f({arguments})\n")
    } else {
        format!("        local value = f({arguments})\n")
    };
    Ok(form(
        generic,
        &format!("a: {{T}}, f: ({input}) -> {callback_result}"),
        result,
        &format!(
            r#"    local n = #a
{initial}    for i = {range} do
        local v = a[i]
{call}        if #a ~= n then __ulua_trap(2) end
{step}    end
{finish}"#
        ),
    ))
}

const SORT: &str = r#"    local n = #a
    local source: {T} = {}
    for i = 1, n do source[i] = a[i] end
    local width = 1
    while width < n do
        local out: {T} = {}
        local start = 1
        while start <= n do
            local middle = start + width
            if middle > n + 1 then middle = n + 1 end
            local finish = start + 2 * width
            if finish > n + 1 then finish = n + 1 end
            local left, right, dest = start, middle, start
            while left < middle and right < finish do
                local order = compare(source[left], source[right])
                if #a ~= n then __ulua_trap(2) end
                if order ~= order or order <= 0 then
                    out[dest] = source[left]
                    left += 1
                else
                    out[dest] = source[right]
                    right += 1
                end
                dest += 1
            end
            while left < middle do
                out[dest] = source[left]
                left += 1
                dest += 1
            end
            while right < finish do
                out[dest] = source[right]
                right += 1
                dest += 1
            end
            start = finish
        end
        source = out
        width *= 2
    end
    for i = 1, n do a[i] = source[i] end
    return a
"#;
