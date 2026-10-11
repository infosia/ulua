//! Strict Luau emission from typed forms.
use crate::ir;
use std::fmt::Write;

pub(crate) fn quote(value: &str) -> String {
    let mut out = String::from("\"");
    for byte in value.bytes() {
        if (0x20..=0x7e).contains(&byte) && byte != b'"' && byte != b'\\' {
            out.push(char::from(byte));
        } else {
            // Formatting into a String cannot fail.
            let _ = write!(out, "\\{byte:03}");
        }
    }
    out.push('"');
    out
}
pub(crate) fn emit(module: &ir::Module) -> String {
    let mut out = String::from("--!strict\nlocal __ulua_0 = print\n");
    for (index, function) in module.functions.iter().enumerate() {
        let _ = writeln!(out, "local function __ulua_{}(): ()", index + 1);
        for statement in &function.body {
            match statement {
                ir::Statement::Print(value) => {
                    let _ = writeln!(out, "    __ulua_0({})", quote(value));
                }
            }
        }
        out.push_str("end\n");
    }
    out.push_str("return {\n");
    for (index, function) in module.functions.iter().enumerate() {
        let _ = writeln!(
            out,
            "    [{}] = __ulua_{},",
            quote(&function.name),
            index + 1
        );
    }
    out.push_str("}\n");
    out
}
