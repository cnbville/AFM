//! Pro scouting (topic 13) and rookie minicamp tryouts (topic 12).
//!
//! Your club's pro personnel department: a director of pro personnel and
//! three to six pro scouts (people like the college scouts), who grade
//! every player in the league each spring from coaches' film (each scout
//! his assigned clubs), follow their clubs through the season, and keep the
//! street list for the emergency board. Tuesday tryouts work out street
//! free agents. AI clubs' pro reads stay the built grade (their GM's skill,
//! sharper with seasons of film), which is how their department's head
//! reads the league.
//!
//! After the undrafted rush, the players nobody signed get rookie minicamp
//! tryouts on a trial basis, and a few earn a contract that weekend.

use std::collections::HashMap;

use super::readers::PRO_SCOUT;
use super::{Kind, SeatLook};
use crate::calendar::{Date, Weekday};
use crate::rng::Rng;
use crate::world::pro::scouts::{Board, Role, Scout};
use crate::world::season::key_uniform;
use crate::world::{Stage, World};

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct ProSide {
    /// Your pro department (the director first).
    #[serde(default)]
    pub scouts: Vec<Scout>,
    /// Each pro scout's clubs (by place in `scouts`).
    #[serde(default)]
    pub clubs: Vec<(u8, Vec<usize>)>,
    /// Players you've worked out (player id, date).
    #[serde(default)]
    pub tryouts: Vec<(u32, Date)>,
    /// Your supplemental draft bids this summer: (player id, round).
    #[serde(default)]
    pub supp_bids: Vec<(u32, u32)>,
    /// Last spring's minicamp tryouts: (player id, clubs, signed with).
    #[serde(default)]
    pub minicamp: Vec<(u32, Vec<usize>, Option<usize>)>,
}

/// Staffs your pro department when your seat is at an NFL club (sized by
/// era and the owner's budget: three pro scouts in the 1990s, five or six
/// in the 2020s).
fn staff(w: &mut World, t: usize) {
    if !w.scouting.pro.scouts.is_empty() {
        return;
    }
    let year = w.year;
    let owner = w.nfl.get(t).map_or(10.0, |x| x.owner);
    let n = match year {
        ..=1999 => 3,
        2000..=2014 => 4,
        _ => 5,
    } + if owner >= 15.0 { 1 } else { 0 };
    let mut rng = Rng::new(w.options.seed ^ 0x9205 ^ (t as u32) << 8 ^ year as u32);
    let budget = (0.55 + 0.03 * (owner - 10.0)).clamp(0.3, 1.0);
    let mut v = Vec::new();
    for k in 0..=n {
        let mean = if k == 0 { 12.0 } else { 9.5 } + 3.0 * (budget - 0.6);
        fn r(rng: &mut Rng, m: f64) -> u8 {
            crate::rng::clamp(rng.normal(m, 2.6), 1.0, 20.0).round() as u8
        }
        let mut know = [0u8; crate::world::coach::GROUP_COUNT];
        for x in know.iter_mut() {
            *x = r(&mut rng, mean);
        }
        let born = year - rng.int(if k == 0 { 38 } else { 26 }, 58);
        let eth = if rng.chance(0.3) {
            crate::world::ethnicity::Ethnicity::Black
        } else {
            crate::world::ethnicity::Ethnicity::White
        };
        let name = crate::world::names::person_name(eth, born, &mut rng);
        v.push(Scout {
            name,
            role: if k == 0 { Role::Director } else { Role::Area },
            region: k as u8,
            home: rng.int(0, 7) as u8,
            judge: r(&mut rng, mean),
            potential: r(&mut rng, mean),
            know,
            athletic: r(&mut rng, mean),
            character: r(&mut rng, mean),
            network: r(&mut rng, mean),
            work: r(&mut rng, mean),
            conviction: r(&mut rng, 10.0),
            bias: crate::world::pro::scouts::Bias::Tape,
            key: ((rng.next_u32() as u64) << 32) | rng.next_u32() as u64,
            since: year,
            born,
        });
    }
    // Each pro scout covers his share of the league's other clubs.
    let others: Vec<usize> = crate::world::franchise::active_teams(year).into_iter().filter(|&c| c != t).collect();
    let scouts = n.max(1);
    let mut clubs: Vec<(u8, Vec<usize>)> = (1..=scouts).map(|k| (k as u8, Vec::new())).collect();
    for (j, c) in others.into_iter().enumerate() {
        clubs[j % scouts].1.push(c);
    }
    w.scouting.pro.scouts = v;
    w.scouting.pro.clubs = clubs;
}

/// The pro department's year: the league grade in the spring (each scout
/// his clubs' players, from film), a look at each of his clubs' players
/// every few weeks in season, the street list on Tuesdays.
pub(crate) fn day(w: &mut World, d: Date) {
    let Some(t) = w.seat.filter(|s| s.level == crate::calendar::Level::Nfl).and_then(|s| s.nfl_team) else {
        return;
    };
    staff(w, t);
    let (_, m, day) = d.parts();
    let spring_grade = m == 3 && (1..=28).contains(&day) && d.weekday() == Weekday::Mon;
    let in_season = matches!(m, 9..=12 | 1) && d.weekday() == Weekday::Mon;
    if !spring_grade && !in_season {
        return;
    }
    let week = (d.0 / 7) as usize;
    for (k, clubs) in w.scouting.pro.clubs.clone() {
        let who = PRO_SCOUT + k as u32;
        let mine: Vec<usize> = clubs.into_iter().filter(|&c| c < w.clubs.len()).collect();
        // The spring grade covers all of his clubs over four Mondays; in
        // season one club a week.
        let take: Vec<usize> = if spring_grade {
            let q = (mine.len() as f64 / 4.0).ceil() as usize;
            mine.iter().copied().skip((week % 4) * q).take(q).collect()
        } else {
            mine.get(week % mine.len().max(1)).copied().into_iter().collect()
        };
        for c in take {
            let ids: Vec<u32> = w.clubs[c].roster.iter().map(|&i| w.people[i].player.id).collect();
            for id in ids {
                super::dept::add(w, id, who, Kind::ProFilm, d);
            }
        }
    }
    // Keep the store small: a pro's looks older than two years go.
    let cut = d.add(-730);
    for v in w.scouting.dept.looks.values_mut() {
        v.retain(|l: &SeatLook| l.kind != Kind::ProFilm || l.date >= cut);
    }
}

/// A Tuesday tryout: your head coach and a position coach work out a street
/// free agent (a day of a coach's time).
pub fn tryout(w: &mut World, id: u32) -> Result<(), String> {
    let t = w.seat.and_then(|s| s.nfl_team).ok_or("Only an NFL club holds tryouts.")?;
    let i = super::place(w, id).ok_or("No such player.")?;
    let p = &w.people[i];
    if p.pro.as_ref().is_some_and(|c| c.team.is_some()) {
        return Err("He's under contract with a club.".into());
    }
    let d = w.today;
    let coach = w.nfl[t].head_coach.map_or(crate::world::pro::scouts::HC as u32, |c| super::readers::COACH + c as u32);
    super::dept::add(w, id, coach, Kind::Tryout, d);
    w.scouting.pro.tryouts.push((id, d));
    Ok(())
}

/// Rookie minicamp tryouts (topic 12): the best players left unsigned get a
/// trial weekend at a club with room, by its board (one to three invites
/// each), and about one in seven earns a contract on the spot (proposed,
/// not confirmed). The rest are done. Returns who was tried (place in the
/// people list to the clubs that tried him).
pub(crate) fn minicamp_tryouts(w: &mut World, unsigned: &[usize], boards: &[Board]) -> HashMap<usize, Vec<usize>> {
    let mut out: HashMap<usize, Vec<usize>> = HashMap::new();
    if w.clubs.is_empty() {
        return out;
    }
    let year = w.year;
    let teams = crate::world::franchise::active_teams(year);
    for &t in &teams {
        if t >= boards.len() {
            continue;
        }
        let mut list: Vec<(f64, usize)> = unsigned
            .iter()
            .filter(|i| !boards[t].dnd.contains(i))
            .filter_map(|&i| boards[t].grade.get(&i).map(|&g| (g, i)))
            .filter(|x| x.0 >= 46.0)
            .collect();
        list.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
        // About a dozen tryout players a club (real: rookie minicamps carry
        // 10-30 on a trial basis).
        for (_, i) in list.into_iter().take(12) {
            out.entry(i).or_default().push(t);
        }
    }
    let date = w.dates.draft[2].date.add(7);
    let mut rec = Vec::new();
    let mut tried: Vec<(usize, Vec<usize>)> = out.iter().map(|(&i, v)| (i, v.clone())).collect();
    tried.sort_by_key(|x| x.0);
    for (i, clubs) in tried {
        let id = w.people[i].player.id;
        let ov = w.people[i].player.overall();
        let mut signed = None;
        for &t in &clubs {
            let p = 0.08 + 0.01 * (ov - 50.0).clamp(0.0, 10.0);
            if key_uniform(0x3C17 ^ (t as u64) << 24 ^ year as u64, id as u64) < p {
                signed = Some(t);
                break;
            }
        }
        let names: Vec<&str> = clubs.iter().map(|&t| w.nfl[t].name).collect();
        match signed {
            Some(t) => {
                let team = w.nfl[t].name;
                let p = &mut w.people[i];
                p.stage = Stage::Pro;
                p.note(format!("Went undrafted, earned a contract with the {team} at their rookie minicamp on a tryout."));
                let terms = crate::world::pro::money::undrafted_offer(year, 0.0, 0.0);
                crate::world::pro::join_undrafted(w, i, t, year, &terms);
                if w.recording {
                    let a = crate::world::Alumnus::from_person(&w.people[i], year, None, false);
                    w.alumni.push(a);
                }
            }
            None => {
                let p = &mut w.people[i];
                p.stage = Stage::Done;
                p.note(format!(
                    "Went undrafted; tried out at rookie minicamp with the {} but wasn't signed, and his football career ended.",
                    names.join(" and the ")
                ));
            }
        }
        rec.push((id, clubs, signed));
    }
    let _ = date;
    w.scouting.pro.minicamp = rec;
    out
}

/// After the draft: your pro department's tryout notes from the minicamps.
pub(crate) fn after_draft(w: &mut World, tried: &HashMap<usize, Vec<usize>>) {
    let _ = tried;
    w.scouting.pro.supp_bids.clear();
}
