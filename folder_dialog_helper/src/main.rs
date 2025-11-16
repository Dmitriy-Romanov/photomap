// Simple helper program to open folder dialog and return selected path

fn main() {
    // Try to open a folder dialog
    match rfd::FileDialog::new()
        .set_title("Select folder for PhotoMap")
        .pick_folder()
    {
        Some(path) => {
            // Print the selected path to stdout
            println!("{}", path.display());
        }
        None => {
            // User cancelled, exit with error code
            std::process::exit(1);
        }
    }
}
