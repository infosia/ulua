//! Collection loops and repeated source forms serve both instruction and analyzer gates.
pub const KEYS: &[&str] = &["operations", "copy-compact", "array-copy", "traversal"];
pub fn source(shape: &str, bytes: usize) -> String {
    let prefix =
        "export function main():void{const m=new Map<number,number>();const s=new Set<number>();";
    let unit = match shape {
        "operations" => {
            "m.set(1,2);m.get(1);m.getOr(2,3);m.has(1);m.delete(1);s.add(1);s.has(1);s.delete(1);"
        }
        "copy-compact" => {
            "{for(let i=0;i<16;i++){m.set(i,i);}for(let i=0;i<12;i++){m.delete(i);}const copy=new Map<number,number>(m);copy.clear();m.clear();}"
        }
        "array-copy" => {
            "{const a=new Set<number>([1,2,3,1,2,3]);const b=new Set<number>(a);b.clear();}"
        }
        "traversal" => {
            "m.set(1,2);s.add(1);for(const k of m.keys()){}for(const v of m.values()){}for(const k of s){}m.forEach((v:number,k:number):void=>{});s.forEach((k:number):void=>{});"
        }
        _ => panic!("unknown table shape"),
    };
    let mut out = prefix.to_owned();
    while out.len() + unit.len() < bytes {
        out.push_str(unit);
    }
    out.push('}');
    out.extend(std::iter::repeat_n(' ', bytes - out.len()));
    out
}
pub fn interrupts(bytes: usize) -> Vec<(&'static str, String)> {
    let prefix = "export function main(b:boolean):void{";
    [
        ("table-compact", "const m=new Map<number,number>();for(let i=0;i<256;i++){m.set(i,i);}for(let i=0;i<200;i++){m.delete(i);}"),
        ("table-map-copy", "const m=new Map<number,number>();for(let i=0;i<256;i++){m.set(i,i);}const c=new Map<number,number>(m);"),
        ("table-array-set-copy", "const a:number[]=[];for(let i=0;i<256;i++){a.push(i);}const s=new Set<number>(a);const c=new Set<number>(s);"),
        ("table-foreach", "const m=new Map<number,number>();const s=new Set<number>();for(let i=0;i<256;i++){m.set(i,i);s.add(i);}m.forEach((v:number,k:number):void=>{});s.forEach((k:number):void=>{});"),
        ("table-for-of", "const m=new Map<number,number>();const s=new Set<number>();for(let i=0;i<256;i++){m.set(i,i);s.add(i);}for(const k of m.keys()){}for(const v of m.values()){}for(const k of s){}"),
        ("table-read", "const m=new Map<number,number>();m.set(1,1);for(let i=0;i<256;i++){m.get(1);m.getOr(2,0);m.has(1);}"),
    ].into_iter().map(|(name,body)| {let mut source=format!("{prefix}{body}}}");source.extend(std::iter::repeat_n(' ',bytes-source.len()));(name,source)}).collect()
}
