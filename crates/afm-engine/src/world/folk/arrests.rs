//! Arrests and court days come to you (talk facts lane L8, item 14; the
//! design's section 9, "Discipline"): when one of your players is
//! arrested, you rule the next morning, after he has said his piece; when
//! his case is over, the court's outcome comes back to you.
//!
//! What a level really does. A college program suspends a player
//! indefinitely while the case runs, or sits him a game for a DUI, or
//! dismisses him; a high school holds him out under its athletic code (a
//! share of the season) or until the case is settled; an NFL club leaves an
//! arrest to the league's personal conduct policy, or deactivates him, or
//! releases him (the league's own ruling and exempt list are in
//! `world/people.rs`).
//!
//! The ruling lands as a team-rule ruling does (`rules::carry_out`): fair
//! against what the charge and his record call for, consistent with how you
//! ruled on the others. Whether he did it is the case's hidden fact: only he
//! knows it, so a man who didn't do it takes the same ruling harder, and
//! nothing on screen shows it. On the court day the outcome is public, and
//! what you do then is weighed against the games he has already sat.
//!
//! Sizes are Claude's (proposed, not confirmed).

use super::rules::{self, Over, Response};
use super::scene::{Kind, PickKind, Scene};
use super::talks::{self, Reason};
use super::{Team, player_link};
use crate::calendar::{Date, Level};
use crate::world::World;

/// `Scene.b` of the morning after an arrest (a team-rule break keeps the
/// rule's index, 0-9).
pub const ARREST: u64 = 1000;
/// `Scene.b` of a case's court day.
pub const COURT: u64 = 1001;
/// An indefinite suspension holds this long at most: the longest case runs
/// about 230 days (`courts::step`), and the court day ends it before then.
pub const INDEFINITE_DAYS: i32 = 400;
/// A cleared man taken back: relief (mood).
pub const CLEARED_RELIEF: f32 = 0.06;
/// The room's trust when a man you dismissed over a charge is cleared: they
/// saw you move first (each teammate).
pub const ROOM_WRONG_DISMISS: f32 = -0.01;

/// Your ruling on an arrest, or on its court day.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct ArrestRuling {
    pub id: u32,
    /// The case's day (the courts' `Case.opened`): its key.
    pub case: Date,
    pub charge: String,
    pub response: Response,
    pub date: Date,
    /// A starter or a star at the time.
    pub star: bool,
    /// On the court day (else the morning after the arrest).
    pub court: bool,
}

/// How a case ended (`Scene.f[0]` of a court day).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Dropped,
    Acquitted,
    Pleaded,
    Convicted,
}

impl Outcome {
    fn code(self) -> f32 {
        self as u8 as f32
    }
    fn of(x: f32) -> Outcome {
        match x as u8 {
            0 => Outcome::Dropped,
            1 => Outcome::Acquitted,
            2 => Outcome::Pleaded,
            _ => Outcome::Convicted,
        }
    }
    pub fn guilty(self) -> bool {
        matches!(self, Outcome::Pleaded | Outcome::Convicted)
    }
    fn words(self) -> &'static str {
        match self {
            Outcome::Dropped => "the charges were dropped",
            Outcome::Acquitted => "he was acquitted",
            Outcome::Pleaded => "he pleaded to a lesser charge",
            Outcome::Convicted => "he was convicted",
        }
    }
}

fn violent(charge: &str) -> bool {
    ["fight", "assault", "domestic", "weapons"].iter().any(|k| charge.contains(k))
}

/// Games a guilty man's charge calls for at a school: a game for a DUI, a
/// drug or a theft case, two for violence, one more for a domestic case,
/// one more for a man who has been in trouble before.
pub fn due_games(charge: &str, again: bool) -> i32 {
    (if violent(charge) { 2 } else { 1 }) + charge.contains("domestic") as i32 + again as i32
}

/// Games on the 0-6 severity scale of a ruling (`Response::severity`): a
/// game is a deactivation or a game sat (3), more is a suspension.
fn games_sev(g: i32) -> i32 {
    match g {
        x if x <= 0 => 0,
        1 => 3,
        x => 3 + x.min(3),
    }
}

/// What an arrest calls for on the 0-6 scale, as the room sees it: at a
/// school the charge's games; at a club the league rules, so the club's
/// own part is nothing, a deactivation for a domestic case, a little for
/// other violence.
pub fn due(level: Level, charge: &str, again: bool) -> i32 {
    match level {
        Level::Nfl if charge.contains("domestic") => 3,
        Level::Nfl if violent(charge) => 1 + again as i32,
        Level::Nfl => 0,
        _ => games_sev(due_games(charge, again)),
    }
}

/// The share of the season a high school's athletic code holds a player
/// out for a police matter: a quarter of its games (2 or 3).
fn code_games(w: &World, team: Team) -> u8 {
    (super::facts::team_games(w, team).len() as u8 / 4).clamp(2, 3)
}

/// Your staff's standard: the league at a club; at a school, indefinitely
/// for violence, else the charge's game (the code's share in high school).
fn staff_arrest(w: &World, team: Team, charge: &str) -> Response {
    match team.level() {
        Level::Nfl => Response::Wait,
        _ if violent(charge) => Response::Indefinite,
        Level::HighSchool => Response::Suspend(code_games(w, team)),
        _ => Response::BenchGame,
    }
}

/// Your player's case opened tonight (`courts::amateur`, `courts::mirror`):
/// the ruling waits for you the next morning, and stops the day.
pub(crate) fn opened(w: &mut World, id: u32, case: Date, charge: &str) {
    let Some(team) = Team::seat(w) else { return };
    let Some(i) = super::person(w, id) else { return };
    if !team.roster(w).contains(&i) {
        return;
    }
    let name = w.people[i].name.clone();
    let mut s = Scene::new(Kind::Discipline, w.today.add(1), format!("{name} was arrested"));
    s.a = id as u64;
    s.b = ARREST;
    s.c = case.0 as i64;
    s.team = Some(team);
    s.place = "Your office".into();
    let _ = charge;
    super::scene::add(w, s);
}

/// His case is over (`courts::step`): the outcome comes back to you. A man
/// you dismissed is gone; when he was cleared, the room remembers.
pub(crate) fn settled(w: &mut World, id: u32, case: Date, out: Outcome) {
    let Some(team) = Team::seat(w) else { return };
    let Some(i) = super::person(w, id) else { return };
    let mine = team.roster(w).contains(&i);
    let ruled = last_ruling(w, id, case).map(|r| r.response);
    if !mine {
        if ruled == Some(Response::Dismiss) && !out.guilty() {
            for j in team.roster(w) {
                w.people[j].trust = (w.people[j].trust + ROOM_WRONG_DISMISS).clamp(0.0, 1.0);
            }
            let text = format!(
                "The case against {} is over: {}. You had dismissed him over it; the room remembers.",
                player_link(w, i),
                out.words()
            );
            super::note(w, "rules", "The courts".into(), text, None);
        }
        return;
    }
    // At a club the league rules on a guilty man; a cleared one is simply
    // back to football.
    if team.level() == Level::Nfl && !out.guilty() {
        return;
    }
    let name = w.people[i].name.clone();
    let mut s = Scene::new(Kind::Discipline, w.today.add(1), format!("{name}'s case is over"));
    s.a = id as u64;
    s.b = COURT;
    s.c = case.0 as i64;
    s.f[0] = out.code();
    s.team = Some(team);
    s.place = "Your office".into();
    super::scene::add(w, s);
}

fn last_ruling(w: &World, id: u32, case: Date) -> Option<&ArrestRuling> {
    w.folk.rules.arrests.iter().rev().find(|r| r.id == id && r.case == case)
}

/// The case behind a scene: its charge, whether it's his second, and (open
/// or closed) the courts' record.
fn case_of(w: &World, id: u32, case: Date) -> Option<super::courts::Case> {
    let c = &w.folk.courts;
    c.open.iter().chain(c.closed.iter()).rev().find(|c| c.id == id && c.opened == case).cloned()
}

/// Whether trouble found him before this case: an earlier case closed in
/// the courts, or more than one incident on his record.
fn again(w: &World, i: usize, case: Date) -> bool {
    let id = w.people[i].player.id;
    w.folk.courts.closed.iter().any(|c| c.id == id && c.opened < case)
        || w.people[i].notes.iter().filter(|n| n.starts_with("Off-field trouble")).count() > 1
        || w.folk.rules.arrests.iter().any(|r| r.id == id && r.case < case)
}

/// Games your team played since a day, through today.
fn games_since(w: &World, team: Team, d: Date) -> i32 {
    let today = w.today;
    super::facts::team_games(w, team).iter().filter(|g| g.0 >= d && g.0 < today).count() as i32
}

/// Games he has already sat for this case: a game or a suspension served,
/// or every game since an indefinite suspension.
fn served(w: &World, team: Team, r: Option<&ArrestRuling>) -> i32 {
    match r.map(|r| (r.response, r.date)) {
        Some((Response::Indefinite, d)) => games_since(w, team, d),
        Some((Response::BenchGame, _)) => 1,
        Some((Response::Suspend(n), _)) => n as i32,
        _ => 0,
    }
}

fn mark_final(s: &mut Scene) {
    if let Some(p) = s.picks.last_mut() {
        p.kind = PickKind::Final;
    }
}

pub(crate) fn open(w: &mut World, s: &mut Scene) {
    let id = s.a as u32;
    let Some(i) = super::person(w, id) else {
        s.end();
        return;
    };
    let team = s.team.or(Team::seat(w));
    let Some(team) = team.filter(|t| t.roster(w).contains(&i)) else {
        s.narrate("He's no longer on your team.");
        s.end();
        return;
    };
    let case = Date(s.c as i32);
    let Some(c) = case_of(w, id, case) else {
        s.narrate("The case is no longer on file.");
        s.end();
        return;
    };
    s.who.push(super::scene::Who::Player(id));
    if s.b == COURT {
        court_open(w, s, i, team, &c);
    } else {
        arrest_open(w, s, i, team, &c);
    }
}

fn arrest_open(w: &mut World, s: &mut Scene, i: usize, team: Team, c: &super::courts::Case) {
    let level = team.level();
    let again = again(w, i, c.opened);
    let when = super::claims::words::when_words(c.opened, w.today);
    s.narrate(format!(
        "{} was arrested {when}: {}. {}",
        player_link(w, i),
        c.what,
        match c.step {
            super::courts::Step::Arrest => "Charges are pending.",
            super::courts::Step::Charged => "He has been charged.",
            _ => "His case is in court.",
        }
    ));
    s.narrate(if again {
        "It isn't the first time trouble has found him.".to_string()
    } else {
        "It's the first time he's been in trouble with the police.".to_string()
    });
    // The precedent: how you ruled on the last arrest.
    if let Some(r) = w.folk.rules.arrests.iter().rev().find(|r| !r.court && r.id != c.id).cloned() {
        let who = super::person(w, r.id).map_or_else(|| "a player".into(), |j| player_link(w, j));
        s.narrate(format!("When {who} was arrested ({}), you had him {}.", r.charge, r.response.words(level)));
    }
    if rules::is_star(w, i) {
        s.narrate("He's one of your best players; the room and the town are watching what you do.");
    }
    let standard = staff_arrest(w, team, &c.what);
    s.narrate(format!("Your staff's standard: {}.", standard.words(level)));
    rules::speak_first(w, s, i);
    let d = due(level, &c.what, again);
    let fit = |r: Response, hint: &str| format!("{hint} {}", rules::fit_words(r.severity() as i32 - d));
    match level {
        Level::Nfl => {
            s.pick(
                523,
                "Let the league handle it under the personal conduct policy",
                "Wait",
                "calm",
                &fit(Response::Wait, "The usual answer: the league investigates and rules. He plays meanwhile."),
            );
            s.pick(
                521,
                "Deactivate him for the next game",
                "Bench",
                "firm",
                &fit(Response::BenchGame, "The club's own answer while the league looks into it. The union is told."),
            );
            mark_final(s);
            s.pick(524, "Release him", "Dismiss", "fiery", &fit(Response::Dismiss, "Gone, and his cap charge stays."));
            mark_final(s);
        }
        _ => {
            s.pick(
                520,
                "Suspend him indefinitely while the case plays out",
                "Suspend",
                "firm",
                &fit(Response::Indefinite, "He's out until you reinstate him or the court is done with it."),
            );
            mark_final(s);
            if level == Level::HighSchool {
                let n = code_games(w, team);
                s.pick(
                    522,
                    format!("Hold him out {n} games, as the athletic code reads"),
                    "Suspend",
                    "firm",
                    &fit(Response::Suspend(n), "The school's code for a police matter: a share of the season."),
                );
            } else {
                s.pick(
                    521,
                    "Sit him for the next game",
                    "Bench",
                    "firm",
                    &fit(Response::BenchGame, "The common answer to a first DUI or a minor case."),
                );
            }
            mark_final(s);
            s.pick(
                523,
                "He plays. Let the legal process run its course",
                "Wait",
                "calm",
                &fit(Response::Wait, "Innocent until proven otherwise; the press will ask why he's out there."),
            );
            s.pick(
                524,
                if level == Level::College { "Dismiss him from the team" } else { "Remove him from the team" },
                "Dismiss",
                "fiery",
                &fit(
                    Response::Dismiss,
                    if level == Level::College {
                        "He can transfer. If he's cleared, the room remembers you moved first."
                    } else {
                        "He's off the team. His parents will call the principal."
                    },
                ),
            );
            mark_final(s);
        }
    }
}

fn court_open(w: &mut World, s: &mut Scene, i: usize, team: Team, c: &super::courts::Case) {
    let level = team.level();
    let out = Outcome::of(s.f[0]);
    let ruled = last_ruling(w, c.id, c.opened).cloned();
    let served = served(w, team, ruled.as_ref());
    s.narrate(format!("{}'s case is over ({}): {}.", player_link(w, i), c.what, out.words()));
    let out_now = w.folk.out.iter().any(|x| x.0 == c.id && x.2.starts_with("suspended indefinitely"));
    s.narrate(match ruled.as_ref().map(|r| r.response) {
        Some(Response::Indefinite) if out_now => format!(
            "You suspended him indefinitely on {}. {}",
            ruled.as_ref().map_or(String::new(), |r| r.date.pretty()),
            if served > 0 {
                format!("He has sat {served} {} since.", super::noun(served as u32, "game"))
            } else {
                "No game has come up since.".to_string()
            }
        ),
        Some(Response::Wait) | None => "He kept playing while it ran.".to_string(),
        Some(r) => format!("He was {} for it.", r.words(level)),
    });
    rules::speak_first(w, s, i);
    let again = again(w, i, c.opened);
    let left = if out.guilty() { due_games(&c.what, again) - served } else { -served };
    let d = games_sev(left);
    let fit = |r: Response, hint: &str| format!("{hint} {}", rules::fit_words(r.severity() as i32 - d));
    let more = left.clamp(1, 3) as u8;
    // The games the suspend pick names, for its answer (`answer`, 535).
    s.f[1] = more as f32;
    if level == Level::Nfl {
        s.pick(537, "Keep him. The league will rule on it", "Wait", "calm", &fit(Response::Wait, "The usual answer."));
        s.pick(536, "Release him", "Dismiss", "fiery", &fit(Response::Dismiss, "Gone, and his cap charge stays."));
        mark_final(s);
        return;
    }
    if !out.guilty() {
        if out_now {
            s.pick(530, "Reinstate him today", "Praise", "warm", &fit(Response::Wait, "He's back with the team."));
            s.pick(
                531,
                "Keep him out one more game for the trouble it brought the team",
                "Suspend",
                "firm",
                &fit(Response::Suspend(1), "Your own rules still count, cleared or not."),
            );
            mark_final(s);
        } else {
            s.pick(
                533,
                "Tell him you're glad it's behind him",
                "Praise",
                "warm",
                &fit(Response::Wait, "The book is closed."),
            );
        }
        return;
    }
    match ruled.as_ref().map(|r| r.response) {
        Some(Response::Indefinite) if out_now => {
            s.pick(
                534,
                "Reinstate him: the games he's missed are his price",
                "Tell him straight",
                "firm",
                &fit(Response::Wait, "He's back with the team."),
            );
        }
        _ => {
            s.pick(
                537,
                "He plays. The court has had its say",
                "Tell him straight",
                "calm",
                &fit(Response::Wait, "No more games."),
            );
        }
    }
    s.pick(
        535,
        format!("Suspend him {more} more {}", super::noun(more as u32, "game")),
        "Suspend",
        "firm",
        &fit(Response::Suspend(more), "The team's price on top of the court's."),
    );
    mark_final(s);
    s.pick(
        536,
        if level == Level::College { "Dismiss him from the team" } else { "Remove him from the team" },
        "Dismiss",
        "fiery",
        &fit(Response::Dismiss, "He's gone."),
    );
    mark_final(s);
}

/// A ruling's answer: what it is, and what he thinks he had coming.
fn rule(w: &mut World, s: &mut Scene, resp: Response, staff: bool) {
    let id = s.a as u32;
    let Some(i) = super::person(w, id) else {
        s.end();
        return;
    };
    let Some(team) = s.team.or(Team::seat(w)) else {
        s.end();
        return;
    };
    let case = Date(s.c as i32);
    let Some(c) = case_of(w, id, case) else {
        s.end();
        return;
    };
    let level = team.level();
    let again = again(w, i, case);
    let court = s.b == COURT;
    let (d, his) = if court {
        let out = Outcome::of(s.f[0]);
        let ruled = last_ruling(w, id, case).cloned();
        let served = served(w, team, ruled.as_ref());
        let left = if out.guilty() { due_games(&c.what, again) - served } else { -served };
        // The indefinite suspension ends here, whatever comes next.
        w.folk.out.retain(|x| !(x.0 == id && x.2.starts_with("suspended indefinitely")));
        if !out.guilty() && resp == Response::Wait {
            talks::mood_event(w, id, Reason::Discipline, CLEARED_RELIEF);
        }
        let d = if level == Level::Nfl { due(level, &c.what, again) } else { games_sev(left) };
        (d, d)
    } else {
        let d = due(level, &c.what, again);
        // hidden: whether he did it, his alone to know: a man who didn't
        // takes the same ruling harder. Never shown.
        let his = d - (!c.guilty) as i32;
        (d, his)
    };
    let over = Over::Arrest { charge: c.what.clone(), case, court };
    rules::carry_out(w, s, id, &over, resp, staff, d, his);
}

pub(crate) fn answer(w: &mut World, s: &mut Scene, code: u32) {
    let team = s.team.or(Team::seat(w));
    let resp = match code {
        520 => Response::Indefinite,
        521 => Response::BenchGame,
        522 => Response::Suspend(team.map_or(2, |t| code_games(w, t))),
        524 | 536 => Response::Dismiss,
        531 => Response::Suspend(1),
        // The games the pick named (`court_open`).
        535 => Response::Suspend((s.f[1] as u8).clamp(1, 3)),
        _ => Response::Wait,
    };
    rule(w, s, resp, false);
}

/// Your staff rules by your program's standard: the charge's game, the
/// school's code, indefinitely for violence, the league at a club; on the
/// court day they take a cleared man back and add what a guilty one still
/// owes.
pub(crate) fn hand(w: &mut World, s: &mut Scene) {
    s.handed = Some("your staff".into());
    let id = s.a as u32;
    let (Some(team), Some(i)) = (s.team.or(Team::seat(w)), super::person(w, id)) else {
        s.end();
        return;
    };
    let case = Date(s.c as i32);
    let Some(c) = case_of(w, id, case) else {
        s.end();
        return;
    };
    if !team.roster(w).contains(&i) {
        s.end();
        return;
    }
    let resp = if s.b == COURT {
        let out = Outcome::of(s.f[0]);
        let ruled = last_ruling(w, id, case).cloned();
        let left =
            if out.guilty() { due_games(&c.what, again(w, i, case)) - served(w, team, ruled.as_ref()) } else { 0 };
        match left {
            _ if team.level() == Level::Nfl => Response::Wait,
            x if x <= 0 => Response::Wait,
            1 => Response::BenchGame,
            x => Response::Suspend(x.min(3) as u8),
        }
    } else {
        staff_arrest(w, team, &c.what)
    };
    rule(w, s, resp, true);
}
