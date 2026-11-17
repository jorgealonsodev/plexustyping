#!/usr/bin/env python3
"""Import Common Voice sentences for plexustyping-corpus.

Usage:
    python3 -I scripts/import-commonvoice.py <language> <sentence-collector.txt> <commit sha> <output path>

<language> is one of es, en, pt, fr, de. Input is
`server/data/<language>/sentence-collector.txt` of common-voice/common-voice
(https://github.com/common-voice/common-voice, CC0 1.0) at <commit sha>, one
sentence per line. Treat the download as untrusted data: it is only parsed,
never executed. The output is normally
`crates/plexustyping-corpus/data/<language>/sentences-cv.txt`; the original
`sentences.txt` next to it, when present, is read so that its sentences are
not repeated.

Each line is NFC-normalized and trimmed, then kept only when:
  * it has 40-120 chars;
  * every char is a letter of the language (the alphabet of
    scripts/import-frequencywords.py, either case), a space or one of
    `. , ; : ? !` (and `¿ ¡` for Spanish), so there are no digits, quotes,
    brackets, hyphens or apostrophes;
  * every char is typeable on the language's native layout (layouts/*.toml,
    direct keys plus dead-key compositions);
  * words are separated by single spaces, `. , ; : ? !` follow a letter and
    precede a space or the end, `¿ ¡` open a word, and in Spanish they pair
    up with `?` and `!`;
  * `.` appears only as the last char (no abbreviations or inner dots);
  * it starts with a capital letter (after `¿ ¡`) and ends with `.`, `?` or
    `!`;
  * no word is in the language's word-list PROFANITY;
  * it is not a duplicate of a kept or original sentence.
Survivors are ordered by the SHA-256 of their UTF-8 text and the first LIMIT
are written one per line after `#` header lines, so re-running the script on
the same input gives the same file. Standard library only; prints filter
counts to stderr.
"""

import hashlib
import importlib.util
import os
import sys
import tomllib
import unicodedata

LIMIT = 2000
MIN_CHARS = 40
MAX_CHARS = 120
SIGNS = ".,;:?!"
INVERTED = "¿¡"

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

# Native layout of each language.
LAYOUTS = {
    "es": "es-qwerty",
    "en": "us-qwerty",
    "pt": "pt-abnt2",
    "fr": "fr-azerty",
    "de": "de-qwertz",
}

# Combining mark of each dead key, as in crates/plexustyping-layouts.
DEAD_MARKS = {
    "´": "́",
    "`": "̀",
    "^": "̂",
    "¨": "̈",
    "~": "̃",
}


def load_word_languages():
    """The per-language alphabets and blocklists of the word-list importer."""
    path = os.path.join(ROOT, "scripts", "import-frequencywords.py")
    # -I ignores PYTHONDONTWRITEBYTECODE; keep scripts/ free of __pycache__.
    sys.dont_write_bytecode = True
    spec = importlib.util.spec_from_file_location("import_frequencywords", path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module.LANGUAGES


def layout_chars(layout_id):
    """Chars the layout can type, like `Layout::allowed_chars`."""
    path = os.path.join(ROOT, "layouts", f"{layout_id}.toml")
    with open(path, "rb") as file:
        layout = tomllib.load(file)
    dead = set(layout.get("dead_keys", []))
    direct = {
        value
        for layers in layout["keys"].values()
        for value in layers.values()
    }
    allowed = direct - dead
    for key in dead:
        mark = DEAD_MARKS.get(key)
        if mark is None:
            continue
        for base in direct:
            composed = unicodedata.normalize("NFC", base + mark)
            if len(composed) == 1:
                allowed.add(composed)
    return allowed | {" ", "\n"}


class Filter:
    def __init__(self, code, language):
        self.code = code
        self.language = language
        self.signs = SIGNS + (INVERTED if code == "es" else "")
        self.typeable = layout_chars(LAYOUTS[code])

    def is_letter(self, char):
        return char.isalpha() and bool(self.language.word.fullmatch(char.lower()))

    def classify(self, sentence):
        """Return None when the sentence is kept, otherwise the drop reason."""
        if not MIN_CHARS <= len(sentence) <= MAX_CHARS:
            return "length"
        if any(not (c == " " or c in self.signs or self.is_letter(c))
               for c in sentence):
            return "charset"
        if any(c not in self.typeable for c in sentence):
            return "untypeable"
        if "." in sentence[:-1]:
            return "inner dot"
        if not self.well_spaced(sentence):
            return "spacing"
        first = sentence.lstrip(INVERTED)[:1]
        if not (first.isalpha() and first.isupper()):
            return "capital"
        if sentence[-1] not in ".?!":
            return "end"
        if self.code == "es" and (sentence.count("¿") != sentence.count("?")
                                  or sentence.count("¡") != sentence.count("!")):
            return "unpaired ¿ ¡"
        words = "".join(c if c.isalpha() else " " for c in sentence.lower())
        if any(word in self.language.profanity for word in words.split()):
            return "profanity"
        return None

    def well_spaced(self, sentence):
        if "  " in sentence:
            return False
        last = len(sentence) - 1
        for i, char in enumerate(sentence):
            if char in SIGNS:
                if i == 0 or not self.is_letter(sentence[i - 1]):
                    return False
                if i < last and sentence[i + 1] != " ":
                    return False
            elif char in INVERTED:
                if i > 0 and sentence[i - 1] not in " " + INVERTED:
                    return False
                if i == last or not (self.is_letter(sentence[i + 1])
                                     or sentence[i + 1] in INVERTED):
                    return False
        return True


def read_sentences(path):
    with open(path, encoding="utf-8") as lines:
        for line in lines:
            yield unicodedata.normalize("NFC", line).strip()


def main(argv):
    languages = load_word_languages()
    if len(argv) != 5 or argv[1] not in LAYOUTS:
        sys.exit(__doc__)
    code = argv[1]
    source, commit, output = argv[2:]
    language = languages[code]
    sentence_filter = Filter(code, language)

    original_path = os.path.join(os.path.dirname(output), "sentences.txt")
    seen = set()
    if os.path.exists(original_path):
        seen.update(s for s in read_sentences(original_path)
                    if s and not s.startswith("#"))

    candidates, kept, drops = 0, [], {}
    for sentence in read_sentences(source):
        if not sentence:
            continue
        candidates += 1
        reason = sentence_filter.classify(sentence)
        if reason is None and sentence in seen:
            reason = "duplicate"
        if reason:
            drops[reason] = drops.get(reason, 0) + 1
            continue
        seen.add(sentence)
        kept.append(sentence)

    kept.sort(key=lambda s: hashlib.sha256(s.encode("utf-8")).hexdigest())
    selected = kept[:LIMIT]
    signs = " ".join(sentence_filter.signs)
    header = [
        f"# {language.name} sentences from Mozilla Common Voice.",
        "# Source: https://github.com/common-voice/common-voice/blob/"
        f"{commit}/server/data/{code}/sentence-collector.txt",
        f"# Commit: {commit}",
        "# License: CC0 1.0 Universal"
        " (https://creativecommons.org/publicdomain/zero/1.0/)",
        f"# Filters: NFC, trimmed, {MIN_CHARS}-{MAX_CHARS} chars;"
        f" only {language.name} letters, space and {signs};"
        f" typeable on {LAYOUTS[code]}; capital start; ends with . ? or !;"
        " no inner dots; no word-list profanity; no duplicates",
        f"# Selection: the first {LIMIT} survivors by SHA-256 of the sentence,"
        " by scripts/import-commonvoice.py",
        f"# Count: {len(selected)}",
    ]
    with open(output, "w", encoding="utf-8", newline="\n") as out:
        out.write("\n".join(header + selected) + "\n")
    print(f"{code}: read {candidates} candidates, {len(kept)} passed,"
          f" wrote {len(selected)}", file=sys.stderr)
    for reason in sorted(drops):
        print(f"  dropped {drops[reason]} ({reason})", file=sys.stderr)


if __name__ == "__main__":
    main(sys.argv)
