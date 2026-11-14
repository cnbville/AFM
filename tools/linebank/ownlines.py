"""Cain's own lines (talk facts lane L12; Cain 2026-11-13, "won't running it through the API be way better?"): the
moves and answers you pick in a talk, and your pitch to a recruit, written through the API like the other line
runs. Before this, your side was about 50 fixed sentences in the code (and a few hand-written lines).

Every part below is a key the code asks (`talk_points.rs` `moves`, `with_bank_answers`; `talks.rs`
`add_talk_picks`; `pitch.rs` `pitch_words`), found by `ownlines_src.py`, which reads the Rust source: the point's
facts and the states that pick between today's wordings come from the same code that asks for the line. The code's
own wording is each part's fallback, so a part with no version still says what it said before.

Run it with `run.py --job ownlines ...`; the lines land in `own_lines_run.txt`."""
import re

import ownlines_src as S
from parts import FACT_WORDS, FILES, Part, PARTS
from bank import FACTS

FAMILY = 'ownlines'
FILE = 'own_lines_run.txt'
FILES[FAMILY] = FILE

TARGETS = {}
FACETS = {}

BASE = {'name', 'coach', 'you', 'pos', 'team'}
TALK = BASE | {'friend', 'gone'}

HOLE_WORDS = {
    'ahead': "the man ahead of him at his spot, first name", 'ahead_all': "the men ahead of him, as a short list of "
    "first names", 'ahead_class': "his class, lower case ('senior', 'junior')", 'award': "an award, lower case",
    'bench': "his bench number", 'forty': "his forty time, 4.52", 'best': "a player's name", 'best_round':
    "a draft round in words ('second')", 'building': "a building's name", 'clubs': "a number of NFL clubs",
    'conference': "the conference's name", 'conference_titles': "a number", 'event': "an event, lower case",
    'first_round': "a number", 'game': "a game's name", 'games': "a number of games", 'guys': "a number of players",
    'home': "his home town", 'hours': "a number of hours", 'hs': "his high school's name", 'hs_coach': "his high "
    "school coach's name", 'injury': "his injury, lower case ('ankle')", 'last_record': "last season's record, 7-5",
    'leaving': "a number", 'line': "his stat line, lower case ('three touchdowns')", 'lost': "a number", 'miles':
    "a number of miles", 'mistake': "his mistake, lower case ('two turnovers')", 'name': "a first name", 'names':
    "first names", 'opens': "a number of weeks", 'opp': "the other team's name", 'parent': "who called, lower "
    "case ('mom', 'dad', 'family')", 'pc': "his position coach's name", 'picks': "a number", 'picks_words':
    "a number in words ('three')", 'position': "a position, lower case ('safety')", 'pounds': "a number",
    'promise': "what you promised him, in words", 'punishment': "the punishment, lower case", 'record': "a "
    "record, 7-5", 'rival': "the other man's name", 'rule': "the team rule that was broken, lower case",
    'school': "a school's name", 'seasons': "your years here, in words ('six years')", 'seats': "a crowd size",
    'starters': "a number", 'state': "his state", 'staying': "a number", 'system': "a scheme's name ('spread "
    "offense')", 'time': "a time", 'titles': "a number", 'town': "your town", 'u': "the position group's name "
    "('DBU')", 'weeks': "a number of weeks", 'what': "what is coming", 'when': "the day in words ('today', "
    "'last week', 'on Saturday')", 'won': "a number", 'years': "a number",
    # facts the code adds for your lines (talk_points.rs `you_words`, talks.rs `add_talk_picks`)
    'book': "what your team rules say comes next for an arrest, in words (a suspension, a benching, a ruling "
    "from the league)", 'when_due': "the day the next step comes, in words ('next Tuesday', 'on October 4')",
    'starter': "yes or no, whether he is your starter (a when-check only, never a hole)", 'mate': "a teammate's "
    "first name, who looks out for him",
}
for _k, _v in HOLE_WORDS.items():
    FACT_WORDS.setdefault(_k, _v)

MOVES = {
    'ask': "ASK: you ask him a real question about it and wait for the answer. Warm. No lecture, no demand.",
    'straight': "TELL HIM STRAIGHT: you say the plain truth about it, calm and unhurried. Only facts the part lists.",
    'praise': "PRAISE: you tell him he did something real. Warm. Only what the part lists; never a promise.",
    'challenge': "CHALLENGE: you push him to rise to it. Firm, believes in him. A competitor hears a dare, never an insult.",
    'criticize': "CRITICIZE: you hold him to account, hard and fair: what you saw and what has to change. Never about who he is.",
    'promise': "PROMISE: you give your word about it. The game logs it, and a broken word costs you: so only promise what a head coach could keep, in your own words, never a date.",
    'warn': "WARN: you tell him plainly what happens if it happens again. Firm, never a threat to his family.",
    'hand': "HAND HIM SOMETHING: you give him something real (time, help, a person, a tutor, the room's trust). Warm.",
    'listen': "LISTEN: you say very little (a sentence at most) and give him the floor.",
}
ANSWERS = {
    'straight': "STRAIGHT: you tell him the truth about what he asked, plainly.",
    'soften': "SOFTEN: kinder than the whole truth, but nothing false. Reassuring and honest about what you can say.",
    'promise': "PROMISE: you give your word about what he asked. The game logs it.",
    'dodge': "DODGE: a polite non-answer that moves past it. Never rude to him for asking.",
}
SUBJECTS = {
    'role': "his place on the team: whether he is playing, his reps, how he is used",
    'usage': "how you use him: touches, snaps, targets, carries, assignments",
    'money': "what he gets paid (the NFL's contract, a college NIL deal or a high school player's costs)",
    'behavior': "his conduct: effort, attitude, something in the room",
    'life': "his life away from football: home, family, what he carries",
    'team': "the team: its room, its leaders, how he fits",
    'future': "his future: next year, the draft, the portal, where he is headed",
    'friend': "a close friend of his, {gone}, who left the team",
}
GROUPS = {'playing_time': "playing time", 'money': "the money (NIL)", 'school': "the school and the degree",
          'home': "being close to home", 'people': "the people (staff, trust)", 'league': "getting to the league",
          'winning': "winning and the name"}
TIERS = {
    'elite': "your proof here is the best on his list: say it with confidence",
    'strong': "your proof is among the best on his list: confident",
    'fair': "your proof is in the middle of his list: honest and warm, not overselling",
    'weak': "your proof is weak next to his list: you do not oversell it; you say what is true and what you can offer",
}


def _words(d):
    out = []
    for s in d:
        s = s.strip('\\"').strip()
        if re.fullmatch(r'[a-z_]+', s) or not s:
            continue
        out.append(s)
    return out


def _kind_docs():
    src = S.read('claims', 'kinds.rs')
    docs = {}
    for m in re.finditer(r'((?:\s*///[^\n]*\n)+)\s*([A-Z][A-Za-z]+) "([a-z_]+)" ', src):
        docs[m.group(3)] = ' '.join(x.strip()[3:].strip() for x in m.group(1).strip().split('\n'))
    return docs


KDOC = _kind_docs()


def _when(states):
    return {k: set(v) for k, v in states.items() if v}


def _target(n):
    return min(40, 16 + 8 * max(1, n))


def _add(key, who, facts, note, target, when=None, levels=None, words=(4, 26)):
    unknown = [f for f in facts if f not in FACTS]
    assert not unknown, (key, unknown)
    PARTS[key] = Part(key, FAMILY, who, facts, note, when=when, levels=levels, words=words)
    PARTS[key].phrase_cap = False
    TARGETS[key] = target


def _states_note(states):
    if not states:
        return ''
    return (" The code's wording changes with: " + '; '.join(
        f"`{k}`" + (f" ({', '.join(v)}; any other value is the plain case)" if v else '') for k, v in sorted(states.items()))
        + ". A line written for one of those carries it as a when-check; an untagged line is true in every case.")


# --- A: your moves on his talking points ----------------------------------------------------------------------
for (kind, mv), d in sorted(S.point_moves().items()):
    today = _words(d['today'])
    facts = TALK | {h for h in d['holes'] if h in FACTS}
    if kind == 'arrest':
        facts |= {'book', 'when_due'}
    note = (f"Your move in a one-on-one with a player of yours when {KDOC.get(kind, kind)} "
            f"(his talking point `{kind}`). {MOVES[mv]} Today's wording in the code: " + ' | '.join(f'"{x}"' for x in today)
            + "." + _states_note(d['states']))
    if kind == 'arrest':
        note += (" For the straight move, `{book}` is what your team rules say comes next, in words, and `{when_due}` "
                 "the day it comes: a line may use them.")
    _add(f'talk.point.you.{kind}.{mv}', 'you', facts, note, _target(len(today)), when=_when(d['states']))

# --- B: your four answers --------------------------------------------------------------------------------------
for (kind, a), d in sorted(S.answers().items()):
    today = _words(d['today'])
    facts = TALK | {h for h in d['holes'] if h in FACTS}
    ask = {'lost_job': "what he has to do to get his job back", 'buried': "what he has to do to get on the field",
           'promise_open': "whether the promise you made him is still happening",
           'draft': "what you are hearing about where he goes in the draft",
           'your_job': "whether you are staying at your job (the press says you are on the hot seat)"}[kind]
    note = (f"Your answer to a question he asked you: {ask}. {ANSWERS[a]} Today's wording: "
            + ' | '.join(f'"{x}"' for x in today) + "." + _states_note(d['states']))
    _add(f'talk.point.answer.{kind}.{a}', 'you', facts, note, _target(len(today)), when=_when(d['states']))

# --- C: your moves on a subject you call him in about --------------------------------------------------------
_sm = S.subject_moves()
for (subj, mv), d in sorted(_sm.items()):
    today = _words(d['today'])
    subs = list(SUBJECTS) if subj == 'any' else [subj]
    for sub in subs:
        key = f'talk.you.{sub}.{mv}'
        if key in PARTS:
            continue
        facts = TALK | {'starter'} & FACTS | ({'mate'} if sub == 'friend' else set())
        facts = {f for f in facts if f in FACTS}
        note = (f"Your move when you call a player in about {SUBJECTS[sub]}, with no particular event behind it. "
                f"{MOVES[mv]} Today's wording: " + ' | '.join(f'"{x}"' for x in today) + ". "
                "Whether he is your starter (`starter=yes` or `starter=no`) and the level (hs, college, nfl) may pick the "
                "wording: a line that needs one carries it.")
        _add(key, 'you', facts, note, _target(max(2, len(today))), when={'starter': {'yes', 'no'}})

# --- D: your pitch to a recruit or a transfer ---------------------------------------------------------------------
for kind, d in sorted(S.pitch_kinds().items()):
    today = [x for x in _words(d['today']) if ' ' in x]
    facts = BASE | {h for h in d['holes'] if h in FACTS}
    for tier, td in TIERS.items():
        note = (f"Your pitch to a recruit (or a transfer in the portal) on one thing your program has: {KDOC.get(kind, kind)}. "
                f"How your proof reads against the other schools on his list: {td}. Today's wording: "
                + ' | '.join(f'"{x}"' for x in today[:4]) + "." + _states_note(d['states'])
                + " Never a number the part doesn't list. Never name another school.")
        _add(f'recruit.pitch.{kind}.{tier}', 'you', facts, note, _target(max(1, len(today)) // 2 + 1) - 4,
             when=_when(d['states']), levels={'college'})
for g, gd in GROUPS.items():
    _add(f'recruit.pitch.straight.{g}', 'you', BASE,
         f"You have nothing to show a recruit on {gd}, and you tell him so honestly: others on his list can offer "
         "more there. A real coach saying it plainly, not a sales pitch and not an apology.", 30, levels={'college'})
_add('recruit.pitch.straight.any', 'you', BASE,
     "You have nothing to show a recruit on this point, and you tell him so honestly: others on his list can "
     "offer more there.", 30, levels={'college'})

if __name__ == '__main__':
    import collections
    c = collections.Counter(k.split('.')[0] + '.' + k.split('.')[1] for k in PARTS if PARTS[k].family == FAMILY)
    print(len(TARGETS), 'parts,', sum(TARGETS.values()), 'versions', dict(c))
