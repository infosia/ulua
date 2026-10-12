//! Shared source shapes keep compiler state and full ABI measurements on identical inputs.
pub const SHAPES: &[&str] = &["classes", "methods", "wide", "chains", "constructions"];

pub fn source(shape: &str, bytes: usize) -> String {
    let mut declarations = String::new();
    let mut body = String::new();
    let mut group = 0;
    if shape == "constructions" {
        declarations.push_str(
            "class C{v:number;constructor(v:number){this.v=v;}m():number{return this.v;}}",
        );
    }
    loop {
        let mut unit = String::new();
        let mut calls = String::new();
        match shape {
            "classes" => {
                unit = format!("class C{group}{{v:number=1;m():number{{return this.v;}}}}");
                calls = format!("new C{group}().m();");
            }
            "methods" => {
                unit = format!("class C{group}{{v:number=1;");
                for i in 0..32 {
                    unit.push_str(&format!("m{i}():number{{return this.v;}}"));
                }
                unit.push('}');
                calls = format!("new C{group}().m31();");
            }
            "wide" => {
                unit = format!("class C{group}{{");
                for i in 0..128 {
                    unit.push_str(&format!("v{i}:number=1;"));
                }
                unit.push('}');
                calls = format!("const c{group}=new C{group}();c{group}.v127+=1;");
            }
            "chains" => {
                for i in (0..12).rev() {
                    if i == 11 {
                        unit.push_str(&format!("class C{group}_{i}{{v:number=1;}}"));
                    } else {
                        let next = i + 1;
                        unit.push_str(&format!("class C{group}_{i}{{next():C{group}_{next}{{return new C{group}_{next}();}}}}"));
                    }
                }
                // A call chain has a bounded source depth. Each repeated group has twelve class links.
                calls = format!("new C{group}_0().next().next().next().next();");
            }
            _ => calls.push_str("new C(1).m();"),
        }
        if matches!(shape, "classes" | "methods" | "chains") {
            unit.push_str(&format!("function f{group}():void{{{calls}}}"));
            calls.clear();
        }
        if declarations.len() + unit.len() + body.len() + calls.len() + 31 > bytes {
            break;
        }
        declarations.push_str(&unit);
        body.push_str(&calls);
        group += 1;
    }
    let mut out = format!("{declarations}export function main():void{{{body}}}");
    out.push_str(&" ".repeat(bytes - out.len()));
    out
}
