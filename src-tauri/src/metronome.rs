//! Metronome commands logic (S7): one running metronome at a time, with the
//! wall-clock time of beat 0 so the frontend can measure keystroke offsets.

use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use plexustyping_audio::{AudioError, Metronome, MetronomeConfig};
use serde::Serialize;

/// A playing metronome; dropping it stops the sound.
pub trait Player: Send {
    /// Applies a config with the same tempo live.
    fn update(&self, cfg: MetronomeConfig) -> Result<(), AudioError>;
    /// Instant of beat 0.
    fn started_at(&self) -> Instant;
}

impl Player for Metronome {
    fn update(&self, cfg: MetronomeConfig) -> Result<(), AudioError> {
        Metronome::update(self, cfg)
    }
    fn started_at(&self) -> Instant {
        Metronome::started_at(self)
    }
}

/// Opens a player (the audio device in the app, a fake in tests).
pub type Starter = dyn Fn(MetronomeConfig) -> Result<Box<dyn Player>, AudioError> + Send + Sync;

/// Beat 0 of the running metronome, in ms since the UNIX epoch.
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MetronomeStarted {
    pub epoch_ms: f64,
}

struct Running {
    player: Box<dyn Player>,
    bpm: u32,
    epoch_ms: f64,
}

/// Managed state: the running metronome, if any. Cheap to clone.
#[derive(Clone)]
pub struct MetronomeState {
    running: Arc<Mutex<Option<Running>>>,
    starter: Arc<Starter>,
}

impl MetronomeState {
    pub fn new(starter: Arc<Starter>) -> Self {
        MetronomeState {
            running: Arc::new(Mutex::new(None)),
            starter,
        }
    }

    /// The default output device.
    pub fn audio() -> Self {
        Self::new(Arc::new(|cfg| {
            Metronome::start(cfg).map(|m| Box::new(m) as Box<dyn Player>)
        }))
    }

    fn lock(&self) -> MutexGuard<'_, Option<Running>> {
        self.running.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Starts the metronome; a running one is stopped first (beat 0 restarts).
    pub fn start(&self, cfg: MetronomeConfig) -> Result<MetronomeStarted, String> {
        let cfg = cfg.validated().map_err(spanish)?;
        drop(self.lock().take());
        let player = (self.starter)(cfg).map_err(spanish)?;
        let epoch_ms = epoch_ms(player.started_at(), Instant::now(), now_epoch_ms());
        let running = Running {
            player,
            bpm: cfg.bpm,
            epoch_ms,
        };
        // A metronome started meanwhile is dropped after the lock is released.
        let displaced = self.lock().replace(running);
        drop(displaced);
        Ok(MetronomeStarted { epoch_ms })
    }

    /// Applies `cfg` live; a new tempo restarts beat 0. Returns the current beat 0,
    /// or `None` (and does nothing) when the metronome is stopped.
    pub fn update(&self, cfg: MetronomeConfig) -> Result<Option<MetronomeStarted>, String> {
        let cfg = cfg.validated().map_err(spanish)?;
        {
            let guard = self.lock();
            let Some(running) = guard.as_ref() else {
                return Ok(None);
            };
            if running.bpm == cfg.bpm {
                running.player.update(cfg).map_err(spanish)?;
                return Ok(Some(MetronomeStarted {
                    epoch_ms: running.epoch_ms,
                }));
            }
        }
        self.start(cfg).map(Some)
    }

    /// Stops the metronome; a no-op when stopped.
    pub fn stop(&self) {
        let running = self.lock().take();
        drop(running);
    }

    /// Whether a metronome is playing.
    pub fn is_running(&self) -> bool {
        self.lock().is_some()
    }
}

/// The audio error, for the user.
pub fn spanish(error: AudioError) -> String {
    match error {
        AudioError::InvalidConfig(_) => {
            "Configuración del metrónomo no válida: tempo de 30 a 300 BPM, \
             volumen de 0 a 100 % y acento cada 0 a 16 pulsaciones"
                .into()
        }
        AudioError::NoOutputDevice => "No hay dispositivo de audio".into(),
        AudioError::Stream(detail) => format!("No se pudo reproducir el audio: {detail}"),
    }
}

/// Wall-clock ms of `instant`, given the same moment as an `Instant` and as epoch ms.
pub fn epoch_ms(instant: Instant, now: Instant, now_epoch_ms: f64) -> f64 {
    match now.checked_duration_since(instant) {
        Some(ago) => now_epoch_ms - ago.as_secs_f64() * 1000.0,
        None => now_epoch_ms + instant.duration_since(now).as_secs_f64() * 1000.0,
    }
}

fn now_epoch_ms() -> f64 {
    let now = SystemTime::now().duration_since(UNIX_EPOCH);
    now.map(|d| d.as_secs_f64() * 1000.0).unwrap_or(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use plexustyping_audio::Sound;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;

    #[derive(Default)]
    struct Counts {
        starts: AtomicUsize,
        updates: AtomicUsize,
        drops: AtomicUsize,
    }

    struct Fake {
        counts: Arc<Counts>,
        at: Instant,
    }

    impl Player for Fake {
        fn update(&self, _: MetronomeConfig) -> Result<(), AudioError> {
            self.counts.updates.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
        fn started_at(&self) -> Instant {
            self.at
        }
    }

    impl Drop for Fake {
        fn drop(&mut self) {
            self.counts.drops.fetch_add(1, Ordering::SeqCst);
        }
    }

    fn fake() -> (MetronomeState, Arc<Counts>) {
        let counts = Arc::new(Counts::default());
        let c = Arc::clone(&counts);
        let state = MetronomeState::new(Arc::new(move |_| {
            c.starts.fetch_add(1, Ordering::SeqCst);
            let at = Instant::now();
            Ok(Box::new(Fake {
                counts: Arc::clone(&c),
                at,
            }) as Box<dyn Player>)
        }));
        (state, counts)
    }

    fn cfg(bpm: u32) -> MetronomeConfig {
        MetronomeConfig {
            bpm,
            sound: Sound::Click,
            volume: 0.5,
            accent_every: 4,
        }
    }

    fn counts(c: &Counts) -> [usize; 3] {
        [&c.starts, &c.updates, &c.drops].map(|n| n.load(Ordering::SeqCst))
    }

    #[test]
    fn start_restarts_a_running_metronome_and_stop_is_idempotent() {
        let (state, c) = fake();
        let first = state.start(cfg(60)).unwrap();
        assert!(state.is_running());
        std::thread::sleep(Duration::from_millis(5));
        let second = state.start(cfg(60)).unwrap();
        assert!(second.epoch_ms > first.epoch_ms);
        assert_eq!(counts(&c), [2, 0, 1]);
        state.stop();
        state.stop();
        assert!(!state.is_running());
        assert_eq!(counts(&c), [2, 0, 2]);
    }

    #[test]
    fn update_keeps_beat_0_unless_the_tempo_changes_and_is_a_no_op_when_stopped() {
        let (state, c) = fake();
        assert_eq!(state.update(cfg(60)), Ok(None));
        assert_eq!(counts(&c), [0, 0, 0]);
        let started = state.start(cfg(60)).unwrap();
        assert_eq!(state.update(cfg(60)), Ok(Some(started)));
        assert_eq!(counts(&c), [1, 1, 0]);
        let restarted = state.update(cfg(120)).unwrap().unwrap();
        assert_ne!(restarted, started);
        assert_eq!(counts(&c), [2, 1, 1]);
        assert_eq!(state.update(cfg(120)), Ok(Some(restarted)));
    }

    #[test]
    fn invalid_configs_are_refused_in_spanish_and_change_nothing() {
        let (state, c) = fake();
        state.start(cfg(60)).unwrap();
        let volume = MetronomeConfig {
            volume: 1.5,
            ..cfg(60)
        };
        let accent = MetronomeConfig {
            accent_every: 17,
            ..cfg(60)
        };
        for bad in [cfg(29), cfg(301), volume, accent] {
            let start = state.start(bad).unwrap_err();
            assert!(
                start.starts_with("Configuración del metrónomo no válida"),
                "{start}"
            );
            assert_eq!(state.update(bad).unwrap_err(), start);
        }
        assert!(state.is_running());
        assert_eq!(counts(&c), [1, 0, 0]);
    }

    #[test]
    fn a_device_error_leaves_the_metronome_stopped() {
        let state = MetronomeState::new(Arc::new(|_| Err(AudioError::NoOutputDevice)));
        assert_eq!(
            state.start(cfg(60)),
            Err("No hay dispositivo de audio".into())
        );
        assert!(!state.is_running());
        assert_eq!(
            spanish(AudioError::Stream("boom".into())),
            "No se pudo reproducir el audio: boom"
        );
    }

    #[test]
    fn epoch_ms_converts_past_and_future_instants() {
        let now = Instant::now();
        let past = now - Duration::from_millis(250);
        assert!((epoch_ms(past, now, 10_000.0) - 9_750.0).abs() < 1e-6);
        let future = now + Duration::from_millis(40);
        assert!((epoch_ms(future, now, 10_000.0) - 10_040.0).abs() < 1e-6);
    }

    #[test]
    fn started_serializes_as_camel_case() {
        let json = serde_json::to_value(MetronomeStarted { epoch_ms: 1.5 }).unwrap();
        assert_eq!(json, serde_json::json!({ "epochMs": 1.5 }));
    }

    /// Needs a real output device: `cargo test -p plexustyping-app -- --ignored`.
    #[test]
    #[ignore]
    fn audio_device_starts_and_stops() {
        let state = MetronomeState::audio();
        let started = state.start(cfg(120)).unwrap();
        assert!((started.epoch_ms - now_epoch_ms()).abs() < 5_000.0);
        std::thread::sleep(Duration::from_millis(600));
        state.stop();
        assert!(!state.is_running());
    }
}
