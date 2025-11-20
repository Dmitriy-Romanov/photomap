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

use std::process::Command;
use std::env;

pub fn select_folder_native() -> Option<String> {
    let os = env::consts::OS;

    match os {
        "macos" => {
            // MacOS: Используем AppleScript через osascript
            // Это создает нативное окно Finder, не блокируя основной поток сервера
            let script = "return POSIX path of (choose folder with prompt \"Выберите папку с фото\")";
            let output = Command::new("osascript")
                .arg("-e")
                .arg(script)
                .output()
                .ok()?;

            if output.status.success() {
                let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !path.is_empty() {
                    return Some(path);
                }
            }
        },
        "windows" => {
            // Windows: Используем PowerShell и .NET (System.Windows.Forms)
            // Работает на любой Windows 7/10/11 без установки лишнего софта
            let script = r#"
                [Console]::OutputEncoding = [System.Text.Encoding]::UTF8
                Add-Type -AssemblyName System.Windows.Forms
                
                $dummy = New-Object System.Windows.Forms.Form
                $dummy.TopMost = $true
                $dummy.StartPosition = "CenterScreen"
                $dummy.Opacity = 0
                $dummy.ShowInTaskbar = $false
                $dummy.Show()
                $dummy.Activate()
                
                $f = New-Object System.Windows.Forms.FolderBrowserDialog
                $f.Description = "Выберите папку с фото"
                $f.ShowNewFolderButton = $true
                
                if ($f.ShowDialog($dummy) -eq "OK") { Write-Host $f.SelectedPath }
                
                $dummy.Close()
                $dummy.Dispose()
            "#;
            
            let output = Command::new("powershell")
                .arg("-Sta") // Required for System.Windows.Forms
                .arg("-NoProfile") // Ускоряет запуск
                .arg("-Command")
                .arg(script)
                .output()
                .ok()?;

            if output.status.success() {
                let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !path.is_empty() {
                    return Some(path);
                }
            }
        },
        "linux" => {
            // Linux: Пробуем zenity или kdialog (стандартные утилиты)
            if let Ok(output) = Command::new("zenity").arg("--file-selection").arg("--directory").output() {
                if output.status.success() {
                    return Some(String::from_utf8_lossy(&output.stdout).trim().to_string());
                }
            }
            // Можно добавить fallback на kdialog, если нужно
        },
        _ => {}
    }
    
    None
}

/// Opens the specified URL in the default browser using native commands
pub fn open_browser(url: &str) -> Result<(), std::io::Error> {
    let os = env::consts::OS;
    match os {
        "macos" => {
            Command::new("open").arg(url).spawn()?;
        },
        "windows" => {
            Command::new("cmd").args(&["/C", "start", url]).spawn()?;
        },
        "linux" => {
            Command::new("xdg-open").arg(url).spawn()?;
        },
        _ => {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                format!("Unsupported OS: {}", os),
            ));
        }
    }
    Ok(())
}
