//! Behaviour tests for the SQLite store, through its public API only.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use plexustyping_core::{ErrorMode, KeystrokeRecord};
use plexustyping_store::{ConfusionRow, NewSession, StatsSnapshot, Store, StoreError, UnitStatRow};

/// Fresh directory under the system temp dir, removed on drop.
struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "plexustyping-store-test-{}-{nanos}-{n}",
            std::process::id()
        ));
        TempDir(dir)
    }

    /// A DB path in a not-yet-existing nested directory.
    fn db(&self) -> PathBuf {
        self.0.join("nested").join("plexustyping.sqlite3")
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn raw_user_version(path: &Path) -> i64 {
    let conn = rusqlite::Connection::open(path).unwrap();
    conn.query_row("PRAGMA user_version", [], |r| r.get(0))
        .unwrap()
}

fn rec(index: usize, expected: char, typed: char, ms: Option<f64>) -> KeystrokeRecord {
    KeystrokeRecord {
        index,
        expected,
        typed,
        correct: expected == typed,
        ms_since_previous: ms,
    }
}

fn session<'a>(profile_id: i64, started: i64, records: &'a [KeystrokeRecord]) -> NewSession<'a> {
    NewSession {
        profile_id,
        started_at_ms: started,
        mode: "prose",
        code_language: None,
        error_mode: ErrorMode::AllowContinue,
        text: "año",
        duration_ms: 1500.5,
        gross_wpm: 40.0,
        net_wpm: 38.5,
        accuracy: 0.95,
        keystrokes: records.len(),
        bpm: None,
        records,
    }
}

fn unit(name: &str, e: f64) -> UnitStatRow {
    UnitStatRow {
        unit: name.to_string(),
        ema_error: e,
        ema_slowness: e / 2.0,
        samples: 4,
        slowness_samples: 3,
    }
}

#[test]
fn migrates_empty_db_to_current_version() {
    let store = Store::open_in_memory().unwrap();
    assert_eq!(store.schema_version().unwrap(), 1);
}

#[test]
fn open_creates_parent_dir_and_reopen_is_idempotent() {
    let tmp = TempDir::new();
    let path = tmp.db();
    let profile_id;
    {
        let store = Store::open(&path).unwrap();
        assert_eq!(store.schema_version().unwrap(), 1);
        profile_id = store.default_profile().unwrap().id;
        store.set_setting("theme", "dark").unwrap();
    }
    assert!(path.exists());
    let store = Store::open(&path).unwrap();
    assert_eq!(store.schema_version().unwrap(), 1);
    assert_eq!(store.default_profile().unwrap().id, profile_id);
    assert_eq!(store.get_setting("theme").unwrap().as_deref(), Some("dark"));
}

#[test]
fn newer_schema_is_refused_without_modifying_the_db() {
    let tmp = TempDir::new();
    let path = tmp.db();
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    {
        let conn = rusqlite::Connection::open(&path).unwrap();
        conn.execute_batch("PRAGMA user_version = 99;").unwrap();
    }
    match Store::open(&path) {
        Err(StoreError::NewerSchema { found, supported }) => {
            assert_eq!(found, 99);
            assert_eq!(supported, 1);
        }
        other => panic!("expected NewerSchema, got {other:?}"),
    }
    assert_eq!(raw_user_version(&path), 99);
    let conn = rusqlite::Connection::open(&path).unwrap();
    let tables: i64 = conn
        .query_row("SELECT count(*) FROM sqlite_master", [], |r| r.get(0))
        .unwrap();
    assert_eq!(tables, 0);
}

#[test]
fn default_profile_is_created_once() {
    let store = Store::open_in_memory().unwrap();
    let first = store.default_profile().unwrap();
    assert_eq!(first.name, "Principal");
    assert_eq!(first.language, "es");
    assert_eq!(first.layout, "es-qwerty");
    assert_eq!(first.theme, None);
    assert!(first.created_at_ms > 0);
    assert_eq!(store.default_profile().unwrap(), first);
}

#[test]
fn session_and_keystrokes_round_trip() {
    let mut store = Store::open_in_memory().unwrap();
    let pid = store.default_profile().unwrap().id;
    // Order differs from position (a backspace reopened position 1).
    let records = vec![
        rec(0, 'a', 'a', None),
        rec(1, 'ñ', 'n', Some(120.5)),
        rec(1, 'ñ', 'ñ', Some(300.0)),
        rec(2, 'ó', 'ó', Some(95.25)),
    ];
    let mut s = session(pid, 1_000, &records);
    s.error_mode = ErrorMode::BackspaceCorrect;
    s.bpm = Some(80);
    let id = store.save_session(&s).unwrap();

    assert_eq!(store.session_keystrokes(id).unwrap(), records);
    let recent = store.recent_sessions(pid, 10).unwrap();
    assert_eq!(recent.len(), 1);
    let got = &recent[0];
    assert_eq!(got.id, id);
    assert_eq!(got.started_at_ms, 1_000);
    assert_eq!(got.mode, "prose");
    assert_eq!(got.duration_ms, 1500.5);
    assert_eq!(got.gross_wpm, 40.0);
    assert_eq!(got.net_wpm, 38.5);
    assert_eq!(got.accuracy, 0.95);
    assert_eq!(got.keystrokes, 4);
}

#[test]
fn session_keystrokes_of_unknown_session_is_not_found() {
    let store = Store::open_in_memory().unwrap();
    assert!(matches!(
        store.session_keystrokes(42),
        Err(StoreError::NotFound)
    ));
}

#[test]
fn invalid_stored_char_is_an_error_not_a_panic() {
    let tmp = TempDir::new();
    let path = tmp.db();
    let id = {
        let mut store = Store::open(&path).unwrap();
        let pid = store.default_profile().unwrap().id;
        let records = [rec(0, 'a', 'a', None)];
        store.save_session(&session(pid, 1, &records)).unwrap()
    };
    {
        let conn = rusqlite::Connection::open(&path).unwrap();
        conn.execute("UPDATE keystrokes SET expected = 'ab'", [])
            .unwrap();
    }
    let store = Store::open(&path).unwrap();
    assert!(matches!(
        store.session_keystrokes(id),
        Err(StoreError::InvalidData(_))
    ));
}

#[test]
fn save_session_with_unknown_profile_fails_and_stores_nothing() {
    let mut store = Store::open_in_memory().unwrap();
    let pid = store.default_profile().unwrap().id;
    let records = [rec(0, 'a', 'a', None)];
    assert!(
        store
            .save_session(&session(pid + 100, 1, &records))
            .is_err()
    );
    assert!(store.recent_sessions(pid + 100, 10).unwrap().is_empty());
    assert!(store.recent_sessions(pid, 10).unwrap().is_empty());
}

#[test]
fn recent_sessions_newest_first_limited_and_filtered_by_profile() {
    let mut store = Store::open_in_memory().unwrap();
    let pid = store.default_profile().unwrap().id;
    let records = [rec(0, 'a', 'a', None)];
    let a = store.save_session(&session(pid, 100, &records)).unwrap();
    let c = store.save_session(&session(pid, 300, &records)).unwrap();
    let b = store.save_session(&session(pid, 200, &records)).unwrap();

    let ids: Vec<i64> = store
        .recent_sessions(pid, 10)
        .unwrap()
        .iter()
        .map(|s| s.id)
        .collect();
    assert_eq!(ids, vec![c, b, a]);
    let ids: Vec<i64> = store
        .recent_sessions(pid, 2)
        .unwrap()
        .iter()
        .map(|s| s.id)
        .collect();
    assert_eq!(ids, vec![c, b]);
    assert!(store.recent_sessions(pid + 1, 10).unwrap().is_empty());
}

#[test]
fn stats_replace_semantics_and_per_mode_isolation() {
    let mut store = Store::open_in_memory().unwrap();
    let pid = store.default_profile().unwrap().id;
    assert_eq!(
        store.load_stats(pid, "prose").unwrap(),
        StatsSnapshot {
            units: vec![],
            confusions: vec![],
            mean_interval_ms: None
        }
    );

    let first = StatsSnapshot {
        units: vec![unit("a", 0.5), unit("qu", 0.25), unit("ñ", 0.75)],
        confusions: vec![ConfusionRow {
            expected: 'n',
            typed: 'm',
            count: 3,
        }],
        mean_interval_ms: Some(180.5),
    };
    store.save_stats(pid, "prose", &first).unwrap();
    assert_eq!(store.load_stats(pid, "prose").unwrap(), first);

    let code = StatsSnapshot {
        units: vec![unit("{", 0.1)],
        confusions: vec![],
        mean_interval_ms: None,
    };
    store.save_stats(pid, "code", &code).unwrap();

    let second = StatsSnapshot {
        units: vec![unit("é", 0.3)],
        confusions: vec![ConfusionRow {
            expected: 'é',
            typed: 'e',
            count: 1,
        }],
        mean_interval_ms: None,
    };
    store.save_stats(pid, "prose", &second).unwrap();
    assert_eq!(store.load_stats(pid, "prose").unwrap(), second);
    assert_eq!(store.load_stats(pid, "code").unwrap(), code);
}

#[test]
fn failed_save_stats_keeps_previous_snapshot() {
    let mut store = Store::open_in_memory().unwrap();
    let pid = store.default_profile().unwrap().id;
    let good = StatsSnapshot {
        units: vec![unit("a", 0.5)],
        confusions: vec![],
        mean_interval_ms: Some(200.0),
    };
    store.save_stats(pid, "prose", &good).unwrap();
    let bad = StatsSnapshot {
        units: vec![unit("b", 0.1), unit("b", 0.2)],
        confusions: vec![],
        mean_interval_ms: Some(1.0),
    };
    assert!(store.save_stats(pid, "prose", &bad).is_err());
    assert_eq!(store.load_stats(pid, "prose").unwrap(), good);
}

#[test]
fn settings_upsert() {
    let store = Store::open_in_memory().unwrap();
    assert_eq!(store.get_setting("layout").unwrap(), None);
    store.set_setting("layout", "es-qwerty").unwrap();
    store.set_setting("layout", "us-qwerty").unwrap();
    assert_eq!(
        store.get_setting("layout").unwrap().as_deref(),
        Some("us-qwerty")
    );
}
