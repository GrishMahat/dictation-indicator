# Using dictation

## Start and stop

Use one command for both directions and bind it to a convenient shortcut:

```ini
# Hyprland example
bind = SUPER, grave, exec, dictation toggle
```

The indicator shows `Listening...` while recording and `Dictating...` while
the final audio is being transcribed and typed. Clicking the indicator also
stops dictation.

For scripts, use the explicit commands:

```bash
dictation begin
dictation end
dictation status
```

`dictation config` prints the configuration file path. `dictation config check`
checks the configuration and active model. `dictation doctor` checks the
provider, model management tools, audio interface, and typing setup.

## Models

List the model catalog, install a model, and make it active:

```bash
dictation model list
dictation model install whisper-base.en
dictation model set whisper-base.en
```

Useful model commands:

```text
dictation model list [--backend NAME] [--installed]
dictation model info NAME
dictation model check [NAME]
dictation model recommend
dictation model benchmark AUDIO.wav [--model NAME] [--reference-file TEXT]
dictation model remove NAME
dictation model management off|on
```

The catalog includes Whisper, Vosk, Moonshine, Zipformer, and SenseVoice
models. Model downloads can be large; check the listed size and available disk
space before installing. Downloads are resumable: run the install command
again after an interrupted transfer. Managed models are stored under
`~/.local/share/dictation/models/`.

Whisper offers several model sizes and quantizations. Vosk has small and large
models for many languages. Moonshine includes English and multilingual model
variants; SenseVoice supports English, Chinese, Cantonese, Japanese, and
Korean. The catalog's Zipformer option is an English streaming model.

Recommendations use available system memory as a rough guide. Benchmark a
representative recording on your machine before changing models. You can
select a custom Whisper model file by passing its path to
`dictation model set`.

## Benchmarking

Compare models against the same recording:

```bash
dictation benchmark /path/to/speech.wav \
  --model ~/.local/share/dictation/models/ggml-tiny.en-q5_1.bin \
  --reference-file /path/to/speech.txt
```

The configured model is included as the baseline. The report includes model
load time, decode time, real-time factor, and memory use. Supplying a reference
transcript adds word error rate (WER); without one, the transcription is shown
for manual comparison. Use `dictation benchmark-tune [wav]` to compare Whisper
encoder windows and thread counts.

For background on the benchmark and streaming behavior, see
[Development notes](DEVELOPMENT.md#benchmarking-and-latency).
