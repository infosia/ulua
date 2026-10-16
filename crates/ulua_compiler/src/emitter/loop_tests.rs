//! Small source fixtures pin loop copies and control flow in generated text.
//! Measured Debug cost: five compiler calls, 0.005 seconds; no native VM or external witness.
use crate::{Limits, compile};
use std::time::Instant;

// Cost: one small compiler call per case; no native VM or external witness.
fn check(captured: bool, continues: bool) {
    let start = Instant::now();
    // The first head binding stays uncaptured. This also detects a filtered zip mismatch.
    let capture = if captured {
        "const read = () => i;"
    } else {
        ""
    };
    let jump = if continues {
        "if (i === 1) { continue; }"
    } else {
        ""
    };
    let source = format!(
        "export function main():void{{for(let n=0,i=0;i<5;i++){{{capture}n++;i++;{jump}if(i===4){{break;}}}}}}"
    );
    let luau = compile(source.as_bytes(), Limits::default()).unwrap().luau;
    assert!(luau.contains("while true do\n"));
    assert_eq!(luau.contains("repeat\n"), continues, "{luau}");
    assert_eq!(luau.contains("until true\n"), continues, "{luau}");
    assert_eq!(
        luau.contains("local __ulua_g1_0: boolean = true"),
        continues,
        "{luau}"
    );
    assert_eq!(
        luau.contains("__ulua_g1_0 = (true) :: boolean"),
        continues,
        "{luau}"
    );
    assert_eq!(
        luau.contains("__ulua_g1_0 = (false) :: boolean"),
        continues,
        "{luau}"
    );
    assert_eq!(
        luau.contains("if not __ulua_g1_0 then break end"),
        continues,
        "{luau}"
    );
    let numeric_copies: Vec<_> = luau
        .lines()
        .filter(|line| {
            line.trim_start().starts_with("local __ulua_g1_") && line.contains(": number =")
        })
        .collect();
    assert_eq!(numeric_copies.len(), usize::from(captured), "{luau}");
    assert_eq!(
        luau.contains("local __ulua_g1_"),
        captured || continues,
        "{luau}"
    );
    if captured {
        let copy = if continues {
            "__ulua_g1_1"
        } else {
            "__ulua_g1_0"
        };
        assert!(
            luau.contains(&format!("local {copy}: number = (__ulua_b2) :: number")),
            "{luau}"
        );
        let copy_back = luau
            .find(&format!("__ulua_b2 = ({copy}) :: number"))
            .unwrap();
        let update = luau.rfind("__ulua_b2 = (").unwrap();
        assert!(copy_back < update, "{luau}");
        if continues {
            assert!(luau.find("until true").unwrap() < copy_back, "{luau}");
            assert!(
                luau.find("if not __ulua_g1_0 then break end").unwrap() < copy_back,
                "{luau}"
            );
        }
    } else {
        assert!(!luau.contains("__ulua_b2 = (__ulua_g"), "{luau}");
    }
    eprintln!(
        "loop form captured={captured} continue={continues} cost={:.6}s",
        start.elapsed().as_secs_f64()
    );
}

#[test]
fn uncaptured_plain_while() {
    check(false, false);
}

#[test]
fn uncaptured_continue_tail() {
    check(false, true);
}

#[test]
fn captured_plain_while() {
    check(true, false);
}

#[test]
fn captured_continue_tail() {
    check(true, true);
}

// Cost: one small compiler call; the inner while owns the continue.
#[test]
fn nested_continue_keeps_the_outer_for_plain() {
    let start = Instant::now();
    let source = "export function main():void{for(let i=0;i<3;i++){while(true){continue;}}}";
    let luau = compile(source.as_bytes(), Limits::default()).unwrap().luau;
    assert_eq!(luau.matches("while true do\n").count(), 2, "{luau}");
    assert!(luau.contains("do continue end"), "{luau}");
    assert!(!luau.contains("repeat\n"), "{luau}");
    assert!(!luau.contains("local __ulua_g1_"), "{luau}");
    eprintln!(
        "nested continue form cost={:.6}s",
        start.elapsed().as_secs_f64()
    );
}
