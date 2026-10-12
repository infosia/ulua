//! P2.1 source fixtures serve ABI, analyzer, and growth tests.

/// Shapes for P2.1 growth measurements and analyzer admission checks.
pub const SHAPES: &[&str] = &[
    "nested-functions",
    "arrows",
    "inferred-type-chain",
    "deep-result-types",
    "deep-parameter-types",
    "type-product-assign",
    "type-product-declare",
    "type-product-call",
    "type-product-self",
];

/// The inferred function result violates the arrow annotation rule.
pub fn diagnostic(shape: &str) -> Option<&'static str> {
    (shape == "inferred-type-chain").then_some("C11")
}

/// Fill a source size with declarations, arrows, or nested function annotations.
pub fn source(shape: &str, bytes: usize) -> String {
    if shape.starts_with("type-product-") {
        return type_product(shape, bytes);
    }
    let mut out = String::from("export function main():void{");
    match shape {
        "arrows" => out.push_str("let f=()=>0;"),
        "inferred-type-chain" => out.push_str("const f0=()=>0;"),
        _ => {}
    }
    let result_type = format!("{}number", "()=>".repeat(20));
    let mut parameter_type = String::from("number");
    for _ in 0..20 {
        parameter_type = format!("(f:{parameter_type})=>number");
    }
    let mut count = 0;
    loop {
        let unit = match shape {
            "nested-functions" => format!("function f{count}():number{{return {count};}}"),
            "arrows" => "f=()=>0;".to_owned(),
            "inferred-type-chain" => format!("const f{}=()=>f{count};", count + 1),
            "deep-result-types" | "deep-parameter-types" => {
                let ty = if shape == "deep-result-types" {
                    &result_type
                } else {
                    &parameter_type
                };
                format!("function f{count}(g:{ty}):{ty}{{return g;}}")
            }
            _ => return String::new(),
        };
        if out.len() + unit.len() + 1 > bytes {
            break;
        }
        out.push_str(&unit);
        count += 1;
    }
    out.push('}');
    out.push_str(&" ".repeat(bytes - out.len()));
    out
}

// Both sizes grow: width 15 at 16 KiB and width 31 at 64 KiB.
// The type occupies about one fifth of each source. The remainder holds repeated references.
fn type_product(shape: &str, bytes: usize) -> String {
    let width = ((bytes as f64 / 65536.0).sqrt() * 31.0) as usize;
    let width = width.clamp(1, 31);
    let inner = format!(
        "({}) => number",
        (0..width)
            .map(|i| format!("a{i}: number"))
            .collect::<Vec<_>>()
            .join(", ")
    );
    let ty = format!(
        "({}) => number",
        (0..width)
            .map(|i| format!("a{i}: {inner}"))
            .collect::<Vec<_>>()
            .join(", ")
    );
    let mut out = String::from("export function main():void{}");
    if shape == "type-product-call" {
        out.push_str(&format!("function q(x:{ty}):void{{}}"));
    }
    out.push_str(&format!("function f(g:{ty}):void{{"));
    if matches!(shape, "type-product-assign" | "type-product-self") {
        out.push_str("let k=g;");
    }
    let mut count = 0;
    loop {
        let unit = match shape {
            "type-product-assign" => "k = g;\n".to_owned(),
            "type-product-declare" => format!("const x{count} = g;\n"),
            "type-product-call" => "q(g);\n".to_owned(),
            "type-product-self" => "k = k;\n".to_owned(),
            _ => unreachable!("known product shape"),
        };
        if out.len() + unit.len() + 1 > bytes {
            break;
        }
        out.push_str(&unit);
        count += 1;
    }
    out.push('}');
    out.push_str(&" ".repeat(bytes - out.len()));
    out
}

/// Capture a specified number of distinct locals from two enclosing functions.
pub fn upvalues(count: usize) -> String {
    let first = count.min(100);
    let mut out = String::from("export function score():number{");
    for i in 0..first {
        out.push_str(&format!("let x{i}:number={i};"));
    }
    out.push_str("function middle():number{");
    for i in first..count {
        out.push_str(&format!("let x{i}:number={i};"));
    }
    out.push_str("const inner=():number=>{let total=0;");
    for i in 0..count {
        out.push_str(&format!("total+=x{i};"));
    }
    out.push_str("return total;};return inner();}return middle();}export function main():void{}");
    out
}

/// Capture direct locals and spill fields across three closure levels.
pub fn deep_spills() -> String {
    let mut out = String::from("export function score():number{");
    for level in 0..3 {
        for i in 0..220 {
            out.push_str(&format!("let x{level}_{i}:number={i};"));
        }
        if level < 2 {
            out.push_str(&format!("function f{level}():()=>number{{"));
        }
    }
    out.push_str("const inner=():number=>x0_0+x0_219+x1_0+x1_219+x2_0+x2_219;return inner;}return f1();}const g=f0();return g();}export function main():void{}");
    out
}

/// Keep closures from two iterations of each supported loop kind.
pub fn loop_closures(spill: bool) -> String {
    let mut out = String::from(
        "export function score():number{let a=():number=>-1;let b=():number=>-1;let c=():number=>-1;let d=():number=>-1;",
    );
    if spill {
        for i in 0..220 {
            out.push_str(&format!("let unused{i}={i};"));
        }
    }
    out.push_str("for(let i=0;i<2;i++){const value=i;function get():number{return value+i;}if(i===0){a=get;continue;}else{b=get;}}let j=0;while(j<2){const value=j;const get=():number=>value;if(j===0){c=get;}else{d=get;}j++;}return a()*1000+b()*100+c()*10+d();}export function main():void{}");
    out
}
