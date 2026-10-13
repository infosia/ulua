//! Dense await shapes cover serial requests, nesting, loops, and protected blocks.
pub const SHAPES: &[&str] = &[
    "sequential",
    "nested",
    "loop",
    "try",
    "adopt",
    "arrows",
    "methods",
    "finally-break",
    "finally-continue",
    "capture-finally-break",
    "capture-finally-continue",
    "capture-finally-break-deep",
    "capture-finally-continue-deep",
];
pub fn source(shape: &str, bytes: usize) -> String {
    if shape.contains("finally-") {
        return finalizer(shape, bytes);
    }
    let prefix = "async function v(n:number=1):Promise<number>{return n;}export async function main():Promise<void>{let a:number=0;";
    let unit = match shape {
        "nested" => "a+=await v();".to_owned(),
        "loop" => "for(let i=0;i<2;i++){a+=await v();}".to_owned(),
        "try" => "try{a+=await v();}catch{a+=await v();}finally{a+=await v();}".to_owned(),
        _ => "a+=await v();".to_owned(),
    };
    let mut out = if shape == "methods" {
        "class C{async v():Promise<number>{return 1;}}export async function main():Promise<void>{const c=new C();let a:number=0;".to_owned()
    } else {
        prefix.to_owned()
    };
    let tail = "}";
    let mut index = 0;
    loop {
        let unit = match shape {
            "nested" => format!("a+={}1{};", "await v(".repeat(10), ")".repeat(10)),
            "adopt" => format!(
                "async function f{index}():Promise<number>{{return v();}}a+=await f{index}();"
            ),
            "arrows" => format!(
                "const f{index}=async():Promise<number>=>{{return await v();}};a+=await f{index}();"
            ),
            "methods" => "a+=await c.v();".to_owned(),
            _ => unit.clone(),
        };
        if out.len() + unit.len() + tail.len() > bytes {
            break;
        }
        out.push_str(&unit);
        index += 1;
    }
    out.push_str(tail);
    out.extend(std::iter::repeat_n(' ', bytes - out.len()));
    out
}

// Loop jumps retain old heads while dense finalizers extend the current path.
fn finalizer(shape: &str, bytes: usize) -> String {
    let captured = shape.starts_with("capture-");
    let depth = if shape.ends_with("-deep") { 16 } else { 8 };
    let jump = if shape.contains("continue") {
        "continue"
    } else {
        "break"
    };
    let mut out = String::from("async function t():Promise<number>{return 1;}");
    if captured {
        out.push_str("async function parent(c:boolean):Promise<number>{let x:number|null=1;const f=async():Promise<number>=>{");
    } else {
        out.push_str("async function f(c:boolean):Promise<number>{let x:number|null=1;");
    }
    for i in 0..depth {
        out.push_str(&format!("for(let i{i}=0;i{i}<1;i{i}++){{if(c){jump};"));
    }
    let tail = format!(
        "{}return 0;{}export async function main():Promise<void>{{await {}(false);}}",
        "}".repeat(depth),
        if captured { "};return f();}" } else { "}" },
        if captured { "parent" } else { "f" },
    );
    // Local writes also extend paths when await preserves the body's local narrowing.
    let unit = if captured {
        "if(x==null)return 0;try{}finally{await t()}"
    } else {
        "x=1;if(x==null)return 0;try{}finally{await t()}"
    };
    while out.len() + unit.len() + tail.len() <= bytes {
        out.push_str(unit);
    }
    out.push_str(&tail);
    out.extend(std::iter::repeat_n(' ', bytes - out.len()));
    out
}
