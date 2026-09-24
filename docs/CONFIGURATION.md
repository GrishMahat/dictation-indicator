# Configuration

The file is created on first run at `~/.config/dictation/config.toml`.
`dictation config` prints the active path. Edit it and restart the indicator
for changes to take effect. `dictation config check` reports invalid settings
and missing model files.

Use the generated file as the starting point; model paths are filled with
locations appropriate for your account.

## Indicator

| Setting | Purpose |
| --- | --- |
| `anchor` | Pill location: top or bottom, left, center, or right. |
| `margin` | Distance from the selected screen edge, in pixels. |
| `width`, `height` | Pill dimensions, in pixels. |
| `background`, `accent`, `text` | CSS color values. |
| `font` | Optional label font; GTK uses its default if unavailable. |

## Engine

| Setting | Purpose |
| --- | --- |
| `backend` | `whisper`, `vosk`, `moonshine`, `zipformer`, or `sensevoice`. |
| `provider` | `auto`, `cpu`, `vulkan`, `cuda`, `hip`, or `metal`; GPU applies to Whisper only. The full app currently targets Linux, so Metal is not a supported app configuration. |
| `threads` | Thread count for Whisper and sherpa-onnx. `0` chooses a backend default; Vosk uses its own policy and requires `0`. |
| `typing_backend` | `uinput` (default), `wtype`, `dotool`, `ydotool`, or `xdotool`. |
| `audio_device` | Optional substring of the input-device name; empty selects the system default. |
| `whisper_model` | Path to a Whisper model file. |
| `model_path` | Path to the Vosk model directory. |
| `moonshine_model_dir` | Path to a sherpa-onnx Moonshine model directory. |
| `zipformer_model_dir` | Path to a sherpa-onnx Zipformer model directory. |
| `sense_voice_model_dir` | Path to a sherpa-onnx SenseVoice model directory. |
| `model_management` | Allows model install, set, and remove commands when enabled. |

## Compute provider

GPU support is compiled in at build time. The default CPU build works with
`provider = "auto"` and selects CPU because no GPU backend is compiled in.
Build with `make release GPU=vulkan` (or `cuda` or `hip`) to add one
supported Linux Whisper GPU backend. Metal exists in the Whisper dependency,
but the rest of the app currently depends on Linux/Wayland components. Only
one provider can be compiled into a binary because
Whisper.cpp exposes a general GPU enable switch rather than a provider
selector.

`auto` chooses the compiled provider if its device is visible and otherwise
uses CPU. A pinned provider fails preflight if the feature is missing or no
device is visible. Whisper.cpp can still fall back to CPU if GPU initialization
fails after that preflight, so check the startup log when investigating a GPU
issue. `dictation doctor` reports the provider candidate and compiled features.

Moonshine, Zipformer, SenseVoice, and Vosk use CPU in this build. A GPU
provider with one of those backends is a configuration error.

## Typing backends

| Backend | Requirements | Notes |
| --- | --- | --- |
| `uinput` | Access to `/dev/uinput`, commonly via the `input` group | Built in; no extra process; US-QWERTY key mapping. |
| `wtype` | Wayland and `wtype` | Uses the virtual-keyboard protocol. |
| `dotool` | `dotool` and `/dev/uinput` | Layout-aware; supports `DOTOOL_XKB_LAYOUT`. |
| `ydotool` | `ydotool` and running `ydotoold` | Uses the ydotool daemon. |
| `xdotool` | X11/XWayland and `xdotool` | On Wayland, reaches X11 clients only. |

If an external tool is missing or stops working, the app logs the failure and
falls back to `uinput`. The selected backend is logged to
`~/.cache/dictation-native.log`.
