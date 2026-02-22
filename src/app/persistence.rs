use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct RecentFilters {
    pub filters: Vec<String>,
}

pub fn get_config_path() -> Option<PathBuf> {
    home::home_dir().map(|mut path| {
        path.push(".config");
        path.push("judo");
        path.push("recent_filters.toml");
        path
    })
}

pub fn load_recent_filters() -> Vec<String> {
    if let Some(path) = get_config_path() {
        if path.exists() {
            if let Ok(content) = std::fs::read_to_string(path) {
                if let Ok(recent) = toml::from_str::<RecentFilters>(&content) {
                    return recent.filters;
                }
            }
        }
    }
    Vec::new()
}

pub fn save_recent_filters(filters: &[String]) {
    if let Some(path) = get_config_path() {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        let recent = RecentFilters {
            filters: filters.to_vec(),
        };

        if let Ok(content) = toml::to_string(&recent) {
            let _ = std::fs::write(path, content);
        }
    }
}
