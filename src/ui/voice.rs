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
}

impl VoiceManager {
    pub fn new(event_tx: tokio_mpsc::Sender<InferenceEvent>) -> Self {
        let (tx, rx) = mpsc::sync_channel::<String>(128);
        let enabled = Arc::new(AtomicBool::new(true));
        let cancelled = Arc::new(AtomicBool::new(false));
        let enabled_ctx = enabled.clone();
        let cancelled_ctx = cancelled.clone();
        let sink_shared = Arc::new(tokio::sync::Mutex::new(None));
        let sink_manager_clone = Arc::clone(&sink_shared);

        // Dedicated thread for voice synthesis and playback
        // This solves the 'rodio::OutputStream is not Send' issue.
        let _ = std::thread::Builder::new()
            .name("VoiceManager".into())
            .stack_size(32 * 1024 * 1024) // 32MB Stack for deep ONNX graph optimization
            .spawn(move || {
            let mut tts: Option<TTSKoko> = None;
            let mut _stream: Option<OutputStream> = None;
            // The sink is now passed in from the manager
            
            let _ = event_tx.blocking_send(InferenceEvent::VoiceStatus("Voice Engine: Initializing Audio Pipeline...".into()));

            let _ = event_tx.blocking_send(InferenceEvent::VoiceStatus("Voice Engine: Activating Baked-In Weights...".into()));
            
            // --- STATIC BAKE: Include weights in binary ---
            const MODEL_BYTES: &[u8] = include_bytes!("../../.hematite/assets/voice/kokoro-v1.0.onnx");
            const VOICES_BYTES: &[u8] = include_bytes!("../../.hematite/assets/voice/voices.bin");

            match TTSKoko::new_from_memory(MODEL_BYTES, VOICES_BYTES) {
                Ok(engine) => {
                    tts = Some(engine);
                    enabled_ctx.store(true, Ordering::SeqCst);
                    
                    if let Ok((s, handle)) = OutputStream::try_default() {
                        _stream = Some(s);
                        if let Ok(new_sink) = Sink::try_new(&handle) {
                            let mut lock = sink_shared.blocking_lock();
                            *lock = Some(new_sink);
                        }
                        let _ = event_tx.blocking_send(InferenceEvent::VoiceStatus("Voice Engine: Vibrant & Ready ✅".into()));
                    } else {
                        let _ = event_tx.blocking_send(InferenceEvent::VoiceStatus("Voice Engine: ERROR - No audio device found ❌".into()));
                    }
                }
                Err(e) => {
                    let _ = event_tx.blocking_send(InferenceEvent::VoiceStatus(format!("Voice Engine: ERROR - {} ❌", e)));
                }
            }

            // 4. Threaded Pipeline Initialization
            let (synth_tx, mut synth_rx) = tokio_mpsc::channel::<String>(32);
            let tts_shared = Arc::new(tokio::sync::Mutex::new(tts));
            let tts_synth_clone = Arc::clone(&tts_shared);
            let sink_synth_clone = Arc::clone(&sink_shared);
            let event_tx_synth = event_tx.clone();

            // Stage 2: Background Synthesizer (Lookahead Engine)
            // This thread pulls finished sentences from the queue and synthesizes them as fast as possible.
            std::thread::spawn(move || {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .unwrap();

                rt.block_on(async {
                    while let Some(to_speak) = synth_rx.recv().await {
                        let mut engine_opt = tts_synth_clone.lock().await;
                        if let Some(ref mut engine) = *engine_opt {
                                
                            // TRUE STREAMING: Yield chunks immediately to the audio sink
                            let res = engine.tts_raw_audio_streaming(
                                &to_speak,
                                "en-us",
                                "af_sky",
                                1.0,
                                None, None, None, None,
                                |chunk| {
                                    // CHECK FOR ABORT: mid-stream silence
                                    if cancelled_ctx.load(Ordering::SeqCst) {
                                        return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Interrupted, "Silenced")));
                                    }

                                    if !chunk.is_empty() {
                                        if let Ok(mut snk_opt) = sink_synth_clone.try_lock() {
                                            if let Some(ref mut snk) = *snk_opt {
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
                                // Silent skip for 'Silenced' errors; only log real failures.
                                if e.to_string() != "Silenced" {
                                    let _ = event_tx_synth.send(InferenceEvent::VoiceStatus(format!("Audio Pipeline: Synthesis Error - {}", e))).await;
                                }
                            }
                        }
                        drop(engine_opt); // Release lock immediately for background loading if needed
                    }
                });
            });

            // Stage 1: Token Collector (Sentence Builder)
            // This thread receives raw tokens from the AI and groups them into logical thoughts.
            let mut sentence_buffer = String::new();
            let mut last_activity = std::time::Instant::now();

            loop {
                // Sentence-Streaming Strategy: Shorter timeout for faster response.
                let timeout = std::time::Duration::from_millis(150);
                let result = rx.recv_timeout(timeout);
                
                let token = match result {
                    Ok(t) => {
                        last_activity = std::time::Instant::now();
                        Some(t)
                    },
                    Err(mpsc::RecvTimeoutError::Timeout) => {
                        if !sentence_buffer.is_empty() && last_activity.elapsed() > timeout {
                            None 
                        } else {
                            continue;
                        }
                    },
                    Err(mpsc::RecvTimeoutError::Disconnected) => break,
                };

                if let Some(ref text) = token {
                    if !enabled_ctx.load(Ordering::Relaxed) || text == "\x03" {
                        sentence_buffer.clear();
                        continue;
                    }
                    
                    // Explicit End-of-Message signal handling
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

                // --- PIPELINE: SENTENCE AWARENESS ---
                let to_speak = sentence_buffer.trim().to_string();
                let has_punctuation = to_speak.ends_with('.') || to_speak.ends_with('!') || 
                                     to_speak.ends_with('?') || to_speak.ends_with(':') || 
                                     to_speak.ends_with('\n');
                
                let is_word_boundary = token.as_ref()
                    .map(|t| t.starts_with(' ') || t.starts_with('\n') || t.starts_with('\t'))
                    .unwrap_or(true);

                let is_done = token.is_none();

                // Split at sentence boundaries OR on idle timeout
                if (!to_speak.is_empty() && (has_punctuation && is_word_boundary)) || 
                   (is_done && !to_speak.is_empty()) 
                {
                    sentence_buffer.clear();
                    let _ = synth_tx.blocking_send(to_speak);
                }
            }
        });


        Self { sender: tx, enabled, cancelled, sink: sink_manager_clone }
    }

    pub fn speak(&self, text: String) {
        if self.enabled.load(Ordering::Relaxed) {
            // New utterance: reset cancellation
            self.cancelled.store(false, Ordering::SeqCst);
            let _ = self.sender.try_send(text);
        }
    }

    /// Forces a flush of the current sentence buffer.
    /// Useful for ensuring the final part of a message is spoken upon completion.
    pub fn stop(&self) {
        // 1. Signal ANY active synthesis threads to abort immediately and clear the Stage 1 buffer.
        self.cancelled.store(true, Ordering::SeqCst);
        let _ = self.sender.try_send("\x03".to_string());
        
        // 2. We also send a 'flush' marker to the synth thread so it knows to skip its current queue.
        // In this architecture, setting 'cancelled' to true is enough because the synth thread
        // checks it before every chunk.

        // 3. Kill the audio hardware output instantly.
        // We use try_lock to avoid deadlocks. If the hardware is occupied,
        // the 'cancelled' flag in the synthesis callback will catch it on the next chunk.
        if let Ok(mut lock) = self.sink.try_lock() {
            if let Some(sink) = lock.as_mut() {
                // stop() clears the internal rodio queue and stops samples immediately.
                sink.stop();
                sink.pause();
                sink.play(); // resume in a clean playback state for next sentence.
            }
        }
    }

    pub fn flush(&self) {
        if self.enabled.load(Ordering::Relaxed) {
            // Sending EOF marker to trigger immediate synthesis of the full buffer.
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
}
