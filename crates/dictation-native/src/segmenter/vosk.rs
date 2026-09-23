use super::Segmenter;
use dictation_config::EngineConfig;

pub(super) struct VoskSegmenter {
    recognizer: vosk::Recognizer,
}

impl VoskSegmenter {
    pub(super) fn new(engine: &EngineConfig) -> Result<Self, String> {
        vosk::set_log_level(vosk::LogLevel::Info);
        let model = vosk::Model::new(engine.model_path.clone()).ok_or_else(|| {
            format!(
                "vosk model load failed: `{}` (engine.model_path)",
                engine.model_path
            )
        })?;
        let recognizer = vosk::Recognizer::new(&model, crate::audio::RATE as f32)
            .ok_or_else(|| "vosk recognizer creation failed".to_string())?;
        Ok(Self { recognizer })
    }
}

impl Segmenter for VoskSegmenter {
    fn feed(&mut self, chunk: &[i16]) -> Option<(String, bool)> {
        match self.recognizer.accept_waveform(chunk) {
            // Running = mid-utterance (the busy flag drives the pill's
            // `Dictating...` label); Finalized closes it.
            Ok(vosk::DecodingState::Running) => {
                dictation_core::state::set_busy(true);
                None
            }
            Ok(vosk::DecodingState::Finalized) => {
                dictation_core::state::set_busy(false);
                self.recognizer
                    .result()
                    .single()
                    .map(|r| (r.text.to_string(), true))
            }
            _ => None,
        }
    }

    fn flush(&mut self) -> Option<(String, bool)> {
        self.recognizer
            .final_result()
            .single()
            .map(|r| (r.text.to_string(), true))
    }
}
