use anyhow::Result;
use std::thread;
use std::time::Duration;
use sysinfo::{Signal, System};

use tracing::info;

/// Проверяет, запущен ли уже процесс PhotoMap и убивает его при необходимости
pub fn ensure_single_instance() -> Result<()> {
    info!("🔍 Checking for existing PhotoMap processes...");

    let mut system = System::new_all();
    system.refresh_all();

    let current_pid = std::process::id();
    let mut pids_to_kill = Vec::new();

    for (pid, process) in system.processes() {
        if pid.as_u32() == current_pid {
            continue;
        }

        // Check if the process name contains "photomap_processor"
        if process.name().contains("photomap_processor") {
            pids_to_kill.push(*pid);
        }
    }

    if !pids_to_kill.is_empty() {
        info!(
            "🔄 Found {} existing PhotoMap process(es), terminating...",
            pids_to_kill.len()
        );

        for pid in pids_to_kill {
            if let Some(process) = system.process(pid) {
                 info!("   🚫 Terminating process PID: {}", pid);
                 
                 // Try graceful termination first
                 if process.kill_with(Signal::Term).unwrap_or(false) {
                     // Wait a bit
                     thread::sleep(Duration::from_millis(500));
                     
                     // Refresh system to check if it's still there
                     system.refresh_process(pid);
                     if let Some(p) = system.process(pid) {
                         info!("   ⚡ Process still alive, force killing PID: {}", pid);
                         p.kill_with(Signal::Kill);
                     }
                 } else {
                     // If SIGTERM not supported, try Kill directly
                      info!("   ⚡ Could not send SIGTERM, force killing PID: {}", pid);
                      process.kill_with(Signal::Kill);
                 }
            }
        }
        
        // Give time for cleanup
        thread::sleep(Duration::from_secs(1));
        info!("✅ All existing processes terminated");
    } else {
        info!("✅ No existing PhotoMap processes found");
    }

    Ok(())
}
