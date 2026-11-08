# Writing lines for AFM's line bank

AFM (American Football Manager) is an ultra-realistic American football management sim. You coach a high school, a college or an NFL team, and the people around you speak in short lines picked from a bank: players in your office, captains before a game, your staff, recruits' families, your boss, boosters, agents. You write new versions of one part of that bank. The owner hates stale, repeated text, so every line must be distinct, specific and true to real football in 2026.

## The format

- One version per line: the words, then optionally ` ~ ` (space, tilde, space) and its tags. For example: `I'll take that, Coach. Thank you. ~ humble`
- Plain spoken words in one or two sentences, ending with full punctuation. No quote marks, no dashes of any kind (use a comma or a period), no emoji, no swearing beyond a rare damn or hell, no stage directions.
- `[a|b]` or `[a|b|c]` picks one wording when the line is said. At most one line in ten has a choice, at most two choices a line, never nested, each wording complete on its own.
- `{fact}` is filled in by the game. Use only the facts your part lists, spelled exactly. A line with a fact the game doesn't give is never said.

## Tags

Tags decide who may say a line. A plain tag means the speaker must have it; `!tag` means he must not. At most three of them on a line. Use only the tags your part lists.

- Leave most lines untagged so anyone can say them, then use tags where the words need them. An untagged line must fit every speaker of the part and every level the part is said at.
- A line that names something only one level has must carry that level: high school (Friday nights, varsity, JV, offers, recruiters, official visits, 7-on-7, core GPA, a principal, the school board), college (scholarship, walk-on, redshirt, NIL, revenue share, the collective, the portal, campus, the NCAA, a bowl, Saturdays, the draft as a future), the NFL (contract, guarantees, the cap, the practice squad, the 53, the inactive list, waivers, OTAs, minicamp, free agency, the franchise tag, the GM, the owner, Sundays). Words for school (class, grades, eligibility, study hall) fit high school and college, never the NFL.
- A tag must be true of everyone who carries it. `ol` is tackles, guards and centers ("up front", "my man", "pass pro"); `dl` is edges and interior tackles; `db` is corners and safeties ("the back end", "coverage"); `kicker` is kickers and punters ("my leg", "operation time"). A personality tag (`hot`, `quiet`, `ego`...) means the line must sound like that person.
- Tags that never meet on one speaker can't share a line: two levels, two regions, two positions, `hot` and `calm`, `young` and `vet`, a high school class year with an NFL stage.
- Region tags (`texas`, `south`, `midwest`, `northeast`, `west`) are where he grew up: a light touch ("back home", a turn of phrase), never dialect spelling or caricature. Texans also count as `south`.
- Careers are played from 2026 on, so phones, texts, group chats, posting, film on a tablet, NIL, revenue sharing and the portal all exist. Still tag them: `modern` for phones, texts, DMs, posting, social media; `nil` for NIL, revenue share, collectives and the portal. Never `pre2000` or `pre2010`.
- When-checks (`fact=value`, only where your part lists them) make a line wait for its moment: `lead=down` is said only when you're behind, `!moment=halftime` never at halftime. Use them when the words need the moment, never just to have them.

## Never assert what the world may contradict

The game knows things your line doesn't. Never claim:

- family members by role or name (no mom, dad, brother, sister, wife, husband, girlfriend, son, kids of his own): say "my family", "my people", "back home", "the folks at home". The exceptions are written into the parts that have them.
- a specific game, opponent, result, score, record, streak, stat, injury, award, day or date ("Tuesday", "last night", "tonight"), unless your part gives it as a fact or a when-check settles it.
- a class year, a draft round, a contract, a starter's job or a backup's role, unless a tag or the part's meaning makes it true.
- that a name is singular or plural. A team's name reads "Alabama" at college and "Buffalo Bills" at the NFL, so "the {them}" and "{them} are" fit only lines tagged nfl, and "{them} is" only lines tagged hs or college; "beat {them}" and "the {them} game" fit everyone. Never "a {team}" or "a {them}" (a or an depends on the name).

## Voice

Real people in 2026 talking about football and their lives: players to a head coach they respect, fear, doubt or love; coaches to coaches; families to a recruiter in their living room; bosses, boosters and agents to the coach they deal with. Specific beats generic: name the real football thing (a pressure call, the depth chart, a rep in the two-minute drill, a cut-down day) rather than a cliché. Short and plain beats clever. No speeches unless the part is a speech, no motivational-poster lines, no therapy talk, no sports-movie clichés.

## Variety

- Every line distinct in shape and words: never the same sentence with one word swapped, never a light rewording of a line you're shown.
- Vary the length inside the part's range, the rhythm, questions against statements, and how lines open. Keep each two-word opening ("Coach, I", "I think") to at most one line in twenty, counting the lines you're shown.
- Spread across the tags your angle asks for, and cover every level the part is said at about evenly.
- Write exactly the number of lines asked, one per line, and nothing else.
