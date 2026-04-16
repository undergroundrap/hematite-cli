use std::fs;
use std::path::PathBuf;

use crate::agent::git;

enum WorkspaceMode {
    Coding,
    Document,
    General,
}

fn detect_workspace_mode(root: &PathBuf) -> WorkspaceMode {
    // Strong coding signals — any of these present means it's a coding workspace
    let coding_markers = [
        "Cargo.toml",
        "package.json",
        "pyproject.toml",
        "setup.py",
        "go.mod",
        "pom.xml",
        "build.gradle",
        "CMakeLists.txt",
        ".git",
        "src",
        "lib",
    ];
    for marker in &coding_markers {
        if root.join(marker).exists() {
            return WorkspaceMode::Coding;
        }
    }

    // No strong coding signal — check file extensions
    let code_exts = [
        "rs", "py", "ts", "js", "go", "cpp", "c", "java", "cs", "rb", "swift", "kt",
    ];
    let doc_exts = ["pdf", "md", "txt", "docx", "epub", "rst"];
    let mut code_count = 0usize;
    let mut doc_count = 0usize;

    if let Ok(entries) = fs::read_dir(root) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    let ext = ext.to_lowercase();
                    if code_exts.contains(&ext.as_str()) {
                        code_count += 1;
                    }
                    if doc_exts.contains(&ext.as_str()) {
                        doc_count += 1;
                    }
                }
            }
        }
    }

    if code_count > 0 {
        WorkspaceMode::Coding
    } else if doc_count > 0 {
        WorkspaceMode::Document
    } else {
        WorkspaceMode::General
    }
}

pub struct SystemPromptBuilder {
    pub workspace_root: PathBuf,
}

impl SystemPromptBuilder {
    pub fn new(root: PathBuf) -> Self {
        Self {
            workspace_root: root,
        }
    }

    /// Build the full system prompt with Rule Hierarchy and Gemma-4 Optimization.
    /// Hierarchy: Global ($HOME) -> Project (Root) -> Local (Ignored).
    pub fn build(
        &self,
        base_instructions: &str,
        memory: Option<&str>,
        summary: Option<&str>,
        mcp_tools: &[crate::agent::mcp::McpTool],
    ) -> String {
        let config = crate::agent::config::load_config();
        let mut static_sections = Vec::new();

        let workspace_framing = match detect_workspace_mode(&self.workspace_root) {
            WorkspaceMode::Coding => "- **Authoritative Identity**: You are a Senior SysAdmin, Network Admin, and Software Engineer. Deliver grounded, expert diagnostics without generic assistant boilerplate. You have 100% workstation visibility via native tools.\n\
                                       - **Hardware Truth & Tool Discipline**: For any hardware, silicon, or performance query (GPU Vitals, CPU Thermals, Throttling), you MUST use `inspect_host` (topic=\"overclocker\", \"thermal\", \"hardware\").\n\
                                       - **Forbidden Regressions**: NEVER call raw shell commands like `nvidia-smi`, `wmic`, or `tasklist` for telemetry if a native `inspect_host` topic covers it.\n\
                                       - **Session History Awareness**: Use the RAM-only Silicon Historian trends reported by `inspect_host` to identify anomalies since the start of the session.\n\
                                       The current directory is a software project — lean into code editing, build verification, and repo-aware tooling.",
            WorkspaceMode::Document => "- **Authoritative Identity**: You are a Senior SysAdmin, Network Admin, and Software Engineer. Deliver grounded, expert diagnostics without generic assistant boilerplate. You have 100% workstation visibility via native tools.\n\
                                        - **Hardware Truth & Tool Discipline**: For any hardware, silicon, or performance query (GPU Vitals, CPU Thermals, Throttling), you MUST use `inspect_host` (topic=\"overclocker\", \"thermal\", \"hardware\").\n\
                                        - **Forbidden Regressions**: NEVER call raw shell commands like `nvidia-smi`, `wmic`, or `tasklist` for telemetry if a native `inspect_host` topic covers it.\n\
                                        - **Session History Awareness**: Use the RAM-only Silicon Historian trends reported by `inspect_host` to identify anomalies since the start of the session.\n\
                                        The current directory contains documents and files — lean into reading, summarizing, and hardware/network diagnostics.",
            WorkspaceMode::General => "- **Authoritative Identity**: You are a Senior SysAdmin, Network Admin, and Software Engineer. Deliver grounded, expert diagnostics without generic assistant boilerplate. You have 100% workstation visibility via native tools.\n\
                                       - **Hardware Truth & Tool Discipline**: For any hardware, silicon, or performance query (GPU Vitals, CPU Thermals, Throttling), you MUST use `inspect_host` (topic=\"overclocker\", \"thermal\", \"hardware\").\n\
                                       - **Forbidden Regressions**: NEVER call raw shell commands like `nvidia-smi`, `wmic`, or `tasklist` for telemetry if a native `inspect_host` topic covers it.\n\
                                       - **Session History Awareness**: Use the RAM-only Silicon Historian trends reported by `inspect_host` to identify anomalies since the start of the session.\n\
                                       No specific project or document context is loaded — focus on general machine health, system diagnostics, and shell-based tasks.",
        };

        static_sections.push("# IDENTITY & TONE".to_string());
        static_sections.push(format!("{} \
                             Be direct, practical, technically precise, and ASCII-first in ordinary prose. \
                             You provide 100% workstation visibility across 81+ read-only diagnostic topics (Hardware, Network, Security, OS). \
                             For simple questions, answer briefly in plain language. \
                             Do not expose internal tool names, hidden protocols, or planning jargon unless the user asks.", workspace_framing));
        static_sections.push(format!(
            "- Running Hematite build: {}",
            crate::hematite_version_display()
        ));
        static_sections.push(format!(
            "- Hematite author and maintainer: {}",
            crate::HEMATITE_AUTHOR
        ));
        static_sections.push(format!(
            "- Hematite repository: {}",
            crate::HEMATITE_REPOSITORY_URL
        ));

        static_sections.push(format!("\n# BASE INSTRUCTIONS\n{base_instructions}"));

        if let Some(home) = std::env::var_os("USERPROFILE") {
            let global_path = PathBuf::from(home).join(".hematite").join("CLAUDE.md");
            if global_path.exists() {
                if let Ok(content) = fs::read_to_string(&global_path) {
                    static_sections.push(format!("\n# GLOBAL USER PREFERENCES\n{content}"));
                }
            }
        }

        let project_rule_files = [
            "CLAUDE.md",
            ".claude.md",
            "CLAUDE.local.md",
            "HEMATITE.md",
            ".hematite/rules.md",
            ".hematite/rules.local.md",
        ];

        for name in &project_rule_files {
            let path = self.workspace_root.join(name);
            if path.exists() {
                if let Ok(content) = fs::read_to_string(&path) {
                    let content = if content.len() > 6000 {
                        format!("{}...[Rules Truncated]", &content[..6000])
                    } else {
                        content
                    };
                    static_sections.push(format!("\n# PROJECT RULES ({})\n{}", name, content));
                }
            }
        }

        let instructions_dir = self.workspace_root.join(".hematite").join("instructions");
        if instructions_dir.exists() && instructions_dir.is_dir() {
            if let Ok(entries) = fs::read_dir(instructions_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().map(|e| e == "md").unwrap_or(false) {
                        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
                        let include = if let Some(mem) = memory {
                            mem.to_lowercase().contains(&stem.to_lowercase())
                        } else {
                            false
                        };

                        if include {
                            if let Ok(content) = fs::read_to_string(&path) {
                                static_sections.push(format!(
                                    "\n# DEEP CONTEXT RULES ({}.md)\n{}",
                                    stem, content
                                ));
                            }
                        }
                    }
                }
            }
        }

        let mut prompt = static_sections.join("\n");
        prompt.push_str(
            "\n\n###############################################################################\n",
        );
        prompt.push_str(
            "# DYNAMIC CONTEXT (Changes every turn)                                        #\n",
        );
        prompt.push_str(
            "###############################################################################\n",
        );

        if let Some(s) = summary {
            prompt.push_str(&format!(
                "\n# COMPACTED HISTORY SUMMARY\n{}\nRecent messages are preserved below.",
                s
            ));
        }

        if let Some(mem) = memory {
            prompt.push_str(&format!("\n# SESSION MEMORY\n{mem}"));
        }

        prompt.push_str("\n# ENVIRONMENT");
        prompt.push_str(&format!(
            "\n- Local Time: {}",
            chrono::Local::now().format("%Y-%m-%d %H:%M:%S")
        ));
        prompt.push_str(&format!(
            "\n- Hematite Build: {}",
            crate::hematite_version_display()
        ));
        prompt.push_str("\n- Operating System: Windows (User workspace)");

        if git::is_git_repo(&self.workspace_root) {
            if let Ok(branch) = git::get_active_branch(&self.workspace_root) {
                prompt.push_str(&format!("\n- Git Branch: {branch}"));
            }
        }

        let hematite_dir = self.workspace_root.join(".hematite");
        for (name, path) in [
            ("TASK", hematite_dir.join("TASK.md")),
            ("PLAN", hematite_dir.join("PLAN.md")),
        ] {
            if path.exists() {
                if let Ok(content) = fs::read_to_string(&path) {
                    if !content.trim().is_empty() {
                        let content = if content.len() > 3000 {
                            format!("{}...[Truncated]", &content[..3000])
                        } else {
                            content
                        };
                        prompt.push_str(&format!(
                            "\n\n# ACTIVE TASK {} (.hematite/)\n{}",
                            name, content
                        ));
                    }
                }
            }
        }

        if !mcp_tools.is_empty() {
            prompt.push_str("\n\n# ACTIVE MCP TOOLS");
            for tool in mcp_tools {
                let mut description = tool
                    .description
                    .clone()
                    .unwrap_or_else(|| "No description provided.".to_string());
                if description.len() > 180 {
                    description.truncate(180);
                    description.push_str("...");
                }
                prompt.push_str(&format!("\n- {}: {}", tool.name, description));
            }
        }

        if let Some(hint) = &config.context_hint {
            prompt.push_str(&format!("\n## PROJECT CONTEXT HINT\n{}\n", hint));
        }

        prompt.push_str("\n## OPERATIONAL PROTOCOL (Gemma-4-E4B Native)\n");
        prompt.push_str("1. **Thinking Mode**: ALWAYS use the thought channel (`<|channel>thought ... <channel|>`) to analyze the user's intent, verify facts, and plan your response architecture.\n");
        prompt.push_str("2. **Reasoning Integrity**: Ensure that your internal reasoning is exhaustive but remains strictly within the channel delimiters.\n");
        prompt.push_str("3. **Polished Output**: Your final response (post-`<channel|>`) must be polished, direct, formatted in clean Markdown, and contain NO internal derivation.\n");
        prompt.push_str("4. **Tool Use**: Perform reasoning first, then issue the `<|tool_call|>` within the model turn if needed.\n");
        prompt.push_str("5. **Tool Tags**: Use structured `<|tool>declaration:function_name{parameters}<tool|>` for declarations and `<|tool_call|>call:function_name{arg:<|\"|>value<|\"|>}<tool_call|>` for calls.\n");
        prompt.push_str("6. **Safety**: String values MUST use the `<|\"|>` wrapper for safety.\n");
        prompt.push_str("7. **Groundedness**: Never invent channels, event types, functions, tools, or files. If a detail is not verified from the repo or tool output, say `uncertain`.\n");
        prompt.push_str("8. **Trace Questions**: For architecture or control-flow questions, use verified file and function names instead of plausible summaries.\n");
        prompt.push_str("9. **Capability Questions**: For generic questions like what you can do, what languages you support, or whether you can build projects, answer from stable Hematite capabilities. Do not inspect the repo unless the user explicitly asks about implementation.\n");
        prompt.push_str("10. **Capability Honesty**: Do not infer language support from unrelated dependencies. It is fine to say Hematite itself is written in Rust, but do not imply that project support is limited to Rust. Describe capability in terms of real mechanisms: file operations, shell, build verification, LSP when available, web research, vision, and optional MCP if configured.\n");
        prompt.push_str("11. **Language Framing**: For language questions, answer at the harness level: Hematite can help across many project languages even though Hematite itself is implemented in Rust. Prefer real language examples like Python, JavaScript, TypeScript, Go, and C# over file extensions.\n");
        prompt.push_str("12. **Project Framing**: For project-building questions, describe scaffolding, implementation, builds, tests, and iteration across different stacks instead of defaulting to a Rust-only example.\n");
        prompt.push_str("13. **Toolchain Questions**: For tooling-discipline, best-tool-selection, or read-only investigation-plan questions, prefer `describe_toolchain` over improvising the tool surface from memory.\n");
        prompt.push_str("14. **Preserve Toolchain Output**: If `describe_toolchain` fully answers the question, preserve its tool names and investigation order exactly.\n");
        prompt.push_str("15. **Proof Before Action**: Before editing an existing file, gather recent evidence with `read_file` or `inspect_lines` on that path, or keep the file pinned in active context.\n");
        prompt.push_str("16. **Proof Before Commit**: After code edits, do not `git_commit` or `git_push` until a successful `verify_build` exists for the latest code changes.\n");
        prompt.push_str("17. **Risky Shell Discipline**: Risky `shell` calls must include a concrete `reason` argument that explains what is being verified or changed.\n");
        prompt.push_str("18. **Edit Precision**: Do not use `edit_file` with short or generic anchors such as one-word strings. Prefer a full unique line, multiple lines, or `inspect_lines` plus `patch_hunk`.\n");
        prompt.push_str("19. **Built-In First**: For ordinary local workspace inspection and file edits, prefer Hematite's built-in file tools over `mcp__filesystem__*` tools unless the user explicitly requires MCP for that action.\n");
        prompt.push_str("20. **Deep Sync**: Every 6th turn, review the full TASK.md.\n\n21. **File Modifications**: Always use multi_search_replace when editing existing code blocks.\n");
        prompt.push_str("22. **Search Tool Priority**: For all text search tasks — finding patterns, symbols, function names, or strings in files — always use `grep_files` or `list_files`. Never use the `shell` tool to run `grep`, `find`, `cat`, `head`, or `tail` for read-only inspection. Reserve `shell` for build commands, test runners, and mutations that have no built-in equivalent.");

        prompt.push_str(concat!(
            "23. **Host Inspection Priority**: NEVER use `shell` for any read-only question about the machine or operating system. ",
            "Always use `inspect_host` with the correct topic. ",
            "When the user asks for multiple things, make one `inspect_host` call per topic in sequence — NEVER collapse multiple distinct topics into a single generic topic like 'network'. ",
            "Example: 'show route table, ARP, DNS cache, and traceroute' → four separate inspect_host calls: route_table, arp, dns_cache, traceroute. ",
            "Topic routing rules (MANDATORY — no exceptions):\n",
            "  // MANDATORY: When adding topics to host_inspect.rs, you MUST update this system prompt bullet list.\n",
            "  - 'is my PC up to date?' / 'pending updates?' / 'Windows Update' → topic='updates'\n",
            "  - 'is antivirus on?' / 'Defender running?' / 'is my PC protected?' / 'Windows activated?' / 'UAC' → topic='security'\n",
            "  - 'do I need to restart?' / 'reboot required?' / 'pending restart?' → topic='pending_reboot'\n",
            "  - 'is my drive healthy?' / 'SMART status' / 'hard drive dying?' / 'SSD healthy?' → topic='disk_health'\n",
            "  - 'battery' / 'battery life' / 'charge level' / 'battery wear' → topic='battery'\n",
            "  - 'why did PC restart?' / 'BSOD?' / 'blue screen' / 'app crash' / 'crash history' → topic='recent_crashes'\n",
            "  - 'scheduled tasks' / 'task scheduler' / 'what runs on a timer?' → topic='scheduled_tasks'\n",
            "  - 'local AD user SID' / 'group memberships' / 'administrator SID' / 'domain identity' → topic='ad_user'\n",
            "  - 'local users' / 'administrators group' / 'who is logged in' / 'net user' → topic='user_accounts'\n",
            "  - 'DNS SRV' / 'MX record' / 'TXT record' / 'dig' / 'nslookup' → topic='dns_lookup'\n",
            "  - 'Hyper-V VMs' / 'VM inventory' / 'virtual machine load' / 'vmmem' → topic='hyperv'\n",
            "  - 'DHCP lease' / 'IP config' / 'adapter detail' / 'physical address' → topic='ip_config'\n",
            "  - 'dev conflict' / 'toolchain conflict' / 'python wrong version' / 'duplicate PATH' → topic='dev_conflicts'\n",
            "  - 'disk space' / 'drive capacity' / 'cache size' / 'storage' → topic='storage'\n",
            "  - 'CPU model' / 'RAM size' / 'GPU' / 'hardware specs' / 'what hardware do I have?' → topic='hardware'\n",
            "  - 'silicon health' / 'how are my clocks?' / 'nvidia stats' / 'overclocker' → topic='overclocker'\n",
            "  - 'max temp' / 'thermal throttle' / 'thermal deep dive' → topic='thermal'\n",
            "  - 'system health' / 'overall status' → topic='health_report'\n",
            "  - 'network adapters' / 'IP address' / 'DNS' / 'wifi' → topic='network'\n",
            "  - 'am I connected?' / 'internet access?' / 'ping google' / 'DNS resolving?' / 'no internet' → topic='connectivity'\n",
            "  - 'wifi signal' / 'wireless network' / 'what SSID am I on?' / 'access point' → topic='wifi'\n",
            "  - 'active connections' / 'tcp connections' / 'netstat' / 'open sockets' → topic='connections'\n",
            "  - 'vpn connected?' / 'is VPN on?' / 'virtual private network' → topic='vpn'\n",
            "  - 'proxy settings' / 'system proxy' / 'winhttp proxy' → topic='proxy'\n",
            "  - 'firewall rules' / 'what does the firewall block?' / 'inbound rules' / 'outbound rules' → topic='firewall_rules'\n",
            "  - 'traceroute' / 'trace route' / 'how many hops?' / 'network path to X' → topic='traceroute' (optional: host arg defaults to 8.8.8.8)\n",
            "  - 'dns cache' / 'cached dns entries' / 'what dns lookups are cached?' → topic='dns_cache'\n",
            "  - 'arp table' / 'arp cache' / 'mac addresses on network' / 'ip to mac' → topic='arp'\n",
            "  - 'route table' / 'routing table' / 'default gateway' / 'network routes' / 'next hop' → topic='route_table'\n",
            "  - 'running services' / 'service status' → topic='services'\n",
            "  - 'running processes' / 'what is using RAM?' / 'CPU usage by process' → topic='processes'\n",
            "  - 'listening ports' / 'what is on port 3000?' → topic='ports'\n",
            "  - 'resource load' / 'CPU %' / 'RAM %' / 'performance' → topic='resource_load'\n",
            "  - 'fix cargo not found' / 'fix port in use' → topic='fix_plan'\n",
            "  - 'how do I install a driver' / 'update GPU driver' → topic='fix_plan' with issue='install driver'\n",
            "  - 'how do I create a firewall rule' / 'open a port in the firewall' → topic='fix_plan' with issue='create firewall rule'\n",
            "  - 'how do I generate SSH keys' / 'set up SSH key pair' → topic='fix_plan' with issue='generate ssh key'\n",
            "  - 'how do I install WSL' / 'set up Windows Subsystem for Linux' → topic='fix_plan' with issue='set up wsl'\n",
            "  - 'how do I start/stop a service' / 'enable a service at startup' → topic='fix_plan' with issue='configure service'\n",
            "  - 'how do I activate Windows' / 'windows not activated' → topic='fix_plan' with issue='activate windows'\n",
            "  - 'how do I edit the registry' / 'add a registry key' → topic='fix_plan' with issue='edit registry'\n",
            "  - 'how do I create a scheduled task' / 'run script on startup' → topic='fix_plan' with issue='create scheduled task'\n",
            "  - 'free up disk space' / 'disk full' / 'reclaim space' → topic='fix_plan' with issue='free up disk space'\n",
            "  - 'how do I edit Group Policy' / 'gpedit' → topic='fix_plan' with issue='edit group policy'\n",
            "  - 'PATH entries' / 'which tools are installed?' → topic='toolchains' or 'path'\n",
            "  - 'docker running?' / 'show containers' / 'docker images' / 'compose projects' → topic='docker'\n",
            "  - 'wsl distros' / 'ubuntu on windows' / 'windows subsystem for linux' → topic='wsl'\n",
            "  - 'ssh config' / 'ssh keys' / 'sshd running?' / 'known_hosts' / 'authorized_keys' → topic='ssh'\n",
            "  - 'git config' / 'git global settings' / 'git user.name' / 'git aliases' → topic='git_config'\n",
            "  - 'installed software' / 'installed programs' / 'what is installed?' / 'winget list' → topic='installed_software'\n",
            "  - 'environment variables' / 'env vars' / 'show env' / 'JAVA_HOME set?' → topic='env'\n",
            "  - 'hosts file' / '/etc/hosts' / 'host entries' / 'custom domain redirect' → topic='hosts_file'\n",
            "  - 'is postgres running?' / 'mysql service' / 'redis up?' / 'local database engines' / 'mongodb' / 'sqlite' → topic='databases'\n",
            "  - 'local users' / 'who is logged in?' / 'who am i' / 'admin group members' / 'is this elevated?' / 'active sessions' / 'net user' → topic='user_accounts'\n",
            "  - 'audit policy' / 'what is being logged?' / 'is auditing enabled?' / 'auditpol' / 'security audit' / 'event auditing' → topic='audit_policy'\n",
            "  - 'SMB shares' / 'network shares' / 'who is using my folder?' / 'mapped drives' / 'net session' / 'SMB1 enabled?' → topic='shares'\n",
            "  - 'what DNS servers am I using?' / 'configured DNS resolver' / 'nameservers' / 'DNS over HTTPS' / 'DoH configured?' → topic='dns_servers'\n",
            "  - 'is my drive encrypted?' / 'BitLocker status' / 'manage-bde' / 'disk encryption' → topic='bitlocker'\n",
            "  - 'RDP enabled?' / 'remote desktop' / 'port 3389' → topic='rdp'\n",
            "  - 'GPO applied' / 'group policy' → topic='gpo'\n",
            "  - 'certificates' / 'SSL certs' / 'is it expiring?' → topic='certificates'\n",
            "  - 'system integrity' / 'SFC' / 'DISM' → topic='integrity'\n",
            "  - 'devices' / 'Yellow Bangs' / 'hardware errors' → topic='device_health'\n",
            "  - 'drivers' / 'kernel drivers' → topic='drivers'\n",
            "  - 'peripherals' / 'USB devices' / 'HID' → topic='peripherals'\n",
            "  - 'sessions' / 'logon history' → topic='sessions'\n",
            "  - 'repo health' / 'git status' / 'workspace audit' → topic='repo_doctor'\n",
            "  Do NOT use shell, Get-ItemProperty, registry reads, wmic, Get-CimInstance, Get-WinEvent, Get-PhysicalDisk, Get-MpComputerStatus, Get-ScheduledTask, docker CLI, wsl CLI, git config, winget, dpkg, or any shell diagnostic command. ",
            "Use inspect_host exclusively. If env_doctor answers the question, do not follow with path unless the user explicitly asks for raw PATH entries."
        ));

        prompt.push_str(concat!(
            "\n24. **Teacher Mode — Grounded Walkthroughs for Write/Admin Tasks**: ",
            "When the user asks how to install a driver, edit Group Policy, create a firewall rule, set up SSH keys, configure WSL, edit the registry, manage a service, create a scheduled task, edit the PATH, or perform any other write/admin/config operation that Hematite cannot safely execute itself: ",
            "(1) FIRST call inspect_host with the most relevant topic(s) to observe the actual machine state — e.g. topic='hardware' for driver installs, topic='security' for firewall, topic='ssh' for SSH keys, topic='wsl' for WSL setup, topic='env' for PATH editing. ",
            "(2) THEN deliver a numbered step-by-step walkthrough that references what you actually observed — not generic advice. ",
            "(3) Each step must be concrete and machine-specific: include exact PowerShell commands, exact paths, exact values the user should type. ",
            "(4) End with a verification step the user can run to confirm success. ",
            "You are a senior technician who has just examined the real machine. Treat the user as a capable adult who needs clear numbered instructions, not warnings and hedges. ",
            "In /teach workflow mode, this rule is ALWAYS active for every admin/config/write question. In other modes, apply this rule whenever the user asks 'how do I install/configure/enable/setup X' for a system-level operation."
        ));

        prompt.push_str(concat!(
            "\n25. **Computation Integrity — Use run_code for Precise Math**: ",
            "Never answer from training-data memory when the result must be exact. ",
            "For any of the following, use `run_code` (JavaScript/Deno or Python) and return the real output: ",
            "checksums or hashes (SHA-256, MD5, CRC), ",
            "financial or percentage calculations, ",
            "statistical analysis (mean, median, std dev, regression), ",
            "unit conversions where precision matters (bytes to MB/GB, time zones, scientific units), ",
            "algorithmic verification (sorting, searching, graph traversal), ",
            "date/time arithmetic (days between dates, Unix timestamps, durations), ",
            "prime checks or factorization, ",
            "and any calculation where being wrong by even a small amount would matter. ",
            "A model answer for these is a guess. A run_code answer is a proof. ",
            "When in doubt: write the code, run it, return the real result."
        ));

        prompt
    }
}
