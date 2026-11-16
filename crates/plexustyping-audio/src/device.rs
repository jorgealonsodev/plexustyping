//! `cpal` output stream driving [`BeatClock`] on a dedicated audio thread.

use crate::{AudioError, BeatClock, MetronomeConfig, Sound, SoundCache};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{BufferSize, FromSample, SampleFormat, SizedSample, StreamConfig, SupportedBufferSize};
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::mpsc;
use std::thread::JoinHandle;
use std::time::Instant;

/// Preferred buffer size in frames (~5 ms at 48 kHz) when the device allows it.
const PREFERRED_BUFFER_FRAMES: u32 = 256;
/// Scratch buffer length used to convert to non-`f32` sample formats.
const SCRATCH_SAMPLES: usize = 1024;

/// A playing metronome on the default output device.
///
/// The `cpal` stream lives on a dedicated thread (streams are not `Send` on
/// every platform), so `Metronome` itself is `Send`. Dropping it stops audio.
pub struct Metronome {
    shared: Arc<SharedConfig>,
    stop: Option<mpsc::Sender<()>>,
    thread: Option<JoinHandle<()>>,
    started_at: Instant,
}

impl Metronome {
    /// Opens the default output device and starts playing; beat 0 sounds immediately.
    ///
    /// # Errors
    /// [`AudioError::InvalidConfig`], [`AudioError::NoOutputDevice`] or [`AudioError::Stream`].
    pub fn start(cfg: MetronomeConfig) -> Result<Self, AudioError> {
        let shared = Arc::new(SharedConfig::new(cfg.validated()?));
        let (ready_tx, ready_rx) = mpsc::sync_channel(1);
        let (stop_tx, stop_rx) = mpsc::channel::<()>();
        let thread_shared = Arc::clone(&shared);
        let thread = std::thread::Builder::new()
            .name("plexustyping-metronome".into())
            .spawn(move || match open_stream(thread_shared) {
                Ok((stream, started_at)) => {
                    let _ = ready_tx.send(Ok(started_at));
                    // Blocks until a stop message or the sender is dropped.
                    let _ = stop_rx.recv();
                    drop(stream);
                }
                Err(e) => {
                    let _ = ready_tx.send(Err(e));
                }
            })
            .map_err(|e| AudioError::Stream(e.to_string()))?;
        match ready_rx.recv() {
            Ok(Ok(started_at)) => Ok(Self {
                shared,
                stop: Some(stop_tx),
                thread: Some(thread),
                started_at,
            }),
            Ok(Err(e)) => {
                let _ = thread.join();
                Err(e)
            }
            Err(_) => {
                let _ = thread.join();
                Err(AudioError::Stream("audio thread exited".into()))
            }
        }
    }

    /// Applies a new config live; the beat phase continues and a BPM change
    /// takes effect from the next beat.
    ///
    /// # Errors
    /// [`AudioError::InvalidConfig`] (the previous config stays active).
    pub fn update(&self, cfg: MetronomeConfig) -> Result<(), AudioError> {
        self.shared.store(cfg.validated()?);
        Ok(())
    }

    /// Stops the stream and joins the audio thread.
    pub fn stop(self) {
        drop(self);
    }

    /// Instant at which beat 0 was scheduled (stream start), for offset metrics.
    pub fn started_at(&self) -> Instant {
        self.started_at
    }
}

impl Drop for Metronome {
    fn drop(&mut self) {
        drop(self.stop.take());
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

/// Lock-free config shared with the audio callback.
struct SharedConfig {
    bpm: AtomicU32,
    sound: AtomicU32,
    volume_bits: AtomicU32,
    accent_every: AtomicU32,
}

impl SharedConfig {
    fn new(cfg: MetronomeConfig) -> Self {
        let shared = Self {
            bpm: AtomicU32::new(0),
            sound: AtomicU32::new(0),
            volume_bits: AtomicU32::new(0),
            accent_every: AtomicU32::new(0),
        };
        shared.store(cfg);
        shared
    }

    fn store(&self, cfg: MetronomeConfig) {
        self.bpm.store(cfg.bpm, Ordering::Relaxed);
        self.sound
            .store(sound_index(cfg.sound) as u32, Ordering::Relaxed);
        self.volume_bits
            .store(cfg.volume.to_bits(), Ordering::Relaxed);
        self.accent_every.store(cfg.accent_every, Ordering::Relaxed);
    }

    fn load(&self) -> MetronomeConfig {
        MetronomeConfig {
            bpm: self.bpm.load(Ordering::Relaxed),
            sound: SOUNDS[self.sound.load(Ordering::Relaxed) as usize % SOUNDS.len()],
            volume: f32::from_bits(self.volume_bits.load(Ordering::Relaxed)),
            accent_every: self.accent_every.load(Ordering::Relaxed),
        }
    }
}

const SOUNDS: [Sound; 3] = [Sound::Click, Sound::Wood, Sound::Beep];

fn sound_index(sound: Sound) -> usize {
    match sound {
        Sound::Click => 0,
        Sound::Wood => 1,
        Sound::Beep => 2,
    }
}

fn open_stream(shared: Arc<SharedConfig>) -> Result<(cpal::Stream, Instant), AudioError> {
    let device = cpal::default_host()
        .default_output_device()
        .ok_or(AudioError::NoOutputDevice)?;
    let supported = device
        .default_output_config()
        .map_err(|e| AudioError::Stream(e.to_string()))?;
    let format = supported.sample_format();
    let mut config: StreamConfig = supported.config();
    let fixed = match supported.buffer_size() {
        SupportedBufferSize::Range { min, max } => {
            Some(BufferSize::Fixed(PREFERRED_BUFFER_FRAMES.clamp(*min, *max)))
        }
        SupportedBufferSize::Unknown => None,
    };
    let stream = match fixed {
        Some(size) => {
            config.buffer_size = size;
            build(&device, &config, format, &shared).or_else(|_| {
                config.buffer_size = BufferSize::Default;
                build(&device, &config, format, &shared)
            })
        }
        None => build(&device, &config, format, &shared),
    }?;
    let started_at = Instant::now();
    stream
        .play()
        .map_err(|e| AudioError::Stream(e.to_string()))?;
    Ok((stream, started_at))
}

fn build(
    device: &cpal::Device,
    config: &StreamConfig,
    format: SampleFormat,
    shared: &Arc<SharedConfig>,
) -> Result<cpal::Stream, AudioError> {
    match format {
        SampleFormat::F32 => build_typed::<f32>(device, config, shared),
        SampleFormat::F64 => build_typed::<f64>(device, config, shared),
        SampleFormat::I16 => build_typed::<i16>(device, config, shared),
        SampleFormat::I32 => build_typed::<i32>(device, config, shared),
        SampleFormat::U16 => build_typed::<u16>(device, config, shared),
        SampleFormat::U8 => build_typed::<u8>(device, config, shared),
        other => Err(AudioError::Stream(format!(
            "unsupported sample format {other}"
        ))),
    }
}

fn build_typed<T>(
    device: &cpal::Device,
    config: &StreamConfig,
    shared: &Arc<SharedConfig>,
) -> Result<cpal::Stream, AudioError>
where
    T: SizedSample + FromSample<f32>,
{
    let channels = usize::from(config.channels).max(1);
    let rate = config.sample_rate;
    let caches = SOUNDS.map(|s| SoundCache::new(s, rate));
    let shared = Arc::clone(shared);
    let mut clock = BeatClock::new(rate, shared.load().bpm);
    let step = SCRATCH_SAMPLES / channels * channels;
    let mut scratch = [0.0f32; SCRATCH_SAMPLES];
    device
        .build_output_stream(
            *config,
            move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
                let cfg = shared.load();
                let cache = &caches[sound_index(cfg.sound)];
                if step == 0 {
                    data.fill(T::from_sample(0.0));
                    return;
                }
                for chunk in data.chunks_mut(step) {
                    let buf = &mut scratch[..chunk.len()];
                    clock.render(buf, channels, &cfg, cache);
                    for (dst, &src) in chunk.iter_mut().zip(buf.iter()) {
                        *dst = T::from_sample(src);
                    }
                }
            },
            |err| eprintln!("metronome stream error: {err}"),
            None,
        )
        .map_err(|e| AudioError::Stream(e.to_string()))
}
