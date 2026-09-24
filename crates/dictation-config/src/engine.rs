use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// How recognized text reaches the focused window.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum TypingBackend {
    /// Built-in virtual keyboard; requires `/dev/uinput` access.
    #[default]
    Uinput,
    /// Wayland virtual-keyboard protocol.
    Wtype,
    /// Layout-aware uinput tool.
    Dotool,
    /// `ydotool` plus its daemon.
    Ydotool,
    /// X11/XTEST, including XWayland clients.
    Xdotool,
}

impl TypingBackend {
    /// Spelling used in the config file.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Uinput => "uinput",
            Self::Wtype => "wtype",
            Self::Dotool => "dotool",
            Self::Ydotool => "ydotool",
            Self::Xdotool => "xdotool",
        }
    }
}

/// Where recognition runs.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Provider {
    /// Probe the machine once at startup and use the best device available.
    #[default]
    Auto,
    /// Force CPU inference.
    Cpu,
    /// GPU through Vulkan (any vendor; works on AMD/Intel/NVIDIA).
    Vulkan,
    /// GPU through CUDA (NVIDIA only).
    Cuda,
    /// GPU through HIP/ROCm (AMD discrete GPUs).
    Hip,
    /// GPU through Metal (Apple only).
    Metal,
}

impl Provider {
    /// Spelling used in the config file.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Cpu => "cpu",
            Self::Vulkan => "vulkan",
            Self::Cuda => "cuda",
            Self::Hip => "hip",
            Self::Metal => "metal",
        }
    }

    /// Whether this pins a GPU device (as opposed to `auto` or `cpu`).
    pub fn is_gpu(self) -> bool {
        !matches!(self, Self::Auto | Self::Cpu)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct EngineConfig {
    /// Recognition backend: `whisper`, `vosk`, `moonshine`, `zipformer`, or `sensevoice`.
    pub backend: String,
    /// Compute provider: `auto`, `cpu`, `vulkan`, `cuda`, `hip`, or `metal`.
    /// Only the Whisper backend has GPU paths today; others run on CPU.
    pub provider: Provider,
    /// Whisper/sherpa-onnx threads; `0` selects a backend default. Vosk uses its own policy.
    pub threads: usize,
    /// Vosk model directory.
    pub model_path: String,
    /// Whisper ggml model file.
    pub whisper_model: String,
    /// Directory containing sherpa-onnx Moonshine Tiny English int8 models.
    pub moonshine_model_dir: String,
    /// Directory containing sherpa-onnx streaming Zipformer English models.
    pub zipformer_model_dir: String,
    /// Directory containing a sherpa-onnx SenseVoice model package.
    pub sense_voice_model_dir: String,
    /// Backend that injects recognized text into the focused window.
    pub typing_backend: TypingBackend,
    /// Optional input-device name substring; empty selects the system default.
    pub audio_device: String,
    /// Allow model changes and downloads through the CLI.
    pub model_management: bool,
}

impl Default for EngineConfig {
    fn default() -> Self {
        let data = dirs::data_dir().unwrap_or_else(|| PathBuf::from("/tmp"));
        let vosk = data.join("vocalinux/models/vosk-model-small-en-us-0.15");
        let whisper = data.join("dictation/models/ggml-base.en-q5_1.bin");
        let moonshine = data.join("dictation/models/sherpa-onnx-moonshine-tiny-en-int8");
        let zipformer =
            data.join("dictation/models/sherpa-onnx-streaming-zipformer-en-20M-2023-02-17");
        let sense_voice =
            data.join("dictation/models/sherpa-onnx-sense-voice-zh-en-ja-ko-yue-int8-2025-09-09");
        Self {
            backend: "whisper".into(),
            provider: Provider::Auto,
            threads: 0,
            model_path: vosk.to_string_lossy().into_owned(),
            whisper_model: whisper.to_string_lossy().into_owned(),
            moonshine_model_dir: moonshine.to_string_lossy().into_owned(),
            zipformer_model_dir: zipformer.to_string_lossy().into_owned(),
            sense_voice_model_dir: sense_voice.to_string_lossy().into_owned(),
            typing_backend: TypingBackend::Uinput,
            audio_device: String::new(),
            model_management: true,
        }
    }
}
