//! Native daemon lifecycle: capture, recognize, and type speech.

use crate::formatter::Formatter;
use crate::logging::log;
use crate::{audio, inject, segmenter};
use dictation_config::Config;
use dictation_core::state;
use std::sync::atomic::{AtomicBool, Ordering};

static STOP: AtomicBool = AtomicBool::new(false);

extern "C" fn on_signal(_signal: libc::c_int) {
    STOP.store(true, Ordering::Relaxed);
}

/// Daemon entry point. Returns a process exit code.
pub fn run(config: &Config) -> i32 {
    if dictation_core::native_pid().is_some_and(dictation_core::pid_alive) {
        log("already running, refusing second instance");
        return 0;
    }

    unsafe {
        libc::signal(
            libc::SIGTERM,
            on_signal as extern "C" fn(libc::c_int) as libc::sighandler_t,
        );
        libc::signal(
            libc::SIGINT,
            on_signal as extern "C" fn(libc::c_int) as libc::sighandler_t,
        );
    }
    if let Err(error) = std::fs::write(
        dictation_core::native_pid_path(),
        std::process::id().to_string(),
    ) {
        log(&format!("pidfile: {error}"));
    }
    state::set_busy(false);

    let fail = |message: String| -> i32 {
        log(&message);
        state::set_active(false);
        state::set_busy(false);
        let _ = std::fs::remove_file(dictation_core::native_pid_path());
        1
    };

    log(&format!("starting backend `{}`", config.engine.backend));
    let mut recognizer = match segmenter::create(&config.engine) {
        Ok(recognizer) => recognizer,
        Err(error) => return fail(error),
    };
    let (stream, chunks, dropped_chunks, audio_errors) =
        match audio::open(&config.engine.audio_device) {
            Ok(capture) => capture,
            Err(error) => return fail(format!("audio: {error}")),
        };
    let mut typer = match inject::create(&config.engine) {
        Ok(typer) => typer,
        Err(error) => {
            drop(stream);
            return fail(format!("typing backend: {error}"));
        }
    };
    log(&format!("typing via `{}`", typer.name()));
    let mut formatter = Formatter::new();
    let mut typed = String::new();
    log("listening — native backend active");

    use std::sync::mpsc::RecvTimeoutError;
    let mut last_drop_report = std::time::Instant::now();
    let mut reported_drops = 0u64;
    loop {
        if STOP.load(Ordering::Relaxed) {
            break;
        }
        if let Ok(error) = audio_errors.try_recv() {
            log(&format!("audio stream failed: {error}"));
            break;
        }
        match chunks.recv_timeout(std::time::Duration::from_millis(200)) {
            Ok(chunk) => {
                if let Some((text, fresh)) = recognizer.feed(&chunk) {
                    emit(&mut *typer, &mut formatter, &mut typed, &text, fresh);
                }
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => {
                log("audio channel closed");
                break;
            }
        }
        if last_drop_report.elapsed() >= std::time::Duration::from_secs(5) {
            let total = dropped_chunks.load(Ordering::Relaxed);
            if total > reported_drops {
                log(&format!(
                    "audio capture fell behind: dropped {} × 100 ms chunks in the last 5 s",
                    total - reported_drops
                ));
                reported_drops = total;
            }
            last_drop_report = std::time::Instant::now();
        }
    }

    // Stop capture before finalizing, then let the indicator show the tail.
    state::set_active(false);
    drop(stream);
    if let Some((text, fresh)) = recognizer.flush() {
        emit(&mut *typer, &mut formatter, &mut typed, &text, fresh);
    }
    let _ = std::fs::remove_file(dictation_core::native_pid_path());
    state::set_busy(false);
    log("stopped");
    0
}

/// Type a finalized utterance, or revise a rolling Whisper prefix.
fn emit(
    typer: &mut dyn inject::Typer,
    formatter: &mut Formatter,
    typed: &mut String,
    full: &str,
    fresh: bool,
) {
    if fresh {
        typed.clear();
    }
    if full.is_empty() {
        return;
    }
    if fresh {
        if let Some(text) = formatter.format(full)
            && let Err(error) = typer.type_text(&text)
        {
            log(&format!("inject: {error}"));
        }
        typed.push_str(full);
        return;
    }

    let old: Vec<char> = typed.chars().collect();
    let new: Vec<char> = full.chars().collect();
    let common = old
        .iter()
        .zip(new.iter())
        .take_while(|(a, b)| a == b)
        .count();
    let backspaces = old.len() - common;
    let replacement: String = new[common..].iter().collect();
    if (backspaces > 0 || !replacement.is_empty())
        && let Err(error) = typer.revise(backspaces, &replacement)
    {
        log(&format!("revise: {error}"));
    }
    typed.clear();
    typed.push_str(full);
}
