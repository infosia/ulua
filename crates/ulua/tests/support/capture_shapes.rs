//! Closure construction repeats without a call to the constructed closure.

pub const BASELINE: &str = "function start():number{let i=0;while(true){i++;}return 0;}const stop=start();export function main():void{}";

pub fn source(count: usize, writes: bool, nested: bool) -> String {
    let mut source = String::from("function start():number{");
    for i in 0..count {
        source.push_str(&format!(
            "{} v{i}={i};",
            if writes { "let" } else { "const" }
        ));
    }
    source.push_str(if nested {
        "while(true){const g=():()=>number=>{return ():number=>{let s=0;"
    } else {
        "while(true){const g=():number=>{let s=0;"
    });
    for i in 0..count {
        if writes {
            source.push_str(&format!("v{i}+=1;"));
        }
        source.push_str(&format!("s+=v{i};"));
    }
    source.push_str(if nested {
        "return s;};};}return 0;}const stop=start();export function main():void{}"
    } else {
        "return s;};}return 0;}const stop=start();export function main():void{}"
    });
    source
}

pub fn ratios(cap: usize) -> Vec<(String, String)> {
    let mut shapes = Vec::new();
    for count in [cap, cap + 1, 130, 190] {
        for (kind, writes, nested) in [
            ("read", false, false),
            ("write", true, false),
            ("nested-read", false, true),
            ("nested-write", true, true),
        ] {
            let mut source = source(count, writes, nested);
            source.extend(std::iter::repeat_n(' ', 65536 - source.len()));
            shapes.push((format!("capture-{kind}-{count}"), source));
        }
    }
    shapes
}
