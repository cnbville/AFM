//! Your 53, the practice squad and the waiver wire (lane 10): your cut to
//! 53 (the staff's ranking for whatever you leave open), the four you
//! protect each week, signing players to the squad and up to the 53, and
//! your waiver claims, decided in the league's claim order by the
//! market's own `claims`.

use crate::calendar::Date;
use crate::world::World;
use crate::world::pro::{self, Status, money, squad};

use super::{Duty, Owner};

/// What your cut plan does with a player.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Cut {
    Keep,
    Squad,
    Release,
}

/// Your roster desk.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct RosterDesk {
    /// Your cut plan by player id (players you leave out are the staff's call).
    pub plan: Vec<(u32, Cut)>,
    /// The practice squad players you protect this week.
    pub protect: Vec<u32>,
    /// Your waiver claims: (player id, the player you'd let go to make room).
    pub claims: Vec<(u32, Option<u32>)>,
    /// Players put on the wire, with the day (the claim period), league-wide.
    pub wire: Vec<(u32, Date)>,
}

/// Your cut to 53 on cutdown day, when the cutdown is yours: your plan
/// first, then the staff's ranking for every place you left open.
pub(crate) fn seat_cutdown(w: &World, t: usize, keep: &[(usize, f64)]) -> Option<(Vec<usize>, Vec<usize>, Vec<usize>)> {
    if !super::yours(w, t, Duty::Cutdown) {
        return None;
    }
    let plan = &w.front_office.roster.plan;
    let limit = w.rules.nfl_roster as usize;
    let ps = w.rules.nfl_practice_squad as usize;
    let of = |i: usize| plan.iter().find(|x| x.0 == w.people[i].player.id).map(|x| x.1);
    // The staff's own cut, for the places you left open.
    let (staff53, staff_ps, _) = pro::cut_split(w, keep);
    let mut chosen: Vec<usize> = keep.iter().map(|x| x.0).filter(|&i| of(i) == Some(Cut::Keep)).take(limit).collect();
    for &i in staff53.iter().chain(keep.iter().map(|x| &x.0)) {
        if chosen.len() >= limit {
            break;
        }
        if of(i).is_none() && !chosen.contains(&i) {
            chosen.push(i);
        }
    }
    let (places, most) = squad::veteran_places(w.year);
    let mut vets = 0;
    let mut squad_list: Vec<usize> = Vec::new();
    let eligible = |i: usize, vets: usize| {
        let c = w.people[i].pro.as_ref().unwrap();
        squad::squad_eligible(c, w.people[i].player.age) || (vets < places && c.accrued <= most)
    };
    for &(i, _) in keep {
        if chosen.contains(&i) || of(i) != Some(Cut::Squad) || squad_list.len() >= ps {
            continue;
        }
        if eligible(i, vets) {
            vets += usize::from(!squad::squad_eligible(w.people[i].pro.as_ref().unwrap(), w.people[i].player.age));
            squad_list.push(i);
        }
    }
    for &i in &staff_ps {
        if squad_list.len() >= ps {
            break;
        }
        if !chosen.contains(&i) && of(i).is_none() && !squad_list.contains(&i) && eligible(i, vets) {
            vets += usize::from(!squad::squad_eligible(w.people[i].pro.as_ref().unwrap(), w.people[i].player.age));
            squad_list.push(i);
        }
    }
    let cut: Vec<usize> = keep.iter().map(|x| x.0).filter(|i| !chosen.contains(i) && !squad_list.contains(i)).collect();
    Some((chosen, squad_list, cut))
}

/// Whether a player can go to a practice squad this year.
pub fn squad_ok(w: &World, i: usize) -> bool {
    w.people[i].pro.as_ref().is_some_and(|c| {
        let (places, most) = squad::veteran_places(w.year);
        squad::squad_eligible(c, w.people[i].player.age) || (places > 0 && c.accrued <= most)
    })
}

/// Sets a player's place in your cut plan (None hands him back to the staff).
pub fn set_plan(w: &mut World, id: u32, cut: Option<Cut>) -> Result<(), String> {
    let t = super::club(w).ok_or("You don't run an NFL club.")?;
    let i = super::index_of(w, id).ok_or("No such player.")?;
    if w.people[i].pro.as_ref().and_then(|c| c.team) != Some(t) {
        return Err("He isn't yours.".into());
    }
    if cut == Some(Cut::Squad) && !squad_ok(w, i) {
        return Err("He has too many seasons for the practice squad.".into());
    }
    let plan = &mut w.front_office.roster.plan;
    plan.retain(|x| x.0 != id);
    if let Some(c) = cut {
        plan.push((id, c));
    }
    let keeps = plan.iter().filter(|x| x.1 == Cut::Keep).count();
    if keeps > w.rules.nfl_roster as usize {
        plan.retain(|x| x.0 != id);
        return Err(format!("You already keep {} players: release one first.", keeps - 1));
    }
    w.front_office.set_owner(Duty::Cutdown, Owner::You);
    Ok(())
}

/// The squad players you protect this week, when the squad is yours.
pub(crate) fn protected(w: &World, t: usize) -> Option<Vec<u32>> {
    super::yours(w, t, Duty::PracticeSquad).then(|| w.front_office.roster.protect.clone())
}

/// Protects (or stops protecting) a squad player: four a week from 2020.
pub fn set_protect(w: &mut World, id: u32, on: bool) -> Result<(), String> {
    let year = w.year;
    let n = squad::protected_count(year);
    if n == 0 {
        return Err("Squad players can't be protected before 2020.".into());
    }
    let r = &mut w.front_office.roster.protect;
    r.retain(|&x| x != id);
    if on {
        if r.len() >= n {
            return Err(format!("You protect {n} already."));
        }
        r.push(id);
    }
    w.front_office.set_owner(Duty::PracticeSquad, Owner::You);
    Ok(())
}

/// Signs a free agent to your practice squad for the weeks left.
pub fn sign_to_squad(w: &mut World, id: u32) -> Result<String, String> {
    let t = super::club(w).ok_or("You don't run an NFL club.")?;
    let i = super::index_of(w, id).ok_or("No such player.")?;
    let year = w.year;
    let d = w.today;
    let size = w.rules.nfl_practice_squad as usize;
    if size == 0 {
        return Err("There are no practice squads in this era.".into());
    }
    if d < w.dates.cutdown.date || d > w.dates.super_bowl {
        return Err("Practice squads run from cutdown day to the end of the season.".into());
    }
    let c = w.people[i].pro.as_ref().ok_or("Not a pro.")?;
    if c.status != Status::FreeAgent || c.on_waivers {
        return Err("He isn't a free agent.".into());
    }
    if !squad::squad_room(w, t, i) {
        return Err("He isn't eligible for your squad (seasons, or no veteran place left).".into());
    }
    if pro::count(w, t, Status::PracticeSquad) >= size {
        return Err(format!("Your squad is full ({size})."));
    }
    let weeks_left = ((w.dates.super_bowl.0 - d.0) as f64 / 7.0).clamp(1.0, w.rules.nfl_games as f64 + 1.0);
    let pay = money::practice_squad_week(year) * weeks_left;
    let deal = money::build(t, money::Kind::PracticeSquad, year, &money::Terms::flat(1, pay));
    let name = pro::possessive(w.nfl[t].name);
    let p = &mut w.people[i];
    let c = p.pro.as_mut().unwrap();
    c.team = Some(t);
    c.status = Status::PracticeSquad;
    c.contract = Some(deal);
    c.ps_seasons += 1;
    c.weeks_unsigned = 0;
    if c.teams.last() != Some(&t) {
        c.teams.push(t);
        c.new_team = true;
    }
    let age = p.row_age();
    p.note(format!("Signed to the {name} practice squad at {age}."));
    w.clubs[t].roster.push(i);
    let pname = w.people[i].name.clone();
    let text = format!("Signed {pname} to the practice squad.");
    super::note(w, text.clone(), Some(id));
    Ok(text)
}

/// Signs one of your squad players up to the 53 (a minimum deal through
/// next season, as most are).
pub fn sign_up(w: &mut World, id: u32) -> Result<String, String> {
    let t = super::club(w).ok_or("You don't run an NFL club.")?;
    let i = super::index_of(w, id).ok_or("No such player.")?;
    let year = w.year;
    let c = w.people[i].pro.as_ref().ok_or("Not a pro.")?;
    if c.team != Some(t) || c.status != Status::PracticeSquad {
        return Err("He isn't on your practice squad.".into());
    }
    let limit = roster_limit(w);
    if pro::count(w, t, Status::Roster) >= limit {
        return Err(format!("Your roster is at {limit}: release someone first."));
    }
    let accrued = c.accrued;
    let apy = money::minimum(year, accrued);
    if money::cap_books(year) && pro::cap_room(w, t, year) < apy {
        return Err("No cap room for the minimum.".into());
    }
    let deal = money::build(t, money::Kind::Veteran, year, &money::Terms::flat(2, apy));
    let p = &mut w.people[i];
    let c = p.pro.as_mut().unwrap();
    c.status = Status::Roster;
    c.contract = Some(deal);
    let age = p.row_age();
    p.note(format!("Signed up from the practice squad to the 53 at {age}."));
    w.market.year_mut(year).signed_up += 1;
    let pname = w.people[i].name.clone();
    let text = format!("Signed {pname} up to the 53.");
    super::note(w, text.clone(), Some(id));
    Ok(text)
}

/// Marks a player put on the wire today (every club's), so a career can
/// see him for the claim period.
pub(crate) fn waived(fo: &mut super::FrontOffice, id: u32, d: Date) {
    fo.roster.wire.retain(|x| x.1.days_to(d) <= 2);
    fo.roster.wire.push((id, d));
}

/// The players waived today: in a career they wait a day on the wire, as
/// the real claim period (till 4 p.m. the next day) has it.
pub(crate) fn held_on_wire(w: &World, d: Date) -> Vec<u32> {
    if w.career.is_none() || super::club(w).is_none() {
        return Vec::new();
    }
    w.front_office.roster.wire.iter().filter(|x| x.1 == d).map(|x| x.0).collect()
}

/// Your claim on a waived player, when it is your turn in the order:
/// Some(the player you let go) if you claimed him and can take his deal.
pub(crate) fn seat_claim(w: &World, t: usize, i: usize, limit: usize) -> Option<Option<usize>> {
    let id = w.people[i].player.id;
    let &(_, drop) = w.front_office.roster.claims.iter().find(|x| x.0 == id)?;
    let year = w.year;
    let cost = w.people[i].pro.as_ref()?.contract.as_ref().map_or(0.0, |k| k.cap_hit(year));
    if money::cap_books(year) && pro::cap_room(w, t, year) < cost {
        return None;
    }
    let full = pro::count(w, t, Status::Roster) >= limit;
    let drop =
        drop.and_then(|d| super::index_of(w, d)).filter(|&j| w.people[j].pro.as_ref().and_then(|c| c.team) == Some(t));
    if full && drop.is_none() {
        return None;
    }
    Some(if full { drop } else { None })
}

/// Puts in (or takes back) a waiver claim.
pub fn claim(w: &mut World, id: u32, drop: Option<u32>, on: bool) -> Result<(), String> {
    let _t = super::club(w).ok_or("You don't run an NFL club.")?;
    let i = super::index_of(w, id).ok_or("No such player.")?;
    if on && !w.people[i].pro.as_ref().is_some_and(|c| c.on_waivers) {
        return Err("He isn't on waivers.".into());
    }
    w.front_office.roster.claims.retain(|x| x.0 != id);
    if on {
        w.front_office.roster.claims.push((id, drop));
    }
    w.front_office.set_owner(Duty::Waivers, Owner::You);
    Ok(())
}

/// The desk's day: claims on players who left the wire are done; the cut
/// plan is cleared once the cutdown has passed.
pub(crate) fn day(w: &mut World, d: Date) {
    if d == w.dates.cutdown.date.add(1) {
        w.front_office.roster.plan.clear();
    }
    let ids: Vec<u32> = w.front_office.roster.claims.iter().map(|x| x.0).collect();
    if ids.is_empty() {
        return;
    }
    let at = super::indices(w, &ids);
    let t = super::club(w);
    let mut told: Vec<(usize, String)> = Vec::new();
    let mut keep: Vec<(u32, Option<u32>)> = Vec::new();
    for (id, drop) in w.front_office.roster.claims.clone() {
        let Some(&i) = at.get(&id) else { continue };
        let c = w.people[i].pro.as_ref();
        if c.is_some_and(|c| c.on_waivers) {
            keep.push((id, drop));
            continue;
        }
        let name = w.people[i].name.clone();
        let won = t.is_some() && c.and_then(|c| c.team) == t;
        told.push((
            i,
            if won {
                format!("Your claim on {name} came through: he's yours.")
            } else {
                match c.and_then(|c| c.team) {
                    Some(o) => format!("The {} were ahead of you in the order and claimed {name}.", w.nfl[o].name),
                    None => format!("{name} cleared waivers: your claim lapsed."),
                }
            },
        ));
    }
    w.front_office.roster.claims = keep;
    for (i, text) in told {
        let id = w.people[i].player.id;
        super::note(w, text.clone(), Some(id));
        super::tell_about(w, d, text, false, &[i]);
    }
}

/// Your staff's ranking for the cut (best first, with each player's
/// score) and the cut it would make: (ranking, the 53, the squad).
pub fn staff_cut(w: &World, t: usize) -> (Vec<(usize, f64)>, Vec<usize>, Vec<usize>) {
    let keep = pro::cut_scores(w, t);
    let (a, b, _) = pro::cut_split(w, &keep);
    (keep, a, b)
}

/// The roster limit today for your moves: 90 from the football year's
/// start (the day after the Super Bowl: reserve/future deals count) to the
/// cutdown, 53 in the season.
pub fn roster_limit(w: &World) -> usize {
    if w.today < w.dates.cutdown.date { w.rules.nfl_offseason as usize } else { w.rules.nfl_roster as usize }
}

/// Where your club stands in today's waiver order (1 is first).
pub fn waiver_place(w: &World, t: usize) -> Option<usize> {
    pro::market::waiver_order(w, w.today).iter().position(|&x| x == t).map(|k| k + 1)
}

/// Players a club holds with a status.
pub fn count(w: &World, t: usize, s: Status) -> usize {
    pro::count(w, t, s)
}
