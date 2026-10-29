//! The AFM World Reader's commands (lane R5, spec section 5): one
//! `Reader::call(cmd, args)` that the reader's Mac app and its browser
//! server (`afm_reader_web`) both use, as the game's `Game::call` does.
//!
//! The reader is a debug tool: it shows hidden truths (true potential,
//! ceilings, hidden ratings), marked debug, unless the switch in its sidebar
//! turns them off. The game never links this crate.

pub mod areas;
pub mod book;
pub mod compare;
pub mod demo;
pub mod library;
pub mod pages;
pub mod play;
pub mod player;
pub mod text;

#[cfg(test)]
mod tests;

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use serde_json::{Value, json};

use book::{Book, Filters};
pub use library::{default_home, game_home};

pub struct Reader {
    pub home: PathBuf,
    pub game: PathBuf,
    book: Option<Book>,
    /// The second world of a compare, kept open between calls.
    other: Option<Book>,
    /// The world file the engine tools load once (about 4 GB).
    world: Option<(PathBuf, afm_engine::world::World)>,
    hidden: bool,
    job: library::Shared,
    /// The open book's sidebar and filter values, worked out once.
    cache: Option<Value>,
}

impl Reader {
    pub fn new(home: PathBuf, game: PathBuf) -> Reader {
        let _ = std::fs::create_dir_all(home.join("books"));
        let settings: Value = std::fs::read_to_string(home.join("settings.json"))
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or(Value::Null);
        let mut r = Reader {
            home,
            game,
            book: None,
            other: None,
            world: None,
            hidden: settings["hidden_truths"].as_bool().unwrap_or(true),
            job: Arc::new(Mutex::new(library::Job::default())),
            cache: None,
        };
        // Reopen the last book.
        if let Some(p) = settings["last"].as_str().map(PathBuf::from).filter(|p| p.exists()) {
            r.book = Book::open(&p).ok();
        }
        r
    }

    fn save_settings(&self) {
        let s = json!({ "hidden_truths": self.hidden, "last": self.book.as_ref().map(|b| b.path.to_string_lossy().into_owned()) });
        let _ = std::fs::write(self.home.join("settings.json"), s.to_string());
    }

    fn book(&mut self) -> Result<&mut Book, String> {
        self.book.as_mut().ok_or_else(|| "Open a world first.".to_string())
    }

    fn filters(&self, args: &Value) -> Filters {
        let mut f = Filters::from_json(&args["filters"], self.hidden);
        if !self.hidden {
            f.debug = false;
        }
        f
    }

    fn open(&mut self, path: &Path) -> Result<Value, String> {
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if ext == "afmworld" || ext == "afmsave" || path.to_string_lossy().ends_with(".afmworld.001") {
            let stem = path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
            let book = self.home.join("books").join(format!("{stem}.{}", library::BOOK_EXT));
            if book.exists() {
                return self.open(&book);
            }
            return Ok(
                json!({ "needs_book": true, "path": path.to_string_lossy(), "text": "This file has no book yet. Building one reads the world once (about the game's import time and memory)." }),
            );
        }
        let path = if path.to_string_lossy().ends_with(".001") {
            library::import(&self.home, path)?
        } else {
            path.to_path_buf()
        };
        let b = Book::open(&path)?;
        self.book = Some(b);
        self.other = None;
        self.cache = None;
        self.save_settings();
        self.status()
    }

    fn status(&mut self) -> Result<Value, String> {
        let hidden = self.hidden;
        let job = library::job_json(&self.job);
        let Some(b) = self.book.as_mut() else {
            return Ok(json!({ "open": false, "hidden": hidden, "job": job }));
        };
        if let Some(c) = &self.cache {
            let mut c = c.clone();
            c["hidden"] = json!(hidden);
            c["job"] = job;
            return Ok(c);
        }
        let f = Filters { debug: hidden, ..Filters::default() };
        let cs = pages::checks(b, &f);
        let mut nav: Vec<Value> = vec![json!({ "slug": "overview", "title": "Overview", "marks": pages::marks(&cs) })];
        for (slug, title, codes) in pages::TOPICS {
            let here: Vec<pages::Check> = cs.iter().filter(|c| c.page == slug).cloned().collect();
            nav.push(json!({ "slug": slug, "title": title, "codes": codes, "marks": pages::marks(&here) }));
        }
        let (_, odd_n) = pages::odd(b, None, None, &f, 0);
        nav.push(json!({ "slug": "tools", "title": "Tools", "odd": odd_n }));
        nav.push(json!({ "slug": "compare", "title": "Compare" }));
        // The values each filter can take, from the book's own columns.
        let mut values = serde_json::Map::new();
        for (key, cols) in book::FILTER_COLS {
            let mut vals: Vec<String> = Vec::new();
            for t in b.tables.iter().filter(|t| {
                ["people", "programs", "players", "coaches", "schools", "high_schools", "conferences", "clubs"]
                    .contains(&t.name.as_str())
            }) {
                if let Some(c) = t.pick(cols) {
                    let sql = format!(
                        "SELECT DISTINCT CAST({c} AS TEXT) AS v FROM {} WHERE {c} IS NOT NULL ORDER BY 1 LIMIT 200",
                        book::q(&t.name),
                        c = book::q(c)
                    );
                    for m in b.select_maps(&sql, &[]).unwrap_or_default() {
                        if let Some(v) = m.get("v").and_then(book::cell_text)
                            && !vals.contains(&v)
                        {
                            vals.push(v);
                        }
                    }
                }
            }
            vals.sort();
            values.insert(key.to_string(), json!(vals));
        }
        let years = year_span(b);
        let out = json!({
            "open": true, "hidden": hidden, "job": job,
            "world": pages::world_name(b), "path": b.path.to_string_lossy(), "meta": pages::meta(b),
            "badges": pages::coverage_badges(b), "nav": nav, "values": values, "years": years,
            "tables": b.tables.len(),
        });
        self.cache = Some(out.clone());
        Ok(out)
    }

    /// One page by its route: `overview`, `page/<topic>`, `tools/<tab>`,
    /// `career`, `table/<name>`, `e/<table>/<id>`, `compare/<mode>`.
    fn page(&mut self, route: &str, args: &Value) -> Result<Value, String> {
        let f = self.filters(args);
        let parts: Vec<&str> = route.trim_matches('/').split('/').collect();
        let before = self.before();
        let b = self.book()?;
        let mut p = match parts.as_slice() {
            [] | [""] | ["overview"] => {
                let old = before
                    .as_ref()
                    .and_then(|p| Book::open(p).ok())
                    .map(|ob| (pages::world_name(&ob), pages::checks(&ob, &f)));
                pages::overview(b, &f, old.as_ref().map(|(n, c)| (n.as_str(), c.clone())))
            }
            ["page", "career"] | ["career"] => pages::career(b, &f),
            ["page", "tools"] | ["tools"] => pages::tools(b, "issues", &f, args)?,
            ["page", slug] => areas::topic(b, slug, &f)?,
            ["area", code] => areas::area(b, code, &f)?,
            ["class", year] => areas::keyed(b, "class", "class", year, &f)?,
            ["state", code] => areas::keyed(b, "state", "state", code, &f)?,
            ["tools", "odd", page] => pages::tools(b, "odd", &f, &json!({ "page": page }))?,
            ["tools", tab] => pages::tools(b, tab, &f, args)?,
            ["table", name] => pages::table(b, name, &f, args)?,
            ["e", table, id] => {
                let id: Value = id.parse::<i64>().map(Value::from).unwrap_or_else(|_| json!(id));
                pages::entity(b, table, &id, &f)?
            }
            ["compare"] | ["compare", "worlds"] => {
                let a = args["a"].as_str().map(PathBuf::from);
                match a {
                    Some(a) if a != b.path => {
                        if self.other.as_ref().is_none_or(|o| o.path != a) {
                            self.other = Some(Book::open(&a)?);
                        }
                        let (ob, b) = (self.other.as_ref().expect("opened"), self.book.as_ref().expect("open"));
                        compare::worlds(ob, b, &f)
                    }
                    _ => {
                        let mut p = pages::new_page("Compare worlds", "Compare worlds");
                        p["verdict"] = json!("Pick world A; the world open now is B.");
                        p
                    }
                }
            }
            ["compare", "years"] => compare::years(b, &f, args),
            ["compare", "groups"] => compare::groups(b, &f, args),
            ["compare", "player"] => compare::player(b, args),
            _ => return Err(format!("No page {route}.")),
        };
        if p["compare"].is_null() && parts.first() == Some(&"compare") {
            p["compare"] = json!(parts.get(1).copied().unwrap_or("worlds"));
        }
        p["filters"] = json!(f.words());
        Ok(p)
    }

    /// The library's previous book of this seed, for "what changed".
    fn before(&self) -> Option<PathBuf> {
        let b = self.book.as_ref()?;
        let seed = pages::meta(b).get("seed").and_then(book::cell_text)?;
        let mine = std::fs::metadata(&b.path).and_then(|m| m.modified()).ok()?;
        let mut best: Option<(std::time::SystemTime, PathBuf)> = None;
        for e in std::fs::read_dir(self.home.join("books")).ok()?.flatten() {
            let p = e.path();
            if p == b.path || p.extension().and_then(|e| e.to_str()) != Some(library::BOOK_EXT) {
                continue;
            }
            let Ok(t) = std::fs::metadata(&p).and_then(|m| m.modified()) else { continue };
            if t >= mine || best.as_ref().is_some_and(|(bt, _)| *bt >= t) {
                continue;
            }
            let Ok(ob) = Book::open(&p) else { continue };
            if pages::meta(&ob).get("seed").and_then(book::cell_text).as_deref() == Some(seed.as_str()) {
                best = Some((t, p));
            }
        }
        best.map(|x| x.1)
    }

    pub fn call(&mut self, cmd: &str, args: &Value) -> Result<Value, String> {
        match cmd {
            "status" => self.status(),
            "library" => Ok(library::local(&self.home, &self.game.clone())),
            "online" => library::online(&self.home),
            "download" => {
                library::download(&self.job, &self.home, args["tag"].as_str().ok_or("Which world?")?)?;
                Ok(library::job_json(&self.job))
            }
            "job" => Ok(library::job_json(&self.job)),
            "open" => self.open(Path::new(args["path"].as_str().ok_or("Which file?")?)),
            "build" => Err("Building a book on this Mac comes with the book's builder (lane R4); until then, download a world's book from the library.".into()),
            "close" => {
                self.book = None;
                self.cache = None;
                self.save_settings();
                self.status()
            }
            "hidden" => {
                self.hidden = args["on"].as_bool().unwrap_or(!self.hidden);
                self.save_settings();
                self.status()
            }
            "page" => self.page(args["route"].as_str().unwrap_or("overview"), args),
            "copy" => {
                let route = args["route"].as_str().unwrap_or("overview").to_string();
                let p = self.page(&route, args)?;
                let world = self.book.as_ref().map(pages::world_name).unwrap_or_default();
                Ok(json!(text::render(&p, &world, p["filters"].as_str().unwrap_or(""))))
            }
            "search" => {
                let b = self.book()?;
                Ok(json!(b.search(args["q"].as_str().unwrap_or(""), args["limit"].as_u64().unwrap_or(24) as usize)))
            }
            "rows" => {
                let f = self.filters(args);
                let b = self.book()?;
                let extra = args["col"].as_str().map(|c| (c, &args["val"]));
                b.rows(
                    args["table"].as_str().ok_or("Which table?")?,
                    &f,
                    extra,
                    args["sort"].as_str(),
                    args["desc"].as_bool().unwrap_or(false),
                    args["offset"].as_i64().unwrap_or(0),
                    args["limit"].as_i64().unwrap_or(50),
                )
            }
            "values" => {
                // A column's values, for compare's pickers.
                let b = self.book()?;
                let (t, c) = (args["table"].as_str().ok_or("Which table?")?, args["col"].as_str().ok_or("Which column?")?);
                if !b.table(t).is_some_and(|x| x.has(c)) {
                    return Err(format!("{t} has no column {c}."));
                }
                let sql = format!("SELECT CAST({c} AS TEXT) AS v, COUNT(*) AS n FROM {} WHERE {c} IS NOT NULL GROUP BY 1 ORDER BY n DESC LIMIT 300", book::q(t), c = book::q(c));
                Ok(json!(b.select_maps(&sql, &[])?))
            }
            "play" => {
                let (h, a) = (args["home"].as_u64().ok_or("Pick the home team.")? as usize, args["away"].as_u64().ok_or("Pick the road team.")? as usize);
                let worlds = self.game.join("worlds");
                let path = match args["world"].as_str() {
                    Some(p) => PathBuf::from(p),
                    None => play::world_path(self.book()?, &worlds).ok_or("The world file this book was built from isn't on this Mac; download the world in the game's library, or choose it.")?,
                };
                if self.world.as_ref().is_none_or(|(p, _)| *p != path) {
                    self.world = None;
                    let w = afm_engine::save::read_world(&path).map_err(|e| format!("Could not read the world: {e}"))?;
                    self.world = Some((path, w));
                }
                let w = &self.world.as_ref().expect("loaded").1;
                let seed = args["seed"].as_u64().unwrap_or(2026);
                play::play(w, h, a, seed)
            }
            _ => Err(format!("Unknown command {cmd}.")),
        }
    }
}

/// The first and last year the book's season rows cover, for the slider.
fn year_span(b: &Book) -> Value {
    let m = pages::meta(b);
    let y = |k: &str| m.get(k).and_then(|v| v.as_i64().or_else(|| v.as_str().and_then(|s| s.parse().ok())));
    if let (Some(a), Some(z)) = (y("first"), y("last")) {
        return json!([a, z]);
    }
    let mut lo = i64::MAX;
    let mut hi = i64::MIN;
    let few: Vec<&book::Table> = ["seasons", "games", "checks", "series"].iter().filter_map(|k| b.find(k)).collect();
    for t in few {
        let Some(y) = t.pick(&book::YEAR_COLS) else { continue };
        if !["year", "season"].contains(&y) {
            continue;
        }
        let sql = format!("SELECT MIN({y}), MAX({y}) FROM {}", book::q(&t.name), y = book::q(y));
        if let Ok((a, z)) = b.db.query_row(&sql, [], |r| Ok((r.get::<_, Option<i64>>(0)?, r.get::<_, Option<i64>>(1)?)))
            && let (Some(a), Some(z)) = (a, z)
            && (1900..2300).contains(&a)
            && (1900..2300).contains(&z)
        {
            lo = lo.min(a);
            hi = hi.max(z);
        }
    }
    if lo > hi { Value::Null } else { json!([lo, hi]) }
}
