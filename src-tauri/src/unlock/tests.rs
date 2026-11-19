//! Progressive unlock (phase 3, S1–S5, S7) through the command functions:
//! in-memory or temp-dir stores, an offline or loopback mock AI, no keychain.

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use plexustyping_ai::{AiClient, Endpoints, ProviderId};
use plexustyping_core::{ErrorMode, InputEvent, KeystrokeRecord};
use plexustyping_store::{NewCachedText, NewSession, Store, UnitStatRow};
use serde_json::json;

use super::*;
use crate::ai::mock::{Mock, mock};
use crate::ai::{self, AiSettings, ChainEntry, TextParams};
use crate::code::{DEFAULT_MAX_LINES, PracticeMode};
use crate::commands::{
    self, NextTextRequest, STATS_MODE, ScoredSession, StorageStatus, TextSource, lock_store,
    open_state, score_session,
};
use crate::profiles;
use crate::secrets::{MemoryStore, NoStore, SharedSecrets};

const OPENAI: &str = "/v1/chat/completions";
const DAY: &str = "2026-11-18";
const ES_HOME: &str = "fjdkslañgh";

/// Fresh directory under the system temp dir, removed on drop.
struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        use std::sync::atomic::{AtomicU32, Ordering};
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        TempDir(std::env::temp_dir().join(format!(
            "plexustyping-app-unlock-{}-{}-{n}",
            std::process::id(),
            std::time::UNIX_EPOCH.elapsed().unwrap().as_nanos()
        )))
    }

    fn db(&self) -> PathBuf {
        self.0.join("plexustyping.sqlite3")
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn offline() -> AiClient {
    AiClient::new(Endpoints::loopback("http://127.0.0.1:9"))
}

fn state_with(secrets: SharedSecrets, ai: AiClient) -> AppState {
    let store = Store::open_in_memory().unwrap();
    let profile = store.default_profile().unwrap();
    AppState {
        store: Mutex::new(store),
        profile: Mutex::new(profile),
        status: StorageStatus {
            persistent: false,
            error: None,
        },
        ai,
        secrets,
    }
}

fn memory() -> AppState {
    state_with(Arc::new(NoStore), offline())
}

fn reopen(path: PathBuf) -> AppState {
    open_state(Ok(path), Arc::new(NoStore), offline())
}

fn chars(s: &str) -> Vec<String> {
    s.chars().map(String::from).collect()
}

fn es(state: &AppState) -> UnlockStatus {
    status(state, "es", "es-qwerty").unwrap()
}

fn enable(state: &AppState) {
    set(state, "es", "es-qwerty", Some(true), None).unwrap();
}

/// One char event per typed char, 500 ms apart: slower than every target,
/// so practising alone never masters a key.
fn typing(typed: &str) -> Vec<InputEvent> {
    let events = typed
        .chars()
        .enumerate()
        .map(|(i, ch)| json!({ "kind": "char", "ch": ch.to_string(), "tMs": i as f64 * 500.0 }));
    serde_json::from_value(events.collect()).unwrap()
}

/// Scores a correctly typed prose session of `text` for the active profile.
fn practise(state: &AppState, text: &str) -> ScoredSession {
    let mut store = lock_store(&state.store);
    let profile = state.active_profile();
    let scored = score_session(
        &mut store,
        &profile,
        text,
        ErrorMode::AllowContinue,
        &typing(text),
        1,
    );
    assert!(scored.saved, "{:?}", scored.save_error);
    scored
}

/// Unlocks `n` keys through stalled sessions (3 each).
fn stall_unlock(state: &AppState, n: usize) {
    for _ in 0..n {
        for _ in 0..3 {
            practise(state, "ff");
        }
    }
}

/// Sets the prose statistics of `ch` (samples, ema_error), keeping the others.
fn set_stat(state: &AppState, ch: char, samples: u32, ema_error: f64) {
    let mut store = lock_store(&state.store);
    let id = state.active_profile().id;
    let mut snapshot = store.load_stats(id, STATS_MODE).unwrap();
    snapshot.units.retain(|u| u.unit != ch.to_string());
    snapshot.units.push(UnitStatRow {
        unit: ch.to_string(),
        ema_error,
        ema_slowness: 0.0,
        samples,
        slowness_samples: samples,
    });
    store.save_stats(id, STATS_MODE, &snapshot).unwrap();
}

/// Stores a prose session (no stats) of correct keystrokes on `ch`, one per interval.
fn save_latencies(state: &AppState, ch: char, intervals: &[f64], at: i64) {
    let records: Vec<KeystrokeRecord> = intervals
        .iter()
        .enumerate()
        .map(|(i, ms)| KeystrokeRecord {
            index: i,
            expected: ch,
            typed: ch,
            correct: true,
            ms_since_previous: Some(*ms),
        })
        .collect();
    let text: String = intervals.iter().map(|_| ch).collect();
    let mut store = lock_store(&state.store);
    store
        .save_session(&NewSession {
            profile_id: state.active_profile().id,
            started_at_ms: at,
            mode: "prose",
            code_language: None,
            language: Some("es"),
            layout: Some("es-qwerty"),
            error_mode: ErrorMode::AllowContinue,
            text: &text,
            duration_ms: 1000.0,
            gross_wpm: 10.0,
            net_wpm: 10.0,
            accuracy: 1.0,
            keystrokes: records.len(),
            bpm: None,
            records: &records,
            metronome_offsets_ms: &[],
        })
        .unwrap();
}

/// `ch` meets all three conditions at 30 PPM.
fn master(state: &AppState, ch: char) {
    set_stat(state, ch, 30, 0.0);
    save_latencies(state, ch, &[300.0; 5], 0);
}

fn params(seed: u64, mode: PracticeMode) -> TextParams {
    TextParams {
        language: "es".into(),
        layout: "es-qwerty".into(),
        seed,
        target_count: 2,
        length: 300,
        fallback: vec![],
        day: DAY.into(),
        mode,
        max_lines: DEFAULT_MAX_LINES,
    }
}

fn next(state: &AppState, seed: u64) -> commands::NextText {
    tauri::async_runtime::block_on(ai::next_text(state, &params(seed, PracticeMode::Prose)))
        .unwrap()
}

/// The text the same prose request gets with the mode off (AI off, no cache).
fn normal_text(state: &AppState, seed: u64) -> commands::NextText {
    let store = lock_store(&state.store);
    let model = commands::load_model(&store, &state.active_profile()).unwrap();
    let p = params(seed, PracticeMode::Prose);
    let req = NextTextRequest {
        language: &p.language,
        layout: &p.layout,
        seed,
        target_count: p.target_count,
        length: p.length,
        fallback: &p.fallback,
    };
    commands::next_text(&model, &req).unwrap()
}

fn only(text: &str, allowed: &BTreeSet<char>) {
    for c in text.chars() {
        assert!(c == ' ' || allowed.contains(&c), "{c:?} is locked: {text}");
    }
}

// ---- S2: start ----

#[test]
fn spanish_qwerty_starts_with_its_home_row_and_space() {
    let state = memory();
    assert_eq!(
        serde_json::to_value(es(&state)).unwrap(),
        json!({
            "enabled": false,
            "targetWpm": 30,
            "active": ["f", "j", "d", "k", "s", "l", "a", "ñ", "g", "h"],
            "mastered": [],
            "next": "e",
            "total": 27,
            "stalledSessions": 0
        })
    );
}

#[test]
fn us_dvorak_starts_with_the_letters_on_its_home_row_keys() {
    let state = memory();
    let got = status(&state, "en", "us-dvorak").unwrap();
    assert_eq!(got.active, chars("uhetonasid"));
    assert_eq!((got.next.as_deref(), got.total), (Some("r"), 26));
    // us-qwerty: Semicolon types ';', not a letter.
    let qwerty = status(&state, "en", "us-qwerty").unwrap();
    assert_eq!(qwerty.active, chars("fjdkslagh"));
}

#[test]
fn the_order_is_the_home_row_then_the_language_frequency() {
    let layout = plexustyping_layouts::builtin("es-qwerty").unwrap();
    let corpus = plexustyping_corpus::corpus("es").unwrap();
    let order: String = unlock_order(&layout, corpus).into_iter().collect();
    assert!(order.starts_with("fjdkslañgheonritucmp"), "{order}");
    // Every a–z letter plus ñ; ç (absent from the Spanish corpus) and º are not letters to unlock.
    assert_eq!(order.chars().count(), 27, "{order}");
    let set: BTreeSet<char> = order.chars().collect();
    let expected: BTreeSet<char> = "abcdefghijklmnopqrstuvwxyzñ".chars().collect();
    assert_eq!(set, expected);
}

// ---- S3: mastered key ----

#[test]
fn a_key_needs_thirty_samples() {
    let state = memory();
    set_stat(&state, 'f', 29, 0.0);
    save_latencies(&state, 'f', &[300.0], 0);
    assert!(es(&state).mastered.is_empty());
    set_stat(&state, 'f', 30, 0.0);
    assert_eq!(es(&state).mastered, ["f"]);
}

#[test]
fn a_key_needs_an_error_ema_of_at_most_five_percent() {
    let state = memory();
    save_latencies(&state, 'j', &[300.0], 0);
    set_stat(&state, 'j', 40, 0.05);
    assert_eq!(es(&state).mastered, ["j"]);
    set_stat(&state, 'j', 40, 0.0501);
    assert!(es(&state).mastered.is_empty());
}

#[test]
fn a_key_needs_its_median_latency_within_the_target() {
    let state = memory();
    set_stat(&state, 'd', 30, 0.0);
    // Median of 300, 400, 900 = 400 ms = the 30 PPM target.
    save_latencies(&state, 'd', &[300.0, 400.0, 900.0], 0);
    assert_eq!(es(&state).mastered, ["d"]);
    save_latencies(&state, 'd', &[401.0, 401.0, 401.0], 1);
    // Newest six: 300, 400, 900, 401, 401, 401 → median 401 ms.
    assert!(es(&state).mastered.is_empty());
    // No correct keystroke: no latency, not mastered.
    set_stat(&state, 'k', 30, 0.0);
    assert!(!es(&state).mastered.contains(&"k".to_owned()));
}

#[test]
fn only_the_last_two_hundred_correct_keystrokes_count() {
    let state = memory();
    set_stat(&state, 's', 500, 0.0);
    save_latencies(&state, 's', &[900.0; 300], 0);
    save_latencies(&state, 's', &[350.0; 200], 1);
    assert_eq!(es(&state).mastered, ["s"]);
}

#[test]
fn the_target_is_twelve_thousand_ms_over_the_ppm() {
    assert_eq!(target_ms(30), 400.0);
    assert_eq!(TARGET_WPMS.map(target_ms), [600.0, 400.0, 300.0, 240.0]);
    let state = memory();
    set_stat(&state, 'l', 30, 0.0);
    save_latencies(&state, 'l', &[600.0], 0);
    assert!(es(&state).mastered.is_empty());
    set(&state, "es", "es-qwerty", None, Some(20)).unwrap();
    assert_eq!(es(&state).mastered, ["l"]);
    set(&state, "es", "es-qwerty", None, Some(50)).unwrap();
    assert!(es(&state).mastered.is_empty());
}

// ---- S4: unlock ----

#[test]
fn the_next_key_unlocks_when_every_active_key_is_mastered() {
    let state = memory();
    enable(&state);
    for ch in ES_HOME.chars() {
        master(&state, ch);
    }
    let scored = practise(&state, "ff");
    assert_eq!(scored.unlocked.as_deref(), Some("e"));
    assert_eq!(
        serde_json::to_value(&scored).unwrap()["unlocked"],
        json!("e")
    );
    let after = es(&state);
    assert_eq!(after.active, chars("fjdkslañghe"));
    assert_eq!(
        (after.next.as_deref(), after.stalled_sessions),
        (Some("o"), 0)
    );
    // The new key is not mastered: nothing more unlocks; this session stalls.
    assert_eq!(practise(&state, "ff").unlocked, None);
    assert_eq!(es(&state).stalled_sessions, 1);
}

#[test]
fn the_third_session_without_progress_unlocks_and_resets_the_counter() {
    let state = memory();
    enable(&state);
    assert_eq!(practise(&state, "ff").unlocked, None);
    assert_eq!(es(&state).stalled_sessions, 1);
    assert_eq!(practise(&state, "jj").unlocked, None);
    assert_eq!(es(&state).stalled_sessions, 2);
    assert_eq!(practise(&state, "dd").unlocked.as_deref(), Some("e"));
    let after = es(&state);
    assert_eq!((after.stalled_sessions, after.active.len()), (0, 11));
    assert_eq!(practise(&state, "ff").unlocked, None);
    assert_eq!(es(&state).stalled_sessions, 1);
}

#[test]
fn a_session_that_masters_an_active_key_is_progress() {
    let state = memory();
    enable(&state);
    practise(&state, "kk");
    set_stat(&state, 'f', 28, 0.0);
    save_latencies(&state, 'f', &[300.0; 5], 0);
    // Two more correct f: 30 samples, mastered in this session.
    let scored = practise(&state, "ff");
    assert_eq!(scored.unlocked, None);
    let after = es(&state);
    assert_eq!(after.mastered, ["f"]);
    assert_eq!(
        after.stalled_sessions, 0,
        "a progress session restarts the count: 3 sessions in a row without progress unlock"
    );
}

#[test]
fn uppercase_follows_its_letter_and_signs_wait_for_every_letter() {
    let state = memory();
    enable(&state);
    stall_unlock(&state, 5);
    let at15 = es(&state);
    assert_eq!(at15.active, chars("fjdkslañgheonri"));
    let mut allowed: BTreeSet<char> = "fjdkslañgheonri".chars().collect();
    allowed.extend("FJDKSLAÑGHEONRI".chars());
    let mut upper = false;
    for seed in 0..6 {
        let text = next(&state, seed);
        assert_eq!(text.source, TextSource::Corpus);
        only(&text.text, &allowed);
        upper |= text.text.chars().any(char::is_uppercase);
    }
    assert!(upper, "no capital of an active letter");
    // Every letter unlocked: next is none, and digits and signs unlock with them.
    stall_unlock(&state, 12);
    let all = es(&state);
    assert_eq!((all.active.len(), all.next), (27, None));
    for seed in 0..3 {
        assert_eq!(next(&state, seed), normal_text(&state, seed));
    }
    // Nothing left to unlock.
    for _ in 0..3 {
        assert_eq!(practise(&state, "ff").unlocked, None);
    }
}

#[test]
fn words_and_code_sessions_never_unlock() {
    let state = memory();
    enable(&state);
    for practice in [PracticeMode::Words, PracticeMode::code("rust").unwrap()] {
        for _ in 0..3 {
            let mut store = lock_store(&state.store);
            let events = typing("ff");
            let req = commands::ScoreRequest {
                text: "ff",
                mode: ErrorMode::AllowContinue,
                practice: &practice,
                indent: plexustyping_core::IndentMode::Manual,
                events: &events,
                started_at_ms: 1,
                metronome: &commands::MetronomeTrack::default(),
            };
            let scored = commands::score_practice(&mut store, &state.active_profile(), &req);
            assert!(scored.saved);
            assert_eq!(scored.unlocked, None);
        }
    }
    assert_eq!(es(&state).stalled_sessions, 0);
}

// ---- S5: texts ----

fn ai_on(state: &AppState) {
    let settings = AiSettings {
        enabled: true,
        chain: vec![ChainEntry {
            provider: ProviderId::Openai,
            model: "gpt-5-mini".into(),
        }],
        density: 3.0,
        topic: None,
    };
    ai::save_settings(&lock_store(&state.store), settings).unwrap();
}

fn chat_reply(text: &str) -> String {
    json!({
        "choices": [{"message": {"role": "assistant", "content": json!({"text": text}).to_string()}, "finish_reason": "stop"}],
        "usage": {"prompt_tokens": 80, "completion_tokens": 30}
    })
    .to_string()
}

async fn ai_state(reply: &str) -> (AppState, Mock) {
    let srv = mock(vec![(OPENAI, 200, chat_reply(reply))]).await;
    let memory = MemoryStore::shared();
    let shared: SharedSecrets = memory.clone();
    ai::key_save(&shared, ProviderId::Openai, "sk-test".into())
        .await
        .unwrap();
    let state = state_with(memory, srv.client());
    ai_on(&state);
    (state, srv)
}

fn cache_locked_text(state: &AppState) {
    lock_store(&state.store)
        .cache_text(&NewCachedText {
            provider: "claude",
            model: "m",
            language: "es",
            mode: "prose",
            targets: &[],
            text: "Un texto con letras bloqueadas y puntos.",
            created_at_ms: 1,
            used: false,
        })
        .unwrap();
}

#[tokio::test]
async fn below_fifteen_letters_texts_are_active_words_without_the_ai() {
    let (state, srv) = ai_state("La niña Hilda lee en la orilla del lago helado sin fin").await;
    cache_locked_text(&state);
    enable(&state);
    let home: BTreeSet<char> = ES_HOME.chars().collect();
    for seed in 0..4 {
        let text = ai::next_text(&state, &params(seed, PracticeMode::Prose))
            .await
            .unwrap();
        assert_eq!(
            (text.source, text.notice.clone()),
            (TextSource::Corpus, None)
        );
        only(&text.text, &home);
        let len = text.text.chars().count();
        assert!((300..361).contains(&len), "{len}: {}", text.text);
    }
    // A new profile gets no diagnostic text in this mode.
    let first = ai::first_text(&state, "es", "es-qwerty", &PracticeMode::Prose, 12, 1, DAY)
        .await
        .unwrap();
    assert!(!first.diagnostic);
    only(&first.text.text, &home);
    assert!(srv.requests().is_empty(), "the AI was called");
}

#[test]
fn below_fifteen_letters_the_new_key_and_weak_keys_are_practised() {
    let state = memory();
    enable(&state);
    stall_unlock(&state, 1);
    let text = next(&state, 3);
    assert!(text.targets.contains(&"e".to_owned()), "{:?}", text.targets);
    let es_count = text.text.chars().filter(|c| *c == 'e').count();
    assert!(es_count >= 20, "e {es_count}: {}", text.text);
    // A weak active key is a target too.
    set_stat(&state, 'ñ', 10, 0.9);
    let text = next(&state, 3);
    assert!(text.targets.contains(&"ñ".to_owned()), "{:?}", text.targets);
}

#[tokio::test]
async fn from_fifteen_letters_the_ai_writes_with_the_active_characters() {
    const ACTIVE: &str = "La niña Hilda lee en la orilla del lago helado sin fin";
    let (state, srv) = ai_state(ACTIVE).await;
    enable(&state);
    stall_unlock(&state, 5);
    let mut p = params(1, PracticeMode::Prose);
    p.length = 50;
    let text = ai::next_text(&state, &p).await.unwrap();
    assert_eq!((text.source, text.text.as_str()), (TextSource::Ai, ACTIVE));
    assert_eq!(srv.requests().len(), 1);
}

#[tokio::test]
async fn from_fifteen_letters_a_locked_ai_text_falls_back_to_a_restricted_corpus() {
    let (state, srv) = ai_state("Un texto con letras bloqueadas y puntos.").await;
    enable(&state);
    stall_unlock(&state, 5);
    cache_locked_text(&state);
    let text = ai::next_text(&state, &params(1, PracticeMode::Prose))
        .await
        .unwrap();
    // The AI was asked (the client retries a rejected text once).
    assert!(!srv.requests().is_empty());
    assert_eq!(text.source, TextSource::Corpus, "{}", text.text);
    assert!(text.notice.is_some());
    let mut allowed: BTreeSet<char> = "fjdkslañgheonri".chars().collect();
    allowed.extend("FJDKSLAÑGHEONRI".chars());
    only(&text.text, &allowed);
    // With the AI off, no cached text either.
    let mut settings = ai::load_settings(&lock_store(&state.store));
    settings.enabled = false;
    ai::save_settings(&lock_store(&state.store), settings).unwrap();
    let text = ai::next_text(&state, &params(2, PracticeMode::Prose))
        .await
        .unwrap();
    assert_eq!(text.source, TextSource::Corpus);
    only(&text.text, &allowed);
}

#[tokio::test]
async fn a_continuation_keeps_the_unlock_restriction() {
    // Unlimited (S2): continuations only use the allowed characters.
    let (state, srv) = ai_state("Un texto con letras bloqueadas y puntos.").await;
    enable(&state);
    let p = ai::ContinueParams {
        language: "es".into(),
        layout: "es-qwerty".into(),
        mode: ai::ContinueMode::Prose,
        tail: "la niña lee".into(),
        target_count: 3,
        seed: 1,
        day: DAY.into(),
    };
    // Below fifteen letters: active words, never the AI.
    let text = ai::continue_text(&state, &p).await.unwrap();
    assert_eq!(
        (text.source, text.notice.clone()),
        (TextSource::Corpus, None)
    );
    only(&text.text, &ES_HOME.chars().collect());
    assert!(srv.requests().is_empty(), "the AI was called");
    // From fifteen: the prompt names the active characters; a locked reply
    // falls back to a restricted corpus text.
    stall_unlock(&state, 5);
    let text = ai::continue_text(&state, &p).await.unwrap();
    let sent: serde_json::Value = serde_json::from_str(&srv.requests()[0].body).unwrap();
    let prompt = sent["messages"][0]["content"].as_str().unwrap();
    assert!(
        prompt.starts_with("Continúa de forma natural")
            && prompt.contains("«la niña lee»")
            && prompt.contains("Usa solo estos caracteres:"),
        "{prompt}"
    );
    assert_eq!(text.source, TextSource::Corpus, "{}", text.text);
    assert!(text.notice.is_some());
    let mut allowed: BTreeSet<char> = "fjdkslañgheonri".chars().collect();
    allowed.extend("FJDKSLAÑGHEONRI".chars());
    only(&text.text, &allowed);
}

#[test]
fn words_mode_ignores_the_unlock_mode() {
    let state = memory();
    let words = |state: &AppState| {
        tauri::async_runtime::block_on(ai::next_text(state, &params(5, PracticeMode::Words)))
            .unwrap()
    };
    let off = words(&state);
    enable(&state);
    assert_eq!(words(&state), off);
}

// ---- S7: contract ----

#[test]
fn unlock_set_refuses_other_targets_without_side_effects() {
    let state = memory();
    for bad in [0, 25, 35, 60] {
        assert_eq!(
            set(&state, "es", "es-qwerty", Some(true), Some(bad)),
            Err("Velocidad objetivo no válida".to_owned())
        );
    }
    let unchanged = es(&state);
    assert!(!unchanged.enabled);
    assert_eq!(unchanged.target_wpm, 30);
    for ok in TARGET_WPMS {
        let got = set(&state, "es", "es-qwerty", None, Some(ok)).unwrap();
        assert_eq!((got.enabled, got.target_wpm), (false, ok));
    }
    let on = set(&state, "es", "es-qwerty", Some(true), None).unwrap();
    assert_eq!((on.enabled, on.target_wpm), (true, 50));
    assert_eq!(
        set(&state, "es", "nope", Some(true), None),
        Err("unknown layout: nope".to_owned())
    );
    assert_eq!(
        status(&state, "xx", "es-qwerty"),
        Err("unknown language: xx".to_owned())
    );
}

#[test]
fn progress_survives_reopening_and_turning_the_mode_off_and_on() {
    let dir = TempDir::new();
    {
        let state = reopen(dir.db());
        enable(&state);
        stall_unlock(&state, 1);
        practise(&state, "ff");
        set(&state, "es", "es-qwerty", Some(false), Some(40)).unwrap();
    }
    let state = reopen(dir.db());
    let got = es(&state);
    assert_eq!((got.enabled, got.target_wpm), (false, 40));
    assert_eq!((got.active.len(), got.stalled_sessions), (11, 1));
    let on = set(&state, "es", "es-qwerty", Some(true), None).unwrap();
    assert_eq!((on.active.len(), on.stalled_sessions), (11, 1));
}

#[test]
fn progress_is_per_profile_language_and_layout() {
    let state = memory();
    enable(&state);
    stall_unlock(&state, 1);
    assert_eq!(es(&state).active.len(), 11);
    // Same profile: the mode is on everywhere, progress is per language and layout.
    let latam = status(&state, "es", "latam-qwerty").unwrap();
    assert_eq!((latam.enabled, latam.active.len()), (true, 10));
    let english = status(&state, "en", "es-qwerty").unwrap();
    assert_eq!((english.enabled, english.active.len()), (true, 10));
    // Another profile starts off and from its home row.
    let other = profiles::create(&state, "Otra", "es", "es-qwerty").unwrap();
    profiles::switch(&state, other.id).unwrap();
    let fresh = es(&state);
    assert_eq!((fresh.enabled, fresh.active.len()), (false, 10));
}

#[test]
fn a_deleted_profile_takes_its_progress_with_it() {
    let state = memory();
    let first = state.active_profile().id;
    let other = profiles::create(&state, "Otra", "es", "es-qwerty").unwrap();
    profiles::switch(&state, other.id).unwrap();
    enable(&state);
    stall_unlock(&state, 1);
    profiles::switch(&state, first).unwrap();
    profiles::delete(&state, other.id).unwrap();
    // SQLite may reuse the id of the deleted last row.
    let again = profiles::create(&state, "Nueva", "es", "es-qwerty").unwrap();
    profiles::switch(&state, again.id).unwrap();
    let fresh = es(&state);
    assert_eq!((fresh.enabled, fresh.active.len()), (false, 10));
}

#[test]
fn temporary_mode_keeps_the_state_in_memory() {
    let state = open_state(
        Err("test: no data dir".into()),
        Arc::new(NoStore),
        offline(),
    );
    assert!(!state.status.persistent);
    enable(&state);
    stall_unlock(&state, 1);
    let got = es(&state);
    assert_eq!((got.enabled, got.active.len()), (true, 11));
}

// ---- mode off: unchanged ----

#[tokio::test]
async fn with_the_mode_off_nothing_changes() {
    let state = memory();
    let first = ai::first_text(&state, "es", "es-qwerty", &PracticeMode::Prose, 12, 1, DAY)
        .await
        .unwrap();
    assert!(first.diagnostic);
    for _ in 0..4 {
        let scored = practise(&state, "ff");
        assert_eq!(scored.unlocked, None);
        assert_eq!(
            serde_json::to_value(&scored).unwrap()["unlocked"],
            json!(null)
        );
    }
    let got = es(&state);
    assert_eq!((got.active.len(), got.stalled_sessions), (10, 0));
    for seed in 0..3 {
        let text = ai::next_text(&state, &params(seed, PracticeMode::Prose))
            .await
            .unwrap();
        assert_eq!(text, normal_text(&state, seed));
    }
}
