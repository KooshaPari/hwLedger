//! Build script for hwledger-gui-recorder FFI header generation.
fn main() {
    let crate_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");

    let bindings = cbindgen::Builder::new()
        .with_crate(crate_dir.clone())
        .with_config(
            cbindgen::Config::from_file(format!("{}/cbindgen.toml", crate_dir))
                .expect("Failed to read cbindgen.toml"),
        )
        .generate()
        .expect("Unable to generate bindings");

    bindings.write_to_file(format!("{}/target/hwledger_gui_recorder.h", crate_dir));
}
