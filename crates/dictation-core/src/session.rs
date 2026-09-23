//! Session state and daemon-liveness checks.

use crate::state;

/// User-visible lifecycle of one dictation session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionState {
    Off,
    Listening,
    Dictating,
}

/// Read the shared session markers as one user-visible state.
pub fn session_state() -> SessionState {
    let daemon_up = process_up();
    if state::active() && daemon_up {
        SessionState::Listening
    } else if daemon_up {
        SessionState::Dictating
    } else {
        SessionState::Off
    }
}

/// Pidfile of the native engine daemon.
pub fn native_pid_path() -> std::path::PathBuf {
    dictation_config::state_path().with_file_name("dictation-native.pid")
}

/// Busy flag: exists while the recognizer has an open speech segment.
pub fn busy_path() -> std::path::PathBuf {
    dictation_config::state_path().with_file_name("dictation-busy")
}

pub fn native_pid() -> Option<u32> {
    std::fs::read_to_string(native_pid_path())
        .ok()?
        .trim()
        .parse()
        .ok()
}

pub fn pid_alive(pid: u32) -> bool {
    // The command-line check protects against pid reuse after reboot.
    std::fs::read(format!("/proc/{pid}/cmdline"))
        .map(|cmdline| {
            cmdline
                .windows(b"__native-run".len())
                .any(|w| w == b"__native-run")
        })
        .unwrap_or(false)
}

/// Whether the native engine daemon is alive and matches our entry point.
pub fn process_up() -> bool {
    native_pid().is_some_and(pid_alive)
}

/// Remove stale active/busy markers when no daemon is running.
pub fn cleanup_stale() {
    if !process_up() {
        if state::active() {
            state::set_active(false);
        }
        if state::busy() {
            state::set_busy(false);
        }
    }
}
