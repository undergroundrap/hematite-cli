use serde_json::{json, Value};
use std::fs;

pub fn make_schema() -> Value {
    json!({
        "name": "video_file_tools",
        "description": "Parse video container metadata (MP4/MOV, MKV/WebM, AVI) without external utilities. \
    Actions: info (default — format, duration, video/audio stream summary, creation date, file size), \
    streams (detailed per-stream breakdown: codec, resolution, frame rate, channels, sample rate), \
    metadata (container-level tags: title, encoder, creation date, compatible brands), \
    validate (structural checks: required headers/boxes, stream presence). \
    Pass file (path to video file) or hex (hex-encoded bytes). \
    Example: video_file_tools(file: 'clip.mp4') or video_file_tools(action: 'streams', file: 'video.mkv')",
        "input_schema": {
            "type": "object",
            "properties": {
                "action": { "type": "string", "description": "info|streams|metadata|validate" },
                "file": { "type": "string", "description": "Path to MP4, MOV, MKV, WebM, or AVI file" },
                "hex": { "type": "string", "description": "Hex-encoded video container bytes" }
            },
            "required": []
        }
    })
}

fn get_bytes(args: &Value) -> Option<Vec<u8>> {
    if let Some(p) = args.get("file").and_then(|v| v.as_str()) {
        fs::read(p).ok()
    } else if let Some(h) = args.get("hex").and_then(|v| v.as_str()) {
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
fn ru64be(b: &[u8], o: usize) -> u64 {
    if o + 7 >= b.len() {
        return 0;
    }
    u64::from_be_bytes([
        b[o],
        b[o + 1],
        b[o + 2],
        b[o + 3],
        b[o + 4],
        b[o + 5],
        b[o + 6],
        b[o + 7],
    ])
}

fn tag4(b: &[u8], o: usize) -> [u8; 4] {
    if o + 3 >= b.len() {
        return [0u8; 4];
    }
    [b[o], b[o + 1], b[o + 2], b[o + 3]]
}

fn fourcc(b: [u8; 4]) -> String {
    b.iter()
        .map(|&c| {
            if c.is_ascii_graphic() || c == b' ' {
                c as char
            } else {
                '?'
            }
        })
        .collect()
}

fn human_size(n: u64) -> String {
    if n < 1_048_576 {
        format!("{:.1} KB", n as f64 / 1024.0)
    } else if n < 1_073_741_824 {
        format!("{:.2} MB", n as f64 / 1_048_576.0)
    } else {
        format!("{:.3} GB", n as f64 / 1_073_741_824.0)
    }
}

fn human_dur(secs: f64) -> String {
    if secs < 60.0 {
        format!("{:.1}s", secs)
    } else if secs < 3600.0 {
        format!("{}m {:.0}s", secs as u64 / 60, secs as u64 % 60)
    } else {
        format!("{}h {}m", secs as u64 / 3600, (secs as u64 % 3600) / 60)
    }
}

// Mac timestamp epoch = Jan 1 1904 → offset to Unix epoch Jan 1 1970 = 66 years
const MAC_EPOCH_OFFSET: u64 = 2_082_844_800;

fn mac_ts_to_str(ts: u64) -> String {
    if ts == 0 {
        return "unknown".to_string();
    }
    let unix = ts.saturating_sub(MAC_EPOCH_OFFSET);
    let days = unix / 86400;
    let year = 1970 + days / 365;
    let month = (days % 365) / 30 + 1;
    let day = (days % 365) % 30 + 1;
    format!("{}-{:02}-{:02}", year, month.min(12), day.min(31))
}

fn codec_name(tag: &str) -> &'static str {
    match tag {
        "avc1" | "avc2" | "avc3" | "avc4" | "h264" => "H.264/AVC",
        "hev1" | "hvc1" | "h265" => "H.265/HEVC",
        "av01" => "AV1",
        "vp08" => "VP8",
        "vp09" => "VP9",
        "mp4v" | "m4v " => "MPEG-4 Visual",
        "s263" | "h263" => "H.263",
        "mp4a" => "AAC",
        "ac-3" | "ec-3" => "Dolby AC-3",
        "Opus" | "Opuz" => "Opus",
        "FLAC" | "fLaC" => "FLAC",
        "sowt" | "twos" | "lpcm" | "ipcm" => "PCM",
        "mp3 " | ".mp3" => "MP3",
        "tmcd" => "Timecode",
        "text" | "tx3g" => "Subtitles",
        "jpeg" | "JPEG" => "JPEG",
        _ => "Unknown",
    }
}

// ── MP4 / MOV ─────────────────────────────────────────────────────────────────

#[derive(Default)]
struct Mp4VideoTrack {
    codec: String,
    width: u32,
    height: u32,
    timescale: u32,
    duration: u64,
}
#[derive(Default)]
struct Mp4AudioTrack {
    codec: String,
    channels: u16,
    sample_rate: u32,
    timescale: u32,
    duration: u64,
}

#[derive(Default)]
struct Mp4Meta {
    brand: String,
    compat_brands: Vec<String>,
    timescale: u32,
    duration: u64,
    creation: u64,
    modification: u64,
    video_tracks: Vec<Mp4VideoTrack>,
    audio_tracks: Vec<Mp4AudioTrack>,
    file_size: u64,
}

fn walk_boxes(b: &[u8], mut pos: usize, end: usize, depth: u32, meta: &mut Mp4Meta, ctx: &str) {
    if depth > 8 {
        return;
    }
    while pos + 8 <= end && pos + 8 <= b.len() {
        let raw_size = ru32be(b, pos);
        let tag = tag4(b, pos + 4);
        let tag_str: String = fourcc(tag);
        let (box_size, data_start) = if raw_size == 1 {
            if pos + 16 > b.len() {
                break;
            }
            (ru64be(b, pos + 8), pos + 16)
        } else if raw_size == 0 {
            (end as u64 - pos as u64, pos + 8)
        } else {
            (raw_size as u64, pos + 8)
        };
        if box_size < 8 {
            break;
        }
        let box_end = (pos as u64 + box_size).min(b.len() as u64) as usize;

        match tag_str.as_str() {
            "ftyp" => {
                if data_start + 4 <= box_end {
                    meta.brand = fourcc(tag4(b, data_start));
                    let mut bp = data_start + 8;
                    while bp + 4 <= box_end {
                        let cb = fourcc(tag4(b, bp));
                        if !cb.trim().is_empty() && cb != meta.brand {
                            meta.compat_brands.push(cb);
                        }
                        bp += 4;
                    }
                }
            }
            "moov" | "trak" | "mdia" | "minf" | "stbl" => {
                walk_boxes(b, data_start, box_end, depth + 1, meta, &tag_str);
            }
            "mvhd" => {
                let ver = if data_start < b.len() {
                    b[data_start]
                } else {
                    0
                };
                if ver == 1 {
                    meta.creation = ru64be(b, data_start + 4);
                    meta.modification = ru64be(b, data_start + 12);
                    meta.timescale = ru32be(b, data_start + 20);
                    meta.duration = ru64be(b, data_start + 24);
                } else {
                    meta.creation = ru32be(b, data_start + 4) as u64;
                    meta.modification = ru32be(b, data_start + 8) as u64;
                    meta.timescale = ru32be(b, data_start + 12);
                    meta.duration = ru32be(b, data_start + 16) as u64;
                }
            }
            "mdhd" => {
                let ver = if data_start < b.len() {
                    b[data_start]
                } else {
                    0
                };
                let (ts, dur) = if ver == 1 {
                    (ru32be(b, data_start + 20), ru64be(b, data_start + 24))
                } else {
                    (
                        ru32be(b, data_start + 12),
                        ru32be(b, data_start + 16) as u64,
                    )
                };
                if ctx == "mdia" || depth > 2 {
                    if let Some(vt) = meta.video_tracks.last_mut() {
                        if vt.timescale == 0 {
                            vt.timescale = ts;
                            vt.duration = dur;
                        }
                    }
                    if let Some(at) = meta.audio_tracks.last_mut() {
                        if at.timescale == 0 {
                            at.timescale = ts;
                            at.duration = dur;
                        }
                    }
                }
            }
            "hdlr" => {
                if data_start + 12 <= box_end {
                    let handler = fourcc(tag4(b, data_start + 8));
                    match handler.as_str() {
                        "vide" => meta.video_tracks.push(Mp4VideoTrack::default()),
                        "soun" => meta.audio_tracks.push(Mp4AudioTrack::default()),
                        _ => {}
                    }
                }
            }
            "stsd" => {
                if data_start + 8 <= box_end {
                    let entry_start = data_start + 8;
                    if entry_start + 8 <= box_end {
                        let codec_tag = fourcc(tag4(b, entry_start + 4));
                        if let Some(vt) = meta.video_tracks.last_mut() {
                            if vt.codec.is_empty() {
                                vt.codec =
                                    format!("{} ({})", codec_name(&codec_tag), codec_tag.trim());
                                if entry_start + 32 <= box_end {
                                    vt.width = ru16be(b, entry_start + 28) as u32;
                                    vt.height = ru16be(b, entry_start + 30) as u32;
                                }
                            }
                        } else if let Some(at) = meta.audio_tracks.last_mut() {
                            if at.codec.is_empty() {
                                at.codec =
                                    format!("{} ({})", codec_name(&codec_tag), codec_tag.trim());
                                if entry_start + 26 <= box_end {
                                    at.channels = ru16be(b, entry_start + 24);
                                }
                                if entry_start + 32 <= box_end {
                                    at.sample_rate = ru32be(b, entry_start + 28) >> 16;
                                }
                            }
                        }
                    }
                }
            }
            _ => {}
        }
        pos = box_end;
    }
}

fn parse_mp4(b: &[u8]) -> Option<Mp4Meta> {
    if b.len() < 12 {
        return None;
    }
    let mut meta = Mp4Meta {
        file_size: b.len() as u64,
        ..Default::default()
    };
    walk_boxes(b, 0, b.len(), 0, &mut meta, "root");
    if meta.brand.is_empty() {
        return None;
    }
    Some(meta)
}

fn dispatch_mp4(action: &str, b: &[u8]) -> String {
    let m = match parse_mp4(b) {
        Some(m) => m,
        None => return "Error: not a valid MP4/MOV file (no ftyp box found).".to_string(),
    };
    let dur_secs = if m.timescale > 0 {
        m.duration as f64 / m.timescale as f64
    } else {
        0.0
    };
    let fmt = if m.brand.trim().starts_with("qt") || m.brand.trim() == "moov" {
        "QuickTime MOV"
    } else {
        "MP4 / ISOBMFF"
    };
    match action {
        "info" => {
            let mut out = vec![
                format!("Format:      {}", fmt),
                format!("Brand:       {}", m.brand.trim()),
                format!("Duration:    {}", human_dur(dur_secs)),
                format!("Created:     {}", mac_ts_to_str(m.creation)),
                format!("Video:       {} track(s)", m.video_tracks.len()),
                format!("Audio:       {} track(s)", m.audio_tracks.len()),
                format!("File size:   {}", human_size(m.file_size)),
            ];
            for (i, vt) in m.video_tracks.iter().enumerate() {
                let td = if vt.timescale > 0 {
                    vt.duration as f64 / vt.timescale as f64
                } else {
                    dur_secs
                };
                out.push(format!(
                    "  Video[{}]:  {}  {}×{}  {}",
                    i,
                    vt.codec,
                    vt.width,
                    vt.height,
                    human_dur(td)
                ));
            }
            for (i, at) in m.audio_tracks.iter().enumerate() {
                let td = if at.timescale > 0 {
                    at.duration as f64 / at.timescale as f64
                } else {
                    dur_secs
                };
                out.push(format!(
                    "  Audio[{}]:  {}  {} ch  {} Hz  {}",
                    i,
                    at.codec,
                    at.channels,
                    at.sample_rate,
                    human_dur(td)
                ));
            }
            out.join("\n")
        }
        "streams" => {
            let mut out = vec![];
            for (i, vt) in m.video_tracks.iter().enumerate() {
                let td = if vt.timescale > 0 {
                    vt.duration as f64 / vt.timescale as f64
                } else {
                    dur_secs
                };
                out.push(format!("Video stream {}:", i));
                out.push(format!("  Codec:       {}", vt.codec));
                out.push(format!("  Resolution:  {}×{}", vt.width, vt.height));
                out.push(format!("  Duration:    {}", human_dur(td)));
            }
            for (i, at) in m.audio_tracks.iter().enumerate() {
                let td = if at.timescale > 0 {
                    at.duration as f64 / at.timescale as f64
                } else {
                    dur_secs
                };
                out.push(format!("Audio stream {}:", i));
                out.push(format!("  Codec:       {}", at.codec));
                out.push(format!("  Channels:    {}", at.channels));
                out.push(format!("  Sample rate: {} Hz", at.sample_rate));
                out.push(format!("  Duration:    {}", human_dur(td)));
            }
            if out.is_empty() {
                "No streams found.".to_string()
            } else {
                out.join("\n")
            }
        }
        "metadata" => {
            let mut out = vec![
                format!("Major brand:  {}", m.brand.trim()),
                format!("File type:    {}", fmt),
                format!("Timescale:    {} ticks/sec", m.timescale),
            ];
            if !m.compat_brands.is_empty() {
                out.push(format!(
                    "Compat brands: {}",
                    m.compat_brands
                        .iter()
                        .map(|s| s.trim().to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
            }
            out.push(format!("Created:      {}", mac_ts_to_str(m.creation)));
            out.push(format!("Modified:     {}", mac_ts_to_str(m.modification)));
            out.join("\n")
        }
        "validate" => {
            let mut issues = vec![];
            if m.brand.is_empty() {
                issues.push("Missing ftyp box".to_string());
            }
            if m.timescale == 0 {
                issues.push("Missing mvhd (no timescale)".to_string());
            }
            if m.video_tracks.is_empty() && m.audio_tracks.is_empty() {
                issues.push("No tracks found".to_string());
            }
            if issues.is_empty() {
                "VALID — MP4/MOV structure looks well-formed.".to_string()
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
        _ => dispatch_mp4("info", b),
    }
}

// ── MKV / WebM ────────────────────────────────────────────────────────────────

fn ebml_id_size(b: &[u8], pos: usize) -> Option<(u64, usize)> {
    if pos >= b.len() {
        return None;
    }
    let first = b[pos];
    let (len, mask) = if first & 0x80 != 0 {
        (1, 0x7Fu64)
    } else if first & 0x40 != 0 {
        (2, 0x3Fu64)
    } else if first & 0x20 != 0 {
        (3, 0x1Fu64)
    } else if first & 0x10 != 0 {
        (4, 0x0Fu64)
    } else {
        return None;
    };
    if pos + len > b.len() {
        return None;
    }
    let mut val = (first as u64) & mask;
    for i in 1..len {
        val = (val << 8) | (b[pos + i] as u64);
    }
    Some((val, len))
}

fn ebml_data_size(b: &[u8], pos: usize) -> Option<(u64, usize)> {
    if pos >= b.len() {
        return None;
    }
    let first = b[pos];
    let (len, mask) = if first & 0x80 != 0 {
        (1, 0x7Fu64)
    } else if first & 0x40 != 0 {
        (2, 0x3Fu64)
    } else if first & 0x20 != 0 {
        (3, 0x1Fu64)
    } else if first & 0x10 != 0 {
        (4, 0x0Fu64)
    } else if first & 0x08 != 0 {
        (5, 0x07u64)
    } else if first & 0x04 != 0 {
        (6, 0x03u64)
    } else if first & 0x02 != 0 {
        (7, 0x01u64)
    } else if first & 0x01 != 0 {
        (8, 0x00u64)
    } else {
        return None;
    };
    if pos + len > b.len() {
        return None;
    }
    let mut val = (first as u64) & mask;
    for i in 1..len {
        val = (val << 8) | (b[pos + i] as u64);
    }
    Some((val, len))
}

fn ebml_uint(b: &[u8], pos: usize, len: usize) -> u64 {
    let mut val = 0u64;
    for i in 0..len.min(8) {
        if pos + i < b.len() {
            val = (val << 8) | (b[pos + i] as u64);
        }
    }
    val
}

fn ebml_float(b: &[u8], pos: usize, len: usize) -> f64 {
    if len == 4 && pos + 4 <= b.len() {
        f32::from_be_bytes([b[pos], b[pos + 1], b[pos + 2], b[pos + 3]]) as f64
    } else if len == 8 && pos + 8 <= b.len() {
        f64::from_be_bytes([
            b[pos],
            b[pos + 1],
            b[pos + 2],
            b[pos + 3],
            b[pos + 4],
            b[pos + 5],
            b[pos + 6],
            b[pos + 7],
        ])
    } else {
        0.0
    }
}

fn ebml_str(b: &[u8], pos: usize, len: usize) -> String {
    let end = (pos + len).min(b.len());
    String::from_utf8_lossy(&b[pos..end])
        .trim_end_matches('\0')
        .to_string()
}

#[derive(Default)]
struct MkvVideoTrack {
    codec: String,
    width: u64,
    height: u64,
}
#[derive(Default)]
struct MkvAudioTrack {
    codec: String,
    channels: u64,
    sample_rate: f64,
    bit_depth: u64,
}

#[derive(Default)]
struct MkvMeta {
    is_webm: bool,
    duration_ms: f64,
    timecode_scale: u64,
    title: String,
    muxing_app: String,
    writing_app: String,
    video_tracks: Vec<MkvVideoTrack>,
    audio_tracks: Vec<MkvAudioTrack>,
    file_size: u64,
}

fn scan_ebml(
    b: &[u8],
    start: usize,
    end: usize,
    depth: u32,
    meta: &mut MkvMeta,
    in_track: &mut Option<(u64, String)>,
) {
    if depth > 10 {
        return;
    }
    let mut pos = start;
    while pos < end && pos < b.len() {
        let (id, id_len) = match ebml_id_size(b, pos) {
            Some(v) => v,
            None => break,
        };
        let size_pos = pos + id_len;
        let (data_size, size_len) = match ebml_data_size(b, size_pos) {
            Some(v) => v,
            None => break,
        };
        let data_pos = size_pos + size_len;
        let data_end = (data_pos + data_size as usize).min(b.len()).min(end);

        match id {
            // Info
            0x1549A966 => scan_ebml(b, data_pos, data_end, depth + 1, meta, in_track),
            // TimecodeScale
            0x2AD7B1 => {
                meta.timecode_scale = ebml_uint(b, data_pos, data_size as usize);
            }
            // Duration
            0x4489 => {
                meta.duration_ms = ebml_float(b, data_pos, data_size as usize);
            }
            // Title
            0x7BA9 => {
                meta.title = ebml_str(b, data_pos, data_size as usize);
            }
            // MuxingApp
            0x4D80 => {
                meta.muxing_app = ebml_str(b, data_pos, data_size as usize);
            }
            // WritingApp
            0x5741 => {
                meta.writing_app = ebml_str(b, data_pos, data_size as usize);
            }
            // Tracks container
            0x1654AE6B => scan_ebml(b, data_pos, data_end, depth + 1, meta, in_track),
            // TrackEntry
            0xAE => {
                let mut track_info: Option<(u64, String)> = None;
                scan_ebml(b, data_pos, data_end, depth + 1, meta, &mut track_info);
                if let Some((track_type, codec)) = track_info {
                    match track_type {
                        1 => meta.video_tracks.push(MkvVideoTrack {
                            codec,
                            ..Default::default()
                        }),
                        2 => meta.audio_tracks.push(MkvAudioTrack {
                            codec,
                            ..Default::default()
                        }),
                        _ => {}
                    }
                }
            }
            // TrackType
            0x83 => {
                let t = ebml_uint(b, data_pos, data_size as usize);
                if let Some((ref mut tt, _)) = in_track {
                    *tt = t;
                } else {
                    *in_track = Some((t, String::new()));
                }
            }
            // CodecID
            0x86 => {
                let codec = ebml_str(b, data_pos, data_size as usize);
                if let Some((_, ref mut c)) = in_track {
                    *c = codec;
                } else {
                    *in_track = Some((0, codec));
                }
            }
            // Video container
            0xE0 => {
                if let Some(ref mut vt) = meta.video_tracks.last_mut() {
                    let mut p2 = data_pos;
                    while p2 < data_end {
                        let (id2, il2) = match ebml_id_size(b, p2) {
                            Some(v) => v,
                            None => break,
                        };
                        let (ds2, sl2) = match ebml_data_size(b, p2 + il2) {
                            Some(v) => v,
                            None => break,
                        };
                        let dp2 = p2 + il2 + sl2;
                        match id2 {
                            0xB0 => {
                                vt.width = ebml_uint(b, dp2, ds2 as usize);
                            }
                            0xBA => {
                                vt.height = ebml_uint(b, dp2, ds2 as usize);
                            }
                            _ => {}
                        }
                        p2 = dp2 + ds2 as usize;
                    }
                }
            }
            // Audio container
            0xE1 => {
                if let Some(ref mut at) = meta.audio_tracks.last_mut() {
                    let mut p2 = data_pos;
                    while p2 < data_end {
                        let (id2, il2) = match ebml_id_size(b, p2) {
                            Some(v) => v,
                            None => break,
                        };
                        let (ds2, sl2) = match ebml_data_size(b, p2 + il2) {
                            Some(v) => v,
                            None => break,
                        };
                        let dp2 = p2 + il2 + sl2;
                        match id2 {
                            0xB5 => {
                                at.sample_rate = ebml_float(b, dp2, ds2 as usize);
                            }
                            0x9F => {
                                at.channels = ebml_uint(b, dp2, ds2 as usize);
                            }
                            0x6264 => {
                                at.bit_depth = ebml_uint(b, dp2, ds2 as usize);
                            }
                            _ => {}
                        }
                        p2 = dp2 + ds2 as usize;
                    }
                }
            }
            _ => {}
        }
        pos = data_end;
    }
}

fn mkv_codec_name(id: &str) -> String {
    let base = id
        .trim_start_matches("V_")
        .trim_start_matches("A_")
        .trim_start_matches("S_");
    match id {
        "V_MPEG4/ISO/AVC" => "H.264/AVC".into(),
        "V_MPEGH/ISO/HEVC" => "H.265/HEVC".into(),
        "V_AV1" => "AV1".into(),
        "V_VP8" => "VP8".into(),
        "V_VP9" => "VP9".into(),
        "A_AAC" | "A_AAC/MPEG4/LC" | "A_AAC/MPEG2/LC" => "AAC".into(),
        "A_AC3" => "Dolby AC-3".into(),
        "A_EAC3" => "Dolby E-AC-3".into(),
        "A_FLAC" => "FLAC".into(),
        "A_OPUS" => "Opus".into(),
        "A_VORBIS" => "Vorbis".into(),
        "A_PCM/INT/LIT" | "A_PCM/INT/BIG" => "PCM".into(),
        _ => base.to_string(),
    }
}

fn parse_mkv(b: &[u8]) -> Option<MkvMeta> {
    if b.len() < 4 || &b[0..4] != b"\x1A\x45\xDF\xA3" {
        return None;
    }
    let is_webm = b.windows(6).any(|w| w == b"webm\x00" || w == b"WebM\x00");
    let mut meta = MkvMeta {
        file_size: b.len() as u64,
        is_webm,
        timecode_scale: 1_000_000,
        ..Default::default()
    };
    let mut track_info: Option<(u64, String)> = None;
    scan_ebml(b, 0, b.len().min(8_000_000), 0, &mut meta, &mut track_info);
    // Update codec names
    for vt in &mut meta.video_tracks {
        vt.codec = mkv_codec_name(&vt.codec);
    }
    for at in &mut meta.audio_tracks {
        at.codec = mkv_codec_name(&at.codec);
    }
    Some(meta)
}

fn dispatch_mkv(action: &str, b: &[u8]) -> String {
    let m = match parse_mkv(b) {
        Some(m) => m,
        None => return "Error: not a valid MKV/WebM file.".to_string(),
    };
    let fmt = if m.is_webm { "WebM" } else { "Matroska (MKV)" };
    let dur_secs = if m.timecode_scale > 0 {
        m.duration_ms * m.timecode_scale as f64 / 1_000_000_000.0
    } else {
        m.duration_ms / 1000.0
    };
    match action {
        "info" => {
            let mut out = vec![
                format!("Format:    {}", fmt),
                format!("Duration:  {}", human_dur(dur_secs)),
                format!("Video:     {} track(s)", m.video_tracks.len()),
                format!("Audio:     {} track(s)", m.audio_tracks.len()),
                format!("File size: {}", human_size(m.file_size)),
            ];
            if !m.title.is_empty() {
                out.push(format!("Title:     {}", m.title));
            }
            if !m.writing_app.is_empty() {
                out.push(format!("Written by: {}", m.writing_app));
            }
            for (i, vt) in m.video_tracks.iter().enumerate() {
                out.push(format!(
                    "  Video[{}]: {}  {}×{}",
                    i, vt.codec, vt.width, vt.height
                ));
            }
            for (i, at) in m.audio_tracks.iter().enumerate() {
                out.push(format!(
                    "  Audio[{}]: {}  {} ch  {:.0} Hz",
                    i, at.codec, at.channels, at.sample_rate
                ));
            }
            out.join("\n")
        }
        "streams" => {
            let mut out = vec![];
            for (i, vt) in m.video_tracks.iter().enumerate() {
                out.push(format!(
                    "Video stream {}:\n  Codec: {}\n  Resolution: {}×{}",
                    i, vt.codec, vt.width, vt.height
                ));
            }
            for (i, at) in m.audio_tracks.iter().enumerate() {
                let bd = if at.bit_depth > 0 {
                    format!("  Bit depth: {}", at.bit_depth)
                } else {
                    String::new()
                };
                out.push(format!(
                    "Audio stream {}:\n  Codec: {}\n  Channels: {}\n  Sample rate: {:.0} Hz{}",
                    i, at.codec, at.channels, at.sample_rate, bd
                ));
            }
            if out.is_empty() {
                "No streams found.".to_string()
            } else {
                out.join("\n\n")
            }
        }
        "metadata" => {
            let mut out = vec![format!("Format:     {}", fmt)];
            if !m.title.is_empty() {
                out.push(format!("Title:      {}", m.title));
            }
            if !m.muxing_app.is_empty() {
                out.push(format!("Muxing app: {}", m.muxing_app));
            }
            if !m.writing_app.is_empty() {
                out.push(format!("Writing app: {}", m.writing_app));
            }
            out.push(format!("Timecode scale: {} ns/tick", m.timecode_scale));
            out.join("\n")
        }
        "validate" => {
            let mut issues = vec![];
            if m.video_tracks.is_empty() && m.audio_tracks.is_empty() {
                issues.push("No tracks found".to_string());
            }
            if m.timecode_scale == 0 {
                issues.push("Invalid TimecodeScale (0)".to_string());
            }
            if issues.is_empty() {
                format!("VALID — {} structure looks well-formed.", fmt)
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
        _ => dispatch_mkv("info", b),
    }
}

// ── AVI ───────────────────────────────────────────────────────────────────────

#[derive(Default)]
struct AviMeta {
    width: u32,
    height: u32,
    total_frames: u32,
    us_per_frame: u32,
    video_codec: String,
    audio_codec: String,
    audio_channels: u16,
    audio_sample_rate: u32,
    stream_count: u32,
    file_size: u64,
}

fn parse_avi(b: &[u8]) -> Option<AviMeta> {
    if b.len() < 12 || &b[0..4] != b"RIFF" || &b[8..12] != b"AVI " {
        return None;
    }
    let mut meta = AviMeta {
        file_size: b.len() as u64,
        ..Default::default()
    };
    let mut pos = 12usize;
    while pos + 12 <= b.len() {
        let chunk_tag = &b[pos..pos + 4];
        let chunk_size = ru32le(b, pos + 4) as usize;
        if chunk_tag == b"LIST" {
            let list_type = &b[pos + 8..pos + 12];
            if list_type == b"hdrl" {
                let mut p = pos + 12;
                while p + 8 <= pos + 8 + chunk_size && p + 8 <= b.len() {
                    let sub_tag = &b[p..p + 4];
                    let sub_size = ru32le(b, p + 4) as usize;
                    if sub_tag == b"avih" && p + 8 + 56 <= b.len() {
                        meta.us_per_frame = ru32le(b, p + 8);
                        meta.total_frames = ru32le(b, p + 8 + 16);
                        meta.stream_count = ru32le(b, p + 8 + 24);
                        meta.width = ru32le(b, p + 8 + 32);
                        meta.height = ru32le(b, p + 8 + 36);
                    } else if sub_tag == b"LIST"
                        && p + 12 <= b.len()
                        && &b[p + 8..p + 12] == b"strl"
                    {
                        let mut q = p + 12;
                        let strl_end = p + 8 + sub_size;
                        while q + 8 <= strl_end && q + 8 <= b.len() {
                            let st_tag = &b[q..q + 4];
                            let st_size = ru32le(b, q + 4) as usize;
                            if st_tag == b"strh" && q + 8 + 56 <= b.len() {
                                let fcc_type = &b[q + 8..q + 12];
                                let handler = fourcc(tag4(b, q + 12));
                                if fcc_type == b"vids" {
                                    meta.video_codec = if handler.trim().is_empty() {
                                        "Unknown".into()
                                    } else {
                                        handler
                                    };
                                } else if fcc_type == b"auds" {
                                    // audio: rate/scale
                                    let scale = ru32le(b, q + 8 + 20);
                                    let rate = ru32le(b, q + 8 + 24);
                                    if scale > 0 {
                                        meta.audio_sample_rate = rate / scale;
                                    }
                                }
                            } else if st_tag == b"strf" && q + 8 <= b.len() {
                                // Could be BITMAPINFOHEADER (video) or WAVEFORMATEX (audio)
                                if q + 8 + 18 <= b.len() {
                                    let fmt_tag = ru16le(b, q + 8);
                                    if fmt_tag != 0 && fmt_tag < 0x1000 {
                                        // Likely WAVEFORMATEX
                                        meta.audio_channels = ru16le(b, q + 8 + 2);
                                        if meta.audio_sample_rate == 0 {
                                            meta.audio_sample_rate = ru32le(b, q + 8 + 4);
                                        }
                                        meta.audio_codec = match fmt_tag {
                                            1 => "PCM".into(),
                                            2 => "ADPCM".into(),
                                            3 => "IEEE Float".into(),
                                            6 => "G.711 A-law".into(),
                                            7 => "G.711 µ-law".into(),
                                            0x55 => "MP3".into(),
                                            0x0161 => "WMA".into(),
                                            _ => format!("0x{:04X}", fmt_tag),
                                        };
                                    }
                                }
                            }
                            q += 8 + st_size + (st_size & 1);
                        }
                    }
                    p += 8 + sub_size + (sub_size & 1);
                }
                break;
            }
        }
        pos += 8 + chunk_size + (chunk_size & 1);
    }
    if meta.width == 0 {
        return None;
    }
    Some(meta)
}

fn dispatch_avi(action: &str, b: &[u8]) -> String {
    let m = match parse_avi(b) {
        Some(m) => m,
        None => return "Error: not a valid AVI file.".to_string(),
    };
    let fps = if m.us_per_frame > 0 {
        1_000_000.0 / m.us_per_frame as f64
    } else {
        0.0
    };
    let dur_secs = if fps > 0.0 {
        m.total_frames as f64 / fps
    } else {
        0.0
    };
    match action {
        "info" | "metadata" => {
            let mut out = vec![
                "Format:        AVI (RIFF)".to_string(),
                format!("Dimensions:    {}×{}", m.width, m.height),
                format!("Frame rate:    {:.2} fps", fps),
                format!("Total frames:  {}", m.total_frames),
                format!("Duration:      {}", human_dur(dur_secs)),
                format!("Streams:       {}", m.stream_count),
                format!("File size:     {}", human_size(m.file_size)),
            ];
            if !m.video_codec.is_empty() {
                out.push(format!("Video codec:   {}", m.video_codec));
            }
            if !m.audio_codec.is_empty() {
                out.push(format!("Audio codec:   {}", m.audio_codec));
                out.push(format!(
                    "Audio:         {} ch  {} Hz",
                    m.audio_channels, m.audio_sample_rate
                ));
            }
            out.join("\n")
        }
        "streams" => {
            let mut out = vec![];
            if !m.video_codec.is_empty() {
                out.push(format!("Video stream:\n  Codec: {}\n  Resolution: {}×{}\n  Frame rate: {:.2} fps\n  Frames: {}",
                    m.video_codec, m.width, m.height, fps, m.total_frames));
            }
            if !m.audio_codec.is_empty() {
                out.push(format!(
                    "Audio stream:\n  Codec: {}\n  Channels: {}\n  Sample rate: {} Hz",
                    m.audio_codec, m.audio_channels, m.audio_sample_rate
                ));
            }
            if out.is_empty() {
                "No streams found.".to_string()
            } else {
                out.join("\n\n")
            }
        }
        "validate" => {
            let mut issues = vec![];
            if m.width == 0 || m.height == 0 {
                issues.push("Invalid dimensions".to_string());
            }
            if m.us_per_frame == 0 {
                issues.push("Invalid frame rate (0 µs/frame)".to_string());
            }
            if m.stream_count == 0 {
                issues.push("No streams declared".to_string());
            }
            if issues.is_empty() {
                "VALID — AVI structure looks well-formed.".to_string()
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
        _ => dispatch_avi("info", b),
    }
}

// ── Format detection ──────────────────────────────────────────────────────────

enum VidFormat {
    Mp4,
    Mkv,
    Avi,
    Unknown,
}

fn detect_format(b: &[u8]) -> VidFormat {
    if b.len() < 12 {
        return VidFormat::Unknown;
    }
    if &b[0..4] == b"\x1A\x45\xDF\xA3" {
        return VidFormat::Mkv;
    }
    if &b[0..4] == b"RIFF" && &b[8..12] == b"AVI " {
        return VidFormat::Avi;
    }
    // MP4/MOV: ftyp box often at offset 0 or 4, or 'moov' box
    let mut pos = 0usize;
    while pos + 8 <= b.len().min(1024) {
        let tag = &b[pos + 4..pos + 8];
        if tag == b"ftyp" || tag == b"moov" || tag == b"mdat" {
            return VidFormat::Mp4;
        }
        let sz = ru32be(b, pos) as usize;
        if sz < 8 {
            break;
        }
        pos += sz;
    }
    VidFormat::Unknown
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
                "Error: provide 'file' (path to video file) or 'hex' (hex-encoded bytes). \
             Supported formats: MP4/MOV (ISO Base Media), MKV/WebM (Matroska/EBML), AVI (RIFF). \
             Actions: info (default), streams, metadata, validate."
                    .to_string(),
            )
        }
    };
    Ok(match detect_format(&bytes) {
        VidFormat::Mp4 => dispatch_mp4(action, &bytes),
        VidFormat::Mkv => dispatch_mkv(action, &bytes),
        VidFormat::Avi => dispatch_avi(action, &bytes),
        VidFormat::Unknown => format!(
            "Error: unrecognised video container format. \
             Supported: MP4/MOV (ftyp/moov boxes), MKV/WebM (EBML 0x1A45DFA3), AVI (RIFF/AVI). \
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
