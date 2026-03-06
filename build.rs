use std::fs;
use std::path::{Path, PathBuf};

fn main() {

    let slint_dir = Path::new("src/slint");
    let mut config = slint_build::CompilerConfiguration::new();

    let mut include_paths = vec![slint_dir.to_path_buf()];
    for entry in std::fs::read_dir(slint_dir).unwrap() {
        let entry = entry.expect("Failed to read directory entry");
        if entry.path().is_dir() {
            include_paths.push(entry.path());
        }
    }

    config = config.with_include_paths(include_paths);
    slint_build::compile_with_config("src/slint/main-window.slint", config.clone()).expect("Slint build failed");
}