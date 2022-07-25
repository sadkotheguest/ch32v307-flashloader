use std::path::PathBuf;
use std::{env, fs};

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    println!("cargo:rustc-link-search)={}", out_dir.display());

    fs::copy("loader.x", out_dir.join("loader.x")).unwrap();
    println!("cargo:rerun-if-changed=loader.x");
}
