use std::env;
use std::path::PathBuf;

fn main() {
    //let mut embree_dir = PathBuf::from(env::var("EMBREE_DIR").unwrap());
    //embree_dir.push("lib");
    //println!("cargo:rustc-link-search=native={}", embree_dir.display());
    println!("cargo:rustc-link-lib=embree");

    /*
    let mut tbb_dir = PathBuf::from(env::var("TBB_DIR").unwrap());
    tbb_dir.push("lib/intel64/gcc4.7");
    println!("cargo:rustc-link-search=native={}", tbb_dir.display());
    println!("cargo:rustc-link-lib=tbb");
    println!("cargo:rustc-link-lib=tbbmalloc");
    */
}

