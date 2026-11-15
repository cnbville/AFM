#!/usr/bin/env python3
"""Patch notes that write themselves (Cain 2026-11-15).

One entry per Mac app version that really shipped, written the moment its build
publishes. An entry covers every merge that went into that build: everything
on main after the previous shipped build, up to this build's commit. A build
that was cancelled or never published has no entry; its merges land in the next
shipped one, because that build's range starts at the last shipped commit.

The words come from the pull requests. A PR body may carry a section

    ## Patch note
    **TLDR:** one or two plain sentences on what it gives the player.
    - a plain line a player could read (three to five)
    Look at: what to try first.
    Players: no          (only when a player can see nothing new)

Without that section the entry falls back on the PR's own summary, and it is
kept out of the in-game "What's new" page (nobody has checked those words are
plain). `docs/patch-notes/curated.json` holds hand-written replacements for
merged PRs.

Commands (run from anywhere inside the repository; needs git and gh):
  sync                      read the shipped builds, write the notes, the in-game page
                            and the data files (what the patch-notes workflow runs)
  preview --version N --sha S
                            put build N's own entry in the in-game page, in the working
                            tree only (the Mac build runs it, so the app carries its notes)
  check --pr N              warn when a PR that changes the game has no Patch note section

Files: docs/patch-notes.md (generated above the marker, hand-written below it),
docs/patch-notes/builds.json, prs.json, curated.json, app/ui/changelog.js
(generated between its two auto markers).
"""
import argparse
import hashlib
import json
import os
import re
import subprocess
import sys
import time
import urllib.request

MARK = "<!-- older entries below: hand-written, up to 0.2.492 -->"
JS_START = "  // auto:start (written by .github/scripts/patch_notes.py, do not edit by hand)"
JS_END = "  // auto:end"
HEADER = """# AFM patch notes

One entry per Mac app version that shipped, newest first. The entries write themselves
(`.github/scripts/patch_notes.py`, run by `.github/workflows/patch-notes.yml` when a Mac build
publishes): each covers every PR merged into that build, from the words in the PR's
`## Patch note` section or, without one, its summary. A build that never shipped has no entry;
its changes land in the next one. Do not edit above the marker by hand: change the PR's
`## Patch note` section or `docs/patch-notes/curated.json` and run the workflow again.
The app number is 0.2.<Mac app run>. Entries from 0.2.492 and older are hand-written.

"""

DEFAULT_IGNORE = [
    "docs/**", "**/*.md", "tools/**", "legacy-ts/**", "crates/afm-tools/**", "crates/afm-reader/**",
    "crates/afm-reader-app/**", "reader/**", ".github/workflows/reader-app.yml",
    "data/real-world/ab_real.json", "data/real-world/scripts/**",
    ".github/workflows/ci.yml", ".github/workflows/full-tests.yml", ".github/workflows/build-world.yml",
    ".github/scripts/**",
]


def sh(*args, check=True):
    # GitHub answers a 502 now and then (the first live run died on one): try a gh call 4 times.
    for attempt in range(4 if args[0] == "gh" else 1):
        r = subprocess.run(args, capture_output=True, text=True)
        if r.returncode == 0 or not re.search(r"HTTP 5\d\d|timeout|reset", r.stderr):
            break
        time.sleep(3 * (attempt + 1))
    if check and r.returncode != 0:
        raise RuntimeError(f"{' '.join(args)}: {r.stderr.strip()}")
    return r.stdout


ROOT = sh("git", "rev-parse", "--show-toplevel").strip()


def repo_slug():
    env = os.environ.get("GITHUB_REPOSITORY")
    if env:
        return env
    m = re.search(r"github\.com[:/]([^/]+/[^/.]+)", sh("git", "remote", "get-url", "origin").strip())
    return m.group(1) if m else "cnbville/AFM"


REPO = repo_slug()


def path(*p):
    return os.path.join(ROOT, *p)


def gh_json(endpoint, pages=False):
    """One `gh api` call. With pages=True it walks ?page=1,2,... itself (the --paginate flag
    follows links the agent proxy refuses) and returns the lists joined."""
    if not pages:
        return json.loads(sh("gh", "api", endpoint))
    items, page = [], 1
    while True:
        sep = "&" if "?" in endpoint else "?"
        got = json.loads(sh("gh", "api", f"{endpoint}{sep}per_page=100&page={page}"))
        items.extend(got)
        if len(got) < 100:
            return items
        page += 1


def load(name, default):
    try:
        with open(path("docs", "patch-notes", name), encoding="utf-8") as f:
            return json.load(f)
    except FileNotFoundError:
        return default


def save(name, data):
    os.makedirs(path("docs", "patch-notes"), exist_ok=True)
    with open(path("docs", "patch-notes", name), "w", encoding="utf-8") as f:
        json.dump(data, f, indent=1, ensure_ascii=False, sort_keys=True)
        f.write("\n")


# ---------- what counts as a game change ----------
def ignore_patterns():
    """The paths a Mac build ignores, read from mac-app.yml so the two never drift."""
    try:
        with open(path(".github", "workflows", "mac-app.yml"), encoding="utf-8") as f:
            text = f.read()
        block = text.split("paths-ignore:", 1)[1].split("workflow_dispatch", 1)[0]
        pats = re.findall(r'"([^"]+)"', block) or DEFAULT_IGNORE
    except Exception:
        pats = DEFAULT_IGNORE
    return [glob_re(p) for p in pats]


def glob_re(pat):
    out, i = "", 0
    while i < len(pat):
        if pat.startswith("**/", i):
            out += "(?:.*/)?"
            i += 3
        elif pat.startswith("**", i):
            out += ".*"
            i += 2
        elif pat[i] == "*":
            out += "[^/]*"
            i += 1
        else:
            out += re.escape(pat[i])
            i += 1
    return re.compile("^" + out + "$")


def is_game_file(f, pats):
    if f.startswith(".github/"):
        return False
    return not any(p.match(f) for p in pats)


# ---------- reading a PR body ----------
CODEISH = re.compile(r"::|\.rs\b|\.js\b|\b[a-z]+_[a-z_]+\b|`")
ATTR = re.compile(r"^(<!--.*?-->|_Requested by .*|_Generated by .*|🤖 Generated.*|https://claude\.ai/code/session_.*)$")


def plain(s, limit=None):
    s = re.sub(r"`([^`]*)`", r"\1", s)
    s = re.sub(r"\*\*([^*]+)\*\*", r"\1", s)
    s = re.sub(r"\[([^\]]+)\]\([^)]*\)", r"\1", s)
    s = re.sub(r"\s+", " ", s).strip()
    if limit and len(s) > limit:
        s = s[:limit].rsplit(" ", 1)[0].rstrip(",;:") + "…"
    return s


def first_sentence(s, limit=260):
    s = plain(s)
    m = re.match(r"(.+?[.!?])(\s|$)", s)
    s = m.group(1) if m else s
    return plain(s, limit)


def sections(body):
    """The body split at '## ' headings: [(heading or '', lines)]."""
    out, head, cur = [], "", []
    for line in body.replace("\r", "").split("\n"):
        m = re.match(r"^##\s+(.*?)\s*$", line)
        if m:
            out.append((head, cur))
            head, cur = m.group(1), []
        else:
            cur.append(line)
    out.append((head, cur))
    return out


def parse_body(title, body):
    body = body or ""
    secs = sections(body)
    note = next((ls for h, ls in secs if re.match(r"(?i)^patch notes?\b", h)), None)
    info = {"title": plain(title), "tldr": "", "bullets": [], "look": "", "players": True, "auto": False}
    if note is not None:
        text = [l for l in note if l.strip() and l.strip() != "---"]
        for l in text:
            s = l.strip()
            m = re.match(r"^\W*TLDR\W*:?\W*(.*)$", s, re.I)
            if m and not info["tldr"]:
                info["tldr"] = plain(m.group(1))
                continue
            m = re.match(r"^\W*Look at\W*:?\W*(.*)$", s, re.I)
            if m:
                info["look"] = plain(m.group(1))
                continue
            m = re.match(r"^\W*Title\W*:\W*(.*)$", s, re.I)
            if m and m.group(1).strip():
                info["title"] = plain(m.group(1))
                continue
            m = re.match(r"^\W*Players?\W*:?\W*(no|none)\b", s, re.I)
            if m:
                info["players"] = False
                continue
            m = re.match(r"^[-*]\s+(.*)$", s)
            if m:
                info["bullets"].append(plain(m.group(1)))
            elif not info["tldr"]:
                info["tldr"] = plain(s)
        if re.match(r"(?i)^(none|no game change)", " ".join(text).strip()):
            info["players"] = False
        if not info["tldr"] and info["bullets"]:
            info["tldr"] = info["bullets"].pop(0)
        return info
    # Fallback: the PR's own summary. Good enough for the team notes, kept out of the game's page.
    info["auto"] = True
    pick = next((ls for h, ls in secs[1:] if re.match(r"(?i)^(what|summary|what it does|what changed)\b", h)), None)
    if pick is None:
        pick = secs[0][1]
    lines = [l.strip() for l in pick if l.strip() and not ATTR.match(l.strip()) and l.strip() != "---"]
    paras, bullets = [], []
    for l in lines:
        m = re.match(r"^[-*]\s+(.*)$", l)
        if m:
            if not CODEISH.search(m.group(1)):  # a bullet that reads as code is left out
                bullets.append(plain(m.group(1), 220))
        elif not bullets:
            paras.append(l)
    if paras:
        info["tldr"] = first_sentence(paras[0])
        info["bullets"] = bullets[:2]
    elif bullets:
        info["tldr"] = first_sentence(bullets[0])
        info["bullets"] = bullets[1:3]
    return info


AI_PROMPT = """You write patch notes for a football management game called AFM, for the people who play it.
Read the pull request below and answer with one JSON object and nothing else:
{"tldr": "...", "bullets": ["...", "..."], "look": "...", "players": true}
Rules: plain words and short sentences, no file names, code, function names, PR numbers or build talk.
"tldr": one or two positive sentences on what this gives the player and what to try.
"bullets": two to four short lines on what changed, from the player's side.
"look": one sentence on what to try first.
"players": false when nothing in it can be seen or felt in the game (docs, tests, tools, internal changes).
Use only facts in the pull request. If it changes nothing a player can see, say so in the tldr."""


def ai_rewrite(title, body):
    """Optional: plain words from the PR text, when the repository has an ANTHROPIC_API_KEY secret.
    Any failure returns None and the notes fall back on the PR's own summary."""
    key = os.environ.get("ANTHROPIC_API_KEY")
    if not key or os.environ.get("PATCH_NOTES_NO_AI") == "1":
        return None
    clean = "\n".join(l for l in (body or "").replace("\r", "").split("\n") if not ATTR.match(l.strip()))
    req = urllib.request.Request(
        "https://api.anthropic.com/v1/messages",
        data=json.dumps({
            "model": os.environ.get("PATCH_NOTES_MODEL", "claude-haiku-5-5"),
            "max_tokens": 700,
            "system": AI_PROMPT,
            "messages": [{"role": "user", "content": f"Title: {title}\n\n{clean[:6000]}"}],
        }).encode(),
        headers={"x-api-key": key, "anthropic-version": "2023-06-01", "content-type": "application/json"},
    )
    try:
        with urllib.request.urlopen(req, timeout=60) as r:
            text = json.load(r)["content"][0]["text"]
        out = json.loads(text[text.index("{"): text.rindex("}") + 1])
        tldr = plain(str(out.get("tldr", "")))
        if not tldr:
            return None
        return {"tldr": tldr, "bullets": [plain(str(b)) for b in out.get("bullets", [])][:5],
                "look": plain(str(out.get("look", ""))), "players": bool(out.get("players", True))}
    except Exception as e:
        print(f"warning: AI rewrite failed: {e}", file=sys.stderr)
        return None


# ---------- the builds and their merges ----------
def shipped_releases():
    """Mac builds that published: release app-<n> with the dmg fully uploaded."""
    rels = gh_json(f"repos/{REPO}/releases", pages=True)
    out = {}
    for r in rels:
        m = re.match(r"^app-(\d+)$", r.get("tag_name", ""))
        if not m or r.get("draft"):
            continue
        if not any(a.get("name") == "AFM_universal.dmg" and a.get("state") == "uploaded" for a in r.get("assets", [])):
            continue
        sha = r.get("target_commitish", "")
        if re.fullmatch(r"[0-9a-f]{40}", sha):
            out[int(m.group(1))] = {"sha": sha, "date": (r.get("published_at") or r.get("created_at") or "")[:10]}
    return out


def pr_number(subject):
    m = re.match(r"Merge pull request #(\d+)", subject) or re.search(r"\(#(\d+)\)\s*$", subject)
    return int(m.group(1)) if m else None


def commits_between(base, sha, pats):
    """First-parent commits after base up to sha, oldest first, with their changed files."""
    out = sh("git", "log", "--first-parent", "--reverse", "--format=%H%x1f%s", f"{base}..{sha}", check=False)
    res = []
    for line in out.splitlines():
        h, _, subj = line.partition("\x1f")
        if subj.startswith("Patch notes"):
            continue
        files = sh("git", "diff", "--name-only", f"{h}^1", h, check=False).split()
        res.append({"sha": h, "subject": subj, "pr": pr_number(subj), "game": any(is_game_file(f, pats) for f in files)})
    return res


class Prs:
    def __init__(self, online=True):
        self.cache = load("prs.json", {})
        self.curated = load("curated.json", {})
        self.online = online
        self.dirty = False

    def get(self, n, refresh=False):
        k = str(n)
        if k in self.curated:
            base = dict(self.cache.get(k) or {"title": f"PR #{n}", "tldr": "", "bullets": [], "look": "", "players": True, "auto": False})
            base.update(self.curated[k])
            base["auto"] = False
            return base
        if k in self.cache and not refresh:
            return self.cache[k]
        if self.online:
            try:
                pr = gh_json(f"repos/{REPO}/pulls/{n}")
                h = hashlib.sha1(f"{pr.get('title', '')}\n{pr.get('body', '')}".encode()).hexdigest()[:12]
                if k in self.cache and self.cache[k].get("h") == h:
                    return self.cache[k]  # the PR text has not changed since
                info = parse_body(pr.get("title", ""), pr.get("body", ""))
                if info["auto"]:
                    better = ai_rewrite(pr.get("title", ""), pr.get("body", ""))
                    if better:
                        info.update(better, auto=False, ai=True)
                info["h"] = h
                if self.cache.get(k) != info:
                    self.cache[k] = info
                    self.dirty = True
                return info
            except Exception as e:  # keep going on what we have
                print(f"warning: PR #{n}: {e}", file=sys.stderr)
        return self.cache.get(k) or {"title": f"PR #{n}", "tldr": "", "bullets": [], "look": "", "players": True, "auto": True}

    def save(self):
        if self.dirty:
            save("prs.json", self.cache)


def day(d):
    if not re.fullmatch(r"\d{4}-\d\d-\d\d", d):
        return ""
    months = "Jan Feb Mar Apr May Jun Jul Aug Sep Oct Nov Dec".split()
    return f"{int(d[8:10])} {months[int(d[5:7]) - 1]}"


def build_entries(chain, prs, pats, fresh):
    """chain: [(n, sha, 'YYYY-MM-DD')] ascending, the first is the baseline (no entry). Newest first out."""
    entries = []
    for i in range(1, len(chain)):
        (_, base, _), (n, sha, when) = chain[i - 1], chain[i]
        merges = commits_between(base, sha, pats)
        items = []
        for c in merges:
            info = prs.get(c["pr"], refresh=n in fresh) if c["pr"] else None
            items.append({**c, "info": info})
        entries.append({"n": n, "sha": sha, "date": day(when), "items": items})
    return list(reversed(entries))


# ---------- writing ----------
def lower_first(t):
    return t if len(t) < 2 or t[1].isupper() or t[1].isdigit() else t[0].lower() + t[1:]


def refs(prs):
    return ", ".join(f"#{p}" for p in prs)


def render_team(e):
    game = [x for x in e["items"] if x["game"] and x["pr"]]
    other_game = [x for x in e["items"] if x["game"] and not x["pr"]]
    docs = [x for x in e["items"] if not x["game"]]
    n = e["n"]
    if not game and not other_game:
        head = f"## 0.2.{n} ({e['date']}, shipped): no game change"
        lines = [head, "**TLDR:** Nothing new in the game in this version, so there is no need to update for the game's sake."]
    else:
        titles = [f"{lower_first(x['info']['title'])} (PR #{x['pr']})" for x in game[:3]]
        more = len(game) - 3
        title = ", ".join(titles) + (f", and {more} more" if more > 0 else "")
        if not game:
            title = "other changes"
        lines = [f"## 0.2.{n} ({e['date']}, shipped): {title}"]
        tl = [x["info"]["tldr"] or x["info"]["title"] for x in game]
        if len(game) == 1:
            tldr = tl[0]
        elif game and any(x["info"]["auto"] for x in game):
            tldr = f"{len(game)} changes ship together: " + "; ".join(lower_first(plain(x["info"]["title"], 90)) for x in game[:4])
            tldr += f"; and {len(game) - 4} more." if len(game) > 4 else "."
        elif game:
            tldr = f"{len(game)} changes ship together. " + " ".join(tl[:2])
            if len(game) > 2:
                tldr += f" Plus {len(game) - 2} more below."
        else:
            tldr = "Small changes to the game."
        lines.append(f"**TLDR:** {plain(tldr, 520)}")
        for x in game:
            p, info = x["pr"], x["info"]
            if len(game) > 1:
                lines.append(f"- {info['title']} (PR #{p}): {info['tldr'] or 'no summary in the PR'}")
            for b in info["bullets"]:
                lines.append(f"- {b} (PR #{p})")
        for x in other_game:
            lines.append(f"- {plain(x['subject'], 200)}")
        looks = [f"{x['info']['look']} (PR #{x['pr']})" for x in game if x["info"].get("look")]
        if looks:
            lines.append("- Look at: " + " ".join(looks[:3]))
    if docs:
        ds = [f"{plain(x['info']['title'], 90) if x['info'] else plain(x['subject'], 90)}" + (f" (PR #{x['pr']})" if x["pr"] else "") for x in docs]
        shown = "; ".join(ds[:6]) + (f"; and {len(ds) - 6} more" if len(ds) > 6 else "")
        lines.append(f"- Docs and tooling merged too, no game change: {shown}.")
    return "\n".join(lines) + "\n"


def plain_title(t):
    """A PR title as one line for players: the lane label and bug numbers off, no design or test items."""
    t = plain(t)
    t = re.sub(r"^(live talks( v\d)?),?\s*lane \w+[:,]?\s*", r"\1: ", t, flags=re.I)
    t = re.sub(r"^(lane|part|step) \w+:\s*", "", t, flags=re.I)
    t = re.sub(r"\s*\((proposed|bug \d+[^)]*|minor bug \d+[^)]*|PR #\d+|#\d+)\)", "", t, flags=re.I)
    t = t.strip(" :,")
    if re.match(r"(?i)^(design|docs?|research|trial|tests?|ci)\b", t) or len(t) < 8:
        return ""
    return t[0].upper() + t[1:]


def player_entry(e):
    """The in-game page, so a build never shows nothing new. A PR with a Patch note section (or the
    plain-words rewrite) gives its TLDR and bullets; a PR without one gives a single line from its
    summary. No PR numbers. A build whose merges are all docs and tooling gets no panel."""
    items = [x for x in e["items"] if x["game"] and x["info"] and x["info"]["players"]]
    written = [x for x in items if not x["info"]["auto"] and x["info"]["tldr"]]
    rest = [x for x in items if x["info"]["auto"] or not x["info"]["tldr"]]
    if not items:
        return None

    def clean(t):
        return re.sub(r"\s*\(PR #\d+\)", "", t).strip()

    notes = []
    for x in written:
        for t in [x["info"]["tldr"]] + x["info"]["bullets"]:
            t = clean(t)
            if t and t not in notes:
                notes.append(t)
    notes = notes[:6]
    extra = 0
    for x in reversed(rest):  # newest first
        t = plain_title(x["info"]["title"])
        if not t or t in notes:
            continue
        if len(notes) < 8:
            notes.append(t)
        else:
            extra += 1
    if extra:
        notes.append(f"Plus {extra} smaller change{'s' if extra > 1 else ''} under the hood.")
    first = (written or rest)[-1]["info"]
    title = plain_title(first["title"]) or plain(first["title"])
    if len(items) > 1:
        title += f" and {len(items) - 1} more"
    return {"v": f"0.2.{e['n']}", "date": e["date"], "title": title, "notes": notes}


def js_entry(p):
    notes = ",\n".join(f"      {json.dumps(t, ensure_ascii=False)}" for t in p["notes"])
    return (f"  {{\n    v: {json.dumps(p['v'])}, date: {json.dumps(p['date'])}, title: {json.dumps(p['title'], ensure_ascii=False)},\n"
            f"    notes: [\n{notes},\n    ],\n  }},")


def write_changelog(entries):
    p = path("app", "ui", "changelog.js")
    with open(p, encoding="utf-8") as f:
        text = f.read()
    if JS_START not in text or JS_END not in text:
        raise RuntimeError("app/ui/changelog.js has no auto markers")
    pre, rest = text.split(JS_START, 1)
    _, post = rest.split(JS_END, 1)
    body = "\n".join(js_entry(p) for p in (player_entry(e) for e in entries) if p)
    new = pre + JS_START + "\n" + (body + "\n" if body else "") + JS_END + post
    if new != text:
        with open(p, "w", encoding="utf-8") as f:
            f.write(new)
        return True
    return False


def write_notes(entries):
    p = path("docs", "patch-notes.md")
    with open(p, encoding="utf-8") as f:
        text = f.read()
    if MARK not in text:
        raise RuntimeError("docs/patch-notes.md has no marker")
    old = text.split(MARK, 1)[1]
    new = HEADER + "\n".join(render_team(e) for e in entries) + ("\n" if entries else "") + MARK + old
    if new != text:
        with open(p, "w", encoding="utf-8") as f:
            f.write(new)
        return True
    return False


# ---------- commands ----------
def chain_from(builds, base):
    chain = [(base["n"], base["sha"], base.get("date", ""))]
    for n in sorted(int(k) for k in builds):
        if n > base["n"]:
            chain.append((n, builds[str(n)]["sha"], builds[str(n)]["date"]))
    return chain


def cmd_sync(_):
    data = load("builds.json", None)
    if data is None:
        sys.exit("docs/patch-notes/builds.json is missing")
    base, builds = data["base"], data["builds"]
    shipped = shipped_releases()
    added = [n for n in shipped if n > base["n"] and str(n) not in builds]
    for n in added:
        builds[str(n)] = shipped[n]
    chain = chain_from(builds, base)
    prs = Prs()
    newest = {c[0] for c in chain[-3:]} | set(added)
    entries = build_entries(chain, prs, ignore_patterns(), newest)
    changed = write_notes(entries)
    changed |= write_changelog(entries)
    prs.save()
    save("builds.json", data)
    print(f"shipped builds in the notes: {[c[0] for c in chain[1:]]}; new this run: {sorted(added)}; files changed: {changed}")


def cmd_preview(a):
    data = load("builds.json", None)
    if data is None:
        sys.exit("docs/patch-notes/builds.json is missing")
    base, builds = data["base"], dict(data["builds"])
    try:
        for n, b in shipped_releases().items():
            if n > base["n"]:
                builds.setdefault(str(n), b)
        online = True
    except Exception as e:
        print(f"warning: releases not read ({e}); using the committed list", file=sys.stderr)
        online = False
    builds = {k: v for k, v in builds.items() if int(k) < a.version}
    import datetime
    builds[str(a.version)] = {"sha": a.sha, "date": datetime.date.today().isoformat()}
    chain = chain_from(builds, base)
    prs = Prs(online=online)
    entries = build_entries(chain, prs, ignore_patterns(), {a.version})
    write_changelog(entries)  # working tree only: the Mac build bakes it in, nothing is committed
    print(f"preview for 0.2.{a.version}: {len(entries)} entries")


def cmd_check(a):
    pr = gh_json(f"repos/{REPO}/pulls/{a.pr}")
    files = [f["filename"] for f in gh_json(f"repos/{REPO}/pulls/{a.pr}/files", pages=True)]
    pats = ignore_patterns()
    if not any(is_game_file(f, pats) for f in files):
        print("Docs or tooling only: no Patch note needed.")
        return
    info = parse_body(pr.get("title", ""), pr.get("body", ""))
    if not info["auto"]:
        print("Patch note found.")
        return
    msg = ("This PR changes the game but has no '## Patch note' section. Add one (TLDR, three to five plain lines, "
           "Look at, or 'Players: no') so the patch notes and the in-game What's new page can use your words; "
           "without it the notes fall back on the PR summary and What's new skips it.")
    print(f"::warning title=No Patch note::{msg}")
    if os.environ.get("REQUIRE_PATCH_NOTE") == "1":
        sys.exit(1)


def main():
    ap = argparse.ArgumentParser()
    sub = ap.add_subparsers(dest="cmd", required=True)
    sub.add_parser("sync").set_defaults(f=cmd_sync)
    p = sub.add_parser("preview")
    p.add_argument("--version", type=int, required=True)
    p.add_argument("--sha", required=True)
    p.set_defaults(f=cmd_preview)
    p = sub.add_parser("check")
    p.add_argument("--pr", type=int, required=True)
    p.set_defaults(f=cmd_check)
    a = ap.parse_args()
    a.f(a)


if __name__ == "__main__":
    main()
