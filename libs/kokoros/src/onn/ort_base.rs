use ort::session::Session;
use crate::tts::koko::ModelStrategy;

pub trait OrtBase {
    fn set_sess(&mut self, sess: Session);
    fn sess(&self) -> Option<&Session>;

    fn load_model(&mut self, model_path: String) -> Result<(), Box<dyn std::error::Error>> {
        let sess = Session::builder()?
            .with_execution_providers([
                ort::execution_providers::CPUExecutionProvider::default().build(),
            ])?
            .with_optimization_level(ort::session::builder::GraphOptimizationLevel::Disable)?
            .with_memory_pattern(false)?
            .with_parallel_execution(false)?
            .with_intra_threads(1)?
            .with_inter_threads(1)?
            .commit_from_file(model_path)?;

        self.set_sess(sess);
        Ok(())
    }

    fn load_model_from_memory(&mut self, model_bytes: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
        let sess = Session::builder()?
            .with_execution_providers([
                ort::execution_providers::CPUExecutionProvider::default().build(),
            ])?
            .with_optimization_level(ort::session::builder::GraphOptimizationLevel::Disable)?
            .with_memory_pattern(false)?
            .with_parallel_execution(false)?
            .with_intra_threads(1)?
            .with_inter_threads(1)?
            .commit_from_memory(model_bytes)?;

        self.set_sess(sess);
        Ok(())
    }

    fn inputs(&self) -> Vec<String> {
        self.sess()
            .map(|s| s.inputs().iter().map(|i| i.name().to_string()).collect())
            .unwrap_or_default()
    }

    fn outputs(&self) -> Vec<String> {
        self.sess()
            .map(|s| s.outputs().iter().map(|o| o.name().to_string()).collect())
            .unwrap_or_default()
    }

    fn infer(
        &mut self,
        tokens_batch: Vec<Vec<i64>>,
        style: &[f32],
        speed: f32,
        strategy: &ModelStrategy,
    ) -> Result<Vec<f32>, Box<dyn std::error::Error>>;
}
