pub mod commands;

use std::sync::MutexGuard;

use commands::{
    AppState, FirstText, HeatScore, NextText, NextTextRequest, ScoredSession, StorageStatus,
    Weakness,
};
use plexustyping_core::{ErrorMode, InputEvent};
use plexustyping_layouts::{KeyPress, Layout};
use plexustyping_store::{SessionSummary, Store};
use tauri::{Manager, State};

/// The store; a panic while it was held leaves it usable (each write is transactional).
fn store<'a>(state: &'a State<AppState>) -> MutexGuard<'a, Store> {
    state.store.lock().unwrap_or_else(|e| e.into_inner())
}

/// A per-call seed for text generation.
fn time_seed() -> u64 {
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH);
    now.map(|d| d.as_nanos() as u64).unwrap_or(0)
}

#[tauri::command]
fn diagnostic_text(language: String) -> Result<String, String> {
    commands::diagnostic_text(&language)
}

#[tauri::command]
fn first_text(state: State<AppState>, language: String) -> Result<FirstText, String> {
    commands::first_text(&store(&state), &state.profile, &language, time_seed())
}

#[tauri::command]
fn next_text(
    state: State<AppState>,
    language: String,
    layout: String,
    seed: u64,
    target_count: usize,
    length: usize,
    fallback: Vec<String>,
) -> Result<NextText, String> {
    let model = commands::load_model(&store(&state), &state.profile)?;
    let req = NextTextRequest {
        language: &language,
        layout: &layout,
        seed,
        target_count,
        length,
        fallback: &fallback,
    };
    commands::next_text(&model, &req)
}

#[tauri::command]
fn score_session(
    state: State<AppState>,
    text: String,
    mode: ErrorMode,
    events: Vec<InputEvent>,
    started_at_ms: i64,
) -> ScoredSession {
    let mut store = store(&state);
    commands::score_session(
        &mut store,
        &state.profile,
        &text,
        mode,
        &events,
        started_at_ms,
    )
}

#[tauri::command]
fn weaknesses(state: State<AppState>, limit: usize) -> Result<Vec<Weakness>, String> {
    let model = commands::load_model(&store(&state), &state.profile)?;
    Ok(commands::weaknesses(&model, limit))
}

#[tauri::command]
fn heatmap(state: State<AppState>) -> Result<Vec<HeatScore>, String> {
    let model = commands::load_model(&store(&state), &state.profile)?;
    Ok(commands::heatmap(&model))
}

#[tauri::command]
fn recent_sessions(state: State<AppState>, limit: usize) -> Result<Vec<SessionSummary>, String> {
    commands::recent_sessions(&store(&state), &state.profile, limit)
}

#[tauri::command]
fn storage_status(state: State<AppState>) -> StorageStatus {
    state.status.clone()
}

#[tauri::command]
fn get_layout(layout: String) -> Result<Layout, String> {
    commands::get_layout(&layout)
}

#[tauri::command]
fn key_sequences(layout: String, text: String) -> Result<Vec<Option<Vec<KeyPress>>>, String> {
    commands::key_sequences(&layout, &text)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let path = app.path().app_data_dir().map_err(|e| e.to_string());
            app.manage(commands::open_state(
                path.map(|dir| dir.join("plexustyping.sqlite3")),
            ));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            diagnostic_text,
            first_text,
            next_text,
            score_session,
            weaknesses,
            heatmap,
            recent_sessions,
            storage_status,
            get_layout,
            key_sequences
        ])
        .run(tauri::generate_context!())
        .expect("error while running PlexusTyping");
}
