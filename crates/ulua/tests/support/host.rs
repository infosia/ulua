//! Native host compilation preserves strict warnings on each toolchain.
use std::{path::Path, process::Command};

pub fn compile(
    root: &Path,
    source: &Path,
    library: &Path,
    executable: &Path,
    cpp: bool,
) -> Command {
    if cfg!(target_env = "msvc") {
        // MSVC needs its SDK environment even outside a developer shell.
        let mut cmd = ulua_toolchain::msvc_command().unwrap();
        cmd.current_dir(executable.parent().unwrap())
            // The examples use standard C functions that MSVC marks as deprecated.
            .args(ulua_toolchain::MSVC_FLAGS)
            .args(ulua_toolchain::language_flags(true, cpp))
            .arg(source)
            .arg("/I")
            .arg(root.join("include"))
            .arg(format!("/Fe:{}", executable.display()))
            .arg("/link")
            .arg(library)
            .args(ulua_toolchain::system_libraries());
        cmd
    } else {
        let mut cmd = Command::new(if cpp { "c++" } else { "cc" });
        if cfg!(ulua_sanitize) {
            cmd.arg("-fsanitize=address,undefined");
        }
        cmd.args(ulua_toolchain::language_flags(false, cpp))
            .args(ulua_toolchain::UNIX_FLAGS)
            .args(["-x", if cpp { "c++" } else { "c" }])
            .arg(source)
            .arg("-I")
            .arg(root.join("include"))
            .args(["-x", "none"])
            .arg(library)
            .arg("-o")
            .arg(executable);
        cmd.args(ulua_toolchain::system_libraries());
        cmd
    }
}
