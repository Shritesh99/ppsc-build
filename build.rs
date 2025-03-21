use prost_build::Config as ProstConfig;
use std::path::PathBuf;
use std::{env, fs};
fn main() {
    // Create benches/tmp directory
    let tempdir = tempfile::tempdir().unwrap();

    // Generate prost version
    ProstConfig::new()
        .out_dir(tempdir.path())
        .compile_protos(
            &["src/fixtures/network_protocol/network_protocol.proto"],
            &["src/fixtures/network_protocol"],
        )
        .unwrap();

    let network_protocol_path = tempdir.path().join("network.protocol.rs");

    let benches_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap()).join("benches/tmp");

    // Ensure the target directory exists
    fs::create_dir_all(&benches_dir).expect("Failed to create benches/tmp directory");

    let final_path = benches_dir.join("network.protocol.prost.rs");
    fs::rename(&network_protocol_path, &final_path).expect("Failed to move file");
}
