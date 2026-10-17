//! Each seven-binding cycle requires eight fact rounds, including the stable round.
pub fn source(_: &str, bytes: usize) -> String {
    let mut out = "export function main():void{".to_owned();
    let unit = "{let a=1;let b=a;let c=b;let d=c;let e=d;let f=e;let g=f;a=g;if(g%3===0){}}";
    while out.len() + unit.len() < bytes {
        out.push_str(unit);
    }
    out.push('}');
    out.extend(std::iter::repeat_n(' ', bytes - out.len()));
    out
}
