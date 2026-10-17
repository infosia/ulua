//! JSON calls keep the same static helper as the source grows.
pub const KEYS: &[&str] = &["stringify", "parse"];
pub fn source(key: &str, bytes: usize) -> String {
    let (prefix, unit) = if key == "stringify" {
        (
            "type R={s:string;xs:number[]};export function main():void{const r:R={s:'a',xs:[1,2,3]};",
            "JSON.stringify(r);",
        )
    } else {
        (
            "type R={s:string;xs:number[]};export function main():void{",
            "{const r:R=JSON.parse('{\"s\":\"a\",\"xs\":[1,2,3]}');}",
        )
    };
    let mut out = prefix.to_owned();
    while out.len() + unit.len() < bytes {
        out.push_str(unit);
    }
    out.push('}');
    out.extend(std::iter::repeat_n(' ', bytes - out.len()));
    out
}
