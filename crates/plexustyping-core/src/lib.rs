//! Typing session engine: input replay, error modes, per-keystroke records and stats.

use serde::{Deserialize, Serialize};

pub mod adaptive;

/// How the session reacts to a wrong keystroke.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ErrorMode {
    /// Stay on the same character until the right key is pressed.
    StopOnError,
    /// Advance on every character, right or wrong (default).
    #[default]
    AllowContinue,
    /// Advance on every character; Backspace reopens the previous position.
    BackspaceCorrect,
}

/// One raw input event captured by the frontend, timestamped in milliseconds.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum InputEvent {
    /// A typed character.
    Char {
        /// The character produced by the key press.
        ch: char,
        /// Event timestamp in milliseconds.
        t_ms: f64,
    },
    /// A Backspace key press.
    Backspace {
        /// Event timestamp in milliseconds.
        t_ms: f64,
    },
}

/// Measurement of one applied character keystroke.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeystrokeRecord {
    /// Position in the text (in chars) where the key was typed.
    pub index: usize,
    /// The character the text expected at `index`.
    pub expected: char,
    /// The character actually typed.
    pub typed: char,
    /// Whether `typed == expected`.
    pub correct: bool,
    /// Time since the previous applied event; `None` for the first one.
    pub ms_since_previous: Option<f64>,
}

/// Aggregate speed and accuracy figures for a session.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Stats {
    /// Last applied timestamp minus first applied timestamp.
    pub duration_ms: f64,
    /// Applied events (characters and backspaces).
    pub keystrokes: usize,
    /// Applied character events (one per record).
    pub char_keystrokes: usize,
    /// Correct records.
    pub correct_keystrokes: usize,
    /// Wrong records.
    pub errors: usize,
    /// Positions before the cursor whose final typed char is wrong.
    pub uncorrected_errors: usize,
    /// Final cursor position.
    pub chars_typed: usize,
    /// `correct_keystrokes / char_keystrokes` in `0..=1`; `1.0` without records.
    pub accuracy: f64,
    /// Gross words per minute (a word is 5 chars).
    pub gross_wpm: f64,
    /// Net words per minute, discounting uncorrected errors; never negative.
    pub net_wpm: f64,
    /// Keystrokes per minute.
    pub kpm: f64,
}

/// Outcome of replaying an event log against a text.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionResult {
    /// One record per applied character event, in order.
    pub records: Vec<KeystrokeRecord>,
    /// Aggregate figures.
    pub stats: Stats,
    /// Whether the cursor reached the end of the text.
    pub completed: bool,
    /// Final cursor position, in chars.
    pub cursor: usize,
}

/// Replays `events` against `text` under `mode` and returns records and stats.
///
/// The text is treated as a sequence of `char`s. Events that are not applied
/// (ignored backspaces, anything after the end of the text) leave no trace:
/// no record, no timing and no keystroke count.
pub fn replay(text: &str, mode: ErrorMode, events: &[InputEvent]) -> SessionResult {
    let expected: Vec<char> = text.chars().collect();
    // Final typed char per position; reopened positions are overwritten.
    let mut typed_at: Vec<Option<char>> = vec![None; expected.len()];
    let mut records = Vec::new();
    let mut cursor = 0;
    let mut keystrokes = 0;
    let mut first_t: Option<f64> = None;
    let mut last_t: Option<f64> = None;

    for event in events {
        if cursor >= expected.len() {
            break;
        }
        let t_ms = match *event {
            InputEvent::Char { ch, t_ms } => {
                let want = expected[cursor];
                let correct = ch == want;
                records.push(KeystrokeRecord {
                    index: cursor,
                    expected: want,
                    typed: ch,
                    correct,
                    ms_since_previous: last_t.map(|prev| t_ms - prev),
                });
                if correct || mode != ErrorMode::StopOnError {
                    typed_at[cursor] = Some(ch);
                    cursor += 1;
                }
                t_ms
            }
            InputEvent::Backspace { t_ms } => {
                if mode != ErrorMode::BackspaceCorrect || cursor == 0 {
                    continue;
                }
                cursor -= 1;
                t_ms
            }
        };
        keystrokes += 1;
        first_t.get_or_insert(t_ms);
        last_t = Some(t_ms);
    }

    let uncorrected_errors = (0..cursor)
        .filter(|&i| typed_at[i] != Some(expected[i]))
        .count();
    let stats = compute_stats(
        &records,
        keystrokes,
        cursor,
        uncorrected_errors,
        first_t,
        last_t,
    );
    SessionResult {
        records,
        stats,
        completed: cursor == expected.len(),
        cursor,
    }
}

fn compute_stats(
    records: &[KeystrokeRecord],
    keystrokes: usize,
    chars_typed: usize,
    uncorrected_errors: usize,
    first_t: Option<f64>,
    last_t: Option<f64>,
) -> Stats {
    let correct_keystrokes = records.iter().filter(|r| r.correct).count();
    let char_keystrokes = records.len();
    let duration_ms = match (first_t, last_t) {
        (Some(first), Some(last)) => last - first,
        _ => 0.0,
    };
    let accuracy = if char_keystrokes == 0 {
        1.0
    } else {
        correct_keystrokes as f64 / char_keystrokes as f64
    };
    let (gross_wpm, net_wpm, kpm) = if duration_ms == 0.0 {
        (0.0, 0.0, 0.0)
    } else {
        let minutes = duration_ms / 60_000.0;
        let words = chars_typed as f64 / 5.0;
        (
            words / minutes,
            ((words - uncorrected_errors as f64) / minutes).max(0.0),
            keystrokes as f64 / minutes,
        )
    };
    Stats {
        duration_ms,
        keystrokes,
        char_keystrokes,
        correct_keystrokes,
        errors: char_keystrokes - correct_keystrokes,
        uncorrected_errors,
        chars_typed,
        accuracy,
        gross_wpm,
        net_wpm,
        kpm,
    }
}

/// Distinct `expected` chars of wrong records, sorted by number of wrong
/// records (descending), ties broken by first occurrence. Spaces are included.
pub fn failed_chars(result: &SessionResult) -> Vec<char> {
    // (char, wrong count) in first-occurrence order.
    let mut counts: Vec<(char, usize)> = Vec::new();
    for record in result.records.iter().filter(|r| !r.correct) {
        match counts.iter_mut().find(|(c, _)| *c == record.expected) {
            Some((_, n)) => *n += 1,
            None => counts.push((record.expected, 1)),
        }
    }
    // Stable sort keeps first-occurrence order among equal counts.
    counts.sort_by_key(|&(_, n)| std::cmp::Reverse(n));
    counts.into_iter().map(|(c, _)| c).collect()
}
