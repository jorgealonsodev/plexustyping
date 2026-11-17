# English corpus licenses

## `words.txt`

`words.txt` is licensed under the
[Creative Commons Attribution-ShareAlike 4.0 International License (CC BY-SA 4.0)](https://creativecommons.org/licenses/by-sa/4.0/).

It is adapted from `content/2018/en/en_50k.txt` of
[hermitdave/FrequencyWords](https://github.com/hermitdave/FrequencyWords) by
Hermit Dave (commit `525f9b560de45753a5ea01069454e72e9aa541c6`), which is
derived from the [OpenSubtitles 2018](http://opus.nlpl.eu/OpenSubtitles2018.php)
corpus. Subtitle data courtesy of [OpenSubtitles](https://www.opensubtitles.org/).

Modifications, made with `scripts/import-frequencywords.py`:

- removed the occurrence counts;
- filtered: kept only lowercase words made of English letters (dropping
  apostrophe and hyphen tokens); removed single and two-letter tokens that
  are not English words, contraction fragments, profanity, proper names
  (people, places, nationalities, days, months and holidays), abbreviations,
  interjections and dialect spellings;
- truncated to the 5,000 most frequent remaining words, in frequency order;
- added `#` header lines stating source, commit, license and modifications.

The adapted list is shared under the same license, CC BY-SA 4.0.

## Everything else

The rest of the `plexustyping-corpus` crate, including its code,
`diagnostic.txt` and `sentences.txt`, is original work licensed under
`MIT OR Apache-2.0`, like the rest of PlexusTyping.
