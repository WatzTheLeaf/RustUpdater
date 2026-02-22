use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
#[command(disable_version_flag = true)]
pub struct  Args {
    /// Game name (key)
    #[arg(short, long)]
    pub game: String,

    /// Path to the directory containing the NEW version files
    #[arg(short = 'n', long)]
    pub new_dir: PathBuf,

    /// The new version string (e.g., "1.0.1")
    #[arg(short = 'v', long)]
    pub version: String,

    /// Path to the game executable relative to game root
    #[arg(short, long)]
    pub exe: String,

    /// Output root folder (where games/ and root.json will be stored)
    #[arg(short = 'o', long)]
    pub output: PathBuf,
}