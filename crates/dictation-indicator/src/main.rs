//! GTK application entry point for the dictation indicator.

mod state_watch;
mod ui;

use dictation_config::load_or_create;
use dictation_core::cleanup_stale;
use gtk4::Application;
use gtk4::prelude::*;

fn main() {
    let cfg = load_or_create();
    if !cfg.indicator.enabled {
        eprintln!(
            "dictation-indicator: disabled in {}",
            dictation_config::config_path().display()
        );
        return;
    }

    cleanup_stale();
    let app = Application::builder()
        .application_id("com.example.dictation-indicator")
        .build();
    app.connect_activate(move |app| ui::build(app, &cfg));
    std::process::exit(app.run().value());
}
