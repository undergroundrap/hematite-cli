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
        "secret_scanner" => crate::tools::secret_scanner::execute(args).await,
        "code_metrics" => crate::tools::code_metrics::execute(args).await,
        "dependency_audit" => crate::tools::dependency_audit::execute(args).await,
        "port_check" => crate::tools::port_check::execute(args).await,
        "env_diff" => crate::tools::env_diff::execute(args).await,
        "template_gen" => crate::tools::template_gen::execute(args).await,
        "json_tools" => crate::tools::json_tools::execute(args).await,
        "regex_tools" => crate::tools::regex_tools::execute(args).await,
        "diff_tools" => crate::tools::diff_tools::execute(args).await,
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
