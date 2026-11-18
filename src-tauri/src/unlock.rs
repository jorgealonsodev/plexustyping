//! Progressive unlock (phase 3, S1–S5, S7): the optional beginner mode that
//! starts with the home row and adds one key at a time.
//!
//! Per profile, the store `settings` table keeps whether the mode is on and
//! its target speed (`unlock:<profile id>`, JSON [`ModeSettings`]) and, per
//! language and layout, how many keys of the order are active and how many
//! sessions went by without progress (`unlock:<profile id>:<language>:<layout>`,
//! JSON [`Progress`]). In temporary mode the store, and so this state, lives
//! in memory. Only prose uses the mode: words and code ignore it.

use std::collections::{BTreeMap, BTreeSet};

use plexustyping_core::adaptive::WeaknessModel;
use plexustyping_corpus::{Corpus, LetterOptions, corpus};
use plexustyping_layouts::Layout;
use plexustyping_store::{Profile, Store};
use serde::{Deserialize, Serialize};

use crate::commands::{
    AppState, NextText, STATS_MODE, TextPlan, get_layout, lock_store, unknown_language,
};

/// Target speeds offered, in PPM (S1).
pub const TARGET_WPMS: [u32; 4] = [20, 30, 40, 50];
/// Target speed when the user has not chosen (S1).
pub const DEFAULT_TARGET_WPM: u32 = 30;
/// Samples a mastered key needs at least (S3).
pub const MASTERY_SAMPLES: u32 = 30;
/// Highest error EMA of a mastered key (S3).
pub const MASTERY_ERROR: f64 = 0.05;
/// Newest correct keystrokes whose median latency counts (S3).
pub const LATENCY_WINDOW: usize = 200;
/// Sessions without progress that unlock the next key (S4).
pub const STALLED_LIMIT: u32 = 3;
/// Active letters from which the normal generation (AI included) is used (S5).
pub const AI_MIN_LETTERS: usize = 15;
/// Weak active keys weighted up below [`AI_MIN_LETTERS`], besides the new key.
pub const WEAK_KEYS: usize = 2;
/// Why `unlock_set` refused a target speed.
pub const INVALID_TARGET: &str = "Velocidad objetivo no válida";

/// Home-row keys in unlock order (PRD: "f j d k s l a ñ", then g h).
const HOME_KEYS: [&str; 10] = [
    "KeyF",
    "KeyJ",
    "KeyD",
    "KeyK",
    "KeyS",
    "KeyL",
    "KeyA",
    "Semicolon",
    "KeyG",
    "KeyH",
];

/// The mode for one language and layout of the active profile (S7).
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UnlockStatus {
    pub enabled: bool,
    pub target_wpm: u32,
    /// Active letters, in unlock order (their capitals are active too).
    pub active: Vec<String>,
    /// Active letters that are mastered now, in unlock order.
    pub mastered: Vec<String>,
    /// The letter unlocked next; `None` once every letter is active.
    pub next: Option<String>,
    /// Letters of the full order.
    pub total: usize,
    pub stalled_sessions: u32,
}

/// Per-profile settings, stored under `unlock:<profile id>`.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ModeSettings {
    enabled: bool,
    target_wpm: u32,
}

impl Default for ModeSettings {
    fn default() -> Self {
        ModeSettings {
            enabled: false,
            target_wpm: DEFAULT_TARGET_WPM,
        }
    }
}

/// Per profile, language and layout, stored under `unlock:<id>:<language>:<layout>`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Progress {
    /// Active letters of the order (`None`: just the home row).
    unlocked: Option<usize>,
    stalled_sessions: u32,
}

fn settings_key(profile_id: i64) -> String {
    format!("unlock:{profile_id}")
}

fn progress_key(profile_id: i64, language: &str, layout: &str) -> String {
    format!("unlock:{profile_id}:{language}:{layout}")
}

/// Stored JSON under `key`; the default when missing or unreadable.
fn load_json<T: Default + for<'de> Deserialize<'de>>(store: &Store, key: &str) -> T {
    let stored = store.get_setting(key).ok().flatten();
    stored
        .and_then(|json| serde_json::from_str(&json).ok())
        .unwrap_or_default()
}

fn save_json<T: Serialize>(store: &Store, key: &str, value: &T) -> Result<(), String> {
    let json = serde_json::to_string(value).map_err(|e| e.to_string())?;
    store.set_setting(key, &json).map_err(|e| e.to_string())
}

fn load_settings(store: &Store, profile_id: i64) -> ModeSettings {
    let s: ModeSettings = load_json(store, &settings_key(profile_id));
    if TARGET_WPMS.contains(&s.target_wpm) {
        s
    } else {
        ModeSettings {
            target_wpm: DEFAULT_TARGET_WPM,
            ..s
        }
    }
}

/// Milliseconds per keystroke at `wpm` PPM: 12 000 / PPM (30 → 400 ms, S3).
pub fn target_ms(wpm: u32) -> f64 {
    12_000.0 / f64::from(wpm)
}

/// A lowercase letter with a distinct capital (not `º`, `ª`).
fn is_letter(c: char) -> bool {
    c.is_alphabetic() && c.is_lowercase() && c.to_uppercase().ne(std::iter::once(c))
}

/// The letters the `normal` layer of `layout` puts on the home-row keys, in
/// unlock order (S2).
pub fn home_row(layout: &Layout) -> Vec<char> {
    let mut home = Vec::new();
    for code in HOME_KEYS {
        let normal = layout.keys.get(code).and_then(|k| k.normal);
        if let Some(c) = normal.filter(|c| is_letter(*c) && !layout.dead_keys.contains(c))
            && !home.contains(&c)
        {
            home.push(c);
        }
    }
    home
}

/// The unlock order (S4): the home row, then the other lowercase letters
/// the `normal` layer of `layout` types, most frequent in `corpus` first
/// (ties by char). Letters absent from the corpus are not in the order: they
/// unlock with the digits and signs.
pub fn unlock_order(layout: &Layout, corpus: &Corpus) -> Vec<char> {
    let mut order = home_row(layout);
    let mut rest: Vec<(char, f64)> = layout
        .keys
        .values()
        .filter_map(|k| k.normal)
        .filter(|c| is_letter(*c) && !layout.dead_keys.contains(c) && !order.contains(c))
        .collect::<BTreeSet<char>>()
        .into_iter()
        .map(|c| (c, corpus.frequency(&c.to_string())))
        .filter(|(_, f)| *f > 0.0)
        .collect();
    rest.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
    order.extend(rest.into_iter().map(|(c, _)| c));
    order
}

/// The mode of one profile, language and layout, as stored.
struct Context {
    profile_id: i64,
    key: String,
    settings: ModeSettings,
    progress: Progress,
    layout: Layout,
    order: Vec<char>,
    home: usize,
}

impl Context {
    fn load(store: &Store, profile_id: i64, language: &str, layout: &str) -> Result<Self, String> {
        let corpus = corpus(language).ok_or_else(|| unknown_language(language))?;
        let layout_def = get_layout(layout)?;
        let order = unlock_order(&layout_def, corpus);
        let home = home_row(&layout_def).len();
        let key = progress_key(profile_id, language, layout);
        Ok(Context {
            profile_id,
            settings: load_settings(store, profile_id),
            progress: load_json(store, &key),
            key,
            layout: layout_def,
            order,
            home,
        })
    }

    /// Active letters of the order: at least the home row, at most all.
    fn unlocked(&self) -> usize {
        let n = self.progress.unlocked.unwrap_or(self.home);
        n.clamp(self.home.min(self.order.len()), self.order.len())
    }

    fn active(&self) -> &[char] {
        &self.order[..self.unlocked()]
    }

    fn next(&self) -> Option<char> {
        self.order.get(self.unlocked()).copied()
    }

    fn all_letters(&self) -> bool {
        self.next().is_none()
    }

    /// Characters a text may use (S4): space, the active letters and their
    /// capitals; every character of the layout once every letter is active.
    fn chars(&self) -> BTreeSet<char> {
        let typeable = self.layout.allowed_chars();
        if self.all_letters() {
            return typeable;
        }
        let mut chars = BTreeSet::from([' ']);
        for &c in self.active() {
            chars.insert(c);
            let mut upper = c.to_uppercase();
            if let (Some(u), None) = (upper.next(), upper.next())
                && typeable.contains(&u)
            {
                chars.insert(u);
            }
        }
        chars
    }

    /// Active letters mastered now at the target speed (S3), in order.
    fn mastered(&self, store: &Store) -> Result<Vec<char>, String> {
        let stats = store
            .load_stats(self.profile_id, STATS_MODE)
            .map_err(|e| e.to_string())?;
        let units: BTreeMap<String, (u32, f64)> = stats
            .units
            .into_iter()
            .map(|u| (u.unit, (u.samples, u.ema_error)))
            .collect();
        let target = target_ms(self.settings.target_wpm);
        let mut mastered = Vec::new();
        for &c in self.active() {
            let Some(&(samples, ema_error)) = units.get(&c.to_string()) else {
                continue;
            };
            if samples < MASTERY_SAMPLES || ema_error > MASTERY_ERROR {
                continue;
            }
            let latencies = store
                .correct_latencies(self.profile_id, STATS_MODE, c, LATENCY_WINDOW)
                .map_err(|e| e.to_string())?;
            if median(latencies).is_some_and(|m| m <= target) {
                mastered.push(c);
            }
        }
        Ok(mastered)
    }

    fn status(&self, store: &Store) -> Result<UnlockStatus, String> {
        let strings = |cs: &[char]| cs.iter().map(char::to_string).collect();
        Ok(UnlockStatus {
            enabled: self.settings.enabled,
            target_wpm: self.settings.target_wpm,
            active: strings(self.active()),
            mastered: strings(&self.mastered(store)?),
            next: self.next().map(String::from),
            total: self.order.len(),
            stalled_sessions: self.progress.stalled_sessions,
        })
    }
}

/// Median of `values`; `None` when empty.
fn median(mut values: Vec<f64>) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    values.sort_by(f64::total_cmp);
    let mid = values.len() / 2;
    Some(if values.len() % 2 == 1 {
        values[mid]
    } else {
        (values[mid - 1] + values[mid]) / 2.0
    })
}

/// `unlock_status`: the mode of the active profile for `language` and `layout`.
pub fn status(state: &AppState, language: &str, layout: &str) -> Result<UnlockStatus, String> {
    let store = lock_store(&state.store);
    let profile = state.active_profile();
    Context::load(&store, profile.id, language, layout)?.status(&store)
}

/// `unlock_set`: turns the mode of the active profile on or off and/or sets
/// its target speed (one of [`TARGET_WPMS`], else [`INVALID_TARGET`] and
/// nothing changes); missing values are kept. Progress is never touched.
pub fn set(
    state: &AppState,
    language: &str,
    layout: &str,
    enabled: Option<bool>,
    target_wpm: Option<u32>,
) -> Result<UnlockStatus, String> {
    if target_wpm.is_some_and(|t| !TARGET_WPMS.contains(&t)) {
        return Err(INVALID_TARGET.to_owned());
    }
    let store = lock_store(&state.store);
    let profile = state.active_profile();
    let mut ctx = Context::load(&store, profile.id, language, layout)?;
    if let Some(enabled) = enabled {
        ctx.settings.enabled = enabled;
    }
    if let Some(target) = target_wpm {
        ctx.settings.target_wpm = target;
    }
    save_json(&store, &settings_key(profile.id), &ctx.settings)?;
    ctx.status(&store)
}

/// Whether the mode is on for profile `profile_id`.
pub fn is_enabled(store: &Store, profile_id: i64) -> bool {
    load_settings(store, profile_id).enabled
}

/// Forgets the settings and progress of a deleted profile, so a new profile
/// that gets its id starts afresh.
pub fn forget_profile(store: &Store, profile_id: i64) -> Result<(), String> {
    store
        .delete_settings_under(&settings_key(profile_id))
        .map_err(|e| e.to_string())
}

/// What prose texts may use while the mode is on (S5).
#[derive(Clone, Debug, PartialEq)]
pub struct Restriction {
    /// Active letters, in unlock order.
    pub letters: Vec<char>,
    /// Characters a text may contain (see `Context::chars`).
    pub chars: BTreeSet<char>,
    /// Every letter is active: texts are the normal ones.
    pub all_letters: bool,
    /// The last unlocked letter; `None` while only the home row is active.
    pub new_key: Option<char>,
}

/// The restriction of prose texts for `profile` in `language` and `layout`;
/// `None` when the mode is off.
pub fn restriction(
    store: &Store,
    profile: &Profile,
    language: &str,
    layout: &str,
) -> Result<Option<Restriction>, String> {
    if !is_enabled(store, profile.id) {
        return Ok(None);
    }
    let ctx = Context::load(store, profile.id, language, layout)?;
    let unlocked = ctx.unlocked();
    Ok(Some(Restriction {
        letters: ctx.active().to_vec(),
        chars: ctx.chars(),
        all_letters: ctx.all_letters(),
        new_key: (unlocked > ctx.home).then(|| ctx.order[unlocked - 1]),
    }))
}

/// Restricts `plan` to the active characters: its allowed chars and its
/// targets (a target with a locked char is dropped). No-op once every letter
/// is active.
pub fn restrict_plan(plan: &mut TextPlan, r: &Restriction) {
    if r.all_letters {
        return;
    }
    plan.allowed = plan.allowed.intersection(&r.chars).copied().collect();
    let allowed = plan.allowed.clone();
    plan.targets
        .retain(|t| t.chars().all(|c| allowed.contains(&c)));
}

/// The text below [`AI_MIN_LETTERS`] letters (S5): lowercase words of the
/// active letters from the plan's corpus, completed with pseudowords, with
/// the new key and the [`WEAK_KEYS`] weakest active keys weighted up, in the
/// plan's length. Never the AI. Its targets are the weighted keys it contains.
pub fn letters_text(
    plan: &TextPlan,
    r: &Restriction,
    model: &WeaknessModel,
    seed: u64,
) -> Result<NextText, String> {
    let mut weighted: Vec<char> = r.new_key.into_iter().collect();
    let weak = model.ranked().into_iter().filter_map(|(unit, _)| {
        let mut it = unit.chars();
        match (it.next(), it.next()) {
            (Some(c), None) if r.letters.contains(&c) => Some(c),
            _ => None,
        }
    });
    for c in weak.take(WEAK_KEYS) {
        if !weighted.contains(&c) {
            weighted.push(c);
        }
    }
    let text = plan.corpus.generate_letters(&LetterOptions {
        letters: r.letters.iter().copied().collect(),
        weighted: weighted.clone(),
        length: plan.length.clone(),
        seed,
    });
    if text.is_empty() {
        return Err("could not generate a practice text".to_owned());
    }
    let targets = weighted
        .into_iter()
        .filter(|c| text.contains(*c))
        .map(String::from)
        .collect();
    Ok(NextText::corpus(text, targets))
}

/// What [`after_session`] compares against: the mode before the session was saved.
pub struct Before {
    ctx: Context,
    mastered: Vec<char>,
}

/// The mode of the session's profile, language and layout before a prose
/// session is saved; `None` when the mode is off (or cannot be read, logged).
pub fn before_session(store: &Store, profile: &Profile) -> Option<Before> {
    if !is_enabled(store, profile.id) {
        return None;
    }
    let loaded = Context::load(store, profile.id, &profile.language, &profile.layout)
        .and_then(|ctx| Ok((ctx.mastered(store)?, ctx)));
    match loaded {
        Ok((mastered, ctx)) => Some(Before { ctx, mastered }),
        Err(e) => {
            eprintln!("PlexusTyping: could not read the unlock state: {e}");
            None
        }
    }
}

/// The unlock check after a saved prose session (S4): the next key unlocks
/// when every active key is mastered, or on the [`STALLED_LIMIT`]th session
/// without a newly mastered active key; every unlock resets the counter.
/// Returns the unlocked key. A failure is logged and unlocks nothing.
pub fn after_session(store: &Store, before: Before) -> Option<String> {
    let Before { mut ctx, mastered } = before;
    let next = ctx.next()?;
    let now = match ctx.mastered(store) {
        Ok(now) => now,
        Err(e) => {
            eprintln!("PlexusTyping: could not check the unlock state: {e}");
            return None;
        }
    };
    let progress = now.iter().any(|c| !mastered.contains(c));
    // "3 sesiones sin progreso": in a row, so a session with progress starts over.
    if progress {
        ctx.progress.stalled_sessions = 0;
    } else {
        ctx.progress.stalled_sessions += 1;
    }
    let unlock = now.len() == ctx.active().len() || ctx.progress.stalled_sessions >= STALLED_LIMIT;
    if unlock {
        ctx.progress.unlocked = Some(ctx.unlocked() + 1);
        ctx.progress.stalled_sessions = 0;
    }
    if let Err(e) = save_json(store, &ctx.key, &ctx.progress) {
        eprintln!("PlexusTyping: could not save the unlock state: {e}");
        return None;
    }
    unlock.then(|| next.to_string())
}

#[cfg(test)]
mod tests;
