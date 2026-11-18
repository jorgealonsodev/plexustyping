//! Progressive unlock support (phase 3, S3, S7): keystroke latencies per char
//! and settings removal, through the public API only.

use plexustyping_core::{ErrorMode, KeystrokeRecord};
use plexustyping_store::{NewSession, Store};

fn record(expected: char, typed: char, ms: Option<f64>) -> KeystrokeRecord {
    KeystrokeRecord {
        index: 0,
        expected,
        typed,
        correct: expected == typed,
        ms_since_previous: ms,
    }
}

fn save(store: &mut Store, profile_id: i64, mode: &str, at: i64, records: &[KeystrokeRecord]) {
    store
        .save_session(&NewSession {
            profile_id,
            started_at_ms: at,
            mode,
            code_language: None,
            language: Some("es"),
            layout: Some("es-qwerty"),
            error_mode: ErrorMode::AllowContinue,
            text: "f",
            duration_ms: 1000.0,
            gross_wpm: 10.0,
            net_wpm: 10.0,
            accuracy: 1.0,
            keystrokes: records.len(),
            bpm: None,
            records,
            metronome_offsets_ms: &[],
        })
        .unwrap();
}

#[test]
fn correct_latencies_are_the_newest_correct_intervals_of_the_char_in_the_mode() {
    let mut store = Store::open_in_memory().unwrap();
    let me = store.default_profile().unwrap().id;
    let other = store.create_profile("Otra", "es", "es-qwerty").unwrap().id;
    // Older session, then a newer one: newest first, typing order reversed.
    save(
        &mut store,
        me,
        "prose",
        1,
        &[record('f', 'f', Some(100.0)), record('f', 'f', Some(110.0))],
    );
    save(
        &mut store,
        me,
        "prose",
        2,
        &[
            record('f', 'f', None),
            record('f', 'g', Some(999.0)),
            record('j', 'j', Some(500.0)),
            record('f', 'f', Some(120.0)),
            record('f', 'f', Some(130.0)),
        ],
    );
    save(&mut store, me, "words", 3, &[record('f', 'f', Some(1.0))]);
    save(
        &mut store,
        other,
        "prose",
        4,
        &[record('f', 'f', Some(2.0))],
    );
    assert_eq!(
        store.correct_latencies(me, "prose", 'f', 200).unwrap(),
        [130.0, 120.0, 110.0, 100.0]
    );
    assert_eq!(
        store.correct_latencies(me, "prose", 'f', 3).unwrap(),
        [130.0, 120.0, 110.0]
    );
    assert_eq!(store.correct_latencies(me, "words", 'f', 9).unwrap(), [1.0]);
    assert_eq!(
        store.correct_latencies(me, "prose", 'j', 9).unwrap(),
        [500.0]
    );
    assert!(
        store
            .correct_latencies(me, "prose", 'k', 9)
            .unwrap()
            .is_empty()
    );
}

#[test]
fn delete_settings_under_removes_the_key_and_its_children_only() {
    let store = Store::open_in_memory().unwrap();
    for key in [
        "unlock:1",
        "unlock:1:es:es-qwerty",
        "unlock:12",
        "unlock:12:es:es-qwerty",
        "unlock:1x",
        "ai",
    ] {
        store.set_setting(key, "v").unwrap();
    }
    store.delete_settings_under("unlock:1").unwrap();
    for gone in ["unlock:1", "unlock:1:es:es-qwerty"] {
        assert_eq!(store.get_setting(gone).unwrap(), None, "{gone}");
    }
    for kept in ["unlock:12", "unlock:12:es:es-qwerty", "unlock:1x", "ai"] {
        assert_eq!(
            store.get_setting(kept).unwrap().as_deref(),
            Some("v"),
            "{kept}"
        );
    }
}
