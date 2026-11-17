//! Parsing of the model's `{"text": "..."}` answer.

use crate::ProviderError;

/// Reads the `text` field of a `{"text": "..."}` object, allowing surrounding
/// whitespace and a Markdown code fence (```` ```json ````). Anything else is
/// [`ProviderError::BadResponse`].
pub fn parse_text_json(raw: &str) -> Result<String, ProviderError> {
    let bad =
        || ProviderError::bad_response("The answer was not a {\"text\": \"...\"} JSON object.");
    let mut body = raw.trim();
    if let Some(rest) = body.strip_prefix("```") {
        let rest = rest.strip_suffix("```").ok_or_else(bad)?;
        let info_end = rest.find('\n').unwrap_or(0);
        let info = rest[..info_end].trim();
        if !info.is_empty() && !info.eq_ignore_ascii_case("json") {
            return Err(bad());
        }
        body = rest[info_end..].trim();
    }
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(body) {
        return text_of(&value).ok_or_else(bad);
    }
    // Models without JSON mode may put words around the object: read the
    // outermost {...} if it is a {"text": ...} object.
    if let (Some(start), Some(end)) = (body.find('{'), body.rfind('}'))
        && start < end
        && let Ok(value) = serde_json::from_str::<serde_json::Value>(&body[start..=end])
    {
        return text_of(&value).ok_or_else(bad);
    }
    // Or answer with the text itself: take it as is (validation still checks
    // its length, characters and Markdown). Anything that looks like broken
    // JSON or a broken fence stays a bad response.
    if body.is_empty() || body.starts_with(['{', '[', '`']) {
        return Err(bad());
    }
    let plain = body
        .strip_prefix('"')
        .and_then(|b| b.strip_suffix('"'))
        .unwrap_or(body);
    Ok(plain.trim().to_string())
}

/// The `text` of a `{"text": ...}` object, or a bare JSON string.
fn text_of(value: &serde_json::Value) -> Option<String> {
    value
        .get("text")
        .and_then(serde_json::Value::as_str)
        .or_else(|| value.as_str().filter(|s| !s.trim().is_empty()))
        .map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_object_and_whitespace() {
        assert_eq!(parse_text_json(r#"{"text": "hola"}"#).unwrap(), "hola");
        assert_eq!(
            parse_text_json("\n  {\"text\": \"a \\\"b\\\"\", \"extra\": 1}  \n").unwrap(),
            "a \"b\""
        );
    }

    #[test]
    fn fenced_object() {
        for raw in [
            "```json\n{\"text\": \"hola\"}\n```",
            "  ```JSON\n{\"text\": \"hola\"}```\n",
            "```\n{\"text\": \"hola\"}\n```",
        ] {
            assert_eq!(parse_text_json(raw).unwrap(), "hola", "{raw:?}");
        }
    }

    #[test]
    fn an_object_inside_other_words_or_plain_prose_is_read_too() {
        // Models without JSON mode (Gemma on Gemini) may wrap the object or skip it.
        assert_eq!(
            parse_text_json("Here is your text: {\"text\": \"hola\"}").unwrap(),
            "hola"
        );
        assert_eq!(
            parse_text_json("Aquí tienes:\n{\"text\": \"hola\"}\nSuerte.").unwrap(),
            "hola"
        );
        assert_eq!(
            parse_text_json("  La casa es azul.  ").unwrap(),
            "La casa es azul."
        );
        assert_eq!(
            parse_text_json("\"La casa es azul.\"").unwrap(),
            "La casa es azul."
        );
    }

    #[test]
    fn anything_else_is_bad_response() {
        for raw in [
            "",
            "   ",
            "{\"text\": 3}",
            "{\"other\": \"hola\"}",
            "[\"hola\"]",
            "```json\n{\"text\": \"hola\"}",
        ] {
            assert!(
                matches!(parse_text_json(raw), Err(ProviderError::BadResponse { .. })),
                "{raw:?}"
            );
        }
    }
}
