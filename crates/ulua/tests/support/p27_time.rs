//! Dense try shapes keep their source density at the two witness sizes.
pub const SHAPES: [&str; 5] = [
    "sequential-finally",
    "sequential-catch",
    "sequential-both",
    "nested",
    "loop",
];
pub fn source(shape: &str, bytes: usize) -> String {
    let unit = match shape {
        "sequential-finally" => "try{a++;}finally{}".to_owned(),
        "sequential-catch" => "try{a++;}catch{}".to_owned(),
        "sequential-both" => "try{a++;}catch{}finally{a++;}".to_owned(),
        "nested" => "try{".repeat(21) + "a++;" + &"}catch{}finally{a++;}".repeat(21),
        _ => "for(let i=0;i<2;i++){try{a++;}catch{}finally{a++;}}".to_owned(),
    };
    let mut source = "export function main():void{let a:number=0;".to_owned();
    let tail = "}";
    while source.len() + unit.len() + tail.len() <= bytes {
        source.push_str(&unit);
    }
    source.push_str(tail);
    source.extend(std::iter::repeat_n(' ', bytes - source.len()));
    source
}
