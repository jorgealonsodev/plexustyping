//! UI settings (S1–S5): the `plexustyping.*` values the webview used to keep in
//! localStorage, stored in the `settings` table under [`STORE_PREFIX`] so the dev
//! app and the installed app share them and they never collide with backend
//! settings. Every write is a single autocommitted statement: on disk when it returns.

use std::collections::HashMap;

use crate::commands::{AppState, lock_store};
use crate::i18n::{Locale, Texts};
use crate::messages;

/// Prefix of every UI setting in the `settings` table (`ui:plexustyping.layout`).
pub const STORE_PREFIX: &str = "ui:";
/// The only UI keys accepted (as in localStorage).
pub const KEY_PREFIX: &str = "plexustyping.";
/// Longest accepted key, in bytes.
pub const MAX_KEY_BYTES: usize = 128;
/// Largest accepted value, in bytes (30 imported themes are about 40 KB).
pub const MAX_VALUE_BYTES: usize = 256 * 1024;

/// Why a temporary (in-memory) store is not read: the app keeps localStorage (S5).
pub const TEMPORARY: Texts = messages::UI_SETTINGS_TEMPORARY;

fn checked_key(locale: Locale, key: &str) -> Result<String, String> {
    if key.len() > KEY_PREFIX.len() && key.len() <= MAX_KEY_BYTES && key.starts_with(KEY_PREFIX) {
        Ok(format!("{STORE_PREFIX}{key}"))
    } else {
        Err(messages::INVALID_SETTING.fill(locale, &[("key", key)]))
    }
}

/// Every stored UI setting, by its `plexustyping.*` key; [`TEMPORARY`] when the
/// database could not be opened.
pub fn get(locale: Locale, state: &AppState) -> Result<HashMap<String, String>, String> {
    if !state.status.persistent {
        return Err(locale.pick(TEMPORARY).to_owned());
    }
    let rows = lock_store(&state.store)
        .settings_with_prefix(STORE_PREFIX)
        .map_err(|e| e.to_string())?;
    Ok(rows
        .into_iter()
        .filter_map(|(k, v)| Some((k.strip_prefix(STORE_PREFIX)?.to_owned(), v)))
        .collect())
}

/// Stores UI setting `key` (`plexustyping.*`, at most [`MAX_VALUE_BYTES`]).
pub fn set(locale: Locale, state: &AppState, key: &str, value: &str) -> Result<(), String> {
    let stored = checked_key(locale, key)?;
    if value.len() > MAX_VALUE_BYTES {
        let max = (MAX_VALUE_BYTES / 1024).to_string();
        return Err(messages::SETTING_TOO_LARGE.fill(locale, &[("key", key), ("max", &max)]));
    }
    lock_store(&state.store)
        .set_setting(&stored, value)
        .map_err(|e| e.to_string())
}

/// Forgets UI setting `key`; nothing happens when it is not set.
pub fn remove(locale: Locale, state: &AppState, key: &str) -> Result<(), String> {
    let stored = checked_key(locale, key)?;
    lock_store(&state.store)
        .delete_setting(&stored)
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests;
