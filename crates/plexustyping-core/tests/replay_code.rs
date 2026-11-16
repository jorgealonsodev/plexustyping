//! Code-mode replay (newline, Tab, indentation modes) through the public API,
//! driven by the shared JSON fixtures in `fixtures/replay_code.json`.

use plexustyping_core::{
    ErrorMode, IndentMode, InputEvent, KeystrokeRecord, Stats, replay, replay_with,
};
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Case {
    name: String,
    text: String,
    mode: ErrorMode,
    indent: IndentMode,
    events: Vec<InputEvent>,
    expected: Expected,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Expected {
    completed: bool,
    cursor: usize,
    records: Vec<KeystrokeRecord>,
    stats: Stats,
}

fn assert_close(case: &str, field: &str, actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() < 1e-9,
        "{case}: {field} = {actual}, expected {expected}"
    );
}

#[test]
fn code_fixtures_match_replay_with_field_by_field() {
    let cases: Vec<Case> = serde_json::from_str(include_str!("fixtures/replay_code.json"))
        .expect("fixtures/replay_code.json must parse");
    assert!(cases.len() >= 12, "fixture set unexpectedly small");
    for case in &cases {
        let name = case.name.as_str();
        let result = replay_with(&case.text, case.mode, case.indent, &case.events);
        let exp = &case.expected;
        assert_eq!(result.completed, exp.completed, "{name}: completed");
        assert_eq!(result.cursor, exp.cursor, "{name}: cursor");
        assert_eq!(result.records.len(), exp.records.len(), "{name}: records");
        for (i, (got, want)) in result.records.iter().zip(&exp.records).enumerate() {
            let g = (got.index, got.expected, got.typed, got.correct);
            let w = (want.index, want.expected, want.typed, want.correct);
            assert_eq!(g, w, "{name}: records[{i}]");
            match (got.ms_since_previous, want.ms_since_previous) {
                (Some(g), Some(w)) => assert_close(name, &format!("records[{i}].ms"), g, w),
                (g, w) => assert_eq!(g, w, "{name}: records[{i}].msSincePrevious"),
            }
        }
        let (s, e) = (&result.stats, &exp.stats);
        let counts = |s: &Stats| {
            [
                s.keystrokes,
                s.char_keystrokes,
                s.correct_keystrokes,
                s.errors,
                s.uncorrected_errors,
                s.chars_typed,
            ]
        };
        assert_eq!(counts(s), counts(e), "{name}: stats counts");
        assert_close(name, "durationMs", s.duration_ms, e.duration_ms);
        assert_close(name, "accuracy", s.accuracy, e.accuracy);
        assert_close(name, "grossWpm", s.gross_wpm, e.gross_wpm);
        assert_close(name, "netWpm", s.net_wpm, e.net_wpm);
        assert_close(name, "kpm", s.kpm, e.kpm);
    }
}

#[test]
fn indent_mode_defaults_to_manual_and_serializes_kebab_case() {
    assert_eq!(IndentMode::default(), IndentMode::Manual);
    assert_eq!(
        serde_json::to_string(&IndentMode::Manual).unwrap(),
        "\"manual\""
    );
    assert_eq!(
        serde_json::to_string(&IndentMode::Auto).unwrap(),
        "\"auto\""
    );
}

#[test]
fn tab_event_json_shape() {
    let tab = InputEvent::Tab { t_ms: 3.5 };
    assert_eq!(
        serde_json::to_string(&tab).unwrap(),
        r#"{"kind":"tab","tMs":3.5}"#
    );
    let parsed: InputEvent = serde_json::from_str(r#"{"kind":"tab","tMs":3.5}"#).unwrap();
    assert_eq!(parsed, tab);
}

#[test]
fn replay_is_replay_with_manual_indent() {
    let events = [
        InputEvent::Char { ch: 'a', t_ms: 0.0 },
        InputEvent::Char {
            ch: '\n',
            t_ms: 10.0,
        },
        InputEvent::Tab { t_ms: 20.0 },
        InputEvent::Char {
            ch: 'b',
            t_ms: 30.0,
        },
    ];
    let text = "a\n    b";
    for mode in [
        ErrorMode::StopOnError,
        ErrorMode::AllowContinue,
        ErrorMode::BackspaceCorrect,
    ] {
        assert_eq!(
            replay(text, mode, &events),
            replay_with(text, mode, IndentMode::Manual, &events)
        );
    }
}
