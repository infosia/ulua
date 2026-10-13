//! Dense await shapes cover serial requests, nesting, loops, and protected blocks.
pub const SHAPES: &[&str] = &[
    "sequential",
    "nested",
    "loop",
    "try",
    "adopt",
    "arrows",
    "methods",
];
pub fn source(shape: &str, bytes: usize) -> String {
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
