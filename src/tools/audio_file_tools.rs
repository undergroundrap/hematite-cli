use serde_json::{json, Value};
use std::fs;

pub fn make_schema() -> Value {
    json!({
        "name": "audio_file_tools",
        "description": "Parse audio file metadata (WAV/MP3/FLAC/Ogg Vorbis/Opus) without external utilities. \
    Actions: info (default — format, encoding, channels, sample rate, bit depth, duration, bitrate, tags), \
    tags (ID3 or Vorbis comment fields only: artist, title, album, year, genre, track, etc.), \
    validate (header integrity, tag completeness, byte-rate consistency). \
    Pass file (path to audio file) or hex (hex-encoded bytes). \
    Example: audio_file_tools(file: 'song.mp3') or audio_file_tools(action: 'tags', file: 'track.flac')",
        "input_schema": {
            "type": "object",
            "properties": {
                "action": { "type": "string", "description": "info|tags|validate" },
                "file": { "type": "string", "description": "Path to WAV, MP3, FLAC, or Ogg file" },
                "hex": { "type": "string", "description": "Hex-encoded audio bytes" }
            },
            "required": []
        }
    })
}

fn get_bytes(args: &Value) -> Option<Vec<u8>> {
    if let Some(p) = args.get("file").and_then(|v| v.as_str()) {
        fs::read(p).ok()
    } else if let Some(h) = args
        .get("hex")
        .or_else(|| args.get("bytes"))
        .and_then(|v| v.as_str())
    {
        let clean: String = h.chars().filter(|c| c.is_ascii_hexdigit()).collect();
        if clean.len() % 2 != 0 {
            return None;
        }
        (0..clean.len() / 2)
            .map(|i| u8::from_str_radix(&clean[i * 2..i * 2 + 2], 16))
            .collect::<Result<Vec<_>, _>>()
            .ok()
    } else {
        None
    }
}

fn ru16be(b: &[u8], o: usize) -> u16 {
    if o + 1 >= b.len() {
        return 0;
    }
    u16::from_be_bytes([b[o], b[o + 1]])
}
fn ru16le(b: &[u8], o: usize) -> u16 {
    if o + 1 >= b.len() {
        return 0;
    }
    u16::from_le_bytes([b[o], b[o + 1]])
}
fn ru24be(b: &[u8], o: usize) -> u32 {
    if o + 2 >= b.len() {
        return 0;
    }
    ((b[o] as u32) << 16) | ((b[o + 1] as u32) << 8) | (b[o + 2] as u32)
}
#[allow(dead_code)]
fn ru24le(b: &[u8], o: usize) -> u32 {
    if o + 2 >= b.len() {
        return 0;
    }
    (b[o] as u32) | ((b[o + 1] as u32) << 8) | ((b[o + 2] as u32) << 16)
}
fn ru32be(b: &[u8], o: usize) -> u32 {
    if o + 3 >= b.len() {
        return 0;
    }
    u32::from_be_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]])
}
fn ru32le(b: &[u8], o: usize) -> u32 {
    if o + 3 >= b.len() {
        return 0;
    }
    u32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]])
}

fn human_size(n: u64) -> String {
    if n < 1024 {
        format!("{} B", n)
    } else if n < 1_048_576 {
        format!("{:.1} KB", n as f64 / 1024.0)
    } else {
        format!("{:.2} MB", n as f64 / 1_048_576.0)
    }
}

fn human_dur(secs: f64) -> String {
    if secs < 60.0 {
        format!("{:.1}s", secs)
    } else {
        let m = (secs / 60.0) as u64;
        let s = secs as u64 % 60;
        format!("{}m {}s", m, s)
    }
}

// ── WAV ─────────────────────────────────────────────────────────────────────

struct WavMeta {
    channels: u16,
    sample_rate: u32,
    bits_per_sample: u16,
    audio_format: u16,
    byte_rate: u32,
    data_size: u32,
    file_size: u64,
}

fn parse_wav(b: &[u8]) -> Option<WavMeta> {
    if b.len() < 44 {
        return None;
    }
    if &b[0..4] != b"RIFF" || &b[8..12] != b"WAVE" {
        return None;
    }
    let file_size = b.len() as u64;
    let mut pos = 12usize;
    let mut fmt_off = 0usize;
    let mut data_size = 0u32;
    while pos + 8 <= b.len() {
        let tag = &b[pos..pos + 4];
        let chunk_size = ru32le(b, pos + 4) as usize;
        if tag == b"fmt " {
            fmt_off = pos + 8;
        }
        if tag == b"data" {
            data_size = chunk_size as u32;
        }
        pos += 8 + chunk_size + (chunk_size & 1);
        if pos >= b.len() {
            break;
        }
    }
    if fmt_off + 16 > b.len() {
        return None;
    }
    Some(WavMeta {
        audio_format: ru16le(b, fmt_off),
        channels: ru16le(b, fmt_off + 2),
        sample_rate: ru32le(b, fmt_off + 4),
        byte_rate: ru32le(b, fmt_off + 8),
        bits_per_sample: ru16le(b, fmt_off + 14),
        data_size,
        file_size,
    })
}

fn format_name(n: u16) -> &'static str {
    match n {
        1 => "PCM",
        2 => "ADPCM",
        3 => "IEEE Float",
        6 => "G.711 A-law",
        7 => "G.711 µ-law",
        17 => "IMA ADPCM",
        65534 => "WAVE_FORMAT_EXTENSIBLE",
        _ => "Unknown",
    }
}

fn dispatch_wav(action: &str, b: &[u8]) -> String {
    let m = match parse_wav(b) {
        Some(m) => m,
        None => return "Error: not a valid WAV/RIFF file.".to_string(),
    };
    let duration_secs = if m.byte_rate > 0 {
        m.data_size as f64 / m.byte_rate as f64
    } else {
        0.0
    };
    match action {
        "info" | "metadata" => {
            let mut out = vec![
                "Format:          WAV / RIFF".to_string(),
                format!(
                    "Audio encoding:  {} (code {})",
                    format_name(m.audio_format),
                    m.audio_format
                ),
                format!("Channels:        {}", m.channels),
                format!("Sample rate:     {} Hz", m.sample_rate),
                format!("Bit depth:       {}-bit", m.bits_per_sample),
                format!(
                    "Byte rate:       {} B/s ({:.0} kbps)",
                    m.byte_rate,
                    m.byte_rate as f64 * 8.0 / 1000.0
                ),
                format!(
                    "Data size:       {} ({})",
                    m.data_size,
                    human_size(m.data_size as u64)
                ),
                format!("Duration:        {}", human_dur(duration_secs)),
                format!("File size:       {}", human_size(m.file_size)),
            ];
            if m.channels == 1 {
                out.push("Channel layout:  Mono".to_string());
            } else if m.channels == 2 {
                out.push("Channel layout:  Stereo".to_string());
            } else {
                out.push(format!("Channel layout:  {} channels", m.channels));
            }
            out.join("\n")
        }
        "duration" => format!(
            "Duration: {} ({:.3} s)",
            human_dur(duration_secs),
            duration_secs
        ),
        "validate" => {
            let mut issues = vec![];
            if m.audio_format != 1 && m.audio_format != 3 && m.audio_format != 65534 {
                issues.push(format!("Non-PCM encoding: code {}", m.audio_format));
            }
            if m.channels == 0 {
                issues.push("Invalid channel count: 0".to_string());
            }
            if m.sample_rate == 0 {
                issues.push("Invalid sample rate: 0".to_string());
            }
            if m.bits_per_sample == 0 {
                issues.push("Invalid bit depth: 0".to_string());
            }
            if m.data_size == 0 {
                issues.push("Empty data chunk".to_string());
            }
            let expected_br =
                (m.sample_rate as u32) * (m.channels as u32) * (m.bits_per_sample as u32 / 8);
            if m.audio_format == 1 && expected_br != m.byte_rate {
                issues.push(format!(
                    "Byte rate mismatch: header={} expected={}",
                    m.byte_rate, expected_br
                ));
            }
            if issues.is_empty() {
                "VALID — WAV file structure is well-formed.".to_string()
            } else {
                format!(
                    "WARNINGS:\n{}",
                    issues
                        .iter()
                        .map(|s| format!("  • {}", s))
                        .collect::<Vec<_>>()
                        .join("\n")
                )
            }
        }
        _ => dispatch_wav("info", b),
    }
}

// ── MP3 ─────────────────────────────────────────────────────────────────────

struct Id3v2Tag {
    version: u8,
    revision: u8,
    flags: u8,
    size: u32,
    frames: Vec<(String, String)>,
}

fn decode_syncsafe(b: &[u8]) -> u32 {
    ((b[0] as u32) << 21) | ((b[1] as u32) << 14) | ((b[2] as u32) << 7) | (b[3] as u32)
}

fn parse_id3v2(b: &[u8]) -> Option<Id3v2Tag> {
    if b.len() < 10 || &b[0..3] != b"ID3" {
        return None;
    }
    let version = b[3];
    let revision = b[4];
    let flags = b[5];
    let size = decode_syncsafe(&b[6..10]);
    let tag_end = (10 + size) as usize;
    if tag_end > b.len() {
        return None;
    }
    let mut frames = vec![];
    let mut pos = 10usize;
    while pos + 10 <= tag_end {
        let fid: String = b[pos..pos + 4].iter().map(|&c| c as char).collect();
        if fid.starts_with('\0') {
            break;
        }
        let fsize = if version >= 4 {
            decode_syncsafe(&b[pos + 4..pos + 8]) as usize
        } else {
            ru32be(b, pos + 4) as usize
        };
        pos += 10;
        if fsize == 0 || pos + fsize > tag_end {
            break;
        }
        let fdata = &b[pos..pos + fsize];
        if fid.starts_with('T') || fid == "COMM" || fid == "USLT" {
            let encoding = fdata[0];
            let text_bytes = &fdata[1..];
            let text = decode_id3_text(text_bytes, encoding);
            if !text.is_empty() {
                frames.push((fid.clone(), text));
            }
        }
        pos += fsize;
    }
    Some(Id3v2Tag {
        version,
        revision,
        flags,
        size,
        frames,
    })
}

fn decode_id3_text(b: &[u8], encoding: u8) -> String {
    match encoding {
        0 => b
            .iter()
            .take_while(|&&c| c != 0)
            .map(|&c| c as char)
            .collect(),
        1 | 2 => {
            let (start, bom_be) = if b.len() >= 2 && b[0] == 0xFF && b[1] == 0xFE {
                (2usize, false)
            } else if b.len() >= 2 && b[0] == 0xFE && b[1] == 0xFF {
                (2usize, true)
            } else {
                (0usize, encoding == 2)
            };
            let pairs: Vec<u16> = b[start..]
                .chunks(2)
                .filter(|c| c.len() == 2)
                .take_while(|c| c[0] != 0 || c[1] != 0)
                .map(|c| {
                    if bom_be {
                        u16::from_be_bytes([c[0], c[1]])
                    } else {
                        u16::from_le_bytes([c[0], c[1]])
                    }
                })
                .collect();
            String::from_utf16_lossy(&pairs)
        }
        3 => String::from_utf8_lossy(
            b.iter()
                .take_while(|&&c| c != 0)
                .cloned()
                .collect::<Vec<_>>()
                .as_slice(),
        )
        .into_owned(),
        _ => String::new(),
    }
}

fn read_id3v1(b: &[u8]) -> Vec<(String, String)> {
    if b.len() < 128 {
        return vec![];
    }
    let start = b.len() - 128;
    if &b[start..start + 3] != b"TAG" {
        return vec![];
    }
    let latin1 = |slice: &[u8]| -> String {
        slice
            .iter()
            .take_while(|&&c| c != 0)
            .map(|&c| c as char)
            .collect::<String>()
            .trim()
            .to_string()
    };
    let mut tags = vec![];
    let t = latin1(&b[start + 3..start + 33]);
    if !t.is_empty() {
        tags.push(("TIT2".to_string(), t));
    }
    let a = latin1(&b[start + 33..start + 63]);
    if !a.is_empty() {
        tags.push(("TPE1".to_string(), a));
    }
    let al = latin1(&b[start + 63..start + 93]);
    if !al.is_empty() {
        tags.push(("TALB".to_string(), al));
    }
    let y = latin1(&b[start + 93..start + 97]);
    if !y.is_empty() {
        tags.push(("TDRC".to_string(), y));
    }
    tags
}

const MP3_BITRATES: [[u32; 16]; 5] = [
    [
        0, 32, 64, 96, 128, 160, 192, 224, 256, 288, 320, 352, 384, 416, 448, 0,
    ], // MPEG1 L1
    [
        0, 32, 48, 56, 64, 80, 96, 112, 128, 160, 192, 224, 256, 320, 384, 0,
    ], // MPEG1 L2
    [
        0, 32, 40, 48, 56, 64, 80, 96, 112, 128, 160, 192, 224, 256, 320, 0,
    ], // MPEG1 L3
    [
        0, 32, 48, 56, 64, 80, 96, 112, 128, 144, 160, 176, 192, 224, 256, 0,
    ], // MPEG2 L1
    [
        0, 8, 16, 24, 32, 40, 48, 56, 64, 80, 96, 112, 128, 144, 160, 0,
    ], // MPEG2 L2/L3
];
const MP3_SAMPLERATES: [[u32; 4]; 3] = [
    [44100, 48000, 32000, 0],
    [22050, 24000, 16000, 0],
    [11025, 12000, 8000, 0],
];

struct Mp3Frame {
    bitrate: u32,
    sample_rate: u32,
    channels: u8,
    layer: u8,
    mpeg_ver: u8,
}

fn find_mp3_frame(b: &[u8], start: usize) -> Option<Mp3Frame> {
    let mut i = start;
    while i + 4 <= b.len() {
        if b[i] != 0xFF || (b[i + 1] & 0xE0) != 0xE0 {
            i += 1;
            continue;
        }
        let h = ((b[i] as u32) << 24)
            | ((b[i + 1] as u32) << 16)
            | ((b[i + 2] as u32) << 8)
            | (b[i + 3] as u32);
        let mpeg_idx = ((h >> 19) & 3) as usize;
        let layer_idx = ((h >> 17) & 3) as usize;
        let br_idx = ((h >> 12) & 0xF) as usize;
        let sr_idx = ((h >> 10) & 3) as usize;
        let ch = ((h >> 6) & 3) as u8;
        if mpeg_idx == 1 || layer_idx == 0 || br_idx == 0 || br_idx == 15 || sr_idx == 3 {
            i += 1;
            continue;
        }
        let mpeg_ver: u8 = match mpeg_idx {
            3 => 1,
            2 => 2,
            0 => 25,
            _ => 0,
        };
        let layer: u8 = 4u8.saturating_sub(layer_idx as u8);
        let row = if mpeg_ver == 1 {
            layer as usize - 1
        } else if layer == 1 {
            3
        } else {
            4
        };
        let bitrate = MP3_BITRATES[row][br_idx] * 1000;
        let sr_row = if mpeg_ver == 1 {
            0
        } else if mpeg_ver == 2 {
            1
        } else {
            2
        };
        let sample_rate = MP3_SAMPLERATES[sr_row][sr_idx];
        if bitrate == 0 || sample_rate == 0 {
            i += 1;
            continue;
        }
        let channels = if ch == 3 { 1 } else { 2 };
        return Some(Mp3Frame {
            bitrate,
            sample_rate,
            channels,
            layer,
            mpeg_ver,
        });
    }
    None
}

fn dispatch_mp3(action: &str, b: &[u8]) -> String {
    if b.len() < 4 {
        return "Error: file too small.".to_string();
    }
    let has_id3v2 = &b[0..3] == b"ID3";
    let id3v2 = if has_id3v2 { parse_id3v2(b) } else { None };
    let id3v1 = read_id3v1(b);
    let tags: Vec<(String, String)> = id3v2
        .as_ref()
        .map(|t| t.frames.clone())
        .unwrap_or_default()
        .into_iter()
        .chain(if id3v2.is_none() {
            id3v1.clone()
        } else {
            vec![]
        })
        .collect();
    let frame_start = id3v2.as_ref().map(|t| 10 + t.size as usize).unwrap_or(0);
    let frame = find_mp3_frame(b, frame_start);
    let file_size = b.len() as u64;

    let tag_label = |k: &str| -> String {
        match k {
            "TIT2" => "Title".into(),
            "TPE1" => "Artist".into(),
            "TALB" => "Album".into(),
            "TDRC" | "TYER" => "Year".into(),
            "TRCK" => "Track".into(),
            "TCON" => "Genre".into(),
            "TCOM" => "Composer".into(),
            "TPUB" => "Publisher".into(),
            "TCOP" => "Copyright".into(),
            "COMM" => "Comment".into(),
            "TPOS" => "Disc".into(),
            "TBPM" => "BPM".into(),
            _ => k.into(),
        }
    };

    match action {
        "info" | "metadata" => {
            let mut out = vec!["Format:  MP3".to_string()];
            if let Some(ref f) = frame {
                let ver = match f.mpeg_ver {
                    1 => "MPEG-1",
                    2 => "MPEG-2",
                    _ => "MPEG-2.5",
                };
                out.push(format!("Encoding:  {} Layer {}", ver, f.layer));
                out.push(format!("Bitrate:   {} kbps", f.bitrate / 1000));
                out.push(format!("Sample rate: {} Hz", f.sample_rate));
                out.push(format!(
                    "Channels:  {}",
                    if f.channels == 1 { "Mono" } else { "Stereo" }
                ));
            }
            if let Some(ref t) = id3v2 {
                out.push(format!("ID3v2: v2.{}.{}", t.version, t.revision));
                if t.flags & 0x40 != 0 {
                    out.push("  Extended header present".to_string());
                }
                if t.flags & 0x10 != 0 {
                    out.push("  Footer present".to_string());
                }
            }
            if !id3v1.is_empty() {
                out.push("ID3v1: present".to_string());
            }
            for (k, v) in &tags {
                out.push(format!("  {}: {}", tag_label(k), v));
            }
            out.push(format!("File size: {}", human_size(file_size)));
            out.join("\n")
        }
        "tags" => {
            if tags.is_empty() {
                return "No ID3 tags found.".to_string();
            }
            tags.iter()
                .map(|(k, v)| format!("{}: {}", tag_label(k), v))
                .collect::<Vec<_>>()
                .join("\n")
        }
        "validate" => {
            let mut issues = vec![];
            if !has_id3v2 && id3v1.is_empty() {
                issues.push("No ID3 tags found".to_string());
            }
            if frame.is_none() {
                issues.push("No valid MPEG frame header found".to_string());
            }
            if tags.iter().all(|(k, _)| k != "TIT2") {
                issues.push("Missing title tag (TIT2)".to_string());
            }
            if tags.iter().all(|(k, _)| k != "TPE1") {
                issues.push("Missing artist tag (TPE1)".to_string());
            }
            if issues.is_empty() {
                "VALID — MP3 structure and tags look well-formed.".to_string()
            } else {
                format!(
                    "WARNINGS:\n{}",
                    issues
                        .iter()
                        .map(|s| format!("  • {}", s))
                        .collect::<Vec<_>>()
                        .join("\n")
                )
            }
        }
        _ => dispatch_mp3("info", b),
    }
}

// ── FLAC ─────────────────────────────────────────────────────────────────────

struct FlacInfo {
    min_block: u32,
    max_block: u32,
    min_frame: u32,
    max_frame: u32,
    sample_rate: u32,
    channels: u8,
    bits_per_sample: u8,
    total_samples: u64,
    file_size: u64,
    vorbis_comments: Vec<(String, String)>,
}

fn parse_flac(b: &[u8]) -> Option<FlacInfo> {
    if b.len() < 42 || &b[0..4] != b"fLaC" {
        return None;
    }
    let mut pos = 4usize;
    let mut info: Option<FlacInfo> = None;
    let mut comments = vec![];
    loop {
        if pos + 4 > b.len() {
            break;
        }
        let block_type = b[pos] & 0x7F;
        let is_last = (b[pos] & 0x80) != 0;
        let block_len = ru24be(b, pos + 1) as usize;
        pos += 4;
        if pos + block_len > b.len() {
            break;
        }
        match block_type {
            0 => {
                // STREAMINFO — 34 bytes
                if block_len >= 34 {
                    let d = &b[pos..pos + 34];
                    let min_block = ru16be(d, 0) as u32;
                    let max_block = ru16be(d, 2) as u32;
                    let min_frame = ru24be(d, 4);
                    let max_frame = ru24be(d, 7);
                    // bits 0-19 = sample_rate, bits 20-22 = channels-1, bits 23-27 = bps-1, bits 28-27+36 = total_samples
                    let sr = ((d[10] as u32) << 12) | ((d[11] as u32) << 4) | ((d[12] as u32) >> 4);
                    let channels = ((d[12] >> 1) & 7) + 1;
                    let bps = ((d[12] & 1) << 4) | ((d[13] >> 4) + 1);
                    let ts = (((d[13] & 0xF) as u64) << 32)
                        | ((d[14] as u64) << 24)
                        | ((d[15] as u64) << 16)
                        | ((d[16] as u64) << 8)
                        | (d[17] as u64);
                    info = Some(FlacInfo {
                        min_block,
                        max_block,
                        min_frame,
                        max_frame,
                        sample_rate: sr,
                        channels,
                        bits_per_sample: bps,
                        total_samples: ts,
                        file_size: b.len() as u64,
                        vorbis_comments: vec![],
                    });
                }
            }
            4 => {
                // VORBIS_COMMENT
                let d = &b[pos..pos + block_len];
                let mut cp = 0usize;
                if cp + 4 > d.len() {
                    break;
                }
                let vendor_len = ru32le(d, cp) as usize;
                cp += 4;
                if cp + vendor_len > d.len() {
                    break;
                }
                cp += vendor_len;
                if cp + 4 > d.len() {
                    break;
                }
                let num_comments = ru32le(d, cp) as usize;
                cp += 4;
                for _ in 0..num_comments {
                    if cp + 4 > d.len() {
                        break;
                    }
                    let clen = ru32le(d, cp) as usize;
                    cp += 4;
                    if cp + clen > d.len() {
                        break;
                    }
                    let cs = String::from_utf8_lossy(&d[cp..cp + clen]).into_owned();
                    cp += clen;
                    if let Some(eq) = cs.find('=') {
                        let key = cs[..eq].to_uppercase();
                        let val = cs[eq + 1..].to_string();
                        comments.push((key, val));
                    }
                }
            }
            _ => {}
        }
        pos += block_len;
        if is_last {
            break;
        }
    }
    if let Some(mut i) = info {
        i.vorbis_comments = comments;
        Some(i)
    } else {
        None
    }
}

fn dispatch_flac(action: &str, b: &[u8]) -> String {
    let m = match parse_flac(b) {
        Some(m) => m,
        None => return "Error: not a valid FLAC file.".to_string(),
    };
    let duration = if m.sample_rate > 0 {
        m.total_samples as f64 / m.sample_rate as f64
    } else {
        0.0
    };

    match action {
        "info" | "metadata" => {
            let mut out = vec![
                "Format:          FLAC (Free Lossless Audio Codec)".to_string(),
                format!("Channels:        {}", m.channels),
                format!("Sample rate:     {} Hz", m.sample_rate),
                format!("Bit depth:       {}-bit", m.bits_per_sample),
                format!("Total samples:   {}", m.total_samples),
                format!("Duration:        {}", human_dur(duration)),
                format!("Block size:      {}-{} samples", m.min_block, m.max_block),
                format!("Frame size:      {}-{} bytes", m.min_frame, m.max_frame),
                format!("File size:       {}", human_size(m.file_size)),
            ];
            if !m.vorbis_comments.is_empty() {
                out.push("Vorbis comments:".to_string());
                for (k, v) in &m.vorbis_comments {
                    out.push(format!("  {}: {}", k, v));
                }
            }
            out.join("\n")
        }
        "tags" => {
            if m.vorbis_comments.is_empty() {
                return "No Vorbis comments found.".to_string();
            }
            m.vorbis_comments
                .iter()
                .map(|(k, v)| format!("{}: {}", k, v))
                .collect::<Vec<_>>()
                .join("\n")
        }
        "validate" => {
            let mut issues = vec![];
            if m.sample_rate == 0 {
                issues.push("Sample rate is zero".to_string());
            }
            if m.channels == 0 {
                issues.push("Channel count is zero".to_string());
            }
            if m.bits_per_sample == 0 {
                issues.push("Bit depth is zero".to_string());
            }
            if m.total_samples == 0 {
                issues.push("Total samples is zero (or unknown)".to_string());
            }
            if m.vorbis_comments.iter().all(|(k, _)| k != "TITLE") {
                issues.push("Missing TITLE tag".to_string());
            }
            if issues.is_empty() {
                "VALID — FLAC file structure is well-formed.".to_string()
            } else {
                format!(
                    "WARNINGS:\n{}",
                    issues
                        .iter()
                        .map(|s| format!("  • {}", s))
                        .collect::<Vec<_>>()
                        .join("\n")
                )
            }
        }
        _ => dispatch_flac("info", b),
    }
}

// ── OGG / Vorbis ─────────────────────────────────────────────────────────────

struct OggMeta {
    serial: u32,
    page_count: u32,
    has_vorbis: bool,
    has_opus: bool,
    has_theora: bool,
    sample_rate: u32,
    channels: u8,
    bitrate_nominal: i32,
    comments: Vec<(String, String)>,
    file_size: u64,
}

fn parse_ogg(b: &[u8]) -> Option<OggMeta> {
    if b.len() < 27 || &b[0..4] != b"OggS" {
        return None;
    }
    let mut meta = OggMeta {
        serial: ru32le(b, 14),
        page_count: 0,
        has_vorbis: false,
        has_opus: false,
        has_theora: false,
        sample_rate: 0,
        channels: 0,
        bitrate_nominal: 0,
        comments: vec![],
        file_size: b.len() as u64,
    };
    let mut pos = 0usize;
    while pos + 27 <= b.len() {
        if &b[pos..pos + 4] != b"OggS" {
            break;
        }
        meta.page_count += 1;
        let nseg = b[pos + 26] as usize;
        if pos + 27 + nseg > b.len() {
            break;
        }
        let segtab = &b[pos + 27..pos + 27 + nseg];
        let payload_len: usize = segtab.iter().map(|&s| s as usize).sum();
        let page_start = pos + 27 + nseg;
        if page_start + payload_len > b.len() {
            break;
        }
        let payload = &b[page_start..page_start + payload_len];
        // Identify stream type from first bytes of payload
        if payload.len() >= 7 && &payload[1..7] == b"vorbis" {
            meta.has_vorbis = true;
            if payload[0] == 1 && payload.len() >= 30 {
                // Vorbis identification header
                meta.channels = payload[11];
                meta.sample_rate = ru32le(payload, 12);
                meta.bitrate_nominal =
                    i32::from_le_bytes([payload[20], payload[21], payload[22], payload[23]]);
            } else if payload[0] == 3 && payload.len() >= 16 {
                // Vorbis comment header
                parse_vorbis_comment(payload, 7, &mut meta.comments);
            }
        } else if payload.len() >= 8 && &payload[0..8] == b"OpusHead" {
            meta.has_opus = true;
            if payload.len() >= 19 {
                meta.channels = payload[9];
                meta.sample_rate = 48000; // Opus always outputs at 48kHz internally
            }
        } else if payload.len() >= 8 && &payload[0..8] == b"OpusTags" {
            parse_vorbis_comment(payload, 8, &mut meta.comments);
        } else if payload.len() >= 7 && &payload[1..7] == b"theora" {
            meta.has_theora = true;
        }
        pos = page_start + payload_len;
    }
    Some(meta)
}

fn parse_vorbis_comment(data: &[u8], start: usize, out: &mut Vec<(String, String)>) {
    let mut p = start;
    if p + 4 > data.len() {
        return;
    }
    let vlen = ru32le(data, p) as usize;
    p += 4;
    if p + vlen > data.len() {
        return;
    }
    p += vlen;
    if p + 4 > data.len() {
        return;
    }
    let nc = ru32le(data, p) as usize;
    p += 4;
    for _ in 0..nc {
        if p + 4 > data.len() {
            break;
        }
        let cl = ru32le(data, p) as usize;
        p += 4;
        if p + cl > data.len() {
            break;
        }
        let s = String::from_utf8_lossy(&data[p..p + cl]).into_owned();
        p += cl;
        if let Some(eq) = s.find('=') {
            out.push((s[..eq].to_uppercase(), s[eq + 1..].to_string()));
        }
    }
}

fn dispatch_ogg(action: &str, b: &[u8]) -> String {
    let m = match parse_ogg(b) {
        Some(m) => m,
        None => return "Error: not a valid Ogg file.".to_string(),
    };
    let codec = if m.has_opus {
        "Ogg Opus"
    } else if m.has_vorbis {
        "Ogg Vorbis"
    } else if m.has_theora {
        "Ogg Theora"
    } else {
        "Ogg (unknown codec)"
    };

    match action {
        "info" | "metadata" => {
            let mut out = vec![
                format!("Format:          {}", codec),
                format!("Serial number:   0x{:08X}", m.serial),
                format!("OGG pages:       {}", m.page_count),
                format!("File size:       {}", human_size(m.file_size)),
            ];
            if m.channels > 0 {
                out.push(format!("Channels:        {}", m.channels));
            }
            if m.sample_rate > 0 {
                out.push(format!("Sample rate:     {} Hz", m.sample_rate));
            }
            if m.has_vorbis && m.bitrate_nominal > 0 {
                out.push(format!(
                    "Nominal bitrate: {} kbps",
                    m.bitrate_nominal / 1000
                ));
            }
            if !m.comments.is_empty() {
                out.push("Vorbis comments:".to_string());
                for (k, v) in &m.comments {
                    out.push(format!("  {}: {}", k, v));
                }
            }
            out.join("\n")
        }
        "tags" => {
            if m.comments.is_empty() {
                return "No Vorbis comments found.".to_string();
            }
            m.comments
                .iter()
                .map(|(k, v)| format!("{}: {}", k, v))
                .collect::<Vec<_>>()
                .join("\n")
        }
        "validate" => {
            let mut issues = vec![];
            if !m.has_vorbis && !m.has_opus && !m.has_theora {
                issues.push("No recognised codec found in Ogg stream".to_string());
            }
            if m.channels == 0 {
                issues.push("Channel count is zero".to_string());
            }
            if m.has_vorbis && m.sample_rate == 0 {
                issues.push("Sample rate is zero".to_string());
            }
            if m.comments.iter().all(|(k, _)| k != "TITLE") {
                issues.push("Missing TITLE tag".to_string());
            }
            if issues.is_empty() {
                "VALID — Ogg file structure looks well-formed.".to_string()
            } else {
                format!(
                    "WARNINGS:\n{}",
                    issues
                        .iter()
                        .map(|s| format!("  • {}", s))
                        .collect::<Vec<_>>()
                        .join("\n")
                )
            }
        }
        _ => dispatch_ogg("info", b),
    }
}

// ── Format detection ─────────────────────────────────────────────────────────

enum AudioFormat {
    Wav,
    Mp3,
    Flac,
    Ogg,
    Unknown,
}

fn detect_format(b: &[u8]) -> AudioFormat {
    if b.len() < 4 {
        return AudioFormat::Unknown;
    }
    if b.len() >= 12 && &b[0..4] == b"RIFF" && &b[8..12] == b"WAVE" {
        return AudioFormat::Wav;
    }
    if &b[0..3] == b"ID3" {
        return AudioFormat::Mp3;
    }
    if b.len() >= 4 && &b[0..4] == b"fLaC" {
        return AudioFormat::Flac;
    }
    if &b[0..4] == b"OggS" {
        return AudioFormat::Ogg;
    }
    // raw MPEG sync
    if b.len() >= 2 && b[0] == 0xFF && (b[1] & 0xE0) == 0xE0 {
        return AudioFormat::Mp3;
    }
    AudioFormat::Unknown
}

// ── Entry point ───────────────────────────────────────────────────────────────

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("info");
    let bytes = match get_bytes(args) {
        Some(b) => b,
        None => {
            return Ok(
                "Error: provide 'file' (path to audio file) or 'hex' (hex-encoded bytes). \
             Supported formats: WAV, MP3 (with ID3v1/v2), FLAC, Ogg Vorbis/Opus. \
             Actions: info (default), tags, validate."
                    .to_string(),
            )
        }
    };
    Ok(match detect_format(&bytes) {
        AudioFormat::Wav => dispatch_wav(action, &bytes),
        AudioFormat::Mp3 => dispatch_mp3(action, &bytes),
        AudioFormat::Flac => dispatch_flac(action, &bytes),
        AudioFormat::Ogg => dispatch_ogg(action, &bytes),
        AudioFormat::Unknown => format!(
            "Error: unrecognised audio format. \
             Supported: WAV (RIFF/WAVE), MP3 (ID3 header or raw sync), \
             FLAC (fLaC magic), Ogg Vorbis/Opus (OggS pages). \
             First 8 bytes: {}",
            bytes
                .iter()
                .take(8)
                .map(|b| format!("{:02X}", b))
                .collect::<Vec<_>>()
                .join(" ")
        ),
    })
}
