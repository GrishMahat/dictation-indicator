//! Native audio capture, speech recognition, and text injection.

mod audio;
mod daemon;
mod formatter;
mod inject;
mod logging;
mod segmenter;

pub use daemon::run;
pub use inject::type_text_once;
pub use segmenter::{streaming_bench_model, whisper_bench, whisper_bench_model, whisper_selftest};

pub(crate) use logging::log;
