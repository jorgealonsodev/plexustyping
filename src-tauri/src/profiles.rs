//! Local profiles (S2–S3): the active profile and the profile commands,
//! testable without a Tauri runtime.
//!
//! Lock order everywhere: the store, then the active profile. Every function
//! here holds the store for the whole call, so a switch, update or deletion
//! is never seen half done.

use plexustyping_store::{Profile, ProfileUpdate, Store, StoreError};
use serde::Serialize;

use crate::commands::{AppState, lock_store};
use crate::i18n::Locale;
use crate::messages;

/// `settings` key of the active profile id.
pub const ACTIVE_PROFILE_KEY: &str = "active_profile";

/// Every profile and the active one's id.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileList {
    pub active: i64,
    pub profiles: Vec<Profile>,
}

/// The active profile after a deletion.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileDeleted {
    pub active: Profile,
}

/// Why a profile command failed; [`ProfileError::message`] says it in the
/// interface language at the command boundary.
#[derive(Debug)]
pub enum ProfileError {
    /// A store refusal (empty, long or taken name, last or missing profile) or failure.
    Store(StoreError),
    /// A language without a bundled corpus.
    UnknownLanguage,
    /// A layout that is not built in.
    UnknownLayout,
}

impl From<StoreError> for ProfileError {
    fn from(err: StoreError) -> Self {
        ProfileError::Store(err)
    }
}

impl ProfileError {
    /// The message shown to the user, in `locale`.
    pub fn message(&self, locale: Locale) -> String {
        match self {
            ProfileError::Store(err) => messages::store_error(locale, err),
            ProfileError::UnknownLanguage => locale.pick(messages::UNKNOWN_LANGUAGE).to_owned(),
            ProfileError::UnknownLayout => locale.pick(messages::UNKNOWN_LAYOUT).to_owned(),
        }
    }
}

/// The profile to start with: the saved `active_profile` when it still
/// exists, else [`Store::default_profile`] (missing, unparsable or deleted id).
pub fn active_at_start(store: &Store) -> Result<Profile, StoreError> {
    let saved = store.get_setting(ACTIVE_PROFILE_KEY)?;
    if let Some(id) = saved.and_then(|v| v.trim().parse::<i64>().ok())
        && let Some(profile) = store.profile(id)?
    {
        return Ok(profile);
    }
    store.default_profile()
}

/// `profiles_list`.
pub fn list(state: &AppState) -> Result<ProfileList, ProfileError> {
    let store = lock_store(&state.store);
    let profiles = store.profiles()?;
    let active = state.active_profile().id;
    Ok(ProfileList { active, profiles })
}

/// `profile_create`: a new profile without theme; the active one stays.
pub fn create(
    state: &AppState,
    name: &str,
    language: &str,
    layout: &str,
) -> Result<Profile, ProfileError> {
    check_language(language)?;
    check_layout(layout)?;
    let store = lock_store(&state.store);
    Ok(store.create_profile(name, language, layout)?)
}

/// `profile_update`: `None` fields are left; `theme` `Some("")` clears it.
pub fn update(
    state: &AppState,
    id: i64,
    name: Option<String>,
    language: Option<String>,
    layout: Option<String>,
    theme: Option<String>,
) -> Result<Profile, ProfileError> {
    if let Some(language) = &language {
        check_language(language)?;
    }
    if let Some(layout) = &layout {
        check_layout(layout)?;
    }
    let changes = ProfileUpdate {
        name,
        language,
        layout,
        theme: theme.map(|t| Some(t).filter(|t| !t.is_empty())),
    };
    let store = lock_store(&state.store);
    let profile = store.update_profile(id, &changes)?;
    if state.active_profile().id == id {
        state.set_active_profile(profile.clone());
    }
    Ok(profile)
}

/// `profile_delete`: deletes the profile and all its data; deleting the
/// active one switches to the first remaining profile.
pub fn delete(state: &AppState, id: i64) -> Result<ProfileDeleted, ProfileError> {
    let store = lock_store(&state.store);
    store.delete_profile(id)?;
    // Its progressive unlock state lives in `settings`, outside the cascade.
    if let Err(e) = crate::unlock::forget_profile(&store, id) {
        eprintln!("PlexusTyping: could not forget the unlock state: {e}");
    }
    if state.active_profile().id == id {
        let first = store.default_profile()?;
        // The deletion is done: a failed save only means the next start
        // falls back to the first profile, which is this one anyway.
        if let Err(e) = save_active(&store, first.id) {
            eprintln!("PlexusTyping: could not save the active profile: {e}");
        }
        state.set_active_profile(first);
    }
    Ok(ProfileDeleted {
        active: state.active_profile(),
    })
}

/// `profile_switch`: makes `id` the active profile and saves it.
pub fn switch(state: &AppState, id: i64) -> Result<Profile, ProfileError> {
    let store = lock_store(&state.store);
    let profile = store.profile(id)?.ok_or(StoreError::ProfileNotFound)?;
    save_active(&store, id)?;
    state.set_active_profile(profile.clone());
    Ok(profile)
}

fn save_active(store: &Store, id: i64) -> Result<(), StoreError> {
    store.set_setting(ACTIVE_PROFILE_KEY, &id.to_string())
}

fn check_language(language: &str) -> Result<(), ProfileError> {
    if plexustyping_corpus::languages().contains(&language) {
        Ok(())
    } else {
        Err(ProfileError::UnknownLanguage)
    }
}

fn check_layout(layout: &str) -> Result<(), ProfileError> {
    if plexustyping_layouts::builtin_ids().contains(&layout) {
        Ok(())
    } else {
        Err(ProfileError::UnknownLayout)
    }
}

#[cfg(test)]
mod tests;
