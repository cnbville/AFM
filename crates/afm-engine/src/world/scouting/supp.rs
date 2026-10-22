//! The supplemental draft (topic 11), held each summer through 2019 for
//! players who lost their college eligibility after the January deadline
//! (a dismissal, grades). Clubs bid a round in secret; the highest bid
//! wins, and a club that takes a player forfeits that round's pick in the
//! next draft (real: Josh Gordon cost Cleveland a second-round pick in
//! 2012; Terrelle Pryor a third for Oakland in 2011). Ties go by a lottery
//! weighted toward the worst records. The NFL ended it after 2019.

use crate::calendar::{Date, Event, Kind, Level, Loudness, Weekday};
use crate::world::season::key_uniform;
use crate::world::{Stage, World};

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct SuppPick {
    pub year: i32,
    pub player: u32,
    pub name: String,
    /// The club and the round it gave up, or None when nobody bid.
    pub team: Option<usize>,
    pub round: Option<u32>,
}

/// The second Wednesday of July (real: early to mid July).
pub fn date(year: i32) -> Date {
    Date::nth_weekday(year, 7, Weekday::Wed, 2)
}

pub(crate) fn day(w: &mut World, d: Date) {
    if w.year > 2019 || d != date(w.year) || w.clubs.is_empty() {
        return;
    }
    hold(w, d);
}

/// Who is eligible: college players three years out of high school who
/// lost their eligibility this year (dismissed, or academically ineligible
/// for the fall) and are still in the world.
fn eligible(w: &World) -> Vec<usize> {
    (0..w.people.len())
        .filter(|&i| {
            let p = &w.people[i];
            let lost = p.cal.fall_ineligible
                || p.player.history.last().is_some_and(|s| s.events.has(crate::player::DevelopmentEvent::Dismissed));
            p.stage == Stage::College && lost && w.year - p.class_of >= 3 && p.player.overall() >= 55.0
        })
        .collect()
}

fn hold(w: &mut World, d: Date) {
    let year = w.year;
    let cands = eligible(w);
    if cands.is_empty() {
        return;
    }
    let teams = crate::world::franchise::active_teams(year + 1);
    let seat = w.seat.and_then(|s| s.nfl_team);
    for i in cands {
        let p = &w.people[i];
        let id = p.player.id;
        let ov = p.player.overall();
        // Each club bids the round its read says he'd go in a normal year.
        let mut bids: Vec<(u32, f64, usize)> = Vec::new();
        for &t in &teams {
            if t >= w.clubs.len() || Some(t) == seat && !w.scouting.pro.supp_bids.iter().any(|b| b.0 == id) {
                continue;
            }
            let read = ov
                + 4.0 * (crate::world::season::key_normal(0x5A9 ^ (t as u64) << 20, id as u64))
                    * (1.4 - 0.04 * w.clubs[t].office.gm.skill);
            let round = if Some(t) == seat {
                w.scouting.pro.supp_bids.iter().find(|b| b.0 == id).map_or(0, |b| b.1)
            } else if read >= 74.0 {
                2
            } else if read >= 70.0 {
                3
            } else if read >= 66.0 {
                5
            } else if read >= 63.0 {
                7
            } else {
                0
            };
            if round == 0 {
                continue;
            }
            // A worse record wins ties (a weighted lottery).
            let worst = 100.0 - w.nfl[t].strength();
            let lot = worst * key_uniform(0x10_77 ^ (t as u64) << 16 ^ year as u64, id as u64);
            bids.push((round, lot, t));
        }
        bids.sort_by(|a, b| a.0.cmp(&b.0).then(b.1.total_cmp(&a.1)));
        let name = p.name.clone();
        let school = p.college.map_or(String::new(), |c| w.program_name(c).to_string());
        match bids.first().copied() {
            Some((round, _, t)) => {
                crate::world::pro::trades::forfeit_pick(w, t, year + 1, round);
                let pick = 0;
                let p = &mut w.people[i];
                p.stage = Stage::Pro;
                p.note(format!(
                    "Taken by the {} in the {year} supplemental draft out of {school}; they gave up a round {round} pick in {}.",
                    w.nfl[t].name,
                    year + 1
                ));
                let _ = pick;
                let terms = crate::world::pro::money::undrafted_offer(year, crate::world::pro::money::price_cap(year) * 0.004 / round as f64, 0.0);
                crate::world::pro::join_undrafted(w, i, t, year, &terms);
                let text = format!(
                    "Supplemental draft: the {} take {name} ({school}) and give up their {} round {round} pick.",
                    w.nfl[t].name,
                    year + 1
                );
                w.calendar.post(Event::new(d, Level::Nfl, Kind::DraftPick, Loudness::Medium, text).team(t));
                w.scouting.supplemental.push(SuppPick { year, player: id, name, team: Some(t), round: Some(round) });
            }
            None => {
                w.scouting.supplemental.push(SuppPick { year, player: id, name, team: None, round: None });
            }
        }
    }
    w.scouting.pro.supp_bids.clear();
}
