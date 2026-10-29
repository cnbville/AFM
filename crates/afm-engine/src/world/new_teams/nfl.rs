//! NFL expansion after 2026 (SK68). The owners take up expansion at their
//! annual meeting at the real rate (three awards since the 1970 merger:
//! Seattle and Tampa Bay in 1974, Carolina and Jacksonville in 1993,
//! Houston in 1999), award two clubs to the biggest markets without one,
//! and the clubs play two seasons later. Each old club leaves its open
//! players on a list a week before the expansion draft in February; the
//! new clubs take turns, and the club that lost a player pulls one more
//! back (the 1995 rule). Their places in their first draft are in
//! `franchise::round_order`.

use crate::calendar::{Date, Event, Kind, Level, Loudness};
use crate::rng::Rng;
use crate::world::World;
use crate::world::franchise::{self, Era, NewClub, Roof};

/// US markets without a club that real expansion and relocation bids
/// have named, with their metro population (2020 Census, millions) and a
/// nickname for each (Claude's picks, proposed, not confirmed: a new club
/// lets its fans vote on a name).
pub const MARKETS: [(&str, &str, f64, &str); 13] = [
    ("San Diego", "CA", 3.30, "Tritons"),
    ("St. Louis", "MO", 2.82, "Archers"),
    ("Orlando", "FL", 2.67, "Comets"),
    ("San Antonio", "TX", 2.56, "Marshals"),
    ("Portland", "OR", 2.51, "Steelheads"),
    ("Sacramento", "CA", 2.40, "Miners"),
    ("Austin", "TX", 2.28, "Lonestars"),
    ("Columbus", "OH", 2.14, "Aviators"),
    ("Virginia Beach", "VA", 1.80, "Privateers"),
    ("Oklahoma City", "OK", 1.43, "Drillers"),
    ("Raleigh", "NC", 1.41, "Oaks"),
    ("Memphis", "TN", 1.34, "Blues"),
    ("Salt Lake City", "UT", 1.26, "Summit"),
];

/// A year's chance that expansion comes to the owners' vote: three awards
/// in the 56 seasons since the 1970 merger (real).
pub const VOTE_ODDS: f64 = 3.0 / 56.0;

/// Each owner's yes: his share of the fee against a smaller share of the
/// national money (proposed, not confirmed: every real expansion that
/// reached a vote passed).
const OWNER_YES: f64 = 0.85;

/// Seasons from the award to the first game (real: 1974 to 1976, 1993 to
/// 1995).
pub const LEAD: i32 = 2;

/// The expansion fee against the league's average club value: Carolina
/// and Jacksonville paid $140M in 1993 (about the average then), Houston
/// $700M in 1999 (about 1.75 times). Proposed, not confirmed: 1.4.
const FEE_MULTIPLE: f64 = 1.4;

/// The most players a new club takes in its expansion draft (Claude's
/// pick, proposed, not confirmed; the real clubs took 19 to 37).
const MAX_TAKEN: usize = 42;

/// The share of the cap a new club spends on its expansion draft haul
/// (proposed, not confirmed: Houston stopped at 19 in 2002 over pay).
const HAUL_CAP: f64 = 0.55;

/// A market's club city, if any active or coming club plays there.
fn taken(w: &World, city: &str) -> bool {
    let year = w.year;
    (0..w.nfl.len()).any(|t| {
        let (c, _) = crate::world::finance::nfl::city(w, t, year);
        (franchise::active_in(w, t, year) || w.expansion.club(t).is_some()) && c == city
    }) || w.expansion.clubs.iter().any(|c| c.era.city == city)
}

/// Markets still open, by index into `MARKETS`.
pub fn open_markets(w: &World) -> Vec<usize> {
    (0..MARKETS.len()).filter(|&k| !taken(w, MARKETS[k].0)).collect()
}

/// A club pending: awarded and not playing yet.
pub fn pending(w: &World) -> bool {
    w.expansion.clubs.iter().any(|c| c.first > w.year)
}

/// The owners' annual meeting (from 2027): expansion comes up at the real
/// rate; three in four owners must say yes.
pub(crate) fn vote(w: &mut World, d: Date, rng: &mut Rng) {
    if w.year <= franchise::PRESENT || pending(w) || !rng.chance(VOTE_ODDS) {
        return;
    }
    let mut open = open_markets(w);
    if open.len() < 2 {
        return;
    }
    let owners = franchise::active_teams_in(w, w.year).len() as u32;
    let need = (owners * 3).div_ceil(4);
    let yes = (0..owners).filter(|_| rng.chance(OWNER_YES)).count() as u32;
    let passed = yes >= need;
    w.expansion.votes.push((w.year, passed));
    if !passed {
        let text = format!("The owners vote down expansion, {yes} for and {} against ({need} needed).", owners - yes);
        w.calendar.post(Event::new(d, Level::Nfl, Kind::RuleChange, Loudness::Big, text));
        return;
    }
    // The strongest bids win: the bigger the market, the likelier its
    // ownership group and stadium deal (proposed, not confirmed).
    let mut picked = Vec::new();
    for _ in 0..2 {
        let weights: Vec<(usize, f64)> = open.iter().map(|&k| (k, MARKETS[k].2.powi(2))).collect();
        let k = rng.weighted(&weights);
        open.retain(|&x| x != k);
        picked.push(k);
    }
    let first = w.year + LEAD;
    let teams = award(w, &picked, first);
    let names: Vec<&str> = teams.iter().map(|&t| w.nfl[t].name).collect();
    let fee = w.expansion.club(teams[0]).map_or(0.0, |c| c.fee);
    let text = format!(
        "The owners award expansion clubs to {} and {}, {yes}-{}: the {} and the {} pay {} each and play from {first}.",
        MARKETS[picked[0]].0,
        MARKETS[picked[1]].0,
        owners - yes,
        names[0],
        names[1],
        money(fee),
    );
    w.log.rule_changes.push(text.clone());
    w.calendar.post(Event::new(d, Level::Nfl, Kind::RuleChange, Loudness::Huge, text).team(teams[0]).memorable());
}

fn money(x: f64) -> String {
    if x >= 1.0e9 { format!("${:.1} billion", x / 1.0e9) } else { format!("${:.0} million", x / 1.0e6) }
}

/// Where a division's clubs play (their mean place).
fn division_center(w: &World, div: usize) -> Option<(f64, f64)> {
    let year = w.year;
    let places: Vec<(f64, f64)> = (0..w.nfl.len())
        .filter(|&t| franchise::alignment_in(w, t, year.max(franchise::PRESENT)).is_some_and(|a| a.1 == div))
        .filter_map(|t| {
            let (city, state) = crate::world::finance::nfl::city(w, t, year);
            crate::world::geo::state_index(&state).map(|s| crate::world::geo::place(s, &city))
        })
        .collect();
    if places.is_empty() {
        return None;
    }
    let n = places.len() as f64;
    Some((places.iter().map(|p| p.0).sum::<f64>() / n, places.iter().map(|p| p.1).sum::<f64>() / n))
}

fn market_place(k: usize) -> (f64, f64) {
    let (city, state, _, _) = MARKETS[k];
    crate::world::geo::state_index(state).map_or((39.0, -95.0), |s| crate::world::geo::place(s, city))
}

/// The nearest division of a conference to a market, the smaller first
/// when two are as near (proposed, not confirmed: the league keeps
/// rivalries and places a new club by geography, as it placed Carolina and
/// Houston).
fn nearest_division(w: &World, conf: usize, k: usize) -> (usize, f64) {
    let here = market_place(k);
    (conf * 4..conf * 4 + 4)
        .filter_map(|d| division_center(w, d).map(|c| (d, crate::world::geo::miles_between(here, c))))
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .unwrap_or((conf * 4, 0.0))
}

/// Awards new clubs to these markets (indices into `MARKETS`), playing
/// from `first`: two split between the conferences by geography, one to
/// the smaller conference. Returns their indices in `World::nfl`.
pub fn award(w: &mut World, markets: &[usize], first: i32) -> Vec<usize> {
    let year = w.year.max(franchise::PRESENT);
    let size = |w: &World, c: usize| {
        (0..w.nfl.len()).filter(|&t| franchise::alignment_in(w, t, year).is_some_and(|a| a.0 == c)).count()
            + w.expansion.clubs.iter().filter(|x| x.first > w.year && x.era.div as usize / 4 == c).count()
    };
    let confs: Vec<usize> = match markets {
        [a, b] => {
            let ab = nearest_division(w, 0, *a).1 + nearest_division(w, 1, *b).1;
            let ba = nearest_division(w, 1, *a).1 + nearest_division(w, 0, *b).1;
            if ab <= ba { vec![0, 1] } else { vec![1, 0] }
        }
        _ => markets.iter().map(|_| if size(w, 0) <= size(w, 1) { 0 } else { 1 }).collect(),
    };
    let mut out = Vec::new();
    for (&k, &conf) in markets.iter().zip(&confs) {
        let div = nearest_division(w, conf, k).0;
        out.push(add_club(w, k, div as u8, first));
    }
    out
}

/// The league's average club value now, in dollars.
fn average_value(w: &World) -> f64 {
    let year = w.year;
    let v: Vec<f64> = franchise::active_teams_in(w, year)
        .into_iter()
        .filter_map(|t| w.finance.nfl.get(t).map(|c| c.value as f64))
        .filter(|&x| x > 0.0)
        .collect();
    if v.is_empty() { 7.0e9 } else { v.iter().sum::<f64>() / v.len() as f64 }
}

/// A new club: its league entry, owner and general manager, books and a
/// new stadium. A new stadium is enclosed in a hot or cold market and
/// open on the mild West Coast (proposed, not confirmed: as the stadiums
/// built since 2016 are).
fn add_club(w: &mut World, k: usize, div: u8, first: i32) -> usize {
    let (city, state, metro, nick) = MARKETS[k];
    let t = w.nfl.len();
    let roof = if matches!(state, "CA" | "OR") { Roof::Open } else { Roof::Dome };
    let name: &'static str = Box::leak(format!("{city} {nick}").into_boxed_str());
    let era = Era { from: first, to: 9999, name, city, state, div, roof };
    let mut rng = Rng::new(w.options.seed ^ 0x0E_A9A0 ^ (t as u32) << 8 ^ first as u32);
    let draft_skill = crate::league::new_staff_rating(&mut rng);
    let owner = crate::league::new_staff_rating(&mut rng);
    let coaching = crate::league::new_staff_rating(&mut rng);
    w.nfl.push(crate::league::Team {
        id: t,
        name,
        draft_skill,
        owner,
        coaching,
        depth: Default::default(),
        staff_years: 0,
        head_coach: None,
        oc: None,
        dc: None,
        win_share: Vec::new(),
    });
    let gm_name =
        crate::world::names::person_name(crate::world::ethnicity::Ethnicity::White, w.year - rng.int(38, 55), &mut rng);
    let gm = crate::world::pro::office::new_gm(&mut rng, w.year, draft_skill, gm_name);
    let mut ownr = crate::world::pro::office::new_owner(&mut rng);
    // A new owner who paid the fee gives his club time (proposed, not
    // confirmed: Carolina and Jacksonville's first coaches got four and
    // eight seasons).
    ownr.patience = (ownr.patience + 0.25).min(1.0);
    let power = crate::world::pro::office::draw_power(&mut rng, &ownr);
    let mut club = crate::world::pro::Club::default();
    club.office = crate::world::pro::office::Office { gm, owner: ownr, power, ..Default::default() };
    w.clubs.resize_with(t, Default::default);
    w.clubs.push(club);
    let fee = FEE_MULTIPLE * average_value(w);
    crate::world::finance::nfl::add_club(w, t, city, state, roof != Roof::Open, first);
    let n = w.nfl.len();
    if !w.market.fleece.is_empty() {
        for row in w.market.fleece.iter_mut() {
            row.resize(n, 0.0);
        }
        w.market.fleece.resize(n, vec![0.0; n]);
    }
    for s in [w.nfl_season.as_mut(), w.nfl_prev.as_mut()].into_iter().flatten() {
        s.grow(n);
    }
    w.expansion.clubs.push(NewClub { team: t, awarded: w.year, first, era, fee, metro, drafted: false });
    t
}

// ---------------------------------------------------------------------------
// The expansion draft (February of the first season)
// ---------------------------------------------------------------------------

/// The day each old club's open players are listed, and the expansion
/// draft's day (real: Houston's lists came in on 6 February 2002 and its
/// draft was on the 18th, two weeks after the Super Bowl).
pub fn lists_day(w: &World) -> Date {
    w.dates.start.add(3)
}

pub fn draft_day(w: &World) -> Date {
    w.dates.start.add(14)
}

/// The clubs that play their first season this year and haven't drafted.
pub fn drafting(w: &World) -> Vec<usize> {
    let year = w.year;
    w.expansion.clubs.iter().filter(|c| c.first == year && !c.drafted).map(|c| c.team).collect()
}

/// The clubs whose expansion draft comes next: drafting this February,
/// else awarded and playing next season.
pub fn drafting_or_pending(w: &World) -> Vec<usize> {
    let now = drafting(w);
    if !now.is_empty() {
        return now;
    }
    w.expansion.clubs.iter().filter(|c| c.first == w.year + 1 && !c.drafted).map(|c| c.team).collect()
}

/// How many open players each old club lists: six when two clubs draft
/// (1995), five when one does (1999, 2002).
fn open_count(new: usize) -> usize {
    if new >= 2 { 6 } else { 5 }
}

/// Each old club's open players as `judge` sees them: it protects its
/// best 22 and leaves its oldest others open (the clubs' own grades for
/// the real lists; your staff's grades for its guess).
pub(crate) fn open_lists(w: &World, new: &[usize], judge: impl Fn(usize, usize) -> f64) -> Vec<(usize, usize, f64)> {
    let year = w.year;
    let open_n = open_count(new.len());
    let mut pool = Vec::new();
    for t in franchise::active_teams_in(w, year).into_iter().filter(|t| !new.contains(t)) {
        let mut ranked: Vec<(usize, f64, u32)> = w.clubs[t]
            .roster
            .iter()
            .filter(|&&i| {
                w.people[i].pro.as_ref().is_some_and(|c| c.status != crate::world::pro::Status::PracticeSquad)
            })
            .map(|&i| {
                let age = w.people[i].player.age;
                (i, judge(t, i) - 1.5 * (age as f64 - 29.0).max(0.0), age)
            })
            .collect();
        ranked.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
        let mut tail: Vec<_> = ranked.into_iter().skip(22).collect();
        tail.sort_by(|a, b| b.2.cmp(&a.2).then(b.1.total_cmp(&a.1)).then(a.0.cmp(&b.0)));
        for &(i, keep, _) in tail.iter().take(open_n) {
            pool.push((t, i, keep));
        }
    }
    pool
}

/// The NFL's February days for its new clubs: the lists, then the draft.
pub(crate) fn day(w: &mut World, d: Date) {
    let new = drafting(w);
    if new.is_empty() {
        return;
    }
    if d == lists_day(w) && w.expansion.lists.is_empty() {
        let pool = open_lists(w, &new, |t, i| crate::world::pro::grade(w, t, i));
        w.expansion.lists = pool.iter().map(|&(t, i, _)| (t, w.people[i].player.id)).collect();
        let names: Vec<&str> = new.iter().map(|&t| w.nfl[t].name).collect();
        let text = format!(
            "The other clubs hand in their protected lists: {} players are open to the {} in the expansion draft on {}.",
            pool.len(),
            names.join(" and the "),
            draft_day(w).pretty()
        );
        w.calendar.post(Event::new(d, Level::Nfl, Kind::DraftChoice, Loudness::Big, text).team(new[0]));
    }
    if d == draft_day(w) {
        run_draft(w, d, &new);
    }
}

/// Your club's next pick in the expansion draft: your queue in order, the
/// first still open (player ids).
fn queued(w: &World, open: &[(usize, usize, f64)]) -> Option<usize> {
    w.expansion.queue.iter().find_map(|&id| open.iter().position(|e| w.people[e.1].player.id == id))
}

/// The best street free agent at each position, by a club's grade: what
/// it could sign instead of an open player.
fn street_best(w: &World, t: usize) -> [f64; crate::positions::POSITION_COUNT] {
    let mut best = [0.0f64; crate::positions::POSITION_COUNT];
    for (i, p) in w.people.iter().enumerate() {
        if p.stage != crate::world::Stage::Pro
            || p.pro.as_ref().is_none_or(|c| c.team.is_some() || c.status != crate::world::pro::Status::FreeAgent)
        {
            continue;
        }
        let k = p.player.position.index();
        best[k] = best[k].max(crate::world::pro::grade(w, t, i));
    }
    best
}

/// The expansion draft: the new clubs take turns; after each pick the club
/// that lost a player pulls its most valued open player back (1995). A
/// club stops when the best open player is worth no more to it than the
/// street's best at his spot, when his pay would take the haul past its
/// share of the cap, or at 42.
fn run_draft(w: &mut World, d: Date, new: &[usize]) {
    let year = w.year;
    let ids: std::collections::HashMap<u32, usize> =
        w.people.iter().enumerate().map(|(i, p)| (p.player.id, i)).collect();
    let mut pool: Vec<(usize, usize, f64)> = w
        .expansion
        .lists
        .iter()
        .filter_map(|&(t, id)| ids.get(&id).map(|&i| (t, i)))
        .filter(|&(t, i)| w.people[i].pro.as_ref().is_some_and(|c| c.team == Some(t)))
        .map(|(t, i)| (t, i, crate::world::pro::grade(w, t, i)))
        .collect();
    let mine = w.seat.and_then(|s| s.nfl_team).filter(|t| new.contains(t));
    let cap = crate::world::pro::money::price_cap(year);
    let streets: Vec<_> = new.iter().map(|&t| street_best(w, t)).collect();
    let mut picks: Vec<Vec<usize>> = vec![Vec::new(); new.len()];
    let mut haul = vec![0.0f64; new.len()];
    let mut done = vec![false; new.len()];
    let mut turn = 0;
    while done.iter().any(|x| !x) && !pool.is_empty() {
        let k = turn % new.len();
        turn += 1;
        if done[k] {
            continue;
        }
        let t = new[k];
        let value = |e: &(usize, usize, f64)| {
            let age = w.people[e.1].player.age as f64;
            crate::world::pro::grade(w, t, e.1) - 0.6 * (age - 28.0).max(0.0)
        };
        let hit =
            |i: usize| w.people[i].pro.as_ref().and_then(|c| c.contract.as_ref()).map_or(0.0, |c| c.cap_hit(year));
        // Your queue first at your club; when it runs dry your staff picks.
        let queue = if Some(t) == mine { queued(w, &pool) } else { None };
        let choice = queue.or_else(|| {
            (0..pool.len())
                .filter(|&e| haul[k] + hit(pool[e].1) <= HAUL_CAP * cap)
                .filter(|&e| value(&pool[e]) > streets[k][w.people[pool[e].1].player.position.index()])
                .max_by(|&a, &b| value(&pool[a]).total_cmp(&value(&pool[b])).then(b.cmp(&a)))
        });
        let Some(best) = choice else {
            done[k] = true;
            continue;
        };
        let (from, i, _) = pool.swap_remove(best);
        haul[k] += hit(i);
        picks[k].push(i);
        if picks[k].len() >= MAX_TAKEN {
            done[k] = true;
        }
        if new.len() >= 2
            && let Some(pull) =
                (0..pool.len()).filter(|&e| pool[e].0 == from).max_by(|&a, &b| pool[a].2.total_cmp(&pool[b].2))
        {
            pool.swap_remove(pull);
        }
    }
    for (k, v) in picks.iter().enumerate() {
        for &i in v {
            crate::world::pro::transfer(w, i, new[k], "taken in the expansion draft");
        }
    }
    for c in w.expansion.clubs.iter_mut().filter(|c| new.contains(&c.team)) {
        c.drafted = true;
    }
    w.expansion.taken = picks
        .iter()
        .zip(new)
        .flat_map(|(v, &t)| v.iter().map(move |&i| (t, i)))
        .map(|(t, i)| (t, w.people[i].player.id))
        .collect();
    w.expansion.lists.clear();
    w.expansion.queue.clear();
    crate::world::pro::rebuild(w);
    let parts: Vec<String> =
        new.iter().zip(&picks).map(|(&t, v)| format!("the {} take {}", w.nfl[t].name, v.len())).collect();
    let text =
        format!("Expansion draft: {}. Free agency, the draft and the street fill the rest.", parts.join(" and "));
    w.calendar.post(Event::new(d, Level::Nfl, Kind::DraftChoice, Loudness::Big, text).team(new[0]).memorable());
}
