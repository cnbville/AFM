//! Every page of a book as data (`View`), the same for the app
//! (`Book::page`, JSON) and for `afm_read` and "Copy for Claude"
//! (`Book::text`). A page follows the spec's four layers: a verdict in
//! plain words, the headline checks with their marks, the full tables, and
//! where to look next.
//!
//! Pages: `overview`, the topic pages (`pages::PAGES`), `checks`,
//! `check <id>`, `odd`, `issues`, `knobs`, `coverage`, `tables`,
//! `table <name>`, `dist <metric>`, `record <table> <row>`,
//! `<kind> <id>` (person, alumnus, coach, program, school, club),
//! `search <text>` and `sql <query>`.

use std::fmt::Write;

use rusqlite::types::Value as S;
use serde_json::{Value as J, json};

use crate::book::Book;
use crate::pages::{self, PAGES};
use crate::text::{cell, table};

/// What a page is narrowed to. `extra` holds any other `key=value`: a
/// column filter on table pages, `where=<sql>`, `mark=`, `page=`,
/// `status=`, `kind=`.
#[derive(Clone, Debug, Default)]
pub struct Filters {
    /// FBS, FCS, D2, D3, NAIA, JUCO, HS, NFL or a stage.
    pub level: Option<String>,
    pub division: Option<String>,
    pub conference: Option<String>,
    pub position: Option<String>,
    pub stars: Option<i64>,
    pub state: Option<String>,
    pub year: Option<i64>,
    pub year_from: Option<i64>,
    pub year_to: Option<i64>,
    /// Shows true potential and ceilings (`x_potential`); off by default.
    pub debug: bool,
    pub limit: Option<usize>,
    pub extra: Vec<(String, String)>,
}

impl Filters {
    /// From `key=value` words (`--key=value` too); a bare `debug` turns
    /// debug on.
    pub fn parse(args: &[String]) -> Filters {
        let mut f = Filters::default();
        for a in args {
            let a = a.trim_start_matches("--");
            if a == "debug" {
                f.debug = true;
                continue;
            }
            let Some((k, v)) = a.split_once('=') else { continue };
            let n = v.parse::<i64>().ok();
            match k {
                "level" => f.level = Some(v.into()),
                "division" | "div" => f.division = Some(v.into()),
                "conference" | "conf" => f.conference = Some(v.into()),
                "position" | "pos" => f.position = Some(v.into()),
                "stars" => f.stars = n,
                "state" => f.state = Some(v.into()),
                "year" => f.year = n,
                "year_from" | "from" => f.year_from = n,
                "year_to" | "to" => f.year_to = n,
                "debug" => f.debug = matches!(v, "on" | "1" | "true" | "yes"),
                "limit" => f.limit = v.parse().ok(),
                _ => f.extra.push((k.into(), v.into())),
            }
        }
        f
    }

    pub fn get(&self, k: &str) -> Option<&str> {
        self.extra.iter().find(|(a, _)| a == k).map(|(_, v)| v.as_str())
    }

    fn limit(&self, d: usize) -> usize {
        self.limit.unwrap_or(d)
    }

    /// `AND col` within the year filters.
    fn years(&self, col: &str) -> String {
        let mut s = String::new();
        if let Some(y) = self.year {
            let _ = write!(s, " AND {col} = {y}");
        }
        if let Some(y) = self.year_from {
            let _ = write!(s, " AND {col} >= {y}");
        }
        if let Some(y) = self.year_to {
            let _ = write!(s, " AND {col} <= {y}");
        }
        s
    }

    fn any_year(&self) -> bool {
        self.year.is_some() || self.year_from.is_some() || self.year_to.is_some()
    }

    /// The distribution groups the filters pick (`grp_kind`, `grp`).
    fn groups(&self) -> String {
        let mut or = Vec::new();
        let mut add = |kinds: &str, v: &str| or.push(format!("(grp_kind IN ({kinds}) AND grp = {})", lit(v)));
        if let Some(v) = &self.level {
            add("'level','division','stage'", v);
        }
        if let Some(v) = &self.division {
            add("'division'", v);
        }
        if let Some(v) = &self.conference {
            add("'conference'", v);
        }
        if let Some(v) = &self.position {
            add("'position','college position','pro position'", v);
        }
        if let Some(v) = self.stars {
            add("'stars'", &v.to_string());
        }
        if let Some(v) = &self.state {
            add("'state'", v);
        }
        if or.is_empty() { String::new() } else { format!(" AND ({})", or.join(" OR ")) }
    }
}

/// A text value as an SQL literal.
fn lit(v: &str) -> String {
    if v.parse::<f64>().is_ok() { v.to_string() } else { format!("'{}'", v.replace('\'', "''")) }
}

/// One block of a page.
#[derive(Clone, Debug, Default)]
pub struct Section {
    pub title: String,
    /// checks, table, odd, dist, record, text
    pub kind: String,
    pub columns: Vec<String>,
    pub rows: Vec<Vec<S>>,
    /// Rows the limit left out.
    pub more: usize,
    pub notes: Vec<String>,
}

/// A whole page.
#[derive(Clone, Debug, Default)]
pub struct View {
    pub page: String,
    pub title: String,
    pub verdict: Vec<String>,
    pub sections: Vec<Section>,
    /// Commands to look further.
    pub next: Vec<String>,
}

impl View {
    pub fn to_json(&self) -> J {
        let sections: Vec<J> = self
            .sections
            .iter()
            .map(|s| {
                let rows: Vec<J> = s.rows.iter().map(|r| J::Array(r.iter().map(crate::sink::sql_json).collect())).collect();
                json!({"title": s.title, "kind": s.kind, "columns": s.columns, "rows": rows, "more": s.more, "notes": s.notes})
            })
            .collect();
        json!({"page": self.page, "title": self.title, "verdict": self.verdict, "sections": sections, "next": self.next})
    }

    pub fn to_text(&self) -> String {
        let mut out = format!("# {}\n", self.title);
        for v in &self.verdict {
            let _ = writeln!(out, "{v}");
        }
        for s in &self.sections {
            let _ = writeln!(out, "\n## {}", s.title);
            if !s.rows.is_empty() {
                out.push_str(&table(&s.columns, &s.rows));
            }
            if s.more > 0 {
                let _ = writeln!(out, "... {} more rows (limit=N to see more)", s.more);
            }
            for n in &s.notes {
                let _ = writeln!(out, "{n}");
            }
        }
        if !self.next.is_empty() {
            let _ = writeln!(out, "\n## Next");
            for n in &self.next {
                let _ = writeln!(out, "- {n}");
            }
        }
        out
    }
}

/// A section from a query, at most `limit` rows; an error becomes a note.
fn query(b: &Book, title: &str, kind: &str, sql: &str, limit: usize) -> Section {
    let mut s = Section { title: title.into(), kind: kind.into(), ..Default::default() };
    match b.rows(sql) {
        Ok((names, mut rows)) => {
            s.columns = names;
            if rows.len() > limit {
                s.more = rows.len() - limit;
                rows.truncate(limit);
            }
            if rows.is_empty() {
                s.notes.push("none".into());
            }
            s.rows = rows;
        }
        Err(e) => s.notes.push(format!("not in this book ({e})")),
    }
    s
}

fn note(title: &str, lines: Vec<String>) -> Section {
    Section { title: title.into(), kind: "text".into(), notes: lines, ..Default::default() }
}

impl Book {
    /// A page as text: what `afm_read` prints.
    pub fn text(&self, page: &str, f: &Filters) -> String {
        self.view(page, f).to_text()
    }

    /// A page as JSON for the app.
    pub fn page(&self, page: &str, f: &Filters) -> J {
        self.view(page, f).to_json()
    }

    /// A page as data.
    pub fn view(&self, page: &str, f: &Filters) -> View {
        let words: Vec<&str> = page.split_whitespace().collect();
        let (head, rest) = words.split_first().map_or(("overview", &[][..]), |(h, r)| (*h, r));
        let arg = rest.join(" ");
        match head {
            "overview" => overview(self, f),
            "checks" => checks(self, f),
            "check" => check(self, &arg, f),
            "odd" => odd(self, f),
            "issues" => listing(
                self,
                "Known issues",
                "issues",
                "SELECT id, kind, status, live, title, system, live_why FROM known_issues",
                &["status", "kind", "live"],
                f,
            ),
            "knobs" => listing(
                self,
                "Knobs",
                "knobs",
                "SELECT name, value, file, system, status, note FROM knobs",
                &["system", "status", "file"],
                f,
            ),
            "coverage" => coverage(self, f),
            "tables" => {
                listing(self, "Tables", "tables", "SELECT tbl, source, rows, ncols, what FROM catalog", &["source"], f)
            }
            "table" => table_page(self, &arg, f),
            "dist" => dist(self, &arg, f),
            "record" => {
                let (t, r) = rest
                    .split_first()
                    .map_or(("", 0), |(t, r)| (*t, r.first().and_then(|x| x.parse().ok()).unwrap_or(0)));
                record(self, t, r, f)
            }
            "person" | "alumnus" | "coach" | "program" | "school" | "club" => found(self, head, &arg, f),
            "area" => area(self, &arg.to_ascii_uppercase(), f),
            "panel" => panel_page(self, &arg, None, f),
            "class" => panel_page(self, "class", Some(("class", &arg)), f),
            "state" => panel_page(self, "state", Some(("state", &arg.to_ascii_uppercase())), f),
            "conference" => panel_page(self, "conference", Some(("conference", &arg)), f),
            "search" => search(self, &arg, f),
            "sql" => {
                let mut v = View { page: "sql".into(), title: "SQL".into(), ..Default::default() };
                v.sections.push(query(self, &arg, "table", &arg, f.limit(200)));
                v
            }
            p if PAGES.iter().any(|x| x.id == p) => topic(self, p, f),
            other => {
                let mut v = View { page: other.into(), title: format!("No page called {other}"), ..Default::default() };
                v.verdict.push(
                    "Pages: overview, ".to_string()
                        + &PAGES.iter().skip(1).map(|p| p.id).collect::<Vec<_>>().join(", "),
                );
                v.verdict.push("Also: area <code>, panel <id>, class <year>, state <XX>, conference <name>, checks, check <id>, odd, issues, knobs, coverage, tables, table <name>, dist <metric>, record <table> <row>, person|alumnus|coach|program|school|club <id>, search <text>, sql <query>".into());
                v
            }
        }
    }
}

/// The marks of the checks matching `cond`, as a line.
fn marks_line(b: &Book, cond: &str) -> String {
    let q = format!("SELECT mark, COUNT(*) FROM checks WHERE {cond} GROUP BY mark ORDER BY mark");
    let parts: Vec<String> = b
        .rows(&q)
        .map(|(_, r)| r.iter().map(|r| format!("{} {}", cell(&r[1]), cell(&r[0]))).collect())
        .unwrap_or_default();
    if parts.is_empty() { "no checks".into() } else { parts.join(", ") }
}

fn overview(b: &Book, f: &Filters) -> View {
    let m = |k: &str| b.meta(k).unwrap_or_default();
    let mut v = View {
        page: "overview".into(),
        title: format!("World {} (seed {})", m("world_file"), m("seed")),
        ..Default::default()
    };
    let written = m("world_written");
    v.verdict.push(format!(
        "Seasons {}-{}, world date {}, built at commit {}{}. Checks: {}.",
        m("first_season"),
        m("last_season"),
        m("world_date"),
        short(&m("world_commit")),
        if written.is_empty() { String::new() } else { format!(" on {written}") },
        marks_line(b, "1")
    ));
    let missing: Vec<String> = serde_json::from_str(&m("missing")).unwrap_or_default();
    if missing.is_empty() {
        v.verdict.push(format!("Logs read: {}.", m("log_streams")));
    } else {
        for x in missing {
            v.verdict.push(format!("Not in this book: {x}."));
        }
    }
    let lim = f.limit(15);
    v.sections.push(query(
        b,
        "Furthest from real",
        "checks",
        "SELECT id, page, title, world, real, mark, issues FROM checks WHERE mark IN ('far off','off') ORDER BY mark = 'far off' DESC, gap DESC",
        lim,
    ));
    v.sections.push(query(
        b,
        "Pages",
        "table",
        "SELECT c.page, COUNT(*) AS checks, SUM(c.mark = 'near') AS near, SUM(c.mark = 'off') AS off, SUM(c.mark = 'far off') AS far_off,
            SUM(c.mark IN ('no data','no real')) AS unmarked FROM checks c GROUP BY c.page ORDER BY c.page",
        50,
    ));
    v.sections.push(query(
        b,
        "Odd things",
        "odd",
        "SELECT level, kind, COUNT(*) AS n FROM odd GROUP BY level, kind ORDER BY level, n DESC",
        50,
    ));
    v.sections.push(query(
        b,
        "Coverage of the inventory",
        "table",
        "SELECT status, COUNT(*) AS areas FROM coverage GROUP BY status ORDER BY areas DESC",
        20,
    ));
    v.sections.push(query(
        b,
        "Consistency",
        "table",
        "SELECT status, COUNT(*) AS rules, SUM(failures) AS failures FROM consistency GROUP BY status",
        20,
    ));
    v.next = PAGES.iter().skip(1).map(|p| format!("{} ({})", p.id, p.title)).collect();
    v.next.push("checks mark=far off, odd level=one, coverage status=not stored, search <name>".into());
    v
}

fn short(c: &str) -> &str {
    &c[..c.len().min(10)]
}

fn topic(b: &Book, id: &str, f: &Filters) -> View {
    let p = PAGES.iter().find(|p| p.id == id).expect("page");
    let mut v = View { page: id.into(), title: p.title.into(), ..Default::default() };
    let on = format!("page = {}", lit(id));
    v.verdict.push(format!("Checks on this page: {}.", marks_line(b, &on)));
    let lim = f.limit(40);
    // Checks, with the year's own value when a year is picked.
    let year_col = match f.year {
        Some(y) => format!(", (SELECT world FROM check_years y WHERE y.check_id = c.id AND y.year = {y}) AS in_{y}"),
        None => String::new(),
    };
    v.sections.push(query(
        b,
        "Checks against real",
        "checks",
        &format!(
            "SELECT id, title, world{year_col}, real, mark, real_source, issues FROM checks c WHERE {on}
             ORDER BY CASE mark WHEN 'far off' THEN 0 WHEN 'off' THEN 1 WHEN 'near' THEN 2 ELSE 3 END, gap DESC"
        ),
        lim,
    ));
    v.sections.push(query(
        b,
        "What each check says",
        "text",
        &format!("SELECT id, says FROM checks WHERE {on} AND mark IN ('far off','off','no data')"),
        lim,
    ));
    // Distributions.
    let grp = f.groups();
    let yr = if f.any_year() { f.years("year") } else { " AND year IS NULL".into() };
    let pot = if f.debug { "" } else { " AND metric NOT LIKE '%potential%'" };
    v.sections.push(query(
        b,
        "Distributions",
        "dist",
        &format!(
            "SELECT metric, year, grp_kind, grp, n, mean, sd, p10, p50, p90 FROM dist WHERE {on}{grp}{yr}{pot}
             ORDER BY metric, year, grp_kind = 'all' DESC, grp_kind, n DESC"
        ),
        f.limit(80),
    ));
    let yo = f.years("year");
    v.sections.push(query(
        b,
        "Odd things",
        "odd",
        &format!(
            "SELECT level, kind, source, entity_id, year, says FROM odd WHERE {on}{} ORDER BY level = 'league' DESC, kind, pct DESC",
            if yo.is_empty() { String::new() } else { format!(" AND (year IS NULL OR (1{yo}))") }
        ),
        lim,
    ));
    v.sections.push(query(
        b,
        "Panels (afm_read <book> area <code> prints an area's whole)",
        "table",
        &format!("SELECT area, id, title, kind, n AS rows, says FROM panels WHERE {on} ORDER BY rowid"),
        200,
    ));
    let areas = pages::expand(p.areas).iter().map(|a| lit(a)).collect::<Vec<_>>().join(",");
    v.sections.push(query(
        b,
        "Coverage",
        "table",
        &format!(
            "SELECT code, title, status, checks, metrics, tables FROM coverage WHERE code IN ({areas})
             ORDER BY substr(code, 1, 1), CAST(substr(code, 2) AS INTEGER)"
        ),
        200,
    ));
    v.next.push(format!(
        "check <id> for one check with its years, source and SQL; dist <metric> for a whole spread; odd page={id}"
    ));
    v
}

/// One inventory area: its coverage, its checks and every panel (lane R6).
fn area(b: &Book, code: &str, f: &Filters) -> View {
    let mut v = View { page: format!("area {code}"), title: format!("Area {code}"), ..Default::default() };
    if let Some(t) =
        b.text_of(&format!("SELECT title || ' [' || status || ']' FROM coverage WHERE code = {}", lit(code)))
    {
        v.title = format!("{code} {t}");
    }
    let on = format!("(' ' || REPLACE(areas, ',', ' ') || ' ') LIKE '% {code} %'");
    v.verdict.push(format!("Checks on this area: {}.", marks_line(b, &on)));
    v.sections.push(query(
        b,
        "Checks against real",
        "checks",
        &format!("SELECT id, title, world, real, mark, says FROM checks WHERE {on} ORDER BY gap DESC"),
        f.limit(40),
    ));
    let ids: Vec<String> = b
        .rows(&format!("SELECT id FROM panels WHERE area = {} ORDER BY rowid", lit(code)))
        .map(|(_, r)| r.iter().map(|x| cell(&x[0])).collect())
        .unwrap_or_default();
    for id in ids {
        v.sections.extend(panel_sections(b, &id, None, f));
    }
    if let Some(t) = b.text_of(&format!("SELECT needs_new FROM coverage WHERE code = {}", lit(code)))
        && let Ok(list) = serde_json::from_str::<Vec<String>>(&t)
        && !list.is_empty()
    {
        v.sections.push(note("Not stored yet", list));
    }
    v.next.push("panel <id> for one panel whole; area <code> for another area".into());
    v
}

/// One panel, or every panel of an entity page (`class`, `state`, `conference`) with
/// only the rows for its key.
fn panel_page(b: &Book, what: &str, key: Option<(&str, &str)>, f: &Filters) -> View {
    let mut v = View { page: format!("{what} {}", key.map_or("", |k| k.1)), ..Default::default() };
    match key {
        Some((col, val)) => {
            v.title = match what {
                "class" => format!("The class of {val}"),
                "state" => format!("State {val}"),
                _ => val.to_string(),
            };
            let ids: Vec<String> = b
                .rows(&format!("SELECT id FROM panels WHERE page = {} ORDER BY rowid", lit(what)))
                .map(|(_, r)| r.iter().map(|x| cell(&x[0])).collect())
                .unwrap_or_default();
            for id in ids {
                v.sections.extend(panel_sections(b, &id, Some((col, val)), f));
            }
        }
        None => {
            v.title = format!("Panel {what}");
            v.sections.extend(panel_sections(b, what, None, f));
        }
    }
    v
}

/// A panel as a section: its words line, then its rows (only `col = val`
/// when given and the panel has that column).
fn panel_sections(b: &Book, id: &str, key: Option<(&str, &str)>, f: &Filters) -> Vec<Section> {
    let Ok((_, rows)) =
        b.rows(&format!("SELECT title, kind, note, cols, rows, says, status, more FROM panels WHERE id = {}", lit(id)))
    else {
        return Vec::new();
    };
    let Some(r) = rows.first() else { return vec![note(id, vec!["no such panel".into()])] };
    let cols: Vec<String> = serde_json::from_str(&cell(&r[3])).unwrap_or_default();
    let mut data: Vec<Vec<serde_json::Value>> = serde_json::from_str(&cell(&r[4])).unwrap_or_default();
    if let Some((col, val)) = key {
        if let Some(i) = cols.iter().position(|c| c == col) {
            data.retain(|row| {
                row.get(i).is_some_and(|x| x.as_str().map_or_else(|| x.to_string(), str::to_string) == val)
            });
        }
        if data.is_empty() {
            return Vec::new();
        }
    }
    let mut s = Section {
        title: format!("{} ({id}, {})", cell(&r[0]), cell(&r[1])),
        kind: "table".into(),
        columns: cols,
        ..Default::default()
    };
    let says = cell(&r[5]);
    if !says.is_empty() {
        s.notes.push(says);
    }
    let status = cell(&r[6]);
    if status != "ok" {
        s.notes.push(status);
    }
    let lim = f.limit(60);
    if data.len() > lim {
        s.more = data.len() - lim;
        data.truncate(lim);
    }
    s.rows = data
        .into_iter()
        .map(|row| {
            row.into_iter()
                .map(|x| match x {
                    serde_json::Value::Null => S::Null,
                    serde_json::Value::Number(n) => {
                        n.as_i64().map_or_else(|| S::Real(n.as_f64().unwrap_or(0.0)), S::Integer)
                    }
                    serde_json::Value::String(t) => S::Text(t),
                    other => S::Text(other.to_string()),
                })
                .collect()
        })
        .collect();
    vec![s]
}

fn checks(b: &Book, f: &Filters) -> View {
    let mut v = View { page: "checks".into(), title: "Every check against real".into(), ..Default::default() };
    let mut w = String::from("1");
    for k in ["mark", "page", "needs"] {
        if let Some(x) = f.get(k) {
            let _ = write!(w, " AND {k} = {}", lit(x));
        }
    }
    v.verdict.push(marks_line(b, &w));
    v.sections.push(query(
        b,
        "Checks",
        "checks",
        &format!("SELECT id, page, title, world, real, mark, tolerance, real_source, real_years FROM checks WHERE {w} ORDER BY page, id"),
        f.limit(500),
    ));
    v.next.push("check <id>".into());
    v
}

fn check(b: &Book, id: &str, f: &Filters) -> View {
    let mut v = View { page: format!("check {id}"), title: format!("Check {id}"), ..Default::default() };
    let q = format!("SELECT * FROM checks WHERE id = {}", lit(id));
    let Ok((names, rows)) = b.rows(&q) else { return v };
    let Some(r) = rows.first() else {
        v.verdict.push(format!("No check called {id}; see `checks`."));
        return v;
    };
    let get = |n: &str| names.iter().position(|x| x == n).map(|i| r[i].clone()).unwrap_or(S::Null);
    let t = |n: &str| match get(n) {
        S::Text(s) => s,
        S::Null => String::new(),
        x => cell(&x),
    };
    v.title = t("title");
    v.verdict.push(t("says"));
    v.sections.push(Section {
        title: "Facts".into(),
        kind: "record".into(),
        columns: vec!["what".into(), "value".into()],
        rows: [
            "world",
            "real",
            "mark",
            "gap",
            "tolerance",
            "unit",
            "real_source",
            "real_years",
            "real_status",
            "real_key",
            "page",
            "areas",
            "needs",
            "issues",
        ]
        .iter()
        .map(|k| vec![S::Text(k.to_string()), get(k)])
        .collect(),
        ..Default::default()
    });
    v.sections.push(note("How the world's number is worked out", vec![t("how"), t("detail")]));
    let yr = f.years("year");
    v.sections.push(query(
        b,
        "By year",
        "table",
        &format!("SELECT year, world FROM check_years WHERE check_id = {}{yr} ORDER BY year", lit(id)),
        200,
    ));
    let issues = t("issues");
    if !issues.is_empty() {
        let ids: Vec<String> = issues.split([',', ' ']).filter(|x| !x.is_empty()).map(lit).collect();
        v.sections.push(query(
            b,
            "Known issues that touch it",
            "table",
            &format!("SELECT id, status, live, title, live_why FROM known_issues WHERE id IN ({})", ids.join(",")),
            50,
        ));
    }
    v.sections.push(note("SQL", vec![t("sql"), t("years_sql")]));
    v.next.push(format!("sql <query> to rerun or change it; {} for the page", t("page")));
    v
}

fn odd(b: &Book, f: &Filters) -> View {
    let mut v = View { page: "odd".into(), title: "Odd things".into(), ..Default::default() };
    let mut w = String::from("1");
    for k in ["level", "kind", "page", "source"] {
        if let Some(x) = f.get(k) {
            let _ = write!(w, " AND {k} = {}", lit(x));
        }
    }
    w.push_str(&f.years("year"));
    v.sections.push(query(
        b,
        "Counts",
        "table",
        &format!("SELECT level, kind, COUNT(*) AS n FROM odd WHERE {w} GROUP BY level, kind"),
        50,
    ));
    v.sections.push(query(
        b,
        "Odd things",
        "odd",
        &format!("SELECT level, kind, page, source, what, entity_id, year, says FROM odd WHERE {w} ORDER BY level = 'league' DESC, level, kind, pct DESC"),
        f.limit(100),
    ));
    v.sections.push(query(
        b,
        "Consistency checks",
        "table",
        "SELECT id, title, status, checked, failures, why FROM consistency ORDER BY status DESC, id",
        100,
    ));
    v.next.push("odd level=one source=<rule id>; <kind> <id> for the record".into());
    v
}

/// A table listed with equality filters on the named columns.
fn listing(b: &Book, title: &str, page: &str, sql: &str, cols: &[&str], f: &Filters) -> View {
    let mut v = View { page: page.into(), title: title.into(), ..Default::default() };
    let mut w = String::from("1");
    for k in cols {
        if let Some(x) = f.get(k) {
            let _ = write!(w, " AND {k} = {}", lit(x));
        }
    }
    v.sections.push(query(b, title, "table", &format!("SELECT * FROM ({sql}) WHERE {w}"), f.limit(500)));
    v
}

fn coverage(b: &Book, f: &Filters) -> View {
    let mut v = View { page: "coverage".into(), title: "Coverage of the inventory".into(), ..Default::default() };
    let mut w = String::from("1");
    for k in ["status", "page", "level"] {
        if let Some(x) = f.get(k) {
            let _ = write!(w, " AND {k} = {}", lit(x));
        }
    }
    v.sections.push(query(
        b,
        "By status",
        "table",
        &format!("SELECT status, COUNT(*) AS areas FROM coverage WHERE {w} GROUP BY status"),
        20,
    ));
    v.sections.push(query(
        b,
        "Areas",
        "table",
        &format!(
            "SELECT code, page, title, status, checks, metrics, rules, tables, note FROM coverage WHERE {w}
             ORDER BY substr(code, 1, 1), CAST(substr(code, 2) AS INTEGER)"
        ),
        f.limit(300),
    ));
    v.sections.push(query(
        b,
        "Stores",
        "table",
        "SELECT source, store, tables, rows, what FROM coverage_stores ORDER BY source, rows DESC",
        300,
    ));
    v
}

fn hidden(f: &Filters, c: &str) -> bool {
    !f.debug && (c.contains("potential") || c.contains("ceiling"))
}

fn table_page(b: &Book, name: &str, f: &Filters) -> View {
    let mut v = View { page: format!("table {name}"), title: format!("Table {name}"), ..Default::default() };
    let cols = b.columns(name);
    if cols.is_empty() {
        v.verdict.push(format!("No table called {name}; see `tables`."));
        return v;
    }
    let shown: Vec<String> =
        cols.iter().filter(|c| !hidden(f, c) && *c != "_more").map(|c| format!("\"{c}\"")).collect();
    let mut w = String::from("1");
    for (k, x) in &f.extra {
        if k == "where" {
            let _ = write!(w, " AND ({x})");
        } else if cols.iter().any(|c| c == k) {
            let _ = write!(w, " AND \"{k}\" = {}", lit(x));
        }
    }
    for (k, x) in [
        ("player__position", &f.position),
        ("division", &f.division),
        ("conference", &f.conference),
        ("stage", &f.level),
    ] {
        if let Some(x) = x
            && cols.iter().any(|c| c == k)
        {
            let _ = write!(w, " AND \"{k}\" = {}", lit(x));
        }
    }
    if let Some(x) = f.stars
        && cols.iter().any(|c| c == "stars")
    {
        let _ = write!(w, " AND stars = {x}");
    }
    for yc in ["year", "season", "class_of"] {
        if cols.iter().any(|c| c == yc) {
            w.push_str(&f.years(yc));
            break;
        }
    }
    let what = b.text_of(&format!("SELECT what FROM catalog WHERE tbl = {}", lit(name))).unwrap_or_default();
    let n = b.num(&format!("SELECT COUNT(*) FROM \"{name}\" WHERE {w}")).unwrap_or(0.0);
    v.verdict.push(format!("{what} {} rows match, {} columns.", n as i64, cols.len()));
    let order = f.get("order").map(|o| format!(" ORDER BY {o}")).unwrap_or_default();
    v.sections.push(query(
        b,
        "Rows",
        "table",
        &format!("SELECT {} FROM \"{name}\" WHERE {w}{order}", shown.join(",")),
        f.limit(30),
    ));
    v.sections.push(note("Columns", vec![cols.join(", ")]));
    let kids: Vec<String> =
        b.tables().into_iter().filter(|t| t.strip_prefix(name).is_some_and(|r| r.starts_with("__"))).collect();
    if !kids.is_empty() {
        v.next.push(format!("child tables (join on _parent = {name}._row): {}", kids.join(", ")));
    }
    v.next.push(format!("record {name} <_row> for one whole row; where=<sql> order=<sql> limit=N"));
    v
}

fn dist(b: &Book, id: &str, f: &Filters) -> View {
    let mut v = View { page: format!("dist {id}"), title: format!("Distribution {id}"), ..Default::default() };
    let m = b
        .rows(&format!("SELECT title, unit, rows, note, sql FROM metrics WHERE id = {}", lit(id)))
        .ok()
        .and_then(|(_, r)| r.into_iter().next());
    let Some(m) = m else {
        v.verdict.push(format!("No distribution called {id}."));
        v.sections.push(query(
            b,
            "Distributions",
            "table",
            "SELECT id, page, title, rows FROM metrics ORDER BY page, id",
            200,
        ));
        return v;
    };
    v.title = format!("{} ({})", cell(&m[0]), cell(&m[1]));
    v.verdict.push(format!("{} values. {}", cell(&m[2]), cell(&m[3])));
    let grp = f.groups();
    let yr = if f.any_year() { f.years("year") } else { String::new() };
    v.sections.push(query(
        b,
        "By group and year",
        "dist",
        &format!(
            "SELECT year, grp_kind, grp, n, mean, sd, min, p1, p5, p10, p25, p50, p75, p90, p95, p99, max FROM dist
             WHERE metric = {}{grp}{yr} ORDER BY year IS NOT NULL, year, grp_kind = 'all' DESC, grp_kind, n DESC",
            lit(id)
        ),
        f.limit(200),
    ));
    v.sections.push(query(
        b,
        "Histogram (all years)",
        "table",
        &format!("SELECT grp_kind, grp, lo, hi, n FROM hist WHERE metric = {}{grp} AND (grp_kind = 'all' OR {}) ORDER BY grp_kind, grp, lo", lit(id), !grp.is_empty()),
        f.limit(200),
    ));
    v.sections.push(note("SQL", vec![cell(&m[4])]));
    v
}

fn record(b: &Book, t: &str, row: i64, f: &Filters) -> View {
    let mut v = View { page: format!("record {t} {row}"), title: format!("{t} row {row}"), ..Default::default() };
    match b.record(t, row) {
        Some(mut r) => {
            if !f.debug {
                strip(&mut r);
            }
            let text = serde_json::to_string_pretty(&r).unwrap_or_default();
            v.sections.push(note("Record", vec![text]));
        }
        None => v.verdict.push(format!("No row {row} in {t}.")),
    }
    v
}

/// Takes true potential and ceilings out of a record.
fn strip(v: &mut J) {
    match v {
        J::Object(o) => {
            o.retain(|k, _| !(k.contains("potential") || k.contains("ceiling")));
            o.values_mut().for_each(strip);
        }
        J::Array(a) => a.iter_mut().for_each(strip),
        _ => {}
    }
}

fn found(b: &Book, kind: &str, id: &str, f: &Filters) -> View {
    let q = format!("SELECT tbl, row FROM search WHERE kind = {} AND id = {} LIMIT 1", lit(kind), lit(id));
    match b.rows(&q).ok().and_then(|(_, r)| r.into_iter().next()) {
        Some(r) => {
            let row = match r[1] {
                S::Integer(i) => i,
                _ => 0,
            };
            let t = cell(&r[0]);
            let mut v = record(b, &t, row, f);
            v.title = format!("{kind} {id}");
            // The world log's yearly rows and the odd list for it.
            v.sections.push(query(
                b,
                "Odd things about it",
                "odd",
                &format!("SELECT level, kind, source, year, says FROM odd WHERE entity_id = {}", lit(id)),
                50,
            ));
            if kind == "person" && b.has("log_players") {
                let cols = b.columns("log_players");
                if cols.iter().any(|c| c == "id") {
                    v.sections.push(query(
                        b,
                        "World log, by year",
                        "table",
                        &format!("SELECT * FROM log_players WHERE id = {} ORDER BY season", lit(id)),
                        60,
                    ));
                }
            }
            v
        }
        None => View {
            page: format!("{kind} {id}"),
            title: format!("No {kind} {id}"),
            verdict: vec!["Try `search <name>`.".into()],
            ..Default::default()
        },
    }
}

fn search(b: &Book, text: &str, f: &Filters) -> View {
    let mut v = View { page: format!("search {text}"), title: format!("Search: {text}"), ..Default::default() };
    let like = lit(&format!("%{text}%"));
    let kind = f.get("kind").map(|k| format!(" AND kind = {}", lit(k))).unwrap_or_default();
    v.sections.push(query(
        b,
        "Found",
        "table",
        &format!("SELECT kind, id, name, detail, first_year, last_year FROM search WHERE (name LIKE {like} OR id = {}){kind} ORDER BY name = {} DESC, kind, name", lit(text), lit(text)),
        f.limit(50),
    ));
    v.next.push("<kind> <id> opens one".into());
    v
}
