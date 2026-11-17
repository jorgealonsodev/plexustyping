//! The generation prompt (PRD §4 example, parameterized).

use crate::{GenerationRequest, Mode};

/// Builds the Spanish prompt for `req`. Only the request's own fields go into
/// it (targets with their minimum counts, language, mode, length, allowed
/// characters, topic).
///
/// Each target's count is [`GenerationRequest::min_counts`] at the longest
/// accepted length, so a text that meets it always meets [`crate::validate`].
/// Prose may end with a run of real words holding the targets when sensible
/// sentences cannot reach the counts (like the offline corpus).
pub fn build_prompt(req: &GenerationRequest) -> String {
    let language = language_name(&req.language);
    let longest = req.length.end.saturating_sub(1).max(req.length.start);
    let length = format!("Longitud: {}–{longest} caracteres.", req.length.start);
    let topic = req
        .topic
        .as_deref()
        .map(|t| t.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|t| !t.is_empty())
        .unwrap_or_else(|| "libre".into());
    let mut lines = Vec::new();
    match &req.mode {
        Mode::Prose => {
            lines.push(format!(
                "Escribe un texto en {language} para practicar mecanografía."
            ));
            lines.push(format!("{length} Tema: {topic}."));
        }
        Mode::Code { lang } => {
            lines.push("Eres un generador de código para practicar mecanografía.".to_string());
            lines.push(format!(
                "Escribe un fragmento de código en {} de 5 a 20 líneas, con comentarios en {language}. {length} Tema: {topic}.",
                lang.trim()
            ));
        }
    }
    let targets: Vec<String> = req
        .min_counts(longest)
        .into_iter()
        .map(|(t, n)| {
            let shown = if t.chars().count() > 1 {
                format!("\"{t}\"")
            } else {
                t.to_owned()
            };
            let times = if n == 1 { "vez" } else { "veces" };
            format!("{shown} (al menos {n} {times})")
        })
        .collect();
    if !targets.is_empty() {
        lines.push(format!("Caracteres objetivo: {}.", targets.join(", ")));
        if req.mode == Mode::Prose {
            lines.push(format!(
                "Empieza con frases naturales. Si con frases con sentido no llegas a esas cantidades, \
                 completa el resto con palabras reales del {language} que contengan los caracteres objetivo, \
                 separadas por espacios y con un punto al final."
            ));
        }
    }
    let mut allowed: Vec<String> = Vec::new();
    for ch in req.allowed_chars.chars().filter(|c| !c.is_whitespace()) {
        let ch = ch.to_string();
        if !allowed.contains(&ch) {
            allowed.push(ch);
        }
    }
    let space = match req.mode {
        Mode::Prose => "(y el espacio)",
        Mode::Code { .. } => "(y el espacio y los saltos de línea)",
    };
    lines.push(format!(
        "Usa solo estos caracteres: {} {space}.",
        allowed.join(" ")
    ));
    lines.push(match req.mode {
        Mode::Prose => {
            "Texto plano en una sola línea: sin listas, sin markdown, sin comillas tipográficas."
                .to_string()
        }
        Mode::Code { .. } => "Sin markdown ni bloques ```, sin comillas tipográficas.".to_string(),
    });
    lines.push("Responde solo con JSON: {\"text\": \"...\"}".to_string());
    lines.join("\n")
}

/// The density the prompt and [`crate::validate`] use: clamped to `2.0..=5.0`,
/// 3 when not a number.
pub(crate) fn clamped_density(density: f32) -> f32 {
    if density.is_finite() {
        density.clamp(2.0, 5.0)
    } else {
        3.0
    }
}

/// Spanish name of a language code (`es`, `en-US`, `pt_BR`...); unknown codes
/// are used as given.
fn language_name(code: &str) -> String {
    let base = code
        .trim()
        .split(['-', '_'])
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();
    let name = match base.as_str() {
        "es" => "español",
        "en" => "inglés",
        "fr" => "francés",
        "de" => "alemán",
        "pt" => "portugués",
        "it" => "italiano",
        _ => return code.trim().to_string(),
    };
    name.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn prd() -> GenerationRequest {
        GenerationRequest {
            language: "es".into(),
            mode: Mode::Prose,
            targets: vec!["ñ".into(), "z".into(), "ci".into()],
            // At 300 chars and 3×: ceil(3.33) = 4, ceil(1.98) = 2, ceil(7.65) = 8.
            frequencies: vec![0.0037, 0.0022, 0.0085],
            density: 3.0,
            length: 250..301,
            allowed_chars: "abcñ.,".into(),
            topic: Some("viajes".into()),
        }
    }

    #[test]
    fn prose_prompt_is_short_with_counts_and_the_word_run_fallback() {
        assert_eq!(
            build_prompt(&prd()),
            "Escribe un texto en español para practicar mecanografía.\n\
             Longitud: 250–300 caracteres. Tema: viajes.\n\
             Caracteres objetivo: ñ (al menos 4 veces), z (al menos 2 veces), \"ci\" (al menos 8 veces).\n\
             Empieza con frases naturales. Si con frases con sentido no llegas a esas cantidades, \
             completa el resto con palabras reales del español que contengan los caracteres objetivo, \
             separadas por espacios y con un punto al final.\n\
             Usa solo estos caracteres: a b c ñ . , (y el espacio).\n\
             Texto plano en una sola línea: sin listas, sin markdown, sin comillas tipográficas.\n\
             Responde solo con JSON: {\"text\": \"...\"}"
        );
    }

    #[test]
    fn the_prompt_counts_are_the_validator_counts_at_the_longest_length() {
        let req = prd();
        let shown: Vec<String> = req
            .min_counts(300)
            .into_iter()
            .map(|(t, n)| {
                let t = if t.chars().count() > 1 {
                    format!("\"{t}\"")
                } else {
                    t.to_owned()
                };
                format!("{t} (al menos {n} veces)")
            })
            .collect();
        assert!(build_prompt(&req).contains(&shown.join(", ")));
        // One occurrence is singular; a target without frequency needs one.
        let mut req = prd();
        req.targets = vec!["ntr".into()];
        req.frequencies = Vec::new();
        assert!(
            build_prompt(&req).contains("Caracteres objetivo: \"ntr\" (al menos 1 vez)."),
            "{}",
            build_prompt(&req)
        );
    }

    #[test]
    fn language_names_topic_and_density_are_parameterized() {
        let names = [
            ("en", "inglés"),
            ("fr", "francés"),
            ("de-DE", "alemán"),
            ("pt_BR", "portugués"),
            ("IT", "italiano"),
            ("es", "español"),
            ("nl", "nl"),
        ];
        for (code, name) in names {
            let mut req = prd();
            req.language = code.into();
            let p = build_prompt(&req);
            assert!(
                p.starts_with(&format!("Escribe un texto en {name} para")),
                "{code}"
            );
            assert!(
                p.contains(&format!("palabras reales del {name} que")),
                "{code}"
            );
        }
        let mut req = prd();
        req.topic = None;
        assert!(build_prompt(&req).contains("Tema: libre."));
        req.topic = Some("  \n ".into());
        assert!(build_prompt(&req).contains("Tema: libre."));
        req.topic = Some("la\nmar".into());
        assert!(build_prompt(&req).contains("Tema: la mar."));
        // The density (clamped to 2..=5, 3 when not a number) scales the count:
        // ceil(d × 0.01 × 300).
        req.targets = vec!["ñ".into()];
        req.frequencies = vec![0.01];
        for (density, shown) in [(1.0, "6"), (9.0, "15"), (2.5, "8"), (f32::NAN, "9")] {
            req.density = density;
            assert!(
                build_prompt(&req).contains(&format!("ñ (al menos {shown} veces).")),
                "{density}"
            );
        }
    }

    #[test]
    fn allowed_chars_are_deduplicated_without_whitespace() {
        let mut req = prd();
        req.allowed_chars = "aa b\tc".into();
        assert!(build_prompt(&req).contains("Usa solo estos caracteres: a b c (y el espacio)."));
    }

    #[test]
    fn code_mode_asks_for_a_snippet_with_the_counts() {
        let mut req = prd();
        req.mode = Mode::Code {
            lang: "rust".into(),
        };
        assert_eq!(
            build_prompt(&req),
            "Eres un generador de código para practicar mecanografía.\n\
             Escribe un fragmento de código en rust de 5 a 20 líneas, con comentarios en español. \
             Longitud: 250–300 caracteres. Tema: viajes.\n\
             Caracteres objetivo: ñ (al menos 4 veces), z (al menos 2 veces), \"ci\" (al menos 8 veces).\n\
             Usa solo estos caracteres: a b c ñ . , (y el espacio y los saltos de línea).\n\
             Sin markdown ni bloques ```, sin comillas tipográficas.\n\
             Responde solo con JSON: {\"text\": \"...\"}"
        );
    }
}
