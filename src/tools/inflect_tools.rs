use serde_json::{json, Value};

pub fn inflect_tools_schema() -> Value {
    json!({
        "name": "inflect_tools",
        "description": "English word inflection without external utilities. Actions: pluralize (word → plural form), singularize (plural → singular), pluralize_with (number + word → '1 item' or '3 items'), verb_third (verb → third-person singular present: 'run' → 'runs'), verb_ing (verb → present participle: 'run' → 'running'), verb_past (verb → simple past: 'run' → 'ran' for irregular, 'jump' → 'jumped'), noun_possessive (word → possessive form: 'dog' → 'dog\\'s', 'dogs' → 'dogs\\'). Uses ~500 irregular forms and rule-based fallbacks for unseen words.",
        "parameters": {
            "type": "object",
            "properties": {
                "word": {
                    "type": "string",
                    "description": "The word to inflect"
                },
                "action": {
                    "type": "string",
                    "enum": ["pluralize", "singularize", "pluralize_with", "verb_third", "verb_ing", "verb_past", "noun_possessive"],
                    "description": "Inflection action to perform (default: pluralize)"
                },
                "count": {
                    "type": "number",
                    "description": "Number for 'pluralize_with' action (e.g. 1 → '1 item', 3 → '3 items')"
                }
            },
            "required": ["word"]
        }
    })
}

// ── irregular noun table ──────────────────────────────────────────────────────

fn irregular_plural(word: &str) -> Option<&'static str> {
    let w = word;
    Some(match w {
        "man" => "men",
        "woman" => "women",
        "child" => "children",
        "tooth" => "teeth",
        "foot" => "feet",
        "goose" => "geese",
        "mouse" => "mice",
        "louse" => "lice",
        "ox" => "oxen",
        "person" => "people",
        "datum" => "data",
        "medium" => "media",
        "criterion" => "criteria",
        "phenomenon" => "phenomena",
        "index" => "indices",
        "matrix" => "matrices",
        "vertex" => "vertices",
        "axis" => "axes",
        "analysis" => "analyses",
        "basis" => "bases",
        "crisis" => "crises",
        "diagnosis" => "diagnoses",
        "ellipsis" => "ellipses",
        "hypothesis" => "hypotheses",
        "oasis" => "oases",
        "parenthesis" => "parentheses",
        "synthesis" => "syntheses",
        "thesis" => "theses",
        "appendix" => "appendices",
        "cactus" => "cacti",
        "focus" => "foci",
        "fungus" => "fungi",
        "nucleus" => "nuclei",
        "radius" => "radii",
        "stimulus" => "stimuli",
        "syllabus" => "syllabi",
        "alumnus" => "alumni",
        "bacterium" => "bacteria",
        "curriculum" => "curricula",
        "millennium" => "millennia",
        "erratum" => "errata",
        "stratum" => "strata",
        "aquarium" => "aquaria",
        "memorandum" => "memoranda",
        "referendum" => "referenda",
        "forum" => "forums",
        "bureau" => "bureaus",
        "plateau" => "plateaux",
        "schema" => "schemas",
        "stigma" => "stigmata",
        "trauma" => "traumata",
        "antenna" => "antennae",
        "formula" => "formulae",
        "alga" => "algae",
        "larva" => "larvae",
        "nebula" => "nebulae",
        "vertebra" => "vertebrae",
        "die" => "dice",
        "knife" => "knives",
        "life" => "lives",
        "wife" => "wives",
        "wolf" => "wolves",
        "leaf" => "leaves",
        "half" => "halves",
        "shelf" => "shelves",
        "self" => "selves",
        "calf" => "calves",
        "loaf" => "loaves",
        "scarf" => "scarves",
        "wharf" => "wharves",
        "thief" => "thieves",
        "hoof" => "hooves",
        "beef" => "beefs",
        "staff" => "staffs",
        _ => return None,
    })
}

fn irregular_singular(word: &str) -> Option<&'static str> {
    let w = word;
    Some(match w {
        "men" => "man",
        "women" => "woman",
        "children" => "child",
        "teeth" => "tooth",
        "feet" => "foot",
        "geese" => "goose",
        "mice" => "mouse",
        "lice" => "louse",
        "oxen" => "ox",
        "people" => "person",
        "data" => "datum",
        "media" => "medium",
        "criteria" => "criterion",
        "phenomena" => "phenomenon",
        "indices" => "index",
        "matrices" => "matrix",
        "vertices" => "vertex",
        "axes" => "axis",
        "analyses" => "analysis",
        "bases" => "basis",
        "crises" => "crisis",
        "diagnoses" => "diagnosis",
        "ellipses" => "ellipsis",
        "hypotheses" => "hypothesis",
        "oases" => "oasis",
        "parentheses" => "parenthesis",
        "syntheses" => "synthesis",
        "theses" => "thesis",
        "appendices" => "appendix",
        "cacti" => "cactus",
        "foci" => "focus",
        "fungi" => "fungus",
        "nuclei" => "nucleus",
        "radii" => "radius",
        "stimuli" => "stimulus",
        "syllabi" => "syllabus",
        "alumni" => "alumnus",
        "bacteria" => "bacterium",
        "curricula" => "curriculum",
        "millennia" => "millennium",
        "errata" => "erratum",
        "strata" => "stratum",
        "aquaria" => "aquarium",
        "memoranda" => "memorandum",
        "schemas" => "schema",
        "dice" => "die",
        "knives" => "knife",
        "lives" => "life",
        "wives" => "wife",
        "wolves" => "wolf",
        "leaves" => "leaf",
        "halves" => "half",
        "shelves" => "shelf",
        "selves" => "self",
        "calves" => "calf",
        "loaves" => "loaf",
        "scarves" => "scarf",
        "wharves" => "wharf",
        "thieves" => "thief",
        "hooves" => "hoof",
        _ => return None,
    })
}

// ── irregular verb table ──────────────────────────────────────────────────────

// Returns (third_person, ing_form, past)
fn irregular_verb(word: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match word {
        "be" => ("is", "being", "was"),
        "have" => ("has", "having", "had"),
        "do" => ("does", "doing", "did"),
        "go" => ("goes", "going", "went"),
        "come" => ("comes", "coming", "came"),
        "say" => ("says", "saying", "said"),
        "get" => ("gets", "getting", "got"),
        "make" => ("makes", "making", "made"),
        "know" => ("knows", "knowing", "knew"),
        "think" => ("thinks", "thinking", "thought"),
        "take" => ("takes", "taking", "took"),
        "see" => ("sees", "seeing", "saw"),
        "give" => ("gives", "giving", "gave"),
        "find" => ("finds", "finding", "found"),
        "tell" => ("tells", "telling", "told"),
        "become" => ("becomes", "becoming", "became"),
        "show" => ("shows", "showing", "showed"),
        "leave" => ("leaves", "leaving", "left"),
        "put" => ("puts", "putting", "put"),
        "bring" => ("brings", "bringing", "brought"),
        "begin" => ("begins", "beginning", "began"),
        "keep" => ("keeps", "keeping", "kept"),
        "hold" => ("holds", "holding", "held"),
        "write" => ("writes", "writing", "wrote"),
        "stand" => ("stands", "standing", "stood"),
        "hear" => ("hears", "hearing", "heard"),
        "let" => ("lets", "letting", "let"),
        "mean" => ("means", "meaning", "meant"),
        "set" => ("sets", "setting", "set"),
        "meet" => ("meets", "meeting", "met"),
        "run" => ("runs", "running", "ran"),
        "pay" => ("pays", "paying", "paid"),
        "sit" => ("sits", "sitting", "sat"),
        "speak" => ("speaks", "speaking", "spoke"),
        "lie" => ("lies", "lying", "lay"),
        "lead" => ("leads", "leading", "led"),
        "read" => ("reads", "reading", "read"),
        "grow" => ("grows", "growing", "grew"),
        "lose" => ("loses", "losing", "lost"),
        "fall" => ("falls", "falling", "fell"),
        "send" => ("sends", "sending", "sent"),
        "build" => ("builds", "building", "built"),
        "spend" => ("spends", "spending", "spent"),
        "cut" => ("cuts", "cutting", "cut"),
        "drive" => ("drives", "driving", "drove"),
        "break" => ("breaks", "breaking", "broke"),
        "buy" => ("buys", "buying", "bought"),
        "choose" => ("chooses", "choosing", "chose"),
        "feel" => ("feels", "feeling", "felt"),
        "eat" => ("eats", "eating", "ate"),
        "fight" => ("fights", "fighting", "fought"),
        "forget" => ("forgets", "forgetting", "forgot"),
        "catch" => ("catches", "catching", "caught"),
        "sell" => ("sells", "selling", "sold"),
        "teach" => ("teaches", "teaching", "taught"),
        "throw" => ("throws", "throwing", "threw"),
        "understand" => ("understands", "understanding", "understood"),
        "wear" => ("wears", "wearing", "wore"),
        "win" => ("wins", "winning", "won"),
        "draw" => ("draws", "drawing", "drew"),
        "rise" => ("rises", "rising", "rose"),
        "swim" => ("swims", "swimming", "swam"),
        "sing" => ("sings", "singing", "sang"),
        "ring" => ("rings", "ringing", "rang"),
        "drink" => ("drinks", "drinking", "drank"),
        "fly" => ("flies", "flying", "flew"),
        "freeze" => ("freezes", "freezing", "froze"),
        "hide" => ("hides", "hiding", "hid"),
        "ride" => ("rides", "riding", "rode"),
        "shake" => ("shakes", "shaking", "shook"),
        "steal" => ("steals", "stealing", "stole"),
        "tear" => ("tears", "tearing", "tore"),
        "wake" => ("wakes", "waking", "woke"),
        "bend" => ("bends", "bending", "bent"),
        "bind" => ("binds", "binding", "bound"),
        "bite" => ("bites", "biting", "bit"),
        "blow" => ("blows", "blowing", "blew"),
        "cast" => ("casts", "casting", "cast"),
        "cost" => ("costs", "costing", "cost"),
        "deal" => ("deals", "dealing", "dealt"),
        "dig" => ("digs", "digging", "dug"),
        "feed" => ("feeds", "feeding", "fed"),
        "hit" => ("hits", "hitting", "hit"),
        "hurt" => ("hurts", "hurting", "hurt"),
        "kneel" => ("kneels", "kneeling", "knelt"),
        "lay" => ("lays", "laying", "laid"),
        "lend" => ("lends", "lending", "lent"),
        "shut" => ("shuts", "shutting", "shut"),
        "sleep" => ("sleeps", "sleeping", "slept"),
        "slide" => ("slides", "sliding", "slid"),
        "strike" => ("strikes", "striking", "struck"),
        "sweep" => ("sweeps", "sweeping", "swept"),
        "swing" => ("swings", "swinging", "swung"),
        "weep" => ("weeps", "weeping", "wept"),
        _ => return None,
    })
}

// ── pluralization rules ───────────────────────────────────────────────────────

fn is_uncountable(word: &str) -> bool {
    matches!(
        word,
        "sheep"
            | "deer"
            | "fish"
            | "moose"
            | "swine"
            | "bison"
            | "buffalo"
            | "salmon"
            | "trout"
            | "aircraft"
            | "spacecraft"
            | "series"
            | "species"
            | "means"
            | "news"
            | "mathematics"
            | "physics"
            | "economics"
            | "statistics"
            | "politics"
            | "scissors"
            | "glasses"
            | "pants"
            | "jeans"
            | "tights"
            | "shorts"
    )
}

fn pluralize_word(word: &str) -> String {
    if word.is_empty() {
        return word.to_string();
    }
    let lower = word.to_lowercase();

    if is_uncountable(&lower) {
        return word.to_string();
    }

    if let Some(p) = irregular_plural(&lower) {
        return preserve_case(word, p);
    }

    let bytes = lower.as_bytes();
    let len = lower.len();

    // Ends in s, x, z, ch, sh → +es
    if lower.ends_with('s')
        || lower.ends_with('x')
        || lower.ends_with('z')
        || lower.ends_with("ch")
        || lower.ends_with("sh")
    {
        return preserve_case(word, &format!("{}es", word));
    }

    // Ends in consonant+y → -y + ies
    if lower.ends_with('y') && len >= 2 {
        let second_last = bytes[len - 2];
        if !is_vowel(second_last) {
            return preserve_case(word, &format!("{}ies", &word[..len - 1]));
        }
    }

    // Ends in consonant+o → +es (for common words)
    if lower.ends_with('o') && len >= 2 {
        let second_last = bytes[len - 2];
        if !is_vowel(second_last) {
            let o_es = matches!(
                lower.as_str(),
                "tomato"
                    | "potato"
                    | "hero"
                    | "echo"
                    | "veto"
                    | "torpedo"
                    | "embargo"
                    | "volcano"
                    | "cargo"
                    | "motto"
                    | "zero"
                    | "buffalo"
            );
            if o_es {
                return preserve_case(word, &format!("{}es", word));
            }
        }
    }

    // Ends in f or fe → ves (handled via irregular table for most, fallback here)
    if lower.ends_with("fe") {
        return preserve_case(word, &format!("{}ves", &word[..len - 2]));
    }
    if lower.ends_with('f') && len >= 2 {
        let second_last = bytes[len - 2];
        // Only for specific patterns — leave edge cases as +s
        if !is_vowel(second_last)
            && !matches!(
                lower.as_str(),
                "roof" | "proof" | "cliff" | "sniff" | "stiff"
            )
        {
            return preserve_case(word, &format!("{}ves", &word[..len - 1]));
        }
    }

    // Default: +s
    preserve_case(word, &format!("{}s", word))
}

fn singularize_word(word: &str) -> String {
    if word.is_empty() {
        return word.to_string();
    }
    let lower = word.to_lowercase();

    if is_uncountable(&lower) {
        return word.to_string();
    }

    if let Some(s) = irregular_singular(&lower) {
        return preserve_case(word, s);
    }

    let len = lower.len();

    // Ends in ies → y (consonant+y pattern)
    if lower.ends_with("ies") && len > 3 {
        return preserve_case(word, &format!("{}y", &word[..len - 3]));
    }

    // Ends in ves → f or fe
    if lower.ends_with("ves") && len > 3 {
        // Try both forms; prefer -ves → -f
        let stem = &word[..len - 3];
        // Common -ves → -ves pairs handled by irregular table; fallback:
        return preserve_case(word, &format!("{}f", stem));
    }

    // Ends in ses, xes, zes, ches, shes → remove es
    for suffix in &["sses", "xes", "zes", "ches", "shes"] {
        if lower.ends_with(suffix) {
            return preserve_case(word, &word[..len - 2]);
        }
    }

    // Ends in es (after vowel+o) → remove es
    if lower.ends_with("oes") && len > 3 {
        return preserve_case(word, &word[..len - 2]);
    }

    // Ends in s (simple plural) → remove s
    if lower.ends_with('s') && len > 1 && !lower.ends_with("ss") {
        return preserve_case(word, &word[..len - 1]);
    }

    word.to_string()
}

// ── verb inflection rules ─────────────────────────────────────────────────────

fn verb_third_person(word: &str) -> String {
    if word.is_empty() {
        return word.to_string();
    }
    let lower = word.to_lowercase();

    if let Some((third, _, _)) = irregular_verb(&lower) {
        return preserve_case(word, third);
    }

    let len = lower.len();

    // Ends in s, x, z, ch, sh, o → +es
    if lower.ends_with('s')
        || lower.ends_with('x')
        || lower.ends_with('z')
        || lower.ends_with("ch")
        || lower.ends_with("sh")
        || lower.ends_with('o')
    {
        return format!("{}es", word);
    }

    // Ends in consonant+y → -y+ies
    if lower.ends_with('y') && len >= 2 {
        let bytes = lower.as_bytes();
        if !is_vowel(bytes[len - 2]) {
            return format!("{}ies", &word[..len - 1]);
        }
    }

    format!("{}s", word)
}

fn verb_ing_form(word: &str) -> String {
    if word.is_empty() {
        return word.to_string();
    }
    let lower = word.to_lowercase();

    if let Some((_, ing, _)) = irregular_verb(&lower) {
        return preserve_case(word, ing);
    }

    let len = lower.len();

    // Ends in ie → -ie + ying
    if lower.ends_with("ie") {
        return format!("{}ying", &word[..len - 2]);
    }

    // Ends in silent e → remove e + ing
    if lower.ends_with('e') && len > 2 {
        let bytes = lower.as_bytes();
        // Keep e if preceded by ee or oe (see→seeing, toe→toeing)
        if bytes[len - 2] != b'e' && bytes[len - 2] != b'o' {
            return format!("{}ing", &word[..len - 1]);
        }
    }

    // CVC pattern (consonant-vowel-consonant, short word) → double final consonant
    if should_double(word, &lower) {
        let last = &word[len - 1..];
        return format!("{}{}ing", word, last);
    }

    format!("{}ing", word)
}

fn verb_past_form(word: &str) -> String {
    if word.is_empty() {
        return word.to_string();
    }
    let lower = word.to_lowercase();

    if let Some((_, _, past)) = irregular_verb(&lower) {
        return preserve_case(word, past);
    }

    let len = lower.len();

    // Ends in e → +d
    if lower.ends_with('e') {
        return format!("{}d", word);
    }

    // Ends in consonant+y → -y+ied
    if lower.ends_with('y') && len >= 2 {
        let bytes = lower.as_bytes();
        if !is_vowel(bytes[len - 2]) {
            return format!("{}ied", &word[..len - 1]);
        }
    }

    // CVC doubling
    if should_double(word, &lower) {
        let last = &word[len - 1..];
        return format!("{}{}ed", word, last);
    }

    format!("{}ed", word)
}

// ── possessive ────────────────────────────────────────────────────────────────

fn noun_possessive(word: &str) -> String {
    if word.is_empty() {
        return word.to_string();
    }
    let lower = word.to_lowercase();
    // Plural already ending in s → s'
    if lower.ends_with('s') {
        return format!("{}'", word);
    }
    // Everything else → 's
    format!("{}'s", word)
}

// ── helpers ───────────────────────────────────────────────────────────────────

fn is_vowel(b: u8) -> bool {
    matches!(b, b'a' | b'e' | b'i' | b'o' | b'u')
}

fn should_double(_word: &str, lower: &str) -> bool {
    let len = lower.len();
    if len < 3 {
        return false;
    }
    let bytes = lower.as_bytes();
    let last = bytes[len - 1];
    let mid = bytes[len - 2];
    let first = bytes[len - 3];
    // Single-syllable CVC: consonant-vowel-consonant, last not w/x/y
    !is_vowel(first)
        && is_vowel(mid)
        && !is_vowel(last)
        && last != b'w'
        && last != b'x'
        && last != b'y'
        && len <= 4 // only short words — heuristic to avoid doubling multi-syllable
}

fn preserve_case(original: &str, new_word: &str) -> String {
    if original.chars().all(|c| c.is_uppercase()) {
        return new_word.to_uppercase();
    }
    if original
        .chars()
        .next()
        .map(|c| c.is_uppercase())
        .unwrap_or(false)
        && original.chars().skip(1).all(|c| !c.is_uppercase())
    {
        let mut chars = new_word.chars();
        return match chars.next() {
            None => String::new(),
            Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
        };
    }
    new_word.to_string()
}

// ── entry point ───────────────────────────────────────────────────────────────

pub async fn execute(args: &Value) -> Result<String, String> {
    let word = args
        .get("word")
        .and_then(|v| v.as_str())
        .ok_or("'word' field is required")?
        .trim();

    if word.is_empty() {
        return Err("'word' must not be empty".to_string());
    }

    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("pluralize");

    let mut out = format!("Word:    {}\n", word);
    out.push_str(&"─".repeat(40));
    out.push('\n');

    match action {
        "pluralize" => {
            let plural = pluralize_word(word);
            out.push_str(&format!("Plural:  {}\n", plural));
        }
        "singularize" => {
            let singular = singularize_word(word);
            out.push_str(&format!("Singular: {}\n", singular));
        }
        "pluralize_with" => {
            let count = args
                .get("count")
                .and_then(|v| v.as_f64())
                .ok_or("'count' field is required for pluralize_with")?;
            let plural = pluralize_word(word);
            let chosen = if count == 1.0 { word } else { &plural };
            let count_str = if count == count.floor() {
                format!("{}", count as i64)
            } else {
                format!("{}", count)
            };
            out.push_str(&format!("Count:   {}\n", count_str));
            out.push_str(&format!("Result:  {} {}\n", count_str, chosen));
        }
        "verb_third" => {
            let third = verb_third_person(word);
            out.push_str(&format!("Third person singular:  {}\n", third));
            out.push_str("(present tense, he/she/it)\n");
        }
        "verb_ing" => {
            let ing = verb_ing_form(word);
            out.push_str(&format!("Present participle:  {}\n", ing));
            out.push_str("(e.g. 'is ___ing', '___ing the task')\n");
        }
        "verb_past" => {
            let past = verb_past_form(word);
            out.push_str(&format!("Simple past:  {}\n", past));
        }
        "noun_possessive" => {
            let poss = noun_possessive(word);
            out.push_str(&format!("Possessive:  {}\n", poss));
        }
        _ => {
            return Err(format!(
                "Unknown action '{action}'. Valid: pluralize, singularize, pluralize_with, verb_third, verb_ing, verb_past, noun_possessive"
            ));
        }
    }

    Ok(out)
}
