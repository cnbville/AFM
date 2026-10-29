//! A world's book (lane R4's SQLite file) as the app reads it: its tables and
//! columns, rows paged through with the filters and the year slider, search,
//! entities and every row that points at one. Everything here reads the
//! book's own schema, so a table R4 or a page lane adds shows up without a
//! change here.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use rusqlite::types::{Value as Sql, ValueRef};
use rusqlite::{Connection, OpenFlags};
use serde_json::{Map, Value, json};

/// One table or view of the book.
pub struct Table {
    pub name: String,
    pub cols: Vec<String>,
}

impl Table {
    pub fn has(&self, c: &str) -> bool {
        self.cols.iter().any(|x| x == c)
    }

    /// The first of these columns the table has.
    pub fn pick<'a>(&self, names: &[&'a str]) -> Option<&'a str> {
        names.iter().copied().find(|c| self.has(c))
    }
}

/// What the filters bar and the year slider ask for. Each filter applies to
/// a table only when the table has a column for it.
#[derive(Default, Clone)]
pub struct Filters {
    pub level: Vec<String>,
    pub division: Vec<String>,
    pub conference: Vec<String>,
    pub position: Vec<String>,
    pub stars: Vec<String>,
    pub state: Vec<String>,
    pub from: Option<i64>,
    pub to: Option<i64>,
    /// The year slider: the world as it stood that year.
    pub year: Option<i64>,
    /// Hidden truths shown (true potential, ceilings, hidden ratings).
    pub debug: bool,
}

/// Filter keys and the columns that carry them.
pub const FILTER_COLS: [(&str, &[&str]); 6] = [
    ("level", &["level"]),
    ("division", &["division", "div"]),
    ("conference", &["conference", "conf"]),
    ("position", &["position", "pos"]),
    ("stars", &["stars"]),
    ("state", &["state", "home_state"]),
];
pub const YEAR_COLS: [&str; 3] = ["year", "season", "draft_year"];
const SNAP_COLS: [&str; 3] = ["as_of", "snapshot", "snapshot_year"];
const SPAN_COLS: [(&str, &str); 3] =
    [("first_year", "last_year"), ("from_year", "to_year"), ("start_year", "end_year")];
pub const NAME_COLS: [&str; 5] = ["name", "full_name", "title", "label", "code"];

/// The column that picks one row of a table: its `id`, a person's
/// `player__id` (R4 spreads the player into `player__` columns), or the
/// book's own `_row` (child tables, log rows, clubs, states).
pub fn id_of(t: &Table) -> &'static str {
    if t.name.starts_with("log_") || t.has("_parent") {
        return "_row";
    }
    ["id", "player__id", "_row"].into_iter().find(|c| t.has(c)).unwrap_or("rowid")
}

/// What a column of the book's own naming points at, by its last word:
/// `head_coach`, `job__employer__College`, `person__hs`, a college game's
/// `home` and `away`, a log row's `id`.
fn link_word(table: &str, col: &str) -> Option<&'static str> {
    let word = col.rsplit("__").next().unwrap_or(col);
    // A coach only where the whole column is one (`train__year__coach` is
    // something else).
    if matches!(col, "head_coach" | "coach" | "oc" | "dc" | "hc") {
        return Some("coaches");
    }
    if matches!(col, "pro__team" | "pro__contract__team") {
        return Some("nfl");
    }
    Some(match word {
        "college" | "program" | "College" | "transfer_from" => "programs",
        "home" | "away" if table.starts_with("college") && col == word => "programs",
        "hs" | "high_school" | "HighSchool" => "high_schools",
        "Nfl" => "nfl",
        "home_state" | "state" => "states",
        "id" => match table {
            "log_coaches" => "coaches",
            "log_programs" => "programs",
            "log_schools" => "high_schools",
            // The tables whose `id` is a person's (measured on R4's test
            // book: 87-100% of them are people on record).
            "log_players"
            | "log_growth"
            | "log_people"
            | "log_exits"
            | "log_whys"
            | "college__heisman_vote"
            | "college_prev__heisman_vote"
            | "college__teams__players"
            | "folk__courts__closed"
            | "folk__courts__open"
            | "folk__discipline__recent"
            | "folk__life__events"
            | "recruiting_log__portal"
            | "scouting__rankings__picks"
            | "scouting__rankings__tops"
            | "stats__seasons"
            | "stories__marks" => "people",
            _ => return None,
        },
        _ => return None,
    })
}

impl Filters {
    pub fn from_json(v: &Value, debug: bool) -> Filters {
        let list = |k: &str| -> Vec<String> {
            match &v[k] {
                Value::Array(a) => a.iter().filter_map(cell_text).filter(|s| !s.is_empty()).collect(),
                Value::Null => Vec::new(),
                x => cell_text(x)
                    .map(|s| s.split(',').map(|p| p.trim().to_string()).filter(|p| !p.is_empty()).collect())
                    .unwrap_or_default(),
            }
        };
        let num = |k: &str| v[k].as_i64().or_else(|| v[k].as_str().and_then(|s| s.trim().parse().ok()));
        Filters {
            level: list("level"),
            division: list("division"),
            conference: list("conference"),
            position: list("position"),
            stars: list("stars"),
            state: list("state"),
            from: num("from"),
            to: num("to"),
            year: num("year"),
            debug: v["debug"].as_bool().unwrap_or(debug),
        }
    }

    fn values(&self, key: &str) -> &[String] {
        match key {
            "level" => &self.level,
            "division" => &self.division,
            "conference" => &self.conference,
            "position" => &self.position,
            "stars" => &self.stars,
            _ => &self.state,
        }
    }

    /// The filters in words, for headers and the copied text.
    pub fn words(&self) -> String {
        let mut w: Vec<String> = Vec::new();
        for (k, _) in FILTER_COLS {
            let v = self.values(k);
            if !v.is_empty() {
                w.push(format!("{k} {}", v.join(", ")));
            }
        }
        match (self.from, self.to) {
            (Some(a), Some(b)) => w.push(format!("years {a}-{b}")),
            (Some(a), None) => w.push(format!("from {a}")),
            (None, Some(b)) => w.push(format!("to {b}")),
            _ => {}
        }
        if let Some(y) = self.year {
            w.push(format!("as it stood in {y}"));
        }
        if !self.debug {
            w.push("hidden truths off".into());
        }
        w.join(" · ")
    }

    pub fn any(&self) -> bool {
        FILTER_COLS.iter().any(|(k, _)| !self.values(k).is_empty())
            || self.from.is_some()
            || self.to.is_some()
            || self.year.is_some()
    }
}

/// A cell as plain text (for filters and matching).
pub fn cell_text(v: &Value) -> Option<String> {
    match v {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

/// Whether a column holds a hidden truth: true potential, ceilings, hidden
/// ratings and traits. The book can mark more in its `defs` table.
pub fn debug_name(col: &str) -> bool {
    let c = col.to_ascii_lowercase();
    // What the engine keeps behind `debug.rs`: the hidden traits, bloom,
    // a coach's true ability parts, a recruit's visibility, sportsmanship.
    let last = c.rsplit("__").next().unwrap_or(&c);
    if matches!(
        last,
        "consistency"
            | "temperament"
            | "recovery"
            | "injury_proneness"
            | "longevity"
            | "bloom"
            | "visibility"
            | "sportsmanship"
    ) || c.starts_with("ability__")
        || c.starts_with("x_")
    {
        return true;
    }
    c.contains("potential")
        || c.contains("ceiling")
        || c.starts_with("true_")
        || c.starts_with("hidden")
        || c.ends_with("_hidden")
        || c.ends_with("_debug")
        || c.starts_with("debug_")
        || c == "pot"
}

pub struct Book {
    pub path: PathBuf,
    pub db: Connection,
    pub tables: Vec<Table>,
    counts: HashMap<String, i64>,
    /// (table, column) → the number's definition, from the book's `defs`.
    pub defs: HashMap<(String, String), Map<String, Value>>,
}

/// Table names the reader looks for, in order, for each kind of thing.
pub fn names_for(kind: &str) -> &'static [&'static str] {
    match kind {
        "meta" => &["book_meta", "meta", "world_meta"],
        "checks" => &["checks", "check_results", "results"],
        "odd" => &["odd", "odd_things", "odd_list"],
        "issues" => &["known_issues", "issues"],
        "catalog" => &["catalog"],
        "dist" => &["dist"],
        "metrics" => &["metrics"],
        "check_years" => &["check_years"],
        "knobs" => &["knobs", "tuning_knobs"],
        "coverage" => &["coverage", "coverage_map"],
        "real" => &["real_figures", "real", "real_index", "real_refs"],
        "fitted" => &["fitted", "fitted_numbers", "world_numbers"],
        "whys" => &["log_whys", "whys", "why", "events"],
        "consistency" => &["consistency", "consistency_checks"],
        "defs" => &["defs", "definitions", "columns"],
        "search" => &["search", "search_index", "names"],
        "series" => &["series", "year_series"],
        "people" => &["people", "persons", "players", "person"],
        "seasons" => &["person_seasons", "player_seasons", "seasons", "development", "dev_rows"],
        "games" => &["games", "game", "college__games"],
        "plays" => &["plays", "play_by_play", "snaps"],
        "programs" => &["programs", "program"],
        "coaches" => &["coaches", "coach"],
        "real_players" => &["real_players", "nfl_careers", "real_nfl_careers", "real_careers"],
        "seat" => &["seat", "career", "you", "seat_career"],
        _ => &[],
    }
}

/// Kinds of the book's own worked-out lists, as opposed to the world's
/// records.
const TOOL_KINDS: [&str; 15] = [
    "meta",
    "checks",
    "odd",
    "issues",
    "knobs",
    "coverage",
    "fitted",
    "whys",
    "consistency",
    "defs",
    "search",
    "catalog",
    "dist",
    "metrics",
    "check_years",
];

/// The tables an `<x>_id` column may point at, beyond `x`, `xs`, `xes`.
fn link_names(base: &str) -> &'static [&'static str] {
    match base {
        "person" | "player" | "alumnus" | "pro" | "pid" | "recruit" | "prospect" => &["people", "persons", "players"],
        "coach" | "hc" | "head_coach" => &["coaches"],
        "program" | "college" | "prog" => &["programs"],
        "school" | "hs" | "high_school" => &["high_schools", "schools"],
        "club" | "team" | "franchise" | "nfl_team" => &["clubs", "nfl_clubs", "teams", "franchises"],
        "conference" | "conf" => &["conferences"],
        "game" => &["games"],
        "agent" => &["agents"],
        "state" => &["states"],
        _ => &[],
    }
}

impl Book {
    pub fn open(path: &Path) -> Result<Book, String> {
        let flags = OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX | OpenFlags::SQLITE_OPEN_URI;
        let db = Connection::open_with_flags(path, flags).map_err(|e| format!("Could not open the book: {e}"))?;
        // `more()` reads the world log's packed rows (R4's `Book::with`).
        let db = afm_reader::book::Book::with(db).map_err(|e| format!("Could not open the book: {e}"))?.conn;
        // Big pages read faster with a bigger cache (64 MB) and memory-mapped
        // reads (up to 1 GB); the book is never written here.
        let _ = db.execute_batch("PRAGMA cache_size = -65536; PRAGMA mmap_size = 1073741824; PRAGMA query_only = 1;");
        let mut names: Vec<String> = {
            let mut st = db
                .prepare("SELECT name FROM sqlite_master WHERE type IN ('table','view') AND name NOT LIKE 'sqlite_%' ORDER BY name")
                .map_err(|e| format!("That file is not a world book ({e})."))?;
            st.query_map([], |r| r.get::<_, String>(0)).map_err(|e| e.to_string())?.filter_map(Result::ok).collect()
        };
        names.retain(|n| !n.starts_with('_'));
        let mut tables = Vec::new();
        for name in names {
            let mut st = db.prepare(&format!("PRAGMA table_info({})", q(&name))).map_err(|e| e.to_string())?;
            let cols: Vec<String> = st
                .query_map([], |r| r.get::<_, String>(1))
                .map_err(|e| e.to_string())?
                .filter_map(Result::ok)
                .collect();
            tables.push(Table { name, cols });
        }
        if tables.is_empty() {
            return Err("That file has no tables: it is not a world book.".into());
        }
        let mut b = Book { path: path.to_path_buf(), db, tables, counts: HashMap::new(), defs: HashMap::new() };
        b.load_defs();
        Ok(b)
    }

    fn load_defs(&mut self) {
        let Some(t) = self.find("defs") else { return };
        let (Some(tc), Some(cc)) =
            (t.pick(&["tbl", "table_name", "table"]), t.pick(&["col", "column_name", "column", "key"]))
        else {
            return;
        };
        let (tc, cc, name) = (tc.to_string(), cc.to_string(), t.name.clone());
        let rows = self.select_maps(&format!("SELECT * FROM {}", q(&name)), &[]).unwrap_or_default();
        for r in rows {
            let key = (
                r.get(&tc).and_then(cell_text).unwrap_or_default(),
                r.get(&cc).and_then(cell_text).unwrap_or_default(),
            );
            self.defs.insert(key, r);
        }
    }

    pub fn table(&self, name: &str) -> Option<&Table> {
        self.tables.iter().find(|t| t.name == name)
    }

    /// The book's table for a kind of thing (`names_for`).
    pub fn find(&self, kind: &str) -> Option<&Table> {
        names_for(kind).iter().find_map(|n| self.table(n))
    }

    /// Rows in a table (cached; the book never changes while it is open).
    pub fn count(&mut self, table: &str) -> i64 {
        if let Some(n) = self.counts.get(table) {
            return *n;
        }
        let n =
            self.db.query_row(&format!("SELECT COUNT(*) FROM {}", q(table)), [], |r| r.get::<_, i64>(0)).unwrap_or(0);
        self.counts.insert(table.to_string(), n);
        n
    }

    /// Whether a column is a hidden truth (marked in `defs`, or by its name).
    pub fn is_debug(&self, table: &str, col: &str) -> bool {
        if let Some(d) = self.defs.get(&(table.to_string(), col.to_string()))
            && let Some(v) = d.get("debug")
        {
            return v.as_bool().unwrap_or(false) || v.as_i64().unwrap_or(0) != 0;
        }
        debug_name(col)
    }

    pub fn is_debug_table(&self, table: &str) -> bool {
        let t = table.to_ascii_lowercase();
        t.contains("hidden") || t.contains("debug")
    }

    /// What a column points at: `_parent` its parent table's row, an
    /// `<x>_id` column the table named by `x`, or one of the book's own
    /// names (`link_word`).
    pub fn link_target(&self, table: &str, col: &str) -> Option<&str> {
        let found = |c: &str| self.tables.iter().find(|t| t.name == c).map(|t| t.name.as_str());
        if col == "_parent" {
            // `people__player__history` → `people`: the nearest table up.
            let mut up = table;
            while let Some((a, _)) = up.rsplit_once("__") {
                if let Some(t) = found(a) {
                    return Some(t);
                }
                up = a;
            }
            return None;
        }
        if let Some(t) = link_word(table, col).and_then(found).filter(|t| *t != table || col != "id") {
            return Some(t);
        }
        let base = col.strip_suffix("_id")?;
        let mut cands: Vec<String> = vec![base.into(), format!("{base}s"), format!("{base}es")];
        if let Some(stem) = base.strip_suffix('y') {
            cands.push(format!("{stem}ies"));
        }
        cands.extend(link_names(base).iter().map(|s| s.to_string()));
        // `home_team_id`, `drafted_by_club_id`: the last word names the table.
        if let Some((_, last)) = base.rsplit_once('_') {
            cands.push(format!("{last}s"));
            cands.extend(link_names(last).iter().map(|s| s.to_string()));
        }
        cands.into_iter().find_map(|c| found(&c))
    }

    /// The column of `target` a link from `table.col` matches: `_row` for
    /// a child's `_parent`, else the target's id.
    pub fn link_key(&self, col: &str, target: &str) -> &'static str {
        if col == "_parent" { "_row" } else { self.table(target).map_or("id", id_of) }
    }

    /// The column that names a row of this table, as SQL.
    pub fn name_sql(&self, t: &Table) -> Option<String> {
        if t.has("real_name") && t.has("fictional_name") {
            return Some("COALESCE(\"real_name\", \"fictional_name\")".into());
        }
        if let Some(c) = t.pick(&NAME_COLS) {
            return Some(q(c));
        }
        let first = t.pick(&["first_name", "first"])?;
        let last = t.pick(&["last_name", "last"])?;
        Some(format!("({} || ' ' || {})", q(first), q(last)))
    }

    /// Tables whose rows are things with a page: an `id` and a name.
    pub fn entity_tables(&self) -> Vec<&Table> {
        self.tables
            .iter()
            .filter(|t| {
                (t.has("id") || t.has("player__id"))
                    && self.name_sql(t).is_some()
                    && !t.name.starts_with("log_")
                    && !TOOL_KINDS.iter().any(|k| names_for(k).contains(&t.name.as_str()))
            })
            .collect()
    }

    /// The name of one row, for links.
    pub fn name_of(&self, table: &str, id: &Value) -> Option<String> {
        let t = self.table(table)?;
        let n = self.name_sql(t)?;
        let sql = format!("SELECT {n} FROM {} WHERE {} = ?1", q(table), q(id_of(t)));
        self.db.query_row(&sql, [json_sql(id)], |r| r.get::<_, Option<String>>(0)).ok().flatten()
    }

    /// The SQL condition and its values for one table under the filters.
    pub fn where_for(&self, t: &Table, f: &Filters) -> (String, Vec<Sql>) {
        let mut parts: Vec<String> = Vec::new();
        let mut vals: Vec<Sql> = Vec::new();
        // The book's own lists (checks, odd things, coverage, ...) have
        // columns such as `level` that mean something else: only the years
        // narrow them.
        let tool = TOOL_KINDS.iter().any(|k| names_for(k).contains(&t.name.as_str()));
        for (key, cols) in FILTER_COLS {
            if tool {
                continue;
            }
            let want = f.values(key);
            if want.is_empty() {
                continue;
            }
            let Some(c) = t.pick(cols) else { continue };
            let marks: Vec<String> = want
                .iter()
                .map(|w| {
                    vals.push(Sql::Text(w.clone()));
                    format!("?{}", vals.len())
                })
                .collect();
            parts.push(format!("CAST({} AS TEXT) COLLATE NOCASE IN ({})", q(c), marks.join(",")));
        }
        let year_col = t.pick(&YEAR_COLS);
        if let Some(y) = year_col {
            if let Some(a) = f.from {
                vals.push(Sql::Integer(a));
                parts.push(format!("{} >= ?{}", q(y), vals.len()));
            }
            if let Some(b) = f.to {
                vals.push(Sql::Integer(b));
                parts.push(format!("{} <= ?{}", q(y), vals.len()));
            }
        }
        if let Some(year) = f.year {
            if let Some(s) = t.pick(&SNAP_COLS) {
                vals.push(Sql::Integer(year));
                parts.push(format!("{} = ?{}", q(s), vals.len()));
            } else if let Some(y) = year_col {
                vals.push(Sql::Integer(year));
                parts.push(format!("{} <= ?{}", q(y), vals.len()));
            } else if let Some((a, b)) = SPAN_COLS.iter().find(|(a, b)| t.has(a) && t.has(b)) {
                vals.push(Sql::Integer(year));
                let n = vals.len();
                parts.push(format!(
                    "({a} IS NULL OR {a} <= ?{n}) AND ({b} IS NULL OR {b} >= ?{n})",
                    a = q(a),
                    b = q(b)
                ));
            }
        }
        if parts.is_empty() { (String::new(), vals) } else { (format!(" WHERE {}", parts.join(" AND ")), vals) }
    }

    /// Which filters this table can take, for the filters bar's note.
    pub fn filterable(&self, t: &Table) -> Vec<&'static str> {
        let mut v: Vec<&'static str> =
            FILTER_COLS.iter().filter(|(_, cols)| t.pick(cols).is_some()).map(|(k, _)| *k).collect();
        if t.pick(&YEAR_COLS).is_some() {
            v.push("years");
        }
        if t.pick(&YEAR_COLS).is_some()
            || t.pick(&SNAP_COLS).is_some()
            || SPAN_COLS.iter().any(|(a, b)| t.has(a) && t.has(b))
        {
            v.push("year slider");
        }
        v
    }

    /// Rows as objects (column → value).
    pub fn select_maps(&self, sql: &str, vals: &[Sql]) -> Result<Vec<Map<String, Value>>, String> {
        let mut st = self.db.prepare(sql).map_err(|e| e.to_string())?;
        let names: Vec<String> = st.column_names().iter().map(|s| s.to_string()).collect();
        let rows = st
            .query_map(rusqlite::params_from_iter(vals.iter()), |r| {
                let mut m = Map::new();
                for (i, n) in names.iter().enumerate() {
                    m.insert(n.clone(), cell(r.get_ref(i)?));
                }
                Ok(m)
            })
            .map_err(|e| e.to_string())?;
        Ok(rows.filter_map(Result::ok).collect())
    }

    /// Rows of one table for a page: under the filters, sorted, one page of
    /// them, with each column's facts (hidden truth, link, definition).
    #[allow(clippy::too_many_arguments)]
    pub fn rows(
        &mut self,
        table: &str,
        f: &Filters,
        extra: Option<(&str, &Value)>,
        sort: Option<&str>,
        desc: bool,
        offset: i64,
        limit: i64,
    ) -> Result<Value, String> {
        let t = self.table(table).ok_or_else(|| format!("This book has no table {table}."))?;
        // `_more` is a packed record (msgpack): unreadable as a cell.
        let cols: Vec<String> =
            t.cols.iter().filter(|c| *c != "_more" && (f.debug || !self.is_debug(table, c))).cloned().collect();
        let (mut wh, mut vals) = self.where_for(t, f);
        if let Some((c, v)) = extra {
            if !t.has(c) {
                return Err(format!("{table} has no column {c}."));
            }
            vals.push(json_sql(v));
            let cond = format!("{} = ?{}", q(c), vals.len());
            wh = if wh.is_empty() { format!(" WHERE {cond}") } else { format!("{wh} AND {cond}") };
        }
        let order = match sort.filter(|s| cols.iter().any(|c| c == s)) {
            Some(s) => format!(" ORDER BY {} {}", q(s), if desc { "DESC" } else { "ASC" }),
            None => match t.pick(&YEAR_COLS) {
                Some(y) if extra.is_some() => format!(" ORDER BY {}", q(y)),
                _ => String::new(),
            },
        };
        let list = cols.iter().map(|c| q(c)).collect::<Vec<_>>().join(", ");
        let sql = format!(
            "SELECT {list} FROM {}{wh}{order} LIMIT {} OFFSET {}",
            q(table),
            limit.clamp(1, 1000),
            offset.max(0)
        );
        let maps = self.select_maps(&sql, &vals)?;
        let rows: Vec<Value> = maps
            .iter()
            .map(|m| Value::Array(cols.iter().map(|c| m.get(c).cloned().unwrap_or(Value::Null)).collect()))
            .collect();
        let total = if wh.is_empty() {
            self.count(table)
        } else {
            let sql = format!("SELECT COUNT(*) FROM {}{wh}", q(table));
            self.db.query_row(&sql, rusqlite::params_from_iter(vals.iter()), |r| r.get::<_, i64>(0)).unwrap_or(0)
        };
        let t = self.table(table).expect("checked above");
        let filterable = self.filterable(t);
        // Every id shown as its name: one lookup per linked column.
        let mut names = Map::new();
        for c in &cols {
            let Some(target) = self.link_target(table, c) else { continue };
            let Some(n) = self.table(target).and_then(|tt| self.name_sql(tt)) else { continue };
            let key = self.link_key(c, target);
            let ids: Vec<Sql> = maps.iter().filter_map(|m| m.get(c)).filter(|v| !v.is_null()).map(json_sql).collect();
            if ids.is_empty() {
                continue;
            }
            let marks = (1..=ids.len()).map(|i| format!("?{i}")).collect::<Vec<_>>().join(",");
            let sql = format!("SELECT {} AS id, {n} AS name FROM {} WHERE {} IN ({marks})", q(key), q(target), q(key));
            let mut m = Map::new();
            for r in self.select_maps(&sql, &ids).unwrap_or_default() {
                if let (Some(id), Some(name)) = (r.get("id").and_then(cell_text), r.get("name").and_then(cell_text)) {
                    m.insert(id, Value::String(name));
                }
            }
            names.insert(c.clone(), Value::Object(m));
        }
        Ok(json!({
            "table": table,
            "cols": cols.iter().map(|c| self.col_info(table, c)).collect::<Vec<_>>(),
            "rows": rows,
            "names": names,
            "total": total,
            "offset": offset,
            "filterable": filterable,
        }))
    }

    /// A column's facts for the table view and its hover card.
    pub fn col_info(&self, table: &str, c: &str) -> Value {
        let def = self.defs.get(&(table.to_string(), c.to_string()));
        let mut o = json!({
            "name": c,
            "label": def.and_then(|d| d.get("label")).and_then(cell_text).unwrap_or_else(|| label(c)),
            "debug": self.is_debug(table, c),
        });
        let own = self.table(table).map(id_of);
        if own == Some(c) && self.entity_tables().iter().any(|t| t.name == table) {
            o["link"] = json!(table);
        } else if let Some(t) = self.link_target(table, c)
            && Some(self.link_key(c, t)) == self.table(t).map(id_of)
        {
            o["link"] = json!(t);
        }
        if crate::pages::is_date(c) {
            o["fmt"] = json!("date");
        } else if crate::pages::is_year(c) {
            o["fmt"] = json!("year");
        }
        o["h"] = self.hover(table, c);
        o
    }

    /// The hover card for a column: what it is, how it is worked out (file
    /// and function), the real figure's source and years.
    pub fn hover(&self, table: &str, c: &str) -> Value {
        match self.defs.get(&(table.to_string(), c.to_string())) {
            Some(d) => {
                let g = |ks: &[&str]| ks.iter().find_map(|k| d.get(*k).and_then(cell_text)).unwrap_or_default();
                json!({
                    "what": g(&["what", "means", "description"]),
                    "unit": g(&["unit", "units"]),
                    "file": g(&["file"]),
                    "function": g(&["function", "fn"]),
                    "source": g(&["source", "real_source"]),
                    "years": g(&["years", "real_years"]),
                    "debug": self.is_debug(table, c),
                    "key": format!("{table}.{c}"),
                })
            }
            None => {
                json!({ "what": "", "key": format!("{table}.{c}"), "debug": self.is_debug(table, c), "missing": true })
            }
        }
    }

    /// Search every thing with a page: players, coaches, schools, programs,
    /// clubs, conferences, states. Uses the book's search index when it has
    /// one, else each entity table's names.
    pub fn search(&self, text: &str, limit: usize) -> Vec<Value> {
        let text = text.trim();
        if text.chars().count() < 2 {
            return Vec::new();
        }
        let like = format!("%{}%", text.replace('%', "").replace('_', " "));
        let prefix = format!("{}%", text.replace('%', ""));
        let mut out: Vec<(i32, Value)> = Vec::new();
        if let Some(s) = self.find("search")
            && let (Some(n), Some(k), Some(i)) =
                (s.pick(&["name", "text"]), s.pick(&["tbl", "table_name", "kind"]), s.pick(&["id", "entity_id"]))
        {
            let detail = s.pick(&["detail", "info", "summary"]).map(q).unwrap_or_else(|| "''".into());
            let sql = format!(
                "SELECT {k}, {i}, {n}, {detail} FROM {t} WHERE {n} LIKE ?1 ORDER BY ({n} LIKE ?2) DESC, length({n}) LIMIT {limit}",
                k = q(k),
                i = q(i),
                n = q(n),
                t = q(&s.name)
            );
            if let Ok(mut st) = self.db.prepare(&sql) {
                let rows = st.query_map([&like, &prefix], |r| {
                        Ok(json!({ "table": cell(r.get_ref(0)?), "id": cell(r.get_ref(1)?), "name": cell(r.get_ref(2)?), "detail": cell(r.get_ref(3)?) }))
                    });
                if let Ok(rows) = rows {
                    return rows.filter_map(Result::ok).collect();
                }
            }
        }
        for t in self.entity_tables() {
            let Some(n) = self.name_sql(t) else { continue };
            let detail: Vec<&str> = ["position", "pos", "level", "state", "conference", "year", "class_year"]
                .into_iter()
                .filter(|c| t.has(c))
                .take(3)
                .collect();
            let dsql = if detail.is_empty() {
                "''".to_string()
            } else {
                detail
                    .iter()
                    .map(|c| format!("COALESCE(CAST({} AS TEXT),'')", q(c)))
                    .collect::<Vec<_>>()
                    .join(" || ' · ' || ")
            };
            let sql = format!(
                "SELECT {}, {n}, {dsql}, ({n} LIKE ?2) AS pre FROM {} WHERE {n} LIKE ?1 ORDER BY pre DESC, length({n}) LIMIT {limit}",
                q(id_of(t)),
                q(&t.name)
            );
            let Ok(mut st) = self.db.prepare(&sql) else { continue };
            let rows = st.query_map([&like, &prefix], |r| {
                let pre: i64 = r.get(3).unwrap_or(0);
                Ok((pre as i32, json!({ "table": t.name, "id": cell(r.get_ref(0)?), "name": cell(r.get_ref(1)?), "detail": cell(r.get_ref(2)?) })))
            });
            if let Ok(rows) = rows {
                out.extend(rows.filter_map(Result::ok));
            }
        }
        out.sort_by(|a, b| {
            b.0.cmp(&a.0).then_with(|| a.1["name"].as_str().map(str::len).cmp(&b.1["name"].as_str().map(str::len)))
        });
        out.into_iter().take(limit).map(|x| x.1).collect()
    }

    /// The tables with a column that points at `table`, and that column.
    pub fn pointing_at(&self, table: &str) -> Vec<(String, String)> {
        let mut v = Vec::new();
        for t in &self.tables {
            if t.name == table || TOOL_KINDS.iter().any(|k| names_for(k).contains(&t.name.as_str())) {
                continue;
            }
            // A child table first (`_parent`), else the first column that
            // links here.
            let c = if t.has("_parent") && self.link_target(&t.name, "_parent") == Some(table) {
                Some("_parent".to_string())
            } else {
                t.cols.iter().find(|c| *c != "_parent" && self.link_target(&t.name, c) == Some(table)).cloned()
            };
            if let Some(c) = c {
                v.push((t.name.clone(), c));
            }
        }
        v
    }

    /// Count, mean and percentiles of a numeric column under a condition.
    pub fn stats(&self, table: &str, col: &str, wh: &str, vals: &[Sql]) -> Value {
        let base =
            format!("FROM {}{}{} {} IS NOT NULL", q(table), wh, if wh.is_empty() { " WHERE" } else { " AND" }, q(col));
        let (n, mean): (i64, Option<f64>) = self
            .db
            .query_row(
                &format!("SELECT COUNT(*), AVG({}) {base}", q(col)),
                rusqlite::params_from_iter(vals.iter()),
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap_or((0, None));
        let pct = |p: f64| -> Option<f64> {
            if n == 0 {
                return None;
            }
            let off = ((n - 1) as f64 * p).round() as i64;
            let sql = format!("SELECT {c} {base} ORDER BY {c} LIMIT 1 OFFSET {off}", c = q(col));
            self.db.query_row(&sql, rusqlite::params_from_iter(vals.iter()), |r| r.get::<_, f64>(0)).ok()
        };
        json!({ "n": n, "mean": mean, "p10": pct(0.1), "p50": pct(0.5), "p90": pct(0.9) })
    }

    /// The numeric columns of a table (judged from its first rows).
    pub fn numeric_cols(&self, table: &str) -> Vec<String> {
        let Some(t) = self.table(table) else { return Vec::new() };
        let sample = self.select_maps(&format!("SELECT * FROM {} LIMIT 50", q(table)), &[]).unwrap_or_default();
        t.cols
            .iter()
            .filter(|c| *c != "id" && !c.ends_with("_id") && !YEAR_COLS.contains(&c.as_str()))
            .filter(|c| {
                let vals: Vec<&Value> = sample.iter().filter_map(|m| m.get(*c)).filter(|v| !v.is_null()).collect();
                !vals.is_empty() && vals.iter().all(|v| v.is_number())
            })
            .cloned()
            .collect()
    }
}

/// A SQL name, quoted.
pub fn q(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}

pub fn cell(v: ValueRef) -> Value {
    match v {
        ValueRef::Null => Value::Null,
        ValueRef::Integer(i) => json!(i),
        ValueRef::Real(f) => {
            if f.is_finite() {
                // Ratings were written with 17 digits; four decimals is
                // finer than anything a page shows.
                json!((f * 1e4).round() / 1e4)
            } else {
                Value::Null
            }
        }
        ValueRef::Text(t) => Value::String(String::from_utf8_lossy(t).into_owned()),
        ValueRef::Blob(b) => json!(format!("({} bytes)", b.len())),
    }
}

pub fn json_sql(v: &Value) -> Sql {
    match v {
        Value::Null => Sql::Null,
        Value::Bool(b) => Sql::Integer(*b as i64),
        Value::Number(n) => n.as_i64().map(Sql::Integer).unwrap_or_else(|| Sql::Real(n.as_f64().unwrap_or(0.0))),
        Value::String(s) => s.parse::<i64>().map(Sql::Integer).unwrap_or_else(|_| Sql::Text(s.clone())),
        x => Sql::Text(x.to_string()),
    }
}

/// A column name in words: `games_missed` → "Games missed".
pub fn label(c: &str) -> String {
    match c {
        "_i" => return "#".into(),
        "_row" => return "Row".into(),
        "_parent" => return "Of".into(),
        _ => {}
    }
    // `player__body__height` → "Body height": the player and person parts
    // say nothing on a person's page.
    let mut parts: Vec<&str> = c.split("__").filter(|p| !p.is_empty()).collect();
    while parts.len() > 1 && ["player", "person"].contains(&parts[0]) {
        parts.remove(0);
    }
    let s = parts.join(" ").replace('_', " ");
    let mut ch = s.chars();
    match ch.next() {
        Some(f) => f.to_uppercase().collect::<String>() + ch.as_str(),
        None => s,
    }
}
