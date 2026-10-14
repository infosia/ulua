//! Link the sanitizer runtime into compiler tests under the sanitizer gate.
#[path = "../ulua/build/sanitize.rs"]
mod sanitize;

fn main() {
    if let Err(error) = sanitize::link() {
        eprintln!("Ulua compiler build failed: {error}");
        std::process::exit(1);
    }
}
