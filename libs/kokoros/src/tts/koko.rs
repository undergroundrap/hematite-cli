use std::io::{Cursor};
use crate::onn::ort_base::OrtBase;
use crate::onn::ort_koko::OrtKoko;
use crate::tts::phonemizer::Phonemizer;
use crate::tts::tokenize::tokenize;
use ndarray_npy::NpzReader;
use std::collections::HashMap;
use std::error::Error;
use std::fs::File;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

static VOICES_LOGGED: AtomicBool = AtomicBool::new(false);

#[derive(Debug, Clone)]
pub struct WordAlignment {
    pub word: String,
    pub start_sec: f32,
    pub end_sec: f32,
}

#[derive(Debug, Clone)]
pub enum ModelStrategy {
    Koko,
}

impl ModelStrategy {
    pub fn audio_key(&self) -> &str {
        match self {
            ModelStrategy::Koko => "audio",
        }
    }
}

pub struct TTSKoko {
    onn: Arc<Mutex<OrtKoko>>,
    styles: HashMap<String, Vec<f32>>,
    strategy: ModelStrategy,
    phonemizer: Phonemizer,
}

impl TTSKoko {
    pub fn new(model_path: &str, voices_path: &str) -> Result<Self, Box<dyn Error>> {
        let mut ort = OrtKoko::new();
        ort.load_model(model_path.to_string())?;

        let styles = Self::load_voices(voices_path)?;
        
        // Default to English US phonemizer
        let phonemizer = Phonemizer::new("a");

        if !VOICES_LOGGED.load(Ordering::SeqCst) {
            let keys: Vec<_> = styles.keys().collect();
            tracing::info!("Loaded styles: {:?}", keys);
            VOICES_LOGGED.store(true, Ordering::SeqCst);
        }

        Ok(Self {
            onn: Arc::new(Mutex::new(ort)),
            styles,
            strategy: ModelStrategy::Koko,
            phonemizer,
        })
    }

    pub fn new_from_memory(model_bytes: &[u8], voices_bytes: &[u8]) -> Result<Self, Box<dyn Error>> {
        let mut ort = OrtKoko::new();
        ort.load_model_from_memory(model_bytes)?;

        let styles = Self::load_voices_from_memory(voices_bytes)?;
        
        let phonemizer = Phonemizer::new("a");

        if !VOICES_LOGGED.load(Ordering::SeqCst) {
            let keys: Vec<_> = styles.keys().collect();
            tracing::info!("Loaded (baked) styles: {:?}", keys);
            VOICES_LOGGED.store(true, Ordering::SeqCst);
        }

        Ok(Self {
            onn: Arc::new(Mutex::new(ort)),
            styles,
            strategy: ModelStrategy::Koko,
            phonemizer,
        })
    }

    pub fn generate_full(
        &self,
        text: &str,
        voice: &str,
        speed: f32,
    ) -> Result<(Vec<f32>, Vec<WordAlignment>), Box<dyn Error>> {
        let input_text = text.trim();
        if input_text.is_empty() {
            return Ok((vec![], vec![]));
        }

        // --- SOVEREIGN CHUNKING: 80-TOKEN MICRO-RESET ---
        // Kokoro v1.0 energy is highest with tiny contexts. We split into 80-token 
        // clusters (~12 words) to ensure 100% vocalization throughout.
        let mut full_audio = Vec::new();
        let mut full_alignments = Vec::new();

        // 1. Sentence-Aware Splitter (80-token limit)
        let mut text_chunks = Vec::new();
        let sentences: Vec<_> = input_text.split_inclusive(|c| c == '.' || c == '!' || c == '?' || c == '\n' || c == ';' || c == ',').collect();
        
        let mut current_chunk = String::new();
        for s in sentences {
            let next_text = format!("{}{}", current_chunk, s);
            if tokenize(&self.phonemizer.phonemize(&next_text, true)).len() > 80 {
                if !current_chunk.is_empty() { text_chunks.push(current_chunk.clone()); current_chunk.clear(); }
                // Fallback: word split
                let words: Vec<_> = s.split_whitespace().collect();
                let mut temp_text = String::new();
                for w in words {
                    let next_temp = format!("{} {}", temp_text, w);
                    if tokenize(&self.phonemizer.phonemize(&next_temp, true)).len() > 80 {
                        text_chunks.push(temp_text.trim().to_string()); temp_text = w.to_string();
                    } else { temp_text = next_temp; }
                }
                current_chunk = temp_text;
            } else { current_chunk = next_text; }
        }
        if !current_chunk.is_empty() { text_chunks.push(current_chunk); }

        let mut current_offset = 0.0;
        for chunk_text in text_chunks {
            let mut phonemes = self.phonemizer.phonemize(&chunk_text, true);
            
            // --- FORCED VOCALIZATION (CONVICTION) ---
            // If it doesn't end in strong punctuation, add a '.' to force vocalization vs whispering.
            if !phonemes.ends_with('.') && !phonemes.ends_with('!') && !phonemes.ends_with('?') {
                phonemes.push('.');
            }
            
            let tokens = tokenize(&phonemes);
            if tokens.is_empty() { continue; }

            let style = self.styles.get(voice).ok_or("Voice style not found")?;
            let tokens_batch = vec![tokens.iter().map(|&t| t as i64).collect::<Vec<i64>>()];
            
            let mut onn = self.onn.lock().unwrap();
            let audio_data = onn.infer(tokens_batch, style, speed, &self.strategy)?;
            drop(onn);

            // --- CHUNK PROCESSING: TRIM & NORMALIZE ---
            
            // 1. Aggressive Silence Trimmer (to remove the forced '.' pause)
            let start = audio_data.iter().position(|&s| s.abs() > 0.005).unwrap_or(0);
            let end = audio_data.iter().rposition(|&s| s.abs() > 0.005).unwrap_or(audio_data.len());
            let trimmed = &audio_data[start..end];

            // 2. Local Normalization
            let sq_sum: f32 = trimmed.iter().map(|&s| s * s).sum();
            let rms = (sq_sum / trimmed.len().max(1) as f32).sqrt();
            let gain = if rms > 0.001 { (0.15 / rms).min(100.0) } else { 1.0 };
            
            let mut chunk_audio: Vec<f32> = trimmed.iter().map(|&s| s * gain).collect();

            // 3. Linear Crossfade (10ms)
            let fade_len = 240; 
            if chunk_audio.len() > fade_len * 2 {
                for i in 0..fade_len {
                    let alpha = i as f32 / fade_len as f32;
                    chunk_audio[i] *= alpha;
                    let end_idx = chunk_audio.len() - 1 - i;
                    chunk_audio[end_idx] *= alpha;
                }
            }

            // 4. Alignments
            let chunk_words: Vec<_> = chunk_text.split_whitespace().collect();
            let chunk_dur = chunk_audio.len() as f32 / 24000.0;
            let word_dur = chunk_dur / chunk_words.len().max(1) as f32;
            for (i, word) in chunk_words.iter().enumerate() {
                full_alignments.push(WordAlignment {
                    word: word.to_string(),
                    start_sec: current_offset + (i as f32 * word_dur),
                    end_sec: current_offset + ((i + 1) as f32 * word_dur),
                });
            }

            full_audio.extend(chunk_audio);
            current_offset += chunk_dur;
        }

        tracing::info!("Sovereign Engine: Multi-Pass Vocalization Reset Success.");
        Ok((full_audio, full_alignments))
    }

    /// True Streaming: Yields high-energy audio chunks as soon as they are inferred.
    #[allow(clippy::too_many_arguments)]
    pub fn tts_raw_audio_streaming<F>(
        &self,
        text: &str,
        _lang: &str,
        voice: &str,
        speed: f32,
        _param1: Option<()>,
        _param2: Option<()>,
        _param3: Option<()>,
        _param4: Option<()>,
        mut callback: F,
    ) -> Result<(), Box<dyn Error>>
    where
        F: FnMut(Vec<f32>) -> Result<(), Box<dyn Error>>,
    {
        let input_text = text.trim();
        if input_text.is_empty() { return Ok(()); }

        let mut text_chunks = Vec::new();
        let sentences: Vec<_> = input_text.split_inclusive(|c| c == '.' || c == '!' || c == '?' || c == '\n' || c == ';' || c == ',').collect();
        let mut current_chunk = String::new();
        for s in sentences {
            let next_text = format!("{}{}", current_chunk, s);
            if tokenize(&self.phonemizer.phonemize(&next_text, true)).len() > 80 {
                if !current_chunk.is_empty() { text_chunks.push(current_chunk.clone()); current_chunk.clear(); }
                let words: Vec<_> = s.split_whitespace().collect();
                let mut temp_text = String::new();
                for w in words {
                    let next_temp = format!("{} {}", temp_text, w);
                    if tokenize(&self.phonemizer.phonemize(&next_temp, true)).len() > 80 {
                        text_chunks.push(temp_text.trim().to_string()); temp_text = w.to_string();
                    } else { temp_text = next_temp; }
                }
                current_chunk = temp_text;
            } else { current_chunk = next_text; }
        }
        if !current_chunk.is_empty() { text_chunks.push(current_chunk); }

        for chunk_text in text_chunks {
            let mut ph = self.phonemizer.phonemize(&chunk_text, true);
            // FORCED VOCALIZATION
            if !ph.ends_with('.') && !ph.ends_with('!') && !ph.ends_with('?') {
                ph.push('.');
            }
            
            let tok = tokenize(&ph);
            if tok.is_empty() { continue; }

            let style = self.styles.get(voice).ok_or("Voice style not found")?;
            let mut onn = self.onn.lock().unwrap();
            let raw_audio = onn.infer(vec![tok.iter().map(|&t| t as i64).collect()], style, speed, &self.strategy)?;
            drop(onn);

            // AGGRESSIVE TRIM & NORM
            let start = raw_audio.iter().position(|&s| s.abs() > 0.005).unwrap_or(0);
            let end = raw_audio.iter().rposition(|&s| s.abs() > 0.005).unwrap_or(raw_audio.len());
            let trimmed = &raw_audio[start..end];

            let sq_sum: f32 = trimmed.iter().map(|&s| s * s).sum();
            let rms = (sq_sum / trimmed.len().max(1) as f32).sqrt();
            let gain = if rms > 0.001 { (0.15 / rms).min(100.0) } else { 1.0 };
            let mut chunk_audio: Vec<f32> = trimmed.iter().map(|&s| s * gain).collect();

            let fade_len = 240; 
            if chunk_audio.len() > fade_len * 2 {
                for i in 0..fade_len {
                    let alpha = i as f32 / fade_len as f32;
                    chunk_audio[i] *= alpha;
                    let end_idx = chunk_audio.len() - 1 - i;
                    chunk_audio[end_idx] *= alpha;
                }
            }
            callback(chunk_audio)?;
        }
        Ok(())
    }

    fn load_voices(path: &str) -> Result<HashMap<String, Vec<f32>>, Box<dyn Error>> {
        let file = File::open(path)?;
        let mut npz = NpzReader::new(file)?;
        let mut styles = HashMap::new();
        let names = npz.names()?;
        for name in names {
            if let Ok(array) = npz.by_name::<ndarray::OwnedRepr<f32>, ndarray::Ix3>(&name) {
                let style_vec: Vec<f32> = array
                    .index_axis(ndarray::Axis(0), 0)
                    .iter()
                    .cloned()
                    .collect();
                styles.insert(name.replace(".npy", ""), style_vec);
            }
        }
        Ok(styles)
    }

    fn load_voices_from_memory(bytes: &[u8]) -> Result<HashMap<String, Vec<f32>>, Box<dyn Error>> {
        let cursor = Cursor::new(bytes);
        let mut npz = NpzReader::new(cursor)?;
        let mut styles = HashMap::new();
        let names = npz.names()?;
        for name in names {
            if let Ok(array) = npz.by_name::<ndarray::OwnedRepr<f32>, ndarray::Ix3>(&name) {
                let style_vec: Vec<f32> = array
                    .index_axis(ndarray::Axis(0), 0)
                    .iter()
                    .cloned()
                    .collect();
                styles.insert(name.replace(".npy", ""), style_vec);
            }
        }
        Ok(styles)
    }
}
