//! Per-static-type JSON serializers and target checks. No runtime type descriptors.
use super::*;
use std::collections::HashSet;

pub(super) fn prelude(module: &ir::Module, bound: usize) -> Result<String, Diagnostic> {
    if module.json.is_empty() {
        return Ok(String::new());
    }
    let mut out = String::from(COMMON);
    if module.json.iter().any(|(parse, _)| *parse) {
        out.push_str("__ulua_json.native = (select(6, ...) :: any).json_parse\n");
    }
    for (id, &(parse, root)) in module.json.iter().enumerate() {
        let mut nodes = vec![];
        let mut pending = vec![root];
        let mut seen = HashSet::new();
        // A repeated path requires an active-object set only if the graph has a cycle.
        while let Some(t) = pending.pop() {
            if !seen.insert(t) {
                continue;
            }
            nodes.push(t);
            pending.extend(children(module, t));
        }
        let cycle = !parse && cyclic(module, root);
        let ids: HashMap<_, _> = nodes.iter().enumerate().map(|(i, t)| (*t, i)).collect();
        let name = |t| format!("__ulua_json.f{id}_{}", ids.get(&t).copied().unwrap_or(0));
        for &t in &nodes {
            if out.len() > bound {
                return Err(Diagnostic::at(
                    &module.source,
                    0,
                    "limit:output",
                    "Generated source exceeds the output bound",
                ));
            }
            let f = name(t);
            let _ = writeln!(
                out,
                "{f} = function(v: any, d: number, o: {{string}}, seen: any): {}",
                if parse { "any" } else { "()" }
            );
            out.push_str("    __ulua_interrupt()\n");
            if parse {
                out.push_str("    if v == nil then error(__ulua_error(\"TypeError\", \"Missing JSON value\"), 0) end\n");
                if let Some(inner) = module.types.inner(t) {
                    let _ = writeln!(
                        out,
                        "    if v.k == 0 then return nil end\n    return {}(v, d, o, seen)\nend\n__ulua_interrupt()",
                        name(inner)
                    );
                    continue;
                }
                let kind = if t == Type::Number {
                    1
                } else if t == Type::String {
                    2
                } else if t == Type::Boolean {
                    3
                } else if module.types.element(t).is_some() {
                    4
                } else {
                    5
                };
                let _ = writeln!(
                    out,
                    "    if v.k ~= {kind} then error(__ulua_error(\"TypeError\", \"JSON type mismatch\"), 0) end"
                );
                if matches!(t, Type::Number | Type::String | Type::Boolean) {
                    out.push_str("    return v.v\n");
                } else if let Some(element) = module.types.element(t) {
                    let _ = writeln!(
                        out,
                        "    local r: {{any}} = {{}}\n    __ulua_interrupt()\n    for i = 1, #v.v do\n        __ulua_interrupt()\n        r[i] = {}(v.v[i], d + 1, o, seen)\n    end\n    return r",
                        name(element)
                    );
                } else if let Some(fields) = module.types.declaration_fields(t) {
                    out.push_str("    local r: any = {}\n");
                    for &(key, child) in fields {
                        let _ = writeln!(
                            out,
                            "    __ulua_interrupt()\n    r.__ulua_k{key} = {}(v.v[{}], d + 1, o, seen)",
                            name(child),
                            quote(&module.field_names[&key])
                        );
                    }
                    out.push_str("    return r\n");
                }
            } else {
                if let Some(inner) = module.types.inner(t) {
                    let _ = writeln!(
                        out,
                        "    if v == nil then o[#o + 1] = \"null\"; return end\n    {}(v, d, o, seen)\nend\n__ulua_interrupt()",
                        name(inner)
                    );
                    continue;
                }
                if t == Type::Null {
                    out.push_str("    o[#o + 1] = \"null\"\n");
                } else if t == Type::Number {
                    out.push_str("    if v ~= v or v == __ulua_huge or v == -__ulua_huge then __ulua_trap(6) end\n    o[#o + 1] = __ulua_format(v)\n");
                } else if t == Type::Boolean {
                    out.push_str("    o[#o + 1] = if v then \"true\" else \"false\"\n");
                } else if t == Type::String {
                    out.push_str("    __ulua_json.quote(v, o)\n");
                } else {
                    out.push_str("    if d >= 128 then __ulua_trap(7) end\n");
                    if cycle {
                        out.push_str("    if seen[v] then error(__ulua_error(\"TypeError\", \"JSON cycle\"), 0) end\n    seen[v] = true\n");
                    }
                    if let Some(element) = module.types.element(t) {
                        let _ = writeln!(
                            out,
                            "    o[#o + 1] = \"[\"\n    __ulua_interrupt()\n    for i = 1, #v do\n        __ulua_interrupt()\n        if i > 1 then o[#o + 1] = \",\" end\n        {}(v[i], d + 1, o, seen)\n    end\n    o[#o + 1] = \"]\"",
                            name(element)
                        );
                    } else if let Some(fields) = module.types.declaration_fields(t) {
                        out.push_str("    o[#o + 1] = \"{\"\n");
                        for (i, &(key, child)) in fields.iter().enumerate() {
                            let prefix = format!(
                                "{}\"{}\":",
                                if i == 0 { "" } else { "," },
                                module.field_names[&key]
                            );
                            let _ = writeln!(
                                out,
                                "    __ulua_interrupt()\n    o[#o + 1] = {}\n    {}(v.__ulua_k{key}, d + 1, o, seen)",
                                quote(&prefix),
                                name(child)
                            );
                        }
                        out.push_str("    o[#o + 1] = \"}\"\n");
                    }
                    if cycle {
                        out.push_str("    seen[v] = nil\n");
                    }
                }
            }
            out.push_str("end\n__ulua_interrupt()\n");
        }
        let _ = writeln!(out, "__ulua_json.c{id} = function(v: any): any");
        if parse {
            let _ = writeln!(
                out,
                "    local status, value = __ulua_json.native(v)\n    if status ~= 0 then error(__ulua_error(if status == 1 then \"SyntaxError\" else \"TypeError\", \"Invalid JSON\"), 0) end\n    return {}(value, 0, {{}}, nil)",
                name(root)
            );
        } else {
            let _ = writeln!(
                out,
                "    local o: {{string}} = {{}}\n    {}(v, 0, o, {})\n    return table.concat(o)",
                name(root),
                if cycle { "{}" } else { "nil" }
            );
        }
        out.push_str("end\n__ulua_interrupt()\n");
    }
    Ok(out)
}
fn cyclic(module: &ir::Module, root: Type) -> bool {
    let mut pending = vec![(root, false)];
    let mut colors = HashMap::new();
    while let Some((ty, exit)) = pending.pop() {
        if exit {
            colors.insert(ty, 2);
            continue;
        }
        match colors.get(&ty) {
            Some(1) => return true,
            Some(2) => continue,
            _ => {}
        }
        colors.insert(ty, 1);
        pending.push((ty, true));
        pending.extend(children(module, ty).into_iter().map(|t| (t, false)));
    }
    false
}
fn children(module: &ir::Module, ty: Type) -> Vec<Type> {
    if let Some(t) = module.types.inner(ty).or_else(|| module.types.element(ty)) {
        vec![t]
    } else {
        module
            .types
            .declaration_fields(ty)
            .unwrap_or(&[])
            .iter()
            .map(|f| f.1)
            .collect()
    }
}
const COMMON: &str = r#"local __ulua_json: any = {}
__ulua_json.quote = function(s: string, o: {string}): ()
    o[#o + 1] = '"'
    local start = 1
    __ulua_interrupt()
    for i = 1, #s do
        __ulua_interrupt()
        local b = string.byte(s, i)
        if b < 32 or b == 34 or b == 92 then
            if i > start then o[#o + 1] = string.sub(s, start, i - 1) end
            if b == 34 then o[#o + 1] = '\\\"'
            elseif b == 92 then o[#o + 1] = '\\\\'
            elseif b == 8 then o[#o + 1] = '\\b'
            elseif b == 9 then o[#o + 1] = '\\t'
            elseif b == 10 then o[#o + 1] = '\\n'
            elseif b == 12 then o[#o + 1] = '\\f'
            elseif b == 13 then o[#o + 1] = '\\r'
            else o[#o + 1] = string.format('\\u%04x', b) end
            start = i + 1
        end
    end
    if start <= #s then o[#o + 1] = string.sub(s, start) end
    o[#o + 1] = '"'
end
"#;
