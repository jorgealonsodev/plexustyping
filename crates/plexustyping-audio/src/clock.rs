//! Sample-accurate beat scheduler.

use crate::{MAX_BPM, MIN_BPM, MetronomeConfig, SoundCache};

/// Pure scheduler deciding on which output frame each beat starts.
///
/// Beat `k` starts at frame `round(k * sample_rate * 60 / bpm)`. A BPM change
/// keeps the already scheduled next beat and applies from the beat after it,
/// measured from the beat where it was picked up.
#[derive(Clone, Debug)]
pub struct BeatClock {
    sample_rate: u64,
    anchor_bpm: u32,
    anchor_frame: u64,
    anchor_beat: u64,
    next_beat: u64,
    next_frame: u64,
    frame: u64,
    voice: Option<Voice>,
}

#[derive(Clone, Copy, Debug)]
struct Voice {
    accent: bool,
    pos: usize,
}

impl BeatClock {
    /// Creates a clock whose beat 0 starts at the first rendered frame.
    pub fn new(sample_rate: u32, bpm: u32) -> Self {
        Self {
            sample_rate: u64::from(sample_rate.max(1)),
            anchor_bpm: bpm.clamp(MIN_BPM, MAX_BPM),
            anchor_frame: 0,
            anchor_beat: 0,
            next_beat: 0,
            next_frame: 0,
            frame: 0,
            voice: None,
        }
    }

    /// Fills interleaved `out` (`channels` samples per frame) with the
    /// metronome signal: the same scaled sample on every channel, silence
    /// between beats. Never allocates or blocks.
    pub fn render(
        &mut self,
        out: &mut [f32],
        channels: usize,
        cfg: &MetronomeConfig,
        cache: &SoundCache,
    ) {
        let volume = if cfg.volume.is_finite() {
            cfg.volume.clamp(0.0, 1.0)
        } else {
            0.0
        };
        for frame in out.chunks_mut(channels.max(1)) {
            if self.frame == self.next_frame {
                self.start_beat(cfg);
            }
            let sample = match self.voice.as_mut() {
                Some(voice) => match cache.samples(voice.accent).get(voice.pos) {
                    Some(&s) => {
                        voice.pos += 1;
                        s
                    }
                    None => {
                        self.voice = None;
                        0.0
                    }
                },
                None => 0.0,
            };
            frame.fill(sample * volume);
            self.frame += 1;
        }
    }

    /// Number of beats started so far.
    pub fn beats_started(&self) -> u64 {
        self.next_beat
    }

    fn start_beat(&mut self, cfg: &MetronomeConfig) {
        let k = self.next_beat;
        let accent = cfg.accent_every > 0 && k % u64::from(cfg.accent_every) == 0;
        self.voice = Some(Voice { accent, pos: 0 });
        self.next_beat += 1;
        let bpm = cfg.bpm.clamp(MIN_BPM, MAX_BPM);
        if bpm != self.anchor_bpm {
            self.anchor_bpm = bpm;
            self.anchor_frame = self.frame;
            self.anchor_beat = k;
        }
        let bpm = u64::from(bpm);
        let beats = self.next_beat - self.anchor_beat;
        // Integer round-half-up of beats * rate * 60 / bpm.
        self.next_frame = self.anchor_frame + (2 * beats * self.sample_rate * 60 + bpm) / (2 * bpm);
    }
}
