use std::io::Error;

fn main() -> Result<(), Error> {
    if cfg!(target_os = "linux") {
        println!("cargo:rustc-link-arg=-lbz2");
        println!("cargo:rustc-link-arg=-lpng16");
        println!("cargo:rustc-link-arg=-lbrotlidec");
        println!("cargo:rustc-link-arg=-lEGL");
    }
    Ok(())
}
