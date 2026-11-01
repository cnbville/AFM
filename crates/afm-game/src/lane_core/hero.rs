//! The Home hero's three big slides (update 3, lane 16, HH1-HH5), built
//! from the game's own record and shown before the older slides:
//!
//! - A, under the lights: your stadium at night, its seats filled with the
//!   real crowd share, the captains at midfield, the road from here and what
//!   is asked of you.
//! - B, the program's story: wins in every season on record by head coach,
//!   and what you could be the first to do, each worked out from the record.
//! - C, the room: last season's starters in a team photo with an empty spot
//!   for you, and what the room is like.
//!
//! Day one welcomes you; after it each slide follows the year (HH5): A the
//! next game, the last result and the last home crowd; B's marks as each is
//! reached or missed; C the room's mood, who's unhappy and who's hurt.
//! Nothing hidden shows: no ratings at all, only records, names and words.

use afm_engine::calendar::{Date, Level as CalLevel};
use afm_engine::player::PlayingTime;
use afm_engine::positions::Position;
use afm_engine::world::coach::{Employer, Role};
use afm_engine::world::folk::{Team, boss, takeover, talks};
use afm_engine::world::season_page::{self as sp, Held, PageGame, SeasonPage};
use afm_engine::world::stories::{numbers, traditions};
use afm_engine::world::{Stage, World};
use serde_json::{Value, json};

use crate::views::{self, TeamRef};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Lv {
    Hs,
    College,
    Nfl,
    Other,
}

impl Lv {
    fn key(self) -> &'static str {
        match self {
            Lv::Hs => "hs",
            Lv::College => "college",
            Lv::Nfl => "nfl",
            Lv::Other => "other",
        }
    }
    /// The title its gold caps mark.
    fn title(self) -> &'static str {
        match self {
            Lv::Hs => "state title",
            Lv::College => "conference title",
            Lv::Nfl => "Super Bowl",
            Lv::Other => "league title",
        }
    }
}

/// One season on record, oldest first.
struct Season {
    year: i32,
    w: u32,
    l: u32,
    t: u32,
    coach: Option<usize>,
    /// The level's big title: a state title, a national title, a Super Bowl.
    title: bool,
    /// A conference title (college).
    conf: bool,
    /// The NFL's playoff round reached (0 none, 5 the Super Bowl won).
    round: u32,
}

impl Season {
    fn rec(&self) -> String {
        wlt(self.w, self.l, self.t)
    }
    fn winning(&self) -> bool {
        self.w > self.l
    }
}

fn wlt(w: u32, l: u32, t: u32) -> String {
    if t > 0 { format!("{w}-{l}-{t}") } else { format!("{w}-{l}") }
}

/// Everything the three slides share.
struct Ctx<'a> {
    w: &'a World,
    me: TeamRef,
    lv: Lv,
    you: Option<usize>,
    surname: String,
    name: String,
    colors: [String; 2],
    seasons: Vec<Season>,
    /// `seasons[..before]` came before your time here.
    before: usize,
    day_one: bool,
    /// Your job when you are an assistant ("offensive coordinator"), and the
    /// head coach you work for; None and None when the team is yours.
    role: Option<&'static str>,
    head: Option<usize>,
    /// Draft picks the program sent up (college) or the club made (NFL):
    /// (year, round, pick, NFL team, person), looked up once.
    drafted: Vec<(i32, u32, u32, usize, usize)>,
    /// This season's page and last season's (college and the NFL keep both).
    this: Option<SeasonPage>,
    last: Option<SeasonPage>,
}

impl Ctx<'_> {
    fn coach_name(&self, c: Option<usize>) -> String {
        c.and_then(|c| self.w.coaches.get(c)).map_or_else(|| "an interim coach".to_string(), |c| c.name.clone())
    }
    fn prev(&self) -> &[Season] {
        &self.seasons[..self.before]
    }
    fn first_year(&self) -> i32 {
        self.seasons.first().map_or(self.w.year, |s| s.year)
    }
    /// The season pages newest first: this one, last one, then your kept ones.
    fn pages(&self) -> Vec<&SeasonPage> {
        self.this.iter().chain(self.last.iter()).collect()
    }
    /// Your games this season: (played, still to play) in the regular season.
    fn this_record(&self) -> Option<(u32, u32, u32, usize)> {
        let p = self.this.as_ref()?;
        let regular: Vec<&PageGame> = p.games.iter().filter(|g| !g.playoff && !g.bowl && !g.cancelled).collect();
        let (mut w, mut l, mut t) = (0, 0, 0);
        for g in &regular {
            match (g.score, g.won) {
                (Some([a, b]), _) if a > b => w += 1,
                (Some([a, b]), _) if a < b => l += 1,
                (Some(_), _) => t += 1,
                (None, Some(true)) => w += 1,
                (None, Some(false)) => l += 1,
                _ => {}
            }
        }
        let left = regular.iter().filter(|g| g.score.is_none() && g.won.is_none()).count();
        Some((w, l, t, left))
    }
    /// This season under way with you: some games played, none in history yet.
    fn in_season(&self) -> bool {
        let year = self.this.as_ref().map(|p| p.year);
        self.this_record().is_some_and(|(w, l, t, _)| w + l + t > 0)
            && !self.seasons.iter().any(|s| Some(s.year) == year)
    }
}

fn level_of(t: TeamRef) -> Option<(CalLevel, usize)> {
    match t {
        TeamRef::College(i) => Some((CalLevel::College, i)),
        TeamRef::Nfl(i) => Some((CalLevel::Nfl, i)),
        TeamRef::HighSchool(i) => Some((CalLevel::HighSchool, i)),
        TeamRef::Other(_) => None,
    }
}

fn seasons(w: &World, t: TeamRef) -> Vec<Season> {
    let mut v: Vec<Season> = match t {
        TeamRef::College(p) => w.programs[p]
            .seasons
            .iter()
            .map(|s| Season {
                year: s.year,
                w: s.wins,
                l: s.losses,
                t: 0,
                coach: s.coach,
                title: s.national_title,
                conf: s.conference_title,
                round: 0,
            })
            .collect(),
        TeamRef::Nfl(i) => w
            .nfl_seasons
            .iter()
            .filter(|s| s.team == i)
            .map(|s| Season {
                year: s.year,
                w: s.wins,
                l: s.losses,
                t: s.ties,
                coach: s.coach,
                title: s.round == 5,
                conf: false,
                round: s.round,
            })
            .collect(),
        TeamRef::HighSchool(h) => w.high_schools[h]
            .seasons
            .iter()
            .map(|s| Season {
                year: s.year,
                w: s.wins,
                l: s.losses,
                t: 0,
                coach: s.coach,
                title: s.state_title,
                conf: false,
                round: 0,
            })
            .collect(),
        TeamRef::Other(i) => afm_engine::world::pro::other::team_seasons(w, i)
            .into_iter()
            .map(|(year, wn, l, note)| Season {
                year,
                w: wn,
                l,
                t: 0,
                coach: None,
                title: note.starts_with("won"),
                conf: false,
                round: 0,
            })
            .collect(),
    };
    v.sort_by_key(|s| s.year);
    v.dedup_by_key(|s| s.year);
    let n = v.len();
    v.split_off(n.saturating_sub(30))
}

fn ord(n: usize) -> String {
    let s = match (n % 10, n % 100) {
        (_, 11..=13) => "th",
        (1, _) => "st",
        (2, _) => "nd",
        (3, _) => "rd",
        _ => "th",
    };
    format!("{n}{s}")
}

fn word(n: usize) -> String {
    const W: [&str; 11] = ["No", "One", "Two", "Three", "Four", "Five", "Six", "Seven", "Eight", "Nine", "Ten"];
    W.get(n).map_or_else(|| n.to_string(), |x| x.to_string())
}

fn thousands(n: u32) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

/// Dollars in millions, the minus sign first: "-$14.2M".
fn millions(x: f64) -> String {
    let m = x / 1e6;
    if m < 0.0 { format!("-${:.1}M", -m) } else { format!("${m:.1}M") }
}

fn last_name(full: &str) -> String {
    let mut parts: Vec<&str> = full.split_whitespace().collect();
    while parts.len() > 1 && matches!(*parts.last().unwrap_or(&""), "Jr." | "Sr." | "II" | "III" | "IV") {
        parts.pop();
    }
    parts.last().map_or_else(String::new, |s| s.to_string())
}

/// A year span the way the boards print them: "2010-12".
fn years_text(ys: &[i32]) -> String {
    let mut out: Vec<String> = Vec::new();
    let mut k = 0;
    while k < ys.len() {
        let mut j = k;
        while j + 1 < ys.len() && ys[j + 1] == ys[j] + 1 {
            j += 1;
        }
        out.push(if j > k { format!("{}-{:02}", ys[k], ys[j] % 100) } else { ys[k].to_string() });
        k = j + 1;
    }
    out.join(", ")
}

/// The three slides, A first (HH4: more can join this list later).
pub fn slides(w: &World, me: TeamRef) -> Vec<Value> {
    let lv = match me {
        TeamRef::HighSchool(_) => Lv::Hs,
        TeamRef::College(_) => Lv::College,
        TeamRef::Nfl(_) => Lv::Nfl,
        TeamRef::Other(_) => Lv::Other,
    };
    let you = w.career.as_ref().map(|c| c.coach);
    let surname = you.and_then(|c| w.coaches.get(c)).map_or_else(String::new, |c| last_name(&c.name));
    let seasons = seasons(w, me);
    let before = seasons.iter().position(|s| s.coach.is_some() && s.coach == you).unwrap_or(seasons.len());
    let day_one = w.career.as_ref().is_some_and(|c| c.started == w.today);
    let role = w.career.as_ref().and_then(|c| c.role).filter(|r| *r != Role::HeadCoach);
    let employer = match me {
        TeamRef::HighSchool(i) => Employer::HighSchool(i),
        TeamRef::College(i) => Employer::College(i),
        TeamRef::Nfl(i) => Employer::Nfl(i),
        TeamRef::Other(i) => Employer::Other(i),
    };
    let head = role.and_then(|_| {
        w.coaches.iter().position(|c| c.job.is_some_and(|j| j.employer == employer && j.role == Role::HeadCoach))
    });
    let drafted = drafted(w, me);
    let (this, last) = match level_of(me) {
        Some((l, i)) => (sp::page(w, l, i, Held::This), sp::page(w, l, i, Held::Last)),
        None => (None, None),
    };
    let c = Ctx {
        w,
        me,
        lv,
        you,
        surname,
        name: views::team_name(w, me),
        colors: views::team_colors(w, me),
        seasons,
        before,
        day_one,
        role: role.map(Role::name),
        head,
        drafted,
        this,
        last,
    };
    vec![stadium(&c), story_slide(&c), room(&c)]
}

// ------------------------------------------------------------------ the story

/// The program's story in one sentence and its line under it, from every
/// season on record (yours too, once played).
/// This season's record while it runs, ahead of the story's own line.
fn so_far(c: &Ctx) -> String {
    match c.this_record() {
        Some((w, l, t, _)) if c.in_season() => {
            format!("{} so far in {}. ", wlt(w, l, t), c.this.as_ref().map_or(c.w.year, |p| p.year))
        }
        _ => String::new(),
    }
}

fn story(c: &Ctx) -> (String, String) {
    let s = &c.seasons;
    let Some(last) = s.last() else {
        return (format!("{} starts here.", c.name), "No seasons on record yet.".to_string());
    };
    let coach = c.coach_name(last.coach);
    let first = c.first_year();
    let yours = c.seasons[c.before..].len();
    let lead = if yours > 0 && last.coach == c.you {
        format!("{} in {}, your {} season here.", last.rec(), last.year, ord(yours))
    } else {
        format!("{} in {} under {coach}.", last.rec(), last.year)
    };
    let titles: Vec<i32> = s.iter().filter(|x| x.title).map(|x| x.year).collect();
    let streak = s.iter().rev().take_while(|x| x.title).count();
    let big = match c.lv {
        Lv::Hs => "state titles",
        Lv::College => "national titles",
        Lv::Nfl => "Super Bowls",
        Lv::Other => "league titles",
    };
    if streak >= 2 {
        return (
            format!("{} straight {big}.", word(streak)),
            format!("{lead} {} {big} since {first}: {}.", titles.len(), years_text(&titles)),
        );
    }
    if streak == 1 {
        let champs = match c.lv {
            Lv::Hs => "state champions",
            Lv::College => "national champions",
            Lv::Nfl => "Super Bowl champions",
            Lv::Other => "league champions",
        };
        return (format!("The defending {champs}."), lead);
    }
    if let Some(t) = s.iter().rev().find(|x| x.title)
        && last.year - t.year <= 5
    {
        let one = &big[..big.len() - 1];
        return (
            format!("{} years from a {one}.", word((last.year - t.year) as usize)),
            format!("{lead} The {one} came in {}, {}, under {}.", t.year, t.rec(), c.coach_name(t.coach)),
        );
    }
    // The longest drought that matters at the level.
    let since = |f: &dyn Fn(&Season) -> bool| s.iter().rev().take_while(|x| !f(x)).count();
    let mut droughts: Vec<(usize, String, String)> = Vec::new();
    let no_win = since(&|x| x.winning());
    if no_win >= 3 {
        let then = s.iter().rev().find(|x| x.winning()).map_or_else(
            || none_on_record(c),
            |x| format!("The last winning year was {} under {} in {}.", x.rec(), c.coach_name(x.coach), x.year),
        );
        droughts.push((no_win, format!("{} straight seasons without a winning record.", word(no_win)), then));
    }
    if matches!(c.lv, Lv::College) {
        let no_ten = since(&|x| x.w >= 10);
        if no_ten >= 8 {
            let then = s.iter().rev().find(|x| x.w >= 10).map_or_else(
                || format!("Not once since {first}."),
                |x| format!("The last ten-win year was {} under {} in {}.", x.rec(), c.coach_name(x.coach), x.year),
            );
            droughts.push((no_ten, format!("{no_ten} seasons without ten wins."), then));
        }
    }
    if matches!(c.lv, Lv::Nfl) {
        let out = since(&|x| x.round > 0);
        if out >= 4 {
            let then = s.iter().rev().find(|x| x.round > 0).map_or_else(
                || format!("Not once since {first}."),
                |x| format!("The last playoff year was {}, {}.", x.year, x.rec()),
            );
            droughts.push((out, format!("{out} seasons without the playoffs."), then));
        }
    }
    if let Some((_, head, then)) = droughts.into_iter().max_by_key(|d| d.0) {
        return (head, format!("{lead} {then}"));
    }
    let run = s.iter().rev().take_while(|x| x.winning()).count();
    if run >= 3 {
        return (format!("{} straight winning seasons.", word(run)), lead);
    }
    (format!("{} last season.", last.rec()), lead)
}

// ------------------------------------------------------------------ A: under the lights

/// The home stadium: its name, its place and its seats.
fn venue(w: &World, t: TeamRef) -> (String, String, u32) {
    match t {
        TeamRef::Nfl(i) => {
            let (name, seats) = sp::nfl_stadium(w, i, w.year);
            let (town, st) = afm_engine::world::finance::nfl::city(w, i, w.year);
            (name, format!("{town}, {st}"), seats)
        }
        TeamRef::HighSchool(h) => {
            let s = &w.high_schools[h];
            (
                crate::lane_schedule::school_field(w, h),
                format!("{}, {}", s.town, w.states[s.state].code),
                sp::hs_seats(w, h),
            )
        }
        TeamRef::College(p) => {
            let x = &w.programs[p];
            (
                crate::lane_schedule::college_stadium(w, p, w.year),
                format!("{}, {}", x.city, w.states[x.state].code),
                x.stadium,
            )
        }
        TeamRef::Other(i) => (String::new(), afm_engine::world::pro::other::city(i, w.year).to_string(), 0),
    }
}

/// The crowds at home in the latest season with home games played: the
/// average of the regular season's, its year, and the last home game.
struct Crowds {
    year: i32,
    avg: u32,
    games: usize,
    last: Option<(Date, u32, String)>,
}

fn crowds(c: &Ctx) -> Option<Crowds> {
    for p in c.pages() {
        let home: Vec<&PageGame> = p
            .games
            .iter()
            .filter(|g| g.home && !g.neutral && g.crowd > 0 && (g.score.is_some() || g.won.is_some()))
            .collect();
        let regular: Vec<u32> = home.iter().filter(|g| !g.playoff && !g.bowl).map(|g| g.crowd).collect();
        if regular.is_empty() {
            continue;
        }
        let last = home.iter().max_by_key(|g| g.date).map(|g| {
            let opp = match c.me {
                TeamRef::College(_) => views::team_name(c.w, TeamRef::College(g.opp)),
                TeamRef::Nfl(_) => views::team_name(c.w, TeamRef::Nfl(g.opp)),
                TeamRef::HighSchool(_) => views::team_name(c.w, TeamRef::HighSchool(g.opp)),
                TeamRef::Other(_) => String::new(),
            };
            (g.date, g.crowd, opp)
        });
        return Some(Crowds {
            year: p.year,
            avg: regular.iter().sum::<u32>() / regular.len() as u32,
            games: regular.len(),
            last,
        });
    }
    // Before any page holds a home game: the books' last season.
    match c.me {
        TeamRef::College(p) => {
            let d = c.w.finance.college.get(p)?;
            let s = d.seasons.last().filter(|s| s.home_games > 0)?;
            Some(Crowds {
                year: s.year,
                avg: s.attendance / s.home_games as u32,
                games: s.home_games as usize,
                last: None,
            })
        }
        TeamRef::Nfl(t) => {
            let d = c.w.finance.nfl.get(t)?;
            let s = d.seasons.last().filter(|s| s.home_games > 0)?;
            Some(Crowds {
                year: s.year,
                avg: s.attendance / s.home_games as u32,
                games: s.home_games as usize,
                last: None,
            })
        }
        TeamRef::HighSchool(h) => {
            let d = c.w.finance.hs.get(h)?;
            (d.crowd > 0).then(|| Crowds { year: c.w.year - 1, avg: d.crowd, games: 0, last: None })
        }
        TeamRef::Other(_) => None,
    }
}

/// The end zones' words: the nickname at the near end (the school's name
/// at high school, which has no mascots in the game), the place at the far.
fn end_zones(c: &Ctx) -> (String, String) {
    match c.me {
        TeamRef::College(p) => {
            let real = &c.w.programs[p].real_name;
            let nick = (!c.w.options.fictional_league).then(|| numbers::mascot_of(real)).flatten();
            (nick.unwrap_or_else(|| c.name.clone()).to_uppercase(), c.name.to_uppercase())
        }
        TeamRef::Nfl(_) | TeamRef::Other(_) => {
            let words: Vec<&str> = c.name.split_whitespace().collect();
            let nick = words.last().copied().unwrap_or("");
            let city = if words.len() > 1 { words[..words.len() - 1].join(" ") } else { c.name.clone() };
            (nick.to_uppercase(), city.to_uppercase())
        }
        TeamRef::HighSchool(_) => (c.name.to_uppercase(), c.name.to_uppercase()),
    }
}

/// The crest's letters: the city's first three at the NFL, else initials.
fn crest(c: &Ctx) -> String {
    let words: Vec<&str> =
        c.name.split_whitespace().filter(|x| x.chars().next().is_some_and(char::is_alphabetic)).collect();
    // NFL clubs go by their city: DAL, KC, NYG (two clubs share New York and
    // Los Angeles, so those add the nickname).
    if matches!(c.lv, Lv::Nfl) && words.len() > 1 {
        let city = &words[..words.len() - 1];
        if city.len() == 1 {
            return city[0].chars().take(3).collect::<String>().to_uppercase();
        }
        let mut s: String = city.iter().filter_map(|x| x.chars().next()).collect::<String>().to_uppercase();
        if s == "NY" || s == "LA" {
            s.extend(words[words.len() - 1].chars().next().map(|x| x.to_ascii_uppercase()));
        }
        return s;
    }
    let s: String = words.iter().filter_map(|x| x.chars().next()).take(3).collect();
    if s.chars().count() == 1 { words[0].chars().take(3).collect::<String>().to_uppercase() } else { s.to_uppercase() }
}

/// A captain's line: his last season's numbers, his years, his town.
fn captain_line(c: &Ctx, k: usize) -> String {
    let w = c.w;
    let p = &w.people[k];
    let id = p.player.id;
    let nick = w.nickname_of(id).map(|n| format!("\"{n}\" · "));
    let line = match p.stage {
        Stage::Pro => p
            .pro
            .as_ref()
            .and_then(|x| x.seasons.iter().rev().find(|s| s.games > 0))
            .map(|s| format!("{} starts in {} games, {}", s.starts, s.games, s.year)),
        Stage::College | Stage::Declared => w.stats.career(id).last().map(|r| {
            let numbers = crate::lane_core::mail::stat_line(r.pos, |s| r.get(s));
            // Linemen, and anyone whose line would be all zeros, by games.
            if matches!(r.pos, Position::OT | Position::IOL | Position::LS)
                || numbers.starts_with("0 ")
                || numbers.is_empty()
            {
                format!("{} games in {}", r.games, r.year)
            } else {
                format!("{numbers} in {}", r.year)
            }
        }),
        _ => p.hs_lines.last().map(|l| format!("{} games in {}", l.games, l.year)),
    };
    let line = line.unwrap_or_else(|| format!("From {}", views::hometown(w, p)));
    format!("{}{line}", nick.unwrap_or_default())
}

fn captains(c: &Ctx) -> Vec<usize> {
    let Some(team) = Team::seat(c.w) else { return Vec::new() };
    let mut v = takeover::captains_of(c.w, team);
    v.retain(|&k| c.w.people.get(k).is_some_and(|p| p.stage != Stage::Done));
    v.truncate(6);
    v
}

fn person_brief(c: &Ctx, k: usize) -> Value {
    let p = &c.w.people[k];
    json!({
        "id": p.player.id,
        "name": p.name,
        "last": last_name(&p.name),
        "pos": p.player.position.name(),
        "cls": views::class_text(c.w, p),
    })
}

/// The next dated moments that are yours or your level's, seven of them.
fn road(c: &Ctx) -> Vec<Value> {
    let cal = views::calendar(c.w);
    cal["items"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter(|x| x["mine"].as_bool() == Some(true) || x["loud"].as_str() == Some("date"))
                .take(7)
                .map(|x| {
                    let n = x["in"].as_i64().unwrap_or(0);
                    json!({
                        "date": if n == 0 {
                            "Today".to_string()
                        } else {
                            let d = x["date"].as_str().unwrap_or("");
                            d.strip_suffix(&format!(" {}", c.w.today.year())).unwrap_or(d).to_string()
                        },
                        "text": crate::lane_core::links::strip(x["text"].as_str().unwrap_or("")),
                        "in": if n == 0 { "now".to_string() } else { format!("{n} d") },
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

fn expectation(c: &Ctx) -> Option<String> {
    let w = c.w;
    Some(match c.me {
        TeamRef::College(p) => crate::setup::college_expectation(w.programs[p].prestige_rel).to_string(),
        TeamRef::HighSchool(h) => {
            let s = &w.high_schools[h];
            crate::setup::hs_expectation(s.prestige, s.state_titles).to_string()
        }
        TeamRef::Nfl(t) => {
            let last = c.prev().last();
            crate::setup::nfl_expectation(
                last.map_or(8, |s| s.w),
                last.map_or(0, |s| s.round),
                w.clubs.get(t).map_or(0.5, |x| x.office.owner.patience),
            )
        }
        TeamRef::Other(_) => return None,
    })
}

/// The rival line under what's asked of you: the series at college and high
/// school, last season's division at the NFL, in order.
fn rival(c: &Ctx) -> Option<Value> {
    let w = c.w;
    match c.me {
        TeamRef::College(p) => {
            let r = traditions::college_rivalries(w, p).into_iter().next()?;
            let them = views::team_name(w, TeamRef::College(r.rival));
            let (a, b, t) = (r.wins[0], r.wins[1], r.ties);
            let lead = if a > b {
                format!("{} leads {}", c.name, wlt(a, b, t))
            } else if b > a {
                format!("{them} leads {}", wlt(b, a, t))
            } else {
                format!("all square at {}", wlt(a, b, t))
            };
            let since = r.since.map(|y| format!(" since {y}")).unwrap_or_default();
            let holds = match (&r.trophy, r.holder) {
                (Some(tr), Some(h)) => {
                    format!(
                        " {} holds {tr}.",
                        if h == p { c.name.clone() } else { views::team_name(w, TeamRef::College(h)) }
                    )
                }
                _ => String::new(),
            };
            Some(json!({
                "colors": views::team_colors(w, TeamRef::College(r.rival)),
                "text": format!("{}: {lead}{since}.{holds}", r.game),
                "ref": TeamRef::College(r.rival).json(),
            }))
        }
        TeamRef::HighSchool(h) => {
            let r = w.stories.traditions.hs_rivals.iter().find(|r| r.schools.contains(&(h as u32)))?;
            let k = usize::from(r.schools[1] == h as u32);
            let rv = r.schools[1 - k] as usize;
            let them = views::team_name(w, TeamRef::HighSchool(rv));
            let (a, b) = (r.wins[k] as u32, r.wins[1 - k] as u32);
            let lead = if a > b {
                format!("{} leads {a}-{b}", c.name)
            } else if b > a {
                format!("{them} leads {b}-{a}")
            } else {
                format!("all square at {a}-{b}")
            };
            Some(json!({
                "colors": views::team_colors(w, TeamRef::HighSchool(rv)),
                "text": format!("The {them} game: {lead} since {}.", r.since),
                "ref": TeamRef::HighSchool(rv).json(),
            }))
        }
        TeamRef::Nfl(t) => {
            let div = views::nfl_division(w, t)?;
            let year = w.nfl_seasons.iter().filter(|s| s.team == t).map(|s| s.year).max()?;
            let mut rows: Vec<(f64, u32, String, usize)> = w
                .nfl_seasons
                .iter()
                .filter(|s| s.year == year && views::nfl_division(w, s.team) == Some(div))
                .map(|s| {
                    let g = (s.wins + s.losses + s.ties).max(1) as f64;
                    (
                        (s.wins as f64 + 0.5 * s.ties as f64) / g,
                        s.wins,
                        format!("{} {}", w.nfl_team_name(s.team), wlt(s.wins, s.losses, s.ties)),
                        s.team,
                    )
                })
                .collect();
            rows.sort_by(|a, b| b.0.total_cmp(&a.0).then(b.1.cmp(&a.1)));
            let top = rows.iter().find(|r| r.3 != t).map_or(t, |r| r.3);
            Some(json!({
                "colors": views::team_colors(w, TeamRef::Nfl(top)),
                "text": format!("{div} {year}: {}.", rows.iter().map(|r| r.2.as_str()).collect::<Vec<_>>().join(", ")),
                "ref": TeamRef::Nfl(top).json(),
            }))
        }
        TeamRef::Other(_) => None,
    }
}

fn stadium(c: &Ctx) -> Value {
    let w = c.w;
    let (stadium, place, seats) = venue(w, c.me);
    let crowd = crowds(c);
    let fill = match (&crowd, seats) {
        (Some(x), s) if s > 0 => (x.avg as f64 / s as f64).min(1.0),
        (Some(_), _) => 0.75,
        _ => 0.0,
    };
    let (ez_near, ez_far) = end_zones(c);
    let (story_head, story_sub) = story(c);
    let level_line = views::team_line(w, c.me);

    // The next game and the last result on this season's page.
    let next = c.this.as_ref().and_then(|p| {
        p.games.iter().find(|g| g.score.is_none() && g.won.is_none() && !g.cancelled && g.date >= w.today)
    });
    let last_result = c.this.as_ref().and_then(|p| p.games.iter().rev().find(|g| g.score.is_some()));
    let opp_name = |g: &PageGame| match c.me {
        TeamRef::College(_) => views::team_name(w, TeamRef::College(g.opp)),
        TeamRef::Nfl(_) => views::team_name(w, TeamRef::Nfl(g.opp)),
        TeamRef::HighSchool(_) => views::team_name(w, TeamRef::HighSchool(g.opp)),
        TeamRef::Other(_) => String::new(),
    };
    let at = |g: &PageGame| if g.home || g.neutral { "vs" } else { "at" };
    let near = |g: &PageGame| w.today.days_to(g.date).abs() <= 14;
    let result_text = |g: &PageGame| {
        g.score.map(|[a, b]| {
            let wl = if a > b {
                "Won"
            } else if a < b {
                "Lost"
            } else {
                "Tied"
            };
            format!("{wl} {a}-{b} {} {}", at(g), opp_name(g))
        })
    };

    // The headline: welcome on day one, the games when they're close, the
    // program's story otherwise.
    let (kicker, headline, sub) = if c.day_one {
        (format!("Your first day · {} · {level_line}", c.name), story_head, story_sub)
    } else if let Some(g) = next.filter(|g| near(g)) {
        let days = w.today.days_to(g.date);
        let when = match days {
            0 => "Today".to_string(),
            1 => "Tomorrow".to_string(),
            n => format!("In {n} days, {}", views::date_text(g.date)),
        };
        let named = g.name.clone().map(|n| format!(" · {n}")).unwrap_or_default();
        let last = last_result
            .filter(|g| near(g))
            .and_then(result_text)
            .map(|t| format!(" Last time out: {t}."))
            .unwrap_or_default();
        let record = c.this_record().map(|(a, b, t, _)| format!(" {} so far.", wlt(a, b, t))).unwrap_or_default();
        (
            format!("{} · {} · {level_line}", if days == 0 { "Game day" } else { "Game week" }, c.name),
            format!("{} {}.", if at(g) == "at" { "At" } else { "Vs" }, opp_name(g)),
            format!("{when}{named}.{record}{last}"),
        )
    } else if let Some(g) = last_result.filter(|g| near(g)) {
        let record = c.this_record().map(|(a, b, t, _)| format!("{} this season.", wlt(a, b, t))).unwrap_or_default();
        (
            format!("The last result · {} · {level_line}", c.name),
            format!("{}.", result_text(g).unwrap_or_default()),
            format!("{} {record}", views::date_text(g.date)),
        )
    } else {
        let phase = if c.in_season() { "This season" } else { "Between seasons" };
        (format!("{phase} · {} · {level_line}", c.name), story_head, format!("{}{story_sub}", so_far(c)))
    };

    // The board at the far end (high school and college): welcome on day
    // one, then the last final and the next game.
    let board = if matches!(c.lv, Lv::Nfl | Lv::Other) {
        Value::Null
    } else if c.day_one {
        json!([
            if stadium.is_empty() { c.name.to_uppercase() } else { stadium.to_uppercase() },
            format!("WELCOME, COACH {}", c.surname.to_uppercase())
        ])
    } else if let Some(g) = last_result.filter(|g| near(g)) {
        let [a, b] = g.score.unwrap_or([0, 0]);
        json!(["FINAL", format!("{} {a} · {} {b}", c.name.to_uppercase(), opp_name(g).to_uppercase())])
    } else if let Some(g) = next {
        json!(["NEXT", format!("{} · {}", opp_name(g).to_uppercase(), views::date_text(g.date).to_uppercase())])
    } else {
        let rec = match (c.in_season(), c.this_record(), c.seasons.last()) {
            (true, Some((a, b, t, _)), _) => {
                format!("{} · {}", wlt(a, b, t), c.this.as_ref().map_or(w.year, |p| p.year))
            }
            (_, _, Some(s)) => format!("{} · {}", s.rec(), s.year),
            _ => String::new(),
        };
        json!([c.name.to_uppercase(), rec])
    };

    // The lower third: record, seats, the crowd and the ticket line.
    let page = c.pages().into_iter().find(|p| p.games.iter().any(|g| g.score.is_some() || g.won.is_some()));
    let rec_label = match (page, c.seasons.last()) {
        (Some(p), _) if c.in_season() => format!("this season · {}", p.year),
        (_, Some(s)) if s.title => format!(
            "{} · {}",
            match c.lv {
                Lv::Hs => "state champions",
                Lv::College => "national champions",
                Lv::Nfl => "Super Bowl champions",
                Lv::Other => "league champions",
            },
            s.year
        ),
        (Some(p), Some(s)) if p.year == s.year && p.place.is_some() => {
            let (k, _) = p.place.unwrap_or((0, 0));
            format!("{}, {} · {}", ord(k as usize), p.league_name, p.year)
        }
        (_, Some(s)) => format!("in {}", s.year),
        _ => String::new(),
    };
    let record = match (c.in_season(), c.this_record(), c.seasons.last()) {
        (true, Some((a, b, t, _)), _) => wlt(a, b, t),
        (_, _, Some(s)) => s.rec(),
        _ => views::record_text(w, c.me),
    };
    let (line2, line3) = match c.me {
        TeamRef::College(p) => {
            let d = w.finance.college.get(p);
            (
                d.map(|d| {
                    format!("{} season tickets · {}% renewed", thousands(d.season_tickets), (d.renewal * 100.0).round())
                })
                .unwrap_or_default(),
                if fill >= 0.98 {
                    "a full house every week".to_string()
                } else {
                    "the empty seats above are real".to_string()
                },
            )
        }
        TeamRef::Nfl(t) => {
            let d = w.finance.nfl.get(t);
            let gates: Vec<_> =
                d.map(|d| d.gates.iter().filter(|g| !g.preseason && !g.playoff).collect()).unwrap_or_default();
            let sold = gates.iter().filter(|g| g.sold_out).count();
            (
                if gates.is_empty() {
                    format!("{} on the season-ticket waitlist", thousands(d.map_or(0, |d| d.waitlist)))
                } else {
                    format!(
                        "{sold} of {} sold out · {} on the waitlist",
                        gates.len(),
                        thousands(d.map_or(0, |d| d.waitlist))
                    )
                },
                if !gates.is_empty() && sold == gates.len() {
                    "a full house every week".to_string()
                } else {
                    "the empty seats above are real".to_string()
                },
            )
        }
        TeamRef::HighSchool(h) => {
            let b = &w.front_office.school.boosters;
            let line = if w.front_office.school.school == Some(h) && b.members > 0 {
                format!("{} in the booster club", thousands(b.members))
            } else {
                format!("{} students", thousands(w.high_schools[h].enrollment))
            };
            (line, "the empty seats above are real".to_string())
        }
        TeamRef::Other(_) => (String::new(), String::new()),
    };

    // What's asked of you, by whom.
    let (boss_name, boss_line) = match Team::seat(w) {
        Some(team) => {
            let b = boss::boss(w, team);
            let rep: Vec<String> = b.reputation().iter().take(2).map(|x| x.to_lowercase()).collect();
            let trust = pronouns(boss::trust_words(boss::trust(w, team)), b.she);
            let years = if b.since > 0 { format!(" since {}", b.since) } else { String::new() };
            let rep = if rep.is_empty() { String::new() } else { format!(": {}", rep.join(", ")) };
            (b.name.clone(), format!("{}{years}{rep}. {trust}", b.title.to_lowercase()))
        }
        None => (String::new(), String::new()),
    };

    let caps: Vec<Value> = captains(c)
        .into_iter()
        .map(|k| {
            let mut v = person_brief(c, k);
            v["line"] = json!(captain_line(c, k));
            v
        })
        .collect();

    json!({
        "kind": "stadium",
        "kicker": kicker,
        "title": headline,
        "sub": sub,
        "colors": c.colors,
        "go": { "screen": "team", "args": c.me.json(), "label": "Team page" },
        "hero": {
            "level": c.lv.key(),
            "crest": crest(c),
            "ez": [ez_near, ez_far],
            "board": board,
            "fill": fill,
            "captains": caps,
            "road": road(c),
            "team": {
                "name": c.name,
                "stadium": stadium,
                "place": place,
                "record": record,
                "rec_label": rec_label,
                "seats": seats,
                "avg": crowd.as_ref().map(|x| x.avg),
                "fill": (fill * 100.0).round(),
                "avg_label": crowd.as_ref().map(|x| {
                    if x.games > 0 { format!("a home game in {}", x.year) } else { "a normal home crowd".to_string() }
                }),
                "last_home": crowd.as_ref().and_then(|x| x.last.as_ref()).filter(|_| !c.day_one).map(|(d, n, opp)| {
                    format!("Last home game: {} vs {opp}, {}", thousands(*n), views::date_text(*d))
                }),
                "line2": line2,
                // Without a crowd on record the grey seats are unknown, not empty.
                "line3": if crowd.is_some() { line3 } else { String::new() },
            },
            "asked": {
                "wants": expectation(c),
                "boss": boss_name,
                "line": boss_line,
                "rival": rival(c).map(|mut r| {
                    let t = cap_first(r["text"].as_str().unwrap_or(""));
                    r["text"] = json!(t);
                    r
                }),
            },
        },
    })
}

/// A line written about a man, for a woman when she is one (whole words).
fn pronouns(s: &str, she: bool) -> String {
    if !she {
        return s.to_string();
    }
    s.split(' ')
        .map(|x| {
            let (word, rest) = x.split_at(x.find(|ch: char| !ch.is_alphabetic() && ch != '\'').unwrap_or(x.len()));
            let to = match word {
                "he" => "she",
                "He" => "She",
                "he's" => "she's",
                "He's" => "She's",
                "his" | "him" => "her",
                "His" => "Her",
                "himself" => "herself",
                _ => word,
            };
            format!("{to}{rest}")
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// "None in the 30 seasons on record." (a missing mark, said plainly).
fn none_on_record(c: &Ctx) -> String {
    match c.seasons.len() {
        0 => "None on record yet.".to_string(),
        n => format!("None in the {n} seasons on record."),
    }
}

fn cap_first(s: &str) -> String {
    let mut ch = s.chars();
    ch.next().map_or_else(String::new, |f| f.to_uppercase().collect::<String>() + ch.as_str())
}

// ------------------------------------------------------------------ B: the program's story

/// One mark you could be the first to reach: its badge, words, line and
/// where it stands this season (open, done, missed).
struct Mark {
    n: String,
    t: String,
    l: String,
    state: &'static str,
    /// Wins it takes, for the chart's dashed line.
    wins: Option<u32>,
}

/// Wins a season takes at most (the regular season's games).
fn season_games(c: &Ctx) -> u32 {
    if let Some((w, l, t, left)) = c.this_record() {
        let n = w + l + t + left as u32;
        if n > 0 {
            return n;
        }
    }
    c.prev().last().map_or(12, |s| s.w + s.l + s.t)
}

/// A season-mark's state: done by one of your seasons or this one, missed
/// when this season can't reach it any more.
fn mark_state(c: &Ctx, wins: Option<u32>, f: &dyn Fn(&Season) -> bool) -> (&'static str, Option<String>) {
    if let Some(s) = c.seasons[c.before..].iter().rev().find(|s| f(s)) {
        return ("done", Some(format!("Done in {}: {}.", s.year, s.rec())));
    }
    if c.in_season()
        && let Some(wins) = wins
        && let Some((w, l, t, left)) = c.this_record()
    {
        if w >= wins {
            return ("done", Some(format!("Done this season: {} so far.", wlt(w, l, t))));
        }
        if w + left as u32 + t / 2 < wins {
            return ("missed", Some(format!("Out of reach this season at {}. Next year.", wlt(w, l, t))));
        }
    }
    ("open", None)
}

fn marks(c: &Ctx) -> Vec<Mark> {
    let w = c.w;
    let prev = c.prev();
    let first = c.first_year();
    let games = season_games(c);
    let mut out: Vec<Mark> = Vec::new();
    let mut push = |n: String, t: &str, l: String, state: (&'static str, Option<String>), wins: Option<u32>| {
        let l = match state.1 {
            Some(s) => format!("{s} {}", cap_first(&l)),
            None => cap_first(&l),
        };
        out.push(Mark { n, t: t.to_string(), l, state: state.0, wins });
    };
    let last_when = |f: &dyn Fn(&Season) -> bool| prev.iter().rev().find(|s| f(s));
    let since_txt = |s: Option<&Season>, what: &str| match s {
        Some(s) => format!("{what} {} ({}, {}).", s.year, s.rec(), c.coach_name(s.coach)),
        None => format!("Not once since {first}."),
    };

    // A winning season, when the last one wasn't.
    let win_need = games / 2 + 1;
    if prev.last().is_some_and(|s| !s.winning()) {
        let s = last_when(&|s| s.winning());
        push(
            format!("{win_need}+"),
            "A winning season",
            since_txt(s, "The first since"),
            mark_state(c, Some(win_need), &|s| s.winning()),
            Some(win_need),
        );
    }
    match c.lv {
        Lv::College => {
            if let Some(r) = rival_mark(c) {
                push(r.n, &r.t, r.l, (r.state, None), r.wins);
            }
            let ten = last_when(&|s| s.w >= 10);
            if prev.last().is_some_and(|s| s.w < 10) {
                let best_since = ten.map(|t| prev.iter().filter(|s| s.year > t.year).map(|s| s.w).max().unwrap_or(0));
                let extra =
                    best_since.filter(|&b| b > 0).map(|b| format!(" Nobody since has topped {b}.")).unwrap_or_default();
                push(
                    "10".into(),
                    "Ten wins",
                    format!("{}{extra}", since_txt(ten, "Not done since")),
                    mark_state(c, Some(10), &|s| s.w >= 10),
                    Some(10),
                );
            }
            let conf: Vec<&Season> = prev.iter().filter(|s| s.conf).collect();
            let conf_name = match c.me {
                TeamRef::College(p) => w.conference_name(&w.programs[p].conference),
                _ => String::new(),
            };
            let coaches = {
                let mut v: Vec<usize> = prev.iter().filter_map(|s| s.coach).collect();
                v.sort_unstable();
                v.dedup();
                v.len()
            };
            let (st, _) = mark_state(c, None, &|s| s.conf);
            push(
                if conf_name.len() <= 4 { conf_name.clone() } else { "CONF".into() },
                "A conference title",
                match conf.last() {
                    Some(s) => format!("The last came in {} under {}.", s.year, c.coach_name(s.coach)),
                    None => format!("None in {} years. {} coaches tried.", prev.len(), word(coaches)),
                },
                (st, None),
                None,
            );
            if let Some((name, pos, pick, year, team)) = last_first_rounder(c) {
                if w.year - year > 1 {
                    push(
                        "R1".into(),
                        "A first-round pick",
                        format!("None since {name}, {pos}, No. {pick} in {year} ({team})."),
                        ("open", None),
                        None,
                    );
                }
            } else {
                push("R1".into(), "A first-round pick", "None on record.".into(), ("open", None), None);
            }
            if prev.iter().all(|s| !s.title) {
                push("No. 1".into(), "A national title", none_on_record(c), ("open", None), None);
            }
        }
        Lv::Hs => {
            let titles: Vec<i32> = prev.iter().filter(|s| s.title).map(|s| s.year).collect();
            let streak = prev.iter().rev().take_while(|s| s.title).count();
            // The longest title run on record.
            let mut best = (0usize, 0usize);
            let mut run = 0;
            for (k, s) in prev.iter().enumerate() {
                run = if s.title { run + 1 } else { 0 };
                if run > best.0 {
                    best = (run, k);
                }
            }
            let (st, note) = mark_state(c, None, &|s| s.title);
            if streak >= 1 {
                let done_before = if best.0 > streak {
                    let s0 = &prev[best.1 + 1 - best.0];
                    format!(
                        "Done {} here: {}-{:02} under {}.",
                        if best.0 > streak + 1 { "before" } else { "once" },
                        s0.year,
                        prev[best.1].year % 100,
                        c.coach_name(s0.coach)
                    )
                } else {
                    "Never done here.".to_string()
                };
                push(
                    (streak + 1).to_string(),
                    &format!("A {} straight state title", ord(streak + 1)),
                    done_before,
                    (st, note),
                    None,
                );
            } else {
                push(
                    "State".into(),
                    "A state title",
                    match titles.last() {
                        Some(y) => format!("The first since {y}."),
                        None => none_on_record(c),
                    },
                    (st, note),
                    None,
                );
            }
            let unbeaten: Vec<&Season> = prev.iter().filter(|s| s.l == 0 && s.w > 0).collect();
            let best_rec = prev.iter().max_by_key(|s| (s.w, std::cmp::Reverse(s.l)));
            push(
                format!("{}-0", best_rec.map_or(games, |s| s.w.max(games))),
                "An unbeaten season",
                match unbeaten.as_slice() {
                    [] => format!("Not once since {first}."),
                    [s] => format!("Only the {} team went {}.", s.year, s.rec()),
                    v => format!(
                        "{} teams did it: {}.",
                        word(v.len()),
                        years_text(&v.iter().map(|s| s.year).collect::<Vec<_>>())
                    ),
                },
                mark_state(c, Some(games), &|s| s.l == 0 && s.w > 0),
                Some(best_rec.map_or(games, |s| s.w.max(games))),
            );
            if !titles.is_empty() {
                push(
                    (titles.len() + 1).to_string(),
                    &format!("A {} state title since {first}", ord(titles.len() + 1)),
                    format!("{} so far: {}.", word(titles.len()), years_text(&titles)),
                    ("open", None),
                    None,
                );
            }
            // The most wins by one coach on record.
            let mut by: Vec<(usize, u32, u32, usize)> = Vec::new();
            for s in prev {
                let Some(k) = s.coach else { continue };
                match by.iter_mut().find(|x| x.0 == k) {
                    Some(x) => {
                        x.1 += s.w;
                        x.2 += s.l;
                        x.3 += 1;
                    }
                    None => by.push((k, s.w, s.l, 1)),
                }
            }
            if let Some(top) = by.iter().max_by_key(|x| x.1)
                && top.1 > 0
            {
                let name = c.coach_name(Some(top.0));
                push(
                    top.1.to_string(),
                    &format!("{name}'s wins"),
                    format!("The school mark since {first}, over {} years ({}-{}).", top.3, top.1, top.2),
                    ("open", None),
                    None,
                );
            }
            if let TeamRef::HighSchool(h) = c.me {
                let up = leaving(w, h);
                if !up.is_empty() {
                    push(
                        "D-I".into(),
                        "Send them up",
                        format!(
                            "{} of this year's seniors are headed to college: {}.",
                            word(up.len()),
                            schools_text(w, &up)
                        ),
                        ("open", None),
                        None,
                    );
                }
            }
        }
        Lv::Nfl => {
            if prev.last().is_some_and(|s| s.round == 0) {
                let made = last_when(&|s| s.round > 0);
                push(
                    "NFC".into(),
                    "Back to the playoffs",
                    format!(
                        "Missed in {} at {}. {}",
                        prev.last().map_or(0, |s| s.year),
                        prev.last().map_or_else(String::new, Season::rec),
                        made.map_or_else(|| none_on_record(c), |s| format!("Last made in {}.", s.year))
                    ),
                    mark_state(c, None, &|s| s.round > 0),
                    None,
                );
            }
            let titles: Vec<&Season> = prev.iter().filter(|s| s.title).collect();
            push(
                "SB".into(),
                &match titles.len() {
                    0 => "A Super Bowl".to_string(),
                    1 => "A second Super Bowl".to_string(),
                    n => format!("A {} Super Bowl", ord(n + 1)),
                },
                match titles.last() {
                    Some(s) => format!(
                        "The {} came in {}, {}, under {}.",
                        if titles.len() == 1 { "first" } else { "last" },
                        s.year,
                        s.rec(),
                        c.coach_name(s.coach)
                    ),
                    None => none_on_record(c),
                },
                mark_state(c, None, &|s| s.title),
                None,
            );
            let best = prev.iter().map(|s| s.w).max().unwrap_or(0);
            let tops: Vec<&Season> = prev.iter().filter(|s| s.w == best).collect();
            if !prev.is_empty() {
                push(
                    (best + 1).to_string(),
                    &format!("A {}-win season", best + 1),
                    format!(
                        "Never done here. The best: {}.",
                        tops.iter()
                            .map(|s| format!("{} in {} ({})", s.rec(), s.year, c.coach_name(s.coach)))
                            .collect::<Vec<_>>()
                            .join(" and ")
                    ),
                    mark_state(c, Some(best + 1), &|s| s.w > best),
                    Some(best + 1),
                );
            }
            let draft = w.dates.draft[0].date;
            if draft >= w.today {
                push(
                    "R1".into(),
                    "Your first draft class",
                    format!("Round 1 is {}. Your classes are graded from that night.", views::date_text(draft)),
                    ("open", None),
                    None,
                );
            }
        }
        Lv::Other => {
            let titles: Vec<&Season> = prev.iter().filter(|s| s.title).collect();
            push(
                "Title".into(),
                "A league title",
                match titles.last() {
                    Some(s) => format!("The last came in {}.", s.year),
                    None => none_on_record(c),
                },
                mark_state(c, None, &|s| s.title),
                None,
            );
        }
    }
    out.truncate(5);
    out
}

/// The rivalry as a mark at college: beat them, or keep the trophy.
fn rival_mark(c: &Ctx) -> Option<Mark> {
    let TeamRef::College(p) = c.me else { return None };
    let r = traditions::college_rivalries(c.w, p).into_iter().next()?;
    let them = views::team_name(c.w, TeamRef::College(r.rival));
    let series = wlt(r.wins[0], r.wins[1], r.ties);
    let holds = r.holder == Some(p);
    let l = match (&r.trophy, holds) {
        (Some(t), false) => format!("{} is {series}. Win it and {t} comes home.", r.game),
        (Some(t), true) => format!("{} is {series}, and {t} is here. Keep it.", r.game),
        (None, _) => format!("{} is {series}.", r.game),
    };
    Some(Mark { n: "W".into(), t: format!("Beat {them}"), l, state: "open", wins: None })
}

/// The program's last first-round pick: name, position, pick, year, club.
fn drafted(w: &World, me: TeamRef) -> Vec<(i32, u32, u32, usize, usize)> {
    if !matches!(me, TeamRef::College(_) | TeamRef::Nfl(_)) {
        return Vec::new();
    }
    w.people
        .iter()
        .enumerate()
        .filter_map(|(k, x)| {
            let d = x.pro.as_ref()?.draft?;
            let mine = match me {
                TeamRef::College(p) => x.college == Some(p),
                _ => me == TeamRef::Nfl(d.3),
            };
            mine.then_some((d.0, d.1, d.2, d.3, k))
        })
        .collect()
}

fn last_first_rounder(c: &Ctx) -> Option<(String, &'static str, u32, i32, String)> {
    let w = c.w;
    c.drafted
        .iter()
        .filter(|d| d.1 == 1)
        .max_by_key(|d| (d.0, std::cmp::Reverse(d.2)))
        .map(|d| (w.people[d.4].name.clone(), w.people[d.4].player.position.name(), d.2, d.0, w.nfl_team_name(d.3)))
}

/// Fills a thin wall with the highest picks the draft ever took from here.
fn fill_with_picks(c: &Ctx, v: &mut Vec<(f64, u32, String, String, &'static str)>) {
    let mut picks: Vec<&(i32, u32, u32, usize, usize)> = c.drafted.iter().collect();
    picks.sort_by_key(|d| (d.1, d.2, std::cmp::Reverse(d.0)));
    for d in picks {
        if v.len() >= 4 {
            break;
        }
        let p = &c.w.people[d.4];
        if v.iter().any(|x| x.1 == p.player.id) {
            continue;
        }
        // The pick counts within its round.
        let slot = if d.1 == 1 { format!("No. {} pick", d.2) } else { format!("Round {}, pick {}", d.1, d.2) };
        let what = match c.me {
            TeamRef::Nfl(_) => format!("Drafted here: {slot}, {}", d.0),
            _ => format!("{slot}, {} ({})", d.0, c.w.nfl_team_name(d.3)),
        };
        v.push((0.0, p.player.id, p.name.clone(), what, p.player.position.name()));
    }
}

/// The colleges your seniors go to, most first: "Montana State (3), Montana".
fn schools_text(w: &World, v: &[(usize, usize, bool)]) -> String {
    let mut n: Vec<(usize, usize)> = Vec::new();
    for x in v {
        match n.iter_mut().find(|y| y.0 == x.1) {
            Some(y) => y.1 += 1,
            None => n.push((x.1, 1)),
        }
    }
    n.sort_by_key(|y| std::cmp::Reverse(y.1));
    let mut parts: Vec<String> = n
        .iter()
        .take(4)
        .map(|&(p, k)| {
            let name = views::team_name(w, TeamRef::College(p));
            if k > 1 { format!("{name} ({k})") } else { name }
        })
        .collect();
    if n.len() > 4 {
        parts.push(format!("{} more", n.len() - 4));
    }
    parts.join(", ")
}

/// Your seniors signed with or committed to a college, leaving in June:
/// (person, program, signed).
fn leaving(w: &World, h: usize) -> Vec<(usize, usize, bool)> {
    let mut v: Vec<(usize, usize, bool, u8)> = w.high_schools[h]
        .roster
        .iter()
        .filter_map(|&k| {
            let p = w.people.get(k)?;
            let signed = p.cal.signed.filter(|s| !s.walk_on).map(|s| s.program);
            let prog = signed.or(p.cal.commit.map(|c| c.0))?;
            (p.stage == Stage::HighSchool && views::class_text(w, p) == "Sr").then_some((
                k,
                prog,
                signed.is_some(),
                p.stars,
            ))
        })
        .collect();
    v.sort_by_key(|x| std::cmp::Reverse(x.3));
    v.into_iter().map(|x| (x.0, x.1, x.2)).collect()
}

/// The names on the wall: the program's honored players, or at high
/// school the signed seniors leaving in June.
fn wall(c: &Ctx) -> (String, String, Vec<Value>, String) {
    let w = c.w;
    match c.me {
        TeamRef::HighSchool(h) => {
            let v: Vec<Value> = leaving(w, h)
                .into_iter()
                .take(4)
                .map(|(k, prog, signed)| {
                    let p = &w.people[k];
                    json!({
                        "id": p.player.id, "name": p.name, "pos": p.player.position.name(), "face": true,
                        "what": format!("{} {}", if signed { "Signed with" } else { "Committed to" }, views::team_name(w, TeamRef::College(prog))),
                        "more": format!("{} offers · class of {}", views::offers_of(p).len(), p.class_of),
                    })
                })
                .collect();
            let all = leaving(w, h);
            let signed = all.iter().filter(|x| x.2).count();
            let note = match (signed, all.len() - signed) {
                (0, 0) => "No senior has signed or committed yet.".to_string(),
                (s, 0) => {
                    format!("{} senior{} signed with colleges this year.", word(s), if s == 1 { "" } else { "s" })
                }
                (0, m) => format!("{} senior{} committed, none signed yet.", word(m), if m == 1 { "" } else { "s" }),
                (s, m) => {
                    format!("{} signed with colleges this year, {} more committed.", word(s), word(m).to_lowercase())
                }
            };
            ("Leaving in June".into(), format!("the class of {}", w.year), v, note)
        }
        TeamRef::College(p) => {
            // Honored players who played here, strongest first.
            let mut v: Vec<(f64, u32, String, String, &'static str)> = Vec::new();
            for (&id, hn) in &w.stats.honors {
                let here = w.stats.career(id).iter().any(|r| r.program as usize == p);
                if !here {
                    continue;
                }
                let hall = w.stats.hall.iter().find(|x| x.who == Some(id) && x.programs.contains(&p));
                let pro: Vec<&(i32, &str, u32, String, usize)> =
                    w.nfl_book.awards.iter().filter(|a| a.2 == id).collect();
                let score = hall.map_or(0.0, |_| 100.0)
                    + 30.0 * hn.awards.len() as f64
                    + 12.0 * hn.all_america.len() as f64
                    + 2.0 * hn.all_conference as f64
                    + 20.0 * pro.len() as f64;
                if score < 10.0 {
                    continue;
                }
                let what = if let Some((y, a)) = hn.awards.first() {
                    format!("{a}, {y}")
                } else if !hn.all_america.is_empty() {
                    format!("All-American {}", years_text(&hn.all_america.iter().map(|x| x.0).collect::<Vec<_>>()))
                } else {
                    format!(
                        "All-conference {} time{}",
                        hn.all_conference,
                        if hn.all_conference == 1 { "" } else { "s" }
                    )
                };
                let more = match (hall, pro.first()) {
                    (Some(h), _) => format!("Hall of Fame {}", h.year),
                    (None, Some(a)) => format!("Then {} {} ({})", a.1, a.0, w.nfl_team_name(a.4)),
                    _ => String::new(),
                };
                v.push((
                    score,
                    id,
                    hn.name.clone(),
                    format!("{what}{}", if more.is_empty() { String::new() } else { format!(" · {more}") }),
                    hn.pos.map_or("", |x| x.name()),
                ));
            }
            v.sort_by(|a, b| b.0.total_cmp(&a.0));
            v.truncate(4);
            fill_with_picks(c, &mut v);
            let rows = legends(c, v);
            let note = last_first_rounder(c).map_or_else(
                || "No first-round pick on record.".to_string(),
                |(n, pos, pick, y, t)| format!("The last first-round pick: {n}, {pos}, No. {pick} in {y} ({t})."),
            );
            ("On the wall".into(), "the names they still say".into(), rows, note)
        }
        TeamRef::Nfl(t) => {
            let weight = |a: &str| match a {
                x if x.contains("Most Valuable Player") && x.contains("Super Bowl") => 35.0,
                x if x.contains("Most Valuable Player") || x == "MVP" => 40.0,
                x if x.contains("Defensive Player") || x.contains("Offensive Player") => 30.0,
                x if x.contains("Comeback") => 12.0,
                _ => 15.0,
            };
            let mut v: Vec<(f64, u32, String, String, &'static str)> = Vec::new();
            for a in w.nfl_book.awards.iter().filter(|a| a.4 == t) {
                match v.iter_mut().find(|x| x.1 == a.2) {
                    Some(x) => x.0 += weight(a.1),
                    None => {
                        let pos = views::person_index(w, a.2).map_or("", |k| w.people[k].player.position.name());
                        v.push((weight(a.1), a.2, a.3.clone(), format!("{} {}", a.1, a.0), pos));
                    }
                }
            }
            v.sort_by(|a, b| b.0.total_cmp(&a.0));
            v.truncate(4);
            fill_with_picks(c, &mut v);
            let rows = legends(c, v);
            // Players still here with a name the world gave them.
            let nicks: Vec<String> = views::roster_people(w, c.me)
                .into_iter()
                .filter_map(|k| w.nickname_of(w.people[k].player.id).map(|n| format!("\"{n}\" ({})", w.people[k].name)))
                .take(2)
                .collect();
            let note = if nicks.is_empty() {
                String::new()
            } else {
                format!("Still here with a name: {}.", nicks.join(" and "))
            };
            ("On the wall".into(), "the names they still say".into(), rows, note)
        }
        TeamRef::Other(_) => ("On the wall".into(), String::new(), Vec::new(), String::new()),
    }
}

/// Four legends: a drawn face while he still plays, else his initials.
fn legends(c: &Ctx, v: Vec<(f64, u32, String, String, &'static str)>) -> Vec<Value> {
    let w = c.w;
    v.into_iter()
        .take(4)
        .map(|(_, id, name, what, pos)| {
            let k = views::person_index(w, id);
            let playing = k.is_some_and(|k| matches!(w.people[k].stage, Stage::Pro | Stage::College));
            let nick = w.nickname_of(id).map(str::to_string);
            json!({
                "id": id,
                "name": nick.clone().unwrap_or_else(|| name.clone()),
                "full": name,
                "nick": nick.is_some(),
                "pos": pos,
                "face": playing,
                "what": what,
                "linked": k.is_some(),
            })
        })
        .collect()
}

fn story_slide(c: &Ctx) -> Value {
    let w = c.w;
    // The head coaches' eras, by coach, oldest first.
    let mut eras: Vec<(Option<usize>, i32, i32, u32, u32, u32)> = Vec::new();
    for s in &c.seasons {
        match eras.last_mut() {
            Some(e) if e.0 == s.coach && s.coach.is_some() => {
                e.2 = s.year;
                e.3 += s.w;
                e.4 += s.l;
                e.5 += s.t;
            }
            _ => eras.push((s.coach, s.year, s.year, s.w, s.l, s.t)),
        }
    }
    let before_eras = eras.iter().filter(|e| e.0.is_none() || e.0 != c.you).count();
    let nth = before_eras + 1;
    let sub = format!("{}{}", so_far(c), story(c).1);
    let first = c.first_year();
    // As an assistant the title names the head coach you work for.
    let title = match c.head {
        Some(h) if c.role.is_some() => {
            let n = eras.iter().position(|e| e.0 == Some(h)).map_or(before_eras + 1, |k| k + 1);
            format!("You join {}, the {} head coach since {first}.", c.coach_name(Some(h)), ord(n))
        }
        _ if c.seasons.is_empty() => "The record book here starts with you.".to_string(),
        _ => format!("You are the {} head coach since {first}.", ord(nth)),
    };
    let level_line = views::team_line(w, c.me);
    let place = venue(w, c.me).1;
    let mine = c.before < c.seasons.len() || c.in_season();

    // Three big numbers.
    let since = |f: &dyn Fn(&Season) -> bool| c.seasons.iter().rev().take_while(|x| !f(x)).count();
    let mut stings: Vec<(String, String)> = Vec::new();
    match c.lv {
        Lv::College => {
            let n10 = since(&|s| s.w >= 10);
            stings.push(if n10 > 0 {
                (n10.to_string(), "seasons without ten wins".into())
            } else {
                (c.seasons.iter().filter(|s| s.w >= 10).count().to_string(), format!("ten-win seasons since {first}"))
            });
            let nw = since(&|s| s.winning());
            stings.push(if nw > 0 {
                (nw.to_string(), "seasons without a winning record".into())
            } else {
                (
                    c.seasons.iter().rev().take_while(|s| s.winning()).count().to_string(),
                    "winning seasons in a row".into(),
                )
            });
            stings.push((
                c.seasons.iter().filter(|s| s.conf).count().to_string(),
                format!("conference titles in {} years", c.seasons.len()),
            ));
        }
        Lv::Hs => {
            stings.push((
                c.seasons.iter().filter(|s| s.title).count().to_string(),
                format!("state titles since {first}"),
            ));
            if let Some(b) = c.seasons.iter().max_by_key(|s| (s.w, std::cmp::Reverse(s.l), s.year)) {
                stings.push((b.rec(), format!("the best year, {}", b.year)));
            }
            let t = c.seasons.iter().rev().take_while(|s| s.title).count();
            let wn = c.seasons.iter().rev().take_while(|s| s.winning()).count();
            stings.push(if t > 0 {
                (t.to_string(), "state titles in a row".into())
            } else if wn > 0 {
                (wn.to_string(), "winning seasons in a row".into())
            } else {
                (since(&|s| s.winning()).to_string(), "seasons without a winning record".into())
            });
        }
        Lv::Nfl => {
            let titles: Vec<i32> = c.seasons.iter().filter(|s| s.title).map(|s| s.year).collect();
            stings.push((
                titles.len().to_string(),
                if titles.is_empty() {
                    format!("Super Bowls since {first}")
                } else {
                    format!("Super Bowl{}: {}", if titles.len() == 1 { "" } else { "s" }, years_text(&titles))
                },
            ));
            if let TeamRef::Nfl(t) = c.me
                && let Some(club) = w.clubs.get(t)
            {
                let cap = afm_engine::world::pro::money::price_cap(w.year) as f64;
                let room = cap + club.carryover - afm_engine::world::pro::payroll(w, t, w.year) as f64;
                stings.push((millions(room), "cap room this league year".into()));
            }
            let run = c.seasons.iter().rev().take_while(|s| s.round > 0).count();
            stings.push(if run > 0 {
                (run.to_string(), "playoff years running".into())
            } else {
                (since(&|s| s.round > 0).to_string(), "seasons since the playoffs".into())
            });
        }
        Lv::Other => {
            stings.push((c.seasons.iter().filter(|s| s.title).count().to_string(), format!("titles since {first}")));
        }
    }

    // The chart: every season, a shade per coach, gold caps on titles.
    let era_of = |y: i32| eras.iter().position(|e| (e.1..=e.2).contains(&y)).unwrap_or(0);
    let bars: Vec<Value> = c
        .seasons
        .iter()
        .map(|s| {
            json!({
                "y": s.year, "w": s.w, "l": s.l, "t": s.t, "rec": s.rec(),
                "era": era_of(s.year),
                "cap": if matches!(c.lv, Lv::College) { s.conf || s.title } else { s.title },
                "you": s.coach.is_some() && s.coach == c.you,
            })
        })
        .collect();
    let era_rows: Vec<Value> = eras
        .iter()
        .map(|e| {
            json!({
                "coach": e.0.map_or_else(|| "No coach on record".to_string(), |k| last_name(&c.coach_name(Some(k)))),
                "full": c.coach_name(e.0),
                "id": e.0,
                "from": e.1, "to": e.2,
                "rec": wlt(e.3, e.4, e.5),
                "you": e.0.is_some() && e.0 == c.you,
            })
        })
        .collect();
    let marks = marks(c);
    // The dashed line: the next step up the record hasn't reached lately.
    let recent_best = c.prev().iter().rev().take(5).map(|s| s.w).max().unwrap_or(0);
    let line = marks
        .iter()
        .filter(|m| m.state == "open")
        .filter_map(|m| m.wins.map(|n| (n, m)))
        .filter(|(n, _)| *n > recent_best)
        .min_by_key(|(n, _)| *n)
        .or_else(|| marks.iter().filter_map(|m| m.wins.map(|n| (n, m))).max_by_key(|(n, _)| *n))
        .map(|(n, m)| {
            let last = c.prev().iter().rev().find(|s| s.w >= n);
            json!({
                "wins": n,
                "text": match last {
                    Some(s) => format!("{n} wins: last done in {}", s.year),
                    None => format!("{n} wins: {}", m.t.to_lowercase()),
                },
            })
        });
    // This season's bar: dashed until it is played, filled as it goes.
    // The season under way or still to come on this page, else the next one.
    let open = c.this_record().is_some_and(|(wn, l, t, left)| left > 0 || wn + l + t == 0);
    let this_year =
        c.this.as_ref().map(|p| p.year).filter(|y| (c.in_season() || open) && !c.seasons.iter().any(|s| s.year == *y));
    let now = this_year.or_else(|| {
        let after = c.seasons.last().map(|s| s.year).max(c.this.as_ref().map(|p| p.year));
        Some(after.map_or(w.year, |y| y + 1))
    });
    let you_bar = now.map(|y| {
        let (wn, l, t, _) = if Some(y) == this_year { c.this_record().unwrap_or((0, 0, 0, 0)) } else { (0, 0, 0, 0) };
        json!({ "y": y, "w": wn, "rec": wlt(wn, l, t), "played": wn + l + t > 0 })
    });
    let (wall_title, wall_sub, wall, wall_note) = wall(c);

    json!({
        "kind": "story",
        "kicker": if mine {
            format!("The program's story · {}", c.name)
        } else if c.role.is_some() {
            format!("The program you join · {}", c.name)
        } else {
            format!("The program you take over · {}", c.name)
        },
        "title": title,
        "sub": sub,
        "colors": c.colors,
        "go": { "screen": "history", "args": {}, "label": "History" },
        "hero": {
            "level": c.lv.key(),
            "crest": crest(c),
            "kicker2": format!("{level_line} · {place} · {} seasons on record", c.seasons.len()),
            "stings": stings.into_iter().map(|(n, l)| json!({ "n": n, "l": l })).collect::<Vec<_>>(),
            "bars": bars,
            "eras": era_rows,
            "now": you_bar,
            "line": line,
            "title_word": c.lv.title(),
            "marks": marks.iter().map(|m| json!({ "n": m.n, "t": m.t, "l": m.l, "state": m.state })).collect::<Vec<_>>(),
            "wall": { "title": wall_title, "sub": wall_sub, "rows": wall, "note": wall_note },
        },
    })
}

// ------------------------------------------------------------------ C: the room

/// Last season's starters: the starter flags, filled to each position's
/// starting count from the depth chart where a flag is missing.
fn starters(c: &Ctx) -> Vec<usize> {
    let w = c.w;
    let people = views::roster_people(w, c.me);
    let depth = w.seat_game_depth();
    let mut out: Vec<usize> = Vec::new();
    for &pos in Position::ALL.iter() {
        let n = afm_engine::game::roster::starters(pos).max(1) as usize;
        // Last season's starters until this season's games begin, then the depth chart's.
        let mut here: Vec<usize> = if c.in_season() {
            Vec::new()
        } else {
            people
                .iter()
                .copied()
                .filter(|&k| {
                    w.people[k].player.position == pos
                        && w.people[k].last_role == Some(PlayingTime::Starter)
                        && !out.contains(&k)
                })
                .collect()
        };
        if here.len() < n {
            let order: Vec<usize> = depth
                .as_ref()
                .and_then(|d| d.iter().find(|(p, _)| *p == pos))
                .map(|(_, ids)| {
                    ids.iter().filter_map(|&id| people.iter().copied().find(|&k| w.people[k].player.id == id)).collect()
                })
                .unwrap_or_default();
            for k in order.into_iter().chain(people.iter().copied().filter(|&k| w.people[k].player.position == pos)) {
                if here.len() >= n {
                    break;
                }
                // one face per player: a kicker who punts, or a starter moved
                // to another slot on the chart, sits in the photo once
                if !here.contains(&k) && !out.contains(&k) {
                    here.push(k);
                }
            }
        }
        here.truncate(n.max(here.len().min(n + 1)));
        out.extend(here);
    }
    out
}

fn room(c: &Ctx) -> Value {
    let w = c.w;
    let roster = views::roster_people(w, c.me);
    let n = roster.len();
    let caps = captains(c);
    let starters = starters(c);
    let group = |k: usize| views::unit(w.people[k].player.position);
    let row = |k: usize| {
        let mut v = person_brief(c, k);
        v["cap"] = json!(caps.contains(&k));
        v
    };
    let back: Vec<Value> =
        starters.iter().copied().filter(|&k| group(k) == "OFF" && !caps.contains(&k)).map(row).collect();
    let mid: Vec<Value> =
        starters.iter().copied().filter(|&k| group(k) == "DEF" && !caps.contains(&k)).map(row).collect();
    let st: Vec<Value> =
        starters.iter().copied().filter(|&k| group(k) == "ST" && !caps.contains(&k)).map(row).collect();
    let front_caps: Vec<Value> = caps.iter().copied().map(row).collect();

    // The room's facts.
    let team = Team::seat(w);
    let mood = team.map_or(0.0, |t| talks::team_mood(w, t));
    let mood_words = talks::mood_words(mood);
    let mood_short = mood_words.split([':', ',']).next().unwrap_or(mood_words).trim().to_string();
    let unhappy: Vec<(usize, String)> = {
        let mut v: Vec<(f32, usize, String)> = roster
            .iter()
            .copied()
            .filter(|&k| w.people[k].mood < -0.25)
            .map(|k| {
                let why = talks::reasons(w, w.people[k].player.id).into_iter().next().map(|x| x.1);
                (w.people[k].mood, k, why.unwrap_or_else(|| talks::concern(w, k).1))
            })
            .collect();
        v.sort_by(|a, b| a.0.total_cmp(&b.0));
        v.into_iter().map(|x| (x.1, x.2)).collect()
    };
    let hurt: Vec<(usize, String)> = roster
        .iter()
        .copied()
        .filter_map(|k| {
            let s = views::status_text(w, &w.people[k]);
            (s.starts_with("Injured") || s.starts_with("Non-football")).then_some((k, s))
        })
        .collect();
    let names = |v: &[usize]| {
        let n: Vec<String> = v
            .iter()
            .take(3)
            .map(|&k| format!("{}, {}", w.people[k].name, w.people[k].player.position.name()))
            .collect();
        match n.len() {
            0 => String::new(),
            1 => n[0].clone(),
            _ => format!("{} and {}", n[..n.len() - 1].join("; "), n[n.len() - 1]),
        }
    };
    let last = c.seasons.last();
    let mut facts: Vec<(String, String, u8)> = Vec::new();
    let held = match c.lv {
        Lv::Hs => "players in the program",
        Lv::Nfl | Lv::Other => "players under contract",
        Lv::College => "players on the roster",
    };
    let photo = match (c.in_season(), last) {
        (true, _) | (false, None) => "Your starters are in the photo.".to_string(),
        (false, Some(s)) => format!("The starters from {} are in the photo.", s.year),
    };
    facts.push((n.to_string(), format!("{held}. {photo}"), 0));
    // The mood follows this season's record once it has one.
    let after = match (c.in_season(), c.this_record(), last) {
        (true, Some((wn, l, t, _)), _) => format!(" at {}", wlt(wn, l, t)),
        (_, _, Some(s)) => format!(" after {}", s.rec()),
        _ => String::new(),
    };
    let unhappy_ids: Vec<usize> = unhappy.iter().map(|x| x.0).collect();
    facts.push(if unhappy.is_empty() {
        ("0".into(), "unhappy players. Nobody is talking about leaving.".into(), 0)
    } else {
        (
            unhappy.len().to_string(),
            format!(
                "{} unhappy: {}. {}",
                if unhappy.len() == 1 { "is" } else { "are" },
                names(&unhappy_ids),
                cap_first(&unhappy[0].1)
            ),
            2,
        )
    });
    match c.me {
        TeamRef::College(p) if !c.in_season() => {
            let class: Vec<usize> = w
                .people
                .iter()
                .enumerate()
                .filter(|(_, x)| {
                    x.stage == Stage::HighSchool
                        && (x.cal.signed.is_some_and(|s| s.program == p) || x.cal.commit.is_some_and(|c| c.0 == p))
                })
                .map(|(k, _)| k)
                .collect();
            let four: Vec<usize> = class.iter().copied().filter(|&k| w.people[k].stars >= 4).collect();
            if !class.is_empty() {
                facts.push((
                    class.len().to_string(),
                    format!(
                        "signees and commitments arrive in June{}",
                        if four.is_empty() {
                            ".".to_string()
                        } else {
                            format!(", {} of them four-stars or better: {}.", four.len(), names(&four))
                        }
                    ),
                    0,
                ));
            }
        }
        TeamRef::HighSchool(h) if !c.in_season() => {
            let leave = leaving(w, h);
            if !leave.is_empty() {
                facts.push((
                    leave.len().to_string(),
                    format!("seniors leave for college in June: {}.", schools_text(w, &leave)),
                    0,
                ));
            }
        }
        _ => {}
    }
    let hurt_ids: Vec<usize> = hurt.iter().map(|x| x.0).collect();
    if !hurt.is_empty() {
        facts.push((hurt.len().to_string(), format!("hurt: {}.", names(&hurt_ids)), 1));
    }
    facts.push((
        cap_first(&mood_short),
        format!("The room's mood: {mood_words}{after}."),
        if mood < -0.15 { 2 } else { 0 },
    ));
    if facts.len() > 4 {
        // Keep the count and the mood; the middle two most pressing.
        let mood_fact = facts.pop();
        facts.truncate(3);
        facts.extend(mood_fact);
    }

    let headline = if c.day_one {
        format!("{n} players are waiting for you.")
    } else {
        format!("{n} players, and a {} room.", mood_short.split_whitespace().next().unwrap_or("steady"))
    };
    let cap_names: Vec<String> = caps.iter().map(|&k| last_name(&w.people[k].name)).collect();
    let sub = match cap_names.len() {
        0 => "No captains named yet.".to_string(),
        1 => format!("One captain: {}.", cap_names[0]),
        k => format!("{} captains: {} and {}.", word(k), cap_names[..k - 1].join(", "), cap_names[k - 1]),
    };
    let year = c.this.as_ref().map_or(w.year, |p| p.year);

    json!({
        "kind": "room",
        "kicker": format!("The room · {}", c.name),
        "title": headline,
        "sub": format!("{sub} {}{}", so_far(c), story(c).1),
        "colors": c.colors,
        "go": { "screen": "roster", "args": {}, "label": "Squad" },
        "hero": {
            "level": c.lv.key(),
            "crest": crest(c),
            "word": end_zones(c).0,
            "back": back,
            "mid": mid,
            "st": st,
            "caps": front_caps,
            "you_role": c.role.unwrap_or("head coach"),
            "cap_head": match last {
                Some(s) if s.year != year && !c.in_season() => format!("{}, {year}: the starters from {}", c.name, s.year),
                _ => format!("{}, {year}: the starters", c.name),
            },
            "facts": facts.into_iter().map(|(n, t, tone)| json!({ "n": n, "t": t, "tone": tone })).collect::<Vec<_>>(),
        },
    })
}

/// People to know first (the frame under the hero): the unhappiest with a
/// reason, a captain, and someone hurt or a captain more.
pub fn know(w: &World, me: TeamRef) -> Vec<Value> {
    let roster = views::roster_people(w, me);
    let team = Team::seat(w);
    let caps: Vec<usize> = team.map(|t| takeover::captains_of(w, t)).unwrap_or_default();
    let mut out: Vec<Value> = Vec::new();
    let mut seen: Vec<usize> = Vec::new();
    let mut add = |k: usize, why: String, tone: u8, out: &mut Vec<Value>| {
        if seen.contains(&k) || out.len() >= 3 {
            return;
        }
        seen.push(k);
        let p = &w.people[k];
        out.push(
            json!({ "id": p.player.id, "name": p.name, "pos": p.player.position.name(), "why": why, "tone": tone }),
        );
    };
    if let Some(k) = roster
        .iter()
        .copied()
        .filter(|&k| w.people[k].mood < -0.25)
        .min_by(|&a, &b| w.people[a].mood.total_cmp(&w.people[b].mood))
    {
        let why = talks::reasons(w, w.people[k].player.id)
            .into_iter()
            .next()
            .map(|x| x.1)
            .unwrap_or_else(|| talks::concern(w, k).1);
        add(k, cap_first(&why), 2, &mut out);
    }
    for &k in &caps {
        if w.people.get(k).is_some_and(|p| p.stage != Stage::Done) {
            let line = {
                let p = &w.people[k];
                format!("Captain. {} · {}", views::class_text(w, p), views::hometown(w, p))
            };
            add(k, line, 0, &mut out);
            break;
        }
    }
    if let Some((k, s)) = roster.iter().copied().find_map(|k| {
        let s = views::status_text(w, &w.people[k]);
        (s.starts_with("Injured") || s.starts_with("Non-football")).then_some((k, s))
    }) {
        add(k, s, 1, &mut out);
    }
    for &k in caps.iter().skip(1) {
        if w.people.get(k).is_some_and(|p| p.stage != Stage::Done) {
            let p = &w.people[k];
            add(k, format!("Captain. {} · {}", views::class_text(w, p), views::hometown(w, p)), 0, &mut out);
        }
    }
    out
}
