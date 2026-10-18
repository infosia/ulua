//! Each seven-binding cycle requires eight fact rounds, including the stable round.
pub fn source(shape: &str, bytes: usize) -> String {
    let mut out = "export function main():void{".to_owned();
    let scalar = "{let a=1;let b=a;let c=b;let d=c;let e=d;let f=e;let g=f;a=g;if(g%3===0){}}";
    let element = "{const a=[1];const b=[a[0]];const c=[b[0]];const d=[c[0]];const e=[d[0]];const f=[e[0]];const g=[f[0]];a[0]=g[0];if(g[0]%3===0){}}";
    let unit = if shape == "element-fact-rounds" {
        element
    } else {
        scalar
    };
    while out.len() + unit.len() < bytes {
        out.push_str(unit);
    }
    out.push('}');
    out.extend(std::iter::repeat_n(' ', bytes - out.len()));
    out
}
