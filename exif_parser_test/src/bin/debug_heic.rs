use anyhow::Result;
use libheif_rs::HeifContext;
use std::env;

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("Usage: debug_heic <path_to_heic_file>");
        return Ok(());
    }

    let path = &args[1];
    println!("🔍 Inspecting: {}", path);

    let ctx = HeifContext::read_from_file(path)?;
    let handle = ctx.primary_image_handle()?;

    // Get ALL metadata blocks (type_filter = 0)
    let count = handle.number_of_metadata_blocks(0);
    println!("Found {} metadata blocks.", count);

    if count > 0 {
        let mut ids = vec![0; count as usize];
        let count = handle.metadata_block_ids(&mut ids, 0);
        
        for &id in ids.iter().take(count) {
            let type_str = handle.metadata_type(id);
            let content_type = handle.metadata_content_type(id);
            let size = handle.metadata_size(id);
            
            println!("\nBlock ID: {}", id);
            println!("  Type: {:?}", type_str);
            println!("  Content Type: {:?}", content_type);
            println!("  Size: {} bytes", size);

            if let Ok(data) = handle.metadata(id) {
                print!("  First 16 bytes: ");
                for b in data.iter().take(16) {
                    print!("{:02X} ", b);
                }
                println!();
                
                // Try to parse as ASCII to see if there are readable headers
                let ascii: String = data.iter().take(16)
                    .map(|&b| if b >= 32 && b <= 126 { b as char } else { '.' })
                    .collect();
                println!("  ASCII: {}", ascii);
            }
        }
    }

    Ok(())
}
