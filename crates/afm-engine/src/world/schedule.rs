//! Section 4: the college schedule (topic 3, A27, A28). Each season's games
//! sit on their real Saturdays: conference games by each conference's era
//! rules, rivalries in their slot with their trophy, non-conference games
//! (most FBS teams buy one FCS opponent), senior day and homecoming. The
//! postseason games are added once the season decides who plays
//! (`college.rs`).
//!
//! Conference membership is the 2026 membership for every year, as in the
//! rest of the world (section 2). Every listed rivalry is played in its real
//! years (part B 28), and every year after 2025 unless it has ended.

use std::collections::{BTreeMap, HashMap, HashSet};

use super::World;
use super::data::Division;
use super::geo;
use super::season::key_uniform;
use crate::calendar::data::{RIVALRIES, RivalrySlot, et_offset};
use crate::calendar::rules::conference_rules;
use crate::calendar::{Date, Loudness, YearDates};

/// What a game is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum GameKind {
    Regular,
    ConferenceTitle,
    /// A bowl outside the playoff, by index into `calendar::data::BOWLS`.
    Bowl(usize),
    /// The SWAC-MEAC Celebration Bowl (FCS, since 2015).
    Celebration,
    /// A playoff round for the division's title (1 = the first round played).
    Playoff(u8),
    /// The game for the national title.
    Title,
}

/// Why a game matters beyond the standings (A40, A41).
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Tag {
    /// A listed rivalry, with its trophy and all-time record.
    Rivalry,
    /// A player facing the program he left.
    Revenge,
    /// A head coach facing a program he used to coach.
    CoachReturn,
    /// A player playing near home with his family there.
    Hometown,
    /// A career or season record within reach.
    Milestone,
    /// Both teams ranked in the latest poll.
    Ranked,
    /// A playoff or title-game place on the line.
    PlayoffSpot,
    /// The head coach's job on the line.
    HotSeat,
}

impl Tag {
    pub fn name(self) -> &'static str {
        match self {
            Tag::Rivalry => "rivalry",
            Tag::Revenge => "revenge game",
            Tag::CoachReturn => "coach's return",
            Tag::Hometown => "hometown game",
            Tag::Milestone => "milestone",
            Tag::Ranked => "ranked matchup",
            Tag::PlayoffSpot => "playoff spot on the line",
            Tag::HotSeat => "job on the line",
        }
    }
}

/// A neutral site.
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub struct Site {
    pub state: usize,
    #[serde(deserialize_with = "crate::save::leaked")]
    pub town: crate::save::Text,
    pub indoor: bool,
}

/// What happened in a game.
#[derive(Clone, Copy, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Played {
    pub score: [u32; 2],
    pub home_won: bool,
    pub temperature: f64,
    pub wind: f64,
    pub rain: bool,
    pub snow: bool,
    pub indoor: bool,
    /// Minutes of lightning delay (A37).
    pub delay: u32,
    /// Moved from its first date by severe weather (A38).
    pub moved: bool,
    /// Never played (a no contest).
    pub cancelled: bool,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Game {
    pub date: Date,
    /// Kickoff on Eastern time (hour, minute).
    pub kickoff: (u16, u16),
    pub home: usize,
    pub away: usize,
    pub division: Division,
    pub site: Option<Site>,
    pub kind: GameKind,
    pub conference: bool,
    /// Index into `calendar::data::RIVALRIES`.
    pub rivalry: Option<usize>,
    /// The home side's last home game (A27).
    pub senior_day: bool,
    /// The home side's homecoming (A27).
    pub homecoming: bool,
    /// A bowl or playoff game's name.
    #[serde(deserialize_with = "crate::save::leaked")]
    pub name: Option<crate::save::Text>,
    pub tags: Vec<Tag>,
    pub loud: Loudness,
    /// Each side's own stakes (T8.1): home, away. A game can be huge for one
    /// team (a sixth win for a bowl, a coach's job) and not for the nation.
    pub huge_for: [bool; 2],
    /// Why the game is loud, in plain words (part B 22).
    #[serde(deserialize_with = "crate::save::leaked")]
    pub why: Vec<&'static str>,
    pub played: Option<Played>,
    /// The betting line when it closed (C8): the home side's spread
    /// (negative when favoured) and the total. Information only.
    #[serde(default)]
    pub line: Option<[f32; 2]>,
    /// The crowd (later extras topic 1), once played.
    #[serde(default)]
    pub attendance: u32,
    /// What the game paid once played (update 2, lane 6): 0 nothing, 1 the
    /// host's guarantee to the visitor, 2 a neutral site's payout to each
    /// side, with the dollars. Last season's page reads it as booked.
    #[serde(default)]
    pub paid: Option<(u8, f32)>,
}

impl Game {
    pub fn new(date: Date, home: usize, away: usize, division: Division, kind: GameKind) -> Game {
        Game {
            date,
            kickoff: (15, 30),
            home,
            away,
            division,
            site: None,
            kind,
            conference: false,
            rivalry: None,
            senior_day: false,
            homecoming: false,
            name: None,
            tags: Vec::new(),
            loud: Loudness::Small,
            huge_for: [false; 2],
            why: Vec::new(),
            played: None,
            line: None,
            attendance: 0,
            paid: None,
        }
    }
    pub fn involves(&self, p: usize) -> bool {
        self.home == p || self.away == p
    }
    pub fn opponent(&self, p: usize) -> usize {
        if self.home == p { self.away } else { self.home }
    }
    /// Whether the home side won; None until played (or if cancelled).
    pub fn home_won(&self) -> Option<bool> {
        self.played.filter(|p| !p.cancelled).map(|p| p.home_won)
    }
}

/// Regular-season games per team (topic 3: 12 in FBS; 11 before 2006).
pub fn games_target(d: Division, season: i32) -> u32 {
    match d {
        Division::FBS => {
            if season >= 2006 {
                12
            } else {
                11
            }
        }
        Division::FCS | Division::D2 => 11,
        // NAIA teams play 10 or 11, Division III 10 (proposed, not confirmed: 10 for both).
        Division::NAIA | Division::D3 => 10,
        Division::JUCO => 9,
    }
}

/// The last regular-season Saturday a division plays, as an index into `cfb_weeks`.
pub fn last_week(d: Division, y: &YearDates) -> usize {
    let end = match d {
        Division::FBS => y.fbs_regular_end,
        Division::FCS => y.fcs_regular_end,
        Division::D2 | Division::NAIA | Division::D3 => y.d2_regular_end,
        Division::JUCO => y.juco_regular_end,
    };
    y.cfb_weeks.iter().rposition(|&w| w <= end).unwrap_or(y.cfb_weeks.len() - 1)
}

/// A kickoff time on Eastern clocks: noon, 3:30 or 7:30, with West Coast
/// night games at 10:30 Eastern (7:30 local).
pub fn kickoff(state_code: &str, key: f64, big: bool) -> (u16, u16) {
    let late_zone = et_offset(state_code) >= 2;
    let slot = if big {
        if key < 0.5 { 1 } else { 2 }
    } else if key < 0.38 {
        0
    } else if key < 0.68 {
        1
    } else {
        2
    };
    match (slot, late_zone) {
        (0, _) => (12, 0),
        (1, _) => (15, 30),
        (_, false) => (19, 30),
        (_, true) => (22, 30),
    }
}

struct Builder<'a> {
    w: &'a World,
    y: &'a YearDates,
    season: i32,
    seed: u64,
    busy: Vec<Vec<bool>>,
    opp: Vec<HashSet<usize>>,
    count: Vec<u32>,
    home: Vec<u32>,
    games: Vec<Game>,
}

impl Builder<'_> {
    fn free(&self, p: usize, wk: usize) -> bool {
        !self.busy[p][wk]
    }
    fn key(&self, a: u64, b: u64) -> f64 {
        key_uniform(self.seed ^ a.wrapping_mul(0x9E37), b)
    }
    #[allow(clippy::too_many_arguments)]
    fn add(
        &mut self,
        a: usize,
        b: usize,
        wk: Option<usize>,
        date: Date,
        home_first: bool,
        conf: bool,
        rivalry: Option<usize>,
    ) {
        let (home, away) = if home_first { (a, b) } else { (b, a) };
        let division = self.w.programs[a].division;
        let mut g = Game::new(date, home, away, division, GameKind::Regular);
        g.conference = conf;
        g.rivalry = rivalry;
        if let Some(k) = rivalry {
            g.tags.push(Tag::Rivalry);
            if let Some((st, town)) = RIVALRIES[k].neutral {
                g.site = Some(Site {
                    state: geo::state_index(st).unwrap_or(0),
                    town,
                    // The Bayou Classic is in the Superdome.
                    indoor: town == "New Orleans",
                });
            }
        }
        let code = self.w.states[self.w.programs[home].state].code.clone();
        g.kickoff = kickoff(&code, self.key(home as u64 ^ 0x4B1C, date.0 as u64), rivalry.is_some());
        if let Some(wk) = wk {
            self.busy[a][wk] = true;
            self.busy[b][wk] = true;
        }
        self.opp[a].insert(b);
        self.opp[b].insert(a);
        self.count[a] += 1;
        self.count[b] += 1;
        self.home[home] += 1;
        self.games.push(g);
    }
    /// A week for a contract whose two sides have none open in common: the
    /// games in the way that week (not a rivalry, which keeps its date, nor
    /// a neutral site) move to weeks their own sides have open, latest
    /// first. A week that needs one move is tried before one that needs two.
    fn make_room(&mut self, a: usize, c: usize) -> Option<usize> {
        let last_of = |p: usize| last_week(self.w.programs[p].division, self.y);
        let lim = last_of(a).min(last_of(c));
        for need in 1..=2 {
            for wk in 1..=lim {
                let sides: Vec<usize> = [a, c].into_iter().filter(|&p| !self.free(p, wk)).collect();
                if sides.len() != need {
                    continue;
                }
                let date = self.y.cfb_weeks[wk];
                // (game, side, its opponent, the week it moves to)
                let mut plan: Vec<(usize, usize, usize, usize)> = Vec::new();
                for &side in &sides {
                    let Some(k) = self.games.iter().position(|g| {
                        g.date == date && (g.home == side || g.away == side) && g.rivalry.is_none() && g.site.is_none()
                    }) else {
                        break;
                    };
                    let other = if self.games[k].home == side { self.games[k].away } else { self.games[k].home };
                    let to_lim = last_of(side).min(last_of(other));
                    let Some(to) = (1..=to_lim).rev().find(|&x| x != wk && self.free(side, x) && self.free(other, x))
                    else {
                        break;
                    };
                    plan.push((k, side, other, to));
                }
                if plan.len() != need {
                    continue;
                }
                for (k, side, other, to) in plan {
                    self.busy[side][wk] = false;
                    self.busy[other][wk] = false;
                    self.busy[side][to] = true;
                    self.busy[other][to] = true;
                    let to_date = self.y.cfb_weeks[to];
                    let home = self.games[k].home;
                    let code = self.w.states[self.w.programs[home].state].code.clone();
                    let kick = kickoff(&code, self.key(home as u64 ^ 0x4B1C, to_date.0 as u64), false);
                    let g = &mut self.games[k];
                    g.date = to_date;
                    g.kickoff = kick;
                }
                return Some(wk);
            }
        }
        None
    }
    /// Who hosts: the side with fewer home games so far; a coin flip on a tie.
    fn host_first(&self, a: usize, b: usize) -> bool {
        if self.home[a] != self.home[b] {
            return self.home[a] < self.home[b];
        }
        self.key(a as u64 * 7919 + b as u64, self.season as u64) < 0.5
    }
    /// The weeks a game prefers, best first. Conference play fills the fall;
    /// non-conference games come early, a few in November (FBS buy games).
    fn week_order(&self, a: usize, b: usize, conf: bool, last: usize) -> Vec<usize> {
        let jitter = |wk: usize| self.key((a * 1009 + b) as u64, wk as u64);
        let mut early: Vec<usize> = (1..=3.min(last)).collect();
        let mut late: Vec<usize> = (4..=last).collect();
        early.sort_by(|&x, &y| jitter(x).total_cmp(&jitter(y)));
        late.sort_by(|&x, &y| jitter(x).total_cmp(&jitter(y)));
        // A few teams open in Week 0.
        let week0 = self.key(a.min(b) as u64 ^ 0x77, self.season as u64) < 0.12;
        let mut out = Vec::new();
        if conf {
            out.extend(late);
            out.extend(early);
        } else {
            out.extend(early);
            if week0 {
                out.push(0);
            }
            out.extend(late);
        }
        out
    }
    fn place(&mut self, a: usize, b: usize, conf: bool, rivalry: Option<usize>, last: usize) -> bool {
        if self.opp[a].contains(&b) {
            return false;
        }
        let order = self.week_order(a, b, conf, last);
        let Some(wk) = order.into_iter().find(|&wk| self.free(a, wk) && self.free(b, wk)) else { return false };
        let date = self.y.cfb_weeks[wk];
        let home_first = self.host_first(a, b);
        self.add(a, b, Some(wk), date, home_first, conf, rivalry);
        true
    }
}

/// Fills a conference to `g` games each: a random graph where every member
/// has `g` conference opponents (one short when the count is odd), keeping
/// the rivalries already placed.
fn fill_conference(members: &[usize], g: u32, opp: &[HashSet<usize>], seed: u64) -> Vec<(usize, usize)> {
    let set: HashSet<usize> = members.iter().copied().collect();
    let mut deg: HashMap<usize, u32> =
        members.iter().map(|&i| (i, opp[i].iter().filter(|o| set.contains(o)).count() as u32)).collect();
    let mut adj: HashMap<usize, HashSet<usize>> =
        members.iter().map(|&i| (i, opp[i].iter().copied().filter(|o| set.contains(o)).collect())).collect();
    let mut pairs: Vec<(usize, usize)> = Vec::new();
    for (k, &a) in members.iter().enumerate() {
        for &b in &members[k + 1..] {
            if !adj[&a].contains(&b) {
                pairs.push((a, b));
            }
        }
    }
    pairs.sort_by(|x, y| {
        key_uniform(seed, (x.0 * 4099 + x.1) as u64).total_cmp(&key_uniform(seed, (y.0 * 4099 + y.1) as u64))
    });
    let mut new: Vec<(usize, usize)> = Vec::new();
    let link = |a: usize, b: usize, adj: &mut HashMap<usize, HashSet<usize>>, deg: &mut HashMap<usize, u32>| {
        adj.get_mut(&a).unwrap().insert(b);
        adj.get_mut(&b).unwrap().insert(a);
        *deg.get_mut(&a).unwrap() += 1;
        *deg.get_mut(&b).unwrap() += 1;
    };
    for (a, b) in pairs {
        if deg[&a] < g && deg[&b] < g {
            link(a, b, &mut adj, &mut deg);
            new.push((a, b));
        }
    }
    // Repairs: swap an existing pairing to reach the members still short.
    for _ in 0..400 {
        let short: Vec<usize> = members.iter().copied().filter(|i| deg[i] < g).collect();
        if short.is_empty() {
            break;
        }
        let u = short[0];
        if let Some(&v) = short.iter().skip(1).find(|&&v| !adj[&u].contains(&v)) {
            link(u, v, &mut adj, &mut deg);
            new.push((u, v));
            continue;
        }
        let v = short.get(1).copied();
        let pick = new.iter().position(|&(a, b)| {
            let ok = |x: usize, y: usize| match v {
                Some(v) if g - deg[&u] < 2 => {
                    x != u && y != v && x != v && y != u && !adj[&u].contains(&x) && !adj[&v].contains(&y)
                }
                _ => x != u && y != u && !adj[&u].contains(&x) && !adj[&u].contains(&y),
            };
            ok(a, b) || ok(b, a)
        });
        let Some(k) = pick else { break };
        let (a, b) = new.swap_remove(k);
        adj.get_mut(&a).unwrap().remove(&b);
        adj.get_mut(&b).unwrap().remove(&a);
        *deg.get_mut(&a).unwrap() -= 1;
        *deg.get_mut(&b).unwrap() -= 1;
        let (x, y) = match v {
            Some(v) if g - deg[&u] < 2 => {
                if !adj[&u].contains(&a) && !adj[&v].contains(&b) && a != v && b != u {
                    (a, b)
                } else {
                    (b, a)
                }
            }
            _ => (a, b),
        };
        match v {
            Some(v) if g - deg[&u] < 2 => {
                link(u, x, &mut adj, &mut deg);
                link(v, y, &mut adj, &mut deg);
                new.push((u, x));
                new.push((v, y));
            }
            _ => {
                link(u, x, &mut adj, &mut deg);
                link(u, y, &mut adj, &mut deg);
                new.push((u, x));
                new.push((u, y));
            }
        }
    }
    new
}

fn independent(conf: &str) -> bool {
    conf.contains("Independent")
}

/// Builds a season's regular-season schedule for every division.
pub(crate) fn build(w: &World, season: i32, y: &YearDates) -> Vec<Game> {
    let np = w.programs.len();
    let nw = y.cfb_weeks.len();
    let seed = w.options.seed as u64 ^ ((season as u64) << 24) ^ 0x5C4E_D01E;
    let mut b = Builder {
        w,
        y,
        season,
        seed,
        busy: vec![vec![false; nw]; np],
        opp: vec![HashSet::new(); np],
        count: vec![0; np],
        home: vec![0; np],
        games: Vec::new(),
    };
    let by_name: HashMap<&str, usize> = w.programs.iter().map(|p| (p.real_name.as_str(), p.id)).collect();

    // --- Rivalries in their slots (A28) ----------------------------------------------
    let mut rivalry_week_taken = vec![false; np];
    let mut flex: Vec<(usize, usize, usize)> = Vec::new();
    for (k, r) in RIVALRIES.iter().enumerate() {
        let (Some(&a), Some(&c)) = (by_name.get(r.a), by_name.get(r.b)) else { continue };
        let (pa, pc) = (&w.programs[a], &w.programs[c]);
        if pa.division != pc.division || !pa.plays(season) || !pc.plays(season) {
            continue;
        }
        // Only in its real years: starts, pauses and ends (part B 28).
        if super::data::rivalry_past(r.a, r.b).is_some_and(|h| !h.played(season)) {
            continue;
        }
        let last = last_week(pa.division, y);
        let same_conf =
            pa.conference == pc.conference && !independent(&pa.conference) && r.slot != RivalrySlot::ArmyNavy;
        // Home alternates by year; a neutral site has none.
        let home_first = season % 2 == 0;
        match r.slot {
            RivalrySlot::ArmyNavy => {
                if !b.opp[a].contains(&c) {
                    b.add(a, c, None, y.army_navy, home_first, false, Some(k));
                }
            }
            RivalrySlot::RivalryWeek => {
                if rivalry_week_taken[a] || rivalry_week_taken[c] || !b.free(a, last) || !b.free(c, last) {
                    if same_conf {
                        flex.push((a, c, k));
                    }
                    continue;
                }
                rivalry_week_taken[a] = true;
                rivalry_week_taken[c] = true;
                b.add(a, c, Some(last), y.cfb_weeks[last], home_first, same_conf, Some(k));
            }
            RivalrySlot::Week(n) => {
                let wk = n.min(last);
                if b.opp[a].contains(&c) {
                    continue;
                }
                if b.free(a, wk) && b.free(c, wk) {
                    b.add(a, c, Some(wk), y.cfb_weeks[wk], home_first, same_conf, Some(k));
                } else if same_conf {
                    flex.push((a, c, k));
                }
            }
        }
    }
    for (a, c, k) in flex {
        let last = last_week(w.programs[a].division, y);
        b.place(a, c, true, Some(k), last);
    }
    // --- Conference games -------------------------------------------------------------
    // The leagues set their schedules first, around the rivalries: a
    // league schedule is never short for a member's contracts (update 2,
    // lane 6, R7; the contracts going first left about one FBS team in
    // twelve one or two league games short).
    for division in Division::ALL {
        let last = last_week(division, y);
        let mut confs: BTreeMap<&str, Vec<usize>> = BTreeMap::new();
        for p in w.programs.iter().filter(|p| p.division == division && p.plays(season)) {
            confs.entry(p.conference.as_str()).or_default().push(p.id);
        }
        for (name, list) in confs.iter() {
            if independent(name) || list.len() < 2 {
                continue;
            }
            let n = list.len() as u32;
            let g = if division == Division::FBS { conference_rules(name, season).0 } else { (n - 1).min(8) };
            let g = g.min(n - 1);
            let cseed = seed ^ (name.len() as u64 * 131 + name.bytes().map(|x| x as u64).sum::<u64>());
            let mut edges = fill_conference(list, g, &b.opp, cseed);
            edges.sort_by(|x, y| {
                key_uniform(cseed ^ 3, (x.0 * 4099 + x.1) as u64)
                    .total_cmp(&key_uniform(cseed ^ 3, (y.0 * 4099 + y.1) as u64))
            });
            for (a, c) in edges {
                b.place(a, c, true, None, last);
            }
        }
    }
    // A contract or booking goes in only with room left: a season never
    // runs past its games (a school that changed conferences since signing
    // plays its new league's schedule and drops what doesn't fit).
    let room = |b: &Builder, x: usize| -> bool { b.count[x] < games_target(w.programs[x].division, season) };
    // Non-conference games you booked years ahead (update 2, L10); a
    // neutral-site one is a contract below.
    for (home, away) in super::legacy::college::booked_games(w, season) {
        if home >= np || away >= np || b.opp[home].contains(&away) {
            continue;
        }
        if !w.programs[home].plays(season) || !w.programs[away].plays(season) {
            continue;
        }
        if !room(&b, home) || !room(&b, away) {
            continue;
        }
        let last = last_week(w.programs[home].division, y).min(last_week(w.programs[away].division, y));
        let order = b.week_order(home, away, false, last);
        let wk = order.into_iter().find(|&wk| b.free(home, wk) && b.free(away, wk)).or_else(|| b.make_room(home, away));
        if let Some(wk) = wk {
            b.add(home, away, Some(wk), y.cfb_weeks[wk], true, false, None);
        }
    }
    // The world's non-conference contracts (update 2, lane 6): signed years
    // ago, played now. A neutral-site kickoff game opens the season; the
    // rest go where both have the date.
    let mut deals: Vec<&super::deals::Deal> = super::deals::for_season(w, season).collect();
    deals.sort_by_key(|d| (d.kind != super::deals::DealKind::Neutral, d.home, d.away));
    for d in deals {
        let (home, away) = (d.home, d.away);
        if home >= np || away >= np || b.opp[home].contains(&away) {
            continue;
        }
        let (ph, pa) = (&w.programs[home], &w.programs[away]);
        if !ph.plays(season) || !pa.plays(season) {
            continue;
        }
        if !room(&b, home) || !room(&b, away) {
            continue;
        }
        let last = last_week(ph.division, y).min(last_week(pa.division, y));
        let order = match d.site {
            Some(_) => vec![1, 0, 2],
            None => b.week_order(home, away, false, last),
        };
        // A contract is played: with no week free for both, a league game in
        // the way moves to a week its own sides have open.
        let wk = order
            .into_iter()
            .find(|&wk| wk < nw && b.free(home, wk) && b.free(away, wk))
            .or_else(|| b.make_room(home, away));
        if let Some(wk) = wk {
            b.add(home, away, Some(wk), y.cfb_weeks[wk], true, false, None);
            if let Some(s) = d.site.and_then(|k| super::deals::SITES.get(k as usize)) {
                let g = b.games.last_mut().unwrap();
                g.site = Some(Site { state: geo::state_index(s.state).unwrap_or(0), town: s.town, indoor: s.indoor });
            }
        }
    }

    for division in Division::ALL {
        let last = last_week(division, y);
        let target = games_target(division, season);
        let members: Vec<usize> =
            w.programs.iter().filter(|p| p.division == division && p.plays(season)).map(|p| p.id).collect();
        // --- FBS buy games: most FBS teams host one FCS opponent ------------------------
        if division == Division::FBS {
            let fcs: Vec<usize> =
                w.programs.iter().filter(|p| p.division == Division::FCS && p.plays(season)).map(|p| p.id).collect();
            let fcs_target = games_target(Division::FCS, season);
            // The FCS schools' league games are already in, so a buy game
            // never pushes one past 12 games (one FBS opponent each, rarely two).
            let mut bought = vec![0u32; np];
            for &i in &members {
                if b.count[i] >= target || b.key(i as u64 ^ 0xFC5, season as u64) >= 0.85 {
                    continue;
                }
                // Already hosting an FCS school under contract.
                if b.opp[i].iter().any(|&j| w.programs[j].division == Division::FCS) {
                    continue;
                }
                let mut cands: Vec<(f64, usize)> = fcs
                    .iter()
                    .copied()
                    .filter(|&j| bought[j] == 0 && b.count[j] < fcs_target + 1)
                    .map(|j| {
                        let miles = geo::miles(w.programs[i].state, w.programs[j].state);
                        (miles + 900.0 * b.key(i as u64 * 31 + j as u64, season as u64), j)
                    })
                    .collect();
                cands.sort_by(|x, y| x.0.total_cmp(&y.0));
                for (_, j) in cands.into_iter().take(12) {
                    let order = b.week_order(i, j, false, last);
                    if let Some(wk) = order.into_iter().find(|&wk| b.free(i, wk) && b.free(j, wk)) {
                        let date = y.cfb_weeks[wk];
                        b.add(i, j, Some(wk), date, true, false, None);
                        bought[j] += 1;
                        break;
                    }
                }
            }
        }
        // --- Non-conference games, then a last fill ---------------------------------------
        for pass in 0..2 {
            let mut order: Vec<usize> = members.iter().copied().filter(|&i| b.count[i] < target).collect();
            order.sort_by(|&x, &y| b.key(x as u64 ^ pass, 11).total_cmp(&b.key(y as u64 ^ pass, 11)));
            for i in order {
                while b.count[i] < target {
                    let mut cands: Vec<(f64, usize)> = members
                        .iter()
                        .copied()
                        .filter(|&j| {
                            j != i
                                && b.count[j] < target
                                && !b.opp[i].contains(&j)
                                && (w.programs[i].conference != w.programs[j].conference
                                    || independent(&w.programs[i].conference)
                                    || pass == 1)
                        })
                        .map(|j| {
                            let miles = geo::miles(w.programs[i].state, w.programs[j].state);
                            (miles + 1500.0 * b.key(i as u64 * 131 + j as u64, season as u64 ^ pass), j)
                        })
                        .collect();
                    cands.sort_by(|x, y| x.0.total_cmp(&y.0));
                    let mut placed = false;
                    for (_, j) in cands.into_iter().take(if pass == 0 { 25 } else { usize::MAX }) {
                        if b.place(i, j, false, None, last) {
                            placed = true;
                            break;
                        }
                    }
                    if !placed {
                        break;
                    }
                }
            }
        }
    }

    let mut games = b.games;
    weeknights(&mut games, w, season, y, seed);
    // --- Senior day and homecoming (A27) ------------------------------------------------
    games.sort_by(|x, y| x.date.cmp(&y.date).then(x.home.cmp(&y.home)));
    for p in 0..np {
        let home: Vec<usize> = (0..games.len()).filter(|&k| games[k].home == p && games[k].site.is_none()).collect();
        if let Some(&k) = home.last() {
            games[k].senior_day = true;
        }
        // Homecoming: an October home game, usually against a beatable opponent.
        let oct: Vec<usize> = home.iter().copied().filter(|&k| games[k].date.month() == 10).collect();
        let pick = oct
            .iter()
            .copied()
            .min_by(|&x, &z| w.programs[games[x].away].prestige.total_cmp(&w.programs[games[z].away].prestige))
            .or_else(|| home.get(home.len() / 2).copied());
        if let Some(k) = pick {
            games[k].homecoming = true;
        }
    }
    games
}

/// Rivalry-week games played on Thanksgiving (true) or the day after
/// (false), by years (part B 31; minor bug 46): the real 2001-2025
/// schedules (`data/real-world/memory_checks.json`, `college_holiday_games`
/// `by_pair`), earlier years from memory, and from 2026 each pair keeps its
/// 2025 day. Years not listed stay on Saturday or the MAC's weeknights.
const HOLIDAY_GAMES: &[(&str, &str, &[(i32, i32, bool)])] = &[
    ("Texas", "Texas A&M", &[(1984, 2000, true), (2001, 2007, false), (2008, 2011, true), (2025, 9999, false)]),
    (
        "Ole Miss",
        "Mississippi State",
        &[
            (2001, 2003, true),
            (2007, 2008, false),
            (2013, 2013, true),
            (2017, 2019, true),
            (2021, 2023, true),
            (2024, 9999, false),
        ],
    ),
    ("Nebraska", "Iowa", &[(2011, 9999, false)]),
    ("Missouri", "Arkansas", &[(2014, 2019, false), (2021, 2023, false)]),
    ("LSU", "Arkansas", &[(1992, 2008, false), (2011, 2013, false)]),
    (
        "Washington",
        "Washington State",
        &[(2012, 2013, false), (2015, 2016, false), (2018, 2019, false), (2021, 2021, false)],
    ),
    (
        "Oregon",
        "Oregon State",
        &[
            (2006, 2006, false),
            (2013, 2013, false),
            (2015, 2015, false),
            (2018, 2018, false),
            (2020, 2020, false),
            (2023, 2023, false),
        ],
    ),
    ("Toledo", "Bowling Green", &[(2001, 2001, false), (2007, 2009, false)]),
    ("UCF", "South Florida", &[(2013, 2014, false), (2015, 2015, true), (2017, 2021, false)]),
    ("Virginia", "Virginia Tech", &[(2014, 2014, false), (2017, 2019, false)]),
    ("Pittsburgh", "West Virginia", &[(2000, 2000, false), (2004, 2005, true), (2008, 2011, false)]),
];

/// College games off Saturday (part B 31). Proposed, not confirmed, sized
/// from memory of the TV schedules: FBS Thursday nights from 1990 (one a
/// week, two from 2000, three from 2010) and Friday nights from 1995 (one
/// a week, three from 2005), more on opening weekend; a Labor Day Monday
/// game from 1997; the MAC's November weeknights from 2006; Thanksgiving
/// and Black Friday games in rivalry week. A team moved to a weeknight had
/// a bye before (Thursday and midweek) or played the Saturday before (Friday).
fn weeknights(games: &mut [Game], w: &World, season: i32, y: &YearDates, seed: u64) {
    let key = |a: u64, b: u64| key_uniform(seed ^ 0x3EEC, a.wrapping_mul(7919) ^ b);
    let thursdays = match season {
        ..=1989 => 0,
        1990..=1999 => 1,
        2000..=2009 => 2,
        _ => 3,
    };
    let fridays = match season {
        ..=1994 => 0,
        1995..=2004 => 1,
        _ => 3,
    };
    let last = last_week(Division::FBS, y);
    let night = |w: &World, home: usize| {
        if et_offset(&w.states[w.programs[home].state].code) >= 2 { (22, 30) } else { (19, 30) }
    };
    // Each team's game dates, kept current as games move.
    let mut dates: HashMap<usize, Vec<Date>> = HashMap::new();
    for g in games.iter() {
        dates.entry(g.home).or_default().push(g.date);
        dates.entry(g.away).or_default().push(g.date);
    }
    // At least `days` between the moved game and each of the team's other games.
    let rested = |dates: &HashMap<usize, Vec<Date>>, t: usize, this: Date, to: Date, days: i32| {
        dates[&t].iter().filter(|&&d| d != this).all(|&d| d.days_to(to).abs() >= days)
    };
    for wk in 0..=last {
        let sat = y.cfb_weeks[wk];
        let mut list: Vec<usize> = (0..games.len())
            .filter(|&k| {
                let g = &games[k];
                g.division == Division::FBS && g.kind == GameKind::Regular && g.date == sat && g.site.is_none()
            })
            .collect();
        list.sort_by(|&a, &b| key(a as u64, wk as u64).total_cmp(&key(b as u64, wk as u64)));
        let mut moves: Vec<(usize, Date, (u16, u16))> = Vec::new();
        let opening = wk <= 1;
        let rivalry_week = wk == last;
        // Thanksgiving and Black Friday.
        if rivalry_week && season >= 1990 {
            for &k in &list {
                let g = &games[k];
                let (h, a) = (w.programs[g.home].real_name.as_str(), w.programs[g.away].real_name.as_str());
                let hit = HOLIDAY_GAMES
                    .iter()
                    .filter(|x| (x.0 == h && x.1 == a) || (x.0 == a && x.1 == h))
                    .find_map(|x| x.2.iter().find(|r| (r.0..=r.1).contains(&season)).map(|r| r.2));
                if let Some(thanksgiving) = hit {
                    let day = if thanksgiving { y.thanksgiving } else { y.thanksgiving.add(1) };
                    let ko = if thanksgiving { night(w, g.home) } else { (12, 0) };
                    moves.push((k, day, ko));
                }
            }
        }
        let mut n_thu = if opening { 3 * thursdays } else { thursdays };
        let mut n_fri = if opening { 2 * fridays } else { fridays };
        // A Labor Day Monday night game on opening weekend.
        let mut monday = season >= 1997 && sat.add(2) == y.labor_day;
        let mac_midweek = season >= 2006 && sat.month() == 11;
        for &k in &list {
            if moves.iter().any(|m| m.0 == k) {
                continue;
            }
            let g = &games[k];
            if g.rivalry.is_some() && rivalry_week {
                continue;
            }
            let (h, a) = (g.home, g.away);
            if mac_midweek && g.conference && w.programs[h].conference == "MAC" {
                let day = sat.add(if key(k as u64, 0x3ED) < 0.5 { -4 } else { -3 });
                if rested(&dates, h, sat, day, 6) && rested(&dates, a, sat, day, 6) {
                    moves.push((k, day, (19, 0)));
                    continue;
                }
            }
            if monday && !g.conference {
                moves.push((k, sat.add(2), (20, 0)));
                monday = false;
                continue;
            }
            if n_thu > 0 && rested(&dates, h, sat, sat.add(-2), 9) && rested(&dates, a, sat, sat.add(-2), 9) {
                moves.push((k, sat.add(-2), night(w, h)));
                n_thu -= 1;
                continue;
            }
            if n_fri > 0 && rested(&dates, h, sat, sat.add(-1), 6) && rested(&dates, a, sat, sat.add(-1), 6) {
                moves.push((k, sat.add(-1), night(w, h)));
                n_fri -= 1;
            }
        }
        for (k, day, ko) in moves {
            let g = &mut games[k];
            for t in [g.home, g.away] {
                if let Some(v) = dates.get_mut(&t) {
                    for d in v.iter_mut().filter(|d| **d == g.date) {
                        *d = day;
                    }
                }
            }
            g.date = day;
            g.kickoff = ko;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::WorldOptions;

    #[test]
    fn a_season_schedule_looks_real() {
        // The world opens in 2026 (after its spin-up years), so the programs
        // carry their 2026 divisions and conferences.
        let w = World::empty(WorldOptions {
            scale: 0.01,
            history_years: 0,
            start_year: 2026 + crate::world::SPIN_UP_YEARS as i32,
            ..Default::default()
        });
        assert_eq!(w.year, 2026);
        let y = YearDates::new(2026);
        let games = build(&w, 2026, &y);
        // Programs that dropped football keep their old division but play no one.
        let fbs: Vec<usize> =
            w.programs.iter().filter(|p| p.division == Division::FBS && p.plays(2026)).map(|p| p.id).collect();
        let mut counts: Vec<u32> = fbs.iter().map(|&p| games.iter().filter(|g| g.involves(p)).count() as u32).collect();
        counts.sort();
        let avg = counts.iter().sum::<u32>() as f64 / counts.len() as f64;
        assert!(avg > 11.5, "FBS teams average {avg:.2} games");
        let few: Vec<String> = fbs
            .iter()
            .filter(|&&p| games.iter().filter(|g| g.involves(p)).count() < 10)
            .map(|&p| format!("{} ({})", w.programs[p].real_name, w.programs[p].conference))
            .collect();
        assert!(counts[0] >= 10, "fewest games {}: {few:?}", counts[0]);
        // No team plays twice on one day.
        for &p in &fbs {
            let mut days: Vec<Date> = games.iter().filter(|g| g.involves(p)).map(|g| g.date).collect();
            let n = days.len();
            days.sort();
            days.dedup();
            assert_eq!(days.len(), n);
        }
        // The Iron Bowl in rivalry week, Army-Navy on its own Saturday.
        let id = |n: &str| w.programs.iter().find(|p| p.real_name == n).unwrap().id;
        let iron = games.iter().find(|g| g.involves(id("Alabama")) && g.involves(id("Auburn"))).unwrap();
        assert_eq!(iron.date, Date::ymd(2026, 11, 28));
        let an = games.iter().find(|g| g.involves(id("Army")) && g.involves(id("Navy"))).unwrap();
        assert_eq!(an.date, Date::ymd(2026, 12, 12));
        assert!(an.site.is_some());
        // SEC teams play 9 conference games in 2026.
        let bama = id("Alabama");
        let conf = games.iter().filter(|g| g.involves(bama) && g.conference).count();
        assert!((8..=9).contains(&conf), "Alabama plays {conf} SEC games");
        if std::env::var("AFM_DEBUG").is_ok() {
            for d in Division::ALL {
                let ids: Vec<usize> = w.programs.iter().filter(|p| p.division == d).map(|p| p.id).collect();
                let mut hist = std::collections::BTreeMap::new();
                for &p in &ids {
                    *hist.entry(games.iter().filter(|g| g.involves(p)).count()).or_insert(0) += 1;
                }
                eprintln!("{d:?} games per team {hist:?}");
            }
        }
    }
}
