//! Shared dictation session state and process control.

pub use dictation_config;

pub mod control;
pub mod engine;
mod session;
pub mod state;

pub use session::{
    SessionState, busy_path, cleanup_stale, native_pid, native_pid_path, pid_alive, process_up,
    session_state,
};
