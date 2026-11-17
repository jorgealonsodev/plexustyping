//! Words mode: loose lowercase words of the language, no punctuation.

use std::collections::BTreeSet;
use std::ops::Range;

use plexustyping_corpus::{GenerateOptions, corpus, languages};

fn options(targets: &[&str], length: Range<usize>, seed: u64) -> GenerateOptions {
    GenerateOptions {
        targets: targets.iter().map(|t| (*t).to_owned()).collect(),
        length,
        allowed: None,
        seed,
    }
}

/// Lowercase letters separated by single spaces, nothing else.
fn assert_loose_words(text: &str) {
    assert!(!text.is_empty());
    assert_eq!(text.trim(), text, "outer whitespace: {text:?}");
    assert!(!text.contains("  "), "double space: {text}");
    for ch in text.chars() {
        assert!(
            ch == ' ' || (ch.is_alphabetic() && !ch.is_uppercase()),
            "{ch:?} is not a lowercase letter: {text}"
        );
    }
}

#[test]
fn words_mode_writes_lowercase_words_of_the_list_without_punctuation() {
    for language in languages() {
        let c = corpus(language).unwrap();
        let list: BTreeSet<&str> = c.words().iter().copied().collect();
        for seed in 0..10 {
            let text = c.generate_words(&options(&["a"], 250..301, seed));
            assert_loose_words(&text);
            for word in text.split(' ') {
                assert!(list.contains(word), "{word:?} not in the {language} list");
            }
        }
        let plain = c.generate_words(&options(&[], 100..121, 1));
        assert_loose_words(&plain);
    }
}

#[test]
fn words_mode_respects_the_length_range() {
    let es = corpus("es").unwrap();
    for length in [50..61, 250..301, 300..361, 600..721] {
        for seed in 0..20 {
            for targets in [&["ñ"][..], &[], &["zzz"]] {
                let text = es.generate_words(&options(targets, length.clone(), seed));
                let n = text.chars().count();
                assert!(length.contains(&n), "{n} not in {length:?}: {text}");
            }
        }
    }
}

#[test]
fn words_mode_only_uses_allowed_chars() {
    let es = corpus("es").unwrap();
    let allowed: BTreeSet<char> = "abcdeilmnorstu".chars().collect();
    let opts = GenerateOptions {
        allowed: Some(allowed.clone()),
        ..options(&["r"], 100..121, 3)
    };
    let text = es.generate_words(&opts);
    assert_loose_words(&text);
    assert!(
        text.chars().all(|c| c == ' ' || allowed.contains(&c)),
        "{text}"
    );
    let nothing = GenerateOptions {
        allowed: Some(BTreeSet::new()),
        ..options(&["r"], 100..121, 3)
    };
    assert_eq!(es.generate_words(&nothing), "");
}

#[test]
fn words_mode_prefers_words_with_the_targets() {
    let es = corpus("es").unwrap();
    for seed in 0..10 {
        let text = es.generate_words(&options(&["ñ", "ll"], 300..361, seed));
        let words: Vec<&str> = text.split(' ').collect();
        let hits = words
            .iter()
            .filter(|w| w.contains('ñ') || w.contains("ll"))
            .count();
        assert!(hits * 2 >= words.len(), "{hits}/{}: {text}", words.len());
    }
}

#[test]
fn words_mode_is_deterministic_for_a_seed() {
    let es = corpus("es").unwrap();
    let a = es.generate_words(&options(&["r"], 200..241, 7));
    assert_eq!(a, es.generate_words(&options(&["r"], 200..241, 7)));
    assert_ne!(a, es.generate_words(&options(&["r"], 200..241, 8)));
}
