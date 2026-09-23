//! Microphone capture: cpal in, 16 kHz mono i16 out (what Vosk wants).
//! Handles whatever the hardware gives us (44.1/48 kHz, f32/i16/u16,
//! mono or multi-channel) with a streaming linear resampler.

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, SyncSender, TrySendError, sync_channel};

pub const RATE: u32 = 16_000;
const CHUNK_SAMPLES: usize = RATE as usize / 10; // flush every 100 ms
/// Bounded so a slow recogniser (Whisper blocks its caller while
/// transcribing) can't make the realtime audio callback block. Keep
/// latency bounded: 16 × 100 ms = 1.6 s; overflow drops new chunks and
/// is reported by the daemon.
const CHANNEL_CAP: usize = 16;

/// Streaming linear resampler: source-rate f32 samples in, 16 kHz mono
/// i16 chunks out.
struct Resampler {
    step: f64,
    next_out: f64, // fractional source index of the next output sample
    base: u64,     // global index of buf[0]
    buf: Vec<f32>,
    out: Vec<i16>,
}

impl Resampler {
    fn new(src_rate: u32) -> Self {
        Self {
            step: src_rate as f64 / RATE as f64,
            next_out: 0.0,
            base: 0,
            buf: Vec::new(),
            out: Vec::with_capacity(CHUNK_SAMPLES),
        }
    }

    fn push(
        &mut self,
        samples: impl IntoIterator<Item = f32>,
        tx: &SyncSender<Vec<i16>>,
        dropped_chunks: &AtomicU64,
    ) {
        self.buf.extend(samples);
        let len = self.buf.len() as u64;
        // Need the sample at floor(next_out) and the one after it.
        while self.next_out + 1.0 < (self.base + len) as f64 {
            let idx = (self.next_out - self.base as f64) as usize;
            let frac = (self.next_out - self.base as f64) - idx as f64;
            let s = self.buf[idx] * (1.0 - frac as f32) + self.buf[idx + 1] * frac as f32;
            self.out.push((s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16);
            self.next_out += self.step;
            if self.out.len() >= CHUNK_SAMPLES {
                let mut chunk = std::mem::take(&mut self.out);
                self.out = Vec::with_capacity(CHUNK_SAMPLES);
                match tx.try_send(std::mem::take(&mut chunk)) {
                    Ok(()) | Err(TrySendError::Disconnected(_)) => {}
                    // Consumer behind: drop the chunk rather than
                    // blocking the realtime audio callback.
                    Err(TrySendError::Full(_)) => {
                        dropped_chunks.fetch_add(1, Ordering::Relaxed);
                    }
                }
            }
        }
        // Discard source samples already consumed.
        let keep_from = (self.next_out.floor() - self.base as f64).max(0.0) as usize;
        if keep_from > 0 && keep_from < self.buf.len() {
            self.buf.drain(..keep_from);
            self.base += keep_from as u64;
        }
    }
}

/// Average channels down to mono f32 (interleaved input).
fn to_mono_f32(data: &[f32], channels: usize) -> Vec<f32> {
    if channels <= 1 {
        return data.to_vec();
    }
    data.chunks(channels)
        .map(|frame| frame.iter().sum::<f32>() / channels as f32)
        .collect()
}

fn to_mono_i16(data: &[i16], channels: usize) -> Vec<f32> {
    if channels <= 1 {
        return data.iter().map(|&s| s as f32 / 32768.0).collect();
    }
    data.chunks(channels)
        .map(|frame| frame.iter().map(|&s| s as f32 / 32768.0).sum::<f32>() / channels as f32)
        .collect()
}

fn to_mono_u16(data: &[u16], channels: usize) -> Vec<f32> {
    if channels <= 1 {
        return data.iter().map(|&s| (s as f32 / 32768.0) - 1.0).collect();
    }
    data.chunks(channels)
        .map(|frame| {
            frame
                .iter()
                .map(|&s| (s as f32 / 32768.0) - 1.0)
                .sum::<f32>()
                / channels as f32
        })
        .collect()
}

/// Open the microphone and start streaming 16 kHz i16 chunks.
/// `device_hint` empty = system default, otherwise substring match on
/// the device name.
pub fn open(
    device_hint: &str,
) -> Result<
    (
        cpal::Stream,
        Receiver<Vec<i16>>,
        Arc<AtomicU64>,
        Receiver<String>,
    ),
    String,
> {
    let host = cpal::default_host();
    let device = if device_hint.is_empty() {
        host.default_input_device()
            .ok_or_else(|| "no default input device".to_string())?
    } else {
        host.input_devices()
            .map_err(|e| e.to_string())?
            .find(|d| d.name().map(|n| n.contains(device_hint)).unwrap_or(false))
            .ok_or_else(|| format!("no input device matching `{device_hint}`"))?
    };
    let dev_name = device.name().unwrap_or_else(|_| "?".into());
    let supported = device
        .default_input_config()
        .map_err(|e| format!("default input config: {e}"))?;
    let src_rate = supported.sample_rate().0;
    let channels = supported.channels() as usize;
    let sample_format = supported.sample_format();

    let (tx, rx) = sync_channel::<Vec<i16>>(CHANNEL_CAP);
    let (error_tx, error_rx) = std::sync::mpsc::channel::<String>();
    let dropped_chunks = Arc::new(AtomicU64::new(0));
    let mut resampler = Resampler::new(src_rate);

    let stream = match sample_format {
        cpal::SampleFormat::F32 => {
            let dropped_chunks = dropped_chunks.clone();
            let error_tx = error_tx.clone();
            device.build_input_stream(
                &supported.config(),
                move |data: &[f32], _| {
                    if channels == 1 {
                        resampler.push(data.iter().copied(), &tx, &dropped_chunks);
                    } else {
                        resampler.push(to_mono_f32(data, channels), &tx, &dropped_chunks);
                    }
                },
                move |e| {
                    let _ = error_tx.send(e.to_string());
                },
                None,
            )
        }
        cpal::SampleFormat::I16 => {
            let dropped_chunks = dropped_chunks.clone();
            let error_tx = error_tx.clone();
            device.build_input_stream(
                &supported.config(),
                move |data: &[i16], _| {
                    if channels == 1 {
                        resampler.push(
                            data.iter().map(|&sample| sample as f32 / 32768.0),
                            &tx,
                            &dropped_chunks,
                        );
                    } else {
                        resampler.push(to_mono_i16(data, channels), &tx, &dropped_chunks);
                    }
                },
                move |e| {
                    let _ = error_tx.send(e.to_string());
                },
                None,
            )
        }
        cpal::SampleFormat::U16 => {
            let dropped_chunks = dropped_chunks.clone();
            let error_tx = error_tx.clone();
            device.build_input_stream(
                &supported.config(),
                move |data: &[u16], _| {
                    if channels == 1 {
                        resampler.push(
                            data.iter().map(|&sample| (sample as f32 / 32768.0) - 1.0),
                            &tx,
                            &dropped_chunks,
                        );
                    } else {
                        resampler.push(to_mono_u16(data, channels), &tx, &dropped_chunks);
                    }
                },
                move |e| {
                    let _ = error_tx.send(e.to_string());
                },
                None,
            )
        }
        other => return Err(format!("unsupported sample format {other}")),
    }
    .map_err(|e| format!("build_input_stream: {e}"))?;

    stream.play().map_err(|e| format!("stream play: {e}"))?;
    eprintln!(
        "dictation-native: mic `{dev_name}` {src_rate} Hz ch{channels} {sample_format:?} →16 kHz"
    );
    Ok((stream, rx, dropped_chunks, error_rx))
}
