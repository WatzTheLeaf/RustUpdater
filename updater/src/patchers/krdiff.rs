/*
MIT License

Copyright (c) 2025 TukanDev

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

use std::fs::create_dir_all;
use std::path::Path;
use crate::patchers::KrDiff;
use crate::utils::patch_krdir::KrPatchDir;

/*
WARNING: This shit is extremely cursed and is modification of standard HDiff format, it is not something you should use it can break and go to fuckshit anytime...
This only exists to support TwintailLauncher's use case and is very hacked to hell compared to actual standard HDiff patching part
HERE BE DRAGONS you are warned!!!
#FuckKuroGames btw
*/

impl KrDiff {
    pub fn new(source_path: String, diff_path: String, dest_path: String) -> Self {
        KrDiff { source_path, diff_path, dest_path, cache_size: 0 }
    }

    pub fn set_cache_size(&mut self, cache_size: usize) { self.cache_size = cache_size; }

    pub fn apply(&mut self) -> bool {
        match self.apply_inner() {
            Ok(()) => true,
            Err(e) => { eprintln!("[KrDiff::apply] Error: {}", e); false }
        }
    }

    fn apply_inner(&self) -> Result<(), Box<dyn std::error::Error>> {
        let src = Path::new(&self.source_path);
        let diffp = Path::new(&self.diff_path);

        let dst = std::path::PathBuf::from(&self.dest_path);
        if !src.exists() || !src.is_dir() { return Err(format!("[KrDiff] Source path {} does not exist or is not a directory", src.display()).into()); }
        if !diffp.exists() || !diffp.is_file() { return Err(format!("[KrDiff] Diff file {} does not exist", diffp.display()).into()); }
        if !dst.exists() { create_dir_all(&dst)?; }

        let patcher = KrPatchDir::new(self.diff_path.clone());
        patcher.patch(src.to_str().unwrap_or(""), dst.to_str().unwrap_or(""), None)?;
        Ok(())
    }
}
