//! Local offline corpus and AI-free text generator.
//!
//! Each language bundles a frequency-ordered word list, everyday sentences and
//! a diagnostic text, embedded at compile time. [`Corpus::generate`] builds
//! practice text from them without any network or AI service. Code mode
//! bundles original snippets per programming language; see [`generate_code`].

mod code;

pub use code::{CodeLanguage, CodeOptions, Snippet, code_languages, code_snippets, generate_code};

use std::collections::BTreeSet;
use std::ops::Range;
use std::sync::LazyLock;

/// Number of most frequent words used to fill non-target slots.
const FILLER_WORDS: usize = 500;
/// Random draws per word before scanning the pool for a word that fits.
const DRAWS: usize = 32;
/// Attempts to build one sentence or one whole sentence-mode text.
const ATTEMPTS: usize = 64;
/// Remaining length below which the next sentence must close the text.
const FINAL_SPAN: usize = 60;

/// The embedded corpus of one language.
#[derive(Debug)]
pub struct Corpus {
    language: &'static str,
    diagnostic: &'static str,
    words: Vec<&'static str>,
    sentences: Vec<&'static str>,
}

/// Options for [`Corpus::generate`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GenerateOptions {
    /// Weak characters or bigrams to practice: a word matches when it contains
    /// one of them, case-insensitively. Empty joins corpus sentences instead.
    pub targets: Vec<String>,
    /// Wanted length in chars, end exclusive.
    pub length: Range<usize>,
    /// Characters the output may contain besides spaces and the final `.`.
    pub allowed: Option<BTreeSet<char>>,
    /// Seed of the deterministic generator.
    pub seed: u64,
}

static SPANISH: LazyLock<Corpus> = LazyLock::new(|| Corpus {
    language: "es",
    diagnostic: include_str!("../data/es/diagnostic.txt").trim(),
    words: data_lines(include_str!("../data/es/words.txt")),
    sentences: data_lines(include_str!("../data/es/sentences.txt")),
});

fn data_lines(text: &'static str) -> Vec<&'static str> {
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .collect()
}

/// Returns the corpus of `language` (for example `"es"`), if bundled.
pub fn corpus(language: &str) -> Option<&'static Corpus> {
    (language == "es").then(|| &*SPANISH)
}

/// Lists the codes of every bundled language.
pub fn languages() -> &'static [&'static str] {
    &["es"]
}

impl Corpus {
    /// Language code of this corpus.
    pub fn language(&self) -> &str {
        self.language
    }

    /// Fixed text covering every letter, accent, sign and capital use.
    pub fn diagnostic(&self) -> &str {
        self.diagnostic
    }

    /// Lowercase words, most frequent first.
    pub fn words(&self) -> &[&'static str] {
        &self.words
    }

    /// Everyday sentences, one per entry.
    pub fn sentences(&self) -> &[&'static str] {
        &self.sentences
    }

    /// Builds practice text; identical options always give identical text.
    ///
    /// With targets, it writes capitalized pseudo-sentences of 4 to 10 words
    /// where at least half the words contain a target. Without targets (or
    /// when no word contains one) it joins corpus sentences. The length lies
    /// in `options.length` whenever the corpus allows it, and the text ends
    /// with `.`. It is empty only if no word passes `options.allowed`.
    pub fn generate(&self, options: &GenerateOptions) -> String {
        let allowed = |c: char| options.allowed.as_ref().is_none_or(|set| set.contains(&c));
        let fits_charset = |text: &str| text.chars().all(|c| c == ' ' || c == '.' || allowed(c));
        let targets: Vec<String> = options
            .targets
            .iter()
            .filter(|t| !t.is_empty())
            .map(|t| t.to_lowercase())
            .collect();
        let words: Vec<&str> = self
            .words
            .iter()
            .copied()
            .filter(|w| fits_charset(w))
            .collect();
        let target_words: Vec<&str> = words
            .iter()
            .copied()
            .filter(|w| targets.iter().any(|t| w.contains(t.as_str())))
            .collect();
        let filler: Vec<&str> = self.words[..FILLER_WORDS.min(self.words.len())]
            .iter()
            .copied()
            .filter(|w| fits_charset(w))
            .collect();
        let (min, max) = bounds(&options.length);
        let mut rng = SplitMix64(options.seed);
        if target_words.is_empty() {
            let pool: Vec<&str> = self
                .sentences
                .iter()
                .copied()
                .filter(|s| fits_charset(s))
                .collect();
            if let Some(text) = join_sentences(&pool, min, max, &mut rng) {
                return text;
            }
        }
        let filler = if filler.is_empty() { &words } else { &filler };
        let pools = Pools {
            targets: &target_words,
            filler,
            capital_ok: &|word: &str| capitalize(word).chars().all(allowed),
        };
        pools.text(min, max, &mut rng)
    }
}

/// Inclusive char bounds of a wanted length range, never below one char.
fn bounds(length: &Range<usize>) -> (usize, usize) {
    let min = length.start.max(1);
    (min, length.end.saturating_sub(1).max(min))
}

/// Joins random sentences into a text within `min..=max` ending with `.`.
fn join_sentences(pool: &[&str], min: usize, max: usize, rng: &mut SplitMix64) -> Option<String> {
    if pool.is_empty() {
        return None;
    }
    for _ in 0..ATTEMPTS {
        let mut text = String::new();
        let mut len = 0;
        for _ in 0..ATTEMPTS {
            if len >= min {
                break;
            }
            let sentence = pool[rng.below(pool.len())];
            let added = sentence.chars().count() + usize::from(len > 0);
            if len + added <= max {
                if len > 0 {
                    text.push(' ');
                }
                text.push_str(sentence);
                len += added;
            }
        }
        if len >= min && text.ends_with('.') {
            return Some(text);
        }
    }
    None
}

/// Word pools for target-mode pseudo-sentences.
struct Pools<'a> {
    targets: &'a [&'a str],
    filler: &'a [&'a str],
    capital_ok: &'a dyn Fn(&str) -> bool,
}

impl Pools<'_> {
    /// Joins pseudo-sentences until the text reaches `min` chars.
    fn text(&self, min: usize, max: usize, rng: &mut SplitMix64) -> String {
        let all = self.targets.iter().chain(self.filler);
        let shortest = all.map(|w| w.chars().count()).min().unwrap_or(1);
        let smallest_sentence = 4 * (shortest + 1);
        let mut sentences: Vec<String> = Vec::new();
        let mut len = 0;
        while len < min {
            let sep = usize::from(len > 0);
            let Some(hi) = max.checked_sub(len + sep) else {
                break;
            };
            let lo = min - len - sep;
            let closing = (lo <= FINAL_SPAN)
                .then(|| self.sentence(lo, hi, rng))
                .flatten();
            let room = hi.saturating_sub(smallest_sentence + 1);
            let Some(sentence) = closing.or_else(|| self.sentence(0, room, rng)) else {
                break;
            };
            len += sep + sentence.chars().count();
            sentences.push(sentence);
        }
        if sentences.is_empty() {
            sentences.extend(self.sentence(0, usize::MAX, rng));
        }
        sentences.join(" ")
    }

    /// Builds one sentence of `lo..=hi` chars, retrying random shapes.
    fn sentence(&self, lo: usize, hi: usize, rng: &mut SplitMix64) -> Option<String> {
        (0..ATTEMPTS).find_map(|_| self.try_sentence(lo, hi, rng))
    }

    fn try_sentence(&self, lo: usize, hi: usize, rng: &mut SplitMix64) -> Option<String> {
        let count = 4 + rng.below(7);
        let mut kinds: Vec<bool> = (0..count).map(|i| i < count.div_ceil(2)).collect();
        if self.targets.is_empty() {
            kinds.fill(false);
        }
        for i in (1..count).rev() {
            kinds.swap(i, rng.below(i + 1));
        }
        let lengths = self
            .targets
            .iter()
            .chain(self.filler)
            .map(|w| w.chars().count());
        let (shortest, longest) = lengths.fold((usize::MAX, 0), |(a, b), n| (a.min(n), b.max(n)));
        // Both pools empty (an `allowed` set rejecting every word): no sentence can be built.
        if shortest == usize::MAX {
            return None;
        }
        let mut words: Vec<&str> = Vec::with_capacity(count);
        let mut used = 0;
        for (i, &target) in kinds.iter().enumerate() {
            let rest = count - i - 1;
            let longest_ok = hi.checked_sub(used + 1 + rest * (shortest + 1))?;
            let shortest_ok = lo.saturating_sub(used + 1 + rest * (longest + 1));
            let pool = if target { self.targets } else { self.filler };
            let fits = |w: &str| {
                let n = w.chars().count();
                (shortest_ok..=longest_ok).contains(&n) && (i > 0 || (self.capital_ok)(w))
            };
            let word = pick(pool, rng, fits)?;
            used += word.chars().count() + 1;
            words.push(word);
        }
        let mut sentence = capitalize(&words.join(" "));
        sentence.push('.');
        Some(sentence)
    }
}

/// Picks a word biased toward the front (most frequent) of `pool`.
fn pick<'a>(
    pool: &[&'a str],
    rng: &mut SplitMix64,
    fits: impl Fn(&str) -> bool,
) -> Option<&'a str> {
    if pool.is_empty() {
        return None;
    }
    for _ in 0..DRAWS {
        let u = rng.unit();
        let word = pool[((u * u * pool.len() as f64) as usize).min(pool.len() - 1)];
        if fits(word) {
            return Some(word);
        }
    }
    let start = rng.below(pool.len());
    (0..pool.len())
        .map(|i| pool[(start + i) % pool.len()])
        .find(|w| fits(w))
}

fn capitalize(text: &str) -> String {
    let mut chars = text.chars();
    chars.next().map_or_else(String::new, |first| {
        first.to_uppercase().chain(chars).collect()
    })
}

/// SplitMix64 pseudo-random generator: tiny, fast and deterministic.
struct SplitMix64(u64);

impl SplitMix64 {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform index in `0..n`; `n` must be positive.
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }

    /// Uniform float in `[0, 1)`.
    fn unit(&mut self) -> f64 {
        (self.next() >> 11) as f64 / (1u64 << 53) as f64
    }
}
