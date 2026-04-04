use crate::onn::ort_base::OrtBase;
use crate::tts::koko::ModelStrategy;
use ndarray::{Array, Axis};
use ort::session::Session;

pub struct OrtKoko {
    session: Option<Session>,
}

impl OrtKoko {
    pub fn new() -> Self {
        Self { session: None }
    }
}

impl OrtBase for OrtKoko {
    fn set_sess(&mut self, sess: Session) {
        self.session = Some(sess);
    }

    fn sess(&self) -> Option<&Session> {
        self.session.as_ref()
    }

    fn infer(
        &mut self,
        tokens_batch: Vec<Vec<i64>>,
        style: &[f32],
        speed: f32,
        strategy: &ModelStrategy,
    ) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
        let input_names = self.inputs();
        let input_tokens = input_names.get(0).ok_or("Tokens input not found")?;
        let input_style = input_names.get(1).ok_or("Style input not found")?;
        let input_speed = input_names.get(2).ok_or("Speed input not found")?;

        let session = self.session.as_mut().ok_or("Model not loaded")?;

        // Prepare tensors
        let batch_size = tokens_batch.len();
        let seq_len = tokens_batch[0].len();
        let mut tokens_flat = Vec::with_capacity(batch_size * seq_len);
        for row in tokens_batch {
            tokens_flat.extend(row);
        }

        let tokens_tensor = Array::from_shape_vec((batch_size, seq_len), tokens_flat)?.into_dyn();
        let tokens_val: ort::value::Value = ort::value::Value::from_array(tokens_tensor)?.into();

        // Prepare input values
        // Sovereign Precision: Mathematically perfect inputs for Kokoro v1.0
        // 1. Voice Style: Dynamic Reshape [1, N]
        let style_len = style.len();
        let style_tensor = Array::from_shape_vec((1, style_len), style.to_vec())?.into_dyn();
        let style_val: ort::value::Value = ort::value::Value::from_array(style_tensor)?.into();

        // 2. Speed: Convert to Rank-1 Vector (Shape [1]) as requested by model
        let speed_tensor = Array::from_elem((1,), speed).into_dyn();
        let speed_val: ort::value::Value = ort::value::Value::from_array(speed_tensor)?.into();

        let mut session_inputs = vec![
            (input_tokens.as_str(), tokens_val),
            (input_style.as_str(), style_val),
            (input_speed.as_str(), speed_val),
        ];

        // 3. Noise Clusters: Convert Clarity nodes to Rank-1 Vectors
        let noise_aliases = ["noise_scale", "ns", "p1"];
        let noise_w_aliases = ["noise_scale_w", "nsw", "p2"];

        for node_name in &input_names {
            if noise_aliases.contains(&node_name.as_str()) {
                let ns_tensor = Array::from_elem((1,), 0.667f32).into_dyn(); // Rank-1 (Standard Energy)
                let ns_val: ort::value::Value = ort::value::Value::from_array(ns_tensor)?.into();
                session_inputs.push((node_name.as_str(), ns_val));
                tracing::info!("Sovereign Engine: Diamond Clarity Vector Forced (Node: {}).", node_name);
            }
            if noise_w_aliases.contains(&node_name.as_str()) {
                let nsw_tensor = Array::from_elem((1,), 0.8f32).into_dyn(); // Rank-1 (Standard Energy)
                let nsw_val: ort::value::Value = ort::value::Value::from_array(nsw_tensor)?.into();
                session_inputs.push((node_name.as_str(), nsw_val));
                tracing::info!("Sovereign Engine: Diamond Breath Suppression Vector Forced (Node: {}).", node_name);
            }
        }

        let outputs = session.run(session_inputs)?;

        let audio_key = strategy.audio_key();
        
        // Extract using 2.0 API which returns (Shape, &[T]) in this version
        let (shape, data) = outputs[audio_key].try_extract_tensor::<f32>()?;
        let dims: Vec<usize> = shape.iter().map(|&d| d as usize).collect();
        let audio_array = Array::from_shape_vec(dims, data.to_vec())?.into_dyn();

        let mut full_audio = Vec::new();
        for chunk in audio_array.axis_iter(Axis(0)) {
            full_audio.extend(chunk.iter().cloned());
        }

        Ok(full_audio)
    }
}
