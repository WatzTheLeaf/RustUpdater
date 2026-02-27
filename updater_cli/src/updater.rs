use crate::models::{FileEntry, Manifest, RootJson};
use anyhow::{Context, Result};
use futures::stream::{self, StreamExt};
use reqwest::Client;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use crate::patchers::HDiff;

/// Files larger than this are written to disk via streaming instead of being
/// loaded entirely into RAM first.
const STREAM_THRESHOLD: u64 = 30 * 1024 * 1024; // 30 MB

/// Maximum number of concurrent file operations.
const CONCURRENCY: usize = 8;

// ---------------------------------------------------------------------------
// Benchmark statistics
// ---------------------------------------------------------------------------

/// Counters collected during an update run.
#[derive(Debug, Default)]
pub struct UpdateStats {
    /// Files that were already up to date (skipped).
    pub skipped: u64,
    /// Files updated via binary patch.
    pub patched: u64,
    /// Files downloaded in full.
    pub full_downloads: u64,
    /// Total bytes of patch files downloaded.
    pub patch_bytes: u64,
    /// Total bytes of full files downloaded.
    pub full_bytes: u64,
    /// Files whose full download was streamed (>= STREAM_THRESHOLD).
    pub streamed: u64,
    /// Total wall-clock time spent applying patches (across all threads).
    pub patch_time: Duration,
}

impl UpdateStats {
    /// Total number of files processed.
    pub fn total_files(&self) -> u64 {
        self.skipped + self.patched + self.full_downloads
    }

    pub fn print_summary(&self) {
        println!("========================================");
        println!("  UPDATE BENCHMARK SUMMARY");
        println!("========================================");
        println!("  Total files processed : {}", self.total_files());
        println!("  Already up to date    : {}", self.skipped);
        println!("  Applied patches       : {}", self.patched);
        println!("  Full downloads        : {}", self.full_downloads);
        println!(
            "  Patch data downloaded : {}",
            human_bytes(self.patch_bytes)
        );
        println!(
            "  Full data downloaded  : {}",
            human_bytes(self.full_bytes)
        );
        println!("  Streamed (large) files: {}", self.streamed);
        println!("  Total patch CPU time  : {:.2?}", self.patch_time);
        println!("========================================");
    }
}

/// Shared atomic accumulators used across concurrent tasks.
#[derive(Debug, Default)]
struct AtomicStats {
    skipped: AtomicU64,
    patched: AtomicU64,
    full_downloads: AtomicU64,
    patch_bytes: AtomicU64,
    full_bytes: AtomicU64,
    streamed: AtomicU64,
    patch_nanos: AtomicU64,
}

impl AtomicStats {
    fn snapshot(&self) -> UpdateStats {
        UpdateStats {
            skipped: self.skipped.load(Ordering::Relaxed),
            patched: self.patched.load(Ordering::Relaxed),
            full_downloads: self.full_downloads.load(Ordering::Relaxed),
            patch_bytes: self.patch_bytes.load(Ordering::Relaxed),
            full_bytes: self.full_bytes.load(Ordering::Relaxed),
            streamed: self.streamed.load(Ordering::Relaxed),
            patch_time: Duration::from_nanos(self.patch_nanos.load(Ordering::Relaxed)),
        }
    }
}

pub struct ProductUpdater {
    base_url: String,
    client: Client,
}

impl ProductUpdater {
    pub fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.to_string(),
            client: Client::new(),
        }
    }

    /// Fetch the server's root manifest listing all available products.
    pub async fn fetch_root(&self) -> Result<RootJson> {
        let url = format!("{}root.json", self.base_url);
        self.client
            .get(&url)
            .send()
            .await
            .context("Failed to reach update server")?
            .json()
            .await
            .context("Failed to parse root.json")
    }

    /// Read the locally installed version for a product, if any.
    pub fn get_local_version(product: &str) -> Option<String> {
        let path = PathBuf::from("products").join(product).join("version.json");
        let data = fs::read_to_string(path).ok()?;
        let json: serde_json::Value = serde_json::from_str(&data).ok()?;
        json["version"].as_str().map(str::to_string)
    }

    /// Download and apply all file changes for `product_name` to reach `target_version`.
    /// Returns benchmark statistics for the completed run.
    pub async fn perform_update(
        &self,
        product_name: &str,
        target_version: &str,
    ) -> Result<UpdateStats> {
        let manifest = self.fetch_manifest(product_name, target_version).await?;

        let product_dir = PathBuf::from("products").join(product_name);
        fs::create_dir_all(&product_dir).context("Failed to create product directory")?;
        fs::create_dir_all("temp").context("Failed to create temp directory")?;

        let stats = Arc::new(AtomicStats::default());

        // Process all files concurrently, up to CONCURRENCY at a time.
        // Each file gets its own async task; patching runs on a dedicated
        // blocking thread via spawn_blocking so it never stalls the executor.
        let results = stream::iter(manifest.files)
            .map(|(rel_path, file_entry)| {
                let client = self.client.clone();
                let base_url = self.base_url.clone();
                let product_name = product_name.to_string();
                let version = manifest.version.clone();
                let product_dir = product_dir.clone();
                let stats = Arc::clone(&stats);

                async move {
                    update_file(
                        &client,
                        &base_url,
                        &product_name,
                        &version,
                        &product_dir,
                        &rel_path,
                        &file_entry,
                        &stats,
                    )
                    .await
                }
            })
            .buffer_unordered(CONCURRENCY)
            .collect::<Vec<_>>()
            .await;

        // Propagate the first error encountered, if any.
        for result in results {
            result?;
        }

        // Persist the new version marker.
        let version_json =
            serde_json::to_string_pretty(&serde_json::json!({ "version": manifest.version }))?;
        fs::write(product_dir.join("version.json"), version_json)
            .context("Failed to write version.json")?;

        let _ = fs::remove_dir_all("temp");

        Ok(Arc::try_unwrap(stats)
            .expect("stats Arc still has other owners")
            .snapshot())
    }

    async fn fetch_manifest(&self, product_name: &str, version: &str) -> Result<Manifest> {
        let url = format!(
            "{}products/{}/{}/manifest.json",
            self.base_url, product_name, version
        );
        self.client
            .get(&url)
            .send()
            .await
            .with_context(|| format!("Failed to fetch manifest for {} v{}", product_name, version))?
            .json()
            .await
            .context("Failed to parse manifest.json")
    }
}

/// Compute the BLAKE3 hash of a file on disk.
fn file_hash(path: &Path) -> Result<String> {
    let mut file = fs::File::open(path)
        .with_context(|| format!("Failed to open file for hashing: {}", path.display()))?;
    let mut hasher = blake3::Hasher::new();
    std::io::copy(&mut file, &mut hasher)?;
    Ok(hasher.finalize().to_hex().to_string())
}

/// Apply an HDiffPatch binary patch using the compiled C library.
///
/// Supports in-place patching (old_path == out_path): the C wrapper writes
/// to a temp file and renames it over the original.
///
/// Each call spawns its own blocking OS thread via `spawn_blocking`, so
/// multiple patches run truly in parallel without blocking the async executor.
/// Returns `(return_code, elapsed)`.
async fn apply_patch(
    old_path: PathBuf,
    patch_path: PathBuf,
    out_path: PathBuf,
) -> Result<(i32, Duration)> {
    tokio::task::spawn_blocking(move || {
        let t0 = Instant::now();

        // Safe in-place patching: write to a .tmp file if paths match
        let is_in_place = old_path == out_path;
        let actual_out_path = if is_in_place {
            out_path.with_extension("tmp_patch")
        } else {
            out_path.clone()
        };

        // Convert PathBuf to String for HDiff::new
        let old_str = old_path.to_string_lossy().to_string();
        let patch_str = patch_path.to_string_lossy().to_string();
        let out_str = actual_out_path.to_string_lossy().to_string();

        // Initialize and run the Rust patcher
        let mut patcher = HDiff::new(old_str, patch_str, out_str);
        let success = patcher.apply();

        // Handle successful in-place temp file renaming
        if success && is_in_place {
            if let Err(e) = std::fs::rename(&actual_out_path, &out_path) {
                eprintln!("[apply_patch] Failed to rename temp file over original: {}", e);
                return Ok((1, t0.elapsed())); // 1 = failure
            }
        } else if !success && is_in_place {
            // Clean up the temp file if the patch failed
            let _ = std::fs::remove_file(&actual_out_path);
        }

        // Map boolean success to the i32 return code (0 = success, 1 = error)
        let ret = if success { 0 } else { 1 };

        Ok((ret, t0.elapsed()))
    })
        .await
        .context("Patch task panicked")?
}

/// Download a URL and write it to `dest`.
///
/// `known_size` is the expected file size from the manifest (used to decide
/// whether to buffer the response in RAM or stream it directly to disk).
/// Returns the number of bytes written and whether streaming was used.
async fn download_to(
    client: &Client,
    url: &str,
    dest: &Path,
    known_size: u64,
) -> Result<(u64, bool)> {
    let response = client
        .get(url)
        .send()
        .await
        .with_context(|| format!("Failed to download {}", url))?;

    let streamed = known_size >= STREAM_THRESHOLD;

    if !streamed {
        // Small file: buffer entirely in RAM, then write atomically.
        let bytes = response
            .bytes()
            .await
            .with_context(|| format!("Failed to read response body from {}", url))?;
        let len = bytes.len() as u64;
        fs::write(dest, bytes)
            .with_context(|| format!("Failed to write {}", dest.display()))?;
        Ok((len, false))
    } else {
        // Large file: stream directly to disk to avoid high memory usage.
        let mut file = fs::File::create(dest)
            .with_context(|| format!("Failed to create {}", dest.display()))?;
        let mut total = 0u64;
        let mut stream = response.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.with_context(|| format!("Stream error from {}", url))?;
            total += chunk.len() as u64;
            std::io::Write::write_all(&mut file, &chunk)
                .with_context(|| format!("Failed to write chunk to {}", dest.display()))?;
        }
        Ok((total, true))
    }
}

/// Ensure a single product file is at the correct version and record stats.
///
/// Strategy (in order of preference):
/// 1. **Skip** — file already matches the expected hash -> nothing to do.
/// 2. **Binary patch** — a `.patch` file is available and the old file exists ->
///    download the patch, apply it via the HDiffPatch C library, verify hash.
/// 3. **Full download** — fallback when no patch is available or the patch fails.
#[allow(clippy::too_many_arguments)]
async fn update_file(
    client: &Client,
    base_url: &str,
    product_name: &str,
    version: &str,
    product_dir: &Path,
    rel_path: &str,
    entry: &FileEntry,
    stats: &AtomicStats,
) -> Result<()> {
    let dest = product_dir.join(rel_path);

    // Ensure parent directories exist
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create directory {}", parent.display()))?;
    }

    // Already up to date?
    if dest.exists() && file_hash(&dest).unwrap_or_default() == entry.hash {
        stats.skipped.fetch_add(1, Ordering::Relaxed);
        return Ok(());
    }

    // Binary patch available and old file present?
    if let (Some(patch_info), true) = (&entry.patch, dest.exists()) {
        let patch_url = format!(
            "{}products/{}/{}/{}",
            base_url, product_name, version, patch_info.file
        );

        let safe_temp_name = rel_path.replace("/", "_").replace("\\", "_");
        let patch_dest = PathBuf::from("temp").join(format!("{}.patch", safe_temp_name));

        let (patch_bytes, _) = download_to(client, &patch_url, &patch_dest, 0).await?;

        let (ret, elapsed) =
            apply_patch(dest.clone(), patch_dest.clone(), dest.clone()).await?;
        let _ = fs::remove_file(&patch_dest);

        if ret == 0 && file_hash(&dest).unwrap_or_default() == entry.hash {
            stats.patched.fetch_add(1, Ordering::Relaxed);
            stats.patch_bytes.fetch_add(patch_bytes, Ordering::Relaxed);
            stats
                .patch_nanos
                .fetch_add(elapsed.as_nanos() as u64, Ordering::Relaxed);
            return Ok(());
        }
    }

    // Full download fallback
    let full_url = format!(
        "{}products/{}/{}/full/{}",
        base_url, product_name, version, rel_path
    );
    let (full_bytes, streamed) = download_to(client, &full_url, &dest, entry.size).await?;

    stats.full_downloads.fetch_add(1, Ordering::Relaxed);
    stats.full_bytes.fetch_add(full_bytes, Ordering::Relaxed);
    if streamed {
        stats.streamed.fetch_add(1, Ordering::Relaxed);
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Formatting helpers
// ---------------------------------------------------------------------------

fn human_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}
