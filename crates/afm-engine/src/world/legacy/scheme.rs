//! G3, your scheme as a legacy: your way of playing read from what your
//! team runs (its offense, front and coverage), named by the press when it
//! is your own and it wins, copied by the world under your name, and
//! answered by defenses through the engine's own counters (a family few
//! teams run is new to every opponent; it stops being new as it spreads).
//! The copying is real learning: coaches on your tree, at your clinics and
//! near you pick your system up, and a coach who knows a system well
//! enough installs it in the spring. Sizes are proposed, not confirmed.

use super::{Bank, key, memo, pay};
use crate::world::World;
use crate::world::coach::Employer;
use crate::world::stories::TeamKey;
use crate::world::storylines::catalogue::{K, Rating};
use crate::world::systems::{FRONTS, OFFENSES, SHELLS};

/// Your scheme as the world reads it.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Scheme {
    pub named: Option<Named>,
    /// Teams running it each season since it was named.
    pub spread: Vec<Spread>,
    /// Your fingerprint each season: (year, offense, front, shell).
    pub seen: Vec<(i32, [u8; 3])>,
    /// Coaches who heard you at a clinic (ids), and the clinics you gave.
    pub clinic_coaches: Vec<u32>,
    pub clinics: Vec<i32>,
    /// The novelty edge your offense had the year it was named (college).
    pub edge_then: f32,
    /// The year the press last called you stubborn.
    pub stubborn: Option<i32>,
}

/// Your named system.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Named {
    pub name: String,
    /// The outlet that named it.
    pub by: String,
    pub year: i32,
    pub fp: [u8; 3],
    /// The side it's named for: offense, or the defense's coverage.
    pub offense: bool,
    /// Teams already running it when it was named (they aren't copies).
    pub base: Vec<u64>,
}

/// One season's count of teams running your system.
#[derive(Clone, Copy, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Spread {
    pub year: i32,
    pub hs: u16,
    pub college: u16,
    pub nfl: u16,
    pub tree: u16,
    pub clinics: u16,
    pub copiers: u16,
}

impl Spread {
    pub fn total(&self) -> u16 {
        self.hs + self.college + self.nfl
    }
}

fn off_word(k: usize) -> &'static str {
    use crate::game::OffScheme::*;
    match OFFENSES[k.min(OFFENSES.len() - 1)] {
        WestCoast => "West Coast",
        AirRaid => "air raid",
        WideZone => "wide zone",
        PowerGap => "power",
        SpreadOption => "spread option",
        WingT => "wing-T",
    }
}

fn shell_word(k: usize) -> &'static str {
    use crate::game::Shell::*;
    match SHELLS[k.min(SHELLS.len() - 1)] {
        Cover3 => "Cover 3",
        Quarters => "quarters",
        ManPressure => "man pressure",
        Cover2 => "Cover 2",
    }
}

/// A system in words ("air raid, 4-3, quarters").
pub fn words(fp: [u8; 3]) -> String {
    let front = match FRONTS[(fp[1] as usize).min(FRONTS.len() - 1)] {
        crate::game::Front::FourThree => "4-3",
        crate::game::Front::ThreeFour => "3-4",
    };
    format!("{}, {front}, {}", off_word(fp[0] as usize), shell_word(fp[2] as usize))
}

/// The systems a coach knows best (how a high school staff installs).
fn best_known(know: &[f64; 11]) -> [u8; 3] {
    let best = |r: std::ops::Range<usize>| {
        let start = r.start;
        r.max_by(|&a, &b| know[a].total_cmp(&know[b])).map_or(0, |x| x - start) as u8
    };
    [best(0..5), best(5..7), best(7..11)]
}

/// A team's systems this season.
pub fn fingerprint(w: &World, t: TeamKey) -> Option<[u8; 3]> {
    match t {
        TeamKey::College(p) => {
            let i = w.programs.get(p as usize)?.installed;
            Some([i.off as u8, i.front as u8, i.shell as u8])
        }
        TeamKey::Nfl(n) => {
            let i = w.clubs.get(n as usize)?.installed;
            Some([i.off as u8, i.front as u8, i.shell as u8])
        }
        TeamKey::Hs(h) => {
            let c = w.high_schools.get(h as usize)?.coach?;
            Some(best_known(&w.coaches[c].systems.know))
        }
        TeamKey::Other(_) => None,
    }
}

fn code(t: TeamKey) -> u64 {
    match t {
        TeamKey::Hs(h) => h as u64,
        TeamKey::College(p) => 1 << 40 | p as u64,
        TeamKey::Nfl(n) => 2 << 40 | n as u64,
        TeamKey::Other(o) => 3 << 40 | o as u64,
    }
}

/// Whether a team runs your named system: the same side's systems.
fn runs(fp: [u8; 3], named: &Named) -> bool {
    if named.offense {
        fp[0] == named.fp[0] && fp[2] == named.fp[2]
    } else {
        fp[1] == named.fp[1] && fp[2] == named.fp[2]
    }
}

/// Who runs a team's plays: its head coach and coordinators.
fn staff_of(w: &World, t: TeamKey) -> Vec<usize> {
    match t {
        TeamKey::College(p) => {
            let g = &w.programs[p as usize];
            [g.head_coach, g.oc, g.dc].into_iter().flatten().collect()
        }
        TeamKey::Nfl(n) => {
            let g = &w.nfl[n as usize];
            [g.head_coach, g.oc, g.dc].into_iter().flatten().collect()
        }
        TeamKey::Hs(h) => w.high_schools[h as usize].coach.into_iter().collect(),
        TeamKey::Other(_) => Vec::new(),
    }
}

/// The teams near enough to copy you: your state's high schools, your
/// state's and conference's colleges, the whole NFL.
fn near(w: &World, you: TeamKey) -> Vec<TeamKey> {
    let state = match you {
        TeamKey::Hs(h) => Some(w.high_schools[h as usize].state),
        TeamKey::College(p) => Some(w.programs[p as usize].state),
        _ => None,
    };
    let mut v = Vec::new();
    match you {
        TeamKey::Hs(_) => {
            for (h, x) in w.high_schools.iter().enumerate() {
                if Some(x.state) == state && x.coach.is_some() {
                    v.push(TeamKey::Hs(h as u32));
                }
            }
        }
        TeamKey::College(p) => {
            let conf = w.programs[p as usize].conference.clone();
            for (q, x) in w.programs.iter().enumerate() {
                if x.plays(w.year) && (Some(x.state) == state || x.conference == conf) {
                    v.push(TeamKey::College(q as u32));
                }
            }
        }
        TeamKey::Nfl(_) => {
            for t in 0..w.nfl.len().min(w.clubs.len()) {
                v.push(TeamKey::Nfl(t as u16));
            }
        }
        TeamKey::Other(_) => {}
    }
    v
}

/// Your win share over your last two seasons, and a title among them.
fn recent(w: &World, team: TeamKey) -> (f64, bool) {
    let s = super::seasons_at(w, team);
    let last: Vec<&super::Yr> = s.iter().rev().take(2).collect();
    let (wins, losses) = last.iter().fold((0, 0), |a, y| (a.0 + y.wins as u32, a.1 + y.losses as u32));
    (wins as f64 / (wins + losses).max(1) as f64, last.iter().any(|y| y.champion || y.title))
}

fn name_for(w: &World, team: TeamKey, fp: [u8; 3], offense: bool) -> String {
    let side = if offense { off_word(fp[0] as usize) } else { shell_word(fp[2] as usize) };
    // "Allen Cover 3", "Seattle Cover 3": a city for a pro club.
    let school = crate::world::stories::short_name(w, team);
    let me = super::you(w).map_or(String::new(), |x| w.coaches[x.0].name.clone());
    let surname = me.split_whitespace().last().unwrap_or("").to_string();
    let u = crate::world::season::key_uniform(w.options.seed as u64 ^ 0x5C4E, code(team) ^ fp[0] as u64);
    if u < 0.5 || surname.is_empty() { format!("{school} {side}") } else { format!("{surname}'s {side}") }
}

/// The edge your offense's families have on defenses now (college film).
fn edge(w: &World, team: TeamKey) -> f32 {
    let TeamKey::College(p) = team else { return 0.0 };
    let prog = &w.programs[p as usize];
    let nov = crate::world::trends::team_novelty(w, prog.division, &prog.film);
    let fam = prog.film.families();
    let off = crate::game::playbook::Family::Front as usize;
    let (mut s, mut n) = (0.0, 0.0);
    for f in 0..off {
        s += nov[f] * fam[f];
        n += fam[f];
    }
    if n > 0.0 { (s / n) as f32 } else { 0.0 }
}

/// After your season: your fingerprint, the naming, the spread, the pull
/// on the coaches who copy, and the rewards.
pub(crate) fn season(w: &mut World) {
    let Some((me, team)) = super::you(w) else { return };
    if !super::head(w) {
        return;
    }
    let Some(fp) = fingerprint(w, team) else { return };
    let year = w.year;
    if !w.legacy.scheme.seen.iter().any(|x| x.0 == year) {
        w.legacy.scheme.seen.push((year, fp));
    }
    let offense = w.coaches[me].came_up.offense();
    let (pct, title) = recent(w, team);
    let same_two = w.legacy.scheme.seen.iter().rev().take(2).filter(|x| x.1 == fp).count() >= 2;
    // The naming: your own (few near you run it) and winning.
    if w.legacy.scheme.named.is_none() && same_two && (pct >= 0.7 || title) {
        let others = near(w, team);
        let mut running = Vec::new();
        for &t in &others {
            if t != team
                && let Some(f) = fingerprint(w, t)
            {
                let probe = Named { name: String::new(), by: String::new(), year, fp, offense, base: Vec::new() };
                if runs(f, &probe) {
                    running.push(code(t));
                }
            }
        }
        let share = running.len() as f64 / others.len().max(1) as f64;
        if share <= 0.2 {
            let by = crate::world::folk::cast::all(
                w,
                crate::world::storylines::folk_team(team),
                crate::world::folk::cast::Role::Reporter,
            )
            .into_iter()
            .find_map(|c| c.reporter.map(|r| r.outlet))
            .unwrap_or_else(|| "the local paper".into());
            let name = name_for(w, team, fp, offense);
            w.legacy.scheme.edge_then = edge(w, team);
            w.legacy.scheme.named =
                Some(Named { name: name.clone(), by: by.clone(), year, fp, offense, base: running });
            pay(
                w,
                key("G3", 1, year as u64),
                "G3",
                format!("{by} named your system: {name}"),
                0.75,
                Bank::Rating(Rating::Scheme),
                2.0,
            );
            super::story(w, K::SchemeNamed, 1, name.clone(), format!("{by} named it: {name} ({}).", words(fp)));
            memo(
                w,
                "legacy-scheme",
                format!("\"{name}\""),
                vec![
                    format!("{by} has a name for what you run: \"{name}\" ({})", words(fp)),
                    "Coaches near you will be watching the film. The more of them who run it, the less new it is to the defenses you face.".into(),
                ],
            );
        }
    }
    let Some(named) = w.legacy.scheme.named.clone() else { return };
    // The spread: who runs it now, and why.
    let tree_ids: Vec<usize> = (0..w.coaches.len()).filter(|&c| w.coaches[c].mentor == Some(me)).collect();
    let clinic: Vec<usize> = w.legacy.scheme.clinic_coaches.iter().map(|&c| c as usize).collect();
    let mut sp = Spread { year, ..Default::default() };
    let mut teams: Vec<TeamKey> = near(w, team);
    // Your tree's teams anywhere.
    for &c in &tree_ids {
        if let Some(j) = w.coaches[c].job
            && let Some(t) = super::team_of_employer(j.employer)
            && !teams.contains(&t)
        {
            teams.push(t);
        }
    }
    for t in teams {
        let Some(f) = fingerprint(w, t) else { continue };
        if t != team && !runs(f, &named) {
            continue;
        }
        let staff = staff_of(w, t);
        let why_tree = staff.iter().any(|c| tree_ids.contains(c));
        let why_clinic = staff.iter().any(|c| clinic.contains(c));
        if t != team {
            if why_tree {
                sp.tree += 1;
            } else if why_clinic {
                sp.clinics += 1;
            } else if !named.base.contains(&code(t)) {
                sp.copiers += 1;
            } else {
                continue;
            }
        }
        match t {
            TeamKey::Hs(_) => sp.hs += 1,
            TeamKey::College(_) => sp.college += 1,
            TeamKey::Nfl(_) => sp.nfl += 1,
            TeamKey::Other(_) => {}
        }
    }
    w.legacy.scheme.spread.retain(|x| x.year != year);
    w.legacy.scheme.spread.push(sp);
    // The pull: coaches near you learn it from film, faster when you win;
    // your tree and your clinics' coaches faster still.
    let slots: Vec<usize> = if named.offense {
        vec![named.fp[0] as usize, 7 + named.fp[2] as usize]
    } else {
        vec![5 + named.fp[1] as usize, 7 + named.fp[2] as usize]
    };
    let film = 0.02 * (pct - 0.5).max(0.0) * 2.0;
    let mut learners: Vec<(usize, f64)> = Vec::new();
    for t in near(w, team) {
        if t == team {
            continue;
        }
        for c in staff_of(w, t) {
            learners.push((c, film));
        }
    }
    for &c in &tree_ids {
        learners.push((c, 0.06));
    }
    for &c in &clinic {
        learners.push((c, 0.03));
    }
    for (c, by) in learners {
        if c == me || by <= 0.0 {
            continue;
        }
        let k = &mut w.coaches[c].systems.know;
        for &s in &slots {
            k[s] = (k[s] + by).min(1.0);
        }
    }
    // Rewards for the spread.
    let total = sp.total() as u32;
    for (n, lesson, into, rep) in [
        (5u32, 0.5f32, Rating::PlayCalling, 0.5),
        (10, 0.5, Rating::Scheme, 1.0),
        (25, 0.75, Rating::Scheme, 1.5),
        (50, 1.0, Rating::Scheme, 2.0),
    ] {
        if total >= n {
            pay(
                w,
                key("G3", 2, n as u64),
                "G3",
                format!("{} run by {n} teams", named.name),
                lesson,
                Bank::Rating(into),
                rep,
            );
        }
    }
    if total >= 10 {
        super::story_end(w, K::SchemeNamed, 1, "copied");
    }
    // Standing still: the same system six years, and the edge gone or
    // the wins with it.
    let same = w.legacy.scheme.seen.iter().rev().take_while(|x| x.1 == fp).count();
    let flat = edge(w, team) < 0.5 * w.legacy.scheme.edge_then || pct < 0.5;
    if same >= 6 && flat && w.legacy.scheme.stubborn.is_none_or(|y| y <= year - 3) {
        w.legacy.scheme.stubborn = Some(year);
        super::cost(w, key("G3", 3, year as u64), "G3", "Called stubborn: the same system six years".into(), 0.3);
        super::story_end(w, K::SchemeNamed, 1, "caught_up");
        memo(
            w,
            "legacy-stubborn",
            "\"The league has caught up\"".into(),
            vec![format!(
                "{} writes that defenses have caught up with {} and you haven't changed it in six years.",
                named.by, named.name
            )],
        );
    }
    // Clinic invitations: a named system that wins gets asked to speak.
    if pct >= 0.6 && !w.legacy.scheme.clinics.contains(&(year + 1)) {
        invite(w, &named);
    }
}

/// A clinic invitation for the spring: speak, or pass.
fn invite(w: &mut World, named: &Named) {
    let Some((_, team)) = super::you(w) else { return };
    let tag = format!("legacy-clinic-{}", w.year);
    let where_ = match team {
        TeamKey::Hs(h) => format!(
            "{} coaches' clinic",
            crate::world::folk::facts::state_name(&w.states[w.high_schools[h as usize].state].code)
        ),
        TeamKey::College(p) => format!(
            "{} coaches' clinic",
            crate::world::folk::facts::state_name(&w.states[w.programs[p as usize].state].code)
        ),
        _ => "the coaching clinic circuit".into(),
    };
    super::letter(
        w,
        &tag,
        crate::world::desk::Head::Email,
        ("Clinic director".into(), where_.clone()),
        format!("Speak at the {where_}?"),
        vec![format!(
            "Coaches want to hear about {} this spring. A morning on stage, an afternoon on the board with them.",
            named.name
        )],
        vec![
            crate::world::desk::Answer {
                label: "Speak".into(),
                does: "High school coaches learn your system; ties with them.".into(),
                act: super::ACT_CLINIC,
                target: 1,
            },
            crate::world::desk::Answer {
                label: "Not this year".into(),
                does: "Nothing changes.".into(),
                act: super::ACT_CLINIC,
                target: 0,
            },
        ],
    );
}

/// Your answer to a clinic invitation.
pub(crate) fn answer_clinic(w: &mut World, target: u32) {
    let year = w.year + 1;
    w.legacy.scheme.clinics.push(year);
    if target == 0 {
        return;
    }
    let Some((me, team)) = super::you(w) else { return };
    let state = match team {
        TeamKey::Hs(h) => w.high_schools[h as usize].state,
        TeamKey::College(p) => w.programs[p as usize].state,
        _ => return,
    };
    // Some of the state's high school head coaches come (a keyed draw).
    let mut heard = Vec::new();
    for (h, x) in w.high_schools.iter().enumerate() {
        if x.state != state {
            continue;
        }
        let Some(c) = x.coach else { continue };
        let u = crate::world::season::key_uniform(w.options.seed as u64 ^ 0xC11C, (h as u64) << 20 ^ year as u64);
        if u < 0.12 && c != me {
            heard.push(c as u32);
            if let Employer::HighSchool(_) = w.coaches[c].job.map_or(Employer::HighSchool(h), |j| j.employer) {
                *w.coaches[me].ties.entry(h).or_insert(0.0) += 0.05;
            }
        }
    }
    let n = heard.len();
    for c in heard {
        if !w.legacy.scheme.clinic_coaches.contains(&c) {
            w.legacy.scheme.clinic_coaches.push(c);
        }
    }
    if w.legacy.scheme.clinic_coaches.len() > 400 {
        let k = w.legacy.scheme.clinic_coaches.len() - 400;
        w.legacy.scheme.clinic_coaches.drain(..k);
    }
    pay(
        w,
        key("G3", 4, year as u64),
        "G3",
        format!("Spoke at a clinic: {n} head coaches in the room"),
        0.25,
        Bank::Rating(Rating::Network),
        0.3,
    );
}

/// Your scheme's film line for the board.
pub fn film_line(w: &World) -> Option<String> {
    let named = w.legacy.scheme.named.as_ref()?;
    let (_, team) = super::you(w)?;
    let games = w.legacy.games.iter().filter(|g| g.season >= named.year).count();
    let now = edge(w, team);
    let then = w.legacy.scheme.edge_then;
    let edge_words = if matches!(team, TeamKey::College(_)) && then > 0.0 {
        if now < 0.6 * then {
            format!(" Defenses have seen it: the edge is smaller than in {}.", named.year)
        } else {
            " It's still new to most defenses you face.".to_string()
        }
    } else {
        String::new()
    };
    Some(format!("Film on it: {} since {}.{edge_words}", crate::world::news::count(games as u32, "game"), named.year))
}
