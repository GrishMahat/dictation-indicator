//! Moonshine Tiny English int8 backend, with bounded rolling offline decode.
use super::Segmenter;
use dictation_config::EngineConfig;
use sherpa_onnx::{OfflineMoonshineModelConfig, OfflineRecognizer, OfflineRecognizerConfig};
use std::path::Path;
use std::time::{Duration, Instant};

const GATE: f32 = 0.008;
const END_SILENCE_MS: u32 = 500;
const MIN_SPEECH_MS: u32 = 200;
const MAX_SEGMENT_MS: u32 = 6000;
const ROLLING_MS: u32 = 1400;
const THREADS: i32 = 2;

pub(super) struct MoonshineSegmenter {
    recognizer: OfflineRecognizer,
    buf: Vec<i16>,
    speech_ms: u32,
    silence_ms: u32,
    in_speech: bool,
    last_decode: Option<Instant>,
    last_text: String,
    samples: Vec<f32>,
}

impl MoonshineSegmenter {
    pub(super) fn new(engine: &EngineConfig) -> Result<Self, String> {
        let dir = Path::new(&engine.moonshine_model_dir);
        let mut config = OfflineRecognizerConfig::default();
        config.model_config.moonshine = OfflineMoonshineModelConfig {
            preprocessor: Some(dir.join("preprocess.onnx").to_string_lossy().into_owned()),
            encoder: Some(dir.join("encode.int8.onnx").to_string_lossy().into_owned()),
            uncached_decoder: Some(
                dir.join("uncached_decode.int8.onnx")
                    .to_string_lossy()
                    .into_owned(),
            ),
            cached_decoder: Some(
                dir.join("cached_decode.int8.onnx")
                    .to_string_lossy()
                    .into_owned(),
            ),
            merged_decoder: None,
        };
        config.model_config.tokens = Some(dir.join("tokens.txt").to_string_lossy().into_owned());
        config.model_config.provider = Some("cpu".into());
        config.model_config.num_threads = THREADS;
        config.model_config.model_type = Some("moonshine".into());
        let recognizer = OfflineRecognizer::create(&config)
            .ok_or_else(|| format!("could not load Moonshine Tiny int8 models from `{}`; download the sherpa-onnx Moonshine Tiny English package", dir.display()))?;
        Ok(Self {
            recognizer,
            buf: Vec::with_capacity(crate::audio::RATE as usize * MAX_SEGMENT_MS as usize / 1000),
            speech_ms: 0,
            silence_ms: 0,
            in_speech: false,
            last_decode: None,
            last_text: String::new(),
            samples: Vec::with_capacity(
                crate::audio::RATE as usize * MAX_SEGMENT_MS as usize / 1000,
            ),
        })
    }

    fn transcribe(&mut self) -> String {
        self.samples.clear();
        self.samples
            .extend(self.buf.iter().map(|&s| s as f32 / 32768.0));
        let stream = self.recognizer.create_stream();
        stream.accept_waveform(crate::audio::RATE as i32, &self.samples);
        self.recognizer.decode(&stream);
        stream
            .get_result()
            .map(|r| r.text.trim().to_string())
            .unwrap_or_default()
    }

    fn finalize(&mut self) -> Option<(String, bool)> {
        let text = if self.speech_ms >= MIN_SPEECH_MS {
            self.transcribe()
        } else {
            String::new()
        };
        self.reset();
        dictation_core::state::set_busy(false);
        (!text.is_empty()).then_some((text, true))
    }

    fn reset(&mut self) {
        self.buf.clear();
        self.speech_ms = 0;
        self.silence_ms = 0;
        self.in_speech = false;
        self.last_decode = None;
        self.last_text.clear();
    }
}

impl Segmenter for MoonshineSegmenter {
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
            self.buf.extend_from_slice(chunk);
        }

        if self.in_speech
            && (self.silence_ms >= END_SILENCE_MS
                || self.buf.len() >= crate::audio::RATE as usize * MAX_SEGMENT_MS as usize / 1000)
        {
            return self.finalize();
        }
        if self.in_speech
            && self.speech_ms >= MIN_SPEECH_MS
            && self
                .last_decode
                .is_none_or(|t| t.elapsed() >= Duration::from_millis(ROLLING_MS as u64))
        {
            self.last_decode = Some(Instant::now());
            let text = self.transcribe();
            if !text.is_empty() && text != self.last_text {
                self.last_text = text.clone();
                return Some((text, false));
            }
        }
        None
    }

    fn flush(&mut self) -> Option<(String, bool)> {
        if self.in_speech {
            self.finalize()
        } else {
            None
        }
    }
}
