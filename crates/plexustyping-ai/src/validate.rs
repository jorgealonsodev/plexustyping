//! Checks a generated text against its request (PRD §4.6).

use serde::{Deserialize, Serialize};
use unicode_normalization::UnicodeNormalization;

use crate::prompt::clamped_density;
use crate::{GenerationRequest, Mode};

/// Normal frequency assumed for any target, per character of text. A target
/// needs `max(1, ceil(density × BASELINE_PER_CHAR × len))` occurrences.
pub const BASELINE_PER_CHAR: f64 = 0.02;

/// A reason why a generated text was rejected.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum ValidationIssue {
    /// The length in characters is outside the requested range.
    Length {
        /// Length of the normalized text.
        len: usize,
    },
    /// A character the layout cannot type.
    CharNotAllowed {
        /// The character.
        ch: char,
    },
    /// Markdown formatting (`**`, a heading, a code fence or a list item).
    Markdown,
    /// A target appears fewer times than the density asks for.
    LowDensity {
        /// The target.
        target: String,
        /// Case-insensitive occurrences found.
        found: usize,
        /// Occurrences needed.
        needed: usize,
    },
    /// Nothing left after normalization.
    Empty,
}

/// Normalizes `text` and checks it against `req`; returns the normalized text
/// or every issue found.
///
/// Normalization: NFC; typographic quotes, dashes, ellipsis and non-breaking
/// spaces become plain characters; whitespace collapses to single spaces and
/// the ends are trimmed. In [`Mode::Code`] line breaks and indentation are
/// kept (only trailing spaces and blank edge lines go), newlines are allowed
/// and only code fences count as Markdown.
///
/// Issues, in this order: [`ValidationIssue::Empty`] alone for an empty text;
/// otherwise `Length`, each distinct `CharNotAllowed` (space always allowed),
/// `Markdown`, and `LowDensity` per target.
pub fn validate(text: &str, req: &GenerationRequest) -> Result<String, Vec<ValidationIssue>> {
    let code = matches!(req.mode, Mode::Code { .. });
    let plain = plain_chars(text);
    let normalized = if code {
        let lines: Vec<&str> = plain.lines().map(str::trim_end).collect();
        let start = lines.iter().position(|l| !l.is_empty());
        let end = lines.iter().rposition(|l| !l.is_empty());
        match (start, end) {
            (Some(s), Some(e)) => lines[s..=e].join("\n"),
            _ => String::new(),
        }
    } else {
        plain.split_whitespace().collect::<Vec<_>>().join(" ")
    };
    if normalized.is_empty() {
        return Err(vec![ValidationIssue::Empty]);
    }

    let mut issues = Vec::new();
    let len = normalized.chars().count();
    if !req.length.contains(&len) {
        issues.push(ValidationIssue::Length { len });
    }

    let allowed: Vec<char> = req.allowed_chars.nfc().collect();
    let mut rejected: Vec<char> = Vec::new();
    for ch in normalized.chars() {
        let ok = ch == ' ' || (code && ch == '\n') || allowed.contains(&ch);
        if !ok && !rejected.contains(&ch) {
            rejected.push(ch);
        }
    }
    issues.extend(
        rejected
            .into_iter()
            .map(|ch| ValidationIssue::CharNotAllowed { ch }),
    );

    if has_markdown(&plain, code) {
        issues.push(ValidationIssue::Markdown);
    }

    let density = f64::from(clamped_density(req.density));
    let needed = ((density * BASELINE_PER_CHAR * len as f64).ceil() as usize).max(1);
    let lower = normalized.to_lowercase();
    for target in req.targets.iter().filter(|t| !t.is_empty()) {
        let needle: String = target.nfc().collect::<String>().to_lowercase();
        let found = lower.matches(needle.as_str()).count();
        if found < needed {
            issues.push(ValidationIssue::LowDensity {
                target: target.clone(),
                found,
                needed,
            });
        }
    }

    if issues.is_empty() {
        Ok(normalized)
    } else {
        Err(issues)
    }
}

/// NFC plus plain replacements for typographic characters.
fn plain_chars(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.nfc() {
        match ch {
            '\u{2018}' | '\u{2019}' | '\u{201A}' | '\u{201B}' | '\u{2032}' => out.push('\''),
            '\u{201C}' | '\u{201D}' | '\u{201E}' | '\u{201F}' | '\u{2033}' => out.push('"'),
            '\u{2010}'..='\u{2015}' | '\u{2212}' => out.push('-'),
            '\u{2026}' => out.push_str("..."),
            '\u{00A0}' | '\u{2007}' | '\u{202F}' => out.push(' '),
            '\r' => {}
            other => out.push(other),
        }
    }
    out
}

/// `**`, a code fence, or a line starting with `#`, `- ` or `* ` (prose); only
/// code fences in code.
fn has_markdown(text: &str, code: bool) -> bool {
    if text.contains("```") {
        return true;
    }
    if code {
        return false;
    }
    text.contains("**")
        || text
            .lines()
            .map(str::trim_start)
            .any(|line| line.starts_with('#') || line.starts_with("- ") || line.starts_with("* "))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Mode;

    fn req() -> GenerationRequest {
        GenerationRequest {
            language: "es".into(),
            mode: Mode::Prose,
            targets: vec!["z".into()],
            density: 2.0,
            length: 5..60,
            allowed_chars: "abcdefghijklmnopqrstuvwxyzñZ.,'\"-".into(),
            topic: None,
        }
    }

    #[test]
    fn normalizes_quotes_dashes_ellipsis_nbsp_and_whitespace() {
        let raw =
            "  \u{201C}zaza\u{201D}\u{00A0}\u{2018}z\u{2019} \u{2014} zo\u{2026}\n\n ze\u{2013}z ";
        assert_eq!(validate(raw, &req()).unwrap(), "\"zaza\" 'z' - zo... ze-z");
    }

    #[test]
    fn applies_nfc() {
        let mut r = req();
        r.allowed_chars.push('ó');
        // "o" + combining acute becomes the single allowed "ó".
        assert_eq!(validate("zozo zo\u{301}", &r).unwrap(), "zozo zó");
    }

    #[test]
    fn empty_text_is_only_empty() {
        assert_eq!(
            validate(" \n\u{00A0} ", &req()),
            Err(vec![ValidationIssue::Empty])
        );
    }

    #[test]
    fn length_outside_the_range() {
        assert_eq!(
            validate("zz", &req()),
            Err(vec![ValidationIssue::Length { len: 2 }])
        );
        let long = "z".repeat(60);
        assert_eq!(
            validate(&long, &req()),
            Err(vec![ValidationIssue::Length { len: 60 }])
        );
        assert!(validate(&"z".repeat(59), &req()).is_ok());
        assert!(validate("zzzzz", &req()).is_ok());
    }

    #[test]
    fn each_distinct_char_not_allowed_once_in_order() {
        assert_eq!(
            validate("zaz! zQz! Q", &req()),
            Err(vec![
                ValidationIssue::CharNotAllowed { ch: '!' },
                ValidationIssue::CharNotAllowed { ch: 'Q' },
            ])
        );
    }

    #[test]
    fn markdown_is_detected() {
        let mut r = req();
        r.allowed_chars.push_str("*#`");
        for text in [
            "zaz **zo** zu",
            "# zaz zo",
            "zaz\n## zo",
            "zaz\n```\nzo",
            "- zaz\n- zo",
            "* zaz zo",
        ] {
            assert_eq!(
                validate(text, &r),
                Err(vec![ValidationIssue::Markdown]),
                "{text:?}"
            );
        }
        assert!(validate("zaz - zo * zu", &r).is_ok());
    }

    #[test]
    fn low_density_per_target_case_insensitive() {
        let mut r = req();
        r.targets = vec!["z".into(), "ci".into()];
        r.density = 5.0;
        r.length = 1..200;
        // len 50 → needed = ceil(5 × 0.02 × 50) = 5.
        let text = format!("Zaza zzz ci {}", "a".repeat(38));
        assert_eq!(text.chars().count(), 50);
        assert_eq!(
            validate(&text, &r),
            Err(vec![ValidationIssue::LowDensity {
                target: "ci".into(),
                found: 1,
                needed: 5
            }])
        );
        // density clamps to 2..=5 and needed is at least 1.
        r.density = 0.1;
        r.targets = vec!["ci".into()];
        assert!(validate("cici", &r).is_ok());
        assert_eq!(
            validate("abab", &r),
            Err(vec![ValidationIssue::LowDensity {
                target: "ci".into(),
                found: 0,
                needed: 1
            }])
        );
    }

    #[test]
    fn collects_every_issue() {
        assert_eq!(
            validate("**a!", &req()),
            Err(vec![
                ValidationIssue::Length { len: 4 },
                ValidationIssue::CharNotAllowed { ch: '*' },
                ValidationIssue::CharNotAllowed { ch: '!' },
                ValidationIssue::Markdown,
                ValidationIssue::LowDensity {
                    target: "z".into(),
                    found: 0,
                    needed: 1
                },
            ])
        );
    }

    #[test]
    fn code_mode_keeps_lines_and_indentation() {
        let mut r = req();
        r.mode = Mode::Code {
            lang: "python".into(),
        };
        r.allowed_chars.push_str("#():=1`");
        let code = "# zz\ndef z():  \n    zz = 1\n";
        assert_eq!(validate(code, &r).unwrap(), "# zz\ndef z():\n    zz = 1");
        assert_eq!(
            validate("```\nz = zz\n```", &r),
            Err(vec![ValidationIssue::Markdown])
        );
    }
}
