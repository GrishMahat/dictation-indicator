//! Detached process lifecycle for the configured recognition engine.

use dictation_config::EngineConfig;
use std::io;
use std::process::{Command, Stdio};

/// Spawn a command in its own session so keybinds never hold a terminal open.
fn spawn_detached(cmd: &mut Command) -> io::Result<()> {
    // SAFETY: `setsid` is async-signal-safe and runs before exec in the child.
    unsafe {
        use std::os::unix::process::CommandExt;
        cmd.pre_exec(|| {
            libc::setsid();
            Ok(())
        });
    }
    cmd.spawn()?;
    Ok(())
}

/// Launch the configured daemon.
pub fn begin(config: &EngineConfig) -> io::Result<()> {
    match config.backend.as_str() {
        "native" | "vosk" | "whisper" | "moonshine" | "zipformer" => {
            let executable = std::env::current_exe()?;
            let mut command = Command::new(executable);
            command
                .arg("__native-run")
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null());
            spawn_detached(&mut command)
        }
        other => Err(io::Error::new(
            io::ErrorKind::Unsupported,
            format!("backend `{other}` not implemented"),
        )),
    }
}

/// Signal the daemon to flush its active utterance and stop.
pub fn end() {
    if let Some(pid) = crate::native_pid() {
        // SAFETY: pid_alive verifies this pid belongs to our daemon.
        unsafe { libc::kill(pid as libc::pid_t, libc::SIGTERM) };
    }
    for _ in 0..80 {
        if !crate::process_up() {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    // The grace budget expired; force-stop the same verified process.
    if let Some(pid) = crate::native_pid() {
        // SAFETY: the process is checked by process_up before reaching here.
        unsafe { libc::kill(pid as libc::pid_t, libc::SIGKILL) };
        for _ in 0..10 {
            if !crate::process_up() {
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
    }
}
