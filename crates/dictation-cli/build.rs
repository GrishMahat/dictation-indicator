fn main() {
    // sherpa-onnx's shared runtime is copied beside Cargo-built binaries.
    // Keep the installed `dictation` executable relocatable with those libs.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("linux") {
        println!("cargo:rustc-link-arg=-Wl,-rpath,$ORIGIN");
    }
}
