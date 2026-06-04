use serde_json::{json, Value};
use std::fmt::Write as FmtWrite;

pub fn make_schema() -> Value {
    json!({
        "name": "exif_tools",
        "description": "Parse EXIF/IPTC metadata from JPEG and TIFF images without external tools. \
    Actions: info (default — all EXIF fields), camera (make/model/lens/exposure), gps (coordinates + Google Maps link), thumbnail (detect embedded thumbnail). \
    Pass file (JPEG or TIFF path) or hex (hex-encoded bytes). \
    Example: exif_tools(file: 'photo.jpg') or exif_tools(action: 'gps', file: 'photo.jpg') or exif_tools(action: 'camera', hex: 'FFD8...').",
        "input_schema": {
            "type": "object",
            "properties": {
                "action": { "type": "string", "description": "info|camera|gps|thumbnail" },
                "file": { "type": "string", "description": "Path to JPEG or TIFF file" },
                "hex": { "type": "string", "description": "Hex-encoded image bytes" }
            },
            "required": []
        }
    })
}

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("info");

    let bytes = match get_bytes(args) {
        Some(b) => b,
        None => return Ok("Error: provide 'file' (path to JPEG/TIFF) or 'hex' bytes.".to_string()),
    };

    let tiff_data =
        match find_tiff_data(&bytes) {
            Some(d) => d,
            None => return Ok(
                "No EXIF data found. File must be a JPEG with APP1 EXIF segment or a TIFF file."
                    .to_string(),
            ),
        };

    if tiff_data.len() < 8 {
        return Ok("TIFF data too short to parse.".to_string());
    }

    let le = match (tiff_data[0], tiff_data[1]) {
        (0x49, 0x49) => true,
        (0x4D, 0x4D) => false,
        _ => return Ok("Invalid TIFF byte-order header in EXIF data.".to_string()),
    };

    let magic = read_u16(&tiff_data, 2, le);
    if magic != 42 {
        return Ok(format!(
            "Invalid TIFF magic: 0x{:04X} (expected 0x002A).",
            magic
        ));
    }

    let ifd0_off = read_u32(&tiff_data, 4, le) as usize;
    let ifd0 = parse_ifd(&tiff_data, ifd0_off, le, "IFD0");

    let mut exif_entries = Vec::new();
    let mut gps_entries = Vec::new();

    for e in &ifd0 {
        match e.tag {
            0x8769 => {
                if let Some(off) = e.value_offset {
                    exif_entries = parse_ifd(&tiff_data, off as usize, le, "ExifIFD");
                }
            }
            0x8825 => {
                if let Some(off) = e.value_offset {
                    gps_entries = parse_ifd(&tiff_data, off as usize, le, "GPSIFD");
                }
            }
            _ => {}
        }
    }

    let mut all: Vec<ExifEntry> = Vec::new();
    all.extend_from_slice(&ifd0);
    all.extend_from_slice(&exif_entries);
    all.extend_from_slice(&gps_entries);

    Ok(match action {
        "camera" => format_camera(&all),
        "gps" => format_gps(&gps_entries),
        "thumbnail" => format_thumbnail(&tiff_data, ifd0_off, le),
        _ => format_info(&all),
    })
}

fn get_bytes(args: &Value) -> Option<Vec<u8>> {
    if let Some(path) = args.get("file").and_then(|v| v.as_str()) {
        return std::fs::read(path).ok();
    }
    if let Some(hex) = args.get("hex").and_then(|v| v.as_str()) {
        let clean: String = hex.chars().filter(|c| c.is_ascii_hexdigit()).collect();
        let bytes = clean
            .as_bytes()
            .chunks(2)
            .filter_map(|c| u8::from_str_radix(std::str::from_utf8(c).unwrap_or(""), 16).ok())
            .collect();
        return Some(bytes);
    }
    None
}

fn find_tiff_data(bytes: &[u8]) -> Option<Vec<u8>> {
    if bytes.len() < 2 {
        return None;
    }
    // JPEG: FFD8
    if bytes[0] == 0xFF && bytes[1] == 0xD8 {
        return find_jpeg_exif(bytes);
    }
    // TIFF: II or MM
    if (bytes[0] == 0x49 && bytes[1] == 0x49) || (bytes[0] == 0x4D && bytes[1] == 0x4D) {
        return Some(bytes.to_vec());
    }
    None
}

fn find_jpeg_exif(bytes: &[u8]) -> Option<Vec<u8>> {
    let mut pos = 2;
    while pos + 3 < bytes.len() {
        if bytes[pos] != 0xFF {
            break;
        }
        let marker = bytes[pos + 1];
        if marker == 0xD9 {
            break; // EOI
        }
        if marker == 0x00 || (0xD0..=0xD7).contains(&marker) {
            pos += 2;
            continue;
        }
        if pos + 4 > bytes.len() {
            break;
        }
        let seg_len = u16::from_be_bytes([bytes[pos + 2], bytes[pos + 3]]) as usize;
        if seg_len < 2 {
            break;
        }
        let seg_end = (pos + 2 + seg_len).min(bytes.len());

        if marker == 0xE1 && seg_end > pos + 10 {
            let data_start = pos + 4;
            if data_start + 6 <= bytes.len() && &bytes[data_start..data_start + 6] == b"Exif\0\0" {
                return Some(bytes[data_start + 6..seg_end].to_vec());
            }
        }

        pos = seg_end;
    }
    None
}

#[derive(Clone, Debug)]
struct ExifEntry {
    tag: u16,
    name: &'static str,
    ifd: &'static str,
    value: String,
    value_offset: Option<u32>,
}

fn read_u16(data: &[u8], off: usize, le: bool) -> u16 {
    if off + 2 > data.len() {
        return 0;
    }
    if le {
        u16::from_le_bytes([data[off], data[off + 1]])
    } else {
        u16::from_be_bytes([data[off], data[off + 1]])
    }
}

fn read_u32(data: &[u8], off: usize, le: bool) -> u32 {
    if off + 4 > data.len() {
        return 0;
    }
    if le {
        u32::from_le_bytes([data[off], data[off + 1], data[off + 2], data[off + 3]])
    } else {
        u32::from_be_bytes([data[off], data[off + 1], data[off + 2], data[off + 3]])
    }
}

fn read_i32(data: &[u8], off: usize, le: bool) -> i32 {
    if off + 4 > data.len() {
        return 0;
    }
    if le {
        i32::from_le_bytes([data[off], data[off + 1], data[off + 2], data[off + 3]])
    } else {
        i32::from_be_bytes([data[off], data[off + 1], data[off + 2], data[off + 3]])
    }
}

fn parse_ifd(data: &[u8], offset: usize, le: bool, ifd_name: &'static str) -> Vec<ExifEntry> {
    if offset + 2 > data.len() {
        return vec![];
    }
    let count = read_u16(data, offset, le) as usize;
    let mut entries = Vec::with_capacity(count.min(256));

    for i in 0..count {
        let base = offset + 2 + i * 12;
        if base + 12 > data.len() {
            break;
        }

        let tag = read_u16(data, base, le);
        let typ = read_u16(data, base + 2, le);
        let cnt = read_u32(data, base + 4, le) as usize;

        let type_size: usize = match typ {
            1 | 2 | 6 | 7 => 1,
            3 | 8 => 2,
            4 | 9 => 4,
            5 | 10 => 8,
            _ => continue,
        };

        let total = cnt.saturating_mul(type_size);
        let raw_offset = read_u32(data, base + 8, le);
        let data_ptr = if total <= 4 {
            base + 8
        } else {
            raw_offset as usize
        };

        let value_str = format_tag_value(data, data_ptr, typ, cnt, le, tag);
        let value_offset = if matches!(tag, 0x8769 | 0x8825) {
            Some(raw_offset)
        } else {
            None
        };

        entries.push(ExifEntry {
            tag,
            name: tag_name(tag, ifd_name),
            ifd: ifd_name,
            value: value_str,
            value_offset,
        });
    }
    entries
}

fn format_tag_value(data: &[u8], ptr: usize, typ: u16, count: usize, le: bool, tag: u16) -> String {
    if ptr >= data.len() {
        return String::new();
    }
    match typ {
        2 => {
            // ASCII — read until null
            let end = data.len().min(ptr + count);
            let s: Vec<u8> = data[ptr..end]
                .iter()
                .take_while(|&&b| b != 0)
                .cloned()
                .collect();
            String::from_utf8_lossy(&s).trim().to_string()
        }
        1 | 7 => {
            // BYTE / UNDEFINED
            if count == 1 {
                if ptr < data.len() {
                    return format!("{}", data[ptr]);
                }
                return String::new();
            }
            if count <= 8 {
                let end = data.len().min(ptr + count);
                data[ptr..end]
                    .iter()
                    .map(|b| format!("{:02X}", b))
                    .collect::<Vec<_>>()
                    .join(" ")
            } else {
                format!("{} bytes", count)
            }
        }
        3 => {
            // SHORT
            if count == 1 {
                let v = read_u16(data, ptr, le);
                format_short_enum(v, tag)
            } else if count <= 4 {
                (0..count)
                    .map(|i| format!("{}", read_u16(data, ptr + i * 2, le)))
                    .collect::<Vec<_>>()
                    .join(", ")
            } else {
                format!("{} shorts", count)
            }
        }
        4 => {
            // LONG
            format!("{}", read_u32(data, ptr, le))
        }
        5 => {
            // RATIONAL
            if count == 1 {
                let num = read_u32(data, ptr, le);
                let den = read_u32(data, ptr + 4, le);
                fmt_rational(num, den, tag)
            } else if count <= 4 {
                (0..count)
                    .map(|i| {
                        let num = read_u32(data, ptr + i * 8, le);
                        let den = read_u32(data, ptr + i * 8 + 4, le);
                        if den == 0 {
                            format!("{}", num)
                        } else {
                            format!("{}/{}", num, den)
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(", ")
            } else {
                format!("{} rationals", count)
            }
        }
        9 => format!("{}", read_i32(data, ptr, le)),
        10 => {
            // SRATIONAL
            let num = read_i32(data, ptr, le);
            let den = read_i32(data, ptr + 4, le);
            if den == 0 {
                format!("{}", num)
            } else {
                format!("{:.4}", num as f64 / den as f64)
            }
        }
        _ => String::new(),
    }
}

fn fmt_rational(num: u32, den: u32, tag: u16) -> String {
    if den == 0 {
        return format!("{}", num);
    }
    match tag {
        0x829A => {
            // ExposureTime
            let secs = num as f64 / den as f64;
            if secs >= 1.0 {
                format!("{:.1} s", secs)
            } else {
                let recip = (den as f64 / num as f64).round() as u32;
                format!("1/{} s", recip)
            }
        }
        0x829D => format!("f/{:.1}", num as f64 / den as f64),
        0x920A => format!("{:.0} mm", num as f64 / den as f64),
        0x011A | 0x011B => format!("{:.0}", num as f64 / den as f64),
        0x9202 | 0x9205 => {
            let apex = num as f64 / den as f64;
            format!("{:.2} (f/{:.1})", apex, 2_f64.powf(apex / 2.0))
        }
        _ => {
            if den == 1 {
                format!("{}", num)
            } else if num % den == 0 {
                format!("{}", num / den)
            } else {
                format!("{:.4}", num as f64 / den as f64)
            }
        }
    }
}

fn format_short_enum(v: u16, tag: u16) -> String {
    match tag {
        0x0112 => match v {
            1 => "Normal (top-left)".to_string(),
            2 => "Mirrored horizontal".to_string(),
            3 => "Rotated 180°".to_string(),
            4 => "Mirrored vertical".to_string(),
            6 => "Rotated 90° CW".to_string(),
            8 => "Rotated 270° CW".to_string(),
            _ => format!("{}", v),
        },
        0x0128 => match v {
            1 => "None".to_string(),
            2 => "inch".to_string(),
            3 => "centimeter".to_string(),
            _ => format!("{}", v),
        },
        0x8822 => match v {
            0 => "Not defined".to_string(),
            1 => "Manual".to_string(),
            2 => "Normal program".to_string(),
            3 => "Aperture priority".to_string(),
            4 => "Shutter priority".to_string(),
            5 => "Creative (slow)".to_string(),
            6 => "Action (fast)".to_string(),
            7 => "Portrait".to_string(),
            8 => "Landscape".to_string(),
            _ => format!("{}", v),
        },
        0x9207 => match v {
            0 => "Unknown".to_string(),
            1 => "Average".to_string(),
            2 => "Center-weighted".to_string(),
            3 => "Spot".to_string(),
            4 => "Multi-spot".to_string(),
            5 => "Multi-segment (evaluative)".to_string(),
            6 => "Partial".to_string(),
            255 => "Other".to_string(),
            _ => format!("{}", v),
        },
        0x9209 => {
            let fired = v & 0x01 != 0;
            let mode = (v >> 3) & 0x03;
            let base = if fired { "Fired" } else { "Did not fire" };
            let suffix = match mode {
                1 => " (forced on)",
                2 => " (forced off)",
                3 => " (auto)",
                _ => "",
            };
            format!("{}{}", base, suffix)
        }
        0xA001 => match v {
            1 => "sRGB".to_string(),
            65535 => "Uncalibrated".to_string(),
            _ => format!("{}", v),
        },
        0xA402 => match v {
            0 => "Auto".to_string(),
            1 => "Manual".to_string(),
            2 => "Auto bracket".to_string(),
            _ => format!("{}", v),
        },
        0xA403 => match v {
            0 => "Auto".to_string(),
            1 => "Manual".to_string(),
            _ => format!("{}", v),
        },
        0xA406 => match v {
            0 => "Standard".to_string(),
            1 => "Landscape".to_string(),
            2 => "Portrait".to_string(),
            3 => "Night scene".to_string(),
            _ => format!("{}", v),
        },
        _ => format!("{}", v),
    }
}

fn tag_name(tag: u16, ifd: &'static str) -> &'static str {
    if ifd == "GPSIFD" {
        return match tag {
            0x0000 => "GPS Version",
            0x0001 => "GPS Latitude Ref",
            0x0002 => "GPS Latitude",
            0x0003 => "GPS Longitude Ref",
            0x0004 => "GPS Longitude",
            0x0005 => "GPS Altitude Ref",
            0x0006 => "GPS Altitude",
            0x0007 => "GPS Time",
            0x0008 => "GPS Satellites",
            0x0009 => "GPS Status",
            0x000A => "GPS Measure Mode",
            0x000B => "GPS DOP",
            0x000C => "GPS Speed Ref",
            0x000D => "GPS Speed",
            0x000E => "GPS Track Ref",
            0x000F => "GPS Track",
            0x0010 => "GPS Img Direction Ref",
            0x0011 => "GPS Img Direction",
            0x001D => "GPS Date",
            _ => "GPS Unknown",
        };
    }
    match tag {
        0x010E => "Image Description",
        0x010F => "Make",
        0x0110 => "Model",
        0x0112 => "Orientation",
        0x011A => "X Resolution",
        0x011B => "Y Resolution",
        0x0128 => "Resolution Unit",
        0x0131 => "Software",
        0x0132 => "Date/Time",
        0x013B => "Artist",
        0x8298 => "Copyright",
        0x8769 => "EXIF IFD Pointer",
        0x8825 => "GPS IFD Pointer",
        0x829A => "Exposure Time",
        0x829D => "F-Number",
        0x8822 => "Exposure Program",
        0x8824 => "Spectral Sensitivity",
        0x8827 => "ISO",
        0x9000 => "EXIF Version",
        0x9003 => "Date/Time Original",
        0x9004 => "Date/Time Digitized",
        0x9101 => "Components Configuration",
        0x9201 => "Shutter Speed (APEX)",
        0x9202 => "Aperture (APEX)",
        0x9203 => "Brightness Value",
        0x9204 => "Exposure Bias",
        0x9205 => "Max Aperture (APEX)",
        0x9206 => "Subject Distance",
        0x9207 => "Metering Mode",
        0x9208 => "Light Source",
        0x9209 => "Flash",
        0x920A => "Focal Length",
        0x927C => "Maker Note",
        0x9286 => "User Comment",
        0x9290 => "Sub-Second Time",
        0x9291 => "Sub-Second Time Original",
        0x9292 => "Sub-Second Time Digitized",
        0xA000 => "FlashPix Version",
        0xA001 => "Color Space",
        0xA002 => "Pixel X Dimension",
        0xA003 => "Pixel Y Dimension",
        0xA005 => "Interop IFD Pointer",
        0xA217 => "Sensing Method",
        0xA300 => "File Source",
        0xA301 => "Scene Type",
        0xA401 => "Custom Rendered",
        0xA402 => "Exposure Mode",
        0xA403 => "White Balance",
        0xA404 => "Digital Zoom Ratio",
        0xA405 => "Focal Length in 35mm Film",
        0xA406 => "Scene Capture Type",
        0xA407 => "Gain Control",
        0xA408 => "Contrast",
        0xA409 => "Saturation",
        0xA40A => "Sharpness",
        0xA40C => "Subject Distance Range",
        0xA420 => "Image Unique ID",
        0xA430 => "Camera Owner Name",
        0xA431 => "Body Serial Number",
        0xA432 => "Lens Specification",
        0xA433 => "Lens Make",
        0xA434 => "Lens Model",
        0xA435 => "Lens Serial Number",
        _ => "Unknown",
    }
}

fn format_info(entries: &[ExifEntry]) -> String {
    let visible: Vec<&ExifEntry> = entries
        .iter()
        .filter(|e| {
            !e.value.is_empty()
                && e.name != "Unknown"
                && !matches!(e.tag, 0x8769 | 0x8825 | 0xA005 | 0x927C | 0x9101)
        })
        .collect();

    if visible.is_empty() {
        return "No EXIF entries found.".to_string();
    }

    let mut out = format!("EXIF Metadata  ({} fields)\n\n", visible.len());

    let sections: &[(&str, &str)] = &[
        ("IFD0", "Image"),
        ("ExifIFD", "Camera Settings"),
        ("GPSIFD", "GPS"),
    ];
    for (ifd, label) in sections {
        let group: Vec<_> = visible.iter().filter(|e| e.ifd == *ifd).collect();
        if group.is_empty() {
            continue;
        }
        let _ = writeln!(out, "── {} ──", label);
        for e in group {
            let _ = writeln!(out, "  {:32} {}", e.name, e.value);
        }
        out.push('\n');
    }
    out
}

fn format_camera(entries: &[ExifEntry]) -> String {
    let mut out = String::from("Camera & Lens Info\n\n");
    let order: &[u16] = &[
        0x010F, // Make
        0x0110, // Model
        0xA431, // Body Serial
        0xA433, // Lens Make
        0xA434, // Lens Model
        0xA435, // Lens Serial
        0xA432, // Lens Spec
        0x9003, // DateTime Original
        0x8827, // ISO
        0x829A, // Exposure Time
        0x829D, // FNumber
        0xA405, // 35mm focal length
        0x920A, // Focal Length
        0x8822, // Exposure Program
        0xA402, // Exposure Mode
        0xA403, // White Balance
        0x9207, // Metering Mode
        0x9209, // Flash
        0x9204, // Exposure Bias
        0xA002, // Width
        0xA003, // Height
        0xA001, // Color Space
        0x0131, // Software
    ];
    let mut any = false;
    for &tag in order {
        if let Some(e) = entries.iter().find(|e| e.tag == tag) {
            if !e.value.is_empty() {
                let _ = writeln!(out, "  {:28} {}", e.name, e.value);
                any = true;
            }
        }
    }
    if !any {
        out.push_str("  No camera metadata found.\n");
    }
    out
}

fn format_gps(gps: &[ExifEntry]) -> String {
    if gps.is_empty() {
        return "No GPS data found in this image.".to_string();
    }

    let mut out = String::from("GPS Location\n\n");

    let lat_ref = gps
        .iter()
        .find(|e| e.tag == 0x0001)
        .map(|e| e.value.clone())
        .unwrap_or_default();
    let lat_val = gps
        .iter()
        .find(|e| e.tag == 0x0002)
        .map(|e| e.value.clone())
        .unwrap_or_default();
    let lon_ref = gps
        .iter()
        .find(|e| e.tag == 0x0003)
        .map(|e| e.value.clone())
        .unwrap_or_default();
    let lon_val = gps
        .iter()
        .find(|e| e.tag == 0x0004)
        .map(|e| e.value.clone())
        .unwrap_or_default();

    if !lat_val.is_empty() && !lon_val.is_empty() {
        if let (Some(lat_d), Some(lon_d)) =
            (parse_dms_rational(&lat_val), parse_dms_rational(&lon_val))
        {
            let lat_sign = if lat_ref.starts_with('S') { -1.0 } else { 1.0 };
            let lon_sign = if lon_ref.starts_with('W') { -1.0 } else { 1.0 };
            let lat = lat_d * lat_sign;
            let lon = lon_d * lon_sign;
            let _ = writeln!(out, "  Latitude:  {:.6}°  ({})", lat.abs(), lat_ref);
            let _ = writeln!(out, "  Longitude: {:.6}°  ({})", lon.abs(), lon_ref);
            let _ = writeln!(out, "  Decimal:   {:.6}, {:.6}", lat, lon);
            let _ = writeln!(
                out,
                "  Maps:      https://maps.google.com/?q={:.6},{:.6}",
                lat, lon
            );
            out.push('\n');
        } else {
            let _ = writeln!(out, "  Latitude:  {} {}", lat_val, lat_ref);
            let _ = writeln!(out, "  Longitude: {} {}", lon_val, lon_ref);
            out.push('\n');
        }
    }

    for e in gps {
        if !matches!(e.tag, 0x0001..=0x0004) && e.name != "GPS Unknown" && !e.value.is_empty() {
            let _ = writeln!(out, "  {:25} {}", e.name, e.value);
        }
    }

    out
}

fn parse_dms_rational(s: &str) -> Option<f64> {
    // Format: "D/1, M/1, S/100"
    let parts: Vec<&str> = s.split(',').collect();
    if parts.len() != 3 {
        return None;
    }
    fn parse_r(s: &str) -> Option<f64> {
        let s = s.trim();
        if let Some(p) = s.find('/') {
            let n: f64 = s[..p].trim().parse().ok()?;
            let d: f64 = s[p + 1..].trim().parse().ok()?;
            if d == 0.0 {
                None
            } else {
                Some(n / d)
            }
        } else {
            s.parse().ok()
        }
    }
    let d = parse_r(parts[0])?;
    let m = parse_r(parts[1])?;
    let s = parse_r(parts[2])?;
    Some(d + m / 60.0 + s / 3600.0)
}

fn format_thumbnail(data: &[u8], ifd0_off: usize, le: bool) -> String {
    // IFD1 offset is at the end of IFD0
    if ifd0_off + 2 > data.len() {
        return "Could not locate IFD1.".to_string();
    }
    let count = read_u16(data, ifd0_off, le) as usize;
    let ifd1_ptr = ifd0_off + 2 + count * 12;
    if ifd1_ptr + 4 > data.len() {
        return "No thumbnail IFD found.".to_string();
    }
    let ifd1_off = read_u32(data, ifd1_ptr, le) as usize;
    if ifd1_off == 0 {
        return "No embedded thumbnail (IFD1 offset is 0).".to_string();
    }

    let ifd1 = parse_ifd(data, ifd1_off, le, "IFD1");
    let thumb_off = ifd1
        .iter()
        .find(|e| e.tag == 0x0201)
        .and_then(|e| e.value.parse::<usize>().ok());
    let thumb_len = ifd1
        .iter()
        .find(|e| e.tag == 0x0202)
        .and_then(|e| e.value.parse::<usize>().ok());

    match (thumb_off, thumb_len) {
        (Some(off), Some(len)) if off > 0 && len > 0 => {
            format!(
                "Embedded JPEG thumbnail found\n  Offset: {} bytes into TIFF data\n  Size:   {} bytes ({:.1} KB)\n",
                off,
                len,
                len as f64 / 1024.0
            )
        }
        _ => {
            if ifd1.is_empty() {
                "No embedded thumbnail found.".to_string()
            } else {
                format!(
                    "IFD1 present ({} entries) but thumbnail location unresolved.",
                    ifd1.len()
                )
            }
        }
    }
}
