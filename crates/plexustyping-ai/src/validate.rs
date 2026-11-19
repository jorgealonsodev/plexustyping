//! Checks a generated text against its request (PRD §4.6).

use serde::{Deserialize, Serialize};
use unicode_normalization::UnicodeNormalization;

use crate::prompt::clamped_density;
use crate::{GenerationRequest, Mode};

/// Tolerance so an exact product (e.g. `5 × 0.02 × 50`) is not rounded up
/// by float error.
const EPSILON: f64 = 1e-9;

/// Most a target's share of the text may be raised over its natural share:
/// 3× a frequent letter (`a` is about 12 %) would ask for a third of the text.
pub const MAX_EXTRA_SHARE: f64 = 0.05;

/// Occurrences a target needs in a text of `len` chars:
/// `max(1, ceil(min(density × frequency, frequency + MAX_EXTRA_SHARE) × len))`,
/// with the density clamped to `2.0..=5.0` (3 when not a number) and
/// `frequency` the target's natural per-char frequency (a negative or
/// non-finite one counts as 0).
pub fn min_count(density: f32, frequency: f64, len: usize) -> usize {
    let frequency = if frequency.is_finite() {
        frequency.max(0.0)
    } else {
        0.0
    };
    let share = (f64::from(clamped_density(density)) * frequency).min(frequency + MAX_EXTRA_SHARE);
    let wanted = share * len as f64;
    ((wanted - EPSILON).ceil().max(0.0) as usize).max(1)
}

impl GenerationRequest {
    /// Each non-empty target with the [`min_count`] it needs in a text of
    /// `len` chars. [`validate`] checks these in code mode; the code prompt
    /// states them for the longest accepted length.
    pub fn min_counts(&self, len: usize) -> Vec<(&str, usize)> {
        self.targets
            .iter()
            .enumerate()
            .filter(|(_, t)| !t.is_empty())
            .map(|(i, t)| {
                let frequency = self.frequencies.get(i).copied().unwrap_or(0.0);
                (t.as_str(), min_count(self.density, frequency, len))
            })
            .collect()
    }
}

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
    /// A target appears fewer times than the density asks for (code only).
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
/// `Markdown`, and, in [`Mode::Code`] only, `LowDensity` per target below its
/// [`GenerationRequest::min_counts`] for the text's length (the prose prompt
/// names no targets, so prose is not checked for them).
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

    let lower = normalized.to_lowercase();
    let checked = if code {
        req.min_counts(len)
    } else {
        Vec::new()
    };
    for (target, needed) in checked {
        let needle: String = target.nfc().collect::<String>().to_lowercase();
        let found = lower.matches(needle.as_str()).count();
        if found < needed {
            issues.push(ValidationIssue::LowDensity {
                target: target.to_owned(),
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
            frequencies: Vec::new(),
            density: 2.0,
            length: 5..60,
            allowed_chars: "abcdefghijklmnopqrstuvwxyzñZ.,'\"-".into(),
            restrict_chars: false,
            topic: None,
        }
    }

    #[test]
    fn min_count_is_density_times_frequency_times_length_at_least_one() {
        // ceil(3 × 0.002 × 340) = ceil(2.04) = 3.
        assert_eq!(min_count(3.0, 0.002, 340), 3);
        // A frequent letter asks for at most 5 points over its natural share, never
        // 3× of it: ceil((0.12 + 0.05) × 340) = 58, not ceil(3 × 0.12 × 340) = 123.
        assert_eq!(min_count(3.0, 0.12, 340), 58);
        // The cap only bites when density × frequency is higher: 3 × 0.02 = 0.06 < 0.07.
        assert_eq!(min_count(3.0, 0.02, 100), 6);
        // Exact products are not pushed up by float error: 5 × 0.01 × 100 = 5.
        assert_eq!(min_count(5.0, 0.01, 100), 5);
        // Density clamps to 2..=5, 3 when not a number.
        assert_eq!(min_count(0.1, 0.01, 100), 2);
        assert_eq!(min_count(9.0, 0.01, 100), 5);
        assert_eq!(min_count(f32::NAN, 0.01, 100), 3);
        // At least 1, also for an unknown, negative or broken frequency.
        assert_eq!(min_count(3.0, 0.0, 340), 1);
        assert_eq!(min_count(3.0, 0.0001, 10), 1);
        assert_eq!(min_count(3.0, -0.5, 340), 1);
        assert_eq!(min_count(3.0, f64::NAN, 340), 1);
    }

    #[test]
    fn min_counts_pair_each_non_empty_target_with_its_frequency() {
        let mut r = req();
        r.density = 3.0;
        r.targets = vec!["ntr".into(), "".into(), "ú".into(), "ti".into()];
        r.frequencies = vec![0.0019, 0.5, 0.0004];
        assert_eq!(
            r.min_counts(340),
            vec![("ntr", 2), ("ú", 1), ("ti", 1)],
            "ti has no frequency: 1"
        );
    }

    #[test]
    fn the_log_example_needs_small_counts() {
        // L2: 340 chars, density 3, ntr/ú/ti once or twice no longer fail
        // because of a fixed 2 % baseline (it asked for 21 each).
        let mut r = req();
        r.density = 3.0;
        r.length = 300..381;
        r.allowed_chars = "abcdefghijlmnopqrstuvyzáéíóúñDEFLP.,".into();
        r.targets = vec!["ntr".into(), "ú".into(), "ti".into()];
        r.frequencies = vec![0.00186, 0.00037, 0.00223];
        let text = "Entré en la tienda del centro a las nueve. Dentro había poca gente y un señor muy simpático me atendió enseguida. Le pedí un café y un pan tostado, y me senté junto a la ventana para ver la calle. Fuera, la tarde estaba tranquila. Pensé en mi último viaje al norte, en las montañas y en los amigos que dejé allí. Luego pagué y volví a casa.";
        assert_eq!(text.chars().count(), 340);
        assert_eq!(r.min_counts(340), vec![("ntr", 2), ("ú", 1), ("ti", 3)]);
        assert_eq!(validate(text, &r).as_deref(), Ok(text));
    }

    #[test]
    fn natural_sentences_then_a_run_of_target_words_pass() {
        // S5: when sense runs out, real words with the targets fill the rest.
        let mut r = req();
        r.density = 3.0;
        r.length = 80..161;
        r.allowed_chars = "abcdefghijlmnopqrstuvyzáéíóúñMCN.,".into();
        r.targets = vec!["ñ".into(), "ci".into()];
        r.frequencies = vec![0.01, 0.01];
        let text = "Mi abuela vive en el campo. Cada mañana sale a ver el sol. \
                    Niño caña piña uña leña cielo cine cifra cien ciudad.";
        let len = text.chars().count();
        assert_eq!(r.min_counts(len), vec![("ñ", 4), ("ci", 4)]);
        assert_eq!(validate(text, &r).as_deref(), Ok(text));
        // The run on its own line is still one line of prose.
        let broken = text.replacen(". Niño", ".\nNiño", 1);
        assert_eq!(validate(&broken, &r).as_deref(), Ok(text));
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
    fn prose_has_no_density_check() {
        // S9: the prose prompt names no targets, so prose is not checked for them.
        let mut r = req();
        r.targets = vec!["z".into(), "ci".into()];
        r.frequencies = vec![0.02, 0.02];
        r.density = 5.0;
        assert_eq!(
            validate("hola casa bonita.", &r).as_deref(),
            Ok("hola casa bonita.")
        );
        // The other checks still apply.
        assert_eq!(
            validate("hola!", &r),
            Err(vec![ValidationIssue::CharNotAllowed { ch: '!' }])
        );
    }

    #[test]
    fn low_density_per_target_case_insensitive_in_code() {
        let mut r = req();
        r.mode = Mode::Code {
            lang: "rust".into(),
        };
        r.targets = vec!["z".into(), "ci".into()];
        r.frequencies = vec![0.02, 0.02];
        r.density = 5.0;
        r.length = 1..200;
        // len 50 → 5 × 0.02 = 0.10 is capped at 0.02 + 0.05, so needed = ceil(0.07 × 50) = 4.
        let text = format!("Zaza zzz ci {}", "a".repeat(38));
        assert_eq!(text.chars().count(), 50);
        assert_eq!(
            validate(&text, &r),
            Err(vec![ValidationIssue::LowDensity {
                target: "ci".into(),
                found: 1,
                needed: 4
            }])
        );
        // density clamps to 2..=5 and needed is at least 1.
        r.density = 0.1;
        r.targets = vec!["ci".into()];
        r.frequencies = vec![0.02];
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
            ])
        );
        let mut r = req();
        r.mode = Mode::Code {
            lang: "rust".into(),
        };
        r.allowed_chars.push('`');
        r.length = 10..60;
        assert_eq!(
            validate("```\na!", &r),
            Err(vec![
                ValidationIssue::Length { len: 6 },
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
