//! Shared configuration types, validation, and XDG paths.

mod config;
mod engine;
mod indicator;
mod paths;

pub use config::Config;
pub use engine::{EngineConfig, Provider, TypingBackend};
pub use indicator::{Anchor, Horizontal, IndicatorConfig, Vertical};
pub use paths::{config_dir, config_path, default_toml, load_checked, load_or_create, state_path};

#[cfg(test)]
mod tests;
