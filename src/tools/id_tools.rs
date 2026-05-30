use serde_json::Value;
use std::time::{SystemTime, UNIX_EPOCH};

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("ulid");
    match action {
        "ulid" => action_ulid(args),
        "nanoid" => action_nanoid(args),
        "snowflake" => action_snowflake(args),
        "decode" => action_decode(args),
        other => Err(format!(
            "Unknown action '{other}'. Use: ulid, nanoid, snowflake, decode"
        )),
    }
}

// ── Seeded LCG ────────────────────────────────────────────────────────────────

struct Lcg(u64);
impl Lcg {
    fn new(seed: u64) -> Self {
        Lcg(seed ^ 0xdeadbeef_cafebabe)
    }
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0
    }
    fn byte(&mut self) -> u8 {
        (self.next() >> 33) as u8
    }
}

fn get_count(args: &Value) -> usize {
    args.get("count")
        .or_else(|| args.get("n"))
        .and_then(|v| v.as_u64())
        .unwrap_or(1) as usize
}

fn get_seed(args: &Value) -> u64 {
    match args.get("seed").and_then(|v| v.as_u64()) {
        Some(s) => s,
        None => {
            // Use system time as entropy
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_nanos() as u64)
                .unwrap_or(42)
        }
    }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

// ── ULID ──────────────────────────────────────────────────────────────────────
// Format: 10 chars time (Crockford base32) + 16 chars random = 26 chars
// Monotonicity: sequential ULIDs share the same timestamp component

const CROCKFORD: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

fn encode_crockford(mut n: u128, len: usize) -> String {
    let mut chars = vec![b'0'; len];
    for i in (0..len).rev() {
        chars[i] = CROCKFORD[(n & 0x1F) as usize];
        n >>= 5;
    }
    String::from_utf8(chars).unwrap()
}

fn decode_crockford(s: &str) -> Result<u128, String> {
    let mut v: u128 = 0;
    for c in s.chars() {
        let c = c.to_ascii_uppercase();
        // Crockford mappings: I/L → 1, O → 0
        let c = match c {
            'I' | 'L' => '1',
            'O' => '0',
            other => other,
        };
        let idx = CROCKFORD
            .iter()
            .position(|&b| b == c as u8)
            .ok_or_else(|| format!("Invalid Crockford character '{c}' in ULID"))?;
        v = (v << 5) | idx as u128;
    }
    Ok(v)
}

fn generate_ulid(ts_ms: u64, lcg: &mut Lcg) -> String {
    // 48-bit timestamp in top bits
    let ts_part: u128 = (ts_ms as u128) << 80;
    // 80 bits random
    let rand_bytes: u128 = (0..10).fold(0u128, |acc, _| (acc << 8) | lcg.byte() as u128);
    let val = ts_part | rand_bytes;
    encode_crockford(val, 26)
}

fn action_ulid(args: &Value) -> Result<String, String> {
    let count = get_count(args).clamp(1, 100);
    let mut lcg = Lcg::new(get_seed(args));
    let ts_ms = now_ms();

    let mut out = String::from("id_tools — ulid\n\n");
    out.push_str("ULID (Universally Unique Lexicographically Sortable Identifier)\n");
    out.push_str("Format: TTTTTTTTTT RRRRRRRRRRRRRRRR (10 time + 16 random, Crockford base32)\n\n");
    for i in 0..count {
        // Increment least significant random byte for monotonic ordering in same ms
        let ts = ts_ms.wrapping_add(i as u64 / 1000);
        out.push_str(&generate_ulid(ts, &mut lcg));
        out.push('\n');
    }
    Ok(out)
}

// ── NanoID ────────────────────────────────────────────────────────────────────
// URL-safe base62 (default) or custom alphabet

const NANOID_DEFAULT: &[u8; 64] =
    b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz_-";

fn action_nanoid(args: &Value) -> Result<String, String> {
    let count = get_count(args).clamp(1, 100);
    let size = args.get("size").and_then(|v| v.as_u64()).unwrap_or(21) as usize;
    if size == 0 || size > 256 {
        return Err("'size' must be between 1 and 256".to_string());
    }

    let alphabet: Vec<u8> = if let Some(a) = args.get("alphabet").and_then(|v| v.as_str()) {
        if a.is_empty() || a.len() > 256 {
            return Err("'alphabet' must be 1–256 characters".to_string());
        }
        a.bytes().collect()
    } else {
        NANOID_DEFAULT.to_vec()
    };

    let mask = alphabet.len().next_power_of_two() - 1;
    let mut lcg = Lcg::new(get_seed(args));

    let mut out = String::from("id_tools — nanoid\n\n");
    out.push_str(&format!(
        "Size: {size}  |  Alphabet: {} chars\n\n",
        alphabet.len()
    ));

    for _ in 0..count {
        let mut id = String::with_capacity(size);
        while id.len() < size {
            let idx = (lcg.byte() as usize) & mask;
            if idx < alphabet.len() {
                id.push(alphabet[idx] as char);
            }
        }
        out.push_str(&id);
        out.push('\n');
    }
    Ok(out)
}

// ── Snowflake ID ──────────────────────────────────────────────────────────────
// Twitter/Discord style: 41-bit ms timestamp + 10-bit machine id + 12-bit sequence
// Custom epoch: 2024-01-01 00:00:00 UTC = 1704067200000 ms

const SNOWFLAKE_EPOCH: u64 = 1704067200000;

fn action_snowflake(args: &Value) -> Result<String, String> {
    let count = get_count(args).clamp(1, 100);
    let machine_id = args.get("machine_id").and_then(|v| v.as_u64()).unwrap_or(1) & 0x3FF;
    let ts_ms = now_ms();
    let mut seq: u64 = 0;

    let mut out = String::from("id_tools — snowflake\n\n");
    out.push_str(
        "Snowflake ID (Twitter-style: 41-bit timestamp + 10-bit machine + 12-bit sequence)\n",
    );
    out.push_str(&format!(
        "Epoch: 2024-01-01T00:00:00Z  |  Machine ID: {machine_id}\n\n"
    ));

    for _ in 0..count {
        let elapsed = ts_ms.saturating_sub(SNOWFLAKE_EPOCH);
        let id: u64 = ((elapsed & 0x1FFFFFFFFFF) << 22) | (machine_id << 12) | (seq & 0xFFF);
        out.push_str(&format!("{id}\n"));
        seq += 1;
    }
    Ok(out)
}

// ── Decode ────────────────────────────────────────────────────────────────────

fn action_decode(args: &Value) -> Result<String, String> {
    let id_str = args
        .get("id")
        .or_else(|| args.get("input"))
        .and_then(|v| v.as_str())
        .ok_or("Missing 'id' to decode")?
        .trim();

    let mut out = String::from("id_tools — decode\n\n");
    out.push_str(&format!("Input: {id_str}\n\n"));

    // Try ULID (26 uppercase alphanumeric Crockford chars)
    if id_str.len() == 26
        && id_str
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "ILOUILOUILOU".contains(c))
    {
        if let Ok(v) = decode_crockford(id_str) {
            let ts_ms = (v >> 80) as u64;
            let rand_part = v & ((1u128 << 80) - 1);
            let secs = ts_ms / 1000;
            let ms = ts_ms % 1000;
            out.push_str("Format: ULID\n");
            out.push_str(&format!("Timestamp ms: {ts_ms}\n"));
            out.push_str(&format!("Timestamp:    {secs}.{ms:03}s since Unix epoch\n"));
            out.push_str(&format!("Random part:  0x{rand_part:020X}\n"));
            return Ok(out);
        }
    }

    // Try Snowflake (numeric, 17-19 digits)
    if id_str.chars().all(|c| c.is_ascii_digit()) && id_str.len() >= 17 {
        if let Ok(n) = id_str.parse::<u64>() {
            let elapsed = (n >> 22) + SNOWFLAKE_EPOCH;
            let machine = (n >> 12) & 0x3FF;
            let seq = n & 0xFFF;
            let secs = elapsed / 1000;
            let ms = elapsed % 1000;
            out.push_str("Format: Snowflake (Discord/Twitter-style)\n");
            out.push_str(&format!("Timestamp ms: {} (epoch 2024-01-01)\n", elapsed));
            out.push_str(&format!("Timestamp:    {secs}.{ms:03}s since Unix epoch\n"));
            out.push_str(&format!("Machine ID:   {machine}\n"));
            out.push_str(&format!("Sequence:     {seq}\n"));
            return Ok(out);
        }
    }

    out.push_str("Format: unknown (not a recognized ULID or Snowflake)\n");
    out.push_str(
        "Tip: pass a 26-character Crockford base32 ULID or a large numeric Snowflake ID\n",
    );
    Ok(out)
}
