//! Shared native host flags and platform libraries.
#![warn(missing_docs)]
use std::{io, path::PathBuf, process::Command};

/// Convert Windows verbatim drive and UNC paths to plain paths for external tools.
/// Return an error for other verbatim prefixes. Leave paths unchanged on other platforms.
pub fn plain_path(path: PathBuf) -> io::Result<PathBuf> {
    #[cfg(windows)]
    {
        use std::path::{Component, Prefix};
        let mut parts = path.components();
        if let Some(Component::Prefix(prefix)) = parts.next() {
            let mut result = match prefix.kind() {
                Prefix::VerbatimDisk(drive) => PathBuf::from(format!("{}:\\", char::from(drive))),
                Prefix::VerbatimUNC(server, share) => {
                    let mut result = PathBuf::from(r"\\");
                    result.push(server);
                    result.push(share);
                    result
                }
                Prefix::Verbatim(_) => {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "unsupported verbatim path",
                    ));
                }
                _ => return Ok(path),
            };
            if matches!(parts.clone().next(), Some(Component::RootDir)) {
                parts.next();
            }
            result.push(parts.as_path());
            return Ok(result);
        }
    }
    Ok(path)
}

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
    fn plain_paths() {
        for (input, expected) in [
            (r"\\?\C:\repo\host.c", r"C:\repo\host.c"),
            (r"\\?\UNC\server\share\host.c", r"\\server\share\host.c"),
            (r"C:\repo\host.c", r"C:\repo\host.c"),
            ("relative/host.c", "relative/host.c"),
        ] {
            let path = PathBuf::from(input);
            #[cfg(windows)]
            assert_eq!(plain_path(path).unwrap(), PathBuf::from(expected));
            #[cfg(not(windows))]
            {
                let _ = expected;
                assert_eq!(plain_path(path.clone()).unwrap(), path);
            }
        }
        let path = PathBuf::from(r"\\?\Volume{test}\host.c");
        #[cfg(windows)]
        assert_eq!(
            plain_path(path).unwrap_err().kind(),
            io::ErrorKind::InvalidInput
        );
        #[cfg(not(windows))]
        assert_eq!(plain_path(path.clone()).unwrap(), path);
    }

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
