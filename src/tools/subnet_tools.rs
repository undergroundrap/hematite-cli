use serde_json::{json, Value};
use std::fmt::Write as _;

pub fn make_schema() -> Value {
    json!({
        "name": "subnet_tools",
        "description": "Extended IPv4/IPv6 subnet and CIDR operations without external utilities. \
            Actions: split (divide a CIDR into N equal sub-networks), \
            supernet (smallest CIDR containing all given IPs or CIDRs), \
            hosts (enumerate all usable host IPs in a CIDR — paginated with offset/limit), \
            aggregate (summarize a list of IPs/CIDRs into the minimal covering set), \
            overlap (detect overlapping ranges in a list of CIDRs), \
            contains (check if every IP in a list is within a CIDR), \
            range (convert start/end IP range to CIDR list). \
            Pass cidr (e.g. '192.168.1.0/24') or ips array. \
            Example: subnet_tools(action: 'split', cidr: '10.0.0.0/8', n: 4) \
            or subnet_tools(action: 'hosts', cidr: '192.168.1.0/28', limit: 10)",
        "parameters": {
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "description": "split | supernet | hosts | aggregate | overlap | contains | range (default: split when cidr+n given, hosts otherwise)"
                },
                "cidr": { "type": "string", "description": "CIDR notation e.g. '10.0.0.0/8'" },
                "ips": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Array of IPs or CIDRs for aggregate/supernet/overlap/contains actions"
                },
                "n": { "type": "integer", "description": "Number of subnets for split action (must be power of 2)" },
                "limit": { "type": "integer", "description": "Max hosts to list (default 50, max 256)" },
                "offset": { "type": "integer", "description": "Start offset for hosts enumeration (default 0)" },
                "start": { "type": "string", "description": "Start IP for range action" },
                "end": { "type": "string", "description": "End IP for range action" }
            }
        }
    })
}

// ── IPv4 helpers ──────────────────────────────────────────────────────────────

fn parse_ipv4(s: &str) -> Option<u32> {
    let s = s.trim();
    let parts: Vec<&str> = s.split('.').collect();
    if parts.len() != 4 {
        return None;
    }
    let mut n = 0u32;
    for p in &parts {
        let v: u8 = p.parse().ok()?;
        n = (n << 8) | v as u32;
    }
    Some(n)
}

fn fmt_ipv4(ip: u32) -> String {
    format!(
        "{}.{}.{}.{}",
        ip >> 24,
        (ip >> 16) & 0xff,
        (ip >> 8) & 0xff,
        ip & 0xff
    )
}

fn ip_type(ip: u32) -> &'static str {
    if ip >> 24 == 10 {
        return "private (Class A)";
    }
    if ip >> 20 == 0xAC1 {
        return "private (Class B)";
    } // 172.16-31
    if ip >> 16 == 0xC0A8 {
        return "private (Class C)";
    } // 192.168
    if ip >> 24 == 127 {
        return "loopback";
    }
    if ip >> 24 >= 224 {
        return "multicast/reserved";
    }
    "public"
}

struct Cidr4 {
    network: u32,
    prefix: u8,
}

impl Cidr4 {
    fn parse(s: &str) -> Option<Self> {
        let (ip_s, prefix_s) = s.split_once('/')?;
        let prefix: u8 = prefix_s.trim().parse().ok()?;
        if prefix > 32 {
            return None;
        }
        let ip = parse_ipv4(ip_s)?;
        let mask = prefix_mask(prefix);
        Some(Cidr4 {
            network: ip & mask,
            prefix,
        })
    }

    fn mask(&self) -> u32 {
        prefix_mask(self.prefix)
    }
    fn broadcast(&self) -> u32 {
        self.network | !self.mask()
    }
    fn host_count(&self) -> u64 {
        if self.prefix >= 32 {
            return 1;
        }
        if self.prefix == 31 {
            return 2;
        }
        (1u64 << (32 - self.prefix)) - 2
    }
    #[allow(dead_code)]
    fn contains_ip(&self, ip: u32) -> bool {
        ip & self.mask() == self.network
    }
    fn contains_cidr(&self, other: &Cidr4) -> bool {
        other.network & self.mask() == self.network && other.prefix >= self.prefix
    }
}

impl std::fmt::Display for Cidr4 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}", fmt_ipv4(self.network), self.prefix)
    }
}

fn prefix_mask(prefix: u8) -> u32 {
    if prefix == 0 {
        0
    } else {
        !0u32 << (32 - prefix)
    }
}

fn cidr_or_ip_to_cidr4(s: &str) -> Option<Cidr4> {
    if s.contains('/') {
        Cidr4::parse(s)
    } else {
        // bare IP → /32
        let ip = parse_ipv4(s)?;
        Some(Cidr4 {
            network: ip,
            prefix: 32,
        })
    }
}

// ── Actions ───────────────────────────────────────────────────────────────────

fn action_split(cidr_s: &str, n: u64) -> String {
    let c = match Cidr4::parse(cidr_s) {
        Some(v) => v,
        None => return format!("Error: '{}' is not a valid CIDR.", cidr_s),
    };
    if n == 0 || n & (n - 1) != 0 {
        return "Error: n must be a power of 2 (1, 2, 4, 8, 16, ...).".to_string();
    }
    let bits = n.trailing_zeros() as u8;
    let new_prefix = c.prefix + bits;
    if new_prefix > 30 {
        return format!(
            "Error: splitting /{} into {} subnets requires /{} which has no usable hosts.",
            c.prefix, n, new_prefix
        );
    }
    if n > 65536 {
        return "Error: n too large (max 65536 subnets shown at once).".to_string();
    }

    let sub_mask = prefix_mask(new_prefix);
    let sub_size = 1u32 << (32 - new_prefix);
    let host_count = if new_prefix >= 31 {
        sub_size as u64
    } else {
        sub_size as u64 - 2
    };

    let mut out = format!(
        "Splitting {} into {} subnets (/{}):\n\n",
        cidr_s, n, new_prefix
    );
    out.push_str(&format!(
        "{:<5} {:<20} {:<20} {:<20} {:<12}\n",
        "#", "Network", "First Host", "Last Host / Broadcast", "Usable Hosts"
    ));
    out.push_str(&"-".repeat(85));
    out.push('\n');

    for i in 0..n as u32 {
        let net = c.network + i * sub_size;
        let broadcast = net | !sub_mask;
        let (first, last) = if new_prefix >= 31 {
            (fmt_ipv4(net), fmt_ipv4(broadcast))
        } else {
            (fmt_ipv4(net + 1), fmt_ipv4(broadcast - 1))
        };
        let _ = writeln!(
            out,
            "{:<5} {:<20} {:<20} {:<20} {}",
            i + 1,
            format!("{}/{}", fmt_ipv4(net), new_prefix),
            first,
            last,
            host_count,
        );
    }
    let _ = writeln!(
        out,
        "\nParent: {}  Mask: {}  Total usable: {}",
        c,
        fmt_ipv4(c.mask()),
        host_count * n
    );
    out
}

fn action_hosts(cidr_s: &str, offset: usize, limit: usize) -> String {
    let c = match Cidr4::parse(cidr_s) {
        Some(v) => v,
        None => return format!("Error: '{}' is not a valid CIDR.", cidr_s),
    };
    let limit = limit.min(256);
    let host_count = c.host_count();
    let (first_usable, last_usable) = if c.prefix >= 31 {
        (c.network, c.broadcast())
    } else {
        (c.network + 1, c.broadcast() - 1)
    };

    let total = if c.prefix >= 31 {
        c.broadcast() - c.network + 1
    } else {
        host_count as u32
    };
    let start_ip = first_usable + offset as u32;

    if start_ip > last_usable {
        return format!(
            "Error: offset {} exceeds the {} usable hosts in {}.",
            offset, total, cidr_s
        );
    }

    let end_ip = (start_ip + limit as u32 - 1).min(last_usable);
    let shown = (end_ip - start_ip + 1) as usize;

    let mut out = format!(
        "Hosts in {} — showing {} of {} (offset {})\n\n",
        cidr_s, shown, total, offset
    );
    out.push_str(&format!("{:<6} {:<18} {}\n", "#", "IP Address", "Type"));
    out.push_str(&"-".repeat(45));
    out.push('\n');

    for (i, ip) in (start_ip..=end_ip).enumerate() {
        let _ = writeln!(
            out,
            "{:<6} {:<18} {}",
            offset + i + 1,
            fmt_ipv4(ip),
            ip_type(ip)
        );
    }
    if end_ip < last_usable {
        let _ = writeln!(
            out,
            "\n  ... {} more hosts. Use offset={} to continue.",
            last_usable - end_ip,
            offset + shown
        );
    }
    let _ = writeln!(
        out,
        "\nNetwork: {}  Broadcast: {}  Mask: {}",
        fmt_ipv4(c.network),
        fmt_ipv4(c.broadcast()),
        fmt_ipv4(c.mask())
    );
    out
}

fn action_supernet(ips: &[String]) -> String {
    if ips.is_empty() {
        return "Error: provide at least one IP or CIDR in the 'ips' array.".to_string();
    }
    let cidrs: Vec<Cidr4> = match ips
        .iter()
        .map(|s| cidr_or_ip_to_cidr4(s))
        .collect::<Option<Vec<_>>>()
    {
        Some(v) => v,
        None => return "Error: one or more entries are not valid IPs or CIDRs.".to_string(),
    };

    let min_ip = cidrs.iter().map(|c| c.network).min().unwrap();
    let max_ip = cidrs.iter().map(|c| c.broadcast()).max().unwrap();

    // Find smallest prefix that covers min_ip..=max_ip
    let xor = min_ip ^ max_ip;
    let bits = if xor == 0 { 0 } else { 32 - xor.ilog2() - 1 };
    let prefix = bits as u8;
    let supernet = Cidr4 {
        network: min_ip & prefix_mask(prefix),
        prefix,
    };

    let mut out = format!(
        "Supernet for {} CIDRs: {}\n\n",
        cidrs.len(),
        supernet
    );
    out.push_str(&format!("  Prefix:    /{}\n", supernet.prefix));
    out.push_str(&format!("  Network:   {}\n", fmt_ipv4(supernet.network)));
    out.push_str(&format!(
        "  Broadcast: {}\n",
        fmt_ipv4(supernet.broadcast())
    ));
    out.push_str(&format!("  Mask:      {}\n", fmt_ipv4(supernet.mask())));
    out.push_str(&format!("  Usable:    {}\n\n", supernet.host_count()));
    out.push_str("Input CIDRs:\n");
    for (i, (s, c)) in ips.iter().zip(cidrs.iter()).enumerate() {
        let contained = if supernet.contains_cidr(c) {
            "✓"
        } else {
            "✗"
        };
        let _ = writeln!(
            out,
            "  {} {:<20} {}",
            contained,
            s,
            if !supernet.contains_cidr(c) {
                "WARNING: not fully contained"
            } else {
                ""
            }
        );
        let _ = i;
    }
    out
}

fn action_aggregate(ips: &[String]) -> String {
    if ips.is_empty() {
        return "Error: provide at least one IP or CIDR in the 'ips' array.".to_string();
    }
    let mut cidrs: Vec<Cidr4> = match ips
        .iter()
        .map(|s| cidr_or_ip_to_cidr4(s))
        .collect::<Option<Vec<_>>>()
    {
        Some(v) => v,
        None => return "Error: one or more entries are not valid IPs or CIDRs.".to_string(),
    };
    cidrs.sort_by_key(|c| (c.network, c.prefix));

    // Remove CIDRs fully contained within a previous one
    let mut merged: Vec<Cidr4> = Vec::new();
    for c in cidrs {
        if let Some(last) = merged.last() {
            if last.contains_cidr(&c) {
                continue; // absorbed
            }
        }
        merged.push(c);
    }

    // Try to merge adjacent same-prefix pairs
    loop {
        let mut changed = false;
        let mut next: Vec<Cidr4> = Vec::new();
        let mut skip = false;
        for i in 0..merged.len() {
            if skip {
                skip = false;
                continue;
            }
            if i + 1 < merged.len() {
                let a = &merged[i];
                let b = &merged[i + 1];
                if a.prefix == b.prefix && a.prefix > 0 {
                    let super_prefix = a.prefix - 1;
                    let super_mask = prefix_mask(super_prefix);
                    if a.network & super_mask == b.network & super_mask {
                        next.push(Cidr4 {
                            network: a.network & super_mask,
                            prefix: super_prefix,
                        });
                        skip = true;
                        changed = true;
                        continue;
                    }
                }
            }
            next.push(Cidr4 {
                network: merged[i].network,
                prefix: merged[i].prefix,
            });
        }
        merged = next;
        if !changed {
            break;
        }
    }

    let mut out = format!(
        "Aggregated {} input entries → {} CIDR(s):\n\n",
        ips.len(),
        merged.len()
    );
    out.push_str(&format!(
        "{:<5} {:<20} {:<20} {}\n",
        "#", "CIDR", "Network Range", "Hosts"
    ));
    out.push_str(&"-".repeat(70));
    out.push('\n');
    for (i, c) in merged.iter().enumerate() {
        let _ = writeln!(
            out,
            "{:<5} {:<20} {} – {}  {}",
            i + 1,
            c.to_string(),
            fmt_ipv4(if c.prefix >= 31 {
                c.network
            } else {
                c.network + 1
            }),
            fmt_ipv4(if c.prefix >= 31 {
                c.broadcast()
            } else {
                c.broadcast() - 1
            }),
            c.host_count()
        );
    }
    out
}

fn action_overlap(ips: &[String]) -> String {
    if ips.len() < 2 {
        return "Error: provide at least 2 CIDRs to check for overlaps.".to_string();
    }
    let cidrs: Vec<Cidr4> = match ips
        .iter()
        .map(|s| cidr_or_ip_to_cidr4(s))
        .collect::<Option<Vec<_>>>()
    {
        Some(v) => v,
        None => return "Error: one or more entries are not valid IPs or CIDRs.".to_string(),
    };

    let mut overlaps: Vec<(usize, usize, String)> = Vec::new();
    for i in 0..cidrs.len() {
        for j in (i + 1)..cidrs.len() {
            let a = &cidrs[i];
            let b = &cidrs[j];
            // Overlap if a.network <= b.broadcast and b.network <= a.broadcast
            if a.network <= b.broadcast() && b.network <= a.broadcast() {
                let overlap_start = a.network.max(b.network);
                let overlap_end = a.broadcast().min(b.broadcast());
                let desc = if overlap_start == overlap_end {
                    format!("single IP {}", fmt_ipv4(overlap_start))
                } else {
                    format!("{} – {}", fmt_ipv4(overlap_start), fmt_ipv4(overlap_end))
                };
                overlaps.push((i, j, desc));
            }
        }
    }

    if overlaps.is_empty() {
        let mut out = format!("No overlaps detected among {} CIDRs.\n\n", cidrs.len());
        for (i, s) in ips.iter().enumerate() {
            let _ = writeln!(out, "  {} {}", i + 1, s);
        }
        return out;
    }

    let mut out = format!(
        "Found {} overlap(s) among {} CIDRs:\n\n",
        overlaps.len(),
        cidrs.len()
    );
    out.push_str(&format!(
        "{:<5} {:<22} {:<22} {}\n",
        "#", "CIDR A", "CIDR B", "Overlap Range"
    ));
    out.push_str(&"-".repeat(80));
    out.push('\n');
    for (k, (i, j, desc)) in overlaps.iter().enumerate() {
        let _ = writeln!(out, "{:<5} {:<22} {:<22} {}", k + 1, ips[*i], ips[*j], desc);
    }
    out
}

fn action_contains(cidr_s: &str, ips: &[String]) -> String {
    let c = match Cidr4::parse(cidr_s) {
        Some(v) => v,
        None => return format!("Error: '{}' is not a valid CIDR.", cidr_s),
    };
    if ips.is_empty() {
        return "Error: provide IPs in the 'ips' array.".to_string();
    }

    let mut out = format!("Checking {} IPs against {}:\n\n", ips.len(), cidr_s);
    out.push_str(&format!("{:<5} {:<22} {}\n", "#", "IP / CIDR", "Status"));
    out.push_str(&"-".repeat(50));
    out.push('\n');

    let mut all_in = true;
    for (i, s) in ips.iter().enumerate() {
        let (inside, label) = if let Some(sub) = cidr_or_ip_to_cidr4(s) {
            let ok = c.contains_cidr(&sub);
            (ok, if ok { "✓ inside" } else { "✗ outside" })
        } else {
            (false, "✗ invalid")
        };
        if !inside {
            all_in = false;
        }
        let _ = writeln!(out, "{:<5} {:<22} {}", i + 1, s, label);
    }
    let verdict = if all_in {
        "ALL IPs are within the range."
    } else {
        "Some IPs are outside the range."
    };
    let _ = writeln!(out, "\n{}", verdict);
    out
}

fn action_range(start_s: &str, end_s: &str) -> String {
    let start = match parse_ipv4(start_s) {
        Some(v) => v,
        None => return format!("Error: '{}' is not a valid IPv4 address.", start_s),
    };
    let end = match parse_ipv4(end_s) {
        Some(v) => v,
        None => return format!("Error: '{}' is not a valid IPv4 address.", end_s),
    };
    if end < start {
        return "Error: end IP must be >= start IP.".to_string();
    }

    let count = end - start + 1;
    // Decompose the range into CIDRs using the standard algorithm
    let mut cidrs: Vec<Cidr4> = Vec::new();
    let mut current = start;
    while current <= end {
        // Largest block where current is the network address
        let mut max_prefix = 32u8;
        loop {
            if max_prefix == 0 {
                break;
            }
            let p = max_prefix - 1;
            let mask = prefix_mask(p);
            let net = current & mask;
            let bcast = net | !mask;
            if net == current && bcast <= end {
                max_prefix = p;
            } else {
                break;
            }
        }
        let c = Cidr4 {
            network: current,
            prefix: max_prefix,
        };
        let next = c.broadcast().saturating_add(1);
        cidrs.push(c);
        if next <= current {
            break;
        } // overflow guard
        current = next;
    }

    let mut out = format!(
        "IP Range {} – {} ({} addresses) → {} CIDR(s):\n\n",
        start_s,
        end_s,
        count,
        cidrs.len()
    );
    out.push_str(&format!("{:<5} {:<22} {}\n", "#", "CIDR", "Usable Hosts"));
    out.push_str(&"-".repeat(45));
    out.push('\n');
    for (i, c) in cidrs.iter().enumerate() {
        let _ = writeln!(out, "{:<5} {:<22} {}", i + 1, c.to_string(), c.host_count());
    }
    out
}

// ── Entry point ───────────────────────────────────────────────────────────────

pub async fn execute(args: &Value) -> Result<String, String> {
    let cidr = args
        .get("cidr")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim()
        .to_string();
    let ips: Vec<String> = args
        .get("ips")
        .and_then(|v| v.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();
    let n = args.get("n").and_then(|v| v.as_u64()).unwrap_or(0);
    let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(50) as usize;
    let offset = args.get("offset").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
    let start = args
        .get("start")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let end = args
        .get("end")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    // Infer action
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or(if !start.is_empty() && !end.is_empty() {
            "range"
        } else if ips.len() > 1 && cidr.is_empty() {
            "aggregate"
        } else if !cidr.is_empty() && n > 0 {
            "split"
        } else {
            "hosts"
        });

    let out = match action {
        "split" => {
            if cidr.is_empty() { return Ok("Error: 'cidr' is required for split action.".to_string()); }
            if n == 0 { return Ok("Error: 'n' (number of subnets) is required for split action.".to_string()); }
            action_split(&cidr, n)
        }
        "hosts" => {
            if cidr.is_empty() { return Ok("Error: 'cidr' is required for hosts action.".to_string()); }
            action_hosts(&cidr, offset, limit)
        }
        "supernet" => {
            if ips.is_empty() { return Ok("Error: 'ips' array is required for supernet action.".to_string()); }
            action_supernet(&ips)
        }
        "aggregate" => {
            if ips.is_empty() { return Ok("Error: 'ips' array is required for aggregate action.".to_string()); }
            action_aggregate(&ips)
        }
        "overlap" => {
            if ips.is_empty() { return Ok("Error: 'ips' array is required for overlap action.".to_string()); }
            action_overlap(&ips)
        }
        "contains" => {
            if cidr.is_empty() { return Ok("Error: 'cidr' is required for contains action.".to_string()); }
            if ips.is_empty() { return Ok("Error: 'ips' array is required for contains action.".to_string()); }
            action_contains(&cidr, &ips)
        }
        "range" => {
            if start.is_empty() || end.is_empty() {
                return Ok("Error: 'start' and 'end' IPs are required for range action.".to_string());
            }
            action_range(&start, &end)
        }
        other => format!("Error: unknown action '{}'. Use split, hosts, supernet, aggregate, overlap, contains, or range.", other),
    };
    Ok(out)
}
