//! The head coach's guidelines in the world (update 2, lane 5: G1-G3).
//! Every head coach sets them: you on the Plans page, the AI's head coaches
//! from their own leanings (the system they know best, their nature), so
//! every team plays by the same rules. They pull the spring install toward
//! what the head coach wants and lean the play callers on game day
//! (`game::guide`). A coordinator moved off his own systems calls them as
//! well as he knows them: his calling fit reads his knowledge of what he is
//! asked to call (`systems.rs`), never a flat penalty.
//!
//! Sizes are Claude's picks (proposed, not confirmed).

use super::World;
use super::coach::Group;
use super::season::{key_normal, key_uniform};
use super::systems::{FRONTS, Kind, OFFENSES, SHELLS};
use crate::calendar::Level;
use crate::game::guide::{self, BaseFront, Firm, Guidelines, Pace, Want};
use crate::game::{GameTeam, OffScheme, Shell};
use crate::rng::clamp;

/// How firm a head coach is by nature: one who moulds every habit to his
/// system holds his staff to it; one who adapts to his people lets them
/// call their own game.
fn firmness(w: &World, head: usize) -> Firm {
    let mould = w.coaches[head].style().mould;
    if mould >= 13.5 {
        Firm::Firm
    } else if mould <= 7.5 {
        Firm::Theirs
    } else {
        Firm::Lean
    }
}

/// The slot he knows best in a kind and whether it stands clear of the
/// rest (a real leaning, not a coin flip).
fn leaning(know: &[f64; 11], r: std::ops::Range<usize>) -> Option<usize> {
    let start = r.start;
    let mut v: Vec<(usize, f64)> = r.map(|s| (s, know[s])).collect();
    v.sort_by(|a, b| b.1.total_cmp(&a.1));
    let (best, k) = v[0];
    let next = v.get(1).map_or(0.0, |x| x.1);
    (k >= 0.8 && k - next >= 0.15).then_some(best - start)
}

/// An AI head coach's guidelines from his own leanings: the systems he
/// knows best when he clearly prefers one, the pressure his nature likes,
/// the run share his background brings, a nickel base front for some
/// college and high school defensive minds. He says nothing where he has
/// no strong view.
pub fn head_guidelines(w: &World, head: Option<usize>, level: Level) -> Guidelines {
    let Some(h) = head else { return Guidelines::default() };
    let c = &w.coaches[h];
    let firm = firmness(w, h);
    let id = h as u64;
    let know = &c.systems.know;
    let mut g = Guidelines::default();
    let college = level != Level::Nfl;
    if let Some(o) = leaning(know, 0..5) {
        g.system = Want::new(OFFENSES[o], firm);
    }
    if let Some(f) = leaning(know, 5..7) {
        let nickel = college
            && matches!(c.came_up, Group::DB | Group::LB)
            && key_uniform(0x4250 ^ (w.year as u64 / 4), id) < if level == Level::HighSchool { 0.35 } else { 0.25 };
        g.front = Want::new(
            match (FRONTS[f], nickel) {
                (crate::game::Front::FourThree, false) => BaseFront::FourThree,
                (crate::game::Front::FourThree, true) => BaseFront::FourTwoFive,
                (crate::game::Front::ThreeFour, false) => BaseFront::ThreeFour,
                (crate::game::Front::ThreeFour, true) => BaseFront::ThreeThreeFive,
            },
            firm,
        );
    }
    if let Some(s) = leaning(know, 7..11) {
        g.man = Want::new(guide::habit_man(SHELLS[s]), firm);
    }
    // His nature: an aggressive head coach wants more pressure, a careful
    // one less (only when he leans clearly).
    let risk = c.style().risk;
    if (risk - 10.0).abs() >= 3.0 {
        g.pressure = Want::new(guide::habit_pressure(risk), firm);
    }
    // A line or backs coach by background wants to run it; a quarterback
    // or receivers coach to throw it.
    let best_off = (0..5).max_by(|&a, &b| know[a].total_cmp(&know[b])).map_or(OffScheme::WestCoast, |o| OFFENSES[o]);
    let own_run = 1.0 - guide::habit_pass(college, best_off, 10.0);
    let lean = match c.came_up {
        Group::OL | Group::RB => Some(0.05),
        Group::QB | Group::Receivers => Some(-0.05),
        _ => None,
    };
    if let Some(l) = lean {
        g.run = Want::new(clamp(own_run + l + 0.02 * key_normal(0x7255, id), 0.25, 0.65), firm);
    }
    if matches!(best_off, OffScheme::AirRaid | OffScheme::SpreadOption)
        && know[OFFENSES.iter().position(|&o| o == best_off).unwrap_or(0)] >= 0.85
    {
        g.pace = Want::new(if key_uniform(0x9ACE, id) < 0.2 { Pace::NoHuddle } else { Pace::Mixed }, firm);
    }
    g
}

/// A team's guidelines for the season: yours when it is your team, else
/// its head coach's. A head coach who calls a side himself calls his own
/// game there (no pull); when you call a side yourself, your guidelines
/// there are yours to keep (firm).
pub(crate) fn team_guidelines(
    w: &World,
    head: Option<usize>,
    oc_call: Option<usize>,
    dc_call: Option<usize>,
    level: Level,
    seat: bool,
) -> Guidelines {
    if seat {
        // An assistant's team plays to its head coach's guidelines.
        let mut g = if w.seat_head() { w.plans.guide } else { head_guidelines(w, head, level) };
        if let Some(y) = head.and_then(|h| w.coaches[h].yours).filter(|_| w.seat_head()) {
            if y.calls_offense {
                firm_side(&mut g, true);
            }
            if y.calls_defense {
                firm_side(&mut g, false);
            }
        }
        return g;
    }
    let mut g = head_guidelines(w, head, level);
    // Wing-T stays a high school offense for the AI.
    if level != Level::HighSchool && g.system.to == Some(OffScheme::WingT) {
        g.system = Want::default();
    }
    if head.is_some() && oc_call == head {
        g = g.without_offense();
    }
    if head.is_some() && dc_call == head {
        g = g.without_defense();
    }
    g
}

fn firm_side(g: &mut Guidelines, offense: bool) {
    if offense {
        g.system.firm = Firm::Firm;
        g.personnel.firm = Firm::Firm;
        g.pace.firm = Firm::Firm;
        g.run.firm = Firm::Firm;
        g.stick.firm = Firm::Firm;
    } else {
        g.front.firm = Firm::Firm;
        g.man.firm = Firm::Firm;
        g.pressure.firm = Firm::Firm;
    }
}

/// An NFL club's guidelines this season.
pub(crate) fn club_guidelines(w: &World, t: usize) -> Guidelines {
    let tm = &w.nfl[t];
    let oc = super::season::play_caller(w, tm.oc, tm.head_coach, true).or(tm.head_coach);
    let dc = super::season::play_caller(w, tm.dc, tm.head_coach, false).or(tm.head_coach);
    team_guidelines(w, tm.head_coach, oc, dc, Level::Nfl, w.seat_club(t))
}

/// A college program's guidelines this season.
pub(crate) fn program_guidelines(w: &World, prog: &super::program::Program) -> Guidelines {
    let oc = super::season::play_caller(w, prog.oc, prog.head_coach, true).or(prog.head_coach);
    let dc = super::season::play_caller(w, prog.dc, prog.head_coach, false).or(prog.head_coach);
    team_guidelines(w, prog.head_coach, oc, dc, Level::College, w.seat_program(prog.id))
}

/// What a guideline adds to a system's worth in the spring install, in
/// points a game (S3's units): a firm head coach all but decides it, a
/// lean tips a close call, the coordinator's call barely counts.
pub(crate) fn install_bonus(g: &Guidelines, kind: Kind, i: usize) -> f64 {
    let size = |f: Firm| match f {
        Firm::Firm => 6.0,
        Firm::Lean => 1.6,
        Firm::Theirs => 0.3,
    };
    match kind {
        Kind::Offense => match g.system.to {
            // The wing-T is a gap-scheme offense: a power gap install.
            Some(OffScheme::WingT) if OFFENSES[i] == OffScheme::PowerGap => size(g.system.firm),
            Some(s) if OFFENSES[i] == s => size(g.system.firm),
            _ => 0.0,
        },
        Kind::Front => match g.front.to {
            Some(b) if FRONTS[i] == b.front() => size(g.front.firm),
            _ => 0.0,
        },
        Kind::Shell => match g.man.pull() {
            // Toward the family whose man share is nearest the guideline.
            Some((want, k)) => {
                let near = 1.0 - (guide::habit_man(SHELLS[i]) - want).abs() / 0.35;
                k * 4.0 * near.max(0.0)
            }
            None => 0.0,
        },
    }
}

/// Whether a guideline asks for this system firmly enough that the
/// coordinator puts it in even knowing it less than half (a lean or a
/// firm one).
pub(crate) fn install_wanted(g: &Guidelines, kind: Kind, i: usize) -> bool {
    install_bonus(g, kind, i) >= 1.6
}

/// Your high school's systems: what your guidelines ask for (a lean or
/// firm one), else what the staff knows best.
pub(crate) fn wanted_install(g: &Guidelines, mut inst: super::systems::Installed) -> super::systems::Installed {
    for (kind, n) in [(Kind::Offense, OFFENSES.len()), (Kind::Front, FRONTS.len()), (Kind::Shell, SHELLS.len())] {
        let best = (0..n).max_by(|&a, &b| install_bonus(g, kind, a).total_cmp(&install_bonus(g, kind, b)));
        if let Some(i) = best.filter(|&i| install_wanted(g, kind, i)) {
            match kind {
                Kind::Offense => inst.off = i,
                Kind::Front => inst.front = i,
                Kind::Shell => inst.shell = i,
            }
        }
    }
    inst
}

/// The same for an NFL staff, in knowledge units (NFL staffs install what
/// they know best, `nfl_install`).
pub(crate) fn nfl_bonus(g: &Guidelines, kind: Kind, i: usize) -> f64 {
    install_bonus(g, kind, i) / 6.0
}

/// A play caller's fit (S4) for what he is asked to call: his knowledge of
/// the installed coverage family, blended with the family the man-coverage
/// guideline moves him toward by the share of snaps it moves.
fn man_fit(w: &World, dc: Option<usize>, shell: Shell, g: &Guidelines) -> f64 {
    let know = dc.map_or([0.8; 11], |c| w.coaches[c].systems.know);
    let inst = SHELLS.iter().position(|&s| s == shell).unwrap_or(0);
    let own = know[7 + inst];
    let fit = |k: f64| 0.7 + 0.3 * k;
    let Some((want, k)) = g.man.pull() else { return 1.0 };
    let d = k * (want - guide::habit_man(shell));
    if d.abs() < 0.02 {
        return 1.0;
    }
    // More man: the man and pressure family; less: the zone family he
    // knows best.
    let alt = if d > 0.0 {
        2
    } else {
        [0usize, 1, 3].into_iter().max_by(|&a, &b| know[7 + a].total_cmp(&know[7 + b])).unwrap_or(0)
    };
    if alt == inst {
        return 1.0;
    }
    let s = d.abs().min(1.0);
    fit((1.0 - s) * own + s * know[7 + alt]) / fit(own)
}

/// The offense's fit for the wing-T, when you ask for it on a power gap
/// install: a gap-scheme play caller knows about three-quarters of it
/// (proposed, not confirmed).
fn wing_fit(w: &World, oc: Option<usize>) -> f64 {
    let know = oc.map_or(0.8, |c| w.coaches[c].systems.know[3]);
    (0.7 + 0.3 * 0.75 * know) / (0.7 + 0.3 * know)
}

fn scale(c: &mut crate::game::Caller, f: f64) {
    c.skill *= f;
    c.reading *= f;
    c.creativity *= f;
}

/// Sets a game team's guidelines and moves its play callers' fit to what
/// they are asked to call. Safe to run again with new guidelines (your
/// team is refreshed when you change them): the old fit is taken back
/// first.
pub(crate) fn apply(w: &World, team: &mut GameTeam, g: Guidelines, oc: Option<usize>, dc: Option<usize>) {
    let old = team.staff.guide;
    let shell = team.staff.shell;
    let f_old = man_fit(w, dc, shell, &old);
    let f_new = man_fit(w, dc, shell, &g);
    scale(&mut team.staff.defense, f_new / f_old);
    // The wing-T on a power gap install (your team only reaches it).
    let wing = |g: &Guidelines| {
        g.system.to == Some(OffScheme::WingT) && g.system.firm != Firm::Theirs && team_runs_gap(&team.staff)
    };
    let (was, now) = (wing(&old), wing(&g));
    if was != now {
        let f = wing_fit(w, oc);
        if now {
            team.staff.off_scheme = OffScheme::WingT;
            scale(&mut team.staff.offense, f);
        } else {
            team.staff.off_scheme = OffScheme::PowerGap;
            scale(&mut team.staff.offense, 1.0 / f);
        }
    }
    team.staff.guide = g;
}

fn team_runs_gap(s: &crate::game::Staff) -> bool {
    matches!(s.off_scheme, OffScheme::PowerGap | OffScheme::WingT)
}

/// Your team's guidelines and play callers as the day's game team holds
/// them (run after the build and after every refresh).
pub(crate) fn seat_apply(w: &World, team: &mut GameTeam) {
    let Some(s) = w.seat else { return };
    let (head, oc, dc) = match s.level {
        Level::College => match s.program.map(|p| &w.programs[p]) {
            Some(p) => (p.head_coach, p.oc, p.dc),
            None => return,
        },
        Level::Nfl => match s.nfl_team.map(|t| &w.nfl[t]) {
            Some(t) => (t.head_coach, t.oc, t.dc),
            None => return,
        },
        Level::HighSchool => match s.program.map(|h| w.high_schools[h].coach) {
            Some(c) => (c, c, c),
            None => return,
        },
        _ => return,
    };
    let oc_call = super::season::play_caller(w, oc, head, true).or(head);
    let dc_call = super::season::play_caller(w, dc, head, false).or(head);
    let g = team_guidelines(w, head, oc_call, dc_call, s.level, true);
    apply(w, team, g, oc_call, dc_call);
}
