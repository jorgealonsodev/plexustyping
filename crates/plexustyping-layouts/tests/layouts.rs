//! Public-API tests for layout parsing, dead-key composition and key lookup (S5).

use plexustyping_layouts::{
    KeyLayers, KeyPress, Layer, Layout, LayoutError, builtin, builtin_ids, parse,
};

/// The file example quoted from the PRD (RF7).
const PRD_EXAMPLE: &str = r#"
name = "Español QWERTY"
dead_keys = ["´", "`", "^", "¨"]

[keys.KeyQ]
normal = "q"
shift = "Q"

[keys.Semicolon]
normal = "ñ"
shift = "Ñ"

[keys.Digit2]
normal = "2"
shift = "\""
altgr = "@"

[keys.Quote]
normal = "´"
shift = "¨"

[keys.BracketLeft]
normal = "`"
shift = "^"
"#;

fn es() -> Layout {
    builtin("es-qwerty").expect("es-qwerty is built in")
}

fn press(code: &str, layer: Layer) -> KeyPress {
    KeyPress {
        code: code.to_owned(),
        layer,
    }
}

fn parse_err(source: &str) -> String {
    match parse("t", source) {
        Ok(layout) => panic!("expected an error, parsed {layout:?}"),
        Err(err) => err.to_string(),
    }
}

#[test]
fn prd_example_parses() {
    let layout = parse("es-qwerty", PRD_EXAMPLE).unwrap();
    assert_eq!(layout.id, "es-qwerty");
    assert_eq!(layout.name, "Español QWERTY");
    assert_eq!(layout.dead_keys, vec!['´', '`', '^', '¨']);
    assert_eq!(
        layout.keys["KeyQ"],
        KeyLayers {
            normal: Some('q'),
            shift: Some('Q'),
            altgr: None
        }
    );
    assert_eq!(
        layout.keys["Semicolon"],
        KeyLayers {
            normal: Some('ñ'),
            shift: Some('Ñ'),
            altgr: None
        }
    );
    assert_eq!(
        layout.keys["Digit2"],
        KeyLayers {
            normal: Some('2'),
            shift: Some('"'),
            altgr: Some('@')
        }
    );
    assert_eq!(layout.keys.len(), 5);
}

#[test]
fn rejects_invalid_toml_syntax() {
    let msg = parse_err("name = \"x\n");
    assert!(msg.starts_with("layout `t`: invalid TOML: "), "{msg}");
}

#[test]
fn rejects_unknown_top_level_field() {
    let msg = parse_err("name = \"x\"\nauthor = \"me\"\n[keys.KeyA]\nnormal = \"a\"\n");
    assert!(msg.starts_with("layout `t`: invalid TOML: "), "{msg}");
    assert!(msg.contains("unknown field `author`"), "{msg}");
}

#[test]
fn rejects_unknown_layer_field() {
    let msg = parse_err("name = \"x\"\n[keys.KeyA]\nnormal = \"a\"\ncapslock = \"A\"\n");
    assert!(msg.contains("unknown field `capslock`"), "{msg}");
}

#[test]
fn rejects_layer_value_that_is_not_one_char() {
    let too_long = parse_err("name = \"x\"\n[keys.KeyQ]\nnormal = \"qq\"\n");
    assert_eq!(
        too_long,
        "layout `t`: key `KeyQ` layer `normal` must be exactly one character, got \"qq\""
    );
    let empty = parse_err("name = \"x\"\n[keys.KeyQ]\nnormal = \"q\"\naltgr = \"\"\n");
    assert_eq!(
        empty,
        "layout `t`: key `KeyQ` layer `altgr` must be exactly one character, got \"\""
    );
}

#[test]
fn rejects_dead_key_that_is_not_one_char() {
    let msg = parse_err("name = \"x\"\ndead_keys = [\"´´\"]\n[keys.Quote]\nnormal = \"´\"\n");
    assert_eq!(
        msg,
        "layout `t`: dead key \"´´\" must be exactly one character"
    );
}

#[test]
fn rejects_dead_key_not_produced_by_any_key() {
    let msg = parse_err("name = \"x\"\ndead_keys = [\"^\"]\n[keys.KeyA]\nnormal = \"a\"\n");
    assert_eq!(msg, "layout `t`: dead key `^` is not produced by any key");
}

#[test]
fn rejects_empty_name() {
    let msg = parse_err("name = \"  \"\n[keys.KeyA]\nnormal = \"a\"\n");
    assert_eq!(msg, "layout `t`: name must not be empty");
}

#[test]
fn rejects_empty_keys() {
    assert_eq!(
        parse_err("name = \"x\"\n"),
        "layout `t`: at least one key must be defined"
    );
    assert_eq!(
        parse_err("name = \"x\"\n[keys]\n"),
        "layout `t`: at least one key must be defined"
    );
}

#[test]
fn errors_are_typed() {
    assert!(matches!(
        parse("t", "name = \"x\"\n"),
        Err(LayoutError::EmptyKeys { .. })
    ));
}

#[test]
fn compose_dead_keys() {
    let layout = parse("es-qwerty", PRD_EXAMPLE).unwrap();
    assert_eq!(layout.compose('´', 'a'), Some('á'));
    assert_eq!(layout.compose('´', 'A'), Some('Á'));
    assert_eq!(layout.compose('¨', 'u'), Some('ü'));
    assert_eq!(layout.compose('`', 'e'), Some('è'));
    assert_eq!(layout.compose('^', 'o'), Some('ô'));
    assert_eq!(layout.compose('´', 'q'), None);
    assert_eq!(layout.compose('´', ' '), None);
}

#[test]
fn compose_requires_a_declared_dead_key() {
    let layout = parse("es-qwerty", PRD_EXAMPLE).unwrap();
    // `~` maps to a combining tilde but is not a dead key of this layout.
    assert_eq!(layout.compose('~', 'n'), None);
    // A plain character never composes.
    assert_eq!(layout.compose('q', 'a'), None);
    let with_tilde = parse(
        "t",
        "name = \"x\"\ndead_keys = [\"~\"]\n[keys.Digit4]\naltgr = \"~\"\n[keys.KeyN]\nnormal = \"n\"\n",
    )
    .unwrap();
    assert_eq!(with_tilde.compose('~', 'n'), Some('ñ'));
}

#[test]
fn builtin_es_qwerty_is_listed_and_valid() {
    assert_eq!(builtin_ids(), &["es-qwerty", "latam-qwerty", "us-qwerty"]);
    for id in builtin_ids() {
        assert_eq!(builtin(id).map(|layout| layout.id), Some((*id).to_owned()));
    }
    let layout = es();
    assert_eq!(layout.id, "es-qwerty");
    assert_eq!(layout.name, "Español (España)");
    assert_eq!(layout.dead_keys, vec!['´', '`', '^', '¨']);
    assert_eq!(
        layout.keys["KeyQ"],
        KeyLayers {
            normal: Some('q'),
            shift: Some('Q'),
            altgr: None
        }
    );
    assert_eq!(
        layout.keys["Digit2"],
        KeyLayers {
            normal: Some('2'),
            shift: Some('"'),
            altgr: Some('@')
        }
    );
    assert_eq!(
        layout.keys["Backquote"],
        KeyLayers {
            normal: Some('º'),
            shift: Some('ª'),
            altgr: Some('\\')
        }
    );
    assert_eq!(layout.keys["KeyE"].altgr, Some('€'));
    assert_eq!(layout.keys.len(), 49);
    assert_eq!(builtin("missing"), None);
}

#[test]
fn builtin_allowed_chars() {
    let allowed = es().allowed_chars();
    let expected = ('a'..='z')
        .chain('A'..='Z')
        .chain("ñÑáéíóúÁÉÍÓÚü¿?¡!,.;: \n".chars());
    for ch in expected {
        assert!(allowed.contains(&ch), "missing {ch:?}");
    }
    for ch in ['´', '¨', '`', '^'] {
        assert!(
            !allowed.contains(&ch),
            "dead key {ch:?} must not be standalone"
        );
    }
    assert!(allowed.contains(&'Ü') && allowed.contains(&'è') && allowed.contains(&'ô'));
    assert!(!allowed.contains(&'ã'), "~ is not a dead key here");
}

#[test]
fn builtin_locate() {
    let layout = es();
    assert_eq!(
        layout.locate('ñ'),
        Some(vec![press("Semicolon", Layer::Normal)])
    );
    assert_eq!(
        layout.locate('Ñ'),
        Some(vec![press("Semicolon", Layer::Shift)])
    );
    assert_eq!(
        layout.locate('@'),
        Some(vec![press("Digit2", Layer::Altgr)])
    );
    assert_eq!(
        layout.locate('á'),
        Some(vec![
            press("Quote", Layer::Normal),
            press("KeyA", Layer::Normal)
        ])
    );
    assert_eq!(
        layout.locate('Ü'),
        Some(vec![
            press("Quote", Layer::Shift),
            press("KeyU", Layer::Shift)
        ])
    );
    assert_eq!(
        layout.locate(' '),
        Some(vec![press("Space", Layer::Normal)])
    );
    assert_eq!(
        layout.locate('\n'),
        Some(vec![press("Enter", Layer::Normal)])
    );
    assert_eq!(layout.locate('´'), None);
    assert_eq!(layout.locate('ã'), None);
}

#[test]
fn locate_prefers_normal_then_shift_then_altgr() {
    let layout = parse(
        "t",
        "name = \"x\"\n[keys.KeyA]\naltgr = \"x\"\n[keys.KeyB]\nshift = \"x\"\n[keys.KeyC]\nnormal = \"x\"\n",
    )
    .unwrap();
    assert_eq!(layout.locate('x'), Some(vec![press("KeyC", Layer::Normal)]));
}

#[test]
fn layout_serializes_camel_case() {
    let layout = parse("t", "name = \"x\"\n[keys.KeyA]\nnormal = \"a\"\n").unwrap();
    let toml = toml::to_string(&layout).unwrap();
    assert!(toml.contains("deadKeys = []"), "{toml}");
}

#[test]
fn rejects_dead_key_without_known_composition() {
    let source = "name = \"Cedilla\"\ndead_keys = [\"¸\"]\n\n[keys.KeyA]\nnormal = \"a\"\n\n[keys.Quote]\nnormal = \"¸\"\n";
    let err = parse("cedilla", source).unwrap_err();
    assert!(
        matches!(err, LayoutError::UnsupportedDeadKey { dead: '¸', .. }),
        "{err:?}"
    );
    assert_eq!(
        err.to_string(),
        "layout `cedilla`: dead key `¸` has no known composition"
    );
}

#[test]
fn builtin_latam_qwerty_types_spanish_and_code_signs() {
    let layout = builtin("latam-qwerty").expect("latam-qwerty is built in");
    assert_eq!(layout.name, "Español (Latinoamérica)");
    assert_eq!(layout.dead_keys, vec!['´', '¨', '^', '`']);
    assert_eq!(layout.keys.len(), 49);
    assert_eq!(
        layout.keys["Backquote"],
        KeyLayers {
            normal: Some('|'),
            shift: Some('°'),
            altgr: Some('¬')
        }
    );
    assert_eq!(
        layout.locate('ñ'),
        Some(vec![press("Semicolon", Layer::Normal)])
    );
    assert_eq!(layout.locate('@'), Some(vec![press("KeyQ", Layer::Altgr)]));
    assert_eq!(
        layout.locate('\\'),
        Some(vec![press("Minus", Layer::Altgr)])
    );
    assert_eq!(
        layout.locate('~'),
        Some(vec![press("BracketRight", Layer::Altgr)])
    );
    assert_eq!(
        layout.locate('{'),
        Some(vec![press("Quote", Layer::Normal)])
    );
    assert_eq!(
        layout.locate(']'),
        Some(vec![press("Backslash", Layer::Shift)])
    );
    assert_eq!(
        layout.locate('¿'),
        Some(vec![press("Equal", Layer::Normal)])
    );
    assert_eq!(
        layout.locate('á'),
        Some(vec![
            press("BracketLeft", Layer::Normal),
            press("KeyA", Layer::Normal)
        ])
    );
    assert_eq!(
        layout.locate('ü'),
        Some(vec![
            press("BracketLeft", Layer::Shift),
            press("KeyU", Layer::Normal)
        ])
    );
    assert_eq!(
        layout.locate('ê'),
        Some(vec![
            press("Quote", Layer::Altgr),
            press("KeyE", Layer::Normal)
        ])
    );
    assert_eq!(layout.locate('^'), None);
    assert_eq!(layout.locate('ç'), None);
    let allowed = layout.allowed_chars();
    for ch in "ñÑáéíóúÁÉÍÓÚü¿?¡!,.;:<>()[]{}=+-*/_#$%&|@'\"".chars() {
        assert!(allowed.contains(&ch), "missing {ch:?}");
    }
}

#[test]
fn builtin_us_qwerty_is_ansi_without_dead_keys_or_altgr() {
    let layout = builtin("us-qwerty").expect("us-qwerty is built in");
    assert_eq!(layout.name, "English (US)");
    assert!(layout.dead_keys.is_empty());
    assert_eq!(layout.keys.len(), 48);
    assert!(!layout.keys.contains_key("IntlBackslash"));
    assert!(layout.keys.values().all(|key| key.altgr.is_none()));
    assert_eq!(
        layout.keys["Backquote"],
        KeyLayers {
            normal: Some('`'),
            shift: Some('~'),
            altgr: None
        }
    );
    assert_eq!(
        layout.locate('@'),
        Some(vec![press("Digit2", Layer::Shift)])
    );
    assert_eq!(
        layout.locate('^'),
        Some(vec![press("Digit6", Layer::Shift)])
    );
    assert_eq!(
        layout.locate('`'),
        Some(vec![press("Backquote", Layer::Normal)])
    );
    assert_eq!(
        layout.locate('|'),
        Some(vec![press("Backslash", Layer::Shift)])
    );
    assert_eq!(layout.locate('"'), Some(vec![press("Quote", Layer::Shift)]));
    assert_eq!(layout.locate('?'), Some(vec![press("Slash", Layer::Shift)]));
    assert_eq!(layout.locate('ñ'), None);
    assert_eq!(layout.locate('á'), None);
    let allowed = layout.allowed_chars();
    for ch in (' '..='~').chain(['\n']) {
        assert!(allowed.contains(&ch), "missing {ch:?}");
    }
    assert_eq!(allowed.len(), 96);
}
