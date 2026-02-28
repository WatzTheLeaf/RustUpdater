use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct RootJson {
    #[serde(default)]
    pub products: HashMap<String, ProductEntry>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProductEntry {
    pub latest_version: String,
    pub manifest: String,
    #[serde(default)]
    pub versions: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Manifest {
    pub version: String,
    #[serde(default)]
    pub exe: String,
    #[serde(default)]
    pub total_patch_size: u64,
    pub files: HashMap<String, FileEntry>,
    #[serde(default)]
    pub deleted_files: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FileEntry {
    pub hash: String,
    pub size: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub patch: Option<PatchInfo>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PatchInfo {
    pub file: String,
}