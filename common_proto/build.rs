use std::{env, path::PathBuf};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    
    // Compile hello.proto
    tonic_build::configure()
        .file_descriptor_set_path(out_dir.join("hello_descriptor.bin"))
        .compile(&["proto/hello.proto"], &["proto"])
        .unwrap();
    
    // Compile envelope.proto
    tonic_build::configure()
        .file_descriptor_set_path(out_dir.join("envelope_descriptor.bin"))
        .compile(&["proto/envelope.proto"], &["proto"])
        .unwrap();

    Ok(())
}
