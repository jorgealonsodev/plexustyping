//! Ghost session (S1, S2): the best earlier session with a text and its
//! timeline, through the public store API only.

use plexustyping_core::{ErrorMode, IndentMode, InputEvent, KeystrokeRecord, replay_with};
use plexustyping_store::{Ghost, NewSession, Store};

fn ch(c: char, t_ms: f64) -> InputEvent {
    InputEvent::Char { ch: c, t_ms }
}

fn bs(t_ms: f64) -> InputEvent {
    InputEvent::Backspace { t_ms }
}

fn tab(t_ms: f64) -> InputEvent {
    InputEvent::Tab { t_ms }
}

/// Where and how a session was typed.
#[derive(Clone, Copy)]
struct Kind<'a> {
    mode: &'a str,
    code_language: Option<&'a str>,
}

const PROSE: Kind = Kind {
    mode: "prose",
    code_language: None,
};

/// A session of `profile_id` with explicit records; 1 s, 30 net PPM, started at 1 unless changed.
fn new_session<'a>(
    profile_id: i64,
    kind: Kind<'a>,
    text: &'a str,
    error_mode: ErrorMode,
    records: &'a [KeystrokeRecord],
) -> NewSession<'a> {
    NewSession {
        profile_id,
        started_at_ms: 1,
        mode: kind.mode,
        code_language: kind.code_language,
        language: Some("es"),
        layout: Some("es-qwerty"),
        error_mode,
        text,
        duration_ms: 1_000.0,
        gross_wpm: 30.0,
        net_wpm: 30.0,
        accuracy: 1.0,
        keystrokes: records.len(),
        bpm: None,
        records,
        metronome_offsets_ms: &[],
    }
}

fn save(store: &mut Store, session: &NewSession) -> i64 {
    store.save_session(session).unwrap()
}

/// Replays `events` as the app does and stores the result (records and duration from the replay).
fn save_typed(
    store: &mut Store,
    profile_id: i64,
    kind: Kind,
    text: &str,
    error_mode: ErrorMode,
    indent: IndentMode,
    events: &[InputEvent],
) -> i64 {
    let result = replay_with(text, error_mode, indent, events);
    assert!(result.completed, "test typing must complete the text");
    let stats = &result.stats;
    let session = NewSession {
        duration_ms: stats.duration_ms,
        gross_wpm: stats.gross_wpm,
        net_wpm: stats.net_wpm,
        ..new_session(profile_id, kind, text, error_mode, &result.records)
    };
    save(store, &session)
}

/// A session of `text` typed correctly, 100 ms per char, with the given net PPM.
fn save_with_wpm(
    store: &mut Store,
    profile_id: i64,
    kind: Kind,
    text: &str,
    net_wpm: f64,
    started_at_ms: i64,
) -> i64 {
    let events: Vec<_> = text
        .chars()
        .enumerate()
        .map(|(i, c)| ch(c, i as f64 * 100.0))
        .collect();
    let result = replay_with(text, ErrorMode::AllowContinue, IndentMode::Manual, &events);
    let mode = ErrorMode::AllowContinue;
    let session = NewSession {
        started_at_ms,
        duration_ms: result.stats.duration_ms,
        gross_wpm: net_wpm,
        net_wpm,
        ..new_session(profile_id, kind, text, mode, &result.records)
    };
    save(store, &session)
}

fn memory() -> (Store, i64) {
    let store = Store::open_in_memory().unwrap();
    let id = store.default_profile().unwrap().id;
    (store, id)
}

fn ghost_id(store: &Store, profile_id: i64, kind: Kind, text: &str) -> Option<i64> {
    store
        .ghost_for_text(profile_id, text, kind.mode, kind.code_language)
        .unwrap()
        .map(|g| g.session_id)
}

fn times(store: &Store, profile_id: i64, kind: Kind, text: &str) -> Vec<Option<f64>> {
    store
        .ghost_for_text(profile_id, text, kind.mode, kind.code_language)
        .unwrap()
        .expect("a ghost")
        .times
}

// ---- S1: which ghost ----

#[test]
fn no_session_with_the_text_means_no_ghost() {
    let (mut store, me) = memory();
    assert_eq!(
        store.ghost_for_text(me, "hola", "prose", None).unwrap(),
        None
    );
    save_with_wpm(&mut store, me, PROSE, "adiós", 40.0, 1);
    assert_eq!(
        store.ghost_for_text(me, "hola", "prose", None).unwrap(),
        None
    );
}

#[test]
fn only_a_session_with_exactly_the_text_is_the_ghost() {
    let (mut store, me) = memory();
    save_with_wpm(&mut store, me, PROSE, "hola ", 90.0, 1);
    save_with_wpm(&mut store, me, PROSE, "Hola", 80.0, 2);
    save_with_wpm(&mut store, me, PROSE, "hol", 70.0, 3);
    assert_eq!(ghost_id(&store, me, PROSE, "hola"), None);
    let exact = save_with_wpm(&mut store, me, PROSE, "hola", 30.0, 4);
    assert_eq!(ghost_id(&store, me, PROSE, "hola"), Some(exact));
}

#[test]
fn the_ghost_is_the_session_with_the_highest_net_wpm_and_carries_it() {
    let (mut store, me) = memory();
    save_with_wpm(&mut store, me, PROSE, "hola", 40.0, 1);
    let best = save_with_wpm(&mut store, me, PROSE, "hola", 52.0, 2);
    save_with_wpm(&mut store, me, PROSE, "hola", 45.0, 3);
    let ghost = store.ghost_for_text(me, "hola", "prose", None).unwrap();
    let ghost = ghost.expect("a ghost");
    assert_eq!((ghost.session_id, ghost.net_wpm), (best, 52.0));
    assert_eq!(
        ghost.times,
        vec![Some(0.0), Some(100.0), Some(200.0), Some(300.0)]
    );
}

#[test]
fn a_tie_on_net_wpm_goes_to_the_earliest_session() {
    let (mut store, me) = memory();
    // Stored first but started later: the start time decides, not the row id.
    save_with_wpm(&mut store, me, PROSE, "hola", 50.0, 2_000);
    let earliest = save_with_wpm(&mut store, me, PROSE, "hola", 50.0, 1_000);
    save_with_wpm(&mut store, me, PROSE, "hola", 50.0, 3_000);
    assert_eq!(ghost_id(&store, me, PROSE, "hola"), Some(earliest));
}

#[test]
fn the_ghost_has_the_same_mode_and_code_language() {
    let (mut store, me) = memory();
    let words = Kind {
        mode: "words",
        code_language: None,
    };
    let rust = Kind {
        mode: "code",
        code_language: Some("rust"),
    };
    let python = Kind {
        mode: "code",
        code_language: Some("python"),
    };
    let prose = save_with_wpm(&mut store, me, PROSE, "x = 1", 30.0, 1);
    let w = save_with_wpm(&mut store, me, words, "x = 1", 40.0, 2);
    let r = save_with_wpm(&mut store, me, rust, "x = 1", 50.0, 3);
    let p = save_with_wpm(&mut store, me, python, "x = 1", 60.0, 4);
    assert_eq!(ghost_id(&store, me, PROSE, "x = 1"), Some(prose));
    assert_eq!(ghost_id(&store, me, words, "x = 1"), Some(w));
    assert_eq!(ghost_id(&store, me, rust, "x = 1"), Some(r));
    assert_eq!(ghost_id(&store, me, python, "x = 1"), Some(p));
    let go = Kind {
        mode: "code",
        code_language: Some("go"),
    };
    assert_eq!(ghost_id(&store, me, go, "x = 1"), None);
}

#[test]
fn sessions_of_other_profiles_are_ignored() {
    let (mut store, me) = memory();
    let other = store.create_profile("Otro", "es", "es-qwerty").unwrap().id;
    save_with_wpm(&mut store, other, PROSE, "hola", 99.0, 1);
    assert_eq!(ghost_id(&store, me, PROSE, "hola"), None);
    let mine = save_with_wpm(&mut store, me, PROSE, "hola", 20.0, 2);
    assert_eq!(ghost_id(&store, me, PROSE, "hola"), Some(mine));
    assert_ne!(ghost_id(&store, other, PROSE, "hola"), Some(mine));
}

// ---- S2: ghost timeline ----

#[test]
fn allow_continue_passes_a_position_also_with_a_wrong_char() {
    let (mut store, me) = memory();
    let events = [ch('a', 1_000.0), ch('x', 1_100.0), ch('c', 1_250.0)];
    let mode = ErrorMode::AllowContinue;
    save_typed(
        &mut store,
        me,
        PROSE,
        "abc",
        mode,
        IndentMode::Manual,
        &events,
    );
    assert_eq!(
        times(&store, me, PROSE, "abc"),
        vec![Some(0.0), Some(100.0), Some(250.0)]
    );
}

#[test]
fn stop_on_error_passes_a_position_only_when_it_is_corrected() {
    let (mut store, me) = memory();
    let events = [
        ch('a', 0.0),
        ch('x', 100.0),
        ch('y', 150.0),
        ch('b', 180.0),
        ch('c', 300.0),
    ];
    let mode = ErrorMode::StopOnError;
    save_typed(
        &mut store,
        me,
        PROSE,
        "abc",
        mode,
        IndentMode::Manual,
        &events,
    );
    assert_eq!(
        times(&store, me, PROSE, "abc"),
        vec![Some(0.0), Some(180.0), Some(300.0)]
    );
}

#[test]
fn backspace_correct_keeps_the_first_pass_and_the_time_spent_deleting() {
    let (mut store, me) = memory();
    let events = [
        ch('a', 0.0),
        ch('x', 100.0),
        bs(200.0),
        ch('b', 260.0),
        ch('c', 400.0),
    ];
    let mode = ErrorMode::BackspaceCorrect;
    save_typed(
        &mut store,
        me,
        PROSE,
        "abc",
        mode,
        IndentMode::Manual,
        &events,
    );
    // The wrong `x` already moved past position 1; `c` at 400 ms (the deletion's 100 ms included).
    assert_eq!(
        times(&store, me, PROSE, "abc"),
        vec![Some(0.0), Some(100.0), Some(400.0)]
    );
}

#[test]
fn backspace_correct_retyping_several_positions_does_not_pass_them_again() {
    let (mut store, me) = memory();
    let events = [
        ch('a', 0.0),
        ch('x', 100.0),
        ch('y', 200.0),
        bs(300.0),
        bs(350.0),
        ch('b', 400.0),
        ch('c', 500.0),
        ch('d', 600.0),
    ];
    let mode = ErrorMode::BackspaceCorrect;
    save_typed(
        &mut store,
        me,
        PROSE,
        "abcd",
        mode,
        IndentMode::Manual,
        &events,
    );
    assert_eq!(
        times(&store, me, PROSE, "abcd"),
        vec![Some(0.0), Some(100.0), Some(200.0), Some(600.0)]
    );
}

#[test]
fn a_tab_over_spaces_passes_the_whole_run_at_once() {
    let (mut store, me) = memory();
    let rust = Kind {
        mode: "code",
        code_language: Some("rust"),
    };
    let text = "{\n    x\n}";
    let events = [
        ch('{', 0.0),
        ch('\n', 100.0),
        tab(200.0),
        ch('x', 300.0),
        ch('\n', 400.0),
        ch('}', 500.0),
    ];
    let mode = ErrorMode::AllowContinue;
    save_typed(
        &mut store,
        me,
        rust,
        text,
        mode,
        IndentMode::Manual,
        &events,
    );
    let t = |ms| Some(ms);
    assert_eq!(
        times(&store, me, rust, text),
        vec![
            t(0.0),
            t(100.0),
            t(200.0),
            t(200.0),
            t(200.0),
            t(200.0),
            t(300.0),
            t(400.0),
            t(500.0)
        ]
    );
}

#[test]
fn auto_indent_passes_the_filled_indent_with_the_newline() {
    for mode in [
        ErrorMode::AllowContinue,
        ErrorMode::StopOnError,
        ErrorMode::BackspaceCorrect,
    ] {
        let (mut store, me) = memory();
        let rust = Kind {
            mode: "code",
            code_language: Some("rust"),
        };
        let text = "{\n  x\n}";
        let events = [
            ch('{', 0.0),
            ch('\n', 100.0),
            ch('x', 300.0),
            ch('\n', 400.0),
            ch('}', 500.0),
        ];
        save_typed(&mut store, me, rust, text, mode, IndentMode::Auto, &events);
        let t = |ms| Some(ms);
        assert_eq!(
            times(&store, me, rust, text),
            vec![
                t(0.0),
                t(100.0),
                t(100.0),
                t(100.0),
                t(300.0),
                t(400.0),
                t(500.0)
            ],
            "{mode:?}"
        );
    }
}

#[test]
fn positions_the_record_never_reached_have_no_time() {
    let (mut store, me) = memory();
    let records = [
        KeystrokeRecord {
            index: 0,
            expected: 'a',
            typed: 'a',
            correct: true,
            ms_since_previous: None,
        },
        KeystrokeRecord {
            index: 1,
            expected: 'b',
            typed: 'b',
            correct: true,
            ms_since_previous: Some(120.0),
        },
    ];
    let mode = ErrorMode::AllowContinue;
    let partial = NewSession {
        duration_ms: 120.0,
        ..new_session(me, PROSE, "abcd", mode, &records)
    };
    save(&mut store, &partial);
    assert_eq!(
        times(&store, me, PROSE, "abcd"),
        vec![Some(0.0), Some(120.0), None, None]
    );
    let empty = NewSession {
        duration_ms: 0.0,
        ..new_session(me, PROSE, "xy", mode, &[])
    };
    save(&mut store, &empty);
    assert_eq!(times(&store, me, PROSE, "xy"), vec![None, None]);
}

#[test]
fn the_ghost_counts_chars_not_bytes() {
    let (mut store, me) = memory();
    let id = save_with_wpm(&mut store, me, PROSE, "año", 40.0, 1);
    let ghost: Ghost = store
        .ghost_for_text(me, "año", "prose", None)
        .unwrap()
        .unwrap();
    assert_eq!(ghost.session_id, id);
    assert_eq!(ghost.times, vec![Some(0.0), Some(100.0), Some(200.0)]);
}
