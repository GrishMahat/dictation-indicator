# dictation

A dictation toolchain for Wayland, built as a Cargo workspace. One key
starts dictation, the same key stops it; a small pill tells you what it
is doing — `Listening...` while the mic is on, then `Dictating...`
while the daemon finishes the final transcription after toggle-off.
The pill hides after that work finishes; clicking it also stops dictation.

```
crates/dictation-config      # TOML config + XDG paths, shared by everything
crates/dictation-core        # session state, serialized controls, stale guard
crates/dictation-cli         # `dictation begin|end|toggle|status`  ← keybind this
crates/dictation-native      # cpal capture → vosk/whisper → typing (no Python)
crates/dictation-indicator   # GTK4 layer-shell pill (bottom-center by default)
systemd/                     # user unit for indicator autostart
```

IPC is two flags in `$XDG_CACHE_HOME` (usually `~/.cache/`):
`dictation-active` (exists == mic on) and `dictation-busy` (exists ==
a speech segment is open — the daemon creates and removes it as you
speak and pause). The CLI and daemon write them; the indicator watches
the directory with `notify`.

## Dependencies (Arch)

Indicator:

```bash
sudo pacman -S gtk4 gtk4-layer-shell
```

The native backends (`whisper`, `vosk`) have no runtime package deps —
models live under `~/.local/share/dictation/models/` (Whisper) and your
vosk model dir (Vosk). Building whisper-rs needs `cmake`, `g++` and
`libclang` (bindgen); the built-in typing backend needs membership in the
`input` group for `/dev/uinput`.

Only for the typing backends you select in `engine.typing_backend`
(`uinput` needs none of these):

```bash
sudo pacman -S wtype        # Wayland sessions
sudo pacman -S ydotool      # needs the ydotoold daemon
sudo pacman -S xdotool      # X11 / XWayland only
# dotool ships in the AUR, and needs /dev/uinput like the built-in backend
```

The mic and transcription indicators use GTK symbolic icons and need no
special font. The configured font applies to the label; GTK falls back to
the system default if it is unavailable.

## Build

```bash
cargo build --release
```

Binaries: `target/release/dictation` and
`target/release/dictation-indicator`. The shared sherpa-onnx runtime is
copied beside the binaries by Cargo.

Install:

```bash
install -Dm755 target/release/dictation ~/.local/bin/dictation
install -Dm755 target/release/*.so ~/.local/bin/
install -Dm755 target/release/dictation-indicator ~/.local/bin/dictation-indicator
install -Dm644 systemd/dictation-indicator.service ~/.config/systemd/user/dictation-indicator.service
systemctl --user daemon-reload
systemctl --user enable --now dictation-indicator
```

## Your keybind

One command, both directions:

```
# Hyprland example
bind = SUPER, grave, exec, dictation toggle
```

`dictation begin` / `dictation end` exist for scripts. `dictation
status` reports whether the mic is listening, final audio is being
dictated, or the session is off. `dictation config` prints the config path.
Run `dictation config check` to catch misspelled settings, invalid layout
dimensions, and missing model paths before starting a session.

## Config

`~/.config/dictation/config.toml`, created with defaults on first run:

```toml
[indicator]
anchor = "bottom-center"   # top-left, top-center, top-right,
                           # bottom-left, bottom-center, bottom-right
margin = 24                # inset from the edges, px
width = 170
height = 44
background = "#1e1e2e"
accent = "#89b4fa"
text = "#cdd6f4"
font = "JetBrainsMono Nerd Font" # optional; used for the label

[engine]
backend = "whisper"        # whisper | vosk | moonshine | zipformer | sensevoice
typing_backend = "uinput"  # "uinput" (default, built in) | "wtype"
                           # | "dotool" | "ydotool" | "xdotool"
whisper_model = "/home/you/.local/share/dictation/models/ggml-base.en-q5_1.bin"
model_path = "/home/you/.local/share/vocalinux/models/vosk-model-small-en-us-0.15"
moonshine_model_dir = "/home/you/.local/share/dictation/models/sherpa-onnx-moonshine-tiny-en-int8"
zipformer_model_dir = "/home/you/.local/share/dictation/models/sherpa-onnx-streaming-zipformer-en-20M-2023-02-17"
sense_voice_model_dir = "/home/you/.local/share/dictation/models/sherpa-onnx-sense-voice-zh-en-ja-ko-yue-int8-2025-09-09"
audio_device = ""          # mic name substring; empty = system default
model_management = true     # allow dictation model install/set/remove
```

Restart the indicator after editing.

### Model management and diagnostics

`dictation model list` shows each model's family, language, approximate download
size, installed/active status, and the next command to run. Filter the catalog
with `dictation model list --backend vosk`, or show only local models with
`dictation model list --installed`. `dictation model info <name>` shows full
details, the download source, and install/select/remove commands.
`dictation model check` checks the active model; add a catalog name to check
that model's required files.
Install downloads with `curl` and resumes interrupted transfers when run again:

```bash
dictation model install whisper-base.en
dictation model set whisper-base.en
dictation model recommend
dictation model benchmark /path/to/short-sample.wav --model whisper-tiny.en
dictation model remove whisper-tiny.en
dictation model management off  # disable CLI install/set/remove
dictation doctor
```

The catalog includes Whisper, Vosk, Moonshine, SenseVoice, and Zipformer
models. Vosk offers many languages and both small and large models; Moonshine
includes English and multilingual releases; SenseVoice supports English,
Chinese, Cantonese, Japanese, and Korean. Whisper includes multiple sizes and
quantizations. Larger models can require multiple gigabytes of memory and disk.
Custom Whisper model files can be selected with
`dictation model set /path/to/model.bin`. Recommendations use currently
available system memory as a rough starting point; benchmark with representative
audio on the target machine before switching models. Managed downloads live in
`~/.local/share/dictation/models/`.

### Moonshine and Zipformer model files

Install the CPU int8 model packages in the default model directory:

```bash
mkdir -p ~/.local/share/dictation/models
cd ~/.local/share/dictation/models
curl -fL https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/sherpa-onnx-moonshine-tiny-en-int8.tar.bz2 | tar -xj
curl -fL https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/sherpa-onnx-streaming-zipformer-en-20M-2023-02-17.tar.bz2 | tar -xj
```

Then set `engine.backend` to `moonshine` or `zipformer`. Moonshine emits
rolling revisions by decoding the current bounded utterance buffer;
Zipformer decodes continuously and closes a phrase after a short pause.
Both run on CPU and remain English-only.

## Typing backends

`engine.typing_backend` picks how recognized text reaches the focused
window. All five get the same input — whole utterances, plus backspaces
when Whisper revises a word mid-sentence — and they differ only in how
they deliver it:

| `typing_backend` | needs | session | notes |
| --- | --- | --- | --- |
| `uinput` (default) | `input` group | any | built in, no tool, no daemon; US-QWERTY only |
| `wtype` | `wtype` | Wayland | virtual-keyboard protocol; text goes in on stdin |
| `dotool` | `dotool`, `/dev/uinput` | any | layout-aware (`DOTOOL_XKB_LAYOUT`) |
| `ydotool` | `ydotool` + `ydotoold` | any | text goes in on stdin, escapes off |
| `xdotool` | `xdotool` | X11 / XWayland | on Wayland it only reaches X11 clients |

If the selected tool is missing, or belongs to the wrong kind of session
(e.g. `xdotool` with no `$DISPLAY`) — or simply stops working mid-session
(the `ydotoold` daemon died) — dictation logs why and falls back to
`uinput`, permanently from the first failed keystroke on, so neither a
config typo nor a dead daemon can leave you talking to a mute
microphone. Check what actually got picked:

```bash
tail -n 20 ~/.cache/dictation-native.log | grep typing
# dictation-native: typing via `wtype`
```

Per-key pacing is the tool's own (`ydotool` 20 ms, `xdotool` 12 ms,
`dotool` 2 ms); only the built-in backend applies its ~4 ms delay, which
some toolkits need to avoid dropping bursts.

During streaming, a revision (backspace + retype of a revised chunk)
reaches `dotool` as one script and `xdotool` as one chained invocation;
`wtype` and `ydotool` accept a single command each, so they are spawned
twice — and `uinput` doesn't spawn anything.

## Reliability notes

- `notify` watches only the active and daemon pid files. Changes are
  coalesced and drained on a 100 ms timer, with a 300 ms state-check
  backstop for missed filesystem events.
- Stale state/busy files (`kill -9`, reboot, force-killed daemon) are
  discarded on startup unless an engine process is actually running.
- Start, stop, and toggle share a process lock. Startup waits for the
  daemon pid before releasing it, so rapid or simultaneous key presses
  cannot launch duplicate engines.
- Microphone audio uses a bounded 1.6 s queue so a slow decode cannot
  leave dictation several seconds behind. If decoding falls behind far
  enough to drop audio, the daemon records that in
  `~/.cache/dictation-native.log`.
- A microphone stream error stops the failed session, flushes any usable
  final audio, and clears the active state instead of leaving a dead
  session marked as listening.
- The pill shows `Listening...` while the mic is on, then remains visible
  as `Dictating...` while the daemon flushes the final utterance. It hides
  after the daemon exits. Clicking the pill follows the same stop path.
- Turning off never leaves dictation running: `end` waits ~8 s for the
  tail flush, then force-kills the daemon, so nothing can still be
  typing after "off". (Normal toggle-off completes the last words by
  design; a separate hard-stop "force close" command is planned.)
- `dictation begin` detaches the engine with `setsid`, so a keybind
  invocation returns immediately.

## Roadmap

- **Phase 1 (done)**: workspace, config, `dictation toggle`,
  config-driven indicator position/colors.
- **Phase 2 (done)**: native engine, no Python — cpal capture in,
  typing out, both halves selectable:
  - recognition via `engine.backend`:
    - `whisper` — whisper-rs + whisper.cpp, Base.en Q5 (default; best
      accuracy) with rolling-chunk streaming: first text after ~1 s of
      speech and the caller types only the prefix-diff, so text appears
      live while you're still talking; finalizes on a short pause,
    - `vosk` — streaming recognizer, tiny models (ultra-low-resource
      / existing-vosk-model compatibility),
    - `zipformer` — streaming English int8 model through sherpa-onnx,
    - `moonshine` — English and multilingual models with bounded offline updates,
    - `sensevoice` — multilingual SenseVoiceSmall int8 via sherpa-onnx,
  - typing via `engine.typing_backend`: `uinput` (built in) plus the
    external `wtype`, `dotool`, `ydotool` and `xdotool`.
- **Phase 3 (done)**: managed model catalog, resumable downloads, persistent
  model selection, model checks/recommendations, and diagnostics; added Vosk
  language models, Moonshine v2, and SenseVoice support.

## Whisper latency (what makes it feel live)

whisper.cpp pads every transcription to a fixed 30 s encoder window —
a fixed ~7 s of CPU per call on this class of hardware no matter how
short the audio, which made the first word land 5–10 s in. The daemon
sizes the window per call instead (`audio_ctx` = buffer + 1 s guard),
so decode cost follows what was actually said: on the JFK sample, the
runtime-sized 1 s decode took about 0.24 s at 4 threads, putting first
text around 1.25 s after speech starts on this machine. Use
`dictation benchmark-tune [wav]` to compare encoder windows and thread counts
with your model and CPU. On this laptop, 4 threads beat 1 and 2, while 8
threads were much slower.

To compare model files against the configured model on the same recording:

```bash
dictation benchmark /path/to/speech.wav \
  --model ~/.local/share/dictation/models/ggml-tiny.en-q5_1.bin \
  --reference-file /path/to/speech.txt
```

Use `--backend moonshine --backend zipformer` in the same command to
compare those backends on identical audio and reference text.

The configured model is included as the baseline; repeat `--model PATH`
for more candidates. Each model runs in its own process and reports cold
model-load time, decode time and realtime factor, resident memory before
and after loading, and peak RSS. On Linux, peak RSS comes from
`/proc/self/status`. The reference transcript adds word error rate (WER);
without one, the full recognized text is printed for manual comparison.
WER ignores case and punctuation. Use `dictation benchmark-tune [wav]`
for the earlier encoder-window and thread-count sweep.

Timing knobs are constants in
`crates/dictation-native/src/segmenter/whisper.rs`: `FIRST_CHUNK_MS` (1000 —
when the first text appears), `CADENCE_WALL` (1500 — minimum gap
between rolling decodes), `SILENCE_END_MS` (500 — pause that ends an
utterance), `MAX_SEGMENT_MS` (6000 — force-split monologues). If
mid-sentence updates feel slow, lower `CADENCE_WALL` toward 900 and
`MAX_SEGMENT_MS` toward 4000 (costs more CPU while speaking).

## Dev

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
```
