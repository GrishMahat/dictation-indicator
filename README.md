# dictation

[![License: GPL v3+](https://img.shields.io/badge/license-GPLv3%2B-blue.svg)](LICENSE)
![Platform: Linux](https://img.shields.io/badge/platform-Linux%20%7C%20Wayland-89b4fa)
![Inference: Local](https://img.shields.io/badge/inference-local%20%28offline%29-89b4fa)

A small, local dictation app I made for an old laptop. Press a shortcut to
record, speak, and have the words typed into the focused app. A small status
pill shows when it is listening and when it is finishing the transcription.

The app is written in Rust for Linux desktops, with Wayland as the primary
target. Speech recognition runs locally; audio is not sent to a transcription
service. Models are downloaded separately and remain on your machine.

## What it does

- Captures microphone audio and transcribes it with Whisper, Vosk, Moonshine,
  Zipformer, or SenseVoice.
- Types recognized text into the focused application using built-in `uinput`
  or an optional typing tool.
- Shows a lightweight GTK layer-shell indicator and supports a single toggle
  command that fits a desktop keybind.
- Installs, selects, checks, recommends, and benchmarks local speech models
  from the command line.
- Supports CPU inference by default and optional compiled GPU support for
  Whisper.

## Quick start

Grab a prebuilt Linux x86_64 binary from the
[Releases page](https://github.com/GrishMahat/dictation-indicator/releases),
or build from source — see [Installation](docs/INSTALLATION.md) for build
dependencies and setup.

```bash
make release
make install
systemctl --user daemon-reload
systemctl --user enable --now dictation-indicator
```

Bind `dictation toggle` to a shortcut. For example, in Hyprland:

```ini
bind = SUPER, grave, exec, dictation toggle
```

On first run, install a model and select it:

```bash
dictation model list
dictation model install whisper-base.en
dictation model set whisper-base.en
```

## Documentation

- [Installation and build options](docs/INSTALLATION.md)
- [Using dictation and managing models](docs/USAGE.md)
- [Configuration reference](docs/CONFIGURATION.md)
- [Troubleshooting](docs/TROUBLESHOOTING.md)
- [Architecture and development notes](docs/DEVELOPMENT.md)

## License

The project is licensed under the [GNU General Public License, version 3 or
later](LICENSE). Third-party libraries and speech model files have their own
licenses; check each model's terms before downloading or redistributing it.
