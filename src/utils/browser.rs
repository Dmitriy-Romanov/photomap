use std::process::Command;

/// Opens the specified URL in the default browser using native commands.
pub fn open_browser(url: &str) -> Result<(), std::io::Error> {
    match std::env::consts::OS {
        "macos" => {
            Command::new("open").arg(url).spawn()?;
        }
        "windows" => {
            Command::new("cmd").args(["/C", "start", url]).spawn()?;
        }
        "linux" => {
            Command::new("xdg-open").arg(url).spawn()?;
        }
        os => {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                format!("Unsupported OS: {}", os),
            ));
        }
    }
    Ok(())
}
