//! Local SQLite persistence for PlexusTyping: profiles, sessions, keystrokes,
//! adaptive statistics rows, settings, the cache of AI-generated texts and the
//! AI usage counters.
//!
//! Data stays on the user's machine. This store never holds secrets: API keys
//! and other credentials belong in the OS keychain, never in this database.
//!
//! The schema is versioned with `PRAGMA user_version` and upgraded by an
//! ordered list of migrations, each applied in its own transaction.

use std::path::Path;

use plexustyping_core::{ErrorMode, KeystrokeRecord};
use rusqlite::{Connection, OptionalExtension, params};
use serde::Serialize;

/// Ordered schema migrations; migration `i` upgrades `user_version` `i` → `i + 1`.
/// Released migrations are never edited. v2 adds `text_cache` (`targets` is a
/// JSON array; `last_used` is a use sequence, higher = more recent) and
/// `ai_usage` (`day` is the user's local `YYYY-MM-DD`).
const MIGRATIONS: &[&str] = &[
    r#"
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
"#,
    r#"
CREATE TABLE text_cache (
    id INTEGER PRIMARY KEY,
    provider TEXT NOT NULL,
    model TEXT NOT NULL,
    language TEXT NOT NULL,
    mode TEXT NOT NULL,
    targets TEXT NOT NULL,
    text TEXT NOT NULL,
    created_at_ms INTEGER NOT NULL,
    used INTEGER NOT NULL DEFAULT 0,
    last_used INTEGER
);
CREATE INDEX text_cache_by_language_mode_used ON text_cache(language, mode, used);
CREATE TABLE ai_usage (
    provider TEXT NOT NULL,
    day TEXT NOT NULL,
    calls INTEGER NOT NULL,
    input_tokens INTEGER NOT NULL,
    output_tokens INTEGER NOT NULL,
    PRIMARY KEY (provider, day)
);
"#,
];

/// Schema version this build creates and understands.
const SUPPORTED_VERSION: i64 = MIGRATIONS.len() as i64;

/// How long a file connection waits for another connection's lock.
pub const BUSY_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(2);

/// Handle to the PlexusTyping database.
#[derive(Debug)]
pub struct Store {
    conn: Connection,
}

/// Errors returned by [`Store`].
#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    /// Underlying SQLite failure.
    #[error("database error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    /// The database directory could not be created.
    #[error("cannot create database directory: {0}")]
    Io(#[from] std::io::Error),
    /// The requested row does not exist.
    #[error("not found")]
    NotFound,
    /// The database was written by a newer build; it is left untouched.
    #[error("database schema version {found} is newer than supported version {supported}")]
    NewerSchema {
        /// Version found in the file.
        found: i64,
        /// Highest version this build supports.
        supported: i64,
    },
    /// A stored or supplied value is out of range or malformed.
    #[error("invalid data: {0}")]
    InvalidData(String),
}

/// Result alias for store operations.
pub type Result<T> = std::result::Result<T, StoreError>;

/// A user profile.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    /// Row id.
    pub id: i64,
    /// Display name.
    pub name: String,
    /// Language code, e.g. `es`.
    pub language: String,
    /// Keyboard layout id, e.g. `es-qwerty`.
    pub layout: String,
    /// Theme id, if the user picked one.
    pub theme: Option<String>,
    /// Creation time, Unix epoch milliseconds.
    pub created_at_ms: i64,
}

/// A finished session to persist together with its keystroke records.
#[derive(Clone, Debug, PartialEq)]
pub struct NewSession<'a> {
    /// Owning profile.
    pub profile_id: i64,
    /// Start time, Unix epoch milliseconds.
    pub started_at_ms: i64,
    /// Practice mode, e.g. `"prose"`.
    pub mode: &'a str,
    /// Programming language in code mode.
    pub code_language: Option<&'a str>,
    /// Natural language of the text, e.g. `es`.
    pub language: Option<&'a str>,
    /// Keyboard layout id used, e.g. `es-qwerty`.
    pub layout: Option<&'a str>,
    /// Error mode used.
    pub error_mode: ErrorMode,
    /// Text that was typed.
    pub text: &'a str,
    /// Session duration in milliseconds.
    pub duration_ms: f64,
    /// Gross words per minute.
    pub gross_wpm: f64,
    /// Net words per minute.
    pub net_wpm: f64,
    /// Accuracy in `0.0..=1.0`.
    pub accuracy: f64,
    /// Number of applied keystrokes.
    pub keystrokes: usize,
    /// Metronome tempo, if one was used.
    pub bpm: Option<u32>,
    /// Keystroke records, in typing order.
    pub records: &'a [KeystrokeRecord],
    /// Metronome offset of each record (ms after the nearest beat, negative = early);
    /// empty when no metronome ran, else one entry per record.
    pub metronome_offsets_ms: &'a [Option<f64>],
}

/// Summary of a stored session.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionSummary {
    /// Row id.
    pub id: i64,
    /// Start time, Unix epoch milliseconds.
    pub started_at_ms: i64,
    /// Practice mode.
    pub mode: String,
    /// Natural language of the text, if recorded.
    pub language: Option<String>,
    /// Keyboard layout id, if recorded.
    pub layout: Option<String>,
    /// Duration in milliseconds.
    pub duration_ms: f64,
    /// Gross words per minute.
    pub gross_wpm: f64,
    /// Net words per minute.
    pub net_wpm: f64,
    /// Accuracy in `0.0..=1.0`.
    pub accuracy: f64,
    /// Number of applied keystrokes.
    pub keystrokes: usize,
    /// Metronome tempo, if one was used for the whole text.
    pub bpm: Option<u32>,
}

/// Stored statistics for one character or bigram.
#[derive(Clone, Debug, PartialEq)]
pub struct UnitStatRow {
    /// The character or bigram.
    pub unit: String,
    /// Exponential moving average of errors.
    pub ema_error: f64,
    /// Exponential moving average of slowness.
    pub ema_slowness: f64,
    /// Error samples seen.
    pub samples: u32,
    /// Slowness samples seen.
    pub slowness_samples: u32,
}

/// How many times `typed` was pressed instead of `expected`.
#[derive(Clone, Debug, PartialEq)]
pub struct ConfusionRow {
    /// Expected character.
    pub expected: char,
    /// Character typed instead.
    pub typed: char,
    /// Occurrences.
    pub count: u32,
}

/// All adaptive statistics of one profile and mode.
#[derive(Clone, Debug, PartialEq)]
pub struct StatsSnapshot {
    /// Per-unit rows, sorted by unit.
    pub units: Vec<UnitStatRow>,
    /// Confusion pairs, sorted by expected then typed.
    pub confusions: Vec<ConfusionRow>,
    /// The user's mean keystroke interval, if known.
    pub mean_interval_ms: Option<f64>,
}

/// An AI-generated text to keep for offline reuse.
#[derive(Clone, Debug, PartialEq)]
pub struct NewCachedText<'a> {
    /// Provider id, e.g. `"claude"`.
    pub provider: &'a str,
    /// Model id.
    pub model: &'a str,
    /// Language code, e.g. `es`.
    pub language: &'a str,
    /// Practice mode, e.g. `"prose"`.
    pub mode: &'a str,
    /// Units the text practises.
    pub targets: &'a [String],
    /// The text.
    pub text: &'a str,
    /// Generation time, Unix epoch milliseconds.
    pub created_at_ms: i64,
    /// Already shown to the user.
    pub used: bool,
}

/// A cached AI-generated text.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CachedText {
    /// Row id.
    pub id: i64,
    /// Provider id.
    pub provider: String,
    /// Model id.
    pub model: String,
    /// Language code.
    pub language: String,
    /// Practice mode.
    pub mode: String,
    /// Units the text practises.
    pub targets: Vec<String>,
    /// The text.
    pub text: String,
    /// Generation time, Unix epoch milliseconds.
    pub created_at_ms: i64,
}

/// AI calls and tokens of one provider, today and in total.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageRow {
    /// Provider id.
    pub provider: String,
    /// Successful calls today.
    pub calls_today: u64,
    /// Successful calls ever.
    pub calls_total: u64,
    /// Prompt tokens today.
    pub input_tokens_today: u64,
    /// Completion tokens today.
    pub output_tokens_today: u64,
    /// Prompt tokens ever.
    pub input_tokens_total: u64,
    /// Completion tokens ever.
    pub output_tokens_total: u64,
}

impl Store {
    /// Opens (or creates) the database at `path`, creating its parent
    /// directory, migrating the schema, then enabling WAL. Writers wait up to
    /// [`BUSY_TIMEOUT`] for a lock. A file that cannot be migrated keeps its
    /// journal mode.
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(path)?;
        conn.busy_timeout(BUSY_TIMEOUT)?;
        Self::init(conn, true)
    }

    /// Opens a fresh in-memory database with the current schema.
    pub fn open_in_memory() -> Result<Self> {
        Self::init(Connection::open_in_memory()?, false)
    }

    fn init(mut conn: Connection, wal: bool) -> Result<Self> {
        conn.pragma_update(None, "foreign_keys", true)?;
        // Check the version before anything that could alter the file.
        let found = user_version(&conn)?;
        if found > SUPPORTED_VERSION {
            return Err(StoreError::NewerSchema {
                found,
                supported: SUPPORTED_VERSION,
            });
        }
        if found < 0 {
            return Err(StoreError::InvalidData(format!(
                "negative schema version {found}"
            )));
        }
        for (i, sql) in MIGRATIONS.iter().enumerate().skip(found as usize) {
            let tx = conn.transaction()?;
            tx.execute_batch(sql)?;
            tx.pragma_update(None, "user_version", i as i64 + 1)?;
            tx.commit()?;
        }
        // Only a database we fully own switches to WAL (it rewrites the header).
        if wal {
            conn.query_row("PRAGMA journal_mode = WAL", [], |_| Ok(()))?;
        }
        Ok(Store { conn })
    }

    /// Current schema version (`PRAGMA user_version`).
    pub fn schema_version(&self) -> Result<i64> {
        user_version(&self.conn)
    }

    /// Returns the first profile, creating "Principal" (`es`, `es-qwerty`)
    /// when none exists.
    pub fn default_profile(&self) -> Result<Profile> {
        if let Some(p) = self.first_profile()? {
            return Ok(p);
        }
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);
        self.conn.execute(
            "INSERT INTO profiles (name, language, layout, theme, created_at_ms)
             VALUES ('Principal', 'es', 'es-qwerty', NULL, ?1)",
            [now],
        )?;
        self.first_profile()?.ok_or(StoreError::NotFound)
    }

    fn first_profile(&self) -> Result<Option<Profile>> {
        Ok(self
            .conn
            .query_row(
                "SELECT id, name, language, layout, theme, created_at_ms
                 FROM profiles ORDER BY id LIMIT 1",
                [],
                |r| {
                    Ok(Profile {
                        id: r.get(0)?,
                        name: r.get(1)?,
                        language: r.get(2)?,
                        layout: r.get(3)?,
                        theme: r.get(4)?,
                        created_at_ms: r.get(5)?,
                    })
                },
            )
            .optional()?)
    }

    /// Stores a session row and all its keystrokes in one transaction and
    /// returns the new session id.
    pub fn save_session(&mut self, s: &NewSession) -> Result<i64> {
        let tx = self.conn.transaction()?;
        let id = insert_session(&tx, s)?;
        tx.commit()?;
        Ok(id)
    }

    /// Stores a session with its keystrokes and replaces the statistics of
    /// its profile in `mode` with `update(previous statistics)`, all in one
    /// transaction: on any failure nothing is stored and the previous
    /// statistics are kept. Returns the new session id.
    pub fn save_session_with_stats(
        &mut self,
        s: &NewSession,
        mode: &str,
        update: impl FnOnce(StatsSnapshot) -> StatsSnapshot,
    ) -> Result<i64> {
        let tx = self.conn.transaction()?;
        let id = insert_session(&tx, s)?;
        let next = update(load_stats(&tx, s.profile_id, mode)?);
        replace_stats(&tx, s.profile_id, mode, &next)?;
        tx.commit()?;
        Ok(id)
    }

    /// Up to `limit` sessions of `profile_id`, newest first.
    pub fn recent_sessions(&self, profile_id: i64, limit: usize) -> Result<Vec<SessionSummary>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, started_at_ms, mode, language, layout, duration_ms,
                 gross_wpm, net_wpm, accuracy, keystrokes, bpm
             FROM sessions WHERE profile_id = ?1
             ORDER BY started_at_ms DESC, id DESC LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![profile_id, to_i64(limit)?], |r| {
            Ok((
                SessionSummary {
                    id: r.get(0)?,
                    started_at_ms: r.get(1)?,
                    mode: r.get(2)?,
                    language: r.get(3)?,
                    layout: r.get(4)?,
                    duration_ms: r.get(5)?,
                    gross_wpm: r.get(6)?,
                    net_wpm: r.get(7)?,
                    accuracy: r.get(8)?,
                    keystrokes: 0,
                    bpm: r.get(10)?,
                },
                r.get::<_, i64>(9)?,
            ))
        })?;
        rows.map(|row| {
            let (mut summary, keystrokes) = row?;
            summary.keystrokes = to_usize(keystrokes)?;
            Ok(summary)
        })
        .collect()
    }

    /// Keystroke records of a session in typing order;
    /// [`StoreError::NotFound`] if the session does not exist.
    pub fn session_keystrokes(&self, session_id: i64) -> Result<Vec<KeystrokeRecord>> {
        let exists: Option<i64> = self
            .conn
            .query_row("SELECT 1 FROM sessions WHERE id = ?1", [session_id], |r| {
                r.get(0)
            })
            .optional()?;
        if exists.is_none() {
            return Err(StoreError::NotFound);
        }
        let mut stmt = self.conn.prepare(
            "SELECT position, expected, typed, correct, ms_since_previous
             FROM keystrokes WHERE session_id = ?1 ORDER BY idx",
        )?;
        let rows = stmt.query_map([session_id], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, bool>(3)?,
                r.get::<_, Option<f64>>(4)?,
            ))
        })?;
        rows.map(|row| {
            let (position, expected, typed, correct, ms) = row?;
            Ok(KeystrokeRecord {
                index: to_usize(position)?,
                expected: one_char(&expected)?,
                typed: one_char(&typed)?,
                correct,
                ms_since_previous: ms,
            })
        })
        .collect()
    }

    /// Metronome offset of each keystroke of a session in typing order (`None` where
    /// none was measured); [`StoreError::NotFound`] if the session does not exist.
    pub fn session_metronome_offsets(&self, session_id: i64) -> Result<Vec<Option<f64>>> {
        let exists: Option<i64> = self
            .conn
            .query_row("SELECT 1 FROM sessions WHERE id = ?1", [session_id], |r| {
                r.get(0)
            })
            .optional()?;
        if exists.is_none() {
            return Err(StoreError::NotFound);
        }
        let mut stmt = self.conn.prepare(
            "SELECT metronome_offset_ms FROM keystrokes WHERE session_id = ?1 ORDER BY idx",
        )?;
        let rows = stmt.query_map([session_id], |r| r.get(0))?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    /// Statistics of `profile_id` in `mode`; empty when nothing is stored.
    pub fn load_stats(&self, profile_id: i64, mode: &str) -> Result<StatsSnapshot> {
        load_stats(&self.conn, profile_id, mode)
    }

    /// Atomically replaces all statistics of `profile_id` in `mode` with `s`.
    /// On failure (e.g. a duplicate unit) the previous snapshot is kept.
    pub fn save_stats(&mut self, profile_id: i64, mode: &str, s: &StatsSnapshot) -> Result<()> {
        let tx = self.conn.transaction()?;
        replace_stats(&tx, profile_id, mode, s)?;
        tx.commit()?;
        Ok(())
    }

    /// Value of setting `key`, if set.
    pub fn get_setting(&self, key: &str) -> Result<Option<String>> {
        Ok(self
            .conn
            .query_row("SELECT value FROM settings WHERE key = ?1", [key], |r| {
                r.get(0)
            })
            .optional()?)
    }

    /// Inserts or overwrites setting `key`.
    pub fn set_setting(&self, key: &str, value: &str) -> Result<()> {
        self.conn.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            [key, value],
        )?;
        Ok(())
    }
}

impl Store {
    /// Stores a generated text and returns its id; a `used` text gets the
    /// most recent use.
    pub fn cache_text(&self, t: &NewCachedText) -> Result<i64> {
        let targets =
            serde_json::to_string(t.targets).map_err(|e| StoreError::InvalidData(e.to_string()))?;
        self.conn.execute(
            "INSERT INTO text_cache (provider, model, language, mode, targets, text,
                 created_at_ms, used, last_used)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8,
                 CASE WHEN ?8 THEN (SELECT COALESCE(MAX(last_used), 0) + 1 FROM text_cache) END)",
            params![
                t.provider,
                t.model,
                t.language,
                t.mode,
                targets,
                t.text,
                t.created_at_ms,
                t.used
            ],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    /// A cached text of `language` and `mode`, marked used: the unused text
    /// sharing the most `targets` (oldest first on ties, any unused one when
    /// none shares), else the least recently used one; `None` when empty.
    pub fn take_cached_text(
        &self,
        language: &str,
        mode: &str,
        targets: &[String],
    ) -> Result<Option<CachedText>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, targets FROM text_cache
             WHERE language = ?1 AND mode = ?2 AND used = 0 ORDER BY id",
        )?;
        let unused = stmt
            .query_map(params![language, mode], |r| {
                Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        let mut best: Option<(i64, usize)> = None;
        for (id, json) in unused {
            let shared = parse_targets(&json)?
                .iter()
                .filter(|t| targets.contains(t))
                .count();
            if best.is_none_or(|(_, n)| shared > n) {
                best = Some((id, shared));
            }
        }
        let id = match best {
            Some((id, _)) => Some(id),
            None => self
                .conn
                .query_row(
                    "SELECT id FROM text_cache WHERE language = ?1 AND mode = ?2
                     ORDER BY last_used, id LIMIT 1",
                    params![language, mode],
                    |r| r.get::<_, i64>(0),
                )
                .optional()?,
        };
        let Some(id) = id else { return Ok(None) };
        self.conn.execute(
            "UPDATE text_cache SET used = 1,
                 last_used = (SELECT COALESCE(MAX(last_used), 0) + 1 FROM text_cache)
             WHERE id = ?1",
            [id],
        )?;
        let (mut cached, json) = self.conn.query_row(
            "SELECT id, provider, model, language, mode, targets, text, created_at_ms
             FROM text_cache WHERE id = ?1",
            [id],
            |r| {
                let cached = CachedText {
                    id: r.get(0)?,
                    provider: r.get(1)?,
                    model: r.get(2)?,
                    language: r.get(3)?,
                    mode: r.get(4)?,
                    targets: Vec::new(),
                    text: r.get(6)?,
                    created_at_ms: r.get(7)?,
                };
                Ok((cached, r.get::<_, String>(5)?))
            },
        )?;
        cached.targets = parse_targets(&json)?;
        Ok(Some(cached))
    }

    /// Counts one successful call of `provider` on local `day` with its tokens.
    pub fn record_usage(&self, provider: &str, day: &str, input: u64, output: u64) -> Result<()> {
        let input = u64_to_i64(input)?;
        let output = u64_to_i64(output)?;
        self.conn.execute(
            "INSERT INTO ai_usage (provider, day, calls, input_tokens, output_tokens)
             VALUES (?1, ?2, 1, ?3, ?4)
             ON CONFLICT(provider, day) DO UPDATE SET
                 calls = calls + 1,
                 input_tokens = input_tokens + excluded.input_tokens,
                 output_tokens = output_tokens + excluded.output_tokens",
            params![provider, day, input, output],
        )?;
        Ok(())
    }

    /// Per-provider calls and tokens on `today` and in total, sorted by provider.
    pub fn usage_totals(&self, today: &str) -> Result<Vec<UsageRow>> {
        let mut stmt = self.conn.prepare(
            "SELECT provider,
                 SUM(CASE WHEN day = ?1 THEN calls ELSE 0 END), SUM(calls),
                 SUM(CASE WHEN day = ?1 THEN input_tokens ELSE 0 END),
                 SUM(CASE WHEN day = ?1 THEN output_tokens ELSE 0 END),
                 SUM(input_tokens), SUM(output_tokens)
             FROM ai_usage GROUP BY provider ORDER BY provider",
        )?;
        let rows = stmt.query_map([today], |r| {
            let mut n = [0i64; 6];
            for (i, slot) in n.iter_mut().enumerate() {
                *slot = r.get(i + 1)?;
            }
            Ok((r.get::<_, String>(0)?, n))
        })?;
        rows.map(|row| {
            let (provider, n) = row?;
            let [
                calls_today,
                calls_total,
                input_today,
                output_today,
                input_total,
                output_total,
            ] = n.map(|v| u64::try_from(v).unwrap_or(0));
            Ok(UsageRow {
                provider,
                calls_today,
                calls_total,
                input_tokens_today: input_today,
                output_tokens_today: output_today,
                input_tokens_total: input_total,
                output_tokens_total: output_total,
            })
        })
        .collect()
    }
}

fn parse_targets(json: &str) -> Result<Vec<String>> {
    serde_json::from_str(json).map_err(|e| StoreError::InvalidData(format!("cached targets: {e}")))
}

fn u64_to_i64(n: u64) -> Result<i64> {
    i64::try_from(n).map_err(|_| StoreError::InvalidData(format!("{n} does not fit in i64")))
}

/// Inserts a session row and its keystrokes; the caller owns the transaction.
fn insert_session(tx: &Connection, s: &NewSession) -> Result<i64> {
    let offsets = s.metronome_offsets_ms;
    if !offsets.is_empty() && offsets.len() != s.records.len() {
        return Err(StoreError::InvalidData(format!(
            "{} metronome offsets for {} keystrokes",
            offsets.len(),
            s.records.len()
        )));
    }
    tx.execute(
        "INSERT INTO sessions (profile_id, started_at_ms, mode, code_language,
             language, layout, error_mode, text, duration_ms, gross_wpm,
             net_wpm, accuracy, keystrokes, bpm)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
        params![
            s.profile_id,
            s.started_at_ms,
            s.mode,
            s.code_language,
            s.language,
            s.layout,
            error_mode_str(s.error_mode),
            s.text,
            s.duration_ms,
            s.gross_wpm,
            s.net_wpm,
            s.accuracy,
            to_i64(s.keystrokes)?,
            s.bpm,
        ],
    )?;
    let id = tx.last_insert_rowid();
    {
        let mut stmt = tx.prepare(
            "INSERT INTO keystrokes (session_id, idx, position, expected, typed,
                 correct, ms_since_previous, metronome_offset_ms)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        )?;
        for (idx, r) in s.records.iter().enumerate() {
            stmt.execute(params![
                id,
                to_i64(idx)?,
                to_i64(r.index)?,
                r.expected.to_string(),
                r.typed.to_string(),
                r.correct,
                r.ms_since_previous,
                offsets.get(idx).copied().flatten(),
            ])?;
        }
    }
    Ok(id)
}

fn load_stats(conn: &Connection, profile_id: i64, mode: &str) -> Result<StatsSnapshot> {
    let mut stmt = conn.prepare(
        "SELECT unit, ema_error, ema_slowness, samples, slowness_samples
         FROM char_stats WHERE profile_id = ?1 AND mode = ?2 ORDER BY unit",
    )?;
    let units = stmt
        .query_map(params![profile_id, mode], |r| {
            Ok(UnitStatRow {
                unit: r.get(0)?,
                ema_error: r.get(1)?,
                ema_slowness: r.get(2)?,
                samples: r.get(3)?,
                slowness_samples: r.get(4)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let mut stmt = conn.prepare(
        "SELECT expected, typed, count FROM confusions
         WHERE profile_id = ?1 AND mode = ?2 ORDER BY expected, typed",
    )?;
    let confusions = stmt
        .query_map(params![profile_id, mode], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, u32>(2)?,
            ))
        })?
        .map(|row| {
            let (expected, typed, count) = row?;
            Ok(ConfusionRow {
                expected: one_char(&expected)?,
                typed: one_char(&typed)?,
                count,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let mean_interval_ms = conn
        .query_row(
            "SELECT mean_interval_ms FROM profile_stats
             WHERE profile_id = ?1 AND mode = ?2",
            params![profile_id, mode],
            |r| r.get::<_, Option<f64>>(0),
        )
        .optional()?
        .flatten();
    Ok(StatsSnapshot {
        units,
        confusions,
        mean_interval_ms,
    })
}

/// Replaces the statistics rows; the caller owns the transaction.
fn replace_stats(tx: &Connection, profile_id: i64, mode: &str, s: &StatsSnapshot) -> Result<()> {
    for table in ["char_stats", "confusions", "profile_stats"] {
        tx.execute(
            &format!("DELETE FROM {table} WHERE profile_id = ?1 AND mode = ?2"),
            params![profile_id, mode],
        )?;
    }
    {
        let mut stmt = tx.prepare(
            "INSERT INTO char_stats (profile_id, mode, unit, ema_error,
                 ema_slowness, samples, slowness_samples)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        )?;
        for u in &s.units {
            stmt.execute(params![
                profile_id,
                mode,
                u.unit,
                u.ema_error,
                u.ema_slowness,
                u.samples,
                u.slowness_samples,
            ])?;
        }
        let mut stmt = tx.prepare(
            "INSERT INTO confusions (profile_id, mode, expected, typed, count)
             VALUES (?1, ?2, ?3, ?4, ?5)",
        )?;
        for c in &s.confusions {
            stmt.execute(params![
                profile_id,
                mode,
                c.expected.to_string(),
                c.typed.to_string(),
                c.count,
            ])?;
        }
    }
    tx.execute(
        "INSERT INTO profile_stats (profile_id, mode, mean_interval_ms)
         VALUES (?1, ?2, ?3)",
        params![profile_id, mode, s.mean_interval_ms],
    )?;
    Ok(())
}

fn user_version(conn: &Connection) -> Result<i64> {
    Ok(conn.query_row("PRAGMA user_version", [], |r| r.get(0))?)
}

/// Stored form of an error mode (matches its serde kebab-case name).
fn error_mode_str(mode: ErrorMode) -> &'static str {
    match mode {
        ErrorMode::StopOnError => "stop-on-error",
        ErrorMode::AllowContinue => "allow-continue",
        ErrorMode::BackspaceCorrect => "backspace-correct",
    }
}

fn one_char(s: &str) -> Result<char> {
    let mut chars = s.chars();
    match (chars.next(), chars.next()) {
        (Some(c), None) => Ok(c),
        _ => Err(StoreError::InvalidData(format!(
            "expected a single character, found {s:?}"
        ))),
    }
}

fn to_i64(n: usize) -> Result<i64> {
    i64::try_from(n).map_err(|_| StoreError::InvalidData(format!("{n} does not fit in i64")))
}

fn to_usize(n: i64) -> Result<usize> {
    usize::try_from(n).map_err(|_| StoreError::InvalidData(format!("{n} is not a valid count")))
}
