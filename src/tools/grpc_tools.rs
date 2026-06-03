use serde_json::{json, Value};

pub fn make_schema() -> Value {
    json!({
        "name": "grpc_tools",
        "description": "Look up gRPC status codes, explain common error causes, and list well-known gRPC metadata headers without external utilities.",
        "parameters": {
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["status", "list", "headers", "explain"],
                    "description": "status=look up code by number or name, list=all 17 status codes, headers=well-known gRPC metadata headers, explain=detailed cause/fix for a status code"
                },
                "code": {
                    "type": "string",
                    "description": "gRPC status code number (0-16) or name (e.g. NOT_FOUND, UNAVAILABLE, DEADLINE_EXCEEDED)"
                }
            }
        }
    })
}

struct GrpcStatus {
    code: u8,
    name: &'static str,
    http_equiv: &'static str,
    summary: &'static str,
    causes: &'static str,
    fix: &'static str,
}

const STATUSES: &[GrpcStatus] = &[
    GrpcStatus {
        code: 0,
        name: "OK",
        http_equiv: "200",
        summary: "Success — operation completed successfully.",
        causes: "N/A",
        fix: "No action needed.",
    },
    GrpcStatus {
        code: 1,
        name: "CANCELLED",
        http_equiv: "499",
        summary: "Client cancelled the request.",
        causes: "Client called cancel() or disconnected. Context deadline exceeded on client side.",
        fix: "Check client timeout settings. Implement retry with cancellation handling.",
    },
    GrpcStatus {
        code: 2,
        name: "UNKNOWN",
        http_equiv: "500",
        summary: "Unknown server error.",
        causes:
            "Server threw an unhandled exception. Error from a different protocol wrapped in gRPC.",
        fix: "Check server logs for stack traces. Add proper error handling on server.",
    },
    GrpcStatus {
        code: 3,
        name: "INVALID_ARGUMENT",
        http_equiv: "400",
        summary: "Client sent invalid arguments.",
        causes: "Required field missing, value out of range, malformed input.",
        fix: "Validate request before sending. Check proto field constraints and oneof rules.",
    },
    GrpcStatus {
        code: 4,
        name: "DEADLINE_EXCEEDED",
        http_equiv: "504",
        summary: "Deadline expired before operation completed.",
        causes: "Request took longer than the deadline. Network latency. Server overloaded.",
        fix: "Increase client deadline. Check server performance. Add circuit breaker pattern.",
    },
    GrpcStatus {
        code: 5,
        name: "NOT_FOUND",
        http_equiv: "404",
        summary: "Requested resource not found.",
        causes: "Entity does not exist. Wrong ID or key. Data deleted.",
        fix: "Check IDs in request. Verify the resource exists before calling.",
    },
    GrpcStatus {
        code: 6,
        name: "ALREADY_EXISTS",
        http_equiv: "409",
        summary: "Resource already exists.",
        causes: "Duplicate creation attempt. Idempotency key conflict.",
        fix: "Check before creating. Use idempotency keys. Handle conflict in client.",
    },
    GrpcStatus {
        code: 7,
        name: "PERMISSION_DENIED",
        http_equiv: "403",
        summary: "Caller lacks permission.",
        causes: "Auth token valid but insufficient permissions. Role/policy misconfiguration.",
        fix: "Check caller's IAM roles or ACLs. Verify service account permissions.",
    },
    GrpcStatus {
        code: 8,
        name: "RESOURCE_EXHAUSTED",
        http_equiv: "429",
        summary: "Resource quota exceeded or rate limit hit.",
        causes: "Too many requests. Memory/CPU/disk exhausted. Per-user quota hit.",
        fix: "Implement exponential backoff and retry. Request quota increase. Shed load.",
    },
    GrpcStatus {
        code: 9,
        name: "FAILED_PRECONDITION",
        http_equiv: "400",
        summary: "System not in required state for the operation.",
        causes: "Trying to delete non-empty directory. Read-after-write ordering violation.",
        fix: "Ensure prerequisites are met before retrying. Check system state.",
    },
    GrpcStatus {
        code: 10,
        name: "ABORTED",
        http_equiv: "409",
        summary: "Operation aborted due to concurrency conflict.",
        causes: "Transaction conflict. Check-and-set failed. Sequencer token mismatch.",
        fix: "Retry the full transaction from the beginning. Use optimistic locking.",
    },
    GrpcStatus {
        code: 11,
        name: "OUT_OF_RANGE",
        http_equiv: "400",
        summary: "Operation attempted past valid range.",
        causes: "Reading past end of file. Page offset beyond list size.",
        fix: "Check bounds before requesting. Adjust pagination parameters.",
    },
    GrpcStatus {
        code: 12,
        name: "UNIMPLEMENTED",
        http_equiv: "501",
        summary: "Operation not implemented or supported.",
        causes: "Server stub not implemented. Calling a method on wrong service version.",
        fix: "Check service version. Implement the missing RPC handler.",
    },
    GrpcStatus {
        code: 13,
        name: "INTERNAL",
        http_equiv: "500",
        summary: "Internal server error.",
        causes: "Broken invariant. Memory corruption. Unexpected state.",
        fix: "Check server logs. This is a serious bug — file a bug report.",
    },
    GrpcStatus {
        code: 14,
        name: "UNAVAILABLE",
        http_equiv: "503",
        summary: "Service is currently unavailable.",
        causes:
            "Server is down or restarting. Network partition. Load balancer health-check failure.",
        fix: "Implement exponential backoff retry. Check server health and connectivity.",
    },
    GrpcStatus {
        code: 15,
        name: "DATA_LOSS",
        http_equiv: "500",
        summary: "Unrecoverable data loss or corruption.",
        causes: "Disk failure. Incomplete write. Corruption in storage layer.",
        fix: "Restore from backup. Alert on-call immediately. Do not retry blindly.",
    },
    GrpcStatus {
        code: 16,
        name: "UNAUTHENTICATED",
        http_equiv: "401",
        summary: "Request lacks valid authentication credentials.",
        causes: "Missing or expired token. Wrong API key. Certificate mismatch.",
        fix: "Refresh credentials. Check token expiry. Verify TLS certificate chain.",
    },
];

struct GrpcHeader {
    name: &'static str,
    direction: &'static str,
    description: &'static str,
}

const HEADERS: &[GrpcHeader] = &[
    GrpcHeader { name: "content-type",          direction: "request/response", description: "Must be 'application/grpc', 'application/grpc+proto', or 'application/grpc+json'. Invalid value causes GRPC_STATUS_415." },
    GrpcHeader { name: "grpc-status",           direction: "trailer",          description: "Numeric gRPC status code (0–16). Sent in HTTP/2 trailers after the response body." },
    GrpcHeader { name: "grpc-message",          direction: "trailer",          description: "URL-encoded human-readable status detail string. Optional; accompanies grpc-status." },
    GrpcHeader { name: "grpc-timeout",          direction: "request",          description: "Request timeout in units: H(ours) M(inutes) S(econds) m(illiseconds) u(microseconds) n(anoseconds). E.g. '5S' = 5 seconds." },
    GrpcHeader { name: "grpc-encoding",         direction: "request/response", description: "Compression algorithm for message body: identity (none), gzip, deflate, snappy, br." },
    GrpcHeader { name: "grpc-accept-encoding",  direction: "response",         description: "Comma-separated list of compression algorithms the server accepts." },
    GrpcHeader { name: "grpc-message-type",     direction: "request",          description: "Fully qualified proto message type (package.MessageName). Optional; used for reflection." },
    GrpcHeader { name: "grpc-status-details-bin", direction: "trailer",       description: "Base64-encoded google.rpc.Status proto with extended error details (errors, metadata, debug info)." },
    GrpcHeader { name: "authorization",         direction: "request",          description: "Standard HTTP Authorization header. Bearer tokens passed here for auth interceptors." },
    GrpcHeader { name: "x-grpc-web",            direction: "request",          description: "Set to '1' by gRPC-Web clients (browser). Proxies like Envoy translate gRPC-Web to gRPC." },
    GrpcHeader { name: "te",                    direction: "request",          description: "Must be 'trailers' in HTTP/2 gRPC requests. Signals that the client accepts HTTP/2 trailers." },
    GrpcHeader { name: "grpc-previous-rpc-attempts", direction: "request",    description: "Number of prior RPC attempts for this same call (retry support). Starts at 1 on first retry." },
    GrpcHeader { name: "grpc-retry-pushback-ms",direction: "trailer",         description: "Milliseconds client should wait before retrying. Negative value means do not retry." },
];

fn find_status(code_str: &str) -> Option<&'static GrpcStatus> {
    let lower = code_str.to_uppercase();
    // Try numeric first
    if let Ok(n) = code_str.parse::<u8>() {
        return STATUSES.iter().find(|s| s.code == n);
    }
    // Try name
    STATUSES.iter().find(|s| s.name == lower.as_str())
}

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("status");

    match action {
        "list" => action_list(),
        "headers" => action_headers(),
        "status" | "explain" => {
            let code_str = args.get("code").and_then(|v| v.as_str()).unwrap_or("");
            if code_str.is_empty() {
                // Show list if no code given
                return action_list();
            }
            match find_status(code_str) {
                Some(s) => {
                    if action == "explain" {
                        action_explain(s)
                    } else {
                        action_status(s)
                    }
                }
                None => Err(format!(
                    "Unknown gRPC status code '{}'. Use a number 0–16 or name like NOT_FOUND.",
                    code_str
                )),
            }
        }
        _ => Err(format!(
            "Unknown action '{}'. Use: status, list, headers, explain",
            action
        )),
    }
}

fn action_list() -> Result<String, String> {
    let mut out = String::new();
    out.push_str("## gRPC Status Codes\n\n");
    out.push_str("Code  Name                    HTTP   Summary\n");
    out.push_str("────  ──────────────────────  ─────  ──────────────────────────────────────\n");
    for s in STATUSES {
        out.push_str(&format!(
            "{:<5} {:<23} {:<6} {}\n",
            s.code,
            s.name,
            s.http_equiv,
            s.summary.split('.').next().unwrap_or("")
        ));
    }
    out.push_str("\nUse 'explain' action with 'code' for detailed causes and fixes.\n");
    Ok(out)
}

fn action_status(s: &GrpcStatus) -> Result<String, String> {
    let mut out = String::new();
    out.push_str(&format!(
        "## gRPC {} ({}) — HTTP {}\n\n{}\n",
        s.name, s.code, s.http_equiv, s.summary
    ));
    Ok(out)
}

fn action_explain(s: &GrpcStatus) -> Result<String, String> {
    let mut out = String::new();
    out.push_str(&format!(
        "## gRPC {} ({})\n\nHTTP Equivalent: {}\n\n**Description:** {}\n\n**Common Causes:**\n{}\n\n**How to Fix:**\n{}\n",
        s.name, s.code, s.http_equiv, s.summary,
        s.causes.split(". ").map(|l| format!("- {}", l)).collect::<Vec<_>>().join("\n"),
        s.fix.split(". ").filter(|l| !l.is_empty()).map(|l| format!("- {}", l)).collect::<Vec<_>>().join("\n"),
    ));

    // Retryability
    let retryable = matches!(s.code, 1 | 4 | 8 | 14);
    let cautious = matches!(s.code, 10);
    let not_retryable = !retryable && !cautious;
    if retryable {
        out.push_str("\n**Retryable:** Yes — use exponential backoff with jitter.\n");
    } else if cautious {
        out.push_str("\n**Retryable:** Yes — but restart the full transaction from scratch.\n");
    } else if not_retryable {
        out.push_str("\n**Retryable:** No — fix the underlying issue before retrying.\n");
    }
    Ok(out)
}

fn action_headers() -> Result<String, String> {
    let mut out = String::new();
    out.push_str("## gRPC Metadata Headers\n\n");
    out.push_str("Header                         Dir               Description\n");
    out.push_str("─────────────────────────────  ────────────────  ──────────────────────────────────────────\n");
    for h in HEADERS {
        out.push_str(&format!(
            "{:<30} {:<17} {}\n",
            h.name,
            h.direction,
            &h.description[..h.description.len().min(60)]
        ));
    }
    out.push_str("\nNote: 'te: trailers' is required by gRPC spec in HTTP/2 requests.\ngRPC-Web uses HTTP/1.1 and requires an Envoy or similar proxy for translation.\n");
    Ok(out)
}
