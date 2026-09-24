use crate::{EngineConfig, IndicatorConfig};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub indicator: IndicatorConfig,
    pub engine: EngineConfig,
}

impl Config {
    /// Report settings that parse correctly but cannot work as configured.
    pub fn validation_errors(&self) -> Vec<String> {
        let mut errors = Vec::new();
        if self.indicator.width <= 0 {
            errors.push("indicator.width must be greater than zero".into());
        }
        if self.indicator.height <= 0 {
            errors.push("indicator.height must be greater than zero".into());
        }
        if self.indicator.margin < 0 {
            errors.push("indicator.margin cannot be negative".into());
        }

        match self.engine.backend.as_str() {
            "whisper" if !PathBuf::from(&self.engine.whisper_model).is_file() => {
                errors.push(format!(
                    "Whisper model file does not exist: {}",
                    self.engine.whisper_model
                ));
            }
            "vosk" | "native"
                if !PathBuf::from(&self.engine.model_path)
                    .join("am/final.mdl")
                    .is_file()
                    || !PathBuf::from(&self.engine.model_path)
                        .join("conf/model.conf")
                        .is_file() =>
            {
                errors.push(format!(
                    "Vosk model files are missing from: {}",
                    self.engine.model_path
                ));
            }
            "moonshine"
                if !([
                    "preprocess.onnx",
                    "encode.int8.onnx",
                    "uncached_decode.int8.onnx",
                    "cached_decode.int8.onnx",
                    "tokens.txt",
                ]
                .iter()
                .all(|file| {
                    PathBuf::from(&self.engine.moonshine_model_dir)
                        .join(file)
                        .is_file()
                }) || ![
                    "encoder_model.ort",
                    "decoder_model_merged.ort",
                    "tokens.txt",
                ]
                .iter()
                .all(|file| {
                    PathBuf::from(&self.engine.moonshine_model_dir)
                        .join(file)
                        .is_file()
                })) =>
            {
                errors.push(format!(
                    "Moonshine model files are missing from: {}",
                    self.engine.moonshine_model_dir
                ));
            }
            "zipformer"
                if ![
                    "encoder-epoch-99-avg-1.int8.onnx",
                    "decoder-epoch-99-avg-1.onnx",
                    "joiner-epoch-99-avg-1.int8.onnx",
                    "tokens.txt",
                ]
                .iter()
                .all(|file| {
                    PathBuf::from(&self.engine.zipformer_model_dir)
                        .join(file)
                        .is_file()
                }) =>
            {
                errors.push(format!(
                    "Zipformer English 20M int8 model files are missing from: {}",
                    self.engine.zipformer_model_dir
                ));
            }
            "sensevoice"
                if !PathBuf::from(&self.engine.sense_voice_model_dir)
                    .join("model.int8.onnx")
                    .is_file()
                    || !PathBuf::from(&self.engine.sense_voice_model_dir)
                        .join("tokens.txt")
                        .is_file() =>
            {
                errors.push(format!(
                    "SenseVoice model files are missing from: {}",
                    self.engine.sense_voice_model_dir
                ));
            }
            "whisper" | "vosk" | "native" | "moonshine" | "zipformer" | "sensevoice" => {}
            backend => errors.push(format!("unsupported engine.backend: {backend}")),
        }
        errors
    }
}
