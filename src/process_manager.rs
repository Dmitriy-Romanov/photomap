use anyhow::Result;
use std::process::Command;
use std::thread;
use std::time::Duration;
use tracing::{info, warn};

#[cfg(target_os = "windows")]
use anyhow::Context;

/// Checks if the PhotoMap process is already running and kills it if necessary
pub fn ensure_single_instance() -> Result<()> {
    info!("🔍 Checking for existing PhotoMap processes...");

    #[cfg(target_os = "windows")]
    let result = kill_existing_windows();

    #[cfg(not(target_os = "windows"))]
    let result = kill_existing_unix();

    result
}

#[cfg(not(target_os = "windows"))]
fn kill_existing_unix() -> Result<()> {
    let current_pid = std::process::id();

    // Use pgrep to find photomap_processor processes
    let output = Command::new("pgrep")
        .arg("-f")
        .arg("photomap_processor")
        .output();

    let pids = match output {
        Ok(out) if out.status.success() => {
            String::from_utf8_lossy(&out.stdout)
                .lines()
                .filter_map(|line| line.trim().parse::<u32>().ok())
                .filter(|&pid| pid != current_pid)
                .collect::<Vec<_>>()
        }
        Ok(_) => {
            // pgrep returns exit code 1 if no processes found
            info!("✅ No existing PhotoMap processes found");
            return Ok(());
        }
        Err(e) => {
            warn!("⚠️  pgrep command failed: {}. Skipping process check.", e);
            return Ok(());
        }
    };

    if pids.is_empty() {
        info!("✅ No existing PhotoMap processes found");
        return Ok(());
    }

    info!(
        "🔄 Found {} existing PhotoMap process(es), terminating...",
        pids.len()
    );

    for pid in pids {
        info!("   🚫 Terminating process PID: {}", pid);

        // Try graceful termination first (SIGTERM)
        let term_result = Command::new("kill")
            .arg("-TERM")
            .arg(pid.to_string())
            .status();

        if term_result.is_ok() {
            thread::sleep(Duration::from_millis(500));

            // Check if process still exists
            let check = Command::new("kill")
                .arg("-0")
                .arg(pid.to_string())
                .status();

            if check.is_ok() {
                // Process still alive, force kill
                info!("   ⚡ Process still alive, force killing PID: {}", pid);
                let _ = Command::new("kill")
                    .arg("-KILL")
                    .arg(pid.to_string())
                    .status();
            }
        } else {
            // SIGTERM failed, try SIGKILL directly
            info!("   ⚡ SIGTERM failed, force killing PID: {}", pid);
            let _ = Command::new("kill")
                .arg("-KILL")
                .arg(pid.to_string())
                .status();
        }
    }

    thread::sleep(Duration::from_secs(1));
    info!("✅ All existing processes terminated");

    Ok(())
}

#[cfg(target_os = "windows")]
fn kill_existing_windows() -> Result<()> {
    let current_pid = std::process::id();

    // Use tasklist to find photomap_processor.exe processes
    let output = Command::new("tasklist")
        .args(&["/FI", "IMAGENAME eq photomap_processor.exe", "/FO", "CSV", "/NH"])
        .output()
        .context("Failed to run tasklist command")?;

    if !output.status.success() {
        info!("✅ No existing PhotoMap processes found");
        return Ok(());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let pids: Vec<u32> = stdout
        .lines()
        .filter_map(|line| {
            // CSV format: "photomap_processor.exe","1234","Console","1","12,345 K"
            let parts: Vec<&str> = line.split(',').collect();
            if parts.len() >= 2 {
                // Second field is PID (quoted)
                parts[1].trim_matches('"').parse::<u32>().ok()
            } else {
                None
            }
        })
        .filter(|&pid| pid != current_pid)
        .collect();

    if pids.is_empty() {
        info!("✅ No existing PhotoMap processes found");
        return Ok(());
    }

    info!(
        "🔄 Found {} existing PhotoMap process(es), terminating...",
        pids.len()
    );

    for pid in pids {
        info!("   🚫 Terminating process PID: {}", pid);

        // Try graceful termination first
        let term_result = Command::new("taskkill")
            .args(&["/PID", &pid.to_string()])
            .status();

        if term_result.is_ok() {
            thread::sleep(Duration::from_millis(500));

            // Check if process still exists
            let check_output = Command::new("tasklist")
                .args(&["/FI", &format!("PID eq {}", pid), "/FO", "CSV", "/NH"])
                .output();

            if let Ok(out) = check_output {
                if !out.stdout.is_empty() && String::from_utf8_lossy(&out.stdout).contains(&pid.to_string()) {
                    // Process still alive, force kill
                    info!("   ⚡ Process still alive, force killing PID: {}", pid);
                    let _ = Command::new("taskkill")
                        .args(&["/F", "/PID", &pid.to_string()])
                        .status();
                }
            }
        } else {
            // Graceful kill failed, force kill
            info!("   ⚡ Graceful kill failed, force killing PID: {}", pid);
            let _ = Command::new("taskkill")
                .args(&["/F", "/PID", &pid.to_string()])
                .status();
        }
    }

    thread::sleep(Duration::from_secs(1));
    info!("✅ All existing processes terminated");

    Ok(())
}
