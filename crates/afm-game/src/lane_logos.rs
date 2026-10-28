//! Lane commands: logo packs you install yourself (update 3, lane 15; the
//! "install your own logo pack" route of later extras topic 2, Cain's yes
//! 2026-10-28). `call` answers the commands this lane owns and returns None
//! for any other.
//!
//! AFM ships no real logos. A pack is a folder of pictures the player picks
//! once, the way Football Manager's logo packs work: the page reads the
//! folder, the game matches each picture to a team, the player checks the
//! matches, and only the pictures in use are copied into the game folder
//! (`<home>/logos/packs/<id>/`), shared by every save and never sent
//! anywhere.
//!
//! A picture finds its team in this order:
//! 1. a mapping file in the pack (`logos.csv`: file, team, from, to);
//! 2. a teams list in the pack (a JSON list of objects with a name and a
//!    logo path, like Gridiron Lab's `teams.json`);
//! 3. its own file name (`Kansas_City_Chiefs_logo.svg`, `iowa-st.svg`).
//!
//! Teams are keyed by their real names (`nfl:Tennessee Titans`,
//! `college:Iowa State`, `other:Toronto Argonauts`, `comp:Big 12`), so a pack
//! works in every world and a logo covers only the years its team had that
//! name: the Titans' logo never shows on a 1997 Oilers line. Dated lines in
//! the mapping file split a team's eras further.
//!
//! Commands: `logos_status`, `logos_teams` (search), `logos_match` (the
//! review), `logos_begin` / `logos_add` / `logos_commit` (install in
//! batches), `logos_pack` and `logos_entry_set` ("Check matches"),
//! `logos_remove`, `logos_set` (the switches), `logos_map` (this world's
//! teams and competitions with their logos) and `logos_pics` (pictures by
//! id, as data URLs: the Mac app has one `call` command and no file server).
//!
//! Your own logo for one team ("Change logo" on a team's page, question 1
//! of the mock-up, Cain's yes 2026-10-28) is kept with the save, beside your
//! photos in its faces folder (`<save>.faces/logos/`), and wins over any
//! pack's: `logos_own_set` and `logos_own_clear`.

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use afm_engine::calendar::data::{BOWLS, CELEBRATION_BOWL};
use afm_engine::world::data::COLLEGES;
use afm_engine::world::franchise::{self, ERAS};
use afm_engine::world::pro::other;
use afm_engine::world::stories::numbers::mascot_of;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::lane_faces::{b64_decode, b64_encode};
use crate::views::TeamRef;
use crate::{Game, Reply};

/// A picture kept from a pack: at most this many bytes (the page shrinks
/// big pictures to 512 pixels before sending them).
const MAX_PICTURE: usize = 2 << 20;
/// Years with no end: a club still playing.
const OPEN: i32 = 9999;

pub fn call(game: &mut Game, cmd: &str, args: &Value) -> Option<Reply> {
    let home = game.home.clone();
    Some(match cmd {
        "logos_status" => Ok(status(game)),
        "logos_teams" => Ok(teams(args)),
        "logos_match" => Ok(match_files(args)),
        "logos_begin" => begin(&home, args),
        "logos_add" => add(&home, args),
        "logos_commit" => commit(&home, args),
        "logos_pack" => pack_view(&home, args),
        "logos_entry_set" => entry_set(&home, args),
        "logos_remove" => remove(&home, args),
        "logos_set" => set(game, args),
        "logos_map" => map(game),
        "logos_pics" => pics(game, args),
        "logos_own_set" => own_set(game, args),
        "logos_own_clear" => own_clear(game, args),
        _ => return None,
    })
}

// --- Files -----------------------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, Debug)]
struct Settings {
    #[serde(default = "yes")]
    show: bool,
    /// Logos in a world built with fictional names (off until switched on).
    #[serde(default)]
    fictional: bool,
}

fn yes() -> bool {
    true
}

impl Default for Settings {
    fn default() -> Self {
        Settings { show: true, fictional: false }
    }
}

/// One picture kept from a pack.
#[derive(Serialize, Deserialize, Clone, Debug)]
struct Entry {
    n: u32,
    /// Its path in the folder it came from.
    file: String,
    /// Its name in the pack's folder here (`0007.svg`).
    stored: String,
    /// The team or competition it shows; None once you drop it.
    key: Option<String>,
    #[serde(default)]
    from: Option<i32>,
    #[serde(default)]
    to: Option<i32>,
    /// Too dark to read on the dark screens (measured by the page).
    #[serde(default)]
    dark: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct Pack {
    name: String,
    installed: String,
    /// False until the last batch is in: a pack cut off half way is ignored
    /// and cleared at the next install.
    done: bool,
    entries: Vec<Entry>,
}

fn root(home: &Path) -> PathBuf {
    home.join("logos")
}

fn packs_dir(home: &Path) -> PathBuf {
    root(home).join("packs")
}

fn settings(home: &Path) -> Settings {
    std::fs::read(root(home).join("settings.json"))
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}

fn save_settings(home: &Path, s: &Settings) -> Result<(), String> {
    std::fs::create_dir_all(root(home)).map_err(|e| e.to_string())?;
    std::fs::write(root(home).join("settings.json"), serde_json::to_vec_pretty(s).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())
}

/// A pack id is `p` and a number; anything else never reaches the disk.
fn pack_path(home: &Path, id: &str) -> Result<PathBuf, String> {
    let ok = id.len() > 1 && id.starts_with('p') && id[1..].bytes().all(|b| b.is_ascii_digit());
    if !ok {
        return Err("No such logo pack.".into());
    }
    Ok(packs_dir(home).join(id))
}

fn read_pack(dir: &Path) -> Option<Pack> {
    serde_json::from_slice(&std::fs::read(dir.join("pack.json")).ok()?).ok()
}

fn write_pack(dir: &Path, p: &Pack) -> Result<(), String> {
    std::fs::write(dir.join("pack.json"), serde_json::to_vec(p).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
}

/// Installed packs, oldest first (a later pack's logo wins for a team).
fn packs(home: &Path) -> Vec<(String, Pack)> {
    let mut out: Vec<(u32, String, Pack)> = std::fs::read_dir(packs_dir(home))
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|d| {
            let id = d.file_name().to_string_lossy().to_string();
            let n: u32 = id.strip_prefix('p')?.parse().ok()?;
            Some((n, id, read_pack(&d.path())?))
        })
        .filter(|(_, _, p)| p.done)
        .collect();
    out.sort_by_key(|x| x.0);
    out.into_iter().map(|(_, id, p)| (id, p)).collect()
}

fn arg_str<'a>(args: &'a Value, k: &str) -> Option<&'a str> {
    args.get(k).and_then(Value::as_str)
}

fn arg_year(v: Option<&Value>) -> Option<i32> {
    match v? {
        Value::Number(n) => n.as_i64().map(|y| y as i32),
        Value::String(s) => s.trim().parse().ok(),
        _ => None,
    }
    .filter(|y| (1850..=2200).contains(y))
}

// --- The teams a picture can show --------------------------------------------------------------------

struct Cand {
    key: String,
    label: String,
    level: String,
    /// Its names as word lists: the name, with its nickname, and the short
    /// names packs use.
    names: Vec<Vec<String>>,
    /// An NFL club's nickname, for a file that names only that.
    nick: Option<String>,
}

/// Names packs use for a school that the world calls by another (Claude's
/// list from the test packs' teams files and NCAA short names; proposed, not
/// confirmed).
const ALIASES: &[(&str, &[&str])] = &[
    ("UMass", &["Massachusetts"]),
    ("FIU", &["Florida International"]),
    ("ULM", &["UL Monroe", "Louisiana Monroe", "La Monroe"]),
    ("Louisiana", &["UL Lafayette", "Louisiana Lafayette"]),
    ("UConn", &["Connecticut"]),
    ("Ole Miss", &["Mississippi"]),
    ("Pittsburgh", &["Pitt"]),
    ("NC State", &["North Carolina State"]),
    ("LSU", &["Louisiana State"]),
    ("UCF", &["Central Florida"]),
    ("USC", &["Southern California"]),
    ("BYU", &["Brigham Young"]),
    ("SMU", &["Southern Methodist"]),
    ("TCU", &["Texas Christian"]),
    ("UTEP", &["Texas El Paso"]),
    ("UTSA", &["Texas San Antonio"]),
    ("UNLV", &["Nevada Las Vegas"]),
    ("UAB", &["Alabama Birmingham"]),
    ("Southern Miss", &["Southern Mississippi"]),
    ("Miami (FL)", &["Miami Florida"]),
    ("Miami (OH)", &["Miami Ohio"]),
    ("Appalachian State", &["App State"]),
    ("Southern", &["Southern University"]),
    ("McNeese", &["McNeese State"]),
    ("Nicholls", &["Nicholls State"]),
    ("Incarnate Word", &["UIW"]),
    ("East Texas A&M", &["Texas A&M Commerce"]),
    ("LIU", &["Long Island", "Long Island University"]),
    ("Houston Christian", &["Houston Baptist"]),
    ("Central Connecticut", &["Central Connecticut State"]),
    ("Northern Iowa", &["UNI"]),
    ("Stephen F. Austin", &["SFA"]),
    ("Arkansas-Pine Bluff", &["UAPB"]),
    ("UT Martin", &["Tennessee Martin"]),
    ("Chattanooga", &["UTC", "Tennessee Chattanooga"]),
    ("Middle Tennessee", &["Middle Tennessee State", "MTSU"]),
    ("East Tennessee State", &["ETSU"]),
    ("Western Kentucky", &["WKU"]),
    ("Florida Atlantic", &["FAU"]),
    ("South Florida", &["USF"]),
];

/// State short forms in packs that are not the start of the state's name
/// (`eastern-ky`, `fla-atlantic`, `nc-central`).
const STATES: &[(&str, &[&str])] = &[
    ("ky", &["kentucky"]),
    ("fla", &["florida"]),
    ("ga", &["georgia"]),
    ("la", &["louisiana"]),
    ("mo", &["missouri"]),
    ("md", &["maryland"]),
    ("va", &["virginia"]),
    ("nc", &["north", "carolina"]),
    ("sc", &["south", "carolina"]),
    ("nm", &["new", "mexico"]),
];
/// Two-letter postal codes, for a school's state added at the end (`albany-ny`).
const POSTAL: &[&str] = &[
    "al", "ak", "az", "ar", "ca", "co", "ct", "de", "fl", "ga", "hi", "id", "il", "in", "ia", "ks", "ky", "la", "me",
    "md", "ma", "mi", "mn", "ms", "mo", "mt", "ne", "nv", "nh", "nj", "nm", "ny", "nc", "nd", "oh", "ok", "or", "pa",
    "ri", "sc", "sd", "tn", "tx", "ut", "vt", "va", "wa", "wv", "wi", "wy",
];

/// Words in a file name that say nothing about the team.
const FILLER: &[&str] = &[
    "logo", "logos", "the", "of", "and", "primary", "svg", "png", "jpg", "jpeg", "webp", "icon", "official", "full",
    "color", "colour", "main",
];
/// Words that mark another version of a team's logo.
const VARIANT: &[&str] = &[
    "wordmark",
    "word",
    "alt",
    "alternate",
    "helmet",
    "old",
    "secondary",
    "script",
    "text",
    "throwback",
    "classic",
    "classical",
    "retro",
    "mono",
    "monochrome",
    "white",
    "black",
    "dark",
    "light",
    "reverse",
    "partial",
    "mark",
    "small",
    "tertiary",
];

fn fold(c: char) -> char {
    match c {
        'á' | 'à' | 'â' | 'ä' | 'ã' | 'å' => 'a',
        'é' | 'è' | 'ê' | 'ë' => 'e',
        'í' | 'ì' | 'î' | 'ï' => 'i',
        'ó' | 'ò' | 'ô' | 'ö' | 'õ' => 'o',
        'ú' | 'ù' | 'û' | 'ü' => 'u',
        'ñ' => 'n',
        'ç' => 'c',
        _ => c,
    }
}

/// A name as words: lower case, accents off, "A&M" as one word, no "&",
/// "and", apostrophes or full stops, "St" as "saint" first and "state"
/// after, state short forms whole, "U" as "university", no filler words or
/// years.
fn words(s: &str) -> Vec<String> {
    let mut t = String::with_capacity(s.len());
    let cs: Vec<char> = s.chars().map(|c| fold(c.to_ascii_lowercase())).collect();
    for (i, &c) in cs.iter().enumerate() {
        match c {
            '\'' | '’' | 'ʻ' | '.' => {}
            '&' => {
                // A&M, S&T: two single letters stay one word (kept apart
                // from "st" below by the "&" until the end).
                let alnum = |j: usize| cs.get(j).is_some_and(|c| c.is_alphanumeric());
                let single_before = i >= 1 && alnum(i - 1) && (i < 2 || !alnum(i - 2));
                let single_after = alnum(i + 1) && !alnum(i + 2);
                t.push(if single_before && single_after { '&' } else { ' ' });
            }
            c if c.is_alphanumeric() => t.push(c),
            _ => t.push(' '),
        }
    }
    let mut out: Vec<String> = Vec::new();
    for w in t.split_whitespace() {
        if FILLER.contains(&w) || (w.len() == 4 && w.starts_with(['1', '2']) && w.bytes().all(|b| b.is_ascii_digit())) {
            continue;
        }
        // `duke-s-mayo-bowl`: an "s" cut off by the apostrophe.
        if w == "s" && !out.is_empty() {
            out.last_mut().unwrap().push('s');
            continue;
        }
        match w {
            "st" if out.is_empty() || out.last().is_some_and(|p| p == "mount") => out.push("saint".into()),
            "st" => out.push("state".into()),
            "mt" => out.push("mount".into()),
            "u" => out.push("university".into()),
            _ => match STATES.iter().find(|(k, _)| *k == w) {
                Some((_, full)) => out.extend(full.iter().map(|x| x.to_string())),
                None => out.push(w.replace('&', "")),
            },
        }
    }
    out
}

static CANDS: LazyLock<Vec<Cand>> = LazyLock::new(|| {
    let mut out: Vec<Cand> = Vec::new();
    let mut seen: HashMap<String, usize> = HashMap::new();
    let mut add =
        |out: &mut Vec<Cand>, key: String, label: &str, level: String, names: Vec<String>, nick: Option<String>| {
            if let Some(&i) = seen.get(&key) {
                for n in names {
                    let w = words(&n);
                    if !w.is_empty() && !out[i].names.contains(&w) {
                        out[i].names.push(w);
                    }
                }
                return;
            }
            seen.insert(key.clone(), out.len());
            let mut ws: Vec<Vec<String>> = Vec::new();
            for n in names {
                let w = words(&n);
                if !w.is_empty() && !ws.contains(&w) {
                    ws.push(w);
                }
            }
            out.push(Cand { key, label: label.to_string(), level, names: ws, nick });
        };
    // NFL clubs by every name they have had.
    for eras in ERAS.iter() {
        for e in eras.iter() {
            let name: &str = &e.name;
            let (from, to) =
                eras.iter().filter(|x| *x.name == *name).fold((OPEN, 0), |(a, b), x| (a.min(x.from), b.max(x.to)));
            let years = if to >= OPEN {
                format!("NFL · {} on", from.max(1920))
            } else {
                format!("NFL · {}-{to}", from.max(1920))
            };
            let mut names = vec![name.to_string()];
            if let Some(rest) = name.strip_prefix("Los Angeles ") {
                names.push(format!("LA {rest}"));
            }
            let nick = name.rsplit(' ').next().filter(|n| *n != "Team").map(|n| words(n).join(" "));
            add(&mut out, format!("nfl:{name}"), name, years, names, nick);
        }
    }
    // Every real college program, with its nickname and its other names.
    for c in COLLEGES.iter() {
        let mut names = vec![c.name.clone()];
        if let Some(m) = mascot_of(&c.name) {
            names.push(format!("{} {m}", c.name));
        }
        if let Some((_, a)) = ALIASES.iter().find(|(n, _)| *n == c.name) {
            names.extend(a.iter().map(|s| s.to_string()));
        }
        let level = format!("{} · {}", c.division.name(), c.conference);
        add(&mut out, format!("college:{}", c.name), &c.name, level, names, None);
    }
    // Clubs in the other pro leagues.
    for d in other::TEAMS.iter() {
        add(
            &mut out,
            format!("other:{}", d.name),
            d.name,
            other::league_short(d.league).to_string(),
            vec![d.name.to_string()],
            None,
        );
    }
    // Competitions: the league, the conferences, the bowls, the playoff.
    add(
        &mut out,
        "comp:NFL".into(),
        "NFL",
        "League".into(),
        vec!["NFL".into(), "National Football League".into(), "NFL Shield".into()],
        None,
    );
    let mut confs: Vec<&str> =
        COLLEGES.iter().map(|c| c.conference.as_str()).filter(|c| !c.is_empty() && *c != "Independent").collect();
    confs.sort_unstable();
    confs.dedup();
    for c in confs {
        add(
            &mut out,
            format!("comp:{c}"),
            c,
            "Conference".into(),
            vec![c.to_string(), format!("{c} Conference")],
            None,
        );
    }
    for b in BOWLS.iter().map(|b| b.name).chain([CELEBRATION_BOWL.0]) {
        add(&mut out, format!("comp:{b}"), b, "Bowl".into(), vec![b.to_string()], None);
    }
    add(
        &mut out,
        "comp:College Football Playoff".into(),
        "College Football Playoff",
        "Playoff".into(),
        vec!["College Football Playoff".into(), "CFP".into()],
        None,
    );
    out
});

fn cand(key: &str) -> Option<&'static Cand> {
    CANDS.iter().find(|c| c.key == key)
}

fn cand_json(c: &Cand) -> Value {
    json!({ "key": c.key, "label": c.label, "level": c.level })
}

/// Teams for "Find a team": every word typed starts a word of the name.
fn teams(args: &Value) -> Value {
    let q = words(arg_str(args, "q").unwrap_or(""));
    if q.is_empty() {
        return json!({ "teams": [] });
    }
    let hits: Vec<Value> = CANDS
        .iter()
        .filter(|c| c.names.iter().any(|n| q.iter().all(|w| n.iter().any(|x| x.starts_with(w.as_str())))))
        .take(24)
        .map(cand_json)
        .collect();
    json!({ "teams": hits })
}

/// One word of a file's name against one of a team's: the same, or a
/// short form of it ("caro" for "carolina", "app" for "appalachian").
fn word_fits(file: &str, name: &str) -> bool {
    file == name || (file.len() >= 2 && name.len() > file.len() && name.starts_with(file))
}

/// Where a state at the end of a name starts (`ny`, or `missouri` from `mo`),
/// when words come before it.
fn state_at_end(ws: &[String]) -> Option<usize> {
    let n = ws.len();
    if n >= 2 && POSTAL.contains(&ws[n - 1].as_str()) {
        return Some(n - 1);
    }
    STATES
        .iter()
        .map(|(_, full)| full.len())
        .find(|&k| {
            n > k && STATES.iter().any(|(_, f)| f.len() == k && ws[n - k..].iter().zip(f.iter()).all(|(a, b)| a == b))
        })
        .map(|k| n - k)
}

/// The teams a name could be, and whether it names another version of the
/// logo (a wordmark, an old one, extra words). Exact names first, then a
/// full name with words added, then short forms, then an NFL nickname alone.
fn find(name: &str) -> (Vec<usize>, bool, &'static str) {
    let all = words(name);
    let core: Vec<String> = all.iter().filter(|w| !VARIANT.contains(&w.as_str())).cloned().collect();
    let variant = core.len() != all.len();
    if core.is_empty() {
        return (Vec::new(), false, "");
    }
    let keys = |v: Vec<usize>| -> Vec<usize> {
        let mut out: Vec<usize> = Vec::new();
        for i in v {
            if !out.iter().any(|&j| CANDS[j].key == CANDS[i].key) {
                out.push(i);
            }
        }
        out
    };
    let exact = |ws: &[String]| -> Vec<usize> {
        (0..CANDS.len()).filter(|&i| CANDS[i].names.iter().any(|n| n == ws)).collect()
    };
    // The whole name first, so "Old Dominion" is not an old logo.
    let whole = exact(&all);
    if !whole.is_empty() {
        return (keys(whole), false, "the file's name");
    }
    let same = exact(&core);
    if !same.is_empty() {
        return (keys(same), variant, "the file's name");
    }
    // A school's state added at the end (`albany-ny`, `lindenwood-mo`).
    if let Some(cut) = state_at_end(&core) {
        let same = exact(&core[..cut]);
        if !same.is_empty() {
            return (keys(same), variant, "the file's name");
        }
    }
    // A full name with more words after it (`Kansas_City_Chiefs_KC_logo`),
    // when the name is most of the words: the longest such name wins.
    let mut best = 0;
    let mut longer: Vec<usize> = Vec::new();
    for (i, c) in CANDS.iter().enumerate() {
        for n in &c.names {
            if n.len() < core.len() && core.starts_with(n) && n.len() * 2 > core.len() {
                if n.len() > best {
                    best = n.len();
                    longer.clear();
                }
                if n.len() == best && !longer.contains(&i) {
                    longer.push(i);
                }
            }
        }
    }
    if !longer.is_empty() {
        return (keys(longer), true, "the file's name");
    }
    // Short forms: `coastal-caro`, `app state`, or the first words of a
    // longer name (`grambling` for Grambling State; `miami` fits three).
    let close: Vec<usize> = (0..CANDS.len())
        .filter(|&i| {
            CANDS[i]
                .names
                .iter()
                .any(|n| n.len() >= core.len() && core.iter().zip(n.iter()).all(|(f, w)| word_fits(f, w)))
        })
        .collect();
    if !close.is_empty() {
        return (keys(close), variant, "a short form of the name");
    }
    let nick: Vec<usize> =
        (0..CANDS.len()).filter(|&i| CANDS[i].nick.as_ref().is_some_and(|k| core.contains(k))).collect();
    (keys(nick), true, "the club's nickname")
}

// --- The review ------------------------------------------------------------------------------------

/// A file's name without its folders and extension.
fn stem(path: &str) -> &str {
    let base = path.rsplit(['/', '\\']).next().unwrap_or(path);
    base.rsplit_once('.').map_or(base, |(s, _)| s)
}

fn base_lower(path: &str) -> String {
    path.rsplit(['/', '\\']).next().unwrap_or(path).to_lowercase()
}

fn picture_kind(path: &str) -> Option<&'static str> {
    let ext = path.rsplit_once('.')?.1.to_ascii_lowercase();
    Some(match ext.as_str() {
        "svg" => "svg",
        "png" => "png",
        "webp" => "webp",
        "jpg" | "jpeg" => "jpg",
        _ => return None,
    })
}

/// A mapping file's lines: file, team, from, to (a header line is skipped).
fn mapping_lines(text: &str) -> Vec<(String, String, Option<i32>, Option<i32>)> {
    let mut out = Vec::new();
    for line in text.lines() {
        let cols: Vec<String> =
            afm_engine::world::data::split_csv_line(line).into_iter().map(|c| c.trim().to_string()).collect();
        if cols.len() < 2 || cols[0].is_empty() || cols[0].eq_ignore_ascii_case("file") {
            continue;
        }
        let year = |k: usize| cols.get(k).and_then(|s| s.parse::<i32>().ok());
        out.push((cols[0].clone(), cols[1].clone(), year(2), year(3)));
    }
    out
}

/// A teams list: any JSON list of objects with a name and a picture path.
fn list_names(text: &str, into: &mut HashMap<String, Vec<String>>) {
    let Ok(v) = serde_json::from_str::<Value>(text) else { return };
    let rows: Vec<&Value> = match &v {
        Value::Array(a) => a.iter().collect(),
        Value::Object(o) => o.values().filter_map(Value::as_array).flatten().collect(),
        _ => return,
    };
    let field = |o: &Value, ks: &[&str]| ks.iter().find_map(|k| o.get(*k).and_then(Value::as_str)).map(str::to_string);
    for r in rows {
        let (Some(name), Some(file)) = (
            field(r, &["name", "team", "school", "displayName", "display_name", "title"]),
            field(r, &["logo", "file", "path", "image", "img", "src"]),
        ) else {
            continue;
        };
        let mut names = vec![name.clone()];
        if let Some(m) = field(r, &["mascot", "nickname"]) {
            names.push(format!("{name} {m}"));
        }
        into.entry(base_lower(&file)).or_default().extend(names);
    }
}

#[derive(Clone)]
struct Row {
    file: String,
    size: u64,
    status: &'static str,
    key: Option<String>,
    options: Vec<usize>,
    how: String,
    variant: bool,
    from: Option<i32>,
    to: Option<i32>,
    rank: u8,
}

/// The review: every picture as matched, unsure (two or more teams fit),
/// spare (another picture of a team that already has one) or no match.
fn match_files(args: &Value) -> Value {
    let files: Vec<(String, u64)> = args
        .get("files")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(|f| {
                    let p = f.get("path").and_then(Value::as_str).or_else(|| f.as_str())?;
                    picture_kind(p)?;
                    Some((p.to_string(), f.get("size").and_then(Value::as_u64).unwrap_or(0)))
                })
                .collect()
        })
        .unwrap_or_default();
    let mut mapping: HashMap<String, (String, Option<i32>, Option<i32>)> = HashMap::new();
    let mut listed: HashMap<String, Vec<String>> = HashMap::new();
    let mut notes: Vec<String> = Vec::new();
    for t in args.get("texts").and_then(Value::as_array).into_iter().flatten() {
        let (Some(path), Some(text)) = (t.get("path").and_then(Value::as_str), t.get("text").and_then(Value::as_str))
        else {
            continue;
        };
        let lower = path.to_lowercase();
        if lower.ends_with(".csv") {
            let lines = mapping_lines(text);
            if !lines.is_empty() {
                notes.push(format!("Mapping file {}: {} lines", base_lower(path), lines.len()));
            }
            for (f, team, from, to) in lines {
                mapping.insert(f.replace('\\', "/").to_lowercase(), (team, from, to));
            }
        } else if lower.ends_with(".json") {
            list_names(text, &mut listed);
        }
    }
    let mut rows: Vec<Row> = files
        .iter()
        .map(|(file, size)| {
            let rel = file.replace('\\', "/").to_lowercase();
            // The mapping file names a picture by its path in the pack or
            // its name alone.
            let mapped =
                mapping.iter().find(|(k, _)| rel == **k || rel.ends_with(&format!("/{k}"))).map(|(_, v)| v.clone());
            let row = |status, key: Option<String>, options, how: &str, variant, rank| Row {
                file: file.clone(),
                size: *size,
                status,
                key,
                options,
                how: how.to_string(),
                variant,
                from: None,
                to: None,
                rank,
            };
            if let Some((team, from, to)) = mapped {
                let hit = cand(&team)
                    .map(|c| vec![CANDS.iter().position(|x| x.key == c.key).unwrap_or(0)])
                    .unwrap_or_else(|| find(&team).0);
                let mut r = match hit.len() {
                    1 => row("matched", Some(CANDS[hit[0]].key.clone()), Vec::new(), "the mapping file", false, 0),
                    0 => row(
                        "none",
                        None,
                        Vec::new(),
                        &format!("the mapping file names \"{team}\", which isn't a team here"),
                        false,
                        9,
                    ),
                    _ => row("unsure", None, hit, "the mapping file", false, 0),
                };
                r.from = from;
                r.to = to;
                return r;
            }
            let (hits, variant, how) = match listed.get(&base_lower(file)) {
                Some(names) => {
                    let found: Vec<(Vec<usize>, bool, &str)> = names.iter().map(|n| find(n)).collect();
                    // The name with its nickname is the surer read.
                    match found.iter().find(|f| f.0.len() == 1).or(found.first()) {
                        Some((h, v, _)) => (h.clone(), *v, "the pack's teams list"),
                        None => (Vec::new(), false, ""),
                    }
                }
                None => find(stem(file)),
            };
            let rank = (if variant { 3 } else { 1 }) + u8::from(how == "a short form of the name");
            match hits.len() {
                0 => row("none", None, Vec::new(), "no team has this name", false, 9),
                1 => row("matched", Some(CANDS[hits[0]].key.clone()), Vec::new(), how, variant, rank),
                _ => row("unsure", None, hits.into_iter().take(8).collect(), how, variant, rank),
            }
        })
        .collect();
    // One picture per team and years: the mapping file's, then a main logo
    // over a wordmark or old one, a drawing (SVG) over a photo, the bigger
    // file. The rest wait as spares.
    let mut best: HashMap<(String, Option<i32>, Option<i32>), usize> = HashMap::new();
    for i in 0..rows.len() {
        let Some(k) = rows[i].key.clone() else { continue };
        let slot = (k, rows[i].from, rows[i].to);
        let score = |r: &Row| (std::cmp::Reverse(r.rank), r.file.to_lowercase().ends_with(".svg"), r.size);
        match best.get(&slot) {
            Some(&j) if score(&rows[j]) >= score(&rows[i]) => rows[i].status = "spare",
            Some(&j) => {
                rows[j].status = "spare";
                best.insert(slot, i);
            }
            None => {
                best.insert(slot, i);
            }
        }
    }
    let count = |s: &str| rows.iter().filter(|r| r.status == s).count();
    let teams: std::collections::HashSet<&String> =
        rows.iter().filter(|r| r.status == "matched").filter_map(|r| r.key.as_ref()).collect();
    let out: Vec<Value> = rows
        .iter()
        .map(|r| {
            let c = r.key.as_deref().and_then(cand);
            json!({
                "file": r.file,
                "status": r.status,
                "key": r.key,
                "team": c.map(|c| c.label.clone()),
                "level": c.map(|c| c.level.clone()),
                "how": r.how,
                "variant": r.variant,
                "from": r.from,
                "to": r.to,
                "options": r.options.iter().map(|&i| cand_json(&CANDS[i])).collect::<Vec<_>>(),
            })
        })
        .collect();
    json!({
        "rows": out,
        "counts": { "teams": teams.len(), "matched": count("matched"), "unsure": count("unsure"), "none": count("none"), "spare": count("spare") },
        "notes": notes,
    })
}

// --- Installing --------------------------------------------------------------------------------------

fn today_text() -> String {
    // The computer's date, for the pack's line (not the world's).
    let secs =
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0) as i64;
    let days = secs.div_euclid(86_400);
    // Civil from days (Howard Hinnant's algorithm).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    const M: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
    format!("{d} {} {y}", M[(m - 1) as usize])
}

/// A new pack's folder; packs cut off half way are cleared first.
fn begin(home: &Path, args: &Value) -> Reply {
    let dir = packs_dir(home);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let mut next = 1;
    for d in std::fs::read_dir(&dir).map_err(|e| e.to_string())?.flatten() {
        let id = d.file_name().to_string_lossy().to_string();
        let Some(n) = id.strip_prefix('p').and_then(|n| n.parse::<u32>().ok()) else { continue };
        next = next.max(n + 1);
        if read_pack(&d.path()).is_none_or(|p| !p.done) {
            let _ = std::fs::remove_dir_all(d.path());
        }
    }
    let id = format!("p{next}");
    let path = dir.join(&id);
    std::fs::create_dir_all(&path).map_err(|e| e.to_string())?;
    let name = arg_str(args, "name").map(str::trim).filter(|s| !s.is_empty()).unwrap_or("Logo pack");
    let pack =
        Pack { name: name.chars().take(80).collect(), installed: today_text(), done: false, entries: Vec::new() };
    write_pack(&path, &pack)?;
    Ok(json!({ "pack": id }))
}

/// A picture from the page: (bytes, extension). PNG, JPEG, WebP or SVG.
fn picture(data: &str) -> Result<(Vec<u8>, &'static str), String> {
    let (head, body) = data.split_once(',').ok_or("That isn't a picture.")?;
    let ext = match head {
        h if h.starts_with("data:image/png") => "png",
        h if h.starts_with("data:image/jpeg") || h.starts_with("data:image/jpg") => "jpg",
        h if h.starts_with("data:image/webp") => "webp",
        h if h.starts_with("data:image/svg+xml") => "svg",
        _ => return Err("Only PNG, JPEG, WebP and SVG pictures can be kept.".into()),
    };
    let bytes = b64_decode(body).ok_or("The picture is damaged.")?;
    if bytes.len() > MAX_PICTURE {
        return Err("The picture is too big (2 MB at most).".into());
    }
    let ok = match ext {
        "png" => bytes.starts_with(b"\x89PNG"),
        "jpg" => bytes.starts_with(&[0xFF, 0xD8]),
        "webp" => bytes.len() > 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP",
        _ => String::from_utf8_lossy(&bytes[..bytes.len().min(4096)]).contains("<svg"),
    };
    if !ok {
        return Err("The picture's contents don't match its type.".into());
    }
    Ok((bytes, ext))
}

/// A batch of the pictures in use, each with its team.
fn add(home: &Path, args: &Value) -> Reply {
    let id = arg_str(args, "pack").ok_or("Missing pack.")?;
    let dir = pack_path(home, id)?;
    let mut pack = read_pack(&dir).ok_or("No such logo pack.")?;
    let mut errors = Vec::new();
    for f in args.get("files").and_then(Value::as_array).into_iter().flatten() {
        let file = f.get("file").and_then(Value::as_str).unwrap_or("");
        let key = f.get("key").and_then(Value::as_str).filter(|k| cand(k).is_some());
        let Some(key) = key else {
            errors.push(format!("{file}: no team"));
            continue;
        };
        match picture(f.get("data").and_then(Value::as_str).unwrap_or("")) {
            Ok((bytes, ext)) => {
                let n = pack.entries.iter().map(|e| e.n + 1).max().unwrap_or(0);
                let stored = format!("{n:04}.{ext}");
                std::fs::write(dir.join(&stored), bytes).map_err(|e| e.to_string())?;
                pack.entries.push(Entry {
                    n,
                    file: file.chars().take(300).collect(),
                    stored,
                    key: Some(key.to_string()),
                    from: arg_year(f.get("from")),
                    to: arg_year(f.get("to")),
                    dark: f.get("dark").and_then(Value::as_bool).unwrap_or(false),
                });
            }
            Err(e) => errors.push(format!("{file}: {e}")),
        }
    }
    write_pack(&dir, &pack)?;
    Ok(json!({ "kept": pack.entries.len(), "errors": errors }))
}

fn commit(home: &Path, args: &Value) -> Reply {
    let id = arg_str(args, "pack").ok_or("Missing pack.")?;
    let dir = pack_path(home, id)?;
    let mut pack = read_pack(&dir).ok_or("No such logo pack.")?;
    if pack.entries.is_empty() {
        let _ = std::fs::remove_dir_all(&dir);
        return Err("No pictures were kept, so the pack wasn't installed.".into());
    }
    pack.done = true;
    write_pack(&dir, &pack)?;
    Ok(json!({ "pack": id, "pictures": pack.entries.len() }))
}

fn remove(home: &Path, args: &Value) -> Reply {
    let dir = pack_path(home, arg_str(args, "pack").ok_or("Missing pack.")?)?;
    if dir.exists() {
        std::fs::remove_dir_all(&dir).map_err(|e| e.to_string())?;
    }
    Ok(json!({ "removed": true }))
}

/// One installed pack's pictures with their teams ("Check matches").
fn pack_view(home: &Path, args: &Value) -> Reply {
    let id = arg_str(args, "pack").ok_or("Missing pack.")?;
    let pack = read_pack(&pack_path(home, id)?).ok_or("No such logo pack.")?;
    let rows: Vec<Value> = pack
        .entries
        .iter()
        .map(|e| {
            let c = e.key.as_deref().and_then(cand);
            json!({ "n": e.n, "id": format!("{id}:{}", e.n), "file": e.file, "key": e.key, "team": c.map(|c| c.label.clone()), "level": c.map(|c| c.level.clone()), "from": e.from, "to": e.to, "dark": e.dark })
        })
        .collect();
    Ok(json!({ "pack": id, "name": pack.name, "installed": pack.installed, "rows": rows }))
}

/// Change one picture's team or years, or drop it (`key` null).
fn entry_set(home: &Path, args: &Value) -> Reply {
    let id = arg_str(args, "pack").ok_or("Missing pack.")?;
    let dir = pack_path(home, id)?;
    let mut pack = read_pack(&dir).ok_or("No such logo pack.")?;
    let n = args.get("n").and_then(Value::as_u64).ok_or("Missing picture.")? as u32;
    let e = pack.entries.iter_mut().find(|e| e.n == n).ok_or("No such picture.")?;
    match args.get("key") {
        Some(Value::String(k)) if cand(k).is_some() => e.key = Some(k.clone()),
        Some(Value::Null) => e.key = None,
        Some(_) => return Err("No such team.".into()),
        None => {}
    }
    if args.get("from").is_some() {
        e.from = arg_year(args.get("from"));
    }
    if args.get("to").is_some() {
        e.to = arg_year(args.get("to"));
    }
    write_pack(&dir, &pack)?;
    pack_view(home, &json!({ "pack": id }))
}

// --- The switches and the status -------------------------------------------------------------------------

fn status(game: &Game) -> Value {
    let s = settings(&game.home);
    let list: Vec<Value> = packs(&game.home)
        .iter()
        .map(|(id, p)| {
            let used: Vec<&Entry> = p.entries.iter().filter(|e| e.key.is_some()).collect();
            let mut teams: Vec<&str> = used.iter().filter_map(|e| e.key.as_deref()).filter(|k| !k.starts_with("comp:")).collect();
            teams.sort_unstable();
            teams.dedup();
            let comps = used.iter().filter(|e| e.key.as_deref().is_some_and(|k| k.starts_with("comp:"))).count();
            let bytes: u64 = p.entries.iter().filter_map(|e| std::fs::metadata(packs_dir(&game.home).join(id).join(&e.stored)).ok()).map(|m| m.len()).sum();
            let sample: Vec<String> = used.iter().take(4).map(|e| format!("{id}:{}", e.n)).collect();
            json!({ "id": id, "name": p.name, "installed": p.installed, "pictures": p.entries.len(), "teams": teams.len(), "comps": comps, "bytes": bytes, "sample": sample })
        })
        .collect();
    let fictional = game.world.as_ref().map(|w| w.options.fictional_league);
    json!({ "packs": list, "show": s.show, "fictional": s.fictional, "world_fictional": fictional })
}

fn set(game: &Game, args: &Value) -> Reply {
    let mut s = settings(&game.home);
    if let Some(v) = args.get("show").and_then(Value::as_bool) {
        s.show = v;
    }
    if let Some(v) = args.get("fictional").and_then(Value::as_bool) {
        s.fictional = v;
    }
    save_settings(&game.home, &s)?;
    Ok(status(game))
}

// --- This world's logos ------------------------------------------------------------------------------------

/// Each team's logos by the years they cover, for the page:
/// `teams["nfl:3"] = [[id, from, to, name, dark], ...]` with the name the
/// team had in those years as this world shows it, and `comps[name]` the same
/// for a competition by its name here.
pub(crate) fn map(game: &Game) -> Reply {
    let w = game.w()?;
    let s = settings(&game.home);
    let fictional = w.options.fictional_league;
    if !s.show {
        return Ok(json!({ "show": false, "year": w.year, "teams": {}, "comps": {} }));
    }
    // A pack's logos match the real teams, so a world with fictional names
    // shows them only when switched on; your own logos always show.
    let packs_on = !fictional || s.fictional;
    // key -> [(id, from, to, dark)], a later pack replacing an earlier one.
    let mut by_key: HashMap<String, Vec<(String, i32, i32, bool)>> = HashMap::new();
    for (id, p) in if packs_on { packs(&game.home) } else { Vec::new() } {
        let mut mine: HashMap<String, Vec<(String, i32, i32, bool)>> = HashMap::new();
        for e in &p.entries {
            if let Some(k) = &e.key {
                mine.entry(k.clone()).or_default().push((
                    format!("{id}:{}", e.n),
                    e.from.unwrap_or(0),
                    e.to.unwrap_or(OPEN),
                    e.dark,
                ));
            }
        }
        by_key.extend(mine);
    }
    let mut teams: BTreeMap<String, Vec<Value>> = BTreeMap::new();
    let mut push = |team: String, key: &str, from: i32, to: i32, name: &str| {
        for (id, f, t, dark) in by_key.get(key).into_iter().flatten() {
            let (a, b) = (from.max(*f), to.min(*t));
            if a <= b {
                teams.entry(team.clone()).or_default().push(json!([id, a, b, name, dark]));
            }
        }
    };
    for (i, eras) in ERAS.iter().enumerate().take(w.nfl.len()) {
        for e in eras.iter() {
            push(
                format!("nfl:{i}"),
                &format!("nfl:{}", e.name),
                e.from,
                e.to,
                franchise::shown_name(fictional, i, e.from.max(1920)),
            );
        }
    }
    for (i, p) in w.programs.iter().enumerate() {
        push(format!("college:{i}"), &format!("college:{}", p.real_name), 0, OPEN, w.program_name(i));
    }
    for d in other::TEAMS {
        push(format!("other:{}", d.fr), &format!("other:{}", d.name), d.from, d.to, d.name);
    }
    for (team, (file, dark)) in own_read(game) {
        let Ok(t) = own_ref(w, &team) else { continue };
        teams.insert(team, vec![json!([format!("own:{file}"), 0, OPEN, crate::views::team_name(w, t), dark])]);
    }
    let mut comps: BTreeMap<String, Value> = BTreeMap::new();
    let mut comp = |shown: String, key: String| {
        if let Some(v) = by_key.get(&key) {
            comps.insert(shown, json!(v.iter().map(|(id, f, t, d)| json!([id, f, t, d])).collect::<Vec<_>>()));
        }
    };
    comp("NFL".into(), "comp:NFL".into());
    comp("National Football League".into(), "comp:NFL".into());
    for c in CANDS.iter().filter(|c| c.key.starts_with("comp:") && c.level == "Conference") {
        comp(w.conference_name(&c.label), c.key.clone());
    }
    for b in BOWLS.iter().map(|b| b.name).chain([CELEBRATION_BOWL.0]) {
        comp(b.to_string(), format!("comp:{b}"));
    }
    comp("College Football Playoff".into(), "comp:College Football Playoff".into());
    Ok(json!({ "show": true, "year": w.year, "teams": teams, "comps": comps }))
}

/// The logo a drawn head's jersey wears (question 4 of the mock-up, Cain's
/// yes 2026-10-28), from `logos_map`'s answer: a college's own logo at the
/// V-neck, the NFL shield at the collar, nothing at the other levels. The
/// id goes into the drawn picture's key, so a head is drawn again when its
/// logo changes.
pub(crate) fn jersey(m: &Value, t: TeamRef) -> Option<Value> {
    let year = m["year"].as_i64().unwrap_or(0);
    let covering = |list: &Value| -> Option<String> {
        list.as_array()?
            .iter()
            .find(|e| e[1].as_i64().unwrap_or(0) <= year && year <= e[2].as_i64().unwrap_or(0))
            .and_then(|e| e[0].as_str())
            .map(str::to_string)
    };
    let (id, at) = match t {
        TeamRef::College(i) => (covering(&m["teams"][format!("college:{i}")])?, "chest"),
        TeamRef::Nfl(_) => (covering(&m["comps"]["NFL"])?, "collar"),
        _ => return None,
    };
    Some(json!({ "id": id, "at": at }))
}

/// Pictures by id (`p3:17`, or `own:<file>` for your own), as data URLs.
fn pics(game: &Game, args: &Value) -> Reply {
    let home = &game.home;
    let mut out = serde_json::Map::new();
    let mut cache: HashMap<String, Option<Pack>> = HashMap::new();
    for id in args.get("ids").and_then(Value::as_array).into_iter().flatten().filter_map(Value::as_str).take(200) {
        let path = if let Some(file) = id.strip_prefix("own:") {
            if !own_file_ok(file) {
                continue;
            }
            own_dir(game).join(file)
        } else {
            let Some((p, n)) = id.split_once(':') else { continue };
            let Ok(dir) = pack_path(home, p) else { continue };
            let pack = cache.entry(p.to_string()).or_insert_with(|| read_pack(&dir));
            let Some(e) = pack.as_ref().and_then(|pk| pk.entries.iter().find(|e| n.parse() == Ok(e.n))) else {
                continue;
            };
            dir.join(&e.stored)
        };
        let Ok(bytes) = std::fs::read(&path) else { continue };
        let mime = match path.extension().and_then(|e| e.to_str()) {
            Some("svg") => "image/svg+xml",
            Some("png") => "image/png",
            Some("webp") => "image/webp",
            _ => "image/jpeg",
        };
        out.insert(id.to_string(), json!(format!("data:{mime};base64,{}", b64_encode(&bytes))));
    }
    Ok(json!({ "pics": out }))
}

// --- Your own logos ----------------------------------------------------------------------------

/// The save's own logos: `own.json` (team -> picture and its dark flag) and
/// the pictures beside it.
fn own_dir(game: &Game) -> PathBuf {
    crate::lane_faces::dir(game).join("logos")
}

#[derive(Serialize, Deserialize)]
struct Own {
    file: String,
    #[serde(default)]
    dark: bool,
}

fn own_read(game: &Game) -> BTreeMap<String, (String, bool)> {
    let m: BTreeMap<String, Own> = std::fs::read(own_dir(game).join("own.json"))
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default();
    m.into_iter().filter(|(_, o)| own_file_ok(&o.file)).map(|(k, o)| (k, (o.file, o.dark))).collect()
}

fn own_write(game: &Game, m: &BTreeMap<String, (String, bool)>) -> Result<(), String> {
    let dir = own_dir(game);
    std::fs::create_dir_all(&dir).map_err(|e| format!("Could not keep the logo: {e}"))?;
    let v: BTreeMap<&String, Own> = m.iter().map(|(k, (f, d))| (k, Own { file: f.clone(), dark: *d })).collect();
    std::fs::write(dir.join("own.json"), serde_json::to_vec_pretty(&v).unwrap_or_default())
        .map_err(|e| format!("Could not keep the logo: {e}"))
}

/// A stored picture's name: letters, digits and dashes with one extension,
/// never a path.
fn own_file_ok(f: &str) -> bool {
    !f.is_empty()
        && f.len() < 80
        && !f.starts_with('.')
        && f.matches('.').count() == 1
        && f.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '.')
}

fn own_key(t: TeamRef) -> String {
    let v = t.json();
    format!("{}:{}", v["kind"].as_str().unwrap_or(""), v["id"])
}

fn own_ref(w: &afm_engine::world::World, key: &str) -> Result<TeamRef, String> {
    let (kind, id) = key.split_once(':').ok_or("No such team.")?;
    TeamRef::from_args(w, &json!({ "kind": kind, "id": id.parse::<u64>().map_err(|_| "No such team.")? }))
}

/// "Change logo": one picture for one team, kept with this save.
fn own_set(game: &mut Game, args: &Value) -> Reply {
    let w = game.w()?;
    if args.get("kind").is_none() {
        return Err("Which team?".into());
    }
    let t = TeamRef::from_args(w, args)?;
    let (bytes, ext) = picture(arg_str(args, "data").ok_or("Missing picture.")?)?;
    let key = own_key(t);
    let mut m = own_read(game);
    // A new name each time, so the page never shows the old picture.
    let n = m.get(&key).and_then(|(f, _)| f.split(['-', '.']).nth(2)?.parse::<u32>().ok()).unwrap_or(0) + 1;
    let file = format!("{}-{n}.{ext}", key.replace(':', "-"));
    let dir = own_dir(game);
    std::fs::create_dir_all(&dir).map_err(|e| format!("Could not keep the logo: {e}"))?;
    std::fs::write(dir.join(&file), bytes).map_err(|e| format!("Could not keep the logo: {e}"))?;
    if let Some((old, _)) = m.insert(key, (file, args.get("dark").and_then(Value::as_bool).unwrap_or(false))) {
        let _ = std::fs::remove_file(dir.join(old));
    }
    own_write(game, &m)?;
    Ok(json!({ "ok": true }))
}

fn own_clear(game: &mut Game, args: &Value) -> Reply {
    let w = game.w()?;
    if args.get("kind").is_none() {
        return Err("Which team?".into());
    }
    let key = own_key(TeamRef::from_args(w, args)?);
    let mut m = own_read(game);
    if let Some((old, _)) = m.remove(&key) {
        let _ = std::fs::remove_file(own_dir(game).join(old));
        own_write(game, &m)?;
    }
    Ok(json!({ "ok": true }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn one(name: &str) -> Option<String> {
        let (h, _, _) = find(name);
        (h.len() == 1).then(|| CANDS[h[0]].key.clone())
    }

    #[test]
    fn file_names_find_their_teams() {
        assert_eq!(one("Kansas_City_Chiefs_logo").as_deref(), Some("nfl:Kansas City Chiefs"));
        assert_eq!(one("LA_Rams_logo").as_deref(), Some("nfl:Los Angeles Rams"));
        assert_eq!(one("New_York_Jets_2024").as_deref(), Some("nfl:New York Jets"));
        assert_eq!(one("iowa-st").as_deref(), Some("college:Iowa State"));
        assert_eq!(one("iowa").as_deref(), Some("college:Iowa"));
        assert_eq!(one("coastal-caro").as_deref(), Some("college:Coastal Carolina"));
        assert_eq!(one("alabama-am").as_deref(), Some("college:Alabama A&M"));
        assert_eq!(one("App State").as_deref(), Some("college:Appalachian State"));
        assert_eq!(one("Massachusetts").as_deref(), Some("college:UMass"));
        assert_eq!(one("San José State").as_deref(), Some("college:San Jose State"));
        assert_eq!(one("National_Football_League_logo").as_deref(), Some("comp:NFL"));
        // NCAA short forms: state short forms, St, U, a state at the end.
        assert_eq!(one("old-dominion").as_deref(), Some("college:Old Dominion"));
        assert_eq!(one("eastern-ky").as_deref(), Some("college:Eastern Kentucky"));
        assert_eq!(one("fla-atlantic").as_deref(), Some("college:Florida Atlantic"));
        assert_eq!(one("la-lafayette").as_deref(), Some("college:Louisiana"));
        assert_eq!(one("nc-at").as_deref(), Some("college:North Carolina A&T"));
        assert_eq!(one("southeast-mo-st").as_deref(), Some("college:Southeast Missouri State"));
        assert_eq!(one("missouri-st").as_deref(), Some("college:Missouri State"));
        assert_eq!(one("southern-u").as_deref(), Some("college:Southern"));
        assert_eq!(one("william-mary").as_deref(), Some("college:William & Mary"));
        assert_eq!(one("albany-ny").as_deref(), Some("college:Albany"));
        assert_eq!(one("lindenwood-mo").as_deref(), Some("college:Lindenwood"));
        assert_eq!(one("duke-s-mayo-bowl").as_deref(), Some("comp:Duke's Mayo Bowl"));
        assert_eq!(one("celebration-bowl").as_deref(), Some("comp:Celebration Bowl"));
        assert!(one("boston-u").is_none(), "Boston University is not Boston College");
        // A wordmark or an old logo still finds its team, as another version.
        let (h, variant, _) = find("Kansas_City_Chiefs_wordmark");
        assert_eq!((CANDS[h[0]].key.as_str(), variant), ("nfl:Kansas City Chiefs", true));
        let (h, variant, _) = find("Cleveland_Browns_B");
        assert_eq!((h.len(), variant), (1, true));
        // "Miami" alone fits more than one team: the review asks.
        assert!(find("Miami").0.len() >= 2);
        assert!(find("Brownie_Elf_logo").0.is_empty());
    }

    #[test]
    fn the_review_keeps_one_picture_per_team() {
        let r = match_files(&json!({
            "files": [
                { "path": "nfl/Kansas_City_Chiefs_logo.svg", "size": 900 },
                { "path": "nfl/Kansas_City_Chiefs_wordmark.svg", "size": 900 },
                { "path": "college/fbs-66.png", "size": 30000 },
                { "path": "college/iowa-st.svg", "size": 4000 },
                { "path": "college/fbs-2390.png", "size": 30000 },
                { "path": "readme.txt", "size": 10 },
            ],
            "texts": [{ "path": "college/teams.json", "text": "[{\"name\":\"Iowa State\",\"mascot\":\"Cyclones\",\"logo\":\"/logos/fbs-66.png\"},{\"name\":\"Miami\",\"logo\":\"/logos/fbs-2390.png\"}]" }],
        }));
        let rows = r["rows"].as_array().unwrap();
        assert_eq!(rows.len(), 5, "the text file is no picture");
        let status = |f: &str| rows.iter().find(|x| x["file"] == f).unwrap()["status"].as_str().unwrap().to_string();
        assert_eq!(status("nfl/Kansas_City_Chiefs_logo.svg"), "matched");
        assert_eq!(status("nfl/Kansas_City_Chiefs_wordmark.svg"), "spare");
        assert_eq!(status("college/fbs-2390.png"), "unsure");
        // Two primary pictures of Iowa State: the drawing wins.
        assert_eq!(status("college/iowa-st.svg"), "matched");
        assert_eq!(status("college/fbs-66.png"), "spare");
        assert_eq!(r["counts"]["teams"], 2);
    }

    #[test]
    fn a_mapping_file_comes_first_and_carries_years() {
        let r = match_files(&json!({
            "files": [{ "path": "x/titans.svg" }, { "path": "x/oilers.svg" }],
            "texts": [{ "path": "x/logos.csv", "text": "file,team,from,to\ntitans.svg,Tennessee Titans,1999,\noilers.svg,Houston Oilers,,1996\n" }],
        }));
        let rows = r["rows"].as_array().unwrap();
        assert_eq!(rows[0]["key"], "nfl:Tennessee Titans");
        assert_eq!(rows[0]["from"], 1999);
        assert_eq!(rows[1]["key"], "nfl:Houston Oilers");
        assert_eq!(rows[1]["to"], 1996);
        assert_eq!(rows[1]["how"], "the mapping file");
    }

    #[test]
    fn a_pack_installs_and_comes_out_as_pictures() {
        let home = std::env::temp_dir().join(format!("afm-logos-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&home);
        let svg = format!("data:image/svg+xml;base64,{}", b64_encode(b"<svg xmlns=\"http://www.w3.org/2000/svg\"/>"));
        let id = begin(&home, &json!({ "name": "Test" })).unwrap()["pack"].as_str().unwrap().to_string();
        assert!(packs(&home).is_empty(), "not used before it's done");
        let r = add(&home, &json!({ "pack": id, "files": [{ "file": "a.svg", "key": "nfl:Chicago Bears", "data": svg }, { "file": "b.png", "key": "nfl:Chicago Bears", "data": "data:image/png;base64,AAAA" }] })).unwrap();
        assert_eq!(r["kept"], 1);
        assert_eq!(r["errors"].as_array().unwrap().len(), 1, "a PNG that isn't one is refused");
        commit(&home, &json!({ "pack": id })).unwrap();
        assert_eq!(packs(&home).len(), 1);
        let g = Game::new(home.clone());
        let p = pics(&g, &json!({ "ids": [format!("{id}:0"), "p9:0", "../x:0", "own:../../etc.png"] })).unwrap();
        assert_eq!(p["pics"].as_object().unwrap().len(), 1);
        assert!(pack_path(&home, "../etc").is_err());
        remove(&home, &json!({ "pack": id })).unwrap();
        assert!(packs(&home).is_empty());
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn your_own_logo_wins_and_stays_with_the_save() {
        let home = std::env::temp_dir().join(format!("afm-logos-own-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&home);
        let mut g = Game::new(home.clone());
        g.call("build_world", &json!({ "seed": 11, "history_years": 1, "scale": 0.03, "engine_from": 2026 })).unwrap();
        let real = g.world.as_ref().unwrap().programs[0].real_name.clone();
        let svg = format!("data:image/svg+xml;base64,{}", b64_encode(b"<svg xmlns=\"http://www.w3.org/2000/svg\"/>"));
        let id = begin(&home, &json!({ "name": "Test" })).unwrap()["pack"].as_str().unwrap().to_string();
        add(
            &home,
            &json!({ "pack": id, "files": [{ "file": "a.svg", "key": format!("college:{real}"), "data": svg }] }),
        )
        .unwrap();
        commit(&home, &json!({ "pack": id })).unwrap();
        let first = |g: &Game, team: &str| map(g).unwrap()["teams"][team][0][0].as_str().map(str::to_string);
        assert_eq!(first(&g, "college:0"), Some(format!("{id}:0")));
        assert_eq!(first(&g, "hs:3"), None, "no pack has a generated school");
        // Your own: the school's crest, and the college's own over the pack's.
        own_set(&mut g, &json!({ "kind": "hs", "id": 3, "data": svg, "dark": true })).unwrap();
        own_set(&mut g, &json!({ "kind": "college", "id": 0, "data": svg })).unwrap();
        own_set(&mut g, &json!({ "kind": "college", "id": 0, "data": svg })).unwrap();
        assert_eq!(first(&g, "hs:3").as_deref(), Some("own:hs-3-1.svg"));
        // The jersey: a college wears its logo; no shield in this pack, no school logo on a high school jersey.
        let m = map(&g).unwrap();
        assert_eq!(jersey(&m, TeamRef::College(0)), Some(json!({ "id": "own:college-0-2.svg", "at": "chest" })));
        assert_eq!(jersey(&m, TeamRef::Nfl(0)), None);
        assert_eq!(jersey(&m, TeamRef::HighSchool(3)), None);
        assert_eq!(first(&g, "college:0").as_deref(), Some("own:college-0-2.svg"), "a new name each time");
        assert!(!own_dir(&g).join("college-0-1.svg").exists(), "the old picture goes");
        assert!(own_dir(&g).starts_with(home.join("faces")), "kept in the save's faces folder");
        let p = pics(&g, &json!({ "ids": ["own:hs-3-1.svg"] })).unwrap();
        assert_eq!(p["pics"].as_object().unwrap().len(), 1);
        assert!(own_set(&mut g, &json!({ "kind": "hs", "id": 3, "data": "data:image/png;base64,AAAA" })).is_err());
        own_clear(&mut g, &json!({ "kind": "college", "id": 0 })).unwrap();
        assert_eq!(first(&g, "college:0"), Some(format!("{id}:0")), "the pack's logo comes back");
        let _ = std::fs::remove_dir_all(&home);
    }
}
