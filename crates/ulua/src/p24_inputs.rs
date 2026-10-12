//! String growth shapes repeat members and bounded concatenation chains.
pub const SHAPES: &[&str] = &[
    "string-members",
    "string-chains",
    "string-concatenation",
    "string-mixed-concatenation",
    "string-iteration",
];
pub fn source(shape: &str, bytes: usize) -> String {
    let prefix = "export function main():void{let s='abc';";
    let unit = match shape {
        "string-members" => {
            "s.slice(1);s.substring(0);s.indexOf('a');s.split(',');s.replaceAll('a','x');s.trim();s.padStart(4);"
        }
        "string-chains" => {
            "s.slice().trim().replace('a','b').padEnd(4).concat('x').substring(0,3);"
        }
        "string-concatenation" => "s=s+'a'+'b'+'c'+'d'+'e'+'f'+'g'+'h';",
        "string-mixed-concatenation" => "s=s+1+true+NaN+Infinity+-0;s+=false;",
        _ => "for(const c of s){s+=c;}",
    };
    let count = bytes.saturating_sub(prefix.len() + 1) / unit.len();
    let mut out = format!("{prefix}{}}}", unit.repeat(count));
    out.push_str(&" ".repeat(bytes - out.len()));
    out
}
