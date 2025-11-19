fn main() {
    let factors = turbojpeg::Decompressor::supported_scaling_factors();
    println!("Supported factors:");
    for f in factors {
        println!("{}/{}", f.num(), f.denom());
    }
}
