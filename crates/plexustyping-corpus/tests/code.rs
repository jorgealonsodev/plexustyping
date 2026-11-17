use std::collections::BTreeMap;

use plexustyping_corpus::{CodeOptions, Snippet, code_languages, code_snippets, generate_code};

const EXPECTED: [(&str, &str); 15] = [
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

/// S2's weak symbols; bracket pairs are checked one char at a time.
const SYMBOLS: [&str; 20] = [
    "{", "}", "[", "]", "(", ")", "<", ">", ";", ":", "=", "->", "=>", "&&", "||", "\"", "'", "_",
    "#", "$",
];

/// Raw data files, to catch malformed headers the lenient parser skips.
const RAW: [(&str, &str); 15] = [
    ("python", include_str!("../data/code/python.txt")),
    ("javascript", include_str!("../data/code/javascript.txt")),
    ("typescript", include_str!("../data/code/typescript.txt")),
    ("java", include_str!("../data/code/java.txt")),
    ("csharp", include_str!("../data/code/csharp.txt")),
    ("cpp", include_str!("../data/code/cpp.txt")),
    ("c", include_str!("../data/code/c.txt")),
    ("go", include_str!("../data/code/go.txt")),
    ("rust", include_str!("../data/code/rust.txt")),
    ("php", include_str!("../data/code/php.txt")),
    ("sql", include_str!("../data/code/sql.txt")),
    ("bash", include_str!("../data/code/bash.txt")),
    ("html-css", include_str!("../data/code/html-css.txt")),
    ("kotlin", include_str!("../data/code/kotlin.txt")),
    ("swift", include_str!("../data/code/swift.txt")),
];

fn syntaxes(language: &str) -> &[&str] {
    match language {
        "html-css" => &["html", "css"],
        other => std::slice::from_ref(&EXPECTED.iter().find(|(id, _)| *id == other).unwrap().0),
    }
}

fn snippets(language: &str) -> &'static [Snippet] {
    code_snippets(language).unwrap_or_else(|| panic!("{language} has snippets"))
}

fn options(language: &str, targets: &[&str], max_lines: usize, seed: u64) -> CodeOptions {
    CodeOptions {
        language: language.to_owned(),
        targets: targets.iter().map(|t| (*t).to_owned()).collect(),
        max_lines,
        seed,
    }
}

fn line_count(snippet: &Snippet) -> usize {
    snippet.code.lines().count()
}

#[test]
fn lists_exactly_the_fifteen_languages_in_order() {
    let got: Vec<(&str, &str)> = code_languages().iter().map(|l| (l.id, l.name)).collect();
    assert_eq!(got, EXPECTED);
}

#[test]
fn every_language_has_at_least_ten_snippets() {
    for (id, _) in EXPECTED {
        let count = snippets(id).len();
        assert!(count >= 10, "{id}: {count} snippets");
    }
}

#[test]
fn unknown_language_has_no_snippets() {
    assert!(code_snippets("cobol").is_none());
    assert!(code_snippets("").is_none());
    assert!(generate_code(&options("cobol", &[], 20, 1)).is_none());
}

#[test]
fn raw_files_have_only_well_formed_headers() {
    for (id, raw) in RAW {
        assert!(raw.starts_with("%% "), "{id}: must start with a header");
        let headers: Vec<&str> = raw.lines().filter(|l| l.starts_with("%%")).collect();
        for header in &headers {
            let syntax = header.strip_prefix("%% ").unwrap_or("");
            assert!(
                syntaxes(id).contains(&syntax),
                "{id}: malformed header {header:?}"
            );
        }
        assert_eq!(headers.len(), snippets(id).len(), "{id}: empty snippet");
    }
}

#[test]
fn every_snippet_has_valid_syntax_and_shape() {
    for (id, _) in EXPECTED {
        for snippet in snippets(id) {
            let code = snippet.code;
            assert!(syntaxes(id).contains(&snippet.syntax), "{id}: {snippet:?}");
            let lines: Vec<&str> = code.lines().collect();
            assert!(
                (5..=20).contains(&lines.len()),
                "{id}: {} lines\n{code}",
                lines.len()
            );
            assert!(!lines[0].trim().is_empty(), "{id}: leading blank\n{code}");
            assert!(
                !lines[lines.len() - 1].trim().is_empty(),
                "{id}: trailing blank\n{code}"
            );
            assert!(code.is_ascii(), "{id}: non-ASCII\n{code}");
            let step = if id == "html-css" { 2 } else { 4 };
            for line in lines {
                assert!(!line.contains('\t'), "{id}: tab in {line:?}");
                assert!(!line.contains('\r'), "{id}: CR in {line:?}");
                assert_eq!(line.trim_end(), line, "{id}: trailing space in {line:?}");
                assert!(line.len() <= 80, "{id}: over 80 columns: {line:?}");
                let indent = line.len() - line.trim_start().len();
                assert_eq!(indent % step, 0, "{id}: indent {indent} in {line:?}");
            }
        }
    }
}

#[test]
fn every_weak_symbol_appears_in_at_least_eight_languages() {
    for symbol in SYMBOLS {
        let languages: Vec<&str> = EXPECTED
            .iter()
            .map(|(id, _)| *id)
            .filter(|id| snippets(id).iter().any(|s| s.code.contains(symbol)))
            .collect();
        assert!(languages.len() >= 8, "{symbol:?} only in {languages:?}");
    }
}

#[test]
fn same_options_give_the_same_snippet() {
    for (id, _) in EXPECTED {
        for seed in 0..20 {
            let opts = options(id, &["{", "_"], 20, seed);
            assert_eq!(generate_code(&opts), generate_code(&opts.clone()), "{id}");
        }
    }
}

#[test]
fn empty_targets_pick_varied_snippets_from_the_corpus() {
    let picked: Vec<Snippet> = (0..200)
        .map(|seed| generate_code(&options("python", &[], 20, seed)).unwrap())
        .collect();
    assert!(picked.iter().all(|s| snippets("python").contains(s)));
    let distinct: std::collections::BTreeSet<&str> = picked.iter().map(|s| s.code).collect();
    assert!(
        distinct.len() >= 5,
        "only {} distinct snippets",
        distinct.len()
    );
}

#[test]
fn snippets_with_the_target_are_chosen_far_more_often() {
    // A word repeated in exactly one snippet of each language: weight 1 + count.
    for (id, _) in EXPECTED {
        let all = snippets(id);
        let mut found = None;
        'search: for (index, snippet) in all.iter().enumerate() {
            for word in snippet
                .code
                .split(|c: char| !c.is_ascii_alphanumeric() && c != '_')
            {
                let count = snippet.code.matches(word).count();
                let elsewhere = all.iter().filter(|s| s.code.contains(word)).count();
                if word.len() >= 4 && count >= 4 && elsewhere == 1 {
                    found = Some((index, word));
                    break 'search;
                }
            }
        }
        let (index, word) = found.unwrap_or_else(|| panic!("{id}: no unique repeated word"));
        let mut hits: BTreeMap<&str, usize> = BTreeMap::new();
        for seed in 0..2000 {
            let snippet = generate_code(&options(id, &[word], 20, seed)).unwrap();
            *hits.entry(snippet.code).or_default() += 1;
        }
        let target = hits.get(all[index].code).copied().unwrap_or(0);
        let uniform = 2000 / all.len();
        assert!(
            target >= 3 * uniform,
            "{id}: {word:?} chosen {target} times"
        );
    }
}

#[test]
fn max_lines_skips_longer_snippets() {
    for (id, _) in EXPECTED {
        let shortest = snippets(id).iter().map(line_count).min().unwrap();
        for max_lines in [shortest, 8, 12, 20] {
            for seed in 0..50 {
                let snippet = generate_code(&options(id, &["("], max_lines, seed)).unwrap();
                let lines = line_count(&snippet);
                assert!(
                    lines <= max_lines.max(shortest),
                    "{id}: {lines} > {max_lines}"
                );
            }
        }
    }
}

#[test]
fn shortest_snippet_when_none_fits() {
    for (id, _) in EXPECTED {
        let shortest = snippets(id).iter().map(line_count).min().unwrap();
        let snippet = generate_code(&options(id, &[], shortest - 1, 7)).unwrap();
        assert_eq!(line_count(&snippet), shortest, "{id}");
    }
}

#[test]
fn types_serialize_in_camel_case() {
    let language = &code_languages()[0];
    let json = serde_json::to_value(language).unwrap();
    assert_eq!(json, serde_json::json!({"id": "python", "name": "Python"}));
    let snippet = &snippets("rust")[0];
    let json = serde_json::to_value(snippet).unwrap();
    assert_eq!(
        json,
        serde_json::json!({"syntax": snippet.syntax, "code": snippet.code})
    );
}

#[test]
fn code_frequency_is_the_per_char_rate_in_the_lowercased_snippets() {
    let sample = code_snippets("rust")
        .unwrap()
        .iter()
        .map(|s| s.code)
        .collect::<Vec<_>>()
        .join("\n")
        .to_lowercase();
    let expected = sample.matches('{').count() as f64 / sample.chars().count() as f64;
    assert!(expected > 0.0);
    assert_eq!(
        plexustyping_corpus::code_frequency("rust", "{"),
        Some(expected)
    );
    assert_eq!(
        plexustyping_corpus::code_frequency("rust", "FN"),
        plexustyping_corpus::code_frequency("rust", "fn")
    );
    assert_eq!(plexustyping_corpus::code_frequency("rust", "ñ"), Some(0.0));
    assert_eq!(plexustyping_corpus::code_frequency("rust", ""), Some(0.0));
    assert_eq!(plexustyping_corpus::code_frequency("cobol", "{"), None);
}
