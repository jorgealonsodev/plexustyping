//! Adaptive weakness model: per-character and per-bigram error/slowness EMAs,
//! confusion pairs and seeded target selection.
//!
//! A *unit* is either one char (whitespace included) or a bigram of two
//! consecutive expected chars, neither of them whitespace.
//!
//! Confusions (expected char typed as another) are kept as queryable pairs.
//! They are trained through the expected char's own unit: every wrong record
//! raises that unit's error EMA, so frequently confused chars rank high in
//! [`WeaknessModel::ranked`] and [`WeaknessModel::select_targets`] without
//! any extra unit.

use crate::SessionResult;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Smoothing factor of every exponential moving average.
pub const ALPHA: f64 = 0.2;
/// Weight of the error EMA in a unit score.
pub const ERROR_WEIGHT: f64 = 0.7;
/// Weight of the slowness EMA in a unit score.
pub const SLOWNESS_WEIGHT: f64 = 0.3;
/// Intervals above this many milliseconds are pauses, ignored for slowness.
pub const PAUSE_MS: f64 = 2000.0;
/// Minimum error samples for a unit to become a ranked candidate.
pub const MIN_SAMPLES: u32 = 3;

/// Extra weight added to each score when drawing targets, so zero scores can win.
const DRAW_BASE_WEIGHT: f64 = 0.05;
/// Largest number of targets per text.
const MAX_TARGETS: usize = 5;

/// Accumulated statistics of one unit.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnitStat {
    /// EMA of error samples (1 wrong, 0 right), in `0..=1`.
    pub ema_error: f64,
    /// EMA of slowness samples, in `0..=1`.
    pub ema_slowness: f64,
    /// Error samples seen.
    pub samples: u32,
    /// Slowness samples seen.
    pub slowness_samples: u32,
}

/// How many times `expected` was typed as `typed`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Confusion {
    /// The char the text expected.
    pub expected: char,
    /// The char actually typed.
    pub typed: char,
    /// Number of wrong records with this pair.
    pub count: u32,
}

/// Serialized shape of [`WeaknessModel`] (tuple map keys are not JSON-safe).
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ModelRepr {
    units: BTreeMap<String, UnitStat>,
    confusions: Vec<Confusion>,
    mean_interval_ms: Option<f64>,
}

/// The user's weaknesses, updated after every session.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(from = "ModelRepr", into = "ModelRepr")]
pub struct WeaknessModel {
    units: BTreeMap<String, UnitStat>,
    confusions: BTreeMap<(char, char), u32>,
    mean_interval_ms: Option<f64>,
}

impl From<ModelRepr> for WeaknessModel {
    fn from(repr: ModelRepr) -> Self {
        Self::from_parts(
            repr.units.into_iter().collect(),
            repr.confusions,
            repr.mean_interval_ms,
        )
    }
}

impl From<WeaknessModel> for ModelRepr {
    fn from(model: WeaknessModel) -> Self {
        ModelRepr {
            confusions: model.confusions(),
            units: model.units,
            mean_interval_ms: model.mean_interval_ms,
        }
    }
}

/// `ALPHA`-EMA step; `first` initializes the average to `x`.
fn ema(current: f64, x: f64, first: bool) -> f64 {
    if first {
        x
    } else {
        ALPHA * x + (1.0 - ALPHA) * current
    }
}

fn unit_score(stat: &UnitStat) -> f64 {
    (ERROR_WEIGHT * stat.ema_error + SLOWNESS_WEIGHT * stat.ema_slowness).clamp(0.0, 1.0)
}

impl WeaknessModel {
    /// An empty model.
    pub fn new() -> Self {
        Self::default()
    }

    /// Rebuilds a model from stored parts; a repeated unit or pair keeps the last value.
    pub fn from_parts(
        units: Vec<(String, UnitStat)>,
        confusions: Vec<Confusion>,
        mean_interval_ms: Option<f64>,
    ) -> Self {
        Self {
            units: units.into_iter().collect(),
            confusions: confusions
                .into_iter()
                .map(|c| ((c.expected, c.typed), c.count))
                .collect(),
            mean_interval_ms,
        }
    }

    /// All units with their statistics, sorted by unit.
    pub fn units(&self) -> Vec<(String, UnitStat)> {
        self.units
            .iter()
            .map(|(u, s)| (u.clone(), s.clone()))
            .collect()
    }

    /// Confusion pairs by count (descending), ties by `(expected, typed)`.
    pub fn confusions(&self) -> Vec<Confusion> {
        let mut list: Vec<Confusion> = self
            .confusions
            .iter()
            .map(|(&(expected, typed), &count)| Confusion {
                expected,
                typed,
                count,
            })
            .collect();
        // Stable sort keeps the map's (expected, typed) order among equal counts.
        list.sort_by_key(|c| std::cmp::Reverse(c.count));
        list
    }

    /// EMA of the user's keystroke interval, ignoring pauses; `None` before any sample.
    pub fn mean_interval_ms(&self) -> Option<f64> {
        self.mean_interval_ms
    }

    /// Folds one session over `text` into the model.
    ///
    /// Each record updates its expected char and, from the second position on,
    /// the bigram ending at it unless either char is whitespace. Every record
    /// counts, including repeated ones at one position (stop-on-error).
    /// Intervals in `0 < ms <= PAUSE_MS` first update the mean interval, then
    /// add a slowness sample `clamp(ms / mean - 1, 0, 1)`.
    pub fn update(&mut self, text: &str, result: &SessionResult) {
        let chars: Vec<char> = text.chars().collect();
        for record in &result.records {
            let mut touched = vec![record.expected.to_string()];
            if record.index > 0 && record.index < chars.len() {
                let (prev, cur) = (chars[record.index - 1], chars[record.index]);
                if !prev.is_whitespace() && !cur.is_whitespace() {
                    touched.push([prev, cur].iter().collect());
                }
            }
            let error = if record.correct { 0.0 } else { 1.0 };
            let slowness = match record.ms_since_previous {
                Some(ms) if ms > 0.0 && ms <= PAUSE_MS => {
                    let mean = match self.mean_interval_ms {
                        Some(mean) => ema(mean, ms, false),
                        None => ms,
                    };
                    self.mean_interval_ms = Some(mean);
                    Some((ms / mean - 1.0).clamp(0.0, 1.0))
                }
                _ => None,
            };
            for unit in touched {
                let stat = self.units.entry(unit).or_default();
                stat.ema_error = ema(stat.ema_error, error, stat.samples == 0);
                stat.samples += 1;
                if let Some(x) = slowness {
                    stat.ema_slowness = ema(stat.ema_slowness, x, stat.slowness_samples == 0);
                    stat.slowness_samples += 1;
                }
            }
            if !record.correct {
                *self
                    .confusions
                    .entry((record.expected, record.typed))
                    .or_default() += 1;
            }
        }
    }

    /// `ERROR_WEIGHT * ema_error + SLOWNESS_WEIGHT * ema_slowness` in `0..=1`;
    /// `None` for an unknown unit.
    pub fn score(&self, unit: &str) -> Option<f64> {
        self.units.get(unit).map(unit_score)
    }

    /// Units with at least `MIN_SAMPLES` samples, by score (descending), ties by unit.
    pub fn ranked(&self) -> Vec<(String, f64)> {
        let mut list: Vec<(String, f64)> = self
            .units
            .iter()
            .filter(|(_, s)| s.samples >= MIN_SAMPLES)
            .map(|(u, s)| (u.clone(), unit_score(s)))
            .collect();
        // Stable sort keeps the map's unit order among equal scores.
        list.sort_by(|a, b| b.1.total_cmp(&a.1));
        list
    }

    /// Scores of single-char units at any sample count, sorted by char (heatmap).
    pub fn char_scores(&self) -> Vec<(char, f64)> {
        let mut list: Vec<(char, f64)> = self
            .units
            .iter()
            .filter_map(|(u, s)| {
                let mut it = u.chars();
                match (it.next(), it.next()) {
                    (Some(c), None) => Some((c, unit_score(s))),
                    _ => None,
                }
            })
            .collect();
        list.sort_by_key(|&(c, _)| c);
        list
    }

    /// Picks `count` (clamped to `1..=5`) distinct targets among the top
    /// `2 * count` ranked units, each draw weighted by score + 0.05.
    ///
    /// Deterministic for a given `seed`; returned by score (descending). With
    /// at most `count` candidates all of them are returned.
    pub fn select_targets(&self, count: usize, seed: u64) -> Vec<String> {
        let count = count.clamp(1, MAX_TARGETS);
        let mut pool = self.ranked();
        if pool.len() <= count {
            return pool.into_iter().map(|(u, _)| u).collect();
        }
        pool.truncate(2 * count);
        let mut rng = SplitMix64(seed);
        let mut picked: Vec<(String, f64)> = Vec::with_capacity(count);
        while picked.len() < count {
            let total: f64 = pool.iter().map(|(_, s)| s + DRAW_BASE_WEIGHT).sum();
            let mut target = rng.next_f64() * total;
            let mut chosen = pool.len() - 1;
            for (i, (_, s)) in pool.iter().enumerate() {
                target -= s + DRAW_BASE_WEIGHT;
                if target < 0.0 {
                    chosen = i;
                    break;
                }
            }
            picked.push(pool.remove(chosen));
        }
        picked.sort_by(|a, b| b.1.total_cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        picked.into_iter().map(|(u, _)| u).collect()
    }
}

/// Minimal SplitMix64 generator for seeded, dependency-free randomness.
struct SplitMix64(u64);

impl SplitMix64 {
    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform in `[0, 1)`.
    fn next_f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }
}
