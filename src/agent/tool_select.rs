use crate::agent::routing::*;
use crate::agent::types::ToolDefinition;

/// Tools always included regardless of query content.
const CORE_TOOL_NAMES: &[&str] = &[
    "read_file",
    "write_file",
    "edit_file",
    "patch_hunk",
    "multi_search_replace",
    "list_files",
    "grep_files",
    "tail_file",
    "create_directory",
    "inspect_host",
    "run_code",
    "verify_build",
    "research_web",
    "fetch_docs",
];

/// Shell is core but excluded when a pure native tool covers the query.
const SHELL_TOOL: &str = "shell";

/// LSP tools included when the query looks like a navigation or rename task.
const LSP_TOOLS: &[&str] = &[
    "lsp_definitions",
    "lsp_references",
    "lsp_hover",
    "lsp_search_symbol",
    "lsp_rename_symbol",
    "lsp_get_diagnostics",
];

/// Git tools included when the query touches version control.
const GIT_TOOLS: &[&str] = &["git_status", "git_diff", "git_commit", "git_log"];

/// Rough token estimate: serialized JSON length / 4.
fn estimate_tokens(tools: &[ToolDefinition]) -> usize {
    tools
        .iter()
        .map(|t| {
            let name_len = t.function.name.len();
            let desc_len = t.function.description.len();
            let params_len = t.function.parameters.to_string().len();
            (name_len + desc_len + params_len + 32) / 4
        })
        .sum()
}

/// Select tools relevant to the current query, capped at ≤15% of `context_limit` tokens.
///
/// Always includes the core set. Adds routing-detected tool families on top.
/// When `shell_has_native_replacement` fires, shell is excluded from the core set.
/// Trims to the budget ceiling from least-priority entries first.
pub fn select_tools(
    all_tools: &[ToolDefinition],
    user_input: &str,
    context_limit: usize,
) -> Vec<ToolDefinition> {
    // Floor: never go below 2048 so we don't starve tiny context windows of all tooling.
    let effective_limit = context_limit.max(2048);
    // Cap at 15% of the live context window.
    let token_budget = effective_limit * 15 / 100;

    let tool_map: std::collections::HashMap<&str, &ToolDefinition> = all_tools
        .iter()
        .map(|t| (t.function.name.as_str(), t))
        .collect();

    let mut names: Vec<&str> = Vec::with_capacity(64);
    let mut seen: std::collections::HashSet<&str> = std::collections::HashSet::new();

    // Deduplicating push — use a macro so we don't need a closure that holds a borrow.
    macro_rules! push {
        ($n:expr) => {
            if seen.insert($n) {
                names.push($n);
            }
        };
    }

    // ── 1. Core set ─────────────────────────────────────────────────────────────
    for &n in CORE_TOOL_NAMES {
        push!(n);
    }
    if !shell_has_native_replacement(user_input) {
        push!(SHELL_TOOL);
    }

    // ── 2. Routing-detected tool families ───────────────────────────────────────
    macro_rules! route {
        ($pred:expr, $($tool:literal),+) => {
            if $pred(user_input) {
                $( push!($tool); )+
            }
        };
    }

    route!(needs_http_request, "http_request");
    route!(needs_docker_ops, "docker_ops");
    route!(needs_docker_compose_tools, "docker_compose_tools");
    route!(needs_dockerfile_tools, "dockerfile_tools");
    route!(needs_secret_scan, "secret_scanner");
    route!(needs_json_tools, "json_tools");
    route!(needs_yaml_tools, "yaml_tools");
    route!(needs_toml_tools, "toml_tools");
    route!(needs_xml_tools, "xml_tools");
    route!(needs_csv_tools, "csv_tools");
    route!(needs_regex_tools, "regex_tools");
    route!(needs_diff_tools, "diff_tools");
    route!(needs_diff3_tools, "diff3_tools");
    route!(needs_encode_tools, "encode_tools");
    route!(needs_hash_tools, "hash_tools");
    route!(needs_hex_tools, "hex_tools");
    route!(needs_ip_tools, "ip_tools");
    route!(needs_subnet_tools, "subnet_tools");
    route!(needs_url_tools, "url_tools");
    route!(needs_sqlite_tools, "sqlite_tools");
    route!(needs_archive_tools, "archive_tools");
    route!(needs_k8s_tools, "k8s_tools");
    route!(needs_helm_tools, "helm_tools");
    route!(needs_uuid_gen, "uuid_gen");
    route!(needs_date_tools, "date_tools");
    route!(needs_duration_tools, "duration_tools");
    route!(needs_semver_tools, "semver_tools");
    route!(needs_jwt_tools, "jwt_tools");
    route!(needs_password_gen, "password_gen");
    route!(needs_har_tools, "har_tools");
    route!(needs_ical_tools, "ical_tools");
    route!(needs_text_tools, "text_tools");
    route!(needs_number_tools, "number_tools");
    route!(needs_cron_tools, "cron_tools");
    route!(needs_color_tools, "color_tools");
    route!(needs_markdown_tools, "markdown_tools");
    route!(needs_markdown_gen_tools, "markdown_gen_tools");
    route!(needs_line_tools, "line_tools");
    route!(needs_path_tools, "path_tools");
    route!(needs_table_tools, "table_tools");
    route!(needs_ini_tools, "ini_tools");
    route!(needs_ansi_tools, "ansi_tools");
    route!(needs_template_tools, "template_tools");
    route!(needs_dotenv_tools, "dotenv_tools");
    route!(needs_char_tools, "char_tools");
    route!(needs_rss_tools, "rss_tools");
    route!(needs_keyval_tools, "keyval_tools");
    route!(needs_stat_tools, "stat_tools");
    route!(needs_nginx_conf_tools, "nginx_conf_tools");
    route!(needs_openapi_tools, "openapi_tools");
    route!(needs_net_lookup_tools, "net_lookup_tools");
    route!(needs_size_tools, "size_tools");
    route!(needs_validate_tools, "validate_tools");
    route!(needs_money_tools, "money_tools");
    route!(needs_financial_tools, "financial_tools");
    route!(needs_token_tools, "token_tools");
    route!(needs_mime_tools, "mime_tools");
    route!(needs_robots_txt_tools, "robots_txt_tools");
    route!(needs_sitemap_tools, "sitemap_tools");
    route!(needs_make_tools, "make_tools");
    route!(needs_changelog_tools, "changelog_tools");
    route!(needs_github_actions_tools, "github_actions_tools");
    route!(needs_gitignore_tools, "gitignore_tools");
    route!(needs_license_tools, "license_tools");
    route!(needs_ssh_config_tools, "ssh_config_tools");
    route!(needs_systemd_tools, "systemd_tools");
    route!(needs_log_parse_tools, "log_parse_tools");
    route!(needs_csp_tools, "csp_tools");
    route!(needs_http_status_tools, "http_status_tools");
    route!(needs_http_parse_tools, "http_parse_tools");
    route!(needs_jq_tools, "jq_tools");
    route!(needs_glob_tools, "glob_tools");
    route!(needs_graph_tools, "graph_tools");
    route!(needs_matrix_tools, "matrix_tools");
    route!(needs_sql_tools, "sql_tools");
    route!(needs_proto_tools, "proto_tools");
    route!(needs_terraform_tools, "terraform_tools");
    route!(needs_git_log_tools, "git_log_tools");
    route!(needs_journald_tools, "journald_tools");
    route!(needs_tsconfig_tools, "tsconfig_tools");
    route!(needs_eslint_tools, "eslint_tools");
    route!(needs_prettier_tools, "prettier_tools");
    route!(needs_jest_tools, "jest_tools");
    route!(needs_stylelint_tools, "stylelint_tools");
    route!(needs_babel_tools, "babel_tools");
    route!(needs_conda_tools, "conda_tools");
    route!(needs_cite_tools, "cite_tools");
    route!(needs_latex_tools, "latex_tools");
    route!(needs_astro_tools, "astro_tools");
    route!(needs_signal_tools, "signal_tools");
    route!(needs_thermo_tools, "thermo_tools");
    route!(needs_optics_tools, "optics_tools");
    route!(needs_mechanics_tools, "mechanics_tools");
    route!(needs_circuit_tools, "circuit_tools");
    route!(needs_quantum_tools, "quantum_tools");
    route!(needs_em_tools, "em_tools");
    route!(needs_relativity_tools, "relativity_tools");
    route!(needs_nuclear_tools, "nuclear_tools");
    route!(needs_cors_tools, "cors_tools");
    route!(needs_coverage_tools, "coverage_tools");
    route!(needs_strace_tools, "strace_tools");
    route!(needs_cmake_tools, "cmake_tools");
    route!(needs_dotnet_tools, "dotnet_tools");
    route!(needs_maven_tools, "maven_tools");
    route!(needs_gradle_tools, "gradle_tools");
    route!(needs_go_mod_tools, "go_mod_tools");
    route!(needs_requirements_tools, "requirements_tools");
    route!(needs_web_manifest_tools, "web_manifest_tools");
    route!(needs_json_patch_tools, "json_patch_tools");
    route!(needs_sort_tools, "sort_tools");
    route!(needs_compression_tools, "compression_tools");
    route!(needs_trie_tools, "trie_tools");
    route!(needs_stack_tools, "stack_tools");
    route!(needs_acoustics_tools, "acoustics_tools");
    route!(needs_materials_tools, "materials_tools");
    route!(needs_pe_tools, "pe_tools");
    route!(needs_macho_tools, "macho_tools");
    route!(needs_pcap_tools, "pcap_tools");
    route!(needs_class_tools, "class_tools");
    route!(needs_dex_tools, "dex_tools");
    route!(needs_tls_tools, "tls_tools");
    route!(needs_protobuf_wire_tools, "protobuf_wire_tools");
    route!(needs_ssh_key_tools, "ssh_key_tools");
    route!(needs_wireguard_tools, "wireguard_tools");
    route!(needs_prometheus_tools, "prometheus_tools");
    route!(needs_http_cache_tools, "http_cache_tools");
    route!(needs_webhook_tools, "webhook_tools");
    route!(needs_jwk_tools, "jwk_tools");
    route!(needs_gitlab_ci_tools, "gitlab_ci_tools");
    route!(needs_junit_tools, "junit_tools");
    route!(needs_ansible_tools, "ansible_tools");
    route!(needs_grpc_tools, "grpc_tools");
    route!(needs_haproxy_tools, "haproxy_tools");
    route!(needs_cvss_tools, "cvss_tools");
    route!(needs_nmap_tools, "nmap_tools");
    route!(needs_postman_tools, "postman_tools");
    route!(needs_ldif_tools, "ldif_tools");
    route!(needs_iptables_tools, "iptables_tools");
    route!(needs_spdx_tools, "spdx_tools");
    route!(needs_aws_tools, "aws_tools");
    route!(needs_curl_tools, "curl_tools");
    route!(needs_oauth_tools, "oauth_tools");
    route!(needs_saml_tools, "saml_tools");
    route!(needs_multipart_tools, "multipart_tools");
    route!(needs_openid_tools, "openid_tools");
    route!(needs_exif_tools, "exif_tools");
    route!(needs_office_tools, "office_tools");
    route!(needs_font_tools, "font_tools");
    route!(needs_svg_tools, "svg_tools");
    route!(needs_image_tools, "image_tools");
    route!(needs_audio_file_tools, "audio_file_tools");
    route!(needs_video_file_tools, "video_file_tools");
    route!(needs_pdf_tools, "pdf_tools");
    route!(needs_epub_tools, "epub_tools");
    route!(needs_sbom_tools, "sbom_tools");

    // Git tools when a git-related query is detected.
    if needs_github_ops(user_input) {
        for &n in GIT_TOOLS {
            push!(n);
        }
    }

    // LSP navigation when the query asks about definitions, references, symbols.
    if needs_lsp_tools(user_input) {
        for &n in LSP_TOOLS {
            push!(n);
        }
    }

    // ── 3. Resolve names → ToolDefinition (drop unknown names silently) ─────────
    let mut selected: Vec<ToolDefinition> = names
        .iter()
        .filter_map(|n| tool_map.get(n).copied().cloned())
        .collect();

    // ── 4. Enforce token budget — trim from the tail (lowest-priority first) ────
    // Floor at 4 so even tiny (8K) contexts always get a minimum viable toolset.
    // Core tools are ordered most→least essential, so trimming the tail first drops
    // lower-value entries (fetch_docs, research_web, ...) before read_file / edit_file.
    const ABSOLUTE_MIN: usize = 4;
    while estimate_tokens(&selected) > token_budget && selected.len() > ABSOLUTE_MIN {
        selected.pop();
    }

    selected
}

/// Returns `true` when the user is asking about code navigation, symbol lookup, or rename.
fn needs_lsp_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    lower.contains("go to definition")
        || lower.contains("find references")
        || lower.contains("rename symbol")
        || lower.contains("workspace symbol")
        || lower.contains("lsp")
        || lower.contains("diagnostic")
        || lower.contains("hover doc")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::tool_registry::get_tools;

    fn all_tools() -> Vec<ToolDefinition> {
        get_tools()
    }

    /// For every standard context tier, the selected tool list must fit in ≤15% of the window.
    #[test]
    fn token_budget_respected_across_context_tiers() {
        let tiers = [8_192usize, 16_384, 32_768, 131_072];
        let all = all_tools();
        for ctx in tiers {
            let budget = ctx * 15 / 100;
            let selected = select_tools(&all, "read this file and edit it", ctx);
            let cost = estimate_tokens(&selected);
            assert!(
                cost <= budget,
                "context={ctx}: tool cost {cost} tokens exceeds budget {budget}"
            );
        }
    }

    /// Core tools survive on a context large enough to fit them all.
    #[test]
    fn core_tools_always_present() {
        let all = all_tools();
        // Use 32K where the full core set fits comfortably within the 15% budget.
        let selected = select_tools(&all, "fix the bug in main.rs", 32_768);
        let names: Vec<&str> = selected.iter().map(|t| t.function.name.as_str()).collect();
        for &core in CORE_TOOL_NAMES {
            // Only assert presence of tools that actually exist in the registry.
            if all.iter().any(|t| t.function.name == core) {
                assert!(
                    names.contains(&core),
                    "Core tool '{core}' missing from selected set (32K context)"
                );
            }
        }
    }

    #[test]
    fn routing_adds_json_tools_for_json_query() {
        let all = all_tools();
        // Use 128K context so budget doesn't interfere with verifying routing logic.
        // "format json" matches needs_json_tools; token budget at 128K is ~19K, well above core set.
        let selected = select_tools(&all, "format json output", 131_072);
        let names: Vec<&str> = selected.iter().map(|t| t.function.name.as_str()).collect();
        assert!(
            names.contains(&"json_tools"),
            "json_tools should be selected for a JSON query"
        );
    }

    #[test]
    fn shell_excluded_when_native_tool_fires() {
        let all = all_tools();
        let selected = select_tools(&all, "parse this csv file", 32_768);
        let names: Vec<&str> = selected.iter().map(|t| t.function.name.as_str()).collect();
        assert!(
            !names.contains(&"shell"),
            "shell should be excluded when csv_tools covers the query"
        );
    }

    #[test]
    fn shell_included_for_general_coding_query() {
        let all = all_tools();
        let selected = select_tools(&all, "run cargo build and fix errors", 32_768);
        let names: Vec<&str> = selected.iter().map(|t| t.function.name.as_str()).collect();
        assert!(
            names.contains(&"shell"),
            "shell should be included for a build/run query"
        );
    }
}
