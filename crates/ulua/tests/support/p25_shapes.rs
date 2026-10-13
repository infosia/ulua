//! Record growth sources serve the ABI and analyzer measurements.
use ulua_compiler::FIELD_BOUND;
pub const SHAPES: &[&str] = &[
    "fields",
    "literals",
    "nested",
    "tests",
    "joins",
    "functions",
    "field-product",
    "fact-product",
    "record-functions",
    "equal-alias-links",
    "break-product",
    "recursive-product",
    "conditional-tests",
    "alternating-logical-tests",
    "conditional-wrapped-tests",
    "logical-wrapped-tests",
    "nullable-conditional-tests",
    "continue-product",
    "ybreak",
    "ycont",
    "distinct-const-break",
    "continue-update-writes",
    "if-end-writes",
];
fn join_source(shape: &str, bytes: usize) -> String {
    let mut out = "function g():string|null{return null;}export function main():void{const c:boolean=false;let y:string|null=g();".to_owned();
    let tail = "}}";
    out.push_str(match shape {
        "ycont" => "for(let i=0;i<1;i++){",
        "continue-update-writes" => "for(let i=0;i<1;y=g()){i+=1;",
        "distinct-const-break" => "while(c){if(y==null){return;}",
        _ => "while(c){",
    });
    let mut i = 0;
    loop {
        let unit = match shape {
            "ybreak" => "if(y==null)break;y=g();".to_owned(),
            "ycont" | "continue-update-writes" => "if(y==null)continue;y=g();".to_owned(),
            "distinct-const-break" => {
                format!("const x{i}:string|null=g();if(x{i}==null){{y=g();break;}}")
            }
            _ => "if(y==null){y=g();}else{y=g();}y=g();".to_owned(),
        };
        if out.len() + unit.len() + tail.len() > bytes {
            break;
        }
        out.push_str(&unit);
        i += 1;
    }
    out.push_str(tail);
    out.push_str(&" ".repeat(bytes - out.len()));
    out
}
pub fn condition_depth(shape: &str) -> usize {
    match shape {
        "conditional-wrapped-tests" | "logical-wrapped-tests" => 8,
        "nullable-conditional-tests" => 20,
        _ => 21,
    }
}
fn condition(shape: &str, depth: usize) -> String {
    let mut test = if shape == "nullable-conditional-tests" {
        "x!=null".to_owned()
    } else {
        "b".to_owned()
    };
    for level in 0..depth {
        test = match shape {
            "alternating-logical-tests" => {
                format!("({test}{}b)", if level % 2 == 0 { "&&" } else { "||" })
            }
            "conditional-wrapped-tests" => format!("(!(({test}?true:false)))"),
            "logical-wrapped-tests" => {
                format!("(!(({test}{}b)))", if level % 2 == 0 { "&&" } else { "||" })
            }
            _ => format!("({test}?true:false)"),
        };
    }
    test
}
pub fn condition_source(shape: &str, depth: usize) -> String {
    let test = condition(shape, depth);
    let body = if shape == "nullable-conditional-tests" {
        "if(x!=null){x.v+=1;}"
    } else {
        ""
    };
    format!(
        "type B={{v:number}};export function main():void{{const b:boolean=true;const x:B|null={{v:0}};if({test}){{{body}}}}}"
    )
}
pub fn make(shape: &str, bytes: usize) -> String {
    if shape == "equal-alias-links" {
        let mut out = String::new();
        let tail = "export function main():void{}";
        let mut i = 0;
        loop {
            let unit = format!(
                "type P{i}={{v:number}};type Q{i}={{v:number}};type R{i}={{xs:Q{i}[];n:Q{i}|null;f:(p:P{i})=>Q{i}}};function f{i}(r:R{i}):number{{const xs:P{i}[]=r.xs;const n:P{i}|null=r.n;const f:(p:Q{i})=>P{i}=r.f;return xs[0].v+(n!=null?n.v:0)+f({{v:1}}).v;}}"
            );
            if out.len() + unit.len() + tail.len() > bytes {
                break;
            }
            out.push_str(&unit);
            i += 1;
        }
        out.push_str(tail);
        out.extend(std::iter::repeat_n(' ', bytes - out.len()));
        return out;
    }
    if matches!(
        shape,
        "ybreak" | "ycont" | "distinct-const-break" | "continue-update-writes" | "if-end-writes"
    ) {
        return join_source(shape, bytes);
    }
    let mut declarations = "type B={v:number};".to_owned();
    let mut prefix = String::new();
    let unit: String;
    let mut tail = "}";
    match shape {
        "conditional-tests"
        | "alternating-logical-tests"
        | "conditional-wrapped-tests"
        | "logical-wrapped-tests"
        | "nullable-conditional-tests" => {
            prefix = "const b:boolean=true;const x:B|null={v:0};".into();
            let test = condition(shape, condition_depth(shape));
            let body = if shape == "nullable-conditional-tests" {
                "if(x!=null){x.v+=1;}"
            } else {
                ""
            };
            unit = format!("if({test}){{{body}}}");
        }
        "continue-product" => {
            let count = bytes / 190;
            for i in 0..count {
                prefix.push_str(&format!("const x{i}:B|null={{v:0}};"));
            }
            prefix.push_str("for(let i=0;i<2;i+=x0.v){");
            for i in 0..count {
                prefix.push_str(&format!("if(x{i}==null){{return;}}"));
            }
            unit = "if(x0.v==1){continue;}".into();
            tail = "i+=1;}}";
        }
        "fields" | "field-product" => {
            let fields = if shape == "fields" {
                (bytes / 48).min(FIELD_BOUND)
            } else {
                bytes / 160
            };
            declarations = format!(
                "type R={{{}}};",
                (0..fields)
                    .map(|i| format!("f{i}:number;"))
                    .collect::<String>()
            );
            prefix = format!(
                "const r:R={{{}}};",
                (0..fields).map(|i| format!("f{i}:0,")).collect::<String>()
            );
            unit = "r.f0=r.f0;".into();
        }
        "literals" => {
            prefix = "const xs:B[]=[];".into();
            unit = "xs.push({v:0});".into();
        }
        "nested" => {
            // Fixed alias depth isolates the cost of repeated nested literals.
            declarations = "type R0={v:number};".into();
            for i in 1..12 {
                declarations.push_str(&format!("type R{i}={{v:R{}}};", i - 1));
            }
            let mut literal = "{v:0}".to_owned();
            for _ in 1..12 {
                literal = format!("{{v:{literal}}}");
            }
            prefix = "const xs:R11[]=[];".into();
            unit = format!("xs.push({literal});");
        }
        "tests" => {
            prefix = "const x:B|null={v:1};".into();
            unit = "if(x!=null){print(`${x.v}`);}".into();
        }
        "joins" => {
            prefix = "let x:B|null={v:1};".into();
            unit = "if(x!=null){if(x.v==0){x=null;}else{x={v:0};}}x?.v??0;"
                .replace("x?.v??0;", "print(`${x?.v??0}`);");
        }
        "functions" => {
            prefix = "const x:B|null={v:1};".into();
            unit = "if(x!=null){const f=():()=>number=>():number=>x.v;const g=f();g();}".into();
        }
        "recursive-product" => {
            // Repeated bounded components keep this accepted shape below the type-depth limit.
            declarations.clear();
            let mut group = 0;
            loop {
                let mut component = String::new();
                for (prefix, count) in [("A", 23), ("C", 24)] {
                    for i in 0..count {
                        component.push_str(&format!(
                            "type {prefix}{group}_{i}={{v:{prefix}{group}_{}}};",
                            (i + 1) % count
                        ));
                    }
                }
                component.push_str(&format!(
                    "function same{group}(a:A{group}_0,b:C{group}_0):boolean{{return a===b;}}"
                ));
                if declarations.len() + component.len() + 35 > bytes {
                    break;
                }
                declarations.push_str(&component);
                group += 1;
            }
            unit = "".into();
        }
        "break-product" => {
            let count = bytes / 160;
            for i in 0..count {
                prefix.push_str(&format!("const x{i}:B|null={{v:0}};"));
            }
            prefix.push_str("while(true){");
            for i in 0..count {
                prefix.push_str(&format!("if(x{i}==null){{return;}}"));
            }
            unit = "if(x0.v==1){break;}".into();
            tail = "break;}}";
        }
        "fact-product" => {
            for i in 0..bytes / 180 {
                prefix.push_str(&format!(
                    "const x{i}:B|null={{v:0}};if(x{i}==null){{return;}}"
                ));
            }
            unit = "if(x0!=null){const f=():number=>x0.v;f();}".into();
        }
        _ => {
            declarations = "type R={f:(n:number)=>number};".into();
            prefix = "const r:R={f:(n:number):number=>n};".into();
            unit = "r.f(0);".into();
        }
    }
    let mut out = format!("{declarations}export function main():void{{{prefix}");
    while !unit.is_empty() && out.len() + unit.len() + tail.len() <= bytes {
        out.push_str(&unit);
    }
    out.push_str(tail);
    assert!(out.len() <= bytes, "{shape}");
    out.push_str(&" ".repeat(bytes - out.len()));
    out
}
