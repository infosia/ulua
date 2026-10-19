//! Host toolchain commands and independent path precedence.
use crate::{Result, io_error};
use std::{
    ffi::{OsStr, OsString},
    io::Write,
    path::{Path, PathBuf},
    process::Command,
};
use ulua_toolchain as flags;

/// Locate the repository from the compiled crate path.
pub(crate) fn repository() -> Option<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    root.join("include/ulua.h").is_file().then_some(root)
}
/// Select one path without coupling the include and library sources.
pub(crate) fn select(
    option: Option<PathBuf>,
    environment: Option<OsString>,
    default: Option<PathBuf>,
) -> Option<PathBuf> {
    option
        .or_else(|| environment.map(PathBuf::from))
        .or(default)
}
fn paths(lib: Option<PathBuf>, include: Option<PathBuf>) -> Result<(PathBuf, PathBuf)> {
    let root = repository();
    let library = select(
        lib,
        std::env::var_os("ULUA_LIB"),
        root.as_ref().map(|r| {
            r.join("target/release").join(if cfg!(target_env = "msvc") {
                "ulua.lib"
            } else {
                "libulua.a"
            })
        }),
    );
    let include = select(
        include,
        std::env::var_os("ULUA_INCLUDE"),
        root.map(|r| r.join("include")),
    );
    let lib_help = "use --lib <file>, ULUA_LIB, or the repository target/release default; run cargo build --release -p ulua";
    let include_help = "use --include <dir>, ULUA_INCLUDE, or the repository include default";
    let library = library.ok_or_else(|| (2, format!("static library missing: {lib_help}")))?;
    let include =
        include.ok_or_else(|| (2, format!("include directory missing: {include_help}")))?;
    for (path, help) in [
        (&library, lib_help),
        (&include.join("ulua.h"), include_help),
    ] {
        let metadata = std::fs::metadata(path)
            .map_err(|e| io_error(format!("{}: {e}; {help}", path.display())))?;
        if !metadata.is_file() {
            return Err(crate::path_error(
                path,
                std::io::Error::new(std::io::ErrorKind::InvalidInput, "expected a file"),
            ));
        }
    }
    Ok((canonical(&library)?, canonical(&include)?))
}
fn canonical(path: &Path) -> Result<PathBuf> {
    std::fs::canonicalize(path).map_err(|e| crate::path_error(path, e))
}
/// Split a compiler setting at ASCII white space, without a shell.
pub(crate) fn compiler_value(value: &OsStr, variable: &str) -> Result<Command> {
    let value = value
        .to_str()
        .ok_or_else(|| (2, format!("{variable}: compiler setting is not UTF-8")))?;
    let mut words = value
        .split(|c: char| c.is_ascii_whitespace() || c == '\x0b')
        .filter(|word| !word.is_empty());
    let program = words
        .next()
        .ok_or_else(|| (2, format!("{variable}: compiler setting is empty")))?;
    let mut cmd = Command::new(program);
    cmd.args(words);
    Ok(cmd)
}
fn compiler(cpp: bool) -> Result<Command> {
    if cfg!(target_env = "msvc") {
        return flags::msvc_command().map_err(|e| io_error(format!("cl.exe: {e}")));
    }
    let variable = if cpp { "CXX" } else { "CC" };
    compiler_value(
        &std::env::var_os(variable)
            .unwrap_or_else(|| OsString::from(if cpp { "c++" } else { "cc" })),
        variable,
    )
}
fn write_compiler_output(command: &Command, bytes: &[u8], stderr: &mut dyn Write) -> Result<()> {
    let mut text = String::from_utf8_lossy(bytes).into_owned();
    // Escape complete argument paths first, including any embedded line endings in a file name.
    for argument in command
        .get_args()
        .chain(std::iter::once(command.get_program()))
    {
        let original = argument.to_string_lossy();
        let escaped = crate::diagnostic::filename(argument);
        if escaped != original {
            text = text.replace(original.as_ref(), &escaped);
        }
    }
    // Keep compiler line endings and source tabs, while escaping terminal controls and Cf characters.
    for line in text.split_inclusive('\n') {
        let (content, ending) = if let Some(content) = line.strip_suffix("\r\n") {
            (content, "\r\n")
        } else if let Some(content) = line.strip_suffix('\n') {
            (content, "\n")
        } else {
            (line, "")
        };
        stderr
            .write_all(crate::diagnostic::escape(content, true).as_bytes())
            .and_then(|()| stderr.write_all(ending.as_bytes()))
            .map_err(|e| io_error(format!("stderr: {e}")))?;
    }
    Ok(())
}
fn run_compiler(mut command: Command, stderr: &mut dyn Write) -> Result<bool> {
    let output = command
        .output()
        .map_err(|e| io_error(format!("{}: {e}", command.get_program().display())))?;
    if output.status.success() {
        return Ok(true);
    }
    write_compiler_output(&command, &output.stdout, stderr)?;
    write_compiler_output(&command, &output.stderr, stderr)?;
    writeln!(stderr, "build: compiler failed ({})", output.status)
        .map_err(|e| io_error(format!("stderr: {e}")))?;
    Ok(false)
}
/// Print link flags or build a native host.
pub(crate) fn execute(
    args: &[OsString],
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> Result<i32> {
    let build = args[0] == "build";
    let mut lib = None;
    let mut include = None;
    let mut cc = if cfg!(target_env = "msvc") {
        "msvc"
    } else {
        "unix"
    };
    let mut seen_cc = false;
    let mut sources = Vec::new();
    let mut directory = None;
    let mut run = false;
    let mut run_args = &args[args.len()..];
    let mut i = 1;
    while i < args.len() {
        match args[i].to_str() {
            Some("--lib" | "--include" | "--cc" | "-o") => {
                let option = args[i].to_str().unwrap_or_default();
                i += 1;
                let value = args
                    .get(i)
                    .ok_or_else(|| crate::usage_error(format!("missing value for {option}")))?;
                if value.is_empty() || value.to_string_lossy().starts_with('-') {
                    return Err(crate::usage_error(format!(
                        "missing value for {option}; found {}",
                        value.display()
                    )));
                }
                match option {
                    "--lib" if lib.is_none() => lib = Some(PathBuf::from(value)),
                    "--include" if include.is_none() => include = Some(PathBuf::from(value)),
                    "--cc" if !build && !seen_cc => {
                        seen_cc = true;
                        cc = match value.to_str() {
                            Some("unix") => "unix",
                            Some("msvc") => "msvc",
                            _ => {
                                return Err(crate::usage_error(format!(
                                    "invalid --cc value {}; expected unix or msvc",
                                    value.display()
                                )));
                            }
                        }
                    }
                    "-o" if build && directory.is_none() => directory = Some(PathBuf::from(value)),
                    _ => {
                        return Err(crate::usage_error(format!(
                            "unexpected or repeated option {option}"
                        )));
                    }
                }
            }
            Some("--host") if build => {
                let before = sources.len();
                while let Some(value) = args
                    .get(i + 1)
                    .filter(|v| !v.to_string_lossy().starts_with('-'))
                {
                    sources.push(PathBuf::from(value));
                    i += 1;
                }
                if sources.len() == before {
                    return Err(crate::usage_error("missing file for --host"));
                }
            }
            Some("--run") if build && !run => run = true,
            Some("--") if build && run => {
                run_args = &args[i + 1..];
                break;
            }
            _ => {
                return Err(crate::usage_error(format!(
                    "unexpected argument {}",
                    args[i].display()
                )));
            }
        }
        i += 1;
    }
    if build && sources.is_empty() {
        return Err(crate::usage_error("missing --host <file>"));
    }
    let languages: Vec<bool> = sources
        .iter()
        .map(|s| match s.extension().and_then(OsStr::to_str) {
            Some("c") => Ok(false),
            Some("cc" | "cpp" | "cxx") => Ok(true),
            _ => Err(crate::usage_error(format!(
                "unsupported --host extension: {}",
                s.display()
            ))),
        })
        .collect::<Result<_>>()?;
    let (library, include) = paths(lib, include)?;
    if !build {
        writeln!(
            stdout,
            "{}{}",
            if cc == "msvc" { "/I" } else { "-I" },
            crate::diagnostic::filename(include.as_os_str())
        )
        .map_err(|e| io_error(format!("stdout: {e}")))?;
        writeln!(
            stdout,
            "{}",
            crate::diagnostic::filename(library.as_os_str())
        )
        .map_err(|e| io_error(format!("stdout: {e}")))?;
        let mut libraries = flags::system_libraries().iter();
        while let Some(item) = libraries.next() {
            if *item == "-framework" {
                if let Some(name) = libraries.next() {
                    writeln!(stdout, "{item} {name}")
                        .map_err(|e| io_error(format!("stdout: {e}")))?;
                }
            } else {
                writeln!(stdout, "{item}").map_err(|e| io_error(format!("stdout: {e}")))?;
            }
        }
        return Ok(0);
    }
    let first = &sources[0];
    let stem = first.file_stem().ok_or_else(|| {
        crate::usage_error(format!("--host file has no stem: {}", first.display()))
    })?;
    let directory =
        directory.unwrap_or_else(|| first.parent().unwrap_or(Path::new(".")).join("ulua-build"));
    let host_includes = sources
        .iter()
        .map(|source| {
            let parent = source
                .parent()
                .filter(|p| !p.as_os_str().is_empty())
                .unwrap_or(Path::new("."));
            canonical(parent)
        })
        .collect::<Result<Vec<_>>>()?;
    // Resolve inputs before the compiler changes its working directory.
    let sources = sources
        .iter()
        .map(|source| canonical(source))
        .collect::<Result<Vec<_>>>()?;
    std::fs::create_dir_all(&directory).map_err(|e| crate::path_error(&directory, e))?;
    let directory = canonical(&directory)?;
    let mut name = stem.to_os_string();
    name.push(std::env::consts::EXE_SUFFIX);
    let executable = directory.join(name);
    let msvc = cfg!(target_env = "msvc");
    let mut objects = Vec::new();
    for (index, (source, cpp)) in sources.iter().zip(&languages).enumerate() {
        let object = directory.join(format!("ulua-{index}.{}", if msvc { "obj" } else { "o" }));
        let mut cmd = compiler(*cpp)?;
        cmd.current_dir(&directory)
            .args(if msvc {
                flags::MSVC_FLAGS
            } else {
                flags::UNIX_FLAGS
            })
            .args(flags::language_flags(msvc, *cpp));
        if !msvc {
            cmd.args(["-x", if *cpp { "c++" } else { "c" }]);
        }
        cmd.arg(if msvc { "/c" } else { "-c" }).arg(source);
        for inc in
            std::iter::once(include.as_path()).chain(host_includes.iter().map(PathBuf::as_path))
        {
            cmd.arg(if msvc { "/I" } else { "-I" }).arg(inc);
        }
        if msvc {
            cmd.arg(format!("/Fo{}", object.display()));
        } else {
            cmd.arg("-o").arg(&object);
        }
        if !run_compiler(cmd, stderr)? {
            return Ok(1);
        }
        objects.push(object);
    }
    let mut cmd = compiler(languages.iter().any(|cpp| *cpp))?;
    cmd.current_dir(&directory);
    if msvc {
        cmd.args(flags::MSVC_FLAGS)
            .args(&objects)
            .arg(format!("/Fe:{}", executable.display()))
            .arg("/link");
    } else {
        cmd.args(&objects).arg("-o").arg(&executable);
    }
    cmd.arg(library).args(flags::system_libraries());
    if !run_compiler(cmd, stderr)? {
        return Ok(1);
    }
    if run {
        let status = Command::new(&executable)
            .args(run_args)
            .stdin(std::process::Stdio::inherit())
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit())
            .status()
            .map_err(|e| crate::path_error(&executable, e))?;
        return Ok(status.code().unwrap_or(1));
    }
    Ok(0)
}
