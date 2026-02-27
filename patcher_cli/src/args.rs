use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
#[command(disable_version_flag = true)]
pub struct  Args {
    /// Product name (key)
    #[arg(short, long)]
    pub product: String,

    /// Path to the directory containing the NEW version files
    #[arg(short = 'n', long)]
    pub new_dir: PathBuf,

    /// The new version string (e.g., "1.0.1")
    #[arg(short = 'v', long)]
    pub version: String,

    /// Path to the product executable relative to product root
    #[arg(short, long)]
    pub exe: String,

    /// Output root folder (where products/ and root.json will be stored)
    #[arg(short = 'o', long)]
    pub output: PathBuf,
}