use crate::Config;
use std::path::PathBuf;

/// User config directory.
pub fn config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("dictation")
}

/// Main TOML configuration file.
pub fn config_path() -> PathBuf {
    config_dir().join("config.toml")
}

/// Active-session marker shared by the CLI, daemon, and indicator.
pub fn state_path() -> PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("dictation-active")
}

/// Load and parse the config, including its exact path in diagnostics.
pub fn load_checked() -> Result<Config, String> {
    let path = config_path();
    let contents = std::fs::read_to_string(&path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    toml::from_str(&contents)
        .map_err(|error| format!("invalid config at {}: {error}", path.display()))
}

/// Load config, creating the default file on first use.
pub fn load_or_create() -> Config {
    let path = config_path();
    if !path.exists() {
        if let Err(error) = std::fs::create_dir_all(config_dir()) {
            eprintln!(
                "dictation: cannot create {}: {error}",
                config_dir().display()
            );
        } else if let Err(error) = std::fs::write(&path, default_toml()) {
            eprintln!("dictation: cannot write {}: {error}", path.display());
        }
    }
    match load_checked() {
        Ok(config) => config,
        Err(error) => {
            eprintln!("dictation: {error}; using built-in defaults");
            Config::default()
        }
    }
}

/// Default TOML written on first run.
pub fn default_toml() -> String {
    let mut out = String::from("# dictation config — edit and restart the indicator to apply.\n\n");
    out.push_str(&toml::to_string_pretty(&Config::default()).unwrap_or_default());
    out
}
