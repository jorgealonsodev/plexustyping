//! S1–S4: daily goal, today's progress and streak, through the plain command functions.

use std::path::PathBuf;
use std::sync::Arc;

use plexustyping_ai::{AiClient, Endpoints};
use plexustyping_core::ErrorMode;
use plexustyping_store::{DailyGoal, GoalKind, NewSession};
use serde_json::json;

use super::*;
use crate::commands::{lock_store, open_state};
use crate::profiles;
use crate::secrets::{NoStore, SharedSecrets};

/// 2026-11-18T00:00:00Z.
const MIDNIGHT_UTC: i64 = 1_794_960_000_000;
const MINUTE: i64 = 60_000;
const HOUR: i64 = 60 * MINUTE;
const DAY: i64 = 24 * HOUR;
/// 2026-11-18T12:00:00Z.
const NOON_UTC: i64 = MIDNIGHT_UTC + 12 * HOUR;

/// Fresh directory under the system temp dir, removed on drop.
struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        use std::sync::atomic::{AtomicU32, Ordering};
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        TempDir(std::env::temp_dir().join(format!(
            "plexustyping-app-goals-{}-{}-{n}",
            std::process::id(),
            std::time::UNIX_EPOCH.elapsed().unwrap().as_nanos()
        )))
    }

    fn db(&self) -> PathBuf {
        self.0.join("plexustyping.sqlite3")
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn open(path: Result<PathBuf, String>) -> AppState {
    let secrets: SharedSecrets = Arc::new(NoStore);
    open_state(
        path,
        secrets,
        AiClient::new(Endpoints::loopback("http://127.0.0.1:9")),
    )
}

/// A temporary (in-memory) state.
fn memory() -> AppState {
    open(Err("test: no data dir".to_owned()))
}

/// Stores a session of the active profile started at `started_at_ms` lasting `duration_ms`.
fn session_at(state: &AppState, mode: &str, started_at_ms: i64, duration_ms: f64) {
    let mut store = lock_store(&state.store);
    let profile_id = state.active_profile().id;
    store
        .save_session(&NewSession {
            profile_id,
            started_at_ms,
            mode,
            code_language: (mode == "code").then_some("rust"),
            language: Some("es"),
            layout: Some("es-qwerty"),
            error_mode: ErrorMode::AllowContinue,
            text: "ab",
            duration_ms,
            gross_wpm: 30.0,
            net_wpm: 30.0,
            accuracy: 1.0,
            keystrokes: 2,
            bpm: None,
            records: &[],
            metronome_offsets_ms: &[],
        })
        .unwrap();
}

fn text_at(state: &AppState, started_at_ms: i64) {
    session_at(state, "prose", started_at_ms, 1_000.0);
}

fn goal(kind: GoalKind, amount: u32) -> Option<DailyGoal> {
    Some(DailyGoal::new(kind, amount).unwrap())
}

fn stored_goal(state: &AppState) -> Option<DailyGoal> {
    let store = lock_store(&state.store);
    store.daily_goal(state.active_profile().id).unwrap()
}

fn expect(goal: Option<DailyGoal>, today: u32, met: bool, streak: u32) -> GoalStatus {
    GoalStatus {
        goal,
        today,
        met,
        streak,
    }
}

// ---- S1 / S4: goal_set ----

#[test]
fn every_allowed_goal_is_set_for_the_active_profile() {
    let state = memory();
    for amount in [5, 10, 15, 20, 30, 45, 60, 90, 120] {
        let status = set(&state, "minutes", Some(amount), 0, NOON_UTC).unwrap();
        assert_eq!(status.goal, goal(GoalKind::Minutes, amount));
        assert_eq!(stored_goal(&state), goal(GoalKind::Minutes, amount));
    }
    for amount in [1, 3, 5, 10, 15, 20, 30] {
        let status = set(&state, "texts", Some(amount), 0, NOON_UTC).unwrap();
        assert_eq!(status.goal, goal(GoalKind::Texts, amount));
        assert_eq!(stored_goal(&state), goal(GoalKind::Texts, amount));
    }
    let status = set(&state, "off", None, 0, NOON_UTC).unwrap();
    assert_eq!(status, expect(None, 0, false, 0));
    assert_eq!(stored_goal(&state), None);
}

#[test]
fn anything_outside_s1_is_refused_and_changes_nothing() {
    let state = memory();
    set(&state, "texts", Some(5), 0, NOON_UTC).unwrap();
    let before = status(&state, 0, NOON_UTC).unwrap();
    let refused: [(&str, Option<u32>); 12] = [
        ("minutes", None),
        ("minutes", Some(0)),
        ("minutes", Some(7)),
        ("minutes", Some(121)),
        ("texts", None),
        ("texts", Some(2)),
        ("texts", Some(45)),
        ("off", Some(5)),
        ("hours", Some(5)),
        ("Minutes", Some(20)),
        ("", None),
        ("texts", Some(u32::MAX)),
    ];
    for (kind, amount) in refused {
        let err = set(&state, kind, amount, 0, NOON_UTC).unwrap_err();
        assert_eq!(err, "Meta diaria no válida", "{kind:?} {amount:?}");
        assert_eq!(stored_goal(&state), goal(GoalKind::Texts, 5));
        assert_eq!(status(&state, 0, NOON_UTC).unwrap(), before);
    }
}

#[test]
fn an_invalid_time_zone_is_refused_and_changes_nothing() {
    let state = memory();
    set(&state, "texts", Some(5), 0, NOON_UTC).unwrap();
    assert_eq!(
        set(&state, "minutes", Some(20), 18 * 60 + 1, NOON_UTC).unwrap_err(),
        "Zona horaria no válida: 1081"
    );
    assert_eq!(stored_goal(&state), goal(GoalKind::Texts, 5));
    assert_eq!(
        status(&state, -18 * 60 - 1, NOON_UTC).unwrap_err(),
        "Zona horaria no válida: -1081"
    );
}

#[test]
fn goals_are_independent_per_profile_and_follow_the_switch() {
    let tmp = TempDir::new();
    let (main_id, work_id);
    {
        let state = open(Ok(tmp.db()));
        main_id = state.active_profile().id;
        let work = profiles::create(&state, "Trabajo", "es", "es-qwerty").unwrap();
        work_id = work.id;
        set(&state, "minutes", Some(20), 0, NOON_UTC).unwrap();
        text_at(&state, NOON_UTC - HOUR);
        profiles::switch(&state, work.id).unwrap();
        // The other profile has no goal and none of the first one's sessions.
        assert_eq!(
            status(&state, 0, NOON_UTC).unwrap(),
            expect(None, 0, false, 0)
        );
        set(&state, "texts", Some(1), 0, NOON_UTC).unwrap();
        assert_eq!(
            status(&state, 0, NOON_UTC).unwrap(),
            expect(goal(GoalKind::Texts, 1), 0, false, 0)
        );
    }
    let state = open(Ok(tmp.db()));
    assert_eq!(state.active_profile().id, work_id);
    assert_eq!(stored_goal(&state), goal(GoalKind::Texts, 1));
    profiles::switch(&state, main_id).unwrap();
    assert_eq!(
        status(&state, 0, NOON_UTC).unwrap(),
        expect(goal(GoalKind::Minutes, 20), 0, false, 0)
    );
}

// ---- S4: JSON shape ----

#[test]
fn the_status_serializes_in_camel_case() {
    let state = memory();
    let off = set(&state, "off", None, 0, NOON_UTC).unwrap();
    assert_eq!(
        serde_json::to_value(off).unwrap(),
        json!({"goal": null, "today": 0, "met": false, "streak": 0})
    );
    text_at(&state, NOON_UTC - HOUR);
    let on = set(&state, "texts", Some(1), 0, NOON_UTC).unwrap();
    assert_eq!(
        serde_json::to_value(on).unwrap(),
        json!({"goal": {"kind": "texts", "amount": 1}, "today": 1, "met": true, "streak": 1})
    );
}

// ---- S2: today's progress ----

#[test]
fn minutes_sum_the_durations_rounded_down_and_are_met_at_the_exact_sum() {
    let state = memory();
    set(&state, "minutes", Some(20), 0, NOON_UTC).unwrap();
    session_at(&state, "prose", MIDNIGHT_UTC + HOUR, (12 * MINUTE) as f64);
    session_at(&state, "code", MIDNIGHT_UTC + 2 * HOUR, 59_999.0);
    assert_eq!(
        status(&state, 0, NOON_UTC).unwrap(),
        expect(goal(GoalKind::Minutes, 20), 12, false, 0)
    );
    // 19:59.999 shows 19 and is not met.
    session_at(
        &state,
        "words",
        MIDNIGHT_UTC + 3 * HOUR,
        (7 * MINUTE) as f64,
    );
    assert_eq!(
        status(&state, 0, NOON_UTC).unwrap(),
        expect(goal(GoalKind::Minutes, 20), 19, false, 0)
    );
    // Exactly 20:00.000 is met.
    session_at(&state, "prose", MIDNIGHT_UTC + 4 * HOUR, 1.0);
    assert_eq!(
        status(&state, 0, NOON_UTC).unwrap(),
        expect(goal(GoalKind::Minutes, 20), 20, true, 1)
    );
}

#[test]
fn texts_count_the_sessions_of_every_mode() {
    let state = memory();
    set(&state, "texts", Some(3), 0, NOON_UTC).unwrap();
    session_at(&state, "prose", MIDNIGHT_UTC + HOUR, 500.0);
    session_at(&state, "code", MIDNIGHT_UTC + 2 * HOUR, 500.0);
    assert_eq!(
        status(&state, 0, NOON_UTC).unwrap(),
        expect(goal(GoalKind::Texts, 3), 2, false, 0)
    );
    session_at(&state, "words", MIDNIGHT_UTC + 3 * HOUR, 500.0);
    assert_eq!(
        status(&state, 0, NOON_UTC).unwrap(),
        expect(goal(GoalKind::Texts, 3), 3, true, 1)
    );
}

#[test]
fn a_positive_offset_moves_local_midnight_before_utc_midnight() {
    let state = memory();
    set(&state, "texts", Some(1), 0, NOON_UTC).unwrap();
    let tz = 120; // UTC+2: local 2026-11-18 starts at 2026-11-17T22:00Z.
    let local_midnight = MIDNIGHT_UTC - 2 * HOUR;
    text_at(&state, local_midnight - 1);
    assert_eq!(
        status(&state, tz, NOON_UTC).unwrap(),
        expect(goal(GoalKind::Texts, 1), 0, false, 1)
    );
    // In UTC both instants are the 17th: still yesterday.
    assert_eq!(status(&state, 0, NOON_UTC).unwrap().today, 0);
    text_at(&state, local_midnight);
    assert_eq!(
        status(&state, tz, NOON_UTC).unwrap(),
        expect(goal(GoalKind::Texts, 1), 1, true, 2)
    );
    assert_eq!(status(&state, 0, NOON_UTC).unwrap().today, 0);
}

#[test]
fn a_negative_offset_moves_local_midnight_after_utc_midnight() {
    let state = memory();
    set(&state, "texts", Some(1), 0, NOON_UTC).unwrap();
    let tz = -300; // UTC-5: local 2026-11-18 starts at 2026-11-18T05:00Z.
    let local_midnight = MIDNIGHT_UTC + 5 * HOUR;
    text_at(&state, local_midnight - 1);
    assert_eq!(
        status(&state, tz, NOON_UTC).unwrap(),
        expect(goal(GoalKind::Texts, 1), 0, false, 1)
    );
    // In UTC that instant is already today.
    assert_eq!(status(&state, 0, NOON_UTC).unwrap().today, 1);
    text_at(&state, local_midnight);
    assert_eq!(
        status(&state, tz, NOON_UTC).unwrap(),
        expect(goal(GoalKind::Texts, 1), 1, true, 2)
    );
    // Just before local midnight "now" is still the 17th: both sessions are after it.
    assert_eq!(
        status(&state, tz, local_midnight - 1).unwrap(),
        expect(goal(GoalKind::Texts, 1), 1, true, 1)
    );
}

// ---- S3: streak ----

#[test]
fn the_streak_ends_today_when_today_is_met() {
    let state = memory();
    set(&state, "texts", Some(1), 0, NOON_UTC).unwrap();
    for days_ago in [0, 1, 2, 4, 5] {
        text_at(&state, NOON_UTC - days_ago * DAY);
    }
    assert_eq!(
        status(&state, 0, NOON_UTC).unwrap(),
        expect(goal(GoalKind::Texts, 1), 1, true, 3)
    );
}

#[test]
fn the_streak_ends_yesterday_while_today_is_not_met() {
    let state = memory();
    set(&state, "texts", Some(3), 0, NOON_UTC).unwrap();
    for days_ago in [1, 2] {
        for i in 0..3 {
            text_at(&state, NOON_UTC - days_ago * DAY + i * MINUTE);
        }
    }
    assert_eq!(
        status(&state, 0, NOON_UTC).unwrap(),
        expect(goal(GoalKind::Texts, 3), 0, false, 2)
    );
    // Some practice today that does not reach the goal keeps yesterday's streak.
    text_at(&state, NOON_UTC - HOUR);
    assert_eq!(
        status(&state, 0, NOON_UTC).unwrap(),
        expect(goal(GoalKind::Texts, 3), 1, false, 2)
    );
}

#[test]
fn a_day_without_sessions_or_below_the_current_goal_breaks_the_streak() {
    let state = memory();
    set(&state, "texts", Some(1), 0, NOON_UTC).unwrap();
    // Nothing yesterday: the day before does not count.
    text_at(&state, NOON_UTC - 2 * DAY);
    assert_eq!(
        status(&state, 0, NOON_UTC).unwrap(),
        expect(goal(GoalKind::Texts, 1), 0, false, 0)
    );
    text_at(&state, NOON_UTC);
    assert_eq!(
        status(&state, 0, NOON_UTC).unwrap(),
        expect(goal(GoalKind::Texts, 1), 1, true, 1)
    );
    // The current goal is applied to past days: with 2 texts a day, none was met.
    let two = set(&state, "minutes", Some(5), 0, NOON_UTC).unwrap();
    assert_eq!(two, expect(goal(GoalKind::Minutes, 5), 0, false, 0));
}

#[test]
fn no_goal_means_no_progress_and_no_streak() {
    let state = memory();
    for days_ago in 0..5 {
        text_at(&state, NOON_UTC - days_ago * DAY);
    }
    assert_eq!(
        status(&state, 0, NOON_UTC).unwrap(),
        expect(None, 0, false, 0)
    );
}

// ---- S4: temporary mode ----

#[test]
fn the_temporary_mode_keeps_the_goal_and_sessions_in_memory() {
    let state = memory();
    assert!(!state.status.persistent);
    set(&state, "minutes", Some(5), 0, NOON_UTC).unwrap();
    session_at(&state, "prose", NOON_UTC - HOUR, (5 * MINUTE) as f64);
    assert_eq!(
        status(&state, 0, NOON_UTC).unwrap(),
        expect(goal(GoalKind::Minutes, 5), 5, true, 1)
    );
}
