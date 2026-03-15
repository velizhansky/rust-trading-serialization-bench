use std::path::Path;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=schemas/trading.fbs");

    // Ensure flatc is discoverable on both macOS (Homebrew) and Linux.
    // flatc-rust invokes `flatc` via PATH, so we add common install locations.
    let path = std::env::var("PATH").unwrap_or_default();
    let extra_paths = [
        "/opt/homebrew/bin", // macOS Homebrew (Apple Silicon)
        "/usr/local/bin",    // macOS Homebrew (Intel) / manual install
        "/usr/bin",          // Linux system package (apt install flatbuffers-compiler)
        "/snap/bin",         // Linux snap
    ];
    let mut new_path = path.clone();
    for extra in &extra_paths {
        if !path.contains(extra) {
            new_path = format!("{}:{}", extra, new_path);
        }
    }
    if new_path != path {
        // SAFETY: build scripts run single-threaded before compilation.
        unsafe { std::env::set_var("PATH", &new_path) };
    }

    // Generate FlatBuffers Rust code from schema
    let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR not set");

    flatc_rust::run(flatc_rust::Args {
        inputs: &[Path::new("schemas/trading.fbs")],
        out_dir: Path::new(&out_dir),
        ..Default::default()
    })
    .expect(
        "Failed to compile FlatBuffers schema. \
         Ensure 'flatc' is installed: \
         macOS: brew install flatbuffers | \
         Ubuntu: sudo apt install flatbuffers-compiler"
    );
}
