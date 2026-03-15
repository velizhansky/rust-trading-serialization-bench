//! Runtime environment capture and validation (Section IV-D.6).
//!
//! Captures CPU model, cache hierarchy, OS/kernel, Rust compiler version,
//! CPU governor, Turbo Boost state, timer resolution, and crate versions.
//! All captures are best-effort: returns "unknown" on unsupported platforms
//! (macOS development) rather than panicking.

use serde::Serialize;
use std::process::Command;
use std::time::Instant;

#[derive(Debug, Clone, Serialize)]
pub struct EnvironmentInfo {
    // Hardware
    pub cpu_model: String,
    pub cpu_cores: usize,
    pub cpu_threads: usize,
    pub l1_cache: String,
    pub l2_cache: String,
    pub l3_cache: String,
    pub memory_total_mb: u64,

    // OS
    pub os: String,
    pub kernel_version: String,

    // Rust
    pub rustc_version: String,
    pub cargo_profile: String,

    // CPU power management
    pub cpu_governor: String,
    pub turbo_boost: String,

    // Timer
    pub timer_resolution_ns: u64,

    // Crate versions
    pub serde_json_version: String,
    pub bincode_version: String,
    pub rkyv_version: String,
    pub prost_version: String,
    pub flatbuffers_version: String,
    pub hdrhistogram_version: String,

    // Benchmark config
    pub warmup_messages: usize,
    pub throughput_window_secs: u64,
    pub hdr_significant_digits: u8,

    // Hostname and timestamp
    pub hostname: String,
    pub capture_timestamp: String,
}

pub fn capture_environment() -> EnvironmentInfo {
    EnvironmentInfo {
        cpu_model: read_cpu_model(),
        cpu_cores: read_cpu_cores(),
        cpu_threads: read_cpu_threads(),
        l1_cache: read_cache_size("index0", "index1"),
        l2_cache: read_cache_level("index2"),
        l3_cache: read_cache_level("index3"),
        memory_total_mb: read_memory_total_mb(),

        os: read_os_name(),
        kernel_version: read_kernel_version(),

        rustc_version: read_rustc_version(),
        cargo_profile: if cfg!(debug_assertions) { "debug" } else { "release" }.to_string(),

        cpu_governor: read_cpu_governor(),
        turbo_boost: read_turbo_boost(),

        timer_resolution_ns: measure_timer_resolution(),

        // Crate versions from Cargo.toml — update here if dependencies change.
        // These are recorded in the environment manifest for reproducibility (Section IV-D.6).
        serde_json_version: "1.0.149".to_string(),
        bincode_version: "2.0.4".to_string(),
        rkyv_version: "0.8.13".to_string(),
        prost_version: "0.14.1".to_string(),
        flatbuffers_version: "25.12.19".to_string(),
        hdrhistogram_version: "7.5.4".to_string(),

        warmup_messages: 5_000,
        throughput_window_secs: 5,
        hdr_significant_digits: 3,

        hostname: read_hostname(),
        capture_timestamp: chrono::Utc::now().to_rfc3339(),
    }
}

/// Check if environment is suitable for accurate benchmarking.
/// Returns a list of warnings (empty = all good).
pub fn check_environment() -> Vec<String> {
    let mut warnings = Vec::new();
    let env = capture_environment();

    if env.cargo_profile != "release" {
        warnings.push("WARNING: Not running in release mode. Results will be inaccurate.".into());
    }
    if env.cpu_governor != "performance" && env.cpu_governor != "unknown" {
        warnings.push(format!(
            "WARNING: CPU governor is '{}', should be 'performance'.",
            env.cpu_governor
        ));
    }
    if env.turbo_boost == "enabled" {
        warnings.push("WARNING: Turbo Boost is enabled. Disable for stable measurements.".into());
    }

    warnings
}

// --- Helper functions (all best-effort, return "unknown" on failure) ---

fn read_file_trimmed(path: &str) -> Option<String> {
    std::fs::read_to_string(path).ok().map(|s| s.trim().to_string())
}

fn run_command(program: &str, args: &[&str]) -> Option<String> {
    Command::new(program)
        .args(args)
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                String::from_utf8(o.stdout).ok().map(|s| s.trim().to_string())
            } else {
                None
            }
        })
}

fn read_cpu_model() -> String {
    // Linux: /proc/cpuinfo
    if let Some(cpuinfo) = read_file_trimmed("/proc/cpuinfo") {
        for line in cpuinfo.lines() {
            if line.starts_with("model name") {
                if let Some(val) = line.split(':').nth(1) {
                    return val.trim().to_string();
                }
            }
        }
    }
    // macOS: sysctl
    run_command("sysctl", &["-n", "machdep.cpu.brand_string"])
        .unwrap_or_else(|| "unknown".to_string())
}

fn read_cpu_cores() -> usize {
    // Linux
    if let Some(cpuinfo) = read_file_trimmed("/proc/cpuinfo") {
        let mut cores = std::collections::HashSet::new();
        let mut phys_id = String::new();
        for line in cpuinfo.lines() {
            if line.starts_with("physical id") {
                if let Some(val) = line.split(':').nth(1) {
                    phys_id = val.trim().to_string();
                }
            } else if line.starts_with("core id") {
                if let Some(val) = line.split(':').nth(1) {
                    cores.insert(format!("{}:{}", phys_id, val.trim()));
                }
            }
        }
        if !cores.is_empty() {
            return cores.len();
        }
    }
    // macOS
    run_command("sysctl", &["-n", "hw.physicalcpu"])
        .and_then(|s| s.parse().ok())
        .unwrap_or(0)
}

fn read_cpu_threads() -> usize {
    // Linux
    if let Some(cpuinfo) = read_file_trimmed("/proc/cpuinfo") {
        let count = cpuinfo.lines().filter(|l| l.starts_with("processor")).count();
        if count > 0 {
            return count;
        }
    }
    // macOS
    run_command("sysctl", &["-n", "hw.logicalcpu"])
        .and_then(|s| s.parse().ok())
        .unwrap_or(0)
}

fn read_cache_size(index_d: &str, index_i: &str) -> String {
    let base = "/sys/devices/system/cpu/cpu0/cache";
    let d_size = read_file_trimmed(&format!("{}/{}/size", base, index_d));
    let i_size = read_file_trimmed(&format!("{}/{}/size", base, index_i));
    match (d_size, i_size) {
        (Some(d), Some(i)) => format!("{}D + {}I", d, i),
        (Some(d), None) => d,
        _ => {
            // macOS
            let l1d = run_command("sysctl", &["-n", "hw.l1dcachesize"])
                .and_then(|s| s.parse::<u64>().ok())
                .map(|b| format!("{}K", b / 1024));
            let l1i = run_command("sysctl", &["-n", "hw.l1icachesize"])
                .and_then(|s| s.parse::<u64>().ok())
                .map(|b| format!("{}K", b / 1024));
            match (l1d, l1i) {
                (Some(d), Some(i)) => format!("{}D + {}I", d, i),
                _ => "unknown".to_string(),
            }
        }
    }
}

fn read_cache_level(index: &str) -> String {
    let path = format!("/sys/devices/system/cpu/cpu0/cache/{}/size", index);
    if let Some(size) = read_file_trimmed(&path) {
        return size;
    }
    // macOS fallback
    let sysctl_key = match index {
        "index2" => "hw.l2cachesize",
        "index3" => "hw.l3cachesize",
        _ => return "unknown".to_string(),
    };
    run_command("sysctl", &["-n", sysctl_key])
        .and_then(|s| s.parse::<u64>().ok())
        .map(|b| {
            if b >= 1024 * 1024 {
                format!("{}M", b / (1024 * 1024))
            } else {
                format!("{}K", b / 1024)
            }
        })
        .unwrap_or_else(|| "unknown".to_string())
}

fn read_memory_total_mb() -> u64 {
    // Linux
    if let Some(meminfo) = read_file_trimmed("/proc/meminfo") {
        for line in meminfo.lines() {
            if line.starts_with("MemTotal:") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    if let Ok(kb) = parts[1].parse::<u64>() {
                        return kb / 1024;
                    }
                }
            }
        }
    }
    // macOS
    run_command("sysctl", &["-n", "hw.memsize"])
        .and_then(|s| s.parse::<u64>().ok())
        .map(|b| b / (1024 * 1024))
        .unwrap_or(0)
}

fn read_os_name() -> String {
    // Linux
    if let Some(release) = read_file_trimmed("/etc/os-release") {
        for line in release.lines() {
            if line.starts_with("PRETTY_NAME=") {
                return line
                    .trim_start_matches("PRETTY_NAME=")
                    .trim_matches('"')
                    .to_string();
            }
        }
    }
    // macOS
    run_command("sw_vers", &["-productName"])
        .and_then(|name| {
            run_command("sw_vers", &["-productVersion"]).map(|ver| format!("{} {}", name, ver))
        })
        .unwrap_or_else(|| "unknown".to_string())
}

fn read_kernel_version() -> String {
    run_command("uname", &["-r"]).unwrap_or_else(|| "unknown".to_string())
}

fn read_rustc_version() -> String {
    run_command("rustc", &["--version"]).unwrap_or_else(|| "unknown".to_string())
}

fn read_cpu_governor() -> String {
    read_file_trimmed("/sys/devices/system/cpu/cpu0/cpufreq/scaling_governor")
        .unwrap_or_else(|| "unknown".to_string())
}

fn read_turbo_boost() -> String {
    // Intel pstate
    if let Some(val) = read_file_trimmed("/sys/devices/system/cpu/intel_pstate/no_turbo") {
        return match val.as_str() {
            "1" => "disabled".to_string(),
            "0" => "enabled".to_string(),
            _ => "unknown".to_string(),
        };
    }
    // AMD boost
    if let Some(val) = read_file_trimmed("/sys/devices/system/cpu/cpufreq/boost") {
        return match val.as_str() {
            "0" => "disabled".to_string(),
            "1" => "enabled".to_string(),
            _ => "unknown".to_string(),
        };
    }
    "unknown".to_string()
}

fn measure_timer_resolution() -> u64 {
    let mut min_delta = u64::MAX;
    for _ in 0..1000 {
        let t1 = Instant::now();
        let t2 = Instant::now();
        let delta = t2.duration_since(t1).as_nanos() as u64;
        if delta > 0 && delta < min_delta {
            min_delta = delta;
        }
    }
    if min_delta == u64::MAX { 0 } else { min_delta }
}

fn read_hostname() -> String {
    // Try /etc/hostname first (Linux)
    if let Some(h) = read_file_trimmed("/etc/hostname") {
        if !h.is_empty() {
            return h;
        }
    }
    run_command("hostname", &[]).unwrap_or_else(|| "unknown".to_string())
}
