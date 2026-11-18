//! Daily goal and streak (phase 3, S1–S4), testable without a Tauri runtime.
//!
//! The goal lives in the active profile's `goals` column (in memory in
//! temporary mode, like the sessions it counts). Lock order: the store, then
//! the active profile.

use std::collections::HashMap;

use plexustyping_store::{DailyGoal, GoalKind, SessionSummary, StoreError};
use serde::Serialize;

use crate::commands::{AppState, lock_store};
use crate::history::MAX_TZ_OFFSET_MIN;

const DAY_MS: i64 = 86_400_000;
const MINUTE_MS: f64 = 60_000.0;

/// `goal_status`: the active profile's goal, today's progress and the streak.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GoalStatus {
    /// `null` when off.
    pub goal: Option<DailyGoal>,
    /// Today's whole minutes (rounded down) or texts; 0 without a goal.
    pub today: u32,
    /// Whether today's goal is reached.
    pub met: bool,
    /// Consecutive local days on which the current goal was met (S3).
    pub streak: u32,
}

/// The goal of `goal_set(kind, amount)`: `"off"` without amount, or
/// `"minutes"` / `"texts"` with an allowed amount; else "Meta diaria no válida".
pub fn parse_goal(kind: &str, amount: Option<u32>) -> Result<Option<DailyGoal>, String> {
    let kind = match (kind, amount) {
        ("off", None) => return Ok(None),
        ("minutes", Some(_)) => GoalKind::Minutes,
        ("texts", Some(_)) => GoalKind::Texts,
        _ => return Err(StoreError::InvalidGoal.to_string()),
    };
    let amount = amount.unwrap_or_default();
    DailyGoal::new(kind, amount)
        .map(Some)
        .map_err(|e| e.to_string())
}

/// `goal_set`: sets the active profile's goal and returns its status;
/// a refused goal or time zone changes nothing.
pub fn set(
    state: &AppState,
    kind: &str,
    amount: Option<u32>,
    tz_offset_min: i32,
    now_ms: i64,
) -> Result<GoalStatus, String> {
    let goal = parse_goal(kind, amount)?;
    check_tz(tz_offset_min)?;
    let store = lock_store(&state.store);
    let profile = state.active_profile();
    store
        .set_daily_goal(profile.id, goal.as_ref())
        .map_err(|e| e.to_string())?;
    let sessions = store
        .sessions(profile.id, None, None)
        .map_err(|e| e.to_string())?;
    Ok(progress(goal, &sessions, tz_offset_min, now_ms))
}

/// `goal_status` of the active profile at `now_ms`; `tz_offset_min` is minutes
/// east of UTC (`-new Date().getTimezoneOffset()`).
pub fn status(state: &AppState, tz_offset_min: i32, now_ms: i64) -> Result<GoalStatus, String> {
    check_tz(tz_offset_min)?;
    let store = lock_store(&state.store);
    let profile = state.active_profile();
    let goal = store.daily_goal(profile.id).map_err(|e| e.to_string())?;
    let sessions = match goal {
        Some(_) => store
            .sessions(profile.id, None, None)
            .map_err(|e| e.to_string())?,
        None => Vec::new(),
    };
    Ok(progress(goal, &sessions, tz_offset_min, now_ms))
}

fn check_tz(tz_offset_min: i32) -> Result<(), String> {
    if (-MAX_TZ_OFFSET_MIN..=MAX_TZ_OFFSET_MIN).contains(&tz_offset_min) {
        Ok(())
    } else {
        Err(format!("Zona horaria no válida: {tz_offset_min}"))
    }
}

/// Local day number (days since 1970-01-01 in the local calendar) of `ms`.
pub(crate) fn local_day(ms: i64, tz_offset_min: i32) -> i64 {
    (ms + i64::from(tz_offset_min) * 60_000).div_euclid(DAY_MS)
}

/// Practice of one local day: summed duration and number of sessions.
#[derive(Clone, Copy, Default)]
struct DayTotal {
    duration_ms: f64,
    texts: u32,
}

impl DayTotal {
    fn meets(&self, goal: &DailyGoal) -> bool {
        match goal.kind {
            GoalKind::Minutes => self.duration_ms >= f64::from(goal.amount) * MINUTE_MS,
            GoalKind::Texts => self.texts >= goal.amount,
        }
    }

    fn shown(&self, goal: &DailyGoal) -> u32 {
        match goal.kind {
            GoalKind::Minutes => (self.duration_ms / MINUTE_MS).floor() as u32,
            GoalKind::Texts => self.texts,
        }
    }
}

/// Today's progress and the streak of `goal` over `sessions` (S2–S3).
pub fn progress(
    goal: Option<DailyGoal>,
    sessions: &[SessionSummary],
    tz_offset_min: i32,
    now_ms: i64,
) -> GoalStatus {
    let Some(daily) = goal else {
        return GoalStatus {
            goal: None,
            today: 0,
            met: false,
            streak: 0,
        };
    };
    let mut days: HashMap<i64, DayTotal> = HashMap::new();
    for s in sessions {
        let total = days
            .entry(local_day(s.started_at_ms, tz_offset_min))
            .or_default();
        total.duration_ms += s.duration_ms.max(0.0);
        total.texts += 1;
    }
    let today = local_day(now_ms, tz_offset_min);
    let met_on = |day: i64| days.get(&day).is_some_and(|t| t.meets(&daily));
    let met = met_on(today);
    let mut day = if met { today } else { today - 1 };
    let mut streak = 0;
    while met_on(day) {
        streak += 1;
        day -= 1;
    }
    GoalStatus {
        goal,
        today: days.get(&today).map_or(0, |t| t.shown(&daily)),
        met,
        streak,
    }
}

#[cfg(test)]
mod tests;
