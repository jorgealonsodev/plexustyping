//! Progressive unlock texts below 15 letters (phase 3, S5): real corpus words
//! made of the active letters, completed with pronounceable pseudowords.

use std::collections::{BTreeMap, BTreeSet};

use plexustyping_corpus::{Corpus, LetterOptions, corpus};

/// The Spanish home row (S2).
const HOME: &str = "asdfghjklñ";
const VOWELS: &str = "aeiou";

fn es() -> &'static Corpus {
    corpus("es").unwrap()
}

fn options(
    letters: &str,
    weighted: &str,
    length: std::ops::Range<usize>,
    seed: u64,
) -> LetterOptions {
    LetterOptions {
        letters: letters.chars().collect(),
        weighted: weighted.chars().collect(),
        length,
        seed,
    }
}

fn is_real(word: &str) -> bool {
    es().words().contains(&word)
}

/// Whether `word` alternates single vowels and single consonants.
fn alternates(word: &str) -> bool {
    let kinds: Vec<bool> = word.chars().map(|c| VOWELS.contains(c)).collect();
    kinds.windows(2).all(|w| w[0] != w[1])
}

#[test]
fn texts_are_lowercase_words_of_the_active_letters_within_the_length() {
    for (letters, length, seed) in [
        (HOME, 300..361, 1),
        (HOME, 50..61, 2),
        ("asdfghjklñeo", 120..145, 3),
        ("asdfghjklñeonri", 600..721, 4),
    ] {
        let opts = options(letters, "", length.clone(), seed);
        let text = es().generate_letters(&opts);
        let len = text.chars().count();
        assert!(length.contains(&len), "{len} not in {length:?}: {text}");
        assert!(!text.contains("  ") && text.trim() == text, "{text:?}");
        for c in text.chars().filter(|c| *c != ' ') {
            assert!(letters.contains(c), "{c:?} is locked: {text}");
        }
        assert_eq!(es().generate_letters(&opts), text, "not deterministic");
    }
}

#[test]
fn real_words_come_first_each_once_then_pronounceable_pseudowords() {
    let opts = options(HOME, "", 300..361, 7);
    let text = es().generate_letters(&opts);
    let words: Vec<&str> = text.split(' ').collect();
    let real: Vec<&str> = words.iter().copied().filter(|w| is_real(w)).collect();
    assert!(real.len() >= 5, "few real words: {text}");
    let distinct: BTreeSet<&str> = real.iter().copied().collect();
    assert_eq!(distinct.len(), real.len(), "a real word repeats: {text}");
    let pseudo: Vec<&str> = words.iter().copied().filter(|w| !is_real(w)).collect();
    assert!(!pseudo.is_empty(), "no pseudowords: {text}");
    for word in pseudo {
        assert!(alternates(word), "{word:?} is not pronounceable: {text}");
    }
    // With many real words available, the text is all real words.
    let rich = es().generate_letters(&options("asdfghjklñeonrit", "", 120..145, 7));
    assert!(rich.split(' ').all(is_real), "{rich}");
}

#[test]
fn weighted_letters_appear_more_often() {
    let count = |weighted: &str, ch: char| -> usize {
        (0..10)
            .map(|seed| {
                let text = es().generate_letters(&options("asdfghjklñe", weighted, 300..361, seed));
                text.chars().filter(|c| *c == ch).count()
            })
            .sum()
    };
    let plain = count("", 'ñ');
    let weighted = count("ñ", 'ñ');
    assert!(
        weighted * 2 > plain * 3,
        "ñ {weighted} weighted vs {plain} plain"
    );
    // A weighted letter outside the active ones is ignored.
    let text = es().generate_letters(&options(HOME, "z", 100..121, 1));
    assert!(!text.contains('z'), "{text}");
}

#[test]
fn without_vowels_or_letters_it_still_behaves() {
    let text = es().generate_letters(&options("sdfghjkl", "", 50..61, 1));
    let len = text.chars().count();
    assert!((50..61).contains(&len), "{len}: {text}");
    let used: BTreeMap<char, usize> =
        text.chars()
            .filter(|c| *c != ' ')
            .fold(BTreeMap::new(), |mut m, c| {
                *m.entry(c).or_default() += 1;
                m
            });
    assert!(used.keys().all(|c| "sdfghjkl".contains(*c)), "{text}");
    assert_eq!(es().generate_letters(&options("", "", 50..61, 1)), "");
}
