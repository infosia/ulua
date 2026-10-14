//! Batch external witnesses keep the direct method test cost bounded.
//! Measured single-worker Debug cost: 11.27 seconds for p24_strings.
#[path = "analyzer.rs"]
mod analyzer;
#[path = "tsc.rs"]
mod tsc;
use super::support::Script;
use std::{
    ffi::{c_char, c_void},
    path::PathBuf,
    process::Command,
    time::Instant,
};
use ulua::*;

pub struct Suite {
    pub directory: PathBuf,
    pub count: usize,
    pub comparable: usize,
    pub traps: usize,
    pub rejects: usize,
    start: Instant,
}
impl Suite {
    pub fn new() -> Self {
        let directory = std::env::temp_dir().join(format!("ulua-p24-{}", std::process::id()));
        std::fs::create_dir_all(&directory).unwrap();
        Self {
            directory,
            count: 0,
            comparable: 0,
            traps: 0,
            rejects: 0,
            start: Instant::now(),
        }
    }
    pub fn case(&mut self, body: &str) {
        let before = self.count;
        self.source(
            &format!(
                "function score():string{{{body}}}export function main():void{{print(score());}}"
            ),
            None,
        );
        assert_eq!(
            self.count,
            before + 1,
            "Direct fixture must compile: {body}"
        );
    }
    pub fn trap(&mut self, body: &str, kind: &str) {
        self.source(&format!("function score():string{{{body}return 'unreachable';}}export function main():void{{print(score());}}"), Some(kind));
    }
    pub fn bytes(&mut self, body: &str, expected: &str) {
        self.case(body);
        let path = self.directory.join(format!("case-{}", self.count - 1));
        let actual = std::fs::read_to_string(path.with_extension("expected")).unwrap();
        assert_eq!(actual, format!("{expected}\n"), "{body}");
        std::fs::rename(path.with_extension("expected"), path.with_extension("byte")).unwrap();
        self.comparable -= 1;
    }
    pub fn source(&mut self, source: &str, trap: Option<&str>) {
        let compiled =
            match ulua_compiler::compile(source.as_bytes(), ulua_compiler::Limits::default()) {
                Ok(c) => c,
                Err(d) => {
                    assert!(trap.is_none(), "{source}: {d:?}");
                    assert!(
                        matches!(d.code, "C5" | "C4" | "C1" | "C11"),
                        "{source}: {d:?}"
                    );
                    self.rejects += 1;
                    return;
                }
            };
        let mut script = Script::new(source);
        let mut lines = Vec::<String>::new();
        let mut host = Host::default();
        host.userdata = std::ptr::from_mut(&mut lines).cast();
        host.output = Some(collect);
        assert_eq!(
            script.instance_with_host(RunLimits::default(), &host).0,
            ULUA_OK
        );
        let (status, code, _) = script.call(script.index("main"), &[]);
        let path = self.directory.join(format!("case-{}", self.count));
        std::fs::write(path.with_extension("ts"), source).unwrap();
        std::fs::write(path.with_extension("luau"), compiled.luau).unwrap();
        if let Some(kind) = trap {
            assert_eq!((status, code.as_str()), (ULUA_FAILED, kind), "{source}");
            self.traps += 1;
        } else {
            assert_eq!((status, code), (ULUA_OK, String::new()), "{source}");
            assert_eq!(lines.len(), 1, "{source}");
            std::fs::write(path.with_extension("expected"), format!("{}\n", lines[0])).unwrap();
            self.comparable += 1;
        }
        self.count += 1;
    }
    pub fn finish(&self) {
        eprintln!("P2.4 fixtures: {:.3}s", self.start.elapsed().as_secs_f64());
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let ts: Vec<_> = (0..self.count)
            .map(|i| self.directory.join(format!("case-{i}.ts")))
            .collect();
        let luau: Vec<_> = ts.iter().map(|p| p.with_extension("luau")).collect();
        let mut tsc = tsc::command(&root, &self.directory, &ts);
        success("tsc", &mut tsc);
        let out = analyzer::output(root.join("target/luau-tools/luau-analyze"), &luau);
        assert!(out.status.success(), "{}", analyzer::failure(&out));
        let runner = self.directory.join("witness.cjs");
        std::fs::write(&runner, r#"const fs=require('fs'),path=require('path');
const ts=require(process.argv[2]);const dir=process.argv[3];let count=0;
for(const file of fs.readdirSync(dir).filter(f=>f.endsWith('.expected'))){
 const base=path.join(dir,file.slice(0,-9));
 const source=fs.readFileSync(base+'.ts','utf8');
 const js=ts.transpileModule(source,{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.CommonJS}}).outputText;
 const exports={};new Function('exports','print',js+'\nexports.score=score;')(exports,()=>{});
 const actual=exports.score()+'\n',expected=fs.readFileSync(base+'.expected','utf8');
 if(actual!==expected)throw Error(file+'\n'+source+'\nNode: '+actual+'\nUlua: '+expected);count++;
}console.log('P2.4 node matches: '+count);"#).unwrap();
        let mut node = Command::new("node");
        node.arg(&runner)
            .arg(root.join("node_modules/typescript"))
            .arg(&self.directory);
        success("node", &mut node);
        eprintln!(
            "P2.4 witnesses: accepted={} node={} traps={} rejected={}",
            self.count, self.comparable, self.traps, self.rejects
        );
    }
}
impl Drop for Suite {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.directory);
    }
}
fn success(name: &str, command: &mut Command) {
    let start = Instant::now();
    let out = command.output().unwrap();
    eprintln!("P2.4 {name}: {:.3}s", start.elapsed().as_secs_f64());
    assert!(
        out.status.success(),
        "{command:?}\n{}",
        analyzer::failure(&out)
    );
    eprint!("{}", String::from_utf8_lossy(&out.stdout));
}

unsafe extern "C" fn collect(userdata: *mut c_void, text: *const c_char, length: usize) -> i32 {
    // SAFETY: The test keeps the output Vec live. The shim supplies readable UTF-8 bytes.
    unsafe {
        let bytes = std::slice::from_raw_parts(text.cast::<u8>(), length);
        (*userdata.cast::<Vec<String>>()).push(String::from_utf8_lossy(bytes).into_owned());
    }
    0
}
