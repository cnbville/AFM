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
from parts import ERA, LEVEL, PARTS, REGION, SPOT, STAGE, TRAITS

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
                 r"grandsons?|granddaughters?|uncle|aunt|auntie|cousins?)\b")
# Where a family may name its own (the speaker's tag, the words it allows).
KIN_OK = {
    'parents.playing_time': {None: re.compile(r"\b(my|our) (sons?|boy|kid)\b")},
    'homevisit.': {'parent': re.compile(r"\b(my|our) (sons?|boy|kid|baby)\b"),
                   'grandparent': re.compile(r"\b(my|our) (grandsons?|grandbaby|boy|baby)\b")},
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
BACKTALK = re.compile(r"((^|[.!?]\s)whatever[.!?]|\b(save it|are we done|we done here|good luck replacing me|"
                      r"that's cute|spare me|is that all\?|waste of my time|wasting my time|i don't need this|"
                      r"put my name|get to the point|not my problem|that's your problem|you don't get to|"
                      r"who do you think)\b)")
TALK_BACK_PARTS = ('talk.', 'rules.react.', 'takeover.meeting.react_', 'takeover.leader.')
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
    def __init__(self, keys=None, pool=None, target=None, repeats=True):
        """`pool`: {key: [lines kept so far]}; `target`: {key: new versions
        the part is to get} (for the opening share)."""
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
            if k not in p.when:
                why.append(f'{k}= is never set here')
            elif v not in p.when[k]:
                why.append(f'{k}={v}: no such value')
        if set(need) & set(nots):
            why.append('a tag and its opposite')
        why += clashes(nd)
        # Facts.
        found = re.findall(r'\{([^}]*)\}', text)
        for f in found:
            if f not in p.facts:
                why.append(f'fact {{{f}}} not given here')
            elif f not in B.FACTS and f not in B.PRONOUNS:
                why.append(f'fact {{{f}}} unknown to the bank')
        why += team_words(low, nd, p)
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
        m = KIN.search(low)
        if m and not kin_ok(key, nd, m.group(0)):
            why.append(f'names family ({m.group(0)})')
        m = DAYS.search(low)
        if m:
            d = m.group(0)
            ok = (d == 'tonight' and key.startswith('gameday.') and said_at == {H}) or \
                 (d == 'last week' and 'last' in p.when and any(k == 'last' for (k, _), _ in when))
            if not ok:
                why.append(f'names a day or time ({d})')
        if SWEAR.search(low):
            why.append('swearing')
        # Talking back.
        if key.startswith(TALK_BACK_PARTS) and BACKTALK.search(low):
            if 'brash' not in B.TAGS:
                why.append('talks back (no brash tag in the bank yet)')
            elif 'brash' not in nd:
                why.append('talks back without brash')
        if 'brash' in nd and (lv & {'hs'} or (p.levels and p.levels <= {'hs'})):
            why.append('brash is never a high school player')
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
            if MILD.search(low) and st.mild + 1 > max(1, (self.target.get(key, 0) or 20) // MILD_SHARE):
                return None, ['mild swearing over 1 in 20']
        st.keep(text)
        if add:
            line = text + ' ~ ' + ' '.join(tags + add)
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
        ck = Checker(repeats=False)
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
