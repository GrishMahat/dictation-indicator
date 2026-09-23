//! Process-safe user commands for controlling a dictation session.

use crate::{cleanup_stale, engine, process_up, state};
use dictation_config::EngineConfig;
use std::io;

/// Toggle between microphone capture and stopped dictation.
pub fn toggle(config: &EngineConfig) -> io::Result<()> {
    with_control_lock(|| {
        cleanup_stale();
        if state::active() {
            stop()
        } else {
            start(config)
        }
    })
}

/// Start capture; repeated calls leave an existing session alone.
pub fn begin(config: &EngineConfig) -> io::Result<()> {
    with_control_lock(|| {
        cleanup_stale();
        start(config)
    })
}

/// Stop capture and let the daemon finish its final utterance.
pub fn end() -> io::Result<()> {
    with_control_lock(stop)
}

fn start(config: &EngineConfig) -> io::Result<()> {
    if state::active() && process_up() {
        return Ok(());
    }
    // A prior stop may still be flushing its tail. Never run two engines.
    if process_up() {
        engine::end();
    }

    // Publish intent before spawning so a fast failure cannot race with the
    // parent and leave a stale active marker behind.
    state::set_active(true);
    if let Err(error) = engine::begin(config) {
        state::set_active(false);
        return Err(error);
    }

    // Keep the inter-process lock until the daemon publishes its pid.
    for _ in 0..50 {
        if process_up() {
            return Ok(());
        }
        if !state::active() {
            return Err(io::Error::other(
                "dictation engine exited during startup; check dictation-native.log",
            ));
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    state::set_active(false);
    Err(io::Error::new(
        io::ErrorKind::TimedOut,
        "dictation engine did not start within one second",
    ))
}

fn stop() -> io::Result<()> {
    state::set_active(false);
    engine::end();
    Ok(())
}

fn with_control_lock<T>(action: impl FnOnce() -> io::Result<T>) -> io::Result<T> {
    use std::fs::OpenOptions;
    use std::os::fd::AsRawFd;

    let path = dictation_config::state_path().with_file_name("dictation-control.lock");
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let lock = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(path)?;

    loop {
        // SAFETY: flock only operates on this open file descriptor.
        if unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX) } == 0 {
            break;
        }
        let error = io::Error::last_os_error();
        if error.kind() != io::ErrorKind::Interrupted {
            return Err(error);
        }
    }
    // Closing the descriptor releases the lock on every return path.
    action()
}
