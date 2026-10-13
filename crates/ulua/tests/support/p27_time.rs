//! Dense try shapes keep their source density at the two witness sizes.
pub const SHAPES: [&str; 18] = [
    "sequential-finally",
    "sequential-catch",
    "sequential-both",
    "nested",
    "loop",
    "arrows",
    "arrows-try",
    "jumps-break-try",
    "jumps-break-catch",
    "jumps-break-finally",
    "jumps-continue-try",
    "jumps-continue-catch",
    "jumps-continue-finally",
    "jumps-return-try",
    "jumps-return-catch",
    "jumps-return-finally",
    "jumps-break-try-readonly",
    "jumps-break-plain",
];
pub fn source(shape: &str, bytes: usize) -> String {
    if shape.starts_with("jumps-") {
        return jumps(shape, bytes);
    }
    if shape.starts_with("arrows") {
        let mut out = "export function main():void{let a:number=0;".to_owned();
        let mut i = 0;
        loop {
            let body = if shape == "arrows" {
                "{a++;}{a=a+1;}"
            } else {
                "try{a++;}finally{a=a+1;}"
            };
            let unit = format!("const g{i}=():number=>{{{body}return 1;}};a=a+g{i}();");
            if out.len() + unit.len() + 1 > bytes {
                break;
            }
            out.push_str(&unit);
            i += 1;
        }
        out.push('}');
        out.extend(std::iter::repeat_n(' ', bytes - out.len()));
        return out;
    }
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

// Many completion edges share one closure. The readonly control writes no source binding in that closure.
fn jumps(shape: &str, bytes: usize) -> String {
    let returning = shape.starts_with("jumps-return-");
    let mut out = if returning {
        "function f():number{let a:number=0;while(a<100){"
    } else {
        "export function main():void{let a:number=0;while(a<100){"
    }
    .to_owned();
    let tail = if shape.ends_with("catch") {
        out.push_str("try{throw new Error('x');}catch{");
        "}finally{a++;}}"
    } else if shape.ends_with("finally") {
        out.push_str("try{a++;}finally{");
        "}}"
    } else if shape.ends_with("plain") {
        "}"
    } else {
        out.push_str("try{");
        "}finally{a++;}}"
    };
    let tail = format!(
        "{tail}{}",
        if returning {
            "return a;}export function main():void{f();}"
        } else {
            "}"
        }
    );
    let jump = if returning {
        "return a;"
    } else if shape.starts_with("jumps-continue-") {
        "continue;"
    } else {
        "break;"
    };
    let operation = if shape.ends_with("readonly") {
        "print('x');"
    } else {
        "a++;"
    };
    let mut i = 0;
    loop {
        let unit = format!("if(a>{i}){{{jump}}}{operation}");
        if out.len() + unit.len() + tail.len() > bytes {
            break;
        }
        out.push_str(&unit);
        i += 1;
    }
    out.push_str(&tail);
    out.extend(std::iter::repeat_n(' ', bytes - out.len()));
    out
}
