//! Ghost session (S1, S2, S6), testable without a Tauri runtime.
//!
//! The ghost of a text is the active profile's best earlier session with it,
//! read from the store (in memory in temporary mode). Lock order: the store,
//! then the active profile.

pub use plexustyping_store::Ghost;

use crate::code::PracticeMode;
use crate::commands::{AppState, lock_store};

/// `ghost_for_text`: the active profile's best session with exactly `text` in
/// `mode` (same code language for code), with its timeline; `None` when none.
pub fn for_text(
    state: &AppState,
    text: &str,
    mode: &PracticeMode,
) -> Result<Option<Ghost>, String> {
    let store = lock_store(&state.store);
    let profile = state.active_profile();
    store
        .ghost_for_text(profile.id, text, mode.session_mode(), mode.code_language())
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests;
