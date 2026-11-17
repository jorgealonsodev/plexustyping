//! Code mode (phase 2): the practice mode switch, code text plans from the
//! code statistics model, the corpus snippet fallback and syntax tokens.

use std::ops::Range;

use plexustyping_core::adaptive::WeaknessModel;
use plexustyping_corpus::{CodeLanguage, CodeOptions, code_languages, generate_code};
use plexustyping_highlight::{Token, TokenClass, highlight};
use serde::Deserialize;

use crate::commands::{NextText, STATS_MODE};

/// Stats mode of every code session, whatever the language (S5).
pub const CODE_STATS_MODE: &str = "code";
/// Snippet lines when the user has not chosen; clamped to 5..=20 (S2).
pub const DEFAULT_MAX_LINES: usize = 12;
/// Shortest accepted AI snippet, in chars (about 5 short lines).
pub const CODE_MIN_CHARS: usize = 80;
/// Average chars per line used to size the AI snippet length.
pub const CHARS_PER_LINE: usize = 40;

/// What is practised: prose, or code of one language. In IPC it is
/// `"prose"` or `{ "kind": "code", "lang": "<id>" }`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(try_from = "RawPracticeMode")]
pub enum PracticeMode {
    #[default]
    Prose,
    Code {
        /// A [`code_languages`] id.
        lang: String,
    },
}

#[derive(Deserialize)]
#[serde(untagged)]
enum RawPracticeMode {
    Name(String),
    Tagged { kind: String, lang: String },
}

impl TryFrom<RawPracticeMode> for PracticeMode {
    type Error = String;

    fn try_from(raw: RawPracticeMode) -> Result<Self, String> {
        match raw {
            RawPracticeMode::Name(name) if name == "prose" => Ok(PracticeMode::Prose),
            RawPracticeMode::Tagged { kind, lang } if kind == "code" => PracticeMode::code(&lang),
            _ => Err("unknown practice mode".to_owned()),
        }
    }
}

impl PracticeMode {
    /// Code of `lang`, which must be a bundled language.
    pub fn code(lang: &str) -> Result<Self, String> {
        language(lang)?;
        Ok(PracticeMode::Code {
            lang: lang.to_owned(),
        })
    }

    /// Mode of the statistics model and of `sessions.mode`.
    pub fn stats_mode(&self) -> &'static str {
        match self {
            PracticeMode::Prose => STATS_MODE,
            PracticeMode::Code { .. } => CODE_STATS_MODE,
        }
    }

    /// Mode of `text_cache` rows: code texts are cached per language.
    pub fn cache_mode(&self) -> String {
        match self {
            PracticeMode::Prose => STATS_MODE.to_owned(),
            PracticeMode::Code { lang } => format!("{CODE_STATS_MODE}:{lang}"),
        }
    }

    /// The code language, `None` for prose.
    pub fn code_language(&self) -> Option<&str> {
        match self {
            PracticeMode::Prose => None,
            PracticeMode::Code { lang } => Some(lang),
        }
    }
}

/// The bundled language `id`.
pub fn language(id: &str) -> Result<&'static CodeLanguage, String> {
    code_languages()
        .iter()
        .find(|l| l.id == id)
        .ok_or_else(|| format!("unknown code language: {id}"))
}

/// Highlighter syntax of AI and cached texts of `lang` (HTML/CSS → HTML).
pub fn default_syntax(lang: &str) -> &str {
    match lang {
        "html-css" => "html",
        other => other,
    }
}

/// Chars an AI snippet may use: printable ASCII and the newline.
pub fn code_allowed_chars() -> String {
    (' '..='~').chain(['\n']).collect()
}

/// Accepted AI snippet length in chars for at most `max_lines` (5..=20) lines.
pub fn code_length(max_lines: usize) -> Range<usize> {
    CODE_MIN_CHARS..max_lines.clamp(5, 20) * CHARS_PER_LINE + 1
}

/// What the next code text practises, before generating it.
#[derive(Clone, Debug, PartialEq)]
pub struct CodePlan {
    pub lang: String,
    /// Weakest units of the code model, else `fallback`; no whitespace.
    pub targets: Vec<String>,
    /// Clamped to 5..=20.
    pub max_lines: usize,
}

/// The plan of a code text of `lang` from the code `model`.
pub fn plan_code(
    model: &WeaknessModel,
    lang: &str,
    target_count: usize,
    max_lines: usize,
    seed: u64,
    fallback: &[String],
) -> Result<CodePlan, String> {
    language(lang)?;
    let count = target_count.clamp(3, 5);
    let eligible = |t: &str| !t.is_empty() && t.chars().all(|c| c.is_ascii_graphic());
    // Ineligible units are dropped before selecting, so they never take a slot.
    let units = model.units().into_iter().filter(|(u, _)| eligible(u));
    let symbols = WeaknessModel::from_parts(
        units.collect(),
        model.confusions(),
        model.mean_interval_ms(),
    );
    let mut targets = symbols.select_targets(count, seed);
    if targets.is_empty() {
        targets = fallback.iter().filter(|t| eligible(t)).cloned().collect();
    }
    targets.truncate(count);
    Ok(CodePlan {
        lang: lang.to_owned(),
        targets,
        max_lines: max_lines.clamp(5, 20),
    })
}

/// A bundled snippet for `plan`; only the targets it contains are reported.
pub fn corpus_code(plan: &CodePlan, seed: u64) -> Result<NextText, String> {
    let snippet = generate_code(&CodeOptions {
        language: plan.lang.clone(),
        targets: plan.targets.clone(),
        max_lines: plan.max_lines,
        seed,
    })
    .ok_or_else(|| format!("unknown code language: {}", plan.lang))?;
    let mut targets = plan.targets.clone();
    targets.retain(|t| snippet.code.contains(t.as_str()));
    let text = NextText::corpus(snippet.code.to_owned(), targets);
    Ok(with_syntax(text, snippet.syntax))
}

/// `text` with its syntax and tokens.
pub fn with_syntax(mut text: NextText, syntax: &str) -> NextText {
    text.tokens = Some(tokens(syntax, &text.text));
    text.syntax = Some(syntax.to_owned());
    text
}

/// Contiguous tokens covering `code`; one plain token when `syntax` is unknown.
pub fn tokens(syntax: &str, code: &str) -> Vec<Token> {
    highlight(syntax, code).unwrap_or_else(|| {
        let end = code.chars().count();
        let plain = Token {
            start: 0,
            end,
            class: TokenClass::Plain,
        };
        if end == 0 { vec![] } else { vec![plain] }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::TextSource;
    use plexustyping_core::adaptive::UnitStat;

    fn model(units: &[(&str, f64)]) -> WeaknessModel {
        let stat = |ema_error| UnitStat {
            ema_error,
            samples: 5,
            ..UnitStat::default()
        };
        let units = units.iter().map(|&(u, e)| (u.to_owned(), stat(e)));
        WeaknessModel::from_parts(units.collect(), vec![], Some(150.0))
    }

    fn contiguous(tokens: &[Token], len: usize) {
        let mut at = 0;
        for t in tokens {
            assert_eq!(t.start, at, "{tokens:?}");
            assert!(t.end > t.start);
            at = t.end;
        }
        assert_eq!(at, len);
    }

    #[test]
    fn practice_mode_parses_prose_and_known_code_languages_only() {
        let parse = |v: serde_json::Value| serde_json::from_value::<PracticeMode>(v);
        assert_eq!(
            parse(serde_json::json!("prose")).unwrap(),
            PracticeMode::Prose
        );
        assert_eq!(
            parse(serde_json::json!({"kind": "code", "lang": "rust"})).unwrap(),
            PracticeMode::Code {
                lang: "rust".into()
            }
        );
        let err = parse(serde_json::json!({"kind": "code", "lang": "cobol"})).unwrap_err();
        assert!(
            err.to_string().contains("unknown code language: cobol"),
            "{err}"
        );
        assert!(parse(serde_json::json!("code")).is_err());
        assert!(parse(serde_json::json!({"kind": "prose", "lang": "rust"})).is_err());
    }

    #[test]
    fn modes_keep_prose_and_code_stats_and_caches_apart() {
        let rust = PracticeMode::code("rust").unwrap();
        assert_eq!(
            (PracticeMode::Prose.stats_mode(), rust.stats_mode()),
            ("prose", "code")
        );
        assert_eq!(PracticeMode::Prose.cache_mode(), "prose");
        assert_eq!(rust.cache_mode(), "code:rust");
        assert_eq!(rust.code_language(), Some("rust"));
        assert_eq!(PracticeMode::Prose.code_language(), None);
    }

    #[test]
    fn ai_snippets_use_printable_ascii_and_newlines_sized_by_lines() {
        let allowed = code_allowed_chars();
        assert_eq!(allowed.chars().count(), 96);
        assert!(allowed.contains('\n') && allowed.contains('{') && allowed.contains('~'));
        assert!(!allowed.contains('\t') && !allowed.contains('ñ'));
        assert_eq!(code_length(12), 80..481);
        assert_eq!(code_length(1), 80..201);
        assert_eq!(code_length(99), 80..801);
        assert_eq!(default_syntax("html-css"), "html");
        assert_eq!(default_syntax("python"), "python");
    }

    #[test]
    fn plan_code_targets_the_weakest_symbols_and_drops_whitespace() {
        let m = model(&[
            ("{", 0.9),
            (" ", 0.95),
            ("\n", 0.85),
            ("=>", 0.8),
            (";", 0.7),
        ]);
        // Whitespace is dropped before selecting, so the plan keeps 3 targets.
        let plan = plan_code(&m, "rust", 3, 12, 1, &[]).unwrap();
        assert_eq!(
            plan.targets,
            vec!["{".to_owned(), "=>".to_owned(), ";".to_owned()]
        );
        assert_eq!(plan.max_lines, 12);
        let fallback = vec![";".to_owned(), "ñ".to_owned()];
        let empty = plan_code(&WeaknessModel::new(), "python", 9, 50, 1, &fallback).unwrap();
        assert_eq!((empty.targets, empty.max_lines), (vec![";".to_owned()], 20));
        let blank = model(&[(" ", 0.95), ("\n", 0.85), ("ñ", 0.8)]);
        let all_filtered = plan_code(&blank, "rust", 3, 12, 1, &fallback).unwrap();
        assert_eq!(all_filtered.targets, vec![";".to_owned()]);
        assert_eq!(
            plan_code(&m, "cobol", 3, 12, 1, &[]),
            Err("unknown code language: cobol".to_owned())
        );
    }

    #[test]
    fn corpus_code_returns_a_snippet_with_its_syntax_and_contiguous_tokens() {
        let plan = CodePlan {
            lang: "html-css".into(),
            targets: vec!["<".into(), "@@@".into()],
            max_lines: 20,
        };
        let text = corpus_code(&plan, 3).unwrap();
        let snippets = plexustyping_corpus::code_snippets("html-css").unwrap();
        let snippet = snippets.iter().find(|s| s.code == text.text).unwrap();
        assert_eq!(text.syntax.as_deref(), Some(snippet.syntax));
        assert_eq!(text.source, TextSource::Corpus);
        assert!(!text.targets.contains(&"@@@".to_owned()));
        let tokens = text.tokens.as_deref().unwrap();
        contiguous(tokens, text.text.chars().count());
        assert!(tokens.iter().any(|t| t.class != TokenClass::Plain));
        assert_eq!(corpus_code(&plan, 3), Ok(text));
    }

    #[test]
    fn tokens_fall_back_to_one_plain_run_for_an_unknown_syntax() {
        assert_eq!(
            tokens("cobol", "añ b"),
            vec![Token {
                start: 0,
                end: 4,
                class: TokenClass::Plain
            }]
        );
        assert_eq!(tokens("cobol", ""), vec![]);
        let rust = tokens("rust", "fn main() {\n    let x = 1;\n}");
        contiguous(&rust, 28);
    }
}
