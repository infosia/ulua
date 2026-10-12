//! Protected shapes retain the same source density at n and 4n.
pub const SHAPES: [&str; 3] = ["sequential", "nested", "loop"];
pub fn source(shape: &str, count: usize) -> String {
    let mut out = "export function main():void{let x:number=0;".to_owned();
    for _ in 0..count {
        match shape {
            "nested" => out.push_str("try{"),
            "loop" => out.push_str("for(let i=0;i<2;i++){try{x++;}catch{}finally{x++;}}"),
            _ => out.push_str("try{x++;}catch{}finally{x++;}"),
        }
    }
    if shape == "nested" {
        out.push_str("x++;");
        out.push_str(&"}catch{}finally{x++;}".repeat(count));
    }
    out.push('}');
    out
}
