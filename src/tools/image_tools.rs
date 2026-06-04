use serde_json::{json, Value};

pub fn make_schema() -> Value {
    json!({
        "name": "image_tools",
        "description": "Parse image file metadata (PNG/JPEG/GIF/WebP/BMP) without external utilities. \
Actions: info (default — format, dimensions, color mode, bit depth, resolution, animation, ICC/sRGB flags), \
dimensions (width × height, aspect ratio, print size at detected DPI), \
color (color mode, bit depth, palette size, alpha presence, color space), \
metadata (embedded text tags, XMP, ICC profile, comment fields), \
validate (format-specific compliance: IHDR/SOF presence, dimensions, palette integrity). \
Pass file (path to image) or hex (hex-encoded bytes). \
Example: image_tools(file: 'photo.png') or image_tools(action: 'color', file: 'logo.gif') or image_tools(action: 'metadata', file: 'image.jpg').",
        "input_schema": {
            "type": "object",
            "properties": {
                "action": { "type": "string", "description": "info|dimensions|color|metadata|validate" },
                "file": { "type": "string", "description": "Path to PNG, JPEG, GIF, WebP, or BMP file" },
                "hex": { "type": "string", "description": "Hex-encoded image bytes" }
            },
            "required": []
        }
    })
}

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args.get("action").and_then(|v| v.as_str()).unwrap_or("info");
    let bytes = match get_bytes(args) {
        Some(b) => b,
        None => return Ok("Error: provide 'file' (path to image) or 'hex' bytes.".to_string()),
    };
    if bytes.len() < 4 {
        return Ok("Error: data too short to identify image format.".to_string());
    }
    Ok(match detect_format(&bytes) {
        ImageFormat::Png  => dispatch_png(action, &bytes),
        ImageFormat::Jpeg => dispatch_jpeg(action, &bytes),
        ImageFormat::Gif  => dispatch_gif(action, &bytes),
        ImageFormat::WebP => dispatch_webp(action, &bytes),
        ImageFormat::Bmp  => dispatch_bmp(action, &bytes),
        ImageFormat::Unknown => format!(
            "Error: unrecognised image format. Magic bytes: {:02X} {:02X} {:02X} {:02X}\n\
             Supported formats: PNG (89 50 4E 47), JPEG (FF D8), GIF (47 49 46), \
             WebP (RIFF/WEBP), BMP (42 4D).",
            bytes[0], bytes[1], bytes[2], bytes.get(3).copied().unwrap_or(0)
        ),
    })
}

fn get_bytes(args: &Value) -> Option<Vec<u8>> {
    if let Some(path) = args.get("file").and_then(|v| v.as_str()) {
        return std::fs::read(path).ok();
    }
    if let Some(hex) = args.get("hex").and_then(|v| v.as_str()) {
        let clean: String = hex.chars().filter(|c| c.is_ascii_hexdigit()).collect();
        return Some(
            clean.as_bytes().chunks(2)
                .filter_map(|c| u8::from_str_radix(std::str::from_utf8(c).unwrap_or(""), 16).ok())
                .collect(),
        );
    }
    None
}

enum ImageFormat { Png, Jpeg, Gif, WebP, Bmp, Unknown }

fn detect_format(b: &[u8]) -> ImageFormat {
    if b.len() >= 8 && b[..8] == [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A] {
        return ImageFormat::Png;
    }
    if b.len() >= 2 && b[..2] == [0xFF, 0xD8] {
        return ImageFormat::Jpeg;
    }
    if b.len() >= 6 && (&b[..6] == b"GIF87a" || &b[..6] == b"GIF89a") {
        return ImageFormat::Gif;
    }
    if b.len() >= 12 && &b[..4] == b"RIFF" && &b[8..12] == b"WEBP" {
        return ImageFormat::WebP;
    }
    if b.len() >= 2 && b[..2] == [0x42, 0x4D] {
        return ImageFormat::Bmp;
    }
    ImageFormat::Unknown
}

// ── byte helpers ─────────────────────────────────────────────────────────────

fn ru16be(b: &[u8], o: usize) -> u16 {
    if o + 1 >= b.len() { return 0; }
    ((b[o] as u16) << 8) | b[o + 1] as u16
}
fn ru32be(b: &[u8], o: usize) -> u32 {
    if o + 3 >= b.len() { return 0; }
    ((b[o] as u32) << 24) | ((b[o+1] as u32) << 16) | ((b[o+2] as u32) << 8) | b[o+3] as u32
}
fn ru16le(b: &[u8], o: usize) -> u16 {
    if o + 1 >= b.len() { return 0; }
    b[o] as u16 | ((b[o + 1] as u16) << 8)
}
fn ru32le(b: &[u8], o: usize) -> u32 {
    if o + 3 >= b.len() { return 0; }
    b[o] as u32 | ((b[o+1] as u32) << 8) | ((b[o+2] as u32) << 16) | ((b[o+3] as u32) << 24)
}
fn ri32le(b: &[u8], o: usize) -> i32 { ru32le(b, o) as i32 }

fn gcd(a: u32, b: u32) -> u32 { if b == 0 { a } else { gcd(b, a % b) } }
fn aspect(w: u32, h: u32) -> String {
    if w == 0 || h == 0 { return "?".to_string(); }
    let g = gcd(w, h);
    format!("{}:{}", w / g, h / g)
}
fn human_size(n: usize) -> String {
    if n >= 1_048_576 { format!("{:.1} MB", n as f64 / 1_048_576.0) }
    else if n >= 1024 { format!("{:.1} KB", n as f64 / 1024.0) }
    else { format!("{} B", n) }
}

// ── PNG ───────────────────────────────────────────────────────────────────────

struct PngMeta {
    width: u32, height: u32, bit_depth: u8, color_type: u8, interlace: u8,
    has_transparency: bool, palette_colors: Option<u32>,
    has_icc: bool, has_srgb: bool, gamma: Option<f64>,
    x_dpi: Option<f64>, y_dpi: Option<f64>,
    animated: bool, frame_count: u32,
    text_tags: Vec<(String, String)>,
    valid_ihdr: bool, file_size: usize,
}

fn parse_png(b: &[u8]) -> PngMeta {
    let mut m = PngMeta {
        width: 0, height: 0, bit_depth: 0, color_type: 0, interlace: 0,
        has_transparency: false, palette_colors: None,
        has_icc: false, has_srgb: false, gamma: None,
        x_dpi: None, y_dpi: None,
        animated: false, frame_count: 0,
        text_tags: Vec::new(),
        valid_ihdr: false, file_size: b.len(),
    };
    let mut pos = 8usize;
    while pos + 12 <= b.len() {
        let len = ru32be(b, pos) as usize;
        if pos + 4 > b.len() { break; }
        let tag = &b[pos + 4..pos + 8];
        let ds = pos + 8;
        let de = ds + len;
        if de > b.len() { break; }
        let d = &b[ds..de];
        match tag {
            b"IHDR" if d.len() >= 13 => {
                m.width = ru32be(d, 0); m.height = ru32be(d, 4);
                m.bit_depth = d[8]; m.color_type = d[9]; m.interlace = d[12];
                m.valid_ihdr = true;
            }
            b"PLTE" => { m.palette_colors = Some(len as u32 / 3); }
            b"tRNS" => { m.has_transparency = true; }
            b"iCCP" => { m.has_icc = true; }
            b"sRGB" => { m.has_srgb = true; }
            b"gAMA" if d.len() >= 4 => { m.gamma = Some(ru32be(d, 0) as f64 / 100_000.0); }
            b"pHYs" if d.len() >= 9 && d[8] == 1 => {
                let xp = ru32be(d, 0);
                let yp = ru32be(d, 4);
                if xp > 0 && yp > 0 {
                    m.x_dpi = Some(xp as f64 * 0.0254);
                    m.y_dpi = Some(yp as f64 * 0.0254);
                }
            }
            b"acTL" if d.len() >= 4 => { m.animated = true; m.frame_count = ru32be(d, 0); }
            b"tEXt" => {
                if let Some(nul) = d.iter().position(|&x| x == 0) {
                    let k = String::from_utf8_lossy(&d[..nul]).to_string();
                    let v = String::from_utf8_lossy(&d[nul + 1..]).chars().take(120).collect();
                    m.text_tags.push((k, v));
                }
            }
            b"iTXt" => {
                if let Some(nul) = d.iter().position(|&x| x == 0) {
                    let k = String::from_utf8_lossy(&d[..nul]).to_string();
                    // skip: comp_flag(1) comp_method(1) lang\0 translated\0 then text
                    let mut p = nul + 3;
                    let mut nuls = 0;
                    while p < d.len() && nuls < 2 { if d[p] == 0 { nuls += 1; } p += 1; }
                    if p < d.len() {
                        let v: String = String::from_utf8_lossy(&d[p..]).chars().take(120).collect();
                        if !v.is_empty() { m.text_tags.push((k, v)); }
                    }
                }
            }
            b"IEND" => break,
            _ => {}
        }
        pos = de + 4; // skip 4-byte CRC
    }
    m
}

fn png_color_label(ct: u8) -> &'static str {
    match ct { 0 => "Grayscale", 2 => "RGB", 3 => "Indexed", 4 => "Grayscale+Alpha", 6 => "RGBA", _ => "Unknown" }
}
fn png_bpp(ct: u8, bd: u8) -> u8 {
    match ct { 2 => bd * 3, 4 => bd * 2, 6 => bd * 4, _ => bd }
}
fn png_has_alpha(ct: u8, tRNS: bool) -> bool { ct == 4 || ct == 6 || tRNS }

fn dispatch_png(action: &str, b: &[u8]) -> String {
    let m = parse_png(b);
    match action {
        "dimensions" => {
            let mut out = format!("Width:  {} px\nHeight: {} px\nAspect: {}\nPixels: {}\n",
                m.width, m.height, aspect(m.width, m.height),
                m.width as u64 * m.height as u64);
            if let (Some(xd), Some(yd)) = (m.x_dpi, m.y_dpi) {
                if (xd - yd).abs() < 1.0 {
                    out += &format!("DPI:    {:.0}\n", xd);
                    out += &format!("Print:  {:.2}\" × {:.2}\"\n", m.width as f64 / xd, m.height as f64 / yd);
                } else {
                    out += &format!("DPI:    {:.0} × {:.0}\n", xd, yd);
                }
            }
            out
        }
        "color" => {
            let mut out = format!("Color Mode:  {} ({} bpp)\n", png_color_label(m.color_type), png_bpp(m.color_type, m.bit_depth));
            out += &format!("Bit Depth:   {} bits/sample\n", m.bit_depth);
            out += &format!("Alpha:       {}\n", if png_has_alpha(m.color_type, m.has_transparency) { "Yes" } else { "No" });
            if let Some(n) = m.palette_colors { out += &format!("Palette:     {} colors\n", n); }
            if m.has_srgb { out += "Color Space: sRGB\n"; }
            if m.has_icc  { out += "ICC Profile: Present\n"; }
            if let Some(g) = m.gamma { out += &format!("Gamma:       {:.4}\n", g); }
            out
        }
        "metadata" => {
            if m.text_tags.is_empty() && !m.has_icc {
                return "No embedded text metadata found.\n".to_string();
            }
            let mut out = String::new();
            for (k, v) in &m.text_tags {
                out += &format!("{:<20} {}\n", k, v);
            }
            if m.has_icc { out += "ICC Profile:         Present\n"; }
            out
        }
        "validate" => {
            let mut issues: Vec<String> = Vec::new();
            if !m.valid_ihdr { issues.push("IHDR chunk missing or corrupt".to_string()); }
            if m.width == 0 || m.height == 0 { issues.push("Zero dimension".to_string()); }
            if m.color_type == 3 && m.palette_colors.is_none() { issues.push("Indexed PNG missing PLTE chunk".to_string()); }
            if issues.is_empty() { format!("VALID\n  ✓ IHDR present, dimensions {}×{}\n", m.width, m.height) }
            else { format!("ISSUES:\n{}\n", issues.iter().map(|s| format!("  ✗ {}", s)).collect::<Vec<_>>().join("\n")) }
        }
        _ => {
            let mut out = format!("Format:     PNG\nFile Size:  {}\n", human_size(m.file_size));
            out += &format!("Dimensions: {} × {}  ({})\n", m.width, m.height, aspect(m.width, m.height));
            out += &format!("Color Mode: {} ({} bpp)\n", png_color_label(m.color_type), png_bpp(m.color_type, m.bit_depth));
            out += &format!("Bit Depth:  {} bits/sample\n", m.bit_depth);
            out += &format!("Alpha:      {}\n", if png_has_alpha(m.color_type, m.has_transparency) { "Yes" } else { "No" });
            if let (Some(xd), Some(yd)) = (m.x_dpi, m.y_dpi) {
                if (xd - yd).abs() < 1.0 { out += &format!("Resolution: {:.0} DPI\n", xd); }
                else { out += &format!("Resolution: {:.0} × {:.0} DPI\n", xd, yd); }
            }
            out += &format!("Interlaced: {}\n", if m.interlace == 0 { "No" } else { "Yes (Adam7)" });
            if m.animated { out += &format!("Animated:   Yes ({} frames, APNG)\n", m.frame_count); }
            if m.has_icc  { out += "ICC Profile: Present\n"; }
            if m.has_srgb { out += "sRGB:       Yes\n"; }
            if let Some(g) = m.gamma { out += &format!("Gamma:      {:.4}\n", g); }
            if let Some(n) = m.palette_colors { out += &format!("Palette:    {} colors\n", n); }
            if !m.text_tags.is_empty() { out += &format!("Text Tags:  {} embedded\n", m.text_tags.len()); }
            out
        }
    }
}

// ── JPEG ─────────────────────────────────────────────────────────────────────

struct JpegMeta {
    width: u16, height: u16, components: u8, bit_depth: u8,
    sof_name: &'static str,
    jfif: bool, jfif_major: u8, jfif_minor: u8,
    density_unit: u8, x_density: u16, y_density: u16,
    has_exif: bool, has_xmp: bool, has_icc: bool,
    comment: Option<String>,
    file_size: usize,
}

fn parse_jpeg(b: &[u8]) -> JpegMeta {
    let mut m = JpegMeta {
        width: 0, height: 0, components: 0, bit_depth: 8,
        sof_name: "Unknown",
        jfif: false, jfif_major: 0, jfif_minor: 0,
        density_unit: 0, x_density: 0, y_density: 0,
        has_exif: false, has_xmp: false, has_icc: false,
        comment: None, file_size: b.len(),
    };
    let mut pos = 2usize; // skip SOI 0xFF 0xD8
    while pos + 2 <= b.len() {
        // scan for 0xFF non-padding byte
        if b[pos] != 0xFF { break; }
        let mut p = pos;
        while p < b.len() && b[p] == 0xFF { p += 1; }
        if p >= b.len() { break; }
        let marker = b[p];
        pos = p + 1;
        if marker == 0xD9 { break; } // EOI
        if marker == 0xD8 || marker == 0x01 || (marker >= 0xD0 && marker <= 0xD7) { continue; }
        if pos + 2 > b.len() { break; }
        let seg_len = ru16be(b, pos) as usize;
        if seg_len < 2 || pos + seg_len > b.len() { break; }
        let d = &b[pos + 2..pos + seg_len];
        match marker {
            0xE0 if d.len() >= 9 && &d[..5] == b"JFIF\0" => {
                m.jfif = true;
                m.jfif_major = d[5]; m.jfif_minor = d[6];
                m.density_unit = d[7];
                if d.len() >= 12 { m.x_density = ru16be(d, 8); m.y_density = ru16be(d, 10); }
            }
            0xE1 => {
                if d.len() >= 6 && &d[..6] == b"Exif\0\0" { m.has_exif = true; }
                else if d.len() >= 29 && &d[..29] == b"http://ns.adobe.com/xap/1.0/\0" { m.has_xmp = true; }
            }
            0xE2 if d.len() >= 12 && &d[..12] == b"ICC_PROFILE\0" => { m.has_icc = true; }
            0xFE => { m.comment = Some(String::from_utf8_lossy(d).chars().take(200).collect()); }
            0xC0 => {
                m.sof_name = "Baseline DCT";
                if d.len() >= 6 { m.bit_depth = d[0]; m.height = ru16be(d, 1); m.width = ru16be(d, 3); m.components = d[5]; }
            }
            0xC1 => {
                m.sof_name = "Extended Sequential DCT";
                if d.len() >= 6 { m.bit_depth = d[0]; m.height = ru16be(d, 1); m.width = ru16be(d, 3); m.components = d[5]; }
            }
            0xC2 => {
                m.sof_name = "Progressive DCT";
                if d.len() >= 6 { m.bit_depth = d[0]; m.height = ru16be(d, 1); m.width = ru16be(d, 3); m.components = d[5]; }
            }
            0xC3 => {
                m.sof_name = "Lossless";
                if d.len() >= 6 { m.bit_depth = d[0]; m.height = ru16be(d, 1); m.width = ru16be(d, 3); m.components = d[5]; }
            }
            _ => {}
        }
        pos += seg_len;
    }
    m
}

fn jpeg_color(c: u8) -> &'static str {
    match c { 1 => "Grayscale", 3 => "YCbCr (RGB)", 4 => "CMYK", _ => "Unknown" }
}
fn jpeg_density_str(unit: u8, xd: u16, yd: u16) -> String {
    match unit {
        1 => if xd == yd { format!("{} DPI", xd) } else { format!("{} × {} DPI", xd, yd) },
        2 => format!("{} × {} DPCM ({:.0} DPI)", xd, yd, xd as f64 * 2.54),
        _ => format!("{}×{} (aspect only)", xd, yd),
    }
}

fn dispatch_jpeg(action: &str, b: &[u8]) -> String {
    let m = parse_jpeg(b);
    match action {
        "dimensions" => {
            let mut out = format!("Width:  {} px\nHeight: {} px\nAspect: {}\n",
                m.width, m.height, aspect(m.width as u32, m.height as u32));
            if m.x_density > 0 { out += &format!("DPI:    {}\n", jpeg_density_str(m.density_unit, m.x_density, m.y_density)); }
            out
        }
        "color" => format!("Color Mode: {}\nBit Depth:  {} bits/sample\nEncoding:   {}\n",
            jpeg_color(m.components), m.bit_depth, m.sof_name),
        "metadata" => {
            let mut out = String::new();
            if m.has_exif { out += "EXIF:        Present (use exif_tools for detail)\n"; }
            if m.has_xmp  { out += "XMP:         Present\n"; }
            if m.has_icc  { out += "ICC Profile: Present\n"; }
            if let Some(c) = &m.comment { out += &format!("Comment:     {}\n", c); }
            if out.is_empty() { out = "No embedded metadata found.\n".to_string(); }
            out
        }
        "validate" => {
            if m.width == 0 || m.height == 0 {
                "ISSUES:\n  ✗ No SOF marker found — dimensions unknown\n".to_string()
            } else {
                format!("VALID\n  ✓ SOF/{} found, dimensions {}×{}\n", m.sof_name, m.width, m.height)
            }
        }
        _ => {
            let mut out = format!("Format:     JPEG ({})\nFile Size:  {}\n", m.sof_name, human_size(m.file_size));
            out += &format!("Dimensions: {} × {}  ({})\n", m.width, m.height, aspect(m.width as u32, m.height as u32));
            out += &format!("Color Mode: {}\nBit Depth:  {} bits/sample\n", jpeg_color(m.components), m.bit_depth);
            if m.jfif { out += &format!("JFIF:       v{}.{:02}\n", m.jfif_major, m.jfif_minor); }
            if m.x_density > 0 { out += &format!("Density:    {}\n", jpeg_density_str(m.density_unit, m.x_density, m.y_density)); }
            if m.has_exif { out += "EXIF:       Present\n"; }
            if m.has_xmp  { out += "XMP:        Present\n"; }
            if m.has_icc  { out += "ICC Profile: Present\n"; }
            if let Some(c) = &m.comment { out += &format!("Comment:    {}\n", c); }
            out
        }
    }
}

// ── GIF ───────────────────────────────────────────────────────────────────────

struct GifMeta {
    version: String,
    width: u16, height: u16,
    gct_colors: u32, color_resolution: u8,
    animated: bool, frame_count: u32,
    loop_count: Option<u16>,
    comments: Vec<String>,
    file_size: usize,
}

fn parse_gif(b: &[u8]) -> GifMeta {
    let mut m = GifMeta {
        version: String::from_utf8_lossy(&b[..6.min(b.len())]).to_string(),
        width: 0, height: 0, gct_colors: 0, color_resolution: 0,
        animated: false, frame_count: 0, loop_count: None,
        comments: Vec::new(), file_size: b.len(),
    };
    if b.len() < 13 { return m; }
    m.width = ru16le(b, 6); m.height = ru16le(b, 8);
    let packed = b[10];
    let has_gct = (packed >> 7) & 1 == 1;
    m.color_resolution = ((packed >> 4) & 0x07) + 1;
    let gct_flag = packed & 0x07;
    m.gct_colors = 2u32.pow(gct_flag as u32 + 1);
    let mut pos = 13usize;
    if has_gct { pos += m.gct_colors as usize * 3; }
    while pos < b.len() {
        match b[pos] {
            0x3B => break,
            0x2C => {
                m.frame_count += 1;
                if pos + 10 > b.len() { break; }
                let local_packed = b[pos + 9];
                let has_lct = (local_packed >> 7) & 1 == 1;
                let lct_sz = local_packed & 0x07;
                pos += 10;
                if has_lct { pos += 3 * 2usize.pow(lct_sz as u32 + 1); }
                pos += 1; // LZW min code size
                pos = skip_sub_blocks(b, pos);
            }
            0x21 if pos + 1 < b.len() => {
                let ext = b[pos + 1];
                pos += 2;
                match ext {
                    0xFE => {
                        let s = read_sub_blocks_str(b, pos);
                        if !s.is_empty() { m.comments.push(s); }
                        pos = skip_sub_blocks(b, pos);
                    }
                    0xFF => {
                        // check for NETSCAPE loop extension
                        if pos < b.len() && b[pos] == 11 && pos + 11 < b.len() {
                            let app = &b[pos + 1..pos + 12];
                            if app == b"NETSCAPE2.0" || app == b"ANIMEXTS1.0" {
                                if pos + 12 < b.len() && b[pos + 12] == 3
                                    && pos + 15 < b.len() && b[pos + 13] == 1 {
                                    m.loop_count = Some(ru16le(b, pos + 14));
                                }
                            }
                        }
                        pos = skip_sub_blocks(b, pos);
                    }
                    _ => { pos = skip_sub_blocks(b, pos); }
                }
            }
            _ => { pos += 1; }
        }
    }
    m.animated = m.frame_count > 1;
    m
}

fn skip_sub_blocks(b: &[u8], mut p: usize) -> usize {
    while p < b.len() {
        let sz = b[p] as usize;
        p += 1;
        if sz == 0 { break; }
        p += sz;
    }
    p
}

fn read_sub_blocks_str(b: &[u8], mut p: usize) -> String {
    let mut v: Vec<u8> = Vec::new();
    while p < b.len() {
        let sz = b[p] as usize;
        p += 1;
        if sz == 0 { break; }
        let end = (p + sz).min(b.len());
        v.extend_from_slice(&b[p..end]);
        p = end;
    }
    String::from_utf8_lossy(&v).chars().take(200).collect()
}

fn dispatch_gif(action: &str, b: &[u8]) -> String {
    let m = parse_gif(b);
    match action {
        "dimensions" => format!("Width:  {} px\nHeight: {} px\nAspect: {}\n",
            m.width, m.height, aspect(m.width as u32, m.height as u32)),
        "color" => format!("Color Mode:       Indexed palette\nPalette Colors:   {}\nColor Resolution: {} bits\n",
            m.gct_colors, m.color_resolution),
        "metadata" => {
            if m.comments.is_empty() { "No comment extensions found.\n".to_string() }
            else {
                m.comments.iter().enumerate()
                    .map(|(i, c)| format!("Comment {}: {}\n", i + 1, c))
                    .collect()
            }
        }
        "validate" => {
            if m.width == 0 || m.height == 0 { "ISSUES:\n  ✗ Zero dimensions\n".to_string() }
            else { format!("VALID\n  ✓ Logical Screen Descriptor parsed, {}×{}\n", m.width, m.height) }
        }
        _ => {
            let mut out = format!("Format:     {}\nFile Size:  {}\n", m.version, human_size(m.file_size));
            out += &format!("Dimensions: {} × {}  ({})\n", m.width, m.height, aspect(m.width as u32, m.height as u32));
            out += &format!("Palette:    {} colors ({}-bit)\n", m.gct_colors, m.color_resolution);
            out += &format!("Animated:   {}", if m.animated { "Yes" } else { "No" });
            if m.animated {
                out += &format!(" ({} frames", m.frame_count);
                if let Some(lc) = m.loop_count {
                    out += &format!(", loop={}", if lc == 0 { "∞".to_string() } else { lc.to_string() });
                }
                out += ")";
            }
            out += "\n";
            if !m.comments.is_empty() {
                out += &format!("Comment:    {}\n", m.comments[0].chars().take(60).collect::<String>());
            }
            out
        }
    }
}

// ── WebP ─────────────────────────────────────────────────────────────────────

struct WebPMeta {
    subtype: &'static str,
    width: u32, height: u32,
    has_alpha: bool, animated: bool,
    has_icc: bool, has_exif: bool, has_xmp: bool,
    file_size: usize,
}

fn parse_webp(b: &[u8]) -> WebPMeta {
    let mut m = WebPMeta {
        subtype: "Unknown", width: 0, height: 0,
        has_alpha: false, animated: false,
        has_icc: false, has_exif: false, has_xmp: false,
        file_size: b.len(),
    };
    if b.len() < 12 { return m; }
    let mut pos = 12usize;
    while pos + 8 <= b.len() {
        let tag = &b[pos..pos + 4];
        let chunk_size = ru32le(b, pos + 4) as usize;
        let ds = pos + 8;
        if ds + chunk_size > b.len() { break; }
        let d = &b[ds..ds + chunk_size];
        match tag {
            b"VP8 " => { m.subtype = "VP8 (Lossy)"; }
            b"VP8L" if d.len() >= 5 && d[0] == 0x2F => {
                m.subtype = "VP8L (Lossless)";
                let bits = (d[1] as u32) | ((d[2] as u32) << 8) | ((d[3] as u32) << 16) | ((d[4] as u32) << 24);
                m.width  = (bits & 0x3FFF) + 1;
                m.height = ((bits >> 14) & 0x3FFF) + 1;
                m.has_alpha = (bits >> 28) & 1 == 1;
            }
            b"VP8X" if d.len() >= 10 => {
                m.subtype = "VP8X (Extended)";
                // VP8X flags byte 0 (LE): bits from LSB: reserved(2), ICC(1), alpha(1), exif(1), xmp(1), anim(1), reserved...
                let flags = d[0];
                m.has_icc   = (flags & 0x20) != 0;
                m.has_alpha = (flags & 0x10) != 0;
                m.has_exif  = (flags & 0x08) != 0;
                m.has_xmp   = (flags & 0x04) != 0;
                m.animated  = (flags & 0x02) != 0;
                m.width  = (d[4] as u32 | ((d[5] as u32) << 8) | ((d[6] as u32) << 16)) + 1;
                m.height = (d[7] as u32 | ((d[8] as u32) << 8) | ((d[9] as u32) << 16)) + 1;
            }
            b"ICCP" => { m.has_icc = true; }
            b"EXIF" => { m.has_exif = true; }
            b"XMP " => { m.has_xmp = true; }
            _ => {}
        }
        let advance = 8 + chunk_size + (chunk_size & 1);
        pos += advance;
    }
    m
}

fn dispatch_webp(action: &str, b: &[u8]) -> String {
    let m = parse_webp(b);
    let dims = if m.width > 0 {
        format!("{} × {}  ({})", m.width, m.height, aspect(m.width, m.height))
    } else {
        "unknown (VP8 bitstream not decoded)".to_string()
    };
    match action {
        "dimensions" => format!("Dimensions: {}\n", dims),
        "color" => format!("Encoding:   {}\nAlpha:      {}\n",
            m.subtype, if m.has_alpha { "Yes" } else { "No" }),
        "metadata" => {
            let mut out = String::new();
            if m.has_exif { out += "EXIF:        Present\n"; }
            if m.has_xmp  { out += "XMP:         Present\n"; }
            if m.has_icc  { out += "ICC Profile: Present\n"; }
            if out.is_empty() { out = "No metadata chunks found.\n".to_string(); }
            out
        }
        "validate" => {
            if m.subtype == "Unknown" { "ISSUES:\n  ✗ No VP8/VP8L/VP8X chunk found\n".to_string() }
            else { format!("VALID\n  ✓ {} chunk found\n", m.subtype) }
        }
        _ => {
            let mut out = format!("Format:     WebP ({})\nFile Size:  {}\n", m.subtype, human_size(m.file_size));
            out += &format!("Dimensions: {}\n", dims);
            out += &format!("Alpha:      {}\n", if m.has_alpha { "Yes" } else { "No" });
            out += &format!("Animated:   {}\n", if m.animated { "Yes" } else { "No" });
            if m.has_exif { out += "EXIF:       Present\n"; }
            if m.has_xmp  { out += "XMP:        Present\n"; }
            if m.has_icc  { out += "ICC Profile: Present\n"; }
            out
        }
    }
}

// ── BMP ───────────────────────────────────────────────────────────────────────

struct BmpMeta {
    file_size_header: u32,
    width: i32, height: i32,
    bpp: u16, planes: u16, compression: u32,
    x_ppm: i32, y_ppm: i32,
    colors_used: u32,
    header_size: u32,
    actual_size: usize,
}

fn parse_bmp(b: &[u8]) -> BmpMeta {
    let mut m = BmpMeta {
        file_size_header: 0, width: 0, height: 0,
        bpp: 0, planes: 1, compression: 0,
        x_ppm: 0, y_ppm: 0, colors_used: 0,
        header_size: 0, actual_size: b.len(),
    };
    if b.len() < 26 { return m; }
    m.file_size_header = ru32le(b, 2);
    // pixel_data_offset = ru32le(b, 10);
    m.header_size = ru32le(b, 14);
    if m.header_size >= 40 && b.len() >= 54 {
        m.width  = ri32le(b, 18);
        m.height = ri32le(b, 22);
        m.planes = ru16le(b, 26);
        m.bpp    = ru16le(b, 28);
        m.compression = ru32le(b, 30);
        m.x_ppm = ri32le(b, 38);
        m.y_ppm = ri32le(b, 42);
        m.colors_used = ru32le(b, 46);
    }
    m
}

fn bmp_compression(c: u32) -> &'static str {
    match c { 0 => "BI_RGB (uncompressed)", 1 => "BI_RLE8", 2 => "BI_RLE4",
        3 => "BI_BITFIELDS", 4 => "BI_JPEG", 5 => "BI_PNG", _ => "Unknown" }
}

fn dispatch_bmp(action: &str, b: &[u8]) -> String {
    let m = parse_bmp(b);
    let abs_h = m.height.unsigned_abs();
    let flip = if m.height < 0 { " (top-down)" } else { " (bottom-up)" };
    match action {
        "dimensions" => {
            let mut out = format!("Width:     {} px\nHeight:    {} px{}\nAspect:    {}\n",
                m.width, abs_h, flip, aspect(m.width.unsigned_abs(), abs_h));
            if m.x_ppm != 0 {
                let xdpi = m.x_ppm as f64 * 0.0254;
                let ydpi = m.y_ppm as f64 * 0.0254;
                if (xdpi - ydpi).abs() < 1.0 { out += &format!("Resolution: {:.0} DPI\n", xdpi); }
                else { out += &format!("Resolution: {:.0} × {:.0} DPI\n", xdpi, ydpi); }
            }
            out
        }
        "color" => format!("Bits/Pixel:  {} bpp\nPlanes:      {}\nPalette:     {} entries\nCompression: {}\n",
            m.bpp, m.planes, m.colors_used, bmp_compression(m.compression)),
        "metadata" => "BMP format does not embed text metadata.\n".to_string(),
        "validate" => {
            let mut issues = Vec::new();
            if m.header_size < 40 { issues.push(format!("Non-standard header size {}", m.header_size)); }
            if m.width == 0 || abs_h == 0 { issues.push("Zero dimensions".to_string()); }
            if issues.is_empty() { format!("VALID\n  ✓ BITMAPINFOHEADER, {}×{}\n", m.width, abs_h) }
            else { format!("ISSUES:\n{}\n", issues.iter().map(|s| format!("  ✗ {}", s)).collect::<Vec<_>>().join("\n")) }
        }
        _ => {
            let mut out = format!("Format:     BMP (Windows DIB)\nFile Size:  {}\n", human_size(m.actual_size));
            out += &format!("Dimensions: {} × {}{}\n", m.width, abs_h, flip);
            out += &format!("Aspect:     {}\n", aspect(m.width.unsigned_abs(), abs_h));
            out += &format!("Color:      {} bpp\n", m.bpp);
            if m.colors_used > 0 { out += &format!("Palette:    {} colors\n", m.colors_used); }
            out += &format!("Compression: {}\n", bmp_compression(m.compression));
            if m.x_ppm != 0 {
                let xdpi = m.x_ppm as f64 * 0.0254;
                let ydpi = m.y_ppm as f64 * 0.0254;
                if (xdpi - ydpi).abs() < 1.0 { out += &format!("Resolution: {:.0} DPI\n", xdpi); }
                else { out += &format!("Resolution: {:.0} × {:.0} DPI\n", xdpi, ydpi); }
            }
            out
        }
    }
}
