# Troubleshooting

## Check configuration and dependencies

Start with:

```bash
dictation config check
dictation doctor
```

The config check reports invalid settings and missing model files. The doctor
command also checks the compute provider, audio interface, model download
tools, and typing backend.

## The pill is not visible

Check that the user service is running:

```bash
systemctl --user status dictation-indicator
journalctl --user -u dictation-indicator -n 80 --no-pager
```

The pill uses GTK4 layer-shell and expects a Wayland session with the
appropriate layer-shell support.

## Dictation starts but no text appears

Check that the selected model is installed and that the configured microphone
is available. The default `uinput` typing backend needs access to `/dev/uinput`;
check your `input` group membership and log out and back in after changing it.
If you use an external typing backend, make sure its executable and any
required daemon are available in the graphical session.

Recent engine messages are in:

```bash
tail -n 80 ~/.cache/dictation-native.log
```

The engine logs which typing backend it selected and reports if audio chunks
are being dropped because inference cannot keep up.

## GPU provider errors or unexpected CPU use

GPU features are build-time options. Confirm the selected provider and
compiled feature list with `dictation doctor`. The binary must have been built
with exactly one GPU feature, and the matching development toolkit and driver
must be available. Whisper can still fall back to CPU if native GPU
initialization fails; inspect the startup output for Whisper backend messages.

## Stop a stale session

If the process was force-killed, check the daemon and indicator service before
starting another session:

```bash
dictation status
systemctl --user status dictation-indicator
```

The application clears stale session markers on startup. Avoid deleting files
from the cache directory while a dictation session is active.
