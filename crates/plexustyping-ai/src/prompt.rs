//! The generation prompt (PRD §4 example, parameterized).

use crate::{GenerationRequest, Mode};

/// Builds the Spanish prompt for `req`. Only the request's own fields go into
/// it (targets, language, mode, length, allowed characters, topic).
pub fn build_prompt(req: &GenerationRequest) -> String {
    let language = language_name(&req.language);
    let length = format!(
        "Longitud: {}–{} caracteres.",
        req.length.start,
        req.length.end.saturating_sub(1).max(req.length.start)
    );
    let topic = req
        .topic
        .as_deref()
        .map(|t| t.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|t| !t.is_empty())
        .unwrap_or_else(|| "libre".into());
    let mut lines = Vec::new();
    match &req.mode {
        Mode::Prose => {
            lines.push("Eres un generador de textos para practicar mecanografía.".to_string());
            lines.push(format!("Idioma: {language}. {length} Tema: {topic}."));
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
        .targets
        .iter()
        .filter(|t| !t.is_empty())
        .map(|t| {
            if t.chars().count() > 1 {
                format!("\"{t}\"")
            } else {
                t.clone()
            }
        })
        .collect();
    if !targets.is_empty() {
        lines.push(format!(
            "Caracteres objetivo: {}. Cada uno debe aparecer al menos {} veces más de lo habitual.",
            targets.join(", "),
            density_text(req.density)
        ));
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
        Mode::Prose => "Sin listas, sin markdown, sin comillas tipográficas.".to_string(),
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

/// `3` or `2,5` (Spanish decimal comma).
fn density_text(density: f32) -> String {
    let d = clamped_density(density);
    if d.fract() == 0.0 {
        format!("{d:.0}")
    } else {
        format!("{d:.1}").replace('.', ",")
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
            density: 3.0,
            length: 250..301,
            allowed_chars: "abcñ.,".into(),
            topic: Some("viajes".into()),
        }
    }

    #[test]
    fn prose_prompt_follows_the_prd_example() {
        assert_eq!(
            build_prompt(&prd()),
            "Eres un generador de textos para practicar mecanografía.\n\
             Idioma: español. Longitud: 250–300 caracteres. Tema: viajes.\n\
             Caracteres objetivo: ñ, z, \"ci\". Cada uno debe aparecer al menos 3 veces más de lo habitual.\n\
             Usa solo estos caracteres: a b c ñ . , (y el espacio).\n\
             Sin listas, sin markdown, sin comillas tipográficas.\n\
             Responde solo con JSON: {\"text\": \"...\"}"
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
            assert!(
                build_prompt(&req).contains(&format!("Idioma: {name}.")),
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
        for (density, shown) in [(1.0, "2"), (9.0, "5"), (2.5, "2,5"), (f32::NAN, "3")] {
            req.density = density;
            assert!(
                build_prompt(&req).contains(&format!("al menos {shown} veces más")),
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
    fn code_mode_asks_for_a_snippet() {
        let mut req = prd();
        req.mode = Mode::Code {
            lang: "rust".into(),
        };
        let p = build_prompt(&req);
        assert!(
            p.contains("fragmento de código en rust de 5 a 20 líneas"),
            "{p}"
        );
        assert!(p.contains("Longitud: 250–300 caracteres."), "{p}");
        assert!(p.contains("Caracteres objetivo: ñ, z, \"ci\"."), "{p}");
        assert!(
            p.ends_with("Responde solo con JSON: {\"text\": \"...\"}"),
            "{p}"
        );
    }
}
