//! Batch equality witnesses. Each accepted source also runs through the C ABI.
//! Measured single-worker Debug cost: 0.46 seconds for p26_classes.
#[path = "analyzer.rs"]
mod analyzer;
#[path = "requests.rs"]
mod requests;
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
        self.run(source, golden, None);
    }
    // The shared equality targets compile this method; only the store-order target uses it.
    #[allow(dead_code)]
    pub fn source_trap(&mut self, source: &str, golden: &str, kind: &str, node_kind: &str) {
        self.run(source, Some(golden), Some((kind, node_kind)));
    }
    #[allow(dead_code)]
    pub fn source_async_trap(&mut self, source: &str, node_kind: &str) {
        self.run(source, None, Some(("trap:uncaught-exception", node_kind)));
    }
    fn run(&mut self, source: &str, golden: Option<&str>, trap: Option<(&str, &str)>) {
        let host_requests = source.contains("// grants: hostLater");
        let grants = if host_requests {
            requests::signatures(source)
        } else {
            vec![]
        };
        let compiled = ulua_compiler::compile_with_grants(
            source.as_bytes(),
            ulua_compiler::Limits::default(),
            &grants,
        )
        .unwrap_or_else(|d| panic!("{source}: {d:?}"));
        let (status, code, output, declarations) = if host_requests {
            requests::run(source, RunLimits::default())
        } else {
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
            (
                status,
                code,
                lines.iter().map(|s| format!("{s}\n")).collect(),
                String::new(),
            )
        };
        let expected = trap.map_or((ULUA_OK, ""), |(kind, _)| (ULUA_FAILED, kind));
        assert_eq!((status, code.as_str()), expected, "{source}");
        if let Some(golden) = golden {
            assert_eq!(output, golden, "{source}");
        }
        let path = self.directory.join(format!("case-{}", self.count));
        std::fs::write(path.with_extension("ts"), source).unwrap();
        std::fs::write(path.with_extension("luau"), compiled.luau).unwrap();
        std::fs::write(path.with_extension("expected"), output).unwrap();
        if host_requests {
            std::fs::write(path.with_extension("d.ts"), declarations).unwrap();
        }
        if let Some((_, node_kind)) = trap {
            std::fs::write(path.with_extension("trap"), node_kind).unwrap();
        }
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
            "{:?}\n{}{}",
            out.status,
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
        let out = analyzer::command(analyze).args(files).output().unwrap();
        assert!(
            out.status.success(),
            "luau-analyze {}",
            analyzer::failure(&out)
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
