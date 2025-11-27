use std::path::PathBuf;

/// Returns the cross-platform directory for application data
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

/// Ensures the directory exists, creating it if necessary
pub fn ensure_directory_exists(path: &PathBuf) -> Result<(), std::io::Error> {
    if !path.exists() {
        std::fs::create_dir_all(path)?;
    }
    Ok(())
}

/// Returns the path to the application configuration file
pub fn get_config_path() -> PathBuf {
    let mut config_dir = get_app_data_dir();
    config_dir.push("photomap.ini");
    config_dir
}



use std::process::Command;
use std::env;

/// Select multiple folders using native OS dialogs (max 5)
/// Returns a vector of selected folder paths (deduplicated)
pub fn select_folders_native() -> Vec<String> {
    let os = env::consts::OS;
    let mut folders = Vec::new();

    match os {
        "macos" => {
            // MacOS: AppleScript can select multiple items at once!
            let script = r#"
set folderList to choose folder with prompt "Select photo folders (max 5, Cmd+Click for multiple)" with multiple selections allowed
set pathList to {}
repeat with aFolder in folderList
    set end of pathList to POSIX path of aFolder
end repeat
return pathList
"#;
            
            if let Ok(output) = Command::new("osascript")
                .arg("-e")
                .arg(script)
                .output()
            {
                if output.status.success() {
                    let paths_str = String::from_utf8_lossy(&output.stdout);
                    // AppleScript returns comma-separated paths
                    folders = paths_str
                        .split(", ")
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .take(5)  // Limit to 5
                        .collect();
                }
            }
        },
        "windows" => {
            // Windows: Use rfd for native multi-select support
            #[cfg(windows)]
            {
                use rfd::FileDialog;
                
                if let Some(paths) = FileDialog::new()
                    .set_title("Select photo folders (max 5, Ctrl+Click for multiple)")
                    .pick_folders()
                {
                    folders = paths
                        .into_iter()
                        .map(|p| p.to_string_lossy().to_string())
                        .take(5)
                        .collect();
                }
            }
            
            /* OLD PowerShell implementation (sequential dialogs, kept for reference)
            let mut attempt = 0;
            while attempt < 5 {
                let prompt = if folders.is_empty() {
                    "Select folder 1 (max 5)".to_string()
                } else {
                    format!("Add folder {}? (Cancel = Done)", folders.len() + 1)
                };
                
                let script = format!(r#"
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
                    $f.Description = "{}"
                    $f.ShowNewFolderButton = $true
                    
                    if ($f.ShowDialog($dummy) -eq "OK") {{ Write-Host $f.SelectedPath }}
                    
                    $dummy.Close()
                    $dummy.Dispose()
                "#, prompt);
                
                if let Ok(output) = Command::new("powershell")
                    .arg("-Sta")
                    .arg("-NoProfile")
                    .arg("-Command")
                    .arg(&script)
                    .output()
                {
                    if output.status.success() {
                        let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                        if !path.is_empty() {
                            folders.push(path);
                            attempt += 1;
                        } else {
                            // User cancelled
                            break;
                        }
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            }
            */
        },
        "linux" => {
            // Linux: Use zenity with --multiple flag
            if let Ok(output) = Command::new("zenity")
                .arg("--file-selection")
                .arg("--directory")
                .arg("--multiple")
                .arg("--separator=|")
                .arg("--title=Select photo folders (max 5)")
                .output()
            {
                if output.status.success() {
                    let paths_str = String::from_utf8_lossy(&output.stdout);
                    folders = paths_str
                        .split('|')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .take(5)
                        .collect();
                }
            }
        },
        _ => {}
    }
    
    // Deduplicate folders (in case user selected same folder multiple times)
    let mut unique_folders = Vec::new();
    for folder in folders {
        if !unique_folders.contains(&folder) {
            unique_folders.push(folder);
        }
    }
    
    unique_folders
}

/// Opens the specified URL in the default browser using native commands
pub fn open_browser(url: &str) -> Result<(), std::io::Error> {
    let os = env::consts::OS;
    match os {
        "macos" => {
            Command::new("open").arg(url).spawn()?;
        },
        "windows" => {
            Command::new("cmd").args(["/C", "start", url]).spawn()?;
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
