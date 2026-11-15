"""Reads the game's own wordings for Cain's side of the talks out of the Rust source (lane L12), so the run's
parts are exactly the keys the code asks, each with today's wording(s), the facts its branches use and the states
that pick between them. Nothing here is a design choice: it only lists what the code does."""
import os
import re

from bank import ENGINE

FOLK = os.path.join(ENGINE, 'src', 'world', 'folk')


def read(*p):
    return open(os.path.join(FOLK, *p), encoding='utf-8').read()


def kind_keys():
    """{Name: key} from the fact sheet's catalogue."""
    src = read('claims', 'kinds.rs')
    return dict(re.findall(r'^\s+([A-Z][A-Za-z]+) "([a-z_]+)" ', src, re.M))


def calls(text, opener):
    """Every `opener(...)` call's inner text, balanced."""
    out, i = [], 0
    while True:
        i = text.find(opener, i)
        if i < 0:
            return out
        j, depth = i + len(opener), 1
        while depth and j < len(text):
            c = text[j]
            if c == '"':
                j += 1
                while text[j] != '"':
                    j += 2 if text[j] == '\\' else 1
            depth += (c == '(') - (c == ')')
            j += 1
        out.append(text[i + len(opener):j - 1])
        i = j


def strings(expr):
    """The words inside an expression's string literals, `{}` kept."""
    return [s for s in re.findall(r'"((?:[^"\\]|\\.)*)"', expr)]


def facts_of(text):
    holes = sorted(set(re.findall(r'\b(?:g|n|opt)\("(\w+)"\)', text)) | set(re.findall(r'\bget\("(\w+)"\)', text)))
    states = {}
    for k, v in re.findall(r'\.is\("(\w+)", "(\w+)"\)', text):
        states.setdefault(k, set()).add(v)
    cur = None
    for m in re.finditer(r'check\("(\w+)"\)|Some\("(\w+)"\)', text):
        if m.group(1):
            cur = m.group(1)
            states.setdefault(cur, set())
        elif cur:
            states[cur].add(m.group(2))
    return holes, {k: sorted(v) for k, v in states.items()}


def arms(body, names):
    """Split a `match p.kind {` body into (kind keys, text) arms."""
    out, cur, head = [], [], None
    for line in body.split('\n'):
        m = re.match(r'^        (K::[A-Za-z]+(?: \| K::[A-Za-z]+)*|k if k\.is_life\(\)|_) =>', line)
        if m:
            if head is not None:
                out.append((head, '\n'.join(cur)))
            head, cur = m.group(1), [line]
        elif head is not None:
            cur.append(line)
    if head is not None:
        out.append((head, '\n'.join(cur)))
    res = []
    life = ['SickParent', 'Death', 'Divorce', 'MoneyLost', 'Homesick']
    for head, text in out:
        if head == '_':
            continue
        if head.startswith('k if'):
            ks = life
        else:
            ks = [h.replace('K::', '') for h in head.split(' | ')]
        res.append(([names[k] for k in ks if k in names], text))
    return res


def point_moves():
    """{(kind key, move): [(tone, today's strings)]} with the arm's facts and states."""
    src = read('talk_points.rs')
    names = kind_keys()
    body = src[src.index('pub fn moves('):src.index('// One option a move.')]
    body = body[body.index('match p.kind {') + 14:]
    parts = {}
    for keys, text in arms(body, names):
        holes, states = facts_of(text)
        for c in calls(text, 'add('):
            m = re.match(r'\s*Move::(\w+),\s*Tone::(\w+),(.*)', c, re.S)
            if not m:
                continue
            mv, tone, rest = m.groups()
            strs = [s for s in strings(rest)]
            # the last string is the hint
            strs = strs[:-1] if len(strs) > 1 else strs
            for k in keys:
                e = parts.setdefault((k, mv.lower()), {'tone': tone, 'today': [], 'holes': set(), 'states': {}})
                e['today'] += [s for s in strs if s not in e['today']]
                e['holes'] |= set(holes)
                for a, b in states.items():
                    e['states'].setdefault(a, set()).update(b)
    return parts


def answers():
    src = read('talk_points.rs')
    body = src[src.index('pub fn question('):src.index('/// What an answer does')]
    out = {}
    for blk in re.split(r'\n        (?=K::|_ =>)', body):
        m = re.match(r'(K::[A-Za-z]+(?: \| K::[A-Za-z]+)*)', blk)
        if not m and 'K::YourJob' not in blk:
            continue
        key = None
        km = re.search(r'key: (?:p\.kind\.key\(\)|K::(\w+)\.key\(\))', blk)
        names = kind_keys()
        if km and km.group(1):
            key = names[km.group(1)]
        elif m:
            key = names[m.group(1).replace('K::', '').split(' | ')[0]]
        if not key:
            continue
        holes, states = facts_of(blk)
        for c in calls(blk, '(Answer::'):
            pass
        for am in re.finditer(r'Answer::(\w+),(.*?)(?=\n\s*\(Answer::|\n\s*\(\s*\n?\s*Answer::|\],)', blk, re.S):
            pass
        for a in ('Straight', 'Soften', 'Promise', 'Dodge'):
            idx = [mm.start() for mm in re.finditer(r'Answer::' + a + r',', blk)]
            for st in idx:
                seg = blk[st:st + 700]
                seg = re.split(r'\n\s*\(\s*Answer::|\n\s*\),\n\s*\(|\],', seg)[0]
                ss = strings(seg)
                ss = ss[:-1] if len(ss) > 1 else ss
                e = out.setdefault((key, a.lower()), {'today': [], 'holes': set(), 'states': {}})
                e['today'] += [s for s in ss if s not in e['today']]
                e['holes'] |= set(holes)
                for k2, v2 in states.items():
                    e['states'].setdefault(k2, set()).update(v2)
    return out


def subject_moves():
    src = read('talks.rs')
    body = src[src.index('fn add_talk_picks('):]
    body = body[:body.index('\n}\n')]
    out = {}
    cur = None
    for line in body.split('\n'):
        m = re.match(r'^        Subject::([A-Za-z]+(?: \| Subject::[A-Za-z]+)*) =>', line)
        if m:
            cur = [x.lower() for x in m.group(1).replace('Subject::', '').split(' | ')]
        elif re.match(r'^    add\(|^    match subject', line):
            cur = cur if line.startswith('    match') else ['any']
    # split by arm text
    blocks = re.split(r'\n(?=        Subject::|    add\()', body)
    cur = ['any']
    for blk in blocks:
        m = re.match(r'\s*Subject::([A-Za-z]+(?: \| Subject::[A-Za-z]+)*) =>', blk)
        if m:
            cur = [x.lower() for x in m.group(1).replace('Subject::', '').split(' | ')]
        elif blk.startswith('    add('):
            cur = ['any']
        for c in calls(blk, 'add('):
            mm = re.match(r'\s*Move::(\w+),\s*Tone::(\w+),(.*)', c, re.S)
            if not mm:
                continue
            mv, tone, rest = mm.groups()
            ss = strings(rest)
            ss = ss[:-1] if len(ss) > 1 else ss
            for k in cur:
                e = out.setdefault((k, mv.lower()), {'tone': tone, 'today': []})
                e['today'] += [s.strip('\\"') for s in ss if s.strip('\\"') not in e['today']]
    return out


def pitch_kinds():
    src = read('pitch.rs')
    names = kind_keys()
    start = src.index('fn claim_words(')
    body = src[start:src.index('let said = if missing.get()', start)]
    body = body[body.index('let said = match pt.kind {') + 26:]
    out = {}
    blocks = re.split(r'\n        (?=Kind::|_ =>)', body)
    for blk in blocks:
        m = re.match(r'Kind::(\w+)', blk)
        if not m:
            continue
        key = names.get(m.group(1))
        if not key:
            continue
        holes, states = facts_of(blk)
        out[key] = {'today': strings(blk), 'holes': holes, 'states': states}
    return out


if __name__ == '__main__':
    import json
    pm = point_moves()
    print(len(pm), 'point moves')
    for k, v in list(pm.items())[:6]:
        print(k, v)
    an = answers()
    print(len(an), 'answers')
    for k, v in an.items():
        print(k, v)
    sm = subject_moves()
    print(len(sm), 'subject moves')
    for k, v in sm.items():
        print(k, v)
    pk = pitch_kinds()
    print(len(pk), 'pitch kinds')
    for k, v in list(pk.items())[:4]:
        print(k, v)
