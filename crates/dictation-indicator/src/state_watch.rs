//! Filesystem watcher that hands session changes back to the GTK thread.

use gtk4::glib;
use notify::{Config, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::cell::Cell;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::Duration;

const DRAIN_MS: u64 = 100;
const BACKSTOP_TICKS: u32 = 3; // 300 ms backstop for missed filesystem events

/// Refresh the view on relevant state-file events, with a low-frequency
/// fallback in case the filesystem notification is lost.
pub fn start(path: PathBuf, refresh: Rc<dyn Fn()>) {
    let active_path = path;
    let pid_path = dictation_core::native_pid_path();
    let watch_dir = active_path
        .parent()
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    let (tx, rx) = std::sync::mpsc::sync_channel::<()>(1);

    std::thread::spawn(move || {
        let mut watcher: RecommendedWatcher = Watcher::new(
            move |result: Result<notify::Event, notify::Error>| {
                let Ok(event) = result else { return };
                if !matches!(
                    event.kind,
                    EventKind::Create(_)
                        | EventKind::Remove(_)
                        | EventKind::Modify(_)
                        | EventKind::Other
                ) {
                    return;
                }
                if event
                    .paths
                    .iter()
                    .any(|changed| changed == &active_path || changed == &pid_path)
                {
                    // Coalesce bursts: the UI only needs to re-read the latest state.
                    let _ = tx.try_send(());
                }
            },
            Config::default(),
        )
        .expect("failed to create file watcher");
        if let Err(error) = watcher.watch(&watch_dir, RecursiveMode::NonRecursive) {
            eprintln!(
                "dictation-indicator: cannot watch {}: {error}",
                watch_dir.display()
            );
        }
        // Keep the watcher alive for the lifetime of the application.
        loop {
            std::thread::park();
        }
    });

    let ticks = Rc::new(Cell::new(0u32));
    glib::timeout_add_local(Duration::from_millis(DRAIN_MS), move || {
        let changed = rx.try_recv().is_ok();
        let next = ticks.get() + 1;
        ticks.set(next);
        if changed || next.is_multiple_of(BACKSTOP_TICKS) {
            refresh();
        }
        glib::ControlFlow::Continue
    });
}
