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
use anyhow::{Context, Result};
use std::fs::{self, File};
use std::path::{Path};
use std::process::{Command, Stdio};
use walkdir::WalkDir;

/// Calculates hash of a file
pub fn file_blake3(path: &Path) -> Result<String> {
    let mut file = File::open(path)?;
    let mut hasher = blake3::Hasher::new();
    std::io::copy(&mut file, &mut hasher)?;
    Ok(hasher.finalize().to_hex().to_string())
}

/// Recursively collects all files in a directory, returning relative paths
pub fn collect_files(root: &Path) -> Result<Vec<String>> {
    let mut files = Vec::new();
    for entry in WalkDir::new(root).into_iter().filter_map(|e| e.ok()) {
        if entry.file_type().is_file() {
            let path = entry.path();
            // Convert absolute path to relative path
            let relative = path.strip_prefix(root)?.to_string_lossy().replace("\\", "/");
            files.push(relative);
        }
    }
    Ok(files)
}

/// Runs hdiffz to generate a patch
pub fn generate_patch(old_file: &Path, new_file: &Path, out_file: &Path) -> Result<bool> {
    if let Some(parent) = out_file.parent() {
        fs::create_dir_all(parent)?;
    }

    // Locate hdiffz next to the current executable
    let current_exe = std::env::current_exe()?;
    let exe_dir = current_exe.parent().context("Failed to get exe dir")?;
    let hdiffz_path = exe_dir.join("hdiffz.exe");

    let status = Command::new(hdiffz_path)
        .arg(old_file)
        .arg(new_file)
        .arg(out_file)
        .arg("-c-zstd-21")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .context("Failed to execute hdiffz")?;

    Ok(status.success())
}