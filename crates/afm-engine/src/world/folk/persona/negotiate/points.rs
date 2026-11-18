//! True points in a money talk (`docs/design/negotiation-arguments.md`,
//! items 3a and 3c): a stat you state about his season, checked against the
//! world's record of it, sized by how far it sits from the standard at his
//! spot, cut by how much of it his side shows wasn't on him, and kept for
//! the season so the desk that prices him reads it (`pull`).
//!
//! - **His season** is the world's: a pro's last season with games
//!   (`ProSeason.stats`), your own college player's lines in your games
//!   kept whole this season (`World::seat_games`).
//! - **The standard** is his peers' at his spot: every pro at his position
//!   last season, or every man at his position in your games this season
//!   (both sides), with enough of the stat's plays (`Stat::min`).
//! - **On him**: what his side reads for context from the same season
//!   (interceptions against his turnover-worthy throws, a low completion
//!   share against his expected completions, sacks against the pressure he
//!   faced). At least `ON_HIM_MIN`.
//! - **A cherry-pick**: a stat that's bad in a season whose grade says it
//!   was a good one. His side catches it by skill.
//!
//! Every size here is Claude's (proposed, not confirmed).

use super::super::Them;
use super::super::memory::Point;
use crate::game::stats::StatLine;
use crate::positions::Position;
use crate::world::World;
use crate::world::folk::claims::truth::Stated;
use crate::world::folk::{Team, person};

/// One strong true point (one spread under his peers, all on him) takes
/// this share of the way from his ask to his market.
pub const POINT_PULL: f64 = 0.5;
/// The least share of a stat his side can show wasn't on him.
pub const ON_HIM_MIN: f64 = 0.3;
/// A stated number this close to the real one (or this share of it) is
/// true.
pub const SLACK: (f64, f64) = (2.0, 0.15);
/// His season's grade at or above this, with a bad stat, is a cherry-pick
/// (the grade's centre, `StatLine::grade`).
pub const CHERRY_GRADE: f64 = 60.0;
/// His side catches a cherry-pick with this chance plus this much of his
/// agent's skill (0-1) or his decider's football read.
pub const CATCH: (f64, f64) = (0.3, 0.5);
/// A caught cherry-pick costs this much of his side's trust in your word.
pub const CAUGHT_WORD: f32 = 0.05;
/// A trait z score past this makes a man proud (ego) or a professional
/// (professionalism), item 3c.
pub const PROUD: f64 = 0.5;
/// A proud man's trust cost a point, x (1 + ego z), and his lean a point
/// of ego z.
pub const PROUD_TRUST: f64 = 0.02;
pub const PROUD_LEAN: f64 = 0.03;
/// A professional's trust gain when the point was true and his.
pub const STRAIGHT_TRUST: f32 = 0.01;
/// A point is kept this many days (one money season).
pub const KEEP_DAYS: i32 = 200;

/// A stat a point can name: the words that name it, the spots it's about,
/// its value and the plays it's out of, and whether low is good.
pub struct Stat {
    pub key: &'static str,
    pub words: &'static [&'static str],
    pub spots: &'static [Position],
    /// (the count or share you'd say, the rate peers are measured on, the
    /// plays it's out of).
    pub get: fn(&StatLine) -> (f64, f64, f64),
    pub low_good: bool,
    /// The least plays for a line to count as a peer's (or his).
    pub min: f64,
    /// A share said as a percent ("62%").
    pub pct: bool,
    /// The season term his side would want incentives on, earning it back
    /// (`folk::terms::LINES`), and whether it's an "at most" term.
    pub back: (&'static str, bool),
}

use Position as P;

const QB: &[Position] = &[P::QB];
const BALL: &[Position] = &[P::QB, P::RB, P::WR, P::TE];
const HANDS: &[Position] = &[P::RB, P::WR, P::TE];
const LINE: &[Position] = &[P::OT, P::IOL, P::TE];
const RUSH: &[Position] = &[P::EDGE, P::DT, P::LB];
const DEF: &[Position] = &[P::EDGE, P::DT, P::LB, P::CB, P::S];
const COVER: &[Position] = &[P::CB, P::S, P::LB];

fn r(n: f64, d: f64) -> f64 {
    if d > 0.0 { n / d } else { 0.0 }
}

/// The stats, longest words first where two share a word ("sacks allowed"
/// before "sacks").
pub const STATS: &[Stat] = &[
    Stat {
        key: "interceptions",
        words: &["interception", "picks", "picked off", " ints", "int "],
        spots: QB,
        get: |l| (l.int as f64, r(l.int as f64, l.att as f64), l.att as f64),
        low_good: true,
        min: 100.0,
        pct: false,
        back: ("pass_tds", false),
    },
    Stat {
        key: "completion",
        words: &["completion", "completing", "accura"],
        spots: QB,
        get: |l| (100.0 * r(l.cmp as f64, l.att as f64), r(l.cmp as f64, l.att as f64), l.att as f64),
        low_good: false,
        min: 100.0,
        pct: true,
        back: ("completion_pct", false),
    },
    Stat {
        key: "sacks_taken",
        words: &["sacked", "sacks taken", "took sacks", "holding the ball"],
        spots: QB,
        get: |l| (l.sacked as f64, r(l.sacked as f64, l.dropbacks as f64), l.dropbacks as f64),
        low_good: true,
        min: 100.0,
        pct: false,
        back: ("pass_yards", false),
    },
    Stat {
        key: "sacks_allowed",
        words: &["sacks allowed", "allowed sacks", "gave up sacks", "sacks he gave up", "gave up"],
        spots: LINE,
        get: |l| (l.sacks_allowed, r(l.sacks_allowed, l.pb_snaps as f64), l.pb_snaps as f64),
        low_good: true,
        min: 150.0,
        pct: false,
        back: ("sacks_allowed", true),
    },
    Stat {
        key: "pressures_allowed",
        words: &["pressures allowed", "allowed pressure", "pressure he gave up"],
        spots: LINE,
        get: |l| (l.pressures_allowed as f64, r(l.pressures_allowed as f64, l.pb_snaps as f64), l.pb_snaps as f64),
        low_good: true,
        min: 150.0,
        pct: false,
        back: ("sacks_allowed", true),
    },
    Stat {
        key: "drops",
        words: &["drop"],
        spots: HANDS,
        get: |l| (l.drops as f64, r(l.drops as f64, l.targets as f64), l.targets as f64),
        low_good: true,
        min: 30.0,
        pct: false,
        back: ("catches", false),
    },
    Stat {
        key: "fumbles",
        words: &["fumble", "put the ball on the ground", "ball security"],
        spots: BALL,
        get: |l| {
            let touches = (l.rush + l.rec + l.dropbacks) as f64;
            (l.fumbles as f64, r(l.fumbles as f64, touches), touches)
        },
        low_good: true,
        min: 60.0,
        pct: false,
        back: ("scrimmage_yards", false),
    },
    Stat {
        key: "yards_per_carry",
        words: &["yards a carry", "yards per carry", "per carry", "ypc", "a carry"],
        spots: &[P::RB],
        get: |l| (r(l.rush_yds, l.rush as f64), r(l.rush_yds, l.rush as f64), l.rush as f64),
        low_good: false,
        min: 60.0,
        pct: false,
        back: ("rush_yards", false),
    },
    Stat {
        key: "sacks",
        words: &["sack", "get home", "got home"],
        spots: RUSH,
        get: |l| (l.sacks, r(l.sacks, l.rush_snaps as f64), l.rush_snaps as f64),
        low_good: false,
        min: 120.0,
        pct: false,
        back: ("sacks", false),
    },
    Stat {
        key: "pressures",
        words: &["pressure"],
        spots: RUSH,
        get: |l| (l.pressures as f64, r(l.pressures as f64, l.rush_snaps as f64), l.rush_snaps as f64),
        low_good: false,
        min: 120.0,
        pct: false,
        back: ("pressures", false),
    },
    Stat {
        key: "missed_tackles",
        words: &["missed tackle", "miss tackles", "missing tackles", "whiff"],
        spots: DEF,
        get: |l| {
            let tries = (l.tackles + l.assists + l.missed) as f64;
            (l.missed as f64, r(l.missed as f64, tries), tries)
        },
        low_good: true,
        min: 25.0,
        pct: false,
        back: ("tackles", false),
    },
    Stat {
        key: "catches_allowed",
        words: &["catches allowed", "allowed catches", "completions allowed", "gave up catches", "got beat", "beaten"],
        spots: COVER,
        get: |l| (l.cmp_allowed as f64, r(l.cmp_allowed as f64, l.tgt_allowed as f64), l.tgt_allowed as f64),
        low_good: true,
        min: 25.0,
        pct: false,
        back: ("pbus", false),
    },
    Stat {
        key: "field_goals",
        words: &["field goal", "kicks", "missed kick"],
        spots: &[P::K],
        get: |l| (100.0 * l.fg_pct(), l.fg_pct(), l.fg_att.iter().sum::<u32>() as f64),
        low_good: false,
        min: 12.0,
        pct: true,
        back: ("fg_pct", false),
    },
];

/// The stat a point's words name for his spot.
pub fn stat_for(pos: Position, words: &str) -> Option<&'static Stat> {
    let v = format!(" {} ", words.to_lowercase());
    STATS.iter().find(|s| s.spots.contains(&pos) && s.words.iter().any(|x| v.contains(x)))
}

/// What a point came to, read from the world (engine numbers, never shown).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Read {
    pub key: &'static str,
    /// What he really put up (the count or share you'd say).
    pub real: f64,
    /// The words match it.
    pub true_: bool,
    /// How far under his peers it sits, in their spread, 0-1 (0 when it's
    /// at or better than the standard).
    pub bad: f64,
    /// The share of it his side can't show away, `ON_HIM_MIN` to 1.
    pub on_him: f64,
    /// His season's grade (the grade's centre is 60).
    pub grade: f64,
    /// A bad stat in a good season.
    pub cherry: bool,
    /// What his side says back, when it says anything.
    pub back: Option<&'static str>,
}

/// The level his money season is read at, and his line.
fn season_line(w: &World, i: usize) -> Option<StatLine> {
    let p = &w.people[i];
    if let Some(c) = p.pro.as_ref() {
        let s = c.seasons.iter().rev().find(|s| s.games >= 4)?;
        return Some(s.stats.clone());
    }
    let mut line = StatLine::default();
    let id = p.player.id;
    let mut any = false;
    for g in season_games(w) {
        for side in &g.stats.players {
            if let Some(l) = side.get(&id) {
                line.add(l);
                any = true;
            }
        }
    }
    any.then_some(line)
}

/// Your college games this season (or last, before this one's first game).
fn season_games(w: &World) -> impl Iterator<Item = &crate::world::seat::SeatGame> {
    let college = |g: &&crate::world::seat::SeatGame| g.level == crate::calendar::Level::College;
    let start = w.dates.fall_camp;
    let any_now = w.seat_games.iter().any(|g| college(&g) && g.date >= start);
    let from = if any_now {
        start
    } else {
        w.seat_games.iter().filter(college).map(|g| g.date).max().map_or(start, |d| d.add(-200))
    };
    w.seat_games.iter().filter(move |g| college(g) && g.date >= from)
}

/// His peers' rates for a stat: every pro at his position last season, or
/// every man at his position in your college games.
fn peers(w: &World, i: usize, s: &Stat) -> Vec<f64> {
    let p = &w.people[i];
    let pos = p.player.position;
    let mut v = Vec::new();
    if let Some(year) = p.pro.as_ref().and_then(|c| c.seasons.iter().rev().find(|x| x.games >= 4)).map(|x| x.year) {
        for q in w.people.iter().filter(|q| q.player.position == pos) {
            if let Some(x) = q.pro.as_ref().and_then(|c| c.seasons.iter().rev().find(|x| x.year == year)) {
                let (_, rate, n) = (s.get)(&x.stats);
                if n >= s.min {
                    v.push(rate);
                }
            }
        }
    } else {
        // Your games: your own men over the season, each opponent over the
        // game or two he played you.
        let mut lines: std::collections::HashMap<u32, (StatLine, u32)> = std::collections::HashMap::new();
        for g in season_games(w) {
            for side in &g.stats.players {
                for (id, l) in side {
                    let e = lines.entry(*id).or_default();
                    e.0.add(l);
                    e.1 += 1;
                }
            }
        }
        for (id, (l, games)) in &lines {
            let at = person(w, *id).is_some_and(|j| w.people[j].player.position == pos);
            let (_, rate, n) = (s.get)(l);
            // A college season is a dozen games: half a season's plays,
            // pro rata for the games he played you.
            if at && n >= 0.5 * s.min * (*games as f64 / 12.0).min(1.0) {
                v.push(rate);
            }
        }
    }
    v.sort_by(f64::total_cmp);
    v
}

/// The median and a spread (the middle half's width over 1.35) of peers.
fn centre(v: &[f64]) -> Option<(f64, f64)> {
    if v.len() < 5 {
        return None;
    }
    let q = |x: f64| v[((v.len() - 1) as f64 * x).round() as usize];
    Some((q(0.5), ((q(0.75) - q(0.25)) / 1.35).max(1e-6)))
}

/// What his side reads as not on him.
fn on_him(s: &Stat, l: &StatLine) -> (f64, Option<&'static str>) {
    let (share, back) = match s.key {
        // A pick off a good throw (tipped, a receiver's slip) isn't his.
        "interceptions" if l.int > 0 => {
            (l.turnover_worthy as f64 / l.int as f64, Some("a lot of those came off throws that weren't bad decisions"))
        }
        // Under his expected completions, the misses are his; at them, his
        // receivers' and the throws he was asked to make.
        "completion" if l.att > 0 => {
            let under = (l.x_cmp - l.cmp as f64) / (0.06 * l.att as f64);
            (under, Some("look at what he was asked to throw, and who dropped them"))
        }
        // Sacks with men in his face are his line's.
        "sacks_taken" if l.dropbacks > 0 => {
            let pressured = r(l.pressured as f64, l.dropbacks as f64);
            (1.0 - 1.5 * pressured, Some("he was under pressure all year"))
        }
        _ => (1.0, None),
    };
    let share = share.clamp(ON_HIM_MIN, 1.0);
    (share, if share < 0.8 { back } else { None })
}

/// Reads a stated stat about him against his season. None when the words
/// name no stat at his spot, his season has too few plays, or there are
/// too few peers to set a standard.
pub fn read(w: &World, i: usize, stated: &Stated) -> Option<Read> {
    let pos = w.people[i].player.position;
    let s = stat_for(pos, &stated.value)?;
    let line = season_line(w, i)?;
    let (real, rate, n) = (s.get)(&line);
    if n < 0.5 * s.min {
        return None;
    }
    let (mid, spread) = centre(&peers(w, i, s))?;
    let said = crate::world::folk::claims::check::recruit::number(&stated.value);
    let true_ = said.is_none_or(|x| {
        let x = if s.pct && x <= 1.0 { 100.0 * x } else { x };
        (x - real).abs() <= SLACK.0.max(SLACK.1 * real.abs())
    });
    let z = if s.low_good { (rate - mid) / spread } else { (mid - rate) / spread };
    let bad = z.clamp(0.0, 1.0);
    let (share, back) = on_him(s, &line);
    let grade = line.grade();
    Some(Read {
        key: s.key,
        real,
        true_,
        bad,
        on_him: share,
        grade,
        cherry: bad > 0.0 && line.graded >= 50 && grade >= CHERRY_GRADE,
        back,
    })
}

/// The share of the way from his ask to his number the points kept for
/// him this season come to, 0-1.
pub fn pull(w: &World, id: u32) -> f64 {
    let today = w.today;
    w.folk
        .live
        .persona
        .points
        .iter()
        .filter(|p| p.with == Them::Player(id) && p.day.days_to(today) < KEEP_DAYS)
        .map(|p| p.pull as f64)
        .sum::<f64>()
        .min(1.0)
}

/// Whether a performance point landed for him this season (a prove-it
/// deal's start), and the stat it named.
pub fn named(w: &World, id: u32) -> Option<&'static Stat> {
    let today = w.today;
    w.folk
        .live
        .persona
        .points
        .iter()
        .filter(|p| p.with == Them::Player(id) && p.day.days_to(today) < KEEP_DAYS && p.pull > 0.0)
        .max_by(|a, b| a.pull.total_cmp(&b.pull))
        .and_then(|p| STATS.iter().find(|s| s.key == p.key))
}

/// Who answers for him in a money talk: a pro's agent when he has one,
/// else himself.
pub fn side(w: &World, i: usize) -> Them {
    w.people[i].pro.as_ref().and_then(|c| c.agent).map_or(Them::Player(w.people[i].player.id), Them::Agent)
}

/// How well his side reads a season for context (0-1): an agent's skill
/// (1-20, read over 20);
/// a college player answering for himself, a middling read. Proposed, not
/// confirmed.
pub fn skill(w: &World, i: usize, speaker: Them) -> f64 {
    match speaker {
        Them::Agent(a) => w.market.agents.get(a).map_or(0.5, |x| (x.skill / 20.0).clamp(0.0, 1.0)),
        _ => {
            let _ = i;
            0.5
        }
    }
}

/// Whether his side catches a cherry-pick: `CATCH`'s chance with his
/// side's skill, drawn once a day for him (the same in the turn and its
/// apply).
pub fn caught(w: &World, i: usize, skill: f64) -> bool {
    let id = w.people[i].player.id as u64;
    crate::world::season::key_uniform(w.options.seed as u64 ^ 0xC4E7, id ^ ((w.today.0 as u64) << 32))
        < CATCH.0 + CATCH.1 * skill.clamp(0.0, 1.0)
}

/// What a point would add to his pull, without keeping it: nothing when
/// it's false or he heard that stat already this season, nothing when a
/// cherry-pick is caught and half when it isn't. Read-only, so the line
/// that makes the point can count it for its own offer (`Offer::argued`).
pub fn adds(w: &World, i: usize, rd: &Read, skill: f64) -> f64 {
    let him = Them::Player(w.people[i].player.id);
    let today = w.today;
    let heard =
        w.folk.live.persona.points.iter().any(|p| p.with == him && p.key == rd.key && p.day.days_to(today) < KEEP_DAYS);
    if !rd.true_ || heard {
        return 0.0;
    }
    let pull = POINT_PULL * rd.bad * rd.on_him;
    match (rd.cherry, rd.cherry && caught(w, i, skill)) {
        (true, true) => 0.0,
        (true, false) => 0.5 * pull,
        _ => pull,
    }
}

/// What his side does with a point: a pull kept for the season, or a
/// cherry-pick caught (your word with his side falls). `skill` is his
/// side's read (`skill`). Returns the pull it added.
pub fn land(w: &mut World, i: usize, speaker: Them, rd: &Read, skill: f64) -> f64 {
    let id = w.people[i].player.id;
    let today = w.today;
    let him = Them::Player(id);
    let pull = adds(w, i, rd, skill);
    let heard = w.folk.live.persona.points.iter().any(|p| p.with == him && p.key == rd.key);
    if !rd.true_ || heard {
        return 0.0;
    }
    if rd.cherry && caught(w, i, skill) {
        let key = crate::world::folk::persona::makeup::key(speaker);
        let word = w.folk.live.word_of(key);
        w.folk.live.set_word(key, word - CAUGHT_WORD);
    }
    let m = &mut w.folk.live.persona;
    m.points.retain(|p| p.day.days_to(today) < KEEP_DAYS);
    m.points.push(Point { with: him, day: today, key: rd.key.into(), pull: pull as f32 });
    pull
}

/// His makeup's answer to a performance point that landed (item 3c): a
/// change to his trust in you, and to his lean to leave (college: his
/// portal lean). Proud (ego z over `PROUD`): 0.02 x (1 + ego z) off his
/// trust, x1.5 when his side showed less than half of it was on him, and
/// 0.03 x ego z on his lean. Professional (professionalism z over
/// `PROUD`): nothing when the blame was his, and 0.01 more trust for being
/// straight with him. Anyone else: half the proud man's cost.
pub fn makeup_cost(w: &World, i: usize, rd: &Read) -> (f32, f64) {
    use crate::player::Trait;
    let t = w.people[i].player.personality.traits;
    let (ego, pro) = (t.z(Trait::Ego), t.z(Trait::Professionalism));
    let not_his = if rd.on_him < 0.5 { 1.5 } else { 1.0 };
    if ego > PROUD {
        (-(PROUD_TRUST * (1.0 + ego) * not_his) as f32, PROUD_LEAN * ego)
    } else if pro > PROUD {
        (if rd.on_him >= 0.5 { STRAIGHT_TRUST } else { -(0.5 * PROUD_TRUST * not_his) as f32 }, 0.0)
    } else {
        (-(0.5 * PROUD_TRUST * (1.0 + ego.max(0.0)) * not_his) as f32, 0.0)
    }
}

/// The team whose season a point reads: his own today.
pub fn team(w: &World, i: usize) -> Option<Team> {
    crate::world::folk::claims::team_of(w, i)
}

/// How far an "earn it back" clause asks him to move his season on the
/// stat his side would want paid (a prove-it deal, item 3b): his last
/// season's number this share better. Proposed, not confirmed.
pub const EARN_BACK: f64 = 0.15;

/// The season term a prove-it deal pays on: the stat's own line
/// (`Stat::back`) at his last season's number moved `EARN_BACK` better
/// ("at most" for a stat where low is good). None when his season has no
/// line.
pub fn earn_back(w: &World, i: usize, s: &Stat) -> Option<crate::world::folk::usage::Usage> {
    use crate::world::folk::terms;
    let line = season_line(w, i)?;
    let (key, most) = s.back;
    let l = terms::LINES.iter().find(|l| l.key == key)?;
    let now = (l.get)(&line);
    let target = if most { now * (1.0 - EARN_BACK) } else { now * (1.0 + EARN_BACK) };
    let target = if l.rate { target.min(1.0) } else { target.round().max(1.0) };
    terms::read(Some(w.people[i].player.position), key, Some(target as f32), most)
}
