use std::env;
use std::path::PathBuf;

fn main() {
    if let Ok(e) = env::var("EMBREE_DIR") {
        let mut embree_dir = PathBuf::from(e);
        embree_dir.push("lib");
        println!("cargo:rustc-link-search=native={}", embree_dir.display());
    } else {
        println!("cargo:error=Please set EMBREE_DIR=<path to embree3 root>");
        panic!("Failed to find embree");
    }
    println!("cargo:rerun-if-env-changed=EMBREE_DIR");
    println!("cargo:rustc-link-lib=embree3");
}

