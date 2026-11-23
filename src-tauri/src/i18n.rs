//! Language of the messages the backend sends to the interface (ui-i18n S1).
//!
//! The frontend sets it at start-up and on every change (`set_ui_language`).
//! Message functions take a [`Locale`] argument, so tests pass one explicitly
//! and never touch the process-wide value (tests run in parallel).

use std::sync::atomic::{AtomicU8, Ordering};

/// The five interface languages, in the frontend's order.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Locale {
    Es,
    /// Used until the frontend says otherwise, as its own fallback (ui-i18n S2).
    #[default]
    En,
    Pt,
    Fr,
    De,
}

const ORDER: [Locale; 5] = [Locale::Es, Locale::En, Locale::Pt, Locale::Fr, Locale::De];

impl Locale {
    /// The language of code `id` (`"es"`, `"en"`, `"pt"`, `"fr"`, `"de"`).
    pub fn parse(id: &str) -> Option<Self> {
        match id {
            "es" => Some(Self::Es),
            "en" => Some(Self::En),
            "pt" => Some(Self::Pt),
            "fr" => Some(Self::Fr),
            "de" => Some(Self::De),
            _ => None,
        }
    }

    fn index(self) -> u8 {
        ORDER.iter().position(|l| *l == self).unwrap_or(1) as u8
    }

    /// Picks this language's text from one text per language.
    pub fn pick<T: Copy>(self, texts: Texts<T>) -> T {
        match self {
            Self::Es => texts.es,
            Self::En => texts.en,
            Self::Pt => texts.pt,
            Self::Fr => texts.fr,
            Self::De => texts.de,
        }
    }
}

/// One text per interface language.
#[derive(Clone, Copy, Debug)]
pub struct Texts<T = &'static str> {
    pub es: T,
    pub en: T,
    pub pt: T,
    pub fr: T,
    pub de: T,
}

impl Texts {
    /// This text in `locale`, with each `{name}` replaced by its value.
    pub fn fill(self, locale: Locale, params: &[(&str, &str)]) -> String {
        params
            .iter()
            .fold(locale.pick(self).to_owned(), |text, (name, value)| {
                text.replace(&format!("{{{name}}}"), value)
            })
    }
}

static CURRENT: AtomicU8 = AtomicU8::new(1);

/// The interface language the frontend last set.
pub fn current() -> Locale {
    ORDER
        .get(CURRENT.load(Ordering::Relaxed) as usize)
        .copied()
        .unwrap_or_default()
}

/// Makes `id` the interface language of the backend's messages.
pub fn set(id: &str) -> Result<(), String> {
    let locale = Locale::parse(id).ok_or_else(|| format!("unknown interface language: {id}"))?;
    CURRENT.store(locale.index(), Ordering::Relaxed);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const HELLO: Texts = Texts {
        es: "Hola, {name}",
        en: "Hello, {name}",
        pt: "Olá, {name}",
        fr: "Bonjour, {name}",
        de: "Hallo, {name}",
    };

    #[test]
    fn parses_the_five_languages_and_rejects_others() {
        let parsed: Vec<_> = ["es", "en", "pt", "fr", "de"]
            .iter()
            .map(|id| Locale::parse(id))
            .collect();
        assert_eq!(parsed, ORDER.map(Some).to_vec());
        assert_eq!(Locale::parse("ja"), None);
    }

    #[test]
    fn fill_picks_the_language_and_replaces_placeholders() {
        assert_eq!(HELLO.fill(Locale::De, &[("name", "Ana")]), "Hallo, Ana");
        assert_eq!(HELLO.fill(Locale::Es, &[("name", "Ana")]), "Hola, Ana");
    }

    #[test]
    fn default_is_english() {
        assert_eq!(Locale::default(), Locale::En);
        assert_eq!(Locale::En.index(), 1);
    }
}
