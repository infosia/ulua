//! P6.S14 source-ceiling witnesses shared by instruction and step-time probes.
use super::fill_at;

pub fn all(bytes: usize) -> Vec<(&'static str, String)> {
    let mut shapes = vec![];
    for (name, ty, value) in [
        ("update-number", "number", "1"),
        ("update-boolean", "boolean", "true"),
    ] {
        let fields = (0..16)
            .map(|i| format!("x{i}:{ty}={value};"))
            .collect::<String>();
        let updates = (0..16)
            .map(|i| format!("a.x{i}={value};"))
            .collect::<String>();
        shapes.push((
            name,
            fill_at(
                bytes,
                &format!(
                    "class A{{{fields}}}export function main(b:boolean):void{{const a=new A();"
                ),
                &updates,
                "}",
            ),
        ));
    }
    // Seven statements precede the header setup. Later body operations demand a moved
    // point, then enough work to demand a local point. Repetition reaches C15's ceiling.
    for (name, body) in [
        (
            "moved-point",
            "for(let i=0;i<2;i++){x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;}",
        ),
        (
            "moved-nested",
            "for(let i=0;i<2;i++){for(let j=0;j<2;j++){x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;}}",
        ),
        (
            "moved-break",
            "for(let i=0;i<2;i++){x=x+1;if(b){break;}x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;}",
        ),
        (
            "moved-continue",
            "for(let i=0;i<2;i++){x=x+1;if(b){continue;}x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;}",
        ),
        (
            "moved-condition",
            "let i=0;while(i<2){i++;x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;}",
        ),
    ] {
        let unit = format!(
            "{}{{{body}}}",
            "x=x+1;".repeat(if name == "moved-nested" { 2 } else { 7 })
        );
        shapes.push((
            name,
            fill_at(
                bytes,
                "export function main(b:boolean):void{let x=0;",
                &unit,
                "}",
            ),
        ));
    }
    for (name, prefix, unit) in [
        (
            "update-record",
            "export function main(b:boolean):void{const a={x:0};",
            "a.x=1;",
        ),
        (
            "update-compound",
            "export function main(b:boolean):void{const a={x:0};",
            "a.x+=1;",
        ),
        (
            "update-remainder",
            "export function main(b:boolean):void{const a={x:7};",
            "a.x%=3;",
        ),
        (
            "moved-reset-call",
            "function reset():void{}export function main(b:boolean):void{let x=0;",
            "x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;for(let i=0;i<2;i++){x=x+1;reset();x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;}",
        ),
        (
            "moved-reset-branch",
            "function reset():void{}export function main(b:boolean):void{let x=0;",
            "x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;for(let i=0;i<2;i++){if(b){reset();}x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;}",
        ),
        (
            "moved-reset-return",
            "export function main(b:boolean):void{let x=0;",
            "x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;for(let i=0;i<2;i++){if(b){return;}x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;}",
        ),
        (
            "moved-local-point",
            "export function main(b:boolean):void{let x=0;",
            "x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;for(let i=0;i<2;i++){x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;x=x+1;}",
        ),
    ] {
        shapes.push((name, fill_at(bytes, prefix, unit, "}")));
    }
    shapes
}

/// Captured block declarations must move their demanded point on zero-trip paths too.
pub fn headers(bytes: usize) -> Vec<(String, String)> {
    let mut shapes = vec![];
    for (nested, alignments) in [(false, 12..13), (true, 0..16)] {
        for before in alignments {
            let outer = if nested { "while(true){" } else { "" };
            let close = if nested { "break;}" } else { "" };
            let source = format!(
                "export function main(b:boolean):void{{let x=0;const n=b?0:2;{outer}{}for(let i=0;i<n;i++){{let c0=x;let c1=x;const f=():number=>c0+c1;x=x+f();}}{}{close}}}",
                "x=x+1;".repeat(before),
                "x=x+1;".repeat(20)
            );
            shapes.push((
                format!(
                    "moved-header-zero-trip-{}-{before}",
                    if nested { "nested" } else { "single" }
                ),
                fill_at(bytes, &source, " ", ""),
            ));
        }
    }
    shapes
}
