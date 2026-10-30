"""R8 spec helpers: panels, checks, rules, metrics; a Rust generator and a SQL test runner."""
import sqlite3, json, sys, re, os

P, C, R, M, COLS = [], [], [], [], []
W, G, L = "W", "G", "L"   # needs: world file, game log, world log
NEEDS = {"W": "W", "G": "G", "L": "WL"}

def panel(id, page, area, title, kind, note, sql, says=None, needs=W, open=""):
    P.append(dict(id=id, page=page, area=area, title=title, kind=kind, note=note, sql=sql.strip(), says=(says.strip() if says else None), needs=needs, open=open))

def check(id, page, areas, title, unit, world, real, tol, how, scale=1.0, years=None, detail=None, needs=W):
    C.append(dict(id=id, page=page, areas=areas, title=title, unit=unit, world=world.strip(), scale=scale,
                  years=(years.strip() if years else None), detail=(detail.strip() if detail else None), real=real, tol=tol, needs=needs, how=how))

def rule(id, areas, title, checked, failures, why, needs=W):
    R.append(dict(id=id, areas=areas, title=title, checked=checked.strip(), failures=failures.strip(), needs=needs, why=why))

def metric(id, page, areas, title, unit, sql, needs=W):
    M.append(dict(id=id, page=page, areas=areas, title=title, unit=unit, sql=sql.strip(), needs=needs))

def col(table, column):
    COLS.append((table, column))

def real(file):
    return f"(SELECT json FROM real_data WHERE file = '{file}')"

# Real figure sources for checks.
def Key(k, scale=1.0): return ("Key", k, scale)
def Fixed(v, src, yrs): return ("Fixed", v, src, yrs)
def Memory(v, src): return ("Memory", v, src)
def NoReal(why): return ("None", why)

def rs(s):
    """A Rust raw string."""
    h = "#"
    while ('"' + h) in s:
        h += "#"
    return f'r{h}"{s}"{h}'

def opt(s):
    return "Some(" + rs(s) + ")" if s else "None"

def fields(pairs):
    return "".join("        " + k + ": " + v + ",\n" for k, v in pairs)

def gen(path, module_doc):
    out = [module_doc.rstrip() + "\n\nuse crate::defs::{Check, Metric, Outlier, Real, Rule};\nuse crate::panels::Panel;\n\n"]
    out.append('const W: &str = "world file";\nconst G: &str = "game log";\n#[allow(dead_code)]\nconst WL: &str = "world log";\n\n')
    out.append("/// Columns the SQL below reads that a small or early world may lack.\npub const COLUMNS: &[(&str, &str)] = &[\n")
    seen = set()
    for t, c in COLS:
        if (t, c) in seen:
            continue
        seen.add((t, c))
        out.append('    ("' + t + '", "' + c + '"),\n')
    out.append("];\n\npub const CHECKS: &[Check] = &[\n")
    for c in C:
        r = c["real"]
        if r[0] == "Key": rr = "Real::Key(" + rs(r[1]) + ", " + repr(float(r[2])) + ")"
        elif r[0] == "Fixed": rr = "Real::Fixed(" + repr(float(r[1])) + ", " + rs(r[2]) + ", " + rs(r[3]) + ")"
        elif r[0] == "Memory": rr = "Real::Memory(" + repr(float(r[1])) + ", " + rs(r[2]) + ")"
        else: rr = "Real::None(" + rs(r[1]) + ")"
        t = c["tol"]
        out.append("    Check {\n" + fields([("id", '"' + c["id"] + '"'), ("page", '"' + c["page"] + '"'), ("areas", '"' + c["areas"] + '"'),
            ("title", rs(c["title"])), ("unit", '"' + c["unit"] + '"'), ("world", rs(c["world"])), ("scale", repr(float(c["scale"]))),
            ("years", opt(c["years"])), ("detail", opt(c["detail"])), ("real", rr),
            ("tol", "(" + ", ".join(repr(float(x)) for x in t) + ")"), ("needs", NEEDS[c["needs"]]), ("how", rs(c["how"]))]) + "    },\n")
    out.append("];\n\npub const METRICS: &[Metric] = &[\n")
    for m in M:
        out.append("    Metric {\n" + fields([("id", '"' + m["id"] + '"'), ("page", '"' + m["page"] + '"'), ("areas", '"' + m["areas"] + '"'),
            ("title", rs(m["title"])), ("unit", '"' + m["unit"] + '"'), ("sql", rs(m["sql"])), ("needs", NEEDS[m["needs"]])]) + "    },\n")
    out.append("];\n\npub const RULES: &[Rule] = &[\n")
    for r in R:
        out.append("    Rule {\n" + fields([("id", '"' + r["id"] + '"'), ("areas", '"' + r["areas"] + '"'), ("title", rs(r["title"])),
            ("checked", rs(r["checked"])), ("failures", rs(r["failures"])), ("needs", NEEDS[r["needs"]]), ("why", rs(r["why"]))]) + "    },\n")
    out.append("];\n\npub const OUTLIERS: &[Outlier] = &[];\n\npub const PANELS: &[Panel] = &[\n")
    for p in P:
        out.append("    Panel {\n" + fields([("id", '"' + p["id"] + '"'), ("page", '"' + p["page"] + '"'), ("area", '"' + p["area"] + '"'),
            ("title", rs(p["title"])), ("kind", '"' + p["kind"] + '"'), ("note", rs(p["note"])), ("sql", rs(p["sql"])),
            ("says", opt(p["says"])), ("needs", NEEDS[p["needs"]]), ("open", '"' + p["open"] + '"')]) + "    },\n")
    out.append("];\n")
    open(path, "w").write("".join(out))

BANNED = ["floor", "ceil", "ceiling", "log", "log10", "log2", "power", "pow", "mod", "trunc", "sign", "pi", "acos", "asin", "atan", "cos", "sin", "tan", "degrees", "radians"]

def connect(book):
    c = sqlite3.connect(book)
    def ban(name):
        def f(*a): raise Exception(f"no {name} in the book")
        return f
    for b in BANNED:
        for n in (1, 2):
            c.create_function(b, n, ban(b))
    c.create_function("more", 2, lambda t, b: None)
    return c

def test(book, only=None, show=2, width=160):
    c = connect(book)
    have = {r[0] for r in c.execute("SELECT name FROM sqlite_master WHERE type = 'table'")}
    for t, col_ in COLS:
        if t not in have:
            c.execute(f'CREATE TEMP TABLE IF NOT EXISTS "{t}" (_row INTEGER)')
            have_cols = [r[1] for r in c.execute(f'PRAGMA temp.table_info("{t}")')]
            if col_ not in have_cols: c.execute(f'ALTER TABLE temp."{t}" ADD COLUMN "{col_}"')
        else:
            have_cols = [r[1] for r in c.execute(f'PRAGMA table_info("{t}")')]
            if col_ not in have_cols: c.execute(f'ALTER TABLE "{t}" ADD COLUMN "{col_}"')
    bad = 0
    items = [("panel", p["id"], p["sql"], p["says"]) for p in P] + [("check", x["id"], x["world"], x["years"]) for x in C] \
        + [("rule", r["id"], r["failures"], r["checked"]) for r in R] + [("metric", m["id"], m["sql"], None) for m in M]
    for kind, id, sql, extra in items:
        if only and not re.search(only, id): continue
        try:
            cur = c.execute(sql)
            cols = [d[0] for d in cur.description] if cur.description else []
            rows = cur.fetchall()
            note = ""
            if extra:
                try:
                    e = c.execute(extra).fetchone()
                    note = " | " + str(e[0] if e else None)[:width]
                except Exception as ex:
                    note = f" | EXTRA ERR {ex}"; bad += 1
            flag = "MORE " if "more(" in sql else ""
            print(f"ok   {kind:6} {id:34} {len(rows):5} rows {flag}{cols}{note}"[:400])
            for r in rows[:show]:
                print("        ", str(r)[:width])
            if len(rows) > 5000: print("         ! over 5000 rows")
        except Exception as ex:
            bad += 1
            print(f"FAIL {kind:6} {id:34} {ex}")
    print("failures:", bad)
