#[cfg(unix)]
use std::ffi::OsString;
#[cfg(unix)]
use std::os::unix::ffi::OsStrExt;

/// Per-main process hardening steps:
/// - Disables core dumps (to protect model weights/memory).
/// - Disables ptrace attach on Linux/macOS (prevents memory sniffing).
/// - Sanitizes dangerous environment variables (LD_PRELOAD, etc).
pub fn pre_main_hardening() {
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    pre_main_hardening_unix();

    #[cfg(windows)]
    pre_main_hardening_windows();
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn pre_main_hardening_unix() {
    // 1. Disable core dumps
    let rlim = libc::rlimit {
        rlim_cur: 0,
        rlim_max: 0,
    };
    unsafe {
        libc::setrlimit(libc::RLIMIT_CORE, &rlim);
    }

    // 2. Disable ptrace (Anti-Debugging)
    #[cfg(target_os = "macos")]
    unsafe {
        // PT_DENY_ATTACH = 31
        libc::ptrace(31, 0, std::ptr::null_mut(), 0);
    }
    #[cfg(target_os = "linux")]
    unsafe {
        // PR_SET_DUMPABLE = 4
        libc::prctl(4, 0, 0, 0, 0);
    }

    // 3. Sanitize Environment
    remove_env_vars_with_prefix("LD_");
    remove_env_vars_with_prefix("DYLD_");
    remove_env_vars_with_prefix("MallocStackLogging");
}

#[cfg(windows)]
fn pre_main_hardening_windows() {
    // Windows Phase 1: Environment sanitization for risky shells
    // (Note: LD_PRELOAD is Unix-only, but we clear it here in case of cross-platform shells)
    let risky_prefixes = ["LD_", "DYLD_"];
    for prefix in risky_prefixes {
        for (key, _) in std::env::vars() {
            if key.starts_with(prefix) {
                std::env::remove_var(key);
            }
        }
    }
}

#[cfg(unix)]
fn remove_env_vars_with_prefix(prefix: &str) {
    let prefix_bytes = prefix.as_bytes();
    let keys_to_remove: Vec<OsString> = std::env::vars_os()
        .filter_map(|(key, _)| {
            if key.as_os_str().as_bytes().starts_with(prefix_bytes) {
                Some(key)
            } else {
                None
            }
        })
        .collect();

    for key in keys_to_remove {
        std::env::remove_var(key);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_env_sanitization_prefixes() {
        std::env::set_var("LD_TEST_LOG", "1");
        std::env::set_var("DYLD_LIBRARY_PATH", "/tmp");
        std::env::set_var("SAFE_VAR", "preserved");

        // Use the platform-specific internal helper or the main entry
        pre_main_hardening();

        assert!(std::env::var("LD_TEST_LOG").is_err());
        assert!(std::env::var("DYLD_LIBRARY_PATH").is_err());
        assert_eq!(std::env::var("SAFE_VAR").unwrap(), "preserved");
    }
}
