//! Link the sanitizer runtime into the CLI and its tests.
#[path = "../ulua/build/sanitize.rs"]
mod sanitize;

fn main() {
    if let Err(error) = sanitize::link() {
        eprintln!("Ulua CLI build failed: {error}");
        std::process::exit(1);
    }
}
