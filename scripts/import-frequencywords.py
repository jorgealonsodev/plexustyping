#!/usr/bin/env python3
"""Import the Spanish word frequency list for plexustyping-corpus.

Usage:
    python3 -I scripts/import-frequencywords.py <es_50k.txt> <commit sha> <output path>

Input is `content/2018/es/es_50k.txt` from hermitdave/FrequencyWords
(https://github.com/hermitdave/FrequencyWords, CC BY-SA 4.0, derived from
OpenSubtitles 2018), one `word count` pair per line, most frequent first.
Treat the download as untrusted data: it is only parsed, never executed.

Filters, in frequency order:
  * lowercase words made only of Spanish letters a-z, ñ, á, é, í, ó, ú, ü;
  * single letters only when they are Spanish words (a, e, o, u, y);
  * words with `k` or `w` only when listed in K_W_ALLOWLIST;
  * no entry of PROFANITY, PROPER_NAMES or NOISE (English words,
    interjections, abbreviations and accentless misspellings).
The first LIMIT survivors are written one per line after `#` header lines.
Standard library only; prints filter counts to stderr.
"""

import re
import sys

LIMIT = 5000
SPANISH_WORD = re.compile(r"[a-zñáéíóúü]+")
SINGLE_LETTER_WORDS = {"a", "e", "o", "u", "y"}

K_W_ALLOWLIST = {
    "karate", "kilo", "kilos", "kilogramo", "kilogramos", "kilómetro",
    "kilómetros", "kiosco", "sándwich", "sándwiches", "vodka", "whisky",
}

PROFANITY = {
    "cabrón", "cabrona", "cabrones", "capullo", "carajo", "chingada",
    "chingar", "cojones", "coño", "culo", "follar", "gilipollas", "hijoputa",
    "jodan", "jodas", "joder", "jodida", "jodidamente", "jodido", "jódete",
    "mamón", "marica", "maricón", "mierda", "mierdas", "pendeja", "pendejo",
    "pendejos", "perra", "polla", "porno", "prostituta", "puta", "putas",
    "puto", "putos", "puñeta", "tetas", "verga", "zorra", "bastardo",
    "bastardos",
}

PROPER_NAMES = {
    # People (mostly English names from subtitles).
    "aaron", "abby", "adam", "adrian", "albert", "alan", "alex", "ali",
    "alice", "alicia", "allen", "allison", "alison", "amanda", "amber",
    "amy", "ana", "anderson", "andrea", "andy", "angela", "ann", "anna",
    "anne", "annie", "anthony", "antonio", "april", "arthur", "ashley",
    "audrey", "bailey", "barbara", "barney", "barry", "bart", "batman",
    "ben", "benny", "beth", "betty", "bill", "billy", "bob", "bobby",
    "bonnie", "booth", "brad", "brandon", "brenda", "brian", "bruce",
    "bruno", "cal", "cameron", "carl", "carla", "carlos", "carmen", "carol",
    "caroline", "carrie", "carter", "casey", "catherine", "césar", "chan",
    "charles", "charlie", "charlotte", "chase", "chloe", "chris",
    "christian", "christine", "christopher", "cindy", "claire", "claudia",
    "clay", "colin", "cole", "collins", "connie", "connor", "cooper",
    "craig", "crane", "cristo", "curtis", "daisy", "dana", "daniel", "danny", "dave",
    "david", "davis", "dean", "debbie", "denise", "dennis", "diane",
    "diego", "donald", "donna", "doug", "duncan", "dylan", "earl", "eddie",
    "elena", "eli", "elizabeth", "ellen", "ellie", "elliot", "emily",
    "emma", "eric", "ethan", "eva", "evan", "eve", "ford", "francis",
    "francisco", "fred", "freddie", "freddy", "gabriel", "gary", "gene",
    "george", "gibbs", "gina", "gordon", "grace", "graham", "greg", "gus",
    "hall", "hannah",
    "harold", "harper", "harris", "harrison", "harry", "harvey", "heather", "helen",
    "henry", "hill", "hitler", "holly", "holmes", "hunter", "ian", "isabel",
    "jacob", "james", "jamie", "jan", "jane", "janet", "jason", "jay",
    "jean", "jeff", "jen", "jenna", "jennifer", "jenny", "jeremy", "jerry",
    "jess", "jesse", "jessica", "jesucristo", "jesús", "jill", "jim",
    "jimmy", "jin", "joan", "joe", "joey", "john", "johnny", "johnson",
    "jonathan", "jones", "jordan", "josé", "joseph", "josh", "joy", "juan",
    "judy", "jules", "julia", "julian", "julie", "jung", "justin", "lana",
    "lance", "lane", "larry", "laura", "lauren", "leonard", "leslie",
    "lex", "liam", "lily", "lincoln", "lisa", "liz", "logan", "lois",
    "lou", "louis", "louise", "lucas", "lucy", "luis", "mac", "madison",
    "maggie", "malcolm", "marcos", "marcus", "margaret", "maria", "maría",
    "marie", "mario", "marshall", "martha", "martin", "marty", "mary",
    "mason", "matt", "max", "maya", "meg", "megan", "mel", "melissa",
    "mia", "michael", "michelle", "miguel", "miller", "mitch", "mitchell",
    "molly", "morgan", "murphy", "nancy", "natalie", "nate", "nathan",
    "neal", "ned", "neil", "nelson", "nicole", "nina", "noel", "nora",
    "norman", "oliver", "olivia", "oscar", "pablo", "paige", "pam", "pat",
    "paul", "paula", "pedro", "penny", "pete", "peter", "phil", "philip",
    "phoebe", "pierre", "quinn", "rachel", "ralph", "randy", "ray",
    "raymond", "rebecca", "richard", "riley", "rita", "rob", "robert",
    "robin", "roger", "ron", "ronnie", "rory", "rose", "ross", "roy",
    "russell", "ruth", "ryan", "sally", "sam", "samantha", "sammy",
    "sandra", "sandy", "sara", "sarah", "scott", "seth", "shane", "sheila",
    "sid", "simon", "simpson", "smith", "sophie", "spencer", "stan",
    "stanley", "stella", "stephen", "steve", "steven", "stone", "stuart", "sue",
    "superman", "susan", "tara", "taylor", "ted", "teddy", "teresa",
    "terry", "thomas", "tim", "tina", "toby", "todd", "tom", "tommy",
    "tony", "tracy", "travis", "trevor", "troy", "turner", "tyler",
    "vanessa", "vic", "victor", "vince", "vincent", "zach", "zoe",
    # Places.
    "alemania", "américa", "australia", "berlín", "boston", "brasil",
    "california", "canadá", "chicago", "china", "city", "cleveland",
    "corea", "españa", "europa", "florida", "francia", "hong", "india",
    "inglaterra", "island", "israel", "italia", "japón", "jersey",
    "londres", "manhattan", "méxico", "miami", "moscú", "paris", "parís",
    "roma", "rusia", "seattle", "sydney", "texas", "vegas", "vietnam",
    "áfrica",
}

NOISE = {
    # English words and fragments.
    "all", "am", "and", "as", "baby", "be", "big", "blue", "boy", "case",
    "chance", "em", "for", "go", "green", "house", "in", "is", "it",
    "junior", "little", "long", "love", "man", "may", "my", "of", "on", "pop", "sexy", "sheriff",
    "so", "star", "street", "test", "that", "the", "times", "to", "top",
    "use", "yeah", "you", "young", "your",
    # Foreign titles, abbreviations and transcription noise.
    "adn", "bo", "cia", "des", "doc", "ed", "ee", "ei", "fbi", "ho", "ia",
    "ias", "id", "ii", "io", "ios", "lady", "ll", "lord", "ma", "madame",
    "min", "miss", "monsieur", "mr", "na", "pa", "sir", "soo", "sr", "sra",
    "st", "tv", "ud", "uu", "von", "éi",
    # Interjections that are not dictionary words.
    "bang", "bla", "ey", "hey", "hmm", "huh", "ja", "jo", "mm", "mmm",
    "ohh", "ooh", "shh", "uh", "um",
    # Accentless or outdated misspellings of listed words.
    "aca", "adios", "ahi", "algun", "alli", "angel", "angeles", "aqui",
    "asi", "deberia", "dejame", "despues", "dia", "dias", "dificil", "dió",
    "dí", "estabamos", "estan", "fué", "habia", "hare", "increible", "mio",
    "numero", "oido", "oir", "pense", "podria", "policia", "queria",
    "quizas", "rapido", "razon", "sera", "tambien", "tenia", "tio",
    "todavia", "tí", "ultima", "vamonos", "ví", "vió", "ésto",
}

BLOCKLIST = PROFANITY | PROPER_NAMES | NOISE


def classify(word):
    """Return None when the word is kept, otherwise the drop reason."""
    if not SPANISH_WORD.fullmatch(word):
        return "charset"
    if len(word) == 1 and word not in SINGLE_LETTER_WORDS:
        return "single letter"
    if ("k" in word or "w" in word) and word not in K_W_ALLOWLIST:
        return "k/w"
    if word in BLOCKLIST:
        return "blocklist"
    return None


def main(argv):
    if len(argv) != 4:
        sys.exit(__doc__)
    source, commit, output = argv[1:]
    kept, drops, seen = [], {}, 0
    with open(source, encoding="utf-8") as lines:
        for line in lines:
            parts = line.split()
            if not parts:
                continue
            seen += 1
            reason = classify(parts[0])
            if reason:
                drops[reason] = drops.get(reason, 0) + 1
            else:
                kept.append(parts[0])
            if len(kept) == LIMIT:
                break
    if len(kept) < LIMIT:
        sys.exit(f"only {len(kept)} words survived the filters")
    header = [
        "# Spanish words by frequency, most frequent first.",
        "# Source: hermitdave/FrequencyWords content/2018/es/es_50k.txt"
        " (OpenSubtitles 2018)",
        f"# Commit: {commit}",
        "# License: CC BY-SA 4.0 (https://creativecommons.org/licenses/by-sa/4.0/)",
        f"# Modified: filtered (charset, names, profanity, noise) and truncated"
        f" to the first {LIMIT} words by scripts/import-frequencywords.py",
    ]
    with open(output, "w", encoding="utf-8", newline="\n") as out:
        out.write("\n".join(header + kept) + "\n")
    print(f"read {seen} entries, kept {len(kept)}", file=sys.stderr)
    for reason in sorted(drops):
        print(f"dropped {drops[reason]} ({reason})", file=sys.stderr)


if __name__ == "__main__":
    main(sys.argv)
