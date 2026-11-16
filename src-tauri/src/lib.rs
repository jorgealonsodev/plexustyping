pub mod ai;
pub mod commands;
pub mod metronome;
pub mod secrets;

use std::sync::{Arc, MutexGuard};

use ai::{AiFailure, AiSettings, ProviderInfo, TextParams};
use commands::{
    AppState, FirstText, HeatScore, MetronomeTrack, NextText, ScoredSession, StorageStatus,
    Weakness, lock_store,
};
use metronome::{MetronomeStarted, MetronomeState};
use plexustyping_ai::{AiClient, Endpoints, ProviderId};
use plexustyping_audio::MetronomeConfig;
use plexustyping_core::{ErrorMode, InputEvent};
use plexustyping_layouts::{KeyPress, Layout};
use plexustyping_store::{SessionSummary, Store, UsageRow};
use secrets::KeyringStore;
use tauri::{Manager, State};

/// The store; never held across an `.await` (async commands must be `Send`).
fn store<'a>(state: &'a State<AppState>) -> MutexGuard<'a, Store> {
    lock_store(&state.store)
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
async fn first_text(
    state: State<'_, AppState>,
    language: String,
    day: String,
) -> Result<FirstText, String> {
    ai::first_text(&state, &language, time_seed(), &day).await
}

#[allow(clippy::too_many_arguments)]
#[tauri::command]
async fn next_text(
    state: State<'_, AppState>,
    language: String,
    layout: String,
    seed: u64,
    target_count: usize,
    length: usize,
    fallback: Vec<String>,
    day: String,
) -> Result<NextText, String> {
    let params = TextParams {
        language,
        layout,
        seed,
        target_count,
        length,
        fallback,
        day,
    };
    ai::next_text(&state, &params).await
}

#[tauri::command]
fn score_session(
    state: State<AppState>,
    text: String,
    mode: ErrorMode,
    events: Vec<InputEvent>,
    started_at_ms: i64,
    offsets_ms: Option<Vec<Option<f64>>>,
    bpm: Option<u32>,
) -> ScoredSession {
    let mut store = store(&state);
    let metronome = MetronomeTrack { bpm, offsets_ms };
    commands::score_session_with_metronome(
        &mut store,
        &state.profile,
        &text,
        mode,
        &events,
        started_at_ms,
        &metronome,
    )
}

/// Runs a metronome call off the main thread: opening the device blocks.
async fn on_audio_thread<T: Send + 'static>(
    state: &State<'_, MetronomeState>,
    call: impl FnOnce(MetronomeState) -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    let state = MetronomeState::clone(state);
    let joined = tauri::async_runtime::spawn_blocking(move || call(state)).await;
    joined.map_err(|e| format!("Error interno del metrónomo: {e}"))?
}

#[tauri::command]
async fn metronome_start(
    state: State<'_, MetronomeState>,
    config: MetronomeConfig,
) -> Result<MetronomeStarted, String> {
    on_audio_thread(&state, move |m| m.start(config)).await
}

#[tauri::command]
async fn metronome_update(
    state: State<'_, MetronomeState>,
    config: MetronomeConfig,
) -> Result<Option<MetronomeStarted>, String> {
    on_audio_thread(&state, move |m| m.update(config)).await
}

#[tauri::command]
async fn metronome_stop(state: State<'_, MetronomeState>) -> Result<(), String> {
    on_audio_thread(&state, |m| {
        m.stop();
        Ok(())
    })
    .await
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

#[tauri::command]
fn ai_settings_get(state: State<AppState>) -> AiSettings {
    ai::load_settings(&store(&state))
}

#[tauri::command]
fn ai_settings_set(state: State<AppState>, settings: AiSettings) -> Result<AiSettings, AiFailure> {
    ai::save_settings(&store(&state), settings)
}

#[tauri::command]
fn ai_providers() -> Vec<ProviderInfo> {
    ai::providers()
}

#[tauri::command]
async fn ai_key_status(
    state: State<'_, AppState>,
    provider: ProviderId,
) -> Result<bool, AiFailure> {
    ai::key_status(&state.secrets, provider).await
}

/// The key goes straight to the keychain; it is never returned or logged.
#[tauri::command]
async fn ai_key_save(
    state: State<'_, AppState>,
    provider: ProviderId,
    key: String,
) -> Result<(), AiFailure> {
    ai::key_save(&state.secrets, provider, key).await
}

#[tauri::command]
async fn ai_key_delete(state: State<'_, AppState>, provider: ProviderId) -> Result<(), AiFailure> {
    ai::key_delete(&state.secrets, provider).await
}

#[tauri::command]
async fn ai_models(
    state: State<'_, AppState>,
    provider: ProviderId,
) -> Result<Vec<String>, AiFailure> {
    ai::models(&state.ai, &state.secrets, provider).await
}

#[tauri::command]
async fn ai_test(
    state: State<'_, AppState>,
    provider: ProviderId,
    model: String,
) -> Result<(), AiFailure> {
    ai::test_connection(&state.ai, &state.secrets, provider, &model).await
}

#[tauri::command]
fn ai_usage(state: State<AppState>, day: String) -> Result<Vec<UsageRow>, String> {
    ai::usage(&store(&state), &ai::usage_day(&day))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let path = app.path().app_data_dir().map_err(|e| e.to_string());
            app.manage(commands::open_state(
                path.map(|dir| dir.join("plexustyping.sqlite3")),
                Arc::new(KeyringStore::default()),
                AiClient::new(Endpoints::production()),
            ));
            app.manage(MetronomeState::audio());
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
            key_sequences,
            ai_settings_get,
            ai_settings_set,
            ai_providers,
            ai_key_status,
            ai_key_save,
            ai_key_delete,
            ai_models,
            ai_test,
            ai_usage,
            metronome_start,
            metronome_update,
            metronome_stop
        ])
        .run(tauri::generate_context!())
        .expect("error while running PlexusTyping");
}
