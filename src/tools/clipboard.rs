use serde_json::Value;

/// Copy text to the system clipboard. Uses platform-appropriate mechanism:
/// Windows: PowerShell Set-Clipboard via temp file (UTF-8 safe, handles large text).
/// macOS: pbcopy. Linux: xclip or xsel.
pub async fn copy_to_clipboard(args: &Value) -> Result<String, String> {
    let text = args
        .get("text")
        .and_then(|v| v.as_str())
        .ok_or("copy_to_clipboard: missing required 'text'")?;

    if text.is_empty() {
        return Err("copy_to_clipboard: text is empty".to_string());
    }

    let len = text.len();

    #[cfg(windows)]
    {
        copy_windows(text)?;
    }
    #[cfg(target_os = "macos")]
    {
        copy_macos(text)?;
    }
    #[cfg(all(not(windows), not(target_os = "macos")))]
    {
        copy_linux(text)?;
    }

    Ok(format!(
        "Copied {len} bytes to clipboard. The user can now paste with Ctrl+V."
    ))
}

#[cfg(windows)]
fn copy_windows(text: &str) -> Result<(), String> {
    // Write to a temp file, then use Set-Clipboard in PowerShell.
    // This preserves Unicode and large content better than piping to clip.exe.
    let temp_path =
        std::env::temp_dir().join(format!("hematite-clip-tool-{}.txt", std::process::id()));

    std::fs::write(&temp_path, text.as_bytes())
        .map_err(|e| format!("copy_to_clipboard: write temp file: {e}"))?;

    let escaped = temp_path.display().to_string().replace('\'', "''");
    let script = format!(
        "Get-Content -LiteralPath '{escaped}' -Raw | Set-Clipboard; \
         Remove-Item -LiteralPath '{escaped}' -Force -ErrorAction SilentlyContinue"
    );

    let status = std::process::Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-WindowStyle",
            "Hidden",
            "-Command",
            &script,
        ])
        .status()
        .map_err(|e| format!("copy_to_clipboard: spawn powershell: {e}"))?;

    // Clean up even if PowerShell failed
    let _ = std::fs::remove_file(&temp_path);

    if status.success() {
        Ok(())
    } else {
        // Fallback: clip.exe via stdin
        copy_windows_clip(text)
    }
}

#[cfg(windows)]
fn copy_windows_clip(text: &str) -> Result<(), String> {
    use std::io::Write;
    let system_root = std::env::var("SystemRoot").unwrap_or_else(|_| "C:\\Windows".to_string());
    let clip_exe = format!("{system_root}\\System32\\clip.exe");

    let mut child = std::process::Command::new(&clip_exe)
        .stdin(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("copy_to_clipboard: spawn clip.exe: {e}"))?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(text.as_bytes())
            .map_err(|e| format!("copy_to_clipboard: write to clip.exe: {e}"))?;
    }

    child
        .wait()
        .map(|_| ())
        .map_err(|e| format!("copy_to_clipboard: clip.exe wait: {e}"))
}

#[cfg(target_os = "macos")]
fn copy_macos(text: &str) -> Result<(), String> {
    use std::io::Write;
    let mut child = std::process::Command::new("pbcopy")
        .stdin(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("copy_to_clipboard: spawn pbcopy: {e}"))?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(text.as_bytes())
            .map_err(|e| format!("copy_to_clipboard: write to pbcopy: {e}"))?;
    }

    child
        .wait()
        .map(|_| ())
        .map_err(|e| format!("copy_to_clipboard: pbcopy wait: {e}"))
}

#[cfg(all(not(windows), not(target_os = "macos")))]
fn copy_linux(text: &str) -> Result<(), String> {
    use std::io::Write;

    // Try xclip first, fall back to xsel
    for (prog, args) in &[
        ("xclip", vec!["-selection", "clipboard"]),
        ("xsel", vec!["--clipboard", "--input"]),
    ] {
        if let Ok(mut child) = std::process::Command::new(prog)
            .args(args)
            .stdin(std::process::Stdio::piped())
            .spawn()
        {
            if let Some(mut stdin) = child.stdin.take() {
                let _ = stdin.write_all(text.as_bytes());
            }
            if child.wait().map(|s| s.success()).unwrap_or(false) {
                return Ok(());
            }
        }
    }

    Err("copy_to_clipboard: neither xclip nor xsel is available. \
         Install one: apt install xclip OR apt install xsel"
        .to_string())
}
