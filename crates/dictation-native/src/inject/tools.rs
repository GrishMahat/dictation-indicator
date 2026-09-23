//! The four external typing backends: `wtype`, `dotool`, `ydotool` and
//! `xdotool`, each spawned once per utterance.
//!
//! Invocation contracts (checked against each tool's man page/source,
//! since none of them are assumed to be installed):
//!
//! * `wtype` — a bare `-` in argv makes it read the text from stdin, so
//!   an utterance never travels through argv or a shell. No other flags
//!   are passed: the option set differs between wtype builds, and an
//!   extra one would turn a working setup into an "invalid option"
//!   failure. `-P`/`-p` press/release a keysym by name — that is how we
//!   backspace.
//! * `ydotool` — `type -f -` reads stdin, where escape processing is off
//!   (it is on for argv). It talks to the separate `ydotoold` daemon,
//!   which may be socket-activated, so it is not probed up front: a
//!   daemon problem surfaces as ydotool's own stderr message. `key`
//!   speaks raw keycodes, hence backspace is `14:1 14:0`
//!   (`KEY_BACKSPACE` in `linux/input-event-codes.h`).
//! * `dotool` — newline-delimited actions on stdin. `type` consumes the
//!   rest of its line verbatim (no escaping, so a `-` or a quote in an
//!   utterance is just text), which is why embedded newlines become an
//!   explicit `key enter` action. Needs `/dev/uinput` itself.
//! * `xdotool` — X11/XTEST, so on a Wayland session it can only reach
//!   X11 (XWayland) windows; `type` takes the text as argv, and `--`
//!   stops xdotool's getopt from reading a leading `-` as a flag.
//!
//! Every failure is reported as an `io::Error` carrying the tool's
//! stderr, because all four explain themselves there.
//!
//! During streaming a revision (backspace + retype of a revised chunk)
//! is ONE process where the tool allows it: `dotool` takes it as script
//! lines, `xdotool` chains `key BackSpace… type -- text` (its `cmd_key`
//! stops at the next command word — checked against source). `wtype`
//! and `ydotool` accept a single command each, so they are spawned
//! twice; `uinput` doesn't spawn at all (see `Typer::revise`).

use super::Typer;
use dictation_config::TypingBackend;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

/// `KEY_BACKSPACE` from `linux/input-event-codes.h`.
const YDOTOOL_BACKSPACE_DOWN: &str = "14:1";
const YDOTOOL_BACKSPACE_UP: &str = "14:0";

/// Build the backend named by `engine.typing_backend`.
///
/// `TypingBackend::Uinput` is the in-process keyboard and never reaches
/// here — [`super::create`] keeps it in-house.
pub fn create(backend: TypingBackend) -> io::Result<Box<dyn Typer>> {
    match backend {
        TypingBackend::Wtype => Ok(Box::new(Wtype::new()?)),
        TypingBackend::Dotool => Ok(Box::new(Dotool::new()?)),
        TypingBackend::Ydotool => Ok(Box::new(Ydotool::new()?)),
        TypingBackend::Xdotool => Ok(Box::new(Xdotool::new()?)),
        TypingBackend::Uinput => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "`uinput` is the in-process backend, not an external tool",
        )),
    }
}

/// Locate `program` the way a shell would: a name containing `/` is
/// taken as given, a bare name is searched in `$PATH`.
fn resolve(program: &str) -> io::Result<PathBuf> {
    if program.contains('/') {
        let path = Path::new(program);
        return if path.is_file() {
            Ok(path.to_path_buf())
        } else {
            Err(not_installed(program))
        };
    }
    let path = std::env::var_os("PATH").unwrap_or_default();
    std::env::split_paths(&path)
        .map(|dir| dir.join(program))
        .find(|candidate| candidate.is_file())
        .ok_or_else(|| not_installed(program))
}

fn not_installed(program: &str) -> io::Error {
    io::Error::new(
        io::ErrorKind::NotFound,
        format!("`{program}` is not installed (not in PATH)"),
    )
}

/// A tool that exited non-zero: say which one, and repeat what it wrote
/// to stderr.
fn finish(out: Output, name: &str) -> io::Result<()> {
    if out.status.success() {
        return Ok(());
    }
    let detail = String::from_utf8_lossy(&out.stderr);
    let detail = detail.trim();
    Err(io::Error::other(format!(
        "{name} exited with {}: {}",
        out.status,
        if detail.is_empty() {
            "no stderr"
        } else {
            detail
        }
    )))
}

/// Run `cmd` with `payload` on its stdin. The tools that take text this
/// way need EOF before they start typing.
fn run_stdin(mut cmd: Command, payload: &str, name: &str) -> io::Result<()> {
    let mut child = cmd
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| io::Error::new(e.kind(), format!("{name}: {e}")))?;
    if let Some(mut pipe) = child.stdin.take() {
        // A tool that dies before reading stdin (daemon down, wrong
        // session type) closes the pipe; its exit status is the error
        // worth reporting, so this result is deliberately dropped.
        let _ = pipe.write_all(payload.as_bytes());
    }
    finish(child.wait_with_output()?, name)
}

/// Run `cmd` with everything it needs in argv (xdotool's path).
fn run_argv(mut cmd: Command, name: &str) -> io::Result<()> {
    cmd.stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    finish(cmd.output()?, name)
}

// ---------------------------------------------------------------- wtype

/// `-` is wtype's "read the text from stdin" placeholder.
fn wtype_argv() -> [&'static str; 1] {
    ["-"]
}

/// One `-P`/`-p` pair per character: wtype takes keysyms by name.
fn wtype_backspace_argv(n: usize) -> Vec<String> {
    let mut argv = Vec::with_capacity(4 * n);
    for _ in 0..n {
        argv.extend(["-P", "BackSpace", "-p", "BackSpace"].map(String::from));
    }
    argv
}

pub struct Wtype {
    program: PathBuf,
}

impl Wtype {
    fn new() -> io::Result<Self> {
        let program = resolve("wtype")?;
        // wtype speaks the Wayland virtual-keyboard protocol; without a
        // Wayland socket it could only fail later, mid-dictation.
        if std::env::var_os("WAYLAND_DISPLAY").is_none() {
            return Err(io::Error::other(
                "wtype needs a Wayland session (WAYLAND_DISPLAY is unset)",
            ));
        }
        Ok(Self { program })
    }
}

impl Typer for Wtype {
    fn type_text(&mut self, text: &str) -> io::Result<()> {
        if text.is_empty() {
            return Ok(());
        }
        let mut cmd = Command::new(&self.program);
        cmd.args(wtype_argv());
        run_stdin(cmd, text, "wtype")
    }

    fn backspace(&mut self, n: usize) -> io::Result<()> {
        if n == 0 {
            return Ok(());
        }
        let mut cmd = Command::new(&self.program);
        cmd.args(wtype_backspace_argv(n));
        run_argv(cmd, "wtype")
    }

    fn name(&self) -> &'static str {
        "wtype"
    }
}

// --------------------------------------------------------------- dotool

/// dotool actions that type `text`: one `type` action per source line,
/// with an explicit `key enter` for the newlines that `type` cannot
/// carry (`type` eats the rest of its line, so the text sits on it
/// verbatim, dashes and quotes included).
fn dotool_type_script(text: &str) -> String {
    let mut script = String::new();
    for (i, line) in text.split('\n').enumerate() {
        let line = line.strip_suffix('\r').unwrap_or(line);
        if i > 0 {
            script.push_str("key enter\n");
        }
        if !line.is_empty() {
            script.push_str("type ");
            script.push_str(line);
            script.push('\n');
        }
    }
    script
}

fn dotool_backspace_script(n: usize) -> String {
    "key backspace\n".repeat(n)
}

/// Both halves of a revision in one stdin script: the backspaces,
/// then the replacement's `type` lines.
fn dotool_revise_script(n: usize, text: &str) -> String {
    let mut script = dotool_backspace_script(n);
    script.push_str(&dotool_type_script(text));
    script
}

pub struct Dotool {
    program: PathBuf,
}

impl Dotool {
    fn new() -> io::Result<Self> {
        let program = resolve("dotool")?;
        // dotool writes to /dev/uinput itself: same privilege domain as
        // our own backend, so it fails the same way when that is absent.
        if !Path::new("/dev/uinput").exists() {
            return Err(io::Error::other(
                "dotool needs /dev/uinput (load the `uinput` module and make sure the `input` group can write it)",
            ));
        }
        Ok(Self { program })
    }
}

impl Typer for Dotool {
    fn type_text(&mut self, text: &str) -> io::Result<()> {
        let script = dotool_type_script(text);
        if script.is_empty() {
            return Ok(());
        }
        let cmd = Command::new(&self.program);
        run_stdin(cmd, &script, "dotool")
    }

    fn backspace(&mut self, n: usize) -> io::Result<()> {
        if n == 0 {
            return Ok(());
        }
        let cmd = Command::new(&self.program);
        run_stdin(cmd, &dotool_backspace_script(n), "dotool")
    }

    fn revise(&mut self, n: usize, text: &str) -> io::Result<()> {
        let script = dotool_revise_script(n, text);
        if script.is_empty() {
            return Ok(());
        }
        let cmd = Command::new(&self.program);
        run_stdin(cmd, &script, "dotool")
    }

    fn name(&self) -> &'static str {
        "dotool"
    }
}

// -------------------------------------------------------------- ydotool

/// `type` with `-f -`: read the text from stdin, where ydotool turns off
/// the escape processing it applies to argv text.
fn ydotool_type_argv() -> [&'static str; 3] {
    ["type", "-f", "-"]
}

/// `ydotool key` takes `<keycode>:<pressed>` pairs.
fn ydotool_backspace_argv(n: usize) -> Vec<String> {
    let mut argv = Vec::with_capacity(1 + 2 * n);
    argv.push("key".to_string());
    for _ in 0..n {
        argv.push(YDOTOOL_BACKSPACE_DOWN.to_string());
        argv.push(YDOTOOL_BACKSPACE_UP.to_string());
    }
    argv
}

pub struct Ydotool {
    program: PathBuf,
}

impl Ydotool {
    fn new() -> io::Result<Self> {
        // No daemon probe: `ydotoold` is often socket-activated, so a
        // missing process proves nothing, and ydotool reports a dead
        // socket on its own stderr.
        Ok(Self {
            program: resolve("ydotool")?,
        })
    }
}

impl Typer for Ydotool {
    fn type_text(&mut self, text: &str) -> io::Result<()> {
        if text.is_empty() {
            return Ok(());
        }
        let mut cmd = Command::new(&self.program);
        cmd.args(ydotool_type_argv());
        run_stdin(cmd, text, "ydotool")
    }

    fn backspace(&mut self, n: usize) -> io::Result<()> {
        if n == 0 {
            return Ok(());
        }
        let mut cmd = Command::new(&self.program);
        cmd.args(ydotool_backspace_argv(n));
        run_argv(cmd, "ydotool")
    }

    fn name(&self) -> &'static str {
        "ydotool"
    }
}

// -------------------------------------------------------------- xdotool

/// `--` ends option parsing, so an utterance that starts with `-` is
/// typed as text; `--clearmodifiers` matters because the user has just
/// pressed a hotkey. The text goes in as one argument (xdotool does not
/// insert spaces between arguments in every version).
fn xdotool_type_argv(text: &str) -> Vec<String> {
    ["type", "--clearmodifiers", "--", text]
        .map(String::from)
        .to_vec()
}

/// `xdotool key` consumes the rest of its arguments, so N keysyms in one
/// invocation is N backspaces.
fn xdotool_backspace_argv(n: usize) -> Vec<String> {
    let mut argv = vec!["key".to_string(), "--clearmodifiers".to_string()];
    argv.extend(std::iter::repeat_n("BackSpace".to_string(), n));
    argv
}

/// A revision as ONE argv: `key` presses BackSpace `n` times, then
/// `type` delivers the replacement. `cmd_key` stops consuming at any
/// word `is_command()` recognizes (checked against xdotool's source),
/// so the two commands chain safely — and the text rides after `--` as
/// a single element, so a dictated word can never be mistaken for a
/// command.
fn xdotool_revise_argv(n: usize, text: &str) -> Vec<String> {
    let mut argv = Vec::new();
    if n > 0 {
        argv.extend(xdotool_backspace_argv(n));
    }
    if !text.is_empty() {
        argv.extend(xdotool_type_argv(text));
    }
    argv
}

pub struct Xdotool {
    program: PathBuf,
}

impl Xdotool {
    fn new() -> io::Result<Self> {
        let program = resolve("xdotool")?;
        // XTEST needs an X server. On a Wayland session that means
        // XWayland, so only X11 clients can be reached at all.
        if std::env::var_os("DISPLAY").is_none() {
            return Err(io::Error::other(
                "xdotool needs an X display (DISPLAY is unset) — it can only reach X11/XWayland windows",
            ));
        }
        Ok(Self { program })
    }
}

impl Typer for Xdotool {
    fn type_text(&mut self, text: &str) -> io::Result<()> {
        if text.is_empty() {
            return Ok(());
        }
        let mut cmd = Command::new(&self.program);
        cmd.args(xdotool_type_argv(text));
        run_argv(cmd, "xdotool")
    }

    fn backspace(&mut self, n: usize) -> io::Result<()> {
        if n == 0 {
            return Ok(());
        }
        let mut cmd = Command::new(&self.program);
        cmd.args(xdotool_backspace_argv(n));
        run_argv(cmd, "xdotool")
    }

    fn revise(&mut self, n: usize, text: &str) -> io::Result<()> {
        let argv = xdotool_revise_argv(n, text);
        if argv.is_empty() {
            return Ok(());
        }
        let mut cmd = Command::new(&self.program);
        cmd.args(argv);
        run_argv(cmd, "xdotool")
    }

    fn name(&self) -> &'static str {
        "xdotool"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    /// Private scratch dir per test (pid-tagged so reruns stay clean).
    fn scratch(tag: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("dictation-inject-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("scratch dir");
        dir
    }

    /// A stand-in for wtype/dotool/ydotool/xdotool, so the plumbing can
    /// be exercised without typing into anybody's focused window.
    fn stub(dir: &Path, name: &str, body: &str) -> PathBuf {
        let path = dir.join(name);
        std::fs::write(&path, format!("#!/bin/sh\n{body}\n")).expect("write stub");
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755))
            .expect("chmod stub");
        path
    }

    #[test]
    fn text_backends_keep_the_utterance_out_of_argv() {
        // Only xdotool takes the text as an argument; the others read it
        // from stdin, so dashes, quotes and newlines are never parsed.
        assert_eq!(wtype_argv(), ["-"]);
        assert_eq!(ydotool_type_argv(), ["type", "-f", "-"]);
        assert_eq!(
            xdotool_type_argv("-n 'quoted' text"),
            ["type", "--clearmodifiers", "--", "-n 'quoted' text"]
        );
    }

    #[test]
    fn backspace_is_one_key_per_character() {
        assert!(wtype_backspace_argv(0).is_empty());
        assert_eq!(
            wtype_backspace_argv(2),
            [
                "-P",
                "BackSpace",
                "-p",
                "BackSpace",
                "-P",
                "BackSpace",
                "-p",
                "BackSpace"
            ]
        );
        assert_eq!(
            ydotool_backspace_argv(2),
            ["key", "14:1", "14:0", "14:1", "14:0"]
        );
        assert_eq!(
            xdotool_backspace_argv(2),
            ["key", "--clearmodifiers", "BackSpace", "BackSpace"]
        );
    }

    #[test]
    fn dotool_script_encodes_lines_as_actions() {
        assert_eq!(dotool_type_script(""), "");
        assert_eq!(dotool_type_script("hello world"), "type hello world\n");
        assert_eq!(dotool_type_script("-x \"quoted\""), "type -x \"quoted\"\n");
        // `type` cannot carry a newline, so the line break becomes an
        // action of its own (and CRLF loses the CR).
        assert_eq!(
            dotool_type_script("one\ntwo"),
            "type one\nkey enter\ntype two\n"
        );
        assert_eq!(
            dotool_type_script("one\r\ntwo"),
            "type one\nkey enter\ntype two\n"
        );
        assert_eq!(dotool_type_script("one\n"), "type one\nkey enter\n");
        assert_eq!(
            dotool_type_script("one\n\n\n"),
            "type one\nkey enter\nkey enter\nkey enter\n"
        );
        assert_eq!(
            dotool_backspace_script(3),
            "key backspace\nkey backspace\nkey backspace\n"
        );
    }

    /// A revision must reach the tool as ONE process (backspace then
    /// retype), with an all-empty revision producing nothing at all.
    #[test]
    fn revise_batches_backspace_and_text_into_one_call() {
        assert_eq!(
            dotool_revise_script(2, "new text"),
            "key backspace\nkey backspace\ntype new text\n"
        );
        assert_eq!(dotool_revise_script(0, "new"), "type new\n");
        assert_eq!(dotool_revise_script(1, ""), "key backspace\n");
        assert_eq!(dotool_revise_script(0, ""), "");

        assert_eq!(
            xdotool_revise_argv(2, "new"),
            [
                "key",
                "--clearmodifiers",
                "BackSpace",
                "BackSpace",
                "type",
                "--clearmodifiers",
                "--",
                "new"
            ]
        );
        assert_eq!(
            xdotool_revise_argv(0, "new"),
            ["type", "--clearmodifiers", "--", "new"]
        );
        assert_eq!(
            xdotool_revise_argv(2, ""),
            ["key", "--clearmodifiers", "BackSpace", "BackSpace"]
        );
        assert!(xdotool_revise_argv(0, "").is_empty());
    }

    #[test]
    fn resolve_reports_missing_programs() {
        let err = resolve("./definitely-not-installed").unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::NotFound);
        assert!(err.to_string().contains("not installed"), "{err}");
    }

    #[test]
    fn resolve_takes_explicit_paths_verbatim() {
        let dir = scratch("resolve");
        let program = stub(&dir, "stub", "exit 0");
        assert_eq!(resolve(program.to_str().unwrap()).unwrap(), program);
    }

    #[test]
    fn run_stdin_feeds_the_tool() {
        let dir = scratch("stdin-ok");
        let seen = dir.join("seen");
        let program = stub(&dir, "stub", &format!("cat > '{}'", seen.display()));
        let mut cmd = Command::new(program);
        cmd.arg("ignored");
        run_stdin(cmd, "hello\nworld", "stub").expect("stub succeeds");
        assert_eq!(std::fs::read_to_string(&seen).unwrap(), "hello\nworld");
    }

    #[test]
    fn run_stdin_reports_the_tool_stderr_on_failure() {
        let dir = scratch("stdin-fail");
        let program = stub(
            &dir,
            "stub",
            "cat > /dev/null; echo 'socket is not there' >&2; exit 3",
        );
        let cmd = Command::new(program);
        let err = run_stdin(cmd, "hello", "stub").unwrap_err();
        assert!(err.to_string().contains("stub exited with"), "{err}");
        assert!(err.to_string().contains("socket is not there"), "{err}");
    }

    /// A tool that dies before reading stdin closes the pipe mid-write;
    /// that must not mask the exit status, which is the useful part.
    #[test]
    fn run_stdin_survives_a_tool_that_never_reads_stdin() {
        let dir = scratch("stdin-die");
        let program = stub(&dir, "stub", "echo 'no socket' >&2; exit 1");
        let cmd = Command::new(program);
        let err = run_stdin(cmd, &"x".repeat(200_000), "stub").unwrap_err();
        assert!(err.to_string().contains("no socket"), "{err}");
    }

    #[test]
    fn run_argv_reports_both_outcomes() {
        let dir = scratch("argv");
        let mut ok = Command::new(stub(&dir, "ok", "exit 0"));
        ok.arg("anything");
        run_argv(ok, "stub").expect("stub succeeds");

        let bad = Command::new(stub(&dir, "bad", "echo nope >&2; exit 2"));
        let err = run_argv(bad, "stub").unwrap_err();
        assert!(err.to_string().contains("stub exited with"), "{err}");
        assert!(err.to_string().contains("nope"), "{err}");
    }

    /// `uinput` is the in-process keyboard; only `super::create` may
    /// hand it out.
    #[test]
    fn create_rejects_the_in_process_backend() {
        match create(TypingBackend::Uinput) {
            Ok(_) => panic!("uinput is not an external tool"),
            Err(e) => assert_eq!(e.kind(), io::ErrorKind::InvalidInput),
        }
    }
}
