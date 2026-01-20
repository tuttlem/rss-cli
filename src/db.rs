use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct FeedDb {
    pub feeds: Vec<FeedRecord>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct FeedRecord {
    pub title: Option<String>,
    pub url: String,
    pub items: Vec<FeedItem>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct FeedItem {
    pub title: String,
    pub link: Option<String>,
    pub published: Option<String>,
}

pub fn load_db(path: &Path) -> Result<FeedDb> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("failed to read database file {}", path.display()))?;
    match path.extension().and_then(|ext| ext.to_str()) {
        Some("json") => serde_json::from_str(&content)
            .with_context(|| format!("failed to parse JSON in {}", path.display())),
        Some("yml") | Some("yaml") => serde_yaml::from_str(&content)
            .with_context(|| format!("failed to parse YAML in {}", path.display())),
        other => Err(anyhow::anyhow!(
            "unsupported database extension {:?}; use .json, .yml, or .yaml",
            other
        )),
    }
}

pub fn save_db(path: &Path, db: &FeedDb) -> Result<()> {
    let serialized = match path.extension().and_then(|ext| ext.to_str()) {
        Some("json") => serde_json::to_string_pretty(db)
            .with_context(|| format!("failed to serialize JSON for {}", path.display()))?,
        Some("yml") | Some("yaml") => serde_yaml::to_string(db)
            .with_context(|| format!("failed to serialize YAML for {}", path.display()))?,
        other => {
            return Err(anyhow::anyhow!(
                "unsupported database extension {:?}; use .json, .yml, or .yaml",
                other
            ))
        }
    };
    fs::write(path, serialized)
        .with_context(|| format!("failed to write database file {}", path.display()))?;
    Ok(())
}
