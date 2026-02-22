use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct RootJson {
    #[serde(default)]
    pub games: HashMap<String, GameEntry>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GameEntry {
    pub latest_version: String,
    pub manifest: String,
    #[serde(default)]
    pub versions: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Manifest {
    pub version: String,
    pub files: HashMap<String, FileEntry>,
    #[serde(default)]
    pub deleted_files: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct FileEntry {
    pub hash: String,
    pub size: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub patch: Option<PatchInfo>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PatchInfo {
    pub file: String,
}