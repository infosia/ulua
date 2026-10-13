//! Dense protected calls cover loop depth, outer exits, and protected exits.
//! The ceiling uses an independent small compile probe for each form.
use std::sync::OnceLock;
use ulua_compiler::{Limits, compile};

pub const SHAPES: &[&str] = &[
    "0",
    "4",
    "8",
    "12",
    "17",
    "ceiling",
    "ceiling-none",
    "ceiling-continue",
    "ceiling-inner-break",
    "ceiling-inner-continue",
];

pub fn sync_finally(key: &str, bytes: usize) -> String {
    source(false, "finally", key, bytes)
}
pub fn sync_catch(key: &str, bytes: usize) -> String {
    source(false, "catch", key, bytes)
}
pub fn sync_nested(key: &str, bytes: usize) -> String {
    source(false, "nested", key, bytes)
}
pub fn async_finally(key: &str, bytes: usize) -> String {
    source(true, "finally", key, bytes)
}
pub fn async_catch(key: &str, bytes: usize) -> String {
    source(true, "catch", key, bytes)
}
pub fn async_nested(key: &str, bytes: usize) -> String {
    source(true, "nested", key, bytes)
}

pub fn ceiling(asynchronous: bool, kind: &str, edge: &str) -> usize {
    static CEILINGS: OnceLock<[[[usize; 5]; 3]; 2]> = OnceLock::new();
    let values = CEILINGS.get_or_init(|| {
        [false, true].map(|asynchronous| {
            ["finally", "catch", "nested"].map(|kind| {
                ["break", "none", "continue", "inner-break", "inner-continue"].map(|edge| {
                    let mut last = 0;
                    for depth in 1..=48 {
                        let source = at_depth(asynchronous, kind, edge, depth, 0);
                        match compile(source.as_bytes(), Limits::default()) {
                            Ok(_) => last = depth,
                            Err(d) => {
                                assert_eq!(d.code, "limit:nesting-depth", "{kind} {edge}: {d:?}");
                                break;
                            }
                        }
                    }
                    last
                })
            })
        })
    });
    let kind = match kind {
        "finally" => 0,
        "catch" => 1,
        _ => 2,
    };
    let edge = match edge {
        "break" => 0,
        "none" => 1,
        "continue" => 2,
        "inner-break" => 3,
        _ => 4,
    };
    values[usize::from(asynchronous)][kind][edge]
}

pub fn source(asynchronous: bool, kind: &str, key: &str, bytes: usize) -> String {
    let (level, edge) = key.split_once('-').unwrap_or((key, "break"));
    let depth = if level == "ceiling" {
        ceiling(asynchronous, kind, edge)
    } else {
        level.parse().unwrap()
    };
    at_depth(asynchronous, kind, edge, depth, bytes)
}

pub fn at_depth(asynchronous: bool, kind: &str, edge: &str, depth: usize, bytes: usize) -> String {
    let mut out = if asynchronous {
        "async function t():Promise<number>{return 1;}export async function main():Promise<void>{"
    } else {
        "function t():number{return 1;}export function main():void{"
    }
    .to_owned();
    out.push_str("const c:boolean=false;let r:number=0;");
    for k in 0..depth {
        out.push_str(&format!("for(let i{k}=0;i{k}<1;i{k}++){{"));
        if matches!(edge, "break" | "continue") {
            out.push_str(&format!("if(c){edge};"));
        }
    }
    let call = if asynchronous { "await t()" } else { "t()" };
    let jump = edge
        .strip_prefix("inner-")
        .map_or(String::new(), |j| format!("if(c){j};"));
    let unit = match kind {
        "finally" => format!(
            "try{{}}finally{{{}{}{jump}}}",
            if asynchronous {
                call.to_owned()
            } else {
                format!("r={call}")
            },
            if jump.is_empty() { "" } else { ";" }
        ),
        "catch" => format!("try{{{jump}r+={call};}}catch{{}}"),
        _ => format!("try{{try{{{jump}r+={call};}}catch{{}}}}finally{{}}"),
    };
    let tail = "}".repeat(depth + 1);
    if bytes == 0 {
        out.push_str(&unit);
        out.push_str(&tail);
        return out;
    }
    assert!(out.len() + unit.len() + tail.len() <= bytes);
    while out.len() + unit.len() + tail.len() <= bytes {
        out.push_str(&unit);
    }
    out.push_str(&tail);
    out.extend(std::iter::repeat_n(' ', bytes - out.len()));
    out
}
