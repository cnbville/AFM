//! What the game boards hold your numbers against (update 2, lane 9):
//! each level's own passing and line numbers this season and last (the
//! NFL, FBS, FCS, the small colleges, and every state's high schools),
//! added up from every game played on the engine; your state's high school
//! rankings week by week; and the readers the boards need for the college
//! polls and the NFL's playoff race. Small: one fixed-size tally per level
//! a season, two seasons.

use crate::calendar::Date;
use crate::game::{GameStats, PlayType};

use super::World;

/// One level's numbers over a season, both sides of every game.
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Tally {
    /// Team games.
    pub games: u32,
    /// Dropbacks (throws, sacks and scrambles), those under pressure, sacks.
    pub dropbacks: u32,
    pub pressured: u32,
    pub sacks: u32,
    /// Throws at a man (not thrown away), their air yards and time to
    /// throw, and how many went each depth: behind the line, 0-4, 5-9,
    /// 10-14, 15-19, 20-29, 30 and over.
    pub throws: u32,
    pub air: f32,
    pub ttt: f32,
    pub depth: [u32; 7],
    /// Designed runs, their yards and yards before contact, the stuffed
    /// (no gain or a loss) and the runs of 10 or more.
    pub runs: u32,
    pub run_yds: f32,
    pub before: f32,
    pub stuffed: u32,
    pub ten: u32,
}

/// The depth bin of a throw `air` yards past the line.
pub fn depth_bin(air: f64) -> usize {
    match air {
        a if a < 0.0 => 0,
        a if a < 5.0 => 1,
        a if a < 10.0 => 2,
        a if a < 15.0 => 3,
        a if a < 20.0 => 4,
        a if a < 30.0 => 5,
        _ => 6,
    }
}

impl Tally {
    /// Adds a game's plays.
    pub fn add_game(&mut self, s: &GameStats) {
        self.games += 2;
        for r in &s.plays {
            if r.nullified || r.offense > 1 {
                continue;
            }
            match r.kind {
                PlayType::Pass | PlayType::Sack | PlayType::Scramble => {
                    self.dropbacks += 1;
                    if r.pressured || r.kind == PlayType::Sack {
                        self.pressured += 1;
                    }
                    if r.kind == PlayType::Sack {
                        self.sacks += 1;
                    }
                    if r.kind == PlayType::Pass && !r.throw_away {
                        self.throws += 1;
                        self.air += r.air_yards as f32;
                        self.ttt += r.time_to_throw as f32;
                        self.depth[depth_bin(r.air_yards)] += 1;
                    }
                }
                PlayType::Run => {
                    self.runs += 1;
                    self.run_yds += r.yards as f32;
                    self.before += r.before_contact as f32;
                    if r.yards <= 0.0 {
                        self.stuffed += 1;
                    }
                    if r.yards >= 10.0 {
                        self.ten += 1;
                    }
                }
                _ => {}
            }
        }
    }

    pub fn add(&mut self, o: &Tally) {
        self.games += o.games;
        self.dropbacks += o.dropbacks;
        self.pressured += o.pressured;
        self.sacks += o.sacks;
        self.throws += o.throws;
        self.air += o.air;
        self.ttt += o.ttt;
        for (a, b) in self.depth.iter_mut().zip(o.depth) {
            *a += b;
        }
        self.runs += o.runs;
        self.run_yds += o.run_yds;
        self.before += o.before;
        self.stuffed += o.stuffed;
        self.ten += o.ten;
    }
}

/// The levels the tallies are kept by.
pub const NFL: u16 = 0;
pub const FBS: u16 = 1;
pub const FCS: u16 = 2;
pub const SMALL_COLLEGE: u16 = 3;
/// A state's high schools: this plus the state's index.
pub const HS: u16 = 10;

/// The key of a college division's tally.
pub fn college_key(d: super::data::Division) -> u16 {
    use super::data::Division;
    match d {
        Division::FBS => FBS,
        Division::FCS => FCS,
        _ => SMALL_COLLEGE,
    }
}

/// One season's tallies.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Season {
    pub year: i32,
    pub tallies: Vec<(u16, Tally)>,
}

/// One school in a state poll: its rank in that list and its record.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PollRow {
    pub school: u32,
    pub rank: u16,
    pub wins: u8,
    pub losses: u8,
}

/// A state poll as it came out.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct StatePoll {
    pub year: i32,
    pub date: Date,
    /// "Preseason", "Weekly", "Final".
    pub which: String,
    /// The state's top 25 in all classes, best first, and your school's
    /// place when it is lower.
    pub top: Vec<PollRow>,
    /// The top 25 in your school's class, and your school.
    pub class: Vec<PollRow>,
    /// The state and your school it was kept for (none in a save from
    /// before they were kept).
    #[serde(default)]
    pub state: Option<u16>,
    #[serde(default)]
    pub school: Option<u32>,
}

/// The boards' store.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Charts {
    /// This season's and last season's tallies.
    #[serde(default)]
    pub seasons: Vec<Season>,
    /// Your state's high school polls, this season and last (a high school
    /// seat only).
    #[serde(default)]
    pub state_polls: Vec<StatePoll>,
}

/// Seasons of tallies kept.
const KEEP: usize = 2;

impl Charts {
    pub fn add(&mut self, year: i32, key: u16, t: &Tally) {
        if !self.seasons.iter().any(|s| s.year == year) {
            self.seasons.push(Season { year, tallies: Vec::new() });
            self.seasons.sort_by_key(|s| std::cmp::Reverse(s.year));
            self.seasons.truncate(KEEP);
        }
        let Some(s) = self.seasons.iter_mut().find(|s| s.year == year) else { return };
        match s.tallies.iter_mut().find(|x| x.0 == key) {
            Some(x) => x.1.add(t),
            None => s.tallies.push((key, *t)),
        }
    }

    /// A level's tally in a season.
    pub fn get(&self, year: i32, key: u16) -> Option<&Tally> {
        self.seasons.iter().find(|s| s.year == year)?.tallies.iter().find(|x| x.0 == key).map(|x| &x.1)
    }
}

/// Puts a finished game on its level's tally.
pub(crate) fn add_game(w: &mut World, year: i32, key: u16, s: &GameStats) {
    let mut t = Tally::default();
    t.add_game(s);
    w.charts.add(year, key, &t);
}

/// A state's poll (`hs_year::poll`), kept when it is your school's state.
pub(crate) fn state_poll(w: &mut World, s: usize, d: Date, which: &str, ranked: &[(f64, usize)]) {
    let Some(seat) = w.seat else { return };
    if seat.level != crate::calendar::Level::HighSchool {
        return;
    }
    let Some(me) = seat.program else { return };
    if w.high_schools.get(me).is_none_or(|h| h.state != s) {
        return;
    }
    let Some(hs) = w.hs_season.as_ref() else { return };
    let class = w.high_schools[me].classification;
    let row = |k: usize, h: usize| {
        let r = &hs.results[h];
        PollRow { school: h as u32, rank: (k + 1) as u16, wins: r.wins.min(255) as u8, losses: r.losses.min(255) as u8 }
    };
    let keep = |list: Vec<usize>| -> Vec<PollRow> {
        list.iter().enumerate().filter(|&(k, &h)| k < 25 || h == me).map(|(k, &h)| row(k, h)).collect()
    };
    let top = keep(ranked.iter().map(|x| x.1).collect());
    let class = keep(ranked.iter().map(|x| x.1).filter(|&h| w.high_schools[h].classification == class).collect());
    let year = w.year;
    let polls = &mut w.charts.state_polls;
    polls.push(StatePoll {
        year,
        date: d,
        which: which.to_string(),
        top,
        class,
        state: Some(s as u16),
        school: Some(me as u32),
    });
    // This season and last.
    polls.retain(|p| p.year >= year - 1);
}

/// A college poll as the boards read it.
#[derive(Clone, Debug, PartialEq)]
pub struct PollWeek {
    pub date: Date,
    /// "AP", "Coaches", "BCS", "CFP" and the like, and whether it is the
    /// preseason or final poll.
    pub kind: &'static str,
    pub key: &'static str,
    pub top: Vec<usize>,
    pub points: Vec<u32>,
    pub firsts: Vec<u16>,
    /// The season's final poll (the coaches' final ballots are public).
    pub fin: bool,
}

/// One team in the NFL's playoff race after a week.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RaceRow {
    pub wins: u32,
    pub losses: u32,
    pub ties: u32,
    /// The seed it would have if the season ended (1-7), else None.
    pub seed: Option<u8>,
    /// Its place in the conference by the seeding order (1-16): the
    /// division leaders, then everyone else by record.
    pub place: u8,
    /// Leads its division.
    pub leader: bool,
}

/// The NFL's playoff race week by week: after each week played, every
/// team's record and seed.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct NflRace {
    pub year: i32,
    /// Conference (0 AFC, 1 NFC) and division (0-7) of each team.
    pub conf: Vec<usize>,
    pub div: Vec<usize>,
    /// After each week with games played: the week (0-based) and the rows.
    pub weeks: Vec<(usize, Vec<RaceRow>)>,
    /// Each week's first game day, and the games still to play: (week,
    /// home, away, date).
    pub dates: Vec<Option<Date>>,
    pub left: Vec<(usize, usize, usize, Option<Date>)>,
    /// Final seeds once the regular season is over.
    pub booked: bool,
}

impl World {
    /// A level's numbers this season, or last season's while this one is
    /// young (under 40 team games in): the season they are from and the
    /// tally.
    pub fn level_tally(&self, key: u16, season: i32) -> Option<(i32, Tally)> {
        let now = self.charts.get(season, key).copied();
        match now {
            Some(t) if t.games >= 40 => Some((season, t)),
            _ => self.charts.get(season - 1, key).map(|t| (season - 1, *t)).or(now.map(|t| (season, t))),
        }
    }

    /// The college polls of this season (or last season's, `prev`), in
    /// the order they came out.
    pub fn poll_weeks(&self, prev: bool) -> Vec<PollWeek> {
        use super::college::PollKind;
        // Last season's polls move apart once the season turns.
        let polls: &[super::college::Poll] = if prev {
            match self.college_prev.as_ref().filter(|c| !c.polls.is_empty()) {
                Some(c) => &c.polls,
                None => &self.prev_polls,
            }
        } else {
            self.college.as_ref().map_or(&[], |c| &c.polls)
        };
        polls
            .iter()
            .map(|p| PollWeek {
                date: p.date,
                kind: p.kind.name(),
                key: poll_key(p.kind),
                top: p.top.clone(),
                points: p.points.clone(),
                firsts: p.firsts.clone(),
                fin: matches!(p.kind, PollKind::Final | PollKind::CoachesFinal),
            })
            .collect()
    }

    /// The NFL's playoff race this season, week by week, by the seeding
    /// rule the season itself uses (division winners first, then the wild
    /// cards, by record, division wins and strength).
    pub fn nfl_race(&self) -> Option<NflRace> {
        self.race_of(self.nfl_season.as_ref()?)
    }

    /// Last NFL season's race, kept once the new league year opens.
    pub fn nfl_race_prev(&self) -> Option<NflRace> {
        self.race_of(self.nfl_prev.as_ref()?)
    }

    fn race_of(&self, s: &super::nfl_year::NflSeason) -> Option<NflRace> {
        let n = s.conf.len();
        let per_conf = (self.rules.nfl_playoff_teams / 2) as usize;
        let mut w = vec![0u32; n];
        let mut l = vec![0u32; n];
        let mut t = vec![0u32; n];
        let mut dw = vec![0u32; n];
        let mut weeks = Vec::new();
        let mut dates = Vec::new();
        let mut left = Vec::new();
        for (k, games) in s.weeks.iter().enumerate() {
            let slots = s.slots.get(k);
            let mut any = false;
            dates.push(slots.and_then(|v| v.iter().map(|x| x.at.date).min()));
            for (g, &(h, a)) in games.iter().enumerate() {
                let Some(x) = slots.and_then(|v| v.get(g)) else { continue };
                let Some([hp, ap]) = x.score.filter(|_| x.played) else {
                    if !x.played {
                        left.push((k, h, a, Some(x.at.date)));
                    }
                    continue;
                };
                any = true;
                if hp == ap {
                    t[h] += 1;
                    t[a] += 1;
                } else {
                    let (win, lose) = if hp > ap { (h, a) } else { (a, h) };
                    w[win] += 1;
                    l[lose] += 1;
                    if s.div[h] == s.div[a] {
                        dw[win] += 1;
                    }
                }
            }
            if !any {
                continue;
            }
            let key = |x: usize| {
                let g = (w[x] + l[x] + t[x]).max(1) as f64;
                (w[x] as f64 + 0.5 * t[x] as f64) / g
                    + 0.001 * dw[x] as f64
                    + 0.0001 * s.z.get(x).copied().unwrap_or(0.0)
            };
            let mut rows: Vec<RaceRow> =
                (0..n).map(|x| RaceRow { wins: w[x], losses: l[x], ties: t[x], ..RaceRow::default() }).collect();
            for c in 0..2 {
                let mut winners: Vec<usize> = (0..4)
                    .filter_map(|dv| {
                        (0..n)
                            .filter(|&x| s.div[x] == c * 4 + dv && s.active.get(x).copied().unwrap_or(true))
                            .max_by(|&a, &b| key(a).total_cmp(&key(b)))
                    })
                    .collect();
                winners.sort_by(|&a, &b| key(b).total_cmp(&key(a)));
                let mut rest: Vec<usize> = (0..n)
                    .filter(|&x| s.conf[x] == c && !winners.contains(&x) && s.active.get(x).copied().unwrap_or(true))
                    .collect();
                rest.sort_by(|&a, &b| key(b).total_cmp(&key(a)));
                for &x in &winners {
                    rows[x].leader = true;
                }
                for (k, x) in winners.into_iter().chain(rest).enumerate() {
                    rows[x].place = k as u8 + 1;
                    if k < per_conf {
                        rows[x].seed = Some(k as u8 + 1);
                    }
                }
            }
            weeks.push((k, rows));
        }
        Some(NflRace { year: s.year, conf: s.conf.clone(), div: s.div.clone(), weeks, dates, left, booked: s.booked })
    }

    /// The NFL's final seeds this season, once the field is set.
    pub fn nfl_seeds(&self) -> Option<Vec<Option<usize>>> {
        self.nfl_season.as_ref().filter(|s| s.booked).map(|s| s.seed.clone())
    }

    /// Your state's high school polls (a high school seat), oldest first.
    pub fn state_polls(&self) -> &[StatePoll] {
        &self.charts.state_polls
    }
}

/// How well two of your players (person indexes) know each other, 0-1:
/// the seasons they've spent on the same team, as the engine's chemistry
/// reads it. None at a seat in another league (no seasons kept there).
pub fn pair_chemistry(w: &World, a: usize, b: usize) -> Option<f32> {
    super::chemistry::pair(w, a, b)
}

/// Whether a college player (a person index) takes a place under his
/// program's limit this season: the House settlement's designated walk-ons
/// and a COVID senior's extra year don't.
pub fn takes_a_place(w: &World, i: usize, whole_roster: bool, season: i32) -> bool {
    super::recruiting::takes_a_place(&w.people[i], whole_roster, season)
}

/// The order of the coming draft's rounds (clubs by pick, before trades
/// and comp picks), once it is set.
pub fn draft_order(w: &World) -> Option<Vec<usize>> {
    super::nfl_year::draft_order(w)
}

/// A poll's short key for the boards' tabs.
fn poll_key(k: super::college::PollKind) -> &'static str {
    use super::college::PollKind as K;
    match k {
        K::ApPreseason | K::Ap | K::Final => "ap",
        K::CoachesPreseason | K::Coaches | K::CoachesFinal => "coaches",
        K::Bcs => "bcs",
        K::Cfp => "cfp",
        K::Harris => "harris",
    }
}
