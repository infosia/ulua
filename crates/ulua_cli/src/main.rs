//! Process entry for the Ulua developer command.
#![warn(missing_docs)]
fn main() {
    let code = ulua_cli::execute(
        std::env::args_os().skip(1),
        &mut std::io::stdout().lock(),
        &mut std::io::stderr().lock(),
    );
    std::process::exit(code);
}
