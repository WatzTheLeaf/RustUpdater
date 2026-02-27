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

    // Determine the version to install/update to.
    let mut target_version = entry.latest_version.clone();

    let should_proceed = if local_ver.is_none() {
        println!("\n{} is not installed.", product_name);

        // Let the user pick a specific version for a fresh install.
        if !entry.versions.is_empty() {
            println!("Available versions for a fresh install:");
            for (i, v) in entry.versions.iter().enumerate() {
                println!("  {}. {}", i + 1, v);
            }
            print!(
                "Select a version number (or press Enter for latest [{}]): ",
                entry.latest_version
            );
            io::stdout().flush()?;

            let mut v_input = String::new();
            io::stdin().read_line(&mut v_input)?;
            let v_choice: usize = v_input.trim().parse().unwrap_or(0);
            if v_choice > 0 && v_choice <= entry.versions.len() {
                target_version = entry.versions[v_choice - 1].clone();
            }
        }

        print!("Proceed to install version {}? (y/n): ", target_version);
        true
    } else if local_ver.as_ref() != Some(&entry.latest_version) {
        let current = local_ver.unwrap();
        print!(
            "{} update available! ({} -> {}). Update now? (y/n): ",
            product_name, current, entry.latest_version
        );
        target_version = entry.latest_version.clone();
        true
    } else {
        println!(
            "{} is already up to date (Version {}).",
            product_name, entry.latest_version
        );
        return Ok(());
    };

    if should_proceed {
        io::stdout().flush()?;
        let mut confirm = String::new();
        io::stdin().read_line(&mut confirm)?;

        if confirm.trim().to_lowercase() == "y" {
            println!("\nStarting process for {} v{}...", product_name, target_version);

            let wall_start = Instant::now();

            match updater.perform_update(product_name, &target_version).await {
                Ok(stats) => {
                    let wall_time = wall_start.elapsed();
                    println!("\nInstallation/Update finished successfully!");
                    stats.print_summary();
                    println!("  Total wall-clock time : {:.2?}", wall_time);
                    println!("========================================");
                }
                Err(e) => {
                    eprintln!("Update failed: {}", e);
                }
            }
        } else {
            println!("Operation cancelled.");
        }
    }

    Ok(())
}
