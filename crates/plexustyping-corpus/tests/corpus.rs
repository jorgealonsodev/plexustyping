use std::collections::BTreeSet;

use plexustyping_corpus::{Corpus, GenerateOptions, corpus, languages};

const SPANISH_LOWER: &str = "abcdefghijklmnopqrstuvwxyzñáéíóúü";
const RANGES: [std::ops::Range<usize>; 4] = [50..80, 250..301, 500..601, 120..140];

fn es() -> &'static Corpus {
    corpus("es").expect("Spanish corpus is registered")
}

fn options(targets: &[&str], length: std::ops::Range<usize>, seed: u64) -> GenerateOptions {
    GenerateOptions {
        targets: targets.iter().map(|t| (*t).to_owned()).collect(),
        length,
        allowed: None,
        seed,
    }
}

fn assert_shape(text: &str, length: &std::ops::Range<usize>) {
    let count = text.chars().count();
    assert!(
        length.contains(&count),
        "{count} chars not in {length:?}: {text}"
    );
    assert!(!text.contains("  "), "double space: {text}");
    assert_eq!(text.trim(), text, "outer whitespace: {text:?}");
    assert!(text.ends_with('.'), "must end with '.': {text}");
}

/// Splits target-mode output into its pseudo-sentences, each a list of words.
fn pseudo_sentences(text: &str) -> Vec<Vec<String>> {
    let mut sentences = vec![Vec::new()];
    for word in text.split(' ') {
        let current = sentences.last_mut().unwrap();
        current.push(word.trim_end_matches('.').to_lowercase());
        if word.ends_with('.') {
            sentences.push(Vec::new());
        }
    }
    sentences.pop();
    sentences
}

#[test]
fn registry_lists_the_five_languages_spanish_first() {
    assert_eq!(languages(), ["es", "en", "pt", "fr", "de"]);
    assert_eq!(es().language(), "es");
    assert!(corpus("it").is_none());
}

/// Lowercase letters of each new language that its word list may use.
const NEW_ALPHABETS: [(&str, &str); 4] = [
    ("en", ""),
    ("pt", "áâãàéêíóôõúüç"),
    ("fr", "àâæçéèêëîïôœùûüÿ"),
    ("de", "äöüß"),
];

#[test]
fn new_languages_have_5000_words_many_sentences_and_a_full_diagnostic() {
    for (id, extra) in NEW_ALPHABETS {
        let c = corpus(id).unwrap_or_else(|| panic!("{id} is registered"));
        let alphabet: String = ('a'..='z').chain(extra.chars()).collect();
        assert_eq!(c.words().len(), 5000, "{id} words");
        for word in c.words() {
            assert!(word.chars().all(|ch| alphabet.contains(ch)), "{id} word {word:?}");
        }
        assert!(c.sentences().len() >= 40, "{id} sentences");
        let diagnostic = c.diagnostic();
        assert!((280..=340).contains(&diagnostic.chars().count()), "{id} diagnostic length");
        let lower = diagnostic.to_lowercase();
        for letter in ('a'..='z').chain(extra.chars().filter(|ch| !"æœ".contains(*ch))) {
            assert!(lower.contains(letter), "{id} diagnostic lacks {letter}");
        }
    }
}

#[test]
fn every_registered_language_resolves_to_its_own_non_empty_corpus() {
    for &id in languages() {
        let c = corpus(id).unwrap_or_else(|| panic!("{id} is listed but not found"));
        assert_eq!(c.language(), id);
        assert!(!c.diagnostic().is_empty(), "{id} diagnostic");
        assert!(!c.words().is_empty() && !c.sentences().is_empty(), "{id} data");
    }
    assert!(corpus("").is_none());
}

#[test]
fn diagnostic_covers_every_letter_accent_and_sign() {
    let text = es().diagnostic();
    let count = text.chars().count();
    assert!((280..=340).contains(&count), "diagnostic has {count} chars");
    assert_eq!(text.trim(), text);
    for required in SPANISH_LOWER.chars().chain("¿?¡!,.;:".chars()) {
        let present = text.to_lowercase().contains(required) || text.contains(required);
        assert!(present, "diagnostic lacks {required:?}");
    }
    let upper: BTreeSet<char> = text.chars().filter(|c| c.is_uppercase()).collect();
    assert!(upper.len() >= 3, "only {upper:?} uppercase");
}

#[test]
fn sentences_are_many_sized_and_plain() {
    let sentences = es().sentences();
    assert!(sentences.len() >= 40, "{} sentences", sentences.len());
    for sentence in sentences {
        let count = sentence.chars().count();
        assert!((40..=120).contains(&count), "{count} chars: {sentence}");
        assert_eq!(sentence.trim(), *sentence);
        for c in sentence.chars() {
            let plain = c == ' '
                || SPANISH_LOWER.contains(c.to_lowercase().next().unwrap())
                || "¿?¡!,.;:".contains(c);
            assert!(plain, "unexpected {c:?} in {sentence}");
        }
    }
}

#[test]
fn words_are_5000_filtered_lowercase_spanish_entries() {
    let words = es().words();
    assert_eq!(words.len(), 5000);
    assert_eq!(&words[..4], ["de", "que", "no", "a"]);
    let unique: BTreeSet<&str> = words.iter().copied().collect();
    assert_eq!(unique.len(), words.len(), "duplicate words");
    for word in words {
        assert!(
            !word.is_empty() && word.chars().all(|c| SPANISH_LOWER.contains(c)),
            "{word:?}"
        );
    }
    let blocked = [
        "mierda", "puta", "joder", "coño", "john", "michael", "parís", "the", "you", "okay", "hey",
        "aqui",
    ];
    for word in blocked {
        assert!(!unique.contains(word), "blocklisted {word:?} present");
    }
    for kept in ["kilómetros", "whisky", "niño", "vergüenza"] {
        assert!(unique.contains(kept), "{kept:?} missing");
    }
}

#[test]
fn sentence_mode_respects_length_and_shape() {
    for length in RANGES {
        for seed in 0..40 {
            let text = es().generate(&options(&[], length.clone(), seed));
            assert_shape(&text, &length);
        }
    }
}

#[test]
fn sentence_mode_joins_corpus_sentences() {
    let text = es().generate(&options(&[], 250..301, 7));
    assert_shape(&text, &(250..301));
    let mut rest = text.as_str();
    while !rest.is_empty() {
        let sentence = es()
            .sentences()
            .iter()
            .find(|s| rest.starts_with(**s))
            .unwrap_or_else(|| panic!("not a corpus sentence: {rest}"));
        rest = rest[sentence.len()..]
            .strip_prefix(' ')
            .unwrap_or(&rest[sentence.len()..]);
    }
}

#[test]
fn target_mode_respects_length_density_and_shape() {
    for targets in [
        &["ñ"][..],
        &["z", "q"],
        &["V"],
        &["x", "7"],
        &["qu"],
        &["RR", "ñ"],
    ] {
        let lower: Vec<String> = targets.iter().map(|t| t.to_lowercase()).collect();
        for length in RANGES {
            for seed in 0..40 {
                let text = es().generate(&options(targets, length.clone(), seed));
                assert_shape(&text, &length);
                for sentence in pseudo_sentences(&text) {
                    assert!((4..=10).contains(&sentence.len()), "{sentence:?}");
                    let hits = sentence
                        .iter()
                        .filter(|w| lower.iter().any(|t| w.contains(t.as_str())))
                        .count();
                    assert!(
                        hits * 2 >= sentence.len(),
                        "{targets:?} density in {sentence:?}"
                    );
                }
            }
        }
    }
}

#[test]
fn target_mode_capitalizes_every_sentence() {
    let text = es().generate(&options(&["ñ"], 500..601, 3));
    for sentence in text.split(". ") {
        assert!(
            sentence.chars().next().unwrap().is_uppercase(),
            "{sentence}"
        );
    }
}

#[test]
fn unknown_targets_fall_back_to_sentence_mode() {
    for seed in 0..10 {
        let unknown = es().generate(&options(&["7", "@"], 120..200, seed));
        let empty = es().generate(&options(&[], 120..200, seed));
        assert_shape(&unknown, &(120..200));
        assert_eq!(unknown, empty);
    }
}

#[test]
fn same_options_give_same_text_and_seeds_vary_it() {
    let a = options(&["ñ"], 250..301, 42);
    assert_eq!(es().generate(&a), es().generate(&a.clone()));
    let outputs: BTreeSet<String> = (0..10)
        .map(|seed| es().generate(&options(&["ñ"], 250..301, seed)))
        .collect();
    assert!(outputs.len() > 5, "seeds barely change the output");
}

#[test]
fn allowed_set_filters_words_and_capitals() {
    let lower = "abcdefghijlmnopqrstuvyzáéíóú";
    let mut allowed: BTreeSet<char> = lower.chars().collect();
    allowed.extend(['A', 'E', 'L', 'M', 'P']);
    for (targets, seed) in [
        (&["z"][..], 1),
        (&["m"], 2),
        (&[][..], 3),
        (&["ñ"], 4),
        (&["rr"], 5),
    ] {
        let mut opts = options(targets, 120..201, seed);
        opts.allowed = Some(allowed.clone());
        let text = es().generate(&opts);
        assert_shape(&text, &opts.length);
        for c in text.chars().filter(|c| *c != ' ' && *c != '.') {
            assert!(allowed.contains(&c), "{c:?} not allowed in {text}");
        }
    }
}

#[test]
fn allowed_set_without_sentence_matches_still_generates() {
    let allowed: BTreeSet<char> = "aeiouslnrtdcmpESP".chars().collect();
    let opts = GenerateOptions {
        allowed: Some(allowed.clone()),
        ..options(&[], 50..80, 9)
    };
    let text = es().generate(&opts);
    assert_shape(&text, &opts.length);
    for c in text.chars().filter(|c| *c != ' ' && *c != '.') {
        assert!(allowed.contains(&c), "{c:?} not allowed in {text}");
    }
}

#[test]
fn allowed_set_rejecting_every_word_returns_empty_text() {
    for targets in [&[][..], &["ñ"][..]] {
        let mut opts = options(targets, 50..80, 7);
        opts.allowed = Some(BTreeSet::from(['@']));
        assert_eq!(es().generate(&opts), "");
    }
}

#[test]
fn bigram_targets_match_words_containing_the_whole_bigram() {
    for (target, seed) in [("qu", 1), ("RR", 2), ("rr", 3)] {
        let text = es().generate(&options(&[target], 250..301, seed));
        assert_shape(&text, &(250..301));
        let bigram = target.to_lowercase();
        for sentence in pseudo_sentences(&text) {
            let hits = sentence.iter().filter(|w| w.contains(&bigram)).count();
            assert!(hits * 2 >= sentence.len(), "{target:?} in {sentence:?}");
        }
    }
}

#[test]
fn unmatched_bigram_and_empty_targets_fall_back_to_sentence_mode() {
    for seed in 0..5 {
        let empty = es().generate(&options(&[], 120..200, seed));
        assert_eq!(es().generate(&options(&["zx"], 120..200, seed)), empty);
        assert_eq!(es().generate(&options(&[""], 120..200, seed)), empty);
    }
}
