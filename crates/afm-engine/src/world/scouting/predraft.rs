//! The pre-draft season (topic 10): prospects and their camps make their
//! own choices (skip the combine's workouts and test at the pro day, skip
//! the Senior Bowl, train at a testing camp all winter, refuse a club's
//! visit, or, rarely, say they won't play for the club picking first, as
//! John Elway did in 1983 and Eli Manning in 2004); clubs' interest leaks
//! through visits and the press, some of it planted (smokescreens); your
//! war room picks its own top-30 visits, private workouts, interviews and
//! medical rechecks.

use crate::calendar::{Date, Event, Kind as CalKind, Level, Loudness};
use crate::player::Trait;
use crate::world::season::key_uniform;
use crate::world::{Stage, World};

/// A prospect's choices (bit flags).
pub const SKIP_COMBINE: u8 = 1;
pub const SKIP_SENIOR_BOWL: u8 = 2;
pub const TESTING_CAMP: u8 = 4;
pub const PRO_DAY_ONLY_SOME: u8 = 8;

/// A club's interest that came out, true or planted.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Leak {
    pub team: usize,
    pub player: u32,
    pub date: Date,
    /// Real interest, or a smokescreen.
    pub real: bool,
    /// Your war room planted it.
    pub yours: bool,
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct PreDraft {
    /// Player id to choice flags.
    pub choices: std::collections::HashMap<u32, u8>,
    /// A prospect who said he won't play for a club: (player id, club).
    pub refuses: Vec<(u32, usize)>,
    pub leaks: Vec<Leak>,
    /// Visits a prospect turned down: (player id, club).
    pub declined: Vec<(u32, usize)>,
    /// Your war room's lists: (player id, list): top-30 visits, combine
    /// interviews, private workouts, medical rechecks.
    #[serde(default)]
    pub lists: Vec<(u32, u8)>,
}

/// Your war room's lists.
pub const LIST_VISIT: u8 = 0;
pub const LIST_INTERVIEW: u8 = 1;
pub const LIST_WORKOUT: u8 = 2;
pub const LIST_MEDICAL: u8 = 3;
pub const LIST_NAMES: [&str; 4] = ["Top-30 visits", "Combine interviews", "Private workouts", "Medical rechecks"];
/// Each list's limit (real: 30 visits; 60 interviews to 2019, 45 from 2020).
pub fn list_cap(list: u8, year: i32) -> usize {
    match list {
        LIST_VISIT => 30,
        LIST_INTERVIEW => {
            if year >= 2020 {
                45
            } else {
                60
            }
        }
        _ => 40,
    }
}

/// Your list of one kind, when `t` is your club.
pub fn seat_list(w: &World, t: usize, list: u8) -> Vec<u32> {
    if w.seat.and_then(|s| s.nfl_team) != Some(t) {
        return Vec::new();
    }
    w.scouting.predraft.lists.iter().filter(|x| x.1 == list).map(|x| x.0).collect()
}

/// Whether a prospect's camp turns down a club's top-30 visit: a sure
/// first-rounder with a big ego and a club far from his likely pick
/// (proposed, not confirmed: about one visit in eight for such players).
pub fn declines(w: &World, i: usize, t: usize) -> bool {
    let p = &w.people[i];
    let Some(r) = p.cal.draft_read else { return false };
    if r.round != 1 || r.score < 80.0 {
        return false;
    }
    let ego = p.player.personality.traits.z(Trait::Ego);
    key_uniform(0xDEC1 ^ (t as u64) << 32 ^ w.year as u64, p.player.id as u64) < 0.08 + 0.05 * ego.max(0.0)
}

impl PreDraft {
    pub fn has(&self, id: u32, flag: u8) -> bool {
        self.choices.get(&id).is_some_and(|c| c & flag != 0)
    }
}

/// The words for a prospect's choices.
pub fn choice_words(flags: u8) -> Vec<&'static str> {
    let mut v = Vec::new();
    if flags & SKIP_COMBINE != 0 {
        v.push("Skipping the combine workouts; he tests at his pro day");
    }
    if flags & SKIP_SENIOR_BOWL != 0 {
        v.push("Turned down the Senior Bowl");
    }
    if flags & TESTING_CAMP != 0 {
        v.push("Training at a testing camp all winter");
    }
    if flags & PRO_DAY_ONLY_SOME != 0 {
        v.push("Will only throw for a few clubs");
    }
    v
}

/// The day's pre-draft work: choices after the declaration deadline, leaks
/// through the spring.
pub(crate) fn day(w: &mut World, d: Date) {
    let (_, m, day) = d.parts();
    if m == 1 && day == 20 {
        choose(w, d);
    }
    if matches!(m, 3 | 4) && d < w.dates.draft[0].date && d.weekday() == crate::calendar::Weekday::Wed {
        leaks(w, d);
    }
}

/// Each declared prospect's choices from his personality, agent and
/// projection (proposed, not confirmed: a sure first-rounder skips the
/// combine's workouts about a third of the time; a top quarterback with a
/// strong family and agent refuses a club about once a decade).
fn choose(w: &mut World, d: Date) {
    let year = w.year;
    let order = super::super::nfl_year::draft_order(w).unwrap_or_default();
    let mut out: Vec<(u32, u8)> = Vec::new();
    let mut refuse: Vec<(u32, usize, String)> = Vec::new();
    for p in w.people.iter().filter(|p| p.stage == Stage::Declared) {
        let Some(r) = p.cal.draft_read else { continue };
        let id = p.player.id as u64;
        let ego = p.player.personality.traits.z(Trait::Ego);
        let work = (p.player.personality.work_ethic - 10.0) / 5.0;
        let u = |s: u64| key_uniform(0x9D2A ^ s ^ (year as u64) << 20, id);
        let mut f = 0u8;
        if r.round == 1 && u(1) < 0.25 + 0.1 * ego.max(0.0) {
            f |= SKIP_COMBINE;
        }
        if r.round <= 2 && u(2) < 0.2 + 0.1 * ego.max(0.0) {
            f |= SKIP_SENIOR_BOWL;
        }
        if u(3) < 0.35 + 0.15 * work + 0.3 * p.bg.money.min(1.0) {
            f |= TESTING_CAMP;
        }
        if p.player.position == crate::positions::Position::QB && r.round == 1 && u(4) < 0.4 {
            f |= PRO_DAY_ONLY_SOME;
        }
        if f != 0 {
            out.push((p.player.id, f));
        }
        // The rare refusal: a top quarterback, a strong family and agent,
        // a club picking first that he sees as a poor place to start.
        if p.player.position == crate::positions::Position::QB
            && r.round == 1
            && r.score >= 85.0
            && ego > 1.0
            && matches!(
                p.family.decider,
                super::super::background::Decider::Parent | super::super::background::Decider::Handler
            )
            && let Some(&t) = order.first()
            && t < w.nfl.len()
            && w.nfl[t].strength() < 72.0
            && u(5) < 0.12
        {
            refuse.push((p.player.id, t, p.name.clone()));
        }
    }
    for (id, f) in out {
        w.scouting.predraft.choices.insert(id, f);
    }
    for (id, t, name) in refuse {
        w.scouting.predraft.refuses.push((id, t));
        let text = format!(
            "{name}'s camp says he will not play for the {}, who hold the first pick. They want a trade or another team.",
            w.nfl[t].name
        );
        w.calendar.post(Event::new(d, Level::Nfl, CalKind::Transfer, Loudness::Big, text).team(t));
    }
}

/// Visits and interest that come out in the press each week of the spring:
/// most real, some planted by a GM who likes to hide his hand (his
/// overconfidence and impatience stand in for his taste for games).
fn leaks(w: &mut World, d: Date) {
    let year = w.year;
    let teams: Vec<usize> = crate::world::franchise::active_teams(year);
    let pool = super::public::prospects(w);
    if pool.is_empty() {
        return;
    }
    let mut new = Vec::new();
    for &t in &teams {
        if t >= w.clubs.len() || Some(t) == w.seat.and_then(|s| s.nfl_team) {
            continue;
        }
        let u = key_uniform(0x1EA7 ^ (t as u64) << 20, d.0 as u64);
        if u > 0.35 {
            continue;
        }
        let g = &w.clubs[t].office.gm;
        let plant = key_uniform(0x5307 ^ t as u64, d.0 as u64) < 0.15 + 0.2 * g.overconfidence.min(1.0);
        // Its real interest: a prospect from its visits list (its file has
        // a Visit look); a smokescreen names a player it doesn't rate.
        let file = &w.clubs[t].files;
        let visited: Vec<u32> = file
            .iter()
            .filter(|(_, v)| v.iter().any(|l| l.kind == crate::world::pro::scouts::LookKind::Visit))
            .map(|(&id, _)| id)
            .collect();
        let pick_from: Vec<u32> = if plant || visited.is_empty() {
            pool.iter().take(80).map(|&i| w.people[i].player.id).collect()
        } else {
            let mut v = visited;
            v.sort();
            v
        };
        if pick_from.is_empty() {
            continue;
        }
        let k = (key_uniform(0xC0DE ^ t as u64, d.0 as u64) * pick_from.len() as f64) as usize;
        new.push(Leak { team: t, player: pick_from[k.min(pick_from.len() - 1)], date: d, real: !plant, yours: false });
    }
    for l in new {
        if let Some(i) = super::place(w, l.player) {
            let p = &w.people[i];
            let text = format!(
                "Per sources, {} ({}) has visited the {}, who are said to be very interested.",
                p.name, p.player.position, w.nfl[l.team].name
            );
            w.calendar.post(Event::new(d, Level::Nfl, CalKind::Transfer, Loudness::Small, text).team(l.team));
        }
        w.scouting.predraft.leaks.push(l);
    }
}

/// You feed a lead to the press (topic 10): interest in a player you may or
/// may not want. It lands in the news the next day and other clubs read it.
pub fn plant(w: &mut World, id: u32, real: bool) -> Result<String, String> {
    let t = w.seat.and_then(|s| s.nfl_team).ok_or("Only an NFL club leaks to the press.")?;
    let i = super::place(w, id).ok_or("No such player.")?;
    let d = w.today;
    let p = &w.people[i];
    let text = format!(
        "League sources: the {} have done a lot of work on {} ({}) and like him a great deal.",
        w.nfl[t].name, p.name, p.player.position
    );
    w.calendar.post(Event::new(d.add(1), Level::Nfl, CalKind::Transfer, Loudness::Small, text.clone()).team(t));
    w.scouting.predraft.leaks.push(Leak { team: t, player: id, date: d, real, yours: true });
    Ok(text)
}

/// How much a rival's interest moves a club's value of a prospect on draft
/// day: a club picking before the leaking club, with a GM who doesn't read
/// the market well, takes the leak at its word and moves on the player a
/// little (proposed, not confirmed: up to a point and a half).
pub fn leak_pull(w: &World, team: usize, id: u32, slot_of: &dyn Fn(usize) -> Option<usize>, my_slot: usize) -> f64 {
    let Some(club) = w.clubs.get(team) else { return 0.0 };
    let gullible = (1.0 - club.office.gm.skill / 20.0).clamp(0.0, 0.8);
    let mut pull = 0.0;
    for l in &w.scouting.predraft.leaks {
        if l.player != id || l.team == team {
            continue;
        }
        if let Some(s) = slot_of(l.team)
            && s > my_slot
            && s <= my_slot + 10
        {
            pull += 1.5 * gullible * if l.real { 1.0 } else { 0.9 };
        }
    }
    pull.min(2.0)
}

/// Draft morning: what the spring's leaks and refusals did to the clubs'
/// boards. A club a prospect refused passes on him (it takes his camp at
/// its word, as the Colts didn't in 1983 and the Chargers didn't in 2004:
/// Claude's pick, a trade on the clock still can move him); a club that
/// believes a rival behind it wants a player moves him up a little.
pub(crate) fn on_boards(w: &World, slots: &[(usize, u32)], boards: &mut [crate::world::pro::scouts::Board]) {
    let pd = &w.scouting.predraft;
    if pd.leaks.is_empty() && pd.refuses.is_empty() {
        return;
    }
    let places = super::places(w);
    let first = |t: usize| slots.iter().position(|s| s.0 == t);
    for &(id, t) in &pd.refuses {
        if let (Some(&i), Some(b)) = (places.get(&id), boards.get_mut(t)) {
            b.dnd.insert(i);
        }
    }
    let ids: std::collections::HashSet<u32> = pd.leaks.iter().map(|l| l.player).collect();
    for (t, b) in boards.iter_mut().enumerate() {
        let Some(mine) = first(t) else { continue };
        for &id in &ids {
            let Some(&i) = places.get(&id) else { continue };
            let pull = leak_pull(w, t, id, &first, mine);
            if pull > 0.0
                && let Some(g) = b.grade.get_mut(&i)
            {
                *g += pull;
            }
        }
    }
}

/// Whether a prospect refuses a club (his camp said so).
pub fn refuses(w: &World, id: u32, team: usize) -> bool {
    w.scouting.predraft.refuses.iter().any(|&(p, t)| p == id && t == team)
}

/// A combine moment (topic 18): a test far beyond what his body should do
/// becomes a moment in the news, with the fans' reaction.
pub(crate) fn combine_moments(w: &mut World, invited: &[usize]) {
    let d = w.today;
    let mut best: Option<(f64, usize, f64)> = None;
    for &i in invited {
        let p = &w.people[i];
        if w.scouting.predraft.has(p.player.id, SKIP_COMBINE) {
            continue;
        }
        let c = super::file::tests(&p.player, super::Kind::Combine, w.year);
        // A big man's 40: speed for his size against the class.
        let expect = 4.25 + 0.0042 * (p.player.body.weight - 180.0).max(0.0);
        let freak = expect - c.forty;
        if freak > 0.12 && best.is_none_or(|b| freak > b.0) {
            best = Some((freak, i, c.forty));
        }
    }
    if let Some((_, i, forty)) = best {
        let p = &w.people[i];
        let school = p.college.map_or(String::new(), |c| w.program_name(c).to_string());
        let text = format!(
            "The combine moment: {} ({}, {school}) runs a {forty:.2} at {} pounds. The clip is everywhere and the debate is on.",
            p.name,
            p.player.position,
            p.player.body.weight.round()
        );
        let mut e = Event::new(d, Level::Nfl, CalKind::News, Loudness::Medium, text);
        if let Some(c) = p.college {
            e = e.program(c);
        }
        w.calendar.post(e);
    }
}
