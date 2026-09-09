use std::{env, path::PathBuf, process::Command};

#[test]
fn supports_a_renamed_runtime_dependency() {
    let manifest =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/renamed-runtime/Cargo.toml");
    let target_dir = env::temp_dir().join(format!(
        "nexus-rpc-macros-renamed-runtime-{}",
        std::process::id()
    ));

    let status = Command::new(env!("CARGO"))
        .args(["check", "--quiet", "--manifest-path"])
        .arg(manifest)
        .env("CARGO_TARGET_DIR", target_dir)
        .status()
        .expect("run cargo check for renamed-runtime fixture");

    assert!(status.success(), "renamed-runtime fixture did not compile");
}
