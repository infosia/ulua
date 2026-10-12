//! Method growth repeats calls and bounded chains at the source ceiling.
// Runtime array length does not change compiler work.
pub const SHAPES: &[&str] = &["method-calls", "method-chains", "method-callbacks"];

pub fn source(shape: &str, bytes: usize) -> String {
    let prefix = "function m(v:number):number{return v;}function f(v:number):boolean{return v>0;}export function main():void{let x=[1];";
    let unit = match shape {
        "method-calls" => "x.pop();x.unshift(1);x.slice();x.indexOf(1);",
        "method-chains" => "x.map(m).filter(f).map(m).filter(f).map(m).filter(f);",
        _ => "x.forEach(m);x.reduce((a:number,v:number):number=>a+v,0);",
    };
    let count = bytes.saturating_sub(prefix.len() + 1) / unit.len();
    let mut out = format!("{prefix}{}}}", unit.repeat(count));
    out.push_str(&" ".repeat(bytes - out.len()));
    out
}
