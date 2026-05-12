use std::path::{Path, PathBuf};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Prefer the user's protoc if they set PROTOC, otherwise fall back to a
    // vendored protoc binary so contributors don't need a system install.
    if std::env::var_os("PROTOC").is_none() {
        let protoc = protoc_bin_vendored::protoc_bin_path()?;
        std::env::set_var("PROTOC", protoc);
    }
    println!("cargo:rerun-if-env-changed=PROTOC");

    let proto_root = proto_root();
    let protos = [
        proto_root.join("claw/common.proto"),
        proto_root.join("claw/objects.proto"),
    ];

    prost_build::Config::new()
        .out_dir("src/generated")
        .compile_protos(&protos, &[proto_root.as_path()])?;

    rerun_if_changed(&proto_root, "claw/common.proto");
    rerun_if_changed(&proto_root, "claw/objects.proto");
    Ok(())
}

fn proto_root() -> PathBuf {
    let manifest_dir = PathBuf::from(
        std::env::var_os("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is set by Cargo"),
    );
    let packaged = manifest_dir.join("proto");
    if packaged.exists() {
        packaged
    } else {
        manifest_dir.join("../../proto")
    }
}

fn rerun_if_changed(proto_root: &Path, relative: &str) {
    println!(
        "cargo:rerun-if-changed={}",
        proto_root.join(relative).display()
    );
}
