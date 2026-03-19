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
mod args;
mod models;
mod utils;

use rayon::prelude::*;
use std::sync::{Arc, Mutex};
use anyhow::Result;
use clap::Parser;
use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::path::PathBuf;

use args::Args;
use models::{FileEntry, ProductEntry, Manifest, PatchInfo, RootJson};
use utils::{collect_files, file_blake3, generate_patch};

fn main() -> Result<()> {
    let args = Args::parse();

    // Load or Create root.json
    let root_path = args.output.join("root.json");
    let mut root: RootJson = if root_path.exists() {
        let data = fs::read_to_string(&root_path)?;
        serde_json::from_str(&data).unwrap_or_default()
    } else {
        RootJson::default()
    };

    // Detect Previous Version
    let mut previous_version_dir = PathBuf::new();
    let mut has_previous_version = false;

    if let Some(entry) = root.products.get(&args.product) {
        println!("Detected previous version: {}", entry.latest_version);
        previous_version_dir = args.output
            .join("products")
            .join(&args.product)
            .join(&entry.latest_version)
            .join("full");

        if previous_version_dir.exists() {
            has_previous_version = true;
        }
    } else {
        println!("No previous version found (Fresh install).");
    }

    // Prepare Output Directories
    let product_output_base = args.output.join("products").join(&args.product).join(&args.version);
    let full_output_dir = product_output_base.join("full");
    let patch_output_dir = product_output_base.join("patches");

    fs::create_dir_all(&full_output_dir)?;
    if has_previous_version {
        fs::create_dir_all(&patch_output_dir)?;
    }

    // Scan New Files
    println!("Scanning files...");
    let new_files_list = collect_files(&args.new_dir)?;

    // Arc<Mutex> to safely share counters across threads
    let full_size = Arc::new(Mutex::new(0u64));
    let total_patch_size = Arc::new(Mutex::new(0u64));

    println!("Processing {} files (Parallel)...", new_files_list.len());

    // Process Files
    let file_entries_map: HashMap<String, FileEntry> = new_files_list
        .par_iter()
        .map(|rel_path| {
            let src_path = args.new_dir.join(rel_path);
            let dest_path = full_output_dir.join(rel_path);

            // Copy full file
            if let Some(parent) = dest_path.parent() {
                fs::create_dir_all(parent).unwrap_or_default();
            }
            // Use copy, ignore errors if multiple threads try to make dir at same time
            let _ = fs::copy(&src_path, &dest_path);

            // Calculate Hash & Size
            let hash = file_blake3(&src_path).unwrap_or_default();
            let size = fs::metadata(&src_path).map(|m| m.len()).unwrap_or(0);

            // Update shared counter for full size
            {
                let mut f_size = full_size.lock().unwrap();
                *f_size += size;
            }

            let mut patch_info = None;

            // Generate Patch
            if has_previous_version {
                let old_file_path = previous_version_dir.join(rel_path);

                if old_file_path.exists() {
                    let old_hash = file_blake3(&old_file_path).unwrap_or_default();
                    if old_hash != hash {
                        // Content changed -> Generate Patch
                        let patch_filename = format!("{}.patch", rel_path.replace("/", "_"));
                        let patch_path = patch_output_dir.join(&patch_filename);

                        if generate_patch(&old_file_path, &src_path, &patch_path).unwrap_or(false) {
                            let p_size = fs::metadata(&patch_path).map(|m| m.len()).unwrap_or(0);

                            // Update shared counter for patch size
                            {
                                let mut t_size = total_patch_size.lock().unwrap();
                                *t_size += p_size;
                            }

                            patch_info = Some(PatchInfo {
                                file: format!("patches/{}", patch_filename),
                                size: p_size,
                            });
                            println!("Patched: {}", rel_path);
                        }
                    }
                }
            }

            // Return the entry to be collected into the HashMap
            (rel_path.clone(), FileEntry {
                hash,
                size,
                patch: patch_info,
            })
        })
        .collect();

    println!("Processing complete.");

    // Calculate Deleted Files
    let mut deleted_files = Vec::new();
    if has_previous_version {
        let old_files = collect_files(&previous_version_dir).unwrap_or_default();
        let new_files_set: HashSet<_> = new_files_list.iter().collect();

        for old_rel in old_files {
            if !new_files_set.contains(&old_rel) {
                deleted_files.push(old_rel);
            }
        }
    }

    // Write Manifest
    let final_full_size = *full_size.lock().unwrap();
    let final_patch_size = *total_patch_size.lock().unwrap();

    let manifest = Manifest {
        version: args.version.clone(),
        exe: args.exe.clone(),
        files: file_entries_map,
        deleted_files,
        full_size: final_full_size,
        total_patch_size: final_patch_size,
    };

    let manifest_path = product_output_base.join("manifest.json");
    let manifest_file = File::create(&manifest_path)?;
    serde_json::to_writer_pretty(manifest_file, &manifest)?;
    println!("Manifest generated: {:?}", manifest_path);

    // Update root.json
    let mut entry = root.products.entry(args.product.clone()).or_insert(ProductEntry {
        latest_version: String::new(),
        manifest: String::new(),
        versions: Vec::new(),
    }).clone();

    entry.latest_version = args.version.clone();
    entry.manifest = format!("products/{}/{}/manifest.json", args.product, args.version);

    if !entry.versions.contains(&args.version) {
        entry.versions.push(args.version.clone());
    }

    root.products.insert(args.product, entry);

    let root_file = File::create(root_path)?;
    serde_json::to_writer_pretty(root_file, &root)?;
    println!("root.json updated.");

    Ok(())
}