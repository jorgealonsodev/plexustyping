//! Behaviour tests for the pure metronome parts (no audio device needed).

use plexustyping_audio::{
    AudioError, BeatClock, MAX_BPM, MIN_BPM, Metronome, MetronomeConfig, Sound, SoundCache,
    beat_interval_ms, synthesize,
};

const ALL_SOUNDS: [Sound; 3] = [Sound::Click, Sound::Wood, Sound::Beep];

fn cfg(bpm: u32) -> MetronomeConfig {
    MetronomeConfig {
        bpm,
        sound: Sound::Wood,
        volume: 1.0,
        accent_every: 0,
    }
}

/// Exact `round(k * sample_rate * 60 / bpm)` in integer arithmetic.
fn beat_frame(k: u64, sample_rate: u32, bpm: u32) -> usize {
    let num = 2 * k * u64::from(sample_rate) * 60 + u64::from(bpm);
    (num / (2 * u64::from(bpm))) as usize
}

/// Mono reference output: each beat's cached samples scaled by volume.
fn expected(beats: &[(usize, bool)], total: usize, cache: &SoundCache, volume: f32) -> Vec<f32> {
    let mut out = vec![0.0; total];
    for &(start, accent) in beats {
        for (i, s) in cache.samples(accent).iter().enumerate() {
            if let Some(slot) = out.get_mut(start + i) {
                *slot = s * volume;
            }
        }
    }
    out
}

/// Renders `total` frames in chunks of the given (odd) sizes, cycling.
fn render_chunked(
    clock: &mut BeatClock,
    total: usize,
    chunks: &[usize],
    channels: usize,
    cfg: &MetronomeConfig,
    cache: &SoundCache,
) -> Vec<f32> {
    let mut out = vec![9.0; total * channels];
    let mut pos = 0;
    for &n in chunks.iter().cycle() {
        if pos >= total {
            break;
        }
        let end = (pos + n).min(total);
        clock.render(
            &mut out[pos * channels..end * channels],
            channels,
            cfg,
            cache,
        );
        pos = end;
    }
    out
}

fn rms(s: &[f32]) -> f32 {
    (s.iter().map(|x| x * x).sum::<f32>() / s.len() as f32).sqrt()
}

fn zero_crossings(s: &[f32]) -> usize {
    s.windows(2)
        .filter(|w| (w[0] < 0.0) != (w[1] < 0.0))
        .count()
}

#[test]
fn validation_accepts_bpm_bounds_and_rejects_outside() {
    assert_eq!((MIN_BPM, MAX_BPM), (30, 300));
    for bpm in [30, 300] {
        assert_eq!(cfg(bpm).validated().unwrap(), cfg(bpm));
    }
    for bpm in [0, 29, 301] {
        assert!(matches!(
            cfg(bpm).validated(),
            Err(AudioError::InvalidConfig(_))
        ));
    }
}

#[test]
fn validation_checks_volume_and_accent() {
    for volume in [0.0, 0.5, 1.0] {
        assert!(MetronomeConfig { volume, ..cfg(60) }.validated().is_ok());
    }
    for volume in [-0.01, 1.01, f32::NAN] {
        assert!(matches!(
            MetronomeConfig { volume, ..cfg(60) }.validated(),
            Err(AudioError::InvalidConfig(_))
        ));
    }
    for accent_every in [0, 1, 16] {
        let c = MetronomeConfig {
            accent_every,
            ..cfg(60)
        };
        assert!(c.validated().is_ok());
    }
    let c = MetronomeConfig {
        accent_every: 17,
        ..cfg(60)
    };
    assert!(matches!(c.validated(), Err(AudioError::InvalidConfig(_))));
}

#[test]
fn config_serializes_camel_case_with_kebab_sound() {
    let c = MetronomeConfig {
        accent_every: 4,
        ..cfg(120)
    };
    let json = serde_json::to_value(c).unwrap();
    assert_eq!(
        json,
        serde_json::json!({"bpm": 120, "sound": "wood", "volume": 1.0, "accentEvery": 4})
    );
    let back: MetronomeConfig = serde_json::from_value(json).unwrap();
    assert_eq!(back, c);
}

#[test]
fn beat_interval_is_sixty_thousand_over_bpm() {
    assert_eq!(beat_interval_ms(60), 1000.0);
    assert_eq!(beat_interval_ms(120), 500.0);
    assert_eq!(beat_interval_ms(300), 200.0);
    assert!((beat_interval_ms(70) - 857.142_857).abs() < 1e-3);
}

#[test]
fn synthesized_sounds_are_short_bounded_and_distinct() {
    for rate in [44_100, 48_000] {
        let max_len = rate as usize * 60 / 1000;
        let mut seen: Vec<Vec<f32>> = Vec::new();
        for sound in ALL_SOUNDS {
            for accent in [false, true] {
                let s = synthesize(sound, rate, accent);
                assert!(
                    !s.is_empty() && s.len() < max_len,
                    "{sound:?} len {}",
                    s.len()
                );
                assert!(s.iter().all(|x| x.is_finite() && (-1.0..=1.0).contains(x)));
                assert!(s.iter().any(|x| x.abs() > 0.1), "{sound:?} audible");
            }
            let normal = synthesize(sound, rate, false);
            assert!(!seen.contains(&normal), "{sound:?} distinct");
            seen.push(normal);
        }
    }
}

#[test]
fn accent_is_louder_and_brighter() {
    for sound in ALL_SOUNDS {
        let normal = synthesize(sound, 48_000, false);
        let accent = synthesize(sound, 48_000, true);
        assert!(rms(&accent) > rms(&normal) * 1.2, "{sound:?} louder");
    }
    for sound in [Sound::Wood, Sound::Beep] {
        let n = synthesize(sound, 48_000, false);
        let a = synthesize(sound, 48_000, true);
        let len = n.len().min(a.len());
        assert!(
            zero_crossings(&a[..len]) > zero_crossings(&n[..len]),
            "{sound:?} higher pitch"
        );
    }
}

#[test]
fn beats_land_on_exact_frames_across_odd_buffers() {
    let (rate, bpm) = (44_100, 137);
    let cache = SoundCache::new(Sound::Click, rate);
    let c = MetronomeConfig {
        sound: Sound::Click,
        ..cfg(bpm)
    };
    let total = beat_frame(6, rate, bpm) + 100;
    let mut clock = BeatClock::new(rate, bpm);
    let out = render_chunked(&mut clock, total, &[1, 7, 333, 509, 1021], 1, &c, &cache);
    let beats: Vec<_> = (0..=6).map(|k| (beat_frame(k, rate, bpm), false)).collect();
    assert_eq!(out, expected(&beats, total, &cache, 1.0));
    assert_eq!(clock.beats_started(), 7);
    // Silence between beats.
    let gap_start = beat_frame(0, rate, bpm) + cache.samples(false).len();
    assert!(
        out[gap_start..beat_frame(1, rate, bpm)]
            .iter()
            .all(|&x| x == 0.0)
    );
}

#[test]
fn accent_marks_every_nth_beat() {
    let rate = 6_000;
    let cache = SoundCache::new(Sound::Wood, rate);
    assert_ne!(cache.samples(true), cache.samples(false));
    let c = MetronomeConfig {
        accent_every: 3,
        ..cfg(120)
    };
    let total = 3_000 * 7;
    let mut clock = BeatClock::new(rate, 120);
    let out = render_chunked(&mut clock, total, &[997], 1, &c, &cache);
    let beats: Vec<_> = (0..7).map(|k| (k * 3_000, k % 3 == 0)).collect();
    assert_eq!(out, expected(&beats, total, &cache, 1.0));
}

#[test]
fn every_channel_gets_the_same_signal_scaled_by_volume() {
    let rate = 6_000;
    let cache = SoundCache::new(Sound::Beep, rate);
    let c = MetronomeConfig {
        sound: Sound::Beep,
        volume: 0.5,
        accent_every: 2,
        ..cfg(60)
    };
    let total = 6_000 * 3 + 10;
    let mut clock = BeatClock::new(rate, 60);
    let out = render_chunked(&mut clock, total, &[13, 255], 2, &c, &cache);
    let mono = expected(
        &[(0, true), (6_000, false), (12_000, true), (18_000, false)],
        total,
        &cache,
        0.5,
    );
    for (frame, want) in out.chunks(2).zip(&mono) {
        assert_eq!(frame, [*want, *want]);
    }
}

#[test]
fn bpm_change_applies_from_next_beat() {
    let rate = 6_000;
    let cache = SoundCache::new(Sound::Wood, rate);
    let mut clock = BeatClock::new(rate, 60);
    let mut out = vec![0.0; 15_000];
    clock.render(&mut out[..3_000], 1, &cfg(60), &cache);
    // Changed mid-interval: beat 1 keeps its 60 BPM slot (6000), then 120 BPM.
    clock.render(&mut out[3_000..], 1, &cfg(120), &cache);
    let beats = [(0, false), (6_000, false), (9_000, false), (12_000, false)];
    assert_eq!(out, expected(&beats, 15_000, &cache, 1.0));
    assert_eq!(clock.beats_started(), 4);
}

#[test]
fn zero_volume_is_silent_but_still_counts_beats() {
    let rate = 6_000;
    let cache = SoundCache::new(Sound::Click, rate);
    let c = MetronomeConfig {
        volume: 0.0,
        ..cfg(120)
    };
    let mut clock = BeatClock::new(rate, 120);
    let out = render_chunked(&mut clock, 9_001, &[111], 1, &c, &cache);
    assert!(out.iter().all(|&x| x == 0.0));
    assert_eq!(clock.beats_started(), 4);
}

#[test]
fn metronome_is_send() {
    fn assert_send<T: Send>() {}
    assert_send::<Metronome>();
}

/// Plays one second on the default output device. Run with `--ignored`.
#[test]
#[ignore = "needs an audio output device"]
fn plays_on_default_device() {
    let m = Metronome::start(MetronomeConfig {
        accent_every: 4,
        ..cfg(240)
    })
    .expect("start");
    assert!(m.started_at() <= std::time::Instant::now());
    std::thread::sleep(std::time::Duration::from_millis(500));
    m.update(cfg(120)).expect("update");
    assert!(m.update(cfg(301)).is_err());
    std::thread::sleep(std::time::Duration::from_millis(500));
    m.stop();
}
