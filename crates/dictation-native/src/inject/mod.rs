//! Text injection: one trait, five backends.
//!
//! `engine.typing_backend` decides how a finalized utterance reaches the
//! focused window. [`uinput`] is the in-process default — a virtual
//! keyboard we build ourselves, no tool and no daemon. The rest shell
//! out to the system tool of the same name ([`tools`]).
//!
//! A configured tool that turns out to be missing, shadowed by the wrong
//! session type, or otherwise unusable falls back to `uinput` with a log
//! line — at creation, and permanently at the first injection failure —
//! so a stale config or a dead daemon can never leave dictation typing
//! nothing.

mod tools;
mod uinput;

use dictation_config::{EngineConfig, TypingBackend};
use std::io;

/// A text injector.
///
/// `&mut self` because the in-process backend owns a device handle.
pub trait Typer {
    /// Type `text` into whatever window currently has focus.
    fn type_text(&mut self, text: &str) -> io::Result<()>;

    /// Delete `n` characters to the left of the caret.
    ///
    /// Whisper's rolling chunks revise text that was already typed, so
    /// every backend has to be able to take it back.
    fn backspace(&mut self, n: usize) -> io::Result<()>;

    /// Take back `n` characters and type `text` in their place — one
    /// rolling-chunk revision, undoing first if Whisper revised
    /// something already typed. The default performs the two steps
    /// separately; backends whose tool accepts several commands in one
    /// invocation (dotool, xdotool) override this to halve the process
    /// spawns mid-sentence. `uinput` is in-process and gains nothing.
    fn revise(&mut self, n: usize, text: &str) -> io::Result<()> {
        if n > 0 {
            self.backspace(n)?;
        }
        if !text.is_empty() {
            self.type_text(text)?;
        }
        Ok(())
    }

    /// Backend name, for logs.
    fn name(&self) -> &'static str;
}

/// Build the typer for `engine.typing_backend`, falling back to `uinput`
/// when the configured tool cannot be used — at creation time (missing
/// binary, wrong session type) and, armed inside [`Fallback`], at the
/// first injection failure (e.g. `ydotoold` died mid-session).
pub fn create(engine: &EngineConfig) -> io::Result<Box<dyn Typer>> {
    let backend = engine.typing_backend;
    if backend == TypingBackend::Uinput {
        return Ok(Box::new(uinput::Uinput::new()?));
    }
    let primary = match tools::create(backend) {
        Ok(typer) => typer,
        Err(tool_err) => {
            crate::log(&format!(
                "typing backend `{}` unavailable: {tool_err}",
                backend.as_str()
            ));
            match uinput::Uinput::new() {
                Ok(typer) => {
                    crate::log("falling back to the uinput typing backend");
                    return Ok(Box::new(typer));
                }
                Err(uinput_err) => {
                    return Err(io::Error::other(format!(
                        "`{}`: {tool_err}; the uinput fallback is unusable too: {uinput_err}",
                        backend.as_str()
                    )));
                }
            }
        }
    };
    // The tool created fine — still arm a runtime safety net for later.
    Ok(Box::new(Fallback {
        primary,
        fallback: None,
        switched: false,
        make_fallback: Box::new(|| uinput::Uinput::new().map(|u| Box::new(u) as Box<dyn Typer>)),
    }))
}

/// The configured tool with a permanent `uinput` safety net.
///
/// The fallback device is built lazily — only the first real failure
/// pays for it — and from that moment on every keystroke goes straight
/// to `uinput`: a tool that failed once (daemon down, socket gone) must
/// not be retried every utterance while text silently disappears.
struct Fallback {
    primary: Box<dyn Typer>,
    /// Built on first failure by `make_fallback`.
    fallback: Option<Box<dyn Typer>>,
    /// One-way: after the first failure the primary is never touched.
    switched: bool,
    make_fallback: Box<dyn FnMut() -> io::Result<Box<dyn Typer>>>,
}

impl Fallback {
    fn run(&mut self, mut op: impl FnMut(&mut dyn Typer) -> io::Result<()>) -> io::Result<()> {
        if !self.switched {
            match op(&mut *self.primary) {
                Ok(()) => return Ok(()),
                Err(e) => {
                    crate::log(&format!(
                        "typing backend `{}` failed: {e}; permanently switching to uinput",
                        self.primary.name()
                    ));
                    self.switched = true;
                }
            }
        }
        if self.fallback.is_none() {
            self.fallback = Some((self.make_fallback)()?);
        }
        op(self.fallback.as_deref_mut().expect("fallback just built"))
    }
}

impl Typer for Fallback {
    fn type_text(&mut self, text: &str) -> io::Result<()> {
        self.run(|t| t.type_text(text))
    }

    fn backspace(&mut self, n: usize) -> io::Result<()> {
        self.run(|t| t.backspace(n))
    }

    fn name(&self) -> &'static str {
        if self.switched {
            self.fallback
                .as_ref()
                .map_or_else(|| self.primary.name(), |f| f.name())
        } else {
            self.primary.name()
        }
    }
}

/// One-shot injection (used by the hidden `dictation __type` debug cmd).
pub fn type_text_once(engine: &EngineConfig, text: &str) -> io::Result<()> {
    create(engine)?.type_text(text)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    /// Records every operation; `fail` simulates a dead tool.
    struct Spy {
        label: &'static str,
        fail: bool,
        calls: Rc<RefCell<Vec<String>>>,
    }

    impl Typer for Spy {
        fn type_text(&mut self, text: &str) -> io::Result<()> {
            self.calls.borrow_mut().push(format!("type {text}"));
            self.result()
        }

        fn backspace(&mut self, n: usize) -> io::Result<()> {
            self.calls.borrow_mut().push(format!("back {n}"));
            self.result()
        }

        fn name(&self) -> &'static str {
            self.label
        }
    }

    impl Spy {
        fn result(&self) -> io::Result<()> {
            if self.fail {
                Err(io::Error::other("daemon is gone"))
            } else {
                Ok(())
            }
        }
    }

    fn spy(label: &'static str, fail: bool) -> (Spy, Rc<RefCell<Vec<String>>>) {
        let calls = Rc::new(RefCell::new(Vec::new()));
        (
            Spy {
                label,
                fail,
                calls: calls.clone(),
            },
            calls,
        )
    }

    /// A factory that must never run: used where the fallback is either
    /// unnecessary or already built.
    fn never() -> io::Result<Box<dyn Typer>> {
        panic!("fallback must stay lazy")
    }

    #[test]
    fn default_revise_backspaces_then_types() {
        let (mut tool, calls) = spy("tool", false);
        tool.revise(2, "next").unwrap();
        assert_eq!(calls.borrow().as_slice(), ["back 2", "type next"]);
        // Nothing to undo and nothing to add: no operations at all.
        tool.revise(0, "").unwrap();
        assert_eq!(calls.borrow().len(), 2);
    }

    #[test]
    fn healthy_primary_never_touches_the_fallback() {
        let (primary, pcalls) = spy("tool", false);
        let mut fb = Fallback {
            primary: Box::new(primary),
            fallback: None,
            switched: false,
            make_fallback: Box::new(never),
        };
        fb.type_text("hello").unwrap();
        fb.backspace(2).unwrap();
        assert!(!fb.switched);
        assert_eq!(fb.name(), "tool");
        assert_eq!(pcalls.borrow().as_slice(), ["type hello", "back 2"]);
    }

    #[test]
    fn first_failure_switches_permanently_to_the_fallback() {
        let (primary, pcalls) = spy("tool", true);
        let (fallback, fcalls) = spy("uinput", false);
        let mut fb = Fallback {
            primary: Box::new(primary),
            fallback: Some(Box::new(fallback)),
            switched: false,
            make_fallback: Box::new(never),
        };
        // The text is attempted on the tool, then lands via the
        // fallback: a dead daemon loses nothing.
        fb.type_text("hello").unwrap();
        assert_eq!(pcalls.borrow().as_slice(), ["type hello"]);
        assert_eq!(fcalls.borrow().as_slice(), ["type hello"]);
        assert!(fb.switched);
        assert_eq!(fb.name(), "uinput");
        // ...and the broken tool is never called again.
        fb.type_text("world").unwrap();
        fb.backspace(1).unwrap();
        assert_eq!(pcalls.borrow().len(), 1);
        assert_eq!(
            fcalls.borrow().as_slice(),
            ["type hello", "type world", "back 1"]
        );
    }

    #[test]
    fn a_failing_fallback_surfaces_the_error() {
        let (primary, _) = spy("tool", true);
        let (fallback, _) = spy("uinput", true);
        let mut fb = Fallback {
            primary: Box::new(primary),
            fallback: Some(Box::new(fallback)),
            switched: false,
            make_fallback: Box::new(never),
        };
        let err = fb.type_text("attempted").unwrap_err();
        assert!(err.to_string().contains("daemon is gone"), "{err}");
    }

    #[test]
    fn the_fallback_device_is_built_once_on_first_failure() {
        let (primary, _) = spy("tool", true);
        let (fallback, fcalls) = spy("uinput", false);
        let builds = Rc::new(RefCell::new(0usize));
        let count = builds.clone();
        let mut made = Some(fallback);
        let mut fb = Fallback {
            primary: Box::new(primary),
            fallback: None,
            switched: false,
            make_fallback: Box::new(move || {
                *count.borrow_mut() += 1;
                // Taken, not moved: an FnMut may in principle run more
                // than once (our impl runs it exactly once).
                let spy = made.take().expect("fallback built at most once");
                Ok(Box::new(spy) as Box<dyn Typer>)
            }),
        };
        fb.type_text("a").unwrap();
        fb.type_text("b").unwrap();
        assert_eq!(*builds.borrow(), 1);
        assert_eq!(fcalls.borrow().as_slice(), ["type a", "type b"]);
    }
}
