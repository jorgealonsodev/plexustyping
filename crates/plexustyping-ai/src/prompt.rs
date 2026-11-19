//! The generation prompt (PRD §4 example, parameterized).

use crate::{GenerationRequest, Mode};

/// Topics drawn at random when the user leaves the topic free, so models do not
/// write the same "El amanecer en la montaña…" text again and again.
pub const FREE_TOPICS: [&str; 24] = [
    "una receta de cocina",
    "un partido de fútbol",
    "un viaje en tren",
    "un día de trabajo en una oficina",
    "una conversación entre amigos",
    "un descubrimiento científico",
    "una mascota traviesa",
    "un mercado de barrio",
    "una tormenta de verano",
    "una mudanza",
    "un concierto",
    "un huerto urbano",
    "una noticia de tecnología",
    "un museo",
    "una carta a un familiar",
    "un problema de matemáticas",
    "una excursión en bicicleta",
    "una película de misterio",
    "la historia de un inventor",
    "una tienda de reparaciones",
    "un festival popular",
    "una entrevista de trabajo",
    "un día en la playa",
    "una biblioteca",
];

/// The free topic for `nonce` (any number; the caller passes something that varies).
pub fn free_topic(nonce: u64) -> &'static str {
    // SplitMix64 finalizer: neighbouring nonces still pick unrelated topics.
    let mut z = nonce.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^= z >> 31;
    FREE_TOPICS[(z % FREE_TOPICS.len() as u64) as usize]
}

/// The non-blank characters of `chars`, each once, in order.
fn distinct_chars(chars: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for ch in chars.chars().filter(|c| !c.is_whitespace()) {
        let ch = ch.to_string();
        if !out.contains(&ch) {
            out.push(ch);
        }
    }
    out
}

/// Builds the Spanish prompt for `req`. Only the request's own fields go into
/// it.
///
/// Prose asks only for a text in the language, its length range and topic
/// (no targets or allowed characters; [`crate::validate`] checks no density
/// for prose), plus the allowed characters when
/// [`GenerationRequest::restrict_chars`] is set (progressive unlock). Code also names the language, the targets with their minimum
/// counts and the allowed characters; each count is
/// [`GenerationRequest::min_counts`] at the longest accepted length, so a
/// snippet that meets it always meets [`crate::validate`]. Prose also asks
/// for a varied start (not «El» or «La»).
pub fn build_prompt(req: &GenerationRequest) -> String {
    const ANSWER: &str = "Responde solo con JSON: {\"text\": \"...\"}";
    let language = language_name(&req.language);
    let longest = req.length.end.saturating_sub(1).max(req.length.start);
    let topic = req
        .topic
        .as_deref()
        .map(|t| t.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|t| !t.is_empty())
        .unwrap_or_else(|| "libre".into());
    let Mode::Code { lang } = &req.mode else {
        let mut lines = vec![
            format!(
                "Escribe un texto en {language} de {}–{longest} caracteres.",
                req.length.start
            ),
            format!("Tema: {topic}."),
            "No empieces con «El» ni con «La».".to_string(),
        ];
        if req.restrict_chars {
            lines.push(format!(
                "Usa solo estos caracteres: {} (y el espacio). Puedes usar palabras sueltas si no salen frases.",
                distinct_chars(&req.allowed_chars).join(" ")
            ));
        }
        lines.push(ANSWER.to_string());
        return lines.join("\n");
    };
    let length = format!("Longitud: {}–{longest} caracteres.", req.length.start);
    let mut lines = vec![
        "Eres un generador de código para practicar mecanografía.".to_string(),
        format!(
            "Escribe un fragmento de código en {} de 5 a 20 líneas, con comentarios en {language}. {length} Tema: {topic}.",
            lang.trim()
        ),
    ];
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
    }
    lines.push(format!(
        "Usa solo estos caracteres: {} (y el espacio y los saltos de línea).",
        distinct_chars(&req.allowed_chars).join(" ")
    ));
    lines.push("Sin markdown ni bloques ```, sin comillas tipográficas.".to_string());
    lines.push(ANSWER.to_string());
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
            restrict_chars: false,
            topic: Some("viajes".into()),
        }
    }

    #[test]
    fn prose_asks_for_a_varied_start() {
        // "le doy a recargar textos y todos empiezan por EL"
        assert!(build_prompt(&prd()).contains("No empieces con «El» ni con «La»."));
        assert!(!build_prompt(&code(prd())).contains("No empieces"));
    }

    #[test]
    fn free_topics_are_many_and_vary_with_the_nonce() {
        assert!(FREE_TOPICS.len() >= 20);
        let picked: std::collections::BTreeSet<&str> = (0..200).map(free_topic).collect();
        assert!(picked.len() >= 15, "{picked:?}");
        for t in FREE_TOPICS {
            assert!(!t.is_empty() && !t.contains('\n'));
        }
    }

    fn code(mut req: GenerationRequest) -> GenerationRequest {
        req.mode = Mode::Code {
            lang: "rust".into(),
        };
        req
    }

    #[test]
    fn restricted_prose_prompt_lists_only_the_allowed_characters() {
        // Progressive unlock: the AI is told which characters are unlocked.
        let req = GenerationRequest {
            restrict_chars: true,
            ..prd()
        };
        assert_eq!(
            build_prompt(&req),
            "Escribe un texto en español de 250–300 caracteres.\n\
             Tema: viajes.\n\
             No empieces con «El» ni con «La».\n\
             Usa solo estos caracteres: a b c ñ . , (y el espacio). Puedes usar palabras sueltas si no salen frases.\n\
             Responde solo con JSON: {\"text\": \"...\"}"
        );
    }

    #[test]
    fn prose_prompt_only_asks_for_a_text_in_the_language() {
        // S9: no targets, counts, word-run fallback or allowed characters.
        assert_eq!(
            build_prompt(&prd()),
            "Escribe un texto en español de 250–300 caracteres.\n\
             Tema: viajes.\n\
             No empieces con «El» ni con «La».\n\
             Responde solo con JSON: {\"text\": \"...\"}"
        );
    }

    #[test]
    fn the_code_prompt_counts_are_the_validator_counts_at_the_longest_length() {
        let req = code(prd());
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
        let mut req = code(prd());
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
        for (code_name, name) in names {
            let mut req = prd();
            req.language = code_name.into();
            assert!(
                build_prompt(&req).starts_with(&format!("Escribe un texto en {name} de 250–300")),
                "{code_name}"
            );
            req = code(req);
            assert!(
                build_prompt(&req).contains(&format!("comentarios en {name}.")),
                "{code_name}"
            );
        }
        let mut req = prd();
        req.topic = None;
        assert!(build_prompt(&req).contains("\nTema: libre.\n"));
        req.topic = Some("  \n ".into());
        assert!(build_prompt(&req).contains("\nTema: libre.\n"));
        req.topic = Some("la\nmar".into());
        assert!(build_prompt(&req).contains("\nTema: la mar.\n"));
        // A one-length range shows the same bound twice.
        req.length = 40..41;
        assert!(build_prompt(&req).starts_with("Escribe un texto en español de 40–40 caracteres."));
        // In code, the density (clamped to 2..=5, 3 when not a number) scales
        // the count: ceil(d × 0.01 × 300).
        let mut req = code(prd());
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
    fn code_allowed_chars_are_deduplicated_without_whitespace() {
        let mut req = code(prd());
        req.allowed_chars = "aa b\tc".into();
        assert!(
            build_prompt(&req)
                .contains("Usa solo estos caracteres: a b c (y el espacio y los saltos de línea).")
        );
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
