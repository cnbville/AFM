"""Every part the line bank run fills: who says it, the facts and when-checks
it gets, what it is, and how many new versions it gets.

The split is the proposal's (afm/line-bank-api/proposal.md, "Where the
100,000 go"), agreed by Cain with "yes on all" on 2026-11-07: each family's
total, and inside it the groups its "why this many" names, each group
filled thinnest first to an even count (`GROUPS`)."""
import math

from bank import TAGS, bank

# ---------------------------------------------------------------------------
# Who speaks: the tags each kind of speaker can carry (voice.rs, Speaker).

REGION = {'texas', 'south', 'midwest', 'northeast', 'west'}
ERA = {'social', 'modern', 'nil'}          # pre2000 and pre2010 never reach you
LEVEL = {'hs', 'college', 'nfl'}
TRAITS = {'young', 'vet', 'ego', 'humble', 'leader', 'quiet', 'hot', 'calm', 'loyal', 'greedy', 'pro', 'loose',
          'shaky', 'steady', 'competitor'}
SPOT = {'qb', 'rb', 'wr', 'te', 'ol', 'dl', 'lb', 'db', 'kicker', 'snapper'}
GROUP = {'offense', 'defense', 'special', 'skill', 'cover', 'rusher', 'lineman'}
STAGE = {'freshman', 'sophomore', 'junior', 'senior', 'redshirt', 'transfer', 'walkon', 'rookie', 'undrafted',
         'firstround', 'psquad'}
TONE = {'fiery', 'warm', 'firm', 'calm'}
# `brash` (PR #227): a player who talks back in your office. Allowed once the
# tag is in the bank's word list.
BRASH = {'brash'} & TAGS

SPEAKERS = {
    'player': {'player'} | LEVEL | TRAITS | SPOT | GROUP | STAGE | REGION | ERA | BRASH,
    'coach': {'coach', 'she', 'hot', 'calm', 'climber', 'loyal', 'old', 'young', 'expro', 'strict', 'warm', 'cold'}
    | LEVEL | REGION | ERA,
    'cast': {'ad', 'president', 'principal', 'owner', 'gm', 'booster', 'kin', 'she', 'ego', 'hot', 'warm', 'cold',
             'blunt', 'smooth', 'impatient', 'patient', 'meddler', 'tight', 'old'} | LEVEL | REGION | ERA,
    'you': {'you'} | LEVEL | REGION | ERA,
    'parent': {'parent', 'loud', 'online', 'calm', 'absent'} | REGION | ERA,
    # A recruit's side in the recruiting talks (recruit20k): a family at a home visit or on a call (the 'parent'
    # speaker plus whoever decides), or a transfer in the portal (a college 'player'). A version fits the one
    # kind of speaker its tags name; an untagged one fits both. `calm` is left out: it means a calm parent
    # style on one and a calm temper on the other.
    'recruit': {'parent', 'grandparent', 'handler', 'himself', 'loud', 'online', 'absent', 'college', 'freshman',
                'senior', 'redshirt', 'transfer', 'walkon'} | (TRAITS - {'calm'}) | SPOT | GROUP | REGION | ERA,
    # Agents carry no level, region or era: a line with one is never said.
    'agent': {'agent', 'straight', 'slick', 'hot', 'calm', 'big', 'small'},
}

# How each speaker's tags read, for the prompts.
TAG_WORDS = {
    'player': "level (hs, college, nfl: each player has one); career stage (freshman sophomore junior senior "
              "redshirt transfer walkon: school; rookie undrafted firstround psquad: NFL); position (qb rb wr te ol "
              "dl lb db kicker snapper) and groups (offense defense special skill=RB/WR/TE cover=LB/CB/S "
              "rusher=EDGE/DT lineman=OL/EDGE/DT); personality, only the far ends (young: under 23, vet: 28+, ego, "
              "humble, leader, quiet, hot: short fuse, calm, loyal, greedy, pro: very professional, loose: "
              "unprofessional, shaky: unstable background, steady, competitor)"
              + ("; brash: talks back to his coach (an NFL ego or temper, or a college player with both who "
                 "doesn't trust you yet; never a high school player)" if BRASH else ""),
    'coach': "level of his job (hs, college, nfl); she (a woman coach); hot, calm (temper); climber (ambitious); "
             "loyal; old (55+), young (under 35); expro (played in the NFL); strict (big on discipline); warm, "
             "cold (with people)",
    'cast': "role (the part says who); level (hs, college, nfl); she; ego; hot (temper); warm, cold; blunt "
            "(very honest), smooth (spins things); impatient, patient; meddler (gets into football matters); "
            "tight (with money); old (60+)",
    'you': "level of your job (hs, college, nfl). You have no other traits: your voice is the tone you picked",
    'parent': "how they are with their son's football: loud (loud at games), online (posts about it), calm "
              "(calm and private), absent (rarely around)",
    'recruit': "who is talking. FAMILY SIDE (a high school recruit's home visit or call): parent (a parent or "
               "guardian who decides for him: 'my son', 'my boy'), grandparent ('my grandson'), handler (a "
               "trainer or 7-on-7 coach who steers him, talks business), himself (the 17-year-old recruit "
               "speaking for himself, plain and a little guarded), and how a parent is around his football: loud "
               "(loud at games), online (posts about it), absent (rarely around). TRANSFER SIDE (a college "
               "player in the portal, in a college player's words): college, transfer (has transferred before), "
               "freshman, senior, redshirt, walkon, position (qb rb wr te ol dl lb db kicker snapper) and "
               "groups, personality (young, vet, ego, humble, leader, quiet, hot, loyal, greedy, pro, loose, "
               "shaky, steady, competitor). A line tagged for the family side is never said by a transfer and "
               "the other way round, so never mix the two sides' tags on one line; region and era tags go with "
               "either. An untagged line is said by both sides, so its words must fit a family at a living-room "
               "visit and a transfer on the phone alike",
    'agent': "straight (honest), slick (spins and bluffs); hot, calm (temper); big (a big agency), small (a "
             "one-man shop). Never a level, region or era tag",
}

# ---------------------------------------------------------------------------
# The facts a line can name, and what they read like.

FACT_WORDS = {
    'name': "a first name", 'full': "a full name", 'coach': "'Coach' and your last name, filled in by the game (you, as players say it; never write a name yourself)",
    'you': "your last name, filled in by the game", 'friend': "his closest friend still on the team, first name",
    'gone': "the friend who left the team, first name",
    'team': "your team's name, as 'this {team} team' or 'the {team} locker room' (never 'a {team}': a or an "
    "depends on the name)",
    'town': "your team's town", 'year': "the season, 2027", 'old': "the coach before you, full name",
    'old_last': "his last name", 'rival': "your rival's name (the same rules as {them})", 'rivalry': "the rivalry game's name",
    'series': "the series record, 41-38", 'last_record': "last season's record, 7-5", 'record': "the record now, 4-2",
    'count': "a number of players (11)", 'role': "his job, in lower case ('offensive line coach')",
    'came_up': "where he came up, a short phrase ('at Toledo')", 'boss': "the boss's first name",
    'met': "how many of the season's goals you met (a number)", 'want': "what the booster wants, a short phrase",
    'old_school': "the booster's own old high school ('Valley High')", 'them': "the opponent's name: 'Iowa "
    "State', or at the NFL 'Buffalo Bills'. So 'the {them}' as a name and '{them} are' fit only nfl-tagged lines, "
    "'{them} is' only lines tagged hs or college; '{them}' as an object ('beat {them}') and 'the {them} game' fit "
    "every level; never 'a {them}'", 'us': "your team's name (the same rules as {them})", 'unit': "one of your units in lower case "
    "(quarterback, offensive line, receivers, front, linebackers, corners, kicker)",
    'player': "a player's first name", 'captain': "the captain's first name", 'money': "a sum ($14.5M)",
    'pos': "his position as the game writes it: QB, RB, WR, TE, OT, IOL, EDGE, DT, LB, CB, S, K, P or LS. Write "
    "'at {pos}', 'the {pos} room', 'your {pos} group'; never add an s to it, never write 'a {pos}' or 'an {pos}'",
    'club': "an NFL club's full name ('Buffalo Bills': write 'the {club}', plural verbs)", 'first': "what matters most to the "
    "family, lower case (playing time, nil money, the school, close to home, family, getting him to the next "
    "level, winning)", 'second': "what matters next (same list)",
    # The recruit conversation's facts (talk facts lane L5, `claims::recruit`).
    'opp': "the other team in a game, its name ('Kansas'): 'beat {opp}', 'the {opp} game'; never 'a {opp}'",
    'score': "a game's score, your team's first ('31-17' in a win, '17-31' in a loss)", 'when': "the day in words, with its own word: 'today', "
    "'yesterday', 'on Saturday', 'last week', 'on October 4' ('{school} offered {when}', never 'on {when}')",
    'school': "another college's name ('Baylor')", 'spot': "a position as people say it, lower case ('receiver', "
    "'corner', 'quarterback')", 'rival_school': "a school on his list whose proof matches yours ('TCU')",
}
PRONOUN_WORDS = ("{coach_he} {coach_him} {coach_his} {coach_himself}: your pronouns (he or she); {coach_man}: man "
                 "or woman; {coach_sir}: sir or ma'am, to your face")

BASE = {'team', 'town', 'you', 'coach', 'year', 'old', 'old_last', 'rival', 'rivalry', 'series', 'last_record'}
TALK = {'name', 'coach', 'you', 'friend', 'team'}

# When-checks: `fact=value` makes a version wait for that moment.
GAMEDAY_WHEN = {'moment': {'pregame', 'halftime'}, 'lead': {'up', 'down', 'level'}, 'last': {'win', 'loss', 'none'},
                'rival': {'yes'}}
SUBJECT_WHEN = {'subject': {'us', 'them', 'plan', 'stakes', 'seniors', 'one'}}
MISTAKE_WHEN = {'mistake': {'fumble', 'interception', 'extended', 'killed', 'blown', 'pattern'}}
REASON_WHEN = {'reason': {'snaps', 'coach', 'money', 'up', 'choice', 'grad'}}


class Part:
    def __init__(self, key, family, who, facts, note, when=None, levels=None, words=(1, 28), extra=(), drop=()):
        self.key, self.family, self.who, self.note = key, family, who, note
        self.facts = set(facts)
        # Your pronouns always fill, so a part that names you as {coach} may use them.
        if 'coach' in self.facts:
            self.facts |= {'coach_he', 'coach_him', 'coach_his', 'coach_himself', 'coach_man', 'coach_sir'}
        self.when = when or {}
        # The levels the scene happens at: None, any (the speaker's level
        # tag decides); otherwise the only levels it is said at.
        self.levels = levels
        self.words = words
        self.phrase_cap = False
        # Tags this part allows beyond its speaker's (a decider in a home
        # visit, your tone on game day), and speaker tags it never fits.
        self.tags = (SPEAKERS[who] | set(extra)) - set(drop)
        if levels:
            self.tags -= LEVEL - set(levels)


PARTS = {}


def add(key, *a, **k):
    PARTS[key] = Part(key, *a, **k)


# --- talks: one-on-ones in your office (talks.rs) ---------------------------
MOODS = {'sour': "his mood is clearly low", 'good': "his mood is clearly high",
         'flat': "his mood is in between: neutral, polite, guarded, businesslike, a little tired"}
for m, d in MOODS.items():
    add(f'talk.open.{m}', 'talks', 'player', TALK,
        f"His first words on walking in; {d}. Under 15 words, full punctuation. Never names or hints at a "
        "subject (the subject part follows in the same breath).", words=(2, 15))
SUBJECTS = {
    'role': "his place: the depth chart, starting, competing for a job, playing time",
    'usage': "how he's used: touches, targets, carries, snaps, packages, rotation, assignments, scheme fit",
    'money': "what he earns or gets: NFL contract, guarantees, bonuses; college NIL, revenue share; HS costs, a "
             "job",
    'behavior': "his conduct (late, a missed meeting or class, rules, temper, a scuffle, what he posts): he "
                "doesn't know exactly why he's here, so guesses are fine, never an incident stated as fact",
    'life': "life away from football: home, school, sleep, health, faith, friends, how he's doing",
    'team': "the team as a whole: how it's going, the locker room, leaders, the staff, the season (never a "
            "record or result)",
    'future': "what's next: staying, the portal, the draft, a new deal, free agency, recruiting, next season",
}
for s, d in SUBJECTS.items():
    add(f'talk.open.{s}', 'talks', 'player', TALK,
        f"What he says once he knows or guesses the subject: {d}. Said right after his mood part, so no "
        "greeting, and it never assumes a mood (he may or may not have a grievance).")
add('talk.open.friend', 'talks', 'player', TALK | {'gone'},
    "His opening when you called him in about his friend {gone} leaving the team, and he's in a good mood. "
    "Greeting and subject in one.")
for m, d in MOODS.items():
    add(f'talk.open.portal.{m}', 'talks', 'player', TALK,
        f"He asked for this meeting to tell you he means to leave (the transfer portal); {d}. A reason= check "
        "may set why: snaps (playing time), coach (the staff), money, up (a step up), choice (wants a change), "
        "grad (a graduate transfer).", when=REASON_WHEN, levels={'college'})
REVEALS = {
    'role': "he thinks he should be playing more (more, my spot, the chart, a fair shot; never that he never "
            "plays)",
    'usage': "how he's used (scheme, touches, assignments, a decoy, the wrong job)",
    'money': "he thinks he's underpaid (mostly NFL; some college NIL; untagged lines fit any level)",
    'behavior': "what's behind his conduct slipping: owning it, explaining it, asking for help (no incident "
                "stated as fact)",
    'life': "something heavy at home the line can't name (an illness, a death, a divorce, money lost, being "
            "homesick, trouble at home): write its weight, never the event",
    'team': "how the team feels to him, fitting losing, a staff he doesn't trust yet and a sour room alike "
            "(never names one)",
    'future': "thinking about leaving or what's next (mostly college players weighing the portal)",
    'friend': "his friend {gone} leaving: what it did to him",
}
for r, d in REVEALS.items():
    add(f'talk.reveal.{r}', 'talks', 'player', TALK | ({'gone'} if r == 'friend' else set()),
        f"He trusts you enough to say what's really bothering him: {d}. First person, honest, often vulnerable.")
add('talk.deflect', 'talks', 'player', TALK, "Asked what's going on, he won't open up: a polite or guarded "
    "brush-off that fits any subject.")
MOVES = {
    'ask': "you asked what's on his mind and he answers (no reveal: that's another part)",
    'straight': "you told him straight where things stand",
    'praise': "you praised him",
    'challenge': "you challenged him to do more",
    'criticize': "you criticized him",
    'promise': "you promised him something (the depth chart, the ball, the collective, the agent, next year or "
               "you staying: never name which)",
    'warn': "you warned him",
    'listen': "you just listened",
    'hand': "you handed it to the leadership council or gave him time at home (fit both)",
}
OUTCOMES = {'good': "it landed well", 'flat': "it didn't move him much", 'bad': "it landed badly"}
for mv, d in MOVES.items():
    for o, od in OUTCOMES.items():
        add(f'talk.reply.{mv}.{o}', 'talks', 'player', TALK,
            f"His answer to your move: {d}, and {od}. Never names the subject (the part doesn't know it).")
for o, od in OUTCOMES.items():
    add(f'talk.reply.friend.{o}', 'talks', 'player', TALK | {'gone'},
        f"About his friend {{gone}} leaving, his answer to what you told him, and {od}.")
    add(f'talk.reply.friend.hand.{o}', 'talks', 'player', TALK | {'gone'},
        f"About his friend leaving, you asked him to lean on a teammate, {{friend}}, and {od}.")
    add(f'talk.close.{o}', 'talks', 'player', TALK,
        f"An optional last sentence after his reply, when {od}: a short sign-off that fits any move and subject.",
        words=(1, 15))

# --- game day: your words, the captains, the sideline, their quote ----------
GD_SUBJ = ("A subject= check may say what the talk is about: us (our standard, who we are), them (the opponent, "
           "what they said), plan (the plan; at halftime the adjustments), stakes (what's on the line; at halftime "
           "the half), seniors (the veterans at the NFL), one (one player you single out, {player}, often a "
           "starter out hurt: never say whether he plays).")
for t in sorted(TONE):
    add(f'gameday.open.{t}', 'gameday', 'you', {'them'},
        f"The opening of your talk to the team before the game or at halftime, in a {t} tone. One or two "
        "sentences that lead into your point.", when=GAMEDAY_WHEN, extra=TONE)
    add(f'gameday.close.{t}', 'gameday', 'you', {'them'},
        f"An optional last sentence of your talk, in a {t} tone: how you send them out.", when=GAMEDAY_WHEN,
        extra=TONE, words=(2, 18))
POINTS = {
    'ask': "you ask them something and wait for an answer",
    'straight': "you tell them straight what this game needs",
    'praise': "you praise them",
    'challenge': "you challenge them",
    'criticize': "you criticize how they've played or prepared",
    'promise': "you promise them something real (playing time is earned, the plan works; never a win)",
    'warn': "you warn them",
    'listen': "you let them talk: a line that opens the floor",
}
for mv, d in POINTS.items():
    add(f'gameday.{mv}', 'gameday', 'you', {'them', 'player'},
        f"The point of your talk: {d}. Any tone may say it (tag fiery, warm, firm or calm to keep a line to one). "
        + GD_SUBJ, when=GAMEDAY_WHEN | SUBJECT_WHEN, extra=TONE)
add('gameday.captain', 'gameday', 'you', {'them', 'captain'},
    "You hand the room to a captain: the line that gives him the floor (name him with {captain}).",
    when=GAMEDAY_WHEN, extra=TONE, words=(3, 20))
CAPTAIN = {
    'open': "his first words to the team when you hand him the room",
    'own': "what he says about himself or his own part",
    'us': "what he says about the team, the guys in the room",
    'job': "what he says about the job today",
    'break': "the last words, breaking the huddle",
}
for c, d in CAPTAIN.items():
    add(f'gameday.captain.{c}', 'gameday', 'player', {'them'},
        f"A captain speaking to the team before the game or at halftime: {d}. Short, in his own voice.",
        when=GAMEDAY_WHEN, words=(2, 20))
add('gameday.quote', 'gameday', 'player', {'us', 'unit'},
    "An opposing player with an ego, quoted in the paper before the game, talking about one of your units "
    "({unit}) and your team ({us}). Confident, a little disrespectful, never vulgar.", words=(5, 30))
WORDS = {
    'straight': ("tell him straight what went wrong and what to do", {'calm', 'firm'}),
    'praise': ("lift him up", {'warm'}),
    'challenge': ("challenge him to answer it", {'fiery'}),
    'criticize': ("criticize the mistake", {'fiery', 'firm'}),
    'warn': ("warn him that it can't happen again", {'firm'}),
    'listen': ("ask what happened and listen", {'calm'}),
}
for mv, (d, tones) in WORDS.items():
    add(f'gameday.word.{mv}', 'gameday', 'you', {'player'},
        f"Your word on the sideline to {{player}} after a mistake: {d}. Said in a {' or '.join(sorted(tones))} "
        "tone. A mistake= check may name it: fumble, interception, extended (his flag kept their drive alive), "
        "killed (his flag killed ours), blown (a blown assignment on their score), pattern (his third small "
        "mistake today).", when=MISTAKE_WHEN, extra=tones, words=(3, 22))

# --- staff: the game day reads, the Monday meeting, hiring ------------------
STAFF_READ = {
    'good': "the room is right", 'amped': "the room is too amped", 'tight': "the room is tight",
    'flat': "the room is flat", 'hollow': "the room sounds ready but isn't (hollow)",
    'captain': "the captain could say it better than you today (may name him, {player})",
}
for k, d in STAFF_READ.items():
    add(f'gameday.staff.{k}', 'staff', 'coach', {'player'},
        f"One of your coordinators tells you his read of the room before your talk: {d}. Coach to coach, "
        "quiet, one or two sentences.", words=(4, 25))
CASES = {'pressure': "more pressure", 'zone': "more zone coverage", 'run': "running it more",
         'pass': "throwing it more"}
for k, d in CASES.items():
    add(f'staffmeet.case.{k}', 'staff', 'coach', set(),
        f"In the staff meeting the morning after a game, a coordinator makes his case for {d}. Coach talk, "
        "specific, two or three sentences.", words=(8, 40))
REPLIES = {'yes': "you agreed with him", 'half': "you met him halfway", 'no': "you overruled him"}
for k, d in REPLIES.items():
    add(f'staffmeet.reply.{k}', 'staff', 'coach', set(), f"The coordinator's answer when {d}.", words=(2, 25))
add('staff.interview.open', 'staff', 'coach', {'role', 'team', 'came_up'},
    "A coach you are interviewing for a job on your staff opens the interview: who he is and why he wants it.",
    words=(8, 40))

# --- recruiting: home visits, high school coach calls, combine rooms --------
PITCH = {
    'playing': "playing time", 'nil': "NIL money", 'school': "the school itself", 'distance': "staying close "
    "to home", 'family': "family, and how the program treats families", 'development': "getting him to the "
    "next level", 'winning': "winning",
}
DECIDER = ("The speaker is whoever decides in his family: tag himself (the recruit speaks, first person), parent, "
           "grandparent or handler (a trainer or 7-on-7 coach who steers him), or leave it untagged to fit all.")
for k, d in PITCH.items():
    add(f'homevisit.ask.{k}', 'recruit', 'parent', {'name'},
        f"On your home visit, the family's first question to you, about what they care about most: {d}. "
        "{name} is the recruit. " + DECIDER, extra={'himself', 'parent', 'grandparent', 'handler'},
        levels={'hs', 'college'}, words=(5, 35))
add('homevisit.tell', 'recruit', 'parent', {'first', 'second'},
    "You asked the family what matters most; they answer with {first}, then {second}. " + DECIDER,
    extra={'himself', 'parent', 'grandparent', 'handler'}, levels={'hs', 'college'}, words=(5, 30))
for k, d in OUTCOMES.items():
    add(f'homevisit.react.{k}', 'recruit', 'parent', set(),
        f"The family's reaction to your pitch, when {d}. " + DECIDER,
        extra={'himself', 'parent', 'grandparent', 'handler'}, levels={'hs', 'college'}, words=(2, 25))
for k, d in {'football': "the boy as a player: his game, his motor, his ceiling",
             'grades': "his grades and whether he'll qualify", 'character': "his character and the kid off "
             "the field"}.items():
    add(f'hscall.ask.{k}', 'recruit', 'coach', {'name'},
        f"A college coach phones you, the high school coach, about your player {{name}}: he asks about {d}.",
        levels={'hs', 'college'}, drop={'hs', 'nfl'}, words=(5, 35))
for k, d in {'steady': "he stays steady", 'bristles': "he bristles", 'shaken': "he is shaken"}.items():
    add(f'combine.press.{k}', 'recruit', 'player', set(),
        f"At the combine, your club interviews a draft prospect and presses him hard; {d} in his answer.",
        levels={'college', 'nfl'}, drop={'hs', 'nfl'} | STAGE - {'senior', 'redshirt', 'transfer', 'walkon'},
        words=(3, 30))

# --- the AD's office: the boss's meetings and job interviews ----------------
BOSS = "Your boss (high school: the principal; college: the athletic director; the NFL: the owner) "
add('boss.goals.open', 'ad', 'cast', BASE | {'boss'}, BOSS + "opens the meeting where he sets your goals for "
    "the season.", words=(5, 40))
add('boss.goals.point', 'ad', 'cast', BASE | {'boss'}, BOSS + "adds one more point about what he expects this "
    "season (an optional second line).", words=(4, 35))
add('boss.mid.open', 'ad', 'cast', BASE | {'boss', 'record'}, BOSS + "opens the midseason check-in; {record} "
    "is the record so far.", words=(5, 40))
for k, d in {'good': "you met most of your goals", 'mixed': "you met some", 'bad': "you met few"}.items():
    add(f'boss.review.{k}', 'ad', 'cast', (BASE | {'boss', 'met'}) - {'last_record'} - ({'met'} if k == 'bad'
                                                                                      else set()),
        BOSS + f"opens the end-of-season review: {d} ({{met}} is how many).", words=(5, 40))
PANEL = ("On a job interview panel (high school: the principal and the athletic director; college: the "
         "athletic director and the president; the NFL: the owner and the general manager), ")
for k, d in {'plan': "your plan (win now, build it, develop)", 'staff': "your staff (your own people, keep the "
             "good ones, the best available)", 'past': "your past (owning a failure, pointing to results, the "
             "people)"}.items():
    add(f'interview.ask.{k}', 'ad', 'cast', set(), PANEL + f"one of them asks you about {d}.", words=(5, 35))
for k, d in OUTCOMES.items():
    add(f'interview.react.{k}', 'ad', 'cast', set(), PANEL + f"one of them reacts to your answer, when {d}.",
        words=(2, 25))

# --- boosters ----------------------------------------------------------------
BOOST = "A big booster (the NFL: a member of the owner's family) "
add('booster.greet', 'boosters', 'cast', BASE | {'name'}, BOOST + "greets you the first time you meet ({name} "
    "is the booster's own first name).", words=(4, 35))
add('booster.small', 'boosters', 'cast', BASE | {'name'}, BOOST + "makes small talk.", words=(3, 30))
WANTS = {'access': "access (the sideline, the team plane, the coaches' box)", 'building': "his name on the "
         "next building", 'clean': "a clean program (no NCAA trouble, nobody in the paper)", 'hires': "a say in "
         "who you hire", 'nil': "NIL deals that put players in his company's ads", 'oldschool': "his old high "
         "school's best players recruited ({old_school} is that school)", 'player': "a say on players he likes",
         'rival': "beating the rival, every year", 'winnow': "winning now, not in three years"}
for k, d in WANTS.items():
    add(f'booster.want.{k}', 'boosters', 'cast', BASE | {'name', 'want'} | ({'old_school'} if k == 'oldschool'
                                                                             else set()),
        BOOST + f"tells you what he wants: {d} ({{want}} says it in a few words).", words=(5, 40))

# --- the room: team rules, parents, a new job's first meetings --------------
for k, d in {'fair': "fair", 'harsh': "harsh", 'soft': "soft"}.items():
    add(f'rules.react.{k}', 'room', 'player', {'coach'},
        f"A player's reaction to how you handled a team rule, when he thinks it was {d}.", words=(3, 25))
PARENT_AS = ("A loud or online parent (shown as his father, mother, grandmother or guardian, so never say "
             "which) calls you because their boy {name} is unhappy with his role and playing time: ")
for k, d in {'open': "the opening", 'point': "their point (an optional second line)"}.items():
    add(f'parents.call.{k}', 'room', 'parent', {'name', 'full', 'coach', 'you', 'team', 'friend'}, PARENT_AS + d + ".",
        levels={'hs', 'college'}, drop={'calm', 'absent'}, words=(5, 35))
add('parents.playing_time', 'room', 'parent', BASE, "At a high school, a father wants his son to play more.",
    levels={'hs'}, words=(5, 35))
add('takeover.meeting.you_open', 'room', 'you', BASE | {'count'},
    "Your first words at your first team meeting in a new job ({count} is how many players are in the room).",
    words=(8, 45))
for k, d in {'compete': "every job is open, the depth chart starts over", 'honour': "you honour the past "
             "and the men who built it", 'plain': "short: who you are, how you work, then practice", 'purge':
             "some of you won't be here, find out if you want this", 'winnow': "we win now, the standard is a "
             "title"}.items():
    add(f'takeover.meeting.react_{k}', 'room', 'player', BASE,
        f"A player's reaction to your first team meeting, where your message was: {d}.", words=(3, 30))
for k, d in {'close_to_old': "close to the coach before you", 'ego': "with an ego", 'steady': "steady",
             'waiting': "waiting to see"}.items():
    add(f'takeover.leader.{k}', 'room', 'player', BASE,
        f"One of the team's leaders meets you in your first weeks; he is {d}.", words=(4, 35))
for k, d in {'pitch': "his pitch to stay on your staff", 'detail': "one more detail about his work"}.items():
    add(f'takeover.holdover.{k}', 'room', 'coach', BASE | {'role'},
        f"An assistant from the old staff, {{role}}, meets you: {d}.", words=(6, 40))
add('takeover.oldcoach.greet', 'room', 'coach', BASE - {'old', 'old_last'},
    "At a high school, the coach before you (still at the school) greets you.", levels={'hs'}, words=(5, 40))

# --- agents (NFL contract talks) ---------------------------------------------
AG = "An NFL agent answers your contract offer for his client {name}: "
for k, d in {'yes': "it gets it done (he'll still wait for the next market day)", 'range': "it's in range; he'll "
             "weigh it against the others", 'apart': "you're apart", 'far': "you're far apart", 'short':
             "it's short of his number", 'insult': "it isn't a serious offer"}.items():
    add(f'agent.{k}', 'agents', 'agent', {'name', 'money'}, AG + d + ".", levels={'nfl'}, words=(3, 30))
add('agent.near', 'agents', 'agent', {'name', 'money'}, AG + "you're close; {money} closes it.", levels={'nfl'},
    words=(3, 30))
add('agent.done', 'agents', 'agent', {'name'}, "An NFL agent breaks off the talks for his client {name}.",
    levels={'nfl'}, words=(3, 30))
add('agent.claim.real', 'agents', 'agent', {'club', 'money'}, "An NFL agent tells you, truly, another club "
    "(the {club}) is at {money} a year.", levels={'nfl'}, words=(5, 30))
add('agent.claim.bluff', 'agents', 'agent', {'money'}, "An NFL agent claims other clubs are above you, near "
    "{money} a year (it may be a bluff; the line must read the same either way).", levels={'nfl'}, words=(5, 30))
add('agent.claim.market', 'agents', 'agent', {'name', 'money'}, "In a re-signing talk, an NFL agent says the "
    "open market would pay {name} {money} a year.", levels={'nfl'}, words=(5, 30))
add('agent.claim.stay', 'agents', 'agent', {'name'}, "In a re-signing talk, an NFL agent says {name} knows "
    "what the market pays but would like to stay: meet them near their number.", levels={'nfl'}, words=(5, 30))

# Parts the game never asks for (game day words promise and ask are never
# offered): left out of the run.
NEVER = {'gameday.word.promise', 'gameday.word.ask'}

# ---------------------------------------------------------------------------
# How many new versions each part gets.

SPLIT = {'talks': 52000, 'gameday': 23000, 'staff': 7000, 'recruit': 6000, 'ad': 4000, 'agents': 4000,
         'room': 3000, 'boosters': 1000}
FILES = {f: f'{f}_run.txt' for f in SPLIT}

# Inside each family (the proposal's "why this many"): groups of parts by
# key prefix with their share, each filled thinnest first. The game day
# words promise and ask are never offered, so the family's 23,000 go to its
# other 29 parts (Claude's, proposed, not confirmed; the proposal counted 31).
GROUPS = {
    'talks': [(('talk.open.',), 21500), (('talk.reply.',), 18000), (('talk.close.',), 12000),
              (('talk.reveal.friend',), 500)],
    'gameday': [(('gameday.',), 23000)],
    'staff': [(('staff.interview.open',), 800), (('staffmeet.', 'gameday.staff.'), 6200)],
    'recruit': [(('homevisit.', 'hscall.', 'combine.'), 6000)],
    'ad': [(('interview.',), 2500), (('boss.',), 1500)],
    'agents': [(('agent.',), 4000)],
    'room': [(('rules.', 'parents.', 'takeover.'), 3000)],
    'boosters': [(('booster.',), 1000)],
}
# What the proposal fills first: the portal and friend openings, the friend
# and promise replies, the six captain parts, the team leaders at a new job.
FIRST = ('talk.open.portal.', 'talk.open.friend', 'talk.reply.friend.', 'talk.reply.promise.', 'gameday.captain',
         'takeover.leader.')


def now_counts(skip_run=True):
    b = bank(skip=set(FILES.values()) if skip_run else ())
    return {k: len(v) for k, v in b.items()}


def allocation(now=None):
    """{key: new versions to write}: each group's share, filled thinnest
    first to an even count (water filling)."""
    now = now if now is not None else now_counts()
    out = {k: 0 for k in PARTS}
    for fam, groups in GROUPS.items():
        for prefixes, total in groups:
            keys = [k for k, p in PARTS.items() if p.family == fam and k.startswith(prefixes)]
            have = {k: now.get(k, 0) for k in keys}
            lo, hi = 0.0, 1e7
            for _ in range(200):
                mid = (lo + hi) / 2
                lo, hi = (mid, hi) if sum(max(0.0, mid - have[k]) for k in keys) < total else (lo, mid)
            raw = {k: max(0.0, hi - have[k]) for k in keys}
            s = sum(raw.values()) or 1
            for k in keys:
                out[k] = int(round(raw[k] * total / s))
    return out


if __name__ == '__main__':
    now = now_counts()
    al = allocation(now)
    for fam in SPLIT:
        keys = sorted(k for k in PARTS if PARTS[k].family == fam)
        print(f'{fam}: {len(keys)} parts, {sum(now.get(k, 0) for k in keys)} now, +{sum(al[k] for k in keys)}')
        for k in keys:
            print(f'   {k:34} {now.get(k, 0):6} +{al[k]:5}')
    missing = [k for k in now if any(k.startswith(p) for p in ('talk.', 'gameday.', 'staffmeet.', 'staff.interview',
                                                                    'homevisit.', 'hscall.', 'combine.', 'boss.',
                                                                    'interview.', 'booster.', 'rules.', 'parents.',
                                                                    'takeover.', 'agent.'))
               and k not in PARTS and k not in NEVER]
    print('bank parts not in the run:', missing)
