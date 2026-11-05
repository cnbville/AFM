//! The money tally (`money/<season>.tally`, lane NB11, Data overhaul item
//! 40): what each team paid for its players in a season, kept when the
//! season closes (the day after the Super Bowl, still the same league year
//! and NIL year):
//! - an NFL club: the cap, the carryover, the cap charges, dead money and
//!   cash paid, and each player on its books with his cap hit, the cash he
//!   was paid that league year and his deal's average a year
//!   (OverTheCap's numbers);
//! - a college program: its NIL money (the collective's budget, the
//!   school's revenue share inside it from July 2025, the boosters) and
//!   each player's money that year, split into the two pots (the school's
//!   revenue share and the collective's NIL deals, B18) and his brand
//!   deals;
//! - a high school: its football budget and its books so far, line by line
//!   (`finance::hs::LINES`), the fee to play and the crowd.
//!
//! Only reads the world.

use crate::calendar::Date;
use crate::positions::Position;
use crate::world::World;

use super::Level;

/// The values an NFL row keeps, in order. Append only.
pub const NFL_VALS: [&str; 6] = ["cap", "carryover", "charges", "dead", "cash", "players"];
/// A college row's values.
pub const COLLEGE_VALS: [&str; 5] = ["budget", "school", "boosters", "paid", "players"];
/// A high school row's values: the budget's lines, the books' lines so
/// far, the fee to play and the crowd.
pub const HS_EXTRA: [&str; 2] = ["fee", "crowd"];

/// One player's money in a season. NFL: `a` cap hit, `b` cash paid, `c`
/// his deal's average a year. College: `a` the school's revenue share,
/// `b` the collective's NIL deals, `c` brand deals.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Pay {
    pub id: u32,
    pub pos: Option<Position>,
    pub a: f32,
    pub b: f32,
    pub c: f32,
}

/// One team's money in a season.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Row {
    pub level: u8,
    pub team: u32,
    pub season: i32,
    pub vals: Vec<f32>,
    pub pay: Vec<Pay>,
}

impl Row {
    /// A value by its name in the level's list (0 where it isn't kept).
    pub fn val(&self, name: &str) -> f64 {
        let names: &[&str] = match self.level {
            1 => &NFL_VALS,
            0 => &COLLEGE_VALS,
            _ => &HS_EXTRA,
        };
        let off = if self.level == Level::Hs as u8 { 2 * crate::world::finance::hs::N } else { 0 };
        names.iter().position(|n| *n == name).and_then(|k| self.vals.get(off + k)).map_or(0.0, |v| *v as f64)
    }

    /// A high school's budget line (`finance::hs` line index).
    pub fn budget_line(&self, k: usize) -> f64 {
        self.vals.get(k).map_or(0.0, |v| *v as f64)
    }

    /// A high school's books so far on a line.
    pub fn books_line(&self, k: usize) -> f64 {
        self.vals.get(crate::world::finance::hs::N + k).map_or(0.0, |v| *v as f64)
    }
}

/// A row's key: the level, then the team.
pub fn key(level: Level, team: u32) -> u64 {
    (level as u64) << 32 | team as u64
}

/// Every team's money as the season closes.
pub fn rows(w: &World, season: i32) -> Vec<(u64, Row)> {
    let mut out = Vec::new();
    // The NFL: the league year is the season's until March.
    for (t, club) in w.clubs.iter().enumerate().take(w.nfl.len()) {
        let mut pay = Vec::new();
        for &i in &club.roster {
            let p = &w.people[i];
            let Some(c) = p.pro.as_ref().and_then(|c| c.contract.as_ref()) else { continue };
            if c.team != t {
                continue;
            }
            let hit = c.cap_hit(season);
            let cash = c.year(season).map_or(0.0, |y| y.cash());
            if hit == 0.0 && cash == 0.0 {
                continue;
            }
            pay.push(Pay {
                id: p.player.id,
                pos: Some(p.player.position),
                a: hit as f32,
                b: cash as f32,
                c: c.apy() as f32,
            });
        }
        pay.sort_by_key(|x| x.id);
        let cash = club.cash.iter().find(|c| c.0 == season).map_or(0.0, |c| c.1);
        let vals = vec![
            crate::world::pro::money::price_cap(season) as f32,
            club.carryover as f32,
            crate::world::pro::payroll(w, t, season) as f32,
            club.dead_in(season) as f32,
            cash as f32,
            pay.len() as f32,
        ];
        out.push((key(Level::Nfl, t as u32), Row { level: Level::Nfl as u8, team: t as u32, season, vals, pay }));
    }
    // College: the NIL year that began on 1 July of the season.
    let july = Date::ymd(season, 7, 1);
    let mut by_prog: Vec<Vec<Pay>> = vec![Vec::new(); w.programs.len()];
    for p in &w.people {
        if p.stage != crate::world::Stage::College || p.nil_income <= 0.0 {
            continue;
        }
        let Some(j) = p.college else { continue };
        let Some(prog) = w.programs.get(j) else { continue };
        let brand = (p.brand as f64).clamp(0.0, p.nil_income);
        let deal = p.nil_income - brand;
        let budget = prog.collective.budget;
        let share =
            if budget > 0.0 { (crate::world::ncaa::school_pot(prog, july) / budget).clamp(0.0, 1.0) } else { 0.0 };
        by_prog[j].push(Pay {
            id: p.player.id,
            pos: Some(p.player.position),
            a: (deal * share) as f32,
            b: (deal * (1.0 - share)) as f32,
            c: brand as f32,
        });
    }
    for (j, pay) in by_prog.into_iter().enumerate() {
        let prog = &w.programs[j];
        if pay.is_empty() && prog.collective.budget <= 0.0 {
            continue;
        }
        let paid: f64 = pay.iter().map(|x| (x.a + x.b) as f64).sum();
        let vals = vec![
            prog.collective.budget as f32,
            crate::world::ncaa::school_pot(prog, july) as f32,
            prog.collective.boosters() as f32,
            paid as f32,
            pay.len() as f32,
        ];
        out.push((
            key(Level::College, j as u32),
            Row { level: Level::College as u8, team: j as u32, season, vals, pay },
        ));
    }
    // High school: the money year that began on 1 July of the season.
    for (h, s) in w.finance.hs.iter().enumerate() {
        if s.budget.year == 0 && s.now.year == 0 {
            continue;
        }
        let mut vals: Vec<f32> = s.budget.l.to_vec();
        vals.extend_from_slice(&s.now.l);
        vals.push(s.fee as f32);
        vals.push(s.crowd as f32);
        out.push((
            key(Level::Hs, h as u32),
            Row { level: Level::Hs as u8, team: h as u32, season, vals, pay: Vec::new() },
        ));
    }
    out.sort_by_key(|r| r.0);
    out
}
