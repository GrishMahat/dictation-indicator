//! Speech recognition backends and their shared interface.

mod benchmark;
mod moonshine;
mod sensevoice;
mod vosk;
mod whisper;
mod zipformer;

#[cfg(test)]
mod tests;

use dictation_config::EngineConfig;

pub trait Segmenter {
    /// Feed a ~100 ms chunk; returns `(utterance_text_so_far, fresh)`.
    fn feed(&mut self, chunk: &[i16]) -> Option<(String, bool)>;
    /// Flush whatever is mid-utterance on shutdown.
    fn flush(&mut self) -> Option<(String, bool)>;
}

/// Build the backend named by `engine.backend`.
pub fn create(engine: &EngineConfig) -> Result<Box<dyn Segmenter>, String> {
    match engine.backend.as_str() {
        "whisper" => Ok(Box::new(whisper::WhisperSegmenter::new(engine)?)),
        "vosk" | "native" => Ok(Box::new(vosk::VoskSegmenter::new(engine)?)),
        "moonshine" => Ok(Box::new(moonshine::MoonshineSegmenter::new(engine)?)),
        "sensevoice" => Ok(Box::new(sensevoice::SenseVoiceSegmenter::new(engine)?)),
        "zipformer" => Ok(Box::new(zipformer::ZipformerSegmenter::new(engine)?)),
        other => Err(format!("unknown backend `{other}`")),
    }
}

pub use benchmark::streaming_bench_model;
pub use benchmark::whisper_bench;
pub use benchmark::whisper_bench_model;
pub use whisper::whisper_selftest;
