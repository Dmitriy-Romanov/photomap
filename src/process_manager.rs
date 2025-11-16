use std::process::Command;
use anyhow::Result;

/// Проверяет, запущен ли уже процесс PhotoMap и убивает его при необходимости
pub fn ensure_single_instance() -> Result<()> {
    println!("🔍 Checking for existing PhotoMap processes...");

    // Получаем список процессов photomap_processor
    let output = Command::new("pgrep")
        .arg("-f")
        .arg("photomap_processor")
        .output();

    match output {
        Ok(result) => {
            if result.status.success() {
                let pids = String::from_utf8_lossy(&result.stdout);
                let pid_list: Vec<&str> = pids.trim().split_whitespace().collect();

                if !pid_list.is_empty() {
                    println!("🔄 Found {} existing PhotoMap process(es), terminating...", pid_list.len());

                    for &pid in &pid_list {
                        if let Ok(pid_num) = pid.parse::<i32>() {
                            println!("   🚫 Terminating process PID: {}", pid_num);

                            // Сначала пытаемся завершить gracefully (SIGTERM)
                            if let Ok(_) = Command::new("kill")
                                .arg("-TERM")
                                .arg(pid)
                                .output()
                            {
                                // Даем процессу время на завершение
                                std::thread::sleep(std::time::Duration::from_millis(500));

                                // Проверяем, все еще ли процесс жив
                                if let Ok(check_result) = Command::new("kill")
                                    .arg("-0")
                                    .arg(pid)
                                    .output()
                                {
                                    if check_result.status.success() {
                                        // Если все еще жив, принудительно убиваем (SIGKILL)
                                        println!("   ⚡ Force killing PID: {}", pid_num);
                                        let _ = Command::new("kill")
                                            .arg("-KILL")
                                            .arg(pid)
                                            .output();
                                    }
                                }
                            }
                        }
                    }

                    // Даем время на полную очистку
                    std::thread::sleep(std::time::Duration::from_secs(1));
                    println!("✅ All existing processes terminated");
                } else {
                    println!("✅ No existing PhotoMap processes found");
                }
            } else {
                println!("ℹ️  Could not check for existing processes (pgrep not available)");
            }
        }
        Err(_) => {
            println!("ℹ️  Could not check for existing processes");
        }
    }

    Ok(())
}