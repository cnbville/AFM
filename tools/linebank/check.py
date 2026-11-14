#!/usr/bin/env python3
"""The run's checker: one place for every rule lane 13's tools checked
(13b check.py, 13e lint and warn, 13f check13f.py), made strict, because
nobody reads warnings overnight: a line that breaks a rule is dropped and
its part asks again next round.

    check.py FILE...        check files of new lines (@key sections)
    check.py --bank         run the rules over today's bank for the run's
                            parts (no repeat checks): how often the rules
                            would have dropped a line someone already kept

A dropped line never costs the bank anything but its share of the run."""
import collections
import math
import re
import sys

import bank as B
from parts import ERA, GROUP, LEVEL, PARTS, REGION, SPOT, STAGE, TRAITS

NEAR = 0.75         # near repeat: this share of shared words (13f)
START_SHARE = 20    # each two-word opening at most 1 in 20 of a part (13b, 13f)
MILD_SHARE = 20     # mild swearing at most 1 in 20 of a part's new lines

# ---------------------------------------------------------------------------
# Words that place a line. Each pattern names the levels its words fit; an
# untagged line is said at every level its part reaches, so its words must
# fit all of them.
H, C, N = 'hs', 'college', 'nfl'
LEVEL_WORDS = [(re.compile(rx), set(lv)) for rx, lv in [
    (r"\b(fridays?|friday nights?|varsity|jv|pep rally|school board|superintendent|"
     r"state (title|championship|playoffs?|semis?|final)|algebra|report cards?|"
     r"junior day|no pass,? no play|7-on-7|seven-on-seven|core gpa|prom|my offers?|an offer|"
     r"official visits?|unofficial visits?|camps? this summer)\b", {H}),
    (r"\b(study hall|classes|homework|eligib\w*|the sat|the act|gpa|homecoming|booster club|both ways|"
     r"recruiters?)\b", {H, C}),
    (r"\b(scholarships?|walk-?ons?|redshirt\w*|ncaa|bowls?|bowl game|cfp|12-team|twelve-team|signing day|"
     r"campus|dorms?|alumni|compliance|saturdays?|degree|professors?|syllabus|semesters?|spring ball|"
     r"fall camp|portal|nil|collectives?|revenue shar\w*|house settlement|cost of attendance|"
     r"position room meetings|true freshman)\b", {C}),
    (r"\b(the draft|draft (stock|grade|board|projection)|agents?|my deal|new deal)\b", {C, N}),
    (r"\b(salary cap|the cap|cap space|cap hit|gm|general manager|front office|practice squad|the 53|53-man|"
     r"otas?|minicamp|sundays?|trade deadline|traded|free agen\w*|franchise tag|fifth-year option|"
     r"vet minimum|veteran minimum|roster bonus|workout bonus|accrued|pension|waivers?|waived|commissioner|"
     r"pro bowl|guaranteed money|guarantees?|signing bonus|contract year|paycheck|"
     r"the owner|ownership|inactive list|inactives?|game checks?|my rookie deal)\b", {N}),
]]
MODERN = re.compile(r"\b(text|texts|texted|texting|phones?|cell|cellphone|dm|dms|instagram|ig|tiktok|twitter|"
                    r"tweet\w*|group ?chat|social media|online|posted|posting|posts?|ipad|tablet|app|apps|"
                    r"followers|viral|podcast|stream\w*|youtube|snapchat|facetime|zoom)\b")
NIL = re.compile(r"\b(nil|portal|collectives?|rev share|revenue shar\w*|house settlement)\b")
SPOUSE = re.compile(r"\b(wife|wives|husband|girlfriend|boyfriend|fianc\w*)\b")
KIN = re.compile(r"\b(my|our|his|her|your|their|ya) (mom|moms|mama|momma|mother|dad|daddy|father|pops|brothers?|"
                 r"sisters?|sons?|daughters?|kids?|baby|babies|grandma|grandmother|granny|grandpa|grandfather|"
                 r"grandsons?|granddaughters?|uncle|aunt|auntie|cousins?|parents?|folks)\b")
# Where a family may name its own (the speaker's tag, the words it allows).
KIN_OK = {
    'parents.playing_time': {None: re.compile(r"\b(my|our) (sons?|boy|kid)\b")},
    'homevisit.': {'parent': re.compile(r"\b(my|our) (sons?|boy|kid|baby)\b"),
                   'grandparent': re.compile(r"\b(my|our) (grandsons?|grandbaby|boy|baby)\b")},
    # The recruiting talks run (recruit20k): a family speaks of its boy as the home visit parts do.
    'recruit.': {'parent': re.compile(r"\b(my|our) (sons?|boy|kid|baby)\b"),
                 'grandparent': re.compile(r"\b(my|our) (grandsons?|grandbaby|boy|baby)\b")},
    # Who raised him is the part's fact (the game writes "his mother" for a single parent, "his grandmother"
    # for grandparents): the recruit himself and a transfer may name her.
    'recruit.bg.single_parent': {t: re.compile(r"\b(my|our) (mom|mother|mama|momma)\b") for t in ('himself', 'college')},
    'recruit.bg.grandparents': {t: re.compile(r"\b(my|our) (grandmother|grandma|granny|grandparents)\b")
                                for t in ('himself', 'college')},
    'recruit.bg.brother_': {
        'himself': re.compile(r"\b(my|our) (older |big )?brother\b"),
        'college': re.compile(r"\b(my|our) (older |big )?brother\b"),
        'parent': re.compile(r"\b(his|their) (older |big )?brother\b|\b(my|our) (older|oldest|big) (son|boy)\b"),
        'grandparent': re.compile(r"\b(his|their) (older |big )?brother\b|\b(my|our) (older|oldest|big) grandson\b"),
        'handler': re.compile(r"\b(his|their) (older |big )?brother\b")},
}
# Game days read as level words (fridays: high school, saturdays: college,
# sundays: the NFL); every other day or time is a claim the calendar breaks.
DAYS = re.compile(r"\b(mondays?|tuesdays?|wednesdays?|thursdays?|yesterday|tonight|last night|this morning|"
                  r"this afternoon|last week)\b")
SWEAR = re.compile(r"\b(fuck\w*|shit\w*|bitch\w*|ass|asses|asshole|bastard|goddamn\w*|dick|prick|cock\w*|"
                   r"pussy|motherf\w*|bullshit)\b")
MILD = re.compile(r"\b(damn\w*|hell|crap\w*|sucks?|sucked|pissed)\b")
# Talking back to your face (PR #227's word scan, extended): only a brash
# player says these, and never in high school.
BACKTALK = re.compile(r"((^|[.!?]\s)whatever[.!?]|"
                      r"(^|[.!?]\s|\b(sure|okay|ok|fine|alright|eh|cool|yeah|bro|haha)[,.!]?\s)whatever you say\b|"
                      r"\b(save it|are we done|we done here|good luck replacing me|sit me,? then|(this|that)( had)? better be good|"
                      r"that's cute|spare me|is that all\?|waste of my time|wasting my time|i don't need this|"
                      r"put my name|get to the point|not my problem|that's your problem|you don't get to|"
                      r"who do you think)\b)")
TALK_BACK_PARTS = ('talk.', 'rules.react.', 'takeover.meeting.react_', 'takeover.leader.')
# A player denying or admitting something about his own conduct (talk facts,
# lane L2): the bank is fact-free, so "I've done nothing wrong" would reach a
# man with a fresh arrest and "I know I messed up" a man who did nothing.
# The blatant forms are dropped from a run (a new part that must say one gets
# a part card that says what is true, and a fact check). The lines already in
# the bank carry blameless=yes or at_fault=yes and wait for the fact sheet
# (voice.rs, CLAIM_FACTS); a writer never uses those two.
CLAIM_FACTS = ('blameless', 'at_fault')
# Lines held on purpose: nothing sets these (saw_chart: a player reading
# the depth chart, which he never sees; promises lane P1).
HELD_FACTS = CLAIM_FACTS + ('saw_chart',)
CLAIM = re.compile(r"(?<![a-z])(?<!if )(?<!did )(?<!whether )(?<!in case )(?<!hope )(?<!because )(?<!glad )("
                   r"i haven't done anything wrong|i've done nothing wrong|i didn't do anything wrong|"
                   r"i did nothing wrong|i'm not in trouble|i'm clean|nothing to hide|it wasn't me|that wasn't me|"
                   r"it's not my fault|that's not my fault|i didn't do it|i'm innocent|"
                   r"i messed up|i screwed up|i blew it|my fault|i was wrong|i let you down|"
                   r"i let the team down|i crossed a line|i take the blame|i'm to blame)(?![a-z])(?! about)(?! if)")
CLAIM_PARTS = ('talk.',)
# Sir or ma'am to your face follows your he or she: {coach_sir}. A line about
# saying sir ("taught to say yes sir") can't be written that way.
SIR = re.compile(r"\b(?:[Ss]ir|[Mm]a'am)\b")
SIR_SAID = re.compile(r"\b(say|says|said|saying|call|calls|called)\b[^.!?]{0,25}\b(sir|ma'am)\b", re.I)
# A name fact read as a team: never "the {them}", "a {team}", or a verb
# whose number changes with the level (13f).
TEAMISH = ('team', 'them', 'us', 'rival', 'club')
SING = r"(is|was|has|does|isn't|wasn't|hasn't|doesn't|looks|plays|seems|comes|gets|goes|wants|needs|knows|keeps|" \
       r"runs|makes|thinks)"
PLUR = r"(are|were|have|do|aren't|weren't|haven't|don't|look|play|seem|come|get|go|want|need|know|keep|run|" \
       r"make|think)"
OBJ = {'to', 'against', 'over', 'beat', 'beating', 'at', 'with', 'from', 'for', 'by', 'of', 'vs', 'versus', 'play',
       'playing', 'face', 'facing', 'about', 'like', 'than', 'lose', 'into', 'past', 'watch', 'see', 'let', 'make',
       'help', 'hear', 'and', 'or', 'that', 'what', 'how', 'think', 'because'}

PAIRS = [('hot', 'calm'), ('young', 'vet'), ('young', 'old'), ('ego', 'humble'), ('pro', 'loose'),
         ('shaky', 'steady'), ('warm', 'cold'), ('blunt', 'smooth'), ('impatient', 'patient'), ('straight', 'slick'),
         ('big', 'small'), ('offense', 'defense'), ('offense', 'special'), ('defense', 'special')]
SIDE = {'qb': 'offense', 'rb': 'offense', 'wr': 'offense', 'te': 'offense', 'ol': 'offense', 'dl': 'defense',
        'lb': 'defense', 'db': 'defense', 'kicker': 'special', 'snapper': 'special'}
GROUPS = {'skill': {'rb', 'wr', 'te'}, 'cover': {'lb', 'db'}, 'rusher': {'dl'}, 'lineman': {'ol', 'dl'}}
SCHOOL_STAGE = {'freshman', 'sophomore', 'junior', 'senior'}
COLLEGE_STAGE = {'redshirt', 'transfer', 'walkon'}
NFL_STAGE = {'rookie', 'undrafted', 'firstround', 'psquad'}

FREQ = collections.Counter()

# A part's key phrase echoed in line after line ("money's tight, Coach"): the recruiting talks' situation parts
# cap each pair of neighbouring content words at 5% of the part.
PHRASE_SHARE = 20
STOP = set("""i you he she it we they me my your his her our their the a an and or but so if of to in on at for
with from by as is am are was were be been do does did have has had not no yes just that this these those there
here what who how why when then than too very really still even also all any some one can could would will should
coach don't i'm i'll i've you're it's that's we're we've he's she's they're can't won't isn't aren't wasn't didn't
doesn't about up out into over more much many get got going go like know think want need let us him them make
""".split())


def content_pairs(text):
    ws = [w for w in re.findall(r"[a-z0-9']+", re.sub(r'\[[^\]]*\]|\{[^}]*\}', ' ', text.lower())) if w not in STOP]
    return {f'{a} {b}' for a, b in zip(ws, ws[1:])}


def words_of(text):
    """The words as said: the first wording of each choice, a fact as one word."""
    t = re.sub(r'\[([^|\]]*)[^\]]*\]', r'\1', text)
    t = re.sub(r'\{[^}]*\}', 'X', t)
    return re.findall(r"[A-Za-z0-9']+", t)


def start_of(text):
    return ' '.join(B.norm(text).split()[:2])


def prefix(ts):
    order = sorted(ts, key=lambda w: (FREQ[w], w))
    return order[:len(order) - math.ceil(NEAR * len(order)) + 1]


class Part:
    """What a part already holds: its words, a near-repeat index, openings."""

    def __init__(self, key, versions):
        self.key = key
        self.norms = set()
        self.sets = []
        self.index = collections.defaultdict(list)
        self.starts = collections.Counter()
        self.pairs = collections.Counter()
        self.new = 0
        self.mild = 0
        for text in versions:
            self.keep(text, new=False)

    def keep(self, text, new=True):
        self.norms.add(B.norm(text))
        ts = B.toks(text)
        if len(ts) >= 4:
            i = len(self.sets)
            self.sets.append((ts, text))
            for w in prefix(ts):
                self.index[w].append(i)
        self.starts[start_of(text)] += 1
        self.pairs.update(content_pairs(text))
        if new:
            self.new += 1
            if MILD.search(text.lower()):
                self.mild += 1

    def near(self, text):
        ts = B.toks(text)
        if len(ts) < 4:
            return None
        seen = set()
        for w in prefix(ts):
            for i in self.index.get(w, ()):
                if i in seen:
                    continue
                seen.add(i)
                os_, ot = self.sets[i]
                if len(ts & os_) / len(ts | os_) >= NEAR:
                    return ot
        return None


class Checker:
    def __init__(self, keys=None, pool=None, target=None, repeats=True, held_ok=False):
        """`pool`: {key: [lines kept so far]}; `target`: {key: new versions
        the part is to get} (for the opening share). `held_ok`: the bank's own
        lines may carry blameless= and at_fault= (`--bank`); a run's may not."""
        self.held_ok = held_ok
        old = B.bank(skip={f for f in B.os.listdir(B.LINES) if f.endswith('_run.txt')})
        for vs in old.values():
            for text, _, _ in vs:
                FREQ.update(B.toks(text))
        self.repeats = repeats
        self.target = target or {}
        self.parts = {}
        for k in keys or PARTS:
            versions = [t for t, _, _ in old.get(k, [])]
            self.parts[k] = Part(k, versions)
            for line in (pool or {}).get(k, []):
                self.parts[k].keep(B.split(line)[0])
        self.old_size = {k: len(old.get(k, [])) for k in self.parts}

    # -- one line ----------------------------------------------------------
    def check(self, key, raw):
        """-> (line, reasons): the line as kept (an era tag may be added),
        or None and why it was dropped. A kept line is remembered."""
        p = PARTS[key]
        why = []
        line = clean(raw)
        if not line:
            return None, ['empty']
        text, tags = B.split(line)
        # In a part that may name you, sir or ma'am is written {coach_sir};
        # anywhere else the line is dropped below.
        fixed = 'coach_sir' in p.facts and bool(SIR.search(text)) and not SIR_SAID.search(text)
        if fixed:
            text = SIR.sub('{coach_sir}', text)
        low = text.lower()
        need = [t for t in tags if not t.startswith('!') and '=' not in t]
        nots = [t[1:] for t in tags if t.startswith('!') and '=' not in t]
        when = [(t.lstrip('!').split('=', 1), not t.startswith('!')) for t in tags if '=' in t]
        nd = set(need)
        # The written form.
        if re.search(r'["“”]', text):
            why.append('quote marks')
        if re.search(r'[—–]| - ', text):
            why.append('dash')
        if '~' in text or '@' in text or '#' in text or '\t' in text:
            why.append('stray symbol')
        if re.search(r'[^\x20-\x7e]', text):
            why.append('not plain text')
        depth, groups = 0, 0
        for c in text:
            if c in '[{':
                depth += 1
                groups += c == '['
            elif c in ']}':
                depth -= 1
            elif c == '|' and depth == 0:
                why.append('choice bar outside brackets')
                break
            if depth not in (0, 1):
                why.append('nested or unbalanced brackets')
                break
        if depth != 0:
            why.append('bracket left open')
        if groups > 2:
            why.append('more than two choices')
        if re.search(r'\[[^\]|]*\]', text):
            why.append('a choice with one wording')
        # The prompt asks for the part's length; the checker only stops the
        # extremes.
        n = len(words_of(text))
        if n < 1 or n > p.words[1] + 5:
            why.append(f'{n} words (part takes {p.words[0]}-{p.words[1]})')
        if len(text) > 240:
            why.append('too long')
        if not re.search(r'[.!?]$', text.rstrip(')')):
            why.append('no end punctuation')
        # Tags.
        if len(need) + len(nots) > 3:
            why.append('more than 3 tags')
        if len(when) > 2:
            why.append('more than 2 when-checks')
        for t in need + nots:
            if t not in B.TAGS:
                why.append(f'unknown tag {t}')
            elif t not in p.tags:
                why.append(f'tag {t} never fits this speaker')
        for (k, v), _ in when:
            if k in HELD_FACTS:
                if not self.held_ok:
                    why.append(f"{k}= is the fact sheet's, not a writer's tag")
                continue
            if k not in p.when:
                why.append(f'{k}= is never set here')
            elif v not in p.when[k]:
                why.append(f'{k}={v}: no such value')
        if set(need) & set(nots):
            why.append('a tag and its opposite')
        why += clashes(nd)
        if p.family == 'recruittalks':
            if COACH_NAME.search(text):
                why.append('writes a coach\'s name (you are {coach})')
            for t in nd & ECHO:
                if re.search(r'\b' + t[:5], low):
                    why.append(f'writes the tag word "{t}" into the line')
            if TIME_OF_DAY.search(low):
                why.append('names a time of day or a day')
            if KID_COUNT.search(low):
                why.append('counts his sons, boys or brothers')
        if p.who == 'recruit':
            why += recruit_side(key, nd, text, low)
        # Facts.
        found = re.findall(r'\{([^}]*)\}', text)
        for f in found:
            if f not in p.facts:
                why.append(f'fact {{{f}}} not given here')
            elif f not in B.FACTS and f not in B.PRONOUNS:
                why.append(f'fact {{{f}}} unknown to the bank')
        why += team_words(low, nd, p)
        if re.search(r'\b(a|an) \{pos\}|\{pos\}s\b', low):
            why.append('{pos} takes no a, an or s')
        if 'club' in found and not re.search(r'\bthe \{club\}', low):
            why.append('{club} reads "the {club}"')
        if p.key.startswith('gameday.captain.') or p.key == 'gameday.captain':
            pass
        # Words that place a line: its level tag, or the level its stage or
        # role tags imply.
        lv = (nd & LEVEL) or implied(key, nd)
        reach = set(p.levels or LEVEL)
        said_at = lv or reach
        for rx, fits in LEVEL_WORDS:
            m = rx.search(low)
            # A recruit's side talks across high school and college (offers, visits, scholarships, the portal):
            # only the NFL's words are out of place.
            if m and p.family == 'recruittalks' and fits & {H, C}:
                continue
            if m and not said_at <= fits:
                why.append(f'"{m.group(0)}" fits {"/".join(sorted(fits))} only: tag the level')
        if lv and not lv <= reach:
            why.append('level never reached here')
        imp = implied(key, nd)
        if nd & LEVEL and imp and not nd & LEVEL <= imp:
            why.append('a role or stage tag off its level')
        # Family, days, swearing.
        if SPOUSE.search(low):
            why.append('names a spouse or partner')
        for m in KIN.finditer(low):
            if not kin_ok(key, nd, m.group(0)):
                why.append(f'names family ({m.group(0)})')
                break
        m = DAYS.search(low)
        if m:
            d = m.group(0)
            ok = (d == 'tonight' and key.startswith('gameday.') and said_at == {H}) or \
                 (d == 'last week' and 'last' in p.when and any(k == 'last' for (k, _), _ in when))
            if not ok:
                why.append(f'names a day or time ({d})')
        if SWEAR.search(low):
            why.append('swearing')
        if SIR.search(text):
            why.append("sir or ma'am outright (only {coach_sir}, in a part that names you)")
        # Talking back.
        if key.startswith(TALK_BACK_PARTS) and BACKTALK.search(low):
            if 'brash' not in B.TAGS:
                why.append('talks back (no brash tag in the bank yet)')
            elif 'brash' not in nd:
                why.append('talks back without brash')
        if 'brash' in nd and (lv & {'hs'} or (p.levels and p.levels <= {'hs'})):
            why.append('brash is never a high school player')
        if key.startswith(CLAIM_PARTS) and CLAIM.search(low) and not any(k in CLAIM_FACTS for (k, v), _ in when):
            why.append('denies or admits his own conduct (only the fact sheet may say it)')
        if p.who == 'you' and re.search(r"\b(i|i'm|i've|my) (wife|husband)\b", low):
            why.append('you have no spouse in the bank')
        # Era words: tagged as lane 13 did (every speaker you meet from 2026
        # has them, so the tag only keeps the record).
        add = []
        if p.who != 'agent':
            if NIL.search(low) and 'nil' not in nd:
                add.append('nil')
            elif MODERN.search(low) and not nd & ERA:
                add.append('modern')
        if add and len(need) + len(nots) + len(add) > 3:
            why.append('needs an era tag and has no room')
        if why:
            return None, why
        st = self.parts[key]
        if self.repeats:
            nt = B.norm(text)
            if nt in st.norms:
                return None, ['repeat']
            o = st.near(text)
            if o:
                return None, [f'near repeat of: {o}']
            size = self.old_size.get(key, 0) + self.target.get(key, 0)
            cap = max(3, math.ceil(size / START_SHARE))
            s = start_of(text)
            if st.starts[s] >= cap:
                return None, [f'opening "{s}" used {st.starts[s]} times (cap {cap})']
            if p.family == 'recruittalks' and p.phrase_cap:
                pcap = max(6, math.ceil(size / PHRASE_SHARE))
                for pair in content_pairs(text):
                    if st.pairs[pair] >= pcap:
                        return None, [f'phrase "{pair}" already in {st.pairs[pair]} lines (cap {pcap})']
            if MILD.search(low) and st.mild + 1 > max(1, (self.target.get(key, 0) or 20) // MILD_SHARE):
                return None, ['mild swearing over 1 in 20']
        st.keep(text)
        if add or fixed:
            line = text + (' ~ ' + ' '.join(tags + add) if tags or add else '')
        return line, []


def clean(raw):
    """A reply line as written: list marks, numbers and wrapping dropped."""
    s = raw.strip().replace('’', "'").replace('‘', "'").replace('…', '...')
    s = re.sub(r'^(\d+[.)]\s+|[-*•]\s+)', '', s)
    if not s or s.startswith(('#', '@', '```')) or s.lower().startswith(('here are', 'here is', 'note:')):
        return ''
    if s.startswith('`') and s.endswith('`'):
        s = s.strip('`').strip()
    if ' ~ ' in s:
        # Tags written with commas ("~ college, walkon") are the same tags.
        t, g = s.rsplit(' ~ ', 1)
        s = t.rstrip() + ' ~ ' + ' '.join(g.replace(',', ' ').replace(';', ' ').split())
    return s


def clashes(nd):
    out = []
    if len(nd & LEVEL) > 1:
        out.append('two levels')
    for a, b in PAIRS:
        if a in nd and b in nd:
            out.append(f'{a} and {b}')
    r = nd & REGION
    if len(r - {'south'}) > 1 or ('south' in r and r & {'midwest', 'northeast', 'west'}):
        out.append('two regions')
    spots = nd & SPOT
    if len(spots) > 1:
        out.append('two positions')
    for s in spots:
        if SIDE[s] in {'offense', 'defense', 'special'} - {SIDE[s]} & nd:
            out.append(f'{s} on the wrong side')
        for g, members in GROUPS.items():
            if g in nd and s not in members:
                out.append(f'{s} is not {g}')
    st = nd & STAGE
    if len(st & SCHOOL_STAGE) > 1:
        out.append('two class years')
    if st & SCHOOL_STAGE - {'freshman', 'senior'} and nd & {'college', 'nfl'}:
        out.append('a high school class year off high school')
    if st & COLLEGE_STAGE and nd & {'hs', 'nfl'}:
        out.append('a college stage off college')
    if st & NFL_STAGE and nd & {'hs', 'college'}:
        out.append('an NFL stage off the NFL')
    if st & SCHOOL_STAGE and st & NFL_STAGE:
        out.append('a class year and an NFL stage')
    if 'young' in nd and nd & {'vet'}:
        out.append('young and vet')
    if 'brash' in nd and 'hs' in nd:
        out.append('brash in high school')
    if nd & {'pre2000', 'pre2010'}:
        out.append('an old era (never reaches you)')
    return out


FAMILY_SIDE = {'parent', 'grandparent', 'handler', 'himself', 'loud', 'online', 'absent'}
DECIDERS = {'parent', 'grandparent', 'handler', 'himself'}
STYLES = {'loud', 'online', 'absent'}
TRANSFER_SIDE = {'college', 'freshman', 'senior', 'redshirt', 'transfer', 'walkon'} | TRAITS | SPOT | GROUP
# Words only a transfer in the portal has.
TRANSFER_WORDS = re.compile(r"\b(the portal|my old (school|coach|coaches|team|staff|program)|transferred|"
                            r"transfer(ring)? credits?|my eligibility|my last (school|coach)|where i left)\b")
SELF_WORDS = re.compile(r"\b(i'?m (seventeen|17|a (junior|senior))|my (junior|senior) (year|season)|"
                        r"my (parents?|mom|dad|folks) (said|think|want))\b")


# The example name the briefs show ("Coach Reed") must never be written into a line: you are {coach}.
COACH_NAME = re.compile(r"\bCoach [A-Z][a-z]+|\bReed\b")
# What the top-up's checker kept flagging (2026-11-13): a greeting by time of day, a day or a frequency, and how many
# sons a parent has. None of it is a stored fact.
TIME_OF_DAY = re.compile(r"\b(good )?(morning|afternoon|evening|tonight)\b|\bevery (day|night|friday|saturday|"
                         r"sunday|game|morning|weekend)\b|\bfor years\b|\blast (week|night)\b")
KID_COUNT = re.compile(r"\b(both|two|three|my other|one more|second|other) (of )?(my |our |his )?(sons?|boys?|kids?|"
                       r"grandsons?|brothers?)\b|\b(raised|have|had) (two|three|four) (boys|sons|kids)\b")
WE = re.compile(r"\b(we|we're|we've|we'll|we'd|us|our|ours)\b")
# A trait tag's own word in the line ("I'm not greedy", "I'm a competitor"): the tag is the reason he can say
# it, not the thing he says.
ECHO = {'ego', 'humble', 'leader', 'quiet', 'loyal', 'greedy', 'loose', 'shaky', 'steady', 'competitor'}


# Parts about the recruit's own life: a line there that says "I", "my" or "me" about him is true of only one side
# (a transfer or the recruit himself says it, a parent says "my son"), so it must carry that side's tag. An untagged
# line in these parts is neutral: a question, a reaction, a statement about you or about "the kid".
OWN_LIFE = ('recruit.bg.', 'recruit.committed.', 'recruit.other.', 'recruit.want.', 'recruit.visit.')
FIRST = re.compile(r"\b(i|i'm|i've|i'd|i'll|my|mine|myself)\b|\b(to|for|about|with|at|from|on) me\b|\b\w+ed me\b")


def recruit_side(key, nd, text, low):
    """The union speaker of the recruiting talks: a family side and a transfer side that never meet on a line."""
    out = []
    if nd & FAMILY_SIDE and nd & TRANSFER_SIDE:
        out.append('family-side and transfer-side tags on one line')
    if len(nd & DECIDERS) > 1:
        out.append('two deciders')
    if len(nd & STYLES) > 1:
        out.append('two parent styles')
    if nd & STYLES and 'himself' in nd:
        out.append('a parent style on the recruit himself')
    if not nd & (FAMILY_SIDE | TRANSFER_SIDE) and WE.search(low):
        out.append('we, us or our on a line both sides say')
    if key.startswith(OWN_LIFE) and not nd & (FAMILY_SIDE | TRANSFER_SIDE) and FIRST.search(low):
        out.append('I, my or me on a line both sides say, in a part about his own life')
    if '{name}' in text and not nd & {'parent', 'grandparent', 'handler'}:
        out.append('{name} is said by a parent, grandparent or handler only')
    if TRANSFER_WORDS.search(low) and 'college' not in nd:
        out.append('a transfer\'s words on a line not tagged college')
    if SELF_WORDS.search(low) and not nd & {'himself'}:
        out.append('the recruit himself talking on a line not tagged himself')
    return out


def implied(key, nd):
    """The levels a line's stage or role tags put it at."""
    out = set(LEVEL)
    if nd & NFL_STAGE:
        out &= {N}
    if nd & COLLEGE_STAGE:
        out &= {C}
    if nd & {'sophomore', 'junior'}:
        out &= {H}
    if nd & {'freshman', 'senior'}:
        out &= {H, C}
    roles = {'principal': {H}, 'superintendent': {H}, 'president': {C}, 'owner': {N}, 'gm': {N}, 'kin': {N},
             'booster': {H, C}, 'ad': {C} if key.startswith('boss.') else {H, C}}
    for r, lv in roles.items():
        if r in nd:
            out &= lv
    return out if out != set(LEVEL) else set()


def kin_ok(key, nd, words):
    # Coaches, bosses and boosters call the players kids.
    if re.search(r"\b(our|my|your|their) kids?\b", words) and PARTS[key].who in ('coach', 'cast', 'you'):
        return True
    for pre, rules in KIN_OK.items():
        if key.startswith(pre):
            for tag, rx in rules.items():
                if (tag is None or tag in nd) and rx.search(words):
                    return True
    return False


def team_words(low, nd, p):
    out = []
    lv = nd & LEVEL
    if re.search(r'\b(a|an) \{old_school\}', low):
        out.append('"a {old_school}" (a or an depends on the name)')
    nfl_only = 'nfl' in lv or (p.levels is not None and p.levels <= {'nfl'})
    for f in TEAMISH:
        if f == 'club':
            continue
        if re.search(r'\b(a|an) \{' + f + r'\}', low):
            out.append(f'"a {{{f}}}" (a or an depends on the name)')
        # "the Buffalo Bills" reads right, "the Alabama" doesn't: "the {them}"
        # as a name on its own is the NFL's ("the {them} game" fits all).
        if re.search(r'\bthe \{' + f + r'\}(?=\s*([.,!?;:]|$)|\s+(is|are|was|were|has|have|will|would|can|'
                     r'could|should|did|does|do|and|or|but|to|in|on|at|this|that|who|if|when)\b)', low) \
                and not nfl_only:
            out.append(f'"the {{{f}}}" reads "the Alabama" off the NFL: tag nfl or drop "the"')
        for m in re.finditer(r"(\b[a-z']+\s+)?\{" + f + r"\}('s)? " + SING + r"\b", low):
            if (m.group(1) or '').strip() in OBJ:
                continue
            if f in ('team', 'them', 'us') and not lv & {'hs', 'college'} and (p.levels is None or 'nfl' in p.levels):
                out.append(f'"{{{f}}} {m.group(3)}" reads "the Bills {m.group(3)}" at the NFL')
        for m in re.finditer(r"(\b[a-z']+\s+)?\{" + f + r"\}('s)? " + PLUR + r"\b", low):
            if (m.group(1) or '').strip() in OBJ:
                continue
            if f in ('team', 'them', 'us') and 'nfl' not in lv and not (p.levels and p.levels <= {'nfl'}):
                out.append(f'"{{{f}}} {m.group(3)}" reads "Alabama {m.group(3)}" off the NFL')
    return out


def main():
    args = sys.argv[1:]
    if args == ['--bank']:
        ck = Checker(repeats=False, held_ok=True)
        bad = collections.Counter()
        tot = collections.Counter()
        why = collections.Counter()
        b = B.bank()
        for k in PARTS:
            for text, tags, _ in b.get(k, []):
                line = text + (' ~ ' + ' '.join(tags) if tags else '')
                kept, w = ck.check(k, line)
                tot[PARTS[k].family] += 1
                if kept is None:
                    bad[PARTS[k].family] += 1
                    for x in w:
                        why[re.sub(r'"[^"]*"|\{[^}]*\}|\([^)]*\)|\d+', '_', x)] += 1
        for f in tot:
            print(f'{f}: {bad[f]} of {tot[f]} kept lines would be dropped ({100 * bad[f] / tot[f]:.1f}%)')
        for x, c in why.most_common(25):
            print(f'  {c:6} {x}')
        return
    ck = Checker()
    kept = collections.Counter()
    drops = collections.Counter()
    for path in args:
        for key, text, tags, n in B.parse(path):
            if key not in PARTS:
                print(f'ERROR {path}:{n} not a run part [{key}]')
                continue
            line = text + (' ~ ' + ' '.join(tags) if tags else '')
            got, why = ck.check(key, line)
            if got is None:
                drops[key] += 1
                print(f'DROP  {path}:{n} [{key}] {line}  <- {"; ".join(why)}')
            else:
                kept[key] += 1
    for k in sorted(set(kept) | set(drops)):
        print(f'{k}: {kept[k]} kept, {drops[k]} dropped')


if __name__ == '__main__':
    main()
