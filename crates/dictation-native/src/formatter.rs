//! Output normalization shared by all recognition backends.

/// Capitalize the first emitted character and separate later utterances.
pub(crate) struct Formatter {
    first: bool,
    last_had_space: bool,
}

impl Formatter {
    pub(crate) fn new() -> Self {
        Self {
            first: true,
            last_had_space: false,
        }
    }

    pub(crate) fn format(&mut self, raw: &str) -> Option<String> {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return None;
        }
        let mut text = trimmed.to_string();
        if self.first {
            let mut chars = text.chars();
            if let Some(first) = chars.next() {
                text = format!("{}{}", first.to_ascii_uppercase(), chars.as_str());
            }
            self.first = false;
        } else if !self.last_had_space {
            text.insert(0, ' ');
        }
        self.last_had_space = raw.ends_with(' ');
        Some(text)
    }
}
