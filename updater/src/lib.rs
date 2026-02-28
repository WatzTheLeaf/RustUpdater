extern crate zstd; // Keep this to avoid compiler stripping it

pub mod models;
pub mod patchers;
pub mod updater;
pub mod utils;

// Re-export primary interface
pub use updater::ProductUpdater;