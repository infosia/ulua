//! Analyzer fixtures use the production compiler at the source ceiling.
use std::path::Path;
use ulua_compiler::{Limits, SOURCE_CEILING};

#[path = "../../ulua/src/s5_inputs.rs"]
mod s5;
pub(crate) const FORMS: &[&str] = &[
    "and",
    "or",
    "conditional",
    "if",
    "while",
    "nested-bindings",
    "nested-writes",
    "function-bindings",
    "module-bindings",
    "short-functions",
    "default-calls",
];

pub(crate) fn source(form: &str) -> Result<String, String> {
    if matches!(
        form,
        "nested-bindings"
            | "nested-writes"
            | "function-bindings"
            | "module-bindings"
            | "short-functions"
            | "default-calls"
    ) {
        return Ok(s5::source(form, SOURCE_CEILING as usize));
    }
    let (prefix, unit, suffix) = match form {
        "and" => ("export function main():void {let b=true;", "b=b&&b;", "}"),
        "or" => ("export function main():void {let b=true;", "b=b||b;", "}"),
        "conditional" => (
            "function g():number{return 1;} export function main():void {let b=true;let x=0;",
            "x=b?x:g();",
            "}",
        ),
        "if" => (
            "export function main():void {let b=true;let x=0;",
            "if(b){x=x+1;}",
            "}",
        ),
        "while" => (
            "export function main():void {let b=true;let x=0;",
            "while(b){x=x+1;break;}",
            "}",
        ),
        _ => return Err(format!("Unknown stress form: {form}")),
    };
    let count = (SOURCE_CEILING as usize - prefix.len() - suffix.len()) / unit.len();
    let mut source = format!("{prefix}{}{suffix}", unit.repeat(count));
    source.push_str(&" ".repeat(SOURCE_CEILING as usize - source.len()));
    Ok(source)
}

pub(crate) fn emit(directory: &Path) -> Result<(), String> {
    std::fs::create_dir_all(directory).map_err(|e| e.to_string())?;
    for form in FORMS {
        let source = source(form)?;
        let compiled = ulua_compiler::compile(source.as_bytes(), Limits::default())
            .map_err(|d| format!("{form}: {} at {}: {}", d.code, d.offset, d.message))?;
        std::fs::write(directory.join(format!("{form}.luau")), compiled.luau)
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}
