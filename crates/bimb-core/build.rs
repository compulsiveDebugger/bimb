use std::{env, path::PathBuf, process::Command};
fn main() {
    println!("cargo:rerun-if-changed=native/MediaWorker.swift");
    if env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        let out = PathBuf::from(env::var_os("OUT_DIR").unwrap());
        let status = Command::new("xcrun")
            .args(["swiftc", "-O", "-parse-as-library", "-module-cache-path"])
            .arg(out.join("swift-cache"))
            .arg("native/MediaWorker.swift")
            .arg("-o")
            .arg(out.join("bimb-media"))
            .status()
            .expect("Apple command line tools are required");
        assert!(status.success(), "Native media worker compilation failed");
    }
}
