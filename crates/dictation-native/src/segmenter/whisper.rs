use super::Segmenter;
use dictation_config::EngineConfig;
use std::time::{Duration, Instant};

use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

/// RMS above this counts as you talking (quiet speech ~0.02+, room
/// noise usually <0.005; 0.008 leaves headroom for quiet mics).
const SPEECH_ON: f32 = 0.008;
/// First rolling transcription once this much audio (buffer duration
/// wall-time, pauses included) has accumulated.
const FIRST_CHUNK_MS: u32 = 1000;
/// Minimum wall-time between rolling decodes — a slow decode can never
/// turn into a back-to-back decode storm (this was the "first text at
/// 5–10 s" suspect: unchecked growing-buffer redecodes blocked the
/// feed loop while audio piled up and got dropped).
const CADENCE_WALL: Duration = Duration::from_millis(1500);
/// This much consecutive quiet ends the utterance.
const SILENCE_END_MS: u32 = 500;
/// Shorter than this = noise blip, don't waste a transcription on it.
const MIN_SPEECH_MS: u32 = 200;
/// Force-split monologues so the audio buffer (and latency) stay bounded.
const MAX_SEGMENT_MS: u32 = 6_000;
/// Whisper's own per-segment speech confidence: above this it's room
/// noise/typing, not you talking — drop it instead of typing
/// hallucinated words ("(chiming)", "Thanks for watching…").
const NO_SPEECH_MAX: f32 = 0.6;
/// Whisper scales poorly past ~8 threads; oversubscription on big
/// machines makes it *slower*.
const MAX_THREADS: i32 = 8;
/// Whisper's mel runs at 50 frames/s (16 000 / 320 samples).
const FRAMES_PER_SEC: usize = 50;
/// Extra encoder window beyond the buffer so the model still sees a
/// little air after the last word.
const AC_GUARD_FRAMES: usize = 50;
/// Round the encoder window to this many frames: the KV cache only
/// reallocs when the padded size crosses this step.
const AC_ROUND: usize = 25;
/// The model's full (30 s) window — the ceiling for `audio_ctx`.
const AC_MAX: usize = 1500;

pub(super) fn buf_frames(len: usize) -> usize {
    len.div_ceil(crate::audio::RATE as usize / FRAMES_PER_SEC)
}

/// Encoder window for a buffer of `len` samples.
///
/// Without this Whisper pads every call up to the full 30 s window —
/// a fixed ~6 s of compute on this class of CPU no matter how short
/// the audio, which is what made the first text land at 5–10 s.
/// Sizing the window to the buffer (+1 s guard) makes decode cost
/// proportional to what was actually said.
pub(super) fn auto_audio_ctx(len: usize) -> i32 {
    let frames = buf_frames(len) + AC_GUARD_FRAMES;
    (frames.div_ceil(AC_ROUND) * AC_ROUND).clamp(AC_ROUND, AC_MAX) as i32
}

pub(super) fn buf_ms(len: usize) -> u32 {
    (len as u64 * 1000 / crate::audio::RATE as u64) as u32
}

/// Decode a PCM slice to text (shared by rolling chunks and the tail).
/// Empty string on error, all-noise, or placeholder-only output —
/// typing nothing beats typing "(upbeat music)".
///
/// `audio_ctx` sizes the encoder window (0 = Whisper's 30 s default,
/// which wastes ~6 s of compute padding short buffers — see
/// [`auto_audio_ctx`]).
pub(super) fn decode(
    state: &mut whisper_rs::WhisperState,
    threads: i32,
    pcm: &[i16],
    audio_ctx: i32,
    samples: &mut Vec<f32>,
) -> String {
    if pcm.is_empty() {
        return String::new();
    }
    samples.clear();
    samples.extend(pcm.iter().map(|&sample| sample as f32 / 32768.0));
    let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
    params.set_language(Some("en"));
    params.set_n_threads(threads);
    params.set_no_timestamps(true);
    params.set_audio_ctx(audio_ctx);
    if let Err(e) = state.full(params, &samples) {
        crate::log(&format!("whisper full() failed: {e:?}"));
        return String::new();
    }
    let n = state.full_n_segments();
    let mut out = String::new();
    for i in 0..n {
        if let Some(seg) = state.get_segment(i) {
            if seg.no_speech_probability() > NO_SPEECH_MAX {
                continue; // noise, not speech — type nothing
            }
            if let Ok(t) = seg.to_str() {
                out.push_str(t);
            }
        }
    }
    strip_parens(out.trim())
}

/// Drop Whisper's stage directions — everything it writes in
/// parentheses ("(upbeat music)", "(crickets chirping)") is its
/// annotation of non-speech, not words you said (it never wraps *your*
/// words that way). Placeholder-only output becomes "", so room noise
/// types nothing; unbalanced parens are left alone rather than guessed
/// at.
pub(super) fn strip_parens(s: &str) -> String {
    let opens = s.matches('(').count();
    if opens == 0 || opens != s.matches(')').count() {
        return s.to_string();
    }
    let mut out = String::with_capacity(s.len());
    let mut depth = 0usize;
    for c in s.chars() {
        match c {
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            _ if depth == 0 => out.push(c),
            _ => {}
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Concatenate two consecutive transcriptions without losing/gaining a
/// stray space at the seam.
fn join_parts(a: &str, b: &str) -> String {
    if a.is_empty() {
        return b.to_string();
    }
    if b.is_empty() {
        return a.to_string();
    }
    if b.starts_with(char::is_whitespace) || a.ends_with(char::is_whitespace) {
        format!("{a}{b}")
    } else {
        format!("{a} {b}")
    }
}

pub(super) fn excerpt(s: &str) -> String {
    s.chars().take(80).collect()
}

pub(super) struct WhisperSegmenter {
    pub(super) state: whisper_rs::WhisperState,
    pub(super) threads: i32,
    /// Reused PCM conversion buffer; rolling decodes otherwise allocate
    /// a fresh float vector every time.
    pub(super) decode_samples: Vec<f32>,
    buf: Vec<i16>,
    speech_ms: u32,
    silence_ms: u32,
    in_speech: bool,
    /// Sample offset in `buf` already covered by a rolling chunk
    /// (finalize transcribes only the tail after this).
    transcribed: usize,
    /// Emitted text of the current utterance (diff base + tail prefix).
    last_text: String,
    /// Whether a rolling chunk has been emitted this utterance.
    emitted: bool,
    /// When the last rolling decode started (storm guard).
    last_chunk_at: Option<Instant>,
    /// Loudest RMS seen this segment — logged for gate tuning.
    max_rms: f32,
}

impl WhisperSegmenter {
    pub(super) fn new(
        engine: &EngineConfig,
        provider: crate::provider::ProviderCandidate,
    ) -> Result<Self, String> {
        let path = &engine.whisper_model;
        // GPU builds default to `use_gpu = true`, so an explicit
        // `provider = "cpu"` must turn it back off instead of being ignored.
        let mut context_params = WhisperContextParameters::new();
        context_params.use_gpu(provider.is_gpu());
        let ctx = WhisperContext::new_with_params(path, context_params)
            .map_err(|e| format!("whisper model load failed: `{path}` ({e:?})"))?;
        let state = ctx
            .create_state()
            .map_err(|e| format!("whisper state creation failed ({e:?})"))?;
        let auto_threads = std::thread::available_parallelism()
            .map(|n| n.get().min(MAX_THREADS as usize) as i32)
            .unwrap_or(4);
        let threads = crate::provider::resolve_threads(engine, auto_threads);
        Ok(Self {
            state,
            threads,
            decode_samples: Vec::with_capacity(
                crate::audio::RATE as usize * MAX_SEGMENT_MS as usize / 1000,
            ),
            buf: Vec::new(),
            speech_ms: 0,
            silence_ms: 0,
            in_speech: false,
            transcribed: 0,
            last_text: String::new(),
            emitted: false,
            last_chunk_at: None,
            max_rms: 0.0,
        })
    }

    /// End the utterance: decode whatever the rolling chunks haven't
    /// covered yet (tail-only — Whisper's state supplies the context,
    /// so this is fast) unless nothing was emitted yet (short
    /// utterance → decode the whole buffer). Resets all state.
    fn finalize(&mut self) -> Option<(String, bool)> {
        let heard = self.speech_ms >= MIN_SPEECH_MS;
        let was_emitted = self.emitted;
        let transcribed = self.transcribed;
        let prior = std::mem::take(&mut self.last_text);
        let buf = std::mem::take(&mut self.buf);
        let ms = buf_ms(buf.len());
        let peak = self.max_rms;
        self.in_speech = false;
        self.speech_ms = 0;
        self.silence_ms = 0;
        self.last_chunk_at = None;
        self.emitted = false;
        self.transcribed = 0;
        self.max_rms = 0.0;
        dictation_core::state::set_busy(false);
        if !heard || buf.is_empty() {
            return None;
        }

        let t0 = Instant::now();
        let text = if was_emitted && transcribed > 0 && transcribed <= buf.len() {
            join_parts(
                &prior,
                &decode(
                    &mut self.state,
                    self.threads,
                    &buf[transcribed..],
                    auto_audio_ctx(buf.len() - transcribed),
                    &mut self.decode_samples,
                ),
            )
        } else {
            decode(
                &mut self.state,
                self.threads,
                &buf,
                auto_audio_ctx(buf.len()),
                &mut self.decode_samples,
            )
        };
        crate::log(&format!(
            "finalize buf={ms}ms decode={:?} peak_rms={peak:.3} emitted={was_emitted} -> {:?}",
            t0.elapsed(),
            excerpt(&text)
        ));
        if text.is_empty() {
            return None;
        }
        Some((text, !was_emitted))
    }
}

impl Segmenter for WhisperSegmenter {
    fn feed(&mut self, chunk: &[i16]) -> Option<(String, bool)> {
        let n = chunk.len().max(1) as f64;
        let mean_sq: f64 = chunk
            .iter()
            .map(|&s| {
                let v = s as f64 / 32768.0;
                v * v
            })
            .sum::<f64>()
            / n;
        let rms = mean_sq.sqrt() as f32;

        if !self.in_speech {
            if rms < SPEECH_ON {
                return None; // still quiet: don't buffer, don transcribe
            }
            self.in_speech = true;
            self.speech_ms = 0;
            self.silence_ms = 0;
            self.last_chunk_at = None;
            self.max_rms = rms;
            self.buf.clear();
            crate::log(&format!("segment open rms={rms:.3} (gate {SPEECH_ON:.3})"));
            dictation_core::state::set_busy(true);
        }
        if rms > self.max_rms {
            self.max_rms = rms;
        }
        self.buf.extend_from_slice(chunk); // trailing silence stays in: harmless
        // Account the callback's REAL duration: cpal does not guarantee
        // 100 ms per callback. Assuming it let two 75 ms mic blasts fake
        // 200 ms of "speech" and get typed as "(upbeat music)".
        let cms = buf_ms(chunk.len());
        if rms >= SPEECH_ON {
            self.speech_ms += cms;
            self.silence_ms = 0;
        } else {
            self.silence_ms += cms;
        }

        let ms = buf_ms(self.buf.len());
        // Rolling chunk: buffer wall-time based (pauses don't delay
        // it), only while currently voiced (don't decode a pause that's
        // about to end the segment), rate-limited by wall-clock.
        if self.in_speech
            && self.silence_ms == 0
            && ms >= FIRST_CHUNK_MS
            && self
                .last_chunk_at
                .is_none_or(|t| t.elapsed() >= CADENCE_WALL)
        {
            let t0 = Instant::now();
            let text = decode(
                &mut self.state,
                self.threads,
                &self.buf,
                auto_audio_ctx(self.buf.len()),
                &mut self.decode_samples,
            );
            self.last_chunk_at = Some(Instant::now());
            crate::log(&format!(
                "chunk buf={ms}ms decode={:?} peak_rms={:.3} -> {:?}",
                t0.elapsed(),
                self.max_rms,
                excerpt(&text)
            ));
            if text.is_empty() {
                return None; // don't count as emitted; retry after cadence
            }
            let fresh = !self.emitted;
            self.last_text = text.clone();
            self.emitted = true;
            self.transcribed = self.buf.len();
            return Some((text, fresh));
        }

        if (self.silence_ms >= SILENCE_END_MS && self.speech_ms >= MIN_SPEECH_MS)
            || ms >= MAX_SEGMENT_MS
        {
            self.finalize()
        } else {
            None
        }
    }

    fn flush(&mut self) -> Option<(String, bool)> {
        if self.in_speech {
            self.finalize()
        } else {
            None
        }
    }
}

/// Hidden `dictation __whisper-selftest`: exercise the whole path
/// (model load → state → full() → segment read) on a synthetic tone —
/// no microphone or speech needed. Whatever text comes back (even "")
/// proves every stage ran.
pub fn whisper_selftest(engine: &EngineConfig) -> Result<String, String> {
    let mut candidate = engine.clone();
    candidate.backend = "whisper".to_string();
    let provider = crate::provider::resolve(&candidate)?;
    let mut w = WhisperSegmenter::new(&candidate, provider)?;
    // 1 s of 440 Hz at 16 kHz.
    w.buf = (0..crate::audio::RATE)
        .map(|i| {
            let v = (i as f32 * 440.0 * 2.0 * std::f32::consts::PI / crate::audio::RATE as f32)
                .sin()
                * 0.25;
            (v * i16::MAX as f32) as i16
        })
        .collect();
    w.speech_ms = 1000;
    w.in_speech = true;
    Ok(w.finalize().map(|(t, _)| t).unwrap_or_default())
}
