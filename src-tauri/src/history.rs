//! Session history and CSV export (phase 3, S6–S7), testable without a Tauri runtime.

use std::fs::{File, OpenOptions};
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};

use plexustyping_store::{Profile, SessionSummary, Store};
use serde::Serialize;

/// First line of every export (S7).
pub const CSV_HEADER: &str = "fecha,modo,lenguaje_codigo,idioma,layout,duracion_s,ppm_brutas,ppm_netas,precision,pulsaciones,bpm";
/// Error when the active filter has no sessions (S7).
pub const NO_SESSIONS: &str = "No hay sesiones que exportar.";
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

/// The stored mode of a history filter: `None` (all), `"prose"` or `"code"`.
pub fn parse_mode(mode: Option<&str>) -> Result<Option<&'static str>, String> {
    match mode {
        None => Ok(None),
        Some("prose") => Ok(Some("prose")),
        Some("code") => Ok(Some("code")),
        Some(other) => Err(format!("Modo desconocido: {other}")),
    }
}

/// Sessions of `profile` for the filter `mode`, newest first, at most `limit`;
/// empty in temporary mode (`persistent == false`).
pub fn session_history(
    store: &Store,
    profile: &Profile,
    persistent: bool,
    mode: Option<&str>,
    limit: Option<u32>,
) -> Result<Vec<SessionSummary>, String> {
    let mode = parse_mode(mode)?;
    if !persistent {
        return Ok(Vec::new());
    }
    let limit = limit.map(|n| n as usize);
    store
        .sessions(profile.id, mode, limit)
        .map_err(|e| e.to_string())
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

/// One CSV row of `s` (without the line end).
fn csv_row(s: &SessionSummary, tz_offset_min: i32) -> String {
    let text = |v: &Option<String>| csv_field(v.as_deref().unwrap_or(""));
    let fields = [
        local_iso(s.started_at_ms, tz_offset_min),
        csv_field(&s.mode),
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
pub fn write_unique(dir: &Path, date: &str, contents: &str) -> Result<PathBuf, String> {
    write_unique_with(dir, date, contents, |file, bytes| {
        file.write_all(bytes)?;
        file.flush()
    })
}

/// [`write_unique`] with the write step injected, so tests can make it fail.
fn write_unique_with(
    dir: &Path,
    date: &str,
    contents: &str,
    write: impl Fn(&mut File, &[u8]) -> std::io::Result<()>,
) -> Result<PathBuf, String> {
    let fail = |e: std::io::Error| format!("No se pudo exportar el CSV: {e}");
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
    Err("No se pudo exportar el CSV: no hay nombre libre".to_owned())
}

/// Exports every session of the filter `mode` to the directory given by `dir`
/// (resolved only when there is something to write), naming the file after
/// the local date of `now_ms`.
pub fn export_sessions_csv(
    store: &Store,
    profile: &Profile,
    persistent: bool,
    mode: Option<&str>,
    tz_offset_min: i32,
    now_ms: i64,
    dir: impl FnOnce() -> Result<PathBuf, String>,
) -> Result<CsvExport, String> {
    parse_mode(mode)?;
    if !(-MAX_TZ_OFFSET_MIN..=MAX_TZ_OFFSET_MIN).contains(&tz_offset_min) {
        return Err(format!("Zona horaria no válida: {tz_offset_min}"));
    }
    let sessions = session_history(store, profile, persistent, mode, None)?;
    if sessions.is_empty() {
        return Err(NO_SESSIONS.to_owned());
    }
    let rows = u32::try_from(sessions.len()).map_err(|e| e.to_string())?;
    let dir = dir().map_err(|e| format!("No se pudo exportar el CSV: {e}"))?;
    let csv = sessions_csv(&sessions, tz_offset_min);
    let path = write_unique(&dir, &local_date(now_ms, tz_offset_min), &csv)?;
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
            duration_ms: 1_500.0,
            gross_wpm: 40.0,
            net_wpm: 38.5,
            accuracy: 0.95,
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
        assert_eq!(parse_mode(None), Ok(None));
        assert_eq!(parse_mode(Some("prose")), Ok(Some("prose")));
        assert_eq!(parse_mode(Some("code")), Ok(Some("code")));
        for bad in ["xx", "", "Prose"] {
            assert_eq!(
                parse_mode(Some(bad)),
                Err(format!("Modo desconocido: {bad}")),
                "{bad:?}"
            );
        }
    }

    #[test]
    fn session_history_filters_by_mode_and_limit_newest_first() {
        let (mut store, profile) = memory();
        let p1 = save(&mut store, &profile, "prose", 100);
        let c1 = save(&mut store, &profile, "code", 200);
        let p2 = save(&mut store, &profile, "prose", 300);
        let history = |mode, limit| session_history(&store, &profile, true, mode, limit);
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
            session_history(&store, &profile, true, Some("xx"), None),
            Err("Modo desconocido: xx".to_owned())
        );
    }

    #[test]
    fn session_history_is_empty_in_temporary_mode() {
        let (mut store, profile) = memory();
        save(&mut store, &profile, "prose", 100);
        for mode in [None, Some("prose")] {
            assert_eq!(
                session_history(&store, &profile, false, mode, None),
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
    fn write_unique_appends_a_suffix_and_never_overwrites() {
        let dir = TempDir::new();
        let first = write_unique(&dir.0, "20261117", "uno\n").unwrap();
        assert_eq!(first, dir.0.join("plexustyping-sesiones-20261117.csv"));
        let second = write_unique(&dir.0, "20261117", "dos\n").unwrap();
        assert_eq!(second, dir.0.join("plexustyping-sesiones-20261117-2.csv"));
        let third = write_unique(&dir.0, "20261117", "tres\n").unwrap();
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
        let result = write_unique_with(&dir.0, "20261117", "uno\n", |file, bytes| {
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
        let result = write_unique(&missing, "20261117", "uno\n");
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
        let export =
            |mode| export_sessions_csv(&store, &profile, true, mode, 60, now, || Ok(dir.0.clone()));
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
            export_sessions_csv(&store, &profile, true, None, 60, T0_MS, resolve),
            Err(NO_SESSIONS.to_owned())
        );
        assert_eq!(NO_SESSIONS, "No hay sesiones que exportar.");
        save(&mut store, &profile, "prose", T0_MS);
        // The filter has no session.
        assert_eq!(
            export_sessions_csv(&store, &profile, true, Some("code"), 60, T0_MS, resolve),
            Err(NO_SESSIONS.to_owned())
        );
        // Temporary mode behaves as an empty history.
        assert_eq!(
            export_sessions_csv(&store, &profile, false, None, 60, T0_MS, resolve),
            Err(NO_SESSIONS.to_owned())
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
            export_sessions_csv(&store, &profile, true, Some("xx"), 60, T0_MS, resolve),
            Err("Modo desconocido: xx".to_owned())
        );
        for bad in [MAX_TZ_OFFSET_MIN + 1, -MAX_TZ_OFFSET_MIN - 1] {
            assert_eq!(
                export_sessions_csv(&store, &profile, true, None, bad, T0_MS, resolve),
                Err(format!("Zona horaria no válida: {bad}"))
            );
        }
        assert_eq!(dir.files(), Vec::<String>::new());
    }

    #[test]
    fn export_reports_an_unavailable_directory() {
        let (mut store, profile) = memory();
        save(&mut store, &profile, "prose", T0_MS);
        let result = export_sessions_csv(&store, &profile, true, None, 60, T0_MS, || {
            Err("sin Descargas".to_owned())
        });
        assert_eq!(
            result,
            Err("No se pudo exportar el CSV: sin Descargas".to_owned())
        );
    }
}
