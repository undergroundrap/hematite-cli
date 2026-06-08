use serde_json::{json, Value};
use std::collections::HashMap;
use std::fs;

pub fn schema() -> Value {
    json!({
        "name": "pcap_tools",
        "description": "Parse and analyze PCAP/PCAPNG packet capture files without external tools — no Wireshark or tcpdump required. 6 actions: info (default — file format, byte order, link type, packet count, capture duration, byte stats), packets (tabular listing of packets with number/timestamp/length/protocol/source/dest; 'limit' to cap, default 20), protocols (protocol distribution table — counts and percentages for Ethernet/IP/TCP/UDP/ICMP/DNS/HTTP/ARP/TLS etc.), conversations (top host pairs by packet count and byte volume), dns (all DNS queries and responses with name/type/answer), http (HTTP request/response pairs with method/status/host/path/user-agent). Pass 'file' with a path to a .pcap or .pcapng file.",
        "input_schema": {
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["info", "packets", "protocols", "conversations", "dns", "http"],
                    "description": "Operation to perform (default: info)"
                },
                "file": { "type": "string", "description": "Path to a .pcap or .pcapng file" },
                "limit": { "type": "integer", "description": "Maximum number of rows to return (default 20)" }
            },
            "required": ["file"]
        }
    })
}

// ── PCAP global header ────────────────────────────────────────────────────────

const PCAP_MAGIC_LE: u32 = 0xA1B2_C3D4;
const PCAP_MAGIC_BE: u32 = 0xD4C3_B2A1;
const PCAP_MAGIC_NS_LE: u32 = 0xA1B2_3C4D; // nanosecond variant
const PCAP_MAGIC_NS_BE: u32 = 0x4D3C_B2A1;
const PCAPNG_MAGIC: u32 = 0x0A0D_0D0A; // Section Header Block type

fn r16le(d: &[u8], o: usize) -> Option<u16> {
    d.get(o..o + 2).map(|b| u16::from_le_bytes([b[0], b[1]]))
}
fn r16be(d: &[u8], o: usize) -> Option<u16> {
    d.get(o..o + 2).map(|b| u16::from_be_bytes([b[0], b[1]]))
}
fn r32le(d: &[u8], o: usize) -> Option<u32> {
    d.get(o..o + 4)
        .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}
fn r32be(d: &[u8], o: usize) -> Option<u32> {
    d.get(o..o + 4)
        .map(|b| u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
}
fn r64le(d: &[u8], o: usize) -> Option<u64> {
    d.get(o..o + 8)
        .map(|b| u64::from_le_bytes(b.try_into().unwrap()))
}

#[allow(dead_code)]
fn read_str(d: &[u8], off: usize, max: usize) -> String {
    d.get(off..off + max.min(d.len().saturating_sub(off)))
        .map(|s| {
            s.iter()
                .take_while(|&&c| c != 0)
                .map(|&c| c as char)
                .collect()
        })
        .unwrap_or_default()
}

// ── link type name ─────────────────────────────────────────────────────────────

fn link_type_name(t: u32) -> &'static str {
    match t {
        0 => "NULL/Loopback",
        1 => "Ethernet",
        6 => "IEEE 802.5 Token Ring",
        9 => "PPP",
        10 => "FDDI",
        12 => "Raw IP",
        14 => "Raw IPv4",
        101 => "Raw IPv4",
        113 => "Linux Cooked",
        127 => "IEEE 802.11 (Wi-Fi)",
        129 => "PPP over Ethernet",
        141 => "IEEE 802.11 with RadioTap",
        143 => "IEEE 802.15.4",
        163 => "IEEE 802.11 with AVS",
        _ => "Unknown",
    }
}

// ── Packet representation ─────────────────────────────────────────────────────

#[derive(Debug, Clone)]
struct Packet {
    num: usize,
    ts_sec: u32,
    ts_frac: u32, // microseconds or nanoseconds
    orig_len: u32,
    data: Vec<u8>,
    link_type: u32,
}

impl Packet {
    fn timestamp_f(&self) -> f64 {
        self.ts_sec as f64 + (self.ts_frac as f64 / 1_000_000.0)
    }

    // Parse Ethernet frame → return (proto_str, src_str, dst_str, ip_data, ip_proto)
    fn parse_eth(&self) -> (String, String, String, Option<&[u8]>, u8) {
        let d = &self.data;
        if d.len() < 14 {
            return ("?".into(), "?".into(), "?".into(), None, 0);
        }
        let dst_mac = format!(
            "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
            d[0], d[1], d[2], d[3], d[4], d[5]
        );
        let src_mac = format!(
            "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
            d[6], d[7], d[8], d[9], d[10], d[11]
        );
        let etype = u16::from_be_bytes([d[12], d[13]]);
        // Handle 802.1Q VLAN tag
        let (etype, payload_off) = if etype == 0x8100 && d.len() >= 16 {
            (u16::from_be_bytes([d[14], d[15]]), 16usize)
        } else {
            (etype, 14usize)
        };
        match etype {
            0x0800 => {
                // IPv4
                if d.len() < payload_off + 20 {
                    return ("IPv4".into(), src_mac, dst_mac, None, 0);
                }
                let ip = &d[payload_off..];
                let ihl = ((ip[0] & 0x0F) as usize) * 4;
                let ip_proto = ip[9];
                let src_ip = format!("{}.{}.{}.{}", ip[12], ip[13], ip[14], ip[15]);
                let dst_ip = format!("{}.{}.{}.{}", ip[16], ip[17], ip[18], ip[19]);
                let payload = d.get(payload_off + ihl..);
                (
                    ip_proto_name(ip_proto).into(),
                    src_ip,
                    dst_ip,
                    payload,
                    ip_proto,
                )
            }
            0x86DD => {
                // IPv6
                if d.len() < payload_off + 40 {
                    return ("IPv6".into(), src_mac, dst_mac, None, 0);
                }
                let ip = &d[payload_off..];
                let next_hdr = ip[6];
                let src = fmt_ipv6(&ip[8..24]);
                let dst = fmt_ipv6(&ip[24..40]);
                let payload = d.get(payload_off + 40..);
                (ip_proto_name(next_hdr).into(), src, dst, payload, next_hdr)
            }
            0x0806 => ("ARP".into(), src_mac, dst_mac, None, 0),
            _ => ("Ethernet".into(), src_mac, dst_mac, None, 0),
        }
    }

    // For loopback / raw IP link types
    fn parse_raw_ip(&self) -> (String, String, String, Option<&[u8]>, u8) {
        let d = &self.data;
        // loopback: 4-byte family prefix
        let off = if self.link_type == 0 { 4 } else { 0 };
        if d.len() < off + 20 {
            return ("?".into(), "?".into(), "?".into(), None, 0);
        }
        let ip = &d[off..];
        let ver = ip[0] >> 4;
        if ver == 4 {
            let ihl = ((ip[0] & 0x0F) as usize) * 4;
            let ip_proto = ip[9];
            let src = format!("{}.{}.{}.{}", ip[12], ip[13], ip[14], ip[15]);
            let dst = format!("{}.{}.{}.{}", ip[16], ip[17], ip[18], ip[19]);
            let payload = d.get(off + ihl..);
            (ip_proto_name(ip_proto).into(), src, dst, payload, ip_proto)
        } else if ver == 6 && d.len() >= off + 40 {
            let next_hdr = ip[6];
            let src = fmt_ipv6(&ip[8..24]);
            let dst = fmt_ipv6(&ip[24..40]);
            let payload = d.get(off + 40..);
            (ip_proto_name(next_hdr).into(), src, dst, payload, next_hdr)
        } else {
            ("?".into(), "?".into(), "?".into(), None, 0)
        }
    }

    fn dissect(&self) -> (String, String, String, Option<Vec<u8>>, u8) {
        let (proto, src, dst, ip_payload, ip_proto) = match self.link_type {
            0 | 12 | 14 | 101 => self.parse_raw_ip(),
            _ => self.parse_eth(),
        };
        (proto, src, dst, ip_payload.map(|s| s.to_vec()), ip_proto)
    }
}

fn fmt_ipv6(b: &[u8]) -> String {
    if b.len() < 16 {
        return "::".into();
    }
    let groups: Vec<String> = b
        .chunks(2)
        .map(|g| format!("{:x}", u16::from_be_bytes([g[0], g[1]])))
        .collect();
    groups.join(":")
}

fn ip_proto_name(p: u8) -> &'static str {
    match p {
        1 => "ICMP",
        2 => "IGMP",
        6 => "TCP",
        17 => "UDP",
        41 => "IPv6",
        47 => "GRE",
        50 => "ESP",
        51 => "AH",
        58 => "ICMPv6",
        89 => "OSPF",
        132 => "SCTP",
        _ => "IP",
    }
}

// ── PCAP parser ───────────────────────────────────────────────────────────────

struct PcapFile {
    format: String,
    link_type: u32,
    packets: Vec<Packet>,
    #[allow(dead_code)]
    nano_ts: bool,
    is_be: bool,
}

fn parse_pcap(data: &[u8]) -> Result<PcapFile, String> {
    if data.len() < 4 {
        return Err("File too small".into());
    }
    let magic = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
    match magic {
        PCAP_MAGIC_LE | PCAP_MAGIC_NS_LE => parse_pcap_classic(data, false, magic == PCAP_MAGIC_NS_LE),
        PCAP_MAGIC_BE | PCAP_MAGIC_NS_BE => parse_pcap_classic(data, true, magic == PCAP_MAGIC_NS_BE),
        PCAPNG_MAGIC => parse_pcapng(data),
        _ => Err(format!(
            "Not a PCAP file (magic = 0x{:08X}). Expected 0xA1B2C3D4 (pcap) or 0x0A0D0D0A (pcapng).",
            magic
        )),
    }
}

fn parse_pcap_classic(data: &[u8], be: bool, nano: bool) -> Result<PcapFile, String> {
    if data.len() < 24 {
        return Err("PCAP header truncated".into());
    }
    let r32 = if be { r32be } else { r32le };

    let link_type = r32(data, 20).unwrap_or(1);
    let mut packets = Vec::new();
    let mut off = 24usize;
    let mut num = 1usize;

    while off + 16 <= data.len() {
        let ts_sec = r32(data, off).unwrap_or(0);
        let ts_frac = r32(data, off + 4).unwrap_or(0);
        let incl_len = r32(data, off + 8).unwrap_or(0) as usize;
        let orig_len = r32(data, off + 12).unwrap_or(0);
        off += 16;
        if incl_len > 65536 || off + incl_len > data.len() {
            break;
        }
        let pkt_data = data[off..off + incl_len].to_vec();
        packets.push(Packet {
            num,
            ts_sec,
            ts_frac,
            orig_len,
            data: pkt_data,
            link_type,
        });
        off += incl_len;
        num += 1;
    }

    Ok(PcapFile {
        format: if nano {
            "PCAP (nanosecond)".into()
        } else {
            "PCAP".into()
        },
        link_type,
        packets,
        nano_ts: nano,
        is_be: be,
    })
}

fn parse_pcapng(data: &[u8]) -> Result<PcapFile, String> {
    // Walk blocks. First must be SHB (0x0A0D0D0A).
    // IDB (0x00000001) defines link type.
    // EPB (0x00000006) and OPB (0x00000002) carry packets.
    if data.len() < 28 {
        return Err("PCAPNG too small".into());
    }

    // Determine byte order from SHB byte-order magic at offset 8
    let bom_le = u32::from_le_bytes([data[8], data[9], data[10], data[11]]);
    let be = bom_le != 0x1A2B_3C4D;

    let r32 = if be { r32be } else { r32le };
    let _r64 = if be {
        |d: &[u8], o: usize| {
            d.get(o..o + 8)
                .map(|b| u64::from_be_bytes(b.try_into().unwrap()))
        }
    } else {
        r64le
    };

    let mut packets = Vec::new();
    let mut link_type = 1u32;
    let mut off = 0usize;
    let mut num = 1usize;

    while off + 8 <= data.len() {
        let block_type = r32(data, off).unwrap_or(0);
        let block_len = r32(data, off + 4).unwrap_or(0) as usize;
        if block_len < 12 || off + block_len > data.len() {
            break;
        }

        match block_type {
            0x0000_0001 => {
                // IDB — Interface Description Block
                link_type = r16(data, off + 8, be).unwrap_or(1) as u32;
            }
            0x0000_0006 => {
                // EPB — Enhanced Packet Block
                if block_len >= 28 {
                    let ts_hi = r32(data, off + 12).unwrap_or(0) as u64;
                    let ts_lo = r32(data, off + 16).unwrap_or(0) as u64;
                    let ts_us = (ts_hi << 32) | ts_lo;
                    let ts_sec = (ts_us / 1_000_000) as u32;
                    let ts_frac = (ts_us % 1_000_000) as u32;
                    let cap_len = r32(data, off + 20).unwrap_or(0) as usize;
                    let orig_len = r32(data, off + 24).unwrap_or(0);
                    let pkt_off = off + 28;
                    if pkt_off + cap_len <= data.len() {
                        let pkt_data = data[pkt_off..pkt_off + cap_len].to_vec();
                        packets.push(Packet {
                            num,
                            ts_sec,
                            ts_frac,
                            orig_len,
                            data: pkt_data,
                            link_type,
                        });
                        num += 1;
                    }
                }
            }
            0x0000_0002 => {
                // OPB — Obsolete Packet Block
                if block_len >= 28 {
                    let cap_len = r32(data, off + 12).unwrap_or(0) as usize;
                    let orig_len = r32(data, off + 8).unwrap_or(0);
                    let ts_hi = r32(data, off + 16).unwrap_or(0) as u64;
                    let ts_lo = r32(data, off + 20).unwrap_or(0) as u64;
                    let ts_us = (ts_hi << 32) | ts_lo;
                    let ts_sec = (ts_us / 1_000_000) as u32;
                    let ts_frac = (ts_us % 1_000_000) as u32;
                    let pkt_off = off + 28;
                    if pkt_off + cap_len <= data.len() {
                        let pkt_data = data[pkt_off..pkt_off + cap_len].to_vec();
                        packets.push(Packet {
                            num,
                            ts_sec,
                            ts_frac,
                            orig_len,
                            data: pkt_data,
                            link_type,
                        });
                        num += 1;
                    }
                }
            }
            _ => {}
        }
        off += block_len;
    }

    Ok(PcapFile {
        format: "PCAPNG".into(),
        link_type,
        packets,
        nano_ts: false,
        is_be: be,
    })
}

fn r16(data: &[u8], off: usize, be: bool) -> Option<u16> {
    if be {
        r16be(data, off)
    } else {
        r16le(data, off)
    }
}

// ── TCP/UDP port labels ────────────────────────────────────────────────────────

fn port_proto(port: u16) -> Option<&'static str> {
    match port {
        20 | 21 => Some("FTP"),
        22 => Some("SSH"),
        23 => Some("Telnet"),
        25 => Some("SMTP"),
        53 => Some("DNS"),
        67 | 68 => Some("DHCP"),
        80 => Some("HTTP"),
        110 => Some("POP3"),
        123 => Some("NTP"),
        143 => Some("IMAP"),
        161 | 162 => Some("SNMP"),
        443 => Some("HTTPS"),
        445 => Some("SMB"),
        465 => Some("SMTPS"),
        587 => Some("SMTP"),
        993 => Some("IMAPS"),
        995 => Some("POP3S"),
        1194 => Some("OpenVPN"),
        1234 => Some("LM Studio"),
        3306 => Some("MySQL"),
        3389 => Some("RDP"),
        5432 => Some("PostgreSQL"),
        5672 => Some("AMQP"),
        6379 => Some("Redis"),
        8080 | 8000 | 8888 => Some("HTTP-Alt"),
        11434 => Some("Ollama"),
        27017 => Some("MongoDB"),
        _ => None,
    }
}

#[allow(dead_code)]
fn fmt_endpoint(ip: &str, payload: &Option<Vec<u8>>, ip_proto: u8) -> String {
    if let Some(ref p) = payload {
        if (ip_proto == 6 || ip_proto == 17) && p.len() >= 4 {
            let sport = u16::from_be_bytes([p[0], p[1]]);
            let dport = u16::from_be_bytes([p[2], p[3]]);
            let _ = (sport, dport); // used via caller
        }
    }
    ip.to_string()
}

// ── DNS parser ─────────────────────────────────────────────────────────────────

fn parse_dns(buf: &[u8]) -> Option<(String, String, Vec<String>)> {
    if buf.len() < 12 {
        return None;
    }
    let qr = (buf[2] >> 7) & 1;
    let _qdcount = u16::from_be_bytes([buf[4], buf[5]]) as usize;
    let ancount = u16::from_be_bytes([buf[6], buf[7]]) as usize;

    // Parse first question
    let mut off = 12usize;
    let mut qname = String::new();
    loop {
        if off >= buf.len() {
            return None;
        }
        let len = buf[off] as usize;
        if len == 0 {
            off += 1;
            break;
        }
        if (len & 0xC0) == 0xC0 {
            // pointer
            off += 2;
            break;
        }
        off += 1;
        if off + len > buf.len() {
            return None;
        }
        if !qname.is_empty() {
            qname.push('.');
        }
        qname.push_str(&String::from_utf8_lossy(&buf[off..off + len]));
        off += len;
    }
    if off + 4 > buf.len() {
        return None;
    }
    let qtype = u16::from_be_bytes([buf[off], buf[off + 1]]);
    let qtype_str = match qtype {
        1 => "A",
        28 => "AAAA",
        5 => "CNAME",
        15 => "MX",
        16 => "TXT",
        2 => "NS",
        6 => "SOA",
        12 => "PTR",
        _ => "?",
    };
    off += 4;

    // Parse answers
    let mut answers: Vec<String> = Vec::new();
    for _ in 0..ancount {
        // skip name
        off = skip_dns_name(buf, off);
        if off + 10 > buf.len() {
            break;
        }
        let rtype = u16::from_be_bytes([buf[off], buf[off + 1]]);
        let rdlen = u16::from_be_bytes([buf[off + 8], buf[off + 9]]) as usize;
        off += 10;
        if off + rdlen > buf.len() {
            break;
        }
        let rdata = &buf[off..off + rdlen];
        match rtype {
            1 if rdlen == 4 => answers.push(format!(
                "{}.{}.{}.{}",
                rdata[0], rdata[1], rdata[2], rdata[3]
            )),
            28 if rdlen == 16 => answers.push(fmt_ipv6(rdata)),
            5 => {
                let cname = parse_dns_name(buf, off);
                answers.push(cname);
            }
            _ => answers.push(format!("<rtype={}>", rtype)),
        }
        off += rdlen;
    }

    let kind = if qr == 0 { "Query" } else { "Response" };
    Some((qname, format!("{} {}", kind, qtype_str), answers))
}

fn skip_dns_name(buf: &[u8], mut off: usize) -> usize {
    loop {
        if off >= buf.len() {
            return off;
        }
        let l = buf[off] as usize;
        if l == 0 {
            return off + 1;
        }
        if (l & 0xC0) == 0xC0 {
            return off + 2;
        }
        off += 1 + l;
    }
}

fn parse_dns_name(buf: &[u8], off: usize) -> String {
    let mut name = String::new();
    let mut cur = off;
    let mut hops = 0u8;
    loop {
        if cur >= buf.len() || hops > 20 {
            break;
        }
        let l = buf[cur] as usize;
        if l == 0 {
            break;
        }
        if (l & 0xC0) == 0xC0 {
            if cur + 1 >= buf.len() {
                break;
            }
            cur = ((((l & 0x3F) as u16) << 8) | buf[cur + 1] as u16) as usize;
            hops += 1;
            continue;
        }
        cur += 1;
        if cur + l > buf.len() {
            break;
        }
        if !name.is_empty() {
            name.push('.');
        }
        name.push_str(&String::from_utf8_lossy(&buf[cur..cur + l]));
        cur += l;
    }
    name
}

// ── HTTP line parser ───────────────────────────────────────────────────────────

fn try_parse_http(payload: &[u8]) -> Option<String> {
    let s = std::str::from_utf8(payload).ok()?;
    let first_line = s.lines().next()?;
    // Request: METHOD path HTTP/1.x
    if let Some(rest) = first_line
        .strip_prefix("GET ")
        .or_else(|| first_line.strip_prefix("POST "))
        .or_else(|| first_line.strip_prefix("PUT "))
        .or_else(|| first_line.strip_prefix("DELETE "))
        .or_else(|| first_line.strip_prefix("HEAD "))
        .or_else(|| first_line.strip_prefix("OPTIONS "))
        .or_else(|| first_line.strip_prefix("PATCH "))
    {
        let method = first_line.split_whitespace().next()?;
        let path = rest.split_whitespace().next().unwrap_or("/");
        let host = s
            .lines()
            .find(|l| l.to_lowercase().starts_with("host:"))
            .and_then(|l| l.split_once(':').map(|(_, v)| v))
            .map(|h| h.trim())
            .unwrap_or("-");
        return Some(format!("REQUEST {} {} (host: {})", method, path, host));
    }
    // Response: HTTP/1.x STATUS
    if first_line.starts_with("HTTP/") {
        let code = first_line.split_whitespace().nth(1).unwrap_or("?");
        return Some(format!("RESPONSE {}", code));
    }
    None
}

// ── actions ───────────────────────────────────────────────────────────────────

fn action_info(pcap: &PcapFile) -> String {
    let count = pcap.packets.len();
    let (first_ts, last_ts) = if count == 0 {
        (0.0f64, 0.0f64)
    } else {
        (
            pcap.packets.first().unwrap().timestamp_f(),
            pcap.packets.last().unwrap().timestamp_f(),
        )
    };
    let duration = last_ts - first_ts;
    let total_bytes: u64 = pcap.packets.iter().map(|p| p.orig_len as u64).sum();
    let cap_bytes: u64 = pcap.packets.iter().map(|p| p.data.len() as u64).sum();

    let mut out = String::new();
    out.push_str("PCAP FILE INFO\n");
    out.push_str(&format!("  Format    : {}\n", pcap.format));
    out.push_str(&format!(
        "  Byte order: {}\n",
        if pcap.is_be {
            "Big-Endian"
        } else {
            "Little-Endian"
        }
    ));
    out.push_str(&format!(
        "  Link type : {} ({})\n",
        pcap.link_type,
        link_type_name(pcap.link_type)
    ));
    out.push_str(&format!("  Packets   : {}\n", count));
    if count > 0 {
        out.push_str(&format!(
            "  Duration  : {:.3}s  ({:.1} pps avg)\n",
            duration,
            if duration > 0.0 {
                count as f64 / duration
            } else {
                0.0
            }
        ));
        out.push_str(&format!(
            "  Total bytes    : {} ({} MB)\n",
            total_bytes,
            total_bytes / 1_048_576
        ));
        out.push_str(&format!(
            "  Captured bytes : {} ({} MB)\n",
            cap_bytes,
            cap_bytes / 1_048_576
        ));
        let avg = total_bytes / count as u64;
        out.push_str(&format!("  Avg pkt len: {} bytes\n", avg));
    }
    out
}

fn action_packets(pcap: &PcapFile, limit: usize) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "{:>5}  {:>12}  {:>6}  {:>12}  {:>15}  {:>15}  {}\n",
        "No.", "Time (s)", "Len", "Protocol", "Source", "Destination", "Info"
    ));
    out.push_str(&"-".repeat(90));
    out.push('\n');

    let base_ts = pcap.packets.first().map(|p| p.timestamp_f()).unwrap_or(0.0);

    for pkt in pcap.packets.iter().take(limit) {
        let rel_ts = pkt.timestamp_f() - base_ts;
        let (proto, src, dst, payload, ip_proto) = pkt.dissect();

        // Annotate port-based protocol
        let proto_label = if ip_proto == 6 || ip_proto == 17 {
            if let Some(ref p) = payload {
                if p.len() >= 4 {
                    let sp = u16::from_be_bytes([p[0], p[1]]);
                    let dp = u16::from_be_bytes([p[2], p[3]]);
                    let label = port_proto(sp).or_else(|| port_proto(dp));
                    if let Some(l) = label {
                        format!("{}/{}", proto, l)
                    } else {
                        proto.clone()
                    }
                } else {
                    proto.clone()
                }
            } else {
                proto.clone()
            }
        } else {
            proto.clone()
        };

        let src_short = if src.len() > 15 { &src[..15] } else { &src };
        let dst_short = if dst.len() > 15 { &dst[..15] } else { &dst };

        // Build info string
        let info = if let Some(ref p) = payload {
            if ip_proto == 17 && p.len() > 8 {
                let dp = u16::from_be_bytes([p[2], p[3]]);
                if dp == 53 || u16::from_be_bytes([p[0], p[1]]) == 53 {
                    if let Some((name, kind, _)) = parse_dns(&p[8..]) {
                        format!("{} {}", kind, name)
                    } else {
                        String::new()
                    }
                } else {
                    String::new()
                }
            } else if ip_proto == 6 && p.len() > 4 {
                let dp = u16::from_be_bytes([p[2], p[3]]);
                let tcp_data_off = ((p[12] as usize) >> 4) * 4;
                if dp == 80 || dp == 8080 || dp == 8000 {
                    p.get(tcp_data_off..)
                        .and_then(try_parse_http)
                        .unwrap_or_default()
                } else {
                    String::new()
                }
            } else {
                String::new()
            }
        } else {
            String::new()
        };

        out.push_str(&format!(
            "{:>5}  {:>12.6}  {:>6}  {:>12}  {:>15}  {:>15}  {}\n",
            pkt.num, rel_ts, pkt.orig_len, proto_label, src_short, dst_short, info
        ));
    }

    let total = pcap.packets.len();
    if total > limit {
        out.push_str(&format!("... {} more packets\n", total - limit));
    }
    out
}

fn action_protocols(pcap: &PcapFile) -> String {
    let mut counts: HashMap<String, usize> = HashMap::new();
    let total = pcap.packets.len();

    for pkt in &pcap.packets {
        let (proto, _, _, payload, ip_proto) = pkt.dissect();

        // Count layer-2
        if pkt.link_type == 1 && pkt.data.len() >= 14 {
            *counts.entry("Ethernet".into()).or_insert(0) += 1;
        }

        // Refine UDP/TCP by port
        let final_proto = if ip_proto == 6 || ip_proto == 17 {
            if let Some(ref p) = payload {
                if p.len() >= 4 {
                    let sp = u16::from_be_bytes([p[0], p[1]]);
                    let dp = u16::from_be_bytes([p[2], p[3]]);
                    if let Some(l) = port_proto(sp).or_else(|| port_proto(dp)) {
                        l.to_string()
                    } else {
                        proto
                    }
                } else {
                    proto
                }
            } else {
                proto
            }
        } else {
            proto
        };

        *counts.entry(final_proto).or_insert(0) += 1;
    }

    let mut entries: Vec<(String, usize)> = counts.into_iter().collect();
    entries.sort_by(|a, b| b.1.cmp(&a.1));

    let mut out = String::new();
    out.push_str("PROTOCOL DISTRIBUTION\n");
    out.push_str(&format!("  Total packets: {}\n\n", total));
    out.push_str(&format!(
        "{:<14}  {:>8}  {:>7}  {}\n",
        "Protocol", "Packets", "Percent", "Bar"
    ));
    out.push_str(&"-".repeat(55));
    out.push('\n');
    for (proto, cnt) in &entries {
        let pct = if total > 0 {
            *cnt as f64 / total as f64 * 100.0
        } else {
            0.0
        };
        let bar_len = (pct / 2.0) as usize;
        let bar: String = "█".repeat(bar_len);
        out.push_str(&format!(
            "{:<14}  {:>8}  {:>6.1}%  {}\n",
            proto, cnt, pct, bar
        ));
    }
    out
}

fn action_conversations(pcap: &PcapFile) -> String {
    // Key: (a, b) where a < b lexicographically
    let mut pairs: HashMap<(String, String), (usize, u64)> = HashMap::new();

    for pkt in &pcap.packets {
        let (_, src, dst, _, _) = pkt.dissect();
        if src == "?" || dst == "?" {
            continue;
        }
        let key = if src <= dst {
            (src.clone(), dst.clone())
        } else {
            (dst.clone(), src.clone())
        };
        let e = pairs.entry(key).or_insert((0, 0));
        e.0 += 1;
        e.1 += pkt.orig_len as u64;
    }

    let mut entries: Vec<((String, String), (usize, u64))> = pairs.into_iter().collect();
    entries.sort_by(|a, b| b.1 .0.cmp(&a.1 .0));

    let mut out = String::new();
    out.push_str("TOP CONVERSATIONS\n");
    out.push_str(&format!(
        "{:<20}  {:<20}  {:>8}  {:>12}\n",
        "Host A", "Host B", "Packets", "Bytes"
    ));
    out.push_str(&"-".repeat(68));
    out.push('\n');

    for ((a, b), (pkts, bytes)) in entries.iter().take(20) {
        let a_s = if a.len() > 20 { &a[..20] } else { a };
        let b_s = if b.len() > 20 { &b[..20] } else { b };
        out.push_str(&format!(
            "{:<20}  {:<20}  {:>8}  {:>12}\n",
            a_s, b_s, pkts, bytes
        ));
    }
    if entries.len() > 20 {
        out.push_str(&format!("... {} more conversations\n", entries.len() - 20));
    }
    out
}

fn action_dns(pcap: &PcapFile) -> String {
    let mut out = String::new();
    out.push_str("DNS QUERIES / RESPONSES\n");
    out.push_str(&format!(
        "{:>5}  {:<35}  {:<18}  {}\n",
        "No.", "Name", "Type", "Answer(s)"
    ));
    out.push_str(&"-".repeat(80));
    out.push('\n');

    let mut found = 0;
    for pkt in &pcap.packets {
        let (_, _, _, payload, ip_proto) = pkt.dissect();
        if ip_proto != 17 {
            continue;
        }
        if let Some(ref p) = payload {
            if p.len() < 9 {
                continue;
            }
            let sp = u16::from_be_bytes([p[0], p[1]]);
            let dp = u16::from_be_bytes([p[2], p[3]]);
            if sp != 53 && dp != 53 {
                continue;
            }
            if let Some((name, kind, answers)) = parse_dns(&p[8..]) {
                let ans = if answers.is_empty() {
                    "-".into()
                } else {
                    answers.join(", ")
                };
                let name_s = if name.len() > 35 { &name[..35] } else { &name };
                out.push_str(&format!(
                    "{:>5}  {:<35}  {:<18}  {}\n",
                    pkt.num, name_s, kind, ans
                ));
                found += 1;
            }
        }
    }
    if found == 0 {
        out.push_str("  No DNS packets found.\n");
    }
    out
}

fn action_http(pcap: &PcapFile) -> String {
    let mut out = String::new();
    out.push_str("HTTP TRAFFIC\n");
    out.push_str(&format!(
        "{:>5}  {:>15}  {:>15}  {}\n",
        "No.", "Source", "Destination", "Info"
    ));
    out.push_str(&"-".repeat(80));
    out.push('\n');

    let mut found = 0;
    for pkt in &pcap.packets {
        let (_, src, dst, payload, ip_proto) = pkt.dissect();
        if ip_proto != 6 {
            continue;
        }
        if let Some(ref p) = payload {
            if p.len() < 5 {
                continue;
            }
            let dp = u16::from_be_bytes([p[2], p[3]]);
            if dp != 80 && dp != 8080 && dp != 8000 && dp != 443 {
                continue;
            }
            let tcp_hdr = ((p[12] as usize) >> 4) * 4;
            if let Some(http_data) = p.get(tcp_hdr..) {
                if let Some(info) = try_parse_http(http_data) {
                    let src_s = if src.len() > 15 { &src[..15] } else { &src };
                    let dst_s = if dst.len() > 15 { &dst[..15] } else { &dst };
                    out.push_str(&format!(
                        "{:>5}  {:>15}  {:>15}  {}\n",
                        pkt.num, src_s, dst_s, info
                    ));
                    found += 1;
                }
            }
        }
    }
    if found == 0 {
        out.push_str("  No HTTP traffic found (port 80/8080/8000).\n");
    }
    out
}

// ── entry point ───────────────────────────────────────────────────────────────

pub async fn execute(args: &Value) -> Result<String, String> {
    let file = args["file"]
        .as_str()
        .ok_or_else(|| "Required: 'file' path to a .pcap or .pcapng file.".to_string())?;

    let data = fs::read(file).map_err(|e| format!("Cannot read '{}': {}", file, e))?;

    let pcap = parse_pcap(&data)?;

    let action = args["action"].as_str().unwrap_or("info");
    let limit = args["limit"].as_u64().unwrap_or(20) as usize;

    let result = match action {
        "info" => action_info(&pcap),
        "packets" => action_packets(&pcap, limit),
        "protocols" => action_protocols(&pcap),
        "conversations" => action_conversations(&pcap),
        "dns" => action_dns(&pcap),
        "http" => action_http(&pcap),
        other => {
            return Err(format!(
                "Unknown action '{}'. Valid: info, packets, protocols, conversations, dns, http.",
                other
            ))
        }
    };

    Ok(result)
}
