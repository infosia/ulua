//! Dense host calls cover values, defaults, methods, arrows, and protected bodies.
pub const SHAPES: &[&str] = &[
    "calls",
    "arguments",
    "strings",
    "booleans",
    "defaults",
    "methods",
    "arrows",
    "protected",
    "declarations",
];
pub fn grants(source: &str) -> Vec<ulua_compiler::HostSignature> {
    let Some(shape) = source
        .lines()
        .next()
        .and_then(|l| l.strip_prefix("// host-shape:"))
    else {
        return vec![];
    };
    if shape == "declarations" {
        let mut grants = vec![];
        for index in 0..source.matches("();").count() {
            grants.push(ulua_compiler::HostSignature::new(
                format!("host{index}"),
                vec![],
                1,
            ));
        }
        grants
    } else {
        let (p, r) = match shape {
            "strings" => (vec![3], 3),
            "booleans" => (vec![2], 2),
            "arguments" => (vec![1, 1], 1),
            _ => (vec![], 1),
        };
        vec![ulua_compiler::HostSignature::new("h", p, r)]
    }
}
pub fn source(shape: &str, bytes: usize) -> String {
    let declaration = format!("// host-shape:{shape}\n");
    let (prefix, unit, tail) = match shape {
        "defaults" => (
            "function f(n:number=h()):number{return n;}export function main():void{",
            "f();",
            "}",
        ),
        "methods" => (
            "class C{f():number{return h();}}export function main():void{const c=new C();",
            "c.f();",
            "}",
        ),
        "arrows" => (
            "export function main():void{const f=():number=>h();",
            "f();",
            "}",
        ),
        "protected" => ("export function main():void{", "try{h();}finally{}", "}"),
        "strings" => ("export function main():void{", "h('');", "}"),
        "booleans" => ("export function main():void{", "h(false);", "}"),
        "arguments" => ("export function main():void{", "h(h(1,2),h(3,4));", "}"),
        _ => ("export function main():void{", "h();", "}"),
    };
    if shape == "declarations" {
        let mut s = format!("// host-shape:{shape}\n");
        let mut i = 0;
        s.push_str("export function main():void{");
        let tail = "}";
        loop {
            let unit = format!("host{i}();");
            if s.len() + unit.len() + tail.len() > bytes {
                break;
            }
            s.push_str(&unit);
            i += 1;
        }
        s.push_str(tail);
        s.extend(std::iter::repeat_n(' ', bytes - s.len()));
        return s;
    }
    let mut s = format!("{declaration}{prefix}");
    while s.len() + unit.len() + tail.len() <= bytes {
        s.push_str(unit);
    }
    s.push_str(tail);
    s.extend(std::iter::repeat_n(' ', bytes - s.len()));
    s
}
