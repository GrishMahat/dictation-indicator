//! State-file IPC shared by the CLI, daemon, and indicator.

use dictation_config::state_path;

pub fn active() -> bool {
    state_path().exists()
}

pub fn set_active(on: bool) {
    touch(state_path(), on);
}

/// Whether the recognizer currently has an open speech segment.
pub fn busy() -> bool {
    crate::busy_path().exists()
}

/// Update the recognizer's speech-segment marker.
pub fn set_busy(on: bool) {
    touch(crate::busy_path(), on);
}

fn touch(path: std::path::PathBuf, on: bool) {
    if on {
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let _ = std::fs::write(path, "");
    } else {
        let _ = std::fs::remove_file(path);
    }
}
