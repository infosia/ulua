//! Link the sanitizer runtime into the corpus runner and its tests.
#[path = "../ulua/build/sanitize.rs"]
mod sanitize;

fn main() {
    if let Err(error) = sanitize::link() {
        eprintln!("Ulua corpus build failed: {error}");
        std::process::exit(1);
    }
}
