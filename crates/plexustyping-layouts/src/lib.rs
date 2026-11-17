//! Keyboard layouts loaded from TOML, with dead-key composition.
//!
//! A layout file maps physical keys (`KeyboardEvent.code`) to the character each
//! layer (normal, Shift, AltGr) produces, and lists the dead keys:
//!
//! ```toml
//! name = "Español QWERTY"
//! dead_keys = ["´", "`", "^", "¨"]
//!
//! [keys.KeyQ]
//! normal = "q"
//! shift = "Q"
//! ```

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use unicode_normalization::UnicodeNormalization;

/// Built-in layouts as `(id, TOML source)`.
const BUILTINS: &[(&str, &str)] = &[
    ("es-qwerty", include_str!("../../../layouts/es-qwerty.toml")),
    (
        "latam-qwerty",
        include_str!("../../../layouts/latam-qwerty.toml"),
    ),
    ("us-qwerty", include_str!("../../../layouts/us-qwerty.toml")),
    ("pt-abnt2", include_str!("../../../layouts/pt-abnt2.toml")),
    ("fr-azerty", include_str!("../../../layouts/fr-azerty.toml")),
    ("de-qwertz", include_str!("../../../layouts/de-qwertz.toml")),
];

/// Built-in layout ids, in the same order as [`BUILTINS`].
const BUILTIN_IDS: &[&str] = &[
    "es-qwerty",
    "latam-qwerty",
    "us-qwerty",
    "pt-abnt2",
    "fr-azerty",
    "de-qwertz",
];

/// The key that types `'\n'` when a layout does not map it explicitly.
const ENTER_CODE: &str = "Enter";

/// A validated keyboard layout.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Layout {
    /// Layout id, e.g. `es-qwerty`.
    pub id: String,
    /// Human-readable name.
    pub name: String,
    /// Characters that act as dead keys, in file order.
    pub dead_keys: Vec<char>,
    /// Layers of each physical key, keyed by `KeyboardEvent.code` (e.g. `KeyQ`).
    pub keys: BTreeMap<String, KeyLayers>,
}

/// The character each layer of one physical key produces.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyLayers {
    /// Character without modifiers.
    pub normal: Option<char>,
    /// Character with Shift.
    pub shift: Option<char>,
    /// Character with AltGr.
    pub altgr: Option<char>,
}

impl KeyLayers {
    /// Layers in lookup preference order.
    fn layers(&self) -> [(Layer, Option<char>); 3] {
        [
            (Layer::Normal, self.normal),
            (Layer::Shift, self.shift),
            (Layer::Altgr, self.altgr),
        ]
    }
}

/// A modifier layer of a key.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Layer {
    /// No modifier.
    Normal,
    /// Shift held.
    Shift,
    /// AltGr held.
    Altgr,
}

/// One press of a physical key on a given layer.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyPress {
    /// `KeyboardEvent.code` of the key.
    pub code: String,
    /// Layer active during the press.
    pub layer: Layer,
}

/// Why a layout file was rejected.
#[derive(Debug, thiserror::Error)]
pub enum LayoutError {
    /// Invalid TOML syntax, unknown field or wrong value type.
    #[error("layout `{id}`: invalid TOML: {message}")]
    Toml {
        /// Layout id.
        id: String,
        /// Parser message, naming the offending key or field.
        message: String,
    },
    /// A layer value is not exactly one character.
    #[error(
        "layout `{id}`: key `{key}` layer `{layer}` must be exactly one character, got {value:?}"
    )]
    NotOneChar {
        /// Layout id.
        id: String,
        /// Key code.
        key: String,
        /// Layer name as written in the file.
        layer: &'static str,
        /// Rejected value.
        value: String,
    },
    /// A `dead_keys` entry is not exactly one character.
    #[error("layout `{id}`: dead key {value:?} must be exactly one character")]
    DeadKeyNotOneChar {
        /// Layout id.
        id: String,
        /// Rejected value.
        value: String,
    },
    /// A dead key has no known combining mark, so it would compose nothing.
    #[error("layout `{id}`: dead key `{dead}` has no known composition")]
    UnsupportedDeadKey {
        /// Layout id.
        id: String,
        /// The dead key without a combining mark.
        dead: char,
    },
    /// A dead key that no layer of any key produces.
    #[error("layout `{id}`: dead key `{dead}` is not produced by any key")]
    DeadKeyNotProduced {
        /// Layout id.
        id: String,
        /// The unreachable dead key.
        dead: char,
    },
    /// `name` is empty or blank.
    #[error("layout `{id}`: name must not be empty")]
    EmptyName {
        /// Layout id.
        id: String,
    },
    /// No `[keys.*]` table is defined.
    #[error("layout `{id}`: at least one key must be defined")]
    EmptyKeys {
        /// Layout id.
        id: String,
    },
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawLayout {
    name: String,
    #[serde(default)]
    dead_keys: Vec<String>,
    #[serde(default)]
    keys: BTreeMap<String, RawKey>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawKey {
    normal: Option<String>,
    shift: Option<String>,
    altgr: Option<String>,
}

/// Returns the only character of `value`, if it has exactly one.
fn single_char(value: &str) -> Option<char> {
    let mut chars = value.chars();
    match (chars.next(), chars.next()) {
        (Some(ch), None) => Some(ch),
        _ => None,
    }
}

/// Parses and validates the TOML `source` of the layout `id`.
pub fn parse(id: &str, source: &str) -> Result<Layout, LayoutError> {
    let raw: RawLayout = toml::from_str(source).map_err(|err| LayoutError::Toml {
        id: id.to_owned(),
        message: err.to_string().trim_end().to_owned(),
    })?;
    if raw.name.trim().is_empty() {
        return Err(LayoutError::EmptyName { id: id.to_owned() });
    }
    if raw.keys.is_empty() {
        return Err(LayoutError::EmptyKeys { id: id.to_owned() });
    }

    let mut keys = BTreeMap::new();
    for (code, raw_key) in raw.keys {
        let layer = |name: &'static str, value: Option<String>| match value {
            None => Ok(None),
            Some(value) => single_char(&value)
                .map(Some)
                .ok_or_else(|| LayoutError::NotOneChar {
                    id: id.to_owned(),
                    key: code.clone(),
                    layer: name,
                    value,
                }),
        };
        let layers = KeyLayers {
            normal: layer("normal", raw_key.normal)?,
            shift: layer("shift", raw_key.shift)?,
            altgr: layer("altgr", raw_key.altgr)?,
        };
        keys.insert(code, layers);
    }

    let mut dead_keys = Vec::with_capacity(raw.dead_keys.len());
    for value in raw.dead_keys {
        let dead = single_char(&value).ok_or_else(|| LayoutError::DeadKeyNotOneChar {
            id: id.to_owned(),
            value: value.clone(),
        })?;
        if combining_mark(dead).is_none() {
            return Err(LayoutError::UnsupportedDeadKey {
                id: id.to_owned(),
                dead,
            });
        }
        let produced = keys
            .values()
            .any(|key| key.layers().iter().any(|&(_, ch)| ch == Some(dead)));
        if !produced {
            return Err(LayoutError::DeadKeyNotProduced {
                id: id.to_owned(),
                dead,
            });
        }
        dead_keys.push(dead);
    }

    Ok(Layout {
        id: id.to_owned(),
        name: raw.name,
        dead_keys,
        keys,
    })
}

/// Ids of the layouts embedded in the binary.
pub fn builtin_ids() -> &'static [&'static str] {
    BUILTIN_IDS
}

/// Loads the built-in layout `id`, or `None` if there is no such layout.
///
/// # Panics
///
/// Panics if an embedded layout file is invalid, which the test suite rules out.
pub fn builtin(id: &str) -> Option<Layout> {
    BUILTINS
        .iter()
        .find(|(builtin_id, _)| *builtin_id == id)
        .map(|(builtin_id, source)| {
            parse(builtin_id, source).unwrap_or_else(|err| panic!("invalid built-in layout: {err}"))
        })
}

/// Combining mark applied by a dead-key character.
fn combining_mark(dead: char) -> Option<char> {
    match dead {
        '´' => Some('\u{0301}'),
        '`' => Some('\u{0300}'),
        '^' => Some('\u{0302}'),
        '¨' => Some('\u{0308}'),
        '~' => Some('\u{0303}'),
        _ => None,
    }
}

impl Layout {
    /// Character produced by pressing the dead key `dead` and then `base`
    /// (e.g. `´` + `a` = `á`), counted as one character.
    ///
    /// Returns `None` when `dead` is not a dead key of this layout or the pair
    /// has no precomposed character (e.g. `´` + `q`).
    pub fn compose(&self, dead: char, base: char) -> Option<char> {
        if !self.dead_keys.contains(&dead) {
            return None;
        }
        let mark = combining_mark(dead)?;
        let composed: String = [base, mark].into_iter().nfc().collect();
        single_char(&composed)
    }

    /// Every character this layout can type: direct key characters (except dead
    /// keys), every dead-key composition, a space and a newline.
    pub fn allowed_chars(&self) -> BTreeSet<char> {
        let direct = self.direct_chars();
        let mut allowed: BTreeSet<char> = direct
            .iter()
            .copied()
            .filter(|ch| !self.dead_keys.contains(ch))
            .collect();
        for &dead in &self.dead_keys {
            allowed.extend(direct.iter().filter_map(|&base| self.compose(dead, base)));
        }
        allowed.extend([' ', '\n']);
        allowed
    }

    /// Shortest key sequence that types `ch`: one press (preferring Normal, then
    /// Shift, then AltGr) or a dead key followed by its base.
    ///
    /// A space falls back to the `Space` key and a newline to `Enter` when the
    /// layout does not map them. Dead-key characters alone are not typeable.
    pub fn locate(&self, ch: char) -> Option<Vec<KeyPress>> {
        if self.dead_keys.contains(&ch) {
            return None;
        }
        if let Some(single) = self.single_press(ch) {
            return Some(vec![single]);
        }
        match ch {
            ' ' => {
                return Some(vec![KeyPress {
                    code: "Space".to_owned(),
                    layer: Layer::Normal,
                }]);
            }
            '\n' => {
                return Some(vec![KeyPress {
                    code: ENTER_CODE.to_owned(),
                    layer: Layer::Normal,
                }]);
            }
            _ => {}
        }
        for &dead in &self.dead_keys {
            let Some(dead_press) = self.single_press(dead) else {
                continue;
            };
            let base = self.presses().find(|&(_, _, base)| {
                !self.dead_keys.contains(&base) && self.compose(dead, base) == Some(ch)
            });
            if let Some((code, layer, _)) = base {
                return Some(vec![
                    dead_press,
                    KeyPress {
                        code: code.to_owned(),
                        layer,
                    },
                ]);
            }
        }
        None
    }

    /// All `(code, layer, char)` presses, ordered by layer preference, then code.
    fn presses(&self) -> impl Iterator<Item = (&str, Layer, char)> {
        (0..3).flat_map(move |index| {
            self.keys.iter().filter_map(move |(code, key)| {
                let (layer, ch) = key.layers()[index];
                ch.map(|ch| (code.as_str(), layer, ch))
            })
        })
    }

    /// Preferred single press producing `ch`.
    fn single_press(&self, ch: char) -> Option<KeyPress> {
        self.presses()
            .find(|&(_, _, produced)| produced == ch)
            .map(|(code, layer, _)| KeyPress {
                code: code.to_owned(),
                layer,
            })
    }

    /// Characters produced directly by any layer of any key.
    fn direct_chars(&self) -> BTreeSet<char> {
        self.presses().map(|(_, _, ch)| ch).collect()
    }
}
