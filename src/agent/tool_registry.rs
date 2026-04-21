use crate::agent::config::HematiteConfig;
use crate::agent::inference::tool_metadata_for_name;
use crate::agent::types::{ToolDefinition, ToolFunction};
use serde_json::Value;

fn make_tool(name: &str, description: &str, parameters: Value) -> ToolDefinition {
    ToolDefinition {
        tool_type: "function".into(),
        function: ToolFunction {
            name: name.into(),
            description: description.into(),
            parameters,
        },
        metadata: tool_metadata_for_name(name),
    }
}

/// Returns the full set of tools exposed to the model.
pub fn get_tools() -> Vec<ToolDefinition> {
    let os = std::env::consts::OS;
    let mut tools = vec![
        make_tool(
            "shell",
            &format!(
                "Execute a command in the host shell ({os}). \
                     Use this ONLY for building, testing, or advanced system operations that have no dedicated Hematite tool. \
                     FORBIDDEN: Never use shell to run `mkdir`, `rm`, `cat`, `head`, `tail`, or `write-file` equivalents. \
                     Use the dedicated surgical tools (create_directory, read_file, tail_file) instead. \
                     Output is capped at 64KB. Prefer non-interactive commands."
            ),
            serde_json::json!({
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "The command to run"
                    },
                    "reason": {
                        "type": "string",
                        "description": "For risky shell calls, explain what this command is verifying or changing."
                    },
                    "timeout_secs": {
                        "type": "integer",
                        "description": "Optional timeout in seconds (default 60)"
                    }
                },
                "required": ["command"]
            }),
        ),
        make_tool(
            "run_code",
            "Execute a short JavaScript/TypeScript or Python snippet in a sandboxed subprocess. \
             No network access, no filesystem escape, hard 10-second timeout. \
             Use this to verify logic, test algorithms, compute values, or test functions \
             when you need real output rather than a guess. \
             ALWAYS include the `language` field — there is no default. \
             \
             JAVASCRIPT/TYPESCRIPT (language: \"javascript\"): \
             Runs via Deno, NOT Node.js. `require()` does not exist — never use it. \
             URL imports (e.g. from 'https://deno.land/...') are blocked — network is off. \
             Use built-in Web APIs only: `crypto.subtle`, `TextEncoder`, `URL`, `atob`/`btoa`, etc. \
             SHA-256 example: \
               const buf = await crypto.subtle.digest('SHA-256', new TextEncoder().encode('hello')); \
               console.log([...new Uint8Array(buf)].map(b=>b.toString(16).padStart(2,'0')).join('')); \
             \
             PYTHON (language: \"python\"): \
             Standard library is available. `hashlib`, `json`, `math`, `datetime`, `re`, `itertools` all work. \
             `subprocess`, `socket`, `urllib`, `requests` are blocked. \
             SHA-256 example: import hashlib; print(hashlib.sha256(b'hello').hexdigest()) \
             \
             Do NOT use this tool for PowerShell or shell scripting. This is strictly for high-precision computation in JavaScript, TypeScript, or Python only. \
             Do NOT fall back to shell to run deno, python, or node — use this tool directly.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "language": {
                        "type": "string",
                        "enum": ["javascript", "typescript", "python"],
                        "description": "The language to run. javascript/typescript requires Deno; python requires Python 3."
                    },
                    "code": {
                        "type": "string",
                        "description": "The code to execute. Keep it short and self-contained. Print results to stdout."
                    },
                    "timeout_seconds": {
                        "type": "integer",
                        "description": "Max execution time in seconds (default 10, max 60). Use higher values for longer computations."
                    }
                },
                "required": ["language", "code"]
            }),
        ),

        make_tool(
            "trace_runtime_flow",
            "Return an authoritative read-only trace of Hematite runtime flow. \
             Use this for architecture questions about keyboard input to final output, \
             reasoning/specular separation, startup wiring, runtime subsystems, \
             voice synthesis and Ctrl+T toggle, or \
             session reset commands like /clear, /new, and /forget. Prefer this over guessing.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "topic": {
                        "type": "string",
                        "enum": ["user_turn", "session_reset", "reasoning_split", "runtime_subsystems", "startup", "voice"],
                        "description": "Which verified runtime report to return. Use 'voice' for any question about Ctrl+T, voice toggle, or TTS pipeline. Use 'user_turn' for keyboard-to-output flow. Use 'session_reset' for /clear, /forget, /new. Use 'startup' for startup wiring. Use 'reasoning_split' for specular/thought routing. Use 'runtime_subsystems' for background subsystem overview."
                    },
                    "input": {
                        "type": "string",
                        "description": "Optional user input to label a normal user-turn trace"
                    },
                    "command": {
                        "type": "string",
                        "enum": ["/clear", "/new", "/forget", "all"],
                        "description": "Optional reset command when topic=session_reset"
                    }
                },
                "required": ["topic"]
            }),
        ),
        make_tool(
            "describe_toolchain",
            "Return an authoritative read-only description of Hematite's actual tool surface and investigation strategy. \
             Use this for tooling-discipline questions, best-tool selection, or read-only plans for tracing runtime behavior. \
             Prefer this over improvising tool names or investigation steps from memory.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "topic": {
                        "type": "string",
                        "enum": ["read_only_codebase", "user_turn_plan", "voice_latency_plan", "host_inspection_plan", "all"],
                        "description": "Which authoritative toolchain report to return"
                    },
                    "question": {
                        "type": "string",
                        "description": "Optional user question to label or tailor the read-only investigation plan"
                    }
                }
            }),
        ),
        make_tool(
            "inspect_host",
            "Return a structured read-only inspection of the current machine and environment. \
             Prefer this over raw shell for questions about OS configuration (firewall, power, uptime), plain-English system health reports, silicon health and high-fidelity hardware telemetry (NVIDIA clocks/fans/power, CPU frequency averaging), installed developer tools, PATH issues, package-manager and environment health, network state, service state, running processes, desktop items, Downloads size, listening ports, repo health, or directory/disk summaries. \
             For high-performance hardware testing, use topic=disk_benchmark to measure real-time kernel disk queue intensity. \
             For remediation questions phrased like 'how do I fix cargo not found', 'how do I fix port 3000 already in use', or 'how do I fix LM Studio not reachable', use topic=fix_plan instead of diagnosis-only topics like env_doctor, path, or ports. \
             Use topic=summary for a compact host snapshot, topic=toolchains for common dev tool versions, topic=path for PATH analysis, topic=env_doctor for package-manager and PATH health, topic=fix_plan for structured remediation plans, topic=network for adapters/IPs/gateways/DNS, topic=services for service status and startup mode, \
             topic=processes for top processes by memory/cpu and real-time disk/network I/O stats (look for [I/O R:N/W:N] tags to identify disk-heavy processes), \
             topic=desktop or topic=downloads for known folders, topic=ports for listening endpoints, topic=repo_doctor for a structured workspace health report, \
             topic=log_check for recent critical/error events from system event logs or journalctl, topic=startup_items for programs and services that run at boot (registry Run keys and startup folders on Windows; systemd enabled units on Linux), \
             topic=health_report for a plain-English tiered system health verdict (disk, RAM, tools, recent errors), \
             topic=storage for all drives with capacity/free space plus large developer cache directories, \
             topic=hardware for CPU model/cores, RAM size/speed, GPU name/driver, motherboard, BIOS, and display configuration, \
             topic=updates for Windows Update status (last install date, pending update count, WU service state), \
             topic=security for Windows Defender real-time protection status, last scan date, signature age, firewall profile states, Windows activation, and UAC state, \
             topic=pending_reboot to check whether a system restart is required and why (Windows Update, CBS, file rename operations), \
             topic=disk_health for physical drive health via Get-PhysicalDisk and SMART failure prediction, \
             topic=battery for charge level, status, estimated runtime, and wear level (laptops only — reports no battery on desktops), \
             topic=recent_crashes for BSOD and unexpected shutdown events plus application crash/hang events from the Windows event log, \
             topic=scheduled_tasks for all non-disabled scheduled tasks including name, path, last run time, and executable, \
             topic=dev_conflicts for cross-tool environment conflict detection (Node.js version managers, Python 2 vs 3 ambiguity, conda env shadowing, Rust toolchain path conflicts, Git identity/signing config, duplicate PATH entries), \
             topic=bitlocker for drive encryption status (BitLocker on Windows, LUKS on Linux), \
             topic=ad_user for Active Directory / Managed Identity details (SID, group memberships, domain role), \
             topic=user_accounts for Local User and Group diagnostics (Built-in Administrators, local account state), \
             topic=rdp for Remote Desktop configuration, port, and active sessions, \
             topic=shadow_copies for Volume Shadow Copies (VSS) and system restore points, \
             topic=pagefile for Windows page file configuration and current usage, \
             topic=windows_features for enabled Windows optional features (IIS, Hyper-V, etc.), \
             topic=printers for installed printers and active print jobs, \
             topic=winrm for Windows Remote Management (WinRM) and PS Remoting status, \
             topic=network_stats for adapter throughput (RX/TX), errors, and dropped packets, \
             topic=udp_ports for active UDP listeners and notable port annotations, \
             topic=gpo for applied Group Policy Objects, topic=certificates for local personal certificates, topic=integrity for Windows component store health (SFC/DISM state), topic=domain for Active Directory and domain join status, \
             topic=device_health for identifying malfunctioning hardware with ConfigManager error codes (Yellow Bangs), topic=drivers for auditing active system drivers and their states, topic=peripherals for enumerating connected USB, input, and display hardware, \
             topic=sessions for auditing active and disconnected user logon sessions, \
             topic=ad_user for specific Active Directory user identity, SID, and group membership auditing, \
             topic=dns_lookup for precision DNS record queries (SRV, MX, TXT), \
             topic=hyperv for local Hyper-V VM inventory and real-time load, \
             topic=ip_config for detailed adapter configuration and DHCP lease state, \
             topic=disk_benchmark for high-performance silicon-aware stress testing, \
             and topic=directory or topic=disk for arbitrary paths.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "topic": {
                        "type": "string",
                        "enum": ["summary", "toolchains", "path", "env_doctor", "fix_plan", "network", "services", "processes", "desktop", "downloads", "directory", "disk", "ports", "repo_doctor", "log_check", "startup_items", "health_report", "storage", "hardware", "updates", "security", "pending_reboot", "disk_health", "battery", "recent_crashes", "scheduled_tasks", "dev_conflicts", "os_config", "bitlocker", "rdp", "shadow_copies", "pagefile", "windows_features", "printers", "winrm", "network_stats", "udp_ports", "gpo", "certificates", "integrity", "domain", "device_health", "drivers", "peripherals", "disk_benchmark", "permissions", "login_history", "registry_audit", "share_access", "thermal", "activation", "patch_history", "ad_user", "dns_lookup", "hyperv", "ip_config"],
                        "description": "Which structured host inspection to run. Use topic=ad_user for domain identity audit, topic=dns_lookup for SRV/MX records, topic=hyperv for VM load, and topic=ip_config for detailed adapter info."
                    },
                    "name": {
                        "type": "string",
                        "description": "Optional when topic=processes or topic=services. Case-insensitive substring filter for process or service names."
                    },
                    "issue": {
                        "type": "string",
                        "description": "Optional when topic=fix_plan. Plain-English issue description such as 'cargo not found', 'port 3000 already in use', or 'LM Studio not reachable on localhost:1234'."
                    },
                    "path": {
                        "type": "string",
                        "description": "Required when topic=directory. Optional for topic=disk or topic=repo_doctor. Absolute or relative path to inspect."
                    },
                    "port": {
                        "type": "integer",
                        "description": "Optional when topic=ports or topic=fix_plan. Filter the result to one listening TCP port or anchor a port-conflict fix plan."
                    },
                    "max_entries": {
                        "type": "integer",
                        "description": "Optional cap for listed entries. Defaults to 10 and is capped internally."
                    }
                }
            }),
        ),
        make_tool(
            "resolve_host_issue",
            "A safe, bounded tool for remediating OS and environment issues automatically with user approval. \
             Use this to fix missing dependencies, restart stuck services, or clear disk space instead of using raw shell. \
             The user will be prompted to approve the action. Keep targets exact.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["install_package", "restart_service", "clear_temp"],
                        "description": "The type of remediation to perform."
                    },
                    "target": {
                        "type": "string",
                        "description": "The specific target (e.g., 'python' for install_package, or 'docker' for restart_service). Optional for clear_temp."
                    }
                },
                "required": ["action"]
            }),
        ),
        make_tool(
            "run_hematite_maintainer_workflow",
            "Run one of Hematite's known maintainer or release workflows with explicit approval. \
             Prefer this over raw shell when the user explicitly asks to run one of Hematite's own scripts such as `clean.ps1`, `scripts/package-windows.ps1`, or `release.ps1`. \
             Use workflow=clean for cleanup, workflow=package_windows for rebuilding the local Windows portable or installer, and workflow=release for the normal version bump/tag/push/publish flow. \
             Keep this tool constrained to Hematite's own known workflows instead of inventing ad hoc shell commands or pretending to run arbitrary project scripts.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "workflow": {
                        "type": "string",
                        "enum": ["clean", "package_windows", "release"],
                        "description": "Which known Hematite maintainer workflow to run."
                    },
                    "deep": {
                        "type": "boolean",
                        "description": "For workflow=clean. Also remove heavy build/runtime artifacts such as target/ and vein.db."
                    },
                    "reset": {
                        "type": "boolean",
                        "description": "For workflow=clean. Reset PLAN/TASK state in addition to normal cleanup."
                    },
                    "prune_dist": {
                        "type": "boolean",
                        "description": "For workflow=clean. Keep only the current Cargo.toml version under dist/."
                    },
                    "installer": {
                        "type": "boolean",
                        "description": "For workflow=package_windows. Also build the Windows installer."
                    },
                    "add_to_path": {
                        "type": "boolean",
                        "description": "For workflow=package_windows or workflow=release. Update the user PATH to the rebuilt portable."
                    },
                    "version": {
                        "type": "string",
                        "description": "For workflow=release. Exact semantic version such as 0.4.5."
                    },
                    "bump": {
                        "type": "string",
                        "enum": ["patch", "minor", "major"],
                        "description": "For workflow=release. Ask release.ps1 to calculate the next version."
                    },
                    "push": {
                        "type": "boolean",
                        "description": "For workflow=release. Push main and the new tag."
                    },
                    "skip_installer": {
                        "type": "boolean",
                        "description": "For workflow=release. Skip the Windows installer build."
                    },
                    "publish_crates": {
                        "type": "boolean",
                        "description": "For workflow=release. Publish hematite-cli to crates.io after a successful push."
                    },
                    "publish_voice_crate": {
                        "type": "boolean",
                        "description": "For workflow=release. Publish hematite-kokoros first, then hematite-cli."
                    }
                },
                "required": ["workflow"]
            }),
        ),
        make_tool(
            "run_workspace_workflow",
            "Run an approval-gated workflow or script in the locked project workspace root. \
             Use this for the current project's build, test, lint, fix, package.json scripts, just/task/make targets, explicit local script paths, exact workspace commands, or typed website server control. \
             Website workflows are preferred when working on a local web app because they give Hematite a structured start/probe/validate/status/stop loop with stored runtime metadata instead of improvised shell. \
             FORBIDDEN: The `command` field MUST be a real executable shell command (e.g. `npm install`, `cargo build`). \
             NEVER put natural language, user-requests, or conversational intent into the `command` field. \
             This tool is for the active workspace, not for Hematite's own maintainer scripts.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "workflow": {
                        "type": "string",
                        "enum": ["build", "test", "lint", "fix", "package_script", "task", "just", "make", "script_path", "command", "website_start", "website_probe", "website_validate", "website_status", "website_stop"],
                        "description": "Which workspace workflow to run."
                    },
                    "name": {
                        "type": "string",
                        "description": "Required for workflow=package_script, task, just, or make. The script or target name."
                    },
                    "path": {
                        "type": "string",
                        "description": "Required for workflow=script_path. Relative path to a script inside the locked workspace root."
                    },
                    "command": {
                        "type": "string",
                        "description": "Required for workflow=command. Exact command to execute from the locked workspace root."
                    },
                    "mode": {
                        "type": "string",
                        "enum": ["dev", "preview", "start"],
                        "description": "Optional for workflow=website_start. Which website server mode to infer. Defaults to dev."
                    },
                    "script": {
                        "type": "string",
                        "description": "Optional for workflow=website_start. Exact package.json script to run instead of inferring one."
                    },
                    "url": {
                        "type": "string",
                        "description": "Optional for workflow=website_start, website_probe, or website_validate. Explicit local URL to probe, such as http://127.0.0.1:5173/."
                    },
                    "host": {
                        "type": "string",
                        "description": "Optional for workflow=website_start. Host used when constructing an inferred probe URL. Defaults to 127.0.0.1."
                    },
                    "port": {
                        "type": "integer",
                        "description": "Optional for workflow=website_start. Port used when constructing an inferred probe URL."
                    },
                    "label": {
                        "type": "string",
                        "description": "Optional for website workflows. Logical server name for storing runtime metadata. Defaults to default."
                    },
                    "routes": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Optional for workflow=website_validate. Relative routes or absolute URLs to validate, such as [\"/\", \"/pricing\", \"/about\"]."
                    },
                    "asset_limit": {
                        "type": "integer",
                        "description": "Optional for workflow=website_validate. Maximum number of linked local assets to probe after route validation."
                    },
                    "request_timeout_ms": {
                        "type": "integer",
                        "description": "Optional for workflow=website_start. Per-request HTTP timeout used by the readiness probe."
                    },
                    "timeout_ms": {
                        "type": "integer",
                        "description": "Optional timeout override in milliseconds. For website_start this is the boot/readiness timeout. For website_probe and website_status it is the probe timeout."
                    }
                },
                "required": ["workflow"]
            }),
        ),
        make_tool(
            "read_file",
            "Read the contents of a file. For large files, use 'offset' and 'limit' to navigate.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Path to the file, relative to the project root"
                    },
                    "offset": {
                        "type": "integer",
                        "description": "Starting line number (0-indexed)"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Number of lines to read"
                    }
                },
                "required": ["path"]
            }),
        ),
        make_tool(
            "lsp_definitions",
            "Get the precise definition location (file:line:char) for a symbol at a specific position. \
             Use this to jump to function/struct source code accurately.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "File path" },
                    "line": { "type": "integer", "description": "0-indexed line" },
                    "character": { "type": "integer", "description": "0-indexed character" }
                },
                "required": ["path", "line", "character"]
            }),
        ),
        make_tool(
            "lsp_references",
            "Find all locations where a symbol is used across the entire workspace. \
             Use this to understand the impact of a refactor or discover internal API users.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "File path" },
                    "line": { "type": "integer", "description": "0-indexed line" },
                    "character": { "type": "integer", "description": "0-indexed character" }
                },
                "required": ["path", "line", "character"]
            }),
        ),
        make_tool(
            "lsp_hover",
            "Get hover information (documentation, function signature, type details) for a symbol. \
             Use this for rapid spatial awareness without opening every file.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "File path" },
                    "line": { "type": "integer", "description": "0-indexed line" },
                    "character": { "type": "integer", "description": "0-indexed character" }
                },
                "required": ["path", "line", "character"]
            }),
        ),
        make_tool(
            "lsp_rename_symbol",
            "Rename a symbol project-wide using the Language Server. Ensures all references are updated safely.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "File path" },
                    "line": { "type": "integer", "description": "0-indexed line" },
                    "character": { "type": "integer", "description": "0-indexed character" },
                    "new_name": { "type": "string", "description": "The new name for the symbol" }
                },
                "required": ["path", "line", "character", "new_name"]
            }),
        ),
        make_tool(
            "lsp_get_diagnostics",
            "Get a list of current compiler errors and warnings for a specific file. \
             Use this to verify your code compiles and and to find exactly where errors are located.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "File path" }
                },
                "required": ["path"]
            }),
        ),
        make_tool(
            "vision_analyze",
            "Send an image file (screenshot, diagram, or UI mockup) to the multimodal vision model for technical analysis. \
             Use this to identify UI bugs, confirm visual states, or understand architectural diagrams.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Absolute or relative path to the image file." },
                    "prompt": { "type": "string", "description": "The specific question or analysis request for the vision model." }
                },
                "required": ["path", "prompt"]
            }),
        ),
        make_tool(
            "patch_hunk",
            "Replace a specific line range [start_line, end_line] with new content. \
             This is the most precise way to edit code and avoids search string failures.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "File path" },
                    "start_line": { "type": "integer", "description": "Starting line (1-indexed)" },
                    "end_line": { "type": "integer", "description": "Ending line (inclusive)" },
                    "replacement": { "type": "string", "description": "The new content for this range" }
                },
                "required": ["path", "start_line", "end_line", "replacement"]
            }),
        ),
        make_tool(
            "multi_search_replace",
            "Replace multiple existing code blocks in a single file with new content. \
             Each hunk specifies an EXACT 'search' string and a 'replace' string. \
             The 'search' string MUST exactly match the existing file contents (including whitespace). \
             This is the safest and most reliable way to make multiple structural edits.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "File path" },
                    "hunks": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "search": { "type": "string", "description": "Exact existing text to find and replace" },
                                "replace": { "type": "string", "description": "The new replacement text" }
                            },
                            "required": ["search", "replace"]
                        }
                    }
                },
                "required": ["path", "hunks"]
            }),
        ),
        make_tool(
            "write_file",
            "Write content to a file, creating it (and any parent dirs) if needed. \
             Overwrites existing files. \
             SOVEREIGN PATHING: For files in common areas, use `@DESKTOP/file.txt`, `@DOCUMENTS/file.txt`, `@DOWNLOADS/file.txt`, or `@HOME/file.txt` to ensure 100% path accuracy.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "File path" },
                    "content": { "type": "string", "description": "Full file content to write" }
                },
                "required": ["path", "content"]
            }),
        ),
        make_tool(
            "create_directory",
            "Authoritatively create a new directory (and any parent dirs) if they do not exist. \
             Use this instead of raw shell (mkdir) for all filesystem organization. \
             Supports both relative paths and absolute paths. \
             SOVEREIGN PATHING: For directories in common areas, use `@DESKTOP/folder`, `@DOCUMENTS/folder`, `@DOWNLOADS/folder`, or `@HOME/folder` to ensure 100% path accuracy.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Relative or absolute directory path" }
                },
                "required": ["path"]
            }),
        ),
        make_tool(
            "research_web",
            "Perform a zero-cost technical search using DuckDuckGo. \
             Use this to find documentation, latest API changes, or solutions to complex errors \
             when your internal knowledge is insufficient. Returns snippets and URLs.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "The technical search query" }
                },
                "required": ["query"]
            }),
        ),
        make_tool(
            "fetch_docs",
            "Fetch a URL and convert it to clean Markdown. Use this to 'read' the documentation \
             links found via research_web. This tool uses a proxy to bypass IP blocks.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "url": { "type": "string", "description": "The URL of the documentation to fetch" }
                },
                "required": ["url"]
            }),
        ),
        make_tool(
            "edit_file",
            "Edit a file by replacing an exact string with another. \
             The 'search' string does NOT need perfectly matching indentation (it is fuzzy), \
             but the non-whitespace text must match exactly. Use this for targeted edits.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "File path" },
                    "search": {
                        "type": "string",
                        "description": "The exact text to find (must match whitespace/indentation precisely)"
                    },
                    "replace": {
                        "type": "string",
                        "description": "The replacement text"
                    }
                },
                "required": ["path", "search", "replace"]
            }),
        ),
        make_tool(
            "auto_pin_context",
            "Select 1-3 core files to 'Lock' into prioritized memory. \
             Use this to ensure the most important architecture files \
             are always visible during complex refactorings.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "paths": {
                        "type": "array",
                        "items": { "type": "string" }
                    },
                    "reason": { "type": "string" }
                },
                "required": ["paths", "reason"]
            }),
        ),
        make_tool(
            "list_pinned",
            "List all files currently pinned in the model's active context.",
            serde_json::json!({
                "type": "object",
                "properties": {}
            }),
        ),
        make_tool(
            "list_files",
            "List files in a directory, optionally filtered by extension.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Directory to list (default: current dir)"
                    },
                    "extension": {
                        "type": "string",
                        "description": "Only return files with this extension, e.g. 'rs', 'toml' (no dot)"
                    }
                },
                "required": []
            }),
        ),
        make_tool(
            "tail_file",
            "Read the last N lines of a file — useful for log files, test output, \
             build artifacts, and any large file where only the tail is relevant. \
             Supports an optional grep filter to show only matching lines from the tail. \
             Use this instead of read_file when you only need the end of a large file.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Path to the file, relative to the project root"
                    },
                    "lines": {
                        "type": "integer",
                        "description": "Number of lines to return from the end (default: 50, max: 500)"
                    },
                    "grep": {
                        "type": "string",
                        "description": "Optional regex pattern — only return lines matching this pattern (applied before the tail slice)"
                    }
                },
                "required": ["path"]
            }),
        ),
        make_tool(
            "grep_files",
            "Search file contents for a regex pattern. Supports context lines, files-only mode, \
             and pagination. Returns file:line:content format by default.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "pattern": {
                        "type": "string",
                        "description": "Regex pattern to search for (case-insensitive by default)"
                    },
                    "path": {
                        "type": "string",
                        "description": "Directory to search (default: current dir)"
                    },
                    "extension": {
                        "type": "string",
                        "description": "Only search files with this extension, e.g. 'rs'"
                    },
                    "mode": {
                        "type": "string",
                        "enum": ["content", "files_only"],
                        "description": "'content' (default) returns matching lines; 'files_only' returns only filenames"
                    },
                    "context": {
                        "type": "integer",
                        "description": "Lines of context before AND after each match (like rg -C)"
                    },
                    "before": {
                        "type": "integer",
                        "description": "Lines of context before each match (overrides context)"
                    },
                    "after": {
                        "type": "integer",
                        "description": "Lines of context after each match (overrides context)"
                    },
                    "head_limit": {
                        "type": "integer",
                        "description": "Max hunks (or files in files_only) to return (default: 50)"
                    },
                    "offset": {
                        "type": "integer",
                        "description": "Skip first N hunks/files - for pagination (default: 0)"
                    }
                },
                "required": ["pattern"]
            }),
        ),
        make_tool(
            "git_commit",
            "Stage all changes (git add -A) and create a commit. You MUST use 'Conventional Commits' (e.g. 'feat: description').",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "message": { "type": "string", "description": "Commit message (Conventional Commit style)" }
                },
                "required": ["message"]
            }),
        ),
        make_tool(
            "git_push",
            "Push current branched changes to the remote origin. Requires an existing remote connection.",
            serde_json::json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        ),
        make_tool(
            "git_remote",
            "View or manage git remotes. Use this for onboarding to GitHub/GitLab services.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["list", "add", "remove"],
                        "description": "Operation to perform"
                    },
                    "name": { "type": "string", "description": "Remote name (e.g. origin)" },
                    "url": { "type": "string", "description": "Remote URL (for 'add' action)" }
                },
                "required": ["action"]
            }),
        ),
        make_tool(
            "git_onboarding",
            "High-level wizard to connect this repository to a remote host (GitHub/GitLab). \
             Handles adding the remote and performing the initial tracking push in one step.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "url": { "type": "string", "description": "The remote repository URL (HTTPS or SSH)" },
                    "name": { "type": "string", "description": "The remote name (default: origin)" },
                    "push": { "type": "boolean", "description": "Whether to perform an initial push to establish tracking (default: false)" }
                },
                "required": ["url"]
            }),
        ),
        make_tool(
            "verify_build",
            "Run project verification for build, test, lint, or fix workflows. \
             Prefer per-project verify profiles from `.hematite/settings.json`, and fall back to \
             auto-detected defaults when no profile is configured. Returns BUILD OK or BUILD FAILED \
             with command output. ALWAYS call this after scaffolding a new project or making structural changes.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["build", "test", "lint", "fix"],
                        "description": "Which verification action to run. Defaults to build."
                    },
                    "profile": {
                        "type": "string",
                        "description": "Optional named verify profile from `.hematite/settings.json`."
                    },
                    "timeout_secs": {
                        "type": "integer",
                        "description": "Optional timeout override for this verification run."
                    }
                }
            }),
        ),
        make_tool(
            "git_worktree",
            "Manage Git worktrees - isolated working directories on separate branches. \
             Use 'add' to create a safe sandbox for risky/experimental work, \
             'list' to see all worktrees, 'remove' to clean up, 'prune' to remove stale entries.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["list", "add", "remove", "prune"],
                        "description": "Worktree operation to perform"
                    },
                    "path": {
                        "type": "string",
                        "description": "Directory path for the new worktree (required for add/remove)"
                    },
                    "branch": {
                        "type": "string",
                        "description": "Branch name for the worktree (add only; defaults to path basename)"
                    }
                },
                "required": ["action"]
            }),
        ),
        make_tool(
            "clarify",
            "Ask the user a clarifying question when you genuinely cannot proceed without \
             more information. Use this ONLY when you are blocked and cannot make a \
             reasonable assumption. Do NOT use it to ask permission - just act.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "question": {
                        "type": "string",
                        "description": "The specific question to ask the user"
                    }
                },
                "required": ["question"]
            }),
        ),
        make_tool(
            "manage_tasks",
            "Manage the persistent task ledger in .hematite/TASK.md. Use this to track long-term goals across restarts.",
            crate::tools::tasks::get_tasks_params(),
        ),
        make_tool(
            "maintain_plan",
            "Document the architectural strategy and session blueprint in .hematite/PLAN.md. Use this to maintain context across restarts.",
            crate::tools::plan::get_plan_params(),
        ),
        make_tool(
            "generate_walkthrough",
            "Generate a final session report in .hematite/WALKTHROUGH.md including achievements and verification results.",
            crate::tools::plan::get_walkthrough_params(),
        ),
        make_tool(
            "swarm",
            "Delegate high-volume parallel tasks to a swarm of background workers. \
             Use this for large-scale refactors, multi-file research, or parallel documentation updates. \
             You must provide a 'tasks' array where each task has an 'id', 'target' (file), and 'instruction'.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "tasks": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "id": { "type": "string" },
                                "target": { "type": "string", "description": "Target file or directory" },
                                "instruction": { "type": "string", "description": "Specific task for this worker" }
                            },
                            "required": ["id", "target", "instruction"]
                        }
                    },
                    "max_workers": {
                        "type": "integer",
                        "description": "Max parallel workers (default 3, auto-throttled by hardware)",
                        "default": 3
                    }
                },
                "required": ["tasks"]
            }),
        ),
    ];

    let lsp_defs = crate::tools::lsp_tools::get_lsp_definitions();
    tools.push(make_tool(
        "lsp_search_symbol",
        "Find the location (file/line) of any function, struct, or variable in the entire project workspace. \
         This is the fastest 'Golden Path' for navigating to a symbol by name.",
        serde_json::json!({
            "type": "object",
            "properties": {
                "query": { "type": "string", "description": "The name of the symbol to find (e.g. 'initialize_mcp')" }
            },
            "required": ["query"]
        }),
    ));
    for def in lsp_defs {
        let name = def["name"].as_str().unwrap();
        tools.push(ToolDefinition {
            tool_type: "function".into(),
            function: ToolFunction {
                name: name.into(),
                description: def["description"].as_str().unwrap().into(),
                parameters: def["parameters"].clone(),
            },
            metadata: tool_metadata_for_name(name),
        });
    }

    tools
}

pub async fn dispatch_builtin_tool(
    name: &str,
    args: &Value,
    config: &HematiteConfig,
    budget_tokens: usize,
) -> Result<String, String> {
    match name {
        "shell" => crate::tools::shell::execute(args, budget_tokens).await,
        "run_code" => crate::tools::code_sandbox::execute(args).await,
        "trace_runtime_flow" => crate::tools::runtime_trace::trace_runtime_flow(args).await,
        "describe_toolchain" => crate::tools::toolchain::describe_toolchain(args).await,
        "inspect_host" => crate::tools::host_inspect::inspect_host(args).await,
        "resolve_host_issue" => crate::tools::host_inspect::resolve_host_issue(args).await,
        "run_hematite_maintainer_workflow" => {
            crate::tools::repo_script::run_hematite_maintainer_workflow(args).await
        }
        "run_workspace_workflow" => crate::tools::workspace_workflow::run_workspace_workflow(args).await,
        "read_file" => crate::tools::file_ops::read_file(args, budget_tokens).await,
        "inspect_lines" => crate::tools::file_ops::inspect_lines(args).await,
        "tail_file" => crate::tools::file_ops::tail_file(args).await,
        "write_file" => crate::tools::file_ops::write_file(args).await,
        "create_directory" => crate::tools::file_ops::create_directory(args).await,
        "edit_file" => crate::tools::file_ops::edit_file(args).await,
        "patch_hunk" => crate::tools::file_ops::patch_hunk(args).await,
        "multi_search_replace" => crate::tools::file_ops::multi_search_replace(args).await,
        "list_files" => crate::tools::file_ops::list_files(args, budget_tokens).await,
        "grep_files" => crate::tools::file_ops::grep_files(args, budget_tokens).await,
        "git_commit" => crate::tools::git::execute(args).await,
        "git_push" => crate::tools::git::execute_push(args).await,
        "git_remote" => crate::tools::git::execute_remote(args).await,
        "git_onboarding" => crate::tools::git_onboarding::execute(args).await,
        "verify_build" => crate::tools::verify_build::execute(args).await,
        "git_worktree" => crate::tools::git::execute_worktree(args).await,
        "health" => crate::tools::health::execute(args).await,
        "research_web" => {
            crate::tools::research::execute_search(args, config.searx_url.clone()).await
        }
        "fetch_docs" => crate::tools::research::execute_fetch(args).await,
        "manage_tasks" => crate::tools::tasks::manage_tasks(args).await,
        "maintain_plan" => crate::tools::plan::maintain_plan(args).await,
        "generate_walkthrough" => crate::tools::plan::generate_walkthrough(args).await,
        "clarify" => {
            let q = args.get("question").and_then(|v| v.as_str()).unwrap_or("?");
            Ok(format!("[clarify] {q}"))
        }
        "vision_analyze" => Err(
            "Tool 'vision_analyze' must be dispatched by ConversationManager (it requires hardware engine access)."
                .into(),
        ),
        other => {
            if other.contains('.') || other.contains('/') || other.contains('\\') {
                Err(format!(
                    "'{}' is a PATH, not a tool. You correctly identified the location, but you MUST use `read_file` or `list_files` (internal) or `powershell` (external) to access it.",
                    other
                ))
            } else if matches!(other.to_lowercase().as_str(), "hematite" | "assistant" | "ai") {
                Err(format!(
                    "'{}' is YOUR IDENTITY, not a tool. Use list_files or read_file to explore the codebase.",
                    other
                ))
            } else if matches!(
                other.to_lowercase().as_str(),
                "thought" | "think" | "reasoning" | "thinking" | "internal"
            ) {
                Err(format!(
                    "'{}' is NOT a tool - it is a reasoning tag. Output your answer as plain text after your <think> block.",
                    other
                ))
            } else {
                Err(format!("Unknown tool: '{}'", other))
            }
        }
    }
}

pub fn get_mutation_label(name: &str, args: &Value) -> Option<String> {
    match name {
        "shell" => {
            let cmd = args.get("command").and_then(|v| v.as_str()).unwrap_or("");
            if cmd.contains("rm ") || cmd.contains("del ") {
                Some("Destructive File Deletion".into())
            } else if cmd.contains("mkdir ") {
                Some("Directory Creation".into())
            } else {
                Some("Execute Shell Command".into())
            }
        }
        "write_file" => {
            let path = args.get("path").and_then(|v| v.as_str()).unwrap_or("file");
            Some(format!("Create/Overwrite File: {}", path))
        }
        "create_directory" => {
            let path = args
                .get("path")
                .and_then(|v| v.as_str())
                .unwrap_or("folder");
            Some(format!("Create Directory: {}", path))
        }
        "edit_file" | "patch_hunk" | "multi_search_replace" => {
            let path = args.get("path").and_then(|v| v.as_str()).unwrap_or("file");
            Some(format!("Surgical Code Mutation: {}", path))
        }
        "git_commit" => Some("Permanent Version History Commit".into()),
        "git_push" => Some("Remote Origin Synchronisation (Push)".into()),
        "resolve_host_issue" => Some("System-Level Host Remediation".into()),
        "run_workspace_workflow" => Some("Automated Workspace Re-alignment".into()),
        _ => None,
    }
}
