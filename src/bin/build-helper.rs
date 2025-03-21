use ppsc_build;

fn main() {
    Config::new()
        .out_dir(tempdir.path())
        .compile_protos(
            &["src/fixtures/network_protocol/network_protocol.proto"],
            &["src/fixtures/network_protocol"],
        )
        .unwrap();
}
