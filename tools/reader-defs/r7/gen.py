# Writes crates/afm-reader/src/defs_programs.rs (world reader lane R7) from
# programs.py, hs.py and coaches.py: run it from anywhere.
import os, subprocess, sys, textwrap
HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
OUT = os.path.join(HERE, '../../../crates/afm-reader/src/defs_programs.rs')
import programs, hs, coaches

def raw(s):
    s = textwrap.dedent(s) if s.startswith('\n') else s
    assert '"#' not in s, s[:80]
    return 'r#"' + s + '"#'

def opt(s):
    return 'None' if s is None else f'Some({raw(s)})'

def real(r):
    k = r[0]
    if k == 'Key':
        return f'Real::Key({raw(r[1])}, {float(r[2])!r})'
    if k == 'Fixed':
        return f'Real::Fixed({float(r[1])!r}, {raw(r[2])}, {raw(r[3])})'
    if k == 'Memory':
        return f'Real::Memory({float(r[1])!r}, {raw(r[2])})'
    return f'Real::None({raw(r[1])})'

NEEDS = {'world file': 'W', 'world log': 'WL', 'game log': 'GL'}

def sql(s):
    # Keep the SQL as written, each line after the first indented under the field.
    lines = s.strip('\n').split('\n')
    return '\n'.join([lines[0]] + ['         ' + l.lstrip() if l.strip() else '' for l in lines[1:]])

out = []
w = out.append
w('''//! The programs, high school and coaches pages' checks, consistency checks
//! and panels (world reader lane R7): every inventory area of Programs
//! (C1-C17, C19-C22, C42, T13), Money's C14, High school (C23-C27, G7, G27,
//! N32), Coaches (S1-S31, S33), S34 on the NFL page and the conference
//! pages. Types in `defs.rs` and `panels.rs`; the shared pieces:
//! - a program season's level and conference are its real history's through
//!   2025 (`programs__history`: from, to, level, conference), else today's;
//! - a coach's job's role is `role` (head coach, coordinators) or
//!   `role__PositionCoach` (the group); a job's years overlap a season at
//!   each end, so tenure counts runs of the season's coach, as real data does;
//! - a child table's `_parent` is compared with `+x._row`, which keeps its
//!   index on books from before `_parent` was typed;
//! - real figures in the panels are read straight from the real files
//!   (`real_data.json`, `real_<csv>` tables).
//!
//! Written by `tools/reader-defs/r7/gen.py`: edit the Python there and run it
//! again.

use crate::defs::{Check, Metric, Outlier, Real, Rule};
use crate::panels::Panel;

const W: &str = "world file";
const WL: &str = "world log";
const GL: &str = "game log";

/// Columns the SQL below reads that a small or early world may lack.
pub const COLUMNS: &[(&str, &str)] = &[''')
cols = [("coaches", "born"), ("coaches", "job__employer__Other"), ("coaches__history", "employer__Other"), ("high_schools", "county"), ("high_schools", "oc"),
        ("high_schools", "dc"), ("high_schools__seasons", "playoff_wins"), ("high_schools__seasons", "semifinal"), ("college_prev__polls", "points"),
        ("college_prev__polls", "firsts"), ("log_schools", "school__prestige"), ("log_coaches", "coach__retired"), ("log_coaches", "coach__reputation"),
        ("log_coaches", "coach__job__role"), ("log_coaches", "coach__job__employer__College"),
        ("log_programs", "result__expected"), ("log_programs", "result__wins"), ("log_programs", "result__losses"),
        ("people", "player__train__prep"), ("people", "player__train__rust"), ("people", "player__train__load")]
cols += [("log_coaches", f"coach__ability__teaching__{i}") for i in range(8)]
cols += [("log_coaches", f"coach__ability__{k}") for k in ["scheme", "game_management", "recruiting", "network"]]
for t, c in cols:
    w(f'    ("{t}", "{c}"),')
w('];\n')
w('pub const CHECKS: &[Check] = &[')
for m in (programs, hs, coaches):
    for c in m.CHECKS:
        w(f'''    Check {{
        id: "{c['id']}",
        page: "{c['page']}",
        areas: "{c['areas']}",
        title: {raw(c['title'])},
        unit: "{c['unit']}",
        world: {raw(sql(c['world']))},
        scale: {float(c['scale'])!r},
        years: {opt(sql(c['years']) if c['years'] else None)},
        detail: {opt(sql(c['detail']) if c['detail'] else None)},
        real: {real(c['real'])},
        tol: ({float(c['tol'][0])!r}, {float(c['tol'][1])!r}, {float(c['tol'][2])!r}),
        needs: {NEEDS[c['needs']]},
        how: {raw(c['how'])},
    }},''')
w('];\n')
w('pub const METRICS: &[Metric] = &[];\n')
w('pub const RULES: &[Rule] = &[')
for m in (programs, hs, coaches):
    for r in m.RULES:
        w(f'''    Rule {{
        id: "{r['id']}",
        areas: "{r['areas']}",
        title: {raw(r['title'])},
        checked: {raw(sql(r['checked']))},
        failures: {raw(sql(r['failures']))},
        needs: {NEEDS[r['needs']]},
        why: {raw(r['why'])},
    }},''')
w('];\n')
w('pub const OUTLIERS: &[Outlier] = &[];\n')
w('pub const PANELS: &[Panel] = &[')
for m in (programs, hs, coaches):
    for x in m.PANELS + getattr(m, 'CONF', []):
        w(f'''    Panel {{
        id: "{x['id']}",
        page: "{x['page']}",
        area: "{x['area']}",
        title: {raw(x['title'])},
        kind: "{x['kind']}",
        note: {raw(x['note'])},
        sql: {raw(sql(x['sql']))},
        says: {opt(sql(x['says']) if x['says'] else None)},
        needs: {NEEDS[x['needs']]},
        open: "{x['open']}",
    }},''')
w('];')
open(OUT, 'w').write('\n'.join(out) + '\n')
ids = [x['id'] for m in (programs, hs, coaches) for x in m.PANELS + getattr(m, 'CONF', [])]
assert len(ids) == len(set(ids)), [i for i in ids if ids.count(i) > 1]
print(len(ids), 'panels', sum(len(m.CHECKS) for m in (programs, hs, coaches)), 'checks', sum(len(m.RULES) for m in (programs, hs, coaches)), 'rules')
subprocess.run(['rustfmt', '--edition', '2024', '--config', 'max_width=120', OUT], check=True)
