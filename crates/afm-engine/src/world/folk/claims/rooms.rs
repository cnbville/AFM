//! The staff and scouting rooms (talk facts lane L11; items 19, 23, 24 and
//! 25 of `/afm/talk-facts/proposal.md`, section 9). Four sets of trees that
//! are not a player across a desk: your job interview and the weekly staff
//! meeting, your staff's rooms (a holdover, a candidate, a firing, a
//! proposal), the combine and visit rooms, and the agents. Each reads the
//! world here and hands its words real values, and offers an option only
//! when the world backs it (rule 1 of section 4).
//!
//! Nothing here changes a number in the game: the readers only read, and
//! the trees keep every effect they had. Hidden numbers stay hidden
//! (`claims/mod.rs`, rule 2): a hidden read is a gate, never a value.
//!
//! Every size in this module is Claude's (proposed, not confirmed).

use super::words;
use crate::world::background::{Bloodline, FamilyStructure, ParentStyle};
use crate::world::coach::{Employer, Group, Role};
use crate::world::folk::voice::Facts;
use crate::world::folk::{Team, facts};
use crate::world::seat::SeatGame;
use crate::world::{Stage, World};

// ---------------------------------------------------------------------------
// Your job interview (item 19): your record, and the chair you apply for.

/// Games a record needs before you can point to it (or be asked about it):
/// a record of five games is a few results, not a body of work.
/// Proposed, not confirmed.
pub const RESULTS_GAMES: u32 = 8;

/// One of your seasons as a head coach.
#[derive(Clone, Debug, PartialEq)]
pub struct Year {
    pub year: i32,
    pub wins: u32,
    pub losses: u32,
    pub team: String,
}

/// One of your guys drafted: his name, the round and the year.
#[derive(Clone, Debug, PartialEq)]
pub struct Drafted {
    pub name: String,
    pub round: u32,
    pub year: i32,
}

/// What a committee can see of you: your seasons, your record, and the
/// players who came through your rooms and were drafted.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Past {
    /// Your seasons as a head coach, oldest first.
    pub head: Vec<Year>,
    /// Your record where you led a program, or on every staff you were on
    /// when you never did (`as_head` says which).
    pub wins: u32,
    pub losses: u32,
    pub as_head: bool,
    /// A stretch on a staff that went badly, when you never led a program.
    pub stretch: Option<(u32, u32, String)>,
    /// Your guys who were drafted, the best pick first.
    pub drafted: Vec<Drafted>,
}

/// Your record as the committee reads it.
pub fn past(w: &World) -> Past {
    let Some(me) = w.career.as_ref().map(|c| c.coach).filter(|&c| c < w.coaches.len()) else {
        return Past::default();
    };
    let co = &w.coaches[me];
    let mut leads: Vec<Employer> =
        co.history.iter().filter(|s| s.role == Role::HeadCoach).map(|s| s.employer).collect();
    if let Some(j) = co.job.filter(|j| j.role == Role::HeadCoach) {
        leads.push(j.employer);
    }
    let mut seen: Vec<Employer> = Vec::new();
    let mut head = Vec::new();
    for e in leads {
        if seen.contains(&e) {
            continue;
        }
        seen.push(e);
        let team = Team::from_employer(e);
        let name = team.name(w);
        for s in facts::seasons(w, team, 60) {
            if s.coach == Some(me) {
                head.push(Year { year: s.year, wins: s.wins, losses: s.losses, team: name.clone() });
            }
        }
    }
    head.sort_by_key(|y| y.year);
    let mut p = Past { head, ..Past::default() };
    if p.head.is_empty() {
        // Never led a program: the committee reads the staffs you were on.
        let mut stints: Vec<(u32, u32, String)> = co
            .history
            .iter()
            .map(|s| (s.wins, s.losses, Team::from_employer(s.employer).name(w)))
            .chain(co.job.map(|j| (co.wins, co.losses, Team::from_employer(j.employer).name(w))))
            .filter(|s| s.0 + s.1 > 0)
            .collect();
        p.wins = stints.iter().map(|s| s.0).sum();
        p.losses = stints.iter().map(|s| s.1).sum();
        stints.retain(|s| s.1 > s.0 && s.0 + s.1 >= RESULTS_GAMES);
        stints.sort_by(|a, b| (a.0 as f64 / (a.0 + a.1) as f64).total_cmp(&(b.0 as f64 / (b.0 + b.1) as f64)));
        p.stretch = stints.into_iter().next();
    } else {
        p.as_head = true;
        p.wins = p.head.iter().map(|y| y.wins).sum();
        p.losses = p.head.iter().map(|y| y.losses).sum();
    }
    let mut guys: Vec<Drafted> = w
        .legacy
        .guys
        .iter()
        .filter_map(|g| {
            let (year, round, ..) = g.pick?;
            Some(Drafted { name: g.name.clone(), round: round as u32, year })
        })
        .collect();
    guys.sort_by_key(|d| (d.round, d.year));
    p.drafted = guys;
    p
}

impl Past {
    /// Games on the record.
    pub fn games(&self) -> u32 {
        self.wins + self.losses
    }

    /// The record in words: "41-19 over five seasons", "9-3 in 2031", or
    /// "30-22 on the staffs you've been on" ("I've" when `mine`: your own
    /// answer). None with no games.
    pub fn record_words(&self, mine: bool) -> Option<String> {
        if self.games() == 0 {
            return None;
        }
        Some(if self.as_head {
            match self.head.as_slice() {
                [y] => format!("{}-{} in {}", y.wins, y.losses, y.year),
                _ => format!("{}-{} over {}", self.wins, self.losses, words::counted(self.head.len() as u32, "season")),
            }
        } else {
            format!("{}-{} on the staffs {} been on", self.wins, self.losses, if mine { "I've" } else { "you've" })
        })
    }

    /// Whether you can point to results: enough games, and more won than
    /// lost. An answer that points to results is offered only then.
    pub fn strong(&self) -> bool {
        self.games() >= RESULTS_GAMES && self.wins > self.losses
    }

    /// The year it went wrong, in words ("the 3-9 season in 2031"): your
    /// worst losing season as a head coach, or the worst stretch on a
    /// staff. An answer that owns a failure is offered only when there is
    /// one.
    pub fn failure(&self) -> Option<String> {
        let worst = self
            .head
            .iter()
            .filter(|y| y.losses > y.wins)
            .min_by(|a, b| {
                let f = |y: &Year| y.wins as f64 / (y.wins + y.losses).max(1) as f64;
                f(a).total_cmp(&f(b)).then(b.year.cmp(&a.year))
            })
            .map(|y| format!("the {}-{} season in {}", y.wins, y.losses, y.year));
        worst.or_else(|| self.stretch.as_ref().map(|(w, l, team)| format!("the {w}-{l} stretch at {team}")))
    }

    /// The pick to name: your best drafted player. An answer that points to
    /// the people is offered only when you have one.
    pub fn people(&self) -> Option<&Drafted> {
        self.drafted.first()
    }

    /// Puts what is true of your past into a line's facts: `yours` (the
    /// record), `strong=yes` when you can point to it, `weak=yes` when there
    /// are enough games and it lost more than it won (a short or an even
    /// record is neither), `worst` (the year it
    /// went wrong), `pick_name`, `pick_round`, `pick_year`.
    pub fn fill(&self, f: &mut Facts) {
        if let Some(r) = self.record_words(false) {
            f.set("yours", r);
            if self.strong() {
                f.set("strong", "yes");
            } else if self.games() >= RESULTS_GAMES && self.losses > self.wins {
                // Enough games to judge it, and it lost more than it won.
                f.set("weak", "yes");
            }
        }
        if let Some(x) = self.failure() {
            f.set("worst", x);
        }
        if let Some(d) = self.people() {
            f.set("pick_name", d.name.clone());
            f.set("pick_round", words::round_word(d.round));
            f.set("pick_year", d.year.to_string());
        }
    }
}

/// The chair you apply for: its name, and its last finished season with
/// `form=up|down` (a winning record or not).
pub fn chair(w: &World, team: Team, f: &mut Facts) {
    f.set("team", team.name(w));
    if let Some(s) = facts::seasons(w, team, 1).last() {
        f.set("last_record", format!("{}-{}", s.wins, s.losses));
        f.set("last_year", s.year.to_string());
        f.set("form", if s.wins > s.losses { "up" } else { "down" });
    }
}

// ---------------------------------------------------------------------------
// The weekly staff meeting (item 19): the numbers behind the case.

/// The numbers a coordinator brings out of the game he is arguing from.
#[derive(Clone, Debug, PartialEq)]
pub struct Case {
    /// The other side's name.
    pub opp: String,
    /// The score, yours first.
    pub score: [i32; 2],
    /// Points your defense gave up, and your offense scored.
    pub allowed: i32,
    pub scored: i32,
    /// How often your front got to their passer, in whole percent of their
    /// dropbacks.
    pub pressure: u32,
    /// Explosive plays they had (the game's count: a play of 20 yards or
    /// more, or a run of 10 or more).
    pub big: u32,
    /// Your yards a carry, to one decimal, and your share of runs among
    /// your plays, in whole percent.
    pub ypc: f64,
    pub run_share: u32,
}

/// The game's numbers from your side (the same ones the coordinator's case
/// is made from: `dialogue.rs`, `case_from`).
pub fn case(g: &SeatGame, me: usize) -> Case {
    let side = if g.teams[0] == me { 0 } else { 1 };
    let them = 1 - side;
    let st = &g.stats;
    let (ours, theirs) = (&st.team[side], &st.team[them]);
    let carries = ours.plays.saturating_sub(ours.dropbacks).max(1) as f64;
    Case {
        opp: g.names[them].clone(),
        score: [st.score[side], st.score[them]],
        allowed: st.score[them],
        scored: st.score[side],
        pressure: (100.0 * theirs.pressures as f64 / theirs.dropbacks.max(1) as f64).round() as u32,
        big: theirs.explosive,
        ypc: (10.0 * ours.rush_yards / carries).round() / 10.0,
        run_share: (100.0 * carries / ours.plays.max(1) as f64).round() as u32,
    }
}

impl Case {
    /// Whether your side won it.
    pub fn won(&self) -> bool {
        self.score[0] > self.score[1]
    }

    /// Puts the game into a line's facts: `opp`, `score`, `result`,
    /// `allowed`, `scored`, `pressure`, `big`, `ypc`, `run_share`.
    pub fn fill(&self, f: &mut Facts) {
        f.set("opp", self.opp.clone());
        f.set("score", format!("{}-{}", self.score[0], self.score[1]));
        f.set(
            "result",
            match self.score[0].cmp(&self.score[1]) {
                std::cmp::Ordering::Greater => "win",
                std::cmp::Ordering::Less => "loss",
                std::cmp::Ordering::Equal => "tie",
            },
        );
        f.set("allowed", self.allowed.to_string());
        f.set("scored", self.scored.to_string());
        f.set("pressure", self.pressure.to_string());
        f.set("big", self.big.to_string());
        f.set("ypc", format!("{:.1}", self.ypc));
        f.set("run_share", self.run_share.to_string());
    }
}

// ---------------------------------------------------------------------------
// Your staff's rooms (item 23): a coach's real record, his unit, the players.

/// Points a team scores in a game at each level, the middle of the real
/// numbers (`data/real-world`): what a unit is judged against.
/// Proposed, not confirmed.
pub fn usual_points(team: Team) -> f64 {
    match team {
        Team::Nfl(_) | Team::Other(_) => USUAL_POINTS_PRO,
        Team::College(_) => USUAL_POINTS_COLLEGE,
        Team::High(_) => USUAL_POINTS_HIGH,
    }
}

/// Points a team scores a game in the pros, in college and in high school
/// (the middle of the real numbers). Proposed, not confirmed.
pub const USUAL_POINTS_PRO: f64 = 22.5;
pub const USUAL_POINTS_COLLEGE: f64 = 28.0;
pub const USUAL_POINTS_HIGH: f64 = 27.0;

/// The games of a team's latest season on file are the ones within this many
/// days of the last. Proposed, not confirmed.
pub const SEASON_DAYS: i32 = 150;

/// Games a unit's points need before they say anything. Proposed, not
/// confirmed.
pub const UNIT_GAMES: usize = 3;

/// How strongly a high school tie must run before a coach's file names the
/// school, and how many schools it names. Proposed, not confirmed.
pub const TIE_BAR: f64 = 0.4;
pub const TIES_NAMED: usize = 2;

/// How far back practice one-on-ones are read (days), and how many a man
/// needs before his record in them is worth saying. Proposed, not confirmed.
pub const REPS_DAYS: i32 = 28;
pub const REPS_MIN: u32 = 4;

/// How many of the latest staff notes are read for a trainer's load note,
/// and how many days one is good for. Proposed, not confirmed.
pub const NOTES_READ: usize = 200;
pub const LOAD_DAYS: i32 = 14;

/// How far from the usual a unit's points a game must be to count as good
/// or poor, as a share. Proposed, not confirmed.
pub const UNIT_SWING: f64 = 0.15;

/// How far a position coach's players growing against the norm must be to
/// count (the visible teaching record: rating points a player-season).
/// Proposed, not confirmed.
pub const TEACHING_BAR: f64 = 0.5;

/// How a coach's unit has done this year.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Form {
    Good,
    Middling,
    Poor,
}

/// A coach on your staff, as the building sees him.
#[derive(Clone, Debug, PartialEq)]
pub struct Unit {
    pub coach: usize,
    pub role: Role,
    /// Seasons with you, counting this one.
    pub years: i32,
    /// The team's record while he has held the job.
    pub wins: u32,
    pub losses: u32,
    /// Points a game his unit scored (an offense) or gave up (a defense)
    /// in the games on file, and how many games that is. None for a
    /// position coach or with fewer than three games.
    pub points: Option<(f64, u32)>,
    /// How he has done, from his unit's points or, for a position coach, his
    /// visible teaching record. None with nothing to go on.
    pub form: Option<Form>,
}

/// The games of your team's latest season on file: the ones within 150
/// days of the last.
fn latest_games(w: &World) -> Vec<&SeatGame> {
    let Some(last) = w.seat_games.last().map(|g| g.date) else { return Vec::new() };
    w.seat_games.iter().filter(|g| g.date.days_to(last) <= SEASON_DAYS && g.date <= last).collect()
}

/// Points a game your side scored and gave up in the games on file.
fn points_of(w: &World) -> Option<(f64, f64, u32)> {
    let (_, me) = w.seat_team()?;
    let games = latest_games(w);
    if games.len() < UNIT_GAMES {
        return None;
    }
    let (mut f, mut a) = (0.0, 0.0);
    for g in &games {
        let side = if g.teams[0] == me { 0 } else { 1 };
        f += g.stats.score[side] as f64;
        a += g.stats.score[1 - side] as f64;
    }
    let n = games.len() as f64;
    Some((f / n, a / n, games.len() as u32))
}

/// A coach on your staff, as the building sees him.
pub fn unit(w: &World, c: usize) -> Option<Unit> {
    let co = w.coaches.get(c)?;
    let job = co.job?;
    let team = Team::from_employer(job.employer);
    let (for_, against, n) = points_of(w).map_or((None, None, 0), |(f, a, n)| (Some(f), Some(a), n));
    let usual = usual_points(team);
    let by_points = |scored: bool| -> (Option<(f64, u32)>, Option<Form>) {
        let p = if scored { for_ } else { against };
        match p {
            None => (None, None),
            Some(x) => {
                let hi = x >= usual * (1.0 + UNIT_SWING);
                let lo = x <= usual * (1.0 - UNIT_SWING);
                let form = match (scored, hi, lo) {
                    (true, true, _) | (false, _, true) => Form::Good,
                    (true, _, true) | (false, true, _) => Form::Poor,
                    _ => Form::Middling,
                };
                (Some((x, n)), Some(form))
            }
        }
    };
    let (points, form) = match job.role {
        Role::OffensiveCoordinator => by_points(true),
        Role::DefensiveCoordinator => by_points(false),
        Role::PositionCoach(g) if g != Group::Specialists => {
            let t = co.units[g as usize].growth_read();
            (
                None,
                Some(if t > TEACHING_BAR {
                    Form::Good
                } else if t < -TEACHING_BAR {
                    Form::Poor
                } else {
                    Form::Middling
                }),
            )
        }
        _ => (None, None),
    };
    let (wins, losses) = record_since(w, team, job.first_season());
    Some(Unit { coach: c, role: job.role, years: (w.year - job.first_season() + 1).max(1), wins, losses, points, form })
}

/// The team's record since a season began: its finished seasons from then
/// on, and this season's games so far (not counted twice once the season's
/// row is in). A coach's own wins and losses are a head coach's alone, so an
/// assistant's record with you is the team's.
fn record_since(w: &World, team: Team, first: i32) -> (u32, u32) {
    let seasons = facts::seasons(w, team, 60);
    let mut r = (0, 0);
    for s in seasons.iter().filter(|s| s.year >= first) {
        r.0 += s.wins;
        r.1 += s.losses;
    }
    if !seasons.iter().any(|s| s.year >= w.year) {
        let now = facts::record_now(w, team);
        r = (r.0 + now.0, r.1 + now.1);
    }
    r
}

impl Unit {
    /// Whether his results back "it's the results" (a poor unit).
    pub fn poor(&self) -> bool {
        self.form == Some(Form::Poor)
    }

    /// His unit in words ("the defense gave up 31 a game over 9 games"),
    /// None when there is nothing to say.
    pub fn unit_words(&self) -> Option<String> {
        let (x, n) = self.points?;
        let x = x.round() as i64;
        Some(match self.role {
            Role::OffensiveCoordinator => format!("The offense scored {x} a game over {n} games."),
            _ => format!("The defense gave up {x} a game over {n} games."),
        })
    }

    /// His record with you in words ("the team is 19-17 since he took the
    /// job"). None before a game was played.
    pub fn record_words(&self) -> Option<String> {
        (self.wins + self.losses > 0).then(|| format!("{}-{}", self.wins, self.losses))
    }

    /// Puts him into a line's facts: `record` (the team's while he held the
    /// job), `unit` (his unit's year in words) and `form=good|poor` when
    /// there is a read. The tree sets `role` and `years` its own way.
    pub fn fill(&self, f: &mut Facts) {
        if let Some(r) = self.record_words() {
            f.set("record", r);
        }
        if let Some(u) = self.unit_words() {
            f.set("unit", u);
        }
        match self.form {
            Some(Form::Good) => {
                f.set("form", "good");
            }
            Some(Form::Poor) => {
                f.set("form", "poor");
            }
            _ => {}
        }
    }
}

impl Unit {
    /// What the building sees of his unit this year, in a sentence: his
    /// side's points a game, or for a position coach what his players'
    /// growth says. None when there is nothing to say.
    pub fn year_words(&self) -> Option<String> {
        self.unit_words().or_else(|| match (self.role, self.form) {
            (Role::PositionCoach(_), Some(Form::Good)) => Some("His players have grown more than most.".into()),
            (Role::PositionCoach(_), Some(Form::Poor)) => Some("His room hasn't grown the way it should.".into()),
            _ => None,
        })
    }
}

/// A coach you are interviewing, as the file has him: the record of his
/// teams where he works or last worked, and the high schools he knows best.
#[derive(Clone, Debug, PartialEq)]
pub struct Candidate {
    /// "his teams are 19-17 at Oregon State since 2029", "his teams went
    /// 12-12 at Rice from 2027 to 2028".
    pub record: Option<String>,
    /// The two schools his ties run strongest to.
    pub ties: Vec<String>,
}

pub fn candidate(w: &World, c: usize) -> Candidate {
    let co = &w.coaches[c];
    let record = match co.job {
        Some(j) if co.wins + co.losses > 0 => Some(format!(
            "his teams are {}-{} at {} since {}",
            co.wins,
            co.losses,
            Team::from_employer(j.employer).name(w),
            j.first_season()
        )),
        _ => co.history.iter().rev().find(|s| s.wins + s.losses > 0).map(|s| {
            format!(
                "his teams went {}-{} at {} from {} to {}",
                s.wins,
                s.losses,
                Team::from_employer(s.employer).name(w),
                s.from,
                s.to
            )
        }),
    };
    let mut ties: Vec<(usize, f64)> = co.ties.iter().map(|(&h, &v)| (h, v)).filter(|x| x.1 > TIE_BAR).collect();
    ties.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
    Candidate {
        record,
        ties: ties
            .into_iter()
            .take(TIES_NAMED)
            .filter(|x| x.0 < w.high_schools.len())
            .map(|x| w.school_name(x.0).to_string())
            .collect(),
    }
}

/// Where a player is in his career, ready to follow "is": "a sophomore", "a
/// redshirt junior", "a rookie", "in his second year", "a 4-year veteran".
pub fn class_phrase(w: &World, i: usize) -> String {
    let c = facts::class_words(w, i);
    if c.starts_with("a ") || c.starts_with("in ") { c } else { format!("a {c}") }
}

/// The player who starts for you at a group, by the depth chart your team
/// would take into a game today: his place on the people list. None while
/// your team can't dress a game team.
pub fn starter_in(w: &World, g: Group) -> Option<usize> {
    let depth = w.seat_game_depth()?;
    for (pos, ids) in &depth {
        if Group::of(*pos) == g
            && let Some(&id) = ids.first()
            && let Some(i) = crate::world::folk::person(w, id)
        {
            return Some(i);
        }
    }
    None
}

/// A man's one-on-ones in practice over the last `REPS_DAYS`: (won, lost).
pub fn reps(w: &World, id: u32) -> (u32, u32) {
    let (mut won, mut lost) = (0u32, 0u32);
    for l in w.training.log.iter().filter(|l| l.monday <= w.today && l.monday.days_to(w.today) <= REPS_DAYS) {
        for &(a, b, wins, losses) in &l.battles {
            // (a, b, a's wins, a's losses)
            if a == id {
                won += wins as u32;
                lost += losses as u32;
            } else if b == id {
                won += losses as u32;
                lost += wins as u32;
            }
        }
    }
    (won, lost)
}

/// How a young player and the veteran he is said to have passed have done in
/// practice one-on-ones lately (each against whoever he faced: the log pairs
/// an offensive man with a defensive one, never two of a group).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Duel {
    pub young: (u32, u32),
    pub vet: (u32, u32),
}

impl Duel {
    /// Whether the young player has won a bigger share than the veteran.
    pub fn young_ahead(&self) -> bool {
        let (y, v) = (self.young, self.vet);
        y.0 * (v.0 + v.1) > v.0 * (y.0 + y.1)
    }
}

/// The two men's practice records, when each has at least `REPS_MIN`
/// one-on-ones in the window. None otherwise.
pub fn duel(w: &World, young: u32, vet: u32) -> Option<Duel> {
    let (y, v) = (reps(w, young), reps(w, vet));
    (y.0 + y.1 >= REPS_MIN && v.0 + v.1 >= REPS_MIN).then_some(Duel { young: y, vet: v })
}

/// Whether your trainers wrote his load up in the last `LOAD_DAYS`.
pub fn load_note(w: &World, id: u32) -> bool {
    w.training
        .notes
        .iter()
        .rev()
        .take(NOTES_READ)
        .any(|n| n.id == id && n.dept == "trainers" && n.text.contains("load") && n.date.days_to(w.today) <= LOAD_DAYS)
}

// ---------------------------------------------------------------------------
// The combine and visit rooms (item 24): his real incident and family.

/// The trouble he was in, from his own story.
#[derive(Clone, Debug, PartialEq)]
pub struct Incident {
    /// What it was, with its article: "a DUI", "failing grades".
    pub kind: String,
    /// His age then.
    pub age: u32,
}

/// The last off-field trouble on his story (`season.rs` writes "Off-field
/// trouble at 20: a DUI."). None when his story names none.
pub fn incident(w: &World, i: usize) -> Option<Incident> {
    let p = w.people.get(i)?;
    p.notes.iter().rev().find_map(|n| {
        let rest = n.strip_prefix("Off-field trouble at ")?;
        let (age, kind) = rest.split_once(": ")?;
        Some(Incident { age: age.trim().parse().ok()?, kind: kind.trim_end_matches('.').to_string() })
    })
}

/// His family as the scouting file always has it: who raised him, a father
/// who played, a brother at a school.
#[derive(Clone, Debug, PartialEq)]
pub struct Home {
    pub structure: FamilyStructure,
    pub bloodline: Bloodline,
    /// The program his older brother plays for, and whether he means to
    /// follow him. Only while he is still choosing a school or at one.
    pub brother: Option<(String, bool)>,
    /// His parents are not around his football.
    pub parents_absent: bool,
}

pub fn home(w: &World, i: usize) -> Option<Home> {
    let p = w.people.get(i)?;
    let f = p.family();
    Some(Home {
        structure: f.structure,
        bloodline: f.bloodline,
        brother: f
            .older_brother_at
            .filter(|&b| {
                b < w.programs.len() && matches!(p.stage, Stage::HighSchool | Stage::PrepSchool | Stage::College)
            })
            .map(|b| (w.program_name(b).to_string(), f.follows_brother)),
        parents_absent: f.parent_style == ParentStyle::Absent,
    })
}

/// The room's facts for a line (`incident`, `age_then`) and the family
/// words the answers use.
pub fn room_facts(w: &World, i: usize) -> (Option<Incident>, Option<Home>) {
    let p = &w.people[i];
    // A player who has left football still has his story, but no room.
    if p.stage == Stage::Done {
        return (None, None);
    }
    (incident(w, i), home(w, i))
}

// ---------------------------------------------------------------------------
// The agents (item 25): what his client wants besides money.

/// How strongly a want must pull for his agent to name it. Proposed, not
/// confirmed.
pub const WANT_BAR: f64 = 0.6;

/// What a client wants besides money.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Want {
    Role,
    Winning,
    City,
    Security,
    Home,
    Loyalty,
}

impl Want {
    pub fn key(self) -> &'static str {
        match self {
            Want::Role => "role",
            Want::Winning => "winning",
            Want::City => "city",
            Want::Security => "security",
            Want::Home => "home",
            Want::Loyalty => "loyalty",
        }
    }
}

/// The want his agent leads with besides money: the strongest one at or
/// over the bar. Loyalty counts only in a talk with his own club. None when
/// nothing but the money pulls him.
pub fn top_want(w: &World, i: usize, own_club: bool) -> Option<Want> {
    // hidden: his goals are gates only; the agent says the want, never its size.
    let g = crate::world::pro::market::goals(w, i);
    let mut v = vec![
        (Want::Role, g.role),
        (Want::Winning, g.winning),
        (Want::City, g.city),
        (Want::Security, g.security),
        (Want::Home, g.home),
    ];
    if own_club {
        v.push((Want::Loyalty, g.loyalty));
    }
    v.into_iter().filter(|x| x.1 >= WANT_BAR).max_by(|a, b| a.1.total_cmp(&b.1)).map(|x| x.0)
}

/// What an agent can say about your club next to a want: the man ahead of
/// his client on your chart (role), and your club's last record
/// (winning). Put into a line's facts as `ahead`, `last_record` and
/// `form=up|down`.
pub fn club_facts(w: &World, i: usize, club: usize, want: Want, f: &mut Facts) {
    match want {
        // The chart is the one your seat plays from: only your own club's.
        Want::Role if w.seat.and_then(|s| s.nfl_team) == Some(club) => {
            let me = w.people[i].player.id;
            let pos = w.people[i].player.position;
            let ahead = w
                .seat_game_depth()
                .and_then(|d| d.into_iter().find(|(p, _)| *p == pos))
                .and_then(|(_, ids)| ids.first().copied())
                .filter(|&id| id != me)
                .and_then(|id| crate::world::folk::person(w, id));
            if let Some(a) = ahead {
                f.set("ahead", w.people[a].name.clone());
            }
        }
        Want::Winning => {
            if let Some(s) = facts::seasons(w, Team::Nfl(club), 1).last() {
                f.set("last_record", format!("{}-{}", s.wins, s.losses));
                f.set("form", if s.wins > s.losses { "up" } else { "down" });
            }
        }
        _ => {}
    }
}
