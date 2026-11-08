"""The line bank as it stands: its files, its tag and fact words, and the
helpers every line tool shares (parsing, normalizing, near-repeat tokens).
Built from lane 13's tools (bank.py, check13f.py)."""
import glob
import os
import re

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.abspath(os.path.join(HERE, '..', '..'))
ENGINE = os.path.join(ROOT, 'crates', 'afm-engine')
LINES = os.path.join(ENGINE, 'data', 'lines')


def concat_words(name):
    """The words of one `concat!` list in tests/lines.rs (TAGS or FACTS)."""
    src = open(os.path.join(ENGINE, 'src', 'tests', 'lines.rs'), encoding='utf-8').read()
    m = re.search(r'const ' + name + r': &str = concat!\((.*?)\);', src, re.S)
    body = re.sub(r'//[^\n]*', '', m.group(1))
    return set(' '.join(re.findall(r'"([^"]*)"', body)).split())


TAGS = concat_words('TAGS')
FACTS = concat_words('FACTS')
PRONOUNS = {f'{w}_{p}' for w in ('coach', 'lead') for p in ('he', 'him', 'his', 'himself', 'man', 'sir')}


def split(line):
    """'words ~ tags' -> (words, [tags])."""
    if ' ~ ' in line:
        t, g = line.rsplit(' ~ ', 1)
        return t.strip(), g.split()
    return line.strip(), []


def norm(text):
    t = text.lower().replace('{', '').replace('}', '')
    t = re.sub(r'[^a-z0-9 \[\]|]', ' ', t)
    return ' '.join(t.split())


def toks(text):
    """The word set two versions are compared on (choices dropped, facts as X)."""
    t = re.sub(r'\[[^\]]*\]', ' ', text.lower())
    t = re.sub(r'\{[^}]*\}', ' X ', t)
    return set(re.findall(r"[a-z0-9']+", t))


def parse(path):
    """Yield (key, words, tags, line number) for every version in a file."""
    key = None
    for n, raw in enumerate(open(path, encoding='utf-8'), 1):
        line = raw.strip()
        if not line or line.startswith('#'):
            continue
        if line.startswith('@key '):
            key = line[5:].strip()
            continue
        if line.startswith('@'):
            continue
        if key:
            text, tags = split(line)
            yield key, text, tags, n


def bank(skip=()):
    """Every version in the bank: {key: [(words, tags, file)]}. `skip` leaves
    files out by name (the run's own files when it re-reads its pool)."""
    out = {}
    for f in sorted(glob.glob(os.path.join(LINES, '*.txt'))):
        name = os.path.basename(f)
        if name in skip:
            continue
        for key, text, tags, _ in parse(f):
            out.setdefault(key, []).append((text, tags, name))
    return out
