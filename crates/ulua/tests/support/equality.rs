//! Batch equality witnesses. Each accepted source also runs through the C ABI.
use super::support::Script;
use std::{
    ffi::{c_char, c_void},
    path::PathBuf,
    process::Command,
};
use ulua::*;

pub struct Suite {
    directory: PathBuf,
    label: &'static str,
    pub count: usize,
}
impl Suite {
    pub fn new() -> Self {
        Self::named("D17")
    }
    pub fn named(label: &'static str) -> Self {
        let directory = std::env::temp_dir().join(format!("ulua-{label}-{}", std::process::id()));
        std::fs::create_dir_all(&directory).unwrap();
        Self {
            directory,
            label,
            count: 0,
        }
    }
    pub fn source(&mut self, source: &str, golden: Option<&str>) {
        let compiled = ulua_compiler::compile(source.as_bytes(), ulua_compiler::Limits::default())
            .unwrap_or_else(|d| panic!("{source}: {d:?}"));
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
        assert_eq!((status, code), (ULUA_OK, String::new()), "{source}");
        let output: String = lines.iter().map(|s| format!("{s}\n")).collect();
        if let Some(golden) = golden {
            assert_eq!(output, golden, "{source}");
        }
        let path = self.directory.join(format!("case-{}", self.count));
        std::fs::write(path.with_extension("ts"), source).unwrap();
        std::fs::write(path.with_extension("luau"), compiled.luau).unwrap();
        std::fs::write(path.with_extension("expected"), output).unwrap();
        self.count += 1;
    }
    pub fn finish(&self) {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let runner = self.directory.join("witness.mjs");
        std::fs::write(&runner, include_str!("equality.mjs")).unwrap();
        let out = Command::new("node")
            .arg(&runner)
            .arg(&root)
            .arg(&self.directory)
            .arg(self.label)
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        eprint!("{}", String::from_utf8_lossy(&out.stdout));
        let files: Vec<_> = (0..self.count)
            .map(|i| self.directory.join(format!("case-{i}.luau")))
            .collect();
        let analyze = std::env::var_os("LUAU_ANALYZE")
            .map(PathBuf::from)
            .unwrap_or_else(|| root.join("target/luau-tools/luau-analyze"));
        let out = Command::new(analyze).args(files).output().unwrap();
        assert!(
            out.status.success(),
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        eprintln!(
            "{} ABI and luau-analyze: {} accepted",
            self.label, self.count
        );
    }
}
impl Drop for Suite {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.directory);
    }
}
unsafe extern "C" fn collect(userdata: *mut c_void, text: *const c_char, length: usize) -> i32 {
    // SAFETY: The test keeps the output Vec live. The shim supplies readable UTF-8 bytes.
    unsafe {
        let bytes = std::slice::from_raw_parts(text.cast::<u8>(), length);
        (*userdata.cast::<Vec<String>>()).push(String::from_utf8_lossy(bytes).into_owned());
    }
    0
}
