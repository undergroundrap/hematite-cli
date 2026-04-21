use std::env;

/// Known terminal and multiplexer categories derived from environment variables.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalName {
    WindowsTerminal,
    VsCode,
    Iterm2,
    AppleTerminal,
    Ghostty,
    Kitty,
    WezTerm,
    Alacritty,
    Tmux,
    Zellij,
    Unknown,
}

impl TerminalName {
    pub fn label(&self) -> &'static str {
        match self {
            TerminalName::WindowsTerminal => "WindowsTerminal",
            TerminalName::VsCode => "VsCode",
            TerminalName::Iterm2 => "iTerm2",
            TerminalName::AppleTerminal => "AppleTerminal",
            TerminalName::Ghostty => "Ghostty",
            TerminalName::Kitty => "Kitty",
            TerminalName::WezTerm => "WezTerm",
            TerminalName::Alacritty => "Alacritty",
            TerminalName::Tmux => "Tmux",
            TerminalName::Zellij => "Zellij",
            TerminalName::Unknown => "Terminal",
        }
    }
}

/// Detects the active terminal environment using heuristics borrowed from Codex-RS.
pub fn detect_terminal() -> TerminalName {
    // 1. Multiplexers (probed first as they can wrap other terms)
    if env::var("ZELLIJ").is_ok() || env::var("ZELLIJ_SESSION_NAME").is_ok() {
        return TerminalName::Zellij;
    }
    if env::var("TMUX").is_ok() {
        return TerminalName::Tmux;
    }

    // 2. Explicit TERM_PROGRAM identifiers
    if let Ok(prog) = env::var("TERM_PROGRAM") {
        match prog.to_lowercase().as_str() {
            "vscode" => return TerminalName::VsCode,
            "iterm.app" => return TerminalName::Iterm2,
            "apple_terminal" => return TerminalName::AppleTerminal,
            "ghostty" => return TerminalName::Ghostty,
            "wezterm" => return TerminalName::WezTerm,
            _ => {}
        }
    }

    // 3. Platform-specific and Capability variables
    if env::var("WT_SESSION").is_ok() {
        return TerminalName::WindowsTerminal;
    }
    if env::var("KITTY_WINDOW_ID").is_ok() {
        return TerminalName::Kitty;
    }
    if env::var("ALACRITTY_SOCKET").is_ok() {
        return TerminalName::Alacritty;
    }

    // 4. TERM fallback
    if let Ok(term) = env::var("TERM") {
        let lower = term.to_lowercase();
        if lower.contains("iterm") {
            return TerminalName::Iterm2;
        }
        if lower.contains("kitty") {
            return TerminalName::Kitty;
        }
        if lower.contains("alacritty") {
            return TerminalName::Alacritty;
        }
        if lower.contains("wezterm") {
            return TerminalName::WezTerm;
        }
    }

    TerminalName::Unknown
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_windows_terminal_detection() {
        // Clear env noise
        let old_term = std::env::var("TERM_PROGRAM").ok();
        std::env::remove_var("TERM_PROGRAM");

        std::env::set_var("WT_SESSION", "test-session");
        assert_eq!(detect_terminal(), TerminalName::WindowsTerminal);

        std::env::remove_var("WT_SESSION");
        if let Some(val) = old_term {
            std::env::set_var("TERM_PROGRAM", val);
        }
    }

    #[test]
    fn test_vscode_detection() {
        std::env::set_var("TERM_PROGRAM", "vscode");
        assert_eq!(detect_terminal(), TerminalName::VsCode);
        std::env::remove_var("TERM_PROGRAM");
    }
}
