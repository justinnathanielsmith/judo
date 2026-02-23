use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct RecentFilters {
    pub filters: Vec<String>,
}

#[must_use]
pub fn get_config_path() -> Option<PathBuf> {
    home::home_dir().map(|mut path| {
        path.push(".config");
        path.push("judo");
        path.push("recent_filters.toml");
        path
    })
}

#[must_use]
pub fn load_recent_filters() -> Vec<String> {
    if let Some(path) = get_config_path() {
        if path.exists() {
            match std::fs::read_to_string(&path) {
                Ok(content) => match toml::from_str::<RecentFilters>(&content) {
                    Ok(recent) => return recent.filters,
                    Err(e) => eprintln!("Failed to parse {}: {}", path.display(), e),
                },
                Err(e) => eprintln!("Failed to read {}: {}", path.display(), e),
            }
        }
    }
    Vec::new()
}

pub fn save_recent_filters(filters: &[String]) {
    if let Some(path) = get_config_path() {
        if let Some(parent) = path.parent() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                eprintln!("Failed to create directory {}: {}", parent.display(), e);
                return;
            }
        }

        let recent = RecentFilters {
            filters: filters.to_vec(),
        };

        match toml::to_string(&recent) {
            Ok(content) => {
                if let Err(e) = std::fs::write(&path, content) {
                    eprintln!("Failed to write {}: {}", path.display(), e);
                }
            }
            Err(e) => eprintln!("Failed to serialize recent filters: {}", e),
        }
    }
}
