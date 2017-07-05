use std::env;

fn main() {
    // TODO: Should use std::Path to append lib here
    let embree_dir = env::var("EMBREE_DIR").unwrap();
    println!("cargo:warning=embree dir = {}", embree_dir);
    println!("cargo:rustc-link-search=native={}lib", embree_dir);
    println!("cargo:rustc-link-lib=dylib=embree");
}

