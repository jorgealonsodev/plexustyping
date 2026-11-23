//! Session history and CSV export (phase 3, S6–S7), testable without a Tauri runtime.

use std::collections::BTreeMap;
use std::fs::{File, OpenOptions};
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};

use plexustyping_store::{Profile, SessionSummary, Store};
use serde::Serialize;

use crate::code::CODE_STATS_MODE;
use crate::commands::STATS_MODE;
use crate::i18n::Locale;
use crate::messages;

/// First line of every export (S7).
pub const CSV_HEADER: &str = "fecha,modo,lenguaje_codigo,idioma,layout,duracion_s,ppm_brutas,ppm_netas,precision,pulsaciones,bpm";
/// Error when the active filter has no sessions (S7).
pub const NO_SESSIONS: crate::i18n::Texts = messages::NO_SESSIONS;
/// Largest accepted UTC offset, in minutes (ISO 8601: ±18:00).
pub const MAX_TZ_OFFSET_MIN: i32 = 18 * 60;

/// Result of a CSV export.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CsvExport {
    /// Full path of the written file.
    pub path: String,
    /// Sessions written.
    pub rows: u32,
}

/// The stored mode of a history filter: `None` (all), `"prose"`, `"words"` or `"code"`.
pub fn parse_mode(locale: Locale, mode: Option<&str>) -> Result<Option<&'static str>, String> {
    match mode {
        None => Ok(None),
        Some("prose") => Ok(Some("prose")),
        Some("words") => Ok(Some("words")),
        Some("code") => Ok(Some("code")),
        Some(other) => Err(messages::UNKNOWN_MODE.fill(locale, &[("mode", other)])),
    }
}

/// Sessions of `profile` for the filter `mode`, newest first, at most `limit`;
/// empty in temporary mode (`persistent == false`).
pub fn session_history(
    locale: Locale,
    store: &Store,
    profile: &Profile,
    persistent: bool,
    mode: Option<&str>,
    limit: Option<u32>,
) -> Result<Vec<SessionSummary>, String> {
    let mode = parse_mode(locale, mode)?;
    if !persistent {
        return Ok(Vec::new());
    }
    let limit = limit.map(|n| n as usize);
    store
        .sessions(profile.id, mode, limit)
        .map_err(|e| e.to_string())
}

/// Days of [`history_by_day`] when not given.
pub const DEFAULT_DAYS: u32 = 30;
/// Most days [`history_by_day`] returns.
pub const MAX_DAYS: u32 = 366;
/// Fewest error samples for a character to be listed by [`history_by_char`].
pub const MIN_CHAR_SAMPLES: u32 = 5;

/// One local day of practice (S2, S5).
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DayRow {
    /// Local date, `YYYY-MM-DD`.
    pub day: String,
    /// Sessions started that day.
    pub sessions: u32,
    /// Practised minutes (summed durations, not rounded).
    pub minutes: f64,
    /// Mean net WPM, weighted by session duration.
    pub net_wpm: f64,
    /// Mean accuracy in `0.0..=1.0`, weighted by session duration.
    pub accuracy: f64,
}

/// Statistics of one character (S3, S5).
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CharRow {
    /// The character.
    pub unit: String,
    /// Error samples seen.
    pub samples: u32,
    /// EMA of errors, `0.0..=1.0`.
    pub error: f64,
    /// EMA of relative slowness, `0.0..=1.0`.
    pub slowness: f64,
}

/// Refuses an offset outside ±[`MAX_TZ_OFFSET_MIN`] with "Zona horaria no válida: N".
pub fn check_tz(locale: Locale, tz_offset_min: i32) -> Result<(), String> {
    if (-MAX_TZ_OFFSET_MIN..=MAX_TZ_OFFSET_MIN).contains(&tz_offset_min) {
        Ok(())
    } else {
        let offset = tz_offset_min.to_string();
        Err(messages::INVALID_TZ.fill(locale, &[("offset", &offset)]))
    }
}

/// Practice per local day of `profile` for the filter `mode`: the newest `days`
/// days that have sessions (default [`DEFAULT_DAYS`], at most [`MAX_DAYS`]),
/// newest first; `tz_offset_min` is minutes east of UTC. Net WPM and accuracy
/// are means weighted by session duration. Empty in temporary mode.
pub fn history_by_day(
    locale: Locale,
    store: &Store,
    profile: &Profile,
    persistent: bool,
    mode: Option<&str>,
    tz_offset_min: i32,
    days: Option<u32>,
) -> Result<Vec<DayRow>, String> {
    let mode = parse_mode(locale, mode)?;
    check_tz(locale, tz_offset_min)?;
    let days = days.unwrap_or(DEFAULT_DAYS).min(MAX_DAYS) as usize;
    let sessions = session_history(locale, store, profile, persistent, mode, None)?;
    // Sessions come newest first, so each day's sessions are contiguous.
    let mut groups: Vec<(i64, Vec<&SessionSummary>)> = Vec::new();
    for s in &sessions {
        let day = crate::goals::local_day(s.started_at_ms, tz_offset_min);
        if let Some((_, group)) = groups.last_mut().filter(|(last, _)| *last == day) {
            group.push(s);
        } else if groups.len() == days {
            break;
        } else {
            groups.push((day, vec![s]));
        }
    }
    Ok(groups
        .into_iter()
        .map(|(day, group)| day_row(day, &group))
        .collect())
}

/// The row of local day number `day` from its sessions.
fn day_row(day: i64, group: &[&SessionSummary]) -> DayRow {
    let durations: Vec<f64> = group.iter().map(|s| s.duration_ms.max(0.0)).collect();
    let total: f64 = durations.iter().sum();
    // Without any duration every session weighs the same.
    let weight = |i: usize| if total > 0.0 { durations[i] } else { 1.0 };
    let weights: f64 = (0..group.len()).map(weight).sum();
    let mean = |value: fn(&SessionSummary) -> f64| {
        group
            .iter()
            .enumerate()
            .map(|(i, s)| value(s) * weight(i))
            .sum::<f64>()
            / weights
    };
    let t = local_time(day * 86_400_000, 0);
    DayRow {
        day: format!("{:04}-{:02}-{:02}", t.year, t.month, t.day),
        sessions: group.len() as u32,
        minutes: total / 60_000.0,
        net_wpm: mean(|s| s.net_wpm),
        accuracy: mean(|s| s.accuracy),
    }
}

/// Characters of `profile` with at least [`MIN_CHAR_SAMPLES`] error samples
/// in the statistics of the filter `mode` (`None` merges prose and code,
/// words share the prose statistics), worst first: by error, then slowness,
/// then character. Empty in temporary mode.
pub fn history_by_char(
    locale: Locale,
    store: &Store,
    profile: &Profile,
    persistent: bool,
    mode: Option<&str>,
) -> Result<Vec<CharRow>, String> {
    let modes: &[&str] = match parse_mode(locale, mode)? {
        None => &[STATS_MODE, CODE_STATS_MODE],
        Some("code") => &[CODE_STATS_MODE],
        Some(_) => &[STATS_MODE],
    };
    if !persistent {
        return Ok(Vec::new());
    }
    // Per character: error samples, error × samples, slowness samples, slowness × samples.
    let mut merged: BTreeMap<String, (u32, f64, u32, f64)> = BTreeMap::new();
    for stats_mode in modes {
        let snapshot = store
            .load_stats(profile.id, stats_mode)
            .map_err(|e| e.to_string())?;
        for u in snapshot
            .units
            .into_iter()
            .filter(|u| u.unit.chars().count() == 1)
        {
            let entry = merged.entry(u.unit).or_default();
            entry.0 += u.samples;
            entry.1 += u.ema_error * f64::from(u.samples);
            entry.2 += u.slowness_samples;
            entry.3 += u.ema_slowness * f64::from(u.slowness_samples);
        }
    }
    let ratio = |sum: f64, n: u32| if n > 0 { sum / f64::from(n) } else { 0.0 };
    let mut rows: Vec<CharRow> = merged
        .into_iter()
        .filter(|(_, (samples, ..))| *samples >= MIN_CHAR_SAMPLES)
        .map(|(unit, (samples, error, slow_n, slowness))| CharRow {
            unit,
            samples,
            error: ratio(error, samples),
            slowness: ratio(slowness, slow_n),
        })
        .collect();
    // Stable: equal error and slowness keep the character order of the map.
    rows.sort_by(|a, b| {
        b.error
            .total_cmp(&a.error)
            .then(b.slowness.total_cmp(&a.slowness))
    });
    Ok(rows)
}

/// Civil date and time of a local instant.
struct LocalTime {
    year: i64,
    month: u32,
    day: u32,
    secs_of_day: i64,
}

/// `ms` (Unix epoch) shifted by `tz_offset_min`, as a civil date and time.
fn local_time(ms: i64, tz_offset_min: i32) -> LocalTime {
    let secs = ms.div_euclid(1000) + i64::from(tz_offset_min) * 60;
    let (days, secs_of_day) = (secs.div_euclid(86_400), secs.rem_euclid(86_400));
    // Days since 1970-01-01 to a proleptic Gregorian date (H. Hinnant, `civil_from_days`).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let year = yoe + era * 400 + i64::from(month <= 2);
    LocalTime {
        year,
        month,
        day,
        secs_of_day,
    }
}

/// `ms` (Unix epoch) as ISO 8601 local time with offset, e.g. `2026-11-17T09:30:05+01:00`.
pub fn local_iso(ms: i64, tz_offset_min: i32) -> String {
    let t = local_time(ms, tz_offset_min);
    let sign = if tz_offset_min < 0 { '-' } else { '+' };
    let offset = tz_offset_min.unsigned_abs();
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}{sign}{:02}:{:02}",
        t.year,
        t.month,
        t.day,
        t.secs_of_day / 3600,
        t.secs_of_day / 60 % 60,
        t.secs_of_day % 60,
        offset / 60,
        offset % 60
    )
}

/// Local date of `ms` as `AAAAMMDD`.
pub fn local_date(ms: i64, tz_offset_min: i32) -> String {
    let t = local_time(ms, tz_offset_min);
    format!("{:04}{:02}{:02}", t.year, t.month, t.day)
}

/// One RFC 4180 field: quoted when it holds `,`, `"`, CR or LF.
pub fn csv_field(value: &str) -> String {
    if value.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_owned()
    }
}

/// The CSV `modo` of a stored mode: `palabras` for words, others as stored.
fn csv_mode(mode: &str) -> &str {
    match mode {
        "words" => "palabras",
        other => other,
    }
}

/// One CSV row of `s` (without the line end).
fn csv_row(s: &SessionSummary, tz_offset_min: i32) -> String {
    let text = |v: &Option<String>| csv_field(v.as_deref().unwrap_or(""));
    let fields = [
        local_iso(s.started_at_ms, tz_offset_min),
        csv_field(csv_mode(&s.mode)),
        text(&s.code_language),
        text(&s.language),
        text(&s.layout),
        format!("{:.1}", s.duration_ms / 1000.0),
        format!("{:.1}", s.gross_wpm),
        format!("{:.1}", s.net_wpm),
        format!("{:.1}", s.accuracy * 100.0),
        s.keystrokes.to_string(),
        s.bpm.map(|b| b.to_string()).unwrap_or_default(),
    ];
    fields.join(",")
}

/// The whole CSV (header and one row per session, in the given order).
pub fn sessions_csv(sessions: &[SessionSummary], tz_offset_min: i32) -> String {
    let mut csv = format!("{CSV_HEADER}\n");
    for s in sessions {
        csv.push_str(&csv_row(s, tz_offset_min));
        csv.push('\n');
    }
    csv
}

/// Writes `contents` to `dir/plexustyping-sesiones-<date>.csv`, or `-2`, `-3`…
/// when taken; never overwrites. Returns the path written.
pub fn write_unique(
    locale: Locale,
    dir: &Path,
    date: &str,
    contents: &str,
) -> Result<PathBuf, String> {
    write_unique_with(locale, dir, date, contents, |file, bytes| {
        file.write_all(bytes)?;
        file.flush()
    })
}

/// [`write_unique`] with the write step injected, so tests can make it fail.
fn write_unique_with(
    locale: Locale,
    dir: &Path,
    date: &str,
    contents: &str,
    write: impl Fn(&mut File, &[u8]) -> std::io::Result<()>,
) -> Result<PathBuf, String> {
    let fail = |e: std::io::Error| messages::CSV_FAILED.fill(locale, &[("error", &e.to_string())]);
    for n in 1u32.. {
        let name = match n {
            1 => format!("plexustyping-sesiones-{date}.csv"),
            n => format!("plexustyping-sesiones-{date}-{n}.csv"),
        };
        let path = dir.join(name);
        // `create_new` fails instead of replacing a file that appeared meanwhile.
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(mut file) => {
                if let Err(e) = write(&mut file, contents.as_bytes()) {
                    // Only this call created `path`: never leave a truncated export behind.
                    drop(file);
                    let _ = std::fs::remove_file(&path);
                    return Err(fail(e));
                }
                return Ok(path);
            }
            Err(e) if e.kind() == ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(fail(e)),
        }
    }
    Err(locale.pick(messages::CSV_NO_FREE_NAME).to_owned())
}

/// Exports every session of the filter `mode` to the directory given by `dir`
/// (resolved only when there is something to write), naming the file after
/// the local date of `now_ms`.
#[allow(
    clippy::too_many_arguments,
    reason = "the command's arguments plus the interface language"
)]
pub fn export_sessions_csv(
    locale: Locale,
    store: &Store,
    profile: &Profile,
    persistent: bool,
    mode: Option<&str>,
    tz_offset_min: i32,
    now_ms: i64,
    dir: impl FnOnce() -> Result<PathBuf, String>,
) -> Result<CsvExport, String> {
    parse_mode(locale, mode)?;
    check_tz(locale, tz_offset_min)?;
    let sessions = session_history(locale, store, profile, persistent, mode, None)?;
    if sessions.is_empty() {
        return Err(locale.pick(NO_SESSIONS).to_owned());
    }
    let rows = u32::try_from(sessions.len()).map_err(|e| e.to_string())?;
    let dir = dir().map_err(|e| messages::CSV_FAILED.fill(locale, &[("error", &e)]))?;
    let csv = sessions_csv(&sessions, tz_offset_min);
    let path = write_unique(locale, &dir, &local_date(now_ms, tz_offset_min), &csv)?;
    Ok(CsvExport {
        path: path.display().to_string(),
        rows,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use plexustyping_core::{ErrorMode, KeystrokeRecord};
    use plexustyping_store::NewSession;

    /// Fresh directory under the system temp dir, removed on drop.
    struct TempDir(PathBuf);

    impl TempDir {
        fn new() -> Self {
            use std::sync::atomic::{AtomicU32, Ordering};
            static COUNTER: AtomicU32 = AtomicU32::new(0);
            let dir = std::env::temp_dir().join(format!(
                "plexustyping-history-test-{}-{}-{}",
                std::process::id(),
                std::time::UNIX_EPOCH.elapsed().unwrap().as_nanos(),
                COUNTER.fetch_add(1, Ordering::Relaxed)
            ));
            std::fs::create_dir_all(&dir).unwrap();
            TempDir(dir)
        }

        fn files(&self) -> Vec<String> {
            let mut names: Vec<String> = std::fs::read_dir(&self.0)
                .unwrap()
                .map(|e| e.unwrap().file_name().into_string().unwrap())
                .collect();
            names.sort();
            names
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    /// 2026-11-17T08:30:05Z.
    const T0_MS: i64 = 1_794_904_205_000;

    fn summary() -> SessionSummary {
        SessionSummary {
            id: 1,
            started_at_ms: T0_MS,
            mode: "prose".into(),
            code_language: None,
            language: Some("es".into()),
            layout: Some("es-qwerty".into()),
            duration_ms: 61_540.0,
            gross_wpm: 40.0,
            net_wpm: 38.46,
            accuracy: 0.987,
            keystrokes: 312,
            bpm: None,
        }
    }

    fn memory() -> (Store, Profile) {
        let store = Store::open_in_memory().unwrap();
        let profile = store.default_profile().unwrap();
        (store, profile)
    }

    /// Saves a session of `mode` (`code` uses Rust at 80 BPM) started at `started_at_ms`.
    fn save(store: &mut Store, profile: &Profile, mode: &str, started_at_ms: i64) -> i64 {
        save_with(store, profile, mode, started_at_ms, (1_500.0, 38.5, 0.95))
    }

    /// [`save`] with the session's `(duration_ms, net_wpm, accuracy)`.
    fn save_with(
        store: &mut Store,
        profile: &Profile,
        mode: &str,
        started_at_ms: i64,
        (duration_ms, net_wpm, accuracy): (f64, f64, f64),
    ) -> i64 {
        let records = [KeystrokeRecord {
            index: 0,
            expected: 'a',
            typed: 'a',
            correct: true,
            ms_since_previous: None,
        }];
        let code = mode == "code";
        let session = NewSession {
            profile_id: profile.id,
            started_at_ms,
            mode,
            code_language: code.then_some("rust"),
            language: Some("es"),
            layout: Some("es-qwerty"),
            error_mode: ErrorMode::AllowContinue,
            text: "a",
            duration_ms,
            gross_wpm: 40.0,
            net_wpm,
            accuracy,
            keystrokes: 1,
            bpm: code.then_some(80),
            records: &records,
            metronome_offsets_ms: &[],
        };
        store.save_session(&session).unwrap()
    }

    fn ids(sessions: &[SessionSummary]) -> Vec<i64> {
        sessions.iter().map(|s| s.id).collect()
    }

    #[test]
    fn parse_mode_accepts_all_prose_and_code_and_rejects_others() {
        assert_eq!(parse_mode(Locale::Es, None), Ok(None));
        assert_eq!(parse_mode(Locale::Es, Some("prose")), Ok(Some("prose")));
        assert_eq!(parse_mode(Locale::Es, Some("code")), Ok(Some("code")));
        assert_eq!(parse_mode(Locale::Es, Some("words")), Ok(Some("words")));
        for bad in ["xx", "", "Prose"] {
            assert_eq!(
                parse_mode(Locale::Es, Some(bad)),
                Err(format!("Modo desconocido: {bad}")),
                "{bad:?}"
            );
        }
    }

    #[test]
    fn errors_follow_the_interface_language() {
        let (mut store, profile) = memory();
        assert_eq!(
            parse_mode(Locale::En, Some("xx")),
            Err("Unknown mode: xx".to_owned())
        );
        assert_eq!(
            check_tz(Locale::De, 2000),
            Err("Ungültige Zeitzone: 2000".to_owned())
        );
        let resolve = || -> Result<PathBuf, String> { Err("no Downloads".to_owned()) };
        assert_eq!(
            export_sessions_csv(Locale::En, &store, &profile, true, None, 60, T0_MS, resolve),
            Err("There are no sessions to export.".to_owned())
        );
        save(&mut store, &profile, "prose", T0_MS);
        assert_eq!(
            export_sessions_csv(Locale::En, &store, &profile, true, None, 60, T0_MS, resolve),
            Err("Could not export the CSV: no Downloads".to_owned())
        );
    }

    #[test]
    fn session_history_filters_by_mode_and_limit_newest_first() {
        let (mut store, profile) = memory();
        let p1 = save(&mut store, &profile, "prose", 100);
        let c1 = save(&mut store, &profile, "code", 200);
        let p2 = save(&mut store, &profile, "prose", 300);
        let history =
            |mode, limit| session_history(Locale::Es, &store, &profile, true, mode, limit);
        assert_eq!(history(None, None).map(|s| ids(&s)), Ok(vec![p2, c1, p1]));
        assert_eq!(
            history(Some("prose"), None).map(|s| ids(&s)),
            Ok(vec![p2, p1])
        );
        assert_eq!(history(Some("code"), None).map(|s| ids(&s)), Ok(vec![c1]));
        assert_eq!(history(None, Some(2)).map(|s| ids(&s)), Ok(vec![p2, c1]));
        assert_eq!(
            history(Some("prose"), Some(1)).map(|s| ids(&s)),
            Ok(vec![p2])
        );
        let code = history(Some("code"), None).unwrap();
        assert_eq!(code, store.recent_sessions(profile.id, 2).unwrap()[1..]);
    }

    #[test]
    fn session_history_rejects_an_unknown_mode() {
        let (mut store, profile) = memory();
        save(&mut store, &profile, "prose", 100);
        assert_eq!(
            session_history(Locale::Es, &store, &profile, true, Some("xx"), None),
            Err("Modo desconocido: xx".to_owned())
        );
    }

    #[test]
    fn session_history_is_empty_in_temporary_mode() {
        let (mut store, profile) = memory();
        save(&mut store, &profile, "prose", 100);
        for mode in [None, Some("prose")] {
            assert_eq!(
                session_history(Locale::Es, &store, &profile, false, mode, None),
                Ok(vec![])
            );
        }
    }

    #[test]
    fn local_iso_applies_positive_and_negative_offsets() {
        assert_eq!(local_iso(T0_MS, 60), "2026-11-17T09:30:05+01:00");
        assert_eq!(local_iso(T0_MS, 0), "2026-11-17T08:30:05+00:00");
        // Milliseconds are dropped, not rounded.
        assert_eq!(local_iso(T0_MS + 999, 60), "2026-11-17T09:30:05+01:00");
        // 2026-11-17T02:00:00Z is still the 16th in New York.
        assert_eq!(
            local_iso(1_794_880_800_000, -300),
            "2026-11-16T21:00:00-05:00"
        );
        // 2026-12-31T20:00:00Z is already next year in India.
        assert_eq!(
            local_iso(1_798_747_200_000, 330),
            "2027-01-01T01:30:00+05:30"
        );
        assert_eq!(
            local_iso(1_709_208_000_000, -570),
            "2024-02-29T02:30:00-09:30"
        );
        assert_eq!(local_iso(-1, 0), "1969-12-31T23:59:59+00:00");
    }

    #[test]
    fn local_date_is_the_compact_local_day() {
        assert_eq!(local_date(T0_MS, 60), "20261117");
        // 2026-11-17T23:30:00Z.
        assert_eq!(local_date(1_794_958_200_000, 60), "20261118");
        assert_eq!(local_date(1_794_880_800_000, -300), "20261116");
    }

    #[test]
    fn csv_header_is_exact_and_the_file_has_no_bom() {
        let csv = sessions_csv(&[], 60);
        assert_eq!(csv, format!("{CSV_HEADER}\n"));
        assert_eq!(
            CSV_HEADER,
            "fecha,modo,lenguaje_codigo,idioma,layout,duracion_s,ppm_brutas,ppm_netas,precision,pulsaciones,bpm"
        );
        assert!(!csv.starts_with('\u{feff}'));
    }

    #[test]
    fn csv_field_quotes_commas_quotes_and_line_breaks() {
        assert_eq!(csv_field("es-qwerty"), "es-qwerty");
        assert_eq!(csv_field(""), "");
        assert_eq!(csv_field("a,b"), "\"a,b\"");
        assert_eq!(csv_field("di \"hola\""), "\"di \"\"hola\"\"\"");
        assert_eq!(csv_field("a\nb"), "\"a\nb\"");
        assert_eq!(csv_field("a\rb"), "\"a\rb\"");
        let s = SessionSummary {
            layout: Some("es,\"x\"".into()),
            ..summary()
        };
        let csv = sessions_csv(&[s], 60);
        let row = csv.lines().nth(1).unwrap();
        assert!(row.contains(",es,\"es,\"\"x\"\"\",61.5,"), "{row}");
    }

    #[test]
    fn csv_rows_use_one_decimal_percent_and_integer_keystrokes() {
        let csv = sessions_csv(&[summary()], 60);
        assert_eq!(
            csv,
            format!(
                "{CSV_HEADER}\n2026-11-17T09:30:05+01:00,prose,,es,es-qwerty,61.5,40.0,38.5,98.7,312,\n"
            )
        );
    }

    #[test]
    fn csv_leaves_code_language_and_bpm_empty_only_when_absent() {
        let prose = SessionSummary {
            language: None,
            layout: None,
            ..summary()
        };
        let code = SessionSummary {
            id: 2,
            started_at_ms: T0_MS + 60_000,
            mode: "code".into(),
            code_language: Some("rust".into()),
            bpm: Some(80),
            accuracy: 1.0,
            ..summary()
        };
        let csv = sessions_csv(&[code, prose], -300);
        let rows: Vec<&str> = csv.lines().skip(1).collect();
        assert_eq!(
            rows,
            [
                "2026-11-17T03:31:05-05:00,code,rust,es,es-qwerty,61.5,40.0,38.5,100.0,312,80",
                "2026-11-17T03:30:05-05:00,prose,,,,61.5,40.0,38.5,98.7,312,",
            ]
        );
        assert!(csv.ends_with(",\n") && !csv.contains('\r'));
    }

    #[test]
    fn csv_writes_words_sessions_as_palabras() {
        let words = SessionSummary {
            mode: "words".into(),
            ..summary()
        };
        let csv = sessions_csv(&[words], 60);
        assert_eq!(
            csv,
            format!(
                "{CSV_HEADER}\n2026-11-17T09:30:05+01:00,palabras,,es,es-qwerty,61.5,40.0,38.5,98.7,312,\n"
            )
        );
    }

    #[test]
    fn write_unique_appends_a_suffix_and_never_overwrites() {
        let dir = TempDir::new();
        let first = write_unique(Locale::Es, &dir.0, "20261117", "uno\n").unwrap();
        assert_eq!(first, dir.0.join("plexustyping-sesiones-20261117.csv"));
        let second = write_unique(Locale::Es, &dir.0, "20261117", "dos\n").unwrap();
        assert_eq!(second, dir.0.join("plexustyping-sesiones-20261117-2.csv"));
        let third = write_unique(Locale::Es, &dir.0, "20261117", "tres\n").unwrap();
        assert_eq!(third, dir.0.join("plexustyping-sesiones-20261117-3.csv"));
        assert_eq!(std::fs::read_to_string(&first).unwrap(), "uno\n");
        assert_eq!(std::fs::read_to_string(&second).unwrap(), "dos\n");
        assert_eq!(std::fs::read_to_string(&third).unwrap(), "tres\n");
        assert_eq!(dir.files().len(), 3);
    }

    #[test]
    fn write_unique_removes_the_file_it_created_when_the_write_fails() {
        let dir = TempDir::new();
        // A file that was already there is skipped and left untouched.
        let taken = dir.0.join("plexustyping-sesiones-20261117.csv");
        std::fs::write(&taken, "previo\n").unwrap();
        let result = write_unique_with(Locale::Es, &dir.0, "20261117", "uno\n", |file, bytes| {
            file.write_all(&bytes[..1])?;
            Err(std::io::Error::other("disco lleno"))
        });
        assert_eq!(
            result,
            Err("No se pudo exportar el CSV: disco lleno".to_owned())
        );
        assert_eq!(dir.files(), ["plexustyping-sesiones-20261117.csv"]);
        assert_eq!(std::fs::read_to_string(&taken).unwrap(), "previo\n");
    }

    #[test]
    fn write_unique_reports_a_missing_directory() {
        let dir = TempDir::new();
        let missing = dir.0.join("no-existe");
        let result = write_unique(Locale::Es, &missing, "20261117", "uno\n");
        let err = result.unwrap_err();
        assert!(err.starts_with("No se pudo exportar el CSV: "), "{err}");
        assert!(!missing.exists());
        assert_eq!(dir.files(), Vec::<String>::new());
    }

    #[test]
    fn export_writes_the_filtered_sessions_newest_first() {
        let dir = TempDir::new();
        let (mut store, profile) = memory();
        save(&mut store, &profile, "prose", T0_MS);
        save(&mut store, &profile, "code", T0_MS + 1_000);
        save(&mut store, &profile, "prose", T0_MS + 2_000);
        // Exported at 2026-11-17T23:30:00Z: already the 18th at +01:00.
        let now = 1_794_958_200_000;
        let export = |mode| {
            export_sessions_csv(Locale::Es, &store, &profile, true, mode, 60, now, || {
                Ok(dir.0.clone())
            })
        };
        let all = export(None).unwrap();
        let path = dir.0.join("plexustyping-sesiones-20261118.csv");
        let expected = CsvExport {
            path: path.display().to_string(),
            rows: 3,
        };
        assert_eq!(all, expected);
        let sessions = store.sessions(profile.id, None, None).unwrap();
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            sessions_csv(&sessions, 60)
        );
        let lines: Vec<String> = std::fs::read_to_string(&path)
            .unwrap()
            .lines()
            .map(|l| l.split(',').take(2).collect::<Vec<_>>().join(","))
            .collect();
        assert_eq!(
            lines,
            [
                "fecha,modo",
                "2026-11-17T09:30:07+01:00,prose",
                "2026-11-17T09:30:06+01:00,code",
                "2026-11-17T09:30:05+01:00,prose",
            ]
        );
        let prose = export(Some("prose")).unwrap();
        let prose_path = dir.0.join("plexustyping-sesiones-20261118-2.csv");
        assert_eq!(prose.path, prose_path.display().to_string());
        assert_eq!(prose.rows, 2);
        let prose_csv = std::fs::read_to_string(&prose_path).unwrap();
        assert!(prose_csv.lines().skip(1).all(|l| l.contains(",prose,")));
        assert_eq!(
            serde_json::to_value(&prose).unwrap(),
            serde_json::json!({ "path": prose_path.display().to_string(), "rows": 2 })
        );
    }

    #[test]
    fn export_without_sessions_fails_and_writes_nothing() {
        let dir = TempDir::new();
        let (mut store, profile) = memory();
        let resolve = || -> Result<PathBuf, String> { Ok(dir.0.clone()) };
        assert_eq!(
            export_sessions_csv(Locale::Es, &store, &profile, true, None, 60, T0_MS, resolve),
            Err(NO_SESSIONS.es.to_owned())
        );
        assert_eq!(NO_SESSIONS.es, "No hay sesiones que exportar.");
        save(&mut store, &profile, "prose", T0_MS);
        // The filter has no session.
        assert_eq!(
            export_sessions_csv(
                Locale::Es,
                &store,
                &profile,
                true,
                Some("code"),
                60,
                T0_MS,
                resolve
            ),
            Err(NO_SESSIONS.es.to_owned())
        );
        // Temporary mode behaves as an empty history.
        assert_eq!(
            export_sessions_csv(
                Locale::Es,
                &store,
                &profile,
                false,
                None,
                60,
                T0_MS,
                resolve
            ),
            Err(NO_SESSIONS.es.to_owned())
        );
        assert_eq!(dir.files(), Vec::<String>::new());
    }

    #[test]
    fn export_rejects_an_unknown_mode_or_offset_and_writes_nothing() {
        let dir = TempDir::new();
        let (mut store, profile) = memory();
        save(&mut store, &profile, "prose", T0_MS);
        let resolve = || -> Result<PathBuf, String> { Ok(dir.0.clone()) };
        assert_eq!(
            export_sessions_csv(
                Locale::Es,
                &store,
                &profile,
                true,
                Some("xx"),
                60,
                T0_MS,
                resolve
            ),
            Err("Modo desconocido: xx".to_owned())
        );
        for bad in [MAX_TZ_OFFSET_MIN + 1, -MAX_TZ_OFFSET_MIN - 1] {
            assert_eq!(
                export_sessions_csv(
                    Locale::Es,
                    &store,
                    &profile,
                    true,
                    None,
                    bad,
                    T0_MS,
                    resolve
                ),
                Err(format!("Zona horaria no válida: {bad}"))
            );
        }
        assert_eq!(dir.files(), Vec::<String>::new());
    }

    #[test]
    fn export_reports_an_unavailable_directory() {
        let (mut store, profile) = memory();
        save(&mut store, &profile, "prose", T0_MS);
        let result =
            export_sessions_csv(Locale::Es, &store, &profile, true, None, 60, T0_MS, || {
                Err("sin Descargas".to_owned())
            });
        assert_eq!(
            result,
            Err("No se pudo exportar el CSV: sin Descargas".to_owned())
        );
    }

    // ---- History by day and by character (history charts S2, S3, S5) ----

    /// 2026-11-18T00:00:00Z.
    const MIDNIGHT_18_UTC: i64 = 1_794_960_000_000;
    const MINUTE: i64 = 60_000;
    const DAY: i64 = 24 * 60 * MINUTE;

    fn day_row(day: &str, sessions: u32, minutes: f64, net_wpm: f64, accuracy: f64) -> DayRow {
        DayRow {
            day: day.to_owned(),
            sessions,
            minutes,
            net_wpm,
            accuracy,
        }
    }

    fn by_day(
        store: &Store,
        profile: &Profile,
        mode: Option<&str>,
        tz: i32,
        days: Option<u32>,
    ) -> Result<Vec<DayRow>, String> {
        history_by_day(Locale::Es, store, profile, true, mode, tz, days)
    }

    #[test]
    fn by_day_splits_at_local_midnight_with_a_positive_offset() {
        let (mut store, profile) = memory();
        // Local midnight at +01:00 is 23:00Z of the 17th.
        let before = MIDNIGHT_18_UTC - 70 * MINUTE; // 23:50 local, the 17th
        let after = MIDNIGHT_18_UTC - 50 * MINUTE; // 00:10 local, the 18th
        save_with(
            &mut store,
            &profile,
            "prose",
            before,
            (120_000.0, 30.0, 1.0),
        );
        save_with(&mut store, &profile, "prose", after, (60_000.0, 40.0, 0.5));
        assert_eq!(
            by_day(&store, &profile, None, 60, None),
            Ok(vec![
                day_row("2026-11-18", 1, 1.0, 40.0, 0.5),
                day_row("2026-11-17", 1, 2.0, 30.0, 1.0),
            ])
        );
        // In UTC both are on the 17th.
        assert_eq!(
            by_day(&store, &profile, None, 0, None).map(|d| d.len()),
            Ok(1)
        );
    }

    #[test]
    fn by_day_splits_at_local_midnight_with_a_negative_offset() {
        let (mut store, profile) = memory();
        // Local midnight at -05:00 is 05:00Z of the 18th.
        let before = MIDNIGHT_18_UTC + 4 * 60 * MINUTE + 50 * MINUTE; // 23:50 local, the 17th
        let after = MIDNIGHT_18_UTC + 5 * 60 * MINUTE + 10 * MINUTE; // 00:10 local, the 18th
        save_with(
            &mut store,
            &profile,
            "prose",
            before,
            (120_000.0, 30.0, 1.0),
        );
        save_with(&mut store, &profile, "prose", after, (60_000.0, 40.0, 0.5));
        assert_eq!(
            by_day(&store, &profile, None, -300, None),
            Ok(vec![
                day_row("2026-11-18", 1, 1.0, 40.0, 0.5),
                day_row("2026-11-17", 1, 2.0, 30.0, 1.0),
            ])
        );
        // In UTC both are on the 18th.
        assert_eq!(
            by_day(&store, &profile, None, 0, None),
            Ok(vec![day_row("2026-11-18", 2, 3.0, 100.0 / 3.0, 2.5 / 3.0)])
        );
    }

    #[test]
    fn by_day_weights_wpm_and_accuracy_by_duration() {
        let (mut store, profile) = memory();
        let noon = MIDNIGHT_18_UTC + 12 * 60 * MINUTE;
        save_with(&mut store, &profile, "prose", noon, (60_000.0, 40.0, 0.5));
        save_with(
            &mut store,
            &profile,
            "code",
            noon + MINUTE,
            (180_000.0, 60.0, 1.0),
        );
        // Unweighted means would be 50 WPM and 75 %.
        assert_eq!(
            by_day(&store, &profile, None, 0, None),
            Ok(vec![day_row("2026-11-18", 2, 4.0, 55.0, 0.875)])
        );
        // Minutes are not rounded: 90 s is 1.5.
        save_with(
            &mut store,
            &profile,
            "prose",
            noon + 2 * MINUTE,
            (30_000.0, 40.0, 0.5),
        );
        assert_eq!(
            by_day(&store, &profile, Some("prose"), 0, None),
            Ok(vec![day_row("2026-11-18", 2, 1.5, 40.0, 0.5)])
        );
        assert_eq!(
            by_day(&store, &profile, Some("code"), 0, None),
            Ok(vec![day_row("2026-11-18", 1, 3.0, 60.0, 1.0)])
        );
        assert_eq!(by_day(&store, &profile, Some("words"), 0, None), Ok(vec![]));
    }

    #[test]
    fn by_day_with_zero_durations_uses_the_plain_mean() {
        let (mut store, profile) = memory();
        save_with(
            &mut store,
            &profile,
            "prose",
            MIDNIGHT_18_UTC,
            (0.0, 40.0, 0.5),
        );
        save_with(
            &mut store,
            &profile,
            "prose",
            MIDNIGHT_18_UTC + 1,
            (0.0, 60.0, 1.0),
        );
        assert_eq!(
            by_day(&store, &profile, None, 0, None),
            Ok(vec![day_row("2026-11-18", 2, 0.0, 50.0, 0.75)])
        );
    }

    #[test]
    fn by_day_returns_the_newest_days_with_sessions_30_by_default_and_at_most_366() {
        let (mut store, profile) = memory();
        // One session a day for 400 days, plus gaps: only days with sessions count.
        for i in 0..400 {
            save(&mut store, &profile, "prose", MIDNIGHT_18_UTC - 2 * i * DAY);
        }
        let days = |n| by_day(&store, &profile, None, 0, n).unwrap();
        let default = days(None);
        assert_eq!(DEFAULT_DAYS, 30);
        assert_eq!(default.len(), 30);
        assert_eq!(default[0].day, "2026-11-18");
        assert_eq!(default[1].day, "2026-11-16");
        assert_eq!(
            default[29].day,
            local_iso(MIDNIGHT_18_UTC - 58 * DAY, 0)[..10]
        );
        assert_eq!(days(Some(5)).len(), 5);
        assert_eq!(days(Some(366)).len(), 366);
        assert_eq!(MAX_DAYS, 366);
        assert_eq!(days(Some(367)).len(), 366);
        assert_eq!(days(Some(u32::MAX)).len(), 366);
        assert_eq!(days(Some(0)), vec![]);
        let all = days(Some(366));
        assert!(all.windows(2).all(|w| w[0].day > w[1].day));
    }

    #[test]
    fn by_day_refuses_an_unknown_mode_or_offset_without_side_effects() {
        let (mut store, profile) = memory();
        save(&mut store, &profile, "prose", MIDNIGHT_18_UTC);
        let before = store.sessions(profile.id, None, None).unwrap();
        assert_eq!(
            by_day(&store, &profile, Some("xx"), 0, None),
            Err("Modo desconocido: xx".to_owned())
        );
        for bad in [MAX_TZ_OFFSET_MIN + 1, -MAX_TZ_OFFSET_MIN - 1] {
            assert_eq!(
                by_day(&store, &profile, None, bad, None),
                Err(format!("Zona horaria no válida: {bad}"))
            );
            // Refused in temporary mode too.
            assert_eq!(
                history_by_day(Locale::Es, &store, &profile, false, None, bad, None),
                Err(format!("Zona horaria no válida: {bad}"))
            );
        }
        for edge in [MAX_TZ_OFFSET_MIN, -MAX_TZ_OFFSET_MIN] {
            assert_eq!(
                by_day(&store, &profile, None, edge, None).map(|d| d.len()),
                Ok(1)
            );
        }
        assert_eq!(store.sessions(profile.id, None, None).unwrap(), before);
    }

    #[test]
    fn by_day_is_empty_in_temporary_mode() {
        let (mut store, profile) = memory();
        save(&mut store, &profile, "prose", MIDNIGHT_18_UTC);
        for mode in [None, Some("prose")] {
            assert_eq!(
                history_by_day(Locale::Es, &store, &profile, false, mode, 60, None),
                Ok(vec![])
            );
        }
    }

    #[test]
    fn day_row_serializes_in_camel_case() {
        assert_eq!(
            serde_json::to_value(day_row("2026-11-18", 2, 4.0, 55.0, 0.875)).unwrap(),
            serde_json::json!({
                "day": "2026-11-18",
                "sessions": 2,
                "minutes": 4.0,
                "netWpm": 55.0,
                "accuracy": 0.875
            })
        );
    }

    use plexustyping_store::{StatsSnapshot, UnitStatRow};

    /// `(unit, samples, ema_error, ema_slowness, slowness_samples)`.
    type Unit<'a> = (&'a str, u32, f64, f64, u32);

    fn save_units(store: &mut Store, profile: &Profile, mode: &str, units: &[Unit]) {
        let mut units: Vec<UnitStatRow> = units
            .iter()
            .map(
                |&(unit, samples, ema_error, ema_slowness, slowness_samples)| UnitStatRow {
                    unit: unit.to_owned(),
                    ema_error,
                    ema_slowness,
                    samples,
                    slowness_samples,
                },
            )
            .collect();
        units.sort_by(|a, b| a.unit.cmp(&b.unit));
        let snapshot = StatsSnapshot {
            units,
            confusions: vec![],
            mean_interval_ms: None,
        };
        store.save_stats(profile.id, mode, &snapshot).unwrap();
    }

    fn char_row(unit: &str, samples: u32, error: f64, slowness: f64) -> CharRow {
        CharRow {
            unit: unit.to_owned(),
            samples,
            error,
            slowness,
        }
    }

    #[test]
    fn by_char_lists_characters_with_5_samples_worst_first() {
        let (mut store, profile) = memory();
        save_units(
            &mut store,
            &profile,
            "prose",
            &[
                ("a", 5, 0.2, 0.1, 5),
                ("b", 4, 0.9, 0.9, 4),    // below the threshold
                ("ab", 10, 0.9, 0.9, 10), // a bigram, not a character
                (" ", 6, 0.5, 0.0, 6),
                ("\n", 5, 0.2, 0.3, 5), // same error as "a", slower
                ("\t", 7, 0.0, 0.25, 7),
                ("c", 9, 0.0, 0.25, 9), // same error and slowness as Tab
            ],
        );
        assert_eq!(MIN_CHAR_SAMPLES, 5);
        assert_eq!(
            history_by_char(Locale::Es, &store, &profile, true, Some("prose")),
            Ok(vec![
                char_row(" ", 6, 0.5, 0.0),
                char_row("\n", 5, 0.2, 0.3),
                char_row("a", 5, 0.2, 0.1),
                char_row("\t", 7, 0.0, 0.25),
                char_row("c", 9, 0.0, 0.25),
            ])
        );
        // Words practice shares the prose statistics; code has its own.
        assert_eq!(
            history_by_char(Locale::Es, &store, &profile, true, Some("words")),
            history_by_char(Locale::Es, &store, &profile, true, Some("prose"))
        );
        assert_eq!(
            history_by_char(Locale::Es, &store, &profile, true, Some("code")),
            Ok(vec![])
        );
    }

    #[test]
    fn by_char_all_merges_prose_and_code_weighted_by_samples() {
        let (mut store, profile) = memory();
        save_units(
            &mut store,
            &profile,
            "prose",
            &[("a", 3, 0.0, 0.25, 1), ("p", 5, 0.1, 0.0, 0)],
        );
        save_units(
            &mut store,
            &profile,
            "code",
            &[("a", 3, 0.5, 0.75, 3), ("{", 8, 0.75, 0.5, 8)],
        );
        // "a" has 3 samples in each mode: only listed when merged.
        assert_eq!(
            history_by_char(Locale::Es, &store, &profile, true, None),
            Ok(vec![
                char_row("{", 8, 0.75, 0.5),
                char_row("a", 6, 0.25, 0.625),
                char_row("p", 5, 0.1, 0.0),
            ])
        );
        assert_eq!(
            history_by_char(Locale::Es, &store, &profile, true, Some("prose")),
            Ok(vec![char_row("p", 5, 0.1, 0.0)])
        );
        assert_eq!(
            history_by_char(Locale::Es, &store, &profile, true, Some("code")),
            Ok(vec![char_row("{", 8, 0.75, 0.5)])
        );
    }

    #[test]
    fn by_char_refuses_an_unknown_mode_and_is_empty_in_temporary_mode() {
        let (mut store, profile) = memory();
        save_units(&mut store, &profile, "prose", &[("a", 9, 0.5, 0.5, 9)]);
        let before = store.load_stats(profile.id, "prose").unwrap();
        assert_eq!(
            history_by_char(Locale::Es, &store, &profile, true, Some("xx")),
            Err("Modo desconocido: xx".to_owned())
        );
        assert_eq!(
            history_by_char(Locale::Es, &store, &profile, false, Some("xx")),
            Err("Modo desconocido: xx".to_owned())
        );
        for mode in [None, Some("prose"), Some("code")] {
            assert_eq!(
                history_by_char(Locale::Es, &store, &profile, false, mode),
                Ok(vec![])
            );
        }
        assert_eq!(store.load_stats(profile.id, "prose").unwrap(), before);
    }

    #[test]
    fn char_row_serializes_in_camel_case() {
        assert_eq!(
            serde_json::to_value(char_row("a", 6, 0.25, 0.625)).unwrap(),
            serde_json::json!({ "unit": "a", "samples": 6, "error": 0.25, "slowness": 0.625 })
        );
    }
}
