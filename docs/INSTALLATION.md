# Installation

This guide covers the source build. The project currently targets Linux, with
Wayland as its primary desktop environment. GTK4 layer-shell is used for the
indicator.

## Build dependencies

On Arch Linux:

```bash
sudo pacman -S base-devel cmake clang gtk4 gtk4-layer-shell alsa-lib vosk-api
```

Install a stable Rust toolchain with Cargo. Building Whisper also needs a C/C++
toolchain and libclang for bindings generation; the package group above
provides those on Arch. `vosk-api` provides `libvosk.so`, which the Vosk
backend links against; it is required to link the binaries even when you
plan to use a different backend.

Model installation uses `curl` and the archive tools `tar` and `unzip`. Install
them if they are not already present:

```bash
sudo pacman -S curl tar unzip
```

The default typing backend uses `/dev/uinput`. Add your user to the `input`
group and log out and back in for the permission change to apply:

```bash
sudo usermod -aG input "$USER"
```

Optional external typing backends:

```bash
sudo pacman -S wtype       # Wayland
sudo pacman -S ydotool     # requires ydotoold
sudo pacman -S xdotool     # X11 and XWayland clients
# dotool is available from the AUR and uses /dev/uinput
```

## Build and install

From the repository root:

```bash
make release
make install
systemctl --user daemon-reload
systemctl --user enable --now dictation-indicator
```

`make install` places the binaries and sherpa-onnx shared libraries in
`~/.local/bin`, and installs the user service in
`~/.config/systemd/user`. The service starts the indicator with your graphical
session. Reload and enable it as shown above.

The paths can be adjusted with `PREFIX`, `BINDIR`, and `SYSTEMD_USER_DIR`;
the installed service's `ExecStart` follows `BINDIR`.

Build without the indicator service if you only want to inspect the binaries:

```bash
make build       # debug build
make release     # optimized CPU build
make check       # type-check workspace
make fmt         # check formatting
```

### Optional Whisper GPU builds

GPU support is compiled into Whisper. Build with one provider per binary:

```bash
make release GPU=vulkan
# or GPU=cuda or GPU=hip
```

Vulkan requires Vulkan development headers, shaderc, and a working vendor ICD.
CUDA requires the CUDA toolkit; HIP requires ROCm. The Whisper dependency also
has a Metal feature, but the full app currently depends on Linux/Wayland
components and does not support macOS.
The installed binary can use only the provider it was compiled with. If GPU
initialization fails at runtime, Whisper may fall back to CPU; see
[Configuration](CONFIGURATION.md#compute-provider) for details.

### Uninstall

Disable the service, then remove the installed files:

```bash
systemctl --user disable --now dictation-indicator
make uninstall
```

The configuration, downloaded models, and logs are kept in your XDG user
directories.
