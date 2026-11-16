//! Adaptive weakness model behavior through the public API (S1–S3).

use plexustyping_core::adaptive::{
    ALPHA, Confusion, ERROR_WEIGHT, MIN_SAMPLES, PAUSE_MS, SLOWNESS_WEIGHT, UnitStat, WeaknessModel,
};
use plexustyping_core::{ErrorMode, InputEvent, SessionResult, replay};

/// Replays `typed` against `text`, one char per timestamp in `times`.
fn session(text: &str, mode: ErrorMode, typed: &str, times: &[f64]) -> SessionResult {
    let chars: Vec<char> = typed.chars().collect();
    assert_eq!(chars.len(), times.len(), "one timestamp per typed char");
    let events: Vec<InputEvent> = chars
        .iter()
        .zip(times)
        .map(|(&ch, &t_ms)| InputEvent::Char { ch, t_ms })
        .collect();
    replay(text, mode, &events)
}

/// Replays `typed` with a constant 100 ms interval.
fn steady(text: &str, typed: &str) -> SessionResult {
    let times: Vec<f64> = (0..typed.chars().count())
        .map(|i| i as f64 * 100.0)
        .collect();
    session(text, ErrorMode::AllowContinue, typed, &times)
}

fn trained(text: &str, typed: &str) -> WeaknessModel {
    let mut model = WeaknessModel::new();
    model.update(text, &steady(text, typed));
    model
}

fn stat(model: &WeaknessModel, unit: &str) -> UnitStat {
    model
        .units()
        .into_iter()
        .find(|(u, _)| u == unit)
        .map(|(_, s)| s)
        .unwrap_or_else(|| panic!("unit {unit:?} missing"))
}

fn close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() < 1e-12,
        "got {actual}, expected {expected}"
    );
}

fn unit_stat(ema_error: f64, ema_slowness: f64, samples: u32) -> UnitStat {
    UnitStat {
        ema_error,
        ema_slowness,
        samples,
        slowness_samples: samples,
    }
}

/// Twelve eligible units "a".."l" with strictly decreasing scores.
fn twelve_units() -> WeaknessModel {
    let units = "abcdefghijkl"
        .chars()
        .enumerate()
        .map(|(i, c)| (c.to_string(), unit_stat(1.0 - i as f64 * 0.08, 0.0, 5)))
        .collect();
    WeaknessModel::from_parts(units, Vec::new(), None)
}

fn score_of(model: &WeaknessModel, unit: &str) -> f64 {
    model.score(unit).expect("known unit")
}

#[test]
fn constants_match_the_prd_and_decisions() {
    assert_eq!(ALPHA, 0.2);
    assert_eq!(ERROR_WEIGHT, 0.7);
    assert_eq!(SLOWNESS_WEIGHT, 0.3);
    assert_eq!(PAUSE_MS, 2000.0);
    assert_eq!(MIN_SAMPLES, 3);
}

#[test]
fn error_ema_initializes_with_first_sample_then_uses_alpha() {
    let mut model = WeaknessModel::new();
    assert!(model.units().is_empty());
    assert_eq!(model.mean_interval_ms(), None);
    model.update("a", &steady("a", "x"));
    let first = stat(&model, "a");
    assert_eq!(first.samples, 1);
    close(first.ema_error, 1.0);
    for typed in ["a", "a", "x"] {
        model.update("a", &steady("a", typed));
    }
    let s = stat(&model, "a");
    assert_eq!(s.samples, 4);
    // 1.0 → 0.8 → 0.64 → 0.2 + 0.8 * 0.64
    close(s.ema_error, 0.712);
    assert_eq!(s.slowness_samples, 0);
    close(s.ema_slowness, 0.0);
    close(score_of(&model, "a"), 0.7 * 0.712);
    assert_eq!(model.mean_interval_ms(), None);
}

#[test]
fn first_correct_sample_initializes_error_to_zero() {
    let model = trained("a", "a");
    close(stat(&model, "a").ema_error, 0.0);
    close(score_of(&model, "a"), 0.0);
}

#[test]
fn score_weights_error_and_slowness() {
    let model = WeaknessModel::from_parts(
        vec![("q".into(), unit_stat(0.5, 0.25, 4))],
        Vec::new(),
        None,
    );
    close(score_of(&model, "q"), 0.7 * 0.5 + 0.3 * 0.25);
    assert_eq!(model.score("z"), None);
    let max = WeaknessModel::from_parts(vec![("m".into(), unit_stat(1.0, 1.0, 4))], vec![], None);
    close(score_of(&max, "m"), 1.0);
}

#[test]
fn bigrams_skip_whitespace_and_treat_accented_chars_as_one() {
    let model = trained("añ bñ", "añ bñ");
    let units: Vec<String> = model.units().into_iter().map(|(u, _)| u).collect();
    // No "ñ " nor " b": whitespace never joins a bigram.
    assert_eq!(units, vec![" ", "a", "añ", "b", "bñ", "ñ"]);
    assert_eq!(stat(&model, "ñ").samples, 2);
    assert_eq!(stat(&model, "añ").samples, 1);
    assert_eq!(stat(&model, " ").samples, 1);
}

#[test]
fn bigram_errors_follow_the_record_at_its_second_char() {
    // "b" typed wrong: the "ab" bigram carries the error, "a" does not.
    let model = trained("ab", "ax");
    close(stat(&model, "a").ema_error, 0.0);
    close(stat(&model, "b").ema_error, 1.0);
    close(stat(&model, "ab").ema_error, 1.0);
    assert_eq!(stat(&model, "ab").samples, 1);
}

#[test]
fn slowness_uses_mean_interval_ema_and_skips_pauses() {
    // Intervals: none, 100, 200, 2100 (pause).
    let result = session(
        "abcd",
        ErrorMode::AllowContinue,
        "abcd",
        &[0.0, 100.0, 300.0, 2400.0],
    );
    let mut model = WeaknessModel::new();
    model.update("abcd", &result);
    // mean: 100, then 0.2 * 200 + 0.8 * 100 = 120; the pause does not move it.
    close(model.mean_interval_ms().unwrap(), 120.0);
    assert_eq!(stat(&model, "a").slowness_samples, 0);
    let b = stat(&model, "b");
    assert_eq!(b.slowness_samples, 1);
    close(b.ema_slowness, 0.0);
    let c = stat(&model, "c");
    close(c.ema_slowness, 200.0 / 120.0 - 1.0);
    close(stat(&model, "bc").ema_slowness, 200.0 / 120.0 - 1.0);
    close(stat(&model, "ab").ema_slowness, 0.0);
    let d = stat(&model, "d");
    assert_eq!(d.samples, 1);
    assert_eq!(d.slowness_samples, 0);
    assert_eq!(stat(&model, "cd").slowness_samples, 0);
}

#[test]
fn slowness_ema_updates_with_alpha_and_persists_mean_across_sessions() {
    let mut model = WeaknessModel::new();
    model.update(
        "aaa",
        &session("aaa", ErrorMode::AllowContinue, "aaa", &[0.0, 100.0, 300.0]),
    );
    let sample = 200.0 / 120.0 - 1.0;
    let a = stat(&model, "a");
    assert_eq!(a.slowness_samples, 2);
    close(a.ema_slowness, ALPHA * sample);
    let aa = stat(&model, "aa");
    assert_eq!(aa.samples, 2);
    close(aa.ema_slowness, ALPHA * sample);
    // A new session continues the mean EMA: 0.2 * 120 + 0.8 * 120.
    model.update(
        "ab",
        &session("ab", ErrorMode::AllowContinue, "ab", &[0.0, 120.0]),
    );
    close(model.mean_interval_ms().unwrap(), 120.0);
    close(stat(&model, "b").ema_slowness, 0.0);
}

#[test]
fn slowness_sample_is_clamped_and_boundaries_respected() {
    // Intervals 100 then 1000: mean 280, 1000/280 - 1 > 1 → 1.
    let mut model = WeaknessModel::new();
    model.update(
        "abc",
        &session(
            "abc",
            ErrorMode::AllowContinue,
            "abc",
            &[0.0, 100.0, 1100.0],
        ),
    );
    close(stat(&model, "c").ema_slowness, 1.0);
    close(model.mean_interval_ms().unwrap(), 280.0);
    // Exactly PAUSE_MS counts; zero intervals are skipped.
    let mut edge = WeaknessModel::new();
    edge.update(
        "abc",
        &session(
            "abc",
            ErrorMode::AllowContinue,
            "abc",
            &[0.0, 2000.0, 2000.0],
        ),
    );
    close(edge.mean_interval_ms().unwrap(), 2000.0);
    assert_eq!(stat(&edge, "b").slowness_samples, 1);
    assert_eq!(stat(&edge, "c").slowness_samples, 0);
    assert_eq!(stat(&edge, "c").samples, 1);
    // Faster than the mean clamps to 0.
    let mut fast = WeaknessModel::new();
    fast.update(
        "abc",
        &session("abc", ErrorMode::AllowContinue, "abc", &[0.0, 500.0, 600.0]),
    );
    close(stat(&fast, "c").ema_slowness, 0.0);
}

#[test]
fn stop_on_error_counts_every_record_at_a_position() {
    let result = session("ab", ErrorMode::StopOnError, "xab", &[0.0, 100.0, 200.0]);
    assert_eq!(result.records.len(), 3);
    let mut model = WeaknessModel::new();
    model.update("ab", &result);
    let a = stat(&model, "a");
    assert_eq!(a.samples, 2);
    close(a.ema_error, 0.8);
    assert_eq!(stat(&model, "ab").samples, 1);
    assert_eq!(
        model.confusions(),
        vec![Confusion {
            expected: 'a',
            typed: 'x',
            count: 1
        }]
    );
}

#[test]
fn confusions_count_pairs_ordered_by_count_then_pair() {
    let mut model = trained("nnnmmba", "mmmnnvs");
    model.update("n", &steady("n", "m"));
    let pair = |expected, typed, count| Confusion {
        expected,
        typed,
        count,
    };
    assert_eq!(
        model.confusions(),
        vec![
            pair('n', 'm', 4),
            pair('m', 'n', 2),
            pair('a', 's', 1),
            pair('b', 'v', 1),
        ]
    );
    assert!(trained("ab", "ab").confusions().is_empty());
}

#[test]
fn ranked_requires_min_samples_and_orders_by_score_then_unit() {
    let model = WeaknessModel::from_parts(
        vec![
            ("z".into(), unit_stat(0.5, 0.0, 3)),
            ("b".into(), unit_stat(0.5, 0.0, 9)),
            ("qu".into(), unit_stat(0.9, 0.0, 4)),
            ("x".into(), unit_stat(1.0, 1.0, 2)),
            ("c".into(), unit_stat(0.1, 0.0, 3)),
        ],
        Vec::new(),
        None,
    );
    let ranked = model.ranked();
    let names: Vec<&str> = ranked.iter().map(|(u, _)| u.as_str()).collect();
    assert_eq!(names, vec!["qu", "b", "z", "c"]);
    close(ranked[0].1, 0.7 * 0.9);
    assert!(WeaknessModel::new().ranked().is_empty());
}

#[test]
fn char_scores_cover_single_chars_at_any_sample_count() {
    let model = WeaknessModel::from_parts(
        vec![
            ("ñ".into(), unit_stat(1.0, 0.0, 1)),
            ("ab".into(), unit_stat(1.0, 0.0, 5)),
            (" ".into(), unit_stat(0.0, 1.0, 4)),
            ("a".into(), unit_stat(0.5, 0.5, 2)),
        ],
        Vec::new(),
        None,
    );
    let scores = model.char_scores();
    let chars: Vec<char> = scores.iter().map(|(c, _)| *c).collect();
    assert_eq!(chars, vec![' ', 'a', 'ñ']);
    close(scores[0].1, 0.3);
    close(scores[1].1, 0.5);
    close(scores[2].1, 0.7);
}

#[test]
fn from_parts_round_trips_through_accessors_sorted() {
    let units = vec![
        ("rr".to_string(), unit_stat(0.4, 0.2, 3)),
        ("a".to_string(), unit_stat(0.1, 0.3, 7)),
    ];
    let confusions = vec![
        Confusion {
            expected: 'n',
            typed: 'm',
            count: 1,
        },
        Confusion {
            expected: 'e',
            typed: 'r',
            count: 5,
        },
    ];
    let model = WeaknessModel::from_parts(units.clone(), confusions.clone(), Some(150.0));
    assert_eq!(model.units(), vec![units[1].clone(), units[0].clone()]);
    assert_eq!(
        model.confusions(),
        vec![confusions[1].clone(), confusions[0].clone()]
    );
    assert_eq!(model.mean_interval_ms(), Some(150.0));
    let rebuilt = WeaknessModel::from_parts(model.units(), model.confusions(), Some(150.0));
    assert_eq!(rebuilt, model);
    assert_eq!(WeaknessModel::default(), WeaknessModel::new());
}

#[test]
fn serde_round_trip_uses_camel_case() {
    let mut model = trained("ñandú", "ñamdú");
    model.update(
        "ab",
        &session("ab", ErrorMode::AllowContinue, "ab", &[0.0, 150.0]),
    );
    let json = serde_json::to_string(&model).unwrap();
    for key in [
        "\"meanIntervalMs\"",
        "\"emaError\"",
        "\"emaSlowness\"",
        "\"slownessSamples\"",
        "\"expected\"",
        "\"typed\"",
        "\"count\"",
    ] {
        assert!(json.contains(key), "{key} missing in {json}");
    }
    let back: WeaknessModel = serde_json::from_str(&json).unwrap();
    assert_eq!(back, model);
    assert_eq!(back.confusions(), model.confusions());
}

#[test]
fn select_targets_on_empty_or_ineligible_model_is_empty() {
    assert!(WeaknessModel::new().select_targets(3, 7).is_empty());
    assert!(trained("ab", "xx").select_targets(3, 7).is_empty());
}

#[test]
fn select_targets_clamps_count_to_one_through_five() {
    let model = twelve_units();
    assert_eq!(model.select_targets(0, 1).len(), 1);
    assert_eq!(model.select_targets(1, 1).len(), 1);
    assert_eq!(model.select_targets(4, 1).len(), 4);
    assert_eq!(model.select_targets(5, 1).len(), 5);
    assert_eq!(model.select_targets(99, 1).len(), 5);
}

#[test]
fn select_targets_is_deterministic_per_seed() {
    let model = twelve_units();
    for seed in [0, 1, 42, u64::MAX] {
        assert_eq!(model.select_targets(3, seed), model.select_targets(3, seed));
    }
}

#[test]
fn select_targets_varies_across_seeds() {
    let model = twelve_units();
    let distinct: std::collections::BTreeSet<Vec<String>> =
        (0..50).map(|seed| model.select_targets(3, seed)).collect();
    assert!(distinct.len() > 1, "always picked {distinct:?}");
}

#[test]
fn select_targets_are_distinct_sorted_and_within_pool() {
    let model = twelve_units();
    for count in 1..=5usize {
        let pool: Vec<String> = model
            .ranked()
            .into_iter()
            .take(2 * count)
            .map(|(u, _)| u)
            .collect();
        for seed in 0..200 {
            let picked = model.select_targets(count, seed);
            assert_eq!(picked.len(), count);
            let set: std::collections::BTreeSet<&String> = picked.iter().collect();
            assert_eq!(set.len(), count, "duplicates in {picked:?}");
            assert!(
                picked.iter().all(|u| pool.contains(u)),
                "{picked:?} outside {pool:?}"
            );
            let scores: Vec<f64> = picked.iter().map(|u| score_of(&model, u)).collect();
            assert!(
                scores.windows(2).all(|w| w[0] >= w[1]),
                "{picked:?} unsorted"
            );
        }
    }
}

#[test]
fn select_targets_can_pick_zero_score_units() {
    let units = ["a", "b", "c", "d"]
        .iter()
        .map(|u| (u.to_string(), unit_stat(0.0, 0.0, 3)))
        .chain([("e".to_string(), unit_stat(1.0, 1.0, 3))])
        .collect();
    let model = WeaknessModel::from_parts(units, Vec::new(), None);
    let picked_zero = (0..100).any(|seed| model.select_targets(2, seed).iter().any(|u| u != "e"));
    assert!(picked_zero);
}

#[test]
fn select_targets_with_few_candidates_returns_all_sorted() {
    let model = WeaknessModel::from_parts(
        vec![
            ("a".into(), unit_stat(0.2, 0.0, 3)),
            ("b".into(), unit_stat(0.9, 0.0, 3)),
            ("c".into(), unit_stat(1.0, 0.0, 2)),
        ],
        Vec::new(),
        None,
    );
    for seed in 0..10 {
        assert_eq!(model.select_targets(3, seed), vec!["b", "a"]);
        assert_eq!(model.select_targets(2, seed), vec!["b", "a"]);
    }
}
