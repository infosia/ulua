//! Dense callbacks share the spill prefix and closure context of the dense family.
#[allow(dead_code)]
#[path = "../../src/dense_inputs.rs"]
mod dense;

pub const KEYS: &[&str] = &["inline-callbacks", "block-callbacks"];

pub fn source(shape: &str, bytes: usize) -> String {
    let reference = dense::source("assign", bytes);
    let (prefix, _) = reference.split_once("x=x;").unwrap();
    let unit = match shape {
        "inline-callbacks" => "z.map((t:number)=>t);",
        "block-callbacks" => "z.map((t:number):number=>{return t;});",
        _ => panic!("unknown callback growth shape"),
    };
    let mut source = prefix.to_owned();
    let suffix = "};}";
    while source.len() + unit.len() + suffix.len() <= bytes {
        source.push_str(unit);
    }
    source.push_str(suffix);
    source.extend(std::iter::repeat_n(' ', bytes - source.len()));
    source
}
