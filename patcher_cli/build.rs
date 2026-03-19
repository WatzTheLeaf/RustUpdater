/*
MIT License

Copyright (c) 2026 Gaëtan Dezeiraud

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
*/
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