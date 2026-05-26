use rand::Rng;

pub async fn execute(args: &serde_json::Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("generate");

    match action {
        "generate" => generate(args),
        "passphrase" => passphrase(args),
        "strength" => strength(args),
        "pin" => pin(args),
        other => Err(format!(
            "password_gen: unknown action '{other}'. Valid: generate, passphrase, strength, pin"
        )),
    }
}

// ── Character sets ────────────────────────────────────────────────────────────

const LOWER: &[u8] = b"abcdefghijklmnopqrstuvwxyz";
const UPPER: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ";
const DIGITS: &[u8] = b"0123456789";
const SYMBOLS: &[u8] = b"!@#$%^&*()-_=+[]{}|;:,.<>?";
const AMBIGUOUS: &[u8] = b"0O1lI";

// Simple word list for passphrases (short, unambiguous, common English words)
const WORDS: &[&str] = &[
    "able", "acid", "aged", "also", "area", "army", "away", "baby", "back", "ball", "band", "bank",
    "base", "bath", "bear", "beat", "been", "bell", "best", "bird", "bite", "blue", "boat", "body",
    "bold", "bond", "bone", "book", "boom", "born", "both", "bowl", "bulk", "burn", "busy", "cake",
    "call", "calm", "came", "camp", "card", "care", "cart", "case", "cash", "cast", "cave", "cell",
    "chef", "chip", "city", "clam", "clan", "clay", "clip", "club", "clue", "coal", "coat", "code",
    "coin", "cold", "come", "cook", "cool", "copy", "core", "corn", "cost", "crew", "crop", "cube",
    "curl", "cute", "damp", "dark", "data", "date", "dawn", "days", "dead", "deal", "dear", "deep",
    "deny", "desk", "diet", "dirt", "disk", "dock", "done", "door", "down", "draw", "drop", "drum",
    "dual", "duel", "dusk", "dust", "duty", "each", "earl", "earn", "east", "easy", "edge", "else",
    "emit", "epic", "even", "ever", "exit", "face", "fact", "fail", "fair", "fall", "fame", "farm",
    "fast", "fear", "feed", "feel", "fell", "file", "fill", "film", "find", "fine", "fire", "firm",
    "fish", "fist", "flag", "flat", "flew", "flip", "flow", "foam", "fold", "folk", "fond", "font",
    "food", "fork", "form", "frog", "from", "fuel", "full", "fund", "fuse", "gain", "game", "gate",
    "gave", "gear", "gift", "give", "glad", "glow", "glue", "goal", "gold", "golf", "good", "grab",
    "gray", "grew", "grid", "grin", "grip", "grow", "gulf", "half", "hall", "hand", "hang", "hard",
    "harm", "hate", "have", "hawk", "head", "heap", "heat", "heel", "held", "help", "herb", "here",
    "hero", "high", "hill", "hint", "hold", "hole", "home", "hook", "hope", "host", "hour", "huge",
    "hull", "hung", "hunt", "hurt", "idea", "idle", "inch", "into", "iron", "item", "jade", "jail",
    "jump", "just", "keen", "keep", "kind", "king", "knew", "knot", "know", "lack", "lake", "lamp",
    "land", "lane", "last", "late", "lawn", "lead", "leaf", "lean", "leap", "left", "lend", "lens",
    "less", "lick", "life", "lift", "like", "lime", "line", "link", "lion", "list", "live", "load",
    "loan", "lock", "loft", "long", "look", "loop", "lose", "loss", "lost", "loud", "love", "luck",
    "lung", "made", "main", "make", "mall", "malt", "many", "mark", "mars", "mass", "mast", "math",
    "meal", "mean", "meet", "melt", "menu", "mild", "milk", "mill", "mind", "mine", "mint", "miss",
    "mist", "mode", "mold", "moon", "more", "most", "move", "much", "mule", "must", "name", "near",
    "neck", "need", "nest", "news", "next", "nice", "nine", "node", "none", "norm", "nose", "note",
    "noun", "odds", "once", "only", "open", "oval", "over", "pace", "pack", "page", "paid", "pain",
    "pair", "pale", "palm", "park", "part", "pass", "past", "path", "peak", "pear", "pick", "pile",
    "pine", "pink", "pipe", "plan", "play", "plot", "plow", "plug", "plus", "poem", "poet", "pole",
    "poll", "pond", "pool", "port", "pose", "post", "pour", "prey", "push", "quit", "race", "rack",
    "raft", "rage", "rail", "rain", "rank", "rate", "read", "real", "reel", "rent", "rest", "rice",
    "rich", "ride", "ring", "rise", "risk", "road", "roam", "rock", "role", "roll", "roof", "room",
    "rope", "rose", "ruin", "rule", "rush", "rust", "safe", "sage", "sail", "salt", "same", "sand",
    "save", "scan", "seal", "seed", "self", "sell", "send", "ship", "shop", "shot", "show", "shut",
    "sign", "silk", "sing", "sink", "site", "size", "skin", "skip", "slam", "slap", "slim", "slip",
    "slow", "snap", "snow", "soak", "soar", "soft", "soil", "sole", "some", "song", "soon", "sort",
    "soup", "span", "spin", "spot", "star", "stay", "stem", "step", "stop", "strap", "stub",
    "such", "suit", "sung", "swim", "tail", "take", "tale", "talk", "tall", "tank", "tape", "task",
    "team", "tear", "tell", "tent", "term", "test", "than", "that", "them", "then", "they", "thin",
    "this", "thus", "tide", "tied", "tile", "time", "tiny", "tire", "toad", "told", "toll", "tone",
    "tool", "toss", "tour", "town", "trap", "tree", "trim", "trip", "true", "tube", "tune", "turn",
    "type", "unit", "upon", "used", "user", "vary", "vast", "very", "vest", "view", "void", "volt",
    "vote", "wade", "wait", "walk", "wall", "ward", "warm", "warn", "warp", "wary", "wash", "wave",
    "weak", "wear", "weed", "week", "well", "went", "were", "west", "what", "when", "wide", "wife",
    "wild", "will", "wind", "wing", "wink", "wire", "wise", "wish", "with", "wolf", "wood", "word",
    "work", "worm", "worn", "wrap", "wren", "yard", "year", "your", "zero", "zone",
];

// ── Strength scoring ──────────────────────────────────────────────────────────

struct StrengthResult {
    score: u32, // 0-4
    label: &'static str,
    entropy_bits: f64,
    feedback: Vec<&'static str>,
}

fn score_password(pw: &str) -> StrengthResult {
    let len = pw.chars().count();
    let has_lower = pw.chars().any(|c| c.is_lowercase());
    let has_upper = pw.chars().any(|c| c.is_uppercase());
    let has_digit = pw.chars().any(|c| c.is_ascii_digit());
    let has_symbol = pw.chars().any(|c| !c.is_alphanumeric());

    let charset_size = {
        let mut n = 0u32;
        if has_lower {
            n += 26;
        }
        if has_upper {
            n += 26;
        }
        if has_digit {
            n += 10;
        }
        if has_symbol {
            n += 32;
        }
        n.max(1)
    };

    let entropy = (len as f64) * (charset_size as f64).log2();

    let mut feedback = Vec::new();
    if len < 8 {
        feedback.push("Too short (aim for 12+)");
    }
    if !has_lower {
        feedback.push("Add lowercase letters");
    }
    if !has_upper {
        feedback.push("Add uppercase letters");
    }
    if !has_digit {
        feedback.push("Add digits");
    }
    if !has_symbol {
        feedback.push("Add symbols for extra strength");
    }

    let score = if entropy < 28.0 {
        0
    } else if entropy < 36.0 {
        1
    } else if entropy < 60.0 {
        2
    } else if entropy < 100.0 {
        3
    } else {
        4
    };

    let label = match score {
        0 => "Very Weak",
        1 => "Weak",
        2 => "Fair",
        3 => "Strong",
        _ => "Very Strong",
    };

    StrengthResult {
        score,
        label,
        entropy_bits: entropy,
        feedback,
    }
}

// ── Actions ───────────────────────────────────────────────────────────────────

fn generate(args: &serde_json::Value) -> Result<String, String> {
    let length = args
        .get("length")
        .and_then(|v| v.as_u64())
        .unwrap_or(16)
        .clamp(4, 128) as usize;

    let use_upper = args.get("upper").and_then(|v| v.as_bool()).unwrap_or(true);
    let use_lower = args.get("lower").and_then(|v| v.as_bool()).unwrap_or(true);
    let use_digits = args.get("digits").and_then(|v| v.as_bool()).unwrap_or(true);
    let use_symbols = args
        .get("symbols")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    let no_ambiguous = args
        .get("no_ambiguous")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let count = args
        .get("count")
        .and_then(|v| v.as_u64())
        .unwrap_or(1)
        .min(20) as usize;

    let mut charset: Vec<u8> = Vec::new();
    if use_lower {
        charset.extend_from_slice(LOWER);
    }
    if use_upper {
        charset.extend_from_slice(UPPER);
    }
    if use_digits {
        charset.extend_from_slice(DIGITS);
    }
    if use_symbols {
        charset.extend_from_slice(SYMBOLS);
    }

    if no_ambiguous {
        charset.retain(|c| !AMBIGUOUS.contains(c));
    }

    if charset.is_empty() {
        return Err("password_gen: no character classes selected".into());
    }

    // Guarantee at least one of each required class
    let mut required: Vec<u8> = Vec::new();
    if use_lower {
        required.push(
            *LOWER
                .iter()
                .find(|&&c| !no_ambiguous || !AMBIGUOUS.contains(&c))
                .unwrap_or(&b'a'),
        );
    }
    if use_upper {
        required.push(
            *UPPER
                .iter()
                .find(|&&c| !no_ambiguous || !AMBIGUOUS.contains(&c))
                .unwrap_or(&b'A'),
        );
    }
    if use_digits {
        required.push(
            *DIGITS
                .iter()
                .find(|&&c| !no_ambiguous || !AMBIGUOUS.contains(&c))
                .unwrap_or(&b'2'),
        );
    }
    if use_symbols {
        required.push(SYMBOLS[0]);
    }

    let mut rng = rand::thread_rng();

    let mut out = format!("PASSWORD GENERATE\n{}\n", "─".repeat(50));
    out.push_str(&format!("Length   : {length}\n"));
    out.push_str(&format!(
        "Charset  : {}{}{}{}",
        if use_lower { "lowercase " } else { "" },
        if use_upper { "uppercase " } else { "" },
        if use_digits { "digits " } else { "" },
        if use_symbols { "symbols" } else { "" },
    ));
    out.push('\n');

    for i in 0..count {
        let mut pw: Vec<u8> = Vec::with_capacity(length);

        // Fill remaining
        let fill_count = length.saturating_sub(required.len());
        for _ in 0..fill_count {
            pw.push(charset[rng.gen_range(0..charset.len())]);
        }

        // Inject required characters
        for req in &required {
            if pw.len() < length {
                pw.push(*req);
            }
        }

        // Shuffle
        for j in (1..pw.len()).rev() {
            let k = rng.gen_range(0..=j);
            pw.swap(j, k);
        }

        let pw_str = String::from_utf8(pw).unwrap();
        let s = score_password(&pw_str);

        if count == 1 {
            out.push_str(&format!("\nPassword : {pw_str}\n"));
            out.push_str(&format!("Strength : {} (score {}/4)\n", s.label, s.score));
            out.push_str(&format!("Entropy  : {:.0} bits\n", s.entropy_bits));
        } else {
            out.push_str(&format!("  {:2}. {pw_str}\n", i + 1));
        }
    }
    Ok(out)
}

fn passphrase(args: &serde_json::Value) -> Result<String, String> {
    let words_count = args
        .get("words")
        .and_then(|v| v.as_u64())
        .unwrap_or(4)
        .clamp(3, 12) as usize;

    let separator = args
        .get("separator")
        .and_then(|v| v.as_str())
        .unwrap_or("-");

    let capitalize = args
        .get("capitalize")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let add_number = args.get("number").and_then(|v| v.as_bool()).unwrap_or(true);

    let count = args
        .get("count")
        .and_then(|v| v.as_u64())
        .unwrap_or(1)
        .min(10) as usize;

    let mut rng = rand::thread_rng();

    let mut out = format!("PASSPHRASE GENERATE\n{}\n", "─".repeat(50));
    out.push_str(&format!("Words    : {words_count}\n"));

    for i in 0..count {
        let mut chosen: Vec<String> = (0..words_count)
            .map(|_| {
                let word = WORDS[rng.gen_range(0..WORDS.len())];
                if capitalize {
                    let mut c = word.chars();
                    match c.next() {
                        None => String::new(),
                        Some(f) => f.to_uppercase().to_string() + c.as_str(),
                    }
                } else {
                    word.to_string()
                }
            })
            .collect();

        if add_number {
            chosen.push(rng.gen_range(10u32..99).to_string());
        }

        let phrase = chosen.join(separator);
        let s = score_password(&phrase);

        if count == 1 {
            out.push_str(&format!("\nPassphrase : {phrase}\n"));
            out.push_str(&format!(
                "Strength   : {} ({:.0} bits entropy)\n",
                s.label, s.entropy_bits
            ));
        } else {
            out.push_str(&format!("  {:2}. {phrase}\n", i + 1));
        }
    }
    Ok(out)
}

fn strength(args: &serde_json::Value) -> Result<String, String> {
    let input = args
        .get("input")
        .and_then(|v| v.as_str())
        .ok_or("password_gen strength: 'input' is required")?;

    let s = score_password(input);
    let bar_filled = s.score as usize + 1;
    let bar = format!("[{}{}]", "█".repeat(bar_filled), "░".repeat(5 - bar_filled));

    let mut out = format!("PASSWORD STRENGTH\n{}\n", "─".repeat(50));
    out.push_str(&format!(
        "Input    : {}\n",
        "*".repeat(input.chars().count().min(20))
    ));
    out.push_str(&format!(
        "Length   : {} characters\n",
        input.chars().count()
    ));
    out.push_str(&format!("Strength : {} {bar} {}/4\n", s.label, s.score));
    out.push_str(&format!("Entropy  : {:.1} bits\n", s.entropy_bits));

    let has_lower = input.chars().any(|c| c.is_lowercase());
    let has_upper = input.chars().any(|c| c.is_uppercase());
    let has_digit = input.chars().any(|c| c.is_ascii_digit());
    let has_symbol = input.chars().any(|c| !c.is_alphanumeric());
    out.push_str(&format!(
        "Lowercase: {}\n",
        if has_lower { "✓" } else { "✗" }
    ));
    out.push_str(&format!(
        "Uppercase: {}\n",
        if has_upper { "✓" } else { "✗" }
    ));
    out.push_str(&format!(
        "Digits   : {}\n",
        if has_digit { "✓" } else { "✗" }
    ));
    out.push_str(&format!(
        "Symbols  : {}\n",
        if has_symbol { "✓" } else { "✗" }
    ));

    if !s.feedback.is_empty() {
        out.push_str("\nSuggestions:\n");
        for tip in &s.feedback {
            out.push_str(&format!("  • {tip}\n"));
        }
    }
    Ok(out)
}

fn pin(args: &serde_json::Value) -> Result<String, String> {
    let length = args
        .get("length")
        .and_then(|v| v.as_u64())
        .unwrap_or(6)
        .clamp(4, 12) as usize;

    let count = args
        .get("count")
        .and_then(|v| v.as_u64())
        .unwrap_or(1)
        .min(10) as usize;

    let mut rng = rand::thread_rng();
    let mut out = format!("PIN GENERATE\n{}\n", "─".repeat(50));
    out.push_str(&format!("Length : {length} digits\n\n"));
    for i in 0..count {
        let pin: String = (0..length)
            .map(|_| (b'0' + rng.gen_range(0..10)) as char)
            .collect();
        out.push_str(&format!("  {:2}. {pin}\n", i + 1));
    }
    Ok(out)
}
