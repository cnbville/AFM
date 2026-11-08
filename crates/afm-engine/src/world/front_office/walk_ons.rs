//! Walk-ons at your college program (lane B of the Jan 1 start): the open
//! tryout you hold and the walk-ons you put on scholarship. The AI's head
//! coaches do both at fall camp (`recruiting::camp_walk_ons`); at your
//! program they are your calls, and your staff names the walk-ons it thinks
//! earned a scholarship.

use crate::calendar::Date;
use crate::player::PlayingTime;
use crate::world::data::Division;
use crate::world::{Stage, World, key_normal, key_uniform, ncaa, recruiting as rec};

use super::{index_of, note, program, tell_about};

/// Your open tryout this season: the day you held it, who came out with
/// your staff's grade (player id, low, high, forty timed that day) and who
/// you kept.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Tryout {
    pub season: i32,
    pub date: Date,
    pub came: Vec<TryoutRow>,
    pub kept: Vec<u32>,
}

#[derive(Clone, Copy, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct TryoutRow {
    pub id: u32,
    /// Your staff's grade of what he is now: a range on the overall scale.
    pub low: f32,
    pub high: f32,
    /// His 40 at the tryout, hand-timed.
    pub forty: f32,
}

/// How many students come out and are worth a look at a tryout (proposed,
/// not confirmed: a real open tryout draws dozens, a dozen of them played
/// high school football well enough for a staff to time).
pub const TRYOUT_LOOKS: usize = 12;

/// The share of the region's unsigned new graduates who enroll and try
/// out (proposed, not confirmed).
const SHOWS_UP: f64 = 0.35;

/// How far over a full squad for its level a staff will carry walk-ons
/// where no limit counts them (proposed, not confirmed: about 15%).
const SQUAD_OVER: f64 = 1.15;

/// A walk-on who played his way into the two-deep here last season: his
/// last role counts only from a season at this program, not his high
/// school's or another college's.
pub fn in_two_deep(w: &World, i: usize, j: usize) -> bool {
    let p = &w.people[i];
    matches!(p.last_role, Some(PlayingTime::Starter | PlayingTime::Rotation))
        && p.college_staff.last().is_some_and(|s| s.1 == w.programs[j].id)
}

/// The days you can hold your tryout: from summer enrollment, when the
/// students who never signed are on campus, to about the first week of
/// fall classes, three weeks into camp (proposed, not confirmed; real
/// open tryouts come at the start of a term).
pub fn window(w: &World) -> (Date, Date) {
    (w.dates.summer_enrollment, w.dates.fall_camp.add(21))
}

/// Your roster's room: (players, the most it can carry, under the House
/// limit). Under the House limit every player counts against it; elsewhere
/// the walk-ons sit outside the scholarship limit, up to a full squad
/// and a bit.
pub fn squad_room(w: &World, j: usize) -> (usize, usize, bool) {
    let prog = &w.programs[j];
    let season = w.dates.season;
    let whole = ncaa::whole_roster_limit(prog, season);
    let here = |i: &&usize| w.people[**i].stage == Stage::College && w.people[**i].college == Some(j);
    if whole {
        let n = prog.roster.iter().filter(here).filter(|&&i| rec::takes_a_place(&w.people[i], true, season)).count();
        (n, ncaa::roster_limits(prog, season).0, true)
    } else {
        let n = prog.roster.iter().filter(here).count();
        (n, (rec::level_roster(prog.division) * SQUAD_OVER).round() as usize, false)
    }
}

/// The scholarships: (on aid, the limit), or the reason your school gives
/// none. Under the House limit every place can carry aid.
pub fn aid_room(w: &World, j: usize) -> Result<(usize, usize), String> {
    let prog = &w.programs[j];
    if prog.division == Division::D3 {
        return Err("Division III gives no athletic aid.".into());
    }
    let season = w.dates.season;
    if ncaa::aid(prog, season) != ncaa::Aid::Athletic {
        return Err(format!("{} gives no athletic scholarships.", prog.real_name));
    }
    let limit = ncaa::roster_limits(prog, season).0;
    let on_aid = prog
        .roster
        .iter()
        .filter(|&&i| {
            let p = &w.people[i];
            p.stage == Stage::College && p.college == Some(j) && !p.walk_on
        })
        .count();
    Ok((on_aid, limit))
}

fn yours(w: &World, id: u32) -> Result<(usize, usize), String> {
    let j = program(w).ok_or("You have no college head job.")?;
    let i = index_of(w, id).ok_or("No such player.")?;
    let p = &w.people[i];
    if p.stage != Stage::College || p.college != Some(j) || p.cut_here().is_some() {
        return Err("He isn't on your roster.".into());
    }
    Ok((i, j))
}

/// What putting him on scholarship does, or why you can't.
pub fn scholarship_words(w: &World, id: u32) -> Result<String, String> {
    let (i, j) = yours(w, id)?;
    let p = &w.people[i];
    if !p.walk_on {
        return Err("He's on scholarship already.".into());
    }
    if p.cal.portal.is_some() {
        return Err("He's in the portal.".into());
    }
    let (on_aid, limit) = aid_room(w, j)?;
    let season = w.dates.season;
    let whole = ncaa::whole_roster_limit(&w.programs[j], season);
    if !whole && on_aid >= limit {
        return Err(format!("All {limit} of your scholarships are taken."));
    }
    // A designated walk-on of the House settlement sits outside the limit
    // only while he pays his own way: on aid he counts against it.
    if whole && !rec::takes_a_place(p, true, season) {
        let (n, cap, _) = squad_room(w, j);
        if n >= cap {
            return Err(format!("Your roster is at the {cap}-player limit: on aid he would count against it."));
        }
    }
    let left = if whole {
        format!("Under the House limit every one of your {limit} places can carry aid; {on_aid} do now.")
    } else {
        format!("He takes one of your {limit} scholarships: {} left after him.", limit - on_aid - 1)
    };
    let pays = if ncaa::aid_kept(&w.programs[j], w.dates.season) {
        "He keeps it while he stays in school (the power conferences' 2015 rule)."
    } else {
        "It's a one-year award you can renew."
    };
    Ok(format!("He stops paying his own way from this term. {left} {pays}"))
}

/// Puts one of your walk-ons on scholarship (lane B): any day, the way
/// real head coaches do it, often in front of the team in fall camp.
pub fn award(w: &mut World, id: u32) -> Result<String, String> {
    let words = scholarship_words(w, id)?;
    let (i, j) = yours(w, id)?;
    let name = w.programs[j].real_name.clone();
    let fbs = w.programs[j].division == Division::FBS;
    let today = w.today;
    let (who, pos, starter) = {
        let p = &mut w.people[i];
        p.walk_on = false;
        let age = p.row_age();
        p.note(format!("Earned a scholarship at {name} at {age}: his head coach put the former walk-on on aid."));
        (p.name.clone(), p.player.position, p.last_role == Some(PlayingTime::Starter))
    };
    let starter = starter && in_two_deep(w, i, j);
    w.log.walk_on_scholarships += 1;
    if fbs && starter {
        let text = format!("{name} puts former walk-on {who} ({pos}) on scholarship.");
        let e = crate::calendar::Event::new(
            today,
            crate::calendar::Level::College,
            crate::calendar::Kind::Signing,
            crate::calendar::Loudness::Small,
            text,
        )
        .program(j)
        .person(id, &who)
        .school_link(j, &name);
        w.calendar.post(e);
    }
    let text = format!("{who} is on scholarship. {words}");
    note(w, text.clone(), Some(id));
    Ok(text)
}

/// Why you can't hold a tryout today, if you can't.
pub fn tryout_rule(w: &World) -> Result<usize, String> {
    let j = program(w).ok_or("You have no college head job.")?;
    let (from, to) = window(w);
    if w.today < from || w.today > to {
        return Err(format!(
            "Open tryouts run from summer enrollment ({}) to the first weeks of fall classes ({}).",
            from.pretty(),
            to.pretty()
        ));
    }
    if w.late_signing.is_some() {
        return Err("The last late signing day closes at office hours today: hold your tryout after it.".into());
    }
    if w.front_office.college.tryout.as_ref().is_some_and(|t| t.season == w.dates.season) {
        return Err("You held this year's tryout already.".into());
    }
    if !w.programs[j].plays(w.dates.season) {
        return Err("Your school fields no team this season.".into());
    }
    Ok(j)
}

/// The students who could come out: new graduates who never signed and
/// junior college players nobody took this year, from your region.
fn students(w: &World, j: usize) -> Vec<usize> {
    let prog = &w.programs[j];
    let region = super::super::program::region(prog);
    let season = w.dates.season;
    let mut v: Vec<usize> = w
        .people
        .iter()
        .enumerate()
        .filter(|(_, p)| {
            let fresh = p.class_of == season && p.path.is_empty() && p.college.is_none();
            let juco = p.juco && p.college.is_some_and(|c| w.programs[c].division == Division::JUCO);
            p.stage == Stage::Done
                && (fresh || juco)
                && p.pro.is_none()
                && (w.high_schools[p.hs].state == prog.state || region[p.home_state] >= 0.5)
        })
        .map(|(i, _)| i)
        .collect();
    v.sort_by(|&a, &b| w.people[b].player.overall().total_cmp(&w.people[a].player.overall()).then(a.cmp(&b)));
    v
}

/// Holds your open tryout today (lane B): about a dozen students worth a
/// look come out, your staff times them and grades what they are now, and
/// you keep whoever you want while the roster has room.
pub fn hold_tryout(w: &mut World) -> Result<String, String> {
    let j = tryout_rule(w)?;
    let season = w.dates.season;
    let seed = w.career.as_ref().map_or(0, |c| c.seed as u64) ^ 0x7A1C_0000 ^ (season as u64) << 8;
    let prog = &w.programs[j];
    // Your staff's eye: the position coach's group and yours.
    let hc_eye = prog.head_coach.map_or(10.0, |c| w.coaches[c].evaluation());
    let mut came: Vec<TryoutRow> = Vec::new();
    for i in students(w, j) {
        if came.len() >= TRYOUT_LOOKS {
            break;
        }
        let p = &w.people[i];
        let id = p.player.id as u64;
        if key_uniform(seed, id) >= SHOWS_UP {
            continue;
        }
        let group = crate::world::coach::Group::of(p.player.position) as usize;
        let pc_eye = prog.position_coaches[group].map_or(hc_eye, |c| w.coaches[c].evaluation());
        let eye = 0.6 * pc_eye + 0.4 * hc_eye;
        let sd = (3.5 - 0.15 * (eye - 10.0)).clamp(1.0, 6.0);
        let centre = p.player.overall() + sd * key_normal(seed ^ 0x61, id);
        let half = 1.5 + 0.6 * sd;
        let mut rng = crate::rng::Rng::new(((seed ^ id) & 0xFFFF_FFFF) as u32);
        let forty = crate::body::combine(&p.player, &mut rng).forty + 0.05 * key_normal(seed ^ 0x40, id);
        came.push(TryoutRow {
            id: p.player.id,
            low: (centre - half) as f32,
            high: (centre + half) as f32,
            forty: forty as f32,
        });
    }
    let n = came.len();
    w.front_office.college.tryout = Some(Tryout { season, date: w.today, came, kept: Vec::new() });
    let text = match n {
        0 => "Nobody worth a second look came out to your tryout.".to_string(),
        1 => "One student worth a look came out to your tryout. Your staff timed and graded him.".to_string(),
        n => format!("{n} students worth a look came out to your tryout. Your staff timed and graded them."),
    };
    note(w, text.clone(), None);
    Ok(text)
}

/// Keeps one of the students from your tryout as a walk-on.
pub fn keep(w: &mut World, id: u32) -> Result<String, String> {
    let j = program(w).ok_or("You have no college head job.")?;
    let season = w.dates.season;
    let t = w.front_office.college.tryout.as_ref().filter(|t| t.season == season).ok_or("Hold your tryout first.")?;
    if !t.came.iter().any(|r| r.id == id) {
        return Err("He didn't come out to your tryout.".into());
    }
    if t.kept.contains(&id) {
        return Err("He's on your team already.".into());
    }
    let (_, to) = window(w);
    if w.today > to {
        return Err(format!("The tryout's students had to be kept by {}.", to.pretty()));
    }
    let (n, cap, whole) = squad_room(w, j);
    if n >= cap {
        return Err(if whole {
            format!("Your roster is at the {cap}-player limit.")
        } else {
            format!("Your squad is full at {n}: a staff carries about {cap} at most.")
        });
    }
    let i = index_of(w, id).ok_or("No such player.")?;
    if w.people[i].stage != Stage::Done {
        return Err("He has gone elsewhere.".into());
    }
    let name = w.programs[j].real_name.clone();
    {
        let p = &mut w.people[i];
        p.stage = Stage::College;
        p.college = Some(j);
        p.path.push(j);
        p.walk_on = true;
        p.cal.late = false;
        if p.juco {
            p.college_seasons = p.college_seasons.min(2);
        } else {
            p.college_seasons = 0;
            p.redshirt_used = false;
        }
        let age = p.row_age();
        p.note(format!("Made the team at {name} at {age} at an open tryout its head coach held."));
    }
    let at = w.programs[j].roster.partition_point(|&x| x < i);
    w.programs[j].roster.insert(at, i);
    w.log.tryouts += 1;
    if let Some(t) = w.front_office.college.tryout.as_mut() {
        t.kept.push(id);
    }
    let who = w.people[i].name.clone();
    let pos = w.people[i].player.position;
    let text = format!("{who} ({pos}) made your team as a walk-on.");
    note(w, text.clone(), Some(id));
    Ok(text)
}

/// Fall camp at your program (lane B): the AI's head coaches put the
/// walk-ons who played their way into the two-deep on scholarship; yours
/// names them and leaves the call to you.
pub(crate) fn camp_word(w: &mut World, earned: &[usize], free: usize) {
    if earned.is_empty() {
        return;
    }
    let names: Vec<String> = earned
        .iter()
        .map(|&i| {
            let p = &w.people[i];
            format!("{} ({})", p.name, p.player.position)
        })
        .collect();
    let list = match names.len() {
        1 => names[0].clone(),
        n => format!("{} and {}", names[..n - 1].join(", "), names[n - 1]),
    };
    let room = if free == usize::MAX {
        "Every place on your roster can carry aid.".to_string()
    } else {
        format!("You have {free} scholarship{} free.", if free == 1 { "" } else { "s" })
    };
    let text = format!(
        "Your staff says {list} played their way into the two-deep as walk-ons and earned a scholarship. {room} The call is yours, from their player pages or the Walk-ons screen."
    );
    let today = w.today;
    tell_about(w, today, text, false, earned);
}
