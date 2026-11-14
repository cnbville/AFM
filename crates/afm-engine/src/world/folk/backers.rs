//! Booster promises that can be broken (talk facts lane L8, item 17; the
//! design's section 9, "Boosters"). A promise to a booster (takeover item
//! 14) is held to a real test of what he wanted, and each test is a thing
//! you do or don't do:
//!
//! - Access: his pass is tested the first time he crosses the line in a
//!   season (in the locker room after a loss): pull it and the promise is
//!   broken; keep him and the room sees it.
//! - A say in hires: the coordinators you hire must meet him first (a
//!   pick in the interview); hire one he never met and it breaks that day.
//! - A say on a player: the local kid he named gets a real look (games or
//!   a start) by the promise's day.
//! - His old high school: its best prospect in each class hears from you.
//! - His name on a building: when your boss grants a facility, you back
//!   his name or leave it to the AD.
//! - A clean program: no arrest of your players since you promised.
//! - Beating the rival, winning now and NIL deals keep the tests they had
//!   (`boss::want_met`).
//!
//! Broken, it costs his trust (`talks::settle_word`, as before), and he
//! takes it to your boss and cuts what he gives (`broken`). A big giver's
//! walking away is the collective's yearly read (`boss::booster_push`).
//! Sizes are Claude's (proposed, not confirmed).

use super::cast::{self, Cast, CastRef, Want};
use super::scene::{self, Kind, Scene, Who};
use super::talks::{PromiseKind, PromiseTo};
use super::{Team, player_link};
use crate::calendar::{Date, Level, Loudness};
use crate::player::PlayingTime;
use crate::world::{Stage, World};

/// Your boss's trust when a big giver tells him you broke your word to
/// him (a smaller giver: `BOSS_HEARS_SMALL`), times `MEDDLER_HEARS` for a
/// meddler (meddling 14 or more).
pub const BOSS_HEARS: f32 = -0.04;
pub const BOSS_HEARS_SMALL: f32 = -0.02;
pub const MEDDLER_HEARS: f32 = 1.5;
/// The share of his yearly gift a booster you broke your word to stops
/// giving to the collective (the same share an NIL promise added).
pub const GIFT_CUT: f64 = 0.15;
/// A real look for his kid: this many games for you this season, or a
/// start.
pub const KID_GAMES: u32 = 4;
/// The odds a booster with a pass crosses the line after a loss: 0.2 at
/// meddling 10, 0.06 a point more, 0.05 to 0.6; once a season.
pub const CROSS_BASE: f64 = 0.2;
pub const CROSS_PER: f64 = 0.06;
/// The room when you let a booster stay in the locker room (each player's
/// trust), and when you pull his pass or put him in his place.
pub const ROOM_LET_GO: f32 = -0.01;
pub const ROOM_PULLED: f32 = 0.006;
pub const ROOM_TALKED: f32 = 0.003;
/// His trust when you tell him the locker room is off limits.
pub const TALKED_TO: f32 = -0.04;
/// Your boss's trust when you back a booster's name on his building (the
/// naming is his to sell), and when you leave it to him.
pub const NAME_PUSHED: f32 = -0.02;
pub const NAME_LEFT: f32 = 0.01;

/// What your booster promises hang on.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Book {
    /// The local kid a booster wants a real look for: (his key, player id).
    pub kid: Vec<(u64, u32)>,
    /// Coordinator candidates a booster met before you hired them.
    pub met: Vec<usize>,
    /// Seasons a booster's pass was tested: (his key, season).
    pub tested: Vec<(u64, i32)>,
}

/// `Scene.b` of a booster's pass put to the test (a takeover booster
/// meeting keeps 0).
pub const PASS_TEST: u64 = 7;
/// `Scene.b` of the facility's naming, asked at the boss's meeting.
pub const NAMING: u32 = 150;

/// An open promise to a booster of yours for a want.
fn open_promise(w: &World, want: Want) -> Option<(usize, CastRef, u16)> {
    w.folk.promises.iter().enumerate().find_map(|(k, p)| match (p.to, p.kind) {
        (PromiseTo::Cast(r, term), PromiseKind::Booster(x)) if x == want && p.kept.is_none() => Some((k, r, term)),
        _ => None,
    })
}

/// The booster himself, if he still holds the job he held then.
fn backer(w: &World, r: CastRef, term: u16) -> Option<Cast> {
    let c = cast::get(w, r);
    (c.term == term).then_some(c)
}

/// Settles an open booster promise now, kept or broken, as the promise's
/// day would (`talks::check_promises`).
fn settle(w: &mut World, k: usize, kept: bool, why: &str) {
    let p = w.folk.promises[k].clone();
    w.folk.promises[k].kept = Some(kept);
    super::talks::settle_word(
        w,
        p.to,
        kept,
        if p.public { 2.0 } else { 1.0 },
        if kept { super::talks::Reason::PromiseKept } else { super::talks::Reason::PromiseBroken },
    );
    if !kept && let PromiseTo::Cast(r, term) = p.to {
        broken(w, r, term);
    }
    let text = format!(
        "{} the promise of {} you made to {} on {}: {why}",
        if kept { "You kept" } else { "You broke" },
        p.kind.words(),
        p.who,
        p.made.pretty()
    );
    super::note(w, "talk", "Your promises".into(), text.clone(), None);
    super::post_mine(w, w.today, Loudness::Small, false, "people: promise", &text);
}

/// What a broken promise costs beyond his trust: he takes it to your boss,
/// and gives less (`talks::check_promises` and `settle` call it).
pub(crate) fn broken(w: &mut World, r: CastRef, term: u16) {
    let Some(c) = backer(w, r, term) else { return };
    let Some(b) = c.booster.as_ref() else { return };
    let Some(team) = Team::seat(w) else { return };
    let size = if b.big { BOSS_HEARS } else { BOSS_HEARS_SMALL } * if c.meddling >= 14.0 { MEDDLER_HEARS } else { 1.0 };
    let boss = super::boss::boss(w, team);
    w.folk.mine.nudge(boss.key(), size);
    if let Team::College(p) = team
        && b.yearly > 0.0
    {
        let col = &mut w.programs[p].collective;
        col.budget = (col.budget - b.yearly * GIFT_CUT).max(0.0);
    }
}

/// Whether a booster's want is met on his promise's day (`talks::kept`).
/// The tests settled on the spot (his pass, a hire, the naming) leave
/// nothing to break by then.
pub(crate) fn kept(w: &World, to: PromiseTo, made: Date, want: Want) -> bool {
    let Some(team) = Team::seat(w) else { return true };
    match want {
        Want::Clean => clean_since(w, team, made),
        Want::SayOnPlayer => match to {
            PromiseTo::Cast(r, term) => kid_kept(w, team, r.key(term)),
            _ => true,
        },
        Want::OldSchool => match to {
            PromiseTo::Cast(r, term) => backer(w, r, term)
                .and_then(|c| c.booster.and_then(|b| b.old_school))
                .is_none_or(|h| old_school_kept(w, team, h, made)),
            _ => true,
        },
        Want::Access | Want::SayInHires | Want::Building => true,
        _ => super::boss::want_met(w, want),
    }
}

/// No case opened against your players since a day (an arrest you ruled on
/// counts even after he left).
fn clean_since(w: &World, team: Team, since: Date) -> bool {
    let ids: Vec<u32> = team.roster(w).iter().map(|&i| w.people[i].player.id).collect();
    let c = &w.folk.courts;
    !c.open.iter().chain(c.closed.iter()).any(|c| c.opened >= since && ids.contains(&c.id))
        && !w.folk.rules.arrests.iter().any(|r| r.case >= since)
}

/// The local kid a booster means: the best man on your roster from your
/// state who doesn't start (what anyone saw of him, `takeover::standing`).
pub fn kid(w: &World, team: Team) -> Option<usize> {
    let st = team.state(w)?;
    team.roster(w)
        .into_iter()
        .filter(|&i| w.people[i].home_state == st && !super::talks::is_starter(w, i))
        .max_by(|&a, &b| super::takeover::standing(w, a).total_cmp(&super::takeover::standing(w, b)))
}

/// Remembers the kid you promised a booster a look for.
pub(crate) fn promised_kid(w: &mut World, key: u64, id: u32) {
    w.folk.backers.kid.retain(|x| x.0 != key);
    w.folk.backers.kid.push((key, id));
}

/// The kid got his look: games or a start for you this season; gone on his
/// own (graduated, drafted, out of football) it holds; transferred or
/// dismissed, the booster blames you.
fn kid_kept(w: &World, team: Team, key: u64) -> bool {
    let Some(&(_, id)) = w.folk.backers.kid.iter().find(|x| x.0 == key) else { return true };
    let Some(i) = super::person(w, id) else { return true };
    if !team.roster(w).contains(&i) {
        let p = &w.people[i];
        return match team {
            Team::College(c) => {
                p.college.is_none_or(|x| x == c) && matches!(p.stage, Stage::Pro | Stage::Declared | Stage::Done)
            }
            _ => true,
        };
    }
    real_look(w, team, i)
}

/// A real look: `KID_GAMES` games or a start this season, or a starter's
/// or rotation role.
fn real_look(w: &World, team: Team, i: usize) -> bool {
    use crate::world::tally::Level as T;
    let id = w.people[i].player.id;
    let level = match team.level() {
        Level::Nfl => T::Nfl,
        Level::HighSchool => T::Hs,
        _ => T::College,
    };
    let line = w.tallies.players.get(level, id).filter(|_| w.tallies.season == w.dates.season);
    line.is_some_and(|s| s.games as u32 >= KID_GAMES || s.starts > 0)
        || matches!(w.people[i].last_role, Some(PlayingTime::Starter | PlayingTime::Rotation))
}

/// His old school's best prospect in each class since the promise heard
/// from you: a contact, an offer, or he signed with you. A school with no
/// prospect of three stars or more asks nothing.
fn old_school_kept(w: &World, team: Team, h: usize, made: Date) -> bool {
    let Team::College(p) = team else { return true };
    let mut best: Vec<(i32, usize)> = Vec::new();
    for (i, q) in w.people.iter().enumerate() {
        if q.hs != h || q.stars < 3 || q.class_of <= made.year() || q.class_of > w.year + 1 {
            continue;
        }
        match best.iter_mut().find(|x| x.0 == q.class_of) {
            Some(x) if w.people[x.1].stars < q.stars => x.1 = i,
            Some(_) => {}
            None => best.push((q.class_of, i)),
        }
    }
    best.iter().all(|&(_, i)| {
        let c = &w.people[i].cal;
        c.offers.contains(&p)
            || c.early_offers.iter().any(|o| o.0 == p)
            || c.visits.iter().any(|v| v.0 == p && v.1 >= made)
            || c.signed.is_some_and(|s| s.program == p)
    })
}

// ---------------------------------------------------------------------------
// A say in hires: the candidate meets him first.

/// The pick the interview offers while you owe a booster a say in hires,
/// for a coordinator's job: (code, label).
pub fn meet_pick(w: &World, c: usize, coordinator: bool) -> Option<(u32, String)> {
    if !coordinator || w.folk.backers.met.contains(&c) {
        return None;
    }
    let (_, r, term) = open_promise(w, Want::SayInHires)?;
    let b = backer(w, r, term)?;
    Some((68, format!("Have {} meet him over lunch before you decide", b.name)))
}

/// The candidate meets the booster: logged, so hiring him keeps your word.
/// A proud candidate notices who is vetting him.
pub(crate) fn meet(w: &mut World, s: &mut Scene, c: usize) {
    let Some((_, r, term)) = open_promise(w, Want::SayInHires) else { return };
    let Some(b) = backer(w, r, term) else { return };
    w.folk.backers.met.push(c);
    if w.folk.backers.met.len() > 40 {
        w.folk.backers.met.remove(0);
    }
    let name = w.coaches[c].name.clone();
    let proud = w.coaches[c].personality.ambition >= 14.0;
    if proud {
        w.folk.mine.nudge(super::coach_key(c), -0.02);
    }
    s.narrate(format!(
        "{} takes {name} to lunch at his club. {}",
        b.link(),
        if proud {
            "He goes, and makes it clear he's interviewing for you, not for him."
        } else {
            "He comes back saying the man knows his football."
        }
    ));
}

/// You hired a coordinator (`hiring::hire_now`): a booster you promised a
/// say in hires either met him first, or you broke your word today.
pub(crate) fn hired(w: &mut World, c: usize, coordinator: bool) {
    if !coordinator {
        return;
    }
    let Some((k, r, term)) = open_promise(w, Want::SayInHires) else { return };
    let met = w.folk.backers.met.contains(&c);
    let name = w.coaches[c].name.clone();
    let who = backer(w, r, term).map_or_else(|| "him".to_string(), |b| b.name);
    if met {
        settle(w, k, true, &format!("{who} met {name} before you hired him."));
    } else {
        settle(w, k, false, &format!("you hired {name} without letting {who} meet him first."));
    }
}

// ---------------------------------------------------------------------------
// Access: his pass put to the test.

/// The Monday after a loss, a booster you promised access may cross the
/// line once a season: in the locker room, at a player's locker. It comes
/// to you that morning.
pub(crate) fn day(w: &mut World, d: Date) {
    if d.weekday() != crate::calendar::Weekday::Mon {
        return;
    }
    let Some(team) = Team::seat(w) else { return };
    let Some((_, r, term)) = open_promise(w, Want::Access) else { return };
    let Some(b) = backer(w, r, term) else { return };
    let season = w.dates.season;
    if w.folk.backers.tested.iter().any(|x| x.0 == b.key() && x.1 == season) {
        return;
    }
    let lost = w.seat_games.last().filter(|g| g.date.days_to(d) <= 3).is_some_and(|g| {
        let (_, me) = team.parts();
        let side = if g.teams[0] == me { 0 } else { 1 };
        g.stats.score[side] < g.stats.score[1 - side]
    });
    if !lost {
        return;
    }
    let p = (CROSS_BASE + CROSS_PER * (b.meddling - 10.0)).clamp(0.05, 0.6);
    if super::roll(w, 0xB005 ^ d.0 as u64, b.key()) >= p {
        return;
    }
    w.folk.backers.tested.push((b.key(), season));
    if w.folk.backers.tested.len() > 40 {
        w.folk.backers.tested.remove(0);
    }
    let mut s = Scene::new(Kind::Booster, d, format!("{} in the locker room", b.name));
    s.a = r.slot as u64;
    s.b = PASS_TEST;
    s.team = Some(team);
    s.place = "Your office".into();
    scene::add(w, s);
}

pub(crate) fn pass_open(w: &mut World, s: &mut Scene, team: Team, b: &Cast) {
    s.who.push(Who::Cast(b.r.token()));
    let caps = super::takeover::captains_of(w, team);
    let who = caps.first().map_or_else(|| "One of your captains".into(), |&i| player_link(w, i));
    s.narrate(format!(
        "{who} stops by your office. After the loss {} was in the locker room with his sideline pass, going locker to locker telling players what went wrong. Some of the young ones didn't know who he was.",
        b.link()
    ));
    s.narrate("You promised him that pass. The room is waiting to see what you do about it.");
    s.pick(
        140,
        "Pull his pass. The locker room is for the team",
        "Tell him straight",
        "firm",
        "The room sees where the line is. Your promise to him is broken, and he'll tell your boss.",
    );
    s.pick(
        141,
        format!("Call {}: the sideline stays, the locker room is off limits", b.first()),
        "Tell him straight",
        "calm",
        "You keep your word and draw the line; a proud man takes it badly.",
    );
    s.pick(
        142,
        "Let it go. He's earned his place around here",
        "Listen",
        "warm",
        "Your word holds. The players saw him in their room, and saw you let it go.",
    );
}

pub(crate) fn pass_answer(w: &mut World, s: &mut Scene, team: Team, b: &Cast, code: u32) {
    let roster = team.roster(w);
    let room = |w: &mut World, by: f32| {
        for &j in &roster {
            w.people[j].trust = (w.people[j].trust + by).clamp(0.0, 1.0);
        }
    };
    let k = open_promise(w, Want::Access).map(|x| x.0);
    match code {
        140 => {
            room(w, ROOM_PULLED);
            if let Some(k) = k {
                settle(w, k, false, "you pulled his sideline pass after he went into the locker room.");
            }
            s.outcome.push(format!("You pulled {}'s pass. The room noticed.", b.name));
        }
        141 => {
            room(w, ROOM_TALKED);
            let by = TALKED_TO * if b.ego >= 14.0 { 1.5 } else { 1.0 };
            w.folk.mine.nudge(b.key(), by);
            s.outcome.push(format!("{} keeps the sideline; the locker room is off limits.", b.name));
        }
        _ => {
            room(w, ROOM_LET_GO);
            s.outcome.push(format!("You let it go. {} keeps his pass, and his habits.", b.name));
        }
    }
    s.end();
}

// ---------------------------------------------------------------------------
// His name on a building: the naming, when the facility comes.

/// After your boss grants a facility, he asks where you stand on its name
/// while you owe a booster your vote: the picks.
pub(crate) fn naming_picks(w: &World, s: &mut Scene) -> bool {
    let Some((_, r, term)) = open_promise(w, Want::Building) else { return false };
    let Some(b) = backer(w, r, term) else { return false };
    s.narrate(format!(
        "\"The naming rights go with the gift,\" he says. \"I hear you've talked to {}. Where are you on it?\"",
        b.link()
    ));
    s.picks.clear();
    s.pick(
        NAMING,
        format!("Tell him it should carry {}'s name", b.name),
        "Promise",
        "firm",
        "You keep your word to the booster; the naming is your boss's to sell, and he notes it.",
    );
    s.pick(
        NAMING + 1,
        "Leave the name to him and the donors",
        "Tell him straight",
        "calm",
        "Your boss likes it. Your promise to the booster is broken.",
    );
    true
}

pub(crate) fn naming_answer(w: &mut World, s: &mut Scene, team: Team, code: u32) {
    let Some((k, r, term)) = open_promise(w, Want::Building) else { return };
    let who = backer(w, r, term).map_or_else(|| "the booster".to_string(), |b| b.name);
    let boss = super::boss::boss(w, team);
    if code == NAMING {
        w.folk.mine.nudge(boss.key(), NAME_PUSHED);
        settle(w, k, true, &format!("you backed {who}'s name on the new building."));
        s.outcome.push(format!("You backed {who}'s name on the building."));
    } else {
        w.folk.mine.nudge(boss.key(), NAME_LEFT);
        settle(w, k, false, &format!("you left the building's name to {} and the donors.", boss.name));
        s.outcome.push("You left the naming to your boss.".into());
    }
}

/// The want a booster's meeting is about, for the takeover's booster scene.
pub fn is_pass_test(s: &Scene) -> bool {
    s.kind == Kind::Booster && s.b == PASS_TEST
}
