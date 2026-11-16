pub mod commands;

use commands::ScoredSession;
use plexustyping_core::{ErrorMode, InputEvent};
use plexustyping_layouts::{KeyPress, Layout};

#[tauri::command]
fn diagnostic_text(language: String) -> Result<String, String> {
    commands::diagnostic_text(&language)
}

#[tauri::command]
fn next_text(
    language: String,
    layout: String,
    targets: Vec<char>,
    seed: u64,
) -> Result<String, String> {
    commands::next_text(&language, &layout, &targets, seed)
}

#[tauri::command]
fn score_session(text: String, mode: ErrorMode, events: Vec<InputEvent>) -> ScoredSession {
    commands::score_session(&text, mode, &events)
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
        .invoke_handler(tauri::generate_handler![
            diagnostic_text,
            next_text,
            score_session,
            get_layout,
            key_sequences
        ])
        .run(tauri::generate_context!())
        .expect("error while running PlexusTyping");
}
