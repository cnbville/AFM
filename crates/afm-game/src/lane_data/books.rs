//! Stat history (items 15-22, board 43) and the teams' own books (item 17).

use afm_engine::calendar::Date;
use afm_engine::world::World;
use afm_engine::world::data::Division;
use afm_engine::world::marks::{self, CAREER, GAME, Mark, RATES, SEASON, Tier};
use afm_engine::world::records::{Book, RECORD_STATS, RecordEntry, Scope, Stat};
use serde_json::{Value, json};

use super::{division_arg, fmt_num, state_arg, team_json, tier_arg, tier_key};
use crate::views::{self, TeamRef};

/// The real NFL records (item 22), shown on top of the world's lists and
/// marked: regular season, through the 2025 season. (kind, `RECORD_STATS`
/// index or rate, value, holder, year). Tackles and tackles for loss have
/// no official records, and return touchdowns aren't listed.
const REAL_NFL: [(u8, usize, f64, &str, &str); 30] = [
    (SEASON, 0, 5477.0, "Peyton Manning", "2013"),
    (SEASON, 1, 55.0, "Peyton Manning", "2013"),
    (SEASON, 2, 2105.0, "Eric Dickerson", "1984"),
    (SEASON, 3, 28.0, "LaDainian Tomlinson", "2006"),
    (SEASON, 4, 149.0, "Michael Thomas", "2019"),
    (SEASON, 5, 1964.0, "Calvin Johnson", "2012"),
    (SEASON, 6, 23.0, "Randy Moss", "2007"),
    (SEASON, 8, 23.0, "Myles Garrett", "2025"),
    (SEASON, 9, 14.0, "Dick \"Night Train\" Lane", "1952"),
    (SEASON, 11, 44.0, "David Akers", "2011"),
    (CAREER, 0, 89214.0, "Tom Brady", "2000-2022"),
    (CAREER, 1, 649.0, "Tom Brady", "2000-2022"),
    (CAREER, 2, 18355.0, "Emmitt Smith", "1990-2004"),
    (CAREER, 3, 164.0, "Emmitt Smith", "1990-2004"),
    (CAREER, 4, 1549.0, "Jerry Rice", "1985-2004"),
    (CAREER, 5, 22895.0, "Jerry Rice", "1985-2004"),
    (CAREER, 6, 197.0, "Jerry Rice", "1985-2004"),
    (CAREER, 8, 200.0, "Bruce Smith", "1985-2003"),
    (CAREER, 9, 81.0, "Paul Krause", "1964-1979"),
    (CAREER, 11, 599.0, "Adam Vinatieri", "1996-2019"),
    (GAME, 0, 554.0, "Norm Van Brocklin", "1951"),
    (GAME, 1, 7.0, "Eight passers, last Drew Brees", "2015"),
    (GAME, 2, 296.0, "Adrian Peterson", "2007"),
    (GAME, 3, 6.0, "Ernie Nevers and Alvin Kamara", "1929, 2020"),
    (GAME, 4, 21.0, "Brandon Marshall", "2009"),
    (GAME, 5, 336.0, "Willie \"Flipper\" Anderson", "1989"),
    (GAME, 6, 5.0, "Bob Shaw, Kellen Winslow and Jerry Rice", "1950-1990"),
    (GAME, 8, 7.0, "Derrick Thomas", "1990"),
    (GAME, 11, 8.0, "Rob Bironas", "2007"),
    // Rates: kind 3 a season's.
    (3, 0, 122.5, "Aaron Rodgers", "2011"),
];

/// The real 2018 completion record, kept apart since the table above is
/// counts plus the passer rating: Drew Brees's 74.4% (minimum 14 attempts
/// a team game).
const REAL_CMP: (f64, &str, &str) = (74.4, "Drew Brees", "2018");

fn real(kind: u8, j: usize) -> Value {
    if kind == 3 && j == 1 {
        return json!({ "value": format!("{:.1}", REAL_CMP.0), "holder": REAL_CMP.1, "year": REAL_CMP.2 });
    }
    REAL_NFL.iter().find(|r| r.0 == kind && r.1 == j).map_or(Value::Null, |r| {
        json!({ "value": if kind == 3 { format!("{:.1}", r.2) } else { fmt_num(r.2) }, "holder": r.3, "year": r.4 })
    })
}

const KINDS: [&str; 3] = ["career", "season", "game"];

fn kind_of(b: Book) -> &'static str {
    match b {
        Book::Career(_) => "career",
        Book::Season(_) => "season",
        Book::Game(_) => "game",
        Book::RateSeason(_) => "rate_season",
        Book::RateCareer(_) => "rate_career",
        Book::Streak(_) | Book::WinStreak => "streak",
    }
}

pub(crate) fn title_case(s: &str) -> String {
    let mut c = s.chars();
    c.next().map_or_else(String::new, |f| f.to_uppercase().collect::<String>() + c.as_str())
}

fn rate_value(k: usize, v: f64) -> String {
    match k {
        1 | 4 => format!("{v:.1}%"),
        0 => format!("{v:.1}"),
        _ => format!("{v:.2}"),
    }
}

fn year_span(from: i32, year: i32) -> String {
    if from != year && from != 0 { format!("{from}-{year}") } else { year.to_string() }
}

fn date_text(d: Date) -> String {
    views::date_text(d)
}

/// A level list's entry.
fn entry_json(w: &World, tier: Tier, rank: usize, e: &RecordEntry, b: Book) -> Value {
    // Entries from before lane L20 carry the school in the name and no team.
    let team = if e.program == usize::MAX { Value::Null } else { team_json(w, tier, e.program) };
    let value = match b {
        Book::RateSeason(k) | Book::RateCareer(k) => rate_value(k, e.value),
        _ => fmt_num(e.value),
    };
    let per_game = (e.games > 0 && matches!(b, Book::Season(_) | Book::Career(_))).then(|| e.value / e.games as f64);
    json!({
        "rank": rank + 1,
        "value": value,
        "raw": e.value,
        "id": e.id,
        "name": e.name,
        "linked": e.id != 0 && views::person_index(w, e.id).is_some(),
        "pos": e.pos.map(|p| p.name()),
        "team": team,
        "year": year_span(e.from, e.year),
        "games": (e.games > 0).then_some(e.games),
        "per_game": per_game.map(|x| format!("{x:.1}")),
        "era": (e.era > 0.0).then(|| e.era.round() as i32),
        "live": e.live,
        "game": e.game.map(|g| json!({
            "date": date_text(g.date),
            "opp": team_json(w, tier, g.opp),
            "home": g.home,
            "score": format!("{}-{}", g.score[0], g.score[1]),
            "won": g.score[0] > g.score[1],
        })),
    })
}

/// Which level book a page shows.
fn scope_of(w: &World, tier: Tier, args: &Value) -> Scope {
    match tier {
        Tier::Nfl => Scope::Nfl,
        Tier::Hs => Scope::State(state_arg(w, args)),
        Tier::College => {
            let d = division_arg(w, args);
            match args.get("conference").and_then(Value::as_str).filter(|c| !c.is_empty()) {
                Some(c) => Scope::Conference(d, c.to_string()),
                None => Scope::Division(d),
            }
        }
    }
}

fn scope_name(w: &World, s: &Scope) -> String {
    match s {
        Scope::Division(d) => d.name().to_string(),
        Scope::Conference(d, c) => format!("{c} ({})", d.name()),
        Scope::State(i) => w.states.get(*i).map_or_else(|| "a state".into(), |x| x.code.clone()),
        Scope::Nfl => "NFL".into(),
    }
}

/// The record stats a level keeps (high school has no return touchdowns).
fn stats_for(tier: Tier) -> Vec<usize> {
    (0..RECORD_STATS.len()).filter(|&j| tier != Tier::Hs || RECORD_STATS[j] != Stat::ReturnTd).collect()
}

/// The rate records' minimums, in words (`marks::season_qualifies`).
fn rate_min(k: usize, tier: Tier, career: bool) -> String {
    let nfl = tier == Tier::Nfl;
    let half = if tier == Tier::Hs { 0.5 } else { 1.0 };
    if career {
        return match (nfl, k) {
            (true, 0 | 1) => "Minimum 1,500 attempts".into(),
            (true, 2) => "Minimum 750 carries".into(),
            (true, 3) => "Minimum 200 catches".into(),
            (true, _) => "Minimum 100 field goals".into(),
            (false, 0 | 1) => format!("Minimum {} completions", (325.0 * half) as u32),
            (false, 2) => format!("Minimum {} carries", (300.0 * half) as u32),
            (false, 3) => format!("Minimum {} catches", (100.0 * half) as u32),
            (false, _) => format!("Minimum {} field goal tries", (40.0 * half) as u32),
        };
    }
    match k {
        0 | 1 => format!("Minimum {} attempts a team game", if nfl { 14 } else { 15 }),
        2 => "Minimum 6.25 carries a team game".into(),
        3 => "Minimum 1.875 catches a team game".into(),
        _ => "Minimum one field goal try a team game".into(),
    }
}

pub fn stat_history(w: &World, args: &Value) -> Value {
    let tier = tier_arg(w, args);
    let scope = scope_of(w, tier, args);
    let tab = args.get("tab").and_then(Value::as_str).unwrap_or("career");
    let focus = args.get("stat").and_then(Value::as_u64).map(|x| x as usize);
    let depth = |j: usize| if focus == Some(j) { 25 } else { 10 };
    let get = |b: Book| w.stats.books.get(&(scope.clone(), b));
    let conference = matches!(scope, Scope::Conference(..));
    let mut lists = Vec::new();
    let list_json = |b: Book, top: usize, name: String, extra: Value| {
        let rows: Vec<Value> =
            get(b).into_iter().flatten().take(top).enumerate().map(|(i, e)| entry_json(w, tier, i, e, b)).collect();
        let mut v = json!({ "key": format!("{}:{}", kind_of(b), match b { Book::Season(j) | Book::Career(j) | Book::Game(j) | Book::RateSeason(j) | Book::RateCareer(j) | Book::Streak(j) => j, Book::WinStreak => 0 }), "kind": kind_of(b), "name": name, "rows": rows });
        if let (Value::Object(m), Value::Object(x)) = (&mut v, extra) {
            m.extend(x);
        }
        v
    };
    match tab {
        "career" | "season" | "game" => {
            for j in stats_for(tier) {
                let (b, kind) = match tab {
                    "career" => (Book::Career(j), CAREER),
                    "season" => (Book::Season(j), SEASON),
                    _ => (Book::Game(j), GAME),
                };
                if conference && tab == "career" {
                    break;
                }
                let real = if tier == Tier::Nfl { real(kind, j) } else { Value::Null };
                lists.push(list_json(b, depth(j), title_case(&b.name()), json!({ "stat": j, "real": real })));
            }
            if tab == "career" && !conference {
                for (k, s) in afm_engine::world::records::STREAKS.iter().enumerate() {
                    if get(Book::Streak(k)).is_some_and(|l| !l.is_empty()) {
                        lists.push(list_json(Book::Streak(k), 10, title_case(s), json!({ "stat": null })));
                    }
                }
            }
        }
        "rates" => {
            for k in 0..RATES.len() {
                for career in [false, true] {
                    if conference && career {
                        continue;
                    }
                    let b = if career { Book::RateCareer(k) } else { Book::RateSeason(k) };
                    let name = if k == 0 && tier == Tier::College {
                        format!("{} passing efficiency", if career { "Career" } else { "Single-season" })
                    } else {
                        title_case(&b.name())
                    };
                    let real = if tier == Tier::Nfl && !career { real(3, k) } else { Value::Null };
                    lists.push(list_json(
                        b,
                        10,
                        name,
                        json!({ "stat": k, "min": rate_min(k, tier, career), "real": real }),
                    ));
                }
            }
        }
        _ => {}
    }
    let leaders = if tab == "leaders" { leaders(w, tier, &scope, focus.unwrap_or(0)) } else { Value::Null };
    let team = match (args.get("team"), tab) {
        (Some(t), "team") if !t.is_null() => TeamRef::from_args(w, t).ok(),
        (_, "team") => views::mine(w).filter(|t| same_tier(*t, tier)),
        _ => None,
    };
    let team_book = team.map(|t| team_book_json(w, t, focus));
    // Your own team's #1 in each stat, beside the lists.
    let own = views::mine(w).filter(|t| same_tier(*t, tier)).map(|t| own_top(w, t));
    json!({
        "level": tier_key(tier),
        "scope": scope_name(w, &scope),
        "division": match &scope { Scope::Division(d) | Scope::Conference(d, _) => Value::from(d.name()), _ => Value::Null },
        "conference": match &scope { Scope::Conference(_, c) => Value::from(c.as_str()), _ => Value::Null },
        "state": match &scope { Scope::State(s) => w.states.get(*s).map_or(Value::Null, |x| Value::from(x.code.as_str())), _ => Value::Null },
        "divisions": Division::ALL.iter().map(|d| d.name()).collect::<Vec<_>>(),
        "conferences": conferences(w, &scope),
        "states": w.states.iter().map(|s| s.code.as_str()).collect::<Vec<_>>(),
        "stats": RECORD_STATS.iter().map(|s| title_case(s.name())).collect::<Vec<_>>(),
        "tab": tab,
        "lists": lists,
        "leaders": leaders,
        "team_book": team_book,
        "own": own,
        "chasing": chasing(w, tier, &scope),
        "fell": fell(w, tier, &scope),
        "since": since(w, tier),
        "note": note(tier),
        "source": super::sources(w)["line"],
    })
}

fn same_tier(t: TeamRef, tier: Tier) -> bool {
    matches!(
        (t, tier),
        (TeamRef::College(_), Tier::College) | (TeamRef::Nfl(_), Tier::Nfl) | (TeamRef::HighSchool(_), Tier::Hs)
    )
}

fn note(tier: Tier) -> &'static str {
    match tier {
        Tier::Nfl => {
            "Regular season only, as the league keeps its books. Era: 100 is that season's average for a regular at his position."
        }
        Tier::College => {
            "Bowls and playoff games count, as the NCAA's books have since 2002. A career counts his seasons in the division. Era: 100 is that season's average for a regular at his position in the division."
        }
        Tier::Hs => {
            "Playoff games count, as the state associations keep them. A career counts his seasons in the state. Era: 100 is that season's average for a regular at his position in the state."
        }
    }
}

/// The first season in a level's books.
fn since(w: &World, tier: Tier) -> Option<i32> {
    let f = w.marks.first[tier as usize];
    if f != 0 {
        return Some(f);
    }
    match tier {
        Tier::College => w.stats.year_index.first().map(|x| x.0),
        _ => None,
    }
}

fn conferences(w: &World, scope: &Scope) -> Vec<String> {
    let d = match scope {
        Scope::Division(d) | Scope::Conference(d, _) => *d,
        _ => return Vec::new(),
    };
    let mut v: Vec<String> = w
        .programs
        .iter()
        .filter(|p| p.division == d && !p.conference.is_empty())
        .map(|p| p.conference.clone())
        .collect();
    v.sort();
    v.dedup();
    v
}

/// Each year's leader and runner-up in one stat (item 15). College reads
/// the stat book, which keeps every season; the NFL and the states read the
/// record store's.
fn leaders(w: &World, tier: Tier, scope: &Scope, j: usize) -> Value {
    let j = j.min(RECORD_STATS.len() - 1);
    let b = Book::Season(j);
    let rows: Vec<Value> = match scope {
        Scope::Division(_) | Scope::Conference(..) => {
            let stat = RECORD_STATS[j];
            w.stats
                .year_index
                .iter()
                .rev()
                .filter_map(|&(year, _)| {
                    let mut best: Vec<&afm_engine::world::records::SeasonRecord> = w
                        .stats
                        .year(year)
                        .iter()
                        .filter(|r| match scope {
                            Scope::Division(d) => r.division == *d,
                            Scope::Conference(d, c) => {
                                r.division == *d
                                    && w.programs.get(r.program as usize).is_some_and(|p| p.conference == *c)
                            }
                            _ => false,
                        })
                        .filter(|r| r.get(stat) > 0.0)
                        .collect();
                    best.sort_by(|a, b| b.get(stat).total_cmp(&a.get(stat)).then(a.id.cmp(&b.id)));
                    let entry = |r: &afm_engine::world::records::SeasonRecord| {
                        let norm =
                            w.stats.norms.get(&(year, r.division, r.pos)).map_or(0.0, |n| n[stat as usize] as f64);
                        RecordEntry {
                            value: r.get(stat),
                            id: r.id,
                            name: marks::name_of(w, r.id).unwrap_or_else(|| "a former player".into()),
                            program: r.program as usize,
                            year,
                            from: year,
                            games: r.games as u16,
                            pos: Some(r.pos),
                            era: marks::era(r.get(stat), norm * r.games as f64),
                            ..Default::default()
                        }
                    };
                    let first = best.first()?;
                    Some(json!({
                        "year": year,
                        "first": entry_json(w, tier, 0, &entry(first), b),
                        "second": best.get(1).map(|r| entry_json(w, tier, 1, &entry(r), b)),
                    }))
                })
                .collect()
        }
        _ => {
            let mut v: Vec<&marks::Leader> =
                w.marks.leaders.iter().filter(|l| l.scope == *scope && l.stat as usize == j).collect();
            v.sort_by_key(|l| std::cmp::Reverse(l.year));
            v.iter()
                .map(|l| {
                    json!({
                        "year": l.year,
                        "first": entry_json(w, tier, 0, &l.first, b),
                        "second": l.second.as_ref().map(|e| entry_json(w, tier, 1, e, b)),
                    })
                })
                .collect()
        }
    };
    json!({ "stat": j, "name": title_case(RECORD_STATS[j].name()), "rows": rows })
}

fn chasing(w: &World, tier: Tier, scope: &Scope) -> Vec<Value> {
    marks::chasing(w, scope)
        .iter()
        .map(|c| {
            let season = matches!(c.book, Book::Season(_));
            json!({
                "book": title_case(&c.book.name()),
                "kind": if season { "season" } else { "career" },
                "id": c.id,
                "name": c.name,
                "linked": views::person_index(w, c.id).is_some(),
                "team": team_json(w, tier, c.team),
                "have": fmt_num(c.have),
                "pace": c.pace.map(fmt_num),
                "games": c.games,
                "record": fmt_num(c.record),
                "holder": c.holder,
                "holder_year": c.holder_year,
                "needs": fmt_num((c.record - c.have).max(0.0)),
                "pct": (100.0 * c.pace.unwrap_or(c.have) / c.record.max(1e-9)).round() as i64,
            })
        })
        .collect()
}

/// Whether a team at a level is in a level book's reach (a division's or
/// conference's programs, the NFL, a state's schools).
fn in_scope(w: &World, scope: &Scope, team: usize) -> bool {
    match scope {
        Scope::Division(d) => w.programs.get(team).is_some_and(|p| p.division == *d),
        Scope::Conference(d, c) => w.programs.get(team).is_some_and(|p| p.division == *d && p.conference == *c),
        Scope::State(s) => w.high_schools.get(team).is_some_and(|h| h.state == *s),
        Scope::Nfl => true,
    }
}

/// The records that fell this season and last (item 21): the level's own
/// lists, then its teams' books.
fn fell(w: &World, tier: Tier, scope: &Scope) -> Vec<Value> {
    let mut v: Vec<&marks::Fell> = w
        .marks
        .fell
        .iter()
        .filter(|f| f.tier() == tier)
        .filter(|f| match (&f.scope, f.team) {
            (Some(s), _) => s == scope,
            (None, Some((_, t))) => in_scope(w, scope, t as usize),
            _ => false,
        })
        .collect();
    v.sort_by(|a, b| b.date.cmp(&a.date).then(a.team.is_some().cmp(&b.team.is_some())));
    v.iter()
        .take(40)
        .map(|f| {
            let rate = matches!(f.book, Book::RateSeason(_) | Book::RateCareer(_));
            json!({
                "date": date_text(f.date),
                "year": f.year,
                "book": title_case(&f.book.name()),
                "where": match f.team {
                    Some((t, i)) => team_json(w, t, i as usize),
                    None => Value::Null,
                },
                "level": f.scope.as_ref().map(|s| scope_name(w, s)),
                "id": f.id,
                "name": f.name,
                "linked": views::person_index(w, f.id).is_some(),
                "team": team_json(w, tier, f.program),
                "value": if rate { format!("{:.1}", f.value) } else { fmt_num(f.value) },
                "old": {
                    "id": f.old_id,
                    "name": f.old_name,
                    "linked": f.old_id != 0 && views::person_index(w, f.old_id).is_some(),
                    "value": if rate { format!("{:.1}", f.old_value) } else { fmt_num(f.old_value) },
                    "year": f.old_year,
                },
            })
        })
        .collect()
}

fn tier_of(t: TeamRef) -> Option<(Tier, usize)> {
    match t {
        TeamRef::College(i) => Some((Tier::College, i)),
        TeamRef::Nfl(i) => Some((Tier::Nfl, i)),
        TeamRef::HighSchool(i) => Some((Tier::Hs, i)),
        TeamRef::Other(_) => None,
    }
}

fn mark_json(w: &World, tier: Tier, rank: usize, m: &Mark) -> Value {
    let kind = m.kind();
    let game = (kind == GAME).then(|| {
        let (opp, score) = m.game_parts();
        json!({
            "date": date_text(Date(m.date)),
            "opp": team_json(w, tier, opp),
            "home": m.home(),
            "score": format!("{}-{}", score[0], score[1]),
            "won": score[0] > score[1],
        })
    });
    json!({
        "rank": rank + 1,
        "value": fmt_num(m.value as f64),
        "id": m.id,
        "name": marks::name_of(w, m.id).unwrap_or_else(|| "a former player".into()),
        "linked": views::person_index(w, m.id).is_some(),
        "year": if kind == CAREER { year_span(m.first_year(), m.year as i32) } else { m.year.to_string() },
        "games": (kind != GAME && m.games > 0).then_some(m.games),
        "live": m.live(),
        "game": game,
    })
}

fn team_book_json(w: &World, t: TeamRef, focus: Option<usize>) -> Value {
    let Some((tier, team)) = tier_of(t) else { return Value::Null };
    let book = w.marks.team_book(tier, team);
    let mut lists = Vec::new();
    for (kind, word) in [(CAREER, "Career"), (SEASON, "Season"), (GAME, "Single-game")] {
        for j in stats_for(tier) {
            if focus.is_some_and(|f| f != j) {
                continue;
            }
            let rows: Vec<Value> =
                marks::team_list(book, kind, j).iter().enumerate().map(|(i, m)| mark_json(w, tier, i, m)).collect();
            lists.push(json!({
                "kind": KINDS[kind as usize],
                "stat": j,
                "name": format!("{word} {}", RECORD_STATS[j].name()),
                "rows": rows,
            }));
        }
    }
    json!({
        "team": views::team_brief(w, t),
        "level": tier_key(tier),
        "top": tier.team_top(),
        "since": since(w, tier),
        "lists": lists,
    })
}

/// Your team's record holder in each stat (career, season and game), for
/// the side panel.
fn own_top(w: &World, t: TeamRef) -> Value {
    let Some((tier, team)) = tier_of(t) else { return Value::Null };
    let book = w.marks.team_book(tier, team);
    let rows: Vec<Value> = stats_for(tier)
        .into_iter()
        .filter_map(|j| {
            let first = |k: u8| marks::team_list(book, k, j).first().map(|m| mark_json(w, tier, 0, m));
            let (c, s, g) = (first(CAREER), first(SEASON), first(GAME));
            (c.is_some() || s.is_some() || g.is_some()).then(|| {
                json!({ "stat": j, "name": title_case(RECORD_STATS[j].name()), "career": c, "season": s, "game": g })
            })
        })
        .collect();
    json!({ "team": views::team_brief(w, t), "rows": rows })
}

/// A team's own book (item 17).
pub fn team_book(w: &World, args: &Value) -> Value {
    let Ok(t) = TeamRef::from_args(w, args) else { return json!({ "error": "No such team." }) };
    let focus = args.get("stat").and_then(Value::as_u64).map(|x| x as usize);
    let mut v = team_book_json(w, t, focus);
    if let Value::Object(m) = &mut v {
        m.insert("stats".into(), json!(RECORD_STATS.iter().map(|s| title_case(s.name())).collect::<Vec<_>>()));
    }
    v
}
