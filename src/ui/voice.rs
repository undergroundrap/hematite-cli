use crate::agent::inference::InferenceEvent;
use tokio::sync::mpsc as tokio_mpsc;
use std::sync::mpsc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use rodio::{OutputStream, Sink};
use kokoros::tts::koko::TTSKoko;

/// Manages the local Text-to-Speech pipeline.
/// Uses the all-Rust `kokoros` engine for streaming synthesis.
pub struct VoiceManager {
    sender: mpsc::SyncSender<String>,
    enabled: Arc<AtomicBool>,
    cancelled: Arc<AtomicBool>, // Immediate abort flag
    sink: Arc<tokio::sync::Mutex<Option<Sink>>>,
    /// Currently active voice ID — updated live by /voice command.
    current_voice: Arc<std::sync::Mutex<String>>,
    /// Speech speed multiplier (0.5–2.0). Read at synthesis time.
    current_speed: Arc<std::sync::Mutex<f32>>,
    /// Output volume (0.0–3.0). Applied to the rodio Sink.
    current_volume: Arc<std::sync::Mutex<f32>>,
}

impl VoiceManager {
    pub fn new(event_tx: tokio_mpsc::Sender<InferenceEvent>) -> Self {
        let cfg = crate::agent::config::load_config();
        let initial_voice  = crate::agent::config::effective_voice(&cfg);
        let initial_speed  = crate::agent::config::effective_voice_speed(&cfg);
        let initial_volume = crate::agent::config::effective_voice_volume(&cfg);
        // Large buffer so tokens arriving during model load (~30-60s) aren't dropped.
        let (tx, rx) = mpsc::sync_channel::<String>(1024);
        let enabled = Arc::new(AtomicBool::new(true));
        let cancelled = Arc::new(AtomicBool::new(false));
        let enabled_ctx = enabled.clone();
        let cancelled_ctx = cancelled.clone();
        let sink_shared = Arc::new(tokio::sync::Mutex::new(None));
        let current_voice  = Arc::new(std::sync::Mutex::new(initial_voice));
        let current_speed  = Arc::new(std::sync::Mutex::new(initial_speed));
        let current_volume = Arc::new(std::sync::Mutex::new(initial_volume));
        let voice_synth  = Arc::clone(&current_voice);
        let speed_synth  = Arc::clone(&current_speed);
        let volume_synth = Arc::clone(&current_volume);
        let sink_manager_clone = Arc::clone(&sink_shared);

        // Dedicated thread for voice synthesis and playback
        // This solves the 'rodio::OutputStream is not Send' issue.
        let _ = std::thread::Builder::new()
            .name("VoiceManager".into())
            .stack_size(32 * 1024 * 1024) // 32MB Stack for deep ONNX graph optimization
            .spawn(move || {
            let mut _stream: Option<OutputStream> = None;

            let _ = event_tx.blocking_send(InferenceEvent::VoiceStatus("Voice Engine: Initializing Audio Pipeline...".into()));
            let _ = event_tx.blocking_send(InferenceEvent::VoiceStatus("Voice Engine: Activating Baked-In Weights...".into()));

            // --- STATIC BAKE: Include weights in binary ---
            const MODEL_BYTES: &[u8] = include_bytes!("../../.hematite/assets/voice/kokoro-v1.0.onnx");
            const VOICES_BYTES: &[u8] = include_bytes!("../../.hematite/assets/voice/voices.bin");

            let _ = event_tx.blocking_send(InferenceEvent::VoiceStatus(
                "Voice Engine: Loading model (first start may take ~30s)...".into()
            ));

            // Catch panics from ONNX Runtime init (e.g. API version mismatch with system DLL)
            let tts_result = std::panic::catch_unwind(|| {
                TTSKoko::new_from_memory(MODEL_BYTES, VOICES_BYTES)
            });

            let tts = match tts_result {
                Ok(Ok(engine)) => {
                    enabled_ctx.store(true, Ordering::SeqCst);
                    if let Ok((s, handle)) = OutputStream::try_default() {
                        _stream = Some(s);
                        if let Ok(new_sink) = Sink::try_new(&handle) {
                            let mut lock = sink_shared.blocking_lock();
                            *lock = Some(new_sink);
                        }
                        let _ = event_tx.blocking_send(InferenceEvent::VoiceStatus(
                            "Voice Engine: Vibrant & Ready ✅".into()
                        ));
                    } else {
                        let _ = event_tx.blocking_send(InferenceEvent::VoiceStatus(
                            "Voice Engine: ERROR - No audio device found ❌".into()
                        ));
                    }
                    Some(engine)
                }
                Ok(Err(e)) => {
                    let _ = event_tx.blocking_send(InferenceEvent::VoiceStatus(
                        format!("Voice Engine: ERROR - {} ❌", e)
                    ));
                    None
                }
                Err(panic_val) => {
                    let msg = panic_val
                        .downcast_ref::<String>()
                        .map(|s| s.as_str())
                        .or_else(|| panic_val.downcast_ref::<&str>().copied())
                        .unwrap_or("unknown panic");
                    let _ = event_tx.blocking_send(InferenceEvent::VoiceStatus(
                        format!("Voice Engine: CRASH - {} ❌", msg)
                    ));
                    None
                }
            };

            // Stage 2: Background Synthesizer
            let (synth_tx, mut synth_rx) = tokio_mpsc::channel::<String>(64);
            let tts_shared = Arc::new(tokio::sync::Mutex::new(tts));
            let tts_synth_clone = Arc::clone(&tts_shared);
            let sink_synth_clone = Arc::clone(&sink_shared);
            let event_tx_synth = event_tx.clone();

            std::thread::spawn(move || {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .unwrap();

                rt.block_on(async {
                    while let Some(to_speak) = synth_rx.recv().await {
                        let mut engine_opt = tts_synth_clone.lock().await;
                        if let Some(ref mut engine) = *engine_opt {
                            let voice_id = voice_synth.lock().map(|v| v.clone()).unwrap_or_else(|_| "af_sky".to_string());
                            let speed    = speed_synth.lock().map(|v| *v).unwrap_or(1.0);
                            let volume   = volume_synth.lock().map(|v| *v).unwrap_or(1.0);
                            let res = engine.tts_raw_audio_streaming(
                                &to_speak,
                                "en-us",
                                &voice_id,
                                speed,
                                None, None, None, None,
                                |chunk| {
                                    if cancelled_ctx.load(Ordering::SeqCst) {
                                        return Err(Box::new(std::io::Error::new(
                                            std::io::ErrorKind::Interrupted, "Silenced"
                                        )));
                                    }
                                    if !chunk.is_empty() {
                                        if let Ok(mut snk_opt) = sink_synth_clone.try_lock() {
                                            if let Some(ref mut snk) = *snk_opt {
                                                snk.set_volume(volume);
                                                let source = rodio::buffer::SamplesBuffer::new(1, 24000, chunk);
                                                snk.append(source);
                                                snk.play();
                                            }
                                        }
                                    }
                                    Ok(())
                                }
                            );
                            if let Err(e) = res {
                                if e.to_string() != "Silenced" {
                                    let _ = event_tx_synth.send(InferenceEvent::VoiceStatus(
                                        format!("Audio Pipeline: Synthesis Error - {}", e)
                                    )).await;
                                }
                            }
                        }
                        drop(engine_opt);
                    }
                });
            });

            // Stage 1: Token Collector — builds tokens into sentences, then forwards to Stage 2.
            // Runs after model load. Tokens that arrived during load are buffered in the 1024-cap channel.
            let mut sentence_buffer = String::new();
            let mut last_activity = std::time::Instant::now();

            loop {
                let timeout = std::time::Duration::from_millis(150);
                let result = rx.recv_timeout(timeout);

                let token = match result {
                    Ok(t) => {
                        last_activity = std::time::Instant::now();
                        Some(t)
                    }
                    Err(mpsc::RecvTimeoutError::Timeout) => {
                        if !sentence_buffer.is_empty() && last_activity.elapsed() > timeout {
                            None
                        } else {
                            continue;
                        }
                    }
                    Err(mpsc::RecvTimeoutError::Disconnected) => break,
                };

                if let Some(ref text) = token {
                    if !enabled_ctx.load(Ordering::Relaxed) || text == "\x03" {
                        sentence_buffer.clear();
                        continue;
                    }
                    if text == "\x04" {
                        if !sentence_buffer.is_empty() {
                            let to_speak = sentence_buffer.trim().to_string();
                            sentence_buffer.clear();
                            let _ = synth_tx.blocking_send(to_speak);
                        }
                        continue;
                    }
                    sentence_buffer.push_str(text);
                }

                let to_speak = sentence_buffer.trim().to_string();
                let has_punctuation = to_speak.ends_with('.')
                    || to_speak.ends_with('!')
                    || to_speak.ends_with('?')
                    || to_speak.ends_with(':')
                    || to_speak.ends_with('\n');

                let is_word_boundary = token.as_ref()
                    .map(|t| t.starts_with(' ') || t.starts_with('\n') || t.starts_with('\t'))
                    .unwrap_or(true);

                let is_done = token.is_none();

                if (!to_speak.is_empty() && has_punctuation && is_word_boundary)
                    || (is_done && !to_speak.is_empty())
                {
                    sentence_buffer.clear();
                    let _ = synth_tx.blocking_send(to_speak);
                }
            }
        });


        Self { sender: tx, enabled, cancelled, sink: sink_manager_clone, current_voice, current_speed, current_volume }
    }

    pub fn speak(&self, text: String) {
        if self.enabled.load(Ordering::Relaxed) {
            // New utterance: reset cancellation
            self.cancelled.store(false, Ordering::SeqCst);
            let _ = self.sender.try_send(text);
        }
    }

    /// Forces a flush of the current sentence buffer.
    pub fn stop(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
        let _ = self.sender.try_send("\x03".to_string());
        if let Ok(mut lock) = self.sink.try_lock() {
            if let Some(sink) = lock.as_mut() {
                sink.stop();
                sink.pause();
                sink.play();
            }
        }
    }

    pub fn flush(&self) {
        if self.enabled.load(Ordering::Relaxed) {
            let _ = self.sender.try_send("\x04".to_string());
        }
    }

    pub fn toggle(&self) -> bool {
        let current = self.enabled.load(Ordering::Relaxed);
        let next = !current;
        self.enabled.store(next, Ordering::Relaxed);
        next
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }

    /// Change the active voice. Takes effect on the next spoken sentence.
    pub fn set_voice(&self, voice_id: &str) {
        if let Ok(mut v) = self.current_voice.lock() {
            *v = voice_id.to_string();
        }
    }

    pub fn current_voice_id(&self) -> String {
        self.current_voice.lock().map(|v| v.clone()).unwrap_or_else(|_| "af_sky".to_string())
    }

    pub fn set_speed(&self, speed: f32) {
        if let Ok(mut v) = self.current_speed.lock() {
            *v = speed.clamp(0.5, 2.0);
        }
    }

    pub fn set_volume(&self, volume: f32) {
        if let Ok(mut v) = self.current_volume.lock() {
            *v = volume.clamp(0.0, 3.0);
        }
    }
}

/// All voices baked into voices.bin, grouped for display.
pub const VOICE_LIST: &[(&str, &str)] = &[
    ("af_alloy",    "American Female — Alloy"),
    ("af_aoede",    "American Female — Aoede"),
    ("af_bella",    "American Female — Bella ⭐"),
    ("af_heart",    "American Female — Heart ⭐"),
    ("af_jessica",  "American Female — Jessica"),
    ("af_kore",     "American Female — Kore"),
    ("af_nicole",   "American Female — Nicole"),
    ("af_nova",     "American Female — Nova"),
    ("af_river",    "American Female — River"),
    ("af_sarah",    "American Female — Sarah"),
    ("af_sky",      "American Female — Sky (default)"),
    ("am_adam",     "American Male   — Adam"),
    ("am_echo",     "American Male   — Echo"),
    ("am_eric",     "American Male   — Eric"),
    ("am_fenrir",   "American Male   — Fenrir"),
    ("am_liam",     "American Male   — Liam"),
    ("am_michael",  "American Male   — Michael ⭐"),
    ("am_onyx",     "American Male   — Onyx"),
    ("am_puck",     "American Male   — Puck"),
    ("bf_alice",    "British Female  — Alice"),
    ("bf_emma",     "British Female  — Emma ⭐"),
    ("bf_isabella", "British Female  — Isabella"),
    ("bf_lily",     "British Female  — Lily"),
    ("bm_daniel",   "British Male    — Daniel"),
    ("bm_fable",    "British Male    — Fable ⭐"),
    ("bm_george",   "British Male    — George ⭐"),
    ("bm_lewis",    "British Male    — Lewis"),
    ("ef_dora",     "Spanish Female  — Dora"),
    ("em_alex",     "Spanish Male    — Alex"),
    ("ff_siwis",    "French Female   — Siwis"),
    ("hf_alpha",    "Hindi Female    — Alpha"),
    ("hf_beta",     "Hindi Female    — Beta"),
    ("hm_omega",    "Hindi Male      — Omega"),
    ("hm_psi",      "Hindi Male      — Psi"),
    ("if_sara",     "Italian Female  — Sara"),
    ("im_nicola",   "Italian Male    — Nicola"),
    ("jf_alpha",    "Japanese Female — Alpha"),
    ("jf_gongitsune","Japanese Female — Gongitsune"),
    ("jf_nezumi",   "Japanese Female — Nezumi"),
    ("jf_tebukuro", "Japanese Female — Tebukuro"),
    ("jm_kumo",     "Japanese Male   — Kumo"),
    ("zf_xiaobei",  "Chinese Female  — Xiaobei"),
    ("zf_xiaoni",   "Chinese Female  — Xiaoni"),
    ("zf_xiaoxiao", "Chinese Female  — Xiaoxiao"),
    ("zf_xiaoyi",   "Chinese Female  — Xiaoyi"),
    ("zm_yunjian",  "Chinese Male    — Yunjian"),
    ("zm_yunxi",    "Chinese Male    — Yunxi"),
    ("zm_yunxia",   "Chinese Male    — Yunxia"),
    ("zm_yunyang",  "Chinese Male    — Yunyang"),
];
