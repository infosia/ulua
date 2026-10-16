//! Each case costs one small compiler call. No VM or external witness runs here.
use crate::{Limits, compile};
use std::time::Instant;

// Cost: 31 small compiler calls span the storage transition that can disable numeric lowering.
#[test]
fn counted_storage_reservations_converge_at_the_spill_boundary() {
    let start = Instant::now();
    for count in 150..=180 {
        let declarations = (0..count)
            .map(|i| format!("let v{i}=3;"))
            .collect::<String>();
        let luau = code(&format!(
            "{declarations}for(let i=0;i<v{};i++){{}}",
            count - 1
        ));
        assert!(luau.contains("for __ulua_g") || luau.contains("while true do"));
    }
    eprintln!(
        "counted spill transition cases=31 cost={:.6}s",
        start.elapsed().as_secs_f64()
    );
}

fn code(body: &str) -> String {
    compile(
        format!("export function main(b:number):void{{{body}}}").as_bytes(),
        Limits::default(),
    )
    .unwrap_or_else(|d| panic!("{body}: {d:?}"))
    .luau
}

#[test]
fn counted_forms_select_the_limit_and_step_from_ir() {
    let start = Instant::now();
    for (head, rounding, adjustment, step) in [
        ("let i=0;i<b;i++", "ceil", " - 1", "1.0"),
        ("let i=-2;i<=b;i+=3", "floor", "", "3.0"),
        ("let i=10;i>b;i--", "floor", " + 1", "-1.0"),
        ("let i=10;i>=b;i-=3", "ceil", "", "-3.0"),
        ("let i=0;i<3.5;i++", "ceil", " - 1", "1.0"),
        ("let i=0;i>=-3.5;i--", "ceil", "", "-1.0"),
    ] {
        let luau = code(&format!("for({head}){{const v=i;}}"));
        assert!(!luau.contains("while true do"), "{head}\n{luau}");
        assert!(luau.contains(&format!("__ulua_{rounding}(")), "{luau}");
        let limit = luau
            .lines()
            .find(|s| s.contains("local __ulua_g1_0:"))
            .unwrap();
        assert!(limit.ends_with(&format!("){adjustment}")), "{limit}");
        assert!(
            luau.contains(&format!(", __ulua_g1_0, {step} do")),
            "{luau}"
        );
        assert!(luau.contains("= (__ulua_g1_1) :: number"), "{luau}");
    }
    eprintln!(
        "counted positive forms cost={:.6}s",
        start.elapsed().as_secs_f64()
    );
}

#[test]
fn every_failed_counted_precondition_retains_while() {
    let start = Instant::now();
    let cases = [
        "let i=0;for(;i<b;i++){}",
        "for(let i=0,j=0;i<b;i++){}",
        "for(let i=0.5;i<b;i++){}",
        "for(let i=b;i<10;i++){}",
        "for(let i=0+0;i<b;i++){}",
        "for(let i=0;i<b;){}",
        "for(let i=0;;i++){}",
        "for(let i=0;i<b;i=1){}",
        "for(let i=0;i<b;i*=2){}",
        "for(let i=0;i<b;i+=0){}",
        "for(let i=0;i<b;i+=0.5){}",
        "for(let i=0;i<b;i+=-1){}",
        "const k=1;for(let i=0;i<b;i+=k){}",
        "let j=0;for(let i=0;i<b;j++){}",
        "for(let i=0;i<b;i--){}",
        "for(let i=0;i>=b;i++){}",
        "for(let i=0;i===b;i++){}",
        "for(let i=0;b>i;i++){}",
        "for(let i=0;i<i;i++){}",
        "for(let i=0;i<b+1;i++){}",
        "const a=[3];for(let i=0;i<a[0];i++){}",
        "const f=()=>b;for(let i=0;i<f();i++){}",
        "for(let i=0;i<b;i++){b--;}",
        "for(let i=0;i<b;i++){if(i===1){b=0;}}",
        "for(let i=0;i<b;i++){while(false){b--;}}",
        "for(let i=0;i<b;i++){for(let j=0;j<1;j++){b--;}}",
        "const get=()=>b;for(let i=0;i<b;i++){}",
        "for(let i=0;i<b;i++){i++;}",
        "for(let i=0;i<b;i++){if(i===1){i=2;}}",
        "for(let i=0;i<b;i++){const read=()=>i;}",
        "for(let i=0;i<b;i++){continue;}",
        "for(let i=0;i<b;i++){try{}finally{b--;}}",
    ];
    for body in cases {
        let luau = code(body);
        assert!(luau.contains("while true do"), "{body}\n{luau}");
        // A nested eligible loop is independent of the outer loop's failed precondition.
        if !body.contains("let j=0;j<1") {
            assert!(!luau.contains("for __ulua_g"), "{body}\n{luau}");
        }
    }
    let declarations = (0..200).map(|i| format!("let v{i}=3;")).collect::<String>();
    let luau = code(&format!("{declarations}for(let i=0;i<v199;i++){{}}"));
    assert!(
        luau.contains("while true do"),
        "a spilled bound must retain while"
    );
    assert!(!luau.contains("for __ulua_g"), "{luau}");
    eprintln!(
        "counted fallback forms cases={} cost={:.6}s",
        cases.len() + 1,
        start.elapsed().as_secs_f64()
    );
}
