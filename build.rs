use std::path::Path;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=schemas/trading.fbs");
    println!("cargo:rerun-if-changed=schemas/trading.proto");
    println!("cargo:rerun-if-changed=Cargo.lock");

    // --- FlatBuffers PATH setup ---
    let path = std::env::var("PATH").unwrap_or_default();
    let extra_paths = [
        "/opt/homebrew/bin", // macOS Homebrew (Apple Silicon)
        "/usr/local/bin",    // macOS Homebrew (Intel) / manual install
        "/usr/bin",          // Linux system package
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

    // --- Generate FlatBuffers Rust code ---
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
         Ubuntu: sudo apt install flatbuffers-compiler",
    );

    // --- Generate Protobuf Rust code from .proto schema (Section V-C) ---
    prost_build::Config::new()
        .out_dir(Path::new(&out_dir))
        .compile_protos(&["schemas/trading.proto"], &["schemas/"])
        .expect(
            "Failed to compile Protobuf schema. \
             Ensure 'protoc' is installed: \
             macOS: brew install protobuf | \
             Ubuntu: sudo apt install protobuf-compiler",
        );

    // --- Build-time environment capture (Section IV-D.6) ---
    // Emit compile-time env vars for environment.rs to pick up via env!().

    // Cargo build profile, optimization level, and LTO setting
    let profile = std::env::var("PROFILE").unwrap_or_else(|_| "unknown".into());
    let opt_level = std::env::var("OPT_LEVEL").unwrap_or_else(|_| "unknown".into());
    // LTO is not directly available as an env var in build scripts;
    // parse from Cargo.toml [profile.release] if present.
    let lto = parse_lto_from_cargo_toml().unwrap_or_else(|| "default".into());
    println!("cargo:rustc-env=BUILD_PROFILE={}", profile);
    println!("cargo:rustc-env=BUILD_OPT_LEVEL={}", opt_level);
    println!("cargo:rustc-env=BUILD_LTO={}", lto);

    // rustc version
    if let Ok(output) = std::process::Command::new("rustc")
        .arg("--version")
        .output()
        && let Ok(version) = String::from_utf8(output.stdout)
    {
        println!("cargo:rustc-env=BUILD_RUSTC_VERSION={}", version.trim());
    }

    // Parse dependency versions from Cargo.lock (single source of truth)
    if let Ok(lock_content) = std::fs::read_to_string("Cargo.lock") {
        let deps = [
            ("serde_json", "DEP_SERDE_JSON_VERSION"),
            ("bincode-next", "DEP_BINCODE_VERSION"),
            ("rkyv", "DEP_RKYV_VERSION"),
            ("prost", "DEP_PROST_VERSION"),
            ("flatbuffers", "DEP_FLATBUFFERS_VERSION"),
            ("hdrhistogram", "DEP_HDRHISTOGRAM_VERSION"),
        ];
        for (name, env_key) in deps {
            if let Some(version) = parse_cargo_lock_version(&lock_content, name) {
                println!("cargo:rustc-env={}={}", env_key, version);
            }
        }
    }
}

/// Extract LTO setting from Cargo.toml [profile.release] section.
fn parse_lto_from_cargo_toml() -> Option<String> {
    let content = std::fs::read_to_string("Cargo.toml").ok()?;
    let mut in_profile_release = false;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed == "[profile.release]" {
            in_profile_release = true;
        } else if trimmed.starts_with('[') {
            in_profile_release = false;
        } else if in_profile_release
            && trimmed.starts_with("lto")
            && let Some(val) = trimmed.split('=').nth(1)
        {
            return Some(val.trim().trim_matches('"').to_string());
        }
    }
    None
}

/// Extract version for a package from Cargo.lock TOML.
fn parse_cargo_lock_version(content: &str, package_name: &str) -> Option<String> {
    let mut found_name = false;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("name = ") {
            let name = trimmed.trim_start_matches("name = ").trim_matches('"');
            found_name = name == package_name;
        } else if found_name && trimmed.starts_with("version = ") {
            let version = trimmed.trim_start_matches("version = ").trim_matches('"');
            return Some(version.to_string());
        } else if trimmed == "[[package]]" {
            found_name = false;
        }
    }
    None
}
