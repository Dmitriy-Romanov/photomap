use std::path::{Path, PathBuf};
use anyhow::{Result, Context};
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::fs::{File, OpenOptions};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub last_folder: Option<String>,
    pub port: u16,
    #[serde(default)]
    pub auto_open_browser: bool,
    pub info_panel_width: u8,
    #[serde(default)]
    pub show_progress: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            last_folder: None,
            port: 3001,
            auto_open_browser: false,
            info_panel_width: 20,
            show_progress: true,
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
                if let Some(port_str) = config_map.get("port") {
            if let Ok(port) = port_str.parse::<u16>() {
                settings.port = port;
            }
        }
        if let Some(width_str) = config_map.get("info_panel_width") {
            if let Ok(width) = width_str.parse::<u8>() {
                settings.info_panel_width = width;
            }
        }
        if let Some(auto_open_str) = config_map.get("auto_open_browser") {
            if let Ok(auto_open) = auto_open_str.parse::<bool>() {
                settings.auto_open_browser = auto_open;
            }
        }
         if let Some(show_progress_str) = config_map.get("show_progress") {
            if let Ok(show_progress) = show_progress_str.parse::<bool>() {
                settings.show_progress = show_progress;
            }
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
                content.push_str(&format!("port = {}\n", self.port));
        content.push_str(&format!("info_panel_width = {}\n", self.info_panel_width));
        content.push_str(&format!("auto_open_browser = {}\n", self.auto_open_browser));
        content.push_str(&format!("show_progress = {}\n", self.show_progress));

        std::fs::write(&config_path, content).context("Failed to write to config file")?;
        Ok(())
    }

    pub fn update_last_folder<P: AsRef<Path>>(&mut self, folder_path: P) {
        self.last_folder = folder_path.as_ref().to_str().map(|s| s.to_string());
    }

    pub fn config_path() -> PathBuf {
        let mut path = std::env::current_exe()
            .unwrap_or_default()
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."))
            .to_path_buf();

        if path.ends_with("target/debug") || path.ends_with("target/release") {
            path.pop();
            path.pop();
        }
        path.push("photomap.ini");
        path
    }
}

