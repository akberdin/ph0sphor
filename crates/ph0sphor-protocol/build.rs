use std::path::PathBuf;

fn main() {
    // Use a vendored protoc binary so the build is hermetic — no system
    // protoc needed on developer machines or CI runners.
    let protoc = protoc_bin_vendored::protoc_bin_path().expect("vendored protoc available");
    std::env::set_var("PROTOC", protoc);

    let proto_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("proto");
    let proto_file = proto_root.join("ph0sphor.proto");

    println!("cargo:rerun-if-changed={}", proto_file.display());

    let mut config = prost_build::Config::new();

    // Serde derives are needed for the JSON debug mirror. Applied to every
    // generated message so the public wire types can be (de)serialized with
    // serde_json in tests and `--debug-json` dumps.
    config.type_attribute(".", "#[derive(serde::Serialize, serde::Deserialize)]");

    config
        .compile_protos(&[proto_file], &[proto_root])
        .expect("compile ph0sphor.proto");
}
