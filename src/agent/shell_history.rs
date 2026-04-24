/// Reads recent shell command history from the host and returns a compact
/// context block for injection into the system prompt.
///
/// Sources (in priority order):
///   Windows — PSReadLine history file (PowerShell 5+ / PowerShell Core)
///   Linux/macOS — ~/.bash_history or ~/.zsh_history
///
/// The block is loaded once at session start and injected every turn so the
/// model knows what the user was recently doing without them explaining it.

const MAX_COMMANDS: usize = 20;
const MIN_CMD_LEN: usize = 4;

pub fn load_shell_history_block() -> Option<String> {
    let commands = read_history()?;
    if commands.is_empty() {
        return None;
    }
    let mut block = String::from("## Recent Shell History (last session commands)\n");
    for cmd in &commands {
        block.push_str(&format!("  $ {}\n", cmd));
    }
    block.push_str("Use this context to understand what the user was recently working on.\n");
    Some(block)
}

fn read_history() -> Option<Vec<String>> {
    let path = history_path()?;
    let raw = std::fs::read_to_string(&path).ok()?;

    let mut seen = LinkedHashSet::new();
    let mut cmds: Vec<String> = Vec::new();

    for line in raw.lines().rev() {
        let cmd = line.trim();
        if cmd.len() < MIN_CMD_LEN {
            continue;
        }
        // Skip trivials
        if matches!(cmd, "ls" | "pwd" | "cd" | "clear" | "exit" | "cls" | "history") {
            continue;
        }
        // Skip lines that look like prompts or comments
        if cmd.starts_with('#') || cmd.starts_with("PS ") {
            continue;
        }
        if seen.contains(cmd) {
            continue;
        }
        seen.insert(cmd.to_string());
        cmds.push(cmd.to_string());
        if cmds.len() >= MAX_COMMANDS {
            break;
        }
    }

    cmds.reverse(); // oldest-first for readability
    if cmds.is_empty() { None } else { Some(cmds) }
}

fn history_path() -> Option<std::path::PathBuf> {
    #[cfg(target_os = "windows")]
    {
        // PSReadLine: %APPDATA%\Microsoft\Windows\PowerShell\PSReadLine\ConsoleHost_history.txt
        let appdata = std::env::var("APPDATA").ok()?;
        let p = std::path::PathBuf::from(appdata)
            .join("Microsoft")
            .join("Windows")
            .join("PowerShell")
            .join("PSReadLine")
            .join("ConsoleHost_history.txt");
        if p.exists() { Some(p) } else { None }
    }
    #[cfg(not(target_os = "windows"))]
    {
        let home = std::env::var("HOME").ok()?;
        // Prefer zsh, fall back to bash
        let zsh = std::path::PathBuf::from(&home).join(".zsh_history");
        if zsh.exists() {
            return Some(zsh);
        }
        let bash = std::path::PathBuf::from(&home).join(".bash_history");
        if bash.exists() {
            return Some(bash);
        }
        None
    }
}

// Minimal insertion-ordered dedup set — avoids pulling in a crate.
struct LinkedHashSet(Vec<String>);

impl LinkedHashSet {
    fn new() -> Self { Self(Vec::new()) }
    fn contains(&self, s: &str) -> bool { self.0.iter().any(|x| x == s) }
    fn insert(&mut self, s: String) { self.0.push(s); }
}
