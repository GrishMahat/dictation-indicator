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
    // Preflight the compute provider once, before model loading, and log the
    // candidate. Whisper.cpp may still fall back if GPU initialization fails.
    let provider = crate::provider::resolve(engine)?;
    crate::log(&format!(
        "compute provider candidate `{}` (configured `{}`)",
        provider.as_str(),
        engine.provider.as_str()
    ));
    match engine.backend.as_str() {
        "whisper" => Ok(Box::new(whisper::WhisperSegmenter::new(engine, provider)?)),
        "vosk" | "native" => Ok(Box::new(vosk::VoskSegmenter::new(engine)?)),
        "moonshine" => Ok(Box::new(moonshine::MoonshineSegmenter::new(
            engine, provider,
        )?)),
        "sensevoice" => Ok(Box::new(sensevoice::SenseVoiceSegmenter::new(
            engine, provider,
        )?)),
        "zipformer" => Ok(Box::new(zipformer::ZipformerSegmenter::new(
            engine, provider,
        )?)),
        other => Err(format!("unknown backend `{other}`")),
    }
}

pub use benchmark::streaming_bench_model;
pub use benchmark::whisper_bench;
pub use benchmark::whisper_bench_model;
pub use whisper::whisper_selftest;
