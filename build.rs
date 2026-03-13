use std::path::Path;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=schemas/trading.fbs");
    
    // Ensure flatc is in PATH
    if std::env::var("PATH").map(|p| !p.contains("/opt/homebrew/bin")).unwrap_or(true) {
        let path = std::env::var("PATH").unwrap_or_default();
        unsafe {
            std::env::set_var("PATH", format!("/opt/homebrew/bin:{}", path));
        }
    }
    
    // Generate FlatBuffers code
    let out_dir = std::env::var("OUT_DIR").unwrap();
    
    flatc_rust::run(flatc_rust::Args {
        inputs: &[Path::new("schemas/trading.fbs")],
        out_dir: Path::new(&out_dir),
        ..Default::default()
    })
    .expect("Failed to compile FlatBuffers schema");
}
