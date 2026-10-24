//! Stories change the world (S4), always through people and always as one
//! influence among many, never a flat bonus:
//! - Mood: heat on a player weighs on him through his ego and stability (a
//!   calm veteran shrugs, a fragile kid presses); good stories lift.
//! - The locker room: a battle or a rift splits the room the longer it
//!   runs; a clean ending heals it.
//! - Recruits read the news: a scandal or a hot seat is rivals' material
//!   (negative recruiting), a rising program's story helps
//!   (`Program::story_push`, read by every recruit's appeal).
//! - The boss: public pressure moves his patience, more for a boss who
//!   listens to boosters and fans (`review_push`, read by the review).
//! - Fans: the big stories move the fans' mood, which drives the gate and
//!   the boosters' money.
//! - The cast's own choices: the quarterback who lost the job leans toward
//!   the portal by his personality and how it went.
//!
//! Every size here is Claude's pick (proposed, not confirmed).

use super::{Family, K, Part, Story, Who};
use crate::calendar::Date;
use crate::player::Trait;
use crate::world::World;
use crate::world::folk::talks::{self, Reason};
use crate::world::stories::TeamKey;

/// The weekly effects of the open public stories.
pub(crate) fn week(w: &mut World, _d: Date) {
    let stories: Vec<Story> =
        w.storylines.open.iter().filter(|s| s.public && s.phase.open() && s.heat >= 15.0).cloned().collect();
    let mut push = vec![0.0f32; w.programs.len()];
    for s in &stories {
        mood(w, s);
        room(w, s);
        fans(w, s);
        if let Some(TeamKey::College(p)) = s.team {
            push[p as usize] += recruit_read(s);
        }
    }
    // A program's recruiting read eases back when the stories end.
    for (k, p) in w.programs.iter_mut().enumerate() {
        let now = push[k].clamp(-0.05, 0.035);
        p.story_push = 0.5 * p.story_push + 0.5 * now;
        if p.story_push.abs() < 0.0005 {
            p.story_push = 0.0;
        }
    }
    w.storylines.push = push;
}

/// How a story reads to a recruit choosing between schools.
fn recruit_read(s: &Story) -> f32 {
    use K::*;
    let kind = K::ALL.get(s.kind as usize).copied();
    let sign = match kind {
        Some(
            HotSeat | BuyoutWatch | VoteOfConfidence | NcaaCase | PortalExodus | Graveyard | LosingStreak | FanRevolt
            | Arrest | RoomRift | ChoppingBlock | Decline,
        ) => -1.0,
        Some(
            Unbeaten | ProgramRise | Turnaround | Dynasty | PlayoffPush | TitleDefense | AwardRace | FacilityPush
            | ConferenceRace,
        ) => 1.0,
        Some(NflWantsCoach | BigJobWantsCoach | JobRumors) => -0.6,
        _ => 0.0,
    };
    sign * 0.022 * s.heat / 100.0
}

/// Heat on the people in it: a player's ego and stability decide whether
/// the noise gets to him.
fn mood(w: &mut World, s: &Story) {
    let good = s.def().tone > 0;
    let mine = s.you.is_some();
    for m in s.cast.iter().filter(|m| matches!(m.part, Part::Lead | Part::Rival)) {
        let Who::Player(id) = m.who else { continue };
        let Some(i) = super::pi(w, id) else { continue };
        let p = &w.people[i];
        let ego = p.player.personality.traits.get(Trait::Ego) as f32;
        let steady = p.bg.stability as f32;
        // How much the noise reaches him: a fragile kid presses, a calm
        // veteran shrugs.
        let reach = (1.15 - (steady - 10.0) / 12.0).clamp(0.4, 1.7);
        let losing = match (m.part, s.family()) {
            // In a battle the public's lean decides who feels the noise.
            (Part::Lead, Family::Battle) => s.belief > 0.2,
            (Part::Rival, Family::Battle) => s.belief < -0.2,
            _ => !good,
        };
        let size = 0.05 * s.heat / 100.0 * reach;
        let delta = if losing {
            -size * (1.0 + (ego - 10.0).max(0.0) / 15.0)
        } else if good || matches!(s.family(), Family::Battle) {
            0.6 * size
        } else {
            0.0
        };
        if delta.abs() < 0.004 {
            continue;
        }
        if mine && w.people[i].college.is_some() || mine && w.people[i].pro.is_some() {
            talks::mood_event(w, id, Reason::Noise, delta);
        } else {
            let p = &mut w.people[i];
            p.mood = (p.mood + 0.6 * delta).clamp(-1.0, 1.0);
        }
    }
}

/// A battle or a rift splits the room the longer it runs.
fn room(w: &mut World, s: &Story) {
    let split = match s.family() {
        Family::Battle => 0.006,
        _ if s.kind == K::RoomRift.id() || s.kind == K::PublicFeud.id() => 0.012,
        Family::Seat => 0.003,
        _ => return,
    };
    let by = -(split * s.heat / 100.0) * (1.0 + s.age(w.today) as f32 / 60.0).min(2.0);
    match s.team {
        Some(TeamKey::College(p)) => {
            let m = &mut w.programs[p as usize].team_mood;
            *m = (*m + by).clamp(-1.0, 1.0);
        }
        Some(TeamKey::Nfl(n)) => {
            if let Some(c) = w.clubs.get_mut(n as usize) {
                c.team_mood = (c.team_mood + by).clamp(-1.0, 1.0);
            }
        }
        _ => {}
    }
}

/// The big stories move the fans' mood.
fn fans(w: &mut World, s: &Story) {
    if s.heat < 35.0 {
        return;
    }
    let tone = s.def().tone as f64;
    if tone == 0.0 {
        return;
    }
    let by = 0.008 * tone * s.heat as f64 / 100.0;
    match s.team {
        Some(TeamKey::Nfl(n)) => {
            if let Some(c) = w.clubs.get_mut(n as usize) {
                c.office.fans = (c.office.fans + by).clamp(-1.0, 1.0);
            }
        }
        Some(TeamKey::College(p)) => {
            let f = &mut w.programs[p as usize].fans;
            *f = (*f + 0.15 * by).clamp(0.0, 1.0);
        }
        _ => {}
    }
}

/// What an ending does to the people in it: the room heals or breaks, the
/// loser of a battle weighs leaving, a grudge stays.
pub(crate) fn ended(w: &mut World, id: u32) {
    let Some(s) = w.storylines.get(id).cloned() else { return };
    let v = s.valence();
    // A clean ending heals the room; a bad one leaves a mark.
    let heal = match s.family() {
        Family::Battle | Family::OffField => 0.02 * v,
        _ if s.kind == K::RoomRift.id() => 0.08 * v,
        _ => 0.0,
    };
    if heal != 0.0 {
        match s.team {
            Some(TeamKey::College(p)) => {
                let m = &mut w.programs[p as usize].team_mood;
                *m = (*m + heal).clamp(-1.0, 1.0);
            }
            Some(TeamKey::Nfl(n)) => {
                if let Some(c) = w.clubs.get_mut(n as usize) {
                    c.team_mood = (c.team_mood + heal).clamp(-1.0, 1.0);
                }
            }
            _ => {}
        }
    }
    // The battle's loser: stay, sulk or go, by who he is and how it went
    // (who backed him, what was said in public).
    if s.family() == Family::Battle {
        let ending = s.ending_key();
        let loser = match ending {
            "won_job" | "benched" | "lost_job" => s.lead_player(),
            "kept_job" | "held_job" | "won_back" => s.rival_player(),
            _ => None,
        };
        if let Some(id) = loser
            && let Some(i) = super::pi(w, id)
            && w.people[i].college.is_some()
        {
            let p = &w.people[i];
            let t = &p.player.personality.traits;
            let ego = t.get(Trait::Ego);
            let loyal = t.get(Trait::Loyalty);
            let comp = p.player.personality.competitiveness;
            let backed = s.cast.iter().find(|m| m.who == Who::Player(id)).map_or(0, |m| m.side);
            let lean = (0.08 + 0.012 * (ego - 10.0) + 0.01 * (comp - 10.0) - 0.012 * (loyal - 10.0)
                + if backed < 0 { 0.03 } else { 0.0 }
                + if s.stance != 0 { 0.02 } else { 0.0 })
            .clamp(0.0, 0.3);
            let today = w.today;
            let p = &mut w.people[i];
            p.cal.lean = (p.cal.lean + lean as f64).min(0.95);
            p.cal.lean_events.push((today, "lost a position battle", lean as f64));
            let hit = -0.05 - 0.01 * (ego - 10.0).max(0.0) as f32;
            if s.you.is_some() {
                talks::mood_event(w, id, Reason::Benched, hit);
            } else {
                p.mood = (p.mood + hit).clamp(-1.0, 1.0);
            }
        }
    }
}

/// Public pressure on a coach's review (read by the college and high
/// school reviews): an open, public hot seat or fan revolt moves the
/// boss's read by up to about a third of a tenth of a season, more for a
/// boss with less judgement of his own; for you, the hazard premium's
/// patience in a volatile job's first two seasons pulls the other way.
pub fn review_push(w: &World, c: usize) -> f64 {
    let Some(team) =
        w.coaches.get(c).and_then(|co| co.job).map(|j| crate::world::folk::Team::from_employer(j.employer))
    else {
        return 0.0;
    };
    let t = super::team_key(team);
    let judgement = match t {
        TeamKey::College(p) => w.programs[p as usize].ad.as_ref().map_or(10.0, |a| a.judgement),
        _ => 10.0,
    };
    let mut heat = 0.0f64;
    for s in w.storylines.open.iter().filter(|s| s.public && s.team == Some(t)) {
        let k = K::ALL.get(s.kind as usize).copied();
        if matches!(k, Some(K::HotSeat | K::FanRevolt | K::BuyoutWatch)) && s.lead() == Some(Who::Coach(c as u32))
            || matches!(k, Some(K::FanRevolt))
        {
            heat = heat.max(s.heat as f64 * (1.0 - s.sides[1] as f64));
        }
    }
    let pressure = 0.03 * heat / 100.0 * (1.0 + (10.0 - judgement) / 10.0).clamp(0.5, 1.8);
    pressure.min(0.04) + super::credit::patience(w, c)
}
