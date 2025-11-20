use std::env;
use std::fs::File;
use std::io::Read;

fn main() -> std::io::Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("Usage: check_header <path>");
        return Ok(());
    }

    let path = &args[1];
    let mut file = File::open(path)?;
    let mut buffer = [0; 32];
    file.read_exact(&mut buffer)?;

    println!("File: {}", path);
    print!("Hex: ");
    for b in &buffer {
        print!("{:02X} ", b);
    }
    println!();

    print!("ASCII: ");
    for b in &buffer {
        if *b >= 32 && *b <= 126 {
            print!("{}", *b as char);
        } else {
            print!(".");
        }
    }
    println!();

    Ok(())
}
