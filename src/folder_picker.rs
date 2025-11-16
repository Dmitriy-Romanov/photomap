use anyhow::Result;
use std::path::PathBuf;
use std::collections::HashMap;
use tokio::sync::{mpsc, Mutex};
use std::sync::Arc;

/// Folder selection with fallback for macOS threading issues
pub fn select_folder(initial_dir: Option<String>) -> Result<Option<PathBuf>> {
    // For macOS, we can't use rfd in async contexts due to main thread requirements
    #[cfg(target_os = "macos")]
    {
        // For now, return the current directory as a fallback
        // This allows the application to work while we implement a proper solution
        if let Some(dir) = initial_dir {
            if std::path::Path::new(&dir).exists() {
                return Ok(Some(PathBuf::from(dir)));
            }
        }

        // Fallback to user's home directory
        match dirs::home_dir() {
            Some(home) => Ok(Some(home)),
            None => Ok(Some(std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/")))),
        }
    }

    // For other platforms, use rfd normally
    #[cfg(not(target_os = "macos"))]
    {
        let mut dialog = rfd::FileDialog::new()
            .set_title("Выберите папку с фотографиями");

        if let Some(dir) = initial_dir {
            if let Ok(path) = std::path::Path::new(&dir).canonicalize() {
                if path.exists() {
                    dialog = dialog.set_directory(path);
                }
            }
        }

        Ok(dialog.pick_folder())
    }
}

/// Channel-based folder selection for async contexts
pub struct FolderRequestHandler {
    pub request_sender: mpsc::UnboundedSender<String>,
    pub response_handlers: Arc<Mutex<HashMap<String, tokio::sync::oneshot::Sender<Option<PathBuf>>>>>,
}

impl FolderRequestHandler {
    pub fn new() -> Self {
        let (request_sender, mut request_receiver) = mpsc::unbounded_channel::<String>();
        let response_handlers = Arc::new(Mutex::new(HashMap::<String, tokio::sync::oneshot::Sender<Option<PathBuf>>>::new()));

        // Spawn the folder selection handler task
        let response_handlers_clone = response_handlers.clone();
        tokio::spawn(async move {
            while let Some(request_id) = request_receiver.recv().await {
                println!("📁 Received folder request: {}", request_id);

                // Handle folder selection in this async context
                let selected_path = handle_folder_selection_async().await;

                // Send response back
                let mut handlers = response_handlers_clone.lock().await;
                if let Some(response_tx) = handlers.remove(&request_id) {
                    let _ = response_tx.send(selected_path);
                }
            }
        });

        Self {
            request_sender,
            response_handlers,
        }
    }

    pub async fn select_folder_async(&self) -> Result<Option<PathBuf>> {
        // Generate unique request ID
        let request_id = format!("request_{}", std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis());

        // Create response channel
        let (response_tx, response_rx) = tokio::sync::oneshot::channel::<Option<PathBuf>>();

        // Register response handler
        {
            let mut handlers = self.response_handlers.lock().await;
            handlers.insert(request_id.clone(), response_tx);
        }

        // Send request
        if let Err(_) = self.request_sender.send(request_id.clone()) {
            println!("❌ Failed to send folder request");
            return Ok(None);
        }

        // Wait for response
        match response_rx.await {
            Ok(path) => Ok(path),
            Err(_) => {
                println!("❌ Failed to receive folder selection response");
                Ok(None)
            }
        }
    }
}

async fn handle_folder_selection_async() -> Option<PathBuf> {
    println!("🗂️  Launching external folder dialog helper");

    // Try to launch the external helper program that can open a real folder dialog
    let helper_path = {
        // First, try to find the helper relative to the current directory
        let current_dir = std::env::current_dir().unwrap_or_default();
        let mut helper_path = current_dir.clone();

        // Check if we're in target/release or target/debug and go up to project root
        if helper_path.ends_with("target/release") || helper_path.ends_with("target/debug") {
            helper_path.pop(); // remove debug/release
            helper_path.pop(); // remove target
        }

        helper_path.push("folder_dialog_helper");
        helper_path.push("target");
        helper_path.push("release");
        helper_path.push(if cfg!(target_os = "windows") { "folder_dialog_helper.exe" } else { "folder_dialog_helper" });
        helper_path
    };

    if helper_path.exists() {
        println!("🚀 Executing folder dialog helper: {}", helper_path.display());

        match tokio::process::Command::new(&helper_path)
            .output()
            .await
        {
            Ok(output) => {
                if output.status.success() {
                    let path_str_owned = String::from_utf8_lossy(&output.stdout).trim().to_string();
                    if !path_str_owned.is_empty() {
                        let selected_path = PathBuf::from(path_str_owned);
                        println!("✅ Folder selected via helper: {}", selected_path.display());
                        return Some(selected_path);
                    } else {
                        println!("❌ No path received from helper");
                    }
                } else {
                    let exit_code = output.status.code().unwrap_or(-1);
                    println!("❌ Folder dialog helper cancelled (exit code: {})", exit_code);
                    return None; // Explicitly return None for Cancel
                }
            }
            Err(e) => {
                println!("❌ Failed to execute folder dialog helper: {}", e);
            }
        }
    } else {
        println!("⚠️  Folder dialog helper not found at: {}", helper_path.display());
    }

    // Fallback to enhanced approach if helper fails
    println!("🍎 Falling back to enhanced folder selection");
    #[cfg(target_os = "macos")]
    {
        // Try multiple fallback approaches
        if let Some(home) = dirs::home_dir() {
            // Try Desktop first, then Downloads, then home
            let desktop = home.join("Desktop");
            if desktop.exists() {
                return Some(desktop);
            }

            let downloads = home.join("Downloads");
            if downloads.exists() {
                return Some(downloads);
            }

            return Some(home);
        }

        Some(std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/")))
    }

    #[cfg(not(target_os = "macos"))]
    {
        // On other platforms, we could use rfd here in the future
        println!("📁 Non-macOS: Using fallback folder selection");
        if let Some(home) = dirs::home_dir() {
            Some(home)
        } else {
            Some(std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/")))
        }
    }
}