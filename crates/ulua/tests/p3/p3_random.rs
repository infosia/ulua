//! Fixed-seed host programs compare all fixtures and call positions with three witnesses.
//! This test reports ABI, Node, tsc, and analyzer counts. Measured Debug cost: 7.494 seconds.
// External ABI descriptors are non-exhaustive and require field assignments.
#![allow(clippy::field_reassign_with_default)]
#[path = "../support/analyzer.rs"]
mod analyzer;
#[path = "../support/p3_declaration_text.rs"]
mod declarations;
#[allow(dead_code)]
#[path = "../../../ulua_corpus/src/fixtures.rs"]
mod fixtures;

use std::{
    ffi::{c_char, c_void},
    path::PathBuf,
    process::Command,
    ptr,
    time::Instant,
};
use ulua::*;
unsafe extern "C" fn output(data: *mut c_void, text: *const c_char, length: usize) -> i32 {
    // SAFETY: The fixture owns the output vector and the runtime supplies readable UTF-8 text.
    unsafe {
        let bytes = std::slice::from_raw_parts(text.cast(), length);
        let Ok(text) = std::str::from_utf8(bytes) else {
            return 1;
        };
        (*data.cast::<Vec<String>>()).push(text.into());
    }
    0
}
fn execute(source: &str, signatures: &[ulua_compiler::HostSignature]) -> Vec<String> {
    let mut f = fixtures::Fixtures::new();
    let mut c = ptr::null_mut();
    let mut m = ptr::null_mut();
    let mut i = ptr::null_mut();
    let mut d = Diagnostic::default();
    // SAFETY: The fixture owns every descriptor and object; callback userdata remains stable through the run.
    unsafe {
        assert_eq!(ulua_context_new(&mut c), ULUA_OK);
        let grants = f.grants(c, signatures).unwrap();
        assert_eq!(
            ulua_compile(
                c,
                source.as_ptr().cast(),
                source.len(),
                grants.as_ptr(),
                grants.len(),
                ptr::null(),
                &mut m,
                &mut d
            ),
            ULUA_OK
        );
        let mut host = Host::default();
        host.userdata = ptr::from_mut(&mut f.output).cast();
        host.output = Some(output);
        assert_eq!(
            ulua_instance_new(c, m, &host, ptr::null(), &mut i, &mut d),
            ULUA_OK
        );
        let mut index = 0;
        assert_eq!(
            ulua_module_find_export(m, c"main".as_ptr(), 4, &mut index),
            ULUA_OK
        );
        assert_eq!(
            ulua_call(i, index, ptr::null(), 0, &mut Value::default(), &mut d),
            ULUA_OK
        );
        assert_eq!(ulua_context_free(c), ULUA_OK);
    }
    f.output
}
#[test]
fn random_hosts_match_node_tsc_and_strict_analysis() {
    let now = Instant::now();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let dir = std::env::temp_dir().join(format!("ulua-p3-random-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let mut seed = 0x31ab_921d_7291_u64;
    let mut paths = vec![];
    for index in 0..192 {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        let a = seed % 41;
        let b = (seed >> 8) % 13;
        let flag = if seed & 1 == 0 { "true" } else { "false" };
        let text = ["é", "🙂", "", "x\\0y"][(seed as usize >> 9) % 4];
        let source = format!(
            r#"// grants: hostAdd, hostConcat, hostNot, hostLog
let n:number={a};
let base:number=hostAdd({a},{b});
let word:string=hostConcat('{text}','{index}');
let flag:boolean=hostNot({flag});
function next():number{{n=hostAdd(n,1);hostLog(`${{n}}`);return n;}}
function numberDefault(x:number=hostAdd(next(),next())):number{{return hostAdd(x,{b});}}
function stringDefault(s:string=hostConcat(word,'!')):string{{return hostConcat(s,'?');}}
function booleanDefault(b:boolean=hostNot(flag)):boolean{{return hostNot(b);}}
class Box{{n:number=hostAdd(base,1);s:string=hostConcat(word,'c');b:boolean=hostNot(flag);
method():string{{hostLog(this.s);this.n=hostAdd(this.n,2);this.b=hostNot(this.b);return hostConcat(this.s,`${{this.n}} ${{this.b}}`);}}}}
export function main():void{{
const numberArrow=(x:number):number=>hostAdd(x,{b});
const stringArrow=(s:string):string=>hostConcat(s,word);
const booleanArrow=(b:boolean):boolean=>hostNot(b);
const voidArrow=(s:string):void=>{{hostLog(s);}};
print(`${{hostAdd(next(),next())}}`);print(`${{numberDefault()}}`);print(stringDefault());print(`${{booleanDefault()}}`);
const object=new Box();print(object.method());
print(`${{numberArrow(base)}}`);print(stringArrow(word));print(`${{booleanArrow(flag)}}`);voidArrow(word);
const xs:number[]=[1,2,3];print(xs.map((x:number):number=>hostAdd(x,{a})).join(','));
if(hostNot(hostNot(flag))){{hostLog('yes');}}else{{hostLog('no');}}
try{{hostLog(hostConcat(word,'try'));print(`${{hostAdd(next(),next())}}`);throw new Error(word);}}
catch(e){{if(e instanceof Error){{hostLog(hostConcat(e.message,'catch'));print(`${{hostNot(flag)}}`);}}}}
finally{{hostLog(hostConcat(word,'finally'));print(`${{hostAdd(next(),next())}}`);}}
print(`${{n}}`);
}}"#
        );
        let grants = fixtures::Fixtures::signatures(source.as_bytes()).unwrap();
        let compiled = ulua_compiler::compile_with_grants(
            source.as_bytes(),
            ulua_compiler::Limits::default(),
            &grants,
        )
        .unwrap();
        let actual = execute(&source, &grants);
        let path = dir.join(format!("case-{index}.ts"));
        std::fs::write(&path, &source).unwrap();
        std::fs::write(path.with_extension("luau"), compiled.luau).unwrap();
        std::fs::write(path.with_extension("expected"), actual.join("\n")).unwrap();
        paths.push(path);
    }
    let mut declarations_context = ptr::null_mut();
    let mut f = fixtures::Fixtures::new();
    let signatures =
        fixtures::Fixtures::signatures(b"// grants: hostAdd, hostConcat, hostNot, hostLog")
            .unwrap();
    // SAFETY: This temporary context and fixture own all declaration grant storage.
    unsafe {
        assert_eq!(ulua_context_new(&mut declarations_context), ULUA_OK);
        let grants = f.grants(declarations_context, &signatures).unwrap();
        std::fs::write(dir.join("host.d.ts"), declarations::text(&grants).unwrap()).unwrap();
        assert_eq!(ulua_context_free(declarations_context), ULUA_OK);
    }
    let fixtures_cost = now.elapsed().as_secs_f64();
    let runner = dir.join("run.mjs");
    std::fs::write(&runner, include_str!("../support/p3_node.mjs")).unwrap();
    let node = Command::new("node")
        .arg(&runner)
        .arg(&root)
        .arg(&dir)
        .output()
        .unwrap();
    assert!(
        node.status.success(),
        "{}{}",
        String::from_utf8_lossy(&node.stdout),
        String::from_utf8_lossy(&node.stderr)
    );
    eprint!("{}", String::from_utf8_lossy(&node.stdout));
    for batch in paths.chunks(48) {
        let tsc = Command::new("node")
            .current_dir(&root)
            .args([
                "node_modules/typescript/bin/tsc",
                "--noEmit",
                "--strict",
                "--target",
                "es2022",
                "--module",
                "esnext",
                "--lib",
                "es2022",
                "prelude/ulua.d.ts",
            ])
            .arg(dir.join("host.d.ts"))
            .args(batch)
            .output()
            .unwrap();
        assert!(
            tsc.status.success(),
            "{}{}",
            String::from_utf8_lossy(&tsc.stdout),
            String::from_utf8_lossy(&tsc.stderr)
        );
    }
    let analysis = analyzer::command(root.join("target/luau-tools/luau-analyze"))
        .args(paths.iter().map(|p| p.with_extension("luau")))
        .output()
        .unwrap();
    assert!(
        analysis.status.success(),
        "{}",
        analyzer::failure(&analysis)
    );
    std::fs::remove_dir_all(dir).unwrap();
    eprintln!(
        "P3 random ABI=192 node=192 tsc=192 analyzer=192 fixtures=4 ABI_cost={fixtures_cost:.3}s total_cost={:.3}s",
        now.elapsed().as_secs_f64()
    );
}
