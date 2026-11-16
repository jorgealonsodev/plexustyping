//! Sound synthesis and the per-rate sample cache.

use crate::Sound;
use std::f32::consts::TAU;

/// Synthesizes one beat as mono samples in `-1.0..=1.0`, shorter than 60 ms.
///
/// An accented beat is louder and higher-pitched (brighter for `Click`).
/// Output is deterministic for a given input.
pub fn synthesize(sound: Sound, sample_rate: u32, accent: bool) -> Vec<f32> {
    let rate = sample_rate.max(1) as f32;
    let (gain, pitch) = if accent { (0.95, 1.25) } else { (0.55, 1.0) };
    let len_ms = match sound {
        Sound::Click => 12.0,
        Sound::Wood => 40.0,
        Sound::Beep => 50.0,
    };
    let len = ((len_ms / 1000.0 * rate) as usize).max(1);
    let mut noise = Noise(0x9E37_79B9);
    let mut prev_noise = 0.0;
    (0..len)
        .map(|i| {
            let t = i as f32 / rate;
            let n = noise.next();
            let s = match sound {
                Sound::Click => {
                    // The accent uses differentiated (high-passed, brighter) noise.
                    let burst = if accent { (n - prev_noise) * 0.7 } else { n };
                    prev_noise = n;
                    burst * (-t / 0.0015).exp()
                }
                Sound::Wood => {
                    let tone = (TAU * 1000.0 * pitch * t).sin() * 0.9;
                    (tone + n * 0.1 * (-t / 0.001).exp()) * (-t / 0.006).exp()
                }
                Sound::Beep => {
                    let attack = (t / 0.003).min(1.0);
                    let release = 1.0 - i as f32 / len as f32;
                    (TAU * 880.0 * pitch * t).sin() * attack * release * release
                }
            };
            (s * gain).clamp(-1.0, 1.0)
        })
        .collect()
}

/// Deterministic xorshift noise in `-1.0..1.0`.
struct Noise(u32);

impl Noise {
    fn next(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        self.0 as f32 / u32::MAX as f32 * 2.0 - 1.0
    }
}

/// Precomputed normal and accent samples for one sound at one sample rate,
/// so the audio callback never synthesizes or allocates.
#[derive(Clone, Debug)]
pub struct SoundCache {
    normal: Vec<f32>,
    accent: Vec<f32>,
}

impl SoundCache {
    /// Synthesizes both variants of `sound` at `sample_rate`.
    pub fn new(sound: Sound, sample_rate: u32) -> Self {
        Self {
            normal: synthesize(sound, sample_rate, false),
            accent: synthesize(sound, sample_rate, true),
        }
    }

    /// Cached mono samples for a normal or accented beat.
    pub fn samples(&self, accent: bool) -> &[f32] {
        if accent { &self.accent } else { &self.normal }
    }
}
