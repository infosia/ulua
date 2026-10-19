//! P6.S13 form witnesses; each fixture builds the allowed or forbidden IR form.
use crate::{Limits, compile};

fn code(source: &str) -> String {
    compile(source.as_bytes(), Limits::default())
        .unwrap_or_else(|d| panic!("{source}\n{d:?}"))
        .luau
}
fn local(body: &str) -> String {
    code(&format!("export function main():void{{{body}}}"))
}
fn copied(code: &str, id: usize) -> bool {
    code.lines().any(|l| {
        l.trim_start().starts_with("__ulua_t") && l.contains(&format!("= (__ulua_b{id}) ::"))
    })
}
macro_rules! receiver {
    ($name:ident, $body:expr) => {
        #[test]
        fn $name() {
            let luau = local($body);
            assert!(!copied(&luau, 1), "{luau}");
            assert!(
                luau.contains("__ulua_b1.__ulua_k") || luau.contains("__ulua_b1["),
                "{luau}"
            );
        }
    };
}
receiver!(receiver_field_read, "const a={x:1};const x=a.x;");
receiver!(receiver_field_write, "const a={x:1};a.x=2;");
receiver!(receiver_field_compound, "const a={x:1};a.x+=2;");
receiver!(receiver_element_read, "const a=[1];const x=a[0];");
receiver!(receiver_element_write, "const a=[1];a[0]=2;");
receiver!(receiver_element_compound, "const a=[1];a[0]+=2;");
receiver!(
    receiver_uncaptured_call,
    "const a={x:1};const f=():number=>2;a.x+=f();"
);

#[test]
fn receiver_shared_without_call() {
    for (decl, access, suffix) in [
        ("const a={x:1};", "const x=a.x;", ".__ulua_k"),
        ("const a={x:1};", "a.x=2;", ".__ulua_k"),
        ("const a={x:1};", "a.x+=2;", ".__ulua_k"),
        ("const a=[1];", "const x=a[0];", "["),
        ("const a=[1];", "a[0]=2;", "["),
        ("const a=[1];", "a[0]+=2;", "["),
    ] {
        for source in [
            format!("{decl}export function main():void{{{access}}}"),
            format!("export function main():void{{{decl}const f=()=>a;{access}}}"),
        ] {
            let luau = code(&source);
            let id = usize::from(source.starts_with("export"));
            assert!(!copied(&luau, id), "{luau}");
            assert!(luau.contains(&format!("__ulua_b{id}{suffix}")), "{luau}");
        }
    }
}
#[test]
fn receiver_shared_with_call_keeps_snapshot() {
    for access in ["const x=a[f()]", "a[0]=f()", "a[0]+=f()"] {
        for source in [
            format!(
                "const a=[1];function f():number{{return 0;}}export function main():void{{{access};}}"
            ),
            format!(
                "export function main():void{{const a=[1];const f=():number=>{{const b=a;return 0;}};{access};}}"
            ),
        ] {
            let luau = code(&source);
            let id = usize::from(source.starts_with("export"));
            assert!(copied(&luau, id), "{luau}");
        }
    }
}
#[test]
fn receiver_spilled_keeps_snapshot() {
    let vars = (0..180)
        .map(|i| format!("const v{i}=0;"))
        .collect::<String>();
    let luau = local(&format!("{vars}const a={{x:1}};const x=a.x;"));
    assert!(
        luau.lines().any(|l| l.trim_start().starts_with("__ulua_t")
            && l.contains("= (__ulua_spill")
            && l.contains(".__ulua_b181)")),
        "{luau}"
    );
}
fn check_length(prefix: &str, body: &str, hoisted: bool) {
    let luau = local(&format!("{prefix}for(let i=0;i<2;i++){{{body}}}"));
    let head = luau.find("for __ulua_g").expect("counted loop");
    let reads: Vec<_> = luau
        .match_indices("number = #")
        .filter(|(i, _)| {
            luau[..*i]
                .lines()
                .last()
                .unwrap()
                .trim_start()
                .starts_with("local __ulua_g")
        })
        .collect();
    if hoisted {
        assert_eq!(reads.len(), 1, "{luau}");
        assert!(reads[0].0 < head, "{luau}");
        assert!(!luau[head..].contains(" < #"), "{luau}");
    } else {
        assert!(reads.is_empty(), "{luau}");
        assert!(luau[head..].contains(" < #"), "{luau}");
    }
}
#[test]
fn length_field_writes_and_builtin_aliases() {
    check_length(
        "const a=[1,2];const p={x:0};",
        "p.x+=Math.abs(a[i]);p.x=Math.sqrt(a[i]);",
        true,
    );
}
#[test]
fn length_binding_write_blocks() {
    check_length("let a=[1,2];", "const x=a[i];a=[2,3];", false);
}
#[test]
fn length_binding_declaration_blocks() {
    check_length("", "const a=[1,2];const x=a[i];", false);
}
#[test]
fn length_call_blocks() {
    check_length("const a=[1,2];const f=()=>0;", "const x=a[i];f();", false);
}
#[test]
fn length_non_alias_math_call_blocks() {
    check_length("const a=[1,2];", "const x=a[i];Math.round(1.5);", false);
}
#[test]
fn length_new_blocks() {
    let luau = code(
        "class C{x:number=0;}export function main():void{const a=[1,2];for(let i=0;i<2;i++){const x=a[i];const c=new C();}}",
    );
    assert!(
        !luau
            .lines()
            .any(|l| l.trim_start().starts_with("local __ulua_g") && l.contains("number = #")),
        "{luau}"
    );
    assert!(luau.contains(" < #"), "{luau}");
}
#[test]
fn length_element_write_blocks() {
    check_length("const a=[1,2];const b=[0];", "const x=a[i];b[0]=1;", false);
}
#[test]
fn length_push_blocks() {
    check_length(
        "const a=[1,2];const b=[0];",
        "const x=a[i];b.push(1);",
        false,
    );
}
#[test]
fn length_nested_outermost() {
    let luau = local("const a=[1,2];for(let j=0;j<2;j++){for(let i=0;i<2;i++){const x=a[i];}}");
    assert_eq!(
        luau.lines()
            .filter(|l| l.trim_start().starts_with("local __ulua_g") && l.contains("number = #"))
            .count(),
        1,
        "{luau}"
    );
    assert!(
        luau.find("number = #").unwrap() < luau.find("for __ulua_g").unwrap(),
        "{luau}"
    );
}
#[test]
fn length_outer_body_declaration() {
    let luau = local("for(let j=0;j<2;j++){const a=[1,2];for(let i=0;i<2;i++){const x=a[i];}}");
    let heads: Vec<_> = luau.match_indices("for __ulua_g").map(|(i, _)| i).collect();
    assert_eq!(
        luau.lines()
            .filter(|l| l.trim_start().starts_with("local __ulua_g") && l.contains("number = #"))
            .count(),
        1,
        "{luau}"
    );
    let length = luau.find("number = #").unwrap();
    assert!(heads[0] < length && length < heads[1], "{luau}");
}
#[test]
fn length_full_local_budget() {
    let vars = (0..180)
        .map(|i| format!("const v{i}=0;"))
        .collect::<String>();
    check_length(&format!("const a=[1,2];{vars}"), "const x=a[i];", false);
}

#[test]
fn receiver_shared_field_rhs_call_keeps_snapshot() {
    for op in ["=", "+="] {
        for source in [
            format!(
                "const a={{x:1}};function f():number{{return 2;}}export function main():void{{a.x{op}f();}}"
            ),
            format!(
                "export function main():void{{const a={{x:1}};const f=():number=>{{const b=a;return 2;}};a.x{op}f();}}"
            ),
        ] {
            let luau = code(&source);
            let id = usize::from(source.starts_with("export"));
            assert!(copied(&luau, id), "{luau}");
        }
    }
}
#[test]
fn length_new_error_blocks() {
    check_length(
        "const a=[1,2];",
        "const x=a[i];const e=new Error('x');",
        false,
    );
}
#[test]
fn length_array_method_blocks() {
    check_length("const a=[1,2];", "const x=a[i];const b=a.slice();", false);
}
#[test]
fn length_nullable_binding_keeps_guarded_body_read() {
    check_length(
        "let a:number[]|null=null;",
        "if(a!==null){const x=a[i];}",
        false,
    );
}
#[test]
fn length_field_receiver_is_not_a_binding() {
    check_length("const p={a:[1,2]};", "const x=p.a[i];", false);
}
#[test]
fn length_checked_and_integral_forms_use_the_local() {
    for index in ["0.5", "-1"] {
        check_length("const a=[1,2];", &format!("const x=a[{index}];"), true);
    }
}
#[test]
fn length_nested_loop_effect_blocks_outer() {
    check_length(
        "const a=[1,2];",
        "for(let j=0;j<2;j++){a.push(1);}const x=a[i];",
        false,
    );
}
#[test]
fn length_empty_loop_still_reads_length_before_head() {
    let luau = local("const a:number[]=[];for(let i=0;i<0;i++){const x=a[i];}");
    assert!(
        luau.find("number = #").unwrap() < luau.find("for __ulua_g").unwrap(),
        "{luau}"
    );
}
