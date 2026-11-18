//! Phase 3 profiles (S1): list, create, update and delete through the public API.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};

use plexustyping_core::{ErrorMode, KeystrokeRecord};
use plexustyping_store::{
    NewSession, Profile, ProfileUpdate, StatsSnapshot, Store, StoreError, UnitStatRow,
};

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
            "plexustyping-store-profiles-{}-{nanos}-{n}",
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

fn records() -> Vec<KeystrokeRecord> {
    vec![
        KeystrokeRecord {
            index: 0,
            expected: 'a',
            typed: 'a',
            correct: true,
            ms_since_previous: None,
        },
        KeystrokeRecord {
            index: 1,
            expected: 'b',
            typed: 'x',
            correct: false,
            ms_since_previous: Some(120.0),
        },
    ]
}

fn session(profile_id: i64, records: &[KeystrokeRecord]) -> NewSession<'_> {
    NewSession {
        profile_id,
        started_at_ms: 1_000,
        mode: "prose",
        code_language: None,
        language: Some("es"),
        layout: Some("es-qwerty"),
        error_mode: ErrorMode::AllowContinue,
        text: "ab",
        duration_ms: 500.0,
        gross_wpm: 30.0,
        net_wpm: 25.0,
        accuracy: 0.5,
        keystrokes: records.len(),
        bpm: None,
        records,
        metronome_offsets_ms: &[],
    }
}

fn stats(unit: &str) -> StatsSnapshot {
    StatsSnapshot {
        units: vec![UnitStatRow {
            unit: unit.into(),
            ema_error: 0.5,
            ema_slowness: 0.1,
            samples: 3,
            slowness_samples: 2,
        }],
        confusions: vec![],
        mean_interval_ms: Some(140.0),
    }
}

/// A profile with one session and prose statistics.
fn with_data(store: &mut Store, profile: &Profile, unit: &str) -> i64 {
    let recs = records();
    let id = store.save_session(&session(profile.id, &recs)).unwrap();
    store.save_stats(profile.id, "prose", &stats(unit)).unwrap();
    id
}

/// Everything a rejected call must leave untouched.
fn snapshot(store: &Store) -> Vec<(Profile, usize, StatsSnapshot)> {
    let profiles = store.profiles().unwrap();
    profiles
        .into_iter()
        .map(|p| {
            let sessions = store.sessions(p.id, None, None).unwrap().len();
            let stats = store.load_stats(p.id, "prose").unwrap();
            (p, sessions, stats)
        })
        .collect()
}

fn named(store: &Store, name: &str) -> Profile {
    store.create_profile(name, "es", "es-qwerty").unwrap()
}

#[test]
fn profiles_are_listed_by_id_and_found_by_id() {
    let store = Store::open_in_memory().unwrap();
    let main = store.default_profile().unwrap();
    let work = store.create_profile("Trabajo", "en", "us-qwerty").unwrap();
    let kid = named(&store, "Peque");
    assert_eq!(
        store.profiles().unwrap(),
        vec![main.clone(), work.clone(), kid]
    );
    assert_eq!(store.profile(work.id).unwrap(), Some(work));
    assert_eq!(store.profile(9_999).unwrap(), None);
}

#[test]
fn create_profile_trims_the_name_and_stores_language_and_layout() {
    let store = Store::open_in_memory().unwrap();
    store.default_profile().unwrap();
    let p = store
        .create_profile("  Trabajo \t", "en", "us-dvorak")
        .unwrap();
    assert_eq!(p.name, "Trabajo");
    assert_eq!(
        (p.language.as_str(), p.layout.as_str()),
        ("en", "us-dvorak")
    );
    assert_eq!(p.theme, None);
    assert!(p.created_at_ms > 0);
    assert_eq!(store.profile(p.id).unwrap(), Some(p));
}

#[test]
fn create_profile_refuses_an_empty_or_blank_name_and_stores_nothing() {
    let mut store = Store::open_in_memory().unwrap();
    let main = store.default_profile().unwrap();
    with_data(&mut store, &main, "a");
    let before = snapshot(&store);
    for name in ["", "   ", "\t\n"] {
        let err = store.create_profile(name, "es", "es-qwerty").unwrap_err();
        assert!(matches!(err, StoreError::EmptyProfileName), "{err:?}");
        assert_eq!(err.to_string(), "El nombre del perfil no puede estar vacío");
    }
    assert_eq!(snapshot(&store), before);
}

#[test]
fn create_profile_accepts_40_characters_and_refuses_41() {
    let store = Store::open_in_memory().unwrap();
    store.default_profile().unwrap();
    // 40 chars, multi-byte: the limit counts characters, not bytes.
    let forty = "ñ".repeat(40);
    let p = store.create_profile(&forty, "es", "es-qwerty").unwrap();
    assert_eq!(p.name, forty);
    // Surrounding spaces do not count.
    let padded = format!("  {}  ", "b".repeat(40));
    assert_eq!(
        store
            .create_profile(&padded, "es", "es-qwerty")
            .unwrap()
            .name,
        "b".repeat(40)
    );
    let before = snapshot(&store);
    let err = store
        .create_profile(&"c".repeat(41), "es", "es-qwerty")
        .unwrap_err();
    assert!(matches!(err, StoreError::ProfileNameTooLong), "{err:?}");
    assert_eq!(
        err.to_string(),
        "El nombre del perfil admite como máximo 40 caracteres"
    );
    assert_eq!(snapshot(&store), before);
}

#[test]
fn create_profile_refuses_a_name_that_exists_ignoring_case() {
    let store = Store::open_in_memory().unwrap();
    store.default_profile().unwrap();
    named(&store, "Ñandú");
    let before = snapshot(&store);
    for name in ["Principal", "principal", "  PRINCIPAL ", "ñANDÚ"] {
        let err = store.create_profile(name, "es", "es-qwerty").unwrap_err();
        assert!(
            matches!(err, StoreError::DuplicateProfileName),
            "{name}: {err:?}"
        );
        assert_eq!(err.to_string(), "Ya existe un perfil con ese nombre");
    }
    assert_eq!(snapshot(&store), before);
}

#[test]
fn update_profile_changes_only_the_given_fields() {
    let store = Store::open_in_memory().unwrap();
    let main = store.default_profile().unwrap();
    let other = named(&store, "Otro");
    let renamed = store
        .update_profile(
            main.id,
            &ProfileUpdate {
                name: Some("  Casa ".into()),
                ..ProfileUpdate::default()
            },
        )
        .unwrap();
    assert_eq!(
        renamed,
        Profile {
            name: "Casa".into(),
            ..main.clone()
        }
    );
    let changed = store
        .update_profile(
            main.id,
            &ProfileUpdate {
                language: Some("en".into()),
                layout: Some("us-colemak".into()),
                theme: Some(Some("nord".into())),
                ..ProfileUpdate::default()
            },
        )
        .unwrap();
    assert_eq!(
        changed,
        Profile {
            language: "en".into(),
            layout: "us-colemak".into(),
            theme: Some("nord".into()),
            ..renamed.clone()
        }
    );
    // No field: nothing changes.
    let same = store
        .update_profile(main.id, &ProfileUpdate::default())
        .unwrap();
    assert_eq!(same, changed);
    // `theme: Some(None)` clears it.
    let cleared = store
        .update_profile(
            main.id,
            &ProfileUpdate {
                theme: Some(None),
                ..ProfileUpdate::default()
            },
        )
        .unwrap();
    assert_eq!(cleared.theme, None);
    assert_eq!(store.profile(main.id).unwrap(), Some(cleared));
    // The other profile is untouched.
    assert_eq!(store.profile(other.id).unwrap(), Some(other));
}

#[test]
fn update_profile_may_change_the_case_of_its_own_name() {
    let store = Store::open_in_memory().unwrap();
    let main = store.default_profile().unwrap();
    let p = store
        .update_profile(
            main.id,
            &ProfileUpdate {
                name: Some("PRINCIPAL".into()),
                ..ProfileUpdate::default()
            },
        )
        .unwrap();
    assert_eq!(p.name, "PRINCIPAL");
}

#[test]
fn update_profile_rejections_change_nothing() {
    let mut store = Store::open_in_memory().unwrap();
    let main = store.default_profile().unwrap();
    let other = named(&store, "Otro");
    with_data(&mut store, &main, "a");
    let before = snapshot(&store);
    let rename = |name: &str| ProfileUpdate {
        name: Some(name.into()),
        language: Some("en".into()),
        theme: Some(Some("nord".into())),
        ..ProfileUpdate::default()
    };
    let cases: [(i64, ProfileUpdate, &str); 4] = [
        (
            main.id,
            rename("  "),
            "El nombre del perfil no puede estar vacío",
        ),
        (
            main.id,
            rename(&"x".repeat(41)),
            "El nombre del perfil admite como máximo 40 caracteres",
        ),
        (
            main.id,
            rename("otro"),
            "Ya existe un perfil con ese nombre",
        ),
        (9_999, rename("Nuevo"), "Ese perfil no existe"),
    ];
    for (id, update, message) in cases {
        let err = store.update_profile(id, &update).unwrap_err();
        assert_eq!(err.to_string(), message);
    }
    let err = store
        .update_profile(other.id + 100, &ProfileUpdate::default())
        .unwrap_err();
    assert!(matches!(err, StoreError::ProfileNotFound), "{err:?}");
    assert_eq!(snapshot(&store), before);
}

#[test]
fn delete_profile_cascades_to_its_data_and_leaves_other_profiles_intact() {
    let tmp = TempDir::new();
    let path = tmp.db();
    let (main, kept_session, gone_session);
    {
        let mut store = Store::open(&path).unwrap();
        main = store.default_profile().unwrap();
        let other = named(&store, "Otro");
        kept_session = with_data(&mut store, &main, "a");
        gone_session = with_data(&mut store, &other, "b");
        store.save_stats(other.id, "code", &stats("{")).unwrap();
        store.delete_profile(other.id).unwrap();

        assert_eq!(store.profiles().unwrap(), vec![main.clone()]);
        assert_eq!(store.profile(other.id).unwrap(), None);
        assert_eq!(store.sessions(other.id, None, None).unwrap(), vec![]);
        assert!(matches!(
            store.session_keystrokes(gone_session),
            Err(StoreError::NotFound)
        ));
        assert_eq!(store.load_stats(other.id, "prose").unwrap().units, vec![]);
        assert_eq!(store.load_stats(other.id, "code").unwrap().units, vec![]);
        // The remaining profile keeps everything.
        assert_eq!(store.sessions(main.id, None, None).unwrap().len(), 1);
        assert_eq!(store.session_keystrokes(kept_session).unwrap(), records());
        assert_eq!(store.load_stats(main.id, "prose").unwrap(), stats("a"));
    }
    // No orphan rows of the deleted profile remain in any table.
    let conn = rusqlite::Connection::open(&path).unwrap();
    let count = |sql: &str| -> i64 { conn.query_row(sql, [], |r| r.get(0)).unwrap() };
    assert_eq!(count("SELECT count(*) FROM sessions"), 1);
    assert_eq!(
        count("SELECT count(*) FROM keystrokes"),
        records().len() as i64
    );
    assert_eq!(
        count(&format!(
            "SELECT count(*) FROM keystrokes WHERE session_id = {gone_session}"
        )),
        0
    );
    for table in ["char_stats", "confusions", "profile_stats"] {
        let others = count(&format!(
            "SELECT count(*) FROM {table} WHERE profile_id != {}",
            main.id
        ));
        assert_eq!(others, 0, "{table}");
    }
}

#[test]
fn delete_profile_refuses_the_last_profile_and_changes_nothing() {
    let mut store = Store::open_in_memory().unwrap();
    let main = store.default_profile().unwrap();
    let session = with_data(&mut store, &main, "a");
    let before = snapshot(&store);
    let err = store.delete_profile(main.id).unwrap_err();
    assert!(matches!(err, StoreError::LastProfile), "{err:?}");
    assert_eq!(err.to_string(), "No se puede borrar el único perfil");
    assert_eq!(snapshot(&store), before);
    assert_eq!(store.session_keystrokes(session).unwrap(), records());
}

#[test]
fn delete_profile_refuses_an_unknown_id_and_changes_nothing() {
    let mut store = Store::open_in_memory().unwrap();
    let main = store.default_profile().unwrap();
    named(&store, "Otro");
    with_data(&mut store, &main, "a");
    let before = snapshot(&store);
    let err = store.delete_profile(9_999).unwrap_err();
    assert!(matches!(err, StoreError::ProfileNotFound), "{err:?}");
    assert_eq!(err.to_string(), "Ese perfil no existe");
    assert_eq!(snapshot(&store), before);
}

#[test]
fn a_deleted_name_can_be_reused_and_the_first_profile_follows_deletion() {
    let store = Store::open_in_memory().unwrap();
    let main = store.default_profile().unwrap();
    let other = named(&store, "Otro");
    store.delete_profile(main.id).unwrap();
    // default_profile() is now the first remaining one, not a new "Principal".
    assert_eq!(store.default_profile().unwrap(), other);
    let again = named(&store, "Principal");
    assert_eq!(store.profiles().unwrap(), vec![other, again]);
}
