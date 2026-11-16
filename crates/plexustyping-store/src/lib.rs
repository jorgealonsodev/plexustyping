//! Local SQLite persistence for PlexusTyping: profiles, sessions, keystrokes,
//! adaptive statistics rows and settings.
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
const MIGRATIONS: &[&str] = &[r#"
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
"#];

/// Schema version this build creates and understands.
const SUPPORTED_VERSION: i64 = MIGRATIONS.len() as i64;

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

impl Store {
    /// Opens (or creates) the database at `path`, creating its parent
    /// directory, enabling WAL and foreign keys, and migrating the schema.
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent)?;
        }
        Self::init(Connection::open(path)?, true)
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
        if wal {
            conn.query_row("PRAGMA journal_mode = WAL", [], |_| Ok(()))?;
        }
        for (i, sql) in MIGRATIONS.iter().enumerate().skip(found as usize) {
            let tx = conn.transaction()?;
            tx.execute_batch(sql)?;
            tx.pragma_update(None, "user_version", i as i64 + 1)?;
            tx.commit()?;
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
        tx.execute(
            "INSERT INTO sessions (profile_id, started_at_ms, mode, code_language,
                 error_mode, text, duration_ms, gross_wpm, net_wpm, accuracy,
                 keystrokes, bpm)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                s.profile_id,
                s.started_at_ms,
                s.mode,
                s.code_language,
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
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, NULL)",
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
                ])?;
            }
        }
        tx.commit()?;
        Ok(id)
    }

    /// Up to `limit` sessions of `profile_id`, newest first.
    pub fn recent_sessions(&self, profile_id: i64, limit: usize) -> Result<Vec<SessionSummary>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, started_at_ms, mode, duration_ms, gross_wpm, net_wpm,
                 accuracy, keystrokes
             FROM sessions WHERE profile_id = ?1
             ORDER BY started_at_ms DESC, id DESC LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![profile_id, to_i64(limit)?], |r| {
            Ok((
                SessionSummary {
                    id: r.get(0)?,
                    started_at_ms: r.get(1)?,
                    mode: r.get(2)?,
                    duration_ms: r.get(3)?,
                    gross_wpm: r.get(4)?,
                    net_wpm: r.get(5)?,
                    accuracy: r.get(6)?,
                    keystrokes: 0,
                },
                r.get::<_, i64>(7)?,
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

    /// Statistics of `profile_id` in `mode`; empty when nothing is stored.
    pub fn load_stats(&self, profile_id: i64, mode: &str) -> Result<StatsSnapshot> {
        let mut stmt = self.conn.prepare(
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
        let mut stmt = self.conn.prepare(
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
        let mean_interval_ms = self
            .conn
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

    /// Atomically replaces all statistics of `profile_id` in `mode` with `s`.
    /// On failure (e.g. a duplicate unit) the previous snapshot is kept.
    pub fn save_stats(&mut self, profile_id: i64, mode: &str, s: &StatsSnapshot) -> Result<()> {
        let tx = self.conn.transaction()?;
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
