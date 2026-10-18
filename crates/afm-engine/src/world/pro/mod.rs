//! Section 5: the living NFL (topic 1) and team management. Every pro is a
//! person who came up through high school and college: drafted and
//! undrafted players join a team and stay people, with a contract, a
//! roster spot, snaps, development, cuts, free agency and retirement. At
//! the start of history the league is filled with generated veterans spread
//! like a real roster; the world's own classes replace them.
//!
//! Each team's depth chart is mirrored into `World::nfl` (the league's
//! `Member` lists, each carrying the person's index), so the strength,
//! need and draft logic read real players.

pub mod games;
pub mod honors;
pub mod leagues;
pub mod market;
pub mod money;
pub mod office;
pub mod trades;

use super::season::{key_normal, key_uniform};
use super::{Person, PersonCal, Stage, World};
use crate::calendar::{Date, Event, Kind, Level as CalLevel, Loudness};
use crate::league::{Member, depth_slots};
use crate::player::{Level, PlayingTime, ProspectOptions, generate_prospect};
use crate::positions::{POSITION_COUNT, Position};
use crate::rng::{Rng, clamp, round_half_up};

/// Where a pro is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    /// On a team's roster: the offseason roster, then the 53.
    Roster,
    PracticeSquad,
    /// Injured reserve (or the physically unable to perform list).
    Reserve,
    /// Unsigned: the street pool.
    FreeAgent,
    /// Playing in another league (index into `leagues`).
    Other(u8),
}

/// One pro season: where he played, how much and what he earned.
#[derive(Clone, Debug, Default)]
pub struct ProSeason {
    pub year: i32,
    pub team: Option<usize>,
    /// Games dressed and games started.
    pub games: u32,
    pub starts: u32,
    /// Offensive, defensive and special teams snaps.
    pub snaps: [u32; 3],
    /// The average strength (in standard deviations) of the teams he took
    /// his snaps against (topic 1.1: how hard the snaps were).
    pub opp: f64,
    /// Weeks on a practice squad and on injured reserve.
    pub ps_weeks: u32,
    pub ir_weeks: u32,
    /// Cash paid this league year.
    pub pay: f64,
    pub pro_bowl: bool,
    /// 1 first team, 2 second team.
    pub all_pro: u8,
    pub awards: Vec<&'static str>,
    /// His statistics (filled by the engine games).
    pub stats: crate::game::stats::StatLine,
}

/// A pro's career.
#[derive(Clone, Debug)]
pub struct ProCareer {
    pub team: Option<usize>,
    pub status: Status,
    pub contract: Option<money::Contract>,
    /// Accrued seasons (six games on full pay status) and credited
    /// seasons (three), for free agency and the minimum.
    pub accrued: u32,
    pub credited: u32,
    /// Draft year, round, pick and team; None for an undrafted player.
    pub draft: Option<(i32, u32, u32, usize)>,
    pub first_year: i32,
    pub seasons: Vec<ProSeason>,
    pub earnings: f64,
    pub agent: Option<usize>,
    /// Game sharpness, 0-1 (topic 1.2): drops while he is out of football
    /// and comes back within three games.
    pub sharpness: f64,
    /// Weeks unsigned in a row (topic 1.3).
    pub weeks_unsigned: u32,
    /// Weeks left out injured.
    pub out_weeks: u32,
    /// Every team he has been with, in order.
    pub teams: Vec<usize>,
    /// His entry in `World::alumni` (drafted and undrafted players).
    pub alumnus: Option<usize>,
    /// Signed a big new deal this offseason (fix list 4.5).
    pub paid: bool,
    /// A new team this year (an adjustment in camp).
    pub new_team: bool,
    /// Seasons on a practice squad (the eligibility limit).
    pub ps_seasons: u32,
    /// Times franchise or transition tagged by his current team.
    pub tags: u32,
    /// How each league team's staff sees him, set when he is graded:
    /// none until his first camp.
    pub last_grade: Option<f64>,
    /// This summer's preseason: grade points weighted by snaps, and snaps.
    pub preseason: (f64, f64),
    /// His ideal package while he is on the market (hidden; topic 5).
    pub ideal: Option<market::Ideal>,
    /// A starting job promised in his deal: (team, league year).
    pub promise: Option<(usize, i32)>,
    /// His trust in his team, 0-1 (promises kept, shopping rumors).
    pub trust: f64,
    pub hold: market::Hold,
    /// Asked for a trade.
    pub wants_out: bool,
    /// A restricted free agent's tender until the offer sheet deadline.
    pub rfa: Option<money::Tender>,
    /// The team he last left (comp picks, waivers, loyalty).
    pub former: Option<usize>,
    /// Released and on waivers until the next claim time.
    pub on_waivers: bool,
    /// Came through the International Player Pathway.
    pub ipp: bool,
    /// Spring snaps abroad (an NFL Europe allocation), as a share of a season.
    pub spring_snaps: f64,
}

impl ProCareer {
    fn new(team: Option<usize>, status: Status, year: i32) -> ProCareer {
        ProCareer {
            team,
            status,
            contract: None,
            accrued: 0,
            credited: 0,
            draft: None,
            first_year: year,
            seasons: Vec::new(),
            earnings: 0.0,
            agent: None,
            sharpness: 1.0,
            weeks_unsigned: 0,
            out_weeks: 0,
            teams: team.into_iter().collect(),
            alumnus: None,
            paid: false,
            new_team: true,
            ps_seasons: 0,
            tags: 0,
            last_grade: None,
            preseason: (0.0, 0.0),
            ideal: None,
            promise: None,
            trust: 1.0,
            hold: market::Hold::None,
            wants_out: false,
            rfa: None,
            former: None,
            on_waivers: false,
            ipp: false,
            spring_snaps: 0.0,
        }
    }

    /// On a team's books: the roster, practice squad or reserve lists.
    pub fn held(&self) -> bool {
        self.team.is_some() && matches!(self.status, Status::Roster | Status::PracticeSquad | Status::Reserve)
    }

    pub fn season_mut(&mut self, year: i32) -> &mut ProSeason {
        if self.seasons.last().is_none_or(|s| s.year != year) {
            self.seasons.push(ProSeason { year, team: self.team, ..Default::default() });
        }
        self.seasons.last_mut().unwrap()
    }
}

/// A franchise's football side beyond the league table: who it holds, its
/// depth chart and its books.
#[derive(Clone, Debug, Default)]
pub struct Club {
    /// Everyone on its books: roster, practice squad and reserve lists.
    pub roster: Vec<usize>,
    /// The depth chart at each position, best first (the active roster).
    pub depth: Vec<Vec<usize>>,
    /// Dead money charged to each league year, charge by charge.
    pub dead: Vec<DeadCharge>,
    /// Unused cap carried into the next league year (2011 on).
    pub carryover: f64,
    /// The room left when a league year's regular season ended, with every
    /// charge of that year in: what carries over.
    pub unused: Option<(i32, f64)>,
    /// The systems its coordinators run (S3) and its own tape.
    pub installed: super::systems::Installed,
    pub film: crate::game::Film,
    /// Players brought back from injured reserve this season.
    pub ir_used: u32,
    /// Its general manager, owner and fans (topic 7).
    pub office: office::Office,
    /// Cash paid to players each league year (the spending floor).
    pub cash: Vec<(i32, f64)>,
}

/// Why a team carries dead money.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeadWhy {
    /// A cap casualty: released to clear room.
    CapCut,
    /// Any other release (cutdown, a roster move, a practice squad deal ending).
    Released,
    /// Guaranteed pay owed once a waived player clears waivers.
    Waived,
    /// A roster deal replaced by a practice squad deal at cutdown.
    PracticeSquad,
    Retired,
    /// The bonus left with the seller of a traded player.
    Traded,
    /// Bonus spread over void years when a deal ends.
    Voided,
    /// Incentives settled after the season (can be a credit).
    Incentives,
}

/// One dead-money charge, as real cap pages list them: the league year it
/// lands on, the amount and the player it is for.
#[derive(Clone, Copy, Debug)]
pub struct DeadCharge {
    pub year: i32,
    pub amount: f64,
    pub player: u32,
    pub why: DeadWhy,
}

impl Club {
    pub fn dead_in(&self, year: i32) -> f64 {
        self.dead.iter().filter(|d| d.year == year).map(|d| d.amount).sum()
    }
    fn add_dead(&mut self, year: i32, amount: f64, player: u32, why: DeadWhy) {
        if amount > 0.0 {
            self.dead.push(DeadCharge { year, amount, player, why });
        }
    }
}

// --- Rosters ------------------------------------------------------------------------

/// Rebuilds every club's list from the people (after the yearly compaction
/// moves indices) and the mirror into the league table.
pub(crate) fn rebuild(w: &mut World) {
    let n = w.nfl.len();
    w.clubs.resize_with(n, Club::default);
    for c in w.clubs.iter_mut() {
        c.roster.clear();
    }
    for (i, p) in w.people.iter().enumerate() {
        if p.stage != Stage::Pro {
            continue;
        }
        if let Some(pro) = p.pro.as_ref()
            && pro.held()
            && let Some(t) = pro.team
        {
            w.clubs[t].roster.push(i);
        }
    }
    for t in 0..n {
        order_depth(w, t);
    }
    mirror(w);
}

/// A team's view of a player (topic 7: each staff's own eyes): his overall
/// with a keyed misread that shrinks with the GM's judgment and the film
/// on him.
pub(crate) fn grade(w: &World, t: usize, i: usize) -> f64 {
    let p = &w.people[i];
    let ovr = p.player.overall();
    let seasons = p.pro.as_ref().map_or(0, |c| c.seasons.len()) as f64;
    let skill = w.nfl.get(t).map_or(10.0, |x| x.draft_skill);
    let sd = crate::career::draft_board_noise(skill) * 0.6 / (1.0 + 0.4 * seasons);
    ovr + sd * key_normal(w.options.seed as u64 ^ 0x6A0E ^ (t as u64) << 40, p.player.id as u64 ^ (w.year as u64) << 32)
}

/// Orders a team's active players at each position by its view of them:
/// last year's starter keeps the job unless clearly beaten, and staffs lean
/// from a fading veteran to the younger man (as the class model's camp).
fn order_depth(w: &mut World, t: usize) {
    let mut depth: Vec<Vec<(usize, f64)>> = vec![Vec::new(); POSITION_COUNT];
    for &i in &w.clubs[t].roster {
        let p = &w.people[i];
        let Some(pro) = p.pro.as_ref() else { continue };
        if pro.status != Status::Roster {
            continue;
        }
        let pos = p.player.position;
        let mut v = grade(w, t, i);
        let age = p.player.age as f64;
        if !pos.is_specialist() {
            v -= (age - 30.0).max(0.0) * 0.6;
        }
        if pro.seasons.last().is_some_and(|s| s.team == Some(t) && s.starts >= 8) && age < 32.0 {
            v += if pos == Position::QB { 3.0 } else { 1.5 };
        }
        if pro.out_weeks > 0 || pro.hold == market::Hold::Out {
            v -= 50.0;
        }
        // 3.12 the owner's pet keeps his job.
        if w.clubs[t].office.owner.pet == Some(i) {
            v += 4.0;
        }
        depth[pos.index()].push((i, v));
    }
    let club = &mut w.clubs[t];
    club.depth = depth
        .into_iter()
        .map(|mut v| {
            v.sort_by(|a, b| b.1.total_cmp(&a.1));
            v.into_iter().map(|x| x.0).collect()
        })
        .collect();
}

/// The league table's depth charts from the people: each active player
/// with his rating and age, the practice squad below them.
pub(crate) fn mirror(w: &mut World) {
    for t in 0..w.nfl.len().min(w.clubs.len()) {
        let mut lists: [Vec<Member>; POSITION_COUNT] = Default::default();
        for pos in Position::ALL {
            let k = pos.index();
            let slots = depth_slots(pos);
            for (slot, &i) in w.clubs[t].depth.get(k).map_or(&[][..], |v| v.as_slice()).iter().enumerate() {
                let p = &w.people[i].player;
                let role = if slot < slots.starters {
                    PlayingTime::Starter
                } else if slot < slots.starters + slots.rotation {
                    PlayingTime::Rotation
                } else {
                    PlayingTime::Backup
                };
                lists[k].push(Member {
                    rating: p.overall(),
                    age: p.age,
                    id: Some(i as u32),
                    role: Some(role),
                    depth: None,
                });
            }
        }
        for &i in &w.clubs[t].roster {
            let p = &w.people[i];
            if p.pro.as_ref().is_some_and(|c| c.status == Status::PracticeSquad) {
                let k = p.player.position.index();
                lists[k].push(Member {
                    rating: p.player.overall(),
                    age: p.player.age,
                    id: Some(i as u32),
                    role: Some(PlayingTime::PracticeSquad),
                    depth: None,
                });
            }
        }
        w.nfl[t].depth = lists;
    }
}

/// Players a team holds with a status.
fn count(w: &World, t: usize, status: Status) -> usize {
    w.clubs[t].roster.iter().filter(|&&i| w.people[i].pro.as_ref().is_some_and(|c| c.status == status)).count()
}

/// The roster limit today: the offseason limit until cutdown day, then the
/// regular-season roster through the Super Bowl.
pub(crate) fn roster_limit(w: &World, d: Date) -> usize {
    let y = &w.dates;
    if d >= y.cutdown.date || d < y.league_year.date {
        w.rules.nfl_roster as usize
    } else {
        w.rules.nfl_offseason as usize
    }
}

// --- Money on the books -----------------------------------------------------------------

/// A team's cap charges in a league year: every contract it holds and its dead money.
pub fn payroll(w: &World, t: usize, year: i32) -> f64 {
    // The offseason rule of 51 (real): until the first game only a team's
    // 51 biggest cap hits count. A stack buffer: this runs for every offer.
    let top51 = money::capped(year) && (year > w.year || (year == w.year && w.today < w.dates.nfl_kickoff));
    let mut buf = [0.0f64; 160];
    let mut n = 0;
    let mut all = 0.0;
    for &i in &w.clubs[t].roster {
        if let Some(c) = w.people[i].pro.as_ref().and_then(|c| c.contract.as_ref())
            && c.team == t
        {
            let hit = c.cap_hit(year);
            all += hit;
            if n < buf.len() {
                buf[n] = hit;
                n += 1;
            }
        }
    }
    let held = if top51 && n > TOP_51 {
        let hits = &mut buf[..n];
        hits.select_nth_unstable_by(TOP_51 - 1, |a, b| b.total_cmp(a));
        hits[..TOP_51].iter().sum()
    } else {
        all
    };
    held + w.clubs[t].dead_in(year)
}

/// The offseason rule of 51.
const TOP_51: usize = 51;

/// The cap space a team has in a league year (the carryover added). With
/// no cap, the owner's budget plays the part: about the league's average
/// payroll, more for a free-spending owner (proposed, not confirmed).
pub fn space(w: &World, t: usize, year: i32) -> f64 {
    let cap = money::price_cap(year);
    let room =
        if money::capped(year) { cap + w.clubs[t].carryover } else { cap * (0.95 + 0.02 * (w.nfl[t].owner - 10.0)) };
    // While only the top 51 count, a team keeps room for what counts from
    // the first game: the rest of the 53, the practice squad and injury
    // replacements (proposed, not confirmed: 5% of the cap).
    let reserve = if money::capped(year) && (year > w.year || (year == w.year && w.today < w.dates.nfl_kickoff)) {
        cap * IN_SEASON_RESERVE
    } else {
        0.0
    };
    room - reserve - payroll(w, t, year)
}

/// See `space`.
const IN_SEASON_RESERVE: f64 = 0.05;

// --- Joining the league ------------------------------------------------------------------

/// A generated veteran for the league at the start of history: a full
/// person (name, family, high school and college) with a player at the
/// rating and age the slot calls for.
fn veteran(w: &mut World, pos: Position, rating: f64, age: u32, seed: u32) -> Person {
    let mut rng = Rng::new(seed);
    let id = w.next_person;
    w.next_person += 1;
    // His home: a state by its share of talent, a high school there.
    let weights: Vec<(usize, f64)> = (0..w.states.len())
        .filter(|&s| !w.states[s].schools.is_empty())
        .map(|s| (s, w.states[s].class_share))
        .collect();
    let state = rng.weighted(&weights);
    let schools = &w.states[state].schools;
    let hs = schools[rng.int(0, schools.len() as i32 - 1) as usize];
    let z = rng.normal(2.6, 0.4);
    let mut player = generate_prospect(
        &mut rng,
        ProspectOptions { id: Some(id), position: Some(pos), age: Some(age), talent_z: Some(z) },
    );
    // He is at or near his peak: ceilings a little above today.
    let head = clamp(27.0 - age as f64, 0.0, 5.0) + rng.range(0.0, 1.5);
    let peak = rating + head;
    let pot = crate::positions::overall_for(pos, &player.ceilings).max(1.0);
    for a in crate::attributes::Attr::ALL {
        player.ceilings[a] = clamp(player.ceilings[a] * peak / pot, 20.0, 99.0);
    }
    for _ in 0..3 {
        let now = player.overall().max(1.0);
        for a in crate::attributes::Attr::ALL {
            player.attributes[a] = clamp(
                if now > 0.0 { player.attributes[a].max(1.0) * rating / now } else { rating },
                20.0,
                player.ceilings[a],
            );
        }
    }
    player.original_potential = crate::positions::overall_for(pos, &player.ceilings);
    let school = &w.high_schools[hs];
    let (mut bg, structure) = super::background::generate_background(
        super::background::BackgroundSeed {
            area_money: school.area_money,
            area_safety: school.area_safety,
            school_resources: school.resources,
        },
        &mut rng,
    );
    let family = super::background::generate_family(&bg, Some(structure), &mut rng);
    super::background::apply_background(&mut player, &mut bg, &family, &mut rng);
    let ethnicity = super::ethnicity::player_ethnicity(state, z, &mut rng);
    let year = w.year;
    // His college: an FBS program by its prestige, a few from below.
    let colleges: Vec<(usize, f64)> = w
        .programs
        .iter()
        .filter(|p| p.division <= super::data::Division::FCS)
        .map(|p| (p.id, if p.division == super::data::Division::FBS { p.prestige.max(0.05) } else { 0.05 }))
        .collect();
    let college = rng.weighted(&colleges);
    Person {
        name: super::names::person_name(ethnicity, year - age as i32, &mut rng),
        ethnicity,
        player,
        rng,
        bg,
        family,
        home_state: state,
        hs,
        stage: Stage::Pro,
        college: Some(college),
        class_of: year - age as i32 + 18,
        stars: 0,
        camp_stars: 0,
        visibility: 0.0,
        college_seasons: 4,
        redshirt_used: false,
        sit_out: 0.0,
        steered: false,
        nil_income: 0.0,
        pending_dip: None,
        last_role: None,
        last_snaps: 0,
        college_snaps: 0,
        last_performance: None,
        transfers: 0,
        juco: false,
        path: vec![college],
        on_notice: false,
        gpa_trend: 0.0,
        gpa_at_signing: None,
        academic_redshirt: false,
        walk_on: false,
        transfer_from: None,
        college_staff: Vec::new(),
        incident: false,
        notes: Vec::new(),
        stay_home: 0.0,
        childhood_team: None,
        cal: PersonCal::default(),
        pro: None,
    }
}

/// Fills the league at the start of history (topic 1): every team that
/// plays gets a full offseason roster of generated veterans, spread like a
/// real roster by depth slot (the class model's background league), each
/// on a contract with one to four years left, and a pool of unsigned
/// veterans waits on the street.
pub(crate) fn fill_league(w: &mut World) {
    let year = w.year;
    let n = w.nfl.len();
    w.clubs = vec![Club::default(); n];
    let mut rng = Rng::new(w.options.seed ^ 0x5E7E_4A45);
    let mut made: Vec<(Person, Option<usize>, f64)> = Vec::new();
    for t in 0..n {
        if !super::franchise::active(t, year) {
            continue;
        }
        let talent = crate::league::talent_of(&w.nfl[t], &mut rng);
        for pos in Position::ALL {
            let slots = depth_slots(pos);
            // The 53's slots, then camp bodies to the offseason roster (about
            // a third more, proposed, not confirmed).
            let extra = (slots.roster as f64 * 0.45).round() as usize;
            for slot in 0..slots.roster + extra {
                let s = slot.min(slots.roster - 1);
                let m = crate::league::new_background(
                    &mut rng,
                    pos,
                    s,
                    talent - if slot >= slots.roster { 1.0 } else { 0.0 },
                    slot >= slots.roster,
                );
                let seed = rng.next_u32();
                let p = veteran(w, pos, m.rating, m.age.clamp(21, 38), seed);
                made.push((p, Some(t), m.rating));
            }
        }
    }
    // The street: veterans between jobs (proposed, not confirmed: about
    // five per team).
    for _ in 0..(5 * n) {
        let pos = Position::ALL[rng.int(0, Position::ALL.len() as i32 - 1) as usize];
        let rating = clamp(rng.normal(66.0, 4.0), 50.0, 78.0);
        let age = rng.int(24, 33) as u32;
        let seed = rng.next_u32();
        let p = veteran(w, pos, rating, age, seed);
        made.push((p, None, rating));
    }
    // Each team's veteran deals fit under its cap the way real payrolls do
    // (proposed, not confirmed: 86-96% of the cap in the top 51).
    let cap = money::price_cap(year);
    let mut asks: Vec<f64> = made
        .iter()
        .map(|(p, team, rating)| match team {
            Some(_) => (cap * money::market_share(p.player.position, *rating))
                .max(money::minimum(year, p.player.age.saturating_sub(22))),
            None => 0.0,
        })
        .collect();
    for t in 0..n {
        let mut mine: Vec<usize> = (0..made.len()).filter(|&k| made[k].1 == Some(t)).collect();
        if mine.is_empty() {
            continue;
        }
        mine.sort_by(|&a, &b| asks[b].total_cmp(&asks[a]));
        let top: f64 = mine.iter().take(TOP_51).map(|&k| asks[k]).sum();
        let target = cap * (0.86 + 0.10 * key_uniform(w.options.seed as u64 ^ 0xCA9F, t as u64));
        if top > target {
            let f = target / top;
            for &k in &mine {
                let floor = money::minimum(year, made[k].0.player.age.saturating_sub(22));
                asks[k] = (asks[k] * f).max(floor);
            }
        }
    }
    for ((mut p, team, rating), apy) in made.into_iter().zip(asks) {
        let age = p.player.age;
        let accrued = age.saturating_sub(22);
        let mut c = ProCareer::new(
            team,
            if team.is_some() { Status::Roster } else { Status::FreeAgent },
            year - accrued as i32,
        );
        c.accrued = accrued;
        c.credited = accrued;
        c.new_team = false;
        c.sharpness = if team.is_some() { 1.0 } else { 0.8 };
        if let Some(t) = team {
            let grade = rating;
            let years_left = 1 + (key_uniform(w.options.seed as u64 ^ 0x7E4A, p.player.id as u64) * 4.0) as u32;
            let mut terms = money::Terms::flat(years_left, apy);
            terms.bonus_share = if grade >= 76.0 { 0.25 } else { 0.05 };
            c.contract = Some(money::build(t, money::Kind::Veteran, year, &terms));
        }
        p.note(format!("A pro since {} (generated veteran at the start of history).", year - accrued as i32));
        p.pro = Some(Box::new(c));
        w.people.push(p);
    }
    for t in 0..n {
        let tm = &w.nfl[t];
        let inst = super::systems::nfl_install(w, Default::default(), tm.oc.or(tm.head_coach), tm.dc.or(tm.head_coach));
        w.clubs[t].installed = inst;
    }
    office::staff_offices(w);
    market::staff_agents(w);
    rebuild(w);
    market::set_ladders(w);
    for i in 0..w.people.len() {
        if w.people[i].stage == Stage::Pro {
            market::sign_agent(w, i);
        }
    }
}

/// A drafted player joins his team on his rookie deal.
pub(crate) fn join_drafted(w: &mut World, i: usize, t: usize, year: i32, pick: u32, round: u32) {
    let terms = money::rookie_terms(year, pick, round);
    let mut c = ProCareer::new(Some(t), Status::Roster, year);
    c.draft = Some((year, round, pick, t));
    c.contract = Some(money::build(t, money::Kind::Rookie, year, &terms));
    c.alumnus = w.recording.then(|| w.alumni.len().saturating_sub(1));
    w.people[i].pro = Some(Box::new(c));
    w.clubs[t].roster.push(i);
    market::sign_agent(w, i);
    // 3.12 a meddling owner's pet: his first-round quarterback.
    let owner = &w.clubs[t].office.owner;
    if round == 1 && w.people[i].player.position == Position::QB && owner.meddling > 0.6 && owner.pet.is_none() {
        w.clubs[t].office.owner.pet = Some(i);
    }
}

/// An undrafted player's team in the rush: the team where he has the
/// clearest path to the 53 (its depth at his position against his grade),
/// with some luck in who called first.
pub(crate) fn undrafted_team(w: &World, i: usize) -> usize {
    let p = &w.people[i];
    let pos = p.player.position;
    let limit = w.rules.nfl_offseason as usize;
    let all: Vec<usize> = (0..w.nfl.len()).filter(|&t| super::franchise::active(t, w.year)).collect();
    let open: Vec<usize> =
        all.iter().copied().filter(|&t| w.clubs.get(t).is_none_or(|c| c.roster.len() < limit)).collect();
    let teams = if open.is_empty() { all } else { open };
    let mut best = (teams.first().copied().unwrap_or(0), f64::MIN);
    for &t in &teams {
        let g = grade(w, t, i);
        let depth = w.nfl[t].depth[pos.index()].iter().filter(|m| m.rating >= g).count() as f64;
        let room = depth_slots(pos).roster as f64 - depth;
        let v = room + 1.5 * key_normal(w.options.seed as u64 ^ 0x0DFA, (t as u64) << 32 ^ p.player.id as u64);
        if v > best.1 {
            best = (t, v);
        }
    }
    best.0
}

/// An undrafted player signs with a team in the rush.
pub(crate) fn join_undrafted(w: &mut World, i: usize, t: usize, year: i32, wanted: f64) {
    let terms = money::undrafted_terms(year, wanted);
    let mut c = ProCareer::new(Some(t), Status::Roster, year);
    c.contract = Some(money::build(t, money::Kind::Undrafted, year, &terms));
    c.alumnus = w.recording.then(|| w.alumni.len().saturating_sub(1));
    w.people[i].pro = Some(Box::new(c));
    w.clubs[t].roster.push(i);
    market::sign_agent(w, i);
    // The next team in the rush sees him on its board.
    let p = &w.people[i].player;
    let m = Member { rating: p.overall(), age: p.age, id: Some(i as u32), role: None, depth: None };
    w.nfl[t].depth[p.position.index()].push(m);
}

/// After the draft and the rush: depth charts from the people.
pub(crate) fn after_draft(w: &mut World) {
    for t in 0..w.clubs.len() {
        order_depth(w, t);
    }
    mirror(w);
}

/// A player moves to another team with his contract (an expansion draft
/// pick, a franchise move).
pub(crate) fn transfer(w: &mut World, i: usize, to: usize, why: &str) {
    let Some(from) = w.people[i].pro.as_ref().and_then(|c| c.team) else { return };
    w.clubs[from].roster.retain(|&x| x != i);
    w.clubs[to].roster.push(i);
    let name = w.nfl[to].name;
    let p = &mut w.people[i];
    let c = p.pro.as_mut().unwrap();
    c.team = Some(to);
    if let Some(k) = c.contract.as_mut() {
        k.team = to;
    }
    c.teams.push(to);
    c.new_team = true;
    let age = p.row_age();
    p.note(format!("Moved to the {name} at {age} ({why})."));
}

/// A franchise moves (the Browns to Baltimore in 1996): its players,
/// contracts and books go with it.
pub(crate) fn move_club(w: &mut World, from: usize, to: usize, why: &str) {
    if w.clubs.len() <= from.max(to) {
        return;
    }
    for i in w.clubs[from].roster.clone() {
        transfer(w, i, to, why);
    }
    let dead = std::mem::take(&mut w.clubs[from].dead);
    w.clubs[to].dead.extend(dead);
    w.clubs[to].carryover += std::mem::take(&mut w.clubs[from].carryover);
    w.clubs[to].unused = w.clubs[from].unused.take().or(w.clubs[to].unused);
}

/// Fills a team's holes so it can field a team: at each position it signs
/// the best unsigned player there for the minimum, or, when the street has
/// nobody, a player from another league (the CFL, the Arena League, NFL
/// Europe; a generated person, as the veterans at the start of history).
/// `full` fills to the 53's depth (camp), otherwise to the starters and
/// rotation (after cutdown).
fn fill_holes(w: &mut World, d: Date, full: bool) {
    let year = w.year;
    for t in 0..w.clubs.len() {
        if !super::franchise::active(t, year) {
            continue;
        }
        for pos in Position::ALL {
            let s = depth_slots(pos);
            let need = if full { s.roster } else { s.starters + s.rotation };
            loop {
                let have = w.clubs[t]
                    .roster
                    .iter()
                    .filter(|&&i| {
                        let p = &w.people[i];
                        p.player.position == pos
                            && p.pro.as_ref().is_some_and(|c| c.status == Status::Roster && c.out_weeks == 0)
                    })
                    .count();
                if have >= need {
                    break;
                }
                let street = (0..w.people.len())
                    .filter(|&i| {
                        let p = &w.people[i];
                        p.stage == Stage::Pro
                            && p.player.position == pos
                            && p.pro.as_ref().is_some_and(|c| c.status == Status::FreeAgent && !c.on_waivers)
                            && !w.market.pending.iter().any(|x| x.person == i)
                    })
                    .max_by(|&a, &b| w.people[a].player.overall().total_cmp(&w.people[b].player.overall()));
                let i = match street {
                    Some(i) => i,
                    None => {
                        let mut rng = w.rng.fork();
                        let rating = clamp(rng.normal(61.0, 3.0), 52.0, 68.0);
                        let age = rng.int(23, 28) as u32;
                        let seed = rng.next_u32();
                        let mut p = veteran(w, pos, rating, age, seed);
                        let mut c = ProCareer::new(None, Status::FreeAgent, year);
                        c.sharpness = 0.9;
                        p.note(format!("Came to the NFL from another league at {age} (generated player)."));
                        p.pro = Some(Box::new(c));
                        w.people.push(p);
                        w.people.len() - 1
                    }
                };
                let accrued = w.people[i].pro.as_ref().map_or(0, |c| c.accrued);
                sign(w, i, t, d, money::minimum(year, accrued), 1, money::Kind::Minimum);
            }
        }
    }
}

// --- The NFL year for the people ---------------------------------------------------------

/// The pro side of the NFL day (the team management side of the NFL
/// year, `docs/design/team-management.md`): re-signing talks and tags
/// before the league year, cap moves, the tampering window and the
/// market, comp picks, offer sheets, fifth-year options, trades in their
/// windows, extension talks, holdouts, camp, cutdown and waivers, the
/// weekly street in season, promises checked in week 1, the other leagues,
/// futures deals and the season's end.
pub(crate) fn day(w: &mut World, d: Date) {
    if w.clubs.is_empty() {
        return;
    }
    if w.market.agents.is_empty() {
        market::staff_agents(w);
    }
    let y = w.dates.clone();
    let mut changed = false;
    super::clock::timed("pro leagues", || leagues::day(w, d));
    if d == y.tag_window_open {
        // The year's pay ladders: the veterans by position after last season.
        market::set_ladders(w);
        // Cap cuts and restructures start right after the season, so teams
        // can talk with their own free agents (real: February into March).
        set_carryover(w);
        market::cap_moves(w, d, true);
        market::resign_talks(w, d, false);
        changed = true;
    }
    if d == y.tag_deadline.date {
        market::resign_talks(w, d, true);
        market::tags_and_tenders(w, d);
        changed = true;
    }
    if d == y.league_year.date.add(-1) {
        set_carryover(w);
        market::cap_moves(w, d, true);
        changed = true;
    }
    if d == y.league_year.date {
        league_year(w, d);
        changed = true;
    }
    // The market: every day from the tampering window to two weeks into
    // the league year, then twice a week until camp.
    let opened = y.tampering_open.map_or(y.league_year.date, |m| m.date);
    if d >= opened && d < y.veterans_report {
        let days = y.league_year.date.days_to(d);
        if days <= 14 || matches!(d.weekday(), crate::calendar::Weekday::Tue | crate::calendar::Weekday::Fri) {
            super::clock::timed("pro market", || market::market_day(w, d));
            super::clock::timed("pro sheets", || market::offer_sheets(w, d));
            changed = true;
        }
    }
    if d == y.comp_picks {
        trades::award_comp_picks(w, d);
    }
    if d == y.fifth_year_option {
        market::fifth_year_options(w, d);
    }
    if d == y.league_year.date.add(3)
        || d == y.draft[0].date.add(-2)
        || d == y.veterans_report.add(7)
        || d == y.cutdown.date.add(-2)
    {
        trades::trade_window(w, d, false);
        changed = true;
    }
    if d == y.trade_deadline.date.add(-4) || d == y.trade_deadline.date {
        trades::trade_window(w, d, true);
        changed = true;
    }
    if d == y.june_1
        || d == y.tag_long_term_deadline.date
        || d == y.veterans_report.add(-1)
        || d == y.nfl_kickoff.add(-3)
    {
        market::extension_talks(w, d, d != y.june_1);
        changed = true;
    }
    if d > y.league_year.date
        && d < y.veterans_report
        && (d.days_to(y.league_year.date) % 7 == 0 || d.weekday() == crate::calendar::Weekday::Tue)
    {
        changed |= super::clock::timed("pro street", || street_signings(w, d, false));
    }
    if d == y.veterans_report {
        market::holdouts(w, d, true);
        fill_holes(w, d, true);
        camp(w, d);
        changed = true;
    } else if d > y.veterans_report && d <= y.super_bowl && d.weekday() == crate::calendar::Weekday::Tue {
        market::holdouts(w, d, false);
    }
    let weeks = preseason_dates(w);
    if let Some(k) = weeks.iter().position(|&x| x == d) {
        preseason_week(w, d, k, weeks.len());
    }
    if d == y.cutdown.date {
        cutdown(w, d);
        fill_holes(w, d, false);
        changed = true;
    }
    // Waiver claims each day a player is on the wire.
    super::clock::timed("pro claims", || market::claims(w, d));
    if d == y.nfl_kickoff {
        // From the first game every contract counts against the cap.
        market::cap_moves(w, d, false);
        market::check_promises(w);
        // Cap use on opening day: each active team's payroll over its cap.
        let year = w.year;
        let cap = money::price_cap(year);
        let teams = super::franchise::active_teams(year);
        let spread = |mut v: Vec<f64>| {
            v.sort_by(|a, b| a.total_cmp(b));
            if v.is_empty() { [0.0; 3] } else { [v[0], v[v.len() / 2], v[v.len() - 1]] }
        };
        let carry = |t: usize| if money::capped(year) { w.clubs[t].carryover } else { 0.0 };
        let used = spread(teams.iter().map(|&t| payroll(w, t, year) / (cap + carry(t))).collect());
        let dead = spread(teams.iter().map(|&t| w.clubs[t].dead_in(year) / cap).collect());
        let carried = spread(teams.iter().map(|&t| carry(t) / cap).collect());
        let m = w.market.year_mut(year);
        m.cap_use = used;
        m.dead = dead;
        m.carryover = carried;
    }
    // In season, each Tuesday: injured players to reserve, the street's
    // calls, the unsigned veteran's week (topic 1.3).
    if d >= y.cutdown.date && d.weekday() == crate::calendar::Weekday::Tue && d <= y.super_bowl {
        super::clock::timed("pro heal", || heal_week(w));
        changed |= super::clock::timed("pro street", || street_signings(w, d, true));
        super::clock::timed("pro unsigned", || unsigned_week(w, d));
    }
    // The Hall of Fame class is named the Saturday before the Super Bowl.
    if d == y.super_bowl.add(-1) {
        honors::hall_vote(w, d);
    }
    if d == y.super_bowl {
        // Futures deals for the playoff teams, the spending floor, the season's end.
        let teams: Vec<usize> =
            w.nfl_season.as_ref().map_or(Vec::new(), |s| (0..s.seed.len()).filter(|&t| s.seed[t].is_some()).collect());
        market::futures(w, d, &teams);
        market::cash_floor(w, d);
        season_end(w, d);
        changed = true;
    }
    if changed {
        super::clock::timed("pro depth", || {
            for t in 0..w.nfl.len() {
                order_depth(w, t);
            }
            mirror(w);
        });
    }
}

/// The new league year (March): expiring deals end and their players hit
/// the market (`market::open_league_year`), the staffs install their
/// systems, last year's tape fades and each front office picks its course.
fn league_year(w: &mut World, d: Date) {
    // The owner's pet (3.12) is only his while he is on the team.
    for t in 0..w.clubs.len() {
        if let Some(i) = w.clubs[t].office.owner.pet
            && w.people.get(i).and_then(|p| p.pro.as_ref()).is_none_or(|c| c.team != Some(t))
        {
            w.clubs[t].office.owner.pet = None;
        }
    }
    market::open_league_year(w, d);
    // The staffs install their systems for the year (S3), and last year's
    // tape fades (a new head coach's team shows little of the old one).
    for t in 0..w.clubs.len() {
        let tm = &w.nfl[t];
        let inst =
            super::systems::nfl_install(w, w.clubs[t].installed, tm.oc.or(tm.head_coach), tm.dc.or(tm.head_coach));
        let new_coach = tm.staff_years == 0;
        let c = &mut w.clubs[t];
        c.installed = inst;
        c.film.fade(if new_coach { 0.1 } else { 0.3 });
        c.ir_used = 0;
    }
    office::choose_modes(w);
    for i in 0..w.people.len() {
        if w.people[i].stage == Stage::Pro {
            market::sign_agent(w, i);
        }
    }
    for t in 0..w.clubs.len() {
        order_depth(w, t);
    }
    mirror(w);
}

/// Releases a player: he goes to the street; his remaining bonus and
/// guaranteed pay land on the team's cap as dead money (split over two
/// years after June 1).
pub(crate) fn release(w: &mut World, i: usize, d: Date, why: &str) {
    release_with(w, i, d, why, false, true, DeadWhy::Released);
}

/// From June 1 to the Super Bowl a release splits its dead money over
/// two league years by itself (real).
pub(crate) fn after_june1(w: &World, d: Date) -> bool {
    d >= w.dates.june_1 && d < w.dates.super_bowl
}

/// A release with its terms: `june1` splits the dead money (a June 1
/// release or a post-June 1 designation), `waivers` sends a player who
/// qualifies to the waiver wire, where a claiming team takes over his
/// contract (his old team then owes only the bonus already paid).
pub(crate) fn release_with(w: &mut World, i: usize, d: Date, why: &str, june1: bool, waivers: bool, cause: DeadWhy) {
    let year = w.year;
    let june1 = june1 || after_june1(w, d);
    let waive = waivers && market::waivable(w, i, d);
    let Some(c) = w.people[i].pro.as_mut() else { return };
    let Some(t) = c.team else { return };
    let mut dead = (0.0, 0.0);
    if let Some(k) = c.contract.as_ref() {
        dead = k.dead_if_cut(year, d, june1);
        if waive {
            dead.0 -= k.guaranteed_left(year, d);
        }
    }
    if waive {
        c.on_waivers = true;
        if let Some(k) = c.contract.as_mut() {
            for y in k.years.iter_mut().filter(|y| y.year >= year) {
                y.proration = 0.0;
            }
        }
    } else {
        c.contract = None;
    }
    c.status = Status::FreeAgent;
    c.team = None;
    c.weeks_unsigned = 0;
    c.former = Some(t);
    c.rfa = None;
    c.hold = market::Hold::None;
    c.wants_out = false;
    c.ideal = None;
    w.clubs[t].roster.retain(|&x| x != i);
    let id = w.people[i].player.id;
    w.clubs[t].add_dead(year, dead.0.max(0.0), id, cause);
    w.clubs[t].add_dead(year + 1, dead.1, id, cause);
    let name = w.nfl[t].name;
    let p = &mut w.people[i];
    let age = p.row_age();
    p.note(format!(
        "Released by the {name} at {age} ({why}): rated {}{}.",
        round_half_up(p.player.overall()),
        if waive { ", placed on waivers" } else { "" }
    ));
}

/// Carryover (2011 on, real): a team declares last league year's unused
/// room and it comes with the team. The room is the one left when the
/// season ended: a player released or retired since was paid for the year,
/// so his charge stays on that year's books.
fn set_carryover(w: &mut World) {
    let year = w.year;
    let prev = year - 1;
    for t in 0..w.clubs.len() {
        w.clubs[t].carryover = match w.clubs[t].unused {
            Some((y, room)) if year >= 2011 && y == prev && money::capped(prev) => room,
            _ => 0.0,
        };
    }
}

/// What a free agent asks for a year and for how long: his market price
/// by grade and age (the league's consensus grade, his overall), at least
/// the minimum for his seasons.
/// The ask comes down the longer nobody calls: about 4% a week from the
/// opening of the league year, and in season for each week unsigned
/// (proposed, not confirmed until topic 5's market replaces it).
fn ask(w: &World, i: usize, year: i32, d: Date) -> (f64, u32) {
    let p = &w.people[i];
    let c = p.pro.as_ref();
    let accrued = c.map_or(0, |c| c.accrued);
    let g = p.player.overall();
    let weeks = (w.dates.league_year.date.days_to(d).max(0) / 7) as u32 + c.map_or(0, |c| c.weeks_unsigned);
    let min = money::minimum(year, accrued);
    let rel = market::market_grade(w, p.player.position, g);
    let price =
        money::price_cap(year) * money::market_share(p.player.position, rel) * (1.0 - 0.04 * weeks as f64).max(0.0);
    (price.max(min), money::usual_years(p.player.position, g, p.player.age))
}

/// Free agents find teams: each team with an opening at a position (fewer
/// players than the depth it carries, or a starter worse than him) and the
/// room to pay his price bids, and he signs with the best offer. In
/// season, only injuries open places. Proposed, not confirmed until the
/// full market (topic 5) replaces it.
fn street_signings(w: &mut World, d: Date, in_season: bool) -> bool {
    let year = w.year;
    let limit = roster_limit(w, d);
    // Before camp the open market (`market::market_day`) handles players
    // priced above the street.
    let market = d < w.dates.veterans_report;
    let mut pool: Vec<usize> = (0..w.people.len())
        .filter(|&i| {
            let p = &w.people[i];
            p.stage == Stage::Pro
                && p.pro.as_ref().is_some_and(|c| c.status == Status::FreeAgent && !c.on_waivers)
                && !(market && market::consensus(w, i) >= 66.0)
                && !w.market.pending.iter().any(|x| x.person == i)
        })
        .collect();
    pool.sort_by(|&a, &b| w.people[b].player.overall().total_cmp(&w.people[a].player.overall()));
    let mut changed = false;
    for i in pool {
        let pos = w.people[i].player.position;
        let (price, years) = ask(w, i, year, d);
        let mut best: Option<(usize, f64)> = None;
        for t in 0..w.clubs.len() {
            if !super::franchise::active(t, year) || w.clubs[t].roster.len() >= limit + if in_season { 20 } else { 0 } {
                continue;
            }
            let active = count(w, t, Status::Roster);
            if active >= limit {
                continue;
            }
            let g = grade(w, t, i);
            let slots = depth_slots(pos);
            let healthy: Vec<f64> = w.clubs[t].depth[pos.index()]
                .iter()
                .filter(|&&j| w.people[j].pro.as_ref().is_some_and(|c| c.out_weeks == 0))
                .map(|&j| grade(w, t, j))
                .collect();
            let need = if in_season {
                healthy.len() < slots.starters + slots.rotation
            } else {
                healthy.len() < slots.roster + slots.roster / 3
                    || healthy.get(slots.starters.saturating_sub(1)).is_some_and(|&x| x < g - 1.0)
            };
            if !need || space(w, t, year) < price || g < 58.0 {
                continue;
            }
            // He goes where he plays most: the starter he would beat, or a
            // team short of bodies at his spot.
            let worst = healthy.iter().take(slots.starters).copied().fold(f64::MAX, f64::min);
            let gain = if healthy.len() < slots.starters { 10.0 } else { g - worst };
            let v = gain + 0.5 * key_normal(w.options.seed as u64 ^ 0xF4EE ^ d.0 as u64, (t as u64) << 32 ^ i as u64);
            if best.is_none_or(|b| v > b.1) {
                best = Some((t, v));
            }
        }
        let Some((t, _)) = best else { continue };
        sign(w, i, t, d, price, years, money::Kind::Veteran);
        changed = true;
    }
    changed
}

/// A free agent signs with a team.
pub(crate) fn sign(w: &mut World, i: usize, t: usize, d: Date, apy: f64, years: u32, kind: money::Kind) {
    let year = w.year;
    let mut terms = money::Terms::flat(years.max(1), apy);
    terms.bonus_share = if apy > money::minimum(year, 4) * 3.0 { 0.3 } else { 0.0 };
    let deal = money::build(t, kind, year, &terms);
    let name = w.nfl[t].name;
    let p = &mut w.people[i];
    let c = p.pro.as_mut().unwrap();
    let new = c.teams.last() != Some(&t);
    c.team = Some(t);
    c.status = Status::Roster;
    c.contract = Some(deal);
    c.new_team |= new;
    c.paid = apy > money::price_cap(year) * 0.03;
    c.weeks_unsigned = 0;
    if new {
        c.teams.push(t);
    }
    let age = p.row_age();
    let when = if d >= w.dates.cutdown.date { " in season" } else { "" };
    p.note(format!(
        "Signed with the {name}{when} at {age}: {} year{} at ${:.2}M a year.",
        years.max(1),
        if years.max(1) == 1 { "" } else { "s" },
        apy / 1e6
    ));
    w.clubs[t].roster.push(i);
}

/// Camp (the yearly checkpoint, as college fall camp): every pro develops
/// through the year (`development.rs`, NFL level) with his role from last
/// season, his staff's teaching and his surroundings; rookies and players
/// on a new team adjust.
fn camp(w: &mut World, d: Date) {
    let year = w.year;
    let league_strength = {
        let teams: Vec<f64> =
            (0..w.nfl.len()).filter(|&t| super::franchise::active(t, year)).map(|t| w.nfl[t].strength()).collect();
        teams.iter().sum::<f64>() / teams.len().max(1) as f64
    };
    let pros: Vec<usize> = (0..w.people.len()).filter(|&i| w.people[i].stage == Stage::Pro).collect();
    for i in pros {
        let (team, status, first, new_team, payday) = {
            let c = w.people[i].pro.as_ref().unwrap();
            (c.team, c.status, c.seasons.is_empty(), c.new_team, c.paid)
        };
        let role = match (team, status) {
            (None, _) | (_, Status::FreeAgent) | (_, Status::Other(_)) => PlayingTime::Unsigned,
            (_, Status::PracticeSquad) => PlayingTime::PracticeSquad,
            (Some(t), _) => {
                let pos = w.people[i].player.position;
                let slot = w.clubs[t].depth[pos.index()].iter().position(|&j| j == i).unwrap_or(usize::MAX);
                let s = depth_slots(pos);
                if slot < s.starters {
                    PlayingTime::Starter
                } else if slot < s.starters + s.rotation {
                    PlayingTime::Rotation
                } else {
                    PlayingTime::Backup
                }
            }
        };
        let (coaching, situation) = match team {
            Some(t) => {
                let tm = &w.nfl[t];
                let cast = (tm.strength() - league_strength) / 1.8;
                let org = (tm.organisation() - 11.5) / 2.5;
                let reliance = match w.people[i].player.position {
                    Position::QB => 1.0,
                    Position::WR | Position::TE | Position::RB => 0.7,
                    _ => 0.4,
                };
                (tm.coaching, clamp(reliance * (0.5 * cast + 0.5 * org), -1.0, 1.0))
            }
            None => (6.0, 0.0),
        };
        let p = &mut w.people[i];
        let mut adjustment = 0.0;
        if team.is_some() && (first || new_team) && role != PlayingTime::Unsigned {
            let step = if first {
                match p.college.map(|c| w.programs[c].division) {
                    Some(super::data::Division::FCS) => crate::career::Step::FcsToNfl,
                    Some(super::data::Division::FBS) | None => crate::career::Step::FbsToNfl,
                    _ => crate::career::Step::SmallCollegeToNfl,
                }
            } else {
                crate::career::Step::NewTeam
            };
            adjustment = crate::career::first_year_adjustment(&p.player, step, &mut p.rng).dip;
        }
        // Last season's real snaps, weighted by how strong the teams he
        // took them against were (topic 1.1; proposed, not confirmed: a
        // standard deviation of opponent strength is worth 10%). A season
        // on the stand-in has none: his role decides.
        let snaps = p.pro.as_ref().and_then(|c| c.seasons.iter().find(|s| s.year == year - 1)).and_then(|s| {
            let n = (s.snaps[0] + s.snaps[1]) as f64;
            (s.stats.graded > 0 || n > 0.0).then(|| {
                let hard = if n > 0.0 { s.opp / n } else { 0.0 };
                n / games::NFL_SEASON_SNAPS as f64 * clamp(1.0 + 0.1 * hard, 0.8, 1.2)
            })
        });
        let spring = p.pro.as_ref().map_or(0.0, |c| c.spring_snaps);
        let snaps = if spring > 0.0 { Some(snaps.unwrap_or(0.0) + spring) } else { snaps };
        // A holdout misses camp (proposed, not confirmed: a small dip).
        if p.pro.as_ref().is_some_and(|c| c.hold == market::Hold::Out) {
            adjustment -= 0.3;
        }
        let ctx = crate::development::SeasonContext {
            adjustment,
            situation,
            payday,
            snaps,
            ..crate::development::SeasonContext::new(Level::NFL, clamp(coaching, 1.0, 20.0), role)
        };
        crate::development::develop_season(&mut p.player, &ctx, &mut p.rng);
        if let Some(row) = p.player.history.last_mut() {
            row.team = team;
            row.depth = Some(role.name().to_string());
        }
        let c = p.pro.as_mut().unwrap();
        c.paid = false;
        c.new_team = false;
        c.spring_snaps = 0.0;
        c.preseason = (0.0, 0.0);
        c.season_mut(year).team = team;
    }
    let _ = d;
}

/// The preseason's game days by era: four weeks before 2021 (the Hall of
/// Fame game aside), none in 2020, three from 2021.
fn preseason_dates(w: &World) -> Vec<Date> {
    let y = &w.dates;
    match w.year {
        2020 => Vec::new(),
        ..=2019 => vec![y.preseason[0].add(-7), y.preseason[0], y.preseason[1], y.preseason[2]],
        _ => y.preseason.to_vec(),
    }
}

/// A preseason week on the engine: teams paired by a keyed draw, the
/// backups and rookies playing (see `games::preseason_side`).
fn preseason_week(w: &mut World, d: Date, week: usize, weeks: usize) {
    let year = w.year;
    let on = w.options.engine_from.is_none_or(|from| year >= from);
    if !on {
        return;
    }
    let seed = w.options.seed as u64 ^ (year as u64) << 20 ^ (week as u64) << 8 ^ 0x9E;
    let mut teams: Vec<usize> = (0..w.clubs.len()).filter(|&t| super::franchise::active(t, year)).collect();
    teams.sort_by(|&a, &b| key_uniform(seed, a as u64).total_cmp(&key_uniform(seed, b as u64)));
    let pairs: Vec<(usize, usize)> = teams.chunks(2).filter(|c| c.len() == 2).map(|c| (c[0], c[1])).collect();
    let sides: Vec<Option<(games::Side, games::Side)>> = pairs
        .iter()
        .map(|&(h, a)| Some((games::preseason_side(w, h, week, weeks)?, games::preseason_side(w, a, week, weeks)?)))
        .collect();
    use rayon::prelude::*;
    let results: Vec<Option<(crate::game::GameResult, [games::Side; 2])>> = sides
        .into_par_iter()
        .zip(pairs.par_iter())
        .map(|(x, &(h, a))| {
            let (sa, sb) = x?;
            let v = games::venue(w, h, a, crate::calendar::Moment::at(d, 19, 0), false, -0.25);
            let r = crate::game::quick_game_in(
                &sa.team,
                &sb.team,
                Level::NFL,
                seed ^ (h as u64) << 8 ^ a as u64,
                Default::default(),
                v,
            );
            Some((r, [sa, sb]))
        })
        .collect();
    for (r, sides) in results.into_iter().flatten() {
        games::record_preseason(w, &r, &sides);
    }
    w.log.nfl_preseason_games += pairs.len() as u32;
}

/// Cutdown day (the class model's camp battle, on people): everyone at a
/// position is ranked on what the staff sees (its read of him, rust and a
/// first camp at the level), a rookie deal buys early picks patience
/// (`PIPELINE.rookie_protection`, for three years), last year's starter
/// keeps an edge, teams lean young and a veteran backup costs more than a
/// rookie who plays as well. Each position keeps its share of the era's
/// roster limit, the best of the rest fill it; young players close to the
/// line go to the practice squad and the rest are released.
fn cutdown(w: &mut World, d: Date) {
    let year = w.year;
    let limit = w.rules.nfl_roster as usize;
    let ps = w.rules.nfl_practice_squad as usize;
    for t in 0..w.clubs.len() {
        if !super::franchise::active(t, year) {
            continue;
        }
        let mut keep: Vec<(usize, f64)> = Vec::new();
        for &i in &w.clubs[t].roster {
            let p = &w.people[i];
            let c = p.pro.as_ref().unwrap();
            if c.status == Status::Reserve {
                continue;
            }
            let pos = p.player.position;
            let seasons = c.seasons.iter().filter(|s| s.games > 0).count() as u32;
            let first = c.seasons.iter().all(|s| s.year == year);
            let adjusting = if first {
                -4.0
            } else if c.new_team {
                -1.0
            } else {
                0.0
            };
            let camp = 0.2 * adjusting + (p.player.personality.competitiveness - 10.0) * 0.03;
            // The preseason tape (proposed, not confirmed: a grade 10 points
            // above average over a full game's snaps is worth 1.5 points).
            let (g, n) = c.preseason;
            let tape = if n > 0.0 { clamp(0.15 * (g / n - 60.0) * (n / 60.0).min(1.0), -3.0, 3.0) } else { 0.0 };
            let seen = grade(w, t, i) + camp + tape - 3.0 * (1.0 - c.sharpness);
            let round = c.draft.map_or(9, |x| x.1);
            let protection = if seasons + c.ps_seasons < 3 && round <= 8 {
                crate::career::PIPELINE.rookie_protection[round as usize]
            } else {
                0.0
            };
            let starter = c.seasons.iter().any(|s| s.year == year - 1 && s.team == Some(t) && s.starts >= 8);
            let youth = if pos.is_specialist() { 0.0 } else { 0.3 * (27.0 - p.player.age as f64) };
            let ovr = p.player.overall();
            let cost = if seasons >= 4 && ovr < crate::career::NFL_LEVELS.starter {
                (0.5 * (seasons as f64 - 3.0)).min(3.0)
            } else {
                0.0
            };
            // The GM's biases (3.1 sunk cost, 3.2 impatience, 3.3 "not my
            // guy") and the owner's pet (3.12).
            let gm = &w.clubs[t].office.gm;
            let mut bias = 0.0;
            if c.contract
                .as_ref()
                .is_some_and(|k| k.team == t && k.signed >= gm.since && k.apy() > money::price_cap(year) * 0.03)
            {
                bias += 2.0 * gm.sunk_cost;
            }
            if let Some((dy, r, _, dt)) = c.draft
                && dt == t
            {
                if r <= 3 && seasons >= 2 && ovr < crate::career::NFL_LEVELS.starter {
                    bias -= 2.0 * gm.impatience;
                }
                if dy < gm.since {
                    bias -= 2.0 * gm.not_my_guy;
                }
            }
            if w.clubs[t].office.owner.pet == Some(i) {
                bias += 10.0;
            }
            keep.push((i, seen + protection + if starter { 3.0 } else { 0.0 } + youth - cost + bias));
        }
        keep.sort_by(|a, b| b.1.total_cmp(&a.1));
        // Each position's share of the limit first, then the best of the rest.
        let mut chosen: Vec<usize> = Vec::new();
        for pos in Position::ALL {
            let s = depth_slots(pos);
            let quota = (s.starters + s.rotation).max(s.roster * limit / 53);
            chosen.extend(keep.iter().filter(|x| w.people[x.0].player.position == pos).take(quota).map(|x| x.0));
        }
        for &(i, _) in &keep {
            if chosen.len() >= limit {
                break;
            }
            if !chosen.contains(&i) {
                chosen.push(i);
            }
        }
        let cut_line = keep.iter().filter(|x| chosen.contains(&x.0)).map(|x| x.1).fold(f64::MAX, f64::min);
        // The practice squad: young players with at most two seasons on a
        // roster, close to the line (the era's size; none before 1989).
        let mut squad: Vec<usize> = Vec::new();
        let mut cut: Vec<usize> = Vec::new();
        for &(i, v) in &keep {
            if chosen.contains(&i) {
                continue;
            }
            let c = w.people[i].pro.as_ref().unwrap();
            let seasons = c.seasons.iter().filter(|s| s.games > 0).count() as u32;
            let eligible = seasons <= 2 && c.ps_seasons < 3 && w.people[i].player.age <= 26 && v >= cut_line - 7.0;
            if squad.len() < ps && eligible {
                squad.push(i);
            } else {
                cut.push(i);
            }
        }
        for i in chosen {
            w.people[i].pro.as_mut().unwrap().status = Status::Roster;
        }
        for i in squad {
            // A practice squad deal replaces his roster contract (real: a
            // weekly salary about a quarter of the rookie minimum's); a
            // bonus already paid stays on the cap.
            let c = w.people[i].pro.as_ref().unwrap();
            if c.contract.as_ref().is_some_and(|k| k.kind != money::Kind::PracticeSquad) {
                let (now, later) = c.contract.as_ref().unwrap().dead_if_cut(year, d, true);
                let id = w.people[i].player.id;
                w.clubs[t].add_dead(year, now.max(0.0), id, DeadWhy::PracticeSquad);
                w.clubs[t].add_dead(year + 1, later, id, DeadWhy::PracticeSquad);
                let weeks = w.rules.nfl_games as f64 + 1.0;
                let pay = money::practice_squad_week(year) * weeks;
                let deal = money::build(t, money::Kind::PracticeSquad, year, &money::Terms::flat(1, pay));
                w.people[i].pro.as_mut().unwrap().contract = Some(deal);
            }
            let c = w.people[i].pro.as_mut().unwrap();
            if c.status != Status::PracticeSquad {
                c.ps_seasons += 1;
            }
            c.status = Status::PracticeSquad;
            let name = w.nfl[t].name;
            let p = &mut w.people[i];
            let age = p.row_age();
            p.note(format!("Cut from the {name}'s roster at {age}, he stayed on the practice squad."));
        }
        for i in cut {
            release(w, i, d, "cut at the final cutdown");
        }
    }
}

/// Black Monday for the front offices (topic 7): the fans' verdict on the
/// season, the owner's trust in his GM, and a GM who lost it fired (one
/// in about 30 a year is hired away or retires besides).
pub(crate) fn black_monday(w: &mut World) {
    let year = w.year;
    // The books close on the regular season: what is left carries over.
    for t in 0..w.clubs.len() {
        w.clubs[t].unused = Some((year, space(w, t, year).max(0.0)));
    }
    office::fan_year(w);
    market::settle_incentives(w);
    let out: Vec<usize> = w
        .nfl_season
        .as_ref()
        .map_or(Vec::new(), |s| (0..s.seed.len()).filter(|&t| s.active[t] && s.seed[t].is_none()).collect());
    let today = w.today;
    market::futures(w, today, &out);
    for t in 0..w.clubs.len() {
        if !super::franchise::active(t, year) {
            continue;
        }
        let o = &w.clubs[t].office;
        let tenure = year - o.gm.since;
        let fired = o.owner.trust < 0.3 && tenure >= 2;
        let gone = w.rng.chance(0.03);
        if fired {
            office::replace_gm(w, t, "fired after the season");
        } else if gone {
            office::replace_gm(w, t, "moved on");
        }
    }
}

/// A week of the season for the injured and the lists: injuries heal a
/// week; a player healed on injured reserve comes back while his team has
/// returns left (from 2012), else he waits out the season there.
fn heal_week(w: &mut World) {
    let year = w.year;
    let pros: Vec<usize> = (0..w.people.len()).filter(|&i| w.people[i].stage == Stage::Pro).collect();
    for i in pros {
        let Some(c) = w.people[i].pro.as_mut() else { continue };
        let back = c.out_weeks == 1;
        c.out_weeks = c.out_weeks.saturating_sub(1);
        let status = c.status;
        let team = c.team;
        {
            let s = c.season_mut(year);
            match status {
                Status::Reserve => s.ir_weeks += 1,
                Status::PracticeSquad => s.ps_weeks += 1,
                _ => {}
            }
        }
        if back
            && status == Status::Reserve
            && let Some(t) = team
            && w.clubs[t].ir_used < games::ir_returns(year)
        {
            w.clubs[t].ir_used += 1;
            w.people[i].pro.as_mut().unwrap().status = Status::Roster;
            let over = count(w, t, Status::Roster) > w.rules.nfl_roster as usize;
            if over {
                // Room for him: the team lets its lowest-rated healthy player at the deepest spot go.
                let cut = w.clubs[t]
                    .roster
                    .iter()
                    .copied()
                    .filter(|&j| j != i && w.people[j].pro.as_ref().is_some_and(|c| c.status == Status::Roster))
                    .min_by(|&a, &b| grade(w, t, a).total_cmp(&grade(w, t, b)));
                if let Some(j) = cut {
                    let d = w.today;
                    release(w, j, d, "cut to make room for a player back from injured reserve");
                }
            }
        }
    }
}

/// The unsigned veteran's week (topic 1.3): he weighs his chance of a call
/// (his level against the last man on rosters at his position, the
/// injuries around the league) against his age, what he has earned, his
/// family's means and his love of the game. Most give up by midseason; a
/// few hold out for a call (proposed, not confirmed sizes).
fn unsigned_week(w: &mut World, d: Date) {
    let year = w.year;
    let mut floor = [f64::MAX; POSITION_COUNT];
    for t in 0..w.nfl.len() {
        for pos in Position::ALL {
            let s = depth_slots(pos);
            let list = &w.nfl[t].depth[pos.index()];
            if let Some(m) =
                list.iter().filter(|m| m.role != Some(PlayingTime::PracticeSquad)).nth(s.roster.saturating_sub(1))
            {
                floor[pos.index()] = floor[pos.index()].min(m.rating);
            }
        }
    }
    let pros: Vec<usize> = (0..w.people.len())
        .filter(|&i| {
            w.people[i].stage == Stage::Pro
                && w.people[i].pro.as_ref().is_some_and(|c| c.status == Status::FreeAgent && !c.on_waivers)
                && !w.market.pending.iter().any(|x| x.person == i)
        })
        .collect();
    for i in pros {
        let p = &mut w.people[i];
        let c = p.pro.as_mut().unwrap();
        c.weeks_unsigned += 1;
        // Out of football, he loses his edge (topic 1.2).
        let ethic = p.player.personality.work_ethic;
        c.sharpness = clamp(c.sharpness - 0.02 - 0.002 * (10.0 - ethic), 0.4, 1.0);
        let pos = p.player.position;
        let gap = p.player.overall() - floor[pos.index()].min(99.0);
        let call = clamp(0.5 + gap / 8.0, 0.02, 0.95) * if pos == Position::QB { 1.3 } else { 1.0 };
        let age = p.player.age as f64;
        let love = p.player.personality.love_of_game;
        let rich = clamp(c.earnings / (money::price_cap(year) * 0.05), 0.0, 1.0);
        let weekly = clamp(
            0.03 + 0.05 * (1.0 - call) + 0.01 * (age - 28.0).max(0.0) + 0.02 * rich
                - 0.004 * (love - 10.0)
                - 0.01 * (1.0 - p.bg.money),
            0.005,
            0.4,
        );
        if p.rng.chance(weekly) {
            let weeks = c.weeks_unsigned;
            retire(w, i, d, &format!("unsigned for {weeks} weeks"));
        }
    }
}

/// He walks away: his career is written into his alumni record and he
/// leaves the league.
pub(crate) fn retire(w: &mut World, i: usize, d: Date, why: &str) {
    if w.people[i].pro.as_ref().is_some_and(|c| !c.seasons.is_empty()) {
        honors::hall_case(w, i);
    }
    if w.people[i].pro.as_ref().is_some_and(|c| c.team.is_some()) {
        let year = w.year;
        let t = w.people[i].pro.as_ref().unwrap().team.unwrap();
        // A retiring player under contract leaves his bonus on the cap.
        if let Some(k) = w.people[i].pro.as_mut().unwrap().contract.take() {
            let dead = k.dead_if_cut(year, d, d >= w.dates.june_1);
            let id = w.people[i].player.id;
            w.clubs[t].add_dead(year, dead.0, id, DeadWhy::Retired);
            w.clubs[t].add_dead(year + 1, dead.1, id, DeadWhy::Retired);
        }
        w.clubs[t].roster.retain(|&x| x != i);
    }
    let p = &mut w.people[i];
    let age = p.row_age();
    p.note(format!("Retired at {age} ({why})."));
    p.stage = Stage::Done;
    if let Some(c) = p.pro.as_mut() {
        c.team = None;
        c.status = Status::FreeAgent;
    }
    if let Some(k) = p.pro.as_ref().and_then(|c| c.alumnus)
        && let Some(a) = w.alumni.get_mut(k)
    {
        a.pro = p.pro.clone();
        a.player = Some(Box::new(p.player.clone()));
        a.notes = p.notes.clone();
    }
}

/// The season's end (the Super Bowl): seasons accrue, pay is booked and
/// each player decides whether to go on (the class model's retirement,
/// read on persons).
fn season_end(w: &mut World, d: Date) {
    let year = w.year;
    let pros: Vec<usize> = (0..w.people.len()).filter(|&i| w.people[i].stage == Stage::Pro).collect();
    // A season on the stand-in has no box scores: a player on a roster at
    // the end played the season, a starter started it.
    if !w.options.engine_from.is_none_or(|from| year >= from) {
        let games = if year >= 2021 { 17 } else { 16 };
        for t in 0..w.clubs.len() {
            for pos in Position::ALL {
                let starters = depth_slots(pos).starters;
                let list = w.clubs[t].depth.get(pos.index()).cloned().unwrap_or_default();
                for (slot, i) in list.into_iter().enumerate() {
                    let Some(c) = w.people[i].pro.as_mut() else { continue };
                    // A futures deal signed in January is for next season.
                    if c.contract.as_ref().is_some_and(|k| k.signed > year)
                        || c.seasons.last().is_some_and(|s| s.year == year && s.ps_weeks > 8)
                    {
                        continue;
                    }
                    let s = c.season_mut(year);
                    s.team = Some(t);
                    s.games = games;
                    s.starts = if slot < starters { games } else { 0 };
                }
            }
        }
    }
    for &i in &pros {
        let p = &mut w.people[i];
        let c = p.pro.as_mut().unwrap();
        let held = c.held();
        let squad = c.status == Status::PracticeSquad;
        let pay = c.contract.as_ref().map_or(0.0, |k| {
            k.year(year).map_or(0.0, |x| x.cash()) + if k.signed == year { k.signing_bonus } else { 0.0 }
        });
        if let Some(t) = c.team {
            match w.clubs[t].cash.last_mut() {
                Some(x) if x.0 == year => x.1 += pay,
                _ => w.clubs[t].cash.push((year, pay)),
            }
        }
        let c = w.people[i].pro.as_mut().unwrap();
        let s = c.season_mut(year);
        s.pay += pay;
        // Six games on full pay status accrue a season (games fill in from the engine).
        let accrues = held && (s.games >= 6 || s.games == 0 && !squad && s.ps_weeks == 0);
        c.earnings += pay;
        if accrues {
            c.accrued += 1;
            c.credited += 1;
        }
    }
    for i in pros {
        let p = &w.people[i];
        let c = p.pro.as_ref().unwrap();
        let age = p.player.age as f64;
        let love = p.player.personality.love_of_game;
        let ovr = p.player.overall();
        let pos = p.player.position;
        if c.status == Status::FreeAgent {
            continue;
        }
        let typical =
            retire_age(pos) + p.player.longevity * 0.8 + (love - 10.0) * 0.2 - (p.player.wear - 6.0).max(0.0) * 0.3;
        let mut standing = if ovr >= 85.0 {
            0.35
        } else if ovr >= 78.0 {
            0.6
        } else {
            1.0
        };
        if age > typical + 2.0 {
            standing += (1.0 - standing) * ((age - typical - 2.0) / 3.0).min(1.0);
        }
        let chance =
            (0.008 + 0.5 / (1.0 + (-(age - typical) / 1.3).exp())) * standing + (age - typical - 4.0).max(0.0) * 0.08;
        let roll = key_uniform(w.options.seed as u64 ^ 0x2E71 ^ (year as u64) << 32, p.player.id as u64);
        if roll < chance {
            retire(w, i, d, "end of the season");
        }
    }
}

/// The usual age a player walks away at each position (the class model's).
fn retire_age(pos: Position) -> f64 {
    crate::career::retire_age(pos)
}

/// A short line on a pro for news and pages.
pub fn describe(w: &World, i: usize) -> String {
    let p = &w.people[i];
    let team = p.pro.as_ref().and_then(|c| c.team).map_or("free agent", |t| w.nfl[t].name);
    format!("{} ({}, {}, {})", p.name, p.player.position, p.player.age, team)
}

#[allow(dead_code)]
fn post(w: &mut World, d: Date, team: Option<usize>, loud: Loudness, text: String) {
    let mut e = Event::new(d, CalLevel::Nfl, Kind::Transfer, loud, text);
    if let Some(t) = team {
        e = e.team(t);
    }
    w.calendar.post(e);
}
