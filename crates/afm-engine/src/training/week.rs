//! The world's training week (topic 11, topic 12 choice 1): every Monday
//! each team's staff plan the week by their styles under the day's rules
//! (`plan::styled`, `plan::fit`), and every player's growth lands by what
//! the week really worked for his role. Teams with the same plan shape
//! share one worked-out week, so a full world's Monday stays quick. Your
//! own team runs its real plan (`seat::week`).

use super::plan::{Slot, Template, Week, fit, fit_block, neutral, styled, week_from};
use super::rules::{BLOCKS, Block, Book, Counts, Frame, Tier, block_limits, book, weekly_base};
use super::work::{ROLES, Role, ratio, week_load, week_work};
use super::{FAMILIES, cover, owe, settle};
use crate::calendar::{Date, YearDates};
use crate::player::Trait;
use crate::rng::clamp;
use crate::tendencies::Style;
use crate::world::data::Division;
use crate::world::{Stage, World};
use rayon::prelude::*;
use std::collections::HashMap;

/// What one shape of week gives each role.
#[derive(Clone, Debug)]
pub struct TeamWeek {
    pub block: Block,
    /// The week's practice ratio against the neutral week, per role.
    pub r: [[f64; FAMILIES]; ROLES],
    /// Load units per role.
    pub load: [f64; ROLES],
    /// Full-contact and on-field minutes against the neutral week's (the
    /// NFL's practice injury risk).
    pub risk: f64,
    /// The players' own time: nothing required.
    pub own: bool,
    pub voluntary: bool,
    /// The week's live team snaps (`practice::team_snaps`).
    pub battle: Vec<(super::plan::Kind, super::plan::Who, u8, f32)>,
}

/// The year's frames: each college division, the NFL, each state.
pub struct Frames {
    pub college: [Frame; 6],
    pub college_weeks: [[u8; BLOCKS]; 6],
    pub pro: Frame,
    pub pro_weeks: [u8; BLOCKS],
    pub states: Vec<(Frame, [u8; BLOCKS], &'static str, bool)>,
}

impl Frames {
    pub fn new(w: &World, y: &YearDates) -> Frames {
        let div = [
            (y.conference_championships, true),
            (y.fcs_regular_end, true),
            (y.d2_regular_end, true),
            (y.d2_regular_end, true),
            // Division III has no spring practice.
            (y.d2_regular_end, false),
            (y.juco_regular_end, true),
        ];
        let college = div.map(|(end, spring)| Frame::college(y, end, spring));
        let college_weeks = college.map(|f| f.weeks());
        let pro = Frame::pro(y, false);
        let states = w
            .states
            .iter()
            .enumerate()
            .map(|(s, st)| {
                let cal = crate::calendar::data::hs_calendar(&st.code);
                let (start, finals) =
                    w.hs_season.as_ref().and_then(|h| h.states.get(s)).map(|x| (x.start, x.finals)).unwrap_or_else(
                        || {
                            let (a, _, f) = crate::world::hs_year::state_season_dates(&st.code, y.season);
                            (a, f)
                        },
                    );
                let f = Frame::high(y, start, finals, cal.spring_ball);
                (f, f.weeks(), cal.state, cal.heat_rule)
            })
            .collect();
        Frames { college, college_weeks, pro, pro_weeks: pro.weeks(), states }
    }

    pub fn division(&self, d: Division) -> (&Frame, &[u8; BLOCKS]) {
        (&self.college[d as usize], &self.college_weeks[d as usize])
    }
}

/// The days the camp has run (acclimatization), -1 before it.
pub fn camp_day(f: &Frame, d: Date) -> i32 {
    let start = if f.tier == Tier::Pro { f.report } else { f.camp };
    if d < start { -1 } else { start.days_to(d) }
}

/// The week's seven rule books.
pub fn books(f: &Frame, monday: Date, state: &str, heat: bool) -> [Book; 7] {
    std::array::from_fn(|k| {
        let d = monday.add(k as i32);
        let b = f.block(d);
        book(f.tier, b, f.season, camp_day(f, d), state, heat)
    })
}

/// The week's block: Wednesday's.
pub fn block_of_week(f: &Frame, monday: Date) -> Block {
    f.block(monday.add(2))
}

/// A style read in three steps (protective, middle, physical and so on),
/// for sharing worked-out weeks.
fn bucket(st: &Style) -> u8 {
    let b = |v: f64| {
        if v < 8.0 {
            0u8
        } else if v > 13.0 {
            2
        } else {
            1
        }
    };
    b(st.physical) * 9 + b(st.freedom) * 3 + b(st.risk)
}

fn bucket_style(k: u8) -> Style {
    let v = |x: u8| [5.0, 10.5, 16.0][x as usize];
    Style { physical: v(k / 9), freedom: v(k / 3 % 3), risk: v(k % 3), ..Style::default() }
}

/// Works out a week for a staff style and the week's games: the plan, the
/// neutral plan, and what each role gets against it.
pub fn work_out(
    tier: Tier,
    f: &Frame,
    monday: Date,
    style: &Style,
    games: &[Date],
    bowl: bool,
    state: &str,
    heat: bool,
) -> (TeamWeek, Week) {
    let block = block_of_week(f, monday);
    let bk = books(f, monday, state, heat);
    let wk = staff_week(tier, f, monday, games, bowl, state, heat, &|s| styled(tier, s, style));
    let nk = staff_week(tier, f, monday, games, bowl, state, heat, &|s| neutral(tier, s));
    (team_week(f, block, &bk, &wk, &nk), wk)
}

/// The games a staff plans an earlier week of the block around: one a
/// week in season, on the level's usual day.
fn usual_games(tier: Tier, block: Block, monday: Date) -> Vec<Date> {
    if block != Block::Season {
        return Vec::new();
    }
    vec![monday.add(match tier {
        Tier::Pro => 6,
        Tier::College => 5,
        Tier::High => 4,
    })]
}

/// A staff's week from its templates, fitted to the day's books and to
/// what is left of the block's counts (the earlier weeks of the block
/// rebuilt the same way), so the AI never passes a block's limits.
#[allow(clippy::too_many_arguments)]
pub fn staff_week(
    tier: Tier,
    f: &Frame,
    monday: Date,
    games: &[Date],
    bowl: bool,
    state: &str,
    heat: bool,
    tpl: &dyn Fn(Slot) -> Template,
) -> Week {
    let block = block_of_week(f, monday);
    let build = |m: Date, g: &[Date]| {
        let slot = Slot::of(block, !g.is_empty(), bowl);
        let mut wk = week_from(slot, m, g, &tpl(slot), &tpl(Slot::Bye));
        fit(&mut wk, &books(f, m, state, heat));
        wk
    };
    let mut wk = build(monday, games);
    let lim = block_limits(tier, block, f.season);
    if lim == Counts::unlimited() {
        return wk;
    }
    let mut used = Counts::default();
    let mut m = monday;
    while m.add(-7) >= f.start && block_of_week(f, m.add(-7)) == block {
        m = m.add(-7);
    }
    while m < monday {
        let mut before = build(m, &usual_games(tier, block, m));
        fit_block(&mut before, &used, &lim);
        used.add_week(&before);
        m = m.add(7);
    }
    fit_block(&mut wk, &used, &lim);
    wk
}

/// What a planned week gives each role against the neutral week.
pub fn team_week(f: &Frame, block: Block, bk: &[Book; 7], wk: &Week, nk: &Week) -> TeamWeek {
    let camp = block == Block::Camp;
    let mut r = [[1.0; FAMILIES]; ROLES];
    let mut load = [0.0; ROLES];
    for role in Role::ALL {
        let a = week_work(wk, role, camp, 1.0);
        let b = week_work(nk, role, camp, 1.0);
        r[role as usize] = ratio(&a, &b);
        load[role as usize] = week_load(wk, role, camp).0;
    }
    let (_, full, field) = week_load(wk, Role::Starter, camp);
    let (_, nfull, nfield) = week_load(nk, Role::Starter, camp);
    let risk = if nfield > 0.0 {
        clamp(((full + 10.0) / (nfull + 10.0)).sqrt() * ((field + 30.0) / (nfield + 30.0)).powf(0.3), 0.5, 2.0)
    } else {
        1.0
    };
    let own = bk.iter().all(|b| !b.coaches);
    let voluntary = bk.iter().any(|b| b.voluntary);
    let _ = f;
    let battle = if own { Vec::new() } else { super::practice::team_snaps(wk) };
    TeamWeek { block, r, load, risk, own, voluntary, battle }
}

/// How hard a player works on his own (topic 9): his professionalism and
/// ambition, a little lower for one just paid. Around 1 for the average
/// player (the yearly calibration), 0.6-1.4 at the ends (proposed, not
/// confirmed).
pub fn own_ratio(p: &crate::player::Player, just_paid: bool) -> f64 {
    let t = &p.personality.traits;
    clamp(
        1.0 + 0.12 * t.z(Trait::Professionalism) + 0.06 * t.z(Trait::Ambition) - if just_paid { 0.08 } else { 0.0 },
        0.6,
        1.4,
    )
}

/// Whether he comes to a voluntary week (NFL phases and OTAs, summer
/// workouts): most do; veterans with little to prove skip some
/// (proposed, not confirmed).
pub fn attends(p: &crate::player::Player, key: u64) -> bool {
    let age = p.age as f64;
    let chance = clamp(0.9 + 0.015 * (p.personality.work_ethic - 10.0) - 0.04 * (age - 29.0).max(0.0), 0.45, 0.99);
    crate::world::season::key_uniform(key, p.id as u64) < chance
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct Key {
    tier: u8,
    unit: u16,
    style: u8,
    mask: u8,
    bowl: bool,
}

/// Monday: plan every team's week and land the growth it earned.
pub(crate) fn monday(w: &mut World, d: Date) {
    if w.options.yearly_growth {
        return;
    }
    let y = w.dates.clone();
    let frames = Frames::new(w, &y);
    let seed = w.options.seed as u64 ^ (d.0 as u64) << 24 ^ 0x7EA1;
    // Your team first: its real plan, its battles, its notes.
    let seat_people = crate::world::clock::timed("training seat", || super::seat::week(w, d, &frames));
    // The week's games by team.
    let next = |x: Date| x >= d && x < d.add(7);
    let mut college_games: HashMap<usize, Vec<Date>> = HashMap::new();
    let mut bowl: Vec<bool> = vec![false; w.programs.len()];
    if let Some(cs) = w.college.as_ref() {
        for g in &cs.games {
            if next(g.date) {
                college_games.entry(g.home).or_default().push(g.date);
                college_games.entry(g.away).or_default().push(g.date);
            }
            if g.kind != crate::world::schedule::GameKind::Regular && g.date >= d {
                bowl[g.home] = true;
                bowl[g.away] = true;
            }
        }
    }
    let mut nfl_games: HashMap<usize, Vec<Date>> = HashMap::new();
    if let Some(s) = w.nfl_season.as_ref() {
        for (k, wk) in s.weeks.iter().enumerate() {
            if let Some(&sun) = y.nfl_weeks.get(k)
                && next(sun)
            {
                for &(a, b) in wk {
                    nfl_games.entry(a).or_default().push(sun);
                    nfl_games.entry(b).or_default().push(sun);
                }
            }
        }
    }
    let state_games: Vec<Vec<Date>> = w
        .hs_season
        .as_ref()
        .map(|h| {
            h.states
                .iter()
                .map(|s| s.weeks.iter().chain(s.rounds.iter()).copied().filter(|&x| next(x)).collect())
                .collect()
        })
        .unwrap_or_default();
    // Each team's week shape.
    let style_of = |c: Option<usize>| c.map_or_else(Style::default, |c| w.coaches[c].style());
    let mask = |g: &[Date]| g.iter().fold(0u8, |m, x| m | 1 << d.days_to(*x).clamp(0, 6));
    let mut keys: Vec<Key> = Vec::new();
    let prog_key: Vec<Key> = w
        .programs
        .iter()
        .enumerate()
        .map(|(c, p)| Key {
            tier: 1,
            unit: p.division as u16,
            style: bucket(&style_of(p.head_coach)),
            mask: college_games.get(&c).map_or(0, |g| mask(g)),
            bowl: bowl[c],
        })
        .collect();
    let club_key: Vec<Key> = (0..w.nfl.len())
        .map(|t| Key {
            tier: 2,
            unit: 0,
            style: bucket(&style_of(w.nfl[t].head_coach)),
            mask: nfl_games.get(&t).map_or(0, |g| mask(g)),
            bowl: false,
        })
        .collect();
    let school_key: Vec<Key> = w
        .high_schools
        .iter()
        .map(|h| Key {
            tier: 0,
            unit: h.state as u16,
            style: bucket(&style_of(h.coach)),
            mask: state_games.get(h.state).map_or(0, |g| mask(g)),
            bowl: false,
        })
        .collect();
    keys.extend(prog_key.iter().chain(club_key.iter()).chain(school_key.iter()).copied());
    keys.sort_by_key(|k| (k.tier, k.unit, k.style, k.mask, k.bowl));
    keys.dedup();
    let worked: HashMap<Key, TeamWeek> = keys
        .par_iter()
        .map(|k| {
            let games: Vec<Date> = (0..7).filter(|i| k.mask & (1 << i) != 0).map(|i| d.add(i)).collect();
            let st = bucket_style(k.style);
            let tw = match k.tier {
                0 => {
                    let (f, _, code, heat) = &frames.states[k.unit as usize];
                    work_out(Tier::High, f, d, &st, &games, false, code, *heat).0
                }
                1 => {
                    let f = &frames.college[k.unit as usize];
                    work_out(Tier::College, f, d, &st, &games, k.bowl, "", false).0
                }
                _ => work_out(Tier::Pro, &frames.pro, d, &st, &games, false, "", false).0,
            };
            (*k, tw)
        })
        .collect();
    // Practice battles on the engine: every college and NFL team runs a
    // sample of its week's team snaps (choice 3).
    let cap_of = |tw: &TeamWeek| if tw.own { 0 } else { super::practice::ai_cap(tw.block) };
    let wr: &World = w;
    use crate::world::clock::timed;
    let mut ai: Vec<(Vec<usize>, super::practice::AiTeam)> = timed("training teams", || {
        (0..wr.programs.len())
            .into_par_iter()
            .filter_map(|c| {
                let tw = worked.get(&prog_key[c])?;
                let cap = cap_of(tw);
                if cap == 0 || wr.seat_program(c) || tw.battle.is_empty() {
                    return None;
                }
                let prog = &wr.programs[c];
                let cached = wr.college.as_ref().and_then(|cs| cs.teams.get(c)).and_then(|t| t.clone());
                let team = cached.or_else(|| crate::world::season::game_team(wr, prog))?;
                let level = crate::world::division_level(prog.division);
                Some((prog.roster.clone(), super::practice::AiTeam { team, level, snaps: tw.battle.clone(), cap }))
            })
            .collect()
    });
    ai.extend(
        (0..wr.nfl.len().min(wr.clubs.len()))
            .into_par_iter()
            .filter_map(|t| {
                let tw = worked.get(&club_key[t])?;
                let cap = cap_of(tw);
                if cap == 0 || wr.seat_club(t) || tw.battle.is_empty() {
                    return None;
                }
                let team = crate::world::pro::games::game_side(wr, t)?.team;
                Some((
                    wr.clubs[t].roster.clone(),
                    super::practice::AiTeam { team, level: crate::player::Level::NFL, snaps: tw.battle.clone(), cap },
                ))
            })
            .collect::<Vec<_>>(),
    );
    timed("training battles", || super::practice::world_battles(w, ai, seed ^ 0xBA77));
    let school_snaps: Vec<usize> =
        school_key.iter().map(|k| worked.get(k).filter(|tw| !tw.battle.is_empty()).map_or(0, cap_of)).collect();
    timed("training schools", || super::practice::school_grades(w, &school_snaps, seed ^ 0x5C4));
    // NFL clubs' practice risk for the week's practice injuries.
    w.training.club_risk = club_key.iter().map(|k| worked[k].risk as f32).collect();
    // Each pro's place on his club's depth chart.
    let mut pro_role: HashMap<usize, Role> = HashMap::new();
    for (t, c) in w.clubs.iter().enumerate() {
        if t >= w.nfl.len() {
            break;
        }
        for (pos, list) in crate::positions::Position::ALL.iter().zip(c.depth.iter()) {
            let s = crate::league::depth_slots(*pos);
            for (k, &i) in list.iter().enumerate() {
                let role = if pos.is_specialist() {
                    Role::Specialist
                } else if k < s.starters {
                    Role::Starter
                } else if k < s.starters + s.rotation {
                    Role::Backup
                } else {
                    Role::Young
                };
                pro_role.insert(i, role);
            }
        }
    }
    let mut skip = vec![false; w.people.len()];
    for i in seat_people {
        skip[i] = true;
    }
    let (people, programs, schools) = (&mut w.people, &w.programs, &w.high_schools);
    let n_progs = programs.len();
    let _ = n_progs;
    people.par_iter_mut().enumerate().for_each(|(i, p)| {
        if skip[i] {
            return;
        }
        let (frame, weeks, tw, role) = match p.stage {
            Stage::HighSchool | Stage::PrepSchool => {
                let s = schools[p.hs].state;
                let (f, wk, _, _) = &frames.states[s];
                (f, wk, worked.get(&school_key[p.hs]), Role::of(p.player.position, p.last_role))
            }
            Stage::College => {
                let Some(c) = p.college else { return };
                let (f, wk) = frames.division(programs[c].division);
                (f, wk, worked.get(&prog_key[c]), Role::of(p.player.position, p.last_role))
            }
            Stage::Declared => {
                let (f, wk) = frames.division(Division::FBS);
                (f, wk, None, Role::Young)
            }
            Stage::Pro => {
                let team = p.pro.as_ref().and_then(|c| c.team);
                let tw = team.and_then(|t| club_key.get(t)).and_then(|k| worked.get(k));
                (&frames.pro, &frames.pro_weeks, tw, *pro_role.get(&i).unwrap_or(&Role::Young))
            }
            Stage::Done | Stage::Away => return,
        };
        let block = block_of_week(frame, d);
        let out = p.pro.as_ref().is_some_and(|c| c.out_weeks > 0);
        let load = match tw {
            Some(tw) if !tw.own && !out => tw.load[role as usize],
            _ => 150.0,
        };
        p.player.train.add_load(load);
        if !p.player.train.pending() {
            return;
        }
        let base = weekly_base(frame, weeks, block);
        let b = cover(&mut p.player.train, &base);
        let own = |p: &crate::world::Person| [own_ratio(&p.player, p.pro.as_ref().is_some_and(|c| c.paid)); FAMILIES];
        let mut r = match tw {
            Some(tw) if !tw.own && (!tw.voluntary || attends(&p.player, seed)) => tw.r[role as usize],
            _ => own(p),
        };
        // Hurt: rehab keeps a little of the body work, nothing else.
        let attend = if out {
            r = [0.6, 0.4, 0.4, 0.1, 0.0, 0.2, 0.5];
            0.7
        } else {
            1.0
        };
        let s = super::work::share(&b, &r, attend);
        let last = frame.block(d.add(7)) != block;
        if owe(&mut p.player.train, &s) || last {
            settle(&mut p.player, None, seed);
        }
    });
    // The week's growth reaches the college teams kept for the season.
    refresh_college_teams(w);
}

/// The college teams the season keeps (built at camp) take the players'
/// ratings as they stand now, and their practice impressions.
pub(crate) fn refresh_college_teams(w: &mut World) {
    let people = &w.people;
    let Some(cs) = w.college.as_mut() else { return };
    let programs = &w.programs;
    cs.teams.par_iter_mut().enumerate().for_each(|(c, t)| {
        let Some(t) = t.as_mut() else { return };
        let Some(prog) = programs.get(c) else { return };
        let by_id: HashMap<u32, usize> = prog.roster.iter().map(|&i| (people[i].player.id, i)).collect();
        for g in t.players.iter_mut() {
            if let Some(&i) = by_id.get(&g.id) {
                g.refresh(&people[i].player);
            }
        }
    });
}

/// Template days for a slot, your own where you set them (`seat`).
pub fn template_of(templates: &[(Slot, Template)], tier: Tier, slot: Slot, style: &Style) -> Template {
    templates.iter().find(|(s, _)| *s == slot).map_or_else(|| styled(tier, slot, style), |(_, t)| t.clone())
}
