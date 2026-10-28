"""Builds data/real-world/index.json, the one index of real figures (world reader lane R3).

Reads index_sources.json (the hand-kept catalogue: each file's source, years, holes and inventory areas, the figures
written in code, the memory claims that now have data, the holes and the sources the cloud can't reach), then:
  every real file here: each JSON key down two levels with its value when small, its years and holes; each CSV's
    columns, rows and years, and how many of its rows say they are from memory;
  the Debug Lab's real values (tools/dashboard/build_data.py real_values());
  every "from memory" note in crates/, tools/, app/ and docs/ (the handoff transcripts left out), with file and line,
    and the data that now checks it where there is some.

Usage: python3 scripts/build_index.py   (from data/real-world or anywhere; writes index.json next to this folder)
Development checks only, never in-game tuning.
"""
import csv, json, os, re, sys

HERE = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
REPO = os.path.dirname(os.path.dirname(HERE))
META = {"source", "holes", "note", "years", "about", "_note", "level", "status"}
SMALL = 240
YEAR = re.compile(r"^(19[0-9]{2}|20[0-9]{2})$")
YEARS_IN = re.compile(r"\b(19[0-9]{2}|20[0-9]{2})\b")


def size(v):
    return len(json.dumps(v, ensure_ascii=False, default=str))


def years_of(v, depth=0):
    """Every season named by a key (or a years string) inside v, a few levels down."""
    out = set()
    if isinstance(v, dict) and depth < 4:
        for k, x in v.items():
            if YEAR.match(str(k)):
                out.add(int(k))
            elif k == "years" and isinstance(x, str):
                out.update(int(y) for y in YEARS_IN.findall(x))
            out |= years_of(x, depth + 1)
    elif isinstance(v, list) and depth < 4:
        for x in v[:200]:
            out |= years_of(x, depth + 1)
    return out


def span(ys, fallback):
    return f"{min(ys)}-{max(ys)}" if ys else fallback


def says_memory(v):
    return "from memory" in json.dumps(v, ensure_ascii=False, default=str).lower()


def figure(fname, path, v, entry, parent_holes):
    """One figure; its source, holes and areas are left out when they are its file's."""
    keys = entry.get("keys", {})
    top = path.split(".")[0]
    f = {"key": f"{fname}:{path}", "file": fname,
         "years": span(years_of(v) | {int(x) for x in path.split(".") if YEAR.match(x)}, (v.get("years") if isinstance(v, dict) and isinstance(v.get("years"), str) else None) or entry.get("years")),
         "status": "from memory" if says_memory(v) else entry.get("status")}
    holes = v.get("holes") if isinstance(v, dict) and v.get("holes") else parent_holes
    if holes and holes != entry.get("holes"):
        f["holes"] = holes
    if path in keys or top in keys:
        f["areas"] = keys.get(path, keys.get(top))
    if isinstance(v, dict) and isinstance(v.get("note"), str):
        f["note"] = v["note"]
    if size(v) <= SMALL:
        f["value"] = v
    return f


def json_figures(fname, entry):
    d = json.load(open(os.path.join(HERE, fname)))
    if not isinstance(d, dict):
        return [figure(fname, "(all)", d, entry, entry.get("holes"))]
    out = []
    file_holes = d.get("holes") if isinstance(d.get("holes"), str) else entry.get("holes")
    for k, v in d.items():
        if k in META:
            continue
        subs = [s for s in v if s not in META] if isinstance(v, dict) else []
        if len(subs) > 1 and not all(YEAR.match(str(s)) for s in subs) and size(v) > SMALL and len(subs) <= 60:
            holes = v.get("holes") if isinstance(v.get("holes"), str) else file_holes
            for s in subs:
                f = figure(fname, f"{k}.{s}", v[s], entry, holes)
                if isinstance(v.get("note"), str) and "note" not in f:
                    f["note"] = v["note"]
                out.append(f)
        else:
            out.append(figure(fname, k, v, entry, file_holes))
    return out


def csv_figure(fname, entry):
    with open(os.path.join(HERE, fname), newline="", encoding="utf-8") as fh:
        rows = list(csv.reader(fh))
    head, body = (rows[0], rows[1:]) if rows else ([], [])
    ys = set()
    for i, c in enumerate(head):
        vals = [r[i] for r in body if i < len(r) and r[i]]
        if vals and all(YEAR.match(x) for x in vals[:500]):
            ys.update(int(x) for x in vals)
    mem = sum(1 for r in body if any("from memory" in c.lower() for c in r))
    f = {"key": f"{fname}", "file": fname, "years": span(ys, entry.get("years")), "status": entry.get("status"),
         "columns": head, "rows": len(body)}
    if mem:
        f["rows_from_memory"] = mem
    return f


def debug_lab():
    sys.path.insert(0, os.path.join(REPO, "tools", "dashboard"))
    try:
        import build_data
    except Exception as e:  # the Debug Lab script missing or broken: say so, keep the rest
        return [{"key": "debug_lab", "status": f"not read: {e}"}]
    v = build_data.real_values()
    src = v.get("sources", {})
    out = []
    for k, x in v.items():
        if k == "sources":
            continue
        f = {"key": f"debug_lab.{k}", "file": "tools/dashboard/build_data.py real_values()",
             "source": src.get(k) or src.get("nfl" if k.startswith("nfl") else "college" if k.startswith("fbs") else "baselines"),
             "years": span(years_of(x), None), "status": "from memory" if says_memory(x) else "sourced"}
        notes = x.get("_notes", {}) if isinstance(x, dict) else {}
        if isinstance(notes, dict) and says_memory(notes):
            f["from_memory_parts"] = sorted(s for s, y in notes.items() if says_memory(y))
        if size(x) <= SMALL:
            f["value"] = x
        out.append(f)
    return out


def memory_notes(replacements):
    hits = []
    roots = ["crates", "tools", "app", "docs"]
    for root in roots:
        for dp, dn, fn in os.walk(os.path.join(REPO, root)):
            dn[:] = [d for d in dn if d not in ("target", "node_modules", "handoff", "faces", ".git")]
            for n in fn:
                if not n.endswith((".rs", ".py", ".js", ".md", ".ts")):
                    continue
                p = os.path.join(dp, n)
                rel = os.path.relpath(p, REPO)
                try:
                    lines = open(p, encoding="utf-8").read().split("\n")
                except (UnicodeDecodeError, OSError):
                    continue
                for i, line in enumerate(lines):
                    if "from memory" not in line.lower():
                        continue
                    near = "\n".join(lines[max(0, i - 15):i + 3])
                    h = {"file": rel, "line": i + 1, "text": line.strip()[:220], "status": "from memory"}
                    for m in replacements:
                        if m["file"] == rel and m["match"] in near:
                            h.update({"checked_by": m["key"], "result": m["status"], "check": m["check"]})
                    hits.append(h)
    return hits


def main():
    src = json.load(open(os.path.join(HERE, "index_sources.json")))
    figures, files = [], {}
    for fname, entry in src["files"].items():
        p = os.path.join(HERE, fname)
        if not os.path.exists(p):
            files[fname] = {**entry, "missing": True}
            continue
        files[fname] = {**entry, "bytes": os.path.getsize(p)}
        if fname.endswith(".json"):
            figures += json_figures(fname, entry)
        elif fname.endswith(".csv"):
            figures.append(csv_figure(fname, entry))
        else:
            figures.append({"key": fname, "file": fname, "years": entry.get("years"), "status": entry.get("status")})
    unlisted = sorted(n for n in os.listdir(HERE) if n.endswith((".json", ".csv", ".md")) and n not in src["files"]
                      and n not in ("index.json", "index_sources.json", "README.md"))
    lab = debug_lab()
    notes = memory_notes(src.get("memory_replacements", []))
    areas = {}
    for fname, entry in src["files"].items():
        for a in entry.get("areas", []):
            areas.setdefault(a, []).append(fname)
    for f in figures:
        for a in f.get("areas", []):
            areas.setdefault(a, []).append(f["key"])
    for c in src.get("in_code", []):
        for a in c.get("areas", []):
            areas.setdefault(a, []).append(f"in code: {c['what']}")
    for h in src.get("holes", []):
        for a in h.get("areas", []):
            areas.setdefault(a, []).append(f"hole: {h['what']}")
    count = lambda xs, s: sum(1 for x in xs if x.get("status") == s)
    out = {
        "about": "Every real figure the game is checked against: its key (file:path), value when small, source, years, holes, "
                 "status and the inventory areas (afm/world-reader/spec.md) that use it. A figure without source, holes or areas takes its file's (see files). Built by scripts/build_index.py from "
                 "index_sources.json and the files here; edit those, never this file.",
        "status_words": {"sourced": "taken from the named source", "computed": "worked out here from sourced data",
                         "estimate": "a researched estimate, the file says how", "from memory": "written without a source; to check",
                         "cited in code": "the code names its source", "unconfirmed": "a proposal Raichel hasn't answered"},
        "counts": {"files": len(files), "figures": len(figures), "debug_lab_figures": len(lab), "in_code": len(src.get("in_code", [])),
                   "memory_notes": len(notes), "memory_notes_checked": sum(1 for n in notes if "checked_by" in n),
                   "memory_checks_that_differ": sum(1 for n in src.get("memory_replacements", []) if n["status"] == "differs"),
                   "figures_from_memory": count(figures, "from memory") + count(lab, "from memory"),
                   "holes": len(src.get("holes", [])), "unreachable_sources": len(src.get("unreachable_sources", []))},
        "files": files,
        "unlisted_files": unlisted,
        "figures": figures,
        "debug_lab": lab,
        "in_code": src.get("in_code", []),
        "memory_checks": src.get("memory_replacements", []),
        "memory_notes": notes,
        "trial_claims": src.get("trial_claims", []),
        "holes": src.get("holes", []),
        "unreachable_sources": src.get("unreachable_sources", []),
        "areas": {k: list(dict.fromkeys(areas[k])) for k in sorted(areas, key=lambda a: (re.sub(r"\d", "", a), int(re.sub(r"\D", "", a) or 0)))},
    }
    with open(os.path.join(HERE, "index.json"), "w", encoding="utf-8") as fh:
        json.dump(out, fh, indent=1, ensure_ascii=False, default=str)
        fh.write("\n")
    print(json.dumps(out["counts"]), "unlisted:", unlisted)


if __name__ == "__main__":
    main()
