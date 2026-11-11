//! Hands the linker the Endless patch memory layout (pedal builds only).

use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=patch.ld");
    if std::env::var("TARGET").unwrap().starts_with("thumb") {
        let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
        let out = PathBuf::from(std::env::var("OUT_DIR").unwrap());
        std::fs::copy(manifest.join("patch.ld"), out.join("patch.ld")).unwrap();
        println!("cargo:rustc-link-search={}", out.display());
        println!("cargo:rustc-link-arg=-Tpatch.ld");
    }
}
