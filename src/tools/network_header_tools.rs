use serde_json::{json, Value};

pub fn network_header_tools_schema() -> Value {
    json!({
        "name": "network_header_tools",
        "description": "Parse and analyze raw network protocol headers (IPv4, IPv6, TCP, UDP, ICMP, Ethernet) from hex bytes without external tools. Decode field names, values, flags, and verify checksums. Actions: parse (auto-detect protocol), ipv4, ipv6, tcp, udp, icmp, ethernet. Pass 'hex' with the raw header bytes.",
        "parameters": {
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["parse", "ipv4", "ipv6", "tcp", "udp", "icmp", "ethernet"],
                    "description": "parse (auto-detect from header bytes), ipv4 (decode IPv4 header), ipv6 (decode IPv6 header), tcp (decode TCP header), udp (decode UDP header), icmp (decode ICMP/ICMPv6 header), ethernet (decode Ethernet II frame header)"
                },
                "hex": {
                    "type": "string",
                    "description": "Hex-encoded raw header bytes (spaces and colons ignored). For 'parse', include the full packet from the outermost header."
                },
                "protocol": {
                    "type": "string",
                    "description": "Optional hint for 'parse': ethernet/ipv4/ipv6/tcp/udp/icmp"
                }
            },
            "required": ["hex"]
        }
    })
}

pub async fn execute(args: &Value) -> Result<String, String> {
    let hex_raw = args
        .get("hex")
        .and_then(|v| v.as_str())
        .ok_or("Pass 'hex' with raw header bytes.")?;

    let data = parse_hex(hex_raw)?;

    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("parse");

    match action {
        "ipv4" => decode_ipv4(&data),
        "ipv6" => decode_ipv6(&data),
        "tcp" => decode_tcp(&data),
        "udp" => decode_udp(&data),
        "icmp" => decode_icmp(&data),
        "ethernet" => decode_ethernet(&data),
        _ => {
            // auto-detect
            let hint = args.get("protocol").and_then(|v| v.as_str()).unwrap_or("");
            action_parse(&data, hint)
        }
    }
}

// ── Hex decode ────────────────────────────────────────────────────────────────

fn parse_hex(s: &str) -> Result<Vec<u8>, String> {
    let clean: String = s.chars().filter(|c| c.is_ascii_hexdigit()).collect();
    if clean.len() % 2 != 0 {
        return Err(format!(
            "Hex string has odd length ({}) — incomplete byte.",
            clean.len()
        ));
    }
    (0..clean.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&clean[i..i + 2], 16)
                .map_err(|e| format!("Invalid hex at offset {i}: {e}"))
        })
        .collect()
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn get_u8(data: &[u8], offset: usize) -> Result<u8, String> {
    data.get(offset)
        .copied()
        .ok_or_else(|| format!("Need byte at offset {offset}, only {} bytes.", data.len()))
}

fn get_u16_be(data: &[u8], offset: usize) -> Result<u16, String> {
    if offset + 1 >= data.len() {
        return Err(format!(
            "Need 2 bytes at offset {offset}, only {} bytes.",
            data.len()
        ));
    }
    Ok(u16::from_be_bytes([data[offset], data[offset + 1]]))
}

fn get_u32_be(data: &[u8], offset: usize) -> Result<u32, String> {
    if offset + 3 >= data.len() {
        return Err(format!(
            "Need 4 bytes at offset {offset}, only {} bytes.",
            data.len()
        ));
    }
    Ok(u32::from_be_bytes([
        data[offset],
        data[offset + 1],
        data[offset + 2],
        data[offset + 3],
    ]))
}

fn ipv4_str(data: &[u8], offset: usize) -> String {
    if offset + 3 < data.len() {
        format!(
            "{}.{}.{}.{}",
            data[offset],
            data[offset + 1],
            data[offset + 2],
            data[offset + 3]
        )
    } else {
        "(truncated)".into()
    }
}

fn ip_proto_name(proto: u8) -> &'static str {
    match proto {
        1 => "ICMP",
        2 => "IGMP",
        4 => "IP-in-IP",
        6 => "TCP",
        17 => "UDP",
        41 => "IPv6",
        47 => "GRE",
        50 => "ESP",
        51 => "AH",
        58 => "ICMPv6",
        89 => "OSPF",
        132 => "SCTP",
        _ => "unknown",
    }
}

fn ethertype_name(et: u16) -> &'static str {
    match et {
        0x0800 => "IPv4",
        0x0806 => "ARP",
        0x0842 => "WakeOnLAN",
        0x86DD => "IPv6",
        0x8100 => "802.1Q VLAN",
        0x88CC => "LLDP",
        0x8847 => "MPLS unicast",
        0x8848 => "MPLS multicast",
        0x8863 => "PPPoE Discovery",
        0x8864 => "PPPoE Session",
        _ => "unknown",
    }
}

fn ipv4_checksum(header: &[u8]) -> u16 {
    let mut sum: u32 = 0;
    for i in (0..header.len()).step_by(2) {
        if i + 1 < header.len() {
            let word = u16::from_be_bytes([header[i], header[i + 1]]) as u32;
            sum += word;
        }
    }
    while sum >> 16 != 0 {
        sum = (sum & 0xFFFF) + (sum >> 16);
    }
    !(sum as u16)
}

fn tcp_flag_str(flags: u8) -> String {
    let mut parts = Vec::new();
    if flags & 0x01 != 0 {
        parts.push("FIN");
    }
    if flags & 0x02 != 0 {
        parts.push("SYN");
    }
    if flags & 0x04 != 0 {
        parts.push("RST");
    }
    if flags & 0x08 != 0 {
        parts.push("PSH");
    }
    if flags & 0x10 != 0 {
        parts.push("ACK");
    }
    if flags & 0x20 != 0 {
        parts.push("URG");
    }
    if flags & 0x40 != 0 {
        parts.push("ECE");
    }
    if flags & 0x80 != 0 {
        parts.push("CWR");
    }
    if parts.is_empty() {
        "none".to_string()
    } else {
        parts.join("|")
    }
}

fn icmp_type_name(typ: u8, code: u8) -> String {
    let type_name = match typ {
        0 => "Echo Reply",
        3 => match code {
            0 => "Destination Unreachable: Net",
            1 => "Destination Unreachable: Host",
            2 => "Destination Unreachable: Protocol",
            3 => "Destination Unreachable: Port",
            4 => "Destination Unreachable: Fragmentation Needed",
            _ => "Destination Unreachable",
        },
        4 => "Source Quench",
        5 => "Redirect",
        8 => "Echo Request",
        9 => "Router Advertisement",
        10 => "Router Solicitation",
        11 => match code {
            0 => "Time Exceeded: TTL",
            1 => "Time Exceeded: Fragment Reassembly",
            _ => "Time Exceeded",
        },
        12 => "Parameter Problem",
        13 => "Timestamp",
        14 => "Timestamp Reply",
        _ => "unknown",
    };
    type_name.to_string()
}

fn icmpv6_type_name(typ: u8) -> &'static str {
    match typ {
        1 => "Destination Unreachable",
        2 => "Packet Too Big",
        3 => "Time Exceeded",
        4 => "Parameter Problem",
        128 => "Echo Request",
        129 => "Echo Reply",
        133 => "Router Solicitation",
        134 => "Router Advertisement",
        135 => "Neighbor Solicitation",
        136 => "Neighbor Advertisement",
        137 => "Redirect",
        _ => "unknown",
    }
}

// ── Auto detect ───────────────────────────────────────────────────────────────

fn action_parse(data: &[u8], hint: &str) -> Result<String, String> {
    let hint_l = hint.to_lowercase();
    if (hint_l == "ethernet" || (data.len() >= 14 && hint_l.is_empty()))
        && data.len() >= 14
    {
        let ethertype = u16::from_be_bytes([data[12], data[13]]);
        if ethertype == 0x0800
            || ethertype == 0x86DD
            || ethertype == 0x0806
            || ethertype >= 0x0600
        {
            let mut out = decode_ethernet(data)?;
            out.push('\n');
            let inner = &data[14..];
            if ethertype == 0x0800 && inner.len() >= 20 {
                out.push_str(&decode_ipv4(inner)?);
            } else if ethertype == 0x86DD && inner.len() >= 40 {
                out.push_str(&decode_ipv6(inner)?);
            }
            return Ok(out);
        }
    }

    if (hint_l == "ipv4" || (!data.is_empty() && (data[0] >> 4) == 4 && hint_l != "ipv6"))
        && data.len() >= 20
    {
        let mut out = decode_ipv4(data)?;
        let ihl = (data[0] & 0x0F) as usize * 4;
        let proto = data[9];
        let inner = &data[ihl..];
        if proto == 6 && inner.len() >= 20 {
            out.push('\n');
            out.push_str(&decode_tcp(inner)?);
        } else if proto == 17 && inner.len() >= 8 {
            out.push('\n');
            out.push_str(&decode_udp(inner)?);
        } else if proto == 1 && inner.len() >= 4 {
            out.push('\n');
            out.push_str(&decode_icmp(inner)?);
        }
        return Ok(out);
    }

    if (hint_l == "ipv6" || (!data.is_empty() && (data[0] >> 4) == 6)) && data.len() >= 40 {
        return decode_ipv6(data);
    }

    if hint_l == "tcp" || data.len() >= 20 {
        // try TCP if we have enough bytes and flags field looks reasonable
        if data.len() >= 20 {
            let data_offset = (data[12] >> 4) as usize * 4;
            if data_offset >= 20 && data_offset <= data.len() + 20 {
                return decode_tcp(data);
            }
        }
    }

    if hint_l == "udp" || data.len() == 8 {
        return decode_udp(data);
    }

    if hint_l == "icmp" {
        return decode_icmp(data);
    }

    Err(format!(
        "Cannot auto-detect protocol from {} bytes. Specify action: ipv4/ipv6/tcp/udp/icmp/ethernet.",
        data.len()
    ))
}

// ── Ethernet II ───────────────────────────────────────────────────────────────

fn decode_ethernet(data: &[u8]) -> Result<String, String> {
    if data.len() < 14 {
        return Err(format!(
            "Ethernet header needs 14 bytes, got {}.",
            data.len()
        ));
    }
    let dst = format!(
        "{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
        data[0], data[1], data[2], data[3], data[4], data[5]
    );
    let src = format!(
        "{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
        data[6], data[7], data[8], data[9], data[10], data[11]
    );
    let ethertype = u16::from_be_bytes([data[12], data[13]]);
    let et_name = ethertype_name(ethertype);

    let multicast = data[0] & 0x01 != 0;
    let broadcast = data[..6] == [0xFF; 6];
    let dst_label = if broadcast {
        " (broadcast)"
    } else if multicast {
        " (multicast)"
    } else {
        ""
    };

    let mut out = String::from("ETHERNET II FRAME\n");
    out.push_str(&"─".repeat(50));
    out.push('\n');
    out.push_str(&format!("Destination  {dst}{dst_label}\n"));
    out.push_str(&format!("Source       {src}\n"));
    out.push_str(&format!("EtherType    0x{ethertype:04X}  ({et_name})\n"));
    out.push_str(&format!(
        "Payload      {} bytes\n",
        data.len().saturating_sub(14)
    ));
    Ok(out)
}

// ── IPv4 ──────────────────────────────────────────────────────────────────────

fn decode_ipv4(data: &[u8]) -> Result<String, String> {
    if data.len() < 20 {
        return Err(format!("IPv4 header needs ≥20 bytes, got {}.", data.len()));
    }
    let version = data[0] >> 4;
    if version != 4 {
        return Err(format!("Version field is {version}, expected 4 for IPv4."));
    }
    let ihl = (data[0] & 0x0F) as usize * 4;
    let dscp = data[1] >> 2;
    let ecn = data[1] & 0x03;
    let total_len = get_u16_be(data, 2)?;
    let ident = get_u16_be(data, 4)?;
    let flags_frag = get_u16_be(data, 6)?;
    let flags = (flags_frag >> 13) as u8;
    let frag_offset = (flags_frag & 0x1FFF) * 8;
    let ttl = get_u8(data, 8)?;
    let proto = get_u8(data, 9)?;
    let checksum = get_u16_be(data, 10)?;
    let src = ipv4_str(data, 12);
    let dst = ipv4_str(data, 16);
    let proto_name = ip_proto_name(proto);

    // verify checksum
    let hdr_end = ihl.min(data.len());
    let mut hdr_for_cksum = data[..hdr_end].to_vec();
    hdr_for_cksum[10] = 0;
    hdr_for_cksum[11] = 0;
    let computed = ipv4_checksum(&hdr_for_cksum);
    let cksum_ok = computed == checksum;

    let df = flags & 0x02 != 0;
    let mf = flags & 0x01 != 0;

    let mut out = String::from("IPv4 HEADER\n");
    out.push_str(&"─".repeat(50));
    out.push('\n');
    out.push_str(&format!("Version      {version}\n"));
    out.push_str(&format!(
        "IHL          {ihl} bytes ({} 32-bit words)\n",
        ihl / 4
    ));
    out.push_str(&format!("DSCP/ECN     DSCP={dscp} ECN={ecn}\n"));
    out.push_str(&format!("Total Length {total_len} bytes\n"));
    out.push_str(&format!("Identifier   0x{ident:04X} ({ident})\n"));
    out.push_str(&format!(
        "Flags        0x{:X}  DF={} MF={}\n",
        flags,
        if df { 1 } else { 0 },
        if mf { 1 } else { 0 }
    ));
    out.push_str(&format!("Frag Offset  {frag_offset}\n"));
    out.push_str(&format!("TTL          {ttl}\n"));
    out.push_str(&format!("Protocol     {proto} ({proto_name})\n"));
    out.push_str(&format!(
        "Checksum     0x{checksum:04X}  {}\n",
        if cksum_ok { "✓ valid" } else { "✗ INVALID" }
    ));
    out.push_str(&format!("Source       {src}\n"));
    out.push_str(&format!("Destination  {dst}\n"));
    if ihl > 20 {
        out.push_str(&format!("Options      {} bytes\n", ihl.saturating_sub(20)));
    }
    let payload_len = (total_len as usize).saturating_sub(ihl);
    out.push_str(&format!(
        "Payload      {payload_len} bytes  ({proto_name})\n"
    ));
    Ok(out)
}

// ── IPv6 ──────────────────────────────────────────────────────────────────────

fn decode_ipv6(data: &[u8]) -> Result<String, String> {
    if data.len() < 40 {
        return Err(format!("IPv6 header needs 40 bytes, got {}.", data.len()));
    }
    let version = data[0] >> 4;
    if version != 6 {
        return Err(format!("Version field is {version}, expected 6 for IPv6."));
    }
    let traffic_class = ((data[0] & 0x0F) << 4) | (data[1] >> 4);
    let dscp = traffic_class >> 2;
    let ecn = traffic_class & 0x03;
    let flow_label = ((data[1] as u32 & 0x0F) << 16) | (data[2] as u32) << 8 | data[3] as u32;
    let payload_len = get_u16_be(data, 4)?;
    let next_header = data[6];
    let hop_limit = data[7];
    let src = format_ipv6(&data[8..24]);
    let dst = format_ipv6(&data[24..40]);
    let nh_name = ip_proto_name(next_header);

    let mut out = String::from("IPv6 HEADER\n");
    out.push_str(&"─".repeat(50));
    out.push('\n');
    out.push_str("Version        6\n");
    out.push_str(&format!(
        "Traffic Class  0x{traffic_class:02X}  DSCP={dscp} ECN={ecn}\n"
    ));
    out.push_str(&format!("Flow Label     0x{flow_label:05X}\n"));
    out.push_str(&format!("Payload Length {payload_len} bytes\n"));
    out.push_str(&format!("Next Header    {next_header} ({nh_name})\n"));
    out.push_str(&format!("Hop Limit      {hop_limit}\n"));
    out.push_str(&format!("Source         {src}\n"));
    out.push_str(&format!("Destination    {dst}\n"));
    Ok(out)
}

fn format_ipv6(b: &[u8]) -> String {
    if b.len() < 16 {
        return "(truncated)".into();
    }
    let groups: Vec<String> = (0..8)
        .map(|i| format!("{:04x}", u16::from_be_bytes([b[i * 2], b[i * 2 + 1]])))
        .collect();
    // compress longest run of zeroes
    let joined = groups.join(":");
    // simple compression
    let compressed = compress_ipv6(&groups);
    if compressed.len() < joined.len() {
        compressed
    } else {
        joined
    }
}

fn compress_ipv6(groups: &[String]) -> String {
    let zeros: Vec<bool> = groups.iter().map(|g| g == "0000").collect();
    // find longest run of zeros
    let (mut best_start, mut best_len) = (0, 0);
    let (mut cur_start, mut cur_len) = (0, 0);
    for (i, &z) in zeros.iter().enumerate() {
        if z {
            if cur_len == 0 {
                cur_start = i;
            }
            cur_len += 1;
            if cur_len > best_len {
                best_start = cur_start;
                best_len = cur_len;
            }
        } else {
            cur_len = 0;
        }
    }
    if best_len < 2 {
        return groups
            .iter()
            .map(|g| {
                g.trim_start_matches('0')
                    .to_string()
                    .replace("", "0")
                    .to_string()
            })
            .collect::<Vec<_>>()
            .join(":");
    }
    let before: Vec<String> = groups[..best_start]
        .iter()
        .map(|g| {
            let t = g.trim_start_matches('0');
            if t.is_empty() {
                "0".into()
            } else {
                t.into()
            }
        })
        .collect();
    let after: Vec<String> = groups[best_start + best_len..]
        .iter()
        .map(|g| {
            let t = g.trim_start_matches('0');
            if t.is_empty() {
                "0".into()
            } else {
                t.into()
            }
        })
        .collect();
    format!("{}::{}", before.join(":"), after.join(":"))
}

// ── TCP ───────────────────────────────────────────────────────────────────────

fn decode_tcp(data: &[u8]) -> Result<String, String> {
    if data.len() < 20 {
        return Err(format!("TCP header needs ≥20 bytes, got {}.", data.len()));
    }
    let src_port = get_u16_be(data, 0)?;
    let dst_port = get_u16_be(data, 2)?;
    let seq = get_u32_be(data, 4)?;
    let ack = get_u32_be(data, 8)?;
    let data_offset = (data[12] >> 4) as usize * 4;
    let flags = data[13];
    let window = get_u16_be(data, 14)?;
    let checksum = get_u16_be(data, 16)?;
    let urgent = get_u16_be(data, 18)?;

    let flag_str = tcp_flag_str(flags);
    let well_known = tcp_port_name(src_port)
        .map(|n| format!(" ({n})"))
        .unwrap_or_default();
    let well_known_dst = tcp_port_name(dst_port)
        .map(|n| format!(" ({n})"))
        .unwrap_or_default();

    let mut out = String::from("TCP HEADER\n");
    out.push_str(&"─".repeat(50));
    out.push('\n');
    out.push_str(&format!("Source Port  {src_port}{well_known}\n"));
    out.push_str(&format!("Dest Port    {dst_port}{well_known_dst}\n"));
    out.push_str(&format!("Seq Number   {seq}  (0x{seq:08X})\n"));
    out.push_str(&format!("Ack Number   {ack}  (0x{ack:08X})\n"));
    out.push_str(&format!(
        "Data Offset  {data_offset} bytes ({} 32-bit words)\n",
        data_offset / 4
    ));
    out.push_str(&format!("Flags        0x{flags:02X}  [{flag_str}]\n"));
    out.push_str(&format!("Window       {window}\n"));
    out.push_str(&format!("Checksum     0x{checksum:04X}\n"));
    if urgent > 0 {
        out.push_str(&format!("Urgent Ptr   {urgent}\n"));
    }
    if data_offset > 20 {
        out.push_str(&format!(
            "Options      {} bytes\n",
            data_offset.saturating_sub(20)
        ));
    }
    let payload = data.len().saturating_sub(data_offset);
    out.push_str(&format!("Payload      {payload} bytes\n"));
    Ok(out)
}

fn tcp_port_name(port: u16) -> Option<&'static str> {
    Some(match port {
        20 | 21 => "FTP",
        22 => "SSH",
        23 => "Telnet",
        25 => "SMTP",
        53 => "DNS",
        80 => "HTTP",
        110 => "POP3",
        143 => "IMAP",
        443 => "HTTPS",
        465 => "SMTPS",
        587 => "Submission",
        993 => "IMAPS",
        995 => "POP3S",
        1234 => "LM Studio",
        1433 => "MSSQL",
        3306 => "MySQL",
        3389 => "RDP",
        5432 => "PostgreSQL",
        5672 => "AMQP",
        6379 => "Redis",
        8080 => "HTTP-alt",
        8443 => "HTTPS-alt",
        8888 => "Jupyter",
        9200 => "Elasticsearch",
        11434 => "Ollama",
        27017 => "MongoDB",
        _ => return None,
    })
}

// ── UDP ───────────────────────────────────────────────────────────────────────

fn decode_udp(data: &[u8]) -> Result<String, String> {
    if data.len() < 8 {
        return Err(format!("UDP header needs 8 bytes, got {}.", data.len()));
    }
    let src_port = get_u16_be(data, 0)?;
    let dst_port = get_u16_be(data, 2)?;
    let length = get_u16_be(data, 4)?;
    let checksum = get_u16_be(data, 6)?;

    let udp_port_name = |p: u16| -> Option<&'static str> {
        Some(match p {
            53 => "DNS",
            67 => "DHCP Server",
            68 => "DHCP Client",
            69 => "TFTP",
            123 => "NTP",
            161 => "SNMP",
            162 => "SNMP Trap",
            500 => "IKE",
            514 => "Syslog",
            1900 => "SSDP/UPnP",
            4500 => "NAT-T IKE",
            5353 => "mDNS",
            _ => return None,
        })
    };

    let src_label = udp_port_name(src_port)
        .map(|n| format!(" ({n})"))
        .unwrap_or_default();
    let dst_label = udp_port_name(dst_port)
        .map(|n| format!(" ({n})"))
        .unwrap_or_default();

    let mut out = String::from("UDP HEADER\n");
    out.push_str(&"─".repeat(50));
    out.push('\n');
    out.push_str(&format!("Source Port  {src_port}{src_label}\n"));
    out.push_str(&format!("Dest Port    {dst_port}{dst_label}\n"));
    out.push_str(&format!("Length       {length} bytes (header + payload)\n"));
    out.push_str(&format!("Checksum     0x{checksum:04X}\n"));
    out.push_str(&format!(
        "Payload      {} bytes\n",
        (length as usize).saturating_sub(8)
    ));
    Ok(out)
}

// ── ICMP ──────────────────────────────────────────────────────────────────────

fn decode_icmp(data: &[u8]) -> Result<String, String> {
    if data.len() < 4 {
        return Err(format!("ICMP header needs ≥4 bytes, got {}.", data.len()));
    }
    let typ = data[0];
    let code = data[1];
    let checksum = get_u16_be(data, 2)?;

    // determine if ICMPv6 by type range
    let is_v6 = typ >= 128;
    let type_desc = if is_v6 {
        icmpv6_type_name(typ).to_string()
    } else {
        icmp_type_name(typ, code)
    };

    let header_type = if is_v6 {
        "ICMPv6 HEADER"
    } else {
        "ICMP HEADER"
    };
    let mut out = format!("{header_type}\n");
    out.push_str(&"─".repeat(50));
    out.push('\n');
    out.push_str(&format!("Type         {typ}  ({type_desc})\n"));
    out.push_str(&format!("Code         {code}\n"));
    out.push_str(&format!("Checksum     0x{checksum:04X}\n"));

    // Echo request/reply: show identifier and sequence
    if (typ == 8 || typ == 0 || typ == 128 || typ == 129) && data.len() >= 8 {
        let id = get_u16_be(data, 4)?;
        let seq = get_u16_be(data, 6)?;
        out.push_str(&format!("Identifier   {id}\n"));
        out.push_str(&format!("Sequence     {seq}\n"));
        out.push_str(&format!(
            "Data         {} bytes\n",
            data.len().saturating_sub(8)
        ));
    }
    Ok(out)
}
