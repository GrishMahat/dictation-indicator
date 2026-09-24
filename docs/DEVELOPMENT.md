# Architecture and development

## Workspace layout

| Crate | Responsibility |
| --- | --- |
| `dictation-config` | TOML configuration, defaults, paths, and validation. |
| `dictation-core` | Session state and serialized control operations. |
| `dictation-cli` | `dictation` commands and detached daemon startup. |
| `dictation-native` | Microphone capture, recognition backends, and text injection. |
| `dictation-indicator` | GTK4 layer-shell status pill. |

The normal audio path is microphone capture → bounded audio queue → selected
recognizer → text injection. Whisper supports rolling updates; the other
recognizers follow their backend-specific streaming/finalization behavior.
Recognition runs locally.

## Session coordination

The CLI, daemon, and indicator share state under the XDG cache directory. The
active marker indicates that the microphone is on; the busy marker indicates
that a speech segment is being processed. The indicator watches those files
with filesystem notifications and a short polling backstop.

Start, stop, and toggle serialize through a process lock so rapid key presses
do not launch duplicate daemons. Audio uses a bounded queue of about 1.6
seconds; if inference falls behind, chunks can be dropped and the event is
logged. A normal stop flushes the last usable audio before the daemon exits.

## Benchmarking and latency

Whisper benchmarks compare model load time, decode time, real-time factor, and
memory use. Add a reference transcript to report word error rate. The
`dictation benchmark-tune [wav]` command compares encoder windows and thread
counts on the configured Whisper model.

Whisper uses an encoder window sized to the buffered audio instead of always
decoding against its full 30-second window. The streaming cadence and segment
limits are implementation constants in
`crates/dictation-native/src/segmenter/whisper.rs`; changing them trades
update latency against CPU load and transcription quality. Benchmark with a
representative recording before changing those values.

## Local development

```bash
make build
make check
make fmt
cargo test --workspace
```

Optional GPU builds need the matching system toolkit. See
[Installation](INSTALLATION.md#optional-whisper-gpu-builds) and
[Configuration](CONFIGURATION.md#compute-provider).
