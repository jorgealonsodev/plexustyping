//! Command logic behind the Tauri IPC handlers, testable without a Tauri runtime.

use plexustyping_core::{ErrorMode, InputEvent, SessionResult, failed_chars, replay};
use plexustyping_corpus::{GenerateOptions, corpus};
use serde::{Deserialize, Serialize};

/// Wanted length of a generated practice text, in chars (end exclusive).
pub const NEXT_TEXT_LENGTH: std::ops::Range<usize> = 250..301;

/// Authoritative replay of a finished session plus the chars to practice next.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScoredSession {
    /// Records and stats from `plexustyping-core`.
    pub result: SessionResult,
    /// Expected chars of wrong keystrokes, most failed first.
    pub failed_chars: Vec<char>,
}

fn unknown_language(language: &str) -> String {
    format!("unknown language: {language}")
}

/// The fixed diagnostic text of `language`.
pub fn diagnostic_text(language: &str) -> Result<String, String> {
    corpus(language)
        .map(|c| c.diagnostic().to_owned())
        .ok_or_else(|| unknown_language(language))
}

/// Calls `generate` with `targets`; if that yields nothing, once more without targets.
pub fn generate_with_retry(
    targets: &[char],
    generate: impl Fn(&[char]) -> String,
) -> Result<String, String> {
    let text = generate(targets);
    if !text.is_empty() {
        return Ok(text);
    }
    let text = generate(&[]);
    if text.is_empty() {
        Err("could not generate a practice text".to_owned())
    } else {
        Ok(text)
    }
}

/// A practice text in `language`, typeable on `layout`, that targets `targets`.
pub fn next_text(
    language: &str,
    layout: &str,
    targets: &[char],
    seed: u64,
) -> Result<String, String> {
    let corpus = corpus(language).ok_or_else(|| unknown_language(language))?;
    let layout =
        plexustyping_layouts::builtin(layout).ok_or_else(|| format!("unknown layout: {layout}"))?;
    let allowed = layout.allowed_chars();
    generate_with_retry(targets, |targets| {
        corpus.generate(&GenerateOptions {
            targets: targets.to_vec(),
            length: NEXT_TEXT_LENGTH,
            allowed: Some(allowed.clone()),
            seed,
        })
    })
}

/// Replays the raw event log of a session and lists its failed chars.
pub fn score_session(text: &str, mode: ErrorMode, events: &[InputEvent]) -> ScoredSession {
    let result = replay(text, mode, events);
    let failed_chars = failed_chars(&result);
    ScoredSession {
        result,
        failed_chars,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

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
    fn next_text_targets_failed_chars_within_length_and_layout() {
        let text = next_text("es", "es-qwerty", &['ñ'], 7).expect("text");
        let allowed = plexustyping_layouts::builtin("es-qwerty")
            .unwrap()
            .allowed_chars();
        let len = text.chars().count();
        assert!(NEXT_TEXT_LENGTH.contains(&len), "length {len}");
        assert!(text.chars().all(|c| c == '.' || allowed.contains(&c)));
        assert!(text.contains('ñ'), "targets ñ: {text}");
        assert_eq!(next_text("es", "es-qwerty", &['ñ'], 7), Ok(text));
    }

    #[test]
    fn next_text_rejects_unknown_language_and_layout() {
        assert_eq!(
            next_text("xx", "es-qwerty", &[], 1),
            Err("unknown language: xx".to_owned())
        );
        assert_eq!(
            next_text("es", "dvorak-xx", &[], 1),
            Err("unknown layout: dvorak-xx".to_owned())
        );
    }

    #[test]
    fn generate_with_retry_keeps_a_non_empty_first_result() {
        let calls = RefCell::new(Vec::new());
        let text = generate_with_retry(&['q'], |t| {
            calls.borrow_mut().push(t.to_vec());
            "qué".to_owned()
        });
        assert_eq!(text, Ok("qué".to_owned()));
        assert_eq!(calls.into_inner(), vec![vec!['q']]);
    }

    #[test]
    fn generate_with_retry_retries_once_without_targets() {
        let calls = RefCell::new(Vec::new());
        let text = generate_with_retry(&['q'], |t| {
            calls.borrow_mut().push(t.to_vec());
            if t.is_empty() { "hola." } else { "" }.to_owned()
        });
        assert_eq!(text, Ok("hola.".to_owned()));
        assert_eq!(calls.into_inner(), vec![vec!['q'], vec![]]);
    }

    #[test]
    fn generate_with_retry_fails_when_both_attempts_are_empty() {
        let calls = RefCell::new(0);
        let text = generate_with_retry(&['q'], |_| {
            *calls.borrow_mut() += 1;
            String::new()
        });
        assert_eq!(text, Err("could not generate a practice text".to_owned()));
        assert_eq!(calls.into_inner(), 2);
    }

    #[test]
    fn score_session_replays_and_lists_failed_chars_as_camel_case_json() {
        let events: Vec<InputEvent> = serde_json::from_str(
            r#"[{"kind":"char","ch":"h","tMs":0},{"kind":"char","ch":"x","tMs":100},
                {"kind":"char","ch":"l","tMs":200},{"kind":"char","ch":"a","tMs":300}]"#,
        )
        .unwrap();
        let scored = score_session("hola", ErrorMode::AllowContinue, &events);
        assert_eq!(
            scored.result,
            replay("hola", ErrorMode::AllowContinue, &events)
        );
        assert!(scored.result.completed);
        assert_eq!(scored.failed_chars, vec!['o']);
        let json = serde_json::to_value(&scored).unwrap();
        assert_eq!(json["failedChars"], serde_json::json!(["o"]));
        assert_eq!(json["result"]["stats"]["errors"], 1);
    }
}
