use serde::{Deserialize, Serialize};

/// Where the indicator pill sits on screen.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Anchor {
    TopLeft,
    TopRight,
    TopCenter,
    BottomLeft,
    BottomRight,
    #[default]
    BottomCenter,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Horizontal {
    Left,
    Center,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Vertical {
    Top,
    Bottom,
}

impl Anchor {
    pub fn horizontal(self) -> Horizontal {
        match self {
            Self::TopLeft | Self::BottomLeft => Horizontal::Left,
            Self::TopCenter | Self::BottomCenter => Horizontal::Center,
            Self::TopRight | Self::BottomRight => Horizontal::Right,
        }
    }

    pub fn vertical(self) -> Vertical {
        match self {
            Self::TopLeft | Self::TopRight | Self::TopCenter => Vertical::Top,
            Self::BottomLeft | Self::BottomRight | Self::BottomCenter => Vertical::Bottom,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct IndicatorConfig {
    /// Disable the bundled pill while keeping dictation available.
    pub enabled: bool,
    /// Pill anchor, such as `bottom-center` or `top-right`.
    pub anchor: Anchor,
    /// Inset from the anchored screen edges, in pixels.
    pub margin: i32,
    pub width: i32,
    pub height: i32,
    /// Pill colors.
    pub background: String,
    pub accent: String,
    pub text: String,
    /// Font family for the pill label; the system font is the fallback.
    pub font: String,
    /// Optional user CSS file loaded above the built-in stylesheet.
    pub css_file: String,
}

impl Default for IndicatorConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            anchor: Anchor::BottomCenter,
            margin: 24,
            width: 170,
            height: 44,
            background: "#1e1e2e".into(),
            accent: "#89b4fa".into(),
            text: "#cdd6f4".into(),
            font: "JetBrainsMono Nerd Font".into(),
            css_file: crate::config_dir()
                .join("style.css")
                .to_string_lossy()
                .into_owned(),
        }
    }
}
