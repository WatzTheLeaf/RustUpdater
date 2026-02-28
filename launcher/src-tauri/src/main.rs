// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::PathBuf;
use std::process::Command;
use tauri::{Emitter, AppHandle};
use updater::models::RootJson;
use updater::ProductUpdater;
use serde::Serialize;

#[derive(Clone, Serialize)]
struct ProgressPayload {
    current: usize,
    total: usize,
    percent: f64,
}

/// Validates the server URL, ensuring it ends with '/'
#[tauri::command]
fn validate_server_url(mut url: String) -> Result<String, String> {
    if url.trim().is_empty() {
        return Err(String::from("server url is empty"));
    }
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err("URL must start with http:// or https://".into());
    }
    if !url.ends_with('/') {
        url.push('/');
    }
    Ok(url)
}

#[tauri::command]
async fn fetch_root(server_url: String) -> Result<RootJson, String> {
    let updater = ProductUpdater::new(&server_url);
    updater.fetch_root().await.map_err(|e| e.to_string())
}

#[tauri::command]
fn get_local_version(product_name: String) -> Option<String> {
    ProductUpdater::get_local_version(&product_name)
}

#[tauri::command]
async fn run_update(
    app: AppHandle,
    server_url: String,
    product_name: String,
    target_version: String,
    available_versions: Vec<String>,
) -> Result<String, String> {
    let updater = ProductUpdater::new(&server_url);
    let _ = app.emit("log", format!("Starting update for {} to v{}...", product_name, target_version));

    let app_clone = app.clone();

    let progress_callback = move |current: usize, total: usize| {
        let percent = if total > 0 { (current as f64 / total as f64) * 100.0 } else { 100.0 };
        let payload = ProgressPayload { current, total, percent };
        let _ = app_clone.emit("progress", payload);
    };

    match updater.perform_update(&product_name, &target_version, &available_versions, progress_callback).await {
        Ok(_) => {
            let _ = app.emit("log", "Update finished successfully!".to_string());
            Ok("Success".into())
        }
        Err(e) => {
            let err_msg = format!("Update failed: {}", e);
            let _ = app.emit("log", err_msg.clone());
            Err(err_msg)
        }
    }
}

#[tauri::command]
async fn verify_integrity(
    app: AppHandle,
    server_url: String,
    product_name: String,
    version: String,
) -> Result<Vec<String>, String> {
    let updater = ProductUpdater::new(&server_url);
    let _ = app.emit("log", format!("Verifying files for {} v{}...", product_name, version));

    let app_clone = app.clone();
    let progress_callback = move |current: usize, total: usize| {
        let percent = if total > 0 { (current as f64 / total as f64) * 100.0 } else { 100.0 };
        let payload = ProgressPayload { current, total, percent };
        let _ = app_clone.emit("progress", payload);
    };

    match updater.verify_integrity(&product_name, &version, progress_callback).await {
        Ok(corrupted) => {
            if corrupted.is_empty() {
                let _ = app.emit("log", "Integrity check passed! All files 100% correct.".to_string());
            } else {
                let _ = app.emit("log", format!("CRITICAL: Found {} corrupted files.", corrupted.len()));
            }
            Ok(corrupted)
        }
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
async fn launch_product(
    server_url: String,
    product_name: String,
) -> Result<String, String> {
    let updater = ProductUpdater::new(&server_url);

    // Get the currently installed version
    let local_ver = ProductUpdater::get_local_version(&product_name)
        .ok_or("Product is not installed.")?;

    // Fetch the manifest to see which exe to run
    let manifest = updater.fetch_manifest(&product_name, &local_ver).await
        .map_err(|e| format!("Failed to read manifest: {}", e))?;

    if manifest.exe.is_empty() {
        return Err("No executable specified in the server manifest.".into());
    }

    // Build the path to the executable
    let exe_path = PathBuf::from("products")
        .join(&product_name)
        .join(&manifest.exe);

    if !exe_path.exists() {
        return Err(format!("Executable not found at: {}", exe_path.display()));
    }

    // Launch
    Command::new(&exe_path)
        .current_dir(exe_path.parent().unwrap())
        .spawn()
        .map_err(|e| format!("Failed to launch product: {}", e))?;

    Ok("Launched successfully!".into())
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            validate_server_url,
            fetch_root,
            get_local_version,
            run_update,
            verify_integrity,
            launch_product
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
