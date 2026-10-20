//! College football's rules by era, and the NCAA (dynasty items 11.4-11.8,
//! Raichel 2026-10-15). Scholarship and signing limits follow each era, NIL
//! money starts in July 2021 and has a ceiling, revenue sharing starts in
//! 2025, and before NIL a few programs have a rogue booster who pays recruits
//! under the table, at a real risk of NCAA penalties. Sources and the
//! proposed numbers are in `docs/world-model.md` ("Era limits, NIL and
//! prestige").

use super::coach::{Employer, Group};
use super::data::Division;
use super::news::{Channel, NewsItem, NewsKind};
use super::program::Program;
use super::season::key_uniform;
use super::{Stage, World};
use crate::positions::Position;
use crate::rng::clamp;

/// The first offseason with legal NIL money (the NCAA's interim policy, July 1, 2021).
pub const NIL_START: i32 = 2021;
/// The first season under the House settlement (from July 1, 2025): roster
/// limits replace scholarship limits, and schools may share revenue.
pub const HOUSE_START: i32 = 2025;

/// Whether players may take NIL money in a year's offseason: the portal and
/// signing day after that season. The class of 2021 signed without it; the
/// class of 2022 was the first.
pub fn nil_era(year: i32) -> bool {
    year >= NIL_START
}

// --- Roster and signing limits (11.4) ------------------------------------------------

/// FCS programs that stayed out of the House settlement (SI, July 15, 2025;
/// Saint Francis is not in the world). They keep the old scholarship rules.
const FCS_OPTED_OUT: [&str; 35] = [
    "Eastern Washington",
    "Idaho",
    "Idaho State",
    "Northern Arizona",
    "Portland State",
    "UC Davis",
    "Brown",
    "Columbia",
    "Cornell",
    "Dartmouth",
    "Harvard",
    "Penn",
    "Princeton",
    "Yale",
    "North Carolina Central",
    "Central Connecticut",
    "Mercyhurst",
    "Stonehill",
    "Wagner",
    "Charleston Southern",
    "Gardner-Webb",
    "Tennessee State",
    "Bucknell",
    "Colgate",
    "Holy Cross",
    "Lafayette",
    "Lehigh",
    "Presbyterian",
    "Stetson",
    "The Citadel",
    "VMI",
    "Houston Christian",
    "Incarnate Word",
    "Central Arkansas",
    "West Georgia",
];

/// What a level's rules allow in a season: players on scholarship (or on
/// aid), and new signees a year (None: no NCAA cap). The world has no
/// walk-ons yet, so its rosters are the scholarship players.
/// - FBS: 95 scholarships until 1991, 85 from 1992 (the 1991 convention),
///   and 25 initial counters a year. From 2025 the House settlement's
///   105-man roster, every player may be on scholarship, no yearly cap.
/// - FCS: 63 equivalencies over at most 85 players on aid, 30 initial
///   counters a year (Bylaw 15.5.6.2; 63 since 1994, 70 before). From 2025
///   the programs that opted in follow the FBS rule; the rest keep the old one.
/// - D2: 36 equivalencies and no cap on players on aid, so rosters keep the
///   world's earlier 110 (proposed, not confirmed).
/// - JUCO: the NJCAA's 85 scholarships.
pub fn scholarship_rules(d: Division, season: i32, opted_out: bool) -> (usize, Option<usize>) {
    match d {
        Division::FBS if season >= HOUSE_START => (105, None),
        // The yearly cap before 1992 isn't in the sources; only the spin-up
        // years use it.
        Division::FBS if season < 1992 => (95, Some(25)),
        Division::FBS => (85, Some(25)),
        Division::FCS if season >= HOUSE_START && !opted_out => (105, None),
        Division::FCS => (85, Some(30)),
        Division::D2 => (110, None),
        // NAIA: 24 equivalencies, no cap on players (proposed, not
        // confirmed: rosters like Division II's). Division III gives no
        // athletic aid and carries the biggest rosters, about 115.
        Division::NAIA => (110, None),
        Division::D3 => (115, None),
        Division::JUCO => (85, None),
    }
}

/// Where the NCAA sets no yearly cap, a staff still signs at most about 1.4
/// normal classes (the world's earlier pick, proposed, not confirmed: classes
/// of 28, 24, 30 and 40).
fn staff_cap(d: Division) -> usize {
    match d {
        Division::FBS => 39,
        Division::FCS => 33,
        Division::D2 => 42,
        Division::NAIA => 42,
        Division::D3 => 45,
        Division::JUCO => 56,
    }
}

/// A program's roster limit for a season, and the most new signees it may
/// add, after any NCAA scholarship penalty. Every roster rule reads them here.
pub fn roster_limits(prog: &Program, season: i32) -> (usize, usize) {
    // No team that season or the next (college history): nobody to sign. A
    // new program signs its first class a year early and practices.
    if !prog.plays(season) && !prog.plays(season + 1) {
        return (0, 0);
    }
    let opted_out = prog.division == Division::FCS && FCS_OPTED_OUT.contains(&prog.real_name.as_str());
    let (limit, yearly) = scholarship_rules(prog.division, season, opted_out);
    let cut = prog.era.cut_in(season) as usize;
    (limit.saturating_sub(cut), yearly.unwrap_or_else(|| staff_cap(prog.division)).saturating_sub(cut))
}

/// Whether a program's limit counts its whole roster, walk-ons included:
/// the House settlement's roster limit. Before it (and at the FCS programs
/// that stayed out, D2 and junior colleges) the limit counts players on
/// aid, and walk-ons sit outside it.
pub fn whole_roster_limit(prog: &Program, season: i32) -> bool {
    let opted_out = prog.division == Division::FCS && FCS_OPTED_OUT.contains(&prog.real_name.as_str());
    season >= HOUSE_START && (prog.division == Division::FBS || (prog.division == Division::FCS && !opted_out))
}

// --- NIL money (11.5, 11.7, 11.8) ---------------------------------------------------------

/// The biggest NIL collectives raise about $20M a year: Ohio State's players
/// got "around $20 million" in 2024, Notre Dame's collective took in $20.5M
/// in 2023, the Texas One Fund spent $13.3M, and "very few collectives
/// actually had $20 million dollars to spend a year" (Opendorse). No booster
/// base gives more than this.
pub const COLLECTIVE_CEILING: f64 = 20.0e6;

/// How big the NIL market was each year against 2024 (Opendorse, Power 4
/// totals: $393M in 2021, $667M in 2022, $840M in 2023, $1.0B in 2024).
/// Collectives formed and grew over the first years (proposed, not
/// confirmed: every collective follows the market).
pub fn nil_market(year: i32) -> f64 {
    match year {
        y if y < NIL_START => 0.0,
        2021 => 0.39,
        2022 => 0.67,
        2023 => 0.84,
        _ => 1.0,
    }
}

/// Revenue a school shares with its football players from 2025-26 (House
/// settlement): a $20.5M cap that grows 4% a year. Power 4 schools pay the
/// full cap and give football about 75% (the Texas model, $15.37M). The
/// Group of 5 uses the American's floor, $10M over three years. FCS and below
/// share nothing (proposed, not confirmed: no published figures).
pub fn revenue_share(prog: &Program, year: i32) -> f64 {
    if year < HOUSE_START {
        return 0.0;
    }
    let cap = 20.5e6 * 1.04f64.powi(year - HOUSE_START);
    let paid = match prog.division {
        Division::FBS if prog.power_four() => cap,
        Division::FBS => 10.0e6 / 3.0,
        _ => 0.0,
    };
    0.75 * paid
}

/// NIL money a program can pay its players in a year's offseason (the
/// portal and signing day): its collective, within the ceiling and the
/// market as it stood that year, plus revenue sharing. None before July 2021.
pub fn nil_budget(prog: &Program, year: i32) -> f64 {
    nil_market(year) * prog.collective.boosters() + revenue_share(prog, year)
}

/// The top of the real range for a starter at a major program (CBS via SI,
/// 2024; `data/real-world/rates.json`): no NIL offer goes above it. Tight
/// ends and specialists aren't in the data (proposed, not confirmed: tight
/// ends at the receivers' top, specialists at twice their market rate).
pub fn position_nil_max(pos: Position) -> f64 {
    match pos {
        Position::QB => 800_000.0,
        Position::OT | Position::IOL => 500_000.0,
        Position::EDGE | Position::DT => 600_000.0,
        Position::RB | Position::WR | Position::TE | Position::LB => 300_000.0,
        Position::CB | Position::S => 225_000.0,
        _ => 50_000.0,
    }
}

// --- Rogue boosters and the NCAA (11.6) ----------------------------------------------------

/// A program's standing with the NCAA, and its yearly record for trials.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct ProgramEra {
    /// A rogue booster paying recruits under the table (hidden), before NIL.
    pub rogue: Option<RogueBooster>,
    /// Payments investigators could still find (hidden): the NCAA's statute
    /// of limitations is four years.
    pub(crate) paid: Vec<Payment>,
    /// NCAA cases against the program.
    pub cases: Vec<Case>,
    /// Scholarships, and signees, taken away each season through `cut_until`.
    pub cut: u32,
    pub cut_until: i32,
    /// The last season with no postseason: no conference title, no playoff.
    pub ban_until: i32,
    /// Roster size when signing day began.
    pub(crate) roster_before: usize,
    /// Season by season: limit, roster, signees and money (trial tooling).
    pub ledger: Vec<EraYear>,
    /// NIL offers signed: (year, position, stars, dollars a year) (trial tooling).
    pub offers: Vec<(i32, Position, u8, f64)>,
}

impl ProgramEra {
    /// Scholarships taken away for a season.
    pub fn cut_in(&self, season: i32) -> u32 {
        if season <= self.cut_until { self.cut } else { 0 }
    }

    /// Whether a season's postseason is off limits.
    pub fn banned(&self, season: i32) -> bool {
        season <= self.ban_until
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct RogueBooster {
    pub since: i32,
    /// What he is willing to pay a year, in dollars.
    pub pot: f64,
    /// This signing day's targets: (person index, dollars).
    pub(crate) targets: Vec<(usize, f64)>,
}

#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub struct Payment {
    pub year: i32,
    pub player: u32,
    pub amount: f64,
    pub head_coach: Option<usize>,
    pub position_coach: Option<usize>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Case {
    pub year: i32,
    /// What investigators found within four years: dollars and players.
    pub money: f64,
    pub players: usize,
    /// Scholarships taken away in all, over three seasons.
    pub scholarships: u32,
    pub postseason_years: u32,
    /// The coach named in a show-cause order, its years, and whether he was the head coach.
    pub show_cause: Option<(usize, u32, bool)>,
}

#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub struct EraYear {
    pub season: i32,
    pub limit: u32,
    pub roster: u32,
    pub signees: u32,
    pub nil_budget: f64,
    pub boosters: f64,
    pub banned: bool,
}

/// What a rogue booster pays a recruit, by stars: Alabama booster Logan
/// Young paid about $150,000 to land a top defensive lineman in 2000
/// (proposed, not confirmed: $50K for a 4-star, $15K for a 3-star).
fn booster_price(stars: u8) -> f64 {
    match stars {
        5 => 150_000.0,
        4 => 50_000.0,
        _ => 15_000.0,
    }
}

/// A new rogue booster's yearly pot, $50K-$300K (proposed, not confirmed:
/// SMU's boosters paid 13 players about $61,000 in 1985-86; one Alabama
/// recruit cost $150,000).
fn booster_pot(u: f64) -> f64 {
    (50_000.0f64.ln() + u * (300_000.0f64.ln() - 50_000.0f64.ln())).exp()
}

/// Each year an FBS program without one gains a rogue booster with this
/// chance times its fan base (0-1), half again with a big booster (proposed,
/// not confirmed: about 9 of 138 programs at a time).
const ROGUE_ARRIVAL: f64 = 0.02;
/// A rogue booster stops on his own (falls out with the staff, runs out of
/// money, dies) with this chance a year (proposed, not confirmed).
const ROGUE_QUITS: f64 = 0.1;
/// The chance a year that payments surface is 1 - exp(-paid / this), with
/// `paid` the dollars paid in the last four years (proposed, not confirmed:
/// $500K in four years is found about 6% of the time each year, $1.5M 17%).
const DETECTION_SCALE: f64 = 8.0e6;

/// Signing day, before the offers: every program notes its roster, and each
/// rogue booster picks the recruits he will pay. He goes after the best his
/// program recruits hardest, in its own footprint.
pub(crate) fn before_matching(w: &mut World, recruits: &[usize]) {
    let year = w.year;
    let seed = w.options.seed as u64 ^ ((year as u64) << 20) ^ 0x0B00;
    let ranked: Vec<usize> = recruits.iter().copied().filter(|&i| w.people[i].stars >= 3).collect();
    for j in 0..w.programs.len() {
        w.programs[j].era.roster_before = w.programs[j].roster.len();
        let Some(pot) = w.programs[j].era.rogue.as_ref().map(|r| r.pot) else { continue };
        let mut targets: Vec<(usize, f64)> = Vec::new();
        if !nil_era(year) {
            let prog = &w.programs[j];
            let pseed = seed ^ (j as u64).wrapping_mul(0x1000_0001);
            let mut cands: Vec<(f64, usize)> = ranked
                .iter()
                .filter_map(|&i| {
                    let p = &w.people[i];
                    let reach = prog.footprint[p.home_state];
                    (reach >= 0.3).then(|| (p.stars as f64 + reach + 0.5 * key_uniform(pseed, p.player.id as u64), i))
                })
                .collect();
            cands.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
            let mut left = pot;
            for &(_, i) in &cands {
                let amount = booster_price(w.people[i].stars);
                if amount <= left {
                    left -= amount;
                    targets.push((i, amount));
                }
                if targets.len() >= 4 || left < booster_price(3) {
                    break;
                }
            }
        }
        if let Some(r) = w.programs[j].era.rogue.as_mut() {
            r.targets = targets;
        }
    }
}

/// The booster's cash a program can put into its offer to a recruit.
pub(crate) fn booster_cash(prog: &Program, person: usize) -> f64 {
    prog.era.rogue.as_ref().and_then(|r| r.targets.iter().find(|t| t.0 == person)).map_or(0.0, |t| t.1)
}

/// After signing day: rogue boosters pay the targets who signed, and the
/// season's roster, class and money go on the record.
pub(crate) fn after_signing(w: &mut World) {
    let year = w.year;
    for j in 0..w.programs.len() {
        let targets = w.programs[j].era.rogue.as_mut().map(|r| std::mem::take(&mut r.targets)).unwrap_or_default();
        for (i, amount) in targets {
            let p = &w.people[i];
            if p.stage != Stage::College || p.college != Some(j) || p.path.len() != 1 {
                continue;
            }
            let prog = &w.programs[j];
            let pay = Payment {
                year,
                player: p.player.id,
                amount,
                head_coach: prog.head_coach,
                position_coach: prog.position_coaches[Group::of(p.player.position) as usize],
            };
            w.programs[j].era.paid.push(pay);
        }
        if w.recording {
            let prog = &w.programs[j];
            let season = year + 1;
            let rec = EraYear {
                season,
                limit: roster_limits(prog, season).0 as u32,
                roster: prog.roster.len() as u32,
                signees: prog.roster.len().saturating_sub(prog.era.roster_before) as u32,
                nil_budget: prog.collective.budget,
                boosters: prog.collective.boosters(),
                banned: prog.era.banned(season),
            };
            w.programs[j].era.ledger.push(rec);
        }
    }
}

/// The NCAA each offseason, before the coaching carousel. A booster's
/// payments surface with a chance that grows with how much he paid in the
/// last four years. Rogue boosters come and go before NIL; once NIL is legal
/// their money goes through the collective, though old payments can still
/// surface (Tennessee's 2023 case covered 2018-20).
pub(crate) fn enforcement(w: &mut World) {
    let year = w.year;
    let seed = w.options.seed as u64 ^ ((year as u64) << 20) ^ 0x0C0A;
    for j in 0..w.programs.len() {
        if w.programs[j].division != Division::FBS {
            continue;
        }
        let era = &mut w.programs[j].era;
        era.paid.retain(|p| year - p.year < 4);
        let found: f64 = era.paid.iter().map(|p| p.amount).sum();
        if found > 0.0 && key_uniform(seed, j as u64) < 1.0 - (-found / DETECTION_SCALE).exp() {
            ncaa_case(w, j);
            continue;
        }
        let u = key_uniform(seed ^ 0x77, j as u64);
        let prog = &w.programs[j];
        if prog.era.rogue.is_some() {
            if nil_era(year) || u < ROGUE_QUITS {
                w.programs[j].era.rogue = None;
            }
        } else if !nil_era(year) {
            let arrive = ROGUE_ARRIVAL * prog.fans * if prog.collective.big_booster.is_some() { 1.5 } else { 1.0 };
            if u < arrive {
                let pot = booster_pot(key_uniform(seed ^ 0x99, j as u64));
                w.programs[j].era.rogue = Some(RogueBooster { since: year, pot, targets: Vec::new() });
            }
        }
    }
}

/// An NCAA case: the real penalty types, acting through the world's own
/// systems. Scholarships and signees are cut for three seasons (Alabama 2002:
/// 21 over three years; USC 2010: 30; Ohio State 2011: 9; Miami 2013: 9), the
/// postseason is closed for a year or two (Alabama, USC, Ole Miss: two; Ohio
/// State: one), and one coach gets a show-cause order (USC's McNair one
/// year, Miami's assistants two, Tressel five, Ole Miss assistants five and
/// eight), which ends his college job. Penalties grow with the money found
/// (proposed, not confirmed: one scholarship per $15K, 3 to 30; a one-year
/// ban from $100K and two from $300K; the head coach is named in one case
/// in four, otherwise the recruit's position coach, for 1 year plus one per
/// $100K, up to 8). The program disassociates the booster.
fn ncaa_case(w: &mut World, j: usize) {
    let year = w.year;
    let seed = w.options.seed as u64 ^ ((year as u64) << 20) ^ 0x0CA5E;
    let paid = std::mem::take(&mut w.programs[j].era.paid);
    w.programs[j].era.rogue = None;
    let money: f64 = paid.iter().map(|p| p.amount).sum();
    let scholarships = clamp((money / 15_000.0).round(), 3.0, 30.0) as u32;
    let ban: u32 = if money >= 300_000.0 {
        2
    } else if money >= 100_000.0 {
        1
    } else {
        0
    };
    let biggest = paid.iter().copied().max_by(|a, b| a.amount.total_cmp(&b.amount));
    let head = key_uniform(seed, j as u64) < 0.25;
    let named = biggest.and_then(|b| if head { b.head_coach } else { b.position_coach.or(b.head_coach) });
    let years = clamp(1.0 + (money / 100_000.0).round(), 1.0, 8.0) as u32;
    {
        let era = &mut w.programs[j].era;
        era.cut = scholarships.div_ceil(3);
        era.cut_until = year + 3;
        era.ban_until = era.ban_until.max(year + ban as i32);
    }
    let mut show_cause = None;
    if let Some(c) = named {
        let was_head = biggest.is_some_and(|b| b.head_coach == Some(c));
        w.coaches[c].show_cause = Some(year + years as i32);
        show_cause = Some((c, years, was_head));
        if matches!(w.coaches[c].job, Some(job) if matches!(job.employer, Employer::College(_))) {
            super::staff::leave(w, c, "left under an NCAA show-cause order");
        }
    }
    for pay in &paid {
        if let Some(p) = w.people.iter_mut().find(|p| p.player.id == pay.player) {
            p.note(format!(
                "NCAA investigators found that a booster paid his family about ${:.0}K during his recruitment.",
                pay.amount / 1e3
            ));
        }
    }
    w.programs[j].era.cases.push(Case {
        year,
        money,
        players: paid.len(),
        scholarships,
        postseason_years: ban,
        show_cause,
    });
    if w.recording {
        let name = w.programs[j].name(w.options.names).to_string();
        let ban_text = match ban {
            0 => String::new(),
            1 => ", a one-year postseason ban".to_string(),
            n => format!(", {}-year postseason ban", crate::text::with_article(&n.to_string())),
        };
        let coach_text = show_cause.map_or(String::new(), |(c, y, _)| {
            format!(
                " and {}-year show-cause order for {}",
                crate::text::with_article(&y.to_string()),
                w.coaches[c].name
            )
        });
        let line = format!(
            "NCAA penalties for a booster's payments to recruits: {scholarships} scholarships over three seasons{ban_text}{coach_text}."
        );
        w.programs[j].story.push((year, line));
        let text = format!(
            "The NCAA penalizes {name} for a booster's payments to {} recruits (about ${:.0}K): {scholarships} scholarships over three seasons{ban_text}{coach_text}.",
            paid.len(),
            money / 1e3
        );
        w.news.push(NewsItem {
            date: None,
            year,
            kind: NewsKind::NcaaPenalty,
            channel: Channel::News,
            program: Some(j),
            text,
        });
    }
}
