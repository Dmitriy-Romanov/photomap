use std::path::PathBuf;
use anyhow::{Result, Context};
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::fs::{File, OpenOptions};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub last_folder: Option<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            last_folder: None,
        }
    }
}

impl Settings {
    pub fn load() -> Result<Self> {
        let config_path = Self::config_path();
        let mut settings = Settings::default();
        if !config_path.exists() {
            return Ok(settings);
        }

        let file = File::open(&config_path).context("Failed to open config file")?;
        let reader = BufReader::new(file);
        let mut config_map = HashMap::new();

        for line in reader.lines() {
            let line = line.context("Failed to read line from config")?;
            if line.starts_with('#') || line.trim().is_empty() {
                continue;
            }
            if let Some((key, value)) = line.split_once('=') {
                config_map.insert(key.trim().to_string(), value.trim().to_string());
            }
        }

        if let Some(last_folder) = config_map.get("last_folder") {
            settings.last_folder = Some(last_folder.trim_matches('"').to_string());
        }
                                         
        Ok(settings)
    }

    pub fn save(&self) -> Result<()> {
        let config_path = Self::config_path();
        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent).context("Creating config directory")?;
        }

        let _file = OpenOptions::new().write(true).create(true).truncate(true).open(&config_path)?;

        let mut content = String::new();
        content.push_str("# PhotoMap Configuration File\n");

        if let Some(ref last_folder) = self.last_folder {
            content.push_str(&format!("last_folder = \"{}\"\n", last_folder));
        }
                
        std::fs::write(&config_path, content).context("Failed to write to config file")?;
        Ok(())
    }

    
    fn get_app_data_dir() -> PathBuf {
        // Cross-platform application data directory
        if cfg!(target_os = "macos") {
            let home_dir = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
            let mut path = PathBuf::from(home_dir);
            path.push("Library");
            path.push("Application Support");
            path.push("PhotoMap");
            path
        } else if cfg!(target_os = "windows") {
            // Use %APPDATA%/PhotoMap on Windows
            if let Ok(appdata) = std::env::var("APPDATA") {
                let mut path = PathBuf::from(appdata);
                path.push("PhotoMap");
                path
            } else {
                // Fallback to current directory
                PathBuf::from(".").join("PhotoMap")
            }
        } else {
            // Linux and others: use ~/.local/share/PhotoMap or ~/.photomap
            let home_dir = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
            let mut path = PathBuf::from(home_dir);

            // Try XDG_DATA_HOME first
            if let Ok(xdg_data) = std::env::var("XDG_DATA_HOME") {
                path = PathBuf::from(xdg_data);
            } else {
                path.push(".local");
                path.push("share");
            }
            path.push("PhotoMap");
            path
        }
    }

    pub fn config_path() -> PathBuf {
        let mut app_dir = Self::get_app_data_dir();

        // Create directory if it doesn't exist
        if !app_dir.exists() {
            let _ = std::fs::create_dir_all(&app_dir);
        }

        app_dir.push("photomap.ini");
        app_dir
    }
}

