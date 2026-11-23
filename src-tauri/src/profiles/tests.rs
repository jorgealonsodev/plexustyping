//! S2–S3: active profile and profile commands, through the plain functions.

use std::path::PathBuf;
use std::sync::Arc;

use plexustyping_ai::{AiClient, Endpoints};
use plexustyping_core::ErrorMode;
use plexustyping_store::{StatsSnapshot, Store, UnitStatRow};
use serde_json::json;

use super::*;
use crate::commands::{STATS_MODE, lock_store, open_state, score_session};
use crate::secrets::{NoStore, SharedSecrets};

/// A profile error as the Spanish interface shows it.
fn es(err: ProfileError) -> String {
    err.message(Locale::Es)
}

/// Fresh directory under the system temp dir, removed on drop.
struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        use std::sync::atomic::{AtomicU32, Ordering};
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        TempDir(std::env::temp_dir().join(format!(
            "plexustyping-app-profiles-{}-{}-{n}",
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

fn saved_active(state: &AppState) -> Option<String> {
    lock_store(&state.store)
        .get_setting(ACTIVE_PROFILE_KEY)
        .unwrap()
}

/// Profiles and the stored active id: what a rejected call must not change.
fn snapshot(state: &AppState) -> (ProfileList, Option<String>, Profile) {
    (
        list(state).unwrap(),
        saved_active(state),
        state.active_profile(),
    )
}

/// Stores one prose session for `profile`.
fn practise(state: &AppState, profile: &Profile) {
    let mut store = lock_store(&state.store);
    let events: Vec<plexustyping_core::InputEvent> = serde_json::from_value(json!([
        {"kind": "char", "ch": "a", "tMs": 0.0},
        {"kind": "char", "ch": "b", "tMs": 100.0}
    ]))
    .unwrap();
    let scored = score_session(
        &mut store,
        profile,
        "ab",
        ErrorMode::AllowContinue,
        &events,
        1,
    );
    assert!(scored.saved, "{scored:?}");
}

// ---- S2: active profile ----

#[test]
fn a_fresh_start_uses_the_default_profile_without_a_saved_id() {
    let tmp = TempDir::new();
    let state = open(Ok(tmp.db()));
    assert!(state.status.persistent);
    let active = state.active_profile();
    assert_eq!(active.name, "Principal");
    assert_eq!(active, lock_store(&state.store).default_profile().unwrap());
}

#[test]
fn the_switched_profile_is_active_again_after_reopening() {
    let tmp = TempDir::new();
    let work_id;
    {
        let state = open(Ok(tmp.db()));
        let work = create(&state, "Trabajo", "en", "us-qwerty").unwrap();
        work_id = work.id;
        assert_eq!(switch(&state, work.id).unwrap(), work);
        assert_eq!(state.active_profile(), work);
        assert_eq!(saved_active(&state), Some(work.id.to_string()));
    }
    let state = open(Ok(tmp.db()));
    assert_eq!(state.active_profile().id, work_id);
    assert_eq!(list(&state).unwrap().active, work_id);
}

#[test]
fn an_unparsable_or_deleted_saved_id_falls_back_to_the_default_profile() {
    let tmp = TempDir::new();
    let main_id;
    {
        let store = Store::open(&tmp.db()).unwrap();
        main_id = store.default_profile().unwrap().id;
        store
            .set_setting(ACTIVE_PROFILE_KEY, "not-a-number")
            .unwrap();
    }
    assert_eq!(open(Ok(tmp.db())).active_profile().id, main_id);
    {
        let store = Store::open(&tmp.db()).unwrap();
        let gone = store.create_profile("Borrado", "es", "es-qwerty").unwrap();
        store
            .set_setting(ACTIVE_PROFILE_KEY, &gone.id.to_string())
            .unwrap();
        store.delete_profile(gone.id).unwrap();
        // A well-formed id that never existed as well.
    }
    assert_eq!(open(Ok(tmp.db())).active_profile().id, main_id);
    {
        let store = Store::open(&tmp.db()).unwrap();
        store.set_setting(ACTIVE_PROFILE_KEY, "424242").unwrap();
    }
    assert_eq!(open(Ok(tmp.db())).active_profile().id, main_id);
}

#[test]
fn the_first_profile_is_the_fallback_when_principal_was_deleted() {
    let tmp = TempDir::new();
    let other_id;
    {
        let store = Store::open(&tmp.db()).unwrap();
        let main = store.default_profile().unwrap();
        other_id = store.create_profile("Otro", "es", "es-qwerty").unwrap().id;
        store.delete_profile(main.id).unwrap();
    }
    assert_eq!(open(Ok(tmp.db())).active_profile().id, other_id);
}

#[test]
fn deleting_the_active_profile_switches_to_the_first_remaining_one() {
    let state = memory();
    let main = state.active_profile();
    let a = create(&state, "A", "es", "es-qwerty").unwrap();
    let b = create(&state, "B", "en", "us-qwerty").unwrap();
    switch(&state, b.id).unwrap();
    let deleted = delete(&state, b.id).unwrap();
    assert_eq!(deleted.active, main);
    assert_eq!(state.active_profile(), main);
    assert_eq!(saved_active(&state), Some(main.id.to_string()));
    // Deleting the active first profile picks the next one by id.
    let deleted = delete(&state, main.id).unwrap();
    assert_eq!(deleted.active, a);
    assert_eq!(state.active_profile(), a);
    assert_eq!(saved_active(&state), Some(a.id.to_string()));
}

#[test]
fn deleting_another_profile_keeps_the_active_one_and_its_data() {
    let state = memory();
    let main = state.active_profile();
    let other = create(&state, "Otro", "es", "es-qwerty").unwrap();
    practise(&state, &main);
    practise(&state, &other);
    let stats = lock_store(&state.store)
        .load_stats(main.id, STATS_MODE)
        .unwrap();
    let deleted = delete(&state, other.id).unwrap();
    assert_eq!(deleted.active, main);
    assert_eq!(state.active_profile(), main);
    assert_eq!(saved_active(&state), None);
    let store = lock_store(&state.store);
    assert_eq!(store.sessions(main.id, None, None).unwrap().len(), 1);
    assert_eq!(store.load_stats(main.id, STATS_MODE).unwrap(), stats);
    assert_eq!(store.sessions(other.id, None, None).unwrap(), vec![]);
}

#[test]
fn the_temporary_mode_supports_profiles_in_memory() {
    let state = memory();
    assert!(!state.status.persistent);
    let work = create(&state, "Trabajo", "en", "us-qwerty").unwrap();
    assert_eq!(switch(&state, work.id).unwrap(), work);
    practise(&state, &state.active_profile());
    let store = lock_store(&state.store);
    assert_eq!(store.sessions(work.id, None, None).unwrap().len(), 1);
}

#[test]
fn sessions_and_stats_follow_the_active_profile() {
    let state = memory();
    let main = state.active_profile();
    let work = create(&state, "Trabajo", "es", "es-qwerty").unwrap();
    switch(&state, work.id).unwrap();
    practise(&state, &state.active_profile());
    let store = lock_store(&state.store);
    assert_eq!(store.sessions(work.id, None, None).unwrap().len(), 1);
    assert_eq!(store.sessions(main.id, None, None).unwrap(), vec![]);
    assert_eq!(
        store.load_stats(main.id, STATS_MODE).unwrap(),
        StatsSnapshot {
            units: Vec::<UnitStatRow>::new(),
            confusions: vec![],
            mean_interval_ms: None,
        }
    );
}

// ---- S3: commands ----

#[test]
fn profiles_list_has_the_camel_case_json_shape() {
    let state = memory();
    let main = state.active_profile();
    let work = create(&state, "Trabajo", "en", "us-dvorak").unwrap();
    let value = serde_json::to_value(list(&state).unwrap()).unwrap();
    assert_eq!(
        value,
        json!({
            "active": main.id,
            "profiles": [
                {"id": main.id, "name": "Principal", "language": "es", "layout": "es-qwerty",
                 "theme": null, "createdAtMs": main.created_at_ms},
                {"id": work.id, "name": "Trabajo", "language": "en", "layout": "us-dvorak",
                 "theme": null, "createdAtMs": work.created_at_ms}
            ]
        })
    );
    let deleted = serde_json::to_value(delete(&state, work.id).unwrap()).unwrap();
    assert_eq!(
        deleted,
        json!({"active": serde_json::to_value(&main).unwrap()})
    );
}

#[test]
fn profile_create_trims_the_name_and_does_not_switch() {
    let state = memory();
    let main = state.active_profile();
    let p = create(&state, "  Trabajo ", "en", "us-colemak").unwrap();
    assert_eq!(
        (p.name.as_str(), p.language.as_str(), p.layout.as_str()),
        ("Trabajo", "en", "us-colemak")
    );
    assert_eq!(p.theme, None);
    assert_eq!(state.active_profile(), main);
    assert_eq!(list(&state).unwrap().active, main.id);
    assert_eq!(saved_active(&state), None);
}

#[test]
fn profile_create_rejections_are_spanish_and_change_nothing() {
    let state = memory();
    create(&state, "Otro", "es", "es-qwerty").unwrap();
    let before = snapshot(&state);
    let cases = [
        (
            "   ",
            "es",
            "es-qwerty",
            "El nombre del perfil no puede estar vacío",
        ),
        (
            &*"x".repeat(41),
            "es",
            "es-qwerty",
            "El nombre del perfil admite como máximo 40 caracteres",
        ),
        (
            "OTRO",
            "es",
            "es-qwerty",
            "Ya existe un perfil con ese nombre",
        ),
        ("Nuevo", "xx", "es-qwerty", "Idioma desconocido"),
        ("Nuevo", "es", "xx-qwerty", "Distribución desconocida"),
    ];
    for (name, language, layout, message) in cases {
        assert_eq!(
            create(&state, name, language, layout).map_err(es),
            Err(message.to_owned()),
            "{name:?} {language} {layout}"
        );
    }
    assert_eq!(snapshot(&state), before);
}

#[test]
fn profile_update_changes_the_given_fields_and_the_active_copy() {
    let state = memory();
    let main = state.active_profile();
    let p = update(
        &state,
        main.id,
        Some(" Casa ".into()),
        Some("en".into()),
        Some("us-qwerty".into()),
        Some("nord".into()),
    )
    .unwrap();
    let expected = Profile {
        name: "Casa".into(),
        language: "en".into(),
        layout: "us-qwerty".into(),
        theme: Some("nord".into()),
        ..main.clone()
    };
    assert_eq!(p, expected);
    assert_eq!(state.active_profile(), expected);
    assert_eq!(list(&state).unwrap().profiles, vec![expected.clone()]);
}

#[test]
fn profile_update_theme_null_leaves_it_and_empty_clears_it() {
    let state = memory();
    let main = state.active_profile();
    update(&state, main.id, None, None, None, Some("nord".into())).unwrap();
    let kept = update(&state, main.id, None, None, None, None).unwrap();
    assert_eq!(kept.theme.as_deref(), Some("nord"));
    assert_eq!(
        kept,
        Profile {
            theme: Some("nord".into()),
            ..main.clone()
        }
    );
    let cleared = update(&state, main.id, None, None, None, Some(String::new())).unwrap();
    assert_eq!(cleared.theme, None);
    assert_eq!(state.active_profile().theme, None);
    assert_eq!(list(&state).unwrap().profiles[0].theme, None);
}

#[test]
fn profile_update_of_another_profile_leaves_the_active_one() {
    let state = memory();
    let main = state.active_profile();
    let other = create(&state, "Otro", "es", "es-qwerty").unwrap();
    let renamed = update(&state, other.id, Some("Segundo".into()), None, None, None).unwrap();
    assert_eq!(renamed.name, "Segundo");
    assert_eq!(state.active_profile(), main);
}

#[test]
fn profile_update_rejections_are_spanish_and_change_nothing() {
    let state = memory();
    let main = state.active_profile();
    create(&state, "Otro", "es", "es-qwerty").unwrap();
    let before = snapshot(&state);
    let name = |n: &str| Some(n.to_owned());
    let cases = [
        (
            main.id,
            name(""),
            None,
            None,
            "El nombre del perfil no puede estar vacío",
        ),
        (
            main.id,
            name(&"x".repeat(41)),
            None,
            None,
            "El nombre del perfil admite como máximo 40 caracteres",
        ),
        (
            main.id,
            name("otro"),
            None,
            None,
            "Ya existe un perfil con ese nombre",
        ),
        (main.id, None, name("xx"), None, "Idioma desconocido"),
        (main.id, None, None, name("xx"), "Distribución desconocida"),
        (9_999, name("Nuevo"), None, None, "Ese perfil no existe"),
    ];
    for (id, n, language, layout, message) in cases {
        assert_eq!(
            update(&state, id, n, language, layout, Some("nord".into())).map_err(es),
            Err(message.to_owned())
        );
    }
    assert_eq!(snapshot(&state), before);
}

#[test]
fn profile_delete_rejections_are_spanish_and_change_nothing() {
    let state = memory();
    let main = state.active_profile();
    practise(&state, &main);
    let before = snapshot(&state);
    assert_eq!(
        delete(&state, main.id).map_err(es),
        Err("No se puede borrar el único perfil".to_owned())
    );
    assert_eq!(
        delete(&state, 9_999).map_err(es),
        Err("Ese perfil no existe".to_owned())
    );
    assert_eq!(snapshot(&state), before);
    let store = lock_store(&state.store);
    assert_eq!(store.sessions(main.id, None, None).unwrap().len(), 1);
}

#[test]
fn profile_switch_to_an_unknown_id_is_refused_and_changes_nothing() {
    let state = memory();
    let before = snapshot(&state);
    assert_eq!(
        switch(&state, 9_999).map_err(es),
        Err("Ese perfil no existe".to_owned())
    );
    assert_eq!(snapshot(&state), before);
}

#[test]
fn profile_errors_follow_the_interface_language() {
    let state = memory();
    let main = state.active_profile();
    let en = |err: ProfileError| err.message(Locale::En);
    assert_eq!(
        create(&state, "", "es", "es-qwerty").map_err(en),
        Err("The profile name cannot be empty".to_owned())
    );
    assert_eq!(
        create(&state, "Nuevo", "xx", "es-qwerty").map_err(en),
        Err("Unknown language".to_owned())
    );
    assert_eq!(
        update(&state, main.id, None, None, Some("xx".into()), None).map_err(en),
        Err("Unknown layout".to_owned())
    );
    assert_eq!(
        delete(&state, main.id).map_err(|e| e.message(Locale::De)),
        Err("Das einzige Profil kann nicht gelöscht werden".to_owned())
    );
    assert_eq!(
        switch(&state, 9_999).map_err(|e| e.message(Locale::Pt)),
        Err("Esse perfil não existe".to_owned())
    );
}
