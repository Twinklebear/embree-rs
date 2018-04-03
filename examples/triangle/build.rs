use std::env;
use std::path::PathBuf;

fn main() {
    if let Ok(e) = env::var("EMBREE_DIR") {
        let mut embree_dir = PathBuf::from(e);
        embree_dir.push("lib");
        println!("cargo:rustc-link-search=native={}", embree_dir.display());
    }
    println!("cargo:rustc-link-lib=embree3");
}

