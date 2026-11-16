//! Original code snippets per programming language and a generator that
//! favours snippets rich in the user's weak symbols.
//!
//! Each `data/code/<id>.txt` holds snippets that start with a `%% <syntax>`
//! header line, where `<syntax>` is the highlighter id of the snippet.

use std::sync::LazyLock;

use serde::Serialize;

use crate::SplitMix64;

/// A programming language with bundled snippets.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CodeLanguage {
    /// Stable id, for example `"html-css"`.
    pub id: &'static str,
    /// Display name, for example `"HTML/CSS"`.
    pub name: &'static str,
}

/// One code snippet to practice.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snippet {
    /// Highlighter syntax id, for example `"rust"` or `"css"`.
    pub syntax: &'static str,
    /// Code lines joined by `\n`, without a final newline.
    pub code: &'static str,
}

/// Options for [`generate_code`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CodeOptions {
    /// Language id from [`code_languages`].
    pub language: String,
    /// Weak symbols, chars or bigrams; each case-sensitive occurrence in a
    /// snippet adds one to its weight. Empty picks uniformly.
    pub targets: Vec<String>,
    /// Longest snippet wanted, in lines (5..=20).
    pub max_lines: usize,
    /// Seed of the deterministic generator.
    pub seed: u64,
}

macro_rules! languages {
    ($(($id:literal, $name:literal)),* $(,)?) => {
        const LANGUAGES: &[CodeLanguage] = &[$(CodeLanguage { id: $id, name: $name }),*];
        const SOURCES: &[&str] = &[$(include_str!(concat!("../data/code/", $id, ".txt"))),*];
    };
}

languages![
    ("python", "Python"),
    ("javascript", "JavaScript"),
    ("typescript", "TypeScript"),
    ("java", "Java"),
    ("csharp", "C#"),
    ("cpp", "C++"),
    ("c", "C"),
    ("go", "Go"),
    ("rust", "Rust"),
    ("php", "PHP"),
    ("sql", "SQL"),
    ("bash", "Bash"),
    ("html-css", "HTML/CSS"),
    ("kotlin", "Kotlin"),
    ("swift", "Swift"),
];

static SNIPPETS: LazyLock<Vec<Vec<Snippet>>> =
    LazyLock::new(|| SOURCES.iter().map(|text| parse(text)).collect());

/// Splits a data file at its `%%` header lines. Text before the first header
/// and empty snippets are skipped, so bad data never panics at runtime.
fn parse(text: &'static str) -> Vec<Snippet> {
    let mut snippets = Vec::new();
    let mut current: Option<(&'static str, usize)> = None;
    let mut offset = 0;
    let mut close = |current: Option<(&'static str, usize)>, end: usize| {
        if let Some((syntax, start)) = current {
            let code = text[start..end].trim_end().trim_start_matches(['\n', '\r']);
            if !code.is_empty() {
                snippets.push(Snippet { syntax, code });
            }
        }
    };
    for line in text.split_inclusive('\n') {
        if let Some(syntax) = line.strip_prefix("%%") {
            close(current.take(), offset);
            current = Some((syntax.trim(), offset + line.len()));
        }
        offset += line.len();
    }
    close(current, text.len());
    snippets
}

/// Lists every language with bundled snippets, in display order.
pub fn code_languages() -> &'static [CodeLanguage] {
    LANGUAGES
}

/// Returns the snippets of language `language`, if bundled.
pub fn code_snippets(language: &str) -> Option<&'static [Snippet]> {
    let index = LANGUAGES.iter().position(|l| l.id == language)?;
    Some(&SNIPPETS[index])
}

/// Picks a snippet of `options.language`; identical options give the same one.
///
/// Snippets over `options.max_lines` are skipped (the shortest is used when
/// none fits); each remaining one weighs `1 +` its target occurrences.
/// Returns `None` for an unknown language.
pub fn generate_code(options: &CodeOptions) -> Option<Snippet> {
    let snippets = code_snippets(&options.language)?;
    let lines = |s: &&Snippet| s.code.lines().count();
    let mut pool: Vec<&Snippet> = snippets
        .iter()
        .filter(|s| lines(s) <= options.max_lines)
        .collect();
    if pool.is_empty() {
        pool.extend(snippets.iter().min_by_key(lines));
    }
    let weights: Vec<u64> = pool
        .iter()
        .map(|s| 1 + score(s.code, &options.targets))
        .collect();
    let total: u64 = weights.iter().sum();
    if total == 0 {
        return None;
    }
    let mut roll = SplitMix64(options.seed).next() % total;
    for (snippet, weight) in pool.into_iter().zip(weights) {
        if roll < weight {
            return Some(snippet.clone());
        }
        roll -= weight;
    }
    None
}

/// Total case-sensitive occurrences of every non-empty target in `code`.
fn score(code: &str, targets: &[String]) -> u64 {
    targets
        .iter()
        .filter(|t| !t.is_empty())
        .map(|t| code.matches(t.as_str()).count() as u64)
        .sum()
}
