//! G4 Your guys, for life (board 34's "Your guys"): everyone you coached for
//! a season or more stays tied to you. The bond comes from what happened:
//! the trust he had in you, the chance you gave him, how much he grew, and
//! how it ended (a cut, a trade or a transfer sours it). His career becomes
//! part of yours: drafted, starters, Pro Bowls, rings, the ones coaching
//! now. A player who grew the most under you (a two-star you made a first
//! round pick) pays lesson points into your teaching. They come back as
//! assistants, donors and analysts; veterans follow you for less in free
//! agency (Curtis Martin to Parcells' Jets, 1998); the wronged speak out and
//! steer recruits away. Sizes are Claude's picks (proposed, not confirmed).

use super::World;
use crate::calendar::{Date, Level, Weekday};
use crate::positions::Position;
use crate::world::Stage;
use crate::world::coach::Group;
use crate::world::stories::TeamKey;

/// One of your guys.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Guy {
    pub id: u32,
    pub name: String,
    pub pos: Position,
    /// The team you last coached him at, and your seasons together.
    pub team: TeamKey,
    pub from: i32,
    pub to: i32,
    pub seasons: u8,
    pub stars: u8,
    /// How he came to you: 0 inherited, 1 recruited or drafted by you,
    /// 2 a walk-on, 3 a transfer or free agent.
    pub how: u8,
    pub ovr_in: f32,
    pub ovr_out: f32,
    /// -1 (a grudge) to 1.
    pub bond: f32,
    /// What soured it, or what he says now, in words.
    pub sour: String,
    pub born: i32,
    pub hs: u32,
    /// Where he signed out of high school: program, division index (0 FBS).
    pub signed: Option<(u16, u8)>,
    pub college_start: bool,
    /// Honors while you coached him.
    pub all_conf: u8,
    pub all_am: u8,
    /// His best Heisman vote: (place 1-10, year, finalist), while yours.
    pub heisman: Option<(u8, i32, bool)>,
    pub ac_seen: u32,
    /// Draft year, round, pick and club.
    pub pick: Option<(i32, u8, u16, u16)>,
    pub nfl_starts: u16,
    pub best_starts: u8,
    pub second_deal: bool,
    pub pro_bowls: u8,
    /// Pro Bowls while on your team.
    pub pb_yours: u8,
    pub all_pro: u8,
    pub rings: u8,
    pub gone: bool,
    /// How he came back: 0 not yet, 1 an assistant, 2 a donor, 3 an analyst,
    /// 4 a graduate assistant, 5 signed with you again.
    pub back: u8,
    pub now: String,
    pub next: String,
    /// Paid already: bit 0 teaching credit, bit 1 donor, bit 2 analyst,
    /// bit 3 asked to coach, bit 4 spoke out this year.
    pub flags: u8,
    pub spoke: i32,
}

impl Guy {
    pub fn bond_word(&self) -> &'static str {
        match self.bond {
            b if b <= -0.35 => "Grudge",
            b if b >= 0.45 => "Strong",
            b if b >= 0.1 => "Good",
            _ => "Distant",
        }
    }
    pub fn grudge(&self) -> bool {
        self.bond <= -0.35
    }
    /// "RB · junior", "QB · class of 2028", "EDGE · Bears".
    pub fn line(&self, w: &World) -> String {
        let pos = self.pos.name();
        match self.team {
            TeamKey::Hs(_) => {
                let class = crate::world::storylines::pi(w, self.id).map(|i| w.people[i].class_of).unwrap_or(self.to);
                format!("{pos} · class of {class}")
            }
            TeamKey::Nfl(_) => format!("{pos} · {}", crate::world::stories::short_name(w, self.team)),
            _ => match self.how {
                2 => format!("{pos} · walk-on"),
                3 => format!("{pos} · transfer"),
                _ => format!("{pos} · {}", crate::world::stories::short_name(w, self.team)),
            },
        }
    }
}

fn find(w: &World, id: u32) -> Option<usize> {
    w.legacy.guys.binary_search_by_key(&id, |g| g.id).ok()
}

/// Your best teaching group (where people credit go when it's about players).
pub fn best_group(w: &World) -> usize {
    w.career.as_ref().map_or(0, |c| {
        (0..c.you.teaching.len()).max_by(|&a, &b| c.you.teaching[a].total_cmp(&c.you.teaching[b])).unwrap_or(0)
    })
}

/// Your guys from college who were drafted.
pub fn drafted_count(w: &World) -> u32 {
    w.legacy.guys.iter().filter(|g| g.pick.is_some() && matches!(g.team, TeamKey::College(_))).count() as u32
}

/// Your guys who made a Pro Bowl while you coached them.
pub fn pro_bowlers(w: &World) -> u32 {
    w.legacy.guys.iter().filter(|g| g.pb_yours > 0).count() as u32
}

fn group_word(g: Group, n: u32) -> String {
    let one = match g {
        Group::QB => "quarterback",
        Group::RB => "running back",
        Group::Receivers => "receiver",
        Group::OL => "lineman",
        Group::DL => "pass rusher",
        Group::LB => "linebacker",
        Group::DB => "defensive back",
        Group::Specialists => "kicker",
    };
    if n == 1 {
        one.to_string()
    } else if one == "lineman" {
        "linemen".into()
    } else {
        format!("{one}s")
    }
}

fn count_word(n: u32) -> String {
    const W: [&str; 11] = ["no", "one", "two", "three", "four", "five", "six", "seven", "eight", "nine", "ten"];
    W.get(n as usize).map_or_else(|| n.to_string(), |s| s.to_string())
}

/// "Two of your Northern Illinois quarterbacks reached the NFL."
pub fn drafted_note(w: &World) -> Option<String> {
    let here = super::you(w).map(|x| x.1);
    let mut by: std::collections::BTreeMap<(String, usize), u32> = std::collections::BTreeMap::new();
    for g in w.legacy.guys.iter().filter(|g| g.pick.is_some() && Some(g.team) != here) {
        if matches!(g.team, TeamKey::College(_)) {
            *by.entry((crate::world::stories::short_name(w, g.team), Group::of(g.pos) as usize)).or_default() += 1;
        }
    }
    let ((team, grp), n) = by.into_iter().max_by_key(|x| x.1)?;
    let word = group_word(Group::ALL[grp], n);
    Some(format!("{} of your {team} {word} reached the NFL.", cap(&count_word(n))))
}

fn cap(s: &str) -> String {
    let mut c = s.chars();
    c.next().map_or(String::new(), |f| f.to_uppercase().collect::<String>() + c.as_str())
}

/// "Your Boise quarterback was eighth in the 2022 vote."
pub fn heisman_note(w: &World) -> Option<String> {
    let g = w.legacy.guys.iter().filter(|g| g.heisman.is_some()).min_by_key(|g| g.heisman.map(|h| h.0))?;
    let (place, year, _) = g.heisman?;
    const ORD: [&str; 10] =
        ["first", "second", "third", "fourth", "fifth", "sixth", "seventh", "eighth", "ninth", "tenth"];
    Some(format!(
        "Your {} {} was {} in the {year} vote.",
        crate::world::stories::short_name(w, g.team),
        group_word(Group::of(g.pos), 1),
        ORD.get(place.saturating_sub(1) as usize).copied().unwrap_or("in the top ten")
    ))
}

/// "Two of your Tennessee starters were sixth-round picks."
pub fn late_note(w: &World) -> Option<String> {
    let late: Vec<&Guy> = w
        .legacy
        .guys
        .iter()
        .filter(|g| matches!(g.team, TeamKey::Nfl(_)) && g.pick.is_some_and(|p| p.1 >= 5) && g.best_starts >= 8)
        .collect();
    let first = late.first()?;
    let team = crate::world::stories::short_name(w, first.team);
    let n = late.iter().filter(|g| g.team == first.team).count() as u32;
    let round = first.pick.map_or(6, |p| p.1);
    const R: [&str; 8] = ["", "first", "second", "third", "fourth", "fifth", "sixth", "seventh"];
    Some(if n == 1 {
        format!("One of your {team} starters was a {}-round pick.", R.get(round as usize).copied().unwrap_or("late"))
    } else {
        format!("{} of your {team} starters were late-round picks.", cap(&count_word(n)))
    })
}

/// "Your Tennessee defenses sent four."
pub fn pro_bowl_note(w: &World) -> Option<String> {
    let here = super::you(w).map(|x| x.1);
    let past: Vec<&Guy> = w
        .legacy
        .guys
        .iter()
        .filter(|g| g.pb_yours > 0 && Some(g.team) != here && matches!(g.team, TeamKey::Nfl(_)))
        .collect();
    let first = past.first()?;
    let team = crate::world::stories::short_name(w, first.team);
    let n = past.iter().filter(|g| g.team == first.team).count() as u32;
    Some(format!("Your {team} teams sent {} to the Pro Bowl.", count_word(n)))
}

/// Coach a future NFL player (high school): your best player's road from
/// the ambition's school since it was set.
pub fn future_nfl(w: &World, a: &super::ambitions::Ambition) -> [bool; 5] {
    let from = a.set.map_or(w.year, |d| d.year());
    let mut r = [false; 5];
    for g in w.legacy.guys.iter().filter(|g| g.team == a.team && g.to >= from) {
        r[0] |= g.signed.is_some_and(|s| s.1 <= 1);
        r[1] |= g.college_start;
        r[2] |= g.pick.is_some();
        r[3] |= g.best_starts >= 8;
        r[4] |= g.pro_bowls > 0;
    }
    r
}

/// Coach a Heisman finalist (college), since it was set.
pub fn heisman_steps(w: &World, a: &super::ambitions::Ambition) -> [bool; 5] {
    let from = a.set.map_or(w.year, |d| d.year());
    let mut r = [false; 5];
    for g in w.legacy.guys.iter().filter(|g| matches!(g.team, TeamKey::College(_)) && g.to >= from) {
        r[0] |= g.all_conf > 0;
        r[1] |= g.all_am > 0;
        if let Some((place, y, fin)) = g.heisman
            && y >= from
        {
            r[2] = true;
            r[3] |= fin;
            r[4] |= place == 1;
        }
    }
    r
}

/// Turn a late pick into a starter (NFL): picks in rounds 5-7 by your club
/// since it was set.
pub fn late_steps(w: &World, a: &super::ambitions::Ambition) -> [bool; 5] {
    let from = a.set.map_or(w.year, |d| d.year());
    let TeamKey::Nfl(t) = a.team else { return [false; 5] };
    let mut r = [false; 5];
    for g in w.legacy.guys.iter().filter(|g| g.pick.is_some_and(|p| p.0 >= from && p.1 >= 5 && p.3 == t)) {
        r[0] |= g.nfl_starts > 0;
        r[1] |= g.best_starts >= 8;
        r[2] |= g.best_starts >= 16;
        r[3] |= g.second_deal;
        r[4] |= g.pro_bowls > 0;
    }
    r
}

/// How a veteran weighs your club in free agency: one of your guys follows
/// you (a strong bond up to 12% more), a grudge keeps him away.
pub(crate) fn follow_q(w: &World, id: u32, t: usize) -> f64 {
    if !matches!(super::you(w), Some((_, TeamKey::Nfl(n))) if n as usize == t) {
        return 1.0;
    }
    match find(w, id) {
        Some(k) => {
            let b = w.legacy.guys[k].bond as f64;
            if b >= 0.0 { 1.0 + 0.12 * b } else { 1.0 + 0.1 * b }
        }
        None => 1.0,
    }
}

/// Your guys' pull on recruits toward your program: the son of one pulls
/// (more with a strong bond); a grudge in his school's halls pushes.
/// Read once per signing day (`leans`).
pub(crate) struct Leans {
    pub prog: usize,
    sons: std::collections::HashMap<u32, f64>,
    grudge_hs: std::collections::HashSet<usize>,
}

impl Leans {
    /// The lean of one recruit (by person id, from high school `hs`).
    pub(crate) fn of(&self, id: u32, hs: usize) -> f64 {
        self.sons.get(&id).copied().unwrap_or(0.0) - if self.grudge_hs.contains(&hs) { 0.08 } else { 0.0 }
    }
}

/// The leans when you run a college program and have guys.
pub(crate) fn leans(w: &World) -> Option<Leans> {
    let Some((_, TeamKey::College(p))) = super::you(w) else { return None };
    if w.legacy.guys.is_empty() {
        return None;
    }
    let mut sons = std::collections::HashMap::new();
    for &(son, f) in &w.legacy.sons {
        if let Some(k) = find(w, f) {
            sons.insert(son, 0.12 + 0.12 * w.legacy.guys[k].bond.max(0.0) as f64);
        }
    }
    // A former player who spoke out against you, from the same school.
    let grudge_hs =
        w.legacy.guys.iter().filter(|g| g.grudge() && g.spoke >= w.year - 2).map(|g| g.hs as usize).collect();
    Some(Leans { prog: p as usize, sons, grudge_hs })
}

/// One recruit's lean toward program `prog` (0 unless it is yours).
pub(crate) fn recruit_lean(w: &World, id: u32, hs: usize, prog: usize) -> f64 {
    leans(w).filter(|l| l.prog == prog).map_or(0.0, |l| l.of(id, hs))
}

/// Every week: what became of your guys (their days come as news through
/// the chief of staff's "Your people" memo); the teaching credit when one
/// grew from nothing into a high pick.
pub(crate) fn day(w: &mut World, d: Date) {
    if d.weekday() != Weekday::Tue || w.legacy.guys.is_empty() {
        return;
    }
    let year = w.year;
    let champs: Vec<(i32, usize)> = w.nfl_seasons.iter().filter(|s| s.round == 5).map(|s| (s.year, s.team)).collect();
    let mut credit: Vec<(u32, String, f32, usize)> = Vec::new();
    for k in 0..w.legacy.guys.len() {
        let id = w.legacy.guys[k].id;
        let Some(i) = crate::world::storylines::pi(w, id) else {
            w.legacy.guys[k].gone = true;
            continue;
        };
        let p = &w.people[i];
        let mut g = w.legacy.guys[k].clone();
        if g.signed.is_none()
            && let Some(s) = p.cal.signed.as_ref()
        {
            let div = w.programs[s.program].division as u8;
            g.signed = Some((s.program as u16, div));
        }
        if p.stage == Stage::College && p.last_role == Some(crate::world::PlayingTime::Starter) {
            g.college_start = true;
        }
        if let Some(c) = p.pro.as_ref() {
            if let Some((y, r, pk, t)) = c.draft {
                g.pick = Some((y, r as u8, pk as u16, t as u16));
            }
            g.nfl_starts = c.seasons.iter().map(|s| s.starts).sum::<u32>().min(u16::MAX as u32) as u16;
            g.best_starts = c.seasons.iter().map(|s| s.starts).max().unwrap_or(0).min(255) as u8;
            g.pro_bowls = c.seasons.iter().filter(|s| s.pro_bowl).count() as u8;
            g.all_pro = c.seasons.iter().filter(|s| s.all_pro == 1).count() as u8;
            g.rings = c.seasons.iter().filter(|s| s.team.is_some_and(|t| champs.contains(&(s.year, t)))).count() as u8;
            g.second_deal = c.contract.as_ref().is_some_and(|x| {
                !matches!(x.kind, crate::world::pro::money::Kind::Rookie | crate::world::pro::money::Kind::Undrafted)
            }) || c.seasons.len() > 5;
        }
        g.gone = p.stage == Stage::Done;
        let (now, next) = now_next(w, i, &g);
        g.now = now;
        g.next = next;
        // Teaching credit: a player who grew from little into a high pick.
        if g.flags & 1 == 0
            && let Some((_, round, ..)) = g.pick
            && round <= 3
            && g.how != 3
            && g.seasons >= 2
        {
            let low = if g.how == 2 { 1 } else { g.stars.max(1) };
            let lesson = match (low, round) {
                (..=2, 1) => 1.0,
                (..=2, 2) | (3, 1) => 0.75,
                (..=2, 3) | (3, 2) => 0.5,
                (3, 3) | (4, 1) => 0.25,
                _ => 0.0,
            };
            g.flags |= 1;
            if lesson > 0.0 {
                let what = format!(
                    "{}: {} into a round {round} pick",
                    g.name,
                    if g.how == 2 { "a walk-on".to_string() } else { format!("{}-star", low) }
                );
                credit.push((id, what, lesson, Group::of(g.pos) as usize));
            }
        }
        w.legacy.guys[k] = g;
    }
    for (id, what, lesson, grp) in credit {
        super::pay(w, super::key("G4", id as u64, 1), "G4", what, lesson, super::Bank::Teach(grp), 0.1);
    }
    let _ = year;
}

/// Where he is now and what's next, in a few words.
fn now_next(w: &World, i: usize, g: &Guy) -> (String, String) {
    let p = &w.people[i];
    let year = w.year;
    match p.stage {
        Stage::HighSchool | Stage::PrepSchool => match p.cal.signed.as_ref() {
            Some(s) => (format!("Signed with {}", w.program_name(s.program)), "Enrolls in June".into()),
            None => match p.cal.commit {
                Some((c, ..)) => (format!("Committed to {}", w.program_name(c)), "Signing day".into()),
                None => (w.school_name(p.hs).to_string(), format!("Class of {}", p.class_of)),
            },
        },
        Stage::College => {
            let school = p.college.map_or(String::new(), |c| w.program_name(c).to_string());
            let left = 4u32.saturating_sub(p.college_seasons);
            let next = if p.cal.draft_read.is_some() && p.college_seasons >= 3 {
                "The draft in April".to_string()
            } else if left <= 1 {
                "His last season".to_string()
            } else {
                format!("{} seasons left", count_word(left))
            };
            let now = match p.last_role {
                Some(crate::world::PlayingTime::Redshirt) => format!("{school}, redshirt"),
                Some(crate::world::PlayingTime::Starter) => school,
                _ if p.college_seasons == 0 => format!("{school}, freshman"),
                _ => format!("{school}, backup"),
            };
            (now, next)
        }
        Stage::Declared => ("Declared for the draft".into(), "Draft day".into()),
        Stage::Pro => {
            let Some(c) = p.pro.as_ref() else { return ("In the pros".into(), String::new()) };
            let team = c.team.map(|t| crate::world::stories::short_name(w, TeamKey::Nfl(t as u16)));
            let now = match (c.status, team) {
                (crate::world::pro::Status::PracticeSquad, Some(t)) => format!("{t}, practice squad"),
                (crate::world::pro::Status::Reserve, Some(t)) => format!("{t}, injured reserve"),
                (crate::world::pro::Status::Roster, Some(t))
                    if g.all_pro > 0 && c.seasons.last().is_some_and(|s| s.all_pro == 1) =>
                {
                    format!("{t}, first-team All-Pro")
                }
                (crate::world::pro::Status::Roster, Some(t)) if c.seasons.last().is_some_and(|s| s.starts >= 8) => {
                    format!("{t}, starter")
                }
                (crate::world::pro::Status::Roster, Some(t)) => format!("{t}, backup"),
                (crate::world::pro::Status::Other(_), _) => "Playing in another league".into(),
                _ => "Free agent".into(),
            };
            let next = match c.contract.as_ref() {
                Some(k) if k.last_year() <= year => "A free agent in March".into(),
                Some(k) => format!("Signed through {}", k.last_year()),
                None => "Looking for a team".into(),
            };
            (now, next)
        }
        Stage::Done => {
            let next = if g.grudge() && !g.sour.is_empty() { g.sour.clone() } else { String::new() };
            let now = match g.back {
                1 => "Coaching on your staff".into(),
                2 => "A donor to the program".into(),
                3 => "An analyst on TV".into(),
                4 => "Your graduate assistant".into(),
                _ => "Out of football".into(),
            };
            (now, next)
        }
        _ => ("Away from football".into(), String::new()),
    }
}

/// After your season: everyone on your roster is one of your guys; the
/// bond moves with the season; the ones who left you are read; the ways
/// back come.
pub(crate) fn season(w: &mut World) {
    let Some((you, team)) = super::you(w) else { return };
    let year = w.year;
    let roster = crate::world::storylines::detect::roster(w, team);
    let started = w.career.as_ref().map_or(year, |c| c.started.year());
    let mut fresh: Vec<Guy> = Vec::new();
    for &i in &roster {
        let p = &w.people[i];
        let id = p.player.id;
        let ovr = p.player.overall() as f32;
        let trust = if p.trust_in == you as u32 { p.trust } else { 0.5 };
        let starter = p.last_role == Some(crate::world::PlayingTime::Starter)
            || p.pro
                .as_ref()
                .and_then(|c| c.seasons.iter().find(|s| s.year == year))
                .is_some_and(|s| s.starts_all() >= 8);
        let honors = w.stats.honors.get(&id);
        match find(w, id) {
            Some(k) => {
                let g = &mut w.legacy.guys[k];
                if g.to < year {
                    g.seasons = g.seasons.saturating_add(1);
                }
                g.to = year;
                g.team = team;
                let grew = ovr - g.ovr_out;
                g.ovr_out = ovr;
                g.bond = (g.bond
                    + 0.25 * (trust - 0.5)
                    + if starter { 0.08 } else { 0.0 }
                    + if grew >= 4.0 { 0.06 } else { 0.0 })
                .clamp(-1.0, 1.0);
                if let Some(h) = honors {
                    if h.all_conference > g.ac_seen {
                        g.all_conf = g.all_conf.saturating_add((h.all_conference - g.ac_seen).min(3) as u8);
                    }
                    g.ac_seen = h.all_conference;
                    if h.all_america.iter().any(|x| x.0 == year) {
                        g.all_am = g.all_am.saturating_add(1);
                    }
                }
            }
            None => {
                let how = match team {
                    TeamKey::College(prog) => {
                        if p.walk_on {
                            2
                        } else if p.transfers > 0 {
                            3
                        } else if p.college_staff.first().is_some_and(|s| s.1 == prog as usize && s.2 == Some(you)) {
                            1
                        } else {
                            0
                        }
                    }
                    TeamKey::Nfl(t) => match p.pro.as_ref().and_then(|c| c.draft) {
                        Some((y, .., dt)) if dt == t as usize && y >= started => 1,
                        Some((.., dt)) if dt == t as usize => 0,
                        _ => 3,
                    },
                    TeamKey::Hs(_) => {
                        if p.class_of - 3 >= started {
                            1
                        } else {
                            0
                        }
                    }
                    TeamKey::Other(_) => 0,
                };
                fresh.push(Guy {
                    id,
                    name: p.name.clone(),
                    pos: p.player.position,
                    team,
                    from: year,
                    to: year,
                    seasons: 1,
                    stars: p.stars,
                    how,
                    ovr_in: ovr,
                    ovr_out: ovr,
                    bond: (0.1
                        + 0.4 * (trust - 0.5)
                        + if starter { 0.1 } else { 0.0 }
                        + if how == 1 { 0.05 } else { 0.0 })
                    .clamp(-1.0, 1.0),
                    sour: String::new(),
                    born: p.born.year(),
                    hs: p.hs as u32,
                    signed: None,
                    college_start: false,
                    all_conf: 0,
                    all_am: honors.is_some_and(|h| h.all_america.iter().any(|x| x.0 == year)) as u8,
                    heisman: None,
                    ac_seen: honors.map_or(0, |h| h.all_conference),
                    pick: None,
                    nfl_starts: 0,
                    best_starts: 0,
                    second_deal: false,
                    pro_bowls: 0,
                    pb_yours: 0,
                    all_pro: 0,
                    rings: 0,
                    gone: false,
                    back: 0,
                    now: String::new(),
                    next: String::new(),
                    flags: 0,
                    spoke: 0,
                });
            }
        }
    }
    if !fresh.is_empty() {
        w.legacy.guys.extend(fresh);
        w.legacy.guys.sort_by_key(|g| g.id);
    }
    // The season's Heisman vote and Pro Bowls for the ones on your roster.
    let ids: Vec<u32> = roster.iter().map(|&i| w.people[i].player.id).collect();
    if let Some(cs) = w.college.as_ref() {
        for (place, row) in cs.heisman_vote.iter().enumerate() {
            if ids.contains(&row.id)
                && let Some(k) = find(w, row.id)
            {
                let g = &mut w.legacy.guys[k];
                let p = place as u8 + 1;
                if g.heisman.is_none_or(|h| p < h.0) {
                    g.heisman = Some((p, year, row.finalist));
                }
            }
        }
    }
    for &i in &roster {
        let pb = w.people[i]
            .pro
            .as_ref()
            .and_then(|c| c.seasons.iter().find(|s| s.year == year))
            .is_some_and(|s| s.pro_bowl);
        if pb && let Some(k) = find(w, w.people[i].player.id) {
            w.legacy.guys[k].pb_yours = w.legacy.guys[k].pb_yours.saturating_add(1);
        }
    }
    // The ones who left you this year before their time: cut, traded,
    // transferred out.
    let gone_early: Vec<(usize, String, f32)> = w
        .legacy
        .guys
        .iter()
        .enumerate()
        .filter(|(_, g)| g.team == team && g.to == year - 1 && !ids.contains(&g.id))
        .filter_map(|(k, g)| {
            let i = crate::world::storylines::pi(w, g.id)?;
            let p = &w.people[i];
            match team {
                TeamKey::College(_)
                    if p.stage == Stage::College
                        && p.college.is_some_and(|c| Some(TeamKey::College(c as u32)) != Some(team)) =>
                {
                    let unhappy = p.mood < -0.25;
                    Some((k, format!("transferred {year}"), if unhappy { -0.6 } else { -0.25 }))
                }
                TeamKey::Nfl(t) if p.stage == Stage::Pro => {
                    let c = p.pro.as_ref()?;
                    let cut = c.former == Some(t as usize) && !c.expired;
                    cut.then(|| (k, format!("cut in {year}"), -0.6))
                }
                _ => None,
            }
        })
        .collect();
    for (k, why, by) in gone_early {
        let g = &mut w.legacy.guys[k];
        g.bond = (g.bond + by).clamp(-1.0, 1.0);
        if g.sour.is_empty() {
            g.sour = why;
        }
    }
    ways_back(w);
}

/// The ways back, once a season: an assistant or graduate assistant, a
/// donor, an analyst; and the wronged speaking out.
fn ways_back(w: &mut World) {
    let Some((_, team)) = super::you(w) else { return };
    let year = w.year;
    let seed = w.options.seed as u64;
    let roll =
        |id: u32, salt: u64| crate::world::season::key_uniform(seed ^ 0x6A17 ^ salt ^ (year as u64) << 20, id as u64);
    // One former player asks to coach for you (high school: a walk-on
    // assistant; college: a graduate assistant).
    let asks = w
        .legacy
        .guys
        .iter()
        .filter(|g| g.gone && g.back == 0 && g.flags & 8 == 0 && g.bond >= 0.45)
        .filter(|g| (22..=40).contains(&(year - g.born)))
        .filter(|g| {
            matches!((g.team, team), (TeamKey::Hs(_), TeamKey::Hs(_)) | (TeamKey::College(_), TeamKey::College(_)))
        })
        .filter(|g| roll(g.id, 1) < 0.25)
        .map(|g| (g.id, g.name.clone(), g.pos))
        .next();
    if let Some((id, name, pos)) = asks {
        if let Some(k) = find(w, id) {
            w.legacy.guys[k].flags |= 8;
        }
        let hs = matches!(team, TeamKey::Hs(_));
        let role = if hs { "a volunteer assistant" } else { "a graduate assistant" };
        let body = vec![
            format!("{name} played {} for you and wants to coach. He'd start as {role}.", pos.name()),
            "He knows your system, and your players will listen to him.".to_string(),
        ];
        let answers = vec![
            crate::world::desk::Answer {
                label: "Bring him on".into(),
                does: format!("{name} joins as {role}."),
                act: super::ACT_GUY,
                target: id,
            },
            crate::world::desk::Answer {
                label: "Not now".into(),
                does: "He looks elsewhere.".into(),
                act: super::ACT_GUY,
                target: 0,
            },
        ];
        super::letter(
            w,
            "legacy: guy",
            crate::world::desk::Head::Email,
            (name.clone(), "Former player".into()),
            format!("{name} wants to coach"),
            body,
            answers,
        );
    }
    // Donors: a player who made money gives back to the college or school.
    let donors: Vec<(usize, u32, String, f64)> = w
        .legacy
        .guys
        .iter()
        .enumerate()
        .filter(|(_, g)| g.flags & 2 == 0 && g.bond >= 0.4 && year - g.born >= 28)
        .filter_map(|(k, g)| {
            let i = crate::world::storylines::pi(w, g.id)?;
            let earned = w.people[i].pro.as_ref().map_or(0.0, |c| c.earnings);
            (earned >= 3.0e6 && roll(g.id, 2) < 0.3).then(|| (k, g.id, g.name.clone(), earned))
        })
        .take(2)
        .collect();
    for (k, _, name, earned) in donors {
        w.legacy.guys[k].flags |= 2;
        let at = w.legacy.guys[k].team;
        match at {
            TeamKey::College(p) => {
                let gift = (earned * 0.02).clamp(100_000.0, 1_500_000.0);
                w.programs[p as usize].collective.base += gift;
                if w.legacy.guys[k].back == 0 && w.legacy.guys[k].gone {
                    w.legacy.guys[k].back = 2;
                }
                if at == team {
                    super::memo(
                        w,
                        "legacy: donor",
                        format!("{name} gives back"),
                        vec![format!(
                            "{name} pledged {} a year to the collective. He says he owes his career to this program.",
                            crate::world::front_office::dollars(gift)
                        )],
                    );
                }
            }
            TeamKey::Hs(h) => {
                let gift = (earned * 0.002).clamp(5_000.0, 50_000.0);
                if w.front_office.school.school == Some(h as usize) {
                    w.front_office.school.boosters.balance += gift;
                    w.front_office.school.boosters.ledger.push((w.today, format!("Gift from {name}"), gift));
                }
                if w.legacy.guys[k].gone {
                    w.legacy.guys[k].back = 2;
                }
                if at == team {
                    super::memo(
                        w,
                        "legacy: donor",
                        format!("{name} gives back"),
                        vec![format!(
                            "{name} sent the booster club {}. \"Coach gave me my start,\" he told the paper.",
                            crate::world::front_office::dollars(gift)
                        )],
                    );
                }
            }
            _ => {}
        }
    }
    // Analysts: a retired star on television, for you or against you.
    let on_tv: Vec<usize> = w
        .legacy
        .guys
        .iter()
        .enumerate()
        .filter(|(_, g)| g.gone && g.back == 0 && g.flags & 4 == 0 && (g.nfl_starts >= 60 || g.pro_bowls >= 2))
        .filter(|(_, g)| roll(g.id, 3) < 0.35)
        .map(|(k, _)| k)
        .collect();
    for k in on_tv {
        w.legacy.guys[k].flags |= 4;
        w.legacy.guys[k].back = 3;
    }
    let mut said: Vec<(u32, String, bool)> = Vec::new();
    for g in
        w.legacy.guys.iter_mut().filter(|g| g.back == 3 || (g.grudge() && (g.nfl_starts >= 16 || g.pick.is_some())))
    {
        if g.spoke == year {
            continue;
        }
        let r = crate::world::season::key_uniform(seed ^ 0x7A11 ^ (year as u64) << 20, g.id as u64);
        if g.grudge() && r < 0.3 {
            g.spoke = year;
            const WHAT: [&str; 4] = [
                "Says on a podcast you lie in recruiting",
                "Told a reporter you play favourites",
                "Says you never kept a promise to him",
                "Circles your games on his calendar",
            ];
            g.sour = WHAT[(r * 13.0) as usize % WHAT.len()].to_string();
            said.push((g.id, g.name.clone(), false));
        } else if g.back == 3 && g.bond >= 0.4 && r < 0.25 {
            g.spoke = year;
            said.push((g.id, g.name.clone(), true));
        }
    }
    for (id, name, good) in said {
        if good {
            super::pay(
                w,
                super::key("G4tv", id as u64, year as u64),
                "G4",
                format!("{name} defends you on TV"),
                0.0,
                super::Bank::Rating(crate::world::storylines::Rating::Relationships),
                0.08,
            );
        } else {
            super::cost(
                w,
                super::key("G4x", id as u64, year as u64),
                "G4",
                format!("{name} speaks out against you"),
                0.1,
            );
            if let Some(k) = find(w, id) {
                let words = w.legacy.guys[k].sour.clone();
                let level = team.level();
                if level != Level::HighSchool {
                    let you = crate::world::stories::seat_coach(w).unwrap_or(0);
                    let spot = crate::world::storylines::Spot::new(
                        crate::world::storylines::K::PublicFeud,
                        crate::world::storylines::subj(id as u64, you as u64),
                    )
                    .team(team)
                    .lead(crate::world::storylines::Who::Coach(you as u32))
                    .rival(crate::world::storylines::Who::Player(id))
                    .fact(
                        format!(
                            "{name}, who played for {}: {}.",
                            crate::world::storylines::detect::cname(w, you),
                            words.to_lowercase()
                        ),
                        20.0,
                    )
                    .public();
                    crate::world::storylines::detect::tell(w, spot);
                }
            }
        }
    }
    sons(w);
}

/// The sons of your guys: a former player in his late thirties may have a
/// boy coming through high school in his home state (rare, and only after
/// many years). He leans toward you when you recruit him.
fn sons(w: &mut World) {
    let year = w.year;
    let seed = w.options.seed as u64;
    let fathers: Vec<(u32, String, usize)> = w
        .legacy
        .guys
        .iter()
        .filter(|g| year - g.born >= 36 && g.bond >= 0.3 && !w.legacy.sons.iter().any(|s| s.1 == g.id))
        .filter(|g| crate::world::season::key_uniform(seed ^ 0x50_45 ^ (year as u64) << 20, g.id as u64) < 0.04)
        .filter_map(|g| {
            let state = crate::world::storylines::pi(w, g.id).map(|i| w.people[i].home_state)?;
            let last = g.name.split_whitespace().last()?.to_string();
            Some((g.id, last, state))
        })
        .take(1)
        .collect();
    for (father, last, state) in fathers {
        // A sophomore in his state who has no family tie yet.
        let son = w
            .people
            .iter()
            .enumerate()
            .filter(|(_, p)| p.stage == Stage::HighSchool && p.home_state == state && p.class_of == year + 2)
            .filter(|(_, p)| p.family.older_brother_at.is_none() && !w.legacy.sons.iter().any(|s| s.0 == p.player.id))
            .min_by_key(|(_, p)| p.player.id ^ father)
            .map(|(i, _)| i);
        let Some(i) = son else { continue };
        let first = w.people[i].name.split_whitespace().next().unwrap_or("").to_string();
        w.people[i].name = format!("{first} {last}");
        let id = w.people[i].player.id;
        w.legacy.sons.push((id, father));
        let fname = find(w, father).map_or(String::new(), |k| w.legacy.guys[k].name.clone());
        super::memo(
            w,
            "legacy: son",
            format!("{fname}'s son"),
            vec![format!(
                "{first} {last}, a sophomore at {}, is {fname}'s son. His father played for you.",
                w.school_name(w.people[i].hs)
            )],
        );
    }
}

/// Answers to "wants to coach": the target is his id (0: not now).
pub(crate) fn answer(w: &mut World, target: u32) {
    let Some(k) = find(w, target) else { return };
    let Some((_, team)) = super::you(w) else { return };
    let g = w.legacy.guys[k].clone();
    match team {
        TeamKey::Hs(_) => {
            let born = crate::calendar::Date::ymd(g.born, 6, 1);
            let age = (w.year - g.born).max(18) as u32;
            if let Some(c) = w.career.as_mut() {
                c.hs_staff.push(crate::world::seat::HsAssistant {
                    name: g.name.clone(),
                    age,
                    born,
                    coordinator: None,
                    group: Group::of(g.pos) as u8,
                    subject: None,
                    stipend: 1_500,
                    teaching: (8.0 + 6.0 * g.bond.max(0.0) as f64).min(14.0),
                    since: w.year,
                });
            }
            w.legacy.guys[k].back = 1;
        }
        _ => w.legacy.guys[k].back = 4,
    }
}

/// The counts on the Legacy board, by level: (label, count).
pub fn tiles(w: &World, level: Level) -> Vec<(String, u32)> {
    let gs = &w.legacy.guys;
    let n = |f: &dyn Fn(&Guy) -> bool| gs.iter().filter(|g| f(g)).count() as u32;
    let coaching = n(&|g| matches!(g.back, 1 | 4));
    let grudge = n(&|g| g.grudge());
    match level {
        Level::HighSchool => vec![
            ("to college".into(), n(&|g| g.signed.is_some())),
            ("to D1".into(), n(&|g| g.signed.is_some_and(|s| s.1 <= 1))),
            ("in the NFL".into(), n(&|g| g.pick.is_some() || g.nfl_starts > 0)),
            ("coaching now".into(), coaching),
            (if grudge == 1 { "grudge".into() } else { "grudges".into() }, grudge),
        ],
        Level::Nfl => vec![
            ("Pro Bowls".into(), gs.iter().map(|g| g.pb_yours as u32).sum()),
            ("All-Pro".into(), n(&|g| g.all_pro > 0)),
            (
                "rings".into(),
                gs.iter().filter(|g| matches!(g.team, TeamKey::Nfl(_))).map(|g| g.rings as u32).max().unwrap_or(0),
            ),
            ("coaching now".into(), coaching),
            (if grudge == 1 { "grudge".into() } else { "grudges".into() }, grudge),
        ],
        _ => vec![
            ("drafted".into(), drafted_count(w)),
            ("NFL starters".into(), n(&|g| g.best_starts >= 8)),
            ("Pro Bowls".into(), gs.iter().map(|g| g.pro_bowls as u32).sum()),
            ("coaching now".into(), coaching),
            (if grudge == 1 { "grudge".into() } else { "grudges".into() }, grudge),
        ],
    }
}

/// The guys the board lists: the most notable first, a grudge kept in view.
pub fn notable(w: &World, n: usize) -> Vec<&Guy> {
    let score = |g: &Guy| -> f32 {
        let career = g.pro_bowls as f32 * 3.0
            + g.all_pro as f32 * 4.0
            + g.best_starts as f32 / 8.0
            + g.pick.map_or(0.0, |p| 4.0 - p.1 as f32 * 0.5)
            + g.all_am as f32 * 2.0
            + g.all_conf as f32
            + if g.signed.is_some_and(|s| s.1 == 0) { 1.5 } else { 0.0 };
        career + 2.0 * g.bond.abs() + if g.gone { -1.0 } else { 0.0 } + 0.2 * g.to as f32 / 2000.0
    };
    let mut v: Vec<&Guy> = w.legacy.guys.iter().filter(|g| !g.grudge()).collect();
    v.sort_by(|a, b| score(b).total_cmp(&score(a)).then(a.id.cmp(&b.id)));
    v.truncate(n.saturating_sub(1));
    let worst = w.legacy.guys.iter().filter(|g| g.grudge()).min_by(|a, b| a.bond.total_cmp(&b.bond));
    match worst {
        Some(g) => v.push(g),
        None => {
            if let Some(g) = w
                .legacy
                .guys
                .iter()
                .filter(|g| !v.iter().any(|x| x.id == g.id))
                .max_by(|a, b| score(a).total_cmp(&score(b)))
            {
                v.push(g);
            }
        }
    }
    v
}
