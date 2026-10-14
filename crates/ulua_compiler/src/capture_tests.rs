//! Source counts independently constrain the emitted local declarations.
use crate::{CAPTURE_LOCALS, Limits, compile};

#[test]
fn captured_bindings_spill_at_the_capture_limit() {
    for count in [CAPTURE_LOCALS, CAPTURE_LOCALS + 1, 130, 190] {
        for blocks in [false, true] {
            if blocks && count > CAPTURE_LOCALS + 1 {
                continue;
            }
            let mut source = String::from("export function score():number{");
            for i in 0..count {
                if blocks {
                    source.push('{');
                }
                source.push_str(&format!("const v{i}={i};"));
            }
            source.push_str("function get():number{let s=0;");
            for i in 0..count {
                source.push_str(&format!("s+=v{i};"));
            }
            source.push_str("return s;}return get();");
            if blocks {
                source.push_str(&"}".repeat(count));
            }
            source.push_str("}export function main():void{}");
            let compiled = compile(source.as_bytes(), Limits::default()).unwrap();
            let numeric_locals = compiled
                .luau
                .lines()
                .filter(|line| {
                    line.trim_start().starts_with("local __ulua_b") && line.contains(": number =")
                })
                .count();
            // The closure's sum is the only uncaptured numeric binding in this source.
            assert_eq!(
                numeric_locals,
                count.min(CAPTURE_LOCALS) + 1,
                "N={count} blocks={blocks}"
            );
            if count > CAPTURE_LOCALS {
                assert!(compiled.luau.contains("local __ulua_spill"));
            }
        }
    }
}
