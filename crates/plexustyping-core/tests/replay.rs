//! Replay behavior through the public API, driven by the shared JSON fixtures.

use plexustyping_core::{ErrorMode, InputEvent, KeystrokeRecord, Stats, failed_chars, replay};
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Case {
    name: String,
    text: String,
    mode: ErrorMode,
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

fn load_cases() -> Vec<Case> {
    let raw = include_str!("fixtures/replay.json");
    serde_json::from_str(raw).expect("fixtures/replay.json must parse")
}

fn assert_close(case: &str, field: &str, actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() < 1e-9,
        "{case}: {field} = {actual}, expected {expected}"
    );
}

#[test]
fn fixtures_match_replay_field_by_field() {
    let cases = load_cases();
    assert!(cases.len() >= 15, "fixture set unexpectedly small");
    for case in &cases {
        let name = case.name.as_str();
        let result = replay(&case.text, case.mode, &case.events);
        let exp = &case.expected;
        assert_eq!(result.completed, exp.completed, "{name}: completed");
        assert_eq!(result.cursor, exp.cursor, "{name}: cursor");
        assert_eq!(
            result.records.len(),
            exp.records.len(),
            "{name}: record count"
        );
        for (i, (got, want)) in result.records.iter().zip(&exp.records).enumerate() {
            assert_eq!(got.index, want.index, "{name}: records[{i}].index");
            assert_eq!(got.expected, want.expected, "{name}: records[{i}].expected");
            assert_eq!(got.typed, want.typed, "{name}: records[{i}].typed");
            assert_eq!(got.correct, want.correct, "{name}: records[{i}].correct");
            match (got.ms_since_previous, want.ms_since_previous) {
                (Some(g), Some(w)) => assert_close(name, &format!("records[{i}].ms"), g, w),
                (g, w) => assert_eq!(g, w, "{name}: records[{i}].msSincePrevious"),
            }
        }
        let (s, e) = (&result.stats, &exp.stats);
        assert_eq!(s.keystrokes, e.keystrokes, "{name}: keystrokes");
        assert_eq!(
            s.char_keystrokes, e.char_keystrokes,
            "{name}: charKeystrokes"
        );
        assert_eq!(
            s.correct_keystrokes, e.correct_keystrokes,
            "{name}: correctKeystrokes"
        );
        assert_eq!(s.errors, e.errors, "{name}: errors");
        assert_eq!(
            s.uncorrected_errors, e.uncorrected_errors,
            "{name}: uncorrectedErrors"
        );
        assert_eq!(s.chars_typed, e.chars_typed, "{name}: charsTyped");
        assert_close(name, "durationMs", s.duration_ms, e.duration_ms);
        assert_close(name, "accuracy", s.accuracy, e.accuracy);
        assert_close(name, "grossWpm", s.gross_wpm, e.gross_wpm);
        assert_close(name, "netWpm", s.net_wpm, e.net_wpm);
        assert_close(name, "kpm", s.kpm, e.kpm);
    }
}

#[test]
fn default_error_mode_is_allow_continue() {
    assert_eq!(ErrorMode::default(), ErrorMode::AllowContinue);
}

#[test]
fn json_shapes_match_the_contract() {
    let modes = [
        (ErrorMode::StopOnError, "\"stop-on-error\""),
        (ErrorMode::AllowContinue, "\"allow-continue\""),
        (ErrorMode::BackspaceCorrect, "\"backspace-correct\""),
    ];
    for (mode, json) in modes {
        assert_eq!(serde_json::to_string(&mode).unwrap(), json);
    }
    let ch = InputEvent::Char {
        ch: 'a',
        t_ms: 12.5,
    };
    assert_eq!(
        serde_json::to_string(&ch).unwrap(),
        r#"{"kind":"char","ch":"a","tMs":12.5}"#
    );
    let bs = InputEvent::Backspace { t_ms: 20.0 };
    assert_eq!(
        serde_json::to_string(&bs).unwrap(),
        r#"{"kind":"backspace","tMs":20.0}"#
    );
    let result = replay("a", ErrorMode::AllowContinue, &[ch]);
    let value = serde_json::to_value(&result).unwrap();
    assert_eq!(
        value["records"][0]["msSincePrevious"],
        serde_json::Value::Null
    );
    for key in [
        "durationMs",
        "charKeystrokes",
        "uncorrectedErrors",
        "grossWpm",
        "netWpm",
    ] {
        assert!(value["stats"].get(key).is_some(), "stats.{key} missing");
    }
}

fn chars(t: &[(char, f64)]) -> Vec<InputEvent> {
    t.iter()
        .map(|&(ch, t_ms)| InputEvent::Char { ch, t_ms })
        .collect()
}

#[test]
fn failed_chars_sorted_by_count_then_first_occurrence_including_spaces() {
    // Every key wrong: 'b' x1 (first), 'a' x2, ' ' x1.
    let events = chars(&[('z', 0.0), ('z', 1.0), ('z', 2.0), ('z', 3.0)]);
    let result = replay("ba a", ErrorMode::AllowContinue, &events);
    assert_eq!(failed_chars(&result), vec!['a', 'b', ' ']);
}

#[test]
fn failed_chars_counts_repeated_misses_in_stop_on_error() {
    let events = chars(&[
        ('x', 0.0),
        ('b', 1.0),
        ('a', 2.0),
        ('y', 3.0),
        ('z', 4.0),
        ('w', 5.0),
    ]);
    let result = replay("ab", ErrorMode::StopOnError, &events);
    // 'a' missed twice, 'b' missed three times.
    assert_eq!(failed_chars(&result), vec!['b', 'a']);
}

#[test]
fn failed_chars_empty_when_all_correct() {
    let result = replay(
        "ab",
        ErrorMode::AllowContinue,
        &chars(&[('a', 0.0), ('b', 1.0)]),
    );
    assert!(failed_chars(&result).is_empty());
}
