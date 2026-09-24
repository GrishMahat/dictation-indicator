use super::whisper::{WhisperSegmenter, auto_audio_ctx, buf_frames, decode, excerpt};
use dictation_config::EngineConfig;
use std::time::Instant;

/// Benchmark one model in an isolated CLI process. Keeping this to one model
/// per process makes `VmHWM` meaningful when comparing candidate models.
pub fn whisper_bench_model(
    engine: &EngineConfig,
    wav_path: &str,
    model_path: &str,
    reference: Option<&str>,
) -> Result<String, String> {
    let (pcm, rate) = read_wav(wav_path)?;
    let pcm = resample_to_16k(&pcm, rate);
    if pcm.is_empty() {
        return Err(format!("{wav_path}: WAV contains no audio samples"));
    }

    let mut candidate = engine.clone();
    candidate.backend = "whisper".to_string();
    candidate.whisper_model = model_path.to_string();
    let (rss_before, _) = process_memory_mib();
    let load_started = Instant::now();
    let provider = crate::provider::resolve(&candidate)?;
    let mut whisper = WhisperSegmenter::new(&candidate, provider)?;
    let load_time = load_started.elapsed();
    let (rss_loaded, _) = process_memory_mib();

    // Whisper accepts at most a 30-second window. Split longer recordings
    // into equal, non-overlapping windows and score the complete transcript.
    let window_samples = 30 * crate::audio::RATE as usize;
    let decode_started = Instant::now();
    let mut transcript = String::new();
    for window in pcm.chunks(window_samples) {
        let part = decode(
            &mut whisper.state,
            whisper.threads,
            window,
            auto_audio_ctx(window.len()),
            &mut whisper.decode_samples,
        );
        if !transcript.is_empty() && !part.is_empty() {
            transcript.push(' ');
        }
        transcript.push_str(part.trim());
    }
    let decode_time = decode_started.elapsed();
    let (rss_after, peak_rss) = process_memory_mib();

    let duration = pcm.len() as f64 / crate::audio::RATE as f64;
    let realtime = duration / decode_time.as_secs_f64().max(f64::EPSILON);
    let label = std::path::Path::new(model_path)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(model_path);
    let mut report = format!(
        "model: {label}\n  audio: {duration:.1}s, source rate {rate} Hz\n  cold load: {load_time:.2?}\n  decode: {decode_time:.2?} ({realtime:.2}x realtime)\n  threads: {}\n",
        whisper.threads
    );
    report.push_str(&format_memory(rss_before, rss_loaded, rss_after, peak_rss));
    if let Some(expected) = reference {
        let (errors, words) = word_error_rate(expected, &transcript);
        let wer = if words == 0 {
            "n/a".to_string()
        } else {
            format!(
                "{:.1}% ({errors}/{words} word edits)",
                errors as f64 * 100.0 / words as f64
            )
        };
        report.push_str(&format!("  WER: {wer}\n"));
    }
    report.push_str(&format!("  transcript: {transcript}\n"));
    Ok(report)
}

/// Benchmark one non-Whisper streaming backend against the same WAV input.
pub fn streaming_bench_model(
    engine: &EngineConfig,
    wav_path: &str,
    backend: &str,
    reference: Option<&str>,
) -> Result<String, String> {
    let (pcm, rate) = read_wav(wav_path)?;
    let pcm = resample_to_16k(&pcm, rate);
    if pcm.is_empty() {
        return Err(format!("{wav_path}: WAV contains no audio samples"));
    }
    let mut candidate = engine.clone();
    candidate.backend = backend.to_string();
    let (rss_before, _) = process_memory_mib();
    let load_started = Instant::now();
    let mut segmenter = super::create(&candidate)?;
    let load_time = load_started.elapsed();
    let (rss_loaded, _) = process_memory_mib();
    let duration = pcm.len() as f64 / crate::audio::RATE as f64;
    let decode_started = Instant::now();
    let mut transcript = String::new();
    let mut pending = String::new();
    for chunk in pcm.chunks(crate::audio::RATE as usize / 10) {
        if let Some((text, fresh)) = segmenter.feed(chunk) {
            if fresh {
                if !transcript.is_empty() {
                    transcript.push(' ');
                }
                transcript.push_str(text.trim());
                pending.clear();
            } else {
                pending = text;
            }
        }
    }
    if let Some((text, fresh)) = segmenter.flush() {
        if fresh {
            if !transcript.is_empty() {
                transcript.push(' ');
            }
            transcript.push_str(text.trim());
        } else {
            pending = text;
        }
    }
    if !pending.is_empty() {
        if !transcript.is_empty() {
            transcript.push(' ');
        }
        transcript.push_str(pending.trim());
    }
    let decode_time = decode_started.elapsed();
    let (rss_after, peak_rss) = process_memory_mib();
    let realtime = duration / decode_time.as_secs_f64().max(f64::EPSILON);
    let mut report = format!(
        "backend: {backend}\n  audio: {duration:.1}s, source rate {rate} Hz\n  cold load: {load_time:.2?}\n  decode: {decode_time:.2?} ({realtime:.2}x realtime)\n",
    );
    report.push_str(&format_memory(rss_before, rss_loaded, rss_after, peak_rss));
    if let Some(expected) = reference {
        let (errors, words) = word_error_rate(expected, &transcript);
        let wer = if words == 0 {
            "n/a".to_string()
        } else {
            format!(
                "{:.1}% ({errors}/{words} word edits)",
                errors as f64 * 100.0 / words as f64
            )
        };
        report.push_str(&format!("  WER: {wer}\n"));
    }
    report.push_str(&format!("  transcript: {transcript}\n"));
    Ok(report)
}

fn process_memory_mib() -> (Option<f64>, Option<f64>) {
    let Ok(status) = std::fs::read_to_string("/proc/self/status") else {
        return (None, None);
    };
    let value = |key: &str| {
        status.lines().find_map(|line| {
            let rest = line.strip_prefix(key)?;
            let kib = rest.split_whitespace().next()?.parse::<f64>().ok()?;
            Some(kib / 1024.0)
        })
    };
    (value("VmRSS:"), value("VmHWM:"))
}

fn format_memory(
    before: Option<f64>,
    loaded: Option<f64>,
    after: Option<f64>,
    peak: Option<f64>,
) -> String {
    match (before, loaded, after, peak) {
        (Some(before), Some(loaded), Some(after), Some(peak)) => format!(
            "  RSS: before model {before:.1} MiB, after load {loaded:.1} MiB (delta {:+.1}), after decode {after:.1} MiB\n  peak RSS: {peak:.1} MiB\n",
            loaded - before
        ),
        _ => "  memory: unavailable (requires Linux /proc/self/status)\n".to_string(),
    }
}

fn word_error_rate(reference: &str, hypothesis: &str) -> (usize, usize) {
    fn words(text: &str) -> Vec<String> {
        let mut out = Vec::new();
        let mut word = String::new();
        for ch in text.chars().flat_map(char::to_lowercase) {
            if ch.is_alphanumeric() || ch == '\'' {
                word.push(ch);
            } else if !word.is_empty() {
                out.push(std::mem::take(&mut word));
            }
        }
        if !word.is_empty() {
            out.push(word);
        }
        out
    }

    let reference = words(reference);
    let hypothesis = words(hypothesis);
    let mut previous: Vec<usize> = (0..=hypothesis.len()).collect();
    for (i, expected) in reference.iter().enumerate() {
        let mut current = Vec::with_capacity(hypothesis.len() + 1);
        current.push(i + 1);
        for (j, actual) in hypothesis.iter().enumerate() {
            let cost = usize::from(expected != actual);
            current.push(
                (previous[j + 1] + 1)
                    .min(current[j] + 1)
                    .min(previous[j] + cost),
            );
        }
        previous = current;
    }
    (previous[hypothesis.len()], reference.len())
}

// ------------------------------------------------------------ benching

/// Minimal RIFF/WAVE reader for PCM, returning mono-ish i16 + rate.
fn read_wav(path: &str) -> Result<(Vec<i16>, u32), String> {
    let b = std::fs::read(path).map_err(|e| format!("{path}: {e}"))?;
    if b.len() < 44 || &b[0..4] != b"RIFF" || &b[8..12] != b"WAVE" {
        return Err(format!("{path}: not a RIFF/WAVE file"));
    }
    let u16le = |o: usize| u16::from_le_bytes([b[o], b[o + 1]]);
    let u32le = |o: usize| u32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]]);
    let mut pos = 12usize;
    let mut fmt: Option<(u16, u16, u32, u16)> = None; // (format, ch, rate, bits)
    let mut data: Option<&[u8]> = None;
    while pos + 8 <= b.len() {
        let size = u32le(pos + 4) as usize;
        let body_start = pos + 8;
        let body_end = (body_start + size).min(b.len());
        match &b[pos..pos + 4] {
            b"fmt " if size >= 16 => {
                fmt = Some((
                    u16le(body_start),
                    u16le(body_start + 2),
                    u32le(body_start + 4),
                    u16le(body_start + 14),
                ));
            }
            b"data" => data = Some(&b[body_start..body_end]),
            _ => {}
        }
        pos = body_start + size + (size & 1); // word aligned
    }
    let (format, channels, rate, bits) = fmt.ok_or("no fmt chunk")?;
    let data = data.ok_or("no data chunk")?;
    if format != 1 {
        return Err(format!("compressed wav (format {format}), need PCM"));
    }
    let mut mono: Vec<i16> = Vec::new();
    match bits {
        16 => {
            let frames = data.len() / 2 / channels.max(1) as usize;
            for f in 0..frames {
                let mut acc = 0i32;
                for c in 0..channels as usize {
                    let o = (f * channels as usize + c) * 2;
                    if o + 1 < data.len() {
                        acc += i16::from_le_bytes([data[o], data[o + 1]]) as i32;
                    }
                }
                mono.push((acc / channels.max(1) as i32) as i16);
            }
        }
        other => return Err(format!("{other}-bit wav unsupported for bench")),
    }
    Ok((mono, rate))
}

fn resample_to_16k(samples: &[i16], rate: u32) -> Vec<i16> {
    if rate == crate::audio::RATE {
        return samples.to_vec();
    }
    let step = rate as f64 / crate::audio::RATE as f64;
    let out_len = (samples.len() as f64 / step) as usize;
    (0..out_len)
        .map(|i| {
            let src = i as f64 * step;
            let i0 = src as usize;
            let frac = (src - i0 as f64) as f32;
            let a = samples.get(i0).copied().unwrap_or(0);
            let b = samples.get(i0 + 1).copied().unwrap_or(a);
            (a as f32 * (1.0 - frac) + b as f32 * frac) as i16
        })
        .collect()
}

/// `dictation benchmark-tune [wav]`: compare encoder windows and thread counts
/// against real speech from the current model.
/// Default wav: `~/.cache/dictation-bench.wav`.
pub fn whisper_bench(engine: &EngineConfig, wav_path: &str) -> Result<String, String> {
    let (pcm, rate) = read_wav(wav_path)?;
    let pcm = resample_to_16k(&pcm, rate);
    let mut candidate = engine.clone();
    candidate.backend = "whisper".to_string();
    let provider = crate::provider::resolve(&candidate)?;
    let mut w = WhisperSegmenter::new(&candidate, provider)?;
    let model_name = std::path::Path::new(&engine.whisper_model)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(&engine.whisper_model);
    let mut out = format!(
        "bench: {} ({rate} Hz -> 16 kHz, {} s of audio), threads={} model={}\n",
        wav_path,
        pcm.len() / crate::audio::RATE as usize,
        w.threads,
        model_name
    );
    // 0 = Whisper's default 30 s window; the rest are sized windows.
    for ac in [0, 600, 300, 150] {
        for secs in [1usize, 2, 4, 8] {
            let n = (secs * crate::audio::RATE as usize).min(pcm.len());
            if n < crate::audio::RATE as usize / 2 {
                break;
            }
            let needed = buf_frames(n) as i32;
            if ac > 0 && ac < needed {
                continue; // window smaller than the audio: would truncate
            }
            let label = if ac == 0 {
                "default(30s)".to_string()
            } else {
                format!("audio_ctx={ac}")
            };
            let t0 = Instant::now();
            let text = decode(
                &mut w.state,
                w.threads,
                &pcm[..n],
                ac,
                &mut w.decode_samples,
            );
            let dt = t0.elapsed();
            let x = n as f64 / crate::audio::RATE as f64 / dt.as_secs_f64();
            out.push_str(&format!(
                "  {secs}s [{label:>13}] -> decode {:>7?}  ({x:.1}x realtime)  \"{}\"\n",
                dt,
                excerpt(&text)
            ));
        }
    }

    out.push_str("\nthread scaling with runtime-sized encoder windows:\n");
    for threads in [1, 2, 4, 8] {
        for secs in [1usize, 4] {
            let n = (secs * crate::audio::RATE as usize).min(pcm.len());
            if n < crate::audio::RATE as usize / 2 {
                break;
            }
            let t0 = Instant::now();
            let text = decode(
                &mut w.state,
                threads,
                &pcm[..n],
                auto_audio_ctx(n),
                &mut w.decode_samples,
            );
            let dt = t0.elapsed();
            let x = n as f64 / crate::audio::RATE as f64 / dt.as_secs_f64();
            out.push_str(&format!(
                "  threads={threads} {secs}s -> decode {:>7?} ({x:.1}x realtime) \"{}\"\n",
                dt,
                excerpt(&text)
            ));
        }
    }
    Ok(out)
}
