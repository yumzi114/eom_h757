use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=memory.x");

    let manifest_dir =
        PathBuf::from(
            std::env::var("CARGO_MANIFEST_DIR")
                .expect("CARGO_MANIFEST_DIR is not set"),
        );

    println!(
        "cargo:rustc-link-search={}",
        manifest_dir.display()
    );
}