//! Shared native host flags and platform libraries.
#![warn(missing_docs)]
use std::{io, process::Command};

/// Strict common flags for Unix host compilers.
pub const UNIX_FLAGS: &[&str] = &["-Wall", "-Wextra", "-Wpedantic", "-Werror"];
/// Strict common flags for the MSVC host compiler.
pub const MSVC_FLAGS: &[&str] = &["/nologo", "/W4", "/WX", "/MD", "/D_CRT_SECURE_NO_WARNINGS"];
/// Language flags for the selected compiler and language.
pub fn language_flags(msvc: bool, cpp: bool) -> &'static [&'static str] {
    match (msvc, cpp) {
        (true, true) => &["/TP", "/std:c++17", "/permissive-"],
        (true, false) => &["/TC", "/std:c11"],
        (false, true) => &["-std=c++17"],
        (false, false) => &["-std=c11"],
    }
}
/// System library arguments in gate helper order.
pub fn system_libraries() -> &'static [&'static str] {
    if cfg!(target_env = "msvc") {
        &[
            "advapi32.lib",
            "bcrypt.lib",
            "ntdll.lib",
            "userenv.lib",
            "ws2_32.lib",
        ]
    } else if cfg!(target_os = "macos") {
        &[
            "-lc++",
            "-liconv",
            "-framework",
            "Security",
            "-framework",
            "CoreFoundation",
        ]
    } else {
        &["-lstdc++", "-lpthread", "-ldl", "-lm"]
    }
}
/// Find MSVC with its SDK environment, as the gate helper does.
pub fn msvc_command() -> io::Result<Command> {
    cc::windows_registry::find_tool("x86_64-pc-windows-msvc", "cl.exe")
        .map(|tool| tool.to_command())
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "cannot find cl.exe and its SDK"))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn host_flags() {
        assert_eq!(language_flags(false, false), ["-std=c11"]);
        assert_eq!(language_flags(false, true), ["-std=c++17"]);
        assert!(language_flags(true, true).contains(&"/permissive-"));
        assert!(language_flags(true, false).contains(&"/std:c11"));
        assert!(UNIX_FLAGS.contains(&"-Werror"));
        assert!(MSVC_FLAGS.contains(&"/WX"));
        assert!(!system_libraries().is_empty());
        if cfg!(target_env = "msvc") {
            assert!(msvc_command().is_ok());
        }
    }
}
