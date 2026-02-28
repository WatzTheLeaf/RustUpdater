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

use std::fs::File;
use std::io::{BufWriter, Write};
use crate::patchers::HDiff;
use crate::utils::header::Header;
use crate::utils::patch_dir::PatchDir;
use crate::utils::patch_single::PatchSingle;
use crate::utils::structs::DataReferenceInfo;

impl HDiff {
    pub fn new(source_path: String, diff_path: String, dest_path: String) -> Self {
        HDiff { source_path, diff_path, dest_path, cache_size: 0 }
    }

    pub fn set_cache_size(&mut self, cache_size: usize) {
        self.cache_size = cache_size;
    }

    pub fn apply(&mut self) -> bool {
        match self.apply_inner() {
            Ok(()) => true,
            Err(e) => { eprintln!("[HDiff::apply] Error: {}", e); false }
        }
    }

    fn apply_inner(&self) -> Result<(), Box<dyn std::error::Error>> {
        let mut diff_file = File::open(&self.diff_path)?;
        let mut header_info = Default::default();
        let mut reference_info: DataReferenceInfo = Default::default();
        let is_dir_patch = Header::try_parse_header_info(&mut diff_file, &self.diff_path, &mut header_info, &mut reference_info)?;

        if is_dir_patch && header_info.is_input_dir && header_info.is_output_dir {
            let mut patcher = PatchDir::new(header_info, reference_info, self.diff_path.clone());
            patcher.patch(&self.source_path, &self.dest_path, None)?;
            return Ok(());
        }

        // Validate input file size.
        let mut old_file = File::open(&self.source_path)?;
        let old_len = old_file.metadata()?.len() as i64;
        if old_len != header_info.old_data_size { return Err(format!("[HDiff::apply] Input file size mismatch: expected {} bytes, got {} bytes", header_info.old_data_size, old_len).into()); }

        #[cfg(debug_assertions)]
        println!("[HDiff::apply] Old size: {} ✓ | New size: {}", old_len, header_info.new_data_size);

        // Create output file.
        let out_file = File::create(&self.dest_path)?;
        let mut out_writer = BufWriter::new(out_file);

        let patcher = PatchSingle::new(header_info);
        patcher.patch(&mut old_file, &mut out_writer, &self.diff_path, None)?;
        out_writer.flush()?;
        Ok(())
    }
}
