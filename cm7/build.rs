use std::path::PathBuf;

fn main() {
    let manifest_dir = PathBuf::from(
        std::env::var("CARGO_MANIFEST_DIR").unwrap()
    );

    println!(
        "cargo:rustc-link-search={}",
        manifest_dir.display()
    );

    println!("cargo:rerun-if-changed=memory.x");
}