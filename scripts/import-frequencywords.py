#!/usr/bin/env python3
"""Import a word frequency list for plexustyping-corpus.

Usage:
    python3 -I scripts/import-frequencywords.py <language> <src_50k.txt> <commit sha> <output path>

<language> is one of es, en, pt, fr, de. Input is
`content/2018/<src>/<src>_50k.txt` from hermitdave/FrequencyWords
(https://github.com/hermitdave/FrequencyWords, CC BY-SA 4.0, derived from
OpenSubtitles 2018), where <src> is the language's SOURCE (pt uses pt_br).
Each line is a `word count` pair, most frequent first. Treat the download
as untrusted data: it is only parsed, never executed.

Filters, in frequency order, with the parameters of the chosen language:
  * lowercase words made only of the language's letters (no apostrophes,
    hyphens, digits or dots);
  * single letters only when listed in `single_letters`;
  * two-letter words only when listed in `two_letters` (every language but
    es, whose list predates this filter);
  * words with one of the `foreign` letters only when listed in
    `foreign_allowlist` (es: k, w; pt: k, w, y; fr: k, w);
  * no entry of the language's PROFANITY, PROPER_NAMES or NOISE (foreign
    words, interjections, abbreviations, transcription noise and
    misspellings).
The first LIMIT survivors are written one per line after `#` header lines.
Standard library only; prints filter counts to stderr.
"""

import re
import sys

LIMIT = 5000
COMMON_NAMES = {
    # First and family names frequent in subtitles of every language.
    "aaron", "abby", "adam", "adrian", "albert", "alan", "alex", "alexander",
    "alfred", "ali", "alice", "alicia", "alison", "allen", "allison", "amanda",
    "amy", "andrea", "andrew", "andy", "angela", "ann", "anna", "anne", "annie",
    "anthony", "antonio", "arthur", "ashley", "audrey", "bailey", "barbara",
    "barney", "barry", "bart", "batman", "becky", "ben", "benjamin", "benny",
    "bernard", "beth", "betty", "billy", "blake", "bob", "bobby", "brad",
    "brandon", "brenda", "brian", "brooke", "bruce", "bruno", "caleb",
    "cameron", "carl", "carla", "carlos", "carmen", "carol", "caroline",
    "carrie", "carter", "casey", "catherine", "charles", "charlie", "charlotte",
    "chloe", "chris", "christina", "christine", "christopher", "cindy",
    "claire", "clara", "clark", "claudia", "colin", "collins", "connor",
    "cooper", "craig", "curtis", "daisy", "dan", "dana", "daniel", "danny",
    "dave", "david", "davis", "debbie", "dennis", "derek", "diana", "diane",
    "diego", "donald", "donna", "dorothy", "doug", "duncan", "dylan", "eddie",
    "edward", "elena", "eli", "elizabeth", "ellen", "ellie", "elliot", "emily",
    "emma", "eric", "ethan", "eva", "evan", "felix", "francis", "francisco",
    "frank", "frankie", "franklin", "fred", "freddie", "freddy", "gabriel",
    "gary", "george", "gibbs", "gina", "gloria", "gordon", "graham", "greg",
    "gus", "gwen", "hank", "hannah", "harold", "harper", "harris", "harrison",
    "harry", "harvey", "heather", "hector", "helen", "henry", "hitler",
    "holmes", "howard", "ian", "isabel", "ivan", "jack", "jackie", "jackson",
    "jacob", "jake", "james", "jamie", "jan", "jane", "janet", "jason", "jay",
    "jean", "jeff", "jen", "jenna", "jennifer", "jenny", "jeremy", "jerry",
    "jess", "jesse", "jessica", "jessie", "jill", "jim", "jimmy", "jin",
    "joan", "joe", "joel", "joey", "john", "johnny", "johnson", "jon",
    "jonathan", "jones", "jordan", "joseph", "josh", "juan", "judy", "jules",
    "julia", "julian", "julie", "jung", "justin", "karen", "karl", "kate",
    "katherine", "katie", "keith", "kelly", "kennedy", "kenny", "kevin",
    "kim", "kirk", "kurt", "kyle", "lana", "larry", "laura", "lauren",
    "lawrence", "leo", "leon", "leonard", "leslie", "lewis", "lex", "liam",
    "lily", "lincoln", "linda", "lisa", "liz", "lloyd", "logan", "lois",
    "lou", "louis", "louise", "lucas", "lucy", "luke", "malcolm", "marco",
    "marcus", "margaret", "maria", "marie", "mario", "marshall", "martha",
    "martin", "marty", "mary", "mason", "matt", "matthew", "max", "maya",
    "megan", "melissa", "mia", "michael", "michelle", "mickey", "miguel",
    "mike", "mikey", "miller", "mitch", "mitchell", "molly", "monica",
    "morgan", "murphy", "nancy", "nate", "nathan", "ned", "neil", "nelson",
    "nick", "nicky", "nicole", "nikki", "nina", "noah", "nora", "norman",
    "oliver", "olivia", "oscar", "owen", "paige", "pam", "parker", "patrick",
    "paul", "paula", "pedro", "penny", "pete", "peter", "phil", "philip",
    "phoebe", "pierre", "quinn", "rachel", "ralph", "randy", "raymond",
    "rebecca", "richard", "rick", "ricky", "riley", "rita", "robert", "robin",
    "roger", "ronnie", "rory", "rose", "ross", "roy", "russell", "ruth",
    "ryan", "sally", "sam", "samantha", "sammy", "sandra", "sandy", "sara",
    "sarah", "scott", "sean", "sebastian", "seth", "shane", "shawn", "simon",
    "simpson", "smith", "sophie", "spencer", "stan", "stanley", "stella",
    "stephen", "steve", "steven", "stuart", "superman", "susan", "tara",
    "taylor", "ted", "teddy", "terry", "thomas", "tim", "tina", "toby",
    "todd", "tom", "tommy", "tony", "tracy", "travis", "trevor", "troy",
    "turner", "tyler", "vanessa", "victor", "victoria", "vince", "vincent",
    "walker", "walt", "walter", "watson", "wayne", "wendy", "william",
    "williams", "willie", "wilson", "zach", "zack", "zoe",
    "adams", "alexis", "ana", "anderson", "angie", "beverly", "boris",
    "boyd", "burt", "campbell", "carson", "cassie", "cathy", "chen",
    "christian", "clarence", "connie", "conrad", "damon", "daphne",
    "dawson", "dee", "denise", "douglas", "doyle", "erica", "erik", "erin",
    "fletcher", "florence", "garcia", "glenn", "haley", "hamilton", "han",
    "hans", "hugh", "hugo", "humphrey", "irene", "isaac", "jacques",
    "jefferson", "jeffrey", "juliet", "kane", "kara", "lena", "lenny",
    "lindsay", "lola", "lydia", "madison", "maggie", "manny", "marc",
    "marion", "meg", "meredith", "mick", "miranda", "mona", "morris",
    "murray", "natalie", "neal", "nicholas", "pablo", "palmer", "patty",
    "peggy", "perry", "philippe", "richie", "robbie", "ron", "rosie",
    "rudy", "sasha", "serena", "sharon", "sheila", "shirley", "sid",
    "sonia", "stephanie", "steele", "stevie", "stewart", "tess",
    "teresa", "thompson", "veronica", "vic", "vicky", "willy", "wyatt",
    "yang", "agnes", "arnold", "bates", "cal", "chad", "clyde", "crane",
    "denny", "drake", "elaine", "fanny", "finch", "fiona", "georgie",
    "helena", "julien", "kathy", "lester", "mcgee", "otto", "piper",
    "romeo", "samuel", "stefan", "tucker", "gil", "shaw", "luis",
    "sylvia", "randall", "joão",
}
COMMON_PLACES = {
    "afghanistan", "angeles", "cleveland", "dallas", "orleans", "phoenix",
    "philadelphia", "houston", "boston", "brooklyn", "california", "chicago",
    "florida",
    "hollywood", "hong", "jersey", "kong", "las", "los", "manhattan",
    "miami", "san", "seattle", "sydney", "texas", "vegas", "vietnam",
    "virginia", "washington", "york",
}
FOREIGN_ENGLISH = {
    # English words frequent in foreign subtitles; each language subtracts the
    # ones that are also its own words.
    "all", "and", "are", "baby", "big", "black", "blue", "boy", "but", "can",
    "city", "come", "country", "doctor", "down", "for", "get", "girl", "got",
    "green", "hello", "hey", "house", "island", "just", "know", "lady",
    "let", "like", "little", "long", "love", "man", "may", "miss", "new",
    "not", "now", "one", "out", "over", "red", "sexy", "sir", "street",
    "that", "the", "this", "what", "when", "white", "will", "with", "yeah",
    "you", "young", "your",
}
SHARED_NOISE = {
    # Interjections and transcription noise shared by every list.
    "aah", "ahh", "chffffff", "hmm", "huh", "mmm", "ohh", "ooh", "shh",
    "uhh", "whoa", "wow",
}


class Language:
    def __init__(self, name, source, letters, single_letters, two_letters,
                 foreign, foreign_allowlist, profanity, proper_names, noise):
        self.name = name
        self.source = source
        self.word = re.compile(f"[{letters}]+")
        self.single_letters = single_letters
        self.two_letters = two_letters
        self.foreign = foreign
        self.foreign_allowlist = foreign_allowlist
        self.blocklist = profanity | proper_names | noise

    def classify(self, word):
        """Return None when the word is kept, otherwise the drop reason."""
        if not self.word.fullmatch(word):
            return "charset"
        if len(word) == 1 and word not in self.single_letters:
            return "single letter"
        if (self.two_letters is not None and len(word) == 2
                and word not in self.two_letters):
            return "two letters"
        if (any(c in word for c in self.foreign)
                and word not in self.foreign_allowlist):
            return "/".join(self.foreign)
        if word in self.blocklist:
            return "blocklist"
        return None


# ---------------------------------------------------------------- Spanish

ES_PROPER_NAMES = {
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

SPANISH = Language(
    name="Spanish",
    source="es",
    letters="a-zñáéíóúü",
    single_letters={"a", "e", "o", "u", "y"},
    two_letters=None,
    foreign="kw",
    foreign_allowlist={
        "karate", "kilo", "kilos", "kilogramo", "kilogramos", "kilómetro",
        "kilómetros", "kiosco", "sándwich", "sándwiches", "vodka", "whisky",
    },
    profanity={
        "cabrón", "cabrona", "cabrones", "capullo", "carajo", "chingada",
        "chingar", "cojones", "coño", "culo", "follar", "gilipollas", "hijoputa",
        "jodan", "jodas", "joder", "jodida", "jodidamente", "jodido", "jódete",
        "mamón", "marica", "maricón", "mierda", "mierdas", "pendeja", "pendejo",
        "pendejos", "perra", "polla", "porno", "prostituta", "puta", "putas",
        "puto", "putos", "puñeta", "tetas", "verga", "zorra", "bastardo",
        "bastardos",
    },
    proper_names=ES_PROPER_NAMES,
    noise={
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
    },
)

# ---------------------------------------------------------------- English

ENGLISH = Language(
    name="English",
    source="en",
    letters="a-z",
    single_letters={"a", "i"},
    two_letters={
        "am", "an", "as", "at", "be", "by", "do", "ex", "go", "ha", "he",
        "hi", "if", "in", "is", "it", "me", "my", "no", "of", "oh", "on",
        "or", "ox", "so", "to", "up", "us", "we",
    },
    foreign="",
    foreign_allowlist=set(),
    profanity={
        "arse", "ass", "asses", "asshole", "assholes", "bastard", "bastards",
        "bitch", "bitches", "bloody", "boobs", "bullshit", "butt",
        "cock", "crap", "cunt", "damn", "damned", "dammit", "dick", "fuck",
        "fucked", "fucker", "fuckin", "fucking", "goddamn",
        "motherfucker", "nigga", "nigger", "penis", "piss", "pissed", "porn",
        "prick", "prostitute", "pussy", "rape", "raped", "screwed",
        "screwing", "sexy", "shit", "shitty", "slut", "tits", "whore",
        "sperm", "hooker", "fart",
    },
    proper_names=COMMON_NAMES | COMMON_PLACES | {
        # People, characters and titles.
        "abraham", "allah", "amber", "anderson", "andre", "angela", "arnold",
        "austin", "benny", "blair", "bonnie", "bud", "caesar", "carson",
        "chan", "chang", "charlie", "chuck", "claus", "clay", "cole",
        "dale", "dean", "dorothy", "drake", "earl", "eve", "finn", "ford",
        "gene", "glenn", "grace", "grant", "han", "hercules", "holly",
        "homer", "hunter", "jesus", "christ", "jew", "jewish", "jews", "kang",
        "kay", "kent", "khan", "lance", "lane", "lee", "lin", "lola",
        "lydia", "mac", "marie", "mark", "mars", "maurice", "mel", "mick",
        "miranda", "monk", "morris", "naomi", "nazi", "noel", "patty",
        "peggy", "perry", "pierce", "ralph", "ray", "rex", "richie",
        "rob", "robin", "rocky", "rodney", "roger", "romeo", "rosa",
        "rosie", "ruby", "rusty", "santa", "satan", "sharon", "sid",
        "singh", "sonny", "sue", "tae", "turner", "val", "vera", "veronica",
        "wade", "wallace", "walker", "wang", "warren", "winston", "woody",
        "yang", "chi", "denise", "bernard", "kenny", "erica", "kirk",
        "ken", "kelly", "kitty", "zombie", "cooper", "hall", "lou",
        # Places, nations, languages, days, months and holidays, which
        # English capitalises.
        "africa", "african", "america", "american", "americans", "berlin",
        "brazil", "britain", "british", "canada", "catholic", "china",
        "chinese", "christian", "christmas", "dutch", "egypt", "england",
        "english", "europe", "european", "france", "french", "friday",
        "georgia", "german", "germans", "germany", "greek", "halloween",
        "hawaii", "india", "indian", "indians", "iraq", "irish", "israel",
        "italian", "italy", "january", "japan", "japanese", "july", "june",
        "korea", "korean", "latin", "london", "mexican", "mexico", "monday",
        "moscow", "muslim", "november", "october", "pacific", "paris",
        "polish", "rome", "russia", "russian", "russians", "saturday",
        "scotland", "september", "soviet", "spain", "spanish", "sunday",
        "thanksgiving", "thursday", "tokyo", "tuesday", "wednesday",
        "december", "australia", "april", "february", "asia", "asian",
        "atlantic", "buddha", "elvis", "facebook", "shakespeare",
        "excellency", "hon", "finn", "houston", "mona",
    },
    noise=SHARED_NOISE | {
        # Fragments of contractions and dialect spellings.
        "ain", "aren", "couldn", "didn", "doesn", "don", "dont", "hadn",
        "hasn", "haven", "isn", "mustn", "shouldn", "wasn", "weren",
        "wouldn", "comin", "doin", "gettin", "goin", "lookin", "nothin",
        "somethin", "talkin", "tryin", "gonna", "gotta", "wanna", "kinda",
        "outta", "gimme", "dunno", "cos", "erm", "iike", "aii", "argh",
        "hee", "heh", "ugh", "whoo", "yep", "yup", "nah", "jeez", "geez",
        "gee", "bleep", "hiya", "wha", "mrs", "fbi", "cia", "dna", "del",
        "von", "herr", "los", "san", "pos", "cam", "min", "sec", "bro",
        "monsieur", "madame", "mademoiselle", "señor", "uhh", "ahem", "aha",
        "yay", "woo", "darn", "dong", "kung", "caitlyn", "aww", "unh", "raj",
        "que", "choi", "bam", "bon", "elderman",
    },
)

# ---------------------------------------------------------------- Portuguese

PORTUGUESE = Language(
    name="Brazilian Portuguese",
    source="pt_br",
    letters="a-záâãàéêíóôõúüç",
    single_letters={"a", "e", "o", "é", "à"},
    two_letters={
        "ah", "ai", "aí", "ao", "ar", "as", "ás", "às", "cá", "da", "dá",
        "de", "dê", "do", "ei", "em", "és", "eu", "fã", "fé", "há", "ia",
        "ir", "já", "lá", "lê", "li", "má", "me", "na", "no", "nu", "oh",
        "oi", "os", "ou", "pé", "pó", "se", "si", "só", "te", "ti", "tu",
        "um", "vá", "vê", "vi",
    },
    foreign="kwy",
    foreign_allowlist={"gay", "gays", "rock", "show", "shows"},
    profanity={
        "babaca", "bastardo", "bosta", "bunda", "cadela", "caralho",
        "cretino", "desgraçado", "foda", "foder", "gostosa", "gostoso",
        "merda", "peitos", "pênis", "porra", "pornô", "prostituta", "puta",
        "puto", "seios", "transar", "transando", "vadia", "estupro",
        "canalha", "sexy", "cacete", "merdas", "vagina", "vagabunda",
        "prostitutas", "otário",
    },
    proper_names=COMMON_NAMES | COMMON_PLACES | {
        "alemanha", "américa", "angel", "brasil", "berlim",
        "califórnia", "canadá", "china", "cristo", "césar", "espanha",
        "eua", "europa", "flórida", "frança", "hércules", "índia",
        "inglaterra", "iorque", "iraque", "israel", "itália", "japão",
        "jesus", "josé", "londres", "marte", "méxico", "moscou", "paris",
        "rússia", "roma", "áfrica", "sabrina", "noel", "lee", "grant",
        "hill", "earl", "dale", "dean", "gene", "drew", "clay", "joy",
        "carol", "rosie", "homer", "hunter", "holly", "amber", "lane",
        "piper", "eve", "april", "dane", "houston", "bill", "don", "grace",
        "stone", "bonnie", "ford", "crane", "chase", "pat", "booth", "reed",
        "bud", "sue", "rob", "egito", "austrália", "vietnã",
    },
    noise=(FOREIGN_ENGLISH - {"come", "for"}) | SHARED_NOISE | {
        "case", "doc", "hall", "junior", "king", "mac", "madame",
        "monsieur", "park", "star", "top", "von", "west", "united", "queens",
        "online", "okay", "bang", "blá", "there", "gonna", "pop", "use",
        # Abbreviations and transcription noise.
        "cia", "dna", "fbi", "ihe", "srta", "sra", "vc", "nâo", "nao",
        "voce", "policia", "apos", "prá", "alguem", "lord",
        # Pre-2009 spellings of listed words.
        "idéia", "idéias", "vôo", "jóias", "dêem", "vêem", "heróico",
    },
)

# ---------------------------------------------------------------- French

FRENCH = Language(
    name="French",
    source="fr",
    letters="a-zàâæçéèêëîïôœùûüÿ",
    single_letters={"a", "à", "y"},
    two_letters={
        "ah", "ai", "an", "as", "au", "bu", "ça", "çà", "ce", "de", "dû",
        "du", "eh", "en", "es", "et", "eu", "hé", "il", "je", "la", "là",
        "le", "lu", "ma", "me", "né", "ne", "ni", "nu", "oh", "on", "or",
        "os", "ou", "où", "pu", "sa", "se", "si", "su", "ta", "te", "tu",
        "un", "va", "vu",
    },
    foreign="kw",
    foreign_allowlist={
        "basket", "clown", "cocktail", "interview", "kilomètres", "kilos",
        "parking", "poker", "rock", "sandwich", "show", "steak", "ticket",
        "tickets", "vodka", "whisky",
    },
    profanity={
        "baise", "baiser", "baisé", "bite", "bordel", "bâtard", "chatte",
        "chier", "con", "connard", "connards", "connerie", "conneries",
        "cons", "couilles", "cul", "dégueulasse", "emmerde", "enculé",
        "enfoiré", "enfoirés", "foiré", "fout", "foutez", "foutre", "foutu",
        "foutue", "foutus", "garce", "gueule", "homo", "merde", "merdé",
        "pédé", "pisse", "pisser", "porno", "prostituée", "pute", "putes",
        "putain", "putains", "salaud", "salauds", "salope", "salopard",
        "saloperie", "seins", "sexy", "sperme", "pénis", "viol", "violé",
        "violée", "nichons", "violer", "déconne",
    },
    proper_names=COMMON_NAMES | COMMON_PLACES | {
        "afrique", "allemagne", "amérique", "angleterre", "antoine",
        "berlin", "californie", "canada", "chine", "chloé", "christ",
        "corée", "césar", "damon", "espagne", "europe", "floride",
        "france", "harvard", "inde", "irak", "italie", "japon", "jésus",
        "juliette", "léo", "lindsay", "londres", "marc", "mcgee", "mexique",
        "michel", "moscou", "noël", "palmer", "paris", "piper", "rome",
        "russie", "satan", "tokyo", "usa", "virginie", "madison", "blair",
        "naomi", "neal", "archer", "lee", "grant", "hill", "stone", "dean",
        "earl", "chase", "gene", "lane", "lance", "ford", "drew", "clay",
        "booth", "holly", "hall", "mac", "sue", "ron", "rob", "ray",
        "amber", "eve", "april", "bill", "don", "grace", "homer", "santa",
        "bonnie", "dale", "pat", "reed", "joy", "guy", "mel", "philadelphie",
        "xena",
    } - {"rose", "lance"},
    noise=(FOREIGN_ENGLISH - {"but", "long"}) | SHARED_NOISE | {
        "beach", "business", "bye", "cash", "deal", "enterprise", "french",
        "gentleman", "hall", "job", "lord", "mrs", "star", "team", "time",
        "times", "top", "castle", "road", "happy", "fox", "bay", "bond",
        "hope", "came", "fun", "min", "bang", "doc", "cat", "grey", "dvd",
        "melle", "aprés", "ais",
        # Abbreviations, interjections and transcription noise.
        "adn", "aah", "bah", "bip", "cia", "euh", "fbi", "gps", "heu",
        "hum", "ies", "lls", "mlle", "mme", "nan", "ouah", "ouai", "pos",
        "sms", "svp", "von", "los", "las", "san",
        # Accentless or misspelled forms of listed words.
        "apres", "coté", "ecoute", "ecoutez", "etait", "etat", "etes",
        "etre", "evidemment", "meme", "sure", "surement", "tres", "trés",
        "voila", "coeur", "soeur", "soeurs", "oeil", "oeuvre", "oeufs",
    } - {"come"},
)

# ---------------------------------------------------------------- German

GERMAN = Language(
    name="German",
    source="de",
    letters="a-zäöüß",
    single_letters=set(),
    two_letters={
        "ab", "ah", "am", "an", "da", "du", "ei", "er", "es", "im", "in",
        "ja", "je", "na", "ob", "oh", "öl", "so", "tu", "um", "wo", "zu",
    },
    foreign="",
    foreign_allowlist=set(),
    profanity={
        "arsch", "arschloch", "arschlöcher", "bastard", "beschissen",
        "brüste", "fick", "ficken", "gefickt", "geil", "hure", "hurensohn",
        "miststück", "nigger", "nutte", "penis", "penner", "scheiß",
        "scheiße", "scheißegal", "scheißkerl", "scheisse", "schlampe",
        "titten", "verarschen", "verarscht", "verpiss", "vergewaltigt",
        "wichser", "fresse", "sexy", "dreckskerl", "mistkerl", "shit",
        "fuck", "schwuchtel", "vergewaltigung",
    },
    proper_names=COMMON_NAMES | COMMON_PLACES | {
        "afrika", "alf", "amerika", "anton", "ari", "berlin", "buffy",
        "castle", "chandler", "christus", "clarence", "county", "dexter",
        "deutschland", "douglas", "england", "esther", "europa",
        "frankreich", "franz", "goa", "hanna", "hans", "hastings", "herman",
        "hogan", "houston", "indien", "italien", "japan", "jeannie",
        "jeffrey", "jesus", "jonas", "kalifornien", "kanada", "klaus",
        "london", "lorelai", "macgyver", "marcel", "mexiko", "michel",
        "monroe", "moskau", "mulder", "neill", "orleans", "otto", "paris",
        "peg", "poirot", "reese", "rom", "russland", "santa", "schultz",
        "scully", "serena", "sheldon", "sherlock", "shield", "spanien",
        "spock", "stefan", "stevie", "teal", "tess", "tucker", "walton",
        "china", "island", "amerika", "piper", "blair", "lee", "hill",
        "stone", "grant", "lane", "earl", "dean", "drew", "clay", "chase",
        "ken", "kent", "kirk", "lex", "manny", "monk", "ike", "cal", "mac",
        "ron", "ray", "sue", "bud", "eve", "dale", "gene", "lance",
        "ford", "booth", "hunter", "holly", "carol",
        "francisco", "hollywood", "vegas", "angeles", "brooklyn", "jersey",
        "virginia", "sydney", "boston", "chicago", "miami", "florida",
        "don", "bill", "chuck", "dick", "grace", "bonnie", "brown", "warren",
        "sonny", "homer", "archer", "grey", "duke", "baker", "ruby",
        "rocky", "buck", "pierce", "wade", "pat", "halloween",
        "thanksgiving", "bell", "gossip", "schweden", "schweiz", "central",
        "dads", "deputy", "stars",
        "texas", "york", "washington", "vietnam", "erin", "mona",
    },
    noise=(FOREIGN_ENGLISH - {"man", "not", "will"}) | SHARED_NOISE | {
        "action", "air", "baby", "babe", "beach", "bye", "captain", "colonel",
        "commander", "cop", "cops", "crew", "dad", "daddy", "data", "date",
        "day", "deal", "detective", "dinner", "doctor", "drink", "drinks",
        "east", "enterprise", "football", "gentleman", "gentlemen", "girl",
        "grandma", "grandpa", "group", "guy", "happy", "high", "highschool",
        "hope", "inspector", "kid", "kids", "ladies", "lady", "lieutenant",
        "live", "lord", "madam", "master", "media", "mister", "mom",
        "mommy", "officer", "old", "online", "president", "road",
        "school", "sergeant", "service", "set", "sheriff", "single",
        "song", "sorry", "special", "star", "story", "time", "world",
        "west", "german", "miles", "hall", "queen", "king", "darling",
        "loser", "aliens", "lemon", "lake", "there", "ally", "hood",
        "gates", "cat", "bond", "times", "south", "river", "chief", "grand",
        "fox", "bay",
        # Abbreviations, interjections and transcription noise.
        "aye", "bla", "cia", "dna", "fbi", "hmm", "huh", "lch", "lhnen",
        "lhr", "lhre", "lhrem", "lhren", "lhrer", "mhm", "mrs", "nem",
        "nen", "ner", "psst", "pst", "sdh", "sms", "usa",
        "ähm", "ächzt", "madame", "mademoiselle", "monsieur", "miss", "sir",
        "mylord", "los", "san", "las", "lkw", "ais", "sdi", "habs",
        "tschuldigung",
        # Old or Swiss spellings of listed words.
        "daß", "heisst", "laß", "muß", "mußt", "weiss", "weisst",
    },
)

LANGUAGES = {
    "es": SPANISH,
    "en": ENGLISH,
    "pt": PORTUGUESE,
    "fr": FRENCH,
    "de": GERMAN,
}


def main(argv):
    if len(argv) != 5 or argv[1] not in LANGUAGES:
        sys.exit(__doc__)
    language = LANGUAGES[argv[1]]
    source, commit, output = argv[2:]
    kept, drops, seen = [], {}, 0
    with open(source, encoding="utf-8") as lines:
        for line in lines:
            parts = line.split()
            if not parts:
                continue
            seen += 1
            reason = language.classify(parts[0])
            if reason:
                drops[reason] = drops.get(reason, 0) + 1
            else:
                kept.append(parts[0])
            if len(kept) == LIMIT:
                break
    if len(kept) < LIMIT:
        sys.exit(f"only {len(kept)} words survived the filters")
    src = language.source
    header = [
        f"# {language.name} words by frequency, most frequent first.",
        f"# Source: hermitdave/FrequencyWords content/2018/{src}/{src}_50k.txt"
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
