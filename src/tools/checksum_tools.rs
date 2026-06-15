use serde_json::Value;

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args.get("action").and_then(|v| v.as_str()).unwrap_or("all");
    match action {
        "crc8" => action_crc(args, 8),
        "crc16" => action_crc(args, 16),
        "crc32" => action_crc(args, 32),
        "adler32" => action_adler32(args),
        "fletcher16" => action_fletcher16(args),
        "all" => action_all(args),
        other => Err(format!(
            "Unknown action '{other}'. Use: crc8, crc16, crc32, adler32, fletcher16, all"
        )),
    }
}

fn get_bytes(args: &Value) -> Result<Vec<u8>, String> {
    if let Some(t) = args
        .get("text")
        .or_else(|| args.get("input"))
        .and_then(|v| v.as_str())
    {
        return Ok(t.as_bytes().to_vec());
    }
    if let Some(h) = args.get("hex").and_then(|v| v.as_str()) {
        let cleaned: String = h.chars().filter(|c| c.is_ascii_hexdigit()).collect();
        if !cleaned.len().is_multiple_of(2) {
            return Err("Hex string must have an even number of digits".to_string());
        }
        let bytes = (0..cleaned.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&cleaned[i..i + 2], 16).unwrap())
            .collect();
        return Ok(bytes);
    }
    Err("Missing 'text' (string) or 'hex' (hex-encoded bytes)".to_string())
}

// CRC-8 (polynomial 0x07, no pre/post inversion)
fn crc8(data: &[u8]) -> u8 {
    let mut crc: u8 = 0;
    for &byte in data {
        crc ^= byte;
        for _ in 0..8 {
            if crc & 0x80 != 0 {
                crc = (crc << 1) ^ 0x07;
            } else {
                crc <<= 1;
            }
        }
    }
    crc
}

// CRC-16/MODBUS (polynomial 0x8005, init 0xFFFF, reflected)
fn crc16(data: &[u8]) -> u16 {
    let mut crc: u16 = 0xFFFF;
    for &byte in data {
        crc ^= byte as u16;
        for _ in 0..8 {
            if crc & 1 != 0 {
                crc = (crc >> 1) ^ 0xA001;
            } else {
                crc >>= 1;
            }
        }
    }
    crc
}

// CRC-32 (IEEE 802.3, polynomial 0x04C11DB7, reflected input/output, pre/post XOR 0xFFFFFFFF)
fn crc32(data: &[u8]) -> u32 {
    let mut crc: u32 = 0xFFFFFFFF;
    for &byte in data {
        crc ^= byte as u32;
        for _ in 0..8 {
            if crc & 1 != 0 {
                crc = (crc >> 1) ^ 0xEDB88320;
            } else {
                crc >>= 1;
            }
        }
    }
    crc ^ 0xFFFFFFFF
}

fn adler32(data: &[u8]) -> u32 {
    const MOD: u32 = 65521;
    let mut a: u32 = 1;
    let mut b: u32 = 0;
    for &byte in data {
        a = (a + byte as u32) % MOD;
        b = (b + a) % MOD;
    }
    (b << 16) | a
}

fn fletcher16(data: &[u8]) -> u16 {
    let mut sum1: u16 = 0;
    let mut sum2: u16 = 0;
    for &byte in data {
        sum1 = (sum1 + byte as u16) % 255;
        sum2 = (sum2 + sum1) % 255;
    }
    (sum2 << 8) | sum1
}

fn action_crc(args: &Value, bits: u8) -> Result<String, String> {
    let data = get_bytes(args)?;
    let name = match bits {
        8 => "CRC-8",
        16 => "CRC-16/MODBUS",
        32 => "CRC-32 (IEEE)",
        _ => unreachable!(),
    };
    let (hex, decimal, binary) = match bits {
        8 => {
            let v = crc8(&data);
            (format!("0x{:02X}", v), v as u64, format!("{:08b}", v))
        }
        16 => {
            let v = crc16(&data);
            (format!("0x{:04X}", v), v as u64, format!("{:016b}", v))
        }
        32 => {
            let v = crc32(&data);
            (format!("0x{:08X}", v), v as u64, format!("{:032b}", v))
        }
        _ => unreachable!(),
    };
    let mut out = format!("checksum_tools — {name}\n\n");
    out.push_str(&format!("Input:   {} bytes\n", data.len()));
    out.push_str(&format!("Hex:     {hex}\n"));
    out.push_str(&format!("Decimal: {decimal}\n"));
    out.push_str(&format!("Binary:  {binary}\n"));
    Ok(out)
}

fn action_adler32(args: &Value) -> Result<String, String> {
    let data = get_bytes(args)?;
    let v = adler32(&data);
    let mut out = String::from("checksum_tools — Adler-32\n\n");
    out.push_str(&format!("Input:   {} bytes\n", data.len()));
    out.push_str(&format!("Hex:     0x{:08X}\n", v));
    out.push_str(&format!("Decimal: {v}\n"));
    out.push_str(&format!("A part:  0x{:04X}\n", v & 0xFFFF));
    out.push_str(&format!("B part:  0x{:04X}\n", (v >> 16) & 0xFFFF));
    Ok(out)
}

fn action_fletcher16(args: &Value) -> Result<String, String> {
    let data = get_bytes(args)?;
    let v = fletcher16(&data);
    let mut out = String::from("checksum_tools — Fletcher-16\n\n");
    out.push_str(&format!("Input:   {} bytes\n", data.len()));
    out.push_str(&format!("Hex:     0x{:04X}\n", v));
    out.push_str(&format!("Decimal: {v}\n"));
    Ok(out)
}

fn action_all(args: &Value) -> Result<String, String> {
    let data = get_bytes(args)?;
    let mut out = String::from("checksum_tools — all\n\n");
    out.push_str(&format!("Input: {} bytes\n\n", data.len()));
    out.push_str(&format!(
        "CRC-8:        0x{:02X}  ({})\n",
        crc8(&data),
        crc8(&data)
    ));
    let c16 = crc16(&data);
    out.push_str(&format!("CRC-16/MODBUS: 0x{:04X}  ({})\n", c16, c16));
    let c32 = crc32(&data);
    out.push_str(&format!("CRC-32 (IEEE): 0x{:08X}  ({})\n", c32, c32));
    let a32 = adler32(&data);
    out.push_str(&format!("Adler-32:      0x{:08X}  ({})\n", a32, a32));
    let f16 = fletcher16(&data);
    out.push_str(&format!("Fletcher-16:   0x{:04X}  ({})\n", f16, f16));
    Ok(out)
}
