//! Daemon logging to stderr and the user's cache directory.

use std::fs::OpenOptions;
use std::io::Write;

fn log_path() -> std::path::PathBuf {
    dictation_config::state_path().with_file_name("dictation-native.log")
}

pub(crate) fn log(message: &str) {
    eprintln!("dictation-native: {message}");
    if let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path())
    {
        let secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_secs())
            .unwrap_or(0);
        let _ = writeln!(file, "[{secs}] {message}");
    }
}
