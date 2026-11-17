//! Syntax highlighting for code mode.
//!
//! Parses a snippet with the `syntect` grammars bundled by `two-face` (the
//! `bat` curation, fancy-regex build) and reduces each scope stack to one
//! semantic [`TokenClass`] that the active theme colours.
//!
//! # Scope mapping
//!
//! Each run of text gets the class of its scope stack, decided in two steps:
//!
//! 1. If any scope is a `comment` or `string`, the innermost of those wins
//!    ([`TokenClass::Comment`] / [`TokenClass::String`]), so quote and
//!    comment delimiters colour like their contents.
//! 2. Otherwise the innermost scope with a matching rule wins; among the rules
//!    matching that scope, the longest (most specific) prefix wins. Rules on
//!    `meta.*` scopes only apply to the innermost scope, because grammars use
//!    them to wrap whole constructs (a call's arguments, for instance).
//!
//! | Scope prefix | Class |
//! | --- | --- |
//! | `constant.numeric` | Number |
//! | `constant.language`, `constant.other`, `support.constant` | Constant |
//! | `keyword.operator` | Operator |
//! | `keyword`, `storage`, `storage.modifier`, `storage.type.<decl>` ¹ | Keyword |
//! | `entity.name.function`, `support.function`, `variable.function`, `meta.function-call` | Function |
//! | `entity.name.type`/`class`/`struct`/`enum`/`trait`/`interface`, `support.type`, `support.class`, `storage.type` | Type |
//! | `variable`, `punctuation.definition.variable` | Variable |
//! | `entity.name.tag` | Tag |
//! | `entity.other.attribute-name` | Attribute |
//! | `support.type.property-name`, `meta.property-name`, `variable.other.property`, `meta.mapping.key`, `meta.property.object` | Property |
//! | `punctuation` | Punctuation |
//! | anything else | Plain |
//!
//! ¹ Declaration keywords that grammars scope as `storage.type`: `function`,
//! `keyword`, `class`, `struct`, `enum`, `trait`, `interface`, `impl`.

use std::sync::LazyLock;

use serde::Serialize;
use syntect::parsing::{ParseState, Scope, ScopeStack, SyntaxReference, SyntaxSet};
use syntect::util::LinesWithEndings;

/// Semantic class of a run of code; the theme maps each one to a colour.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum TokenClass {
    /// Text without a recognised scope.
    Plain,
    /// Language keyword or declaration keyword.
    Keyword,
    /// String literal, delimiters included.
    String,
    /// Numeric literal.
    Number,
    /// Comment, delimiters included.
    Comment,
    /// Function name, at a definition or a call.
    Function,
    /// Type or class name.
    Type,
    /// Variable or parameter.
    Variable,
    /// Built-in constant such as `true` or a CSS colour name.
    Constant,
    /// Operator.
    Operator,
    /// Punctuation: brackets, separators, terminators.
    Punctuation,
    /// Markup tag name.
    Tag,
    /// Markup attribute name.
    Attribute,
    /// Property or object key.
    Property,
}

/// A highlighted run of code. `start` and `end` are char indices (Unicode
/// scalar values) into the input; `end` is exclusive.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Token {
    /// First char of the run.
    pub start: usize,
    /// One past the last char of the run.
    pub end: usize,
    /// Semantic class of the run.
    pub class: TokenClass,
}

/// Supported syntax ids, paired with their `two-face` syntax names.
const SYNTAXES: [(&str, &str); 16] = [
    ("python", "Python"),
    ("javascript", "JavaScript"),
    ("typescript", "TypeScript"),
    ("java", "Java"),
    ("csharp", "C#"),
    ("cpp", "C++"),
    ("c", "C"),
    ("go", "Go"),
    ("rust", "Rust"),
    ("php", "PHP Source"),
    ("sql", "SQL"),
    ("bash", "Bourne Again Shell (bash)"),
    ("html", "HTML"),
    ("css", "CSS"),
    ("kotlin", "Kotlin"),
    ("swift", "Swift"),
];

const IDS: [&str; 16] = {
    let mut ids = [""; 16];
    let mut i = 0;
    while i < SYNTAXES.len() {
        ids[i] = SYNTAXES[i].0;
        i += 1;
    }
    ids
};

/// Scope prefix → class rules (see the crate docs).
const RULES: &[(&str, TokenClass)] = {
    use TokenClass::*;
    &[
        ("constant.numeric", Number),
        ("constant.language", Constant),
        ("constant.other", Constant),
        ("support.constant", Constant),
        ("keyword", Keyword),
        ("keyword.operator", Operator),
        ("storage", Keyword),
        ("storage.modifier", Keyword),
        ("storage.type", Type),
        ("storage.type.function", Keyword),
        ("storage.type.keyword", Keyword),
        ("storage.type.class", Keyword),
        ("storage.type.struct", Keyword),
        ("storage.type.enum", Keyword),
        ("storage.type.trait", Keyword),
        ("storage.type.interface", Keyword),
        ("storage.type.impl", Keyword),
        ("entity.name.function", Function),
        ("support.function", Function),
        ("variable.function", Function),
        ("meta.function-call", Function),
        ("entity.name.type", Type),
        ("entity.name.class", Type),
        ("entity.name.struct", Type),
        ("entity.name.enum", Type),
        ("entity.name.trait", Type),
        ("entity.name.interface", Type),
        ("support.type", Type),
        ("support.class", Type),
        ("variable", Variable),
        ("punctuation.definition.variable", Variable),
        ("entity.name.tag", Tag),
        ("entity.other.attribute-name", Attribute),
        ("support.type.property-name", Property),
        ("meta.property-name", Property),
        ("variable.other.property", Property),
        ("meta.mapping.key", Property),
        ("meta.property.object", Property),
        ("punctuation", Punctuation),
    ]
};

/// A compiled rule: prefix scope, its atom count and whether it is `meta.*`.
struct Rule {
    scope: Scope,
    atoms: u32,
    meta: bool,
    class: TokenClass,
}

struct Tables {
    set: SyntaxSet,
    rules: Vec<Rule>,
    comment: Scope,
    string: Scope,
}

static TABLES: LazyLock<Tables> = LazyLock::new(|| {
    let scope = |s: &str| Scope::new(s).expect("rule scopes are valid literals");
    Tables {
        set: two_face::syntax::extra_newlines(),
        rules: RULES
            .iter()
            .map(|&(prefix, class)| {
                let scope = scope(prefix);
                Rule {
                    scope,
                    atoms: scope.len(),
                    meta: prefix.starts_with("meta."),
                    class,
                }
            })
            .collect(),
        comment: scope("comment"),
        string: scope("string"),
    }
});

/// The supported syntax ids, in display order: `python`, `javascript`,
/// `typescript`, `java`, `csharp`, `cpp`, `c`, `go`, `rust`, `php`, `sql`,
/// `bash`, `html`, `css`, `kotlin`, `swift`.
pub fn syntaxes() -> &'static [&'static str] {
    &IDS
}

/// Highlights `code` with the syntax `syntax` (one of [`syntaxes`]).
///
/// Returns `None` for an unknown id. Otherwise the tokens cover the whole
/// input in order, without gaps or overlaps, newlines included, and adjacent
/// runs of the same class are merged. Never panics on malformed code.
pub fn highlight(syntax: &str, code: &str) -> Option<Vec<Token>> {
    let tables = &*TABLES;
    let reference = resolve(&tables.set, syntax, code)?;
    let mut state = ParseState::new(reference);
    let mut stack = ScopeStack::new();
    let mut tokens = Vec::new();
    let mut offset = 0;
    let mut push = |text: &str, class: TokenClass| {
        let end = offset + text.chars().count();
        match tokens.last_mut() {
            _ if end == offset => {}
            Some(Token {
                end: last,
                class: c,
                ..
            }) if *c == class => *last = end,
            _ => tokens.push(Token {
                start: offset,
                end,
                class,
            }),
        }
        offset = end;
    };
    let mut lines = LinesWithEndings::from(code);
    for line in lines.by_ref() {
        let Ok(ops) = state.parse_line(line, &tables.set) else {
            // A grammar error leaves the parser unusable: the rest is plain.
            push(line, TokenClass::Plain);
            break;
        };
        let mut prev = 0;
        for (pos, op) in ops {
            let pos = pos.min(line.len());
            if pos > prev && line.is_char_boundary(pos) {
                push(&line[prev..pos], classify(tables, stack.as_slice()));
                prev = pos;
            }
            // An unbalanced pop only loses colour; keep the coverage intact.
            let _ = stack.apply(&op);
        }
        push(&line[prev..], classify(tables, stack.as_slice()));
    }
    for line in lines {
        push(line, TokenClass::Plain);
    }
    Some(tokens)
}

/// Finds the grammar for `id`. PHP snippets are usually bare code, so they use
/// the embedded `PHP Source` grammar unless they open a `<?` tag.
fn resolve<'a>(set: &'a SyntaxSet, id: &str, code: &str) -> Option<&'a SyntaxReference> {
    let (_, name) = SYNTAXES.iter().find(|(known, _)| *known == id)?;
    let name = if id == "php" && code.contains("<?") {
        "PHP"
    } else {
        name
    };
    set.find_syntax_by_name(name)
}

/// Maps a scope stack to its class (see the crate docs).
fn classify(tables: &Tables, stack: &[Scope]) -> TokenClass {
    for scope in stack.iter().rev() {
        if tables.comment.is_prefix_of(*scope) {
            return TokenClass::Comment;
        }
        if tables.string.is_prefix_of(*scope) {
            return TokenClass::String;
        }
    }
    for (depth, scope) in stack.iter().rev().enumerate() {
        let best = tables
            .rules
            .iter()
            .filter(|rule| (depth == 0 || !rule.meta) && rule.scope.is_prefix_of(*scope))
            .max_by_key(|rule| rule.atoms);
        if let Some(rule) = best {
            return rule.class;
        }
    }
    TokenClass::Plain
}
