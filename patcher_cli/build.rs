use std::env;
use std::fs;
use std::path::{Path};

fn main() {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let vendor_dir = Path::new(&manifest_dir).join("vendor").join("hdiffpatch");

    // Define the output directory (target/debug or target/release)
    let out_dir = env::var("OUT_DIR").unwrap();
    let target_dir = Path::new(&out_dir)
        .ancestors()
        .nth(3)
        .unwrap();

    // List of files to copy
    let files = vec!["hdiffz.exe"];

    for filename in files {
        let src = vendor_dir.join(filename);
        let dest = target_dir.join(filename);

        // Only copy if source exists
        if src.exists() {
            fs::copy(&src, &dest).expect("Failed to copy executable");
            println!("cargo:warning=Copied {} to {:?}", filename, dest);
        } else {
            println!("cargo:warning=Could not find {} in vendor/hdiffpatch", filename);
        }
    }
    
    println!("cargo:rerun-if-changed=vendor/hdiffpatch");
}