use prost_build::Config as ProstConfig;
use std::fs;
use std::path::PathBuf;
fn main() {
    // Create benches/tmp directory
    let out_dir = PathBuf::from("benches/tmp");
    fs::create_dir_all(&out_dir).unwrap();

    // Copy our generated file from fixtures
    fs::copy(
        "src/fixtures/network_protocol/_expected_network_protocol.rs",
        out_dir.join("network.protocol.rs"),
    )
    .unwrap();

    // Generate prost version
    ProstConfig::new()
        .out_dir(&out_dir)
        .compile_protos(
            &["src/fixtures/network_protocol/network_protocol.proto"],
            &["src/fixtures/network_protocol"],
        )
        .unwrap();

    // Copy the prost-generated file to a separate name
    fs::copy(
        out_dir.join("network.protocol.rs"),
        out_dir.join("network.protocol.prost.rs"),
    )
    .unwrap();

    // Remove the prost-generated file since we already copied it with a new name
    fs::remove_file(out_dir.join("network.protocol.rs")).unwrap();

    // Generate ppsc version
    let mut config = Config::default();
    config
        .out_dir(&out_dir)
        .compile_protos(
            &["src/fixtures/network_protocol/network_protocol.proto"],
            &["src/fixtures/network_protocol"],
        )
        .unwrap();
}
