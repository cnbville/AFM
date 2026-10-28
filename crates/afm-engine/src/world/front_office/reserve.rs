//! Injured reserve for your club (minor bug 73). A real club decides it:
//! the list frees a 53-man spot but sits the player at least the league's
//! minimum (`ir_minimum`: six weeks before 2020, four from 2020), and each
//! return uses one of the season's designations (`ir_returns`: eight from
//! 2022, none before 2012). Every other club's staff decides by the
//! engine's rule (`pro::games`); yours recommends and you decide, unless
//! you hand the duty to your staff.

use super::{Duty, tell_about};
use crate::world::World;
use crate::world::pro::{self, Status, games};

/// Whether you decide this player's injured reserve moves today: he is on
/// your club and the duty is yours.
pub(crate) fn you_decide(w: &World, i: usize) -> bool {
    w.people[i].pro.as_ref().and_then(|c| c.team).is_some_and(|t| super::yours(w, t, Duty::InjuredReserve))
}

/// Your medical staff's word on a player out `weeks`: injured reserve or
/// keep him on the 53. A decision in your inbox.
pub(crate) fn recommend(w: &mut World, i: usize, weeks: u32) {
    let year = w.year;
    let min = games::ir_minimum(year);
    let returns = games::ir_returns(year);
    let name = w.people[i].name.clone();
    let pos = w.people[i].player.position.name();
    let back = if returns == 0 {
        "it ends his season (no player comes back from it before 2012)".to_string()
    } else {
        format!("he'd sit at least {min} games and coming back uses one of your {returns} returns")
    };
    let text = if weeks >= min {
        format!(
            "Your medical staff recommends injured reserve for {pos} {name}: out about {weeks} weeks. The list frees his spot on the 53; {back}."
        )
    } else {
        format!(
            "Your medical staff expects {pos} {name} back in about {weeks} weeks: keep him on the 53. Injured reserve would free his spot, but {back}."
        )
    };
    let d = w.today;
    tell_about(w, d, text, weeks >= min, &[i]);
}

/// What putting him on injured reserve would mean today, or why you can't.
pub fn place_words(w: &World, id: u32) -> Result<String, String> {
    let (i, _) = yours(w, id)?;
    let c = w.people[i].pro.as_ref().unwrap();
    if c.status == Status::Reserve {
        return Err("He's on injured reserve already.".into());
    }
    if c.status != Status::Roster {
        return Err("Only a player on your roster goes on injured reserve.".into());
    }
    if c.out_weeks == 0 {
        return Err("He isn't hurt.".into());
    }
    let year = w.year;
    let min = games::ir_minimum(year);
    let weeks = c.out_weeks;
    Ok(if games::ir_returns(year) == 0 {
        format!("He's out about {weeks} weeks. Injured reserve frees his spot on the 53 and ends his season.")
    } else if weeks < min {
        format!(
            "He's out about {weeks} weeks, but the list sits him at least {min} games. It frees his spot on the 53; bringing him back uses a return."
        )
    } else {
        format!(
            "He's out about {weeks} weeks. Injured reserve frees his spot on the 53; bringing him back uses one of this season's returns."
        )
    })
}

/// Puts your player on injured reserve. He sits at least the minimum.
pub fn place(w: &mut World, id: u32) -> Result<String, String> {
    place_words(w, id)?;
    let (i, _) = yours(w, id)?;
    let min = games::ir_minimum(w.year);
    let c = w.people[i].pro.as_mut().unwrap();
    c.status = Status::Reserve;
    c.pup = false;
    // The list's minimum stay: he can't come back sooner, healed or not.
    c.out_weeks = c.out_weeks.max(min);
    let weeks = c.out_weeks;
    let p = &mut w.people[i];
    let age = p.row_age();
    let name = p.name.clone();
    p.note(format!("Placed on injured reserve at {age} (out at least {weeks} weeks)."));
    let text = format!("{name} is on injured reserve: his spot on the 53 is open.");
    super::note(w, text.clone(), Some(id));
    Ok(text)
}

/// What bringing him back from injured reserve would mean, or why not.
pub fn activate_words(w: &World, id: u32) -> Result<String, String> {
    let (i, t) = yours(w, id)?;
    let c = w.people[i].pro.as_ref().unwrap();
    if c.status != Status::Reserve {
        return Err("He isn't on injured reserve.".into());
    }
    if c.out_weeks > 0 {
        return Err(format!(
            "He can't come back for {} more week{}.",
            c.out_weeks,
            if c.out_weeks == 1 { "" } else { "s" }
        ));
    }
    let year = w.year;
    let left = games::ir_returns(year).saturating_sub(w.clubs[t].ir_used);
    if !c.pup && left == 0 {
        return Err(match games::ir_returns(year) {
            0 => "Before 2012 injured reserve ends a player's season.".to_string(),
            n => format!("You've used all {n} returns from injured reserve this season."),
        });
    }
    if pro::count(w, t, Status::Roster) >= w.rules.nfl_roster as usize {
        return Err(format!("Your roster is full ({}): release or move a player first.", w.rules.nfl_roster));
    }
    Ok(if c.pup {
        "He's healed: back on the roster off the physically unable to perform list.".to_string()
    } else {
        format!("He's healed. Bringing him back uses one of your {left} returns left this season.")
    })
}

/// Brings your healed player back from injured reserve.
pub fn activate(w: &mut World, id: u32) -> Result<String, String> {
    activate_words(w, id)?;
    let (i, t) = yours(w, id)?;
    let c = w.people[i].pro.as_mut().unwrap();
    if !c.pup {
        w.clubs[t].ir_used += 1;
    }
    let c = w.people[i].pro.as_mut().unwrap();
    c.status = Status::Roster;
    c.pup = false;
    let name = w.people[i].name.clone();
    let text = format!("{name} is back from injured reserve.");
    super::note(w, text.clone(), Some(id));
    Ok(text)
}

/// Your healed player waits on reserve for your word: a line in your inbox.
pub(crate) fn healed(w: &mut World, i: usize) {
    let t = w.people[i].pro.as_ref().and_then(|c| c.team).unwrap_or(0);
    let left = games::ir_returns(w.year).saturating_sub(w.clubs[t].ir_used);
    let name = w.people[i].name.clone();
    let pos = w.people[i].player.position.name();
    let text = format!(
        "Your medical staff clears {pos} {name} to come back from injured reserve ({left} return{} left this season).",
        if left == 1 { "" } else { "s" }
    );
    let d = w.today;
    tell_about(w, d, text, true, &[i]);
}

fn yours(w: &World, id: u32) -> Result<(usize, usize), String> {
    let t = super::club(w).ok_or("You don't run an NFL club.")?;
    let i = super::index_of(w, id).ok_or("No such player.")?;
    if w.people[i].pro.as_ref().and_then(|c| c.team) != Some(t) {
        return Err("He isn't yours.".into());
    }
    Ok((i, t))
}
