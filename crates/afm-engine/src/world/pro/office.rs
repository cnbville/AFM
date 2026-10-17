//! AI front offices (section 5, topic 7): each team's general manager with
//! his style and hidden biases, its owner's traits, the fans' mood, and the
//! yearly call to contend or rebuild. Every trait is hidden (the game shows
//! only what a GM does). Spreads are Claude's, proposed, not confirmed.

use crate::positions::Position;
use crate::rng::{Rng, clamp};
use crate::world::World;

/// A general manager (topic 7): his judgment, his lean and his biases
/// (3.1-3.10), each a strength from 0 (none) to 1.
#[derive(Clone, Debug, Default)]
pub struct Gm {
    pub name: String,
    pub since: i32,
    /// Judgment, 1-20 (mirrored into the league table's `draft_skill`).
    pub skill: f64,
    /// Topic 2 question 1: 0 reads stats and totals, 1 reads film and efficiency.
    pub film_lean: f64,
    /// 3.1 keeps backing a player he paid or traded big for.
    pub sunk_cost: f64,
    /// 3.2 gives up early on a slow developer.
    pub impatience: f64,
    /// 3.3 marks down the last regime's picks.
    pub not_my_guy: f64,
    /// 3.4 overpays his own veterans.
    pub loyalty: f64,
    /// 3.5 weighs last season over the body of work.
    pub recency: f64,
    /// 3.6 chases the last champion's style.
    pub copycat: f64,
    /// 3.7 trusts his own grades over the consensus.
    pub overconfidence: f64,
    /// 3.8 upside (1) over a safe floor (0).
    pub risk: f64,
    /// 3.9 name value (Pro Bowls, stats) over quieter production.
    pub name_value: f64,
    /// 3.10 a position he overvalues.
    pub fixation: Option<Position>,
    /// Topic 6.1: the classic (Jimmy Johnson) pick chart, or a flatter
    /// surplus-value chart.
    pub classic_chart: bool,
    /// Styles (topic 7): won't pay running backs; restructures every year
    /// (the Saints); void years and early extensions (the Eagles).
    pub pays_backs: bool,
    pub restructures: f64,
    pub early_extensions: f64,
}

/// An owner's hidden traits (3.11-3.17).
#[derive(Clone, Debug, Default)]
pub struct Owner {
    /// 3.11 hands-off (0) to Jerry Jones (1).
    pub meddling: f64,
    /// 3.12 his pet player (a person index), protected from benching and cuts.
    pub pet: Option<usize>,
    /// 3.13 will to put fully guaranteed money in escrow.
    pub cash_will: f64,
    /// 3.14 patience with a rebuild.
    pub patience: f64,
    /// 3.15 the pull to make a splash for the fans.
    pub pr: f64,
    /// 3.16 overrides the draft room for a big name or a local hero.
    pub draft_override: f64,
    /// 3.17 his trust in the GM, 0-1: rises and falls with results.
    pub trust: f64,
}

/// Where a team stands this year (topic 7 question 1).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Mode {
    Contend,
    #[default]
    Middle,
    Rebuild,
}

/// A team's front office.
#[derive(Clone, Debug, Default)]
pub struct Office {
    pub gm: Gm,
    pub owner: Owner,
    /// The fans' mood, -1 to 1: results, hype, star signings, renewals.
    pub fans: f64,
    pub mode: Mode,
    /// Why it chose its mode, for pages and trials.
    pub why: String,
    /// GMs fired or replaced at this team, for trials.
    pub gm_changes: u32,
}

fn trait01(rng: &mut Rng, mean: f64) -> f64 {
    clamp(rng.normal(mean, 0.22), 0.0, 1.0)
}

/// A new general manager: judgment from the league's spread, biases each
/// drawn on their own; analytics (film lean, surplus charts) spread by era.
pub(crate) fn new_gm(rng: &mut Rng, year: i32, skill: f64, name: String) -> Gm {
    let era = clamp((year - 1995) as f64 / 30.0, 0.0, 1.0);
    let fixation = rng.chance(0.3).then(|| Position::ALL[rng.int(0, 10) as usize]);
    Gm {
        name,
        since: year,
        skill,
        film_lean: trait01(rng, 0.35 + 0.3 * era),
        sunk_cost: trait01(rng, 0.45),
        impatience: trait01(rng, 0.35),
        not_my_guy: trait01(rng, 0.4),
        loyalty: trait01(rng, 0.4),
        recency: trait01(rng, 0.45),
        copycat: trait01(rng, 0.4),
        overconfidence: trait01(rng, 0.45),
        risk: trait01(rng, 0.5),
        name_value: trait01(rng, 0.5 - 0.2 * era),
        fixation,
        classic_chart: rng.chance(0.85 - 0.5 * era),
        pays_backs: rng.chance(0.8 - 0.35 * era),
        restructures: trait01(rng, 0.25 + 0.2 * era),
        early_extensions: trait01(rng, 0.2 + 0.3 * era),
    }
}

pub(crate) fn new_owner(rng: &mut Rng) -> Owner {
    Owner {
        meddling: clamp(rng.normal(0.3, 0.25), 0.0, 1.0),
        pet: None,
        cash_will: trait01(rng, 0.5),
        patience: trait01(rng, 0.5),
        pr: trait01(rng, 0.4),
        draft_override: clamp(rng.normal(0.15, 0.2), 0.0, 1.0),
        trust: 0.7,
    }
}

/// Every team's front office at the start of history.
pub(crate) fn staff_offices(w: &mut World) {
    let year = w.year;
    let mut rng = Rng::new(w.options.seed ^ 0x0FF1_CE00);
    for t in 0..w.clubs.len() {
        let name = super::super::names::person_name(super::super::ethnicity::Ethnicity::White, year - 50, &mut rng);
        let since = year - rng.int(0, 8);
        let mut gm = new_gm(&mut rng, since, w.nfl[t].draft_skill, name);
        gm.skill = w.nfl[t].draft_skill;
        w.clubs[t].office = Office { gm, owner: new_owner(&mut rng), ..Default::default() };
    }
}

/// A new general manager takes over (fired, or hired away).
pub(crate) fn replace_gm(w: &mut World, t: usize, why: &str) {
    let year = w.year;
    let mut rng = w.rng.fork();
    let skill = crate::league::new_staff_rating(&mut rng);
    let eth = if rng.chance(0.15 + 0.15 * clamp((year - 1990) as f64 / 35.0, 0.0, 1.0)) {
        super::super::ethnicity::Ethnicity::Black
    } else {
        super::super::ethnicity::Ethnicity::White
    };
    let name = super::super::names::person_name(eth, year - rng.int(38, 55), &mut rng);
    let old = std::mem::take(&mut w.clubs[t].office.gm.name);
    let mut gm = new_gm(&mut rng, year, skill, name.clone());
    // Copycats (3.6) take after the last champion's lean.
    if let Some(&(_, champ)) = w.nfl_champions.last()
        && champ != t
    {
        let c = &w.clubs[champ].office.gm;
        let pull = gm.copycat * 0.5;
        gm.film_lean += pull * (c.film_lean - gm.film_lean);
        gm.pays_backs = if rng.chance(pull) { c.pays_backs } else { gm.pays_backs };
    }
    w.nfl[t].draft_skill = skill;
    let office = &mut w.clubs[t].office;
    office.gm = gm;
    office.owner.trust = 0.7;
    office.gm_changes += 1;
    let team = w.nfl[t].name;
    let text = format!("The {team} replace general manager {old} with {name} ({why}).");
    w.calendar.post(
        crate::calendar::Event::new(
            w.today,
            crate::calendar::Level::Nfl,
            crate::calendar::Kind::Coaching,
            crate::calendar::Loudness::Medium,
            text,
        )
        .team(t),
    );
}

/// The fans' mood after a season (topic 7 question 1): wins against
/// expectation, a playoff run, a title; a big market's fans and press turn
/// faster. Kept between -1 and 1, fading toward neutral.
pub(crate) fn fan_year(w: &mut World) {
    let Some(s) = w.nfl_season.as_ref() else { return };
    let n = s.wins.len().min(w.clubs.len());
    let games = s.weeks.len().max(16) as f64;
    for t in 0..n {
        if !s.active[t] {
            continue;
        }
        let wins = s.wins[t] as f64 + 0.5 * s.ties[t] as f64;
        let over = (wins - s.expect[t]) / 3.0;
        let playoff = if s.seed[t].is_some() { 0.15 } else { -0.05 };
        let run = 0.1 * s.out_round[t].saturating_sub(1) as f64;
        let level = (wins / games - 0.5) * 1.2;
        let o = &mut w.clubs[t].office;
        o.fans = clamp(0.5 * o.fans + 0.3 * over + level + playoff + run, -1.0, 1.0);
        // The owner's trust in his GM follows results and the fans.
        o.owner.trust = clamp(o.owner.trust + 0.1 * over + 0.08 * o.fans, 0.0, 1.0);
    }
}

/// The yearly call to contend or rebuild (topic 7 question 1): the GM's
/// read of his roster, the quarterback's age and contract, the cap ahead,
/// the core's age, owner patience and the fans; a GM on the hot seat (low
/// trust) chases now. A readable reason is kept.
pub(crate) fn choose_modes(w: &mut World) {
    let year = w.year;
    let active: Vec<usize> = (0..w.clubs.len()).filter(|&t| super::super::franchise::active(t, year)).collect();
    let strengths: Vec<(usize, f64)> = active.iter().map(|&t| (t, w.nfl[t].strength())).collect();
    let mean = strengths.iter().map(|x| x.1).sum::<f64>() / strengths.len().max(1) as f64;
    for &(t, st) in &strengths {
        let qb = w.clubs[t].depth.get(Position::QB.index()).and_then(|d| d.first()).map(|&i| &w.people[i].player);
        let qb_good = qb.is_some_and(|p| p.overall() >= 80.0);
        let qb_young = qb.is_some_and(|p| p.age <= 27);
        let core_age = {
            let mut ages = Vec::new();
            for pos in Position::ALL {
                for &i in w.clubs[t]
                    .depth
                    .get(pos.index())
                    .map_or(&[][..], |d| d.as_slice())
                    .iter()
                    .take(super::depth_slots(pos).starters)
                {
                    ages.push(w.people[i].player.age as f64);
                }
            }
            ages.iter().sum::<f64>() / ages.len().max(1) as f64
        };
        let space = super::space(w, t, year) / super::money::price_cap(year);
        let o = &w.clubs[t].office;
        let hot_seat = o.owner.trust < 0.35;
        let x = (st - mean) / 1.5 + if qb_good { 0.6 } else { -0.3 } + 0.3 * space - 0.15 * (core_age - 26.5)
            + if hot_seat { 0.5 } else { 0.0 }
            + 0.3 * o.fans * (1.0 - o.owner.patience);
        let (mode, why) = if x > 0.6 {
            (
                Mode::Contend,
                if qb_good && qb_young { "a young quarterback's window" } else { "a roster built to win now" },
            )
        } else if x < -0.6 && !hot_seat && o.owner.patience > 0.3 {
            (
                Mode::Rebuild,
                if qb_good { "an old core around the quarterback" } else { "no quarterback and a thin roster" },
            )
        } else {
            (Mode::Middle, if hot_seat { "a GM on the hot seat chasing wins" } else { "a team in between" })
        };
        let o = &mut w.clubs[t].office;
        o.mode = mode;
        o.why = why.to_string();
    }
}
