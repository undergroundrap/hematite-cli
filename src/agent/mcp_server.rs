// MCP stdio server mode — run with `hematite --mcp-server`
//
// Protocol: JSON-RPC 2.0, newline-delimited over stdin/stdout.
// stderr is the only safe log channel — stdout is the protocol wire.
//
// Exposes:
//   inspect_host — 116+ read-only diagnostic topics (SysAdmin, Network Admin,
//                  hardware, security, developer tooling)
//
// Privacy modes:
//   --edge-redact        Tier 1 regex: strips usernames, MACs, serials, hostnames, credentials
//   --semantic-redact    Tier 2: local model summarizes output before it leaves; Tier 1 applied after
//   --edge-redact + policy file: per-topic allow/block lists and per-topic redaction level overrides
//
// Claude Desktop config (~/.claude/claude_desktop_config.json):
//   {
//     "mcpServers": {
//       "hematite": { "command": "hematite", "args": ["--mcp-server"] }
//     }
//   }

use crate::agent::redact_audit::{AuditEntry, RedactMode};
use crate::agent::redact_policy::{load_policy, RedactPolicy, RedactionLevel};
use serde_json::{json, Value};
use std::collections::BTreeMap;
type Tier1Hits = BTreeMap<&'static str, usize>;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

const PROTOCOL_VERSION: &str = "2024-11-05";
const SERVER_NAME: &str = "hematite";
const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");

pub async fn run_mcp_server(
    edge_redact: bool,
    semantic_redact: bool,
    api_url: &str,
    semantic_url: &str,
    semantic_model: &str,
) -> anyhow::Result<()> {
    let mode_label = if semantic_redact {
        "semantic+regex"
    } else if edge_redact {
        "regex"
    } else {
        "none"
    };
    eprintln!(
        "[hematite mcp] server v{SERVER_VERSION} started (protocol {PROTOCOL_VERSION}, redact: {mode_label})"
    );

    let policy = load_policy();

    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();
    let mut reader = BufReader::new(stdin);
    let mut writer = tokio::io::BufWriter::new(stdout);
    let mut line = String::new();

    loop {
        line.clear();
        let n = reader.read_line(&mut line).await?;
        if n == 0 {
            break; // EOF — client disconnected
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let msg: Value = match serde_json::from_str(trimmed) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("[hematite mcp] parse error: {e}");
                send_parse_error(&mut writer).await?;
                continue;
            }
        };

        let method = match msg.get("method").and_then(|m| m.as_str()) {
            Some(m) => m.to_string(),
            None => continue,
        };

        let id = msg.get("id").cloned();

        match method.as_str() {
            "initialize" => {
                let resp = json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "protocolVersion": PROTOCOL_VERSION,
                        "capabilities": { "tools": {} },
                        "serverInfo": {
                            "name": SERVER_NAME,
                            "version": SERVER_VERSION,
                            "redactMode": mode_label,
                            "privacyNote": match mode_label {
                                "semantic+regex" => "Tier 2: local model summarizes output before sending; Tier 1 regex applied after. Raw data never forwarded if model is unreachable.",
                                "regex"          => "Tier 1: usernames, MACs, serials, hostnames, and credentials stripped before forwarding.",
                                _                => "No redaction active. Raw diagnostic output is forwarded as-is.",
                            }
                        }
                    }
                });
                send_response(&resp, &mut writer).await?;
            }

            "initialized" => {
                eprintln!("[hematite mcp] client initialized");
            }

            "ping" => {
                if let Some(id) = id {
                    let resp = json!({ "jsonrpc": "2.0", "id": id, "result": {} });
                    send_response(&resp, &mut writer).await?;
                }
            }

            "tools/list" => {
                if let Some(id) = id {
                    let resp = json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "result": { "tools": tool_list() }
                    });
                    send_response(&resp, &mut writer).await?;
                }
            }

            "tools/call" => {
                if let Some(id) = id {
                    let params = msg.get("params").cloned().unwrap_or(Value::Null);
                    let result =
                        dispatch_tool_call(&params, edge_redact, semantic_redact, api_url, semantic_url, semantic_model, &policy)
                            .await;
                    let resp = match result {
                        Ok(text) => json!({
                            "jsonrpc": "2.0",
                            "id": id,
                            "result": {
                                "content": [{ "type": "text", "text": text }],
                                "isError": false
                            }
                        }),
                        Err(e) => json!({
                            "jsonrpc": "2.0",
                            "id": id,
                            "result": {
                                "content": [{ "type": "text", "text": format!("Error: {e}") }],
                                "isError": true
                            }
                        }),
                    };
                    send_response(&resp, &mut writer).await?;
                }
            }

            other => {
                eprintln!("[hematite mcp] unknown method: {other}");
                if let Some(id) = id {
                    let resp = json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "error": { "code": -32601, "message": "Method not found" }
                    });
                    send_response(&resp, &mut writer).await?;
                }
            }
        }
    }

    eprintln!("[hematite mcp] server exiting (client disconnected)");
    Ok(())
}

async fn dispatch_tool_call(
    params: &Value,
    edge_redact: bool,
    semantic_redact: bool,
    api_url: &str,
    semantic_url: &str,
    semantic_model: &str,
    policy: &RedactPolicy,
) -> Result<String, String> {
    let name = params
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing tool name in tools/call params".to_string())?;

    // Strip args to declared schema fields only (jailbreak resistance: Phase 5)
    let args = sanitize_args(
        params
            .get("arguments")
            .cloned()
            .unwrap_or_else(|| Value::Object(Default::default())),
    );

    match name {
        "inspect_host" => {
            let topic = args
                .get("topic")
                .and_then(|v| v.as_str())
                .unwrap_or("summary");

            // Policy: blocked topics return a hard error — never run the inspection
            if policy.is_blocked(topic) {
                return Err(format!(
                    "Topic '{topic}' is blocked by the local redaction policy. \
                     Check .hematite/redact_policy.json."
                ));
            }

            // Run the inspection
            let raw = crate::tools::host_inspect::inspect_host(&args).await?;
            let raw_len = raw.len();

            // Determine effective redaction level.
            // When --semantic-redact is active, the default is Semantic unless the
            // policy file explicitly overrides the topic to "none" or "regex".
            let level = if semantic_redact {
                let per_topic = policy.redaction_level(topic, false); // false = no edge-redact default
                match per_topic {
                    RedactionLevel::None => RedactionLevel::Semantic,
                    other => other,
                }
            } else {
                policy.redaction_level(topic, edge_redact)
            };

            let (output, audit_mode, semantic_applied, tier1_hits) = match level {
                RedactionLevel::None => {
                    let labeled = format!(
                        "[hematite: no redaction active — raw diagnostic output]\n\n{raw}"
                    );
                    (labeled, RedactMode::None, false, Tier1Hits::new())
                }

                RedactionLevel::Regex => {
                    let r = crate::agent::edge_redact::redact(&raw);
                    (
                        format!("{}\n\n{}", r.summary_header, r.text),
                        RedactMode::Regex,
                        false,
                        r.tier1_hits,
                    )
                }

                RedactionLevel::Semantic => {
                    match crate::agent::semantic_redact::summarize(&raw, topic, semantic_url, Some(semantic_model)).await {
                        Ok(summary) => {
                            // Tier 1 as safety net after semantic pass
                            let r = crate::agent::edge_redact::redact(&summary);
                            let header = format!(
                                "[edge-redact: semantic+regex — local model summary applied{}]\n\n",
                                if r.redaction_count > 0 {
                                    format!("; {} tier1 residual hit(s)", r.redaction_count)
                                } else {
                                    String::new()
                                }
                            );
                            (
                                format!("{header}{}", r.text),
                                RedactMode::Semantic,
                                true,
                                r.tier1_hits,
                            )
                        }
                        Err(e) => {
                            // Fail-safe: return the error, never the raw data
                            return Err(e);
                        }
                    }
                }
            };

            // Phase 4: write audit entry (non-blocking — errors go to stderr only)
            let tier1_hits_owned: BTreeMap<String, usize> = tier1_hits
                .into_iter()
                .map(|(k, v)| (k.to_string(), v))
                .collect();
            crate::agent::redact_audit::record(&AuditEntry {
                topic: topic.to_string(),
                mode: audit_mode,
                tier1_hits: tier1_hits_owned,
                semantic_applied,
                input_chars: raw_len,
                output_chars: output.len(),
                caller_pid: std::process::id(),
            });

            Ok(output)
        }

        other => Err(format!("Unknown tool: '{other}'")),
    }
}

/// Strip MCP call arguments to the declared schema fields.
/// Unknown keys are silently dropped — they cannot influence tool behavior.
fn sanitize_args(args: Value) -> Value {
    const ALLOWED: &[&str] = &[
        "topic",
        "host",
        "port",
        "name",
        "type",
        "path",
        "process",
        "event_id",
        "log",
        "source",
        "hours",
        "level",
        "issue",
        "max_entries",
    ];
    match args {
        Value::Object(map) => {
            let cleaned: serde_json::Map<String, Value> = map
                .into_iter()
                .filter(|(k, _)| ALLOWED.contains(&k.as_str()))
                .collect();
            Value::Object(cleaned)
        }
        other => other,
    }
}

fn tool_list() -> Value {
    json!([
        {
            "name": "inspect_host",
            "description": "Run a read-only diagnostic inspection of the local machine. Returns grounded data from 116+ topics covering SysAdmin, Network Admin, hardware, security, and developer tooling. No mutations — all reads. Works on Windows, Linux, and macOS.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "topic": {
                        "type": "string",
                        "description": "The inspection topic. Core topics: summary, processes, services, ports, connections, network, storage, hardware, health_report, security, updates, pending_reboot, disk_health, battery, recent_crashes, app_crashes, scheduled_tasks, dev_conflicts, connectivity, wifi, vpn, proxy, firewall_rules, traceroute, dns_cache, arp, route_table, os_config, resource_load, env, hosts_file, docker, wsl, ssh, installed_software, git_config, databases, user_accounts, audit_policy, shares, dns_servers, bitlocker, rdp, shadow_copies, pagefile, windows_features, printers, winrm, network_stats, udp_ports, gpo, certificates, integrity, domain, device_health, drivers, peripherals, sessions, thermal, activation, patch_history, hyperv, ip_config, overclocker, event_query, display_config, ntp, cpu_power, credentials, tpm, latency, network_adapter, dhcp, mtu, ipv6, tcp_params, wlan_profiles, ipsec, netbios, nic_teaming, snmp, port_test, network_profile, audio, bluetooth, camera, sign_in, installer_health, onedrive, browser_health, identity_auth, outlook, teams, windows_backup, search_index, lan_discovery, toolchains, path, env_doctor, fix_plan, repo_doctor, directory, disk_benchmark, desktop, downloads, disk, permissions, login_history, share_access, registry_audit, ad_user, dns_lookup"
                    },
                    "host": {
                        "type": "string",
                        "description": "Target host (for traceroute, port_test, dns_lookup)"
                    },
                    "port": {
                        "type": "integer",
                        "description": "Port number (for port_test)"
                    },
                    "name": {
                        "type": "string",
                        "description": "Hostname to resolve (for dns_lookup)"
                    },
                    "type": {
                        "type": "string",
                        "description": "DNS record type (for dns_lookup): A, AAAA, MX, TXT, SRV"
                    },
                    "path": {
                        "type": "string",
                        "description": "File path (for directory, disk, permissions, share_access)"
                    },
                    "process": {
                        "type": "string",
                        "description": "Process name filter (for app_crashes)"
                    },
                    "event_id": {
                        "type": "integer",
                        "description": "Windows Event ID to filter on (for event_query)"
                    },
                    "log": {
                        "type": "string",
                        "description": "Event log name (for event_query): System, Application, Security"
                    },
                    "source": {
                        "type": "string",
                        "description": "Event source/provider name (for event_query)"
                    },
                    "hours": {
                        "type": "integer",
                        "description": "Time window in hours (for event_query, default 24)"
                    },
                    "level": {
                        "type": "string",
                        "description": "Event severity level (for event_query): Error, Warning, Information"
                    },
                    "issue": {
                        "type": "string",
                        "description": "Problem description (for fix_plan)"
                    },
                    "max_entries": {
                        "type": "integer",
                        "description": "Maximum results to return (default 20)"
                    }
                },
                "required": ["topic"]
            }
        }
    ])
}

async fn send_response(
    resp: &Value,
    writer: &mut tokio::io::BufWriter<tokio::io::Stdout>,
) -> anyhow::Result<()> {
    let mut bytes = serde_json::to_vec(resp)?;
    bytes.push(b'\n');
    writer.write_all(&bytes).await?;
    writer.flush().await?;
    Ok(())
}

async fn send_parse_error(
    writer: &mut tokio::io::BufWriter<tokio::io::Stdout>,
) -> anyhow::Result<()> {
    let resp = json!({
        "jsonrpc": "2.0",
        "id": null,
        "error": { "code": -32700, "message": "Parse error" }
    });
    send_response(&resp, writer).await
}
