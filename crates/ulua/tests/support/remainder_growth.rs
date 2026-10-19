//! Fact and mark growth covers cycles, call chains, repeated calls, and nested calls.
/// Shapes for scalar, element, parameter facts, and integral marks.
pub const KEYS: &[&str] = &[
    "fact-rounds",
    "element-fact-rounds",
    "parameter-call-chain",
    "parameter-many-calls",
    "parameter-nested-calls",
    "integral-marks",
    "invariant-length",
];

pub fn source(shape: &str, bytes: usize) -> String {
    assert!(KEYS.contains(&shape));
    if shape == "invariant-length" {
        return invariant_length(bytes);
    }
    if shape.starts_with("parameter-") {
        return parameters(shape, bytes);
    }
    let mut out = "export function main():void{".to_owned();
    let scalar = "{let a=1;let b=a;let c=b;let d=c;let e=d;let f=e;let g=f;a=g;if(g%3===0){}}";
    let element = "{const a=[1];const b=[a[0]];const c=[b[0]];const d=[c[0]];const e=[d[0]];const f=[e[0]];const g=[f[0]];a[0]=g[0];if(g[0]%3===0){}}";
    // Both a surviving cycle and removal propagated around a cycle; neither has interval facts.
    let marks = "{const xs=[1];let a=-1;let b=a;let c=b;a=c;xs[c]=7%3;}{const xs=[1];let a=-1;let b=a;let c=b;a=c;a=1/2;xs[c]=7%3;}";
    let unit = if shape == "integral-marks" {
        marks
    } else if shape == "element-fact-rounds" {
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

// Each argument creates one fact write, across declarations or at repeated call sites.
fn parameters(shape: &str, bytes: usize) -> String {
    let mut out = "function f0(p:number):number{return p%3;}".to_owned();
    if shape == "parameter-nested-calls" {
        out.push_str("export function main():number{const k=1%3;");
        // Forty nested calls leave room for the function and callee under the depth ceiling of 48.
        let unit = format!("{}1{};", "f0(".repeat(40), ")".repeat(40));
        let tail = "return k;}";
        while out.len() + unit.len() + tail.len() <= bytes {
            out.push_str(&unit);
        }
        out.push_str(tail);
    } else if shape == "parameter-many-calls" {
        out.push_str("export function main():void{");
        while out.len() + 7 <= bytes {
            out.push_str("f0(1);");
        }
        out.push('}');
    } else {
        let mut last = 0;
        loop {
            let next = last + 1;
            let unit = format!("function f{next}(p:number):number{{return f{last}(p);}}");
            let tail = format!("export function main():number{{return f{next}(1);}}");
            if out.len() + unit.len() + tail.len() > bytes {
                break;
            }
            out.push_str(&unit);
            last = next;
        }
        out.push_str(&format!(
            "export function main():number{{return f{last}(1);}}"
        ));
    }
    out.extend(std::iter::repeat_n(' ', bytes - out.len()));
    out
}

// One counted body reads many arrays repeatedly. No write or call blocks the IR query.
fn invariant_length(bytes: usize) -> String {
    let arrays = (bytes / 256).min(64);
    let mut out = "export function main():number{let s=0;".to_owned();
    for i in 0..arrays {
        out.push_str(&format!("const a{i}=[1];"));
    }
    out.push_str("for(let i=0;i<1;i++){");
    let tail = "}return s;}";
    let mut next = 0;
    loop {
        let read = format!("s+=a{next}[i]%3;");
        if out.len() + read.len() + tail.len() > bytes {
            break;
        }
        out.push_str(&read);
        next = (next + 1) % arrays;
    }
    out.push_str(tail);
    out.extend(std::iter::repeat_n(' ', bytes - out.len()));
    out
}
