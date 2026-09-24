//! Native audio capture, speech recognition, and text injection.

mod audio;
mod daemon;
mod formatter;
mod inject;
mod logging;
mod provider;
mod segmenter;

pub use daemon::run;
pub use inject::type_text_once;
pub use provider::{ProviderCandidate, compiled_features};
pub use segmenter::{streaming_bench_model, whisper_bench, whisper_bench_model, whisper_selftest};

/// Preflight the configured compute provider for diagnostics.
///
/// Returns the same provider candidate the daemon uses, or the reason a
/// pinned provider is unavailable in this build/on this machine.
pub fn provider_status(engine: &dictation_config::EngineConfig) -> Result<String, String> {
    let resolved = provider::resolve(engine)?;
    Ok(format!(
        "candidate `{}` (configured `{}`, gpu features: {})",
        resolved.as_str(),
        engine.provider.as_str(),
        provider::compiled_features()
    ))
}

pub(crate) use logging::log;
