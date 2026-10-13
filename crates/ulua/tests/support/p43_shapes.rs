//! Dense async host calls exercise suspension, adoption, and parallel requests.
pub const SHAPES: &[&str] = &[
    "host-sequential",
    "host-nested",
    "host-then",
    "host-finally",
    "host-all",
    "host-try",
];
pub fn grants() -> Vec<ulua_compiler::HostSignature> {
    let mut grant = ulua_compiler::HostSignature::new("hostLater", vec![1], 1);
    grant.asynchronous = true;
    vec![grant]
}
pub fn source(shape: &str, bytes: usize) -> String {
    let mut out = String::from(
        "// grants: hostLater\nfunction plus(n:number):number{return n+1;}async function done():Promise<void>{await hostLater(0);}export async function main():Promise<void>{let total=0;",
    );
    let (unit,tail)=match shape {
        "host-nested"=>(format!("total+={}1{};","await hostLater(".repeat(10),")".repeat(10)),"}"),
        "host-then"=>("total+=await hostLater(1).then(plus).then(plus);".to_owned(),"}"),
        "host-finally"=>("total+=await hostLater(1).finally(done);".to_owned(),"}"),
        "host-try"=>("try{total+=await hostLater(1);}catch{total+=await hostLater(2);}finally{total+=await hostLater(3);}".to_owned(),"}"),
        "host-all"=>{out.push_str("const ps=[");("hostLater(1),".to_owned(),"];const xs=await Promise.all(ps);}")},
        _=>("total+=await hostLater(1);".to_owned(),"}"),
    };
    while out.len() + unit.len() + tail.len() <= bytes {
        out.push_str(&unit);
    }
    out.push_str(tail);
    out.extend(std::iter::repeat_n(' ', bytes - out.len()));
    out
}
