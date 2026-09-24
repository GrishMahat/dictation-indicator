//! Multilingual SenseVoiceSmall int8 backend using bounded offline decoding.
use super::Segmenter;
use dictation_config::EngineConfig;
use sherpa_onnx::{OfflineRecognizer, OfflineRecognizerConfig, OfflineSenseVoiceModelConfig};
use std::path::Path;

const GATE: f32 = 0.008;
const END_SILENCE_MS: u32 = 500;
const MIN_SPEECH_MS: u32 = 200;
const MAX_SEGMENT_MS: u32 = 8000;

pub(super) struct SenseVoiceSegmenter {
    recognizer: OfflineRecognizer,
    samples: Vec<i16>,
    speech_ms: u32,
    silence_ms: u32,
    in_speech: bool,
}

impl SenseVoiceSegmenter {
    pub(super) fn new(engine: &EngineConfig) -> Result<Self, String> {
        let dir = Path::new(&engine.sense_voice_model_dir);
        let mut config = OfflineRecognizerConfig::default();
        config.model_config.sense_voice = OfflineSenseVoiceModelConfig {
            model: Some(dir.join("model.int8.onnx").to_string_lossy().into_owned()),
            language: Some("auto".into()),
            use_itn: true,
        };
        config.model_config.tokens = Some(dir.join("tokens.txt").to_string_lossy().into_owned());
        config.model_config.provider = Some("cpu".into());
        config.model_config.num_threads = 2;
        config.model_config.model_type = Some("sense_voice".into());
        let recognizer = OfflineRecognizer::create(&config)
            .ok_or_else(|| format!("could not load SenseVoice model from `{}`", dir.display()))?;
        Ok(Self {
            recognizer,
            samples: Vec::with_capacity(
                crate::audio::RATE as usize * MAX_SEGMENT_MS as usize / 1000,
            ),
            speech_ms: 0,
            silence_ms: 0,
            in_speech: false,
        })
    }

    fn decode(&mut self) -> String {
        let samples: Vec<f32> = self.samples.iter().map(|&s| s as f32 / 32768.0).collect();
        let stream = self.recognizer.create_stream();
        stream.accept_waveform(crate::audio::RATE as i32, &samples);
        self.recognizer.decode(&stream);
        stream
            .get_result()
            .map(|r| r.text.trim().to_string())
            .unwrap_or_default()
    }

    fn finish(&mut self) -> Option<(String, bool)> {
        let text = if self.speech_ms >= MIN_SPEECH_MS {
            self.decode()
        } else {
            String::new()
        };
        self.samples.clear();
        self.speech_ms = 0;
        self.silence_ms = 0;
        self.in_speech = false;
        dictation_core::state::set_busy(false);
        (!text.is_empty()).then_some((text, true))
    }
}

impl Segmenter for SenseVoiceSegmenter {
    fn feed(&mut self, chunk: &[i16]) -> Option<(String, bool)> {
        if chunk.is_empty() {
            return None;
        }
        let rms = (chunk
            .iter()
            .map(|&s| (s as f32 / 32768.0).powi(2))
            .sum::<f32>()
            / chunk.len() as f32)
            .sqrt();
        let ms = (chunk.len() as u64 * 1000 / crate::audio::RATE as u64) as u32;
        if rms >= GATE {
            self.in_speech = true;
            self.speech_ms += ms;
            self.silence_ms = 0;
            dictation_core::state::set_busy(true);
        } else if self.in_speech {
            self.silence_ms += ms;
        }
        if self.in_speech {
            self.samples.extend_from_slice(chunk);
        }
        if self.in_speech
            && (self.silence_ms >= END_SILENCE_MS
                || self.samples.len()
                    >= crate::audio::RATE as usize * MAX_SEGMENT_MS as usize / 1000)
        {
            return self.finish();
        }
        None
    }

    fn flush(&mut self) -> Option<(String, bool)> {
        if self.in_speech { self.finish() } else { None }
    }
}
