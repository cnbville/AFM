//! Your team's rooms (talk facts lane L9; items 18, 20, 21 and 26 of
//! `/afm/talk-facts/proposal.md`, section 9): the team talk and the first
//! team meeting, a parent's call, the exit meetings and the game day
//! words. Each reads the world here and hands its words real values, and
//! words an option only as the world backs it (rule 1 of section 4).
//!
//! - `week`: the team talk's week, from your team's sheet (`claims::team`):
//!   the last game while it is fresh, a streak, the next game, the starters
//!   out, the season that ended, and with no game near the next dated
//!   thing (spring practice, camp, the opener).
//! - `family`: what a player's family calls about and what it can see: his
//!   real complaint (his role, a punishment, his money, a position move),
//!   his group in practice as he tells it at home, and his snaps and starts
//!   in your last two games.
//! - `leaving`: why a man is on your exit list, from his sheet and your
//!   staff's own list after a takeover.
//!
//! Nothing here changes a number in the game: the readers only read.
//! Hidden numbers stay hidden (`claims/mod.rs`, rule 2): no reader here
//! reads one, so a line names only what the family, the room or your
//! staff can see.
//!
//! Every size in this module is Claude's (proposed, not confirmed).

use super::kinds::Kind;
use super::sheet::{Point, Ref};
use super::{Reader, words};
use crate::training::seat::{GROUP_APART, GROUP_ONES, GROUP_TWOS};
use crate::world::World;
use crate::world::folk::Team;
use crate::world::folk::voice::Facts;
use crate::world::people::Why;

// ---------------------------------------------------------------------------
// The week (item 18): the team talk and the first team meeting.

/// Your team's week as the room knows it. Every point is on your team's
/// sheet today (`claims::team`), so a game is named only while it is
/// fresh (rule 9), and `ahead` is filled only when no game is.
#[derive(Clone, Debug, Default)]
pub struct Week {
    /// The last game, for a week (`Kind::LastGame`).
    pub last: Option<Point>,
    /// Three or more won or lost in a row, while the last is fresh.
    pub streak: Option<Point>,
    /// The next game, from two weeks out.
    pub next: Option<Point>,
    /// Starters out hurt today.
    pub out: Option<Point>,
    /// The season that ended in the last month.
    pub ended: Option<Point>,
    /// With no game fresh and none near: the next dated thing.
    pub ahead: Option<Point>,
}

/// Your team's week today.
pub fn week(w: &World, team: Team) -> Week {
    let sheet = super::team(w, team);
    let get = |k: Kind| sheet.get(k).cloned();
    let mut wk = Week {
        last: get(Kind::LastGame),
        streak: get(Kind::Streak),
        next: get(Kind::NextGame),
        out: get(Kind::StartersOut),
        ended: get(Kind::SeasonEnded),
        ahead: None,
    };
    if wk.last.is_none() && wk.next.is_none() {
        let r = Reader::new(w);
        wk.ahead = super::season::team_ahead(&r, team, Team::seat(w) == Some(team));
    }
    wk
}

/// The team a point refers to (a game's opponent).
pub fn team_of(p: &Point) -> Option<Team> {
    p.refs.iter().find_map(|r| match r {
        Ref::Team(t) => Some(*t),
        _ => None,
    })
}

impl Week {
    /// The fresh game's result: win, loss or tie.
    pub fn result(&self) -> Option<&'static str> {
        self.last.as_ref().and_then(|p| p.check("result"))
    }
    pub fn won(&self) -> bool {
        self.result() == Some("win")
    }
    pub fn lost(&self) -> bool {
        self.result() == Some("loss")
    }
    /// The streak: winning or losing.
    pub fn run(&self) -> Option<&'static str> {
        self.streak.as_ref().and_then(|p| p.check("run"))
    }
    /// The next game is the rival.
    pub fn rival_week(&self) -> bool {
        self.next.as_ref().is_some_and(|p| p.is("rival", "yes"))
    }
    /// How the season that ended went: good, bad or even.
    pub fn season(&self) -> Option<&'static str> {
        self.ended.as_ref().and_then(|p| p.check("year"))
    }

    /// The week's holes and checks for a line:
    /// - the last game: `opp`, `score`, `when`, `result` (win, loss, tie),
    ///   `margin` (close, clear, big), `place`, and `game` for a bowl;
    /// - a streak: `run` ("three games") and `streak` (winning, losing);
    /// - the next game: `next`, `next_when`, `next_place`, `next_rival=yes`;
    /// - starters out: `names` and `starters` ("two starters");
    /// - the season that ended: `record`, `season_was` (good, bad, even),
    ///   `finish` for a title;
    /// - what is coming: `what` and `ahead_when`.
    pub fn fill(&self, f: &mut Facts) {
        if let Some(p) = &self.last {
            for (k, v) in p.values() {
                f.set(k, v.clone());
            }
            for (k, v) in p.states() {
                f.set(k, *v);
            }
        }
        if let Some(p) = &self.streak {
            if let Some(v) = p.get("run") {
                f.set("run", v.to_string());
            }
            if let Some(v) = p.check("run") {
                f.set("streak", v);
            }
        }
        if let Some(p) = &self.next {
            if let Some(v) = p.get("opp") {
                f.set("next", v.to_string());
            }
            if let Some(v) = p.get("when") {
                f.set("next_when", v.to_string());
            }
            if let Some(v) = p.check("place") {
                f.set("next_place", v);
            }
            if self.rival_week() {
                f.set("next_rival", "yes");
            }
        }
        if let Some(p) = &self.out {
            for k in ["names", "starters"] {
                if let Some(v) = p.get(k) {
                    f.set(k, v.to_string());
                }
            }
        }
        if let Some(p) = &self.ended {
            if let Some(v) = p.get("record") {
                f.set("record", v.to_string());
            }
            if let Some(v) = p.get("finish") {
                f.set("finish", v.to_string());
            }
            if let Some(v) = p.check("year") {
                f.set("season_was", v);
            }
        }
        if let Some(p) = &self.ahead {
            if let Some(v) = p.get("what") {
                f.set("what", v.to_string());
            }
            if let Some(v) = p.get("when") {
                f.set("ahead_when", v.to_string());
            }
        }
    }
}

// ---------------------------------------------------------------------------
// A parent's call (item 20): the real complaint, and what the family sees.

/// What a family calls about (section 9: "his role, a punishment, money,
/// a position move").
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Complaint {
    /// His role: a job he lost, being buried, or just not playing enough.
    Role,
    /// A suspension or a ruling of yours.
    Punishment,
    /// His money: paid under the going rate.
    Money,
    /// A position move he is still learning.
    Position,
}

impl Complaint {
    pub fn key(self) -> &'static str {
        match self {
            Complaint::Role => "role",
            Complaint::Punishment => "punishment",
            Complaint::Money => "money",
            Complaint::Position => "position",
        }
    }
}

/// One of your games as the family saw it: his snaps (every phase) and
/// whether he started.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Seen {
    pub snaps: u32,
    pub started: bool,
}

/// What his family calls about and what it can see.
#[derive(Clone, Debug)]
pub struct Family {
    pub complaint: Complaint,
    /// The point behind it: a job lost, buried, a suspension, a ruling,
    /// his pay, a new position. None for a plain role complaint.
    pub point: Option<Point>,
    /// His group in your newest practice week, as he tells it at home:
    /// ones, twos or apart.
    pub group: Option<&'static str>,
    /// Your last two games he was with you for and not hurt, newest first.
    pub games: Vec<Seen>,
    /// A punishment of yours as a noun ("the fine", "the two-game
    /// suspension"), for a punishment call.
    pub penalty: Option<String>,
}

/// Games of yours the family looks at.
pub const FAMILY_GAMES: usize = 2;

/// What his family would call you about today, if anything: the point
/// on his sheet most on his mind that a family sees (a job lost or being
/// buried, a punishment of yours, being paid under the going rate, a
/// position he is still learning), else his role when that is why he is
/// unhappy. Only for a player on your team.
pub fn family(w: &World, i: usize) -> Option<Family> {
    let team = Team::seat(w)?;
    if !team.roster(w).contains(&i) {
        return None;
    }
    let sheet = super::player(w, i);
    let penalty = penalty_of(w, w.people[i].player.id, team);
    let found = sheet.known().find_map(|p| {
        let c = match p.kind {
            Kind::LostJob | Kind::Buried => Complaint::Role,
            Kind::Suspended if p.is("by", "you") => Complaint::Punishment,
            // A warning or the council's word is nothing a family calls about.
            Kind::RuleBreak if p.is("ruled", "yes") && penalty.is_some() => Complaint::Punishment,
            Kind::Pay => Complaint::Money,
            Kind::NewPosition if !p.is("stage", "ready") => Complaint::Position,
            _ => return None,
        };
        Some((c, Some(p.clone())))
    });
    let (complaint, point) = match found {
        Some(x) => x,
        None if Why::from_u8(w.people[i].mood_why) == Why::Role => (Complaint::Role, None),
        None => return None,
    };
    let penalty = penalty.filter(|_| complaint == Complaint::Punishment);
    Some(Family { complaint, point, group: group_of(w, i), games: seen(w, team, i), penalty })
}

/// Your last ruling on him that a family would call about, as a noun ("the
/// fine", "the two-game suspension"): a fine and up, not a dismissal (he
/// is gone), not a warning or the council's word.
fn penalty_of(w: &World, id: u32, team: Team) -> Option<String> {
    use crate::world::folk::rules::Response as R;
    let r = w.folk.rules.rulings.iter().rev().find(|x| x.id == id)?;
    let nfl = team.level() == crate::calendar::Level::Nfl;
    Some(match r.response {
        R::Talk | R::Council | R::Dismiss => return None,
        R::Fine => "the fine".into(),
        R::Running => "the early-morning running".into(),
        R::BenchStart => "the benching".into(),
        R::BenchGame if nfl => "the deactivation".into(),
        R::BenchGame => "sitting him for a game".into(),
        R::Suspend(n) => format!("the {}-game suspension", crate::world::folk::count_words(n as usize)),
    })
}

/// His group in your newest practice week.
fn group_of(w: &World, i: usize) -> Option<&'static str> {
    let id = w.people[i].player.id;
    let (_, g) = w.training.groups.iter().rev().find(|x| x.0 <= w.today && x.1.iter().any(|y| y.0 == id))?;
    match g.iter().find(|y| y.0 == id)?.1 {
        GROUP_ONES => Some("ones"),
        GROUP_TWOS => Some("twos"),
        GROUP_APART => Some("apart"),
        _ => None,
    }
}

/// Your last games he was with you for and not hurt, newest first, as
/// the family saw them (a start: half the most snaps on his side of the
/// ball, as on his own sheet, `player::START_SHARE`).
fn seen(w: &World, team: Team, i: usize) -> Vec<Seen> {
    let p = &w.people[i];
    let id = p.player.id;
    let offense = words::on_offense(p.player.position);
    let since = w.training.arrived.iter().find(|x| x.0 == id).map(|x| x.1);
    let mut out = Vec::new();
    for g in w.seat_games.iter().rev().filter(|g| g.date <= w.today) {
        let Some(side) = super::side_of(g, team) else { continue };
        let line = g.stats.players[side].get(&id);
        let hurt = w.training.hurt.iter().any(|h| h.id == id && h.from <= g.date && g.date <= h.until);
        if line.is_none() && (hurt || since.is_some_and(|d| d > g.date)) {
            continue;
        }
        let side_snaps = |l: &crate::game::stats::StatLine| if offense { l.snaps_off } else { l.snaps_def };
        let most = g.stats.players[side].values().map(side_snaps).max().unwrap_or(0);
        let mine = line.map_or(0, side_snaps);
        out.push(Seen {
            snaps: line.map_or(0, |l| l.snaps_off + l.snaps_def + l.snaps_st),
            started: most > 0 && mine as f64 >= super::player::START_SHARE * most as f64,
        });
        if out.len() == FAMILY_GAMES {
            break;
        }
    }
    out
}

impl Family {
    /// The family's holes and checks: `complaint`; the point's own
    /// (`rival`, `ahead`, `rule`, `punishment`, `reason`, `position`,
    /// `stage`, `gap`, `by`, `why`...); `group` in words and `with` (ones,
    /// twos, apart); `penalty` ("the fine"); `snaps` ("eleven snaps"),
    /// `starts` ("no starts") and
    /// `seen` ("his last two games") when he was in a game of yours.
    pub fn fill(&self, f: &mut Facts) {
        f.set("complaint", self.complaint.key());
        if let Some(p) = &self.point {
            p.fill(f);
        }
        if let Some(x) = &self.penalty {
            f.set("penalty", x.clone());
        }
        if let Some(g) = self.group {
            f.set("with", g);
            f.set("group", group_words(g));
        }
        if !self.games.is_empty() {
            let snaps: u32 = self.games.iter().map(|g| g.snaps).sum();
            let starts = self.games.iter().filter(|g| g.started).count() as u32;
            f.set("snaps", words::counted(snaps, "snap"));
            f.set("starts", if starts == 0 { "no starts".to_string() } else { words::counted(starts, "start") });
            f.set(
                "seen",
                if self.games.len() == 1 { "his last game".to_string() } else { "his last two games".to_string() },
            );
        }
    }

    /// What the family can see, in your staff's words: "with the second
    /// team in practice, as he tells it; eleven snaps and no starts in his
    /// last two games."
    pub fn sees(&self) -> Option<String> {
        let mut parts: Vec<String> = Vec::new();
        if let Some(g) = self.group {
            parts.push(format!("{} in practice, as he tells it", group_place(g)));
        }
        if !self.games.is_empty() {
            let mut f = Facts::new();
            self.fill(&mut f);
            parts.push(format!(
                "{} and {} in {}",
                f.get("snaps").unwrap_or_default(),
                f.get("starts").unwrap_or_default(),
                f.get("seen").unwrap_or_default()
            ));
        }
        (!parts.is_empty()).then(|| format!("{}.", parts.join("; ")))
    }
}

/// A practice group the way a family says it.
pub fn group_words(g: &str) -> &'static str {
    match g {
        "ones" => "the first team",
        "twos" => "the second team",
        _ => "the young guys, outside the two-deep",
    }
}

fn group_place(g: &str) -> &'static str {
    match g {
        "ones" => "with the first team",
        "twos" => "with the second team",
        _ => "outside the two-deep",
    }
}

// ---------------------------------------------------------------------------
// Exit meetings (item 21): the real reason he is on your list.

/// Why a man is on your exit list, as the talk opens on it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Leaving {
    /// A point on his sheet you know: the draft, wanting out, the portal,
    /// a job lost, buried, his pay, a promise you broke, a friend who
    /// left, a new position coach, missing home, his eligibility.
    Point(Kind),
    /// Close to the coach you replaced (your staff's list at a takeover).
    OldCoach,
    /// Unhappy with his role, with no job lost or buried on his sheet
    /// (the same list).
    Role,
    /// Other programs are already calling (the same list).
    Courted,
    /// A pro who wants a new deal (the same list).
    Deal,
    /// He hasn't been happy for a while.
    Unhappy,
    /// His plans for next year are open: nothing more that you know.
    Plain,
}

/// The points that are a reason to leave, in no order: the sheet's own
/// order (what is most on his mind) picks among them.
pub const LEAVING_KINDS: [Kind; 12] = [
    Kind::Draft,
    Kind::WantsOut,
    Kind::Portal,
    Kind::LostJob,
    Kind::Buried,
    Kind::Pay,
    Kind::PromiseBroken,
    Kind::FriendLeft,
    Kind::NewCoach,
    Kind::Homesick,
    Kind::FailedTerm,
    Kind::Grades,
];

impl Leaving {
    /// Its code in a scene (`Scene.c`'s exit bits); a point's is 0, since
    /// the scene's topic holds the point.
    pub fn code(self) -> i64 {
        match self {
            Leaving::Point(_) | Leaving::Plain => 0,
            Leaving::OldCoach => 1,
            Leaving::Courted => 2,
            Leaving::Deal => 3,
            Leaving::Unhappy => 4,
            Leaving::Role => 5,
        }
    }
    pub fn of_code(c: i64) -> Leaving {
        match c {
            1 => Leaving::OldCoach,
            2 => Leaving::Courted,
            3 => Leaving::Deal,
            4 => Leaving::Unhappy,
            5 => Leaving::Role,
            _ => Leaving::Plain,
        }
    }
    /// The bank's part for his own opening when no point backs it.
    pub fn key(self) -> Option<&'static str> {
        match self {
            Leaving::OldCoach => Some("talk.exit.open.oldcoach"),
            Leaving::Courted => Some("talk.exit.open.courted"),
            Leaving::Deal => Some("talk.exit.open.deal"),
            Leaving::Unhappy => Some("talk.exit.open.unhappy"),
            _ => None,
        }
    }
    /// Your staff's note for a reason no point carries.
    pub fn words(self) -> &'static str {
        match self {
            Leaving::OldCoach => "close to the coach you replaced",
            Leaving::Courted => "other programs are already calling",
            Leaving::Deal => "wants a new deal",
            Leaving::Unhappy => "hasn't been happy for a while",
            Leaving::Role => "unhappy with his role",
            _ => "his plans for next year are open",
        }
    }
}

/// Why he is on your exit list: the point on his sheet you know that is
/// most on his mind among the reasons to leave, else the first reason on
/// your staff's list (`staff`, a takeover's flight list: "close to the
/// old coach, other programs are already calling"; it leads with its
/// biggest reason), else unhappiness you can see, else his open plans.
pub fn leaving(w: &World, i: usize, staff: Option<&str>) -> Leaving {
    let sheet = super::player(w, i);
    if let Some(p) = sheet.known().find(|p| LEAVING_KINDS.contains(&p.kind)) {
        return Leaving::Point(p.kind);
    }
    // Your staff's list leads with its biggest reason.
    let first = staff.and_then(|x| x.split(", ").next()).unwrap_or("");
    for (says, why) in [
        ("old coach", Leaving::OldCoach),
        ("role", Leaving::Role),
        ("other programs", Leaving::Courted),
        ("new deal", Leaving::Deal),
        ("happy", Leaving::Unhappy),
    ] {
        if first.contains(says) {
            return why;
        }
    }
    if w.people[i].mood < UNHAPPY {
        return Leaving::Unhappy;
    }
    Leaving::Plain
}

/// A mood this low is unhappiness his coaches can see (the exit list's
/// own bar, `talks::exit_meetings`).
pub const UNHAPPY: f32 = -0.2;

// ---------------------------------------------------------------------------
// The first team meeting (item 18): the season before you and the man you
// replaced.

pub use super::team::{BAD_YEAR, GOOD_YEAR};

/// What the room knows on your first day: the last season filed (its
/// year, record and any title), how it went, the coach you replaced and
/// why he left, and the next dated thing.
#[derive(Clone, Debug, Default)]
pub struct Arrival {
    pub year: Option<i32>,
    pub record: Option<String>,
    /// good, bad or even.
    pub season_was: Option<&'static str>,
    pub finish: Option<String>,
    /// The coach you replaced and why he left ("he was fired").
    pub old: Option<(usize, &'static str)>,
    pub ahead: Option<Point>,
}

pub fn arrival(w: &World, team: Team) -> Arrival {
    let mut a = Arrival::default();
    if let Some(s) = crate::world::folk::facts::seasons(w, team, 1).last() {
        let share = s.wins as f64 / (s.wins + s.losses).max(1) as f64;
        a.year = Some(s.year);
        a.record = Some(format!("{}-{}", s.wins, s.losses));
        a.season_was = Some(if share >= GOOD_YEAR {
            "good"
        } else if share < BAD_YEAR {
            "bad"
        } else {
            "even"
        });
        a.finish = (!s.note.is_empty()).then(|| s.note.clone());
    }
    let you = w.career.as_ref().map(|c| c.coach);
    if let (Some(o), Some(you)) = (w.folk.takeover.as_ref().and_then(|t| t.old_coach), you) {
        let why = crate::world::folk::facts::predecessor(w, team, you)
            .filter(|x| x.0 == o)
            .map_or("he moved on", |x| crate::world::folk::facts::why_left(&x.1));
        a.old = Some((o, why));
    }
    let r = Reader::new(w);
    a.ahead = super::season::team_ahead(&r, team, Team::seat(w) == Some(team));
    a
}

impl Arrival {
    /// `last_record`, `last_year`, `season_was`, `finish`, `why_left`,
    /// `what` and `ahead_when`.
    pub fn fill(&self, f: &mut Facts) {
        if let Some(r) = &self.record {
            f.set("last_record", r.clone());
        }
        if let Some(y) = self.year {
            f.set("last_year", y.to_string());
        }
        if let Some(s) = self.season_was {
            f.set("season_was", s);
        }
        if let Some(x) = &self.finish {
            f.set("finish", x.clone());
        }
        if let Some((_, why)) = self.old {
            f.set("why_left", why);
        }
        if let Some(p) = &self.ahead {
            if let Some(v) = p.get("what") {
                f.set("what", v.to_string());
            }
            if let Some(v) = p.get("when") {
                f.set("ahead_when", v.to_string());
            }
        }
    }
}
