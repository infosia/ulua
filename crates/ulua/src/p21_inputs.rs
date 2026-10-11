//! P2.1 source fixtures serve ABI, analyzer, and growth tests.

/// Shapes for the P2.1 analyzer and growth gates.
pub const SHAPES: &[&str] = &["nested-functions", "arrows"];

/// Fill a source size with sibling declarations or arrow assignments.
pub fn source(shape: &str, bytes: usize) -> String {
    let mut out = String::from("export function main():void{");
    if shape == "arrows" {
        out.push_str("let f=()=>0;");
    }
    let mut count = 0;
    loop {
        let unit = match shape {
            "nested-functions" => format!("function f{count}():number{{return {count};}}"),
            "arrows" => "f=()=>0;".to_owned(),
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
