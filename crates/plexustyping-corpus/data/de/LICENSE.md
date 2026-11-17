# German corpus licenses

## `words.txt`

`words.txt` is licensed under the
[Creative Commons Attribution-ShareAlike 4.0 International License (CC BY-SA 4.0)](https://creativecommons.org/licenses/by-sa/4.0/).

It is adapted from `content/2018/de/de_50k.txt` of
[hermitdave/FrequencyWords](https://github.com/hermitdave/FrequencyWords) by
Hermit Dave (commit `525f9b560de45753a5ea01069454e72e9aa541c6`), which is
derived from the [OpenSubtitles 2018](http://opus.nlpl.eu/OpenSubtitles2018.php)
corpus. Subtitle data courtesy of [OpenSubtitles](https://www.opensubtitles.org/).

Modifications, made with `scripts/import-frequencywords.py`:

- removed the occurrence counts;
- filtered: kept only lowercase words made of German letters (dropping
  apostrophe and hyphen tokens); removed single and two-letter tokens that
  are not German words, profanity, proper names, English words,
  abbreviations, interjections, transcription noise and old spellings;
- truncated to the 5,000 most frequent remaining words, in frequency order;
- added `#` header lines stating source, commit, license and modifications.

The adapted list is shared under the same license, CC BY-SA 4.0.

## `sentences-cv.txt`

`sentences-cv.txt` is dedicated to the public domain under
[CC0 1.0 Universal](https://creativecommons.org/publicdomain/zero/1.0/).

It is selected from `server/data/de/sentence-collector.txt` of
[Mozilla Common Voice](https://github.com/common-voice/common-voice)
(commit `605c25e48f9fb87cb25ca794cb39bf91d3741f29`) with `scripts/import-commonvoice.py`:

- kept only sentences of 40 to 120 chars made of the letters of the word
  list, spaces and `. , ; : ? !`, typeable on the native layout, starting with a
  capital letter, ending with `.`, `?` or `!`, without inner dots, word-list
  profanity or duplicates (of each other or of `sentences.txt`);
- took up to 2,000 survivors ordered by the SHA-256 of the sentence
  (2,000 sentences);
- added `#` header lines stating source, commit, license, filters and count.

## Everything else

The rest of the `plexustyping-corpus` crate, including its code,
`diagnostic.txt` and `sentences.txt`, is original work licensed under
`MIT OR Apache-2.0`, like the rest of PlexusTyping.
