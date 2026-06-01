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
            "query_data",
            "Execute an analytical SQL query against a local file (CSV, JSON, or SQLite .db) using SQLite semantics. \
             Use this for high-precision data analysis, aggregation, and filtering without writing custom scripts. \
             For CSV and JSON files, the table name is always 'source'. \
             For SQLite (.db) files, use the actual table names defined in the schema. \
             Results are returned as a formatted table (max 100 rows).",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "sql": { "type": "string", "description": "The SQL query to run (e.g. SELECT count(*), category FROM source GROUP BY category;)" },
                    "path": { "type": "string", "description": "Relative path to the data file (CSV, JSON, or .db) inside the project root." },
                    "explain": { "type": "boolean", "description": "If true, returns the SQL execution plan (EXPLAIN QUERY PLAN) instead of the results." }
                },
                "required": ["sql", "path"]
            }),
        ),
        make_tool(
            "export_as_table",
            "Persist a structured list of objects (JSON array) to a local CSV or SQLite file. \
             Use this to save research results, system snapshots, or data analysis outputs for later use. \
             Hematite will automatically create the table schema or CSV header based on the object keys.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "items": { "type": "array", "items": { "type": "object" }, "description": "The list of JSON objects to export." },
                    "path": { "type": "string", "description": "Relative path to save the file (e.g. 'results.csv' or 'audit.db')." },
                    "format": { "type": "string", "enum": ["csv", "sqlite"], "description": "The output format (default: csv)." }
                },
                "required": ["items", "path"]
            }),
        ),
        make_tool(
            "analyze_trends",
            "Perform statistical analysis and generate an ASCII histogram from a SQL query result. \
             This tool pipes SQL data into a Python sandbox to calculate Mean, Median, StdDev, and distribution. \
             Use this to find patterns, anomalies, or trends in large datasets without manual calculation.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "sql": { "type": "string", "description": "The SQL query to run (must return at least one numeric column)." },
                    "path": { "type": "string", "description": "Relative path to the data file (.db, .csv, or .json)." }
                },
                "required": ["sql", "path"]
            }),
        ),
        make_tool(
            "scientific_compute",
            "Advanced computational research: symbolic math, unit-safety, complexity auditing, ledger memory, and dataset math.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "mode": { "type": "string", "enum": ["symbolic", "units", "complexity", "ledger", "dataset"] },
                    "expr": { "type": "string", "description": "Equation/expression for symbolic mode." },
                    "calculation": { "type": "string", "description": "Calculation for units mode (e.g. 10m/2s)." },
                    "snippet": { "type": "string", "description": "Python snippet for complexity auditing (loop over n)." },
                    "target": { "type": "string", "enum": ["solve", "simplify", "integrate", "diff"], "description": "Symbolic operation." },
                    "latex": { "type": "boolean", "description": "Toggle LaTeX output for symbolic mode." },
                    "action": { "type": "string", "enum": ["read", "append"], "description": "Ledger action." },
                    "content": { "type": "string", "description": "Derivation content for ledger append." },
                    "path": { "type": "string", "description": "Path to dataset (.db, .csv, .json) for dataset mode." },
                    "sql": { "type": "string", "description": "SQL query to fetch data for dataset mode." },
                    "python_op": { "type": "string", "description": "Python operation for dataset mode (e.g. 'sum(vals)/len(vals)')." }
                },
                "required": ["mode"]
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
             topic=mdm_enrollment for Intune/MDM enrollment state, Azure AD join, and device management health, \
             topic=hyperv for local Hyper-V VM inventory and real-time load, \
             topic=ip_config for detailed adapter configuration and DHCP lease state, \
             topic=disk_benchmark for high-performance silicon-aware stress testing, \
             topic=storage_spaces for Windows Storage Spaces pools, virtual disks, physical disk health, and Linux mdadm/LVM, \
             topic=defender_quarantine for Windows Defender threat detections, quarantine history, and scan summary, \
             topic=domain_health for domain controller connectivity, LDAP port tests, dsregcmd join state, and GPO last refresh, \
             topic=service_dependencies for service dependency graph (what requires what, restart cascade planning), \
             topic=wmi_health for WMI repository integrity, winmgmt verify, and repair steps, \
             topic=local_security_policy for password/lockout policy, LM compatibility level, and UAC settings, \
             topic=usb_history for USB device connection history from the USBSTOR registry, \
             topic=print_spooler for Print Spooler state, PrintNightmare (CVE-2021-34527) hardening check, and print queue, \
             and topic=directory or topic=disk for arbitrary paths.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "topic": {
                        "type": "string",
                        "enum": ["summary", "toolchains", "path", "env_doctor", "fix_plan", "network", "services", "processes", "desktop", "downloads", "directory", "disk", "ports", "repo_doctor", "log_check", "startup_items", "health_report", "storage", "hardware", "updates", "security", "pending_reboot", "disk_health", "battery", "recent_crashes", "scheduled_tasks", "dev_conflicts", "os_config", "bitlocker", "rdp", "shadow_copies", "pagefile", "windows_features", "printers", "winrm", "network_stats", "udp_ports", "gpo", "certificates", "integrity", "domain", "domain_health", "device_health", "drivers", "peripherals", "disk_benchmark", "permissions", "login_history", "registry_audit", "share_access", "thermal", "activation", "patch_history", "ad_user", "dns_lookup", "hyperv", "ip_config", "mdm_enrollment", "storage_spaces", "defender_quarantine", "service_dependencies", "wmi_health", "local_security_policy", "usb_history", "print_spooler"],
                        "description": "Which structured host inspection to run. Use topic=ad_user for domain identity audit, topic=dns_lookup for SRV/MX records, topic=hyperv for VM load, topic=ip_config for detailed adapter info, topic=mdm_enrollment for Intune/MDM enrollment state, topic=storage_spaces for Windows Storage Spaces/RAID pools, topic=defender_quarantine for Defender threat history, topic=domain_health for DC connectivity and LDAP tests, topic=service_dependencies for restart cascade planning, topic=wmi_health for WMI repository integrity, topic=local_security_policy for password/lockout/NTLMv2 policy, topic=usb_history for USB forensics, and topic=print_spooler for PrintNightmare check."
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
            "github_ops",
            "Interact with GitHub via the `gh` CLI. Requires `gh` installed and `gh auth login` completed. \
             Use for pull requests, issues, CI run status, and repo metadata. \
             Never use `shell` to call `gh` — use this tool instead.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": [
                            "pr_list", "pr_view", "pr_create", "pr_status", "pr_checks", "pr_merge",
                            "issue_list", "issue_view", "issue_create",
                            "ci_status", "run_view",
                            "repo_view", "release_list"
                        ],
                        "description": "GitHub operation to perform"
                    },
                    "title": { "type": "string", "description": "PR or issue title (for create actions)" },
                    "body": { "type": "string", "description": "PR or issue body (for create actions)" },
                    "base": { "type": "string", "description": "Base branch for PR (default: main)" },
                    "draft": { "type": "boolean", "description": "Create PR as draft" },
                    "pr": { "type": "string", "description": "PR number or URL (for view/checks/merge)" },
                    "number": { "description": "Issue number (for issue_view)" },
                    "state": { "type": "string", "enum": ["open", "closed", "all"], "description": "Filter state for listings" },
                    "strategy": { "type": "string", "enum": ["merge", "squash", "rebase"], "description": "Merge strategy for pr_merge" },
                    "branch": { "type": "string", "description": "Branch name for ci_status (defaults to current branch)" },
                    "run_id": { "type": "string", "description": "Run ID for run_view" },
                    "limit": { "type": "integer", "description": "Max results to return (default 10)" }
                },
                "required": ["action"]
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
            "vein_search",
            "Search the local Vein RAG index for code, session memory, or imported chat context \
             relevant to a query. Returns the top matching chunks with file path, room label, and \
             relevance score. Uses hybrid BM25+semantic ranking when an embedding model is loaded, \
             BM25-only otherwise. Use this when you need to recall earlier conversation context, \
             find related code in the project, or retrieve specific imported session content \
             without waiting for the next turn's automatic pre-retrieval pass.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Natural-language search query (e.g. 'how does the inference engine handle streaming?')."
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Maximum number of chunks to return (default 8, max 20)."
                    }
                },
                "required": ["query"]
            }),
        ),
        make_tool(
            "run_with_backtrace",
            "Run a command with RUST_BACKTRACE=full and return a structured crash report — \
             panic message, filtered stack trace (stdlib noise removed), and exit code. \
             Use this when you need to understand WHY a binary panics or crashes at runtime, \
             not just that it does. Prefer this over shell for any 'why does it crash?' question.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "Command to run (e.g. './target/debug/myapp --args'). \
                                        Executed with RUST_BACKTRACE=full automatically."
                    },
                    "timeout_secs": {
                        "type": "integer",
                        "description": "Max runtime in seconds before the process is killed (default 30)."
                    }
                },
                "required": ["command"]
            }),
        ),
        make_tool(
            "profile_process",
            "Run a command and collect a lightweight CPU/RAM profile — wall time, peak memory, \
             peak CPU, sample count, and trimmed output. Polls the live process every 500 ms. \
             Use this when you need to measure how long a command takes or how much memory it peaks at, \
             without installing any extra profiling tools. Works on Windows (Get-Process) and Linux (/proc).",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "Command to profile (e.g. 'cargo build --release' or './target/release/app')."
                    },
                    "timeout_secs": {
                        "type": "integer",
                        "description": "Max runtime in seconds before the process is killed (default 60)."
                    }
                },
                "required": ["command"]
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
            "cargo_errors",
            "Run `cargo check --message-format=json` and return a structured list of compiler \
             errors with file path, line number, error code (e.g. E0716), message, and hints. \
             Far easier to act on than raw cargo output — each error is one line with an exact \
             file:line reference. Use this after a failed verify_build to get a precise action list. \
             Set warnings=true to include compiler warnings. Set tests=true to check test targets. \
             Set action=clippy to run Clippy lints instead of check.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["check", "build", "clippy"],
                        "description": "Which cargo command to run for diagnostics (default: check)"
                    },
                    "warnings": {
                        "type": "boolean",
                        "description": "Include compiler warnings in addition to errors (default false)"
                    },
                    "tests": {
                        "type": "boolean",
                        "description": "Check test targets as well as the main library/binary (default false)"
                    },
                    "explain": {
                        "type": "boolean",
                        "description": "Append rustc --explain output for each unique error code (default false). Useful when you don't recognise an error code."
                    },
                    "package": {
                        "type": "string",
                        "description": "Limit to a specific workspace package (optional)"
                    }
                }
            }),
        ),
        make_tool(
            "git_status",
            "Return a structured snapshot of the working tree: current branch, \
             ahead/behind upstream, staged files (+), modified files (~), and untracked files (?). \
             Faster and more parseable than shelling out `git status`. \
             Use this at the start of any commit/PR workflow to understand what's changed.",
            serde_json::json!({
                "type": "object",
                "properties": {}
            }),
        ),
        make_tool(
            "git_diff",
            "Show differences between the working tree, staged changes, or two git refs. \
             Always returns a --stat summary header (files changed, insertions, deletions) \
             followed by the full unified diff capped at 12 KB. \
             Use mode=stat for a compact file-list-only view. \
             Use staged=true to diff the index vs HEAD (i.e. what will be committed). \
             Use from/to for ref comparison (e.g. from='main', to='HEAD'). \
             Use path to scope to a single file when the full diff is too large.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "mode": {
                        "type": "string",
                        "enum": ["diff", "stat"],
                        "description": "diff = full unified diff with stat header (default); stat = compact summary only"
                    },
                    "staged": {
                        "type": "boolean",
                        "description": "Diff the index (staged changes) against HEAD instead of the working tree (default false)"
                    },
                    "from": {
                        "type": "string",
                        "description": "Starting ref (commit hash, branch, or tag). Omit to diff working tree vs HEAD."
                    },
                    "to": {
                        "type": "string",
                        "description": "Ending ref (optional; defaults to HEAD when from is set)"
                    },
                    "path": {
                        "type": "string",
                        "description": "Scope diff to this file or directory path"
                    }
                }
            }),
        ),
        make_tool(
            "git_log",
            "Show commit history for the current branch or a specific file. \
             Returns hash, author, date, and subject for the last N commits. \
             Use path to see history for a single file. \
             Use from/to for a ref range (e.g. from='main', to='HEAD'). \
             Use oneline=true for a compact one-line-per-commit format.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "n": {
                        "type": "integer",
                        "description": "Number of commits to return (default 20, max 200)"
                    },
                    "oneline": {
                        "type": "boolean",
                        "description": "Compact format: one line per commit showing hash and subject (default false)"
                    },
                    "from": {
                        "type": "string",
                        "description": "Starting ref for a range (e.g. 'main')"
                    },
                    "to": {
                        "type": "string",
                        "description": "Ending ref for a range (default HEAD)"
                    },
                    "path": {
                        "type": "string",
                        "description": "Scope to commits that touched this file or directory"
                    }
                }
            }),
        ),
        make_tool(
            "changelog_gen",
            "Generate a structured changelog from git commit history grouped by conventional commit type \
             (feat, fix, perf, refactor, docs, test, chore, ci, build, style). \
             Supports scoping to a version range (from='v0.11.0', to='v0.12.0') or recent N commits. \
             Output is formatted Markdown with a section per commit type, scope in bold, and short hash. \
             Use title to customize the changelog heading. \
             Example: changelog_gen(from: 'v0.11.0', title: 'v0.12.0') to document a release.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "n": {
                        "type": "integer",
                        "description": "Number of recent commits to include (default 100, max 500). Ignored when from/to is set."
                    },
                    "from": {
                        "type": "string",
                        "description": "Starting ref or tag (e.g. 'v0.11.0'). Commits after this ref are included."
                    },
                    "to": {
                        "type": "string",
                        "description": "Ending ref or tag (default HEAD)."
                    },
                    "title": {
                        "type": "string",
                        "description": "Heading for the changelog (default 'Changelog')."
                    }
                }
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

    tools.push(make_tool(
        "refactor_rename",
        "Rename a symbol (function, type, variable, constant, etc.) across the entire workspace. \
         Performs whole-word replacement — 'run' will NOT match 'run_turn'. \
         Defaults to dry_run=true so you always see a per-file preview before any files are written. \
         Set dry_run=false to apply. Works on .rs files by default; set extensions='rs,toml' for wider scope. \
         Use find_symbol first to verify all sites, then call this to apply in one shot.",
        serde_json::json!({
            "type": "object",
            "properties": {
                "old_name": {
                    "type": "string",
                    "description": "Current symbol name to rename (exact match, whole-word)"
                },
                "new_name": {
                    "type": "string",
                    "description": "Replacement name"
                },
                "dry_run": {
                    "type": "boolean",
                    "description": "Preview changes without writing files (default true). Set false to apply."
                },
                "extensions": {
                    "type": "string",
                    "description": "Comma-separated file extensions (default 'rs'). E.g. 'rs,toml,md'."
                }
            },
            "required": ["old_name", "new_name"]
        }),
    ));
    tools.push(make_tool(
        "find_symbol",
        "Locate where a Rust symbol (function, struct, enum, trait, impl, type, const, macro, mod) \
         is *defined* anywhere in the workspace — no LSP required. Works immediately even without \
         rust-analyzer running. Returns file:line, kind label, and the declaration line for each hit. \
         Use lsp_search_symbol when rust-analyzer is active for richer results; use find_symbol as \
         the always-available fallback or when you only need the declaration site. \
         Set definitions_only=false to also include call/usage sites.",
        serde_json::json!({
            "type": "object",
            "properties": {
                "symbol": {
                    "type": "string",
                    "description": "Exact symbol name to search for (e.g. 'execute_streaming', 'ConversationManager')"
                },
                "kind": {
                    "type": "string",
                    "enum": ["fn", "struct", "enum", "trait", "impl", "type", "const", "static", "mod", "macro"],
                    "description": "Restrict search to a specific declaration kind (optional)"
                },
                "definitions_only": {
                    "type": "boolean",
                    "description": "Only return declaration sites, not usage/call sites (default true)"
                }
            },
            "required": ["symbol"]
        }),
    ));
    tools.push(make_tool(
        "format_code",
        "Run the workspace formatter and return a summary of what changed. \
         Rust: `cargo fmt` (or `rustfmt` for a single file via the `path` arg). \
         Node: `prettier --write`. Python: `ruff format` or `black`. \
         Set check=true to report which files need reformatting without writing any changes. \
         Always run this before git_commit to ensure the committed code is clean. \
         Prefer this over `shell cargo fmt` — you get a structured list of reformatted files.",
        serde_json::json!({
            "type": "object",
            "properties": {
                "check": {
                    "type": "boolean",
                    "description": "If true, only report what would change without writing (default false)."
                },
                "path": {
                    "type": "string",
                    "description": "Optional: relative path to a single file (Rust only). Formats that file instead of the whole workspace."
                }
            },
            "required": []
        }),
    ));
    tools.push(make_tool(
        "lint_code",
        "Run `cargo clippy` and return structured lint results: file path, line, lint code \
         (e.g. clippy::needless_range_loop), message, and machine-applicable fix suggestion. \
         Set fix=true to apply all machine-fixable lints automatically via `cargo clippy --fix`. \
         If the working tree is dirty, also pass allow_dirty=true. \
         Use filter to narrow results to a specific lint name or keyword. \
         Set workspace=true to lint all crates in a workspace. \
         Prefer this over `shell cargo clippy` — you get structured, actionable output.",
        serde_json::json!({
            "type": "object",
            "properties": {
                "fix": {
                    "type": "boolean",
                    "description": "Apply machine-fixable lints automatically (default false)."
                },
                "allow_dirty": {
                    "type": "boolean",
                    "description": "Allow --fix on a dirty working tree (default false)."
                },
                "filter": {
                    "type": "string",
                    "description": "Optional: show only lints whose code or message contains this string."
                },
                "workspace": {
                    "type": "boolean",
                    "description": "Lint all workspace crates (default false = current package)."
                }
            },
            "required": []
        }),
    ));
    tools.push(make_tool(
        "copy_to_clipboard",
        "Copy any text to the user's system clipboard so they can paste it immediately. \
         Works on Windows (PowerShell Set-Clipboard), macOS (pbcopy), and Linux (xclip/xsel). \
         Use this after generating a config snippet, SQL query, command, or any output the user \
         will want to paste somewhere. Returns confirmation with byte count.",
        serde_json::json!({
            "type": "object",
            "properties": {
                "text": {
                    "type": "string",
                    "description": "The text to copy to the clipboard."
                }
            },
            "required": ["text"]
        }),
    ));
    tools.push(make_tool(
        "run_tests",
        "Run the project test suite and return a structured summary: pass/fail counts, \
         elapsed time, and — on failure — the exact failure blocks extracted from output. \
         Works with Rust/Cargo (cargo test), Node/npm (npm test), and Python/pytest. \
         Use the `filter` arg to run a subset of tests by name (Cargo substring match, \
         pytest -k expression). Defaults to a 120-second timeout. \
         Set dry_run=true to preview the command that would run without executing it. \
         Prefer this over raw `shell cargo test` so failures are surfaced cleanly.",
        serde_json::json!({
            "type": "object",
            "properties": {
                "filter": {
                    "type": "string",
                    "description": "Optional test name filter. For Cargo: substring match. For pytest: -k expression."
                },
                "timeout_seconds": {
                    "type": "integer",
                    "description": "Max seconds to wait for the suite (default 120)."
                },
                "dry_run": {
                    "type": "boolean",
                    "description": "If true, report the command that would run without executing (default false)."
                }
            },
            "required": []
        }),
    ));
    tools.push(make_tool(
        "manage_deps",
        "Manage project dependencies without writing raw shell commands. \
         Actions: \
         `list` — parse Cargo.toml and display all dependency sections instantly; \
         `add` — run `cargo add <name> [@version] [--dev] [--features ...]`; \
         `remove` — run `cargo remove <name>`; \
         `tree` — run `cargo tree [--depth N] [-p package]` to visualize the dep graph; \
         `outdated` — run `cargo outdated` (requires cargo-outdated install); \
         `audit` — run `cargo audit` for known security advisories (requires cargo-audit install). \
         Always prefer `manage_deps(action: \"list\")` over reading Cargo.toml raw when you need to \
         inspect what is currently depended on.",
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["add", "remove", "list", "tree", "outdated", "audit"],
                    "description": "What to do with dependencies."
                },
                "name": {
                    "type": "string",
                    "description": "Crate name (required for add/remove)."
                },
                "version": {
                    "type": "string",
                    "description": "Optional version constraint for add, e.g. '1.0', '0.13.1'."
                },
                "dev": {
                    "type": "boolean",
                    "description": "If true, add as a dev-dependency (default false)."
                },
                "features": {
                    "type": "string",
                    "description": "Comma-separated feature flags to enable when adding."
                },
                "package": {
                    "type": "string",
                    "description": "Package filter for `tree` action."
                },
                "depth": {
                    "type": "integer",
                    "description": "Tree depth for `tree` action (default 3)."
                }
            },
            "required": ["action"]
        }),
    ));
    tools.push(make_tool(
        "http_request",
        "Make an HTTP request (GET/POST/PUT/DELETE/PATCH/HEAD) and return the status code, \
         key response headers, and body. Body is auto-pretty-printed when the response is JSON. \
         Supports Bearer token auth, HTTP Basic auth, custom headers, and arbitrary request body. \
         Use this to: test REST APIs, call webhooks, fetch a URL, send JSON payloads, \
         check if an endpoint is reachable, inspect API responses. \
         Prefer this over `shell curl` — you get structured output with the status code and JSON \
         automatically pretty-printed.",
        serde_json::json!({
            "type": "object",
            "properties": {
                "url": {
                    "type": "string",
                    "description": "The request URL (required)."
                },
                "method": {
                    "type": "string",
                    "enum": ["GET", "POST", "PUT", "DELETE", "PATCH", "HEAD"],
                    "description": "HTTP method (default GET)."
                },
                "headers": {
                    "type": "object",
                    "description": "Optional map of request headers, e.g. {\"Accept\": \"application/json\"}."
                },
                "body": {
                    "type": "string",
                    "description": "Optional request body. Detected as JSON if it starts with { or [."
                },
                "content_type": {
                    "type": "string",
                    "description": "Content-Type override (default: auto-detected from body shape)."
                },
                "bearer_token": {
                    "type": "string",
                    "description": "Bearer token for Authorization header."
                },
                "basic_auth": {
                    "type": "string",
                    "description": "Basic auth credentials as 'username:password'."
                },
                "timeout_seconds": {
                    "type": "integer",
                    "description": "Request timeout in seconds (default 30)."
                },
                "follow_redirects": {
                    "type": "boolean",
                    "description": "Follow HTTP redirects (default true)."
                }
            },
            "required": ["url"]
        }),
    ));
    tools.push(make_tool(
        "docker_ops",
        "Manage Docker containers, images, and Compose stacks. \
         Actions: \
         `ps` — list running containers; \
         `ps-all` — list all containers including stopped; \
         `images` — list local images; \
         `stats` — real-time resource usage snapshot (no-stream); \
         `logs` — fetch container logs (use `tail` to limit lines); \
         `start` / `stop` / `restart` — lifecycle control; \
         `rm` — remove a container (pass force=true to remove running containers); \
         `pull` — pull an image; \
         `inspect` — detailed JSON info about a container or image; \
         `build` — build an image from a Dockerfile; \
         `exec` — run a command inside a running container; \
         `compose-ps` / `compose-up` / `compose-down` — Compose stack control. \
         Prefer this over `shell docker` commands — you get structured, readable output.",
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["ps", "ps-all", "images", "stats", "logs", "start", "stop", "restart",
                             "rm", "pull", "inspect", "build", "exec",
                             "compose-ps", "compose-up", "compose-down"],
                    "description": "The Docker operation to perform (required)."
                },
                "container": {
                    "type": "string",
                    "description": "Container name or ID (required for: logs, start, stop, restart, rm, inspect, exec, stats)."
                },
                "image": {
                    "type": "string",
                    "description": "Image name/tag (required for pull; optional for inspect)."
                },
                "tail": {
                    "type": "integer",
                    "description": "Number of log lines to return (default 100, for logs action)."
                },
                "timestamps": {
                    "type": "boolean",
                    "description": "Include timestamps in logs (default false)."
                },
                "command": {
                    "type": "string",
                    "description": "Command to run inside container (required for exec action)."
                },
                "force": {
                    "type": "boolean",
                    "description": "Force-remove a running container (for rm action, default false)."
                },
                "context": {
                    "type": "string",
                    "description": "Build context path (for build action, default '.')."
                },
                "tag": {
                    "type": "string",
                    "description": "Image tag for build action, e.g. 'myapp:latest'."
                },
                "dockerfile": {
                    "type": "string",
                    "description": "Path to Dockerfile for build action (default ./Dockerfile)."
                },
                "no_cache": {
                    "type": "boolean",
                    "description": "Build without cache (default false)."
                },
                "file": {
                    "type": "string",
                    "description": "Compose file path for compose-* actions (default docker-compose.yml)."
                },
                "detach": {
                    "type": "boolean",
                    "description": "Run compose-up in detached mode (default true)."
                },
                "build": {
                    "type": "boolean",
                    "description": "Build images before starting for compose-up (default false)."
                }
            },
            "required": ["action"]
        }),
    ));
    tools.push(make_tool(
        "docker_compose_tools",
        "Parse, inspect, and validate docker-compose.yml files without running Docker or external tools. \
         Actions: services (default — all services with image, ports, restart policy, depends_on, volume/env counts), \
         inspect (full detail for one service including command, entrypoint, healthcheck; pass 'service'), \
         ports (all host:container port mappings across services with well-known port annotations), \
         volumes (named top-level volumes + per-service bind mounts and named volume mounts), \
         networks (defined networks with driver + service→network membership), \
         env (environment variables per service with secrets redacted; optional 'service' filter), \
         validate (check for missing image/build, undefined depends_on targets, privileged mode, host network mode). \
         Pass the compose file content as 'text'. \
         Example: docker_compose_tools(action: 'services', text: '...') or \
         docker_compose_tools(action: 'inspect', text: '...', service: 'api') or \
         docker_compose_tools(action: 'validate', text: '...').",
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "description": "services (default), inspect, ports, volumes, networks, env, validate"
                },
                "text": {
                    "type": "string",
                    "description": "docker-compose.yml content as a string. Also 'yaml'/'compose'/'content'/'input'."
                },
                "service": {
                    "type": "string",
                    "description": "Service name for 'inspect' action, or filter for 'env' action. Partial match supported."
                }
            },
            "required": []
        }),
    ));
    tools.push(make_tool(
        "dockerfile_tools",
        "Parse, inspect, and validate Dockerfiles without external utilities. \
         Pass the Dockerfile content as 'text'. \
         Actions: info (default — base image and tag per stage, exposed ports, labels, WORKDIR, USER, CMD/ENTRYPOINT, instruction counts), \
         layers (all instructions in order with type and full content), \
         validate (check for: latest tag on FROM, running as root, ADD instead of COPY, curl/wget piped to shell, secrets in ENV/ARG, missing CMD/ENTRYPOINT, no HEALTHCHECK). \
         Example: dockerfile_tools(action: 'info', text: '...') or \
         dockerfile_tools(action: 'validate', text: '...') or \
         dockerfile_tools(action: 'layers', text: '...').",
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "description": "info (default), layers, validate"
                },
                "text": {
                    "type": "string",
                    "description": "Dockerfile content as a string. Also 'dockerfile'/'content'/'input'."
                }
            },
            "required": []
        }),
    ));
    tools.push(make_tool(
        "k8s_tools",
        "Parse, inspect, and validate Kubernetes manifests (Deployment, Service, Pod, StatefulSet, DaemonSet, Job, CronJob, Ingress, ConfigMap) without external utilities. \
         Pass the manifest YAML content as 'text'. \
         Actions: info (default — kind, apiVersion, name, namespace, labels, replicas/selector/strategy for workloads, port list for Services, key list for ConfigMaps), \
         containers (per-container breakdown: image, ports, resource requests/limits, env vars, volume mounts, liveness/readiness/startup probes, security context), \
         volumes (volume types with source details: ConfigMap, Secret, PVC, HostPath, EmptyDir, NFS, Projected), \
         validate (checks: missing kind/apiVersion/name, image without pinned tag, missing resource limits, privileged containers, no runAsNonRoot/runAsUser, missing liveness/readiness probes, hostPath volumes, hostNetwork/hostPID, single replica). \
         Example: k8s_tools(action: 'info', text: '...') or \
         k8s_tools(action: 'containers', text: '...') or \
         k8s_tools(action: 'validate', text: '...').",
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "description": "info (default), containers, volumes, validate"
                },
                "text": {
                    "type": "string",
                    "description": "Kubernetes manifest YAML content as a string. Also 'yaml'/'manifest'/'content'/'input'."
                }
            },
            "required": []
        }),
    ));
    tools.push(make_tool(
        "json_tools",
        "Query, transform, and analyze JSON data without needing jq or external tools. \
         Provide JSON inline ('json' arg) or from a file ('file' arg). \
         Actions: \
         `pretty` — pretty-print JSON; \
         `compact` — compact/minify JSON; \
         `keys` — list all keys at the top level or at a path; \
         `get` — extract a value by dot-path (e.g. path='user.address.city' or 'items[0].name'); \
         `filter` — filter array by field equality/comparison (key, value, op: eq/ne/gt/lt/gte/lte/contains/starts_with); \
         `pluck` — extract specific fields from each object in an array (fields: 'name,email,id'); \
         `flatten` — flatten one level of nesting in an array, or flatten a nested array key; \
         `count` — count array elements or object keys; \
         `sort` — sort array (key arg for field sort, reverse: true for descending); \
         `unique` — deduplicate array elements (key arg for field dedup); \
         `merge` — merge two JSON objects (json + with args); \
         `diff` — diff two JSON objects and show added/removed/changed paths (json + with args); \
         `validate` — confirm JSON is valid and report type/size; \
         `schema` — infer the shape/schema of the JSON structure; \
         `stats` — numeric statistics for a number array (min/max/mean/median/stddev); \
         `to-csv` — convert an array of objects to CSV format.",
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "description": "Operation to perform. Default: 'pretty'. Options: pretty, compact, keys, get, filter, pluck, flatten, count, sort, unique, merge, diff, validate, schema, stats, to-csv."
                },
                "json": {
                    "type": "string",
                    "description": "Inline JSON string to operate on."
                },
                "file": {
                    "type": "string",
                    "description": "Path to a JSON file (relative to workspace root or absolute)."
                },
                "path": {
                    "type": "string",
                    "description": "Dot-notation path for 'get' and 'keys' actions (e.g. 'user.name', 'items[0].id')."
                },
                "key": {
                    "type": "string",
                    "description": "Field name for 'filter', 'sort', 'unique', and 'flatten' actions."
                },
                "value": {
                    "description": "Value to compare against in 'filter' action."
                },
                "op": {
                    "type": "string",
                    "description": "Comparison operator for 'filter': eq, ne, gt, lt, gte, lte, contains, starts_with. Default: eq."
                },
                "fields": {
                    "type": "string",
                    "description": "Comma-separated field names for 'pluck' action."
                },
                "reverse": {
                    "type": "boolean",
                    "description": "Sort in descending order for 'sort' action (default false)."
                },
                "with": {
                    "type": "string",
                    "description": "Second JSON object (inline string) for 'merge' and 'diff' actions."
                }
            }
        }),
    ));
    tools.push(make_tool(
        "regex_tools",
        "Test, extract, replace, split, and explain regular expressions without needing external tools. \
         Actions: \
         `test` — check if a pattern matches one or more strings (accepts 'text' or 'texts' array); \
         `extract` — find all matches or named/numbered capture groups in text; \
         `replace` — substitute matches with a replacement string (optional 'limit' arg); \
         `split` — split text on a regex delimiter; \
         `explain` — produce a plain-English breakdown of each component in the pattern; \
         `named-groups` — extract all named capture groups from text by name. \
         Flags: case_insensitive (bool), multiline (bool, ^ and $ match line boundaries), dot_all (bool, . matches newline).",
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "description": "Operation to perform. Default: 'test'. Options: test, extract, replace, split, explain, named-groups."
                },
                "pattern": {
                    "type": "string",
                    "description": "The regular expression pattern (Rust regex syntax)."
                },
                "text": {
                    "type": "string",
                    "description": "The input text to operate on (single string)."
                },
                "texts": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Array of test strings for the 'test' action."
                },
                "replacement": {
                    "type": "string",
                    "description": "Replacement string for 'replace' action. Supports $1, $name capture references."
                },
                "limit": {
                    "type": "integer",
                    "description": "Max number of replacements for 'replace' action (0 = replace all, default 0)."
                },
                "case_insensitive": {
                    "type": "boolean",
                    "description": "Enable case-insensitive matching (flag: i)."
                },
                "multiline": {
                    "type": "boolean",
                    "description": "Enable multiline mode: ^ and $ match line boundaries (flag: m)."
                },
                "dot_all": {
                    "type": "boolean",
                    "description": "Enable dot-all mode: . matches newline characters (flag: s)."
                }
            },
            "required": ["pattern"]
        }),
    ));
    tools.push(make_tool(
        "diff_tools",
        "Compare, diff, and patch text or file content without needing external tools. \
         Actions: \
         `compare` — unified diff between two strings or files, with configurable context lines; \
         `patch` — generate a unified patch from two inputs (optionally write to a file); \
         `apply` — apply a unified patch to a base text or file; \
         `word-diff` — word-level diff showing [+added] and [-removed] tokens inline; \
         `stat` — summary statistics: lines added/deleted/unchanged, similarity %, visual bar. \
         Provide text inline via 'text_a'/'text_b', or paths via 'file_a'/'file_b'.",
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "description": "Operation: compare (default), patch, apply, word-diff, stat."
                },
                "text_a": {
                    "type": "string",
                    "description": "Left/base text content (inline string)."
                },
                "text_b": {
                    "type": "string",
                    "description": "Right/new text content (inline string)."
                },
                "file_a": {
                    "type": "string",
                    "description": "Path to the left/base file (relative to workspace root or absolute)."
                },
                "file_b": {
                    "type": "string",
                    "description": "Path to the right/new file (relative to workspace root or absolute)."
                },
                "context": {
                    "type": "integer",
                    "description": "Lines of context around each change (default 3)."
                },
                "patch": {
                    "type": "string",
                    "description": "Unified patch text to apply (for 'apply' action)."
                },
                "patch_file": {
                    "type": "string",
                    "description": "Path to a .patch file to apply (for 'apply' action)."
                },
                "output": {
                    "type": "string",
                    "description": "Write patch output to this file path (for 'patch' and 'apply' actions)."
                }
            }
        }),
    ));
    tools.push(make_tool(
        "yaml_tools",
        "Validate, format, query, and transform YAML documents without needing external tools. \
         Provide YAML inline ('yaml' arg) or from a file ('file' arg). \
         Actions: \
         `validate` — parse YAML and report root type, depth, and top-level keys; \
         `format` — re-serialize YAML with canonical formatting; \
         `get` — extract a value at a dot-path (e.g. 'metadata.name', 'spec.containers[0].image'); \
         `keys` — list keys/elements at the root or at a dot-path; \
         `to-json` — convert YAML to pretty-printed JSON; \
         `from-json` — convert JSON ('json' arg) to YAML; \
         `merge` — deep-merge a second YAML document ('with' arg) into the base; \
         `diff` — compare two YAML documents and list additions, removals, and changes.",
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "description": "Operation: validate (default), format, get, keys, to-json, from-json, merge, diff."
                },
                "yaml": {
                    "type": "string",
                    "description": "Inline YAML content to process."
                },
                "file": {
                    "type": "string",
                    "description": "Path to a YAML file (relative to workspace root or absolute)."
                },
                "path": {
                    "type": "string",
                    "description": "Dot-path to navigate into (e.g. 'metadata.name', 'spec.containers[0].image'). Used by get and keys."
                },
                "json": {
                    "type": "string",
                    "description": "Inline JSON string to convert (for 'from-json' action)."
                },
                "with": {
                    "type": "string",
                    "description": "Second YAML document to merge or diff against the base (for 'merge' and 'diff' actions)."
                }
            }
        }),
    ));
    tools.push(make_tool(
        "csv_tools",
        "Read, inspect, filter, sort, and convert CSV files without needing external tools. \
         Provide CSV inline ('csv' arg) or from a file ('file' arg). \
         Actions: \
         `read` — display the CSV as a formatted table (paginated, first 50 rows); \
         `head` — show the first N rows (default 10); \
         `columns` — list all column names; \
         `stats` — compute per-column statistics (min/max/mean/median/stddev for numeric, unique count and top values for text); \
         `filter` — filter rows by column value (ops: eq, ne, gt, lt, gte, lte, contains, starts-with, ends-with); \
         `sort` — sort rows by a column (ascending or descending); \
         `to-json` — convert CSV to a JSON array of objects (auto-coerces numbers and booleans); \
         `to-markdown` — convert CSV to a Markdown table; \
         `count` — return total row count (excluding header).",
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "description": "Operation: read (default), head, columns, stats, filter, sort, to-json, to-markdown, count."
                },
                "csv": {
                    "type": "string",
                    "description": "Inline CSV content to process."
                },
                "file": {
                    "type": "string",
                    "description": "Path to a CSV file (relative to workspace root or absolute)."
                },
                "delimiter": {
                    "type": "string",
                    "description": "Field delimiter character (default: ','). Use '\\t' for TSV."
                },
                "n": {
                    "type": "integer",
                    "description": "Number of rows to show (for 'head' action, default 10)."
                },
                "column": {
                    "type": "string",
                    "description": "Column name to filter or sort by."
                },
                "op": {
                    "type": "string",
                    "description": "Filter operator: eq, ne, gt, lt, gte, lte, contains (default), starts-with, ends-with."
                },
                "value": {
                    "type": "string",
                    "description": "Value to compare against (for 'filter' action)."
                },
                "order": {
                    "type": "string",
                    "description": "Sort order: asc (default) or desc (for 'sort' action)."
                }
            }
        }),
    ));
    tools.push(make_tool(
        "encode_tools",
        "Encode and decode data between formats without needing external tools. \
         Actions: \
         `base64-encode` — encode text to Base64 (standard or URL-safe); \
         `base64-decode` — decode Base64 text back to the original string; \
         `url-encode` — percent-encode a string for use in URLs; \
         `url-decode` — decode a percent-encoded string; \
         `hex-encode` — encode text as hexadecimal; \
         `hex-decode` — decode a hex string back to text; \
         `jwt-decode` — decode a JWT token's header and payload (no signature verification); shows exp/iat in human-readable form; \
         `html-encode` — escape HTML entities (&, <, >, \", '); \
         `html-decode` — unescape HTML entities back to plain text. \
         All actions take an 'input' field. base64 actions also accept 'url_safe: true'.",
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "description": "Operation: base64-encode (default), base64-decode, url-encode, url-decode, hex-encode, hex-decode, jwt-decode, html-encode, html-decode."
                },
                "input": {
                    "type": "string",
                    "description": "The string to encode or decode."
                },
                "url_safe": {
                    "type": "boolean",
                    "description": "Use URL-safe Base64 alphabet (no +/ padding). Applies to base64-encode and base64-decode. Default: false."
                }
            },
            "required": ["input"]
        }),
    ));
    tools.push(make_tool(
        "hash_tools",
        "Compute cryptographic hashes of strings or files without needing external tools. \
         Actions: \
         `sha256` (default) — SHA-256 hex digest; \
         `sha512` — SHA-512 hex digest; \
         `md5` — MD5 hex digest (fast; not cryptographically secure for new designs); \
         `hmac-sha256` — HMAC-SHA256 with a secret key ('key' field required); \
         `all` — run MD5 + SHA-256 + SHA-512 on the same input at once. \
         Provide data via 'input' (inline string) or 'file' (path to any file).",
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "description": "Hash algorithm: sha256 (default), sha512, md5, hmac-sha256, all."
                },
                "input": {
                    "type": "string",
                    "description": "Inline string to hash."
                },
                "file": {
                    "type": "string",
                    "description": "Path to a file to hash (relative to workspace root or absolute)."
                },
                "key": {
                    "type": "string",
                    "description": "Secret key for HMAC-SHA256. Required when action is 'hmac-sha256'."
                }
            }
        }),
    ));
    tools.push(make_tool(
        "toml_tools",
        "Validate, format, query, and transform TOML documents without needing external tools. \
         Provide TOML inline ('toml' arg) or from a file ('file' arg). \
         Actions: \
         `validate` — parse TOML and report root type and top-level keys; \
         `format` — re-serialize TOML with canonical pretty-printing; \
         `get` — extract a value at a dot-path (e.g. 'package.name', 'dependencies.serde', 'bin[0].name'); \
         `keys` — list keys at the root or at a dot-path; \
         `to-json` — convert TOML to pretty-printed JSON; \
         `from-json` — convert JSON ('json' arg) to TOML. \
         Works with Cargo.toml, pyproject.toml, config.toml, and any TOML config file.",
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "description": "Operation: validate (default), format, get, keys, to-json, from-json."
                },
                "toml": {
                    "type": "string",
                    "description": "Inline TOML content to process."
                },
                "file": {
                    "type": "string",
                    "description": "Path to a TOML file (relative to workspace root or absolute)."
                },
                "path": {
                    "type": "string",
                    "description": "Dot-path to navigate into (e.g. 'package.name', 'dependencies.tokio', 'bin[0].name'). Used by get and keys."
                },
                "json": {
                    "type": "string",
                    "description": "Inline JSON string to convert (for 'from-json' action)."
                }
            }
        }),
    ));
    tools.push(make_tool(
        "text_tools",
        "Transform, analyze, and manipulate text without needing external tools. \
         All actions take an 'input' field. \
         Case conversion actions (convert between naming conventions): \
         `to-snake` (My Var → my_var), `to-camel` (my_var → myVar), `to-pascal` (my_var → MyVar), \
         `to-kebab` (myVar → my-var), `to-screaming` (my_var → MY_VAR), \
         `to-title` (my var → My Var), `to-lower`, `to-upper`. \
         Other actions: \
         `slugify` — URL-safe slug (lowercase, hyphens, no special chars); \
         `count` — word, line, character, byte, and sentence counts; \
         `truncate` — shorten to 'max' chars with optional 'ellipsis' (default '...'); \
         `pad` — pad to 'width' with 'align' (left/right/center) and optional 'fill' char; \
         `wrap` — word-wrap at 'width' chars (default 80); \
         `repeat` — repeat 'n' times with optional 'sep' separator; \
         `reverse` — reverse the string character by character; \
         `lines` — process lines with optional 'sort', 'dedupe', 'filter_empty' booleans.",
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "description": "Operation: to-snake, to-camel, to-pascal, to-kebab, to-screaming, to-title, to-lower, to-upper, slugify, count (default), truncate, pad, wrap, repeat, reverse, lines."
                },
                "input": {
                    "type": "string",
                    "description": "The text to process."
                },
                "max": {
                    "type": "integer",
                    "description": "Maximum character count (for 'truncate'). Default: 80."
                },
                "ellipsis": {
                    "type": "string",
                    "description": "String appended when truncated (for 'truncate'). Default: '...'."
                },
                "width": {
                    "type": "integer",
                    "description": "Target width in characters (for 'pad' and 'wrap'). Default: 20 for pad, 80 for wrap."
                },
                "align": {
                    "type": "string",
                    "description": "Alignment for 'pad': left, right (default), or center."
                },
                "fill": {
                    "type": "string",
                    "description": "Fill character for 'pad'. Default: space."
                },
                "n": {
                    "type": "integer",
                    "description": "Repeat count for 'repeat'. Default: 2."
                },
                "sep": {
                    "type": "string",
                    "description": "Separator between repetitions for 'repeat'. Default: empty string."
                },
                "sort": {
                    "type": "boolean",
                    "description": "Sort lines alphabetically (for 'lines'). Default: false."
                },
                "dedupe": {
                    "type": "boolean",
                    "description": "Remove duplicate lines (for 'lines'). Default: false."
                },
                "filter_empty": {
                    "type": "boolean",
                    "description": "Remove blank lines (for 'lines'). Default: false."
                }
            },
            "required": ["input"]
        }),
    ));
    tools.push(make_tool(
        "date_tools",
        "Work with dates and times: parse, format, add/subtract, diff, convert timestamps, \
         and describe relative time. All actions are zero-dependency (no internet required). \
         Actions: \
         `now` — current UTC and local time, Unix timestamp, ISO 8601, week number; \
         `parse` — parse a date string in many formats (ISO 8601, RFC 2822, natural like 'June 15, 2024') \
            and show normalized UTC, epoch, day-of-week, day-of-year, week number; \
         `format` — reformat a date using a strftime format string (e.g. '%d/%m/%Y'); \
         `add` — add days/weeks/months/years/hours/minutes to a date; \
         `diff` — calculate the duration between two dates (weeks, days, hours, approx months/years); \
         `timestamp` — convert a date string to Unix epoch seconds (and milliseconds); \
         `from-timestamp` — convert a Unix epoch (seconds or auto-detected milliseconds) to a human date; \
         `relative` — describe a date relative to now ('3 days ago', 'in 2 hours'); \
         `weekday` — get the weekday name and ISO week number for any date.",
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "description": "Operation: now (default), parse, format, add, diff, timestamp, from-timestamp, relative, weekday."
                },
                "input": {
                    "type": "string",
                    "description": "Date/time string to parse. Accepts ISO 8601, RFC 2822, 'June 15 2024', 'dd/mm/yyyy', etc. Required for parse, format, add, diff (as 'from'), timestamp, relative, weekday."
                },
                "format": {
                    "type": "string",
                    "description": "strftime format string for 'format' action (e.g. '%Y/%m/%d', '%B %d, %Y'). Also accepted by 'now' to format current time."
                },
                "from": {
                    "type": "string",
                    "description": "Start date string for 'diff' action."
                },
                "to": {
                    "type": "string",
                    "description": "End date string for 'diff' action."
                },
                "days": {
                    "type": "integer",
                    "description": "Number of days to add (for 'add'). Negative to subtract."
                },
                "weeks": {
                    "type": "integer",
                    "description": "Number of weeks to add (for 'add')."
                },
                "months": {
                    "type": "integer",
                    "description": "Number of months to add (for 'add')."
                },
                "years": {
                    "type": "integer",
                    "description": "Number of years to add (for 'add')."
                },
                "hours": {
                    "type": "integer",
                    "description": "Number of hours to add (for 'add')."
                },
                "minutes": {
                    "type": "integer",
                    "description": "Number of minutes to add (for 'add')."
                }
            },
            "required": []
        }),
    ));
    tools.push(make_tool(
        "number_tools",
        "Number base conversion, formatting, and math utilities. No dependencies required. \
         Actions: \
         `convert` — convert an integer between bases. If no 'to' is given, shows decimal/hex/binary/octal all at once. \
            Accepts 0x, 0b, 0o prefixes for the input. Supports base 2–36 via 'from'/'to' parameters; \
         `format` — format a number with thousands separators, scientific notation, engineering notation, \
            and SI prefix (k/M/G/T/P/E); \
         `roman` — convert a decimal integer (1–3999) to a Roman numeral; \
         `from-roman` — convert a Roman numeral string to a decimal integer; \
         `si` — show the value with the appropriate SI prefix (e.g. 1500 → 1.5k, 2000000 → 2M); \
         `factors` — prime factorization of a positive integer, with primality flag; \
         `gcd` — Euclidean GCD and LCM of two integers ('a' and 'b' fields); \
         `clamp` — clamp a number to [min, max] range ('value', 'min', 'max' fields).",
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "description": "Operation: convert (default), format, roman, from-roman, si, factors, gcd, clamp."
                },
                "input": {
                    "type": ["string", "number"],
                    "description": "The number to process. Accepts integer strings with 0x/0b/0o prefixes for 'convert'."
                },
                "from": {
                    "type": "integer",
                    "description": "Source base for 'convert' (2–36). Default: auto-detected from prefix."
                },
                "to": {
                    "type": "integer",
                    "description": "Target base for 'convert' (2–36). If omitted, all common bases are shown."
                },
                "a": {
                    "type": "integer",
                    "description": "First integer for 'gcd'."
                },
                "b": {
                    "type": "integer",
                    "description": "Second integer for 'gcd'."
                },
                "value": {
                    "type": "number",
                    "description": "Number to clamp (for 'clamp')."
                },
                "min": {
                    "type": "number",
                    "description": "Minimum bound for 'clamp'."
                },
                "max": {
                    "type": "number",
                    "description": "Maximum bound for 'clamp'."
                }
            },
            "required": []
        }),
    ));
    tools.push(make_tool(
        "uuid_gen",
        "Generate, validate, and work with UUIDs (Universally Unique Identifiers). \
         All generation uses cryptographically random bytes — no internet required. \
         Actions: \
         `generate` (default) — generate a single UUID v4 with version/variant metadata; \
         `validate` — validate a UUID string and decode its version and variant; \
         `nil` — return the nil UUID (all zeros: 00000000-0000-0000-0000-000000000000); \
         `bulk` — generate N UUIDs at once (up to 100, default 5). \
         All actions accept 'upper' boolean to output uppercase hex.",
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "description": "Operation: generate (default), validate, nil, bulk."
                },
                "input": {
                    "type": "string",
                    "description": "UUID string to validate (for 'validate' action)."
                },
                "n": {
                    "type": "integer",
                    "description": "Number of UUIDs to generate (for 'bulk'). Default: 5, max: 100."
                },
                "upper": {
                    "type": "boolean",
                    "description": "Output UUIDs in uppercase hex. Default: false."
                }
            },
            "required": []
        }),
    ));
    tools.push(make_tool(
        "cron_tools",
        "Parse, explain, validate, and calculate run times for cron expressions. \
         All computation is local — no internet required. \
         Actions: \
         `explain` (default) — field-by-field breakdown: minute, hour, day-of-month, month, weekday \
            plus a one-line plain-English summary; \
         `validate` — check whether an expression is syntactically valid; \
         `next` — list the next N execution times from now (default 5, max 20); \
         `describe` — one-line natural-language summary only. \
         Accepts expressions via 'expression' or 'input' field. \
         Supports: `*`, `/N` step, `,` list, `-` range, named months (January/Jan) and weekdays (Monday/Mon), \
         and `0`–`7` for Sunday in the weekday field. \
         Example: cron_tools(action: \"explain\", expression: \"0 */6 * * *\") — every 6 hours; \
         cron_tools(action: \"next\", expression: \"30 9 * * 1-5\", n: 10) — next 10 weekday 9:30am runs.",
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "description": "Operation: explain (default), validate, next, describe."
                },
                "expression": {
                    "type": "string",
                    "description": "The cron expression (5 fields: minute hour day month weekday). E.g. '0 */6 * * *'."
                },
                "input": {
                    "type": "string",
                    "description": "Alias for 'expression'."
                },
                "n": {
                    "type": "integer",
                    "description": "Number of next run times to calculate (for 'next'). Default: 5, max: 20."
                }
            },
            "required": []
        }),
    ));
    tools.push(make_tool(
        "ip_tools",
        "IP address parsing, CIDR subnet calculations, and format conversion. Pure Rust, no internet required. \
         Actions: \
         `info` — parse an IPv4 or IPv6 address: class (A/B/C/D/E), type (public/private/loopback/multicast), \
            binary representation, decimal integer, hex, and IPv4-mapped IPv6 form; \
         `cidr` — CIDR breakdown (e.g. '192.168.1.0/24'): network address, subnet mask, broadcast, \
            first/last usable host, usable host count, wildcard mask, binary representations; \
         `contains` — check if an IP address falls within a CIDR network (pass 'ip' and 'cidr' fields); \
         `convert` — convert an IPv4 address between dotted-decimal, decimal integer, hex, and binary; \
         `subnet` — given an IP and a dotted-decimal subnet mask, show network/broadcast/prefix/usable hosts.",
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "description": "Operation: info (default), cidr, contains, convert, subnet."
                },
                "input": {
                    "type": "string",
                    "description": "IP address or CIDR notation (e.g. '192.168.1.1', '10.0.0.0/8', '255', '0xFF000000')."
                },
                "ip": {
                    "type": "string",
                    "description": "IP address to check (for 'contains'). E.g. '192.168.1.50'."
                },
                "cidr": {
                    "type": "string",
                    "description": "CIDR network to test against (for 'contains'). E.g. '192.168.1.0/24'."
                },
                "network": {
                    "type": "string",
                    "description": "Alias for 'cidr'."
                },
                "mask": {
                    "type": "string",
                    "description": "Dotted-decimal subnet mask (for 'subnet'). E.g. '255.255.255.0'."
                }
            },
            "required": []
        }),
    ));
    tools.push(make_tool(
        "color_tools",
        "Color format conversion, analysis, and palette generation. All computation is local. \
         Accepts any of: #RRGGBB, #RGB, rgb(R,G,B), hsl(H,S%,L%), or CSS named colors (red, blue, coral, etc.). \
         Actions: \
         `info` (default) — full breakdown: hex, RGB, HSL, relative luminance, dark/light perception, \
            WCAG contrast ratio against white and black; \
         `convert` — convert a color to all formats (hex, RGB, HSL); \
         `contrast` — WCAG contrast ratio between two colors ('color1' and 'color2') with AA/AAA grade; \
         `mix` — blend two colors at a given ratio (0.0–1.0, default 0.5); \
         `lighten` — increase lightness by 'amount' percent (default 10); \
         `darken` — decrease lightness by 'amount' percent (default 10); \
         `palette` — generate complementary, triadic, analogous, lighter, and darker variants.",
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "description": "Operation: info (default), convert, contrast, mix, lighten, darken, palette."
                },
                "input": {
                    "type": "string",
                    "description": "Color string (#RRGGBB, rgb(R,G,B), hsl(H,S%,L%), or CSS name). Used by: info, convert, lighten, darken, palette."
                },
                "color1": {
                    "type": "string",
                    "description": "First color for 'contrast' or 'mix'."
                },
                "color2": {
                    "type": "string",
                    "description": "Second color for 'contrast' or 'mix'."
                },
                "a": {
                    "type": "string",
                    "description": "Alias for 'color1'."
                },
                "b": {
                    "type": "string",
                    "description": "Alias for 'color2'."
                },
                "ratio": {
                    "type": "number",
                    "description": "Blend ratio for 'mix' (0.0 = all color1, 1.0 = all color2). Default: 0.5."
                },
                "amount": {
                    "type": "number",
                    "description": "Percentage to lighten or darken (for 'lighten'/'darken'). Default: 10."
                }
            },
            "required": []
        }),
    ));
    tools.push(make_tool(
        "semver_tools",
        "Parse, compare, bump, validate, and range-check semantic versions (SemVer 2.0). \
         Accepts versions with or without a leading 'v' prefix. \
         Actions: \
         `parse` (default) — break a version into major/minor/patch, pre-release, build metadata, and stability flag; \
         `compare` — compare two versions ('a' and 'b' fields) and report which is newer or if equal; \
         `bump` — increment a version ('input' + 'part': major/minor/patch/premajor/preminor/prepatch); \
         `validate` — check if a string is valid semver; \
         `satisfies` — check if 'version' matches a 'range' (supports ^, ~, >=, <=, >, <, =, * and || OR ranges); \
         `sort` — sort an array of versions ('versions' field) in 'asc' or 'desc' order.",
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "description": "Operation: parse (default), compare, bump, validate, satisfies, sort."
                },
                "input": {
                    "type": "string",
                    "description": "Version string to parse, bump, or validate. E.g. '1.2.3', 'v2.0.0-beta.1'."
                },
                "version": {
                    "type": "string",
                    "description": "Version to check against a range (for 'satisfies') or alias for 'input'."
                },
                "a": {
                    "type": "string",
                    "description": "First version for 'compare'. Alias: 'version1'."
                },
                "b": {
                    "type": "string",
                    "description": "Second version for 'compare'. Alias: 'version2'."
                },
                "part": {
                    "type": "string",
                    "description": "Part to bump (for 'bump'): major, minor, patch, premajor, preminor, prepatch. Default: patch."
                },
                "range": {
                    "type": "string",
                    "description": "Version range for 'satisfies'. E.g. '^1.2.3', '>=2.0.0 <3.0.0', '~1.4', '1.x', '*'."
                },
                "versions": {
                    "type": "array",
                    "description": "Array of version strings to sort (for 'sort').",
                    "items": { "type": "string" }
                },
                "order": {
                    "type": "string",
                    "description": "Sort order for 'sort': 'asc' (oldest first, default) or 'desc' (newest first)."
                }
            },
            "required": []
        }),
    ));
    tools.push(make_tool(
        "password_gen",
        "Generate secure passwords, passphrases, and PINs, and analyze password strength. \
         All generation uses cryptographically random bytes locally — no internet required. \
         Actions: \
         `generate` (default) — random password with configurable options: \
            'length' (default 16, max 128), 'upper'/'lower'/'digits'/'symbols' booleans (all true by default), \
            'no_ambiguous' to exclude 0/O/1/l/I, 'count' for multiple passwords at once (max 20); \
         `passphrase` — memorable word-based passphrase: \
            'words' (default 4, max 12), 'separator' (default '-'), 'capitalize', 'number' (appends 2-digit random), \
            'count' for multiple; \
         `strength` — analyze a password's strength: score 0-4, entropy bits, character class checklist, \
            and improvement suggestions; \
         `pin` — numeric PIN: 'length' (default 6, max 12), 'count' for multiple.",
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "description": "Operation: generate (default), passphrase, strength, pin."
                },
                "length": {
                    "type": "integer",
                    "description": "Password or PIN length. Default: 16 for generate, 6 for pin. Max: 128 for generate, 12 for pin."
                },
                "upper": {
                    "type": "boolean",
                    "description": "Include uppercase letters (for 'generate'). Default: true."
                },
                "lower": {
                    "type": "boolean",
                    "description": "Include lowercase letters (for 'generate'). Default: true."
                },
                "digits": {
                    "type": "boolean",
                    "description": "Include digits (for 'generate'). Default: true."
                },
                "symbols": {
                    "type": "boolean",
                    "description": "Include symbol characters (for 'generate'). Default: true."
                },
                "no_ambiguous": {
                    "type": "boolean",
                    "description": "Exclude ambiguous characters (0, O, 1, l, I) for readability. Default: false."
                },
                "count": {
                    "type": "integer",
                    "description": "Number of passwords/passphrases/PINs to generate at once. Max: 20."
                },
                "words": {
                    "type": "integer",
                    "description": "Number of words in passphrase (for 'passphrase'). Default: 4, max: 12."
                },
                "separator": {
                    "type": "string",
                    "description": "Word separator for passphrase (for 'passphrase'). Default: '-'."
                },
                "capitalize": {
                    "type": "boolean",
                    "description": "Capitalize first letter of each word (for 'passphrase'). Default: false."
                },
                "number": {
                    "type": "boolean",
                    "description": "Append a random 2-digit number to passphrase (for 'passphrase'). Default: true."
                },
                "input": {
                    "type": "string",
                    "description": "Password to analyze (for 'strength')."
                }
            },
            "required": []
        }),
    ));
    tools.push(make_tool(
        "jwt_tools",
        "Decode, verify, sign, and inspect JSON Web Tokens (JWTs) without external utilities. \
         Supports HS256, HS384, and HS512 HMAC algorithms. \
         Actions: \
         `decode` (default) — decode header and payload, display claims with human-readable exp/iat timestamps; \
            pass 'token'; signature is NOT verified in this action; \
         `verify` — verify HMAC signature and check expiry/nbf; pass 'token' and 'secret'; \
            reports VALID or INVALID with a clear verdict and per-claim expiry state; \
         `sign` — create a new signed JWT; pass 'claims' (JSON object), 'secret', \
            optional 'algorithm' (HS256/HS384/HS512, default HS256); \
         `inspect` — show expiry status, subject, issuer, audience, and validity window without verifying signature.",
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "description": "Operation: decode (default), verify, sign, inspect."
                },
                "token": {
                    "type": "string",
                    "description": "The JWT string (for decode, verify, inspect). Also accepted as 'input'."
                },
                "secret": {
                    "type": "string",
                    "description": "HMAC secret key (for verify and sign). Also accepted as 'key'."
                },
                "claims": {
                    "type": "object",
                    "description": "Claims payload object for 'sign'. Also accepted as 'payload'. Example: {\"sub\": \"user123\", \"exp\": 9999999999}."
                },
                "algorithm": {
                    "type": "string",
                    "description": "HMAC algorithm for 'sign': HS256 (default), HS384, HS512. Also accepted as 'alg'."
                }
            },
            "required": []
        }),
    ));
    tools.push(make_tool(
        "xml_tools",
        "Parse, format, query, and convert XML documents without external utilities. \
         Actions: \
         `validate` (default) — parse the document and summarize root element, element count, depth, and children; \
         `format` — pretty-print the document with 2-space indentation; \
         `get` — navigate to an element by dot-path like 'project.build' or 'deps.dependency[2]' (pass 'path'); \
         `keys` — list immediate child elements and attributes of the root or a path target (pass optional 'path'); \
         `to-json` — convert the full document to JSON (@ prefix for attributes, #text for text content, \
            arrays for repeated elements); \
         `query` — find all elements matching a tag name anywhere in the document (pass 'tag'). \
         Pass 'xml' for inline XML or 'file' for a file path. Works with Maven POMs, Android manifests, \
         Spring configs, SOAP responses, RSS/Atom feeds, SVG, XHTML, and any XML document.",
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "description": "Operation: validate (default), format, get, keys, to-json, query."
                },
                "xml": {
                    "type": "string",
                    "description": "Inline XML string. Provide either 'xml' or 'file'."
                },
                "file": {
                    "type": "string",
                    "description": "Path to an XML file (relative to workspace root or absolute). Provide either 'xml' or 'file'."
                },
                "path": {
                    "type": "string",
                    "description": "Dot-path for 'get' and 'keys' actions. Example: 'project.dependencies' or 'root.items.item[0]'."
                },
                "tag": {
                    "type": "string",
                    "description": "Element tag name to search for in 'query' action. Case-sensitive."
                }
            },
            "required": []
        }),
    ));
    tools.push(make_tool(
        "archive_tools",
        "Inspect and read zip archives without external utilities. \
         Works with .zip, .jar, .whl, .vsix, .apk, and any zip-format archive. \
         Actions: \
         `list` (default) — tabular listing of all entries: name, compressed size, uncompressed size, \
            compression method, file vs directory; supports 'max' (default 100) and 'filter' (name substring); \
         `info` — overall archive statistics: file count, directory count, total size, \
            compression ratio, and archive comment; \
         `inspect` — detailed metadata for a specific entry: size, compression method, \
            CRC-32, last-modified timestamp (pass 'entry' with the entry name); \
         `extract` — read a specific text entry as a UTF-8 string (pass 'entry'; limited to 1 MB). \
         Pass 'file' with the path to the archive.",
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "description": "Operation: list (default), info, inspect, extract."
                },
                "file": {
                    "type": "string",
                    "description": "Path to the zip archive (relative to workspace root or absolute). Also accepted as 'path' or 'input'."
                },
                "entry": {
                    "type": "string",
                    "description": "Name of the entry inside the archive for 'inspect' and 'extract'. Use 'list' first to see entry names."
                },
                "max": {
                    "type": "integer",
                    "description": "Maximum entries to show in 'list' (default 100)."
                },
                "filter": {
                    "type": "string",
                    "description": "Substring filter for entry names in 'list' (case-insensitive)."
                }
            },
            "required": []
        }),
    ));
    tools.push(make_tool(
        "sqlite_tools",
        "Inspect and query SQLite databases in read-only mode — no sqlite3 CLI required. \
         Actions: \
         `tables` (default) — list all tables with row counts, plus views and index count; \
         `schema` — show CREATE SQL and PRAGMA table_info column details; pass 'table' to scope to one table; \
         `query` — execute a SELECT/EXPLAIN/WITH/PRAGMA statement; pass 'sql'; max 100 rows (use 'limit' to override); \
            INSERT/UPDATE/DELETE/DROP/CREATE are blocked — read-only only; \
         `info` — database metadata: file size, SQLite version, page size, encoding, journal mode, user_version; \
         `export` — dump a full table as CSV (default) or JSON; pass 'table', optionally 'format' and 'limit' (default 1000). \
         Pass 'file' with the path to the .sqlite or .db file.",
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "description": "Operation: tables (default), schema, query, info, export."
                },
                "file": {
                    "type": "string",
                    "description": "Path to the SQLite database file (.sqlite, .db). Also accepted as 'db' or 'database'."
                },
                "table": {
                    "type": "string",
                    "description": "Table name for 'schema' (scope to one table) and 'export' (required)."
                },
                "sql": {
                    "type": "string",
                    "description": "SELECT/EXPLAIN/WITH/PRAGMA statement for the 'query' action. Also accepted as 'query'."
                },
                "format": {
                    "type": "string",
                    "description": "Output format for 'export': csv (default) or json."
                },
                "limit": {
                    "type": "integer",
                    "description": "Max rows for 'query' (default 100) or 'export' (default 1000)."
                }
            },
            "required": []
        }),
    ));
    tools.push(make_tool(
        "markdown_tools",
        "Parse and analyze Markdown documents without external tools. \
         Actions: \
         `toc` (default) — generate a table of contents with anchor links; 'depth' limits heading levels (default 3); \
         `stats` — word count, reading time estimate, heading count by level, code block count and lines, \
            link count, image count, list item count, table count, blockquote count; \
         `extract` — extract specific elements; pass 'what' = headings (default) | code | links | images; \
            for code, 'lang' filters by language (e.g. lang: \"rust\"); \
            for headings, 'depth' limits levels; \
         `links` — list all hyperlinks with display text and URL, plus images with URL; \
         `to-html` — render Markdown to HTML; 'wrap: true' for a full HTML document with optional 'title' field; \
         `strip` — remove all Markdown formatting and return plain text. \
         Pass 'text' for inline Markdown or 'file' for a .md file path.",
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "description": "Operation: toc (default), stats, extract, links, to-html, strip."
                },
                "text": {
                    "type": "string",
                    "description": "Inline Markdown content to process."
                },
                "file": {
                    "type": "string",
                    "description": "Path to a .md file (relative to workspace root or absolute). Also accepted as 'input'."
                },
                "what": {
                    "type": "string",
                    "description": "Element type for 'extract': headings (default), code, links, images."
                },
                "depth": {
                    "type": "integer",
                    "description": "Maximum heading level for 'toc' and 'extract' headings (1–6, default 3 for toc / 6 for extract)."
                },
                "lang": {
                    "type": "string",
                    "description": "Language filter for 'extract' code action (e.g. 'rust', 'python')."
                },
                "wrap": {
                    "type": "boolean",
                    "description": "For 'to-html': wrap in a full HTML document (default false)."
                },
                "title": {
                    "type": "string",
                    "description": "Document title for 'to-html' when wrap is true (default 'Document')."
                }
            },
            "required": []
        }),
    ));
    tools.push(make_tool(
        "url_tools",
        "Parse, build, encode, decode, and manipulate URLs without external utilities. \
         Actions: \
         `parse` (default) — break a URL into scheme, host, port, path, query parameters, fragment; \
         `build` — construct a URL from parts: 'scheme', 'host' (required), 'path', optional 'port', \
            'query' (raw string), 'params' (object of key/value pairs), 'fragment'; \
         `params` — inspect or modify query parameters; pass 'op': list (default) | set | remove; \
            'key' and 'value' for set/remove operations; \
         `encode` — percent-encode a string ('input' required); 'component: true' for strict encoding; \
         `decode` — percent-decode a string ('input' required); \
         `normalize` — lowercase scheme/host, resolve dot segments; \
         `validate` — check if a URL is valid and flag common issues (insecure HTTP, localhost). \
         Pass 'url' with the URL string.",
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "description": "Operation: parse (default), build, params, encode, decode, normalize, validate."
                },
                "url": {
                    "type": "string",
                    "description": "The URL to parse, normalize, validate, or use as the base for params operations. Also accepted as 'input'."
                },
                "scheme": {
                    "type": "string",
                    "description": "URL scheme for 'build' (default 'https')."
                },
                "host": {
                    "type": "string",
                    "description": "Hostname or IP for 'build' (required)."
                },
                "path": {
                    "type": "string",
                    "description": "URL path for 'build' (default '/')."
                },
                "port": {
                    "type": "integer",
                    "description": "Port number for 'build' (omit for scheme default)."
                },
                "query": {
                    "type": "string",
                    "description": "Raw query string for 'build' (without leading '?')."
                },
                "params": {
                    "type": "object",
                    "description": "Key-value pairs to encode as the query string for 'build'."
                },
                "fragment": {
                    "type": "string",
                    "description": "URL fragment for 'build' (without leading '#')."
                },
                "op": {
                    "type": "string",
                    "description": "Sub-operation for 'params': list (default), set, remove."
                },
                "key": {
                    "type": "string",
                    "description": "Parameter name for 'params' set/remove."
                },
                "value": {
                    "type": "string",
                    "description": "Parameter value for 'params' set."
                },
                "input": {
                    "type": "string",
                    "description": "String to encode or decode for 'encode'/'decode' actions."
                },
                "component": {
                    "type": "boolean",
                    "description": "For 'encode': use strict component encoding (encodes all non-alphanumeric chars). Default false."
                }
            },
            "required": []
        }),
    ));
    tools.push(make_tool(
        "line_tools",
        "Line-based text processing without external utilities — a self-contained grep/head/tail/sort/cut. \
         Actions: \
         `grep` (default) — filter lines matching 'pattern'; 'regex: true' for regex; \
            'invert: true' for non-matching; 'ignore_case: true'; 'line_numbers: true' (default on); 'max' cap; \
         `head` — first N lines ('n', default 10); \
         `tail` — last N lines ('n', default 10); \
         `sort` — sort lines; 'numeric: true' for numeric order; 'reverse: true'; 'ignore_case: true'; \
            'unique: true' to deduplicate after sort; \
         `unique` — remove duplicates preserving first-occurrence order; 'count: true' to show frequency; \
            'sorted: true' to rank by frequency; 'ignore_case: true'; \
         `count` — line, word, character, and byte counts; \
         `slice` — extract lines 'from' to 'to' (1-based, inclusive); \
         `number` — add line numbers; 'start' (default 1), 'step' (default 1), 'skip_blank: true'; \
         `join` — join all lines into one string; 'sep' (default ', '); 'trim: true' (default); 'skip_blank: true' (default); \
         `replace` — find-and-replace across all lines; 'from' and 'to' required; 'regex: true'; \
            'ignore_case: true'; 'limit' for max replacements; \
         `cut` — extract one field per line by delimiter; 'field' is 1-based (default 1); \
            'delimiter'/'sep'/'d' (default tab). \
         Pass 'text' for inline content or 'file' for a file path.",
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "description": "Operation: grep (default), head, tail, sort, unique, count, slice, number, join, replace, cut."
                },
                "text": {
                    "type": "string",
                    "description": "Inline text to process."
                },
                "file": {
                    "type": "string",
                    "description": "Path to a text file (relative or absolute). Also accepted as 'input'."
                },
                "pattern": {
                    "type": "string",
                    "description": "Search pattern for 'grep'. Also accepted as 'query' or 'search'."
                },
                "regex": {
                    "type": "boolean",
                    "description": "Treat 'pattern'/'from' as a regular expression (default false)."
                },
                "invert": {
                    "type": "boolean",
                    "description": "For 'grep': return lines that do NOT match (default false)."
                },
                "ignore_case": {
                    "type": "boolean",
                    "description": "Case-insensitive matching for grep, sort, unique, replace (default false)."
                },
                "n": {
                    "type": "integer",
                    "description": "Number of lines for 'head' and 'tail' (default 10)."
                },
                "reverse": {
                    "type": "boolean",
                    "description": "For 'sort': reverse order (default false)."
                },
                "numeric": {
                    "type": "boolean",
                    "description": "For 'sort': numeric sort (default false)."
                },
                "unique": {
                    "type": "boolean",
                    "description": "For 'sort': deduplicate after sorting."
                },
                "count": {
                    "type": "boolean",
                    "description": "For 'unique': show frequency count next to each line."
                },
                "from": {
                    "type": "string",
                    "description": "Find string/pattern for 'replace', or start line for 'slice' (1-based)."
                },
                "to": {
                    "type": "string",
                    "description": "Replacement string for 'replace', or end line for 'slice' (1-based, inclusive)."
                },
                "sep": {
                    "type": "string",
                    "description": "Separator for 'join' (default ', ') or delimiter for 'cut' (default tab)."
                },
                "field": {
                    "type": "integer",
                    "description": "Field number for 'cut' (1-based, default 1)."
                },
                "start": {
                    "type": "integer",
                    "description": "Starting line number for 'number' (default 1)."
                },
                "step": {
                    "type": "integer",
                    "description": "Line number increment for 'number' (default 1)."
                }
            },
            "required": []
        }),
    ));
    tools.push(make_tool(
        "path_tools",
        "Path string parsing and manipulation without external utilities or filesystem access. \
         Actions: \
         `parse` (default) — split a path into parent, filename, stem, extension, absolute flag, and components; \
         `join` — join path segments; pass 'base' + 'parts' array, or 'paths' array; \
         `normalize` — logically resolve . and .. segments without touching the filesystem; \
         `relative` — compute the relative path from 'from' to 'to'; \
         `basename` — filename with extension; \
         `stem` — filename without extension; \
         `extension` — current extension; optionally pass 'replace' to swap it; \
         `is-absolute` — YES/NO whether the path is absolute. \
         All actions accept 'path' as the input path string (also 'input' or 'text').",
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "description": "Operation: parse (default), join, normalize, relative, basename, stem, extension, is-absolute."
                },
                "path": {
                    "type": "string",
                    "description": "The path string to operate on. Also accepted as 'input' or 'text'."
                },
                "from": {
                    "type": "string",
                    "description": "Source path for 'relative' action."
                },
                "to": {
                    "type": "string",
                    "description": "Target path for 'relative' action."
                },
                "base": {
                    "type": "string",
                    "description": "Base path for 'join' action."
                },
                "parts": {
                    "type": "array",
                    "description": "Path segments to append for 'join' action.",
                    "items": { "type": "string" }
                },
                "paths": {
                    "type": "array",
                    "description": "All path segments to join for 'join' action (alternative to base+parts).",
                    "items": { "type": "string" }
                },
                "replace": {
                    "type": "string",
                    "description": "New extension to swap in for 'extension' action (e.g. 'txt')."
                }
            },
            "required": []
        }),
    ));
    tools.push(make_tool(
        "table_tools",
        "Format tabular data as ASCII or markdown tables without external utilities. \
         Actions: \
         `format` (default) — render 'rows' (2D array) + optional 'headers' as a table; \
         `from-csv` — parse CSV text (from 'text' or 'csv') and render as a table; \
            'header: true' (default) treats the first row as column headers; \
         `from-json` — format a JSON array of objects or 2D array as a table; pass 'json' or 'text'; \
         `to-markdown` — render any input as a GitHub-flavored markdown table; \
         `transpose` — flip rows and columns; pass 'rows' 2D array and optional 'headers'. \
         Style options via 'style': 'simple' (default — spaces + dash separator), \
            'bordered' (ASCII box drawing with | and +), 'markdown'. \
         Example: table_tools(action: 'format', headers: ['Name','Score'], rows: [['Alice','95'],['Bob','87']]) or \
         table_tools(action: 'from-csv', text: 'name,age\\nAlice,30', style: 'bordered').",
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "description": "Operation: format (default), from-csv, from-json, to-markdown, transpose."
                },
                "rows": {
                    "type": "array",
                    "description": "2D array of data rows. Each row is an array of cell values.",
                    "items": { "type": "array" }
                },
                "headers": {
                    "type": "array",
                    "description": "Optional array of column header strings.",
                    "items": { "type": "string" }
                },
                "text": {
                    "type": "string",
                    "description": "Raw CSV text (for from-csv) or JSON text (for from-json and to-markdown)."
                },
                "csv": {
                    "type": "string",
                    "description": "Raw CSV text — alias for 'text' in from-csv action."
                },
                "json": {
                    "type": "string",
                    "description": "JSON text (array of objects or 2D array) for from-json or to-markdown."
                },
                "style": {
                    "type": "string",
                    "description": "Table style: 'simple' (default), 'bordered' (| and + box), 'markdown'."
                },
                "header": {
                    "type": "boolean",
                    "description": "Whether the first row is a header row in from-csv (default true)."
                }
            },
            "required": []
        }),
    ));
    tools.push(make_tool(
        "duration_tools",
        "Parse, humanize, convert, and add time durations without external utilities. \
         Actions: \
         `parse` (default) — break any duration string into years/days/hours/minutes/seconds breakdown, \
            compact form, and long-form human label; \
         `humanize` — convert a duration to readable text; 'style: compact' for short form (1h 30m 45s); \
         `convert` — express a duration in seconds/minutes/hours/days/weeks; \
            'to' for a specific unit (seconds, minutes, hours, days, weeks) or omit for all; \
         `add` — sum two durations via 'a' and 'b', or sum an array via 'durations'. \
         Pass 'duration' (or 'input'/'value') with the duration string. \
         Input formats: '1h 30m 45s', '90 minutes', '2 days 4 hours', '5400' (seconds), \
         '1:30:45' (HH:MM:SS), 'PT1H30M45S' (ISO 8601).",
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "description": "Operation: parse (default), humanize, convert, add."
                },
                "duration": {
                    "type": "string",
                    "description": "Duration string: '1h 30m 45s', '90 minutes', '5400', '1:30:45', 'PT1H30M45S'. Also 'input' or 'value'."
                },
                "style": {
                    "type": "string",
                    "description": "humanize style: 'long' (default, full words) or 'compact' (1h 30m 45s)."
                },
                "to": {
                    "type": "string",
                    "description": "convert: target unit — seconds, minutes, hours, days, weeks. Omit for all."
                },
                "a": {
                    "type": "string",
                    "description": "add: first duration string."
                },
                "b": {
                    "type": "string",
                    "description": "add: second duration string."
                },
                "durations": {
                    "type": "array",
                    "items": {"type": "string"},
                    "description": "add: array of duration strings to sum."
                }
            },
            "required": []
        }),
    ));
    tools.push(make_tool(
        "dotenv_tools",
        "Parse, validate, convert, and merge .env files without external utilities. \
         Actions: \
         `parse` (default) — display all key-value pairs with line numbers; 'show_values: false' to redact; \
         `validate` — check key names (A-Z, a-z, 0-9, _), quote balance, duplicate keys; \
         `get` — retrieve a specific key's value; pass 'key'; \
         `list` — show key names only (no values); \
         `to-json` — convert to a JSON object; \
         `to-shell` — generate export/SET commands; 'shell: bash' (default), 'powershell', or 'cmd'; \
         `merge` — overlay one .env on another; pass 'base' and 'overlay' text — overlay wins on conflict, \
            base order preserved, overlay-only keys appended. \
         Pass 'text' or 'env' for inline .env content, or 'file' for a file path. \
         Handles: KEY=value, KEY=\"quoted\", KEY='single', # comments, empty values.",
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "description": "Operation: parse (default), validate, get, list, to-json, to-shell, merge."
                },
                "text": {
                    "type": "string",
                    "description": "Inline .env content. Also accepted as 'env' or 'input'."
                },
                "file": {
                    "type": "string",
                    "description": "Path to a .env file."
                },
                "key": {
                    "type": "string",
                    "description": "get: the environment variable key to retrieve."
                },
                "shell": {
                    "type": "string",
                    "description": "to-shell: target shell format — bash (default), powershell, cmd."
                },
                "show_values": {
                    "type": "boolean",
                    "description": "parse: whether to show values (default true). Set false to redact."
                },
                "base": {
                    "type": "string",
                    "description": "merge: base .env content (inline text)."
                },
                "overlay": {
                    "type": "string",
                    "description": "merge: overlay .env content — wins on conflict with base."
                }
            },
            "required": []
        }),
    ));
    tools.push(make_tool(
        "ansi_tools",
        "Process ANSI/VT100 escape codes — strip, colorize, measure, or parse terminal sequences without external utilities. \
         Actions: \
         `strip` (default) — remove all ANSI escape sequences and return plain text; \
         `colorize` — wrap text in ANSI SGR codes using 'fg'/'bg' color name and/or 'style' (single string or array); \
            colors: black, red, green, yellow, blue, magenta, cyan, white, gray, bright_red, bright_green, etc.; \
            styles: bold, dim, italic, underline, blink, reverse, strikethrough; \
         `length` — count visible (non-escape) characters; \
         `parse` — identify and describe each ANSI sequence found in input text.",
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "description": "Operation: strip (default), colorize, length, parse."
                },
                "text": {
                    "type": "string",
                    "description": "Input text (may contain or not contain ANSI sequences). Also 'input'."
                },
                "fg": {
                    "type": "string",
                    "description": "colorize: foreground color name (red, green, blue, yellow, cyan, magenta, white, black, gray, bright_*)."
                },
                "bg": {
                    "type": "string",
                    "description": "colorize: background color name."
                },
                "style": {
                    "description": "colorize: style name or array of style names (bold, dim, italic, underline, blink, reverse, strikethrough)."
                }
            },
            "required": []
        }),
    ));
    tools.push(make_tool(
        "template_tools",
        "Render {{VAR}} placeholder templates, list placeholders, validate, and preview without external utilities. \
         Actions: \
         `render` (default) — substitute {{VAR}} and {{VAR|default}} placeholders using 'vars' object; \
            'strict: true' to error on undefined variables (default: leave undefined as-is); \
         `list` — list all unique {{VAR}} placeholder names found in the template with any defaults; \
         `validate` — check for unbalanced braces; if 'vars' provided, report undefined variables; \
         `preview` — show each placeholder as DEFINED/MISSING with the rendered output using [MISSING:VAR] markers. \
         Pass 'template' (or 'text'/'file') for the template string. \
         Pass 'vars' as a JSON object mapping variable names to values.",
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "description": "Operation: render (default), list, validate, preview."
                },
                "template": {
                    "type": "string",
                    "description": "Template string containing {{VAR}} and {{VAR|default}} placeholders. Also 'text'."
                },
                "file": {
                    "type": "string",
                    "description": "Path to a template file."
                },
                "vars": {
                    "type": "object",
                    "description": "JSON object mapping variable names to their substitution values.",
                    "additionalProperties": true
                },
                "strict": {
                    "type": "boolean",
                    "description": "render: error if any variable is undefined (default false — leaves undefined as placeholder)."
                }
            },
            "required": []
        }),
    ));
    tools.push(make_tool(
        "char_tools",
        "Unicode character inspection, codepoint lookup, escape/unescape, and property checks without external utilities. \
         Actions: \
         `info` (default) — full Unicode info for a char or string: codepoint (U+XXXX), block name, category, decimal/hex/octal/binary, uppercase/lowercase variants; \
         `codepoint` — convert a character to its codepoint (U+XXXX); pass 'codepoint' as number or 'U+XXXX' string to reverse (codepoint → char); \
         `escape` — escape non-printable or non-ASCII chars to Unicode escape sequences; \
            'style: unicode' (default) = \\u{XXXXX}; 'style: json' = \\uXXXX (with surrogate pairs for SMP); 'style: hex' = \\xXX; \
         `unescape` — decode \\u{XXXXX}, \\uXXXX, \\xXX, \\n, \\t, \\r sequences back to characters; \
         `check` — test character properties for every char in 'input': is_ascii, is_alphabetic, is_numeric, is_alphanumeric, is_uppercase, is_lowercase, is_whitespace, is_control, is_ascii_punctuation. \
         Pass 'input' or 'text' for the string. For 'codepoint' action: pass 'codepoint' as a number or 'U+XXXX' string for reverse lookup.",
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "description": "Operation: info (default), codepoint, escape, unescape, check."
                },
                "input": {
                    "type": "string",
                    "description": "Input string to inspect. Also accepted as 'text' or 'char'."
                },
                "codepoint": {
                    "description": "For 'codepoint' action (reverse lookup): a decimal integer or 'U+XXXX' hex string to convert to a character.",
                    "oneOf": [{"type": "integer"}, {"type": "string"}]
                },
                "style": {
                    "type": "string",
                    "description": "For 'escape' action: unicode (default, \\u{XXXXX}), json (\\uXXXX with surrogate pairs), hex (\\xXX)."
                }
            },
            "required": []
        }),
    ));
    tools.push(make_tool(
        "stat_tools",
        "Statistical analysis on number arrays without external utilities. \
         Actions: \
         `describe` (default) — count/sum/min/max/range/mean/median/stddev/variance/Q1/Q3/IQR summary; \
         `histogram` — ASCII bar chart; 'bins' (default 10); 'width' bar width in chars (default 40); \
         `percentile` — compute percentiles; 'p' as a JSON array (e.g. [25, 50, 75, 90, 99]) or single value; \
         `mode` — most frequent values with occurrence counts and percentages; 'top' for top-N limit; \
         `outliers` — find values beyond N standard deviations; 'threshold' sigma cutoff (default 2.0); \
            'method: zscore' (default) or 'method: iqr' (uses IQR fence); \
         `zscore` — normalize each value to its z-score (value − mean) / stddev; \
         `correlate` — Pearson r between two number series; pass 'a' and 'b' as arrays. \
         Pass 'numbers' as a JSON array or 'data' as a comma/space/newline-delimited string.",
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "description": "Operation: describe (default), histogram, percentile, mode, outliers, zscore, correlate."
                },
                "numbers": {
                    "type": "array",
                    "items": {"type": "number"},
                    "description": "Array of numbers to analyze."
                },
                "data": {
                    "type": "string",
                    "description": "Comma, space, semicolon, or newline-delimited numbers (alternative to 'numbers' array)."
                },
                "bins": {
                    "type": "integer",
                    "description": "For 'histogram': number of bins (default 10, max 50)."
                },
                "width": {
                    "type": "integer",
                    "description": "For 'histogram': bar width in characters (default 40)."
                },
                "p": {
                    "description": "For 'percentile': single number or array of percentile values (e.g. [25, 50, 75, 90, 99]).",
                    "oneOf": [{"type": "number"}, {"type": "array", "items": {"type": "number"}}]
                },
                "top": {
                    "type": "integer",
                    "description": "For 'mode': show top-N most frequent values (default 10)."
                },
                "threshold": {
                    "type": "number",
                    "description": "For 'outliers': sigma cutoff (default 2.0). Also 'sigma'."
                },
                "method": {
                    "type": "string",
                    "description": "For 'outliers': zscore (default) or iqr."
                },
                "a": {
                    "type": "array",
                    "items": {"type": "number"},
                    "description": "For 'correlate': first data series."
                },
                "b": {
                    "type": "array",
                    "items": {"type": "number"},
                    "description": "For 'correlate': second data series."
                }
            },
            "required": []
        }),
    ));
    tools.push(make_tool(
        "rss_tools",
        "Parse RSS 2.0 and Atom 1.0 feeds — list entries, extract metadata, links, or search without external utilities. \
         Actions: \
         `list` (default) — show all entries with title/date/author/link/description snippet; 'limit' to cap (default 20); \
         `info` — feed metadata: type (RSS/Atom), title, description, language, generator, last updated, author list; \
         `links` — extract all entry hyperlinks with their titles; \
         `search` — filter entries matching 'query' or 'q' substring against title, description, and author. \
         Pass 'text'/'xml'/'rss' for inline feed content or 'file' for a path to an .xml/.rss file.",
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "description": "Operation: list (default), info, links, search."
                },
                "text": {
                    "type": "string",
                    "description": "Inline RSS/Atom XML content. Also accepted as 'xml' or 'rss'."
                },
                "file": {
                    "type": "string",
                    "description": "Path to an RSS or Atom .xml file."
                },
                "limit": {
                    "type": "integer",
                    "description": "For 'list': maximum number of entries to show (default 20)."
                },
                "query": {
                    "type": "string",
                    "description": "For 'search': substring to match against title, description, and author. Also 'q'."
                }
            },
            "required": []
        }),
    ));
    tools.push(make_tool(
        "keyval_tools",
        "Persistent key-value store in `.hematite/kv.json` — set, get, list, delete, and clear named values across tool calls. \
         Actions: \
         `set` — store a value; 'key' (string) + 'value' (any JSON type: string, number, boolean, array, object); \
         `get` — retrieve a value by 'key'; returns error if not found; \
         `list` — show all keys and values; optional 'prefix' to filter to a namespace; \
         `delete` — remove a key by name; \
         `clear` — wipe all keys, or all keys matching 'prefix'; \
         `keys` — list key names only (no values). \
         Use 'namespace'/'ns' to automatically prefix all keys (e.g. ns='build', key='version' → stored as 'build:version'). \
         Store location: `.hematite/kv.json` in the nearest parent with .hematite/, or `~/.hematite/kv.json` as fallback.",
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "description": "Operation: set, get, list (default), delete, clear, keys."
                },
                "key": {
                    "type": "string",
                    "description": "Key name for set/get/delete operations."
                },
                "value": {
                    "description": "Value to store (any JSON type: string, number, boolean, array, or object). Required for 'set'."
                },
                "namespace": {
                    "type": "string",
                    "description": "Namespace prefix applied to all keys (e.g. 'build' → 'build:key'). Also 'ns'."
                },
                "prefix": {
                    "type": "string",
                    "description": "For 'list'/'clear'/'keys': filter to keys starting with this prefix."
                },
                "store": {
                    "type": "string",
                    "description": "Custom path to the store JSON file (overrides default .hematite/kv.json)."
                }
            },
            "required": []
        }),
    ));
    tools.push(make_tool(
        "net_lookup_tools",
        "Look up well-known TCP/UDP port numbers, service names, and IANA IP protocol numbers — no shell required. \
         Actions: \
         `port` (default) — look up a port number; returns the service name(s) and description; 'port' (number) required; \
            optional 'protocol'/'proto' to filter to tcp or udp; \
         `service` — look up a service name; returns all matching port/protocol entries; 'name'/'service' required; \
         `search` — fuzzy search across service names and descriptions; 'query'/'q' required; \
         `protocol` — look up an IANA IP protocol by number ('number'/'num') or name ('name'/'proto'); omit args to list all. \
         Example: net_lookup_tools(action: 'port', port: 443) or \
         net_lookup_tools(action: 'service', name: 'postgresql') or \
         net_lookup_tools(action: 'search', query: 'database') or \
         net_lookup_tools(action: 'protocol', number: 6).",
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "description": "Operation: port (default), service, search, protocol."
                },
                "port": {
                    "type": "integer",
                    "description": "Port number to look up (for 'port' action). Also 'number'."
                },
                "name": {
                    "type": "string",
                    "description": "Service or protocol name to look up (for 'service' or 'protocol' action)."
                },
                "protocol": {
                    "type": "string",
                    "description": "Filter to 'tcp' or 'udp' (for 'port' action). Also 'proto'."
                },
                "query": {
                    "type": "string",
                    "description": "Search term for 'search' action. Also 'q'."
                },
                "number": {
                    "type": "integer",
                    "description": "IANA IP protocol number to look up (for 'protocol' action). Also 'num'."
                }
            },
            "required": []
        }),
    ));
    tools.push(make_tool(
        "money_tools",
        "Financial calculations: compound interest, loan payments, APR/APY conversion, discounts, \
         tip splitting, and currency formatting — no external libraries needed. \
         Actions: \
         `compound_interest` — final amount and total interest; 'principal', 'rate' (% per year), \
            'periods' (years), 'n' (compounds per year, default 1); \
         `loan` — monthly payment and amortization summary; 'principal', 'annual_rate' (%), 'term_months'; \
         `apr_to_apy` — convert APR to APY; 'apr' (%), 'n' (compounds per year, default 12); \
         `discount` — sale price and savings; 'price' (original), 'percent' (% off); \
         `percent_of` — what percent A is of B, or what X% of N is; \
            'a'/'value' and 'b'/'total', OR 'percent' and 'of'; \
         `format_currency` — format a number with symbol and thousands separators; \
            'amount', optional 'symbol' (default '$'), 'decimals' (default 2); \
         `tip` — tip amount and per-person total; 'bill', 'tip_percent' (default 18), 'people' (default 1); \
         `split_bill` — per-person share; 'total', 'people', optional 'tip_percent'. \
         Example: money_tools(action: 'loan', principal: 250000, annual_rate: 6.5, term_months: 360) or \
         money_tools(action: 'tip', bill: 85.50, tip_percent: 20, people: 4).",
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "description": "Operation: compound_interest, loan, apr_to_apy, discount, percent_of, format_currency, tip, split_bill."
                },
                "principal": {
                    "type": "number",
                    "description": "Loan or investment principal amount."
                },
                "rate": {
                    "type": "number",
                    "description": "Annual interest rate as a percentage (e.g. 5 for 5%)."
                },
                "annual_rate": {
                    "type": "number",
                    "description": "Annual interest rate as a percentage for loan calculations."
                },
                "periods": {
                    "type": "number",
                    "description": "Number of years for compound interest."
                },
                "n": {
                    "type": "integer",
                    "description": "Number of compounding periods per year (default 1 for compound_interest, 12 for apr_to_apy)."
                },
                "term_months": {
                    "type": "integer",
                    "description": "Loan term in months."
                },
                "apr": {
                    "type": "number",
                    "description": "Annual percentage rate to convert (for apr_to_apy)."
                },
                "price": {
                    "type": "number",
                    "description": "Original price (for discount)."
                },
                "percent": {
                    "type": "number",
                    "description": "Percentage value (for discount: % off; for percent_of: the % rate)."
                },
                "a": {
                    "type": "number",
                    "description": "Value part for percent_of (what percent is 'a' of 'b')."
                },
                "b": {
                    "type": "number",
                    "description": "Total for percent_of."
                },
                "of": {
                    "type": "number",
                    "description": "Base for percent_of (what is X% of 'of')."
                },
                "amount": {
                    "type": "number",
                    "description": "Amount to format (for format_currency)."
                },
                "symbol": {
                    "type": "string",
                    "description": "Currency symbol (default '$')."
                },
                "decimals": {
                    "type": "integer",
                    "description": "Decimal places for format_currency (default 2)."
                },
                "bill": {
                    "type": "number",
                    "description": "Bill amount before tip (for tip)."
                },
                "tip_percent": {
                    "type": "number",
                    "description": "Tip percentage (default 18)."
                },
                "people": {
                    "type": "integer",
                    "description": "Number of people splitting (default 1)."
                },
                "total": {
                    "type": "number",
                    "description": "Total bill amount including tax (for split_bill)."
                }
            },
            "required": ["action"]
        }),
    ));
    tools.push(make_tool(
        "size_tools",
        "Parse, convert, format, and compare data sizes (bytes/KB/MB/GB/TB and binary KiB/MiB/GiB/TiB) \
         and estimate bandwidth transfer times — no shell commands needed. \
         Actions: \
         `convert` (default) — show all conversions for a size; optional 'to' unit for a single result; \
         `parse` — resolve a human-readable size string to exact bytes; \
         `format` — format bytes as a human-readable label; 'style: decimal/binary/auto'; \
         `compare` — compare two sizes ('a', 'b') and show ratio and difference; \
         `bandwidth` — estimate transfer time given 'speed' (e.g. '100 Mbps'); \
            or compute speed given 'time' (e.g. '30s'); omit both to see a table of common speeds. \
         Input: 'size'/'input'/'value' as a string like '1.5 GB', '512 MiB', '1073741824'. \
         Accepts B, KB, MB, GB, TB, PB (SI/decimal) and KiB, MiB, GiB, TiB, PiB (IEC/binary). \
         Example: size_tools(action: 'convert', size: '1.5 GB') or \
         size_tools(action: 'bandwidth', size: '4 GB', speed: '100 Mbps').",
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "description": "Operation: convert (default), parse, format, compare, bandwidth."
                },
                "size": {
                    "type": "string",
                    "description": "Size to convert/parse/format (e.g. '1.5 GB', '512 MiB', '2048'). Also 'input'/'value'."
                },
                "to": {
                    "type": "string",
                    "description": "Target unit for 'convert': KB, MB, GB, TB, KiB, MiB, GiB, TiB."
                },
                "style": {
                    "type": "string",
                    "description": "For 'format': 'decimal' (SI), 'binary' (IEC), or 'auto' (default)."
                },
                "a": {
                    "type": "string",
                    "description": "First size for 'compare' (e.g. '2 GB')."
                },
                "b": {
                    "type": "string",
                    "description": "Second size for 'compare'."
                },
                "speed": {
                    "type": "string",
                    "description": "Transfer speed for 'bandwidth' (e.g. '100 Mbps', '1 Gbps', '50 MB/s')."
                },
                "time": {
                    "type": "string",
                    "description": "Transfer duration for 'bandwidth' (e.g. '30s', '5m', '2h')."
                }
            },
            "required": []
        }),
    ));
    tools.push(make_tool(
        "validate_tools",
        "Validate common data formats — email, IPv4, IPv6, CIDR, MAC, URL, credit card (Luhn), \
         ISBN-10/13, UUID, phone (NANP/E.164), SemVer 2.0, hex color — without external utilities. \
         Actions: email, ipv4, ipv6, cidr, mac, url, credit_card, isbn, uuid, phone, semver, hex_color, \
         auto (default — detects the format type automatically). \
         Pass 'value'/'input'/'text' with the string to validate. \
         Example: validate_tools(action: 'email', value: 'user@example.com') or \
         validate_tools(action: 'cidr', value: '192.168.1.0/24') or \
         validate_tools(action: 'auto', value: 'v2.0.0-alpha.1').",
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "description": "Format to validate: email, ipv4, ipv6, cidr, mac, url, credit_card, isbn, uuid, phone, semver, hex_color, auto (default)."
                },
                "value": {
                    "type": "string",
                    "description": "The value to validate. Also 'input'/'text'."
                }
            },
            "required": []
        }),
    ));
    tools.push(make_tool(
        "token_tools",
        "Estimate LLM token counts, check context window budget, compare token costs, and truncate text to a token limit — no external libraries. \
         Actions: estimate (default — chars/4 + words*1.3 heuristics with fill bars for 4K/8K/32K/128K context windows), \
         budget (show fill % and remaining tokens for a specific context window size; 'context_size' defaults to 8192), \
         compare (token cost diff between two texts via 'a'/'b' fields), \
         truncate (cut text to approximately N tokens; 'max_tokens' defaults to 1000). \
         Example: token_tools(action: 'estimate', text: '...') or \
         token_tools(action: 'budget', text: '...', context_size: 4096) or \
         token_tools(action: 'compare', a: '...', b: '...').",
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "description": "estimate (default), budget, compare, truncate"
                },
                "text": {
                    "type": "string",
                    "description": "Input text. Also accepted as 'input' or 'content'."
                },
                "context_size": {
                    "type": "integer",
                    "description": "Context window size in tokens for the budget action (default 8192)."
                },
                "a": {
                    "type": "string",
                    "description": "First text for the compare action."
                },
                "b": {
                    "type": "string",
                    "description": "Second text for the compare action."
                },
                "max_tokens": {
                    "type": "integer",
                    "description": "Token limit for the truncate action (default 1000)."
                }
            },
            "required": []
        }),
    ));
    tools.push(make_tool(
        "mime_tools",
        "Look up MIME content types by file extension, find extensions for a MIME type, search, and list by category — 130+ entries, no external utilities. \
         Actions: from_ext (default — extension to MIME type + category; pass 'ext' like 'js', '.ts', or 'report.pdf'), \
         from_mime (MIME type string to file extensions; pass 'mime' like 'image/png'), \
         search (fuzzy search on extension or MIME type string; pass 'query'), \
         category (list all types in a category — text/image/audio/video/application/font; omit 'category' for a summary). \
         Example: mime_tools(action: 'from_ext', ext: 'pdf') or \
         mime_tools(action: 'from_mime', mime: 'application/json') or \
         mime_tools(action: 'search', query: 'audio') or \
         mime_tools(action: 'category', category: 'image').",
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "description": "from_ext (default), from_mime, search, category"
                },
                "ext": {
                    "type": "string",
                    "description": "File extension for from_ext action — e.g. 'js', '.ts', 'report.pdf'. Also 'extension'/'file'/'input'."
                },
                "mime": {
                    "type": "string",
                    "description": "MIME type string for from_mime action — e.g. 'image/png'. Also 'type'/'input'."
                },
                "query": {
                    "type": "string",
                    "description": "Search term for the search action. Also 'q'/'input'."
                },
                "category": {
                    "type": "string",
                    "description": "Category for the category action: text, image, audio, video, application, font. Omit for a summary."
                }
            },
            "required": []
        }),
    ));
    tools.push(make_tool(
        "dns_tools",
        "Parse, filter, validate, and explain BIND-format DNS zone files without external utilities. \
         Actions: \
         `parse` (default) — list all resource records grouped by type with NAME/TTL/TYPE/DATA columns; \
           shows $ORIGIN and $TTL directives; summary of record counts per type. \
         `records` — filter by type; pass 'type' like 'MX', 'TXT', 'A', 'AAAA'. \
         `validate` — check for common zone file issues: missing SOA/NS, CNAME at zone apex, \
           MX pointing to CNAME, duplicate A/AAAA records, long TXT strings, multiple SPF records. \
         `explain` — plain-English explanation for each record type found; \
           SPF policy decoded token-by-token (include:/ip4:/ip6:/all/-all/~all), \
           DMARC policy decoded (p=/rua=/pct=), DKIM keys noted, CAA policy explained, SOA fields labelled. \
         Input: 'text'/'zone' for inline zone content, or 'file' for a file path. \
         Example: dns_tools(text: '...zone content...') or dns_tools(action: 'records', text: '...', type: 'MX') or \
         dns_tools(action: 'validate', file: '/etc/bind/db.example.com').",
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "description": "Operation: parse (default), records, validate, explain."
                },
                "text": {
                    "type": "string",
                    "description": "Inline zone file content. Also accepted as 'zone' or 'input'."
                },
                "file": {
                    "type": "string",
                    "description": "Path to a zone file on disk."
                },
                "type": {
                    "type": "string",
                    "description": "For 'records' action — record type to filter on, e.g. 'MX', 'TXT', 'A', 'AAAA'. Case-insensitive."
                }
            },
            "required": []
        }),
    ));
    tools.push(make_tool(
        "css_tools",
        "Parse, validate, extract variables, compute statistics, and minify CSS without external utilities. \
         Actions: \
         `parse` (default) — list all selectors with line number, property count, and key declarations; \
           at-rule summary (@media/@keyframes/@supports/etc.) with nested rule counts. \
         `validate` — check for common CSS issues: duplicate selectors, empty rule blocks, \
           duplicate properties within a rule, !important overuse (>5), vendor prefix without standard counterpart, \
           deep selector chains (>4 levels), invalid hex color lengths, z-index > 9999, unknown pseudo-elements. \
         `vars` — extract CSS custom properties: defined variables (property starting with --) with their values and \
           containing selector; usage count per variable (var(--name) references); variables used but not defined. \
         `stats` — overview: total rules/declarations/unique selectors, at-rule breakdown, top-10 most-used properties, \
           selector complexity (ID/class/element/deep), color values found, file size and gzip estimate, \
           !important count, CSS variable usage count. \
         `minify` — strip comments, collapse whitespace, remove trailing semicolons before closing braces; \
           shows original vs minified size and percentage reduction. \
         Input: 'text'/'css' for inline CSS content, or 'file' for a file path. \
         Example: css_tools(text: '...') or css_tools(action: 'validate', file: 'styles.css') or \
         css_tools(action: 'vars', css: '...') or css_tools(action: 'stats', file: 'app.css').",
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "description": "Operation: parse (default), validate, vars, stats, minify."
                },
                "text": {
                    "type": "string",
                    "description": "Inline CSS content. Also accepted as 'css' or 'input'."
                },
                "file": {
                    "type": "string",
                    "description": "Path to a CSS file on disk."
                }
            },
            "required": []
        }),
    ));
    tools.push(make_tool(
        "log_parse_tools",
        "Parse, detect, filter, and aggregate structured log lines without external utilities. \
         Supports JSON Lines, key=value, Apache Common/Combined, and Syslog formats — auto-detected from the first line. \
         Actions: parse (default — detect format and extract fields from each line; 'max' limits lines shown, default 20), \
         detect (identify the log format and show per-format distribution), \
         filter (keep only lines where a named field matches a value; pass 'field' and 'value'), \
         stats (count occurrences of a field's values; 'field' defaults to 'status' for Apache or 'level' for others). \
         Pass 'format' to override detection: json/jsonl, kv/keyvalue, apache/common, combined, syslog. \
         Example: log_parse_tools(text: '...') or \
         log_parse_tools(action: 'filter', text: '...', field: 'status', value: '5') or \
         log_parse_tools(action: 'stats', text: '...', field: 'level').",
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "description": "parse (default), detect, filter, stats"
                },
                "text": {
                    "type": "string",
                    "description": "Log lines as a multi-line string. Also 'input'/'log'/'lines'."
                },
                "format": {
                    "type": "string",
                    "description": "Override format detection: json, kv, apache, combined, syslog."
                },
                "field": {
                    "type": "string",
                    "description": "Field name for filter/stats actions (e.g. 'status', 'level', 'method')."
                },
                "value": {
                    "type": "string",
                    "description": "Value to match for the filter action (case-insensitive substring)."
                },
                "max": {
                    "type": "integer",
                    "description": "Maximum lines to parse and display (default 20)."
                }
            },
            "required": []
        }),
    ));
    tools.push(make_tool(
        "robots_txt_tools",
        "Parse, check, validate, and summarize robots.txt files without external utilities. \
         Actions: parse (default — show all user-agent blocks with Allow/Disallow/crawl-delay; pass 'text'), \
         check (test whether a path is allowed or blocked; pass 'text', 'url'/'path', optional 'agent' default '*'; follows RFC 9309 specificity rules), \
         validate (check for unknown directives, paths without leading slash, missing wildcard block, Disallow: /), \
         summary (table: all blocks with allow/disallow counts and crawl-delay). \
         Example: robots_txt_tools(action: 'check', text: '...', path: '/admin/', agent: 'Googlebot') or \
         robots_txt_tools(action: 'parse', text: '...').",
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "description": "parse (default), check, validate, summary"
                },
                "text": {
                    "type": "string",
                    "description": "robots.txt file content as a string. Also 'input'/'robots'/'content'."
                },
                "url": {
                    "type": "string",
                    "description": "Full URL or path to test for the 'check' action (e.g. '/admin/login' or 'https://example.com/secret/'). Also 'path'."
                },
                "agent": {
                    "type": "string",
                    "description": "User-agent name for the 'check' action (e.g. 'Googlebot', 'Bingbot'). Defaults to '*'. Also 'user_agent'/'ua'."
                }
            },
            "required": []
        }),
    ));
    tools.push(make_tool(
        "sitemap_tools",
        "Parse, search, and analyze sitemap.xml files without external utilities. \
         Handles both urlset (standard sitemap) and sitemapindex (sitemap of sitemaps). \
         Actions: parse (default — list URLs with lastmod/changefreq/priority; 'max' to limit, default 20), \
         search (filter URLs containing a query string; pass 'query'/'q'), \
         stats (total URLs, lastmod/changefreq/priority coverage rates, distribution tables), \
         list (all URL paths or filtered by prefix; pass optional 'filter'). \
         Pass 'xml' with the raw sitemap XML content. \
         Example: sitemap_tools(action: 'parse', xml: '...') or \
         sitemap_tools(action: 'search', xml: '...', query: '/blog/') or \
         sitemap_tools(action: 'stats', xml: '...').",
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "description": "parse (default), search, stats, list"
                },
                "xml": {
                    "type": "string",
                    "description": "sitemap.xml content as a string. Also 'text'/'sitemap'/'input'."
                },
                "query": {
                    "type": "string",
                    "description": "URL fragment to search for in the 'search' action. Also 'q'."
                },
                "filter": {
                    "type": "string",
                    "description": "Path prefix to filter results in the 'list' action. Also 'prefix'."
                },
                "max": {
                    "type": "integer",
                    "description": "Maximum URLs to show in 'parse' output (default 20)."
                }
            },
            "required": []
        }),
    ));
    tools.push(make_tool(
        "make_tools",
        "Parse and analyze Makefiles without external utilities. \
         Actions: list (default — all targets with dependencies, phony flag, and inline comment; tabular view), \
         explain (full detail for one target — description, phony flag, deps, command list; pass 'target'), \
         deps (dependency graph for all targets or a specific one; pass optional 'target'), \
         vars (all variable assignments — name, operator (=/:=/?=/+=), and value). \
         Example: make_tools(action: 'list', text: '...') or \
         make_tools(action: 'explain', text: '...', target: 'build') or \
         make_tools(action: 'vars', text: '...').",
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "description": "list (default), explain, deps, vars"
                },
                "text": {
                    "type": "string",
                    "description": "Makefile content as a string. Also 'makefile'/'content'/'input'."
                },
                "target": {
                    "type": "string",
                    "description": "Target name for the 'explain' or 'deps' actions."
                }
            },
            "required": []
        }),
    ));
    tools.push(make_tool(
        "changelog_tools",
        "Parse, query, and validate CHANGELOG.md files in Keep a Changelog format. \
         Actions: list (default — all releases with version, date, section names, item counts; YANKED flag), \
         get (full body of a specific version; pass 'version' — partial match supported), \
         latest (full body of the most recent non-Unreleased release), \
         validate (Keep a Changelog compliance check — Unreleased section, dates on releases, standard section names: Added/Changed/Deprecated/Removed/Fixed/Security, empty releases, YANKED releases). \
         Example: changelog_tools(action: 'list', text: '...') or \
         changelog_tools(action: 'get', text: '...', version: '1.2.0') or \
         changelog_tools(action: 'validate', text: '...').",
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "description": "list (default), get, latest, validate"
                },
                "text": {
                    "type": "string",
                    "description": "CHANGELOG.md content as a string. Also 'changelog'/'content'/'input'."
                },
                "version": {
                    "type": "string",
                    "description": "Version string for the 'get' action (e.g. '1.2.0', 'Unreleased'). Partial match supported."
                }
            },
            "required": []
        }),
    ));
    tools.push(make_tool(
        "gitignore_tools",
        "Parse, check path inclusion, generate, and explain .gitignore files without external utilities. \
         Actions: parse (default — list all patterns grouped by comment sections with pattern/negation/dir-only counts), \
         check (test if a file path is IGNORED or NOT IGNORED; pass 'path' and 'text'), \
         generate (produce a standard .gitignore for a language; pass 'language': rust/node/python/go/java/dotnet/react/docker), \
         explain (plain-English description of each pattern — scope, glob semantics, negation, directory-only). \
         Example: gitignore_tools(action: 'check', text: '...', path: 'dist/bundle.js') or \
         gitignore_tools(action: 'generate', language: 'node') or \
         gitignore_tools(action: 'explain', text: '...').",
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "description": "parse (default), check, generate, explain"
                },
                "text": {
                    "type": "string",
                    "description": ".gitignore content as a string. Also 'gitignore'/'content'/'input'."
                },
                "path": {
                    "type": "string",
                    "description": "File path to test in the 'check' action (e.g. 'dist/bundle.js'). Also 'file'."
                },
                "language": {
                    "type": "string",
                    "description": "Language/framework for the 'generate' action: rust, node, python, go, java, dotnet, react, docker. Also 'lang'."
                }
            },
            "required": []
        }),
    ));
    tools.push(make_tool(
        "license_tools",
        "Look up, detect, compare, and list software licenses without external utilities. \
         Covers 14 SPDX licenses: MIT, Apache-2.0, GPL-2.0, GPL-3.0, LGPL-2.1, LGPL-3.0, MPL-2.0, AGPL-3.0, BSD-2-Clause, BSD-3-Clause, ISC, Unlicense, CC0-1.0, EUPL-1.2. \
         Actions: info (default — full license detail: SPDX ID, category, copyleft, patent grant, commercial use, sublicensing, conditions, permissions, limitations; pass 'license'), \
         detect (identify license from raw license file text; pass 'text'), \
         compare (side-by-side property comparison of two licenses; pass 'a' and 'b'), \
         list (all licenses grouped by category — Permissive/Weak Copyleft/Strong Copyleft/Public Domain; optional 'category' filter). \
         Example: license_tools(action: 'info', license: 'MIT') or \
         license_tools(action: 'compare', a: 'MIT', b: 'GPL-3.0') or \
         license_tools(action: 'detect', text: 'MIT License...') or \
         license_tools(action: 'list', category: 'permissive').",
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "description": "info (default), detect, compare, list"
                },
                "license": {
                    "type": "string",
                    "description": "License name or SPDX ID for the 'info' action (e.g. 'MIT', 'Apache-2.0', 'gpl'). Also 'id'/'name'."
                },
                "text": {
                    "type": "string",
                    "description": "License file content for the 'detect' action. Also 'content'."
                },
                "a": {
                    "type": "string",
                    "description": "First license for the 'compare' action (SPDX ID or name)."
                },
                "b": {
                    "type": "string",
                    "description": "Second license for the 'compare' action (SPDX ID or name)."
                },
                "category": {
                    "type": "string",
                    "description": "Category filter for the 'list' action: permissive, copyleft, public domain. Also 'filter'."
                }
            },
            "required": []
        }),
    ));
    tools.push(make_tool(
        "ssh_config_tools",
        "Parse, query, explain, and validate SSH config files (~/.ssh/config) without external utilities. \
         Actions: list (default — summary of all Host blocks with HostName, User, Port, IdentityFile, and ProxyJump), \
         get (all options for a named host; pass 'host' — partial match supported), \
         explain (plain-English description of every option for all hosts or a filtered host; pass optional 'host'), \
         validate (check for duplicate Host patterns, StrictHostKeyChecking=no security warnings, relative IdentityFile paths). \
         Pass the config file content as 'text'. \
         Example: ssh_config_tools(action: 'list', text: '...') or \
         ssh_config_tools(action: 'get', text: '...', host: 'prod') or \
         ssh_config_tools(action: 'explain', text: '...') or \
         ssh_config_tools(action: 'validate', text: '...').",
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "description": "list (default), get, explain, validate"
                },
                "text": {
                    "type": "string",
                    "description": "SSH config file content as a string. Also 'config'/'content'/'input'."
                },
                "host": {
                    "type": "string",
                    "description": "Host pattern to look up (for 'get' and 'explain' actions). Partial match supported."
                }
            },
            "required": []
        }),
    ));
    tools.push(make_tool(
        "systemd_tools",
        "Parse, inspect, and validate systemd unit files (.service/.timer/.socket) without external utilities. \
         Pass the unit file content as 'text'. \
         Actions: info (default — unit type, description, [Unit]/[Service]/[Timer]/[Socket]/[Install] summary), \
         service (detailed [Service] section: exec commands, identity, restart policy, environment, security hardening), \
         timer (timer triggers with schedule explanations for OnCalendar/OnBootSec/OnUnitActiveSec, Persistent flag), \
         validate (warn on missing Description, missing ExecStart, Type=forking without PIDFile, no Restart=, running as root, missing security hardening, missing [Install]). \
         Example: systemd_tools(action: 'info', text: '...') or \
         systemd_tools(action: 'service', text: '...') or \
         systemd_tools(action: 'timer', text: '...') or \
         systemd_tools(action: 'validate', text: '...').",
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "description": "info (default), service, timer, validate"
                },
                "text": {
                    "type": "string",
                    "description": "Systemd unit file content as a string. Also 'unit'/'content'/'input'."
                }
            },
            "required": []
        }),
    ));
    tools.push(make_tool(
        "nginx_conf_tools",
        "Parse, inspect, and validate nginx.conf files without external utilities. \
         Actions: list (default — all server blocks with server_name, listen ports, root/proxy, SSL, location count), \
         inspect (full detail for one server block with all directives and location blocks; pass 'server' as server_name or 1-based index), \
         locations (all location blocks with proxy_pass/root/alias targets; optional 'server' filter), \
         directives (global and http-context directives plus upstream definitions), \
         validate (warn on missing server_name, SSL listen without ssl_certificate, proxy_pass without Host header, multiple default servers). \
         Pass the config file content as 'text'. \
         Example: nginx_conf_tools(action: 'list', text: '...') or \
         nginx_conf_tools(action: 'inspect', text: '...', server: 'example.com') or \
         nginx_conf_tools(action: 'validate', text: '...').",
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "description": "list (default), inspect, locations, directives, validate"
                },
                "text": {
                    "type": "string",
                    "description": "nginx.conf content as a string. Also 'config'/'conf'/'content'/'input'."
                },
                "server": {
                    "type": "string",
                    "description": "server_name or 1-based index for 'inspect' and 'locations' actions. Partial match on server_name."
                }
            },
            "required": []
        }),
    ));
    tools.push(make_tool(
        "openapi_tools",
        "Parse, query, search, and validate OpenAPI 3.x / Swagger 2.x specs (YAML or JSON) without external utilities. \
         Actions: info (default — title, API version, description, servers list, endpoint/schema/tag counts, auth schemes), \
         endpoints (all paths + HTTP methods with summary, operationId, tags, deprecated flag; pass 'tag' to filter by tag), \
         schemas (all schema/definition names with type, description, and properties; pass 'schema' to filter by name), \
         search (filter endpoints by path, summary, operationId, tag, or HTTP method keyword; pass 'query'), \
         validate (missing info section, empty paths, missing summaries/operationIds, duplicate operationIds, deprecated endpoints). \
         Pass the spec content as 'text'. \
         Example: openapi_tools(action: 'info', text: '...') or \
         openapi_tools(action: 'endpoints', text: '...', tag: 'users') or \
         openapi_tools(action: 'search', text: '...', query: 'POST') or \
         openapi_tools(action: 'schemas', text: '...') or \
         openapi_tools(action: 'validate', text: '...').",
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "description": "info (default), endpoints, schemas, search, validate"
                },
                "text": {
                    "type": "string",
                    "description": "OpenAPI/Swagger spec content (YAML or JSON). Also 'yaml'/'json'/'spec'/'content'/'input'."
                },
                "tag": {
                    "type": "string",
                    "description": "Tag name to filter endpoints in the 'endpoints' action."
                },
                "schema": {
                    "type": "string",
                    "description": "Schema name filter for the 'schemas' action. Partial match."
                },
                "query": {
                    "type": "string",
                    "description": "Search term for the 'search' action — matches path, summary, operationId, tag, or HTTP method. Also 'q'."
                }
            },
            "required": []
        }),
    ));
    tools.push(make_tool(
        "github_actions_tools",
        "Parse, inspect, and validate GitHub Actions workflow YAML without external utilities. \
         Pass the workflow YAML content as 'text'. \
         Actions: info (default — workflow name, triggers, and per-job summary with runs-on/steps/needs), \
         jobs (detailed job listing — runs-on, step count, needs, matrix, concurrency, env vars), \
         steps (all steps per job with name, uses, run preview, and if condition; optional 'job' filter), \
         triggers (full trigger detail: branches/tags/paths filters, cron schedules, workflow_dispatch inputs, concurrency group), \
         validate (checks: missing 'on' triggers, missing runs-on, undefined needs references, steps without uses/run, missing top-level permissions). \
         Example: github_actions_tools(action: 'info', text: '...') or \
         github_actions_tools(action: 'steps', text: '...', job: 'build') or \
         github_actions_tools(action: 'triggers', text: '...') or \
         github_actions_tools(action: 'validate', text: '...').",
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "description": "info (default), jobs, steps, triggers, validate"
                },
                "text": {
                    "type": "string",
                    "description": "GitHub Actions workflow YAML content. Also 'yaml'/'workflow'/'content'/'input'."
                },
                "job": {
                    "type": "string",
                    "description": "Job ID filter for the 'steps' action. Partial match, case-insensitive."
                }
            },
            "required": []
        }),
    ));
    tools.push(make_tool(
        "terraform_tools",
        "Parse, inspect, and validate Terraform HCL files (.tf) without external utilities. \
         Pass the HCL content as 'text' (also 'hcl'/'tf'/'content'/'input'). \
         Actions: info (default — required_version, provider list with source/version, block counts for resource/data/module/variable/output/local), \
         resources (list all resource blocks with type, name, and key attributes: ami, instance_type, name, location, etc.), \
         variables (list all input variable blocks with type, description, default value or '(required)', SENSITIVE flag), \
         outputs (list all output blocks with value expression and SENSITIVE flag), \
         validate (warn on: missing required_version, permissive/wildcard provider versions, hardcoded credentials in resource bodies, sensitive-named outputs/variables without sensitive=true). \
         Example: terraform_tools(action: 'info', text: '...') or \
         terraform_tools(action: 'resources', text: '...') or \
         terraform_tools(action: 'variables', text: '...') or \
         terraform_tools(action: 'validate', text: '...').",
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "description": "info (default), resources, variables, outputs, validate"
                },
                "text": {
                    "type": "string",
                    "description": "Terraform HCL content. Also 'hcl'/'tf'/'content'/'input'."
                }
            },
            "required": []
        }),
    ));
    tools.push(make_tool(
        "package_json_tools",
        "Parse, inspect, and validate package.json files without external utilities. \
         Pass the JSON content as 'text' (also 'json'/'content'/'input'). \
         Actions: info (default — name, version, description, license, author, main/module/types, engines, script/dep counts, keywords, repository), \
         scripts (list all npm scripts with their command strings; pass 'filter' to narrow by name or command), \
         deps (list dependencies — prod/dev/peer/optional with version ranges, wildcard flags, URL-dep flags; pass 'kind': prod/dev/peer/optional/all), \
         validate (check for missing name/version/description/license, no engines field, wildcard dep versions, http:// deps, missing test/build scripts, no files whitelist, duplicate deps across sections). \
         Example: package_json_tools(action: 'info', text: '...') or \
         package_json_tools(action: 'scripts', text: '...') or \
         package_json_tools(action: 'deps', text: '...', kind: 'dev') or \
         package_json_tools(action: 'validate', text: '...').",
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "description": "info (default), scripts, deps, validate"
                },
                "text": {
                    "type": "string",
                    "description": "package.json content. Also 'json'/'content'/'input'."
                },
                "kind": {
                    "type": "string",
                    "description": "For 'deps' action: prod, dev, peer, optional, or all (default)."
                },
                "filter": {
                    "type": "string",
                    "description": "For 'scripts' or 'deps': filter by name/command substring."
                }
            },
            "required": []
        }),
    ));
    tools.push(make_tool(
        "sql_tools",
        "Parse, explain, and validate SQL statements (DDL and DML) without external utilities. \
         Pass the SQL content as 'text' (also 'sql'/'query'/'content'/'input'). \
         Actions: parse (default — count statements by type, list each with referenced tables, join count, subquery flag), \
         tables (extract CREATE TABLE definitions: column names, types, NOT NULL/PK/FK flags, table-level primary keys, foreign key relationships), \
         explain (plain-English explanation per statement: what it reads/writes, tables involved, joins, filters, subqueries, CTEs), \
         validate (warn on: SELECT *, DELETE/UPDATE without WHERE, DROP TABLE without IF EXISTS, implicit cross joins, NOT IN NULL risk, leading-wildcard LIKE, CREATE TABLE without PK). \
         Example: sql_tools(action: 'parse', text: '...') or \
         sql_tools(action: 'tables', sql: '...') or \
         sql_tools(action: 'explain', text: '...') or \
         sql_tools(action: 'validate', text: '...').",
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "description": "parse (default), tables, explain, validate"
                },
                "text": {
                    "type": "string",
                    "description": "SQL content. Also 'sql'/'query'/'content'/'input'."
                }
            },
            "required": []
        }),
    ));
    tools.push(make_tool(
        "proto_tools",
        "Parse, inspect, and validate Protocol Buffer (.proto) files without external utilities. \
         Pass the .proto content as 'text' (also 'proto'/'content'/'input'). \
         Actions: info (default — syntax version, package, imports, file options, message/enum/service counts with per-item summaries), \
         messages (detailed message and enum listing with field names, types, field numbers, labels optional/repeated/required, and inline field options), \
         services (all service definitions with RPC method signatures, streaming classification: unary/client-streaming/server-streaming/bidirectional), \
         validate (checks: unrecognised syntax, missing package declaration, empty messages, duplicate field numbers, field number 0 or reserved range 19000–19999, proto2 required fields, proto3 enum first value ≠ 0, empty services). \
         Example: proto_tools(action: 'info', text: '...') or \
         proto_tools(action: 'messages', text: '...') or \
         proto_tools(action: 'services', text: '...') or \
         proto_tools(action: 'validate', text: '...').",
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "description": "info (default), messages, services, validate"
                },
                "text": {
                    "type": "string",
                    "description": "Protobuf .proto file content. Also 'proto'/'content'/'input'."
                },
                "filter": {
                    "type": "string",
                    "description": "For 'messages' action: filter by message name substring."
                }
            },
            "required": []
        }),
    ));
    tools.push(make_tool(
        "pem_tools",
        "Inspect, decode, and validate PEM-encoded certificates, certificate chains, and private keys without external utilities. \
         Pass the PEM content as 'text' (also 'pem'/'content'/'input'). \
         Actions: info (default — per-block type label, certificate subject/issuer/validity window/SANs/key algorithm+bits/CA flag and expiry countdown), \
         chain (ordered chain display with issuer→subject linkage verification, self-signed root detection, chain completeness check), \
         validate (checks: expired certs, expiring within 30 days, self-signed leaf cert, weak SHA-1/MD5 signature algorithm, \
         RSA key < 2048 bits, missing SANs on leaf v3 cert, private key bundled alongside cert, chain presented out of order). \
         Example: pem_tools(action: 'info', text: '...') or \
         pem_tools(action: 'chain', text: '...') or \
         pem_tools(action: 'validate', text: '...').",
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "description": "info (default), chain, validate"
                },
                "text": {
                    "type": "string",
                    "description": "PEM file content. Also 'pem'/'content'/'input'."
                }
            },
            "required": []
        }),
    ));
    tools.push(make_tool(
        "env_schema_tools",
        "Validate a .env file against a .env.example schema — check for missing required keys, extra keys, and empty required values. \
         Pass 'example' (.env.example content) and 'env' (.env content); or 'example_file'/'env_file' for file paths. \
         Actions: validate (default — compare .env against .env.example schema, VALID/INVALID verdict with per-key findings), \
         diff (keys present in .env.example but absent from .env), \
         required (list which .env.example keys are required — no default placeholder — vs optional), \
         info (overview of both files — key counts, coverage percentage, required vs optional breakdown). \
         Example: env_schema_tools(action: 'validate', example: '...', env: '...') or \
         env_schema_tools(action: 'diff', example: '...', env: '...') or \
         env_schema_tools(action: 'required', example: '...').",
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "description": "validate (default), diff, required, info"
                },
                "example": {
                    "type": "string",
                    "description": ".env.example content. Also 'example_text'."
                },
                "env": {
                    "type": "string",
                    "description": ".env file content. Also 'env_text'."
                },
                "example_file": {
                    "type": "string",
                    "description": "Path to .env.example file."
                },
                "env_file": {
                    "type": "string",
                    "description": "Path to .env file."
                }
            },
            "required": []
        }),
    ));
    tools.push(make_tool(
        "graphql_tools",
        "Parse, inspect, and validate GraphQL schemas and query documents without external utilities. \
         Pass content as 'text' (also 'schema'/'query'/'graphql'/'content'/'input'). \
         Actions: info (default — document kind: schema definition/query document/mixed, type/interface/input/enum/union/scalar/directive counts, operation counts, schema entry points, conventional root types), \
         types (list all type definitions with fields, argument types+defaults, implements, descriptions, deprecated flags; optional 'filter' by name), \
         queries (list all operations and fragments with top-level field names), \
         validate (checks: missing Query root type, empty types/interfaces/enums/unions, input fields using output types, \
         fields or args referencing undefined types, duplicate type names, operations selecting no fields). \
         Example: graphql_tools(action: 'info', text: '...') or \
         graphql_tools(action: 'types', text: '...', filter: 'User') or \
         graphql_tools(action: 'queries', text: '...') or \
         graphql_tools(action: 'validate', text: '...').",
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "description": "info (default), types, queries, validate"
                },
                "text": {
                    "type": "string",
                    "description": "GraphQL schema or query document content. Also 'schema'/'query'/'graphql'/'content'/'input'."
                },
                "filter": {
                    "type": "string",
                    "description": "For 'types' action: filter by type name substring."
                }
            },
            "required": []
        }),
    ));
    tools.push(make_tool(
        "sql_migrate_tools",
        "Analyze, risk-score, and validate SQL migration files without external utilities. \
         Pass content as 'text' (also 'sql'/'migration'/'content'/'input'). \
         Actions: analyze (default — per-statement risk rating SAFE/LOW/MEDIUM/HIGH/CRITICAL with actionable notes for \
         DROP TABLE, DROP COLUMN, ALTER TABLE ADD COLUMN without DEFAULT, table renames, SET NOT NULL, UPDATE/DELETE without WHERE, \
         TRUNCATE, CREATE INDEX without CONCURRENTLY), \
         risk (show only medium/high/critical risk operations — quick safety scan), \
         ops (operation type summary with risk breakdown — what kinds of statements the migration contains), \
         validate (transaction wrapping check for destructive operations, BEGIN without COMMIT, \
         CONCURRENTLY index creation inside a transaction, overall verdict VALID/WARNINGS/HIGH RISK/CRITICAL). \
         Example: sql_migrate_tools(action: 'analyze', text: '...') or \
         sql_migrate_tools(action: 'risk', text: '...') or \
         sql_migrate_tools(action: 'validate', text: '...').",
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "description": "analyze (default), risk, ops, validate"
                },
                "text": {
                    "type": "string",
                    "description": "SQL migration file content. Also 'sql'/'migration'/'content'/'input'."
                }
            },
            "required": []
        }),
    ));
    tools.push(make_tool(
        "base_tools",
        "Extended base encoding and decoding toolkit — base16 (hex), base32 (RFC 4648), \
         base58 (Bitcoin/IPFS alphabet), and base85 (Z85/ZeroMQ) — without external utilities. \
         Actions: encode (default — encode 'input' text/bytes; specify 'encoding': base16/base32/base58/base85 \
         or omit to show all four at once), \
         decode (reverse; requires 'encoding'; shows UTF-8 and hex representations of the decoded bytes), \
         identify (guess encoding from character set and length heuristics — returns all plausible matches). \
         Example: base_tools(action: 'encode', input: 'hello', encoding: 'base58') or \
         base_tools(action: 'decode', input: 'Cn8eVZg', encoding: 'base58') or \
         base_tools(action: 'identify', input: 'NBSWY3DPEB3W64TMMQ').",
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "description": "encode (default), decode, identify"
                },
                "input": {
                    "type": "string",
                    "description": "Text or encoded string to process. Also 'text'/'data'."
                },
                "encoding": {
                    "type": "string",
                    "description": "base16, base32, base58, base85 (or 'all' for encode). Also 'format'."
                }
            },
            "required": []
        }),
    ));
    tools.push(make_tool(
        "lock_file_tools",
        "Parse and analyze dependency lock files without external utilities. \
         Supports Cargo.lock (cargo), package-lock.json (npm v1/v2/v3), yarn.lock (yarn v1/v2), \
         and poetry.lock (poetry) — auto-detected from filename or content. \
         Actions: info (default — package count, unique names, duplicate-version count, format metadata), \
         list (all packages sorted by name with versions; pass 'limit' to cap at N), \
         search (find packages matching a name substring; pass 'query'/'q'/'name'), \
         duplicates (packages appearing at more than one version — root cause of bundle bloat; \
         includes remediation hint: npm dedupe / yarn dedupe / cargo update). \
         Pass 'file' for a lock file path or 'text' for inline content. Pass 'format' to override detection. \
         Example: lock_file_tools(action: 'info', file: 'Cargo.lock') or \
         lock_file_tools(action: 'duplicates', file: 'package-lock.json') or \
         lock_file_tools(action: 'search', text: '...', query: 'tokio').",
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "description": "info (default), list, search, duplicates"
                },
                "file": {
                    "type": "string",
                    "description": "Path to lock file. Also 'path'."
                },
                "text": {
                    "type": "string",
                    "description": "Inline lock file content. Also 'content'/'lock'."
                },
                "format": {
                    "type": "string",
                    "description": "cargo, npm, yarn, poetry (auto-detected if omitted). Also 'type'."
                },
                "query": {
                    "type": "string",
                    "description": "Package name search term (for search action). Also 'q'/'name'."
                },
                "limit": {
                    "type": "number",
                    "description": "Max packages to show in list action (default 100)."
                }
            },
            "required": []
        }),
    ));
    tools.push(make_tool(
        "fraction_tools",
        "Fraction arithmetic, simplification, conversion, comparison, and mathematical series without external utilities. \
         Actions: simplify (default — reduce a fraction to lowest terms; shows GCD, mixed-number form, decimal, percent; \
         pass 'fraction' string like \"6/9\" or 'numerator'/'denominator' integers), \
         add/sub/mul/div (binary arithmetic on two fractions; 'a' and 'b' as fraction strings like \"1/4\"), \
         convert ('fraction' string → decimal/percent/mixed-number; or 'decimal' number → nearest fraction via continued fractions; \
         optional 'tolerance' for decimal-to-fraction precision), \
         compare ('a' and 'b' fraction strings for two-way comparison showing relation and difference; \
         or 'fractions' array for sorted ranking), \
         series ('type': harmonic (partial sums 1 + 1/2 + 1/3 ... ; 'terms' N), \
         egyptian (decompose a proper fraction into unit fractions; 'fraction'), \
         farey (Farey sequence F_n; 'n')). \
         Example: fraction_tools(action:'add', a:'1/3', b:'2/5') or \
         fraction_tools(action:'simplify', fraction:'18/24') or \
         fraction_tools(action:'convert', decimal:0.142857)",
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "description": "simplify (default), add, sub, mul, div, convert, compare, series"
                },
                "fraction": {
                    "type": "string",
                    "description": "Fraction string like \"3/4\" or \"7\" (for simplify/convert/compare)"
                },
                "numerator": { "type": "number", "description": "Numerator (alternative to 'fraction')" },
                "denominator": { "type": "number", "description": "Denominator (alternative to 'fraction')" },
                "a": { "type": "string", "description": "First fraction for binary operations or compare" },
                "b": { "type": "string", "description": "Second fraction for binary operations or compare" },
                "decimal": { "type": "number", "description": "Decimal number to convert to fraction" },
                "tolerance": { "type": "number", "description": "Precision for decimal-to-fraction conversion (default 1e-6)" },
                "fractions": { "type": "array", "description": "Array of fraction strings for compare" },
                "type": { "type": "string", "description": "Series type: harmonic, egyptian, farey" },
                "terms": { "type": "number", "description": "Number of terms for harmonic series (default 10, max 50)" },
                "n": { "type": "number", "description": "Order N for Farey sequence (default 7, max 20)" }
            },
            "required": []
        }),
    ));
    tools.push(make_tool(
        "number_theory_tools",
        "Pure number-theory calculations: prime factorization, primality testing, GCD/LCM with Bézout coefficients, \
         Euler totient, modular arithmetic, Collatz sequences, Fibonacci numbers, and perfect number classification — \
         all without external utilities. \
         Actions: factor (default — prime factorization, all divisors, divisor sum, perfect/abundant/deficient classification; 'n'), \
         primes ('limit' for sieve listing up to N; 'nth' for the Nth prime; 'test' for primality check of a single number), \
         gcd/lcm ('a' and 'b' integers for pair; or 'numbers' array for multi-value reduction; \
         gcd also shows Bézout coefficients and coprimality), \
         totient (Euler phi function — count of integers < n coprime to n; 'n'), \
         modpow (fast modular exponentiation; 'base', 'exp', 'modulus'), \
         modinv (modular inverse via extended Euclidean; 'a', 'modulus'; reports if none exists), \
         collatz (Collatz sequence from n to 1; 'n'; shows steps, max value, sequence), \
         fibonacci (list first N Fibonacci numbers via 'n'; specific index via 'nth'; membership test via 'test'), \
         perfect (classify n as perfect/abundant/deficient via 'n'; or scan a range via 'limit'). \
         Example: number_theory_tools(action:'factor', n:360) or \
         number_theory_tools(action:'primes', test:97) or \
         number_theory_tools(action:'collatz', n:27)",
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "description": "factor (default), primes, gcd, lcm, totient, modpow, modinv, collatz, fibonacci, perfect"
                },
                "n": { "type": "number", "description": "Main integer argument for factor/primes/totient/collatz/fibonacci/perfect" },
                "a": { "type": "number", "description": "First number for gcd/lcm/modinv" },
                "b": { "type": "number", "description": "Second number for gcd/lcm" },
                "numbers": { "type": "array", "description": "Array of integers for multi-value gcd/lcm" },
                "limit": { "type": "number", "description": "Upper bound for primes sieve or perfect number scan" },
                "nth": { "type": "number", "description": "Find the Nth prime or Nth Fibonacci number" },
                "test": { "type": "number", "description": "Number to test for primality (primes action) or Fibonacci membership" },
                "base": { "type": "number", "description": "Base for modpow" },
                "exp": { "type": "number", "description": "Exponent for modpow" },
                "modulus": { "type": "number", "description": "Modulus for modpow/modinv" }
            },
            "required": []
        }),
    ));
    tools.push(make_tool(
        "cipher_tools",
        "Classical cipher encoding, decoding, and frequency analysis without external utilities. \
         Actions: rot13 (default — ROT13 encode/decode; self-inverse; pass 'text'), \
         caesar (shift cipher; 'text' + 'shift' N; optional 'decode: true'; shows all-shifts brute-force table for short texts), \
         vigenere (polyalphabetic key cipher; 'text' + 'key'; optional 'decode: true'), \
         atbash (A↔Z reversal; 'text'; self-inverse), \
         rail_fence (transposition; 'text' + 'rails' N default 3; optional 'decode: true'; shows rail diagram for short texts), \
         analyze (letter frequency histogram vs English, Index of Coincidence cipher-type hint, automatic Caesar break guess; 'text'). \
         Example: cipher_tools(action:'caesar', text:'Hello World', shift:13) or \
         cipher_tools(action:'vigenere', text:'HELLO', key:'KEY') or \
         cipher_tools(action:'analyze', text:'KHOOR ZRUOG')",
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "description": "rot13 (default), caesar, vigenere, atbash, rail_fence, analyze"
                },
                "text": { "type": "string", "description": "Input text. Also 'input'/'message'." },
                "shift": { "type": "number", "description": "Shift amount for Caesar cipher (default 3)" },
                "key": { "type": "string", "description": "Keyword for Vigenère cipher (alphabetic only)" },
                "rails": { "type": "number", "description": "Number of rails for rail fence cipher (default 3, min 2)" },
                "decode": { "type": "boolean", "description": "Decode mode (default false = encode)" }
            },
            "required": []
        }),
    ));
    tools.push(make_tool(
        "nato_tools",
        "NATO phonetic alphabet encoding/decoding and Morse code translation without external utilities. \
         Actions: nato (default — convert text to NATO phonetic words; 'text'; e.g. SOS → Sierra Oscar Sierra), \
         from_nato (convert NATO phonetic words back to text; 'text' containing words like 'Alpha Bravo Charlie'), \
         morse (encode text to Morse dots/dashes OR decode Morse back to text; 'text'; \
         auto-detects direction from content; 'decode: true' to force decode; \
         letter separator is space, word separator is /), \
         spell (spell out text letter-by-letter with NATO equivalents; 'text'; optional 'separator'). \
         Covers all 26 letters, digits 0–9, and common punctuation in Morse mode. \
         Example: nato_tools(text:'SOS') or \
         nato_tools(action:'morse', text:'SOS') or \
         nato_tools(action:'from_nato', text:'Sierra Oscar Sierra')",
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "description": "nato (default), from_nato, morse, spell"
                },
                "text": { "type": "string", "description": "Input text. Also 'input'/'message'." },
                "decode": { "type": "boolean", "description": "Force decode mode for morse action" },
                "separator": { "type": "string", "description": "Separator between spelled-out letters in spell action (default ', ')" }
            },
            "required": []
        }),
    ));
    tools.push(make_tool(
        "geo_tools",
        "Geographic coordinate calculations without external utilities. \
         Actions: distance (default — Haversine great-circle distance between two points; \
         'lat1','lng1','lat2','lng2'; returns km, miles, nautical miles, and bearing), \
         bearing ('lat1','lng1','lat2','lng2' — initial and back bearing in degrees with compass point), \
         midpoint (geographic centroid of 2+ points; 'lat1'/'lng1'/'lat2'/'lng2' or 'points' array of [lat,lng]), \
         dms (decimal degrees ↔ DMS conversion; decimal→DMS: 'lat'+'lng'; DMS→decimal: 'lat_d'/'lat_m'/'lat_s'/'lat_dir' + 'lng_d'/'lng_m'/'lng_s'/'lng_dir'), \
         bbox (bounding box of a set of points; 'points' array → N/S/E/W bounds, center, dimensions in km), \
         destination (destination point given origin, distance, and bearing; 'lat','lng','distance' km, 'bearing' degrees). \
         Example: geo_tools(action:'distance', lat1:51.5074, lng1:-0.1278, lat2:48.8566, lng2:2.3522)",
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": { "type": "string", "description": "distance (default), bearing, midpoint, dms, bbox, destination" },
                "lat1": { "type": "number", "description": "Latitude of point 1 (decimal degrees, -90 to 90)" },
                "lng1": { "type": "number", "description": "Longitude of point 1 (decimal degrees, -180 to 180)" },
                "lat2": { "type": "number", "description": "Latitude of point 2" },
                "lng2": { "type": "number", "description": "Longitude of point 2" },
                "lat": { "type": "number", "description": "Latitude for dms/destination actions" },
                "lng": { "type": "number", "description": "Longitude for dms/destination actions" },
                "lat_d": { "type": "number", "description": "Degrees for DMS→decimal lat" },
                "lat_m": { "type": "number", "description": "Minutes for DMS→decimal lat" },
                "lat_s": { "type": "number", "description": "Seconds for DMS→decimal lat" },
                "lat_dir": { "type": "string", "description": "N or S for DMS→decimal lat" },
                "lng_d": { "type": "number", "description": "Degrees for DMS→decimal lng" },
                "lng_m": { "type": "number", "description": "Minutes for DMS→decimal lng" },
                "lng_s": { "type": "number", "description": "Seconds for DMS→decimal lng" },
                "lng_dir": { "type": "string", "description": "E or W for DMS→decimal lng" },
                "points": { "type": "array", "description": "Array of [lat, lng] pairs for midpoint/bbox actions" },
                "distance": { "type": "number", "description": "Distance in km for destination action" },
                "bearing": { "type": "number", "description": "Bearing in degrees (0=N, 90=E) for destination action" }
            },
            "required": []
        }),
    ));
    tools.push(make_tool(
        "data_gen_tools",
        "Generate test/mock data without external utilities. \
         Actions: lorem (default — Lorem ipsum text; 'count', 'unit': words/sentences/paragraphs, optional 'seed'), \
         name (random person names; 'count', optional 'seed'), \
         email (random email addresses; 'count', optional 'domain', optional 'seed'), \
         numbers (random numbers in range; 'count', 'min', 'max'; 'float: true' for decimals with 'decimals' precision; \
         optional 'separator'), \
         dates (random dates in range; 'count', 'from' YYYY-MM-DD, 'to' YYYY-MM-DD; \
         'format': iso (default)/us/eu/long), \
         id (generate IDs; 'count', 'kind': seq/hex/uuid; \
         seq: 'prefix', 'start', 'pad'; optional 'seed' for hex/uuid). \
         All actions accept optional 'seed' for reproducible output. \
         Example: data_gen_tools(action:'lorem', count:2, unit:'paragraphs') or \
         data_gen_tools(action:'name', count:5) or \
         data_gen_tools(action:'id', kind:'uuid', count:3)",
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": { "type": "string", "description": "lorem (default), name, email, numbers, dates, id" },
                "count": { "type": "integer", "description": "How many items to generate (default 5 or 50 for lorem)" },
                "unit": { "type": "string", "description": "For lorem: words (default), sentences, paragraphs" },
                "seed": { "type": "integer", "description": "Random seed for reproducible output (default 42)" },
                "domain": { "type": "string", "description": "Fixed email domain override for email action" },
                "min": { "type": "number", "description": "Minimum value for numbers action (default 1)" },
                "max": { "type": "number", "description": "Maximum value for numbers action (default 100)" },
                "float": { "type": "boolean", "description": "Generate floating-point numbers (default false)" },
                "decimals": { "type": "integer", "description": "Decimal places for float numbers (default 2)" },
                "separator": { "type": "string", "description": "Output separator for numbers action (default newline)" },
                "from": { "type": "string", "description": "Start date YYYY-MM-DD for dates action (default 2000-01-01)" },
                "to": { "type": "string", "description": "End date YYYY-MM-DD for dates action (default 2024-12-31)" },
                "format": { "type": "string", "description": "Date format: iso (default), us (MM/DD/YYYY), eu (DD.MM.YYYY), long" },
                "kind": { "type": "string", "description": "ID kind: seq (default), hex, uuid" },
                "prefix": { "type": "string", "description": "ID prefix string for seq/hex/uuid kind" },
                "start": { "type": "integer", "description": "Starting number for seq IDs (default 1)" },
                "pad": { "type": "integer", "description": "Zero-pad width for seq IDs (default 0 = no padding)" }
            },
            "required": []
        }),
    ));
    tools.push(make_tool(
        "unit_tools",
        "Convert values between units of measurement across 13 categories without external utilities. \
         Actions: convert (default — 'value' number, 'from' unit name/symbol, optional 'to' target unit; \
         omit 'to' to see all conversions in the same category), \
         list (all supported units; optional 'category' filter), \
         categories (all 13 categories with unit counts). \
         Categories: length (18 units), mass (13), temperature (4: Celsius/Fahrenheit/Kelvin/Rankine), \
         area (10), volume (17), speed (6), energy (11), power (8), pressure (9), time (12), \
         angle (6), fuel (4), frequency (5). \
         Temperature uses affine conversion (F→C: (F-32)×5/9, K→C: K-273.15). \
         Examples: unit_tools(action:'convert', value:100, from:'km', to:'miles') or \
         unit_tools(action:'convert', value:98.6, from:'fahrenheit', to:'celsius') or \
         unit_tools(action:'convert', value:1, from:'horsepower') — omit 'to' for all power units",
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": { "type": "string", "description": "convert (default), list, categories" },
                "value": { "type": "number", "description": "The numeric value to convert (required for convert)" },
                "amount": { "type": "number", "description": "Alias for value" },
                "from": { "type": "string", "description": "Source unit name, symbol, or alias (e.g. 'km', 'kilometre', 'fahrenheit')" },
                "to": { "type": "string", "description": "Target unit name/symbol (optional — omit to show all conversions in the category)" },
                "category": { "type": "string", "description": "Category filter for list action (e.g. 'temperature', 'energy')" }
            },
            "required": []
        }),
    ));
    tools.push(make_tool(
        "geometry_tools",
        "Compute geometric properties (area, volume, perimeter, triangle solver, circle math) without external utilities. \
         Actions: area (default — 'shape' + dimensions → area; shapes: rectangle/width+height, square/side, \
         circle/radius, ellipse/a+b, triangle/base+height or a+b+c, trapezoid/a+b+height, \
         parallelogram/base+height, rhombus/d1+d2, regular_polygon/sides+side_length, sector/radius+angle), \
         volume ('shape' + dimensions → volume and surface area; shapes: cube/side, rectangular_prism/width+height+depth, \
         sphere/radius, hemisphere/radius, cylinder/radius+height, cone/radius+height, pyramid/base_area+height, \
         torus/major_radius+minor_radius), \
         perimeter (same shapes as area), \
         triangle (SSS solver: 'a','b','c' sides → angles, area, perimeter, inradius, circumradius, type; \
         or SAS: 'a','b','angle_c' → third side then full solve), \
         circle (comprehensive: provide any one of radius/diameter/circumference/area; \
         optional 'angle' in degrees for arc length, sector area, chord length). \
         Examples: geometry_tools(action:'area', shape:'circle', radius:5) or \
         geometry_tools(action:'triangle', a:3, b:4, c:5) or \
         geometry_tools(action:'circle', radius:10, angle:90)",
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": { "type": "string", "description": "area (default), volume, perimeter, triangle, circle" },
                "shape": { "type": "string", "description": "Shape name for area/volume/perimeter (e.g. 'circle', 'rectangle', 'sphere')" },
                "radius": { "type": "number", "description": "Radius for circle/sphere/cylinder/cone/sector" },
                "diameter": { "type": "number", "description": "Diameter (circle action)" },
                "circumference": { "type": "number", "description": "Circumference (circle action)" },
                "area": { "type": "number", "description": "Area (circle action — compute radius from area)" },
                "width": { "type": "number", "description": "Width (rectangle, rectangular_prism)" },
                "height": { "type": "number", "description": "Height (rectangle, trapezoid, cylinder, cone, pyramid)" },
                "depth": { "type": "number", "description": "Depth (rectangular_prism)" },
                "side": { "type": "number", "description": "Side length (square, cube)" },
                "base": { "type": "number", "description": "Base (triangle, parallelogram)" },
                "a": { "type": "number", "description": "Side a (triangle), semi-major axis (ellipse), or parallel side a (trapezoid)" },
                "b": { "type": "number", "description": "Side b (triangle), semi-minor axis (ellipse), or parallel side b (trapezoid)" },
                "c": { "type": "number", "description": "Side c (triangle, trapezoid perimeter)" },
                "d": { "type": "number", "description": "Side d (trapezoid perimeter)" },
                "d1": { "type": "number", "description": "Diagonal 1 (rhombus)" },
                "d2": { "type": "number", "description": "Diagonal 2 (rhombus)" },
                "sides": { "type": "integer", "description": "Number of sides (regular_polygon)" },
                "side_length": { "type": "number", "description": "Side length (regular_polygon)" },
                "angle": { "type": "number", "description": "Central angle in degrees (sector area, arc length; also for circle action)" },
                "angle_c": { "type": "number", "description": "Included angle C in degrees (SAS triangle)" },
                "base_area": { "type": "number", "description": "Base area (pyramid volume)" },
                "major_radius": { "type": "number", "description": "Major radius R (torus)" },
                "minor_radius": { "type": "number", "description": "Minor radius r (torus)" }
            },
            "required": []
        }),
    ));
    tools.push(make_tool(
        "checksum_tools",
        "Compute CRC and checksum values for data integrity verification without external utilities. \
         Actions: all (default — compute all: CRC-8, CRC-16/MODBUS, CRC-32 IEEE, Adler-32, Fletcher-16 in one call), \
         crc8 (CRC-8, polynomial 0x07), \
         crc16 (CRC-16/MODBUS, 0xA001 reflected), \
         crc32 (CRC-32 IEEE 802.3 — same algorithm used by zip and gzip), \
         adler32 (Adler-32 rolling sum — used in zlib/PNG), \
         fletcher16 (Fletcher-16 checksum). \
         Input: 'text' for a UTF-8 string or 'hex' for raw bytes as a hex string. \
         Output: hex, decimal, and binary representations. \
         Examples: checksum_tools(action:'all', text:'Hello, World!') or \
         checksum_tools(action:'crc32', text:'test') or \
         checksum_tools(action:'crc16', hex:'DEADBEEF')",
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": { "type": "string", "description": "all (default), crc8, crc16, crc32, adler32, fletcher16" },
                "text": { "type": "string", "description": "Input as a UTF-8 string" },
                "input": { "type": "string", "description": "Alias for text" },
                "hex": { "type": "string", "description": "Input as hex-encoded bytes (e.g. 'DEADBEEF')" }
            },
            "required": []
        }),
    ));
    tools.push(make_tool(
        "id_tools",
        "Generate and decode time-sortable and compact unique identifiers without external utilities. \
         Actions: ulid (default — ULID: 26-char Crockford base32, time-sortable, lexicographically ordered; \
         'count' up to 100, optional 'seed' for reproducible output), \
         nanoid (compact URL-safe random ID; 'size' chars default 21, optional 'alphabet', 'count'), \
         snowflake (Twitter/Discord-style 64-bit numeric ID; 41-bit timestamp + 10-bit machine ID + 12-bit sequence; \
         optional 'machine_id' 0–1023; 'count'), \
         decode (inspect any ULID or Snowflake — extracts timestamp, machine ID, random component; pass 'id'). \
         ULID properties: monotonic within millisecond, case-insensitive, URL-safe, 128-bit. \
         NanoID properties: smaller than UUID, URL-safe, configurable alphabet. \
         Snowflake properties: 64-bit integer, embeds creation time, sortable. \
         Examples: id_tools(action:'ulid', count:5) or id_tools(action:'nanoid', size:10) or \
         id_tools(action:'decode', id:'01ARZ3NDEKTSV4RRFFQ69G5FAV')",
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": { "type": "string", "description": "ulid (default), nanoid, snowflake, decode" },
                "count": { "type": "integer", "description": "Number of IDs to generate (default 1, max 100)" },
                "n": { "type": "integer", "description": "Alias for count" },
                "seed": { "type": "integer", "description": "Random seed for reproducible output" },
                "size": { "type": "integer", "description": "NanoID character length (default 21)" },
                "alphabet": { "type": "string", "description": "Custom alphabet for NanoID (default: URL-safe 64-char set)" },
                "machine_id": { "type": "integer", "description": "Snowflake machine ID 0–1023 (default 1)" },
                "id": { "type": "string", "description": "ULID or Snowflake ID to decode" },
                "input": { "type": "string", "description": "Alias for id" }
            },
            "required": []
        }),
    ));
    tools.push(make_tool(
        "binary_tools",
        "Bit manipulation, bitfield packing/unpacking, and binary analysis without external utilities. \
         Actions: info (default — show decimal/hex/octal/binary, popcount, parity, leading/trailing zeros, \
         Gray code, IEEE 754 float view; pass 'value' as integer or 0x/0b/0o string; optional 'width' in bits), \
         flags (show each bit position as SET/clear with SET bits highlighted; optional 'names' array for named flags), \
         pack (assemble fields into a packed integer; 'fields' array of {value, bits, name?} from MSB to LSB), \
         unpack (extract named fields from a packed integer; 'value' + 'layout' array of {bits, name?} from MSB to LSB), \
         ops (compute AND/OR/XOR/NOT/NAND/NOR/XNOR, shifts, rotates, popcount, Gray code; \
         'value' for A, optional 'b' for B, optional 'shift' count). \
         Example: binary_tools(action: 'flags', value: '0xFF', width: 8, names: ['b7','b6','b5','b4','b3','b2','b1','b0']) or \
         binary_tools(action: 'pack', fields: [{name: 'version', value: 3, bits: 4}, {name: 'type', value: 1, bits: 4}]) or \
         binary_tools(action: 'unpack', value: '0xABCD', layout: [{name: 'hi', bits: 8}, {name: 'lo', bits: 8}]).",
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "description": "info (default), flags, pack, unpack, ops"
                },
                "value": {
                    "type": ["number", "string"],
                    "description": "Integer value to analyze (decimal, 0x hex, 0b binary, 0o octal). Also 'n'/'input'."
                },
                "b": {
                    "type": ["number", "string"],
                    "description": "Second operand for ops action."
                },
                "width": {
                    "type": "number",
                    "description": "Bit width to display (8, 16, 32, 64). Auto-detected if omitted."
                },
                "shift": {
                    "type": "number",
                    "description": "Shift/rotate amount for ops action (default 1)."
                },
                "names": {
                    "type": "array",
                    "description": "Named flag labels for flags action, MSB first."
                },
                "fields": {
                    "type": "array",
                    "description": "Array of {value, bits, name?} field objects for pack action."
                },
                "layout": {
                    "type": "array",
                    "description": "Array of {bits, name?} field descriptors for unpack action, MSB first."
                }
            },
            "required": []
        }),
    ));
    tools.push(make_tool(
        "ascii_tools",
        "Generate ASCII/Unicode art, box drawings, progress bars, tables, and trees without external utilities. \
         Actions: banner (default — block-letter ASCII art from text; pass 'text', max 30 chars), \
         box (draw a Unicode border box around text lines; 'text' with optional newlines; \
         'style': single/double/rounded/heavy/ascii; optional 'padding' integer), \
         bar (render a fill/progress bar; 'value' required; 'max' default 100, 'width' default 40, \
         'style': block/hash/equals/shade/circle/dot, optional 'label'), \
         table (render a formatted Unicode table; 'headers' string array, 'rows' 2D string array; \
         'style': single/double/heavy/rounded), \
         tree (render a directory-style tree; supply 'root' string + 'nodes' array of {label, children?} objects; \
         OR 'text' indented-outline string with 2-space depth + optional 'root' label). \
         Example: ascii_tools(action: 'box', text: 'Hello!', style: 'double') or \
         ascii_tools(action: 'bar', value: 73, label: 'CPU') or \
         ascii_tools(action: 'table', headers: ['Name','Value'], rows: [['foo','1'],['bar','2']]).",
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "description": "banner (default), box, bar, table, tree"
                },
                "text": {
                    "type": "string",
                    "description": "Text input (required for banner/box; outline for tree). Also 'input'/'label'."
                },
                "style": {
                    "type": "string",
                    "description": "Visual style. box: single/double/rounded/heavy/ascii. bar: block/hash/equals/shade/circle/dot. table: single/double/heavy/rounded."
                },
                "padding": {
                    "type": "number",
                    "description": "Padding spaces inside the box (box action, default 1)."
                },
                "value": {
                    "type": "number",
                    "description": "Current value for bar action. Also 'v'."
                },
                "max": {
                    "type": "number",
                    "description": "Maximum value for bar action (default 100)."
                },
                "width": {
                    "type": "number",
                    "description": "Bar character width (default 40)."
                },
                "label": {
                    "type": "string",
                    "description": "Label prefix for bar action."
                },
                "headers": {
                    "type": "array",
                    "description": "Column header strings for table action."
                },
                "rows": {
                    "type": "array",
                    "description": "2D array of cell values for table action."
                },
                "root": {
                    "type": ["string", "object"],
                    "description": "Root node label string, or root node object for tree action."
                },
                "nodes": {
                    "type": "array",
                    "description": "Top-level child nodes [{label, children?}] for tree action."
                },
                "label_key": {
                    "type": "string",
                    "description": "Object key to use as node label in tree action (default: 'label')."
                },
                "children_key": {
                    "type": "string",
                    "description": "Object key to use as children array in tree action (default: 'children')."
                }
            },
            "required": []
        }),
    ));
    tools.push(make_tool(
        "time_zone_tools",
        "Convert times between timezones, list timezone offsets, and show a world clock without external utilities. \
         Actions: convert (default — convert a datetime from one timezone to another; 'datetime' as ISO 8601 or \
         'YYYY-MM-DD HH:MM:SS', 'from' source timezone abbreviation or numeric offset, 'to' target timezone), \
         list (list all supported timezones with UTC offsets; optional 'filter' string to search by name), \
         offset (get UTC offset for a timezone; 'tz' field), \
         world_clock (show current UTC time converted to multiple zones; 'zones' array of timezone names). \
         Accepts named zones (UTC, GMT, EST, PST, CST, MST, IST, JST, AEST, CET, etc.) and numeric offsets (+05:30, -07:00). \
         Example: time_zone_tools(action: 'convert', datetime: '2024-06-15 14:30:00', from: 'EST', to: 'JST') or \
         time_zone_tools(action: 'world_clock', zones: ['UTC','EST','PST','IST','JST']).",
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "description": "convert (default), list, offset, world_clock"
                },
                "datetime": {
                    "type": "string",
                    "description": "Datetime string to convert (ISO 8601 or 'YYYY-MM-DD HH:MM:SS'). Required for convert."
                },
                "from": {
                    "type": "string",
                    "description": "Source timezone (convert action). Name or numeric offset."
                },
                "to": {
                    "type": "string",
                    "description": "Target timezone (convert action). Name or numeric offset."
                },
                "tz": {
                    "type": "string",
                    "description": "Timezone name or offset for the offset action."
                },
                "zones": {
                    "type": "array",
                    "description": "Array of timezone names for world_clock action."
                },
                "filter": {
                    "type": "string",
                    "description": "Filter substring for list action."
                }
            },
            "required": []
        }),
    ));
    tools.push(make_tool(
        "word_tools",
        "Word frequency analysis, anagram detection, Soundex phonetic matching, palindrome checking, \
         and syllable counting without external utilities. \
         Actions: frequency (default — word frequency table with percentages; 'text', optional 'top' N (default 20), \
         'stop_words' bool to filter common words (default true)), \
         anagram (check if two strings are anagrams; 'a' and 'b' fields), \
         soundex (Soundex phonetic code; 'word' for single, 'words' array, or 'text' for all words; \
         groups phonetically similar words), \
         palindrome (check if text is a palindrome; 'text', optional 'strict' bool; finds longest palindromic substring), \
         syllables (syllable count per word, totals, and Flesch-Kincaid grade estimate; 'text'). \
         Example: word_tools(action: 'anagram', a: 'listen', b: 'silent') or \
         word_tools(action: 'frequency', text: '...', top: 10) or \
         word_tools(action: 'soundex', words: ['Robert','Rupert','Rubin']).",
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "description": "frequency (default), anagram, soundex, palindrome, syllables"
                },
                "text": {
                    "type": "string",
                    "description": "Input text. Required for frequency, palindrome, syllables. Also 'input'."
                },
                "top": {
                    "type": "number",
                    "description": "Max words to show in frequency output (default 20)."
                },
                "stop_words": {
                    "type": "boolean",
                    "description": "Filter common stop words in frequency action (default true)."
                },
                "a": {
                    "type": "string",
                    "description": "First word/phrase for anagram comparison. Also 'word_a'."
                },
                "b": {
                    "type": "string",
                    "description": "Second word/phrase for anagram comparison. Also 'word_b'."
                },
                "word": {
                    "type": "string",
                    "description": "Single word for soundex action."
                },
                "words": {
                    "type": "array",
                    "description": "Array of words for soundex action."
                },
                "strict": {
                    "type": "boolean",
                    "description": "Strict palindrome check (no case/punctuation stripping) for palindrome action."
                }
            },
            "required": []
        }),
    ));
    tools.push(make_tool(
        "string_metric_tools",
        "String distance and similarity metrics — Levenshtein, Damerau-Levenshtein, Jaro, Jaro-Winkler, \
         Hamming, LCS, multi-metric comparison, and fuzzy candidate ranking — without external utilities. \
         Actions: levenshtein (default — edit distance and similarity% between 'a' and 'b'; optional 'case_sensitive' bool), \
         damerau (Damerau-Levenshtein with transposition support; compares to plain Levenshtein), \
         jaro (Jaro similarity 0.0–1.0; 'a' and 'b'), \
         jaro_winkler (Jaro-Winkler with common-prefix boost — best for name matching), \
         hamming (bit-position differences for equal-length strings with position map), \
         lcs (Longest Common Subsequence string, length, and coverage %), \
         similarity (all 4 metrics in one table with average and verdict; 'a' and 'b'), \
         fuzzy (rank a 'candidates' array by composite JW+LCS score against 'query'; optional 'threshold' 0.0–1.0 and 'top' N). \
         Example: string_metric_tools(action: 'similarity', a: 'Robert', b: 'Rupert') or \
         string_metric_tools(action: 'fuzzy', query: 'helo', candidates: ['hello','world','help'])",
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "description": "levenshtein (default), damerau, jaro, jaro_winkler, hamming, lcs, similarity, fuzzy"
                },
                "a": {
                    "type": "string",
                    "description": "First string. Also 's1' or 'source'."
                },
                "b": {
                    "type": "string",
                    "description": "Second string. Also 's2' or 'target'."
                },
                "case_sensitive": {
                    "type": "boolean",
                    "description": "Case-sensitive comparison (levenshtein action, default true)."
                },
                "query": {
                    "type": "string",
                    "description": "Search query for fuzzy action. Also 'a'."
                },
                "candidates": {
                    "type": ["array", "string"],
                    "description": "Array of strings to rank, or newline-separated string (fuzzy action)."
                },
                "threshold": {
                    "type": "number",
                    "description": "Minimum similarity 0.0–1.0 to include in fuzzy results (default 0.0)."
                },
                "top": {
                    "type": "number",
                    "description": "Max results to show in fuzzy action (default 10)."
                }
            },
            "required": []
        }),
    ));
    tools.push(make_tool(
        "calc_tools",
        "Evaluate mathematical expressions, RPN calculations, multi-variable sessions, and numeric sequences \
         without needing run_code or a Python sandbox. \
         Actions: eval (default — infix expression evaluator; 'expr' field; supports +/-/*/÷/^/% operators, \
         parentheses, constants pi/e/tau/phi, and 30+ functions: sqrt, abs, sin, cos, tan, asin, acos, atan, \
         log, ln, log2, exp, floor, ceil, round, factorial, gcd, lcm, min, max, avg, sum, pow, hypot, clamp, \
         choose/nCk, deg, rad; optional 'vars' object for variable substitution like {x: 5, y: 3.14}), \
         rpn (Reverse Polish Notation; 'expr' as space-separated tokens: numbers, +/-/*/^/%/sqrt/abs/neg/dup/swap/drop), \
         variables (sequential statements with assignments; 'statements' array like ['x=5','y=x*2','z=x+y']; \
         each line evaluated in order with accumulated variable context), \
         sequence (generate 'count' terms of an expression using 'n' as index variable; \
         'start'/'step'/'count' control range). \
         Example: calc_tools(expr: '2^10 + factorial(5)') or \
         calc_tools(action: 'sequence', expr: 'n^2 + 1', count: 10) or \
         calc_tools(action: 'rpn', expr: '3 4 + 2 *')",
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "description": "eval (default), rpn, variables, sequence"
                },
                "expr": {
                    "type": "string",
                    "description": "Mathematical expression to evaluate. Also 'expression'/'input'/'text'."
                },
                "vars": {
                    "type": "object",
                    "description": "Variable name→value map for eval action (e.g. {x: 5, r: 3.14})."
                },
                "statements": {
                    "type": ["array", "string"],
                    "description": "Array of assignment/expression statements for variables action."
                },
                "start": {
                    "type": "number",
                    "description": "Starting value of 'n' for sequence action (default 1)."
                },
                "step": {
                    "type": "number",
                    "description": "Step size between terms in sequence action (default 1)."
                },
                "count": {
                    "type": "number",
                    "description": "Number of terms to generate in sequence action (default 10)."
                }
            },
            "required": []
        }),
    ));
    tools.push(make_tool(
        "csp_tools",
        "Parse, explain, validate, and build Content Security Policy (CSP) headers without external utilities. \
         Actions: parse (default — break CSP into directives with per-source descriptions and unsafe flags), \
         explain (plain-English summary of what each directive allows), \
         validate (check for 'unsafe-inline', 'unsafe-eval', wildcard *, missing base-uri/object-src, deprecated report-uri), \
         build (generate a CSP from a 'directives' object or a named 'preset': strict/moderate/api). \
         Strips 'Content-Security-Policy:' prefix automatically. \
         Example: csp_tools(action: 'parse', header: \"default-src 'self'; img-src *\") or \
         csp_tools(action: 'validate', header: \"...\") or \
         csp_tools(action: 'build', preset: 'strict').",
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "description": "parse (default), explain, validate, build"
                },
                "header": {
                    "type": "string",
                    "description": "Raw CSP header value — 'Content-Security-Policy:' prefix is stripped automatically. Also 'policy'/'csp'/'input'."
                },
                "preset": {
                    "type": "string",
                    "description": "Named CSP preset for build action: strict, moderate, or api."
                },
                "directives": {
                    "type": "object",
                    "description": "Directives object for build action: { \"script-src\": [\"'self'\", \"https://cdn.example.com\"] }."
                }
            },
            "required": []
        }),
    ));
    tools.push(make_tool(
        "http_status_tools",
        "Look up, search, and list HTTP status codes — 65 standard codes across all 5 categories, no external utilities. \
         Actions: lookup (default — code number to reason phrase and description; pass 'code' like 404 or '404'), \
         search (keyword search in reason and description; pass 'query'), \
         category (list codes in a category — 1xx/2xx/3xx/4xx/5xx; omit 'category' for a summary with counts), \
         list (all codes or filtered by 'category'). \
         Example: http_status_tools(action: 'lookup', code: 429) or \
         http_status_tools(action: 'category', category: '4xx') or \
         http_status_tools(action: 'search', query: 'redirect').",
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "description": "lookup (default), search, category, list"
                },
                "code": {
                    "type": ["integer", "string"],
                    "description": "HTTP status code number for lookup action — e.g. 404 or '404'. Also 'status'."
                },
                "query": {
                    "type": "string",
                    "description": "Search term for the search action — e.g. 'redirect' or 'rate limit'. Also 'q'."
                },
                "category": {
                    "type": "string",
                    "description": "Category for category/list actions: 1xx, 2xx, 3xx, 4xx, 5xx. Omit for summary."
                }
            },
            "required": []
        }),
    ));
    tools.push(make_tool(
        "http_parse_tools",
        "Parse raw HTTP/1.1 request and response messages — no live requests, static text parsing only. \
         Actions: parse/auto (default — auto-detect request vs response and show all fields), \
         request (force-parse as HTTP request — method, path, query params, headers, body preview), \
         response (force-parse as HTTP response — status code, reason, headers, content analysis, body preview), \
         headers (show all headers with annotations and security header check for responses), \
         cookies (parse Cookie: and Set-Cookie: headers with security flag analysis), \
         auth (analyze Authorization:, WWW-Authenticate:, and API key headers — decodes Basic auth username, \
               shows Bearer/JWT type, parses Digest realm/nonce). \
         Input: 'text'/'http'/'message' for inline HTTP text, or 'file' for a path. \
         Example: http_parse_tools(action: 'parse', text: 'GET / HTTP/1.1\\nHost: example.com') or \
         http_parse_tools(action: 'cookies', text: '...') or \
         http_parse_tools(action: 'auth', file: 'request.txt').",
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "description": "parse (default/auto-detect), request, response, headers, cookies, auth"
                },
                "text": {
                    "type": "string",
                    "description": "Raw HTTP message text. Also accepted as 'http' or 'message'."
                },
                "file": {
                    "type": "string",
                    "description": "Path to a file containing a raw HTTP message."
                }
            },
            "required": []
        }),
    ));
    tools.push(make_tool(
        "jq_tools",
        "jq-inspired JSON path query and filter tool — implements the most useful 80% of jq without the binary. \
         Uses serde_json — no external utilities. \
         Actions: query/get/select (default — evaluate a path expression; pass 'path' or 'q'), \
         keys (list keys of an object or indices of an array at 'path'), \
         values (list values of an object or elements of an array at 'path'), \
         flatten (flatten a nested array; optional 'depth'; navigate with 'path'), \
         map (extract a 'field' from each element of an array at 'path'), \
         filter (keep array elements where 'field' matches 'value' (equality), 'contains' (substring), \
                 'gt'/'lt' (numeric), or 'exists: true/false' (field presence)), \
         count (count elements/keys/characters at 'path'), \
         type (show JSON type and size of value at 'path'). \
         Path syntax: '.' identity, '.field', '.field.nested', '.field[0]', '.field[-1]' (last), \
         '.field[]' iterate array, '.[]' iterate root array, '.a, .b' multi-path. \
         Builtins via pipe: '.field | keys', '.arr | sort', '.arr | unique', '.arr | reverse', \
         '.arr | first', '.arr | last', '.arr | min', '.arr | max', '.arr | add', '.x | length', '.x | type'. \
         Input: 'json' (inline JSON string or value) or 'file' (path to JSON file). \
         Example: jq_tools(action: 'query', json: '[...]', path: '.[0].name') or \
         jq_tools(action: 'filter', file: 'data.json', path: '.users', field: 'age', gt: 30) or \
         jq_tools(action: 'map', file: 'data.json', field: 'name') or \
         jq_tools(action: 'query', json: '...', path: '.items | sort').",
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "description": "query (default), keys, values, flatten, map, filter, count, type"
                },
                "json": {
                    "type": "string",
                    "description": "Inline JSON string or already-parsed JSON value to query."
                },
                "file": {
                    "type": "string",
                    "description": "Path to a JSON file to query."
                },
                "path": {
                    "type": "string",
                    "description": "Path expression — e.g. '.field', '.a.b[0]', '.items[]', '.a, .b'. Also 'q'."
                },
                "field": {
                    "type": "string",
                    "description": "For filter/map: the field name to match or extract from each array element."
                },
                "value": {
                    "type": ["string", "number", "boolean", "null"],
                    "description": "For filter equality: the value to compare against 'field'."
                },
                "contains": {
                    "type": "string",
                    "description": "For filter: keep elements where 'field' contains this substring."
                },
                "gt": {
                    "type": "number",
                    "description": "For filter: keep elements where 'field' (numeric) is greater than this."
                },
                "lt": {
                    "type": "number",
                    "description": "For filter: keep elements where 'field' (numeric) is less than this."
                },
                "exists": {
                    "type": "boolean",
                    "description": "For filter: keep elements where 'field' is present (true) or absent (false)."
                },
                "depth": {
                    "type": "integer",
                    "description": "For flatten: maximum depth to flatten (default: fully flatten)."
                }
            },
            "required": []
        }),
    ));
    tools.push(make_tool(
        "glob_tools",
        "Test, filter, explain, and convert glob patterns without external utilities. \
         Actions: match (test if a single path matches; pass 'pattern' and 'path'), \
         filter (filter a list of paths; pass 'pattern' and 'paths' as JSON array or newline string), \
         explain (tokenize and describe each component of the pattern; pass 'pattern'), \
         convert (show the equivalent regex; pass 'pattern'). \
         Glob syntax: ** matches any depth including separators, * matches a single segment, \
         ? matches one character, [abc] character class, [!abc] negated class. \
         Example: glob_tools(action: 'match', pattern: '**/*.rs', path: 'src/tools/mod.rs') or \
         glob_tools(action: 'filter', pattern: 'src/**/*.ts', paths: ['src/index.ts', 'tests/x.ts']) or \
         glob_tools(action: 'explain', pattern: '**/*.{ts,tsx}').",
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "description": "match, filter, explain (default), convert"
                },
                "pattern": {
                    "type": "string",
                    "description": "Glob pattern to test or explain — e.g. '**/*.rs', 'src/[!_]*.ts'. Also 'glob'/'pat'."
                },
                "path": {
                    "type": "string",
                    "description": "Single path to test against the pattern (for match action). Also 'input'."
                },
                "paths": {
                    "type": ["array", "string"],
                    "description": "Array of paths or newline-delimited string of paths to filter (for filter action). Also 'list'."
                }
            },
            "required": []
        }),
    ));
    tools.push(make_tool(
        "graph_tools",
        "Graph algorithm operations without external utilities. \
         Actions: info (default — node/edge counts, density, degree distribution; pass 'nodes' array and 'edges' array), \
         bfs (breadth-first search; pass 'start'), \
         dfs (depth-first search; pass 'start'), \
         shortest (Dijkstra shortest path; pass 'start' and 'end'), \
         topo (topological sort via Kahn's; detects cycles), \
         cycles (cycle detection — DFS back-edge for directed, union-find for undirected), \
         components (connected components for undirected; Kosaraju SCCs for directed). \
         Pass 'directed: true' for directed graphs (default: undirected). \
         Edges: array of {from, to, weight?} objects or [from, to, weight?] arrays. \
         Example: graph_tools(action: 'shortest', nodes: ['A','B','C'], edges: [{from:'A',to:'B',weight:1},{from:'B',to:'C',weight:2}], start: 'A', end: 'C') or \
         graph_tools(action: 'topo', nodes: ['a','b','c'], edges: [{from:'a',to:'b'},{from:'b',to:'c'}], directed: true).",
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "description": "info (default), bfs, dfs, shortest, topo, cycles, components"
                },
                "nodes": {
                    "type": "array",
                    "description": "Array of node names/IDs as strings. E.g. ['A','B','C']."
                },
                "edges": {
                    "type": "array",
                    "description": "Array of edges as {from, to, weight?} objects or [from, to, weight?] arrays."
                },
                "directed": {
                    "type": "boolean",
                    "description": "True for a directed graph (default: false / undirected)."
                },
                "start": {
                    "type": "string",
                    "description": "Start node for bfs, dfs, and shortest path."
                },
                "end": {
                    "type": "string",
                    "description": "End node for shortest path (Dijkstra)."
                }
            },
            "required": []
        }),
    ));
    tools.push(make_tool(
        "matrix_tools",
        "Linear algebra matrix operations without external utilities. \
         Actions: info (default — shape, rank, trace, determinant, min/max/mean; pass 'matrix'), \
         multiply (matrix multiply A×B; pass 'a' and 'b' as 2D arrays), \
         transpose (flip rows/columns; pass 'matrix'), \
         determinant (det(A); pass 'matrix' — must be square), \
         inverse (A⁻¹ via LU; pass 'matrix' — square and invertible), \
         solve (solve Ax=b via Gaussian elimination; pass 'matrix' for A and 'vector' for b), \
         rank (matrix rank via row reduction; pass 'matrix'). \
         Matrix format: JSON array of arrays e.g. [[1,2],[3,4]]. \
         Example: matrix_tools(action: 'solve', matrix: [[2,1],[5,3]], vector: [1,2]) or \
         matrix_tools(action: 'determinant', matrix: [[1,2,3],[4,5,6],[7,8,9]]).",
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "description": "info (default), multiply, transpose, determinant, inverse, solve, rank"
                },
                "matrix": {
                    "type": "array",
                    "description": "Matrix as a 2D JSON array of number arrays. E.g. [[1,2],[3,4]]."
                },
                "a": {
                    "type": "array",
                    "description": "First matrix for multiply action."
                },
                "b": {
                    "type": "array",
                    "description": "Second matrix for multiply action."
                },
                "vector": {
                    "type": "array",
                    "description": "RHS vector b for solve action. E.g. [8,-11,-3]."
                }
            },
            "required": []
        }),
    ));
    tools.push(make_tool(
        "har_tools",
        "Parse and analyze HTTP Archive (.har) files for web performance analysis without external utilities. \
         Actions: summary (default — entry count, unique domains, errors, total time/size, status distribution, MIME type breakdown), \
         entries (tabular list of all requests: status/method/time/size/URL; 'limit' caps rows, default 25), \
         slowest (top N slowest with per-phase timing: DNS/connect/SSL/send/wait/receive; 'n' default 10), \
         errors (filter 4xx/5xx/network-error entries only with labels), \
         domains (per-domain request count, cumulative time, and total bytes — sorted slowest first), \
         search (filter entries by URL substring; pass 'query' or 'q'). \
         Input: 'har' parsed object, 'json'/'text' JSON string, or 'file' path to a .har file. \
         Example: har_tools(action: 'slowest', file: 'network.har') or \
         har_tools(action: 'errors', file: 'network.har') or \
         har_tools(action: 'domains', file: 'network.har').",
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "description": "summary (default), entries, slowest, errors, domains, search"
                },
                "file": {
                    "type": "string",
                    "description": "Path to a .har file."
                },
                "json": {
                    "type": "string",
                    "description": "HAR content as a JSON string. Also 'text'/'content'."
                },
                "har": {
                    "type": "object",
                    "description": "Pre-parsed HAR JSON object."
                },
                "limit": {
                    "type": "integer",
                    "description": "Max entries to show (for 'entries' action, default 25)."
                },
                "n": {
                    "type": "integer",
                    "description": "How many slowest entries to show (default 10)."
                },
                "query": {
                    "type": "string",
                    "description": "URL substring to filter by (for 'search' action). Also 'q'."
                }
            },
            "required": []
        }),
    ));
    tools.push(make_tool(
        "ical_tools",
        "Parse and inspect iCalendar (.ics) files without external utilities. \
         Actions: parse (default — all VEVENT and VTODO components with title, start/end, location, status, organizer, description), \
         events (same as parse, VEVENT only), \
         todos (VTODO items with due date, status, and priority), \
         info (calendar metadata: iCal version, producer, calendar name, timezone, component counts by type), \
         search (filter events/todos by keyword in any field; pass 'query' or 'q'). \
         Input: 'text'/'ical'/'ics' with iCalendar text, or 'file' with a path to a .ics file. \
         Example: ical_tools(action: 'parse', file: 'calendar.ics') or \
         ical_tools(action: 'todos', file: 'tasks.ics') or \
         ical_tools(action: 'search', file: 'calendar.ics', query: 'standup').",
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "description": "parse (default), events, todos, info, search"
                },
                "file": {
                    "type": "string",
                    "description": "Path to a .ics iCalendar file."
                },
                "text": {
                    "type": "string",
                    "description": "iCalendar content as a string. Also 'ical'/'ics'/'content'."
                },
                "query": {
                    "type": "string",
                    "description": "Keyword to filter events/todos by (for 'search' action). Also 'q'."
                }
            },
            "required": []
        }),
    ));
    tools.push(make_tool(
        "graphviz_tools",
        "Generate and parse DOT language graph descriptions without external utilities. \
         Actions: generate (default — DOT output from 'nodes' + 'edges'; 'directed: true' for digraph; 'name', 'rankdir' TK/LR/RL/BT options), \
         parse (extract nodes and edges from DOT source; pass 'dot' or 'text'), \
         flowchart (sequential flowchart from 'steps' array with optional 'start'/'end' labels), \
         tree (tree DOT from 'root' string + 'children' array of strings or {label, children?} objects). \
         Nodes: strings or {id, label} objects. Edges: {from, to, label?} or [from, to, label?] arrays. \
         Output includes DOT source and render commands (dot -Tpng, dot -Tsvg, dot -Tpdf). \
         Example: graphviz_tools(action: 'generate', directed: true, nodes: ['A','B','C'], edges: [{from:'A',to:'B',label:'step1'},{from:'B',to:'C'}]) or \
         graphviz_tools(action: 'flowchart', steps: ['Input','Validate','Process','Output']).",
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "description": "generate (default), parse, flowchart, tree"
                },
                "nodes": {
                    "type": "array",
                    "description": "Node list — strings or {id, label} objects."
                },
                "edges": {
                    "type": "array",
                    "description": "Edge list — {from, to, label?} objects or [from, to, label?] arrays."
                },
                "directed": {
                    "type": "boolean",
                    "description": "True for directed graph / digraph (default: false)."
                },
                "name": {
                    "type": "string",
                    "description": "Graph name (default: G)."
                },
                "rankdir": {
                    "type": "string",
                    "description": "Layout direction: TB (top-bottom, default), LR, RL, BT."
                },
                "dot": {
                    "type": "string",
                    "description": "DOT source to parse (for 'parse' action). Also 'text'."
                },
                "steps": {
                    "type": "array",
                    "description": "Sequential step labels for 'flowchart' action."
                },
                "root": {
                    "type": "string",
                    "description": "Root node label for 'tree' action."
                },
                "children": {
                    "type": "array",
                    "description": "Children array for 'tree' action — strings or {label, children?} objects."
                }
            },
            "required": []
        }),
    ));
    tools.push(make_tool(
        "mermaid_tools",
        "Generate Mermaid.js diagram syntax without external utilities. Output is a fenced mermaid code block. \
         Actions: flowchart (default — from 'nodes'+'edges' or 'steps'; 'direction': TD/LR/RL/BT), \
         sequence (from 'messages': [{from, to, text, type?}]; type: sync/async/lost), \
         class (from 'classes': [{name, fields?, methods?, relationships?}]), \
         gantt (from 'sections': [{name, tasks: [{name, start, duration, status?}]}]; 'title', 'date_format'), \
         pie (from 'data': {label: value}; 'title'), \
         er (from 'entities' + 'relationships': [{left, right, cardinality, label}]). \
         Paste output into GitHub, GitLab, Notion, Obsidian, or mermaid.live. \
         Example: mermaid_tools(action: 'sequence', messages: [{from:'Client',to:'API',text:'POST /login'},{from:'API',to:'Client',text:'200 OK',type:'async'}]) or \
         mermaid_tools(action: 'pie', title: 'Traffic', data: {\"API\": 45, \"Web\": 35, \"Mobile\": 20}).",
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "description": "flowchart (default), sequence, class, gantt, pie, er"
                },
                "steps": {
                    "type": "array",
                    "description": "Sequential step labels for flowchart shorthand."
                },
                "nodes": {
                    "type": "array",
                    "description": "Node list for flowchart — strings or {id, label, shape?} objects."
                },
                "edges": {
                    "type": "array",
                    "description": "Edge list for flowchart — {from, to, label?, style?} or [from, to, label?] arrays."
                },
                "direction": {
                    "type": "string",
                    "description": "Flowchart direction: TD (default), LR, RL, BT."
                },
                "messages": {
                    "type": "array",
                    "description": "Sequence diagram messages — [{from, to, text, type?}]. type: sync/async/lost."
                },
                "classes": {
                    "type": "array",
                    "description": "Class diagram classes — [{name, fields?, methods?, relationships?}]."
                },
                "sections": {
                    "type": "array",
                    "description": "Gantt chart sections — [{name, tasks: [{name, start, duration, status?}]}]."
                },
                "entities": {
                    "type": "array",
                    "description": "ER diagram entities — [{name, attributes?}]."
                },
                "relationships": {
                    "type": "array",
                    "description": "ER relationships — [{left, right, cardinality, label}]."
                },
                "data": {
                    "type": "object",
                    "description": "Pie chart data — {label: value} object."
                },
                "title": {
                    "type": "string",
                    "description": "Chart/diagram title."
                }
            },
            "required": []
        }),
    ));
    tools.push(make_tool(
        "hex_tools",
        "Hex dump, binary analysis, and hex encoding/decoding without external utilities. \
         Actions: \
         `dump` (default) — xxd-style hex dump with offset, hex bytes, and ASCII sidebar; \
            'width' bytes per row (default 16); 'limit' max bytes (default 4096); 'offset' starting offset; \
         `strings` — extract printable ASCII strings from binary data; \
            'min' minimum string length (default 4); 'max' max results (default 200); \
         `bytes` — byte frequency histogram, null count, high-byte count, Shannon entropy, top-8 bytes; \
         `analyze` — magic byte file type detection (30+ formats) + entropy estimate from first 256 bytes; \
         `to-hex` — encode bytes or text to a hex string; 'sep' separator (default space); 'upper: true'; \
         `from-hex` — decode a hex string back to bytes; attempts UTF-8 interpretation. \
         Pass 'file' for a file path, 'hex' for an existing hex string, or 'text'/'input' for UTF-8 text.",
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "description": "Operation: dump (default), strings, bytes, analyze, to-hex, from-hex."
                },
                "file": {
                    "type": "string",
                    "description": "Path to a file to analyze (relative or absolute)."
                },
                "hex": {
                    "type": "string",
                    "description": "Hex string input (for from-hex or as source bytes)."
                },
                "text": {
                    "type": "string",
                    "description": "UTF-8 text input (treated as bytes). Also accepted as 'input'."
                },
                "width": {
                    "type": "integer",
                    "description": "Bytes per row in 'dump' output (default 16)."
                },
                "limit": {
                    "type": "integer",
                    "description": "Maximum bytes to show in 'dump' (default 4096)."
                },
                "offset": {
                    "type": "integer",
                    "description": "Starting byte offset for 'dump' address column (default 0)."
                },
                "min": {
                    "type": "integer",
                    "description": "Minimum printable string length for 'strings' action (default 4)."
                },
                "sep": {
                    "type": "string",
                    "description": "Separator between hex bytes in 'to-hex' output (default space). Use '' for compact."
                },
                "upper": {
                    "type": "boolean",
                    "description": "Use uppercase hex digits in 'to-hex' output (default false)."
                }
            },
            "required": []
        }),
    ));
    tools.push(make_tool(
        "ini_tools",
        "Parse, query, validate, and convert INI/config files without external utilities. \
         Handles standard INI format: [section] headers, key=value pairs, ; and # comments, \
         inline comments, global keys before any section, and colon-separated key: value syntax. \
         Actions: \
         `parse` (default) — display all sections and key-value pairs with counts; \
         `get` — retrieve a specific value; pass 'key' as 'section.key' dot notation or \
            separate 'section' + 'key' args; \
         `sections` — list all section names with their key counts; \
         `keys` — list all keys in a section; pass 'section' to scope (omit for global keys); \
         `validate` — check for duplicate keys, duplicate section names, and empty sections; \
         `to-json` — convert the full INI document to a JSON object (sections become nested objects); \
         `to-toml` — convert the INI document to TOML format. \
         Pass 'text' or 'ini' for inline INI content, or 'file' for a file path. \
         Example: ini_tools(action: 'get', file: 'config.ini', key: 'database.host') or \
         ini_tools(action: 'to-json', text: '[server]\\nport=8080\\nhost=localhost').",
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "description": "Operation: parse (default), get, sections, keys, validate, to-json, to-toml."
                },
                "text": {
                    "type": "string",
                    "description": "Inline INI content. Also accepted as 'ini' or 'input'."
                },
                "file": {
                    "type": "string",
                    "description": "Path to an INI/config file (relative or absolute)."
                },
                "key": {
                    "type": "string",
                    "description": "Key to retrieve for 'get'. Use 'section.key' dot notation or pair with 'section'."
                },
                "section": {
                    "type": "string",
                    "description": "Section name for 'get' and 'keys'. Omit for global (no-section) keys."
                }
            },
            "required": []
        }),
    ));
    tools.push(make_tool(
        "template_gen",
        "Generate boilerplate files from built-in templates. \
         Supports Dockerfiles (Node.js, Python, Rust, Go multi-stage), \
         GitHub Actions CI workflows, .gitignore files, .env.example, \
         Makefiles, docker-compose.yml, .pre-commit-config.yaml, .editorconfig, \
         Dependabot config, CODEOWNERS, PR template, and GitHub issue templates. \
         Use template='list' to see all 23 available templates. \
         Writes the file to the workspace root (or output path). Won't overwrite existing files. \
         Accepts substitution variables: project_name, port, node_version, python_version, rust_version, go_version, owner. \
         Example: template_gen(template: 'dockerfile-rust', project_name: 'my-server', port: '8080') \
         or template_gen(template: 'ci-github-rust') to scaffold a full CI pipeline.",
        serde_json::json!({
            "type": "object",
            "properties": {
                "template": {
                    "type": "string",
                    "description": "Template name. Use 'list' to see all available templates."
                },
                "output": {
                    "type": "string",
                    "description": "Output file path relative to workspace root. Defaults to the template's canonical filename."
                },
                "dry_run": {
                    "type": "boolean",
                    "description": "Preview the generated content without writing to disk (default false)."
                },
                "project_name": {
                    "type": "string",
                    "description": "Project name substituted into templates (default 'my-app')."
                },
                "port": {
                    "type": "string",
                    "description": "Port number for Dockerfile/docker-compose templates (default '3000')."
                },
                "node_version": {
                    "type": "string",
                    "description": "Node.js version for Node templates (default '20')."
                },
                "python_version": {
                    "type": "string",
                    "description": "Python version for Python templates (default '3.12')."
                },
                "rust_version": {
                    "type": "string",
                    "description": "Rust toolchain version for Rust templates (default '1.82')."
                },
                "go_version": {
                    "type": "string",
                    "description": "Go version for Go templates (default '1.23')."
                },
                "owner": {
                    "type": "string",
                    "description": "GitHub username/team for CODEOWNERS template (default 'your-team')."
                }
            },
            "required": ["template"]
        }),
    ));
    tools.push(make_tool(
        "env_diff",
        "Compare environment variables between two .env files, or between a .env file and the live process environment. \
         Shows additions (+), removals (-), and changed values (~) with secret values automatically redacted. \
         With no arguments, auto-detects .env files in the workspace root (compares .env vs .env.local if both exist, \
         or .env vs process env if only one exists). \
         Useful for debugging CI vs local discrepancies, staging vs production config drift, \
         and validating .env changes before deploying. \
         Example: env_diff(file_a: '.env', file_b: '.env.production') or env_diff(file_a: '.env') to compare against process.",
        serde_json::json!({
            "type": "object",
            "properties": {
                "file_a": {
                    "type": "string",
                    "description": "First .env file path (relative to workspace root or absolute). Auto-detected if omitted."
                },
                "file_b": {
                    "type": "string",
                    "description": "Second .env file path. If omitted with file_a present, compares file_a against the live process environment."
                }
            }
        }),
    ));
    tools.push(make_tool(
        "port_check",
        "Test whether a TCP port is reachable on a given host. \
         Returns OPEN or CLOSED/FILTERED with the resolved IP, response time, and port service annotation. \
         Annotates 40+ well-known ports (SSH, HTTP, HTTPS, MySQL, PostgreSQL, Redis, MongoDB, \
         Elasticsearch, LM Studio, Ollama, Jupyter, Kubernetes, RDP, etc.). \
         Includes actionable hints when a port is closed (how to start the service, config to check). \
         Example: port_check(host: 'localhost', port: 5432) — test if PostgreSQL is up. \
         Default host is localhost, default timeout 3000ms.",
        serde_json::json!({
            "type": "object",
            "properties": {
                "host": {
                    "type": "string",
                    "description": "Hostname or IP address to test (default 'localhost')."
                },
                "port": {
                    "type": "integer",
                    "description": "TCP port number to test (required)."
                },
                "timeout_ms": {
                    "type": "integer",
                    "description": "Connection timeout in milliseconds (default 3000)."
                }
            },
            "required": ["port"]
        }),
    ));
    tools.push(make_tool(
        "dependency_audit",
        "Audit project dependencies for version pinning, wildcard versions, deprecated packages, \
         and missing lock files. Supports Rust (Cargo.toml), Node.js (package.json), \
         Python (requirements.txt / pyproject.toml), and Go (go.mod). \
         Flags unpinned wildcards, known deprecated packages, missing lock files, \
         and outdated major versions of popular libraries. \
         No network required — reads local manifest files only. \
         For CVE scanning, follow up with `cargo audit`, `npm audit`, or `safety check`.",
        serde_json::json!({
            "type": "object",
            "properties": {}
        }),
    ));
    tools.push(make_tool(
        "code_metrics",
        "Analyze codebase health metrics: total lines, code lines, comment lines, blank lines, \
         TODO/FIXME counts, test file ratio, and language breakdown by file extension. \
         Reports the 10 largest files by line count and a test coverage proxy (% of code in test files). \
         Skips build artifacts, binaries, node_modules, vendor, target, and .git. \
         Useful for codebase hygiene checks, onboarding new contributors, or tracking technical debt. \
         Example: code_metrics() for the whole workspace, code_metrics(path: 'src') for a subdirectory.",
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Subdirectory to analyze relative to workspace root (default '.' for entire workspace)."
                }
            }
        }),
    ));
    tools.push(make_tool(
        "secret_scanner",
        "Scan the workspace (or a given subdirectory) for accidentally committed secrets, \
         API keys, tokens, passwords, and credentials. \
         Detects 14 secret patterns: AWS keys, GitHub tokens, Stripe keys, Slack webhooks, \
         private key blocks, generic API keys, database URLs, bearer tokens, password literals, \
         Twilio keys, SendGrid keys, Heroku API keys, and more. \
         Skips binary files, lock files, build artifacts, and obvious placeholder values. \
         Reports findings grouped by file with line numbers, secret type, and a redacted snippet. \
         Includes actionable remediation steps (rotate credentials, .gitignore, git filter-repo).",
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Subdirectory to scan relative to workspace root (default '.' for entire workspace)."
                }
            }
        }),
    ));
    tools.push(make_tool(
        "plist_tools",
        "Parse, inspect, query, validate, and convert Apple Property List (plist) XML files — Info.plist, preference files, iOS/macOS app metadata — without external utilities. \
         Actions: \
         `parse` (default) — display the plist as a human-readable indented tree; highlights Notable Keys for Info.plist files (bundle ID, version, min OS, ATS, permission strings); \
         `get` — navigate to any value by dot-path (e.g. 'NSAppTransportSecurity.NSAllowsArbitraryLoads' or 'UIBackgroundModes[0]'); \
         `keys` — list top-level keys (or keys at a nested path) with type and value preview; \
         `validate` — check for common Info.plist issues: missing CFBundleIdentifier/CFBundleVersion, NSAllowsArbitraryLoads=true, permission booleans without UsageDescription; \
         `to-json` — convert the plist to pretty-printed JSON. \
         Accepts 'text'/'plist'/'xml' for inline plist XML or 'file' for a file path. \
         Example: plist_tools(action: 'parse', file: 'MyApp/Info.plist') or \
         plist_tools(action: 'get', file: 'Info.plist', path: 'CFBundleVersion') or \
         plist_tools(action: 'validate', file: 'Info.plist') or \
         plist_tools(action: 'to-json', text: '<plist version=\"1.0\">...</plist>').",
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "description": "parse (default), get, keys, validate, to-json"
                },
                "file": {
                    "type": "string",
                    "description": "Path to a plist XML file (e.g. 'Info.plist', 'MyApp/Info.plist')."
                },
                "text": {
                    "type": "string",
                    "description": "Inline plist XML string. Also accepted as 'plist' or 'xml'."
                },
                "path": {
                    "type": "string",
                    "description": "Dot-path for the 'get' and 'keys' actions (e.g. 'NSAppTransportSecurity' or 'UIBackgroundModes[0]')."
                }
            },
            "required": []
        }),
    ));
    tools.push(make_tool(
        "bencode_tools",
        "Parse, decode, and inspect BitTorrent bencode format — .torrent files, BitTorrent protocol messages — without external utilities. \
         Actions: \
         `decode` (default) — decode and display bencode in a human-readable indented tree; shows type annotations for integers and binary blobs; \
         `info` — parse as a .torrent file and show a structured summary: name, file count, total size, piece size, piece count, tracker, creator, creation date, comment; \
         `files` — list all files in a torrent: path, size, and cumulative offset (multi-file) or single-file name and size; \
         `trackers` — list all tracker URLs from 'announce' and 'announce-list', grouped by tier, with UDP/HTTP/HTTPS distinction and unique domain count. \
         Accepts 'file' (path to .torrent/.bencode file), 'hex' (hex-encoded bencode bytes), or 'text' (raw bencode string). \
         Example: bencode_tools(action: 'info', file: 'download.torrent') or \
         bencode_tools(action: 'files', file: 'archive.torrent') or \
         bencode_tools(action: 'trackers', file: 'movie.torrent') or \
         bencode_tools(action: 'decode', hex: '6934 3265').",
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "description": "decode (default), info, files, trackers"
                },
                "file": {
                    "type": "string",
                    "description": "Path to a .torrent or .bencode file."
                },
                "hex": {
                    "type": "string",
                    "description": "Hex-encoded bencode bytes (whitespace ignored)."
                },
                "text": {
                    "type": "string",
                    "description": "Raw bencode string (for ASCII-safe portions). Also accepted as 'input'."
                }
            },
            "required": []
        }),
    ));
    tools.push(make_tool(
        "printf_tools",
        "Analyze, simulate, validate, and convert C-style printf format strings without external utilities. \
         Actions: \
         `explain` (default) — parse all format specifiers (%d, %s, %f, etc.) with type, width, precision, flags, and plain-English meaning; warns on dangerous %n; \
         `simulate` — render the format string with a provided args array and return the formatted output; \
         `validate` — check for %n usage, unknown specifiers, arg count mismatches, and null bytes; \
         `convert` — translate to Python % formatting, Python f-string, Rust format!, Go fmt.Sprintf, and JavaScript template literal. \
         Pass 'format' for the format string; 'args' as JSON array for simulate. \
         Example: printf_tools(action: 'explain', format: '%-10s %5.2f') or \
         printf_tools(action: 'simulate', format: 'Hello %s, count: %d', args: ['world', 42]) or \
         printf_tools(action: 'convert', format: '%05.2f').",
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "description": "explain (default), simulate, validate, convert"
                },
                "format": {
                    "type": "string",
                    "description": "The printf format string to analyze (e.g. '%-10s %5.2f' or 'Error %d: %s\\n')."
                },
                "args": {
                    "type": "array",
                    "description": "Argument values for simulate action (JSON array matching the format specifiers)."
                }
            },
            "required": []
        }),
    ));
    tools.push(make_tool(
        "ascii_chart_tools",
        "Renders ASCII/Unicode charts (bar, line, scatter, sparkline) from numeric data arrays in the terminal without external utilities. \
         Actions: \
         `bar` (default) — vertical bar chart with per-bar labels, optional title, configurable width and fill style (block/hash/equals/dot/shade); handles negative values; \
         `line` — time-series line chart with Y-axis scale labels and optional connected dots; \
         `scatter` — XY scatter plot from separate x/y arrays or [[x,y]] pairs; \
         `sparkline` — compact one-row Unicode sparkline (▁▂▃▄▅▆▇█) for inline trend visualization; \
         `hbar` — alias for bar. \
         Pass 'data' as a JSON number array or comma-separated string for bar/line/sparkline; \
         'x' and 'y' arrays (or 'data' as [[x,y]] pairs) for scatter. \
         Example: ascii_chart_tools(action: 'bar', data: [10,25,15,40,30], labels: ['Mon','Tue','Wed','Thu','Fri'], title: 'Daily Sales') or \
         ascii_chart_tools(action: 'sparkline', data: [1,3,2,8,4,6,5]) or \
         ascii_chart_tools(action: 'line', data: [1,4,9,16,25,36], title: 'Squares') or \
         ascii_chart_tools(action: 'scatter', x: [1,2,3,4,5], y: [2.1,3.9,6.2,7.8,10.1]).",
        crate::tools::ascii_chart_tools::ascii_chart_schema()["parameters"].clone(),
    ));
    tools.push(make_tool(
        "sql_format_tools",
        "Format, minify, split, and extract from SQL statements without external utilities. \
         Actions: \
         `format` (default) — pretty-print SQL with configurable indentation ('indent' default '  ') and uppercase keywords ('uppercase' default true); handles SELECT/FROM/WHERE/JOIN/GROUP BY/ORDER BY clause formatting, CASE/WHEN/THEN/END blocks, and subquery indentation; \
         `minify` — compact SQL by stripping all whitespace and comments; reports original size, minified size, and % reduction; \
         `split` — split a multi-statement SQL file on semicolons and number each statement block; \
         `extract` — extract specific elements: 'tables' (all table names), 'columns' (SELECT columns), 'aliases' (AS aliases), 'comments' (-- and /* */ comments); pass 'what' arg. \
         Pass 'sql' for inline SQL text or 'file' for a .sql file path. \
         Example: sql_format_tools(action: 'format', sql: 'select id,name from users where active=1 order by name') or \
         sql_format_tools(action: 'minify', sql: '  SELECT  *  FROM  users  WHERE  id = 1') or \
         sql_format_tools(action: 'extract', sql: '...', what: 'tables') or \
         sql_format_tools(action: 'split', file: 'migrations/001.sql').",
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "description": "format (default), minify, split, extract"
                },
                "sql": {
                    "type": "string",
                    "description": "Inline SQL text to process."
                },
                "file": {
                    "type": "string",
                    "description": "Path to a .sql file to read and process."
                },
                "indent": {
                    "type": "string",
                    "description": "Indentation string for format action (default: '  ' — two spaces)."
                },
                "uppercase": {
                    "type": "boolean",
                    "description": "Uppercase SQL keywords in format output (default: true)."
                },
                "what": {
                    "type": "string",
                    "description": "What to extract: tables, columns, aliases, comments (for extract action)."
                }
            },
            "required": []
        }),
    ));
    tools.push(make_tool(
        "totp_tools",
        "Generate, verify, and inspect TOTP/HOTP one-time passwords (RFC 6238 / RFC 4226) without external utilities or cloud calls. \
         Uses HMAC-SHA1 (standard TOTP default), pure Rust implementation. \
         Actions: \
         `generate` (default) — generate the current TOTP code from a base32 secret; shows code, seconds remaining, previous/current/next codes for context; \
         `verify` — check a user-supplied code against the secret with ±1 window tolerance for clock drift; returns VALID/INVALID with window offset; \
         `hotp` — generate HMAC-based OTP from a monotonic counter; 'count' arg generates N consecutive codes (useful for pre-generating backup codes); \
         `info` — explain TOTP/HOTP parameters or parse an otpauth:// URI to show all parameters; \
         `qr` — build the otpauth:// URI for registering a secret in any authenticator app (Google Authenticator, Authy, 1Password, Bitwarden, etc.) + instructions for generating a QR code image. \
         Pass 'secret' as the base32-encoded secret string (from app setup). \
         Optional: 'digits' (default 6), 'period' (default 30s), 'algorithm' (default SHA1), 'time' (Unix timestamp override for testing). \
         Example: totp_tools(secret: 'JBSWY3DPEHPK3PXP') or \
         totp_tools(action: 'verify', secret: 'JBSWY3DPEHPK3PXP', code: '123456') or \
         totp_tools(action: 'hotp', secret: 'JBSWY3DPEHPK3PXP', counter: 0, count: 10) or \
         totp_tools(action: 'qr', secret: 'JBSWY3DPEHPK3PXP', issuer: 'MyApp', label: 'user@example.com').",
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "description": "generate (default), verify, hotp, info, qr"
                },
                "secret": {
                    "type": "string",
                    "description": "Base32-encoded TOTP/HOTP secret (from QR code or app setup)."
                },
                "code": {
                    "type": ["string", "integer"],
                    "description": "The OTP code to verify (for verify action)."
                },
                "counter": {
                    "type": "integer",
                    "description": "Starting counter value for HOTP (default: 0)."
                },
                "count": {
                    "type": "integer",
                    "description": "Number of codes to generate for HOTP (default: 1, max 20)."
                },
                "digits": {
                    "type": "integer",
                    "description": "Number of OTP digits (default: 6)."
                },
                "period": {
                    "type": "integer",
                    "description": "TOTP time step in seconds (default: 30)."
                },
                "algorithm": {
                    "type": "string",
                    "description": "HMAC algorithm: SHA1 (default, only supported)."
                },
                "time": {
                    "type": "integer",
                    "description": "Unix timestamp override for testing (default: current system time)."
                },
                "window": {
                    "type": "integer",
                    "description": "Verify window tolerance in periods (default: 1 = allow ±1 period)."
                },
                "issuer": {
                    "type": "string",
                    "description": "Service name for qr action (e.g. 'GitHub', 'MyApp')."
                },
                "label": {
                    "type": "string",
                    "description": "Account label for qr action (e.g. 'user@example.com')."
                },
                "uri": {
                    "type": "string",
                    "description": "otpauth:// URI to parse (for info action)."
                }
            },
            "required": []
        }),
    ));
    tools.push(make_tool(
        "tar_tools",
        "Inspect uncompressed TAR archives (.tar) without external utilities or shell commands. \
         Detects and reports compressed variants (.tar.gz / .tar.bz2 / .tar.xz / .tar.zst) with the correct shell command to use. \
         Actions: \
         `list` (default) — tabular listing of all archive entries with permissions (drwxr-xr-x), size, modification date, type (file/dir/symlink/fifo), and symlink targets; 'max' caps output; \
         `info` — archive statistics: total entries, file/dir/symlink counts, total content size vs archive size, owner list, oldest/newest entry dates; \
         `find` — filter entries by name substring; pass 'query' (e.g. '.rs', 'README'); \
         `extract` — read a specific text entry from the archive as a UTF-8 string; pass 'entry' with the exact path; limited to 512 KB. \
         Pass 'file' with the path to the .tar archive (required). \
         Example: tar_tools(file: 'project.tar') or \
         tar_tools(action: 'info', file: 'backup.tar') or \
         tar_tools(action: 'find', file: 'archive.tar', query: 'config') or \
         tar_tools(action: 'extract', file: 'archive.tar', entry: 'project/src/main.rs').",
        crate::tools::tar_tools::tar_tools_schema(),
    ));
    tools.push(make_tool(
        "email_tools",
        "Parse and analyze RFC 2822 email files (.eml) without external utilities. \
         Decodes RFC 2047 encoded words (base64 and quoted-printable Subject/From/To lines), \
         parses folded headers, detects MIME structure, and traces the delivery hop chain. \
         Actions: \
         `parse` (default) — summary of key headers (From/To/Subject/Date/Message-ID/DKIM/SPF) with decoded values + body preview; shows hop count; \
         `headers` — full header table; pass 'name' to retrieve a specific header (e.g. 'Subject', 'From', 'Received'); pass 'filter' to narrow by substring; \
         `structure` — MIME part tree showing content types, encodings, sizes, and attachment listing; \
         `trace` — delivery chain from Received: headers in chronological order — from/by servers, timestamps, and additional routing metadata. \
         Pass 'file' (path to .eml) or 'text' (raw email as a string). \
         Example: email_tools(file: 'message.eml') or \
         email_tools(action: 'headers', file: 'message.eml', name: 'Subject') or \
         email_tools(action: 'trace', file: 'delivery.eml') or \
         email_tools(action: 'structure', text: raw_email_string).",
        crate::tools::email_tools::email_tools_schema(),
    ));
    tools.push(make_tool(
        "wasm_tools",
        "Inspect WebAssembly binary (.wasm) modules without external tools. \
         Parses WASM magic bytes, section headers, LEB128 indices, type signatures, \
         import/export records, and debug name section. \
         Actions: \
         `info` (default) — magic, version, section count, total size, import/export count summary; \
         `sections` — all sections with id, name, size in bytes, and byte offset; \
         `imports` — all imported symbols with module name, item name, and kind (func/table/mem/global) plus type signature for functions; \
         `exports` — all exported symbols with name, kind, and index. \
         Pass 'file' (path to .wasm file) or 'hex' (hex-encoded WASM bytes). \
         Example: wasm_tools(file: 'module.wasm') or \
         wasm_tools(action: 'imports', file: 'module.wasm') or \
         wasm_tools(action: 'sections', file: 'module.wasm') or \
         wasm_tools(action: 'exports', hex: '0061736d...').",
        crate::tools::wasm_tools::wasm_tools_schema(),
    ));
    tools.push(make_tool(
        "jsonschema_tools",
        "Inspect and validate JSON Schema documents (draft-07 and compatible) without external tools. \
         Actions: \
         `info` (default) — schema metadata: $schema dialect, $id, title, description, root type, required count, \
         property count, additionalProperties, enum values, $defs count, numeric/string/array constraints, and combiner (allOf/anyOf/oneOf) count; \
         `properties` — tabular list of all properties with type, required flag, and description; \
         `refs` — enumerate all $ref usages, $defs/definitions entries, and $id anchors in the schema tree; \
         `validate` — validate a JSON instance against the schema with JSON Pointer error paths \
         (supports type, required, enum, const, properties, additionalProperties, items, minItems/maxItems, \
         minLength/maxLength, pattern, minimum/maximum, multipleOf, allOf/anyOf/oneOf/not, $ref). \
         Pass 'schema' (inline JSON or file path) or 'schema_file'. \
         For 'validate' also pass 'instance' (inline JSON or file path) or 'instance_file'. \
         Example: jsonschema_tools(schema_file: 'schema.json') or \
         jsonschema_tools(action: 'validate', schema_file: 'schema.json', instance_file: 'data.json') or \
         jsonschema_tools(action: 'properties', schema: '{...}') or \
         jsonschema_tools(action: 'refs', schema_file: 'api.schema.json').",
        crate::tools::jsonschema_tools::jsonschema_tools_schema(),
    ));
    tools.push(make_tool(
        "cbor_tools",
        "Decode and inspect CBOR (Concise Binary Object Representation) binary data without external utilities. \
         Full RFC 7049 / RFC 8949 decoder: unsigned/negative integers, byte strings, text strings, arrays, maps, \
         tagged values (datetime, epoch, bignum, UUID, self-described CBOR, WebAuthn tags, COSE), floats (half/single/double), \
         simple values (bool/null/undefined), indefinite-length collections. \
         Actions: \
         `decode` (default) — human-readable decoded value with type labels, tag name annotations, and format hints \
         for WebAuthn AttestationObject / COSE Key structures; \
         `info` — root type, total bytes, array/map length, string key list, type distribution table; \
         `annotate` — hex dump with per-byte CBOR major-type labels (uint/nint/bytes/text/array/map/tag/float/simple) and field info. \
         Accepts 'hex' (hex-encoded bytes), 'base64' (standard or URL-safe), or 'file' (path to binary file). \
         Example: cbor_tools(hex: 'a2616101616202') or \
         cbor_tools(file: 'payload.cbor') or \
         cbor_tools(action: 'info', hex: '...') or \
         cbor_tools(action: 'annotate', base64: 'omFhAWFiAg==').",
        crate::tools::cbor_tools::cbor_tools_schema(),
    ));
    tools.push(make_tool(
        "msgpack_tools",
        "Decode and inspect MessagePack binary data without external utilities. \
         Full MessagePack spec decoder: nil, bool, positive/negative fixint, fixmap, fixarray, fixstr, \
         uint8/16/32/64, int8/16/32/64, float32/float64, str8/16/32, bin8/16/32, array16/32, map16/32, \
         fixext1/2/4/8/16, ext8/16/32 (with Timestamp ext -1 decoding). \
         Actions: \
         `decode` (default) — human-readable decoded value with nested indentation; \
         `info` — root type, total bytes, array/map length, string key list, type distribution table; \
         `annotate` — hex dump with per-byte MessagePack format-byte labels (fixint, fixmap, fixarray, fixstr, \
         uint8..., int8..., float32/64, bin8..., ext types, nil/bool). \
         Accepts 'hex' (hex-encoded bytes), 'base64' (standard or URL-safe), or 'file' (path to binary file). \
         Example: msgpack_tools(hex: '82a3666f6f01a362617202') or \
         msgpack_tools(file: 'data.msgpack') or \
         msgpack_tools(action: 'info', hex: '...') or \
         msgpack_tools(action: 'annotate', file: 'payload.msgpack').",
        crate::tools::msgpack_tools::msgpack_tools_schema(),
    ));
    tools.push(make_tool(
        "html_tools",
        "Parse and analyze HTML documents without external utilities. \
         Extracts links, images, forms, tables, scripts, and heading structure; \
         validates accessibility and SEO; strips HTML to plain text. \
         Actions: \
         `parse` (default) — document overview: DOCTYPE, title, meta description, element counts, and heading hierarchy; \
         `links` — all anchor elements with href, visible text, link type (ext/int/anchor/mailto), and nofollow flag; \
         `images` — all img tags with src, alt text, and dimensions; flags missing alt attributes; \
         `forms` — all form elements with method, action, enctype, and input field inventory; \
         `tables` — table structure with row/column counts and first-5-row cell preview; \
         `scripts` — external and inline script tags with src, type, and inline byte count; \
         `validate` — accessibility and SEO checks: DOCTYPE, lang, charset, title, viewport, missing alt, h1 count; \
         `text` — strip all HTML tags and return plain text (decodes common entities); \
         `stats` — element counts, unique tag count, max nesting depth, comment count, text bytes. \
         Pass 'html' (inline HTML string) or 'file' (path to .html/.htm file). \
         Example: html_tools(file: 'index.html') or \
         html_tools(action: 'links', file: 'page.html') or \
         html_tools(action: 'validate', html: '<html lang=\"en\">...').",
        crate::tools::html_tools::html_tools_schema(),
    ));
    tools.push(make_tool(
        "vcf_tools",
        "Parse and analyze vCard contact files (.vcf / .vcard) without external utilities. \
         Supports vCard 2.1, 3.0, and 4.0 format. Handles RFC 6350 line unfolding, \
         property parameters (TYPE=WORK,CELL), structured names (N property), \
         and all standard contact fields. \
         Actions: \
         `parse` (default) — full detail per contact: full name, structured name, org, title, \
         all emails/phones/addresses with type labels, URLs, birthday, categories, notes, UID; \
         `list` — compact summary table: name, primary email, primary phone (one line per contact); \
         `search` — filter contacts by keyword across all fields; pass 'query' or 'q'; \
         `to_json` — structured JSON array with all contact fields; \
         `to_csv` — CSV export with columns: Full Name, Given, Family, Organization, Title, \
         Email, Phone, Address, Birthday, Categories, URL. \
         Pass 'vcf'/'text' (inline vCard content) or 'file' (path to .vcf file). \
         Example: vcf_tools(file: 'contacts.vcf') or \
         vcf_tools(action: 'to_csv', file: 'contacts.vcf') or \
         vcf_tools(action: 'search', file: 'contacts.vcf', query: 'smith') or \
         vcf_tools(action: 'to_json', vcf: 'BEGIN:VCARD...').",
        crate::tools::vcf_tools::vcf_tools_schema(),
    ));
    tools.push(make_tool(
        "network_header_tools",
        "Parse and decode raw network protocol headers (IPv4, IPv6, TCP, UDP, ICMP, Ethernet II) from hex bytes \
         without external tools. Decodes all field names, flag bits, checksums, and well-known port annotations. \
         Actions: `parse` (auto-detect protocol from header bytes — checks EtherType, IP version nibble), \
         `ipv4` (header fields: version, IHL, DSCP, length, ID, flags, TTL, protocol, checksum verification, src/dst IP), \
         `ipv6` (version, traffic class, flow label, payload length, next header, hop limit, src/dst address with compression), \
         `tcp` (src/dst port, seq/ack numbers, data offset, flag bits FIN/SYN/RST/PSH/ACK/URG/ECE/CWR, window, checksum, options), \
         `udp` (src/dst port, length, checksum), \
         `icmp` (type, code, checksum, echo ID/seq for Echo/Reply), \
         `ethernet` (dst MAC, src MAC, EtherType annotation). \
         Pass 'hex' with raw header bytes (spaces and colons stripped automatically). \
         Example: network_header_tools(hex: '45 00 00 3c 1c 46 40 00 40 06 ...') or \
         network_header_tools(action: 'tcp', hex: '00 50 00 50 ...').",
        crate::tools::network_header_tools::network_header_tools_schema(),
    ));
    tools.push(make_tool(
        "tlv_tools",
        "Parse, decode, and build Type-Length-Value (TLV) encoded binary data without external tools. \
         Supports generic TLV with configurable field sizes, ASN.1 BER/DER, DHCP options (RFC 2132), \
         and 802.11 Wi-Fi information elements. \
         Actions: `parse` (generic TLV — configurable 'type_size' 1/2/4 bytes, 'length_size' 1/2/4 bytes, \
         'endian' big/little; decodes each triplet with hex, ASCII, and integer representations), \
         `ber` (ASN.1 BER/DER — variable-length tag and length, recursive SEQUENCE/SET expansion, \
         INTEGER/BOOLEAN/NULL/OID/STRING value decoding), \
         `dhcp` (DHCP options per RFC 2132 — known option names, IP address formatting, lease time, DHCP message type), \
         `wifi` (802.11 information elements — SSID, Supported Rates, channel, ERP, RSN/WPA2, Vendor Specific OUI), \
         `build` (assemble TLV bytes from 'items' JSON array of {type, value_hex?, value_string?, value_u8?} objects). \
         Pass 'hex' with raw bytes (spaces/colons stripped). \
         Example: tlv_tools(hex: '01 04 c0 a8 01 01') or \
         tlv_tools(action: 'ber', hex: '300a020100020101060103') or \
         tlv_tools(action: 'dhcp', hex: '3501013604c0a80001ff') or \
         tlv_tools(action: 'build', items: [{type: 1, value_hex: 'c0a80101'}]).",
        crate::tools::tlv_tools::tlv_tools_schema(),
    ));
    tools.push(make_tool(
        "bin_pack_tools",
        "Pack and unpack binary data using struct-style format strings (similar to Python's struct module) \
         without external utilities. \
         Format string: optional endian prefix < (little-endian) or > (big-endian, default), then field specifiers: \
         b/B (int8/uint8), h/H (int16/uint16), i/I (int32/uint32), q/Q (int64/uint64), \
         f (float32), d (float64), s (length-prefixed UTF-8 string), x (pad byte). Repeat counts allowed: '4B' = four uint8. \
         Actions: `pack` (values array → hex bytes, shows per-field offset and hex dump), \
         `unpack` (hex bytes → typed field values with per-field breakdown), \
         `describe` (explain each field in the format string with type, size, and endian), \
         `size` (total fixed byte size of the format, or minimum size for variable string fields). \
         Optional 'names' array maps positional field names. \
         Example: bin_pack_tools(format: '<HI', action: 'pack', values: [42, 1000]) or \
         bin_pack_tools(format: '>BHI', action: 'unpack', hex: '01 00 2a 00 00 03 e8').",
        crate::tools::bin_pack_tools::bin_pack_tools_schema(),
    ));
    tools.push(make_tool(
        "elf_tools",
        "Inspect ELF (Executable and Linkable Format) binary files — Linux executables, shared libraries (.so), \
         object files (.o), and kernel modules (.ko) — without readelf, objdump, or any external tools. \
         Pass 'file' with the path to an ELF binary, or 'hex' with raw ELF bytes as a hex string. \
         Actions: `info` (default — ELF class 32/64-bit, endian, OS/ABI, type ET_EXEC/ET_DYN/ET_REL, \
         machine architecture, entry point address, program and section header counts, interpreter path), \
         `segments` (program headers — type PT_LOAD/PT_DYNAMIC/PT_INTERP, flags R/W/X, virtual address, file offset, size), \
         `sections` (section headers — name, type SHT_PROGBITS/SHT_SYMTAB/SHT_STRTAB, flags AXW, address, offset, size), \
         `symbols` (symbol table entries — value, size, bind LOCAL/GLOBAL/WEAK, type FUNC/OBJECT/NOTYPE, section, name; \
         tries .symtab first, falls back to .dynsym; up to 200 symbols), \
         `dynamic` (dynamic linking info — DT_NEEDED shared library list, SONAME, RPATH, RUNPATH, all dynamic entries). \
         Example: elf_tools(file: '/usr/bin/ls') or elf_tools(action: 'dynamic', file: 'libfoo.so') or \
         elf_tools(action: 'sections', hex: '7f454c46...').",
        crate::tools::elf_tools::elf_tools_schema(),
    ));
    tools.push(make_tool(
        "leb128_tools",
        "Encodes, decodes, and analyzes LEB128 variable-length integers (ULEB128 unsigned and SLEB128 signed) \
         without external utilities. Used in WASM, DWARF debug info, protobuf, and Android DEX. \
         Actions: encode (integer → LEB128 bytes with hex dump), \
         decode (hex/bytes → integer + bytes consumed + remaining stream), \
         analyze (byte-by-byte bit-field breakdown showing continuation bits and data bits), \
         multi (batch encode a JSON array or decode a stream of concatenated LEB128 values), \
         explain (verbose bit-level walkthrough per group with value accumulation). \
         Pass 'value' (integer) for encode, 'hex' or 'bytes' for decode/analyze/multi, \
         'signed: true' for SLEB128 mode (default: unsigned ULEB128). \
         Example: leb128_tools(action: 'encode', value: 624485) gives e5 8e 26. \
         leb128_tools(action: 'decode', hex: 'e5 8e 26') gives 624485.",
        crate::tools::leb128_tools::leb128_tools_schema(),
    ));
    tools.push(make_tool(
        "unicode_tools",
        "Analyzes Unicode text — script distribution, bidi safety, confusable/homoglyph detection, \
         encoding sizes, and normalization status — without external utilities. \
         Actions: analyze (default — per-character table with codepoint, category, script, UTF-8 bytes; \
         max 200 chars), scripts (script distribution count table), \
         blocks (Unicode block distribution), \
         bidi (RTL character detection + Trojan Source CVE-2021-42574 invisible bidi control char risk), \
         confusables (homoglyph/lookalike detection — Cyrillic/Greek chars that look like ASCII), \
         encoding (UTF-8/UTF-16/UTF-32 byte sequences for each character), \
         normalize (NFC vs NFD status — flags combining mark sequences vs precomposed forms). \
         Pass 'text' with the string to analyze. \
         Example: unicode_tools(action: 'bidi', text: 'hello') or \
         unicode_tools(action: 'confusables', text: 'pаypal') (Cyrillic а vs ASCII a) or \
         unicode_tools(text: 'Hello 世界').",
        crate::tools::unicode_tools::unicode_tools_schema(),
    ));
    tools.push(make_tool(
        "asn1_tools",
        "Parse and inspect ASN.1 DER/BER encoded binary data without external utilities. \
         Used in X.509 certificates, PKCS#8/PKCS#12 keys, SNMP, and LDAP. \
         Actions: parse (default — decode DER/BER structure as an indented tag/length/value tree with \
         Universal tag names: SEQUENCE, INTEGER, BIT STRING, OCTET STRING, OID, UTCTime, etc.), \
         oid (look up an OID number to its human-readable name; pass 'oid' field; 200+ well-known OIDs \
         from X.509, PKCS, EC curves, signature algorithms, AES, and hash algorithms), \
         decode_cert (X.509 certificate summary: scan for OIDs, DN fields, serial number, validity dates), \
         info (tag class/number/constructed flag and byte structure at root level). \
         Pass 'hex' (hex-encoded DER bytes, spaces allowed) or 'file' (path to .der/.cer/.crt/.p8 file). \
         Optional 'max_depth' to limit tree depth (default 20). \
         Example: asn1_tools(action: 'oid', oid: '2.5.4.3') gives 'commonName (CN)'. \
         asn1_tools(action: 'parse', hex: '300d0609...').",
        crate::tools::asn1_tools::asn1_tools_schema(),
    ));
    tools.push(make_tool(
        "jsonl_tools",
        "Process JSONL (JSON Lines / NDJSON) data without external utilities. \
         Each newline-separated JSON object is one record. \
         Actions: parse (default — display records with index and pretty-print; 'limit' caps rows, default 20), \
         filter (keep records where a field matches; 'field' dot-path, 'value', \
         'op': eq/ne/gt/lt/gte/lte/contains/exists/missing), \
         map (extract one field from every record; 'field'), \
         aggregate (count/sum/avg/min/max/distinct on a numeric field; 'field' + 'agg'), \
         keys (union of all keys with type distribution and coverage %), \
         stats (record count, key coverage %, null rate, type distribution per field), \
         to_csv (convert records to CSV using all observed keys as headers), \
         group (group by field value with count bar chart; 'field'), \
         sort (sort records by a field; 'field'; 'order': asc/desc). \
         Pass 'text'/'jsonl' for inline content or 'file' for a .jsonl/.ndjson path. \
         Example: jsonl_tools(action: 'filter', text: '...', field: 'status', value: 'error') or \
         jsonl_tools(action: 'stats', file: 'events.jsonl') or \
         jsonl_tools(action: 'aggregate', file: 'access.jsonl', field: 'latency_ms', agg: 'avg').",
        crate::tools::jsonl_tools::jsonl_tools_schema(),
    ));
    tools.push(make_tool(
        "todo_tools",
        "Scan source files for TODO, FIXME, HACK, XXX, NOTE, DEPRECATED, BUG, OPTIMIZE, \
         WORKAROUND, TEMP, KLUDGE, and NB comments without external utilities. \
         Actions: scan (default — find all annotated comments grouped by label with file:line context), \
         stats (count summary per label with bar chart), \
         list (flat list of all findings with file/line/label/text), \
         filter (show only a specific label; pass 'label'), \
         files (top N files by annotation count; pass 'top' for N, default 20). \
         Options: path (directory/file to scan, default '.'), \
         extensions (comma-separated file extensions), limit (max results, default 100). \
         Example: todo_tools() or todo_tools(action: 'filter', label: 'FIXME') or \
         todo_tools(action: 'stats', path: 'src/').",
        crate::tools::todo_tools::todo_tools_schema(),
    ));
    tools.push(make_tool(
        "grep_tools",
        "Search files for patterns using regular expressions without external utilities. \
         Actions: search (default — matching lines with file:line context and optional before/after context lines), \
         count (match count per file), files (list files with at least one match), \
         matches (flat list of every match with capture group extraction). \
         Options: pattern (required), path, action, case_insensitive, fixed (literal string), \
         whole_word, before, after (context lines), extensions, limit, invert. \
         Example: grep_tools(pattern: 'fn execute', path: 'src/') or \
         grep_tools(pattern: 'TODO', action: 'count') or \
         grep_tools(pattern: r'error|warn', case_insensitive: true, after: 2).",
        crate::tools::grep_tools::grep_tools_schema(),
    ));
    tools.push(make_tool(
        "file_tree_tools",
        "Generate visual file tree representations and directory statistics without external utilities. \
         No `tree` command needed. \
         Actions: tree (default — ASCII directory tree with optional depth limit, file sizes), \
         flat (sorted flat file listing with sizes), stats (file/dir/size breakdown grouped by extension), \
         json (structured JSON tree for programmatic use), sizes (files sorted largest first with % bar). \
         Options: path (default '.'), depth (default 4; 0=unlimited), show_hidden, extensions (filter), \
         skip_dirs (extra dirs to exclude), limit, top (for sizes action). \
         Example: file_tree_tools() or file_tree_tools(path: 'src/', depth: 3) or \
         file_tree_tools(action: 'stats') or file_tree_tools(action: 'sizes', top: 20).",
        crate::tools::file_tree_tools::file_tree_tools_schema(),
    ));
    tools.push(make_tool(
        "find_tools",
        "Find files and directories matching criteria without external utilities. \
         No `find` command needed. \
         Actions: list (default — matching paths with size and age), count (match count + total size), \
         sizes (size summary sorted largest first), recent (sorted by modification time, newest first). \
         Filters: name (glob: '*.rs', substring: 'main'), ext (extension without dot: 'json'), \
         type (file/dir/all), min_size/max_size (bytes), newer_than/older_than (days), \
         depth (recursion limit; 0=unlimited), show_hidden, limit. \
         Skips target/.git/node_modules/vendor/build and other cache dirs by default. \
         Example: find_tools(name: '*.rs') or find_tools(ext: 'json', newer_than: 7) or \
         find_tools(action: 'recent', newer_than: 3) or find_tools(min_size: 1048576, action: 'sizes').",
        crate::tools::find_tools::find_tools_schema(),
    ));
    tools.push(make_tool(
        "text_extract_tools",
        "Extract structured entities from unstructured text without external utilities. \
         No regex CLI needed. \
         Actions: emails, urls, ips (IPv4 and IPv6), phones (US/international), \
         dates (ISO/US/EU formats), uuids, hashes (MD5/SHA-1/SHA-256), \
         all (default — every entity type at once), custom (user-supplied regex pattern). \
         Each action returns a deduplicated list with occurrence counts. \
         Options: unique (default true), limit (max matches per type), \
         case_insensitive (for custom action only). \
         Example: text_extract_tools(text: '...') or text_extract_tools(action: 'emails', text: '...') or \
         text_extract_tools(action: 'custom', pattern: 'JIRA-[0-9]+', text: '...').",
        crate::tools::text_extract_tools::text_extract_tools_schema(),
    ));
    tools.push(make_tool(
        "interval_tools",
        "Date interval operations without external utilities. \
         Actions: overlap (do two intervals overlap — shows overlap range and duration), \
         contains (is a date within an interval — shows distance from boundaries), \
         union (merge overlapping intervals from a list), \
         intersect (find the intersection of two intervals), \
         duration (time between two dates — full breakdown in days/weeks/hours/minutes/seconds), \
         schedule (generate N recurring dates from a start at a regular step). \
         Step formats: '1d' (day), '2w' (weeks), '1m' (month), '1y' (year), '6h' (hours), '30min' (minutes). \
         Accepts ISO 8601 dates (YYYY-MM-DD) and datetimes (YYYY-MM-DDTHH:MM:SS). \
         Example: interval_tools(action: 'overlap', start: '2024-01-01', end: '2024-06-30', start2: '2024-04-01', end2: '2024-12-31') or \
         interval_tools(action: 'schedule', start: '2024-01-01', step: '2w', count: 12) or \
         interval_tools(action: 'duration', start: '2023-03-15', end: '2024-09-01').",
        crate::tools::interval_tools::interval_tools_schema(),
    ));
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
        "query_data" => crate::tools::data_query::query_data(args).await,
        "export_as_table" => crate::tools::data_query::export_as_table(args).await,
        "analyze_trends" => crate::tools::data_query::analyze_trends(args).await,
        "scientific_compute" => crate::tools::scientific::scientific_compute(args).await,
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
        "github_ops" => crate::tools::github::execute(args).await,
        "git_commit" => crate::tools::git::execute(args).await,
        "git_push" => crate::tools::git::execute_push(args).await,
        "git_remote" => crate::tools::git::execute_remote(args).await,
        "cargo_errors" => crate::tools::build_errors::execute(args).await,
        "find_symbol" => crate::tools::symbol_search::execute(args).await,
        "refactor_rename" => crate::tools::refactor::execute_rename(args).await,
        "run_tests" => crate::tools::test_runner::execute_run_tests(args).await,
        "manage_deps" => crate::tools::deps::execute(args).await,
        "copy_to_clipboard" => crate::tools::clipboard::copy_to_clipboard(args).await,
        "lint_code" => crate::tools::linter::execute(args).await,
        "format_code" => crate::tools::formatter::execute(args).await,
        "http_request" => crate::tools::http_client::execute(args).await,
        "docker_ops" => crate::tools::docker_ops::execute(args).await,
        "docker_compose_tools" => crate::tools::docker_compose_tools::execute(args).await,
        "dockerfile_tools" => crate::tools::dockerfile_tools::execute(args).await,
        "k8s_tools" => crate::tools::k8s_tools::execute(args).await,
        "nginx_conf_tools" => crate::tools::nginx_conf_tools::execute(args).await,
        "openapi_tools" => crate::tools::openapi_tools::execute(args).await,
        "github_actions_tools" => crate::tools::github_actions_tools::execute(args).await,
        "terraform_tools" => crate::tools::terraform_tools::execute(args).await,
        "package_json_tools" => crate::tools::package_json_tools::execute(args).await,
        "sql_tools" => crate::tools::sql_tools::execute(args).await,
        "proto_tools" => crate::tools::proto_tools::execute(args).await,
        "pem_tools" => crate::tools::pem_tools::execute(args).await,
        "env_schema_tools" => crate::tools::env_schema_tools::execute(args).await,
        "graphql_tools" => crate::tools::graphql_tools::execute(args).await,
        "sql_migrate_tools" => crate::tools::sql_migrate_tools::execute(args).await,
        "base_tools" => crate::tools::base_tools::execute(args).await,
        "lock_file_tools" => crate::tools::lock_file_tools::execute(args).await,
        "fraction_tools" => crate::tools::fraction_tools::execute(args).await,
        "number_theory_tools" => crate::tools::number_theory_tools::execute(args).await,
        "cipher_tools" => crate::tools::cipher_tools::execute(args).await,
        "nato_tools" => crate::tools::nato_tools::execute(args).await,
        "geo_tools" => crate::tools::geo_tools::execute(args).await,
        "data_gen_tools" => crate::tools::data_gen_tools::execute(args).await,
        "unit_tools" => crate::tools::unit_tools::execute(args).await,
        "geometry_tools" => crate::tools::geometry_tools::execute(args).await,
        "checksum_tools" => crate::tools::checksum_tools::execute(args).await,
        "id_tools" => crate::tools::id_tools::execute(args).await,
        "binary_tools" => crate::tools::binary_tools::execute(args).await,
        "ascii_tools" => crate::tools::ascii_tools::execute(args).await,
        "time_zone_tools" => crate::tools::time_zone_tools::execute(args).await,
        "word_tools" => crate::tools::word_tools::execute(args).await,
        "string_metric_tools" => crate::tools::string_metric_tools::execute(args).await,
        "calc_tools" => crate::tools::calc_tools::execute(args).await,
        "secret_scanner" => crate::tools::secret_scanner::execute(args).await,
        "code_metrics" => crate::tools::code_metrics::execute(args).await,
        "dependency_audit" => crate::tools::dependency_audit::execute(args).await,
        "port_check" => crate::tools::port_check::execute(args).await,
        "env_diff" => crate::tools::env_diff::execute(args).await,
        "template_gen" => crate::tools::template_gen::execute(args).await,
        "json_tools" => crate::tools::json_tools::execute(args).await,
        "regex_tools" => crate::tools::regex_tools::execute(args).await,
        "diff_tools" => crate::tools::diff_tools::execute(args).await,
        "yaml_tools" => crate::tools::yaml_tools::execute(args).await,
        "csv_tools" => crate::tools::csv_tools::execute(args).await,
        "encode_tools" => crate::tools::encode_tools::execute(args).await,
        "hash_tools" => crate::tools::hash_tools::execute(args).await,
        "toml_tools" => crate::tools::toml_tools::execute(args).await,
        "text_tools" => crate::tools::text_tools::execute(args).await,
        "date_tools" => crate::tools::date_tools::execute(args).await,
        "number_tools" => crate::tools::number_tools::execute(args).await,
        "uuid_gen" => crate::tools::uuid_gen::execute(args).await,
        "cron_tools" => crate::tools::cron_tools::execute(args).await,
        "ip_tools" => crate::tools::ip_tools::execute(args).await,
        "color_tools" => crate::tools::color_tools::execute(args).await,
        "semver_tools" => crate::tools::semver_tools::execute(args).await,
        "password_gen" => crate::tools::password_gen::execute(args).await,
        "jwt_tools" => crate::tools::jwt_tools::execute(args).await,
        "xml_tools" => crate::tools::xml_tools::execute(args).await,
        "archive_tools" => crate::tools::archive_tools::execute(args).await,
        "sqlite_tools" => crate::tools::sqlite_tools::execute(args).await,
        "markdown_tools" => crate::tools::markdown_tools::execute(args).await,
        "url_tools" => crate::tools::url_tools::execute(args).await,
        "line_tools" => crate::tools::line_tools::execute(args).await,
        "hex_tools" => crate::tools::hex_tools::execute(args).await,
        "ini_tools" => crate::tools::ini_tools::execute(args).await,
        "path_tools" => crate::tools::path_tools::execute(args).await,
        "table_tools" => crate::tools::table_tools::execute(args).await,
        "duration_tools" => crate::tools::duration_tools::execute(args).await,
        "dotenv_tools" => crate::tools::dotenv_tools::execute(args).await,
        "ansi_tools" => crate::tools::ansi_tools::execute(args).await,
        "template_tools" => crate::tools::template_tools::execute(args).await,
        "char_tools" => crate::tools::char_tools::execute(args).await,
        "stat_tools" => crate::tools::stat_tools::execute(args).await,
        "rss_tools" => crate::tools::rss_tools::execute(args).await,
        "keyval_tools" => crate::tools::keyval_tools::execute(args).await,
        "net_lookup_tools" => crate::tools::net_lookup_tools::execute(args).await,
        "money_tools" => crate::tools::money_tools::execute(args).await,
        "size_tools" => crate::tools::size_tools::execute(args).await,
        "validate_tools" => crate::tools::validate_tools::execute(args).await,
        "token_tools" => crate::tools::token_tools::execute(args).await,
        "mime_tools" => crate::tools::mime_tools::execute(args).await,
        "plist_tools" => crate::tools::plist_tools::execute(args).await,
        "bencode_tools" => crate::tools::bencode_tools::execute(args).await,
        "printf_tools" => crate::tools::printf_tools::execute(args).await,
        "ascii_chart_tools" => crate::tools::ascii_chart_tools::execute(args).await,
        "sql_format_tools" => crate::tools::sql_format_tools::execute(args).await,
        "totp_tools" => crate::tools::totp_tools::execute(args).await,
        "tar_tools" => crate::tools::tar_tools::execute(args).await,
        "email_tools" => crate::tools::email_tools::execute(args).await,
        "wasm_tools" => crate::tools::wasm_tools::execute(args).await,
        "jsonschema_tools" => crate::tools::jsonschema_tools::execute(args).await,
        "cbor_tools" => crate::tools::cbor_tools::execute(args).await,
        "msgpack_tools" => crate::tools::msgpack_tools::execute(args).await,
        "html_tools" => crate::tools::html_tools::execute(args).await,
        "vcf_tools" => crate::tools::vcf_tools::execute(args).await,
        "network_header_tools" => crate::tools::network_header_tools::execute(args).await,
        "tlv_tools" => crate::tools::tlv_tools::execute(args).await,
        "bin_pack_tools" => crate::tools::bin_pack_tools::execute(args).await,
        "elf_tools" => crate::tools::elf_tools::execute(args).await,
        "leb128_tools" => crate::tools::leb128_tools::execute(args).await,
        "unicode_tools" => crate::tools::unicode_tools::execute(args).await,
        "asn1_tools" => crate::tools::asn1_tools::execute(args).await,
        "jsonl_tools" => crate::tools::jsonl_tools::execute(args).await,
        "todo_tools" => crate::tools::todo_tools::execute(args).await,
        "grep_tools" => crate::tools::grep_tools::execute(args).await,
        "file_tree_tools" => crate::tools::file_tree_tools::execute(args).await,
        "find_tools" => crate::tools::find_tools::execute(args).await,
        "text_extract_tools" => crate::tools::text_extract_tools::execute(args).await,
        "interval_tools" => crate::tools::interval_tools::execute(args).await,
        "http_status_tools" => crate::tools::http_status_tools::execute(args).await,
        "http_parse_tools" => crate::tools::http_parse_tools::execute(args).await,
        "jq_tools" => crate::tools::jq_tools::execute(args).await,
        "glob_tools" => crate::tools::glob_tools::execute(args).await,
        "graph_tools" => crate::tools::graph_tools::execute(args).await,
        "matrix_tools" => crate::tools::matrix_tools::execute(args).await,
        "har_tools" => crate::tools::har_tools::execute(args).await,
        "ical_tools" => crate::tools::ical_tools::execute(args).await,
        "graphviz_tools" => crate::tools::graphviz_tools::execute(args).await,
        "mermaid_tools" => crate::tools::mermaid_tools::execute(args).await,
        "dns_tools" => crate::tools::dns_tools::execute(args).await,
        "css_tools" => crate::tools::css_tools::execute(args).await,
        "log_parse_tools" => crate::tools::log_parse_tools::execute(args).await,
        "csp_tools" => crate::tools::csp_tools::execute(args).await,
        "robots_txt_tools" => crate::tools::robots_txt_tools::execute(args).await,
        "sitemap_tools" => crate::tools::sitemap_tools::execute(args).await,
        "gitignore_tools" => crate::tools::gitignore_tools::execute(args).await,
        "license_tools" => crate::tools::license_tools::execute(args).await,
        "make_tools" => crate::tools::make_tools::execute(args).await,
        "changelog_tools" => crate::tools::changelog_tools::execute(args).await,
        "ssh_config_tools" => crate::tools::ssh_config_tools::execute(args).await,
        "systemd_tools" => crate::tools::systemd_tools::execute(args).await,
        "git_status" => crate::tools::git::execute_status(args).await,
        "git_diff" => crate::tools::git::execute_diff(args).await,
        "git_log" => crate::tools::git::execute_log(args).await,
        "changelog_gen" => crate::tools::git::execute_changelog(args).await,
        "git_onboarding" => crate::tools::git_onboarding::execute(args).await,
        "run_with_backtrace" => crate::tools::debugger::execute(args).await,
        "profile_process" => crate::tools::profiler::execute(args).await,
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
        "github_ops" => {
            let action = args.get("action").and_then(|v| v.as_str()).unwrap_or("?");
            match action {
                "pr_create" | "pr_merge" | "issue_create" => Some(format!("GitHub: {}", action)),
                _ => None,
            }
        }
        "refactor_rename" => {
            let old = args.get("old_name").and_then(|v| v.as_str()).unwrap_or("?");
            let new = args.get("new_name").and_then(|v| v.as_str()).unwrap_or("?");
            let dry = args
                .get("dry_run")
                .and_then(|v| v.as_bool())
                .unwrap_or(true);
            if dry {
                None // dry-run previews don't need approval
            } else {
                Some(format!("Workspace-Wide Symbol Rename: {old} → {new}"))
            }
        }
        "manage_deps" => {
            let action = args.get("action").and_then(|v| v.as_str()).unwrap_or("");
            match action {
                "add" => {
                    let name = args.get("name").and_then(|v| v.as_str()).unwrap_or("?");
                    Some(format!("Add Dependency: {name}"))
                }
                "remove" => {
                    let name = args.get("name").and_then(|v| v.as_str()).unwrap_or("?");
                    Some(format!("Remove Dependency: {name}"))
                }
                _ => None, // list/tree/outdated/audit are read-only
            }
        }
        "git_commit" => Some("Permanent Version History Commit".into()),
        "git_push" => Some("Remote Origin Synchronisation (Push)".into()),
        "resolve_host_issue" => Some("System-Level Host Remediation".into()),
        "run_workspace_workflow" => Some("Automated Workspace Re-alignment".into()),
        _ => None,
    }
}
