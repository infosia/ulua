//! Array growth fixtures. Each accepted shape fills the source ceiling.

/// P2.2 shapes include arrays, iteration, and products with function element types.
pub const SHAPES: &[&str] = &[
    "array-literals",
    "array-nested",
    "array-index-reads",
    "array-index-writes",
    "array-forof-body",
    "array-function-product",
    "array-wide-function-product",
    "array-large-literal",
    "array-wide-computed",
    "array-wide-nested",
    "array-wide-computed-module",
];

/// Every array growth shape compiles without a limit diagnostic.
pub fn diagnostic(_shape: &str) -> Option<&'static str> {
    None
}

/// Fill a size with the specified shape. Products grow both the type and reference count.
pub fn source(shape: &str, bytes: usize) -> String {
    let literals = vec!["1"; 64].join(",");
    let nested = format!("{}1{}", "[".repeat(12), "]".repeat(12));
    let (prefix, unit, suffix) = match shape {
        "array-literals" => (
            "export function main():void{let xs:number[]=[];".into(),
            format!("xs=[{literals}];"),
            "}",
        ),
        "array-nested" => (
            format!("export function main():void{{let xs={nested};"),
            format!("xs={nested};"),
            "}",
        ),
        "array-index-reads" => (
            "export function main():void{const xs=[1];let x=0;".into(),
            "x=xs[0];".into(),
            "}",
        ),
        "array-index-writes" => (
            "export function main():void{const xs=[1];".into(),
            "xs[0]+=1;".into(),
            "}",
        ),
        "array-forof-body" => (
            format!(
                "export function main():void{{const xs=[{literals}];let sum=0;for(const x of xs){{"
            ),
            "sum+=x;".into(),
            "}}",
        ),
        "array-function-product" | "array-wide-function-product" => {
            let ty = if shape == "array-function-product" {
                // Depths 12 / 24 at 8 / 32 KiB. Both fit the nesting ceiling.
                format!(
                    "{}number",
                    "()=>".repeat(if bytes <= 8192 { 12 } else { 24 })
                )
            } else {
                let width = if bytes <= 16384 { 15 } else { 31 };
                let inner = format!(
                    "({})=>number",
                    (0..width)
                        .map(|i| format!("a{i}:number"))
                        .collect::<Vec<_>>()
                        .join(",")
                );
                format!(
                    "({})=>number",
                    (0..width)
                        .map(|i| format!("a{i}:{inner}"))
                        .collect::<Vec<_>>()
                        .join(",")
                )
            };
            (
                format!(
                    "export function main():void{{}}function f(xs:({ty})[]):void{{let k=xs[0];"
                ),
                "k=xs[0];".into(),
                "}",
            )
        }
        "array-large-literal" => (
            "export function main():void{const xs=[".into(),
            "1,".into(),
            "];}",
        ),
        "array-wide-computed" => (
            "export function main():void{let n=1;const xs:number[]=[".into(),
            "n,".into(),
            "];}",
        ),
        "array-wide-nested" => (
            "export function main():void{let n=1;const xs:number[][]=[".into(),
            "[n],".into(),
            "];}",
        ),
        "array-wide-computed-module" => (
            "let n=1;const xs:number[]=[".into(),
            "n,".into(),
            "];export function main():void{}",
        ),
        _ => return String::new(),
    };
    let count = bytes.saturating_sub(prefix.len() + suffix.len()) / unit.len();
    let mut out = format!("{prefix}{}{suffix}", unit.repeat(count));
    out.push_str(&" ".repeat(bytes.saturating_sub(out.len())));
    out
}

/// Review shapes combine fresh arrows, array products, and spilled bindings.
pub const SPILL_SHAPES: &[&str] = &["spillA", "spillD"];

/// Build one review shape at the source ceiling.
pub fn spill_source(shape: &str, bytes: usize) -> String {
    let mut source = (0..100)
        .map(|n| format!("function f{n}():void{{}}"))
        .collect::<String>();
    source.push_str("export function main():void{");
    let deep = shape == "spillD";
    if deep {
        for (n, base) in [
            "number",
            "boolean",
            "string",
            "(()=>number)",
            "(()=>boolean)",
            "(()=>string)",
        ]
        .iter()
        .enumerate()
        {
            source.push_str(&format!("let w{n}:{base}{}=[];", "[]".repeat(20)));
        }
    }
    for n in 0..200 {
        source.push_str(&format!("let v{n}=0;"));
    }
    let ty = format!("number{}", "[]".repeat(if deep { 21 } else { 1 }));
    let value = if deep { "[]" } else { "[1]" };
    source.push_str(&format!("const a:{ty}={value};let g=():{ty}[]=>[];"));
    let unit = format!("g=()=>[{}];", vec!["a"; 130].join(","));
    while source.len() + unit.len() < bytes {
        source.push_str(&unit);
    }
    source.push('}');
    source.push_str(&" ".repeat(bytes.saturating_sub(source.len())));
    source
}
