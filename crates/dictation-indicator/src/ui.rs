//! GTK window and pill presentation for the dictation indicator.
//!
//! Position, margins, colors, and font come from
//! `~/.config/dictation/config.toml` (defaults to bottom-center).
//! Session lifecycle comes from `dictation-core`; the state watcher
//! keeps this view synchronized with the CLI and native daemon.

use dictation_config::{Horizontal, Vertical};
use dictation_core::{SessionState, control};
use gtk4::glib;
use gtk4::prelude::*;
use gtk4::{
    Application, ApplicationWindow, Box as GtkBox, CssProvider, Image, Label, Orientation, Spinner,
    Stack, StackTransitionType,
};
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

const ACTIVE_OPACITY: f64 = 0.92;
const PULSE_MS: u64 = 700;
const PULSE_DIM_OPACITY: f64 = 0.4;

/// The mic is on and waiting for speech.
const LABEL_LISTENING: &str = "Listening...";
/// The mic is off while the daemon finishes its transcription tail.
const LABEL_DICTATING: &str = "Dictating...";

const STYLE: &str = include_str!("../assets/style.css");

/// Written to `indicator.css_file` on first run; rules here override the
/// built-in stylesheet (loaded at USER priority).
const SEED_CSS: &str = "\
/* dictation indicator — user overrides.
 *
 * Loaded last, at USER priority: anything here beats the built-in
 * stylesheet (colors/font come from ~/.config/dictation/config.toml).
 * Restart the indicator to apply:
 *   systemctl --user restart dictation-indicator
 *
 * Examples (uncomment to try):
 *   .pill  { background-color: #1e1e2e; border-color: #89b4fa; }
 *   .mic   { color: #89b4fa; }
 *   .activity { color: #89b4fa; }
 *   .label { color: #cdd6f4; font-weight: bold; }
 */
";

/// Width of the (first) output. Used to compute centering margins.
/// Layer surfaces without an explicit output land on the compositor's
/// chosen output; for the common single-display case this matches.
fn output_width() -> i32 {
    gtk4::gdk::Display::default()
        .and_then(|display| display.monitors().item(0))
        .and_then(|m| m.downcast::<gtk4::gdk::Monitor>().ok())
        .map(|m| m.geometry().width())
        .unwrap_or(1920)
}

pub fn build(app: &Application, cfg: &dictation_config::Config) {
    let ind = &cfg.indicator;

    let css = STYLE
        .replace("@BG@", &ind.background)
        .replace("@ACCENT@", &ind.accent)
        .replace("@TEXT@", &ind.text)
        .replace("@FONT@", &ind.font);
    let provider = CssProvider::new();
    provider.load_from_data(&css);
    if let Some(display) = gtk4::gdk::Display::default() {
        gtk4::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );

        // User CSS file: seeded on first run, loaded at USER priority so
        // it wins over both the defaults above and the generated sheet.
        if !ind.css_file.is_empty() {
            let css_path = std::path::PathBuf::from(&ind.css_file);
            if !css_path.exists() {
                if let Some(dir) = css_path.parent() {
                    let _ = std::fs::create_dir_all(dir);
                }
                let _ = std::fs::write(&css_path, SEED_CSS);
            }
            if let Ok(user_css) = std::fs::read_to_string(&css_path)
                && !user_css.trim().is_empty()
            {
                let user_provider = CssProvider::new();
                user_provider.load_from_data(&user_css);
                gtk4::style_context_add_provider_for_display(
                    &display,
                    &user_provider,
                    gtk4::STYLE_PROVIDER_PRIORITY_USER,
                );
            }
        }
    }

    let window = ApplicationWindow::builder()
        .application(app)
        .title("dictation-indicator")
        .decorated(false)
        .resizable(false)
        .default_width(ind.width)
        .default_height(ind.height)
        .build();
    window.set_opacity(0.0);

    // Layer-shell: overlay layer, anchored per config, reserves no space,
    // never takes keyboard focus. Center anchors stretch left+right and
    // center the pill child inside the full-width surface.
    window.init_layer_shell();
    window.set_layer(Layer::Overlay);
    window.set_exclusive_zone(-1);
    window.set_keyboard_mode(KeyboardMode::None);
    window.set_namespace("dictation-indicator");

    match ind.anchor.horizontal() {
        Horizontal::Left => {
            window.set_anchor(Edge::Left, true);
            window.set_margin(Edge::Left, ind.margin);
        }
        Horizontal::Right => {
            window.set_anchor(Edge::Right, true);
            window.set_margin(Edge::Right, ind.margin);
        }
        Horizontal::Center => {
            // Don't stretch left+right: gtk4-layer-shell asks the
            // compositor to size the surface but GTK keeps rendering the
            // window at its fixed default width, leaving a narrow surface
            // pinned left. Anchor left only and center via margin instead.
            window.set_anchor(Edge::Left, true);
            let center_margin = (output_width() - ind.width) / 2;
            window.set_margin(Edge::Left, center_margin.max(0));
        }
    }
    match ind.anchor.vertical() {
        Vertical::Top => {
            window.set_anchor(Edge::Top, true);
            window.set_margin(Edge::Top, ind.margin);
        }
        Vertical::Bottom => {
            window.set_anchor(Edge::Bottom, true);
            window.set_margin(Edge::Bottom, ind.margin);
        }
    }

    // Hidden, the layer must be click-through; while shown, clicking
    // stops the active daemon.
    let set_clickable = {
        let window = window.clone();
        let (w, h) = (ind.width, ind.height);
        Rc::new(move |on: bool| {
            if let Some(surface) = window.surface() {
                let region = if on {
                    gtk4::gdk::cairo::Region::create_rectangle(
                        &gtk4::gdk::cairo::RectangleInt::new(0, 0, w, h),
                    )
                } else {
                    gtk4::gdk::cairo::Region::create()
                };
                surface.set_input_region(&region);
            }
        })
    };

    // 0 = hidden, 1 = listening, 2 = finishing transcription. Start
    // outside that range so the first refresh always applies visibility,
    // even when the application starts while dictation is off.
    let view = Rc::new(Cell::new(u8::MAX));
    // The surface only exists once realized, so later flips update its
    // input region through `apply` (driven by the state file).
    window.connect_realize({
        let set_clickable = set_clickable.clone();
        let view = view.clone();
        move |_| set_clickable(matches!(view.get(), 1 | 2))
    });

    // Clicking the visible pill stops dictation — the same path as
    // `dictation end`: SIGTERM (the daemon flushes the in-flight
    // utterance, up to ~8 s), while the watcher keeps the pill visible
    // as "Dictating..." until the daemon exits.
    let gesture = gtk4::GestureClick::new();
    gesture.connect_pressed(move |_, _, _, _| {
        eprintln!("pill clicked — stopping dictation");
        std::thread::spawn(|| {
            if let Err(error) = control::end() {
                eprintln!("dictation-indicator: stop failed: {error}");
            }
        });
    });
    window.add_controller(gesture);

    let pill = GtkBox::new(Orientation::Horizontal, 8);
    // Natural width, centered — works whether the surface is stretched
    // (center anchors) or sized to the window (corner anchors).
    pill.set_halign(gtk4::Align::Center);
    pill.set_valign(gtk4::Align::Center);
    pill.add_css_class("pill");

    let mic = Image::from_icon_name("audio-input-microphone-symbolic");
    mic.set_pixel_size(18);
    mic.add_css_class("mic");
    let spinner = Spinner::new();
    spinner.set_size_request(18, 18);
    spinner.add_css_class("activity");
    let icon = Stack::new();
    icon.set_transition_type(StackTransitionType::Crossfade);
    icon.set_transition_duration(120);
    icon.add_named(&mic, Some("listening"));
    icon.add_named(&spinner, Some("dictating"));
    let text = Label::new(Some(LABEL_LISTENING));
    text.add_css_class("label");
    pill.append(&icon);
    pill.append(&text);
    window.set_child(Some(&pill));

    let path = dictation_config::state_path();
    let pulse_src: Rc<RefCell<Option<glib::SourceId>>> = Rc::default();
    let pulse_dim = Rc::new(Cell::new(false));

    let apply = {
        let mic = mic.clone();
        let spinner = spinner.clone();
        let icon = icon.clone();
        let text = text.clone();
        let pill = pill.clone();
        let view = view.clone();
        let set_clickable = set_clickable.clone();
        let window = window.clone();
        let pulse_src = pulse_src.clone();
        let pulse_dim = pulse_dim.clone();
        Rc::new(move |session: SessionState| {
            let next = match session {
                SessionState::Off => 0,
                SessionState::Listening => 1,
                SessionState::Dictating => 2,
            };
            if next == view.get() {
                return;
            }
            view.set(next);
            let visible = next != 0;
            window.set_opacity(if visible { ACTIVE_OPACITY } else { 0.0 });
            // A fully transparent layer surface can still linger on screen
            // with some compositors. Unmap it while off, and map it again
            // when listening or while final transcription is running.
            window.set_visible(visible);
            set_clickable(visible);
            if visible {
                let listening = session == SessionState::Listening;
                text.set_label(if listening {
                    LABEL_LISTENING
                } else {
                    LABEL_DICTATING
                });
                pill.set_tooltip_text(Some(if listening {
                    "Microphone is on. Click to stop dictation."
                } else {
                    "Microphone is off. Finishing the final transcription."
                }));
                icon.set_visible_child_name(if listening { "listening" } else { "dictating" });
            } else {
                pill.set_tooltip_text(None);
            }
            spinner.set_spinning(session == SessionState::Dictating);
            if session == SessionState::Listening {
                if pulse_src.borrow().is_none() {
                    let mic = mic.clone();
                    let pulse_dim = pulse_dim.clone();
                    let id = glib::timeout_add_local(Duration::from_millis(PULSE_MS), move || {
                        let dim = !pulse_dim.get();
                        pulse_dim.set(dim);
                        mic.set_opacity(if dim { PULSE_DIM_OPACITY } else { 1.0 });
                        glib::ControlFlow::Continue
                    });
                    *pulse_src.borrow_mut() = Some(id);
                }
            } else {
                if let Some(id) = pulse_src.borrow_mut().take() {
                    id.remove();
                }
                mic.set_opacity(1.0);
                pulse_dim.set(false);
            }
        })
    };

    let refresh = {
        let apply = apply.clone();
        Rc::new(move || {
            apply(dictation_core::session_state());
        })
    };

    window.present();
    // Paint the initial state even if the daemon/state file predate us.
    // This runs after present so an initial Off state can unmap the surface.
    refresh();
    super::state_watch::start(path, refresh);
}
