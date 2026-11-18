pub mod ai;
pub mod code;
pub mod commands;
pub mod history;
pub mod metronome;
pub mod oauth;
pub mod secrets;

use std::sync::{Arc, MutexGuard};

use ai::{AiFailure, AiSettings, ModelName, ProviderInfo, TextParams};
use code::PracticeMode;
use commands::{
    AppState, FirstText, HeatScore, MetronomeTrack, NextText, ScoreRequest, ScoredSession,
    StorageStatus, Weakness, lock_store,
};
use history::CsvExport;
use metronome::{MetronomeStarted, MetronomeState};
use plexustyping_ai::chatgpt_auth::{SignOut, Status};
use plexustyping_ai::{AiClient, Endpoints, ProviderId};
use plexustyping_audio::MetronomeConfig;
use plexustyping_core::{ErrorMode, IndentMode, InputEvent};
use plexustyping_corpus::CodeLanguage;
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

/// Every bundled code language, in display order (S1).
#[tauri::command]
fn code_languages() -> Vec<CodeLanguage> {
    plexustyping_corpus::code_languages().to_vec()
}

/// `mode` is `"prose"` (default), `"words"` or `{ kind: "code", lang }`; `layout` defaults to the profile's.
#[tauri::command]
async fn first_text(
    state: State<'_, AppState>,
    language: String,
    layout: Option<String>,
    day: String,
    mode: Option<PracticeMode>,
    max_lines: Option<usize>,
) -> Result<FirstText, String> {
    let mode = mode.unwrap_or_default();
    let max_lines = max_lines.unwrap_or(code::DEFAULT_MAX_LINES);
    let layout = layout.unwrap_or_else(|| state.profile.layout.clone());
    ai::first_text(
        &state,
        &language,
        &layout,
        &mode,
        max_lines,
        time_seed(),
        &day,
    )
    .await
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
    mode: Option<PracticeMode>,
    max_lines: Option<usize>,
) -> Result<NextText, String> {
    let params = TextParams {
        language,
        layout,
        seed,
        target_count,
        length,
        fallback,
        day,
        mode: mode.unwrap_or_default(),
        max_lines: max_lines.unwrap_or(code::DEFAULT_MAX_LINES),
    };
    ai::next_text(&state, &params).await
}

/// `mode` is the error mode; `practice` is prose (default), words or the code language;
/// `layout` and `language` are the layout typed with and the language of the text,
/// recorded with the session (default: the profile's).
#[allow(clippy::too_many_arguments)]
#[tauri::command]
fn score_session(
    state: State<AppState>,
    text: String,
    mode: ErrorMode,
    events: Vec<InputEvent>,
    started_at_ms: i64,
    offsets_ms: Option<Vec<Option<f64>>>,
    bpm: Option<u32>,
    practice: Option<PracticeMode>,
    indent: Option<IndentMode>,
    layout: Option<String>,
    language: Option<String>,
) -> Result<ScoredSession, String> {
    let mut store = store(&state);
    let metronome = MetronomeTrack { bpm, offsets_ms };
    let practice = practice.unwrap_or_default();
    let req = ScoreRequest {
        text: &text,
        mode,
        practice: &practice,
        indent: indent.unwrap_or_default(),
        events: &events,
        started_at_ms,
        metronome: &metronome,
    };
    let profile = commands::typing_profile(&state.profile, layout.as_deref(), language.as_deref())?;
    Ok(commands::score_practice(&mut store, &profile, &req))
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

/// Of the prose model (also for words), or of the code model when `mode` is code (S5).
#[tauri::command]
fn weaknesses(
    state: State<AppState>,
    limit: usize,
    mode: Option<PracticeMode>,
) -> Result<Vec<Weakness>, String> {
    let stats_mode = mode.unwrap_or_default().stats_mode();
    let model = commands::load_model_for(&store(&state), &state.profile, stats_mode)?;
    Ok(commands::weaknesses(&model, limit))
}

/// Of the prose model (also for words), or of the code model when `mode` is code (S5).
#[tauri::command]
fn heatmap(state: State<AppState>, mode: Option<PracticeMode>) -> Result<Vec<HeatScore>, String> {
    let stats_mode = mode.unwrap_or_default().stats_mode();
    let model = commands::load_model_for(&store(&state), &state.profile, stats_mode)?;
    Ok(commands::heatmap(&model))
}

#[tauri::command]
fn recent_sessions(state: State<AppState>, limit: usize) -> Result<Vec<SessionSummary>, String> {
    commands::recent_sessions(&store(&state), &state.profile, limit)
}

/// `mode`: `"prose"` | `"words"` | `"code"` | none (all); newest first; empty in temporary mode (S6).
#[tauri::command]
fn session_history(
    state: State<AppState>,
    mode: Option<String>,
    limit: Option<u32>,
) -> Result<Vec<SessionSummary>, String> {
    let persistent = state.status.persistent;
    let store = store(&state);
    history::session_history(&store, &state.profile, persistent, mode.as_deref(), limit)
}

/// Writes the sessions of `mode` to the Downloads directory (S7);
/// `tz_offset_min` is minutes east of UTC (`-new Date().getTimezoneOffset()`).
#[tauri::command]
fn export_sessions_csv(
    app: tauri::AppHandle,
    state: State<AppState>,
    mode: Option<String>,
    tz_offset_min: i32,
) -> Result<CsvExport, String> {
    let persistent = state.status.persistent;
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);
    let downloads = || app.path().download_dir().map_err(|e| e.to_string());
    let store = store(&state);
    history::export_sessions_csv(
        &store,
        &state.profile,
        persistent,
        mode.as_deref(),
        tz_offset_min,
        now_ms,
        downloads,
    )
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
    ai::key_status(&state.ai, &state.secrets, provider).await
}

/// The key goes straight to the keychain; it is never returned or logged.
/// Refused for ChatGPT, which uses the signed-in account.
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

/// Whether ChatGPT is signed in, with which account and whether plan use is enabled.
#[tauri::command]
async fn ai_chatgpt_status(state: State<'_, AppState>) -> Result<Status, AiFailure> {
    ai::chatgpt_status(&state.ai).await
}

/// Signs in with ChatGPT in the system browser; resolves once the browser comes
/// back (or the sign-in is cancelled or times out).
#[tauri::command]
async fn ai_chatgpt_sign_in(state: State<'_, AppState>) -> Result<Status, AiFailure> {
    ai::chatgpt_sign_in(&state.ai, oauth::open_browser).await
}

/// Ends a pending ChatGPT sign-in.
#[tauri::command]
fn ai_chatgpt_cancel(state: State<AppState>) {
    ai::chatgpt_cancel(&state.ai);
}

/// Revokes the ChatGPT session and forgets its tokens.
#[tauri::command]
async fn ai_chatgpt_sign_out(state: State<'_, AppState>) -> Result<SignOut, AiFailure> {
    ai::chatgpt_sign_out(&state.ai).await
}

/// The ChatGPT plan's models as `{ id: slug, name: display name }`
/// (`ai_models` keeps returning plain ids for every provider).
#[tauri::command]
async fn ai_chatgpt_models(state: State<'_, AppState>) -> Result<Vec<ModelName>, AiFailure> {
    ai::chatgpt_models(&state.ai).await
}

/// Opens the ChatGPT plan usage page in the system browser.
#[tauri::command]
fn ai_chatgpt_open_usage() -> Result<(), AiFailure> {
    oauth::open_usage_page().map_err(|_| AiFailure {
        kind: "config",
        message: "No se pudo abrir el navegador".to_owned(),
    })
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
            // Sign in with ChatGPT keeps its private record in the app config dir;
            // the AI client holds it (`state.ai.chatgpt()`) for the ChatGPT provider.
            let config_dir = app.path().app_config_dir().ok();
            let chatgpt = Arc::new(oauth::chatgpt_auth(config_dir.as_deref()));
            app.manage(commands::open_state(
                path.map(|dir| dir.join("plexustyping.sqlite3")),
                Arc::new(KeyringStore::default()),
                AiClient::new(Endpoints::production()).with_chatgpt(chatgpt),
            ));
            app.manage(MetronomeState::audio());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            diagnostic_text,
            code_languages,
            first_text,
            next_text,
            score_session,
            weaknesses,
            heatmap,
            recent_sessions,
            session_history,
            export_sessions_csv,
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
            ai_chatgpt_status,
            ai_chatgpt_sign_in,
            ai_chatgpt_cancel,
            ai_chatgpt_sign_out,
            ai_chatgpt_models,
            ai_chatgpt_open_usage,
            ai_usage,
            metronome_start,
            metronome_update,
            metronome_stop
        ])
        .run(tauri::generate_context!())
        .expect("error while running PlexusTyping");
}
