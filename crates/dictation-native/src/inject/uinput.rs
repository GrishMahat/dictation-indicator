//! The default backend: text injection via uinput (no ydotool, no
//! daemon). Builds a virtual keyboard once and translates printable
//! ASCII into key events with shift handling. Requires membership in the
//! `input` group for `/dev/uinput`.

use super::Typer;
use evdev::{AttributeSet, EventType, InputEvent, Key, uinput::VirtualDeviceBuilder};
use std::io;
use std::time::Duration;

/// US-QWERTY: printable ASCII char → (keycode, needs shift).
/// Unsupported chars are skipped (never crash a dictation session over
/// an emoji).
fn lookup(c: char) -> Option<(Key, bool)> {
    let k = match c {
        'a'..='z' => (letter_key(c), false),
        'A'..='Z' => (letter_key(c.to_ascii_lowercase()), true),
        '1'..='9' => (digit_key(c), false),
        '0' => (Key::KEY_0, false),
        ' ' => (Key::KEY_SPACE, false),
        '\n' | '\r' => (Key::KEY_ENTER, false),
        '\t' => (Key::KEY_TAB, false),
        '.' => (Key::KEY_DOT, false),
        ',' => (Key::KEY_COMMA, false),
        '/' => (Key::KEY_SLASH, false),
        ';' => (Key::KEY_SEMICOLON, false),
        '\'' => (Key::KEY_APOSTROPHE, false),
        '[' => (Key::KEY_LEFTBRACE, false),
        ']' => (Key::KEY_RIGHTBRACE, false),
        '-' => (Key::KEY_MINUS, false),
        '=' => (Key::KEY_EQUAL, false),
        '`' => (Key::KEY_GRAVE, false),
        '\\' => (Key::KEY_BACKSLASH, false),
        // Shifted digits.
        '!' => (Key::KEY_1, true),
        '@' => (Key::KEY_2, true),
        '#' => (Key::KEY_3, true),
        '$' => (Key::KEY_4, true),
        '%' => (Key::KEY_5, true),
        '^' => (Key::KEY_6, true),
        '&' => (Key::KEY_7, true),
        '*' => (Key::KEY_8, true),
        '(' => (Key::KEY_9, true),
        ')' => (Key::KEY_0, true),
        // Shifted punctuation.
        ':' => (Key::KEY_SEMICOLON, true),
        '"' => (Key::KEY_APOSTROPHE, true),
        '<' => (Key::KEY_COMMA, true),
        '>' => (Key::KEY_DOT, true),
        '?' => (Key::KEY_SLASH, true),
        '~' => (Key::KEY_GRAVE, true),
        '_' => (Key::KEY_MINUS, true),
        '+' => (Key::KEY_EQUAL, true),
        _ => return None,
    };
    Some(k)
}

fn letter_key(c: char) -> Key {
    const LETTERS: [(char, Key); 26] = [
        ('a', Key::KEY_A),
        ('b', Key::KEY_B),
        ('c', Key::KEY_C),
        ('d', Key::KEY_D),
        ('e', Key::KEY_E),
        ('f', Key::KEY_F),
        ('g', Key::KEY_G),
        ('h', Key::KEY_H),
        ('i', Key::KEY_I),
        ('j', Key::KEY_J),
        ('k', Key::KEY_K),
        ('l', Key::KEY_L),
        ('m', Key::KEY_M),
        ('n', Key::KEY_N),
        ('o', Key::KEY_O),
        ('p', Key::KEY_P),
        ('q', Key::KEY_Q),
        ('r', Key::KEY_R),
        ('s', Key::KEY_S),
        ('t', Key::KEY_T),
        ('u', Key::KEY_U),
        ('v', Key::KEY_V),
        ('w', Key::KEY_W),
        ('x', Key::KEY_X),
        ('y', Key::KEY_Y),
        ('z', Key::KEY_Z),
    ];
    LETTERS
        .iter()
        .find(|(ch, _)| *ch == c)
        .map(|(_, k)| *k)
        .unwrap_or(Key::KEY_RESERVED)
}

fn digit_key(c: char) -> Key {
    const DIGITS: [(char, Key); 9] = [
        ('1', Key::KEY_1),
        ('2', Key::KEY_2),
        ('3', Key::KEY_3),
        ('4', Key::KEY_4),
        ('5', Key::KEY_5),
        ('6', Key::KEY_6),
        ('7', Key::KEY_7),
        ('8', Key::KEY_8),
        ('9', Key::KEY_9),
    ];
    DIGITS
        .iter()
        .find(|(ch, _)| *ch == c)
        .map(|(_, k)| *k)
        .unwrap_or(Key::KEY_RESERVED)
}

struct Keyboard {
    device: evdev::uinput::VirtualDevice,
}

impl Keyboard {
    fn new() -> io::Result<Self> {
        let mut keys = AttributeSet::<Key>::new();
        keys.insert(Key::KEY_LEFTSHIFT);
        // Used by prefix-diff correction (Whisper revises words).
        keys.insert(Key::KEY_BACKSPACE);
        // Register every key any printable char can produce.
        for cp in 0x20u32..=0x7e {
            if let Some((k, _)) = char::from_u32(cp).and_then(lookup) {
                keys.insert(k);
            }
        }
        let device = VirtualDeviceBuilder::new()?
            .name("dictation-native virtual keyboard")
            .with_keys(&keys)?
            .build()?;
        Ok(Self { device })
    }

    fn tap(&mut self, key: Key, shift: bool) -> io::Result<()> {
        let t = EventType::KEY;
        if shift {
            self.device
                .emit(&[InputEvent::new(t, Key::KEY_LEFTSHIFT.code(), 1)])?;
        }
        self.device.emit(&[
            InputEvent::new(t, key.code(), 1),
            InputEvent::new(t, key.code(), 0),
        ])?;
        if shift {
            self.device
                .emit(&[InputEvent::new(t, Key::KEY_LEFTSHIFT.code(), 0)])?;
        }
        Ok(())
    }

    /// Type `text` into whatever window is focused.
    fn type_text(&mut self, text: &str) -> io::Result<()> {
        for c in text.chars() {
            if let Some((key, shift)) = lookup(c) {
                if key == Key::KEY_RESERVED {
                    continue;
                }
                self.tap(key, shift)?;
                // Human-ish pace: some toolkits drop faster key bursts.
                std::thread::sleep(Duration::from_millis(4));
            }
        }
        Ok(())
    }

    /// Delete `n` characters (used when a rolling transcription revises
    /// text we already typed).
    fn backspace(&mut self, n: usize) -> io::Result<()> {
        for _ in 0..n {
            self.tap(Key::KEY_BACKSPACE, false)?;
            std::thread::sleep(Duration::from_millis(4));
        }
        Ok(())
    }
}

/// The default backend: our own `/dev/uinput` virtual keyboard.
pub struct Uinput {
    keyboard: Keyboard,
}

impl Uinput {
    pub fn new() -> io::Result<Self> {
        Keyboard::new()
            .map(|keyboard| Self { keyboard })
            .map_err(|e| {
                io::Error::new(
                    e.kind(),
                    format!("{e} — uinput needs /dev/uinput and `input` group membership"),
                )
            })
    }
}

impl Typer for Uinput {
    fn type_text(&mut self, text: &str) -> io::Result<()> {
        self.keyboard.type_text(text)
    }

    fn backspace(&mut self, n: usize) -> io::Result<()> {
        self.keyboard.backspace(n)
    }

    fn name(&self) -> &'static str {
        "uinput"
    }
}
