//! Fixed-BPM metronome for PlexusTyping: synthesized sounds, a sample-accurate
//! beat scheduler, and a low-latency `cpal` output stream.
//!
//! The pure parts ([`synthesize`], [`SoundCache`], [`BeatClock`]) need no audio
//! device; [`Metronome`] drives them from the default output device.

mod clock;
mod device;
mod sound;

pub use clock::BeatClock;
pub use device::Metronome;
pub use sound::{SoundCache, synthesize};

use serde::{Deserialize, Serialize};

/// Lowest supported tempo, in beats per minute.
pub const MIN_BPM: u32 = 30;
/// Highest supported tempo, in beats per minute.
pub const MAX_BPM: u32 = 300;
/// Highest supported accent period (`accent_every`).
pub const MAX_ACCENT_EVERY: u32 = 16;

/// Metronome sound, synthesized at runtime (no audio files).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Sound {
    /// Short noise burst with a fast decay.
    Click,
    /// Damped ~1 kHz sine with a little noise, like a wood block.
    Wood,
    /// ~880 Hz sine with a soft envelope.
    Beep,
}

/// User-facing metronome settings.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MetronomeConfig {
    /// Tempo; one beat is one expected keystroke. Valid range [`MIN_BPM`]..=[`MAX_BPM`].
    pub bpm: u32,
    /// Sound played on each beat.
    pub sound: Sound,
    /// Linear gain in `0.0..=1.0`.
    pub volume: f32,
    /// Accent every N beats (beat 0 included); `0` disables the accent.
    pub accent_every: u32,
}

impl MetronomeConfig {
    /// Returns the config unchanged if every field is in range.
    ///
    /// # Errors
    /// [`AudioError::InvalidConfig`] when `bpm`, `volume` or `accent_every` is out of range.
    pub fn validated(self) -> Result<Self, AudioError> {
        if !(MIN_BPM..=MAX_BPM).contains(&self.bpm) {
            return Err(AudioError::InvalidConfig(format!(
                "bpm must be between {MIN_BPM} and {MAX_BPM}, got {}",
                self.bpm
            )));
        }
        if !(0.0..=1.0).contains(&self.volume) {
            return Err(AudioError::InvalidConfig(format!(
                "volume must be between 0 and 1, got {}",
                self.volume
            )));
        }
        if self.accent_every > MAX_ACCENT_EVERY {
            return Err(AudioError::InvalidConfig(format!(
                "accent_every must be at most {MAX_ACCENT_EVERY}, got {}",
                self.accent_every
            )));
        }
        Ok(self)
    }
}

/// Errors from configuring or running the metronome.
#[derive(Debug, thiserror::Error)]
pub enum AudioError {
    /// A config field is out of range.
    #[error("invalid metronome config: {0}")]
    InvalidConfig(String),
    /// The host has no default output device.
    #[error("no audio output device available")]
    NoOutputDevice,
    /// The output stream could not be configured, built or played.
    #[error("audio stream error: {0}")]
    Stream(String),
}

/// Milliseconds between two beats: `60000 / bpm`.
pub fn beat_interval_ms(bpm: u32) -> f64 {
    60_000.0 / f64::from(bpm)
}
