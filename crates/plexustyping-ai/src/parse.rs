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
    let value: serde_json::Value = serde_json::from_str(body).map_err(|_| bad())?;
    value
        .get("text")
        .and_then(serde_json::Value::as_str)
        .map(str::to_string)
        .ok_or_else(bad)
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
    fn anything_else_is_bad_response() {
        for raw in [
            "",
            "hola",
            "Here is your text: {\"text\": \"hola\"}",
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
