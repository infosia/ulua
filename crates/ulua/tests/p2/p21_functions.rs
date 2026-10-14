//! P2.1 budgets and closure lifetimes through the ABI. Cost: below one second.
#[allow(dead_code)]
#[path = "../../src/p21_inputs.rs"]
mod inputs;
#[allow(dead_code)]
#[path = "../support/script.rs"]
mod support;
use std::{
    ffi::{CStr, c_char},
    ptr,
};
use support::Script;
use ulua::*;
use ulua_compiler::UPVALUE_BUDGET;

fn result(source: &str) -> f64 {
    let mut script = Script::new(source);
    assert_eq!(
        script.instance(RunLimits::default()),
        (ULUA_OK, String::new())
    );
    let (status, code, value) = script.call(script.index("score"), &[]);
    assert_eq!((status, code), (ULUA_OK, String::new()));
    assert_eq!(value.r#type, ULUA_TYPE_NUMBER);
    if let Ok(directory) = std::env::var("ULUA_P21_OUTPUT_DIR") {
        use std::hash::{Hash, Hasher};
        let mut hash = std::collections::hash_map::DefaultHasher::new();
        source.hash(&mut hash);
        let name = format!("fixture-{}", hash.finish());
        let directory = std::path::Path::new(&directory);
        std::fs::create_dir_all(directory).unwrap();
        let source = source.replace(
            "export function main():void{}",
            "export function main():void{print(`${score()}`);}",
        );
        let compiled =
            ulua_compiler::compile(source.as_bytes(), ulua_compiler::Limits::default()).unwrap();
        std::fs::write(directory.join(format!("{name}.ts")), source).unwrap();
        std::fs::write(directory.join(format!("{name}.luau")), compiled.luau).unwrap();
        std::fs::write(
            directory.join(format!("{name}.golden")),
            format!("{}\n", value.number),
        )
        .unwrap();
    }
    value.number
}
#[test]
fn bound_and_one_past_include_captures_from_multiple_enclosing_functions() {
    // The long addition body captures the interrupt helper, besides the source bindings.
    let bindings = UPVALUE_BUDGET - 1;
    assert_eq!(
        result(&inputs::upvalues(bindings)),
        (bindings * (bindings - 1) / 2) as f64
    );
    Script::compile(
        &inputs::upvalues(bindings + 1),
        CompileLimits::default(),
        "limit:upvalues",
    );
}
#[test]
fn three_nested_closures_read_local_and_spilled_bindings_of_each_owner() {
    assert_eq!(result(&inputs::deep_spills()), 657.0);
}
#[test]
fn for_and_while_body_bindings_have_per_iteration_lifetimes() {
    // Node gives 201 for each source. The for case includes a continue path.
    for spill in [false, true] {
        assert_eq!(result(&inputs::loop_closures(spill)), 201.0);
    }
}
#[test]
fn calls_freeze_function_values_before_arguments_and_allow_mutual_hoisting() {
    let source = "export function score():number{let f=():number=>1;function change():number{f=():number=>2;return 0;}function call(g:()=>number,n:number):number{return g();}const a=call(f,change());const b=even(6);function even(n:number):number{if(n<=0){return 1;}return odd(n-1);}function odd(n:number):number{if(n<=0){return 0;}return even(n-1);}return a*10+b;}export function main():void{}";
    assert_eq!(result(source), 11.0);
}

// SAFETY: These declarations match the private compiler shim linked by ulua.
unsafe extern "C" {
    fn ulua_internal_shim_compile(
        source: *const c_char,
        length: usize,
        out: *mut *mut c_char,
        size: *mut usize,
        error: *mut c_char,
    ) -> i32;
    fn ulua_internal_shim_bytecode_free(out: *mut c_char);
}
#[test]
fn pinned_compiler_accepts_two_hundred_upvalues_and_rejects_two_hundred_one() {
    assert_eq!(ulua_abi_version(), 1);
    for count in [200, 201] {
        let mut source =
            String::from("--!strict\ntype F = () -> number\nlocal outer=function():F\n");
        for i in 0..100 {
            source.push_str(&format!(
                "local x{i}:number=({i})::number\nx{i}=(x{i}+1)::number\n"
            ));
        }
        source.push_str("local middle=function():F\n");
        for i in 100..count {
            source.push_str(&format!(
                "local x{i}:number=({i})::number\nx{i}=(x{i}+1)::number\n"
            ));
        }
        source.push_str("return (function():number\nlocal total:number=(0)::number\n");
        for i in 0..count {
            source.push_str(&format!("total=(total+x{i})::number\n"));
        }
        source.push_str("return total\nend)::F\nend\nreturn middle()\nend\nreturn outer\n");
        let mut out = ptr::null_mut();
        let mut size = 0;
        let mut error = [0; 256];
        // SAFETY: The source and outputs cover their lengths. The shim catches errors and owns the output until free.
        unsafe {
            let status = ulua_internal_shim_compile(
                source.as_ptr().cast(),
                source.len(),
                &mut out,
                &mut size,
                error.as_mut_ptr(),
            );
            ulua_internal_shim_bytecode_free(out);
            if count == 200 {
                assert_eq!(status, 0);
            } else {
                assert_ne!(status, 0);
                assert!(
                    CStr::from_ptr(error.as_ptr())
                        .to_string_lossy()
                        .contains("exceeded limit 200")
                );
            }
        }
    }
}

#[test]
fn upvalue_budget_includes_generated_library_captures() {
    // The closure captures the remainder, Infinity, and interrupt helpers.
    let bindings = UPVALUE_BUDGET - 3;
    let source = inputs::upvalues(bindings).replace("return total;};", "return total%Infinity;};");
    Script::new(&source);
    let source =
        inputs::upvalues(bindings + 1).replace("return total;};", "return total%Infinity;};");
    Script::compile(&source, CompileLimits::default(), "limit:upvalues");
}

#[test]
fn escaped_closures_can_write_the_binding_of_their_own_iteration() {
    let source = "export function score():number{let a=():void=>{};let b=():void=>{};let first=()=>0;let last=()=>0;for(let i=0;i<2;i++){function set():void{i+=10;}function get():number{return i;}if(i===0){a=set;first=get;}else{b=set;last=get;}}a();b();return first()*100+last();}export function main():void{}";
    assert_eq!(result(source), 1011.0);
}

#[test]
fn spilled_captures_share_writes_and_keep_iteration_lifetimes() {
    let start = std::time::Instant::now();
    for count in [
        ulua_compiler::CAPTURE_LOCALS,
        ulua_compiler::CAPTURE_LOCALS + 1,
        130,
        190,
    ] {
        let declarations = (0..count)
            .map(|i| format!("let v{i}={i};"))
            .collect::<String>();
        let writes = (0..count).map(|i| format!("v{i}+=1;")).collect::<String>();
        let reads = (0..count).map(|i| format!("s+=v{i};")).collect::<String>();
        let source = format!(
            "export function score():number{{{declarations}const set=():void=>{{{writes}}};const get=():number=>{{let s=0;{reads}return s;}};set();return get();}}export function main():void{{}}"
        );
        assert_eq!(result(&source), (count * (count + 1) / 2) as f64);
    }
    let count = ulua_compiler::CAPTURE_LOCALS + 1;
    let head = (0..count)
        .map(|i| format!("v{i}=0"))
        .collect::<Vec<_>>()
        .join(",");
    let writes = (0..count).map(|i| format!("v{i}+=10;")).collect::<String>();
    let reads = (0..count).map(|i| format!("s+=v{i};")).collect::<String>();
    let source = format!(
        "export function score():number{{let a=()=>0;let b=()=>0;for(let {head};v0<2;v0++){{const get=():number=>{{{writes}let s=0;{reads}return s;}};if(v0===0){{a=get;continue;}}else{{b=get;}}}}return a()*1000+b()+a();}}export function main():void{{}}"
    );
    assert_eq!(
        result(&source),
        (count * 10 * 1000 + count * 10 + 1 + count * 20) as f64
    );
    eprintln!(
        "P5.5 captured values shapes=5 cost={:.3}s",
        start.elapsed().as_secs_f64()
    );
}

#[test]
fn captured_parameters_spill_with_defaults_and_body_bindings() {
    let count = ulua_compiler::CAPTURE_LOCALS + 2;
    for defaults in [false, true] {
        let params = (0..count)
            .map(|i| format!("p{i}:number{}", if defaults { "=1" } else { "" }))
            .collect::<Vec<_>>()
            .join(",");
        let args = if defaults {
            String::new()
        } else {
            vec!["1"; count].join(",")
        };
        let writes = (0..count)
            .map(|i| format!("p{i}+=1;s+=p{i};"))
            .collect::<String>();
        let source = format!(
            "function work({params}):number{{let v=3;const g=():number=>{{let s=0;{writes}v+=1;return s+v;}};return g()+g();}}export function score():number{{return work({args});}}export function main():void{{}}"
        );
        assert_eq!(result(&source), (5 * count + 9) as f64);
    }
}
