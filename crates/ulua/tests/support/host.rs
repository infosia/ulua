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
        let tool = cc::windows_registry::find_tool("x86_64-pc-windows-msvc", "cl.exe").unwrap();
        let mut cmd = tool.to_command();
        cmd.current_dir(executable.parent().unwrap())
            // The examples use standard C functions that MSVC marks as deprecated.
            .args(["/nologo", "/W4", "/WX", "/MD", "/D_CRT_SECURE_NO_WARNINGS"])
            .args(if cpp {
                vec!["/TP", "/std:c++17", "/permissive-"]
            } else {
                vec!["/TC", "/std:c11"]
            })
            .arg(source)
            .arg("/I")
            .arg(root.join("include"))
            .arg(format!("/Fe:{}", executable.display()))
            .arg("/link")
            .arg(library)
            .args([
                "advapi32.lib",
                "bcrypt.lib",
                "ntdll.lib",
                "userenv.lib",
                "ws2_32.lib",
            ]);
        cmd
    } else {
        let mut cmd = Command::new(if cpp { "c++" } else { "cc" });
        if cfg!(ulua_sanitize) {
            cmd.arg("-fsanitize=address,undefined");
        }
        cmd.arg(if cpp { "-std=c++17" } else { "-std=c11" })
            .args([
                "-Wall",
                "-Wextra",
                "-Wpedantic",
                "-Werror",
                "-x",
                if cpp { "c++" } else { "c" },
            ])
            .arg(source)
            .arg("-I")
            .arg(root.join("include"))
            .args(["-x", "none"])
            .arg(library)
            .arg("-o")
            .arg(executable);
        if cfg!(target_os = "macos") {
            cmd.args([
                "-lc++",
                "-liconv",
                "-framework",
                "Security",
                "-framework",
                "CoreFoundation",
            ]);
        } else {
            cmd.args(["-lstdc++", "-lpthread", "-ldl", "-lm"]);
        }
        cmd
    }
}
