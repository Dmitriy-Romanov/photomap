use std::path::PathBuf;

/// Returns the cross-platform directory for application data.
pub fn get_app_data_dir() -> PathBuf {
    if cfg!(target_os = "macos") {
        let home_dir = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let mut path = PathBuf::from(home_dir);
        path.push("Library");
        path.push("Application Support");
        path.push("PhotoMap");
        path
    } else if cfg!(target_os = "windows") {
        if let Ok(appdata) = std::env::var("APPDATA") {
            let mut path = PathBuf::from(appdata);
            path.push("PhotoMap");
            path
        } else {
            PathBuf::from(".").join("PhotoMap")
        }
    } else if let Ok(xdg_data_home) = std::env::var("XDG_DATA_HOME") {
        let mut path = PathBuf::from(xdg_data_home);
        path.push("PhotoMap");
        path
    } else {
        let home_dir = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let mut path = PathBuf::from(home_dir);
        path.push(".local");
        path.push("share");
        path.push("PhotoMap");
        path
    }
}

/// Ensures the directory exists, creating it if necessary.
pub fn ensure_directory_exists(path: &PathBuf) -> Result<(), std::io::Error> {
    if !path.exists() {
        std::fs::create_dir_all(path)?;
    }
    Ok(())
}

/// Returns the path to the application configuration file.
pub fn get_config_path() -> PathBuf {
    let mut config_dir = get_app_data_dir();
    config_dir.push("photomap.ini");
    config_dir
}
