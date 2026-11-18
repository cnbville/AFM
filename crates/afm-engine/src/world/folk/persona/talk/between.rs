//! You and the coach (Cain 2026-11-16: "do they understand that i'm a new
//! coach with no experience with this team yet?", then "it should always
//! check it's relation with me when speaking to me, every aspect as much as
//! it would itself"). Before this the voice's card said one thing about
//! you, and said it the wrong way round ("you're new to him, and he is
//! still making up his mind about you", to the player): a DT who had met
//! you that week answered as if you had years together. Every talk now
//! reads the whole live history between the man across and you from the
//! world's own records, as `standing` reads his own situation: when you
//! came and the time of year, last season and the team under you, how he
//! came to be here, his games, starts and practice weeks under you, each
//! promise you made him and where it stands, your past talks and how the
//! last one went, your rulings on him, what moved his mood about you, the
//! deals, secrets and lies between you, his trust in you, and your name
//! for keeping your word where he hears it. A coach in his first week has
//! almost nothing here, and the card says so; a coach in his fifth year
//! has years of it.
//!
//! Words only, in the second person: "you" is him, "the coach" is you.
//! Nothing he couldn't know: no rating and no chart he hasn't seen (his
//! place is what `seen` shows him), only dates, games and records.
//! Every cap here is proposed, not confirmed.

use super::super::Them;
use super::super::makeup;
use super::super::memory::Treated;
use super::super::wants::Kind;
use super::stakes;
use crate::calendar::{Date, Level};
use crate::positions::{POSITION_COUNT, Position};
use crate::training::seat::GROUP_ONES;
use crate::world::folk::claims::words;
use crate::world::folk::rules::Response;
use crate::world::folk::scene::{Kind as SceneKind, Said, Scene, Who};
use crate::world::folk::talks::{Promise, PromiseKind, PromiseTo, Reason};
use crate::world::folk::{self as folk, Name, Team, boss, facts, pledge, promise, seen};
use crate::world::{Stage, Visit, World};

/// The most of each kind of memory the card carries, newest first
/// (proposed, not confirmed).
const MAX_PROMISES: usize = 4;
const MAX_TALKS: usize = 3;
const MAX_MOODS: usize = 4;
const MAX_RULINGS: usize = 3;
const MAX_OTHER: usize = 2;
/// A quoted line from your last talk is cut to this many characters.
const QUOTE: usize = 150;
/// Under this many days in the job the coach is new to everyone
/// (proposed, not confirmed).
const NEW_DAYS: i32 = 60;

/// Who the words are about: the man himself, or a parent's son.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Voice {
    You,
    Son,
}

impl Voice {
    /// "you" or "your son".
    fn him(self) -> &'static str {
        match self {
            Voice::You => "you",
            Voice::Son => "your son",
        }
    }
    /// "You" or "Your son".
    fn him_cap(self) -> &'static str {
        match self {
            Voice::You => "You",
            Voice::Son => "Your son",
        }
    }
    /// "your" or "his".
    fn his(self) -> &'static str {
        match self {
            Voice::You => "your",
            Voice::Son => "his",
        }
    }
    /// "you'd" or "he'd".
    fn hed(self) -> &'static str {
        match self {
            Voice::You => "you'd",
            Voice::Son => "he'd",
        }
    }
    /// "you've" or "he's".
    fn have(self) -> &'static str {
        match self {
            Voice::You => "you've",
            Voice::Son => "he's",
        }
    }
}

/// The whole history between the man across and you, in the second person,
/// for the voice's card ("You and the coach").
pub fn words(w: &World, him: Them) -> Vec<String> {
    let mut out = Vec::new();
    let kind = makeup::kind(w, him);
    let building = matches!(kind, Kind::Player | Kind::Family | Kind::Staff | Kind::Boss | Kind::Booster);
    let new = w.folk.mine.since.is_some_and(|d| d.days_to(w.today) < NEW_DAYS);
    if building || matches!(kind, Kind::Recruit | Kind::Transfer) {
        out.extend(arrival(w, kind));
    }
    let team = Team::seat(w);
    if building && let Some(t) = team {
        out.extend(moment(w, t));
        out.extend(record(w, t));
    }
    match (kind, him) {
        (Kind::Player, Them::Player(id)) => {
            if let Some(i) = folk::person(w, id) {
                player(w, i, Voice::You, &mut out);
            }
            promises(w, |to| *to == PromiseTo::Player(id), Voice::You, true, &mut out);
            if let Some(d) = w.folk.mine.told_straight.iter().rev().find(|x| x.0 == id).map(|x| x.1) {
                out.push(format!(
                    "{} the coach told you straight what he could and couldn't promise you",
                    cap(&when(w, d))
                ));
            }
            meetings(w, &mut out);
        }
        (Kind::Family, Them::Kin(id, _)) => {
            let son = folk::person(w, id);
            let ours = son.and_then(|i| makeup::team_of(w, i)).is_some_and(|t| Some(t) == team);
            match son {
                Some(i) if ours => player(w, i, Voice::Son, &mut out),
                Some(i) => courted(w, i, Voice::Son, &mut out),
                None => {}
            }
            promises(w, |to| *to == PromiseTo::Player(id), Voice::Son, ours, &mut out);
        }
        (Kind::Recruit | Kind::Transfer, Them::Player(id)) => {
            if let Some(i) = folk::person(w, id) {
                courted(w, i, Voice::You, &mut out);
            }
            promises(w, |to| *to == PromiseTo::Player(id), Voice::You, false, &mut out);
        }
        (Kind::Staff, Them::Coach(c)) => {
            staff(w, c, &mut out);
            promises(w, |to| *to == PromiseTo::Coach(c), Voice::You, true, &mut out);
        }
        (Kind::Boss, Them::Cast(r, term)) => {
            if let Some(t) = team {
                goals(w, t, &mut out);
            }
            promises(w, |to| *to == PromiseTo::Cast(r, term), Voice::You, false, &mut out);
        }
        (_, Them::Cast(r, term)) => promises(w, |to| *to == PromiseTo::Cast(r, term), Voice::You, false, &mut out),
        (_, Them::Coach(c)) => promises(w, |to| *to == PromiseTo::Coach(c), Voice::You, false, &mut out),
        _ => {}
    }
    let talked = talks(w, him, building && new, &mut out);
    if let Them::Player(id) = him {
        discipline(w, id, &mut out);
        moods(w, id, &mut out);
        bond(w, id, &mut out);
    }
    between(w, him, &mut out);
    let known = matches!(him, Them::Coach(c) if w.folk.network.contacts.iter().any(|x| x.coach_id() == Some(c)));
    if !building && talked == 0 && !known && !matches!(kind, Kind::Recruit | Kind::Transfer) {
        out.push("You know the coach only by what you've heard: you haven't dealt with him before".into());
    }
    out.extend(trust(w, him, kind, new && talked == 0));
    out.extend(name(w, kind));
    out
}

/// What the coach means to him, and the moments that built it (coach and
/// player bonds, section 9): the voice speaks to him as to a father, a big
/// brother or a man he barely knows.
fn bond(w: &World, id: u32, out: &mut Vec<String>) {
    let Some((step, moments)) = folk::bonds::with_you(w, id) else { return };
    out.push(match step.as_str() {
        "Just met" => "You barely know the coach yet".to_string(),
        "Knows you" => "You know the coach a little: you answer him, not much more".to_string(),
        "Trusts you" => "You go to the coach with your problems".to_string(),
        "In his corner" => "The coach is in your corner, and you're in his".to_string(),
        top => format!("The coach is {} to you", top.to_lowercase()),
    });
    for m in moments.iter().take(MAX_MOODS) {
        out.push(format!("The coach {}", m.to_him()));
    }
}

/// The team's history with you, for a line to the whole room: its
/// spokesman answers for them, so his own promises and talks stay off it.
pub fn room(w: &World) -> Vec<String> {
    let mut out = Vec::new();
    out.extend(arrival(w, Kind::Player));
    if let Some(t) = Team::seat(w) {
        out.extend(moment(w, t));
        out.extend(record(w, t));
    }
    meetings(w, &mut out);
    out
}

/// When a past day was: "today", "yesterday", "on Saturday", "last week",
/// "on March 3", with its year once it's most of a year back.
fn when(w: &World, d: Date) -> String {
    if d.days_to(w.today) > 300 {
        format!("on {}, {}", words::date_words(d), d.parts().0)
    } else {
        words::when_words(d, w.today)
    }
}

/// How long ago, in round words: "about 5 weeks ago".
pub fn ago(days: i32) -> String {
    match days {
        ..=0 => "today".into(),
        1 => "yesterday".into(),
        2..=13 => format!("{days} days ago"),
        14..=59 => format!("about {} weeks ago", (days + 3) / 7),
        60..=364 => format!("about {} months ago", (days + 15) / 30),
        365..=545 => "about a year ago".into(),
        _ => format!("about {} years ago", (days + 182) / 365),
    }
}

/// When the coach came, and how new he is.
fn arrival(w: &World, kind: Kind) -> Option<String> {
    let since = w.folk.mine.since?;
    let team = Team::seat(w)?.name(w);
    let role = stakes::your_role(w);
    let days = since.days_to(w.today).max(0);
    let on = when(w, since);
    // "since March 3, 2021", never "since on March 3".
    let from = match days {
        d if d > 300 => format!("{}, {}", words::date_words(since), since.parts().0),
        _ => words::date_words(since),
    };
    // Inside two weeks the day says it ("today", "on Saturday"); past that
    // the date and how long ago.
    let on = if days < 14 { on } else { format!("{on}, {}", ago(days)) };
    Some(match (days, kind) {
        (d, Kind::Recruit | Kind::Transfer) if d < NEW_DAYS => {
            format!("The coach took the {role} job at {team} {on}: he's new there")
        }
        (_, Kind::Recruit | Kind::Transfer) => {
            format!("The coach has been {role} at {team} since {from}, {}", ago(days))
        }
        (d, _) if d < NEW_DAYS => {
            format!("The coach took the {role} job here {on}: he's new here, and he has no history with this team yet")
        }
        _ => format!("The coach has been {role} here since {from}, {}", ago(days)),
    })
}

/// The time of year, as everyone in the building knows it.
fn moment(w: &World, team: Team) -> Option<String> {
    let t = w.today;
    let d = &w.dates;
    let day = |x: Date| ahead(x, t);
    let left: Vec<Date> = facts::team_games(w, team).into_iter().map(|g| g.0).filter(|&g| g >= t).collect();
    // A game of this football year already played.
    let played =
        |from: Date| w.seat_games.iter().any(|g| g.date >= from && g.date <= t && in_game(g.teams, g.level, team));
    let next = left.first().copied();
    Some(match team.level() {
        Level::Nfl => match () {
            _ if t < d.otas.0 => format!("It's the offseason: OTAs start {}", day(d.otas.0)),
            _ if t <= d.minicamp.1 => "It's the spring program: OTAs and minicamp".into(),
            _ if t < d.veterans_report => format!("It's the summer break: camp opens {}", day(d.veterans_report)),
            _ if t < d.nfl_kickoff => {
                format!("Training camp and the preseason are on: the opener is {}", day(d.nfl_kickoff))
            }
            _ if !left.is_empty() => "The season is on".into(),
            _ => "The season is over".into(),
        },
        Level::College => match () {
            _ if t < d.spring_practice.0 => {
                format!("It's the winter offseason: spring practice opens {}", day(d.spring_practice.0))
            }
            _ if t <= d.spring_game => format!("Spring practice is on: the spring game is {}", day(d.spring_game)),
            _ if t < d.fall_camp => format!("It's the summer offseason: fall camp opens {}", day(d.fall_camp)),
            _ if !played(d.fall_camp) => match next {
                Some(o) => format!("Fall camp is on: the opener is {}", day(o)),
                None => "Fall camp is on".into(),
            },
            _ if !left.is_empty() => "The season is on".into(),
            _ => "The season is over".into(),
        },
        _ => match next {
            Some(o) if !played(t.add(-200)) => format!("It's before the season: the first game is {}", day(o)),
            _ if !left.is_empty() => "The season is on".into(),
            _ => "The season is over".into(),
        },
    })
}

/// A day ahead the way people say it, never read off a calendar: "on
/// Saturday" inside a week, "in about a week", "in about two weeks", then
/// "in early March", "in mid-March", "in late March" (a player told "spring
/// opens on March 3" said the date back).
fn ahead(d: Date, t: Date) -> String {
    let month = words::date_words(d).split(' ').next().unwrap_or_default().to_string();
    match t.days_to(d) {
        ..=6 => words::when_words(d, t),
        7..=10 => "in about a week".into(),
        11..=17 => "in about two weeks".into(),
        _ => match d.parts().2 {
            ..=10 => format!("in early {month}"),
            11..=20 => format!("in mid-{month}"),
            _ => format!("in late {month}"),
        },
    }
}

/// A kept game is this team's.
fn in_game(teams: [usize; 2], level: Level, team: Team) -> bool {
    let (k, i) = team.parts();
    teams.contains(&i) && (k == "nfl") == (level == Level::Nfl)
}

/// Last season if it wasn't his, and the team under the coach: each
/// finished season he coached here, and this one so far.
fn record(w: &World, team: Team) -> Vec<String> {
    let mut out = Vec::new();
    let you = makeup::your_coach(w);
    let seasons = facts::seasons(w, team, 40);
    if let Some(last) = seasons.last()
        && last.coach != you
    {
        let by = last.coach.and_then(|c| w.coaches.get(c)).map(|c| format!(" under {}", c.name)).unwrap_or_default();
        out.push(format!(
            "Last season, before the coach came, the team went {}-{}{by}{}",
            last.wins,
            last.losses,
            if last.note.is_empty() { String::new() } else { format!(", {}", last.note) }
        ));
    }
    let mine: Vec<&facts::Season> = seasons.iter().filter(|s| s.coach.is_some() && s.coach == you).collect();
    let (wn, l) = facts::record_now(w, team);
    let done = mine.last().is_some_and(|s| s.year >= w.dates.season);
    let mut parts: Vec<String> = mine.iter().rev().take(4).rev().map(|s| facts::season_words(s)).collect();
    if wn + l > 0 && !done {
        parts.push(format!("{wn}-{l} so far this season"));
    }
    if parts.is_empty() {
        out.push("The team hasn't played a game under the coach yet".into());
    } else {
        out.push(format!("The team under the coach: {}", parts.join("; ")));
    }
    out
}

/// His arrival date in your training room's book, past the first week's
/// fill-in (`training::seat`: everyone there that first week reads as a
/// year in).
fn arrived(w: &World, id: u32) -> Option<Date> {
    w.training.arrived.iter().find(|x| x.0 == id).map(|x| x.1)
}

/// How your player came to be here, his games, starts and practice weeks
/// under you, and the coach's rulings on him.
fn player(w: &World, i: usize, v: Voice, out: &mut Vec<String>) {
    let p = &w.people[i];
    let id = p.player.id;
    let Some(team) = Team::seat(w) else { return };
    let since = w.folk.mine.since.unwrap_or(w.today);
    let name = team.name(w);
    let you = makeup::your_coach(w);
    // The man you replaced, when the takeover names him, else the last
    // head coach whose stint here ended.
    let old = w
        .folk
        .takeover
        .as_ref()
        .filter(|t| t.team == Some(team))
        .and_then(|t| t.old_coach)
        .or_else(|| you.and_then(|y| facts::predecessor(w, team, y)).map(|x| x.0))
        .and_then(|c| w.coaches.get(c))
        .map(|c| c.name.clone());
    let came = match team {
        Team::College(prog) if p.path.len() >= 2 && p.path.last() == Some(&prog) => {
            let from = w.program_name(p.path[p.path.len() - 2]).to_string();
            let after = p.cal.portal.as_ref().is_some_and(|e| e.entered >= since)
                || arrived(w, id).is_some_and(|d| d > since.add(7));
            Some(if after {
                format!("{} transferred here from {from} after the coach came", v.him_cap())
            } else {
                format!("{} transferred here from {from} before the coach came", v.him_cap())
            })
        }
        Team::College(prog) => match p.cal.signed.as_ref().filter(|s| s.program == prog) {
            Some(s) if s.date >= since => Some(format!(
                "The coach recruited {}: {} signed with {name} {}",
                v.him(),
                if v == Voice::You { "you" } else { "he" },
                when(w, s.date)
            )),
            Some(s) => Some(format!(
                "{} {} here before the coach came: {} signed with {name} {}{}, and the coach inherited {}",
                v.him_cap(),
                if v == Voice::You { "were" } else { "was" },
                if v == Voice::You { "you" } else { "he" },
                when(w, s.date),
                old.as_ref().map(|o| format!(", when {o} was the coach")).unwrap_or_default(),
                if v == Voice::You { "you" } else { "him" }
            )),
            None => Some(came_in(w, i, prog, since, v)),
        },
        _ => match arrived(w, id) {
            Some(d) if d > since.add(7) => {
                Some(format!("{} joined {name} after the coach came, {}", v.him_cap(), when(w, d)))
            }
            _ => Some(format!(
                "{} {} with {name} before the coach came",
                v.him_cap(),
                if v == Voice::You { "were" } else { "was" }
            )),
        },
    };
    out.extend(came);
    // What he has seen of his place under you: practice weeks and games.
    let weeks: Vec<(Date, u8)> = seen::weeks(w, id).into_iter().filter(|x| x.0 >= since).collect();
    let games = seen::games(w, id, since, w.today, None);
    if weeks.is_empty() && games.is_empty() {
        out.push(format!(
            "{} practiced or played a game for the coach yet, so {} seen where he has {}",
            if v == Voice::You { "You haven't" } else { "Your son hasn't" },
            if v == Voice::You { "you haven't" } else { "he hasn't" },
            if v == Voice::You { "you" } else { "him" }
        ));
    } else {
        if !games.is_empty() {
            let n = games.len();
            let played = games.iter().filter(|g| g.line.is_some()).count();
            let started = games.iter().filter(|g| g.started()).count();
            let hurt = games.iter().filter(|g| g.hurt && g.line.is_none()).count();
            // The kept games may not reach back to his first: say how many.
            let all = w.seat_games.first().is_none_or(|g| g.date <= since);
            let of =
                if all { format!("the coach's {n} games here") } else { format!("the coach's last {n} games here") };
            let mut s = match (played, started) {
                (0, _) => format!("{} played in none of {of}", v.have()),
                (k, 0) => format!("{} played in {k} of {of} without a start", v.have()),
                (k, st) => format!("{} played in {k} of {of} and started {st}", v.have()),
            };
            if hurt > 0 {
                s += &format!(", and missed {hurt} hurt");
            }
            out.push(cap(&s));
        }
        if !weeks.is_empty() {
            let ones = weeks.iter().filter(|x| x.1 == GROUP_ONES).count();
            let n = weeks.len();
            out.push(match ones {
                0 => format!(
                    "In {n} practice week{} under the coach {} never worked with the first team",
                    if n == 1 { "" } else { "s" },
                    v.him()
                ),
                k if k == n => format!(
                    "In {n} practice week{} under the coach {} worked with the first team every week",
                    if n == 1 { "" } else { "s" },
                    v.him()
                ),
                k => format!("In {n} practice weeks under the coach {} worked with the first team in {k}", v.him()),
            });
        }
    }
}

/// How a player who came to your college out of high school got here,
/// once his signing is off the books (it is cleared when he enrolls): by
/// your arrival book (new on your roster after the coach's first week),
/// else by his first season here (his college coaches' rows, else his
/// class and any years away). He can't have been on campus before the
/// January of that season (an early enrollee), and he was by its August.
/// The coach signed him when he was here by the December before, ahead of
/// both signing days.
fn came_in(w: &World, i: usize, prog: usize, since: Date, v: Voice) -> String {
    let p = &w.people[i];
    let book = arrived(w, p.player.id);
    let first =
        p.college_staff.iter().find(|x| x.1 == prog).map_or(p.class_of + i32::from(p.roads.away_years), |x| x.0);
    let after = book.is_some_and(|d| d > since.add(7)) || Date::ymd(first, 1, 2) > since.add(7);
    let before = !after && (book.is_some() || Date::ymd(first, 8, 1) <= since);
    let (were, you) = if v == Voice::You { ("were", "you") } else { ("was", "he") };
    let joined = match book.filter(|d| *d > since.add(7)) {
        Some(d) => when(w, d),
        None => format!("for the {first} season"),
    };
    if after && !p.walk_on && since <= Date::ymd(first - 1, 12, 1) {
        format!("The coach signed {}, and {you} joined the team {joined}", v.him())
    } else if after {
        format!(
            "{} {} the team after the coach came, {joined}",
            v.him_cap(),
            if p.walk_on { "walked on to" } else { "joined" }
        )
    } else if before {
        format!("{} {were} on the team before the coach came", v.him_cap())
    } else {
        format!("{} came here for the {first} season, about when the coach did", v.him_cap())
    }
}

/// A recruit's history with your program: the offer, the visits, the
/// promises on how you'd use him.
fn courted(w: &World, i: usize, v: Voice, out: &mut Vec<String>) {
    let p = &w.people[i];
    let Some(prog) = w.seat.and_then(|s| s.program) else { return };
    let school = w.program_name(prog).to_string();
    let early = p.cal.early_offers.iter().find(|x| x.0 == prog).map(|x| x.1);
    let offered = early.is_some() || p.cal.offers.contains(&prog);
    if p.stage == Stage::HighSchool {
        out.push(match (offered, early) {
            (true, Some(d)) => format!("{school} offered {} a scholarship {}", v.him(), when(w, d)),
            (true, None) => format!("{school} has offered {} a scholarship", v.him()),
            _ => format!("{school} hasn't offered {} a scholarship", v.him()),
        });
    }
    let mut visits: Vec<&(usize, Date, Visit)> =
        p.cal.visits.iter().filter(|x| x.0 == prog && x.1 <= w.today).collect();
    visits.sort_by_key(|x| x.1);
    for (_, d, k) in visits.iter().rev().take(MAX_OTHER).rev() {
        let on = when(w, *d);
        out.push(match k {
            Visit::Official => format!("{} took an official visit to {school} {on}", v.him_cap()),
            Visit::Unofficial => format!("{} visited {school} on {} own {on}", v.him_cap(), v.his()),
            Visit::DropIn => format!("Someone on the coach's staff came to watch {} at school {on}", v.him()),
            Visit::Home => match v {
                Voice::You => format!("The coach sat in your family's living room {on}"),
                Voice::Son => format!("The coach sat in your living room {on}"),
            },
        });
    }
    let id = p.player.id;
    for (_, _, u) in w.folk.usage_pledges.iter().filter(|x| x.0 == id && x.1 == prog).take(MAX_OTHER) {
        out.push(format!("The coach promised {} {}", v.him(), you_words(&u.words(), v)));
    }
}

/// A coach on your staff: whether you hired him or kept him on.
fn staff(w: &World, c: usize, out: &mut Vec<String>) {
    let Some(co) = w.coaches.get(c) else { return };
    let mine = makeup::your_coach(w).and_then(|y| w.coaches.get(y)).and_then(|y| y.job).map(|j| j.first_season());
    let kept = w.folk.takeover.as_ref().is_some_and(|t| t.kept.contains(&c));
    let first = co.job.map(|j| j.first_season());
    // An assistant hires no one: he and you are on the same staff.
    let head = stakes::your_role(w) == "head coach";
    match (first, mine) {
        _ if kept && head => out.push("You were on the staff here before the coach came, and he kept you on".into()),
        (Some(f), Some(m)) if f < m => out.push("You were on the staff here before the coach came".into()),
        (Some(f), _) if head => out.push(format!(
            "The coach hired you: your first season on his staff {} {f}",
            if f < w.dates.season { "was" } else { "is" }
        )),
        (Some(f), Some(m)) if f == m => {
            out.push(format!("You and the coach both joined this staff for the {f} season"))
        }
        (Some(f), _) => out.push(format!("You joined this staff for the {f} season, after the coach")),
        _ => {}
    }
}

/// Your boss: the goals he set you and the asks you made of him.
fn goals(w: &World, team: Team, out: &mut Vec<String>) {
    let level = team.level();
    let season = boss::goal_season(w, team);
    let now: Vec<&boss::Goal> = w.folk.goals.iter().filter(|g| g.season == season).collect();
    if now.is_empty() {
        out.push(format!("You haven't set the coach's goals for {season} yet"));
    } else {
        let list: Vec<String> = now.iter().map(|g| g.kind.words(level)).collect();
        let set = now.iter().map(|g| g.set_on).max().unwrap_or(w.today);
        out.push(if set == w.today {
            format!("You're setting the coach's goals for {season} in this meeting: {}", list.join("; "))
        } else {
            format!("The goals you set the coach for {season} {}: {}", when(w, set), list.join("; "))
        });
    }
    let last: Vec<&boss::Goal> = w.folk.goals.iter().filter(|g| g.season == season - 1 && g.met.is_some()).collect();
    if !last.is_empty() {
        let met = last.iter().filter(|g| g.met == Some(true)).count();
        let missed: Vec<String> = last.iter().filter(|g| g.met == Some(false)).map(|g| g.kind.words(level)).collect();
        out.push(if missed.is_empty() {
            format!("The coach met all {} of the goals you set him for {}", last.len(), season - 1)
        } else {
            format!(
                "The coach met {met} of the {} goals you set him for {}: he missed {}",
                last.len(),
                season - 1,
                missed.join(" and ")
            )
        });
    }
    for a in w.folk.asks.iter().rev().take(MAX_OTHER).rev() {
        let what = match a.kind {
            boss::AskKind::StaffMoney => "more money for his staff",
            boss::AskKind::Facility => "a facility project",
            boss::AskKind::RecruitingMoney => "a bigger recruiting budget",
            boss::AskKind::NilMoney => "more NIL and revenue-sharing money",
            boss::AskKind::RosterSpots => "practice-squad money",
            boss::AskKind::Extension => "an extension",
        };
        out.push(format!(
            "{} the coach asked you for {what}, and you {}",
            cap(&when(w, a.date)),
            if a.granted { "said yes" } else { "said no" }
        ));
    }
}

/// A promise in the words of the man it was made to (or his family).
fn promise_words(p: &Promise, v: Voice) -> String {
    let pos = |k: u8| Position::ALL[k as usize % POSITION_COUNT];
    let spot = |k: u8| words::spot(pos(k));
    match p.kind {
        PromiseKind::Start => "a starting job".into(),
        PromiseKind::Snaps => "more snaps".into(),
        PromiseKind::Touches => "more touches".into(),
        PromiseKind::Contract => "a contract talk".into(),
        PromiseKind::Stay => "that he'd stay, and not leave for another job".into(),
        PromiseKind::Booster(x) => x.short().into(),
        PromiseKind::Raise => "a raise".into(),
        PromiseKind::Place(0) => format!("that {} keep {} starting job", v.hed(), v.his()),
        PromiseKind::Place(_) => format!("that {} place wouldn't change", v.his()),
        PromiseKind::Join(0) => format!("a starting job when {} came", if v == Voice::You { "you" } else { "he" }),
        PromiseKind::Join(_) => format!("a real role when {} came", if v == Voice::You { "you" } else { "he" }),
        PromiseKind::Recruit(k, at) => match pledge::Kind::of(k) {
            Some(pledge::Kind::Start) => format!("a start in {} first season", v.his()),
            Some(pledge::Kind::Role) => format!("a real role in {} first season", v.his()),
            Some(pledge::Kind::Position) => format!("that {} stay at {}", v.hed(), spot(at)),
            Some(pledge::Kind::Early) => "an early chance to play".into(),
            Some(pledge::Kind::NoRedshirt) => "no redshirt year".into(),
            Some(pledge::Kind::Usage) | None => {
                p.terms.usage.map_or_else(|| "a promise when he signed".into(), |u| you_words(&u.words(), v))
            }
        },
        PromiseKind::Position(at) => format!("that {} stay at {}", v.hed(), spot(at)),
        PromiseKind::Usage => p.terms.usage.map_or_else(|| "how he'd use you".into(), |u| you_words(&u.words(), v)),
        PromiseKind::Draft(r) if r >= 7 => format!("that his club would draft {}", v.him()),
        PromiseKind::Draft(1) => format!("that his club would take {} in the first round", v.him()),
        PromiseKind::Draft(r) => format!("that his club would take {} by round {r}", v.him()),
    }
}

/// A usage term's words ("screens to him") for the man it's about.
fn you_words(s: &str, v: Voice) -> String {
    match v {
        Voice::You => s.replace(" to him", " to you").replace(" him ", " you "),
        Voice::Son => s.to_string(),
    }
}

/// The promises the coach made him (or his son), newest first, each with
/// where it stands as he sees it (`promise::State`), and whether there are
/// none when he would know it (`none`).
fn promises(w: &World, to: impl Fn(&PromiseTo) -> bool, v: Voice, none: bool, out: &mut Vec<String>) {
    let mine: Vec<&Promise> = w.folk.promises.iter().filter(|p| to(&p.to)).collect();
    if mine.is_empty() {
        if none {
            out.push(format!("The coach hasn't made {} a promise", v.him()));
        }
        return;
    }
    for p in mine.iter().rev().take(MAX_PROMISES) {
        let stands = match p.kept {
            Some(true) => "he kept it".to_string(),
            Some(false) => "he broke it".to_string(),
            None => match promise::State::of(p.state) {
                promise::State::Waiting if p.due > w.today => format!("it comes due {}", when(w, p.due)),
                promise::State::Waiting => "nothing has tested it yet".into(),
                promise::State::OnTrack => "so far he's keeping it".into(),
                promise::State::Behind => "so far he's behind on it".into(),
                promise::State::Owed => format!(
                    "{} met {} end of it and {} it yet",
                    v.him(),
                    v.his(),
                    match v {
                        Voice::You => "don't have",
                        Voice::Son => "doesn't have",
                    }
                ),
                promise::State::Missed => format!("{} missed {} end of it", v.him(), v.his()),
                promise::State::Paused => {
                    format!("it waits until {} healthy", if v == Voice::You { "you're" } else { "he's" })
                }
            },
        };
        out.push(format!(
            "{} the coach promised {} {}{}: {stands}",
            cap(&when(w, p.made)),
            v.him(),
            promise_words(p, v),
            if p.public { ", in public" } else { "" }
        ));
    }
}

/// The team meetings your player sat in.
fn meetings(w: &World, out: &mut Vec<String>) {
    let team = Team::seat(w);
    let since = w.folk.mine.since.unwrap_or(w.today);
    let days: Vec<Date> = w
        .folk
        .scenes
        .iter()
        .filter(|s| {
            s.done
                && s.handed.is_none()
                && matches!(s.kind, SceneKind::TeamMeeting | SceneKind::TeamTalk)
                && s.date >= since
                && (s.team.is_none() || s.team == team)
                && s.lines.iter().any(|l| l.by == 1)
        })
        .map(|s| s.date)
        .collect();
    if let Some(&last) = days.last() {
        out.push(match days.len() {
            1 => format!("You were in the room for the coach's team meeting {}", when(w, last)),
            n => format!("You've been in the room for {n} of the coach's team meetings, the last {}", when(w, last)),
        });
    }
}

/// A scene's kind as he'd name the talk.
fn talk_words(k: SceneKind) -> &'static str {
    match k {
        SceneKind::RecruitCall
        | SceneKind::ParentCall
        | SceneKind::FamilyCall
        | SceneKind::CollegeCall
        | SceneKind::PortalCall
        | SceneKind::JobCall => "a call",
        SceneKind::RecruitMessage => "texts",
        SceneKind::HomeVisit => "a home visit",
        SceneKind::PortalVisit | SceneKind::Trip | SceneKind::Tour => "a visit",
        _ => "a talk",
    }
}

/// A talk with his family only.
fn family_kind(k: SceneKind) -> bool {
    matches!(k, SceneKind::Parents | SceneKind::ParentCall | SceneKind::FamilyCall | SceneKind::Family)
}

/// Whether he was across from you in a scene.
fn with(s: &Scene, him: Them) -> bool {
    if matches!(s.kind, SceneKind::TeamMeeting | SceneKind::TeamTalk) {
        return false;
    }
    s.who.iter().any(|x| match (x, him) {
        (Who::Player(a), Them::Player(b)) => *a == b && !family_kind(s.kind),
        (Who::Player(a), Them::Kin(b, _)) => *a == b && (family_kind(s.kind) || s.kind == SceneKind::HomeVisit),
        (Who::Coach(a), Them::Coach(b)) => *a == b,
        (Who::Cast(t), Them::Cast(r, _)) => *t == r.token(),
        _ => false,
    })
}

/// A line cut to `QUOTE` characters at a word, with no quote marks of its
/// own.
fn clip(s: &str) -> String {
    let s = folk::plain(s);
    let s = s.trim().trim_matches(|c| c == '"' || c == '\u{201c}' || c == '\u{201d}').trim();
    let s: String = s.split_whitespace().collect::<Vec<_>>().join(" ");
    if s.chars().count() <= QUOTE {
        return s;
    }
    let mut cut: String = s.chars().take(QUOTE).collect();
    if let Some(k) = cut.rfind(' ') {
        cut.truncate(k);
    }
    format!("{}...", cut.trim_end_matches([',', ';', ':', '.']))
}

/// The last thing you said in a scene that he answered, and his answer.
fn exchange(lines: &[Said]) -> Option<(String, String)> {
    let k = (0..lines.len()).rev().find(|&k| lines[k].by == 1 && lines[k + 1..].iter().any(|l| l.by == 2))?;
    let his = lines[k + 1..].iter().find(|l| l.by == 2)?;
    let (you, his) = (clip(&lines[k].text), clip(&his.text));
    (!you.is_empty() && !his.is_empty()).then_some((you, his))
}

/// A past talk: its day, its kind in words, and your last line he
/// answered with his answer.
type PastTalk = (Date, &'static str, Option<(String, String)>);

/// Your past talks with him, newest last, and how many: each one's kind
/// and day, and what was said in the last one. With none and `first`, it
/// says this is the first.
fn talks(w: &World, him: Them, first: bool, out: &mut Vec<String>) -> usize {
    let mut seen: Vec<PastTalk> = w
        .folk
        .scenes
        .iter()
        .filter(|s| s.done && s.handed.is_none() && s.lines.iter().any(|l| l.by == 1) && with(s, him))
        .map(|s| (s.date, talk_words(s.kind), exchange(&s.lines)))
        .collect();
    // A live talk whose scene the book has dropped.
    for t in w.folk.live.persona.talks.iter().filter(|t| t.with == him && t.day < w.today) {
        if seen.iter().any(|x| x.0 == t.day) {
            continue;
        }
        let last = t.lines.iter().rev().find(|l| !l.his.is_empty()).map(|l| (clip(&l.you), clip(&l.his)));
        seen.push((t.day, "a talk", last));
    }
    seen.sort_by_key(|x| x.0);
    let n = seen.len();
    if n == 0 {
        if first {
            out.push("This is your first talk alone with the coach".into());
        }
        return 0;
    }
    let list: Vec<String> = seen.iter().rev().take(MAX_TALKS).map(|x| format!("{} {}", x.1, when(w, x.0))).collect();
    out.push(format!(
        "You've talked with the coach {} before this one: {}",
        match n {
            1 => "once".to_string(),
            2 => "twice".to_string(),
            n => format!("{n} times"),
        },
        words::list(&list)
    ));
    // The last exchange on record, named as the last talk only when it is.
    if let Some((k, (d, _, Some((you, his))))) = seen.iter().enumerate().rev().find(|x| x.1.2.is_some()) {
        let which = if k + 1 == n { "your last talk" } else { "your talk" };
        out.push(format!(
            "In {which} ({}) the coach said \"{you}\" and you said \"{his}\"",
            when(w, *d).trim_start_matches("on ")
        ));
    }
    n
}

/// The coach's rulings on him, his suspensions, and an early return he
/// pushed.
fn discipline(w: &World, id: u32, out: &mut Vec<String>) {
    let level = Team::seat(w).map_or(Level::College, |t| t.level());
    let rulings: Vec<_> = w.folk.rules.rulings.iter().filter(|r| r.id == id).collect();
    for r in rulings.iter().rev().take(MAX_RULINGS).rev() {
        let did = match r.response {
            Response::Talk => "the coach talked to you and warned you".to_string(),
            Response::Fine => "the coach fined you".into(),
            Response::Running => "the coach gave you early-morning running".into(),
            Response::BenchStart => "the coach benched you for the start".into(),
            Response::BenchGame if level == Level::Nfl => "the coach had you deactivated for a game".into(),
            Response::BenchGame => "the coach sat you for a game".into(),
            Response::Suspend(n) => format!("the coach suspended you for {n} games"),
            Response::Dismiss => "the coach dismissed you".into(),
            Response::Council => "the coach let the leadership council decide".into(),
            Response::Indefinite => "the coach has you out until your case is settled".into(),
            Response::Wait => "the coach let you keep playing while your case runs".into(),
        };
        out.push(format!(
            "{} you broke the team rule on {}, and {did}",
            cap(&when(w, r.date)),
            lower_first(r.kind.name())
        ));
    }
    if let Some((_, through, why)) = w.folk.out.iter().find(|x| x.0 == id && x.1 >= w.today)
        && (why.contains("by you") || why.contains("suspended"))
    {
        out.push(format!(
            "The coach has you out through {}: {}",
            words::date_words(*through),
            why.replace(" by you", "").replace("his ", "your ")
        ));
    }
    if let Some(d) = w.folk.rushed.iter().rev().find(|x| x.0 == id).map(|x| x.1) {
        out.push(format!("{} the coach brought you back early from your injury", cap(&when(w, d))));
    }
}

/// What moved his mood about the coach this season, newest first.
fn moods(w: &World, id: u32, out: &mut Vec<String>) {
    let Some((_, notes)) = w.folk.moods.iter().find(|x| x.0 == id) else { return };
    let mut said: Vec<Reason> = Vec::new();
    for &(d, r, v) in notes.iter().rev() {
        let r = Reason::from_u8(r);
        if said.contains(&r) || said.len() >= MAX_MOODS {
            continue;
        }
        let up = v > 0;
        let what = match (r, up) {
            (Reason::Talk, true) => "a talk with the coach went well for you",
            (Reason::Talk, false) => "a talk with the coach went badly for you",
            (Reason::PromiseKept, _) => "the coach kept his word to you",
            (Reason::PromiseBroken, _) => "the coach broke a promise to you",
            (Reason::Discipline, true) => "you thought the coach handled discipline fairly",
            (Reason::Discipline, false) => "you felt the coach treated you unfairly over discipline",
            (Reason::Praise, _) => "the coach praised you in public",
            (Reason::Benched, _) => "the coach benched you",
            (Reason::TeamTalk, true) => "the coach's team talk got to you",
            (Reason::TeamTalk, false) => "you didn't buy the coach's team talk",
            (Reason::Practice, true) => "you like the way the coach's staff run the week",
            (Reason::Practice, false) => "the grind of the coach's practice weeks wears on you",
            (Reason::LieSeen, _) => "you found out something the coach told you wasn't true",
            (Reason::HardballHeard, _) => "you heard what the coach said about you in your contract talks",
            (Reason::RecruitLie, _) => "you found out that what the coach told you when he recruited you wasn't true",
            _ => continue,
        };
        said.push(r);
        out.push(format!("{} {what}", cap(&when(w, d))));
    }
}

/// How the coach treated him, the deals, answers and secrets between you,
/// and the lies he told you.
fn between(w: &World, him: Them, out: &mut Vec<String>) {
    let m = &w.folk.live.persona;
    let mut kinds: Vec<Treated> = Vec::new();
    for t in m.treated.iter().rev().filter(|t| t.who == him) {
        if kinds.contains(&t.how) || kinds.len() > MAX_OTHER {
            continue;
        }
        kinds.push(t.how);
        let what = match t.how {
            Treated::Hot => "a talk with the coach got heated",
            Treated::Lowballed => "the coach lowballed you in a deal",
            Treated::Asked => "the coach asked something of you",
            Treated::Ignored => "you went to the coach and he brushed you off",
            Treated::Dodged => "the coach dodged a question of yours",
            Treated::Apologised => "the coach apologised to you",
            Treated::Staked => "the coach put his own name on the line for you",
        };
        out.push(format!("{} {what}", cap(&when(w, t.day))));
    }
    for a in m.agreed.iter().rev().filter(|a| a.with == him).take(MAX_OTHER) {
        out.push(format!("{} you and the coach agreed on a deal: \"{}\"", cap(&when(w, a.day)), clip(&a.offer.words)));
    }
    for o in m.owed.iter().filter(|o| o.with == him && o.by >= w.today).take(MAX_OTHER) {
        out.push(format!(
            "You owe the coach an answer by {} on his offer: \"{}\"",
            words::date_words(o.by),
            clip(&o.offer.words)
        ));
    }
    for s in m.secrets.iter().rev().filter(|s| (s.by == him && !s.yours) || (s.yours && s.to == him)).take(MAX_OTHER) {
        out.push(if s.yours {
            format!(
                "{} the coach told you something between the two of you: \"{}\"{}",
                cap(&when(w, s.day)),
                clip(&s.words),
                if s.asked_quiet { ", and asked you to keep it quiet" } else { "" }
            )
        } else {
            format!(
                "{} you told the coach something between the two of you: \"{}\"{}",
                cap(&when(w, s.day)),
                clip(&s.words),
                if s.out { ", and it got out" } else { "" }
            )
        });
    }
    for l in m.his_lies.iter().rev().filter(|l| l.by == him).take(MAX_OTHER) {
        out.push(format!(
            "{} you told the coach something that wasn't true ({}){}",
            cap(&when(w, l.said)),
            clip(&l.words),
            if l.caught { ", and he caught you" } else { "" }
        ));
    }
    for x in m.watch.iter().filter(|x| x.by == him).take(1) {
        out.push(format!("{} you agreed to tell the coach when something happens", cap(&when(w, x.day))));
    }
}

/// His trust in the coach, in his own terms.
fn trust(w: &World, him: Them, kind: Kind, new: bool) -> Option<String> {
    if kind == Kind::Boss {
        let t = Team::seat(w).map_or(0.5, |t| boss::trust(w, t));
        return Some(
            match t {
                t if t >= 0.75 => "You're firmly behind the coach, and you say so in public",
                t if t >= 0.6 => "You're pleased with the coach",
                t if t >= 0.45 => "You're watching how it goes with the coach",
                t if t >= 0.3 => "You're losing patience with the coach, and the people around you know it",
                _ => "You want a change",
            }
            .into(),
        );
    }
    let t = match him {
        Them::Player(id) if kind == Kind::Player => folk::person(w, id).map_or(0.5, |i| w.people[i].trust as f64),
        _ => makeup::trust(w, him),
    };
    Some(match t {
        _ if new && (0.4..=0.7).contains(&t) => {
            "You don't know the coach well enough yet to trust him or not: you're still making up your mind".into()
        }
        t if t > 0.7 => "You trust the coach".into(),
        t if t > 0.45 => "You're still making up your mind about the coach".into(),
        _ => "You don't trust the coach".into(),
    })
}

/// The coach's name for keeping his word where he hears it: your room,
/// the recruiting trail, agents and clubs.
fn name(w: &World, kind: Kind) -> Option<String> {
    let (n, at) = match kind {
        Kind::Player => (Name::Room, "Around the team"),
        Kind::Recruit | Kind::Transfer | Kind::Family | Kind::HsCoach | Kind::Circle => {
            (Name::Trail, "On the recruiting trail")
        }
        Kind::Agent | Kind::Gm => (Name::Agents, "Among agents and clubs"),
        _ => return None,
    };
    match w.folk.mine.name(n) {
        x if x >= 0.35 => Some(format!("{at} the coach is known for keeping his word")),
        x if x <= -0.35 => Some(format!("{at} the coach is known for promises he doesn't keep")),
        _ => None,
    }
}

fn cap(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().chain(c).collect(),
        None => String::new(),
    }
}

fn lower_first(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_lowercase().chain(c).collect(),
        None => String::new(),
    }
}
