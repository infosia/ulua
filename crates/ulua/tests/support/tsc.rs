//! A project file keeps witness paths out of the operating system command line.
use std::{
    path::{Path, PathBuf},
    process::Command,
};

pub fn command(root: &Path, directory: &Path, paths: &[PathBuf]) -> Command {
    let files = std::iter::once(root.join("prelude/ulua.d.ts"))
        .chain(paths.iter().cloned())
        .map(|p| format!("{:?}", p.to_str().unwrap()))
        .collect::<Vec<_>>()
        .join(",");
    let config = directory.join("tsconfig-witness.json");
    std::fs::write(&config, format!(r#"{{"compilerOptions":{{"noEmit":true,"strict":true,"target":"es2022","module":"esnext","lib":["es2022"]}},"files":[{files}],"include":[]}}"#)).unwrap();
    let mut cmd = Command::new("node");
    cmd.current_dir(root)
        .arg(root.join("node_modules/typescript/bin/tsc"))
        .arg("-p")
        .arg(config);
    cmd
}
