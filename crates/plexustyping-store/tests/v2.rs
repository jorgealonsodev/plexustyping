//! Schema v2: v1 → v2 upgrade, generated-text cache and AI usage counters.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};

use plexustyping_store::{
    CachedText, NewCachedText, Store, StoreError, TEXT_CACHE_LIMIT, UsageRow,
};

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
        TempDir(std::env::temp_dir().join(format!(
            "plexustyping-store-v2-{}-{nanos}-{n}",
            std::process::id()
        )))
    }

    fn db(&self) -> PathBuf {
        std::fs::create_dir_all(&self.0).unwrap();
        self.0.join("plexustyping.sqlite3")
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Schema v1 exactly as released (frozen: users' databases have it).
const V1_SCHEMA: &str = r#"
CREATE TABLE profiles (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    language TEXT NOT NULL,
    layout TEXT NOT NULL,
    theme TEXT,
    goals TEXT,
    created_at_ms INTEGER NOT NULL
);
CREATE TABLE sessions (
    id INTEGER PRIMARY KEY,
    profile_id INTEGER NOT NULL REFERENCES profiles(id) ON DELETE CASCADE,
    started_at_ms INTEGER NOT NULL,
    mode TEXT NOT NULL,
    code_language TEXT,
    language TEXT,
    layout TEXT,
    error_mode TEXT NOT NULL,
    text TEXT NOT NULL,
    duration_ms REAL NOT NULL,
    gross_wpm REAL NOT NULL,
    net_wpm REAL NOT NULL,
    accuracy REAL NOT NULL,
    keystrokes INTEGER NOT NULL,
    bpm INTEGER
);
CREATE INDEX sessions_by_profile_time ON sessions(profile_id, started_at_ms);
CREATE TABLE keystrokes (
    session_id INTEGER NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
    idx INTEGER NOT NULL,
    position INTEGER NOT NULL,
    expected TEXT NOT NULL,
    typed TEXT NOT NULL,
    correct INTEGER NOT NULL,
    ms_since_previous REAL,
    metronome_offset_ms REAL,
    PRIMARY KEY (session_id, idx)
);
CREATE TABLE char_stats (
    profile_id INTEGER NOT NULL REFERENCES profiles(id) ON DELETE CASCADE,
    mode TEXT NOT NULL,
    unit TEXT NOT NULL,
    ema_error REAL NOT NULL,
    ema_slowness REAL NOT NULL,
    samples INTEGER NOT NULL,
    slowness_samples INTEGER NOT NULL,
    PRIMARY KEY (profile_id, mode, unit)
);
CREATE TABLE confusions (
    profile_id INTEGER NOT NULL REFERENCES profiles(id) ON DELETE CASCADE,
    mode TEXT NOT NULL,
    expected TEXT NOT NULL,
    typed TEXT NOT NULL,
    count INTEGER NOT NULL,
    PRIMARY KEY (profile_id, mode, expected, typed)
);
CREATE TABLE profile_stats (
    profile_id INTEGER NOT NULL REFERENCES profiles(id) ON DELETE CASCADE,
    mode TEXT NOT NULL,
    mean_interval_ms REAL,
    PRIMARY KEY (profile_id, mode)
);
CREATE TABLE settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
"#;

fn strings(list: &[&str]) -> Vec<String> {
    list.iter().map(|s| (*s).to_owned()).collect()
}

fn cache(store: &Store, language: &str, targets: &[&str], text: &str, used: bool) -> i64 {
    let targets = strings(targets);
    store
        .cache_text(&NewCachedText {
            provider: "claude",
            model: "claude-haiku-5-5",
            language,
            mode: "prose",
            targets: &targets,
            text,
            created_at_ms: 1_000,
            used,
        })
        .unwrap()
}

fn take(store: &Store, targets: &[&str]) -> Option<String> {
    store
        .take_cached_text("es", "prose", &strings(targets))
        .unwrap()
        .map(|c| c.text)
}

#[test]
fn fresh_database_reaches_v2() {
    let store = Store::open_in_memory().unwrap();
    assert_eq!(store.schema_version().unwrap(), 2);
    let tmp = TempDir::new();
    let path = tmp.db();
    assert_eq!(Store::open(&path).unwrap().schema_version().unwrap(), 2);
}

#[test]
fn a_v1_file_is_upgraded_to_v2_keeping_its_data() {
    let tmp = TempDir::new();
    let path = tmp.db();
    {
        let c = rusqlite::Connection::open(&path).unwrap();
        c.execute_batch(V1_SCHEMA).unwrap();
        c.execute_batch(
            "INSERT INTO profiles (name, language, layout, theme, created_at_ms)
                 VALUES ('Principal', 'es', 'es-qwerty', NULL, 5);
             INSERT INTO sessions (profile_id, started_at_ms, mode, error_mode, text,
                 duration_ms, gross_wpm, net_wpm, accuracy, keystrokes)
                 VALUES (1, 77, 'prose', 'allow-continue', 'hola', 1000, 40, 38, 0.9, 4);
             INSERT INTO settings (key, value) VALUES ('theme', 'dark');
             PRAGMA user_version = 1;",
        )
        .unwrap();
    }
    let store = Store::open(&path).unwrap();
    assert_eq!(store.schema_version().unwrap(), 2);
    let profile = store.default_profile().unwrap();
    assert_eq!((profile.id, profile.created_at_ms), (1, 5));
    let sessions = store.recent_sessions(profile.id, 5).unwrap();
    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].started_at_ms, 77);
    assert_eq!(store.get_setting("theme").unwrap().as_deref(), Some("dark"));
    // The new tables work on the upgraded file.
    cache(&store, "es", &["ñ"], "año", false);
    store.record_usage("claude", "2026-11-16", 10, 5).unwrap();
    drop(store);
    let store = Store::open(&path).unwrap();
    assert_eq!(take(&store, &["ñ"]).as_deref(), Some("año"));
    assert_eq!(store.usage_totals("2026-11-16").unwrap()[0].calls_total, 1);
}

#[test]
fn a_newer_schema_is_still_refused() {
    let tmp = TempDir::new();
    let path = tmp.db();
    rusqlite::Connection::open(&path)
        .unwrap()
        .execute_batch("PRAGMA user_version = 3;")
        .unwrap();
    let before = std::fs::read(&path).unwrap();
    match Store::open(&path) {
        Err(StoreError::NewerSchema { found, supported }) => {
            assert_eq!((found, supported), (3, 2));
        }
        other => panic!("expected NewerSchema, got {other:?}"),
    }
    assert_eq!(before, std::fs::read(&path).unwrap());
}

#[test]
fn take_returns_the_full_entry_and_marks_it_used() {
    let store = Store::open_in_memory().unwrap();
    let id = cache(&store, "es", &["ñ", "qu"], "queso añejo", false);
    let got = store
        .take_cached_text("es", "prose", &strings(&["ñ"]))
        .unwrap();
    assert_eq!(
        got,
        Some(CachedText {
            id,
            provider: "claude".into(),
            model: "claude-haiku-5-5".into(),
            language: "es".into(),
            mode: "prose".into(),
            targets: strings(&["ñ", "qu"]),
            text: "queso añejo".into(),
            created_at_ms: 1_000,
        })
    );
    // Used now, but still the least recently used (and only) text.
    assert_eq!(take(&store, &["ñ"]).as_deref(), Some("queso añejo"));
}

#[test]
fn take_prefers_unused_texts_sharing_the_most_targets() {
    let store = Store::open_in_memory().unwrap();
    cache(&store, "es", &["z"], "one", false);
    cache(&store, "es", &["ñ", "z"], "two", false);
    cache(&store, "es", &["ñ", "z", "qu"], "three", false);
    cache(&store, "es", &["ñ", "z", "qu"], "used", true);
    assert_eq!(take(&store, &["ñ", "z", "qu"]).as_deref(), Some("three"));
    assert_eq!(take(&store, &["ñ", "z", "qu"]).as_deref(), Some("two"));
    assert_eq!(take(&store, &["ñ", "z", "qu"]).as_deref(), Some("one"));
}

#[test]
fn take_falls_back_to_any_unused_text_then_the_least_recently_used() {
    let store = Store::open_in_memory().unwrap();
    assert_eq!(take(&store, &["ñ"]), None);
    cache(&store, "es", &["v"], "first", false);
    cache(&store, "es", &["x"], "second", false);
    // Nothing shares "ñ": unused texts, oldest first.
    assert_eq!(take(&store, &["ñ"]).as_deref(), Some("first"));
    assert_eq!(take(&store, &["ñ"]).as_deref(), Some("second"));
    // All used: least recently used first, and taking one makes it the most recent.
    assert_eq!(take(&store, &["x"]).as_deref(), Some("first"));
    assert_eq!(take(&store, &["v"]).as_deref(), Some("second"));
    assert_eq!(take(&store, &["v"]).as_deref(), Some("first"));
}

#[test]
fn take_only_considers_the_requested_language_and_mode() {
    let store = Store::open_in_memory().unwrap();
    cache(&store, "en", &["z"], "zebra", false);
    assert_eq!(take(&store, &["z"]), None);
    assert_eq!(
        store
            .take_cached_text("es", "code", &strings(&["z"]))
            .unwrap(),
        None
    );
    assert_eq!(
        store
            .take_cached_text("en", "prose", &[])
            .unwrap()
            .map(|c| c.text),
        Some("zebra".into())
    );
}

#[test]
fn record_usage_upserts_per_provider_and_day() {
    let store = Store::open_in_memory().unwrap();
    assert_eq!(store.usage_totals("2026-11-16").unwrap(), vec![]);
    store.record_usage("gemini", "2026-11-15", 100, 40).unwrap();
    store.record_usage("gemini", "2026-11-16", 10, 4).unwrap();
    store.record_usage("gemini", "2026-11-16", 20, 6).unwrap();
    store.record_usage("claude", "2026-11-15", 7, 3).unwrap();
    assert_eq!(
        store.usage_totals("2026-11-16").unwrap(),
        vec![
            UsageRow {
                provider: "claude".into(),
                calls_today: 0,
                calls_total: 1,
                input_tokens_today: 0,
                output_tokens_today: 0,
                input_tokens_total: 7,
                output_tokens_total: 3,
            },
            UsageRow {
                provider: "gemini".into(),
                calls_today: 2,
                calls_total: 3,
                input_tokens_today: 30,
                output_tokens_today: 10,
                input_tokens_total: 130,
                output_tokens_total: 50,
            },
        ]
    );
    let json = serde_json::to_value(&store.usage_totals("2026-11-16").unwrap()[0]).unwrap();
    assert_eq!(json["callsTotal"], 1);
    assert_eq!(json["inputTokensTotal"], 7);
}

#[test]
fn record_usage_rejects_counts_that_do_not_fit() {
    let store = Store::open_in_memory().unwrap();
    assert!(matches!(
        store.record_usage("claude", "2026-11-16", u64::MAX, 0),
        Err(StoreError::InvalidData(_))
    ));
    assert_eq!(store.usage_totals("2026-11-16").unwrap(), vec![]);
}

/// The text cached for `language` (prose), marking it used.
fn take_in(store: &Store, language: &str) -> Option<String> {
    store
        .take_cached_text(language, "prose", &[])
        .unwrap()
        .map(|c| c.text)
}

/// `n` unused Spanish fillers.
fn fill(store: &Store, n: usize) {
    for i in 0..n {
        cache(store, "es", &["z"], &format!("filler {i}"), false);
    }
}

#[test]
fn the_cache_keeps_every_text_up_to_the_limit() {
    let store = Store::open_in_memory().unwrap();
    cache(&store, "en", &["z"], "oldest", false);
    fill(&store, TEXT_CACHE_LIMIT - 1);
    assert_eq!(take_in(&store, "en").as_deref(), Some("oldest"));
}

#[test]
fn over_the_limit_the_least_recently_used_used_text_goes_first() {
    assert_eq!(TEXT_CACHE_LIMIT, 500);
    let store = Store::open_in_memory().unwrap();
    cache(&store, "en", &["z"], "used first", true);
    cache(&store, "fr", &["z"], "used second", true);
    fill(&store, TEXT_CACHE_LIMIT - 1);
    assert_eq!(take_in(&store, "en"), None);
    assert_eq!(take_in(&store, "fr").as_deref(), Some("used second"));
}

#[test]
fn unused_texts_outlive_a_newer_used_one() {
    let store = Store::open_in_memory().unwrap();
    cache(&store, "de", &["z"], "oldest unused", false);
    fill(&store, TEXT_CACHE_LIMIT - 1);
    cache(&store, "en", &["z"], "newest used", true);
    assert_eq!(take_in(&store, "en"), None);
    assert_eq!(take_in(&store, "de").as_deref(), Some("oldest unused"));
}

#[test]
fn when_every_text_is_unused_the_oldest_goes() {
    let store = Store::open_in_memory().unwrap();
    cache(&store, "en", &["z"], "first", false);
    cache(&store, "fr", &["z"], "second", false);
    fill(&store, TEXT_CACHE_LIMIT - 1);
    assert_eq!(take_in(&store, "en"), None);
    assert_eq!(take_in(&store, "fr").as_deref(), Some("second"));
}
