# Tauri Delta Updater & Launcher

A high-performance, cross-platform game and application launcher built with Tauri, Vue, and Rust. It features smart delta-patching, asynchronous file streaming, cryptographic hash verification, and automatic disk space checking.

The system is split into two parts:
1. **The Patcher (CLI):** A build tool that compares versions and generates binary patches.
2. **The Launcher (Tauri):** A desktop app that downloads updates, applies patches, and launches your products.

---

## ✨ Features
* **Smart Delta Patching:** Uses `HDiffPatch` to generate tiny binary patches instead of forcing users to redownload huge files.
* **Safe & Non-Blocking:** Built heavily on `tokio` for async I/O, ensuring UI responsiveness even during gigabyte-sized downloads.
* **Integrity Checking:** Uses `BLAKE3` hashing to guarantee every downloaded and patched file is 100% correct.
* **Pre-Update Disk Checks:** Automatically calculates peak disk usage (including temporary patch files) and blocks the update if space is insufficient.
* **Multi-Product Support:** Host multiple games or applications from a single update server.
* **Static Hosting:** The server requires zero backend logic. Just serve the generated output folder via NGINX, Apache, AWS S3, or GitHub Pages.

---

## Part 1: Generating Updates (The Patcher CLI)

Whenever you have a new version of your app or game, you use the Patcher CLI to process the files. It will automatically detect previous versions, calculate the file differences, and generate `.patch` files.

### Usage
Run the patcher with the following arguments:

```bash
cargo run --bin patcher -- \
  --product "my_game" \
  --input "./builds/v1.0.1" \
  --version "1.0.1" \
  --exe "game_executable.exe" \
  --output "./server_public_dir"
```

#### Arguments breakdown:
- `-p, --product`: The unique ID/Name of the product.
- `-i, --input`: The directory containing your fresh, compiled files for this specific version.
- `-v, --version`: The version string (e.g., 1.0.0, 1.0.1).
- `-e, --exe`: The name of the executable file to launch (relative to the input root).
- `-o, --output`: The destination folder. This is the folder you will upload to your web server.

### How it works
1. Fresh Install (v1.0.0): It copies your files to `products/my_game/1.0.0/full/`, generates a `manifest.json`, and updates the global `root.json`.
2. Update (v1.0.1): If it sees `1.0.0` already sitting in the output folder, it compares the new files against the old ones. It copies the new files to `1.0.1/full/`, but also generates delta patches in `1.0.1/patches/`.

### Hosting
Upload the entire contents of your `--output` directory to any static web server. Ensure your server is configured to allow downloading `.jsonè , `.patch`, and binary files.

## Part 2: The Launcher Setup
The Launcher is a Tauri application that reads your static server, downloads exactly what it needs, and boots the game.

### Configuration
Set your server URL and default installation directory:

```
fn main() {
    // Set your public HTTP server URL here (MUST end with a trailing slash '/')
    let default_url = "https://server.com/updates";
    
    // Set where you want the products to be installed on the user's computer
    let default_install_dir = std::env::current_dir().unwrap().join("InstalledSoftwares");
    
...
```

## Architecture Details

- Manifests: Every version has a manifest.json containing the BLAKE3 hash and file size of every asset. The launcher uses this to verify integrity and know exactly what to download.
- Update Strategy: The launcher evaluates the cost of downloading all sequential patches vs. doing a full fresh download. If patches are cheaper, it applies them sequentially. If patching fails (e.g., a hash mismatch), it safely falls back to a full file download.
- Dynamic Temp Directory: All temporary downloads and patching operations occur inside an invisible .temp folder inside the installation directory. This prevents cross-drive I/O bottlenecks.