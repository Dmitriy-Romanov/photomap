use std::path::PathBuf;

/// Возвращает кросс-платформенную директорию для данных приложения
pub fn get_app_data_dir() -> PathBuf {
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
        // Linux and other Unix-like systems
        if let Ok(xdg_data_home) = std::env::var("XDG_DATA_HOME") {
            let mut path = PathBuf::from(xdg_data_home);
            path.push("PhotoMap");
            path
        } else {
            // Fallback to ~/.local/share/PhotoMap
            let home_dir = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
            let mut path = PathBuf::from(home_dir);
            path.push(".local");
            path.push("share");
            path.push("PhotoMap");
            path
        }
    }
}

/// Убеждается, что директория существует, создавая её при необходимости
pub fn ensure_directory_exists(path: &PathBuf) -> Result<(), std::io::Error> {
    if !path.exists() {
        std::fs::create_dir_all(path)?;
    }
    Ok(())
}

/// Возвращает путь к файлу конфигурации приложения
pub fn get_config_path() -> PathBuf {
    let mut config_dir = get_app_data_dir();
    config_dir.push("photomap.ini");
    config_dir
}

/// Возвращает путь к файлу базы данных приложения
pub fn get_database_path() -> String {
    let mut db_dir = get_app_data_dir();
    db_dir.push("photomap.db");
    db_dir.to_string_lossy().to_string()
}