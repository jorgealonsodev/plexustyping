//! S1–S5: UI settings over the `settings` table, through the plain command functions.

use std::path::PathBuf;
use std::sync::Arc;

use plexustyping_ai::{AiClient, Endpoints};

use super::*;
use crate::commands::{lock_store, open_state};
use crate::secrets::{NoStore, SharedSecrets};

/// Fresh directory under the system temp dir, removed on drop.
struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        use std::sync::atomic::{AtomicU32, Ordering};
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        TempDir(std::env::temp_dir().join(format!(
            "plexustyping-app-ui-settings-{}-{}-{n}",
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

#[test]
fn a_new_store_has_no_ui_settings() {
    let tmp = TempDir::new();
    let state = open(Ok(tmp.db()));
    assert_eq!(get(Locale::Es, &state), Ok(HashMap::new()));
}

#[test]
fn set_get_and_remove_round_trip_under_the_reserved_prefix() {
    let tmp = TempDir::new();
    let state = open(Ok(tmp.db()));
    set(Locale::Es, &state, "plexustyping.layout", "us-qwerty").unwrap();
    set(Locale::Es, &state, "plexustyping.theme", "dark").unwrap();
    set(Locale::Es, &state, "plexustyping.theme", "light").unwrap();
    {
        let store = lock_store(&state.store);
        assert_eq!(
            store
                .get_setting("ui:plexustyping.layout")
                .unwrap()
                .as_deref(),
            Some("us-qwerty")
        );
        // An existing backend setting is neither listed nor touched.
        store.set_setting("plexustyping.fake", "backend").unwrap();
    }
    let expected = HashMap::from([
        ("plexustyping.layout".to_owned(), "us-qwerty".to_owned()),
        ("plexustyping.theme".to_owned(), "light".to_owned()),
    ]);
    assert_eq!(get(Locale::Es, &state), Ok(expected));
    remove(Locale::Es, &state, "plexustyping.theme").unwrap();
    remove(Locale::Es, &state, "plexustyping.absent").unwrap();
    assert_eq!(
        get(Locale::Es, &state),
        Ok(HashMap::from([(
            "plexustyping.layout".to_owned(),
            "us-qwerty".to_owned()
        )]))
    );
    assert_eq!(
        lock_store(&state.store)
            .get_setting("plexustyping.fake")
            .unwrap()
            .as_deref(),
        Some("backend")
    );
}

#[test]
fn a_write_is_on_disk_when_it_returns() {
    let tmp = TempDir::new();
    let state = open(Ok(tmp.db()));
    set(Locale::Es, &state, "plexustyping.keyboard", "false").unwrap();
    // A second connection (like the next start) sees it without closing the first.
    let other = open(Ok(tmp.db()));
    assert_eq!(
        get(Locale::Es, &other)
            .unwrap()
            .get("plexustyping.keyboard")
            .map(String::as_str),
        Some("false")
    );
}

#[test]
fn keys_outside_plexustyping_are_rejected_and_nothing_is_stored() {
    let tmp = TempDir::new();
    let state = open(Ok(tmp.db()));
    for key in ["ai", "theme", "plexustyping.", "Plexustyping.theme", ""] {
        assert_eq!(
            set(Locale::Es, &state, key, "x"),
            Err(format!("Ajuste no válido: «{key}»")),
            "{key}"
        );
        assert_eq!(
            remove(Locale::Es, &state, key),
            Err(format!("Ajuste no válido: «{key}»"))
        );
    }
    let long = format!("plexustyping.{}", "a".repeat(MAX_KEY_BYTES));
    assert_eq!(
        set(Locale::Es, &state, &long, "x"),
        Err(format!("Ajuste no válido: «{long}»"))
    );
    assert_eq!(get(Locale::Es, &state), Ok(HashMap::new()));
    assert!(
        lock_store(&state.store)
            .settings_with_prefix("")
            .unwrap()
            .iter()
            .all(|(k, _)| !k.contains("plexustyping") && k != "theme")
    );
}

#[test]
fn values_up_to_the_limit_are_stored_and_larger_ones_rejected() {
    let tmp = TempDir::new();
    let state = open(Ok(tmp.db()));
    let max = "a".repeat(MAX_VALUE_BYTES);
    set(Locale::Es, &state, "plexustyping.customThemes", &max).unwrap();
    let too_big = "b".repeat(MAX_VALUE_BYTES + 1);
    assert_eq!(
        set(Locale::Es, &state, "plexustyping.customThemes", &too_big),
        Err(
            "El ajuste «plexustyping.customThemes» es demasiado grande (máximo 256 KiB)".to_owned()
        )
    );
    assert_eq!(
        get(Locale::Es, &state)
            .unwrap()
            .get("plexustyping.customThemes"),
        Some(&max)
    );
}

#[test]
fn a_temporary_store_refuses_to_be_read_so_the_app_keeps_local_storage() {
    let state = open(Err("test: no data dir".to_owned()));
    assert_eq!(get(Locale::Es, &state), Err(TEMPORARY.es.to_owned()));
    assert_eq!(
        TEMPORARY.es,
        "La base de datos no está disponible: los ajustes siguen en el almacenamiento del navegador"
    );
    assert_eq!(
        get(Locale::En, &state),
        Err("The database is unavailable: the settings stay in the browser’s storage".to_owned())
    );
}

#[test]
fn key_and_size_errors_follow_the_interface_language() {
    let tmp = TempDir::new();
    let state = open(Ok(tmp.db()));
    assert_eq!(
        set(Locale::En, &state, "theme", "x"),
        Err("Invalid setting: “theme”".to_owned())
    );
    assert_eq!(
        remove(Locale::De, &state, "theme"),
        Err("Ungültige Einstellung: „theme“".to_owned())
    );
    let too_big = "b".repeat(MAX_VALUE_BYTES + 1);
    assert_eq!(
        set(Locale::En, &state, "plexustyping.customThemes", &too_big),
        Err("The setting “plexustyping.customThemes” is too large (at most 256 KiB)".to_owned())
    );
}
