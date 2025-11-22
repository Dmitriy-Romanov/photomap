use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub last_folder: Option<String>,
    pub start_browser: bool,
    pub top: i32,
    pub left: i32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            last_folder: None,
            start_browser: true,
            top: 12,
            left: 52,
        }
    }
}

impl Settings {
    pub fn load() -> Result<Self> {
        let config_path = Self::config_path();
        let mut settings = Settings::default();
        
        if !config_path.exists() {
            // Create default settings file
            settings.save().context("Failed to create default settings file")?;
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
        
        if let Some(start_browser) = config_map.get("start_browser") {
            if let Ok(val) = start_browser.trim().parse::<bool>() {
                settings.start_browser = val;
            }
        }

        if let Some(top) = config_map.get("top") {
            if let Ok(val) = top.trim().parse::<i32>() {
                settings.top = val;
            }
        }

        if let Some(left) = config_map.get("left") {
            if let Ok(val) = left.trim().parse::<i32>() {
                settings.left = val;
            }
        }

        // If file exists but some fields are missing, save defaults back to file
        let needs_save = !config_map.contains_key("top") || !config_map.contains_key("left");
        if needs_save {
            println!("⚠️  Settings file missing 'top' or 'left', writing defaults...");
            if let Err(e) = settings.save() {
                eprintln!("Failed to save default settings: {}", e);
            }
        }

        Ok(settings)
    }

    pub fn save(&self) -> Result<()> {
        let config_path = Self::config_path();
        println!("💾 Saving settings to: {:?}", config_path);
        println!("💾 Settings values: top={}, left={}, start_browser={}", self.top, self.left, self.start_browser);
        
        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent).context("Creating config directory")?;
        }

        let _file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&config_path)?;

        let mut content = String::new();
        content.push_str("# PhotoMap Configuration File\n");

        if let Some(ref last_folder) = self.last_folder {
            content.push_str(&format!("last_folder = \"{}\"\n", last_folder));
        }
        
        content.push_str(&format!("start_browser = {}\n", self.start_browser));
        content.push_str(&format!("top = {}\n", self.top));
        content.push_str(&format!("left = {}\n", self.left));

        std::fs::write(&config_path, content).context("Failed to write to config file")?;
        println!("✅ Settings saved successfully");
        Ok(())
    }

    pub fn config_path() -> PathBuf {
        let app_dir = crate::utils::get_app_data_dir();

        // Create directory if it doesn't exist
        let _ = crate::utils::ensure_directory_exists(&app_dir);

        crate::utils::get_config_path()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs;

    #[test]
    fn test_settings_creation() {
        // Create a temp directory to act as HOME
        let mut temp_path = env::temp_dir();
        temp_path.push("photomap_test_settings");
        
        // Clean up previous run if exists
        if temp_path.exists() {
            fs::remove_dir_all(&temp_path).unwrap();
        }
        fs::create_dir_all(&temp_path).unwrap();
        
        // Override HOME/APPDATA/XDG_DATA_HOME based on OS to point to temp dir
        // For this test, we'll just set all potentially used vars to be safe
        unsafe {
            env::set_var("HOME", &temp_path);
            env::set_var("APPDATA", &temp_path);
            env::set_var("XDG_DATA_HOME", &temp_path);
        }

        // Ensure the file doesn't exist yet
        let config_path = Settings::config_path();
        assert!(!config_path.exists());

        // Load settings - this should trigger creation
        let settings = Settings::load();
        assert!(settings.is_ok());

        // Verify file exists now
        assert!(config_path.exists());

        // Verify content
        let content = fs::read_to_string(config_path).unwrap();
        assert!(content.contains("# PhotoMap Configuration File"));
        
        // Cleanup
        let _ = fs::remove_dir_all(&temp_path);
    }
}
