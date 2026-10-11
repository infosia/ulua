//! Analyzer fixtures use the production compiler at the source ceiling.
use std::path::Path;
use ulua_compiler::{Limits, SOURCE_CEILING};

pub(crate) const FORMS: &[&str] = &["and", "or", "conditional", "if", "while", "for", "compound"];

pub(crate) fn source(form: &str) -> Result<String, String> {
    if form == "for" {
        return Ok(for_source());
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
        "compound" => ("export function main():void {let x=0;", "x+=1;", "}"),
        _ => return Err(format!("Unknown stress form: {form}")),
    };
    let count = (SOURCE_CEILING as usize - prefix.len() - suffix.len()) / unit.len();
    let mut source = format!("{prefix}{}{suffix}", unit.repeat(count));
    source.push_str(&" ".repeat(SOURCE_CEILING as usize - source.len()));
    Ok(source)
}

fn for_source() -> String {
    // Keep the P1 sequential-loop budget. Repeat 64 loop forms with dense bodies.
    let count = 64;
    let prefix = "export function main():void {let b=true;let x=0;";
    let open = "for(;b;){";
    let close = "break;}";
    let unit = "x=x+1;";
    let room = SOURCE_CEILING as usize - prefix.len() - 1 - count * (open.len() + close.len());
    let writes = room / unit.len();
    let mut source = prefix.to_owned();
    for i in 0..count {
        source.push_str(open);
        source.push_str(&unit.repeat(writes / count + usize::from(i < writes % count)));
        source.push_str(close);
    }
    source.push('}');
    source.push_str(&" ".repeat(SOURCE_CEILING as usize - source.len()));
    source
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
