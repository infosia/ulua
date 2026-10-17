//! Emit only the fixed Math helpers that the checked module uses.
use crate::math::native;
use std::collections::BTreeSet;

pub(super) fn prelude(members: &BTreeSet<&str>) -> (String, Vec<String>) {
    let mut text = String::new();
    let mut names = vec![];
    if members.iter().any(|n| {
        native(n).is_some() || n.starts_with("__ulua_number_") || n.starts_with("__ulua_case_")
    }) {
        text.push_str("local __ulua_math_native: any = select(6, ...)\n");
        names.push("__ulua_math_native".into());
    }
    if members.contains("__ulua_math_clz32") || members.contains("__ulua_math_imul") {
        text.push_str(UINT32);
        names.push("__ulua_math_uint32".into());
    }
    for &name in members {
        if matches!(name, "__ulua_floor" | "__ulua_ceil") {
            continue;
        }
        names.push(name.into());
        if let Some(member) = name.strip_prefix("__ulua_case_") {
            let arg = if matches!(member, "upper" | "lower") {
                "string"
            } else {
                "number"
            };
            text.push_str(&format!(
                "local {name}: ({arg}) -> string = __ulua_math_native.case_{member}\n"
            ));
        } else if let Some(member) = name.strip_prefix("__ulua_number_") {
            let signature = match member {
                "parseInt" => "(string, number) -> number",
                "parseFloat" => "(string) -> number",
                "toFixed" | "toExponential" => "(number, number?) -> string",
                "toPrecision" | "toString" => "(number, number) -> string",
                _ => "(number) -> boolean",
            };
            text.push_str(&format!(
                "local {name}: {signature} = __ulua_math_native.number_{member}\n"
            ));
        } else if let Some(member) = native(name) {
            let arity = crate::math::member(member).map_or(1, |(_, arity)| arity);
            let params = vec!["number"; arity].join(", ");
            text.push_str(&format!(
                "local {name}: ({params}) -> number = __ulua_math_native.{member}\n"
            ));
        } else {
            text.push_str(match name {
                "__ulua_math_abs" => "local __ulua_math_abs: typeof(math.abs) = math.abs\n",
                "__ulua_math_sqrt" => "local __ulua_math_sqrt: typeof(math.sqrt) = math.sqrt\n",
                "__ulua_math_round" => ROUND,
                "__ulua_math_sign" => SIGN,
                "__ulua_math_trunc" => TRUNC,
                "__ulua_math_max" => MAX,
                "__ulua_math_min" => MIN,
                "__ulua_math_clz32" => CLZ32,
                "__ulua_math_imul" => IMUL,
                "__ulua_math_fround" => FROUND,
                _ => "",
            });
        }
    }
    (text, names)
}
const UINT32: &str = r#"local function __ulua_math_uint32(x: number): number
    if x ~= x or x == __ulua_huge or x == -__ulua_huge or x == 0 then return 0 end
    local n = if x >= 0 then __ulua_floor(x) else __ulua_ceil(x)
    local r = __ulua_fmod(n, 4294967296)
    return if r < 0 then r + 4294967296 else r
end
"#;
const ROUND: &str = r#"local function __ulua_math_round(x: number): number
    local r = __ulua_floor(x)
    if x - r >= 0.5 then r += 1 end
    if r == 0 and (x < 0 or 1 / x == -__ulua_huge) then return -0 end
    return r
end
"#;
const SIGN: &str = r#"local function __ulua_math_sign(x: number): number
    if x == 0 or x ~= x then return x end
    return if x < 0 then -1 else 1
end
"#;
const TRUNC: &str = r#"local function __ulua_math_trunc(x: number): number
    return if x >= 0 then __ulua_floor(x) else __ulua_ceil(x)
end
"#;
const MAX: &str = r#"local function __ulua_math_max(a: number, b: number): number
    if a ~= a or b ~= b then return 0 / 0 end
    if a == 0 and b == 0 then return if 1 / a == __ulua_huge or 1 / b == __ulua_huge then 0 else -0 end
    return if a > b then a else b
end
"#;
const MIN: &str = r#"local function __ulua_math_min(a: number, b: number): number
    if a ~= a or b ~= b then return 0 / 0 end
    if a == 0 and b == 0 then return if 1 / a == -__ulua_huge or 1 / b == -__ulua_huge then -0 else 0 end
    return if a < b then a else b
end
"#;
const CLZ32: &str = r#"local function __ulua_math_clz32(x: number): number
    return bit32.countlz(__ulua_math_uint32(x))
end
"#;
const IMUL: &str = r#"local function __ulua_math_imul(x: number, y: number): number
    local a = __ulua_math_uint32(x)
    local b = __ulua_math_uint32(y)
    local al = a % 65536
    local bl = b % 65536
    local ah = __ulua_floor(a / 65536)
    local bh = __ulua_floor(b / 65536)
    local r = (al * bl + ((ah * bl + al * bh) % 65536) * 65536) % 4294967296
    return if r >= 2147483648 then r - 4294967296 else r
end
"#;
const FROUND: &str = r#"local function __ulua_math_fround(x: number): number
    local result: number = (string.unpack("<f", string.pack("<f", x))) :: number
    return result
end
"#;
