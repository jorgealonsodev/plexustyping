//! Behaviour tests for the public highlighting API.

use std::time::{Duration, Instant};

use plexustyping_highlight::{Token, TokenClass, highlight, syntaxes};

const IDS: [&str; 16] = [
    "python",
    "javascript",
    "typescript",
    "java",
    "csharp",
    "cpp",
    "c",
    "go",
    "rust",
    "php",
    "sql",
    "bash",
    "html",
    "css",
    "kotlin",
    "swift",
];

/// One realistic snippet per supported id.
fn sample(id: &str) -> &'static str {
    match id {
        "python" => {
            "def greet(name):\n    # say hi\n    return f\"hi {name}\" if name else 'none'\n"
        }
        "javascript" => "const add = (a, b) => a + b;\nconsole.log(add(1, 2)); // 3\n",
        "typescript" => {
            "interface P { x: number }\nfunction f(p: P): string { return `${p.x}`; }\n"
        }
        "java" => "public class A {\n  int f(String s) { return s.length(); }\n}\n",
        "csharp" => "public class A { int X => 1; /* c */ }\n",
        "cpp" => "#include <vector>\nint main() { std::vector<int> v{1}; return v[0]; }\n",
        "c" => "#include <stdio.h>\nint main(void) { printf(\"%d\\n\", 1); }\n",
        "go" => "func f(x int) error {\n\treturn fmt.Errorf(\"x %d\", x)\n}\n",
        "rust" => "fn main() {\n    let v: Vec<u8> = vec![1, 2];\n    println!(\"{v:?}\");\n}\n",
        "php" => "$x = strlen(\"a\");\nfunction f($a) { return $a ?? 0; }\n",
        "sql" => "SELECT id, name FROM users WHERE age > 18; -- adults\n",
        "bash" => "for f in *.txt; do\n  echo \"$f\" && cat \"$HOME/$f\"\ndone\n",
        "html" => "<div class=\"a\">\n  <p>Hi &amp; bye</p>\n</div>\n",
        "css" => ".a > p { color: #fff; margin: 0 auto; }\n",
        "kotlin" => "fun f(x: Int): String = \"a\"\nval y = listOf(1, 2)\n",
        "swift" => "func f(x: Int) -> String { return \"a\" }\nlet y = [1, 2]\n",
        other => panic!("no sample for {other}"),
    }
}

/// Asserts the tokens tile `code` exactly: ordered, gap-free, non-empty and
/// with no two adjacent tokens sharing a class.
fn assert_contiguous(code: &str, tokens: &[Token]) {
    let len = code.chars().count();
    let mut next = 0;
    for (i, t) in tokens.iter().enumerate() {
        assert_eq!(t.start, next, "gap or overlap at token {i}: {tokens:?}");
        assert!(t.end > t.start, "empty token {i}: {tokens:?}");
        if i > 0 {
            assert_ne!(tokens[i - 1].class, t.class, "unmerged tokens at {i}");
        }
        next = t.end;
    }
    assert_eq!(next, len, "tokens must cover the whole input");
}

/// The class of the token that covers the `needle` text, which must lie in a
/// single token.
fn class_of(code: &str, tokens: &[Token], needle: &str) -> TokenClass {
    let byte = code.find(needle).expect("needle in code");
    let start = code[..byte].chars().count();
    let end = start + needle.chars().count();
    let t = tokens
        .iter()
        .find(|t| t.start <= start && end <= t.end)
        .unwrap_or_else(|| panic!("{needle:?} spans several tokens: {tokens:?}"));
    t.class
}

fn tokens(id: &str, code: &str) -> Vec<Token> {
    let tokens = highlight(id, code).expect("known syntax");
    assert_contiguous(code, &tokens);
    tokens
}

#[test]
fn lists_the_sixteen_ids_in_order() {
    assert_eq!(syntaxes(), IDS);
}

#[test]
fn every_id_resolves_and_covers_its_sample_contiguously() {
    for id in syntaxes() {
        let code = sample(id);
        let tokens = highlight(id, code).unwrap_or_else(|| panic!("{id} did not resolve"));
        assert_contiguous(code, &tokens);
        assert!(
            tokens.iter().any(|t| t.class != TokenClass::Plain),
            "{id} produced no classes: {tokens:?}"
        );
    }
}

#[test]
fn empty_input_yields_no_tokens() {
    assert_eq!(highlight("rust", ""), Some(vec![]));
}

#[test]
fn rust_keyword_function_number_comment() {
    let code = "fn main() { let x = 42; // hi }";
    let t = tokens("rust", code);
    assert_eq!(class_of(code, &t, "fn"), TokenClass::Keyword);
    assert_eq!(class_of(code, &t, "main"), TokenClass::Function);
    assert_eq!(class_of(code, &t, "42"), TokenClass::Number);
    assert_eq!(class_of(code, &t, "// hi }"), TokenClass::Comment);
    assert_eq!(class_of(code, &t, "="), TokenClass::Operator);
    assert_eq!(class_of(code, &t, "{"), TokenClass::Punctuation);
}

#[test]
fn python_def_and_string() {
    let code = "def f(a):\n    return \"hi\"\n";
    let t = tokens("python", code);
    assert_eq!(class_of(code, &t, "def"), TokenClass::Keyword);
    assert_eq!(class_of(code, &t, "\"hi\""), TokenClass::String);
}

#[test]
fn sql_select_is_keyword() {
    let code = "SELECT id FROM t;";
    let t = tokens("sql", code);
    assert_eq!(class_of(code, &t, "SELECT"), TokenClass::Keyword);
}

#[test]
fn html_tag_and_attribute() {
    let code = "<a href=\"x\">y</a>";
    let t = tokens("html", code);
    assert_eq!(class_of(code, &t, "a"), TokenClass::Tag);
    assert_eq!(class_of(code, &t, "href"), TokenClass::Attribute);
    assert_eq!(class_of(code, &t, "\"x\""), TokenClass::String);
}

#[test]
fn css_property() {
    let code = "a { color: red; }";
    let t = tokens("css", code);
    assert_eq!(class_of(code, &t, "color"), TokenClass::Property);
    assert_eq!(class_of(code, &t, "red"), TokenClass::Constant);
}

#[test]
fn bash_home_is_variable() {
    let code = "echo $HOME";
    let t = tokens("bash", code);
    assert_eq!(class_of(code, &t, "$HOME"), TokenClass::Variable);
}

#[test]
fn typescript_type_annotation() {
    let code = "let n: number = 1;";
    let t = tokens("typescript", code);
    assert_eq!(class_of(code, &t, "number"), TokenClass::Type);
}

#[test]
fn php_without_open_tag_is_code() {
    let code = "$x = 1;";
    let t = tokens("php", code);
    assert_eq!(class_of(code, &t, "$x"), TokenClass::Variable);
}

#[test]
fn php_starting_with_an_open_tag_is_code() {
    for code in [
        "<?php\nfunction f() { return 1; }\n",
        "\n  <?php function f() { return 1; }",
    ] {
        let t = tokens("php", code);
        assert_eq!(
            class_of(code, &t, "function"),
            TokenClass::Keyword,
            "{code:?}"
        );
        assert_eq!(class_of(code, &t, "1"), TokenClass::Number, "{code:?}");
    }
}

#[test]
fn php_template_starting_with_markup_uses_the_html_grammar() {
    for code in [
        "<p>Hola</p>\n<?php echo 1; ?>\n",
        "\u{feff}<p>Hola</p>\n<?php echo 1; ?>\n",
    ] {
        let t = tokens("php", code);
        assert_ne!(class_of(code, &t, "p"), TokenClass::Plain, "{code:?}");
        assert_eq!(class_of(code, &t, "1"), TokenClass::Number, "{code:?}");
    }
}

#[test]
fn php_open_tag_inside_bare_code_keeps_the_source_grammar() {
    let code = "function f() { return \"<?xml\"; } // <?\n";
    let t = tokens("php", code);
    assert_eq!(class_of(code, &t, "function"), TokenClass::Keyword);
    assert_eq!(class_of(code, &t, "return"), TokenClass::Keyword);
    assert_eq!(class_of(code, &t, "\"<?xml\""), TokenClass::String);
}

#[test]
fn indices_are_chars_not_bytes() {
    let code = "let s = \"ñandú\"; // ü\nlet n = 7;";
    let t = tokens("rust", code);
    assert_eq!(class_of(code, &t, "\"ñandú\""), TokenClass::String);
    assert_eq!(class_of(code, &t, "7"), TokenClass::Number);
    let seven = t
        .iter()
        .find(|t| t.class == TokenClass::Number)
        .expect("number");
    assert_eq!((seven.start, seven.end), (30, 31));
}

#[test]
fn unknown_syntax_is_none() {
    assert_eq!(highlight("cobol", "MOVE A TO B."), None);
    assert_eq!(highlight("Rust", "fn x() {}"), None);
}

#[test]
fn garbage_never_panics_and_stays_contiguous() {
    // Deterministic xorshift so failures reproduce.
    let mut seed: u32 = 0x9e37_79b9;
    let unbalanced = [
        "\"unterminated",
        "'",
        "/* open",
        "<div",
        "${",
        "`",
        "\"\"\"",
        "#[",
        "<?php",
    ];
    for id in syntaxes() {
        for round in 0..unbalanced.len() {
            let mut code: String = (0..120)
                .map(|_| {
                    seed ^= seed << 13;
                    seed ^= seed >> 17;
                    seed ^= seed << 5;
                    char::from(b' ' + (seed % 95) as u8)
                })
                .collect();
            code.push('\n');
            code.push_str(unbalanced[round % unbalanced.len()]);
            tokens(id, &code);
        }
    }
}

#[test]
fn tokens_serialize_for_the_frontend() {
    let json = serde_json::to_string(&Token {
        start: 0,
        end: 2,
        class: TokenClass::Punctuation,
    })
    .expect("serialize");
    assert_eq!(json, r#"{"start":0,"end":2,"class":"punctuation"}"#);
}

#[test]
#[ignore = "wall-clock bound; run explicitly with --ignored on an idle machine"]
fn twenty_line_snippet_is_fast_after_warm_up() {
    let code = sample("rust").repeat(5);
    assert_eq!(code.lines().count(), 20);
    highlight("rust", &code).expect("warm-up");
    let started = Instant::now();
    tokens("rust", &code);
    let elapsed = started.elapsed();
    assert!(elapsed < Duration::from_millis(200), "took {elapsed:?}");
}

#[test]
fn shipped_syntax_licences_match_two_face() {
    let mut md = String::new();
    for license in two_face::acknowledgement::listing().for_syntaxes() {
        license.write_md(&mut md);
    }
    let shipped = include_str!("../SYNTAX_LICENSES.md");
    assert!(
        shipped.ends_with(&md),
        "regenerate SYNTAX_LICENSES.md with the acknowledgements example"
    );
}
