extern crate zstd; // To avoid compilator to strip it

mod models;
mod updater;

pub mod patchers;
pub mod utils;

use anyhow::Result;
use std::io::{self, Write};
use std::time::Instant;
use updater::ProductUpdater;

#[tokio::main]
async fn main() -> Result<()> {
    let server_url = "http://127.0.0.1:3000/";
    let updater = ProductUpdater::new(server_url);

    println!("--- Custom Updater Tester ---");
    println!("Fetching root.json from server...");

    let root = match updater.fetch_root().await {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Failed to connect to server: {}", e);
            return Ok(());
        }
    };

    if root.products.is_empty() {
        println!("No products found on server.");
        return Ok(());
    }

    println!("\nAvailable Products:");
    let mut products_list: Vec<(&String, &models::ProductEntry)> = root.products.iter().collect();
    products_list.sort_by_key(|k| k.0);

    for (i, (name, entry)) in products_list.iter().enumerate() {
        let local_ver =
            ProductUpdater::get_local_version(name).unwrap_or_else(|| "Not Installed".to_string());
        println!(
            "{}. {} (Local: {} | Latest: {})",
            i + 1,
            name,
            local_ver,
            entry.latest_version
        );
    }

    print!("\nEnter the number of the product to manage (or 0 to quit): ");
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let choice: usize = input.trim().parse().unwrap_or(0);

    if choice == 0 || choice > products_list.len() {
        println!("Exiting.");
        return Ok(());
    }

    let (product_name, entry) = products_list[choice - 1];
    let local_ver = ProductUpdater::get_local_version(product_name);

    let mut target_version = entry.latest_version.clone();
    let mut needs_update = false;

    if local_ver.is_none() {
        println!("\n{} is not installed.", product_name);
        if !entry.versions.is_empty() {
            println!("Available versions for a fresh install:");
            for (i, v) in entry.versions.iter().enumerate() {
                println!("  {}. {}", i + 1, v);
            }
            print!("Select a version number (or press Enter for latest [{}]): ", entry.latest_version);
            io::stdout().flush()?;
            let mut v_input = String::new();
            io::stdin().read_line(&mut v_input)?;
            let v_choice: usize = v_input.trim().parse().unwrap_or(0);
            if v_choice > 0 && v_choice <= entry.versions.len() {
                target_version = entry.versions[v_choice - 1].clone();
            }
        }
        print!("Proceed to install version {}? (y/n): ", target_version);
        needs_update = true;
    } else if local_ver.as_ref() != Some(&entry.latest_version) {
        println!("{} update available! ({} -> {})", product_name, local_ver.as_ref().unwrap(), entry.latest_version);
        print!("Update now? (y/n): ");
        needs_update = true;
    } else {
        println!("{} is already up to date (Version {}).", product_name, entry.latest_version);
        print!("Would you like to verify file integrity? (y/n): ");
        needs_update = false; // We use the same 'y' logic to trigger verification
    }

    io::stdout().flush()?;
    let mut confirm = String::new();
    io::stdin().read_line(&mut confirm)?;

    if confirm.trim().to_lowercase() == "y" {
        if needs_update {
            println!("\nStarting update process for {} v{}...", product_name, target_version);
            let wall_start = Instant::now();

            match updater.perform_update(product_name, &target_version, &entry.versions).await {
                Ok(_) => {
                    println!("\nUpdate finished successfully in {:.2?}.", wall_start.elapsed());
                }
                Err(e) => {
                    eprintln!("Update failed: {}", e);
                    return Ok(());
                }
            }
        }

        // Integrity check
        println!("\nRunning integrity check...");
        let check_start = Instant::now();
        // Use the target version (the one we just installed) or the local version
        let version_to_check = ProductUpdater::get_local_version(product_name).unwrap_or(target_version);

        match updater.verify_integrity(product_name, &version_to_check).await {
            Ok(corrupted) => {
                if corrupted.is_empty() {
                    println!("Integrity check passed! All files are 100% correct.");
                } else {
                    println!("CRITICAL: Found {} corrupted or missing files:", corrupted.len());
                    for file in corrupted {
                        println!("  [!] {}", file);
                    }
                    println!("Suggestion: Run the update again to repair these files.");
                }
            }
            Err(e) => eprintln!("Integrity check failed to run: {}", e),
        }
        println!("Integrity check took: {:.2?}", check_start.elapsed());

    } else {
        println!("Operation cancelled.");
    }

    Ok(())
}