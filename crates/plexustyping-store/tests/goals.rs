//! Phase 3 daily goal (S1): the goal of each profile, through the public API.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};

use plexustyping_store::{DailyGoal, GoalKind, MINUTE_GOALS, Store, StoreError, TEXT_GOALS};

/// Fresh directory under the system temp dir, removed on drop.
struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        TempDir(std::env::temp_dir().join(format!(
            "plexustyping-store-goals-{}-{nanos}-{n}",
            std::process::id()
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

fn raw_goals(path: &std::path::Path, id: i64) -> Option<String> {
    let conn = rusqlite::Connection::open(path).unwrap();
    conn.query_row("SELECT goals FROM profiles WHERE id = ?1", [id], |r| {
        r.get(0)
    })
    .unwrap()
}

fn set_raw_goals(path: &std::path::Path, id: i64, value: Option<&str>) {
    let conn = rusqlite::Connection::open(path).unwrap();
    conn.execute(
        "UPDATE profiles SET goals = ?2 WHERE id = ?1",
        rusqlite::params![id, value],
    )
    .unwrap();
}

#[test]
fn the_allowed_amounts_are_exactly_those_of_s1() {
    assert_eq!(MINUTE_GOALS, [5, 10, 15, 20, 30, 45, 60, 90, 120]);
    assert_eq!(TEXT_GOALS, [1, 3, 5, 10, 15, 20, 30]);
    for amount in MINUTE_GOALS {
        let goal = DailyGoal::new(GoalKind::Minutes, amount).unwrap();
        assert_eq!((goal.kind, goal.amount), (GoalKind::Minutes, amount));
    }
    for amount in TEXT_GOALS {
        let goal = DailyGoal::new(GoalKind::Texts, amount).unwrap();
        assert_eq!((goal.kind, goal.amount), (GoalKind::Texts, amount));
    }
}

#[test]
fn other_amounts_are_refused_with_the_spanish_message() {
    for amount in [0, 1, 3, 25, 121, 1000] {
        let err = DailyGoal::new(GoalKind::Minutes, amount).unwrap_err();
        assert!(matches!(err, StoreError::InvalidGoal), "{amount}: {err:?}");
        assert_eq!(err.to_string(), "Meta diaria no válida");
    }
    for amount in [0, 2, 4, 25, 31, 120] {
        let err = DailyGoal::new(GoalKind::Texts, amount).unwrap_err();
        assert!(matches!(err, StoreError::InvalidGoal), "{amount}: {err:?}");
    }
}

#[test]
fn a_new_profile_has_no_goal() {
    let store = Store::open_in_memory().unwrap();
    let main = store.default_profile().unwrap();
    let other = store.create_profile("Otro", "es", "es-qwerty").unwrap();
    assert_eq!(store.daily_goal(main.id).unwrap(), None);
    assert_eq!(store.daily_goal(other.id).unwrap(), None);
}

#[test]
fn the_goal_is_stored_as_json_and_read_back_after_reopening() {
    let tmp = TempDir::new();
    let id;
    {
        let store = Store::open(&tmp.db()).unwrap();
        id = store.default_profile().unwrap().id;
        let goal = DailyGoal::new(GoalKind::Minutes, 20).unwrap();
        store.set_daily_goal(id, Some(&goal)).unwrap();
    }
    let raw: serde_json::Value = serde_json::from_str(&raw_goals(&tmp.db(), id).unwrap()).unwrap();
    assert_eq!(
        raw,
        serde_json::json!({"daily": {"kind": "minutes", "amount": 20}})
    );
    let store = Store::open(&tmp.db()).unwrap();
    assert_eq!(
        store.daily_goal(id).unwrap(),
        Some(DailyGoal::new(GoalKind::Minutes, 20).unwrap())
    );
    // Off is stored as a null daily goal.
    store.set_daily_goal(id, None).unwrap();
    assert_eq!(store.daily_goal(id).unwrap(), None);
    drop(store);
    let raw: serde_json::Value = serde_json::from_str(&raw_goals(&tmp.db(), id).unwrap()).unwrap();
    assert_eq!(raw, serde_json::json!({"daily": null}));
}

#[test]
fn null_absent_or_unreadable_goals_count_as_off_and_are_not_overwritten_by_reading() {
    let tmp = TempDir::new();
    let id = Store::open(&tmp.db())
        .unwrap()
        .default_profile()
        .unwrap()
        .id;
    for raw in [
        None,
        Some("null"),
        Some("{}"),
        Some(r#"{"daily":null}"#),
        Some("not json"),
        Some(r#"{"daily":{"kind":"minutes","amount":7}}"#),
        Some(r#"{"daily":{"kind":"pages","amount":5}}"#),
        Some(r#"{"daily":{"kind":"texts"}}"#),
        Some("[1,2]"),
    ] {
        set_raw_goals(&tmp.db(), id, raw);
        let store = Store::open(&tmp.db()).unwrap();
        assert_eq!(store.daily_goal(id).unwrap(), None, "{raw:?}");
        drop(store);
        assert_eq!(raw_goals(&tmp.db(), id).as_deref(), raw, "{raw:?}");
    }
    // A new goal replaces the unreadable value.
    set_raw_goals(&tmp.db(), id, Some("not json"));
    let store = Store::open(&tmp.db()).unwrap();
    let goal = DailyGoal::new(GoalKind::Texts, 3).unwrap();
    store.set_daily_goal(id, Some(&goal)).unwrap();
    assert_eq!(store.daily_goal(id).unwrap(), Some(goal));
}

#[test]
fn a_refused_goal_leaves_the_stored_one_unchanged() {
    let tmp = TempDir::new();
    let store = Store::open(&tmp.db()).unwrap();
    let id = store.default_profile().unwrap().id;
    let goal = DailyGoal::new(GoalKind::Texts, 5).unwrap();
    store.set_daily_goal(id, Some(&goal)).unwrap();
    let before = raw_goals(&tmp.db(), id);
    let bad = DailyGoal {
        kind: GoalKind::Minutes,
        amount: 7,
    };
    let err = store.set_daily_goal(id, Some(&bad)).unwrap_err();
    assert!(matches!(err, StoreError::InvalidGoal), "{err:?}");
    assert_eq!(raw_goals(&tmp.db(), id), before);
    assert_eq!(store.daily_goal(id).unwrap(), Some(goal));
}

#[test]
fn an_unknown_profile_is_refused() {
    let store = Store::open_in_memory().unwrap();
    store.default_profile().unwrap();
    let goal = DailyGoal::new(GoalKind::Texts, 5).unwrap();
    assert!(matches!(
        store.set_daily_goal(4242, Some(&goal)),
        Err(StoreError::ProfileNotFound)
    ));
    assert!(matches!(
        store.daily_goal(4242),
        Err(StoreError::ProfileNotFound)
    ));
}

#[test]
fn goals_are_independent_per_profile() {
    let store = Store::open_in_memory().unwrap();
    let main = store.default_profile().unwrap();
    let other = store.create_profile("Otro", "es", "es-qwerty").unwrap();
    let minutes = DailyGoal::new(GoalKind::Minutes, 30).unwrap();
    let texts = DailyGoal::new(GoalKind::Texts, 10).unwrap();
    store.set_daily_goal(main.id, Some(&minutes)).unwrap();
    assert_eq!(store.daily_goal(other.id).unwrap(), None);
    store.set_daily_goal(other.id, Some(&texts)).unwrap();
    assert_eq!(store.daily_goal(main.id).unwrap(), Some(minutes));
    store.set_daily_goal(main.id, None).unwrap();
    assert_eq!(store.daily_goal(main.id).unwrap(), None);
    assert_eq!(store.daily_goal(other.id).unwrap(), Some(texts));
    // Profile updates do not touch the goal.
    let changes = plexustyping_store::ProfileUpdate {
        name: Some("Renombrado".to_owned()),
        ..Default::default()
    };
    store.update_profile(other.id, &changes).unwrap();
    assert_eq!(store.daily_goal(other.id).unwrap(), Some(texts));
}

#[test]
fn the_goal_serializes_in_camel_case_lowercase_kinds() {
    let goal = DailyGoal::new(GoalKind::Texts, 3).unwrap();
    assert_eq!(
        serde_json::to_value(goal).unwrap(),
        serde_json::json!({"kind": "texts", "amount": 3})
    );
}
