//! S1, S6: `ghost_for_text` for the active profile, through the plain command function.

use std::sync::Arc;

use plexustyping_ai::{AiClient, Endpoints};
use plexustyping_core::{ErrorMode, IndentMode, InputEvent};
use serde_json::json;

use super::*;
use crate::code::PracticeMode;
use crate::commands::{MetronomeTrack, ScoreRequest, open_state, score_practice};
use crate::profiles;
use crate::secrets::{NoStore, SharedSecrets};

/// A temporary (in-memory) state, as when the database cannot be opened.
fn memory() -> AppState {
    let secrets: SharedSecrets = Arc::new(NoStore);
    let ai = AiClient::new(Endpoints::loopback("http://127.0.0.1:9"));
    open_state(Err("test: no data dir".to_owned()), secrets, ai)
}

/// `text` typed correctly at `step_ms` per char, scored and stored as the app does.
fn type_text(
    state: &AppState,
    practice: &PracticeMode,
    text: &str,
    step_ms: f64,
    started: i64,
) -> bool {
    let events: Vec<InputEvent> = text
        .chars()
        .enumerate()
        .map(|(i, ch)| InputEvent::Char {
            ch,
            t_ms: i as f64 * step_ms,
        })
        .collect();
    let req = ScoreRequest {
        text,
        mode: ErrorMode::AllowContinue,
        practice,
        indent: IndentMode::Manual,
        events: &events,
        started_at_ms: started,
        metronome: &MetronomeTrack::default(),
    };
    let mut store = lock_store(&state.store);
    let profile = state.active_profile();
    score_practice(&mut store, &profile, &req).saved
}

#[test]
fn the_ghost_is_json_with_session_id_net_wpm_and_times_in_camel_case() {
    let state = memory();
    assert!(type_text(&state, &PracticeMode::Prose, "hola", 100.0, 1));
    let ghost = for_text(&state, "hola", &PracticeMode::Prose)
        .unwrap()
        .unwrap();
    // 4 chars = 0.8 words in 300 ms: 160 net PPM.
    assert_eq!(
        serde_json::to_value(&ghost).unwrap(),
        json!({
            "sessionId": ghost.session_id,
            "netWpm": 160.0,
            "times": [0.0, 100.0, 200.0, 300.0],
        })
    );
    assert_eq!(serde_json::to_value(None::<Ghost>).unwrap(), json!(null));
}

#[test]
fn no_record_for_the_text_is_null() {
    let state = memory();
    assert_eq!(for_text(&state, "hola", &PracticeMode::Prose), Ok(None));
    assert!(type_text(&state, &PracticeMode::Prose, "adiós", 100.0, 1));
    assert_eq!(for_text(&state, "hola", &PracticeMode::Prose), Ok(None));
}

#[test]
fn temporary_mode_races_the_in_memory_sessions_and_a_new_record_is_the_next_ghost() {
    let state = memory();
    assert!(!state.status.persistent);
    assert!(type_text(&state, &PracticeMode::Prose, "hola", 100.0, 1));
    let first = for_text(&state, "hola", &PracticeMode::Prose)
        .unwrap()
        .unwrap();
    assert!(type_text(&state, &PracticeMode::Prose, "hola", 50.0, 2));
    let next = for_text(&state, "hola", &PracticeMode::Prose)
        .unwrap()
        .unwrap();
    assert_ne!(next.session_id, first.session_id);
    assert_eq!(
        next.times,
        vec![Some(0.0), Some(50.0), Some(100.0), Some(150.0)]
    );
    // A slower attempt does not replace the record.
    assert!(type_text(&state, &PracticeMode::Prose, "hola", 200.0, 3));
    let after = for_text(&state, "hola", &PracticeMode::Prose)
        .unwrap()
        .unwrap();
    assert_eq!(after.session_id, next.session_id);
}

#[test]
fn the_ghost_is_of_the_active_profile_mode_and_code_language() {
    let state = memory();
    let rust = PracticeMode::code("rust").unwrap();
    let python = PracticeMode::code("python").unwrap();
    assert!(type_text(&state, &rust, "x = 1", 100.0, 1));
    assert!(for_text(&state, "x = 1", &rust).unwrap().is_some());
    assert_eq!(for_text(&state, "x = 1", &python), Ok(None));
    assert_eq!(for_text(&state, "x = 1", &PracticeMode::Prose), Ok(None));
    assert_eq!(for_text(&state, "x = 1", &PracticeMode::Words), Ok(None));
    let other = profiles::create(&state, "Otro", "es", "es-qwerty").unwrap();
    profiles::switch(&state, other.id).unwrap();
    assert_eq!(for_text(&state, "x = 1", &rust), Ok(None));
}
