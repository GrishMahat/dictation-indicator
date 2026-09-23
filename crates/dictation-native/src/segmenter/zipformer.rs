//! Sherpa-ONNX streaming Zipformer backend.
use super::Segmenter;
use dictation_config::EngineConfig;
use sherpa_onnx::{OnlineRecognizer, OnlineRecognizerConfig};
use std::path::Path;

pub(super) struct ZipformerSegmenter {
    recognizer: OnlineRecognizer,
    stream: sherpa_onnx::OnlineStream,
    last_text: String,
}

impl ZipformerSegmenter {
    pub(super) fn new(engine: &EngineConfig) -> Result<Self, String> {
        let dir = Path::new(&engine.zipformer_model_dir);
        let mut config = OnlineRecognizerConfig::default();
        config.model_config.transducer.encoder = Some(
            dir.join("encoder-epoch-99-avg-1.int8.onnx")
                .to_string_lossy()
                .into_owned(),
        );
        config.model_config.transducer.decoder = Some(
            dir.join("decoder-epoch-99-avg-1.onnx")
                .to_string_lossy()
                .into_owned(),
        );
        config.model_config.transducer.joiner = Some(
            dir.join("joiner-epoch-99-avg-1.int8.onnx")
                .to_string_lossy()
                .into_owned(),
        );
        config.model_config.tokens = Some(dir.join("tokens.txt").to_string_lossy().into_owned());
        config.model_config.provider = Some("cpu".into());
        config.model_config.num_threads = 2;
        config.enable_endpoint = true;
        // Match the other live backends: close a phrase after about half a
        // second of silence instead of holding context for Sherpa's 1.2 s
        // default. This keeps the final-text tail short after toggle-off.
        config.rule1_min_trailing_silence = 0.5;
        config.rule2_min_trailing_silence = 0.5;
        config.rule3_min_utterance_length = 0.0;
        config.decoding_method = Some("greedy_search".into());
        let recognizer = OnlineRecognizer::create(&config).ok_or_else(|| {
            format!(
                "could not load Zipformer models from `{}`; expected the English int8 model files",
                dir.display()
            )
        })?;
        let stream = recognizer.create_stream();
        Ok(Self {
            recognizer,
            stream,
            last_text: String::new(),
        })
    }

    fn decode_ready(&mut self) -> Option<(String, bool)> {
        let mut output = None;
        while self.recognizer.is_ready(&self.stream) {
            self.recognizer.decode(&self.stream);
            if let Some(result) = self.recognizer.get_result(&self.stream) {
                let text = result.text.trim().to_string();
                if !text.is_empty() && text != self.last_text {
                    self.last_text = text.clone();
                    output = Some((text, false));
                }
            }
            if self.recognizer.is_endpoint(&self.stream) {
                let text = self
                    .recognizer
                    .get_result(&self.stream)
                    .map(|r| r.text.trim().to_string())
                    .unwrap_or_default();
                self.recognizer.reset(&self.stream);
                self.last_text.clear();
                dictation_core::state::set_busy(false);
                if !text.is_empty() {
                    output = Some((text, true));
                }
            }
        }
        output
    }
}

impl Segmenter for ZipformerSegmenter {
    fn feed(&mut self, chunk: &[i16]) -> Option<(String, bool)> {
        if chunk.is_empty() {
            return None;
        }
        dictation_core::state::set_busy(true);
        let samples: Vec<f32> = chunk.iter().map(|&s| s as f32 / 32768.0).collect();
        self.stream
            .accept_waveform(crate::audio::RATE as i32, &samples);
        self.decode_ready()
    }

    fn flush(&mut self) -> Option<(String, bool)> {
        self.stream.input_finished();
        let output = self.decode_ready().map(|(text, _)| (text, true));
        dictation_core::state::set_busy(false);
        output
    }
}
