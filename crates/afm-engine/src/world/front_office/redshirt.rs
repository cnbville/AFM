//! Redshirts on your college team (minor bug 77). The head coach decides
//! them: your staff makes its pick at fall camp, and you keep it or change
//! it. From 2018 a redshirt may play four games and keep his year (the
//! engine's `held_redshirts` lets a camp redshirt play until four and
//! `settle_eligibility` counts the rest); before 2018 any game costs it.

use crate::player::PlayingTime;
use crate::world::{Stage, World, eligibility};

/// Where a player of yours stands this season: redshirting or playing,
/// his games so far, and whether you can still change it (or why not).
pub struct Read {
    pub redshirt: bool,
    pub games: u32,
    /// The four-game rule is on this season (2018 on).
    pub four: bool,
    pub can: Result<(), String>,
}

/// A camp redshirt: the staff (or you) means him to sit this season.
fn sitting(p: &crate::world::Person) -> bool {
    p.last_role == Some(PlayingTime::Redshirt) && p.redshirt_used && !p.cal.count_pending && !p.cal.academic_sit
}

pub fn read(w: &World, id: u32) -> Result<Read, String> {
    let (i, j) = yours(w, id)?;
    let p = &w.people[i];
    let four = w.rules.four_game_redshirt;
    let cs = w.college.as_ref();
    let games = cs.and_then(|c| c.snaps.get(&id)).map_or(0, |s| s.1);
    let redshirt = sitting(p);
    let can = (|| {
        let cs = cs.filter(|c| c.year == w.year && c.started && !c.finished);
        if cs.is_none() {
            return Err("Redshirts are set at fall camp, for the season.".to_string());
        }
        if eligibility::five_in_five(w.year, w.programs[j].division) {
            return Err("Five years to play five: no redshirts any more.".into());
        }
        if p.cal.academic_sit {
            return Err("He sits this year anyway (academics or a transfer year).".into());
        }
        if !redshirt {
            if p.redshirt_used {
                return Err("He used his redshirt year already.".into());
            }
            let burned = games > 0 && (!four || games > 4);
            if burned {
                return Err(format!("He's played {games} games: this season counts."));
            }
        }
        Ok(())
    })();
    Ok(Read { redshirt, games, four, can })
}

/// Redshirts him (`on`) or plays him. A redshirt who plays past four games
/// (any game before 2018) loses the year when the season is settled.
pub fn set(w: &mut World, id: u32, on: bool) -> Result<String, String> {
    let r = read(w, id)?;
    r.can?;
    if r.redshirt == on {
        return Ok(if on { "He's redshirting already." } else { "He's playing already." }.into());
    }
    let (i, _) = yours(w, id)?;
    let p = &mut w.people[i];
    let first = p.name.split_whitespace().next().unwrap_or("He").to_string();
    if on {
        p.last_role = Some(PlayingTime::Redshirt);
        p.redshirt_used = true;
        p.cal.count_pending = false;
    } else {
        // Back on the depth chart: his year counts if he plays past the rule.
        p.last_role = Some(PlayingTime::Backup);
        p.redshirt_used = false;
        p.cal.count_pending = true;
    }
    let text = match (on, r.four) {
        (true, true) if r.games < 4 => {
            format!(
                "{first} redshirts: he can still play {} more game{} and keep the year.",
                4 - r.games,
                if 4 - r.games == 1 { "" } else { "s" }
            )
        }
        (true, _) => format!("{first} redshirts: he sits the rest of the season."),
        (false, true) => format!("{first} plays: four games or fewer and he still keeps his redshirt year."),
        (false, false) => format!("{first} plays: his first game costs his redshirt year."),
    };
    super::note(w, text.clone(), Some(id));
    Ok(text)
}

fn yours(w: &World, id: u32) -> Result<(usize, usize), String> {
    let j = super::program(w).ok_or("You have no college job.")?;
    let i = super::index_of(w, id).ok_or("No such player.")?;
    let p = &w.people[i];
    if p.college != Some(j) || p.stage != Stage::College {
        return Err("He isn't on your roster.".into());
    }
    Ok((i, j))
}
