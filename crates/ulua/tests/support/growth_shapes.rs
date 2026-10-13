//! The analyzer witnesses use each source generator of the ABI growth gates.
#[allow(dead_code)]
#[path = "../../src/p22_inputs.rs"]
mod arrays;
#[path = "../../../ulua_compiler/src/p26_shapes.rs"]
mod classes;
#[path = "../../src/dense_inputs.rs"]
mod dense;
#[path = "p27_time.rs"]
mod exceptions;
#[allow(dead_code)]
#[path = "../../src/p21_inputs.rs"]
mod functions;
#[allow(dead_code)]
#[path = "../../src/s3_inputs.rs"]
mod inputs;
#[path = "../../src/p23_inputs.rs"]
mod methods;
#[allow(dead_code)]
#[path = "p25_shapes.rs"]
mod records;
#[path = "../../src/s6_inputs.rs"]
mod review;
#[path = "../../src/p24_inputs.rs"]
mod strings;

pub struct Shape {
    pub label: String,
    pub key: &'static str,
    pub make: fn(&str, usize) -> String,
    pub diagnostic: Option<&'static str>,
}

fn add(
    out: &mut Vec<Shape>,
    family: &str,
    keys: &'static [&'static str],
    make: fn(&str, usize) -> String,
) {
    out.extend(keys.iter().map(|key| Shape {
        label: format!("{family}-{key}"),
        key,
        make,
        diagnostic: None,
    }));
}

fn p1(shape: &str, bytes: usize) -> String {
    if matches!(shape, "forward" | "backward" | "star") {
        inputs::call_graph(shape, bytes)
    } else {
        inputs::source_ceiling(shape, bytes)
    }
}

fn nil_checks(shape: &str, bytes: usize) -> String {
    let compound = shape == "compound-five";
    let levels = if compound { 5 } else { 30 };
    let accesses = "[0]".repeat(levels);
    let unit = format!("x{accesses}.f{}1;", if compound { "+=" } else { "=" });
    let mut out = format!(
        "type B={{f:number}};function f(x:B{}):void{{",
        "[]".repeat(levels)
    );
    let tail = "}export function main():void{}";
    while out.len() + unit.len() + tail.len() <= bytes {
        out.push_str(&unit);
    }
    out.push_str(tail);
    out.extend(std::iter::repeat_n(' ', bytes - out.len()));
    out
}

pub fn all() -> Vec<Shape> {
    let mut out = vec![];
    add(&mut out, "S4", inputs::SOURCE_SHAPES, p1);
    add(&mut out, "S6", review::SHAPES, review::source);
    add(&mut out, "P21", functions::SHAPES, functions::source);
    for s in &mut out {
        if s.label.starts_with("P21-") {
            s.diagnostic = functions::diagnostic(s.key);
        }
    }
    add(&mut out, "P22", arrays::SHAPES, arrays::source);
    add(&mut out, "P22", arrays::SPILL_SHAPES, arrays::spill_source);
    add(&mut out, "P23", methods::SHAPES, methods::source);
    add(&mut out, "P24", strings::SHAPES, strings::source);
    add(&mut out, "dense", dense::SHAPES, dense::source);
    add(&mut out, "P25", records::SHAPES, records::make);
    add(
        &mut out,
        "P25",
        &["compound-five", "store-thirty"],
        nil_checks,
    );
    add(&mut out, "P26", classes::SHAPES, classes::source);
    add(&mut out, "P27", &exceptions::SHAPES, exceptions::source);
    out
}
