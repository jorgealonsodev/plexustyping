//! Command logic behind the Tauri IPC handlers, testable without a Tauri runtime.

use std::collections::BTreeSet;
use std::ops::Range;
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard};

use plexustyping_ai::AiClient;
use plexustyping_core::adaptive::{Confusion, UnitStat, WeaknessModel};
use plexustyping_core::{ErrorMode, InputEvent, SessionResult, failed_chars, replay};
use plexustyping_corpus::{Corpus, GenerateOptions, corpus};
use plexustyping_layouts::{KeyPress, Layout};
use plexustyping_store::{
    ConfusionRow, NewSession, Profile, SessionSummary, StatsSnapshot, Store, StoreError,
    UnitStatRow,
};
use serde::{Deserialize, Serialize};

use crate::secrets::SharedSecrets;

/// Mode of practice sessions and of their statistics.
pub const STATS_MODE: &str = "prose";
/// Targets per text when the user has not chosen (S2: 3–5).
pub const DEFAULT_TARGETS: usize = 3;
/// Wanted text length in chars when the user has not chosen.
pub const DEFAULT_LENGTH: usize = 300;

/// Shared state of the running app.
pub struct AppState {
    /// The database; in memory when the file could not be opened.
    pub store: Mutex<Store>,
    /// The active profile.
    pub profile: Profile,
    /// Whether sessions survive a restart.
    pub status: StorageStatus,
    /// The one AI client (production endpoints in the app).
    pub ai: AiClient,
    /// Where AI keys live (the system keychain in the app).
    pub secrets: SharedSecrets,
}

/// The store; a panic while it was held leaves it usable (each write is
/// transactional). Never hold the guard across an `.await`.
pub fn lock_store(store: &Mutex<Store>) -> MutexGuard<'_, Store> {
    store.lock().unwrap_or_else(|e| e.into_inner())
}

/// Whether the store is persistent, and why not.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct StorageStatus {
    pub persistent: bool,
    pub error: Option<String>,
}

/// Authoritative replay of a finished session, its failed chars and whether it was stored.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScoredSession {
    /// Records and stats from `plexustyping-core`.
    pub result: SessionResult,
    /// Expected chars of wrong keystrokes, most failed first.
    pub failed_chars: Vec<char>,
    /// The session and the updated statistics were stored.
    pub saved: bool,
    /// Why storing failed; `None` when saved or not meant to be saved.
    pub save_error: Option<String>,
}

/// Where a practice text came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum TextSource {
    /// Generated now by an AI provider.
    Ai,
    /// A previously generated AI text from the local cache.
    Cache,
    /// The local corpus.
    Corpus,
}

/// A practice text, the targets it really practises and its origin.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct NextText {
    pub text: String,
    pub targets: Vec<String>,
    pub source: TextSource,
    /// Provider id (`ai` and `cache` texts).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    /// Model id (`ai` and `cache` texts).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Why the AI was not used although it is enabled (Spanish, for the user).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notice: Option<String>,
}

impl NextText {
    /// A text of the local corpus.
    pub fn corpus(text: String, targets: Vec<String>) -> Self {
        NextText {
            text,
            targets,
            source: TextSource::Corpus,
            provider: None,
            model: None,
            notice: None,
        }
    }
}

/// The text to start the app with.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct FirstText {
    #[serde(flatten)]
    pub text: NextText,
    /// The fixed diagnostic text (no session saved yet).
    pub diagnostic: bool,
}

/// One ranked weak unit (char or bigram).
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Weakness {
    pub unit: String,
    pub score: f64,
}

/// Score of one char, for the keyboard heatmap.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct HeatScore {
    pub ch: char,
    pub score: f64,
}

/// What [`next_text`] needs besides the model.
#[derive(Clone, Debug)]
pub struct NextTextRequest<'a> {
    pub language: &'a str,
    pub layout: &'a str,
    pub seed: u64,
    /// Clamped to 3..=5.
    pub target_count: usize,
    /// Clamped to 50..=600.
    pub length: usize,
    /// Used as targets while the model has no candidates (the last failed chars).
    pub fallback: &'a [String],
}

/// What [`next_text`] decided before generating: shared by the AI and corpus paths.
#[derive(Clone, Debug)]
pub struct TextPlan {
    pub corpus: &'static Corpus,
    pub targets: Vec<String>,
    /// Accepted length in chars.
    pub length: Range<usize>,
    /// Chars the layout can type.
    pub allowed: BTreeSet<char>,
}

/// Opens the database at `path`, or an in-memory one (logging why) so practice still works.
pub fn open_state(path: Result<PathBuf, String>, secrets: SharedSecrets, ai: AiClient) -> AppState {
    let opened = path.and_then(|path| {
        let store = Store::open(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        let profile = store.default_profile().map_err(|e| e.to_string())?;
        Ok((store, profile))
    });
    let (store, profile, status) = match opened {
        Ok((store, profile)) => (
            store,
            profile,
            StorageStatus {
                persistent: true,
                error: None,
            },
        ),
        Err(error) => {
            eprintln!("PlexusTyping: cannot open the database, using a temporary one: {error}");
            let store = Store::open_in_memory().expect("in-memory database");
            let profile = store.default_profile().expect("in-memory profile");
            let status = StorageStatus {
                persistent: false,
                error: Some(error),
            };
            (store, profile, status)
        }
    };
    AppState {
        store: Mutex::new(store),
        profile,
        status,
        ai,
        secrets,
    }
}

/// Rebuilds the model from stored statistics.
pub fn model_from_snapshot(snapshot: StatsSnapshot) -> WeaknessModel {
    let units = snapshot.units.into_iter().map(|u| {
        let stat = UnitStat {
            ema_error: u.ema_error,
            ema_slowness: u.ema_slowness,
            samples: u.samples,
            slowness_samples: u.slowness_samples,
        };
        (u.unit, stat)
    });
    let confusions = snapshot.confusions.into_iter().map(|c| Confusion {
        expected: c.expected,
        typed: c.typed,
        count: c.count,
    });
    WeaknessModel::from_parts(
        units.collect(),
        confusions.collect(),
        snapshot.mean_interval_ms,
    )
}

/// The storable form of `model`.
pub fn snapshot_from_model(model: &WeaknessModel) -> StatsSnapshot {
    let units = model.units().into_iter().map(|(unit, s)| UnitStatRow {
        unit,
        ema_error: s.ema_error,
        ema_slowness: s.ema_slowness,
        samples: s.samples,
        slowness_samples: s.slowness_samples,
    });
    let confusions = model.confusions().into_iter().map(|c| ConfusionRow {
        expected: c.expected,
        typed: c.typed,
        count: c.count,
    });
    StatsSnapshot {
        units: units.collect(),
        confusions: confusions.collect(),
        mean_interval_ms: model.mean_interval_ms(),
    }
}

/// The stored model of `profile`.
pub fn load_model(store: &Store, profile: &Profile) -> Result<WeaknessModel, String> {
    store
        .load_stats(profile.id, STATS_MODE)
        .map(model_from_snapshot)
        .map_err(|e| e.to_string())
}

fn unknown_language(language: &str) -> String {
    format!("unknown language: {language}")
}

/// The built-in layout `layout`.
pub fn get_layout(layout: &str) -> Result<Layout, String> {
    plexustyping_layouts::builtin(layout).ok_or_else(|| format!("unknown layout: {layout}"))
}

/// The key presses that type each char of `text` on `layout`, `None` for untypeable chars.
pub fn key_sequences(layout: &str, text: &str) -> Result<Vec<Option<Vec<KeyPress>>>, String> {
    let layout = get_layout(layout)?;
    Ok(text.chars().map(|ch| layout.locate(ch)).collect())
}

/// The fixed diagnostic text of `language`.
pub fn diagnostic_text(language: &str) -> Result<String, String> {
    corpus(language)
        .map(|c| c.diagnostic().to_owned())
        .ok_or_else(|| unknown_language(language))
}

/// Calls `generate` with `targets`; if that yields nothing, once more without
/// targets, which are then not reported.
pub fn generate_with_retry(
    targets: &[String],
    generate: impl Fn(&[String]) -> String,
) -> Result<NextText, String> {
    let text = generate(targets);
    if !text.is_empty() {
        return Ok(NextText::corpus(text, targets.to_vec()));
    }
    let text = generate(&[]);
    if text.is_empty() {
        Err("could not generate a practice text".to_owned())
    } else {
        Ok(NextText::corpus(text, Vec::new()))
    }
}

/// An adaptive corpus practice text: see [`plan_text`] and [`corpus_text`].
pub fn next_text(model: &WeaknessModel, req: &NextTextRequest) -> Result<NextText, String> {
    corpus_text(&plan_text(model, req)?, req.seed)
}

/// Targets, length and allowed chars of the next text: the model's weakest
/// units, else `fallback`, as targets. Targets no corpus word contains
/// (spaces, signs) are dropped, so the reported targets are the ones the text
/// practises.
pub fn plan_text(model: &WeaknessModel, req: &NextTextRequest) -> Result<TextPlan, String> {
    let corpus = corpus(req.language).ok_or_else(|| unknown_language(req.language))?;
    let layout = get_layout(req.layout)?;
    let count = req.target_count.clamp(3, 5);
    let length = req.length.clamp(50, 600);
    let mut targets = model.select_targets(count, req.seed);
    if targets.is_empty() {
        targets = req.fallback.to_vec();
    }
    targets.retain(|t| {
        let t = t.to_lowercase();
        !t.is_empty() && corpus.words().iter().any(|w| w.contains(&t))
    });
    targets.truncate(count);
    Ok(TextPlan {
        corpus,
        targets,
        length: length..length + length / 5 + 1,
        allowed: layout.allowed_chars(),
    })
}

/// A corpus text for `plan`.
pub fn corpus_text(plan: &TextPlan, seed: u64) -> Result<NextText, String> {
    generate_with_retry(&plan.targets, |targets| {
        plan.corpus.generate(&GenerateOptions {
            targets: targets.to_vec(),
            length: plan.length.clone(),
            allowed: Some(plan.allowed.clone()),
            seed,
        })
    })
}

/// The diagnostic text while `profile` has no saved session.
pub fn diagnostic_for_new_profile(
    store: &Store,
    profile: &Profile,
    language: &str,
) -> Result<Option<FirstText>, String> {
    let sessions = store
        .recent_sessions(profile.id, 1)
        .map_err(|e| e.to_string())?;
    if !sessions.is_empty() {
        return Ok(None);
    }
    Ok(Some(FirstText {
        text: NextText::corpus(diagnostic_text(language)?, Vec::new()),
        diagnostic: true,
    }))
}

/// Replays a session; a completed one with keystrokes is stored and folded
/// into the stored model atomically. A storage failure stores nothing and
/// keeps the result.
pub fn score_session(
    store: &mut Store,
    profile: &Profile,
    text: &str,
    mode: ErrorMode,
    events: &[InputEvent],
    started_at_ms: i64,
) -> ScoredSession {
    let result = replay(text, mode, events);
    let failed_chars = failed_chars(&result);
    let (saved, save_error) = if result.completed && !result.records.is_empty() {
        match persist(store, profile, text, mode, &result, started_at_ms) {
            Ok(()) => (true, None),
            Err(e) => (false, Some(e.to_string())),
        }
    } else {
        (false, None)
    };
    ScoredSession {
        result,
        failed_chars,
        saved,
        save_error,
    }
}

/// Stores the session and folds it into the stored model in one transaction.
fn persist(
    store: &mut Store,
    profile: &Profile,
    text: &str,
    mode: ErrorMode,
    result: &SessionResult,
    started_at_ms: i64,
) -> Result<(), StoreError> {
    let stats = &result.stats;
    let session = NewSession {
        profile_id: profile.id,
        started_at_ms,
        mode: STATS_MODE,
        code_language: None,
        language: Some(&profile.language),
        layout: Some(&profile.layout),
        error_mode: mode,
        text,
        duration_ms: stats.duration_ms,
        gross_wpm: stats.gross_wpm,
        net_wpm: stats.net_wpm,
        accuracy: stats.accuracy,
        keystrokes: stats.keystrokes,
        bpm: None,
        records: &result.records,
    };
    store.save_session_with_stats(&session, STATS_MODE, |snapshot| {
        let mut model = model_from_snapshot(snapshot);
        model.update(text, result);
        snapshot_from_model(&model)
    })?;
    Ok(())
}

/// Up to `limit` weakest ranked units.
pub fn weaknesses(model: &WeaknessModel, limit: usize) -> Vec<Weakness> {
    let ranked = model.ranked().into_iter().take(limit);
    ranked
        .map(|(unit, score)| Weakness { unit, score })
        .collect()
}

/// Score of every practised char.
pub fn heatmap(model: &WeaknessModel) -> Vec<HeatScore> {
    let scores = model.char_scores().into_iter();
    scores.map(|(ch, score)| HeatScore { ch, score }).collect()
}

/// Up to `limit` sessions of `profile`, newest first.
pub fn recent_sessions(
    store: &Store,
    profile: &Profile,
    limit: usize,
) -> Result<Vec<SessionSummary>, String> {
    store
        .recent_sessions(profile.id, limit)
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use plexustyping_layouts::{KeyPress, Layer};
    use std::cell::RefCell;

    fn memory() -> (Store, Profile) {
        let store = Store::open_in_memory().unwrap();
        let profile = store.default_profile().unwrap();
        (store, profile)
    }

    /// One char event per typed char, 100 ms apart.
    fn typing(typed: &str) -> Vec<InputEvent> {
        let events = typed.chars().enumerate().map(|(i, ch)| {
            let t_ms = i as f64 * 100.0;
            serde_json::json!({ "kind": "char", "ch": ch.to_string(), "tMs": t_ms })
        });
        serde_json::from_value(events.collect()).unwrap()
    }

    fn stat(ema_error: f64) -> UnitStat {
        UnitStat {
            ema_error,
            samples: 5,
            ..UnitStat::default()
        }
    }

    fn model(units: &[(&str, f64)]) -> WeaknessModel {
        let units = units.iter().map(|&(u, e)| (u.to_owned(), stat(e)));
        WeaknessModel::from_parts(units.collect(), vec![], Some(150.0))
    }

    fn request(seed: u64, target_count: usize, length: usize) -> NextTextRequest<'static> {
        NextTextRequest {
            language: "es",
            layout: "es-qwerty",
            seed,
            target_count,
            length,
            fallback: &[],
        }
    }

    #[test]
    fn diagnostic_text_returns_the_spanish_corpus_text() {
        let text = diagnostic_text("es").expect("es is bundled");
        assert_eq!(text, corpus("es").unwrap().diagnostic());
        assert!(!text.is_empty());
    }

    #[test]
    fn diagnostic_text_rejects_an_unknown_language() {
        assert_eq!(
            diagnostic_text("xx"),
            Err("unknown language: xx".to_owned())
        );
    }

    #[test]
    fn open_state_falls_back_to_a_temporary_store() {
        for path in [
            Err("no data dir".to_owned()),
            Ok("/dev/null/x.sqlite3".into()),
        ] {
            let secrets: SharedSecrets = std::sync::Arc::new(crate::secrets::NoStore);
            let ai = AiClient::new(plexustyping_ai::Endpoints::loopback("http://127.0.0.1:9"));
            let state = open_state(path, secrets, ai);
            assert!(!state.status.persistent);
            assert!(state.status.error.is_some());
            let mut store = state.store.lock().unwrap();
            let scored = score_session(
                &mut store,
                &state.profile,
                "ab",
                ErrorMode::AllowContinue,
                &typing("ab"),
                1,
            );
            assert!(scored.saved, "practice still works: {scored:?}");
        }
    }

    #[test]
    fn model_and_snapshot_round_trip_through_the_store() {
        let (mut store, profile) = memory();
        let mut model = WeaknessModel::new();
        let text = "queso añejo";
        model.update(
            text,
            &replay(text, ErrorMode::AllowContinue, &typing("qieso anejo")),
        );
        let snapshot = snapshot_from_model(&model);
        assert_eq!(model_from_snapshot(snapshot.clone()), model);
        store.save_stats(profile.id, STATS_MODE, &snapshot).unwrap();
        assert_eq!(load_model(&store, &profile), Ok(model.clone()));
        assert!(!model.confusions().is_empty() && model.mean_interval_ms().is_some());
    }

    #[test]
    fn score_session_saves_a_complete_session_and_updates_stats() {
        let (mut store, profile) = memory();
        let events = typing("hxla");
        let scored = score_session(
            &mut store,
            &profile,
            "hola",
            ErrorMode::AllowContinue,
            &events,
            42,
        );
        assert_eq!(
            scored.result,
            replay("hola", ErrorMode::AllowContinue, &events)
        );
        assert_eq!((scored.saved, scored.save_error), (true, None));
        let sessions = store.recent_sessions(profile.id, 5).unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(
            (sessions[0].started_at_ms, sessions[0].mode.as_str()),
            (42, "prose")
        );
        assert_eq!(
            (
                sessions[0].language.as_deref(),
                sessions[0].layout.as_deref()
            ),
            (Some("es"), Some("es-qwerty"))
        );
        assert_eq!(
            store.session_keystrokes(sessions[0].id).unwrap(),
            scored.result.records
        );
        let mut expected = WeaknessModel::new();
        expected.update("hola", &scored.result);
        assert_eq!(load_model(&store, &profile), Ok(expected.clone()));
        // A second session folds into the stored model.
        score_session(
            &mut store,
            &profile,
            "hola",
            ErrorMode::AllowContinue,
            &events,
            43,
        );
        expected.update("hola", &scored.result);
        assert_eq!(load_model(&store, &profile), Ok(expected));
    }

    #[test]
    fn score_session_does_not_save_incomplete_or_empty_sessions() {
        let (mut store, profile) = memory();
        for (text, typed) in [("hola", "ho"), ("hola", ""), ("", "")] {
            let scored = score_session(
                &mut store,
                &profile,
                text,
                ErrorMode::AllowContinue,
                &typing(typed),
                1,
            );
            assert_eq!(
                (scored.saved, scored.save_error),
                (false, None),
                "{text:?}/{typed:?}"
            );
        }
        assert_eq!(recent_sessions(&store, &profile, 5), Ok(vec![]));
        assert_eq!(load_model(&store, &profile), Ok(WeaknessModel::new()));
    }

    #[test]
    fn score_session_keeps_the_result_when_saving_fails() {
        let (mut store, profile) = memory();
        let ghost = Profile {
            id: 999,
            ..profile.clone()
        };
        let events = typing("hxla");
        let scored = score_session(
            &mut store,
            &ghost,
            "hola",
            ErrorMode::AllowContinue,
            &events,
            1,
        );
        assert_eq!(
            scored.result,
            replay("hola", ErrorMode::AllowContinue, &events)
        );
        assert_eq!(scored.failed_chars, vec!['o']);
        assert!(!scored.saved);
        assert!(
            scored
                .save_error
                .as_deref()
                .is_some_and(|e| e.contains("database error"))
        );
        assert_eq!(recent_sessions(&store, &profile, 5), Ok(vec![]));
        assert_eq!(load_model(&store, &ghost), Ok(WeaknessModel::new()));
    }

    #[test]
    fn score_session_stats_failure_stores_nothing_and_keeps_the_model() {
        let dir = std::env::temp_dir().join(format!(
            "plexustyping-app-test-{}-{}",
            std::process::id(),
            std::time::UNIX_EPOCH.elapsed().unwrap().as_nanos()
        ));
        let path = dir.join("db.sqlite3");
        let mut store = Store::open(&path).unwrap();
        let profile = store.default_profile().unwrap();
        let events = typing("hxla");
        let first = score_session(
            &mut store,
            &profile,
            "hola",
            ErrorMode::AllowContinue,
            &events,
            1,
        );
        assert!(first.saved);
        let model = load_model(&store, &profile).unwrap();
        rusqlite::Connection::open(&path)
            .unwrap()
            .execute_batch(
                "CREATE TRIGGER t BEFORE INSERT ON char_stats
                 BEGIN SELECT RAISE(ABORT, 'boom'); END;",
            )
            .unwrap();
        let scored = score_session(
            &mut store,
            &profile,
            "hola",
            ErrorMode::AllowContinue,
            &events,
            2,
        );
        let sessions = recent_sessions(&store, &profile, 5).map(|s| s.len());
        let after = load_model(&store, &profile);
        drop(store);
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(
            scored.result,
            replay("hola", ErrorMode::AllowContinue, &events)
        );
        assert!(!scored.saved);
        assert!(
            scored
                .save_error
                .as_deref()
                .is_some_and(|e| e.contains("boom")),
            "{:?}",
            scored.save_error
        );
        assert_eq!(sessions, Ok(1), "the failed session is rolled back");
        assert_eq!(after, Ok(model));
    }

    #[test]
    fn score_session_serializes_as_camel_case_json() {
        let (mut store, profile) = memory();
        let scored = score_session(
            &mut store,
            &profile,
            "hola",
            ErrorMode::AllowContinue,
            &typing("hxla"),
            1,
        );
        let json = serde_json::to_value(&scored).unwrap();
        assert_eq!(json["failedChars"], serde_json::json!(["o"]));
        assert_eq!(json["result"]["stats"]["errors"], 1);
        assert_eq!(
            (json["saved"].clone(), json["saveError"].clone()),
            (true.into(), serde_json::Value::Null)
        );
    }

    #[test]
    fn next_text_uses_the_model_targets_within_length_and_layout() {
        let model = model(&[("ñ", 0.9), ("qu", 0.8), ("z", 0.7)]);
        let next = next_text(&model, &request(7, 3, 300)).unwrap();
        assert_eq!(next.targets, model.select_targets(3, 7));
        let allowed = get_layout("es-qwerty").unwrap().allowed_chars();
        let len = next.text.chars().count();
        assert!((300..361).contains(&len), "length {len}");
        assert!(next.text.chars().all(|c| c == '.' || allowed.contains(&c)));
        assert!(
            next.text.contains('ñ') && next.text.contains("qu"),
            "{}",
            next.text
        );
        assert_eq!(next_text(&model, &request(7, 3, 300)), Ok(next));
    }

    #[test]
    fn next_text_clamps_target_count_and_length() {
        let units = [
            ("ñ", 0.9),
            ("qu", 0.8),
            ("z", 0.7),
            ("v", 0.6),
            ("rr", 0.5),
            ("j", 0.4),
            ("x", 0.3),
        ];
        let model = model(&units);
        for (count, length, targets, range) in [(1, 10, 3, 50..61), (9, 5000, 5, 600..721)] {
            let next = next_text(&model, &request(3, count, length)).unwrap();
            assert_eq!(next.targets.len(), targets, "count {count}");
            let len = next.text.chars().count();
            assert!(range.contains(&len), "length {len} for {length}");
        }
    }

    #[test]
    fn next_text_falls_back_to_the_given_chars_without_candidates() {
        let fallback = vec!["ñ".to_owned()];
        let req = NextTextRequest {
            fallback: &fallback,
            ..request(2, 3, 200)
        };
        let next = next_text(&WeaknessModel::new(), &req).unwrap();
        assert_eq!(next.targets, fallback);
        assert!(next.text.contains('ñ'), "{}", next.text);
        // Candidates win over the fallback.
        let next = next_text(&model(&[("z", 0.9)]), &req).unwrap();
        assert_eq!(next.targets, vec!["z".to_owned()]);
    }

    #[test]
    fn next_text_drops_targets_no_word_contains() {
        let model = model(&[(" ", 0.99), (",", 0.95), ("ñ", 0.5)]);
        let next = next_text(&model, &request(1, 3, 200)).unwrap();
        assert_eq!(next.targets, vec!["ñ".to_owned()]);
        let fallback = vec![".".to_owned(), "7".to_owned()];
        let req = NextTextRequest {
            fallback: &fallback,
            ..request(1, 3, 200)
        };
        assert_eq!(
            next_text(&WeaknessModel::new(), &req).unwrap().targets,
            Vec::<String>::new()
        );
    }

    #[test]
    fn next_text_rejects_unknown_language_and_layout() {
        let model = WeaknessModel::new();
        let req = NextTextRequest {
            language: "xx",
            ..request(1, 3, 300)
        };
        assert_eq!(
            next_text(&model, &req),
            Err("unknown language: xx".to_owned())
        );
        let req = NextTextRequest {
            layout: "dvorak-xx",
            ..request(1, 3, 300)
        };
        assert_eq!(
            next_text(&model, &req),
            Err("unknown layout: dvorak-xx".to_owned())
        );
    }

    fn targets(list: &[&str]) -> Vec<String> {
        list.iter().map(|t| (*t).to_owned()).collect()
    }

    #[test]
    fn generate_with_retry_keeps_a_non_empty_first_result_and_its_targets() {
        let calls = RefCell::new(Vec::new());
        let next = generate_with_retry(&targets(&["qu"]), |t| {
            calls.borrow_mut().push(t.to_vec());
            "qué".to_owned()
        });
        assert_eq!(
            next,
            Ok(NextText::corpus("qué".to_owned(), targets(&["qu"])))
        );
        assert_eq!(calls.into_inner(), vec![targets(&["qu"])]);
    }

    #[test]
    fn generate_with_retry_retries_once_without_targets_and_reports_none() {
        let calls = RefCell::new(Vec::new());
        let next = generate_with_retry(&targets(&["q"]), |t| {
            calls.borrow_mut().push(t.to_vec());
            if t.is_empty() { "hola." } else { "" }.to_owned()
        });
        assert_eq!(next, Ok(NextText::corpus("hola.".to_owned(), vec![])));
        assert_eq!(calls.into_inner(), vec![targets(&["q"]), vec![]]);
    }

    #[test]
    fn generate_with_retry_fails_when_both_attempts_are_empty() {
        let calls = RefCell::new(0);
        let next = generate_with_retry(&targets(&["q"]), |_| {
            *calls.borrow_mut() += 1;
            String::new()
        });
        assert_eq!(next, Err("could not generate a practice text".to_owned()));
        assert_eq!(calls.into_inner(), 2);
    }

    #[test]
    fn weaknesses_and_heatmap_come_from_the_model() {
        let model = model(&[("a", 0.2), ("qu", 0.9), ("ñ", 0.5)]);
        let list = weaknesses(&model, 2);
        let units: Vec<&str> = list.iter().map(|w| w.unit.as_str()).collect();
        assert_eq!(units, ["qu", "ñ"]);
        assert_eq!(list[0].score, model.score("qu").unwrap());
        let heat = heatmap(&model);
        let chars: Vec<char> = heat.iter().map(|h| h.ch).collect();
        assert_eq!(chars, ['a', 'ñ']);
        assert_eq!(
            serde_json::to_value(&heat[0]).unwrap(),
            serde_json::json!({ "ch": "a", "score": model.score("a").unwrap() })
        );
    }

    #[test]
    fn key_sequences_locates_each_char_once_as_camel_case_json() {
        let seqs = key_sequences("es-qwerty", "aÁ@¨").expect("es-qwerty is bundled");
        let press = |code: &str, layer| KeyPress {
            code: code.to_owned(),
            layer,
        };
        assert_eq!(
            seqs,
            vec![
                Some(vec![press("KeyA", Layer::Normal)]),
                Some(vec![
                    press("Quote", Layer::Normal),
                    press("KeyA", Layer::Shift)
                ]),
                Some(vec![press("Digit2", Layer::Altgr)]),
                None,
            ]
        );
        assert_eq!(
            serde_json::to_value(&seqs[0]).unwrap(),
            serde_json::json!([{ "code": "KeyA", "layer": "normal" }])
        );
        assert_eq!(key_sequences("es-qwerty", ""), Ok(vec![]));
    }

    #[test]
    fn key_sequences_and_get_layout_reject_an_unknown_layout() {
        assert_eq!(
            key_sequences("xx", "a"),
            Err("unknown layout: xx".to_owned())
        );
        assert_eq!(get_layout("xx"), Err("unknown layout: xx".to_owned()));
    }

    #[test]
    fn get_layout_returns_the_builtin_layout_as_camel_case_json() {
        let layout = get_layout("es-qwerty").expect("es-qwerty is bundled");
        assert_eq!(layout, plexustyping_layouts::builtin("es-qwerty").unwrap());
        let json = serde_json::to_value(&layout).unwrap();
        assert_eq!(json["name"], "Español QWERTY");
        assert_eq!(json["deadKeys"], serde_json::json!(["´", "`", "^", "¨"]));
        assert_eq!(
            json["keys"]["Semicolon"],
            serde_json::json!({ "normal": "ñ", "shift": "Ñ", "altgr": null })
        );
    }
}
