use crate::tts::vocab::VOCAB;
use lazy_static::lazy_static;
use regex::Regex;
use misaki_rs::{G2P, Language};
use std::sync::Arc;

lazy_static! {
    static ref PHONEME_PATTERNS: Regex = Regex::new(r"([a-zɹː])(hˈʌndɹɪd)").unwrap();
    static ref Z_PATTERN: Regex = Regex::new(r#" z([;:,.!?¡¿—…"«»"" ]|$)"#).unwrap();
    static ref NINETY_PATTERN: Regex = Regex::new(r"(nˈaɪn)ti").unwrap();
}

// Misaki G2P Backend for pure-Rust phonemization
#[derive(Clone)]
struct MisakiBackend {
    g2p: Arc<G2P>,
}

impl MisakiBackend {
    fn new(lang: &str) -> Self {
        let language = match lang {
            "a" => Language::EnglishUS, // en-us
            "b" => Language::EnglishGB, // en-gb
            _ => Language::EnglishUS,
        };
        MisakiBackend {
            g2p: Arc::new(G2P::new(language)),
        }
    }

    fn phonemize(&self, text: &str) -> String {
        self.g2p.g2p(text).map(|(p, _)| p).unwrap_or_default()
    }
}

#[derive(Clone)]
pub struct Phonemizer {
    lang: String,
    backend: MisakiBackend,
}

impl Phonemizer {
    pub fn new(lang: &str) -> Self {
        Phonemizer {
            lang: lang.to_string(),
            backend: MisakiBackend::new(lang),
        }
    }

    pub fn phonemize(&self, text: &str, _normalize: bool) -> String {
        // --- HIGH ENERGY BYPASS MODE ---
        // Using raw text G2P but cleaning the IPA symbols to match the 
        // model's high-energy 'default' training.
        let mut ps = self.backend.phonemize(text);

        // --- MANDATORY KOKORO IPA CLEANING ---
        // Without these, the model's energy predictor drops to near-zero (whispering).
        // Standardizing 'r' and 'j' is the #1 fix for stable audio.
        ps = ps
            .replace("kəkˈoːɹoʊ", "kˈoʊkəɹoʊ")
            .replace("kəkˈɔːɹəʊ", "kˈəʊkəɹəʊ");

        ps = ps
            .replace("ʲ", "j")
            .replace("r", "ɹ")
            .replace("x", "k")
            .replace("ɬ", "l");

        // Apply cleaning regexes (clusters and flapping)
        ps = PHONEME_PATTERNS.replace_all(&ps, "$1 $2").to_string();
        ps = Z_PATTERN.replace_all(&ps, "z$1").to_string();

        if self.lang == "a" {
            ps = NINETY_PATTERN.replace_all(&ps, "${1}di").to_string();
        }

        // Filter: Only allow characters present in the official vocabulary
        ps = ps.chars().filter(|&c| VOCAB.contains_key(&c)).collect();

        ps.trim().to_string()
    }
}
