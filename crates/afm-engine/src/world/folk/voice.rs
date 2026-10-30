//! Lines that never go stale (people topic 5, "How a talk plays on screen"):
//! a big bank of written parts, put together per line (an opening, the
//! point, a detail, a close), each part in many versions; every speaker has
//! a voice (his personality, age, where he grew up, his job and the era
//! pick which versions he can say); the world's facts fill each part (names,
//! records, dates, shared history); and each speaker remembers the parts he
//! has used with you, so he reaches for one he hasn't.
//!
//! The bank lives in `data/lines/*.txt`, written and reviewed during
//! development, and can keep growing:
//!
//! ```text
//! @key talk.player.ask.open
//! Coach, honestly? ~ young
//! [Yes sir|Yeah], I've been thinking about it. ~ !ego
//! Not gonna lie, {name}... ~ ego young
//! ```
//!
//! A line under `@key` is one version of that part. `[a|b]` picks one of
//! its choices, `{fact}` takes a fact (a version whose fact is missing is
//! skipped), and the tags after `~` say who may say it (`!tag` who may not).
//! A tag `fact=value` says when (`result=win`: only after a win;
//! `!result=win`: never after one). `@with other.key` under a key adds every
//! version of that other part to this one (a story's up and down quotes
//! also say its quotes that fit either tone).

use std::collections::HashMap;
use std::sync::OnceLock;

use super::cast::{Cast, Role};
use crate::calendar::Date;
use crate::player::Trait;
use crate::rng::Rng;
use crate::world::World;

const BANKS: &[&str] = &[
    include_str!("../../../data/lines/takeover.txt"),
    include_str!("../../../data/lines/players.txt"),
    include_str!("../../../data/lines/staff.txt"),
    include_str!("../../../data/lines/bosses.txt"),
    include_str!("../../../data/lines/boosters.txt"),
    include_str!("../../../data/lines/press.txt"),
    include_str!("../../../data/lines/room.txt"),
    include_str!("../../../data/lines/network.txt"),
    include_str!("../../../data/lines/life.txt"),
    include_str!("../../../data/lines/storylines.txt"),
    // The bank's growth (dialogue D2): more versions of the same parts.
    include_str!("../../../data/lines/talk_open.txt"),
    include_str!("../../../data/lines/talk_reveal.txt"),
    include_str!("../../../data/lines/talk_reply.txt"),
    include_str!("../../../data/lines/press_more.txt"),
    include_str!("../../../data/lines/story_quotes.txt"),
    include_str!("../../../data/lines/bosses_more.txt"),
    include_str!("../../../data/lines/boosters_more.txt"),
    include_str!("../../../data/lines/room_more.txt"),
    include_str!("../../../data/lines/staff_more.txt"),
    include_str!("../../../data/lines/takeover_more.txt"),
    // Game day talks, home visits, interviews, agents, meetings (lane 14).
    include_str!("../../../data/lines/gameday.txt"),
    include_str!("../../../data/lines/dialogue.txt"),
    // The fans (people topic 9; the fan lane, then 13f).
    include_str!("../../../data/lines/fans.txt"),
];

#[derive(Clone, Debug)]
struct Frag {
    id: u32,
    text: String,
    need: Vec<String>,
    not: Vec<String>,
    facts: Vec<String>,
    /// When it may be said: (fact, value, whether it must match).
    when: Vec<(String, String, bool)>,
}

impl Frag {
    fn fits(&self, who: &Speaker, facts: &Facts) -> bool {
        self.need.iter().all(|t| who.has(t))
            && !self.not.iter().any(|t| who.has(t))
            && self.facts.iter().all(|k| facts.get(k).is_some())
            && self.when.iter().all(|(k, v, eq)| (facts.get(k) == Some(v.as_str())) == *eq)
    }
}

#[derive(Default, Debug)]
pub struct Bank {
    by_key: HashMap<String, Vec<Frag>>,
    count: usize,
    /// Versions written twice under one part (kept once).
    repeats: usize,
}

impl Bank {
    fn parse() -> Bank {
        let mut b = Bank::default();
        let mut key = String::new();
        let mut seen: std::collections::HashSet<(String, u32)> = Default::default();
        let mut with: Vec<(String, String)> = Vec::new();
        for src in BANKS {
            for raw in src.lines() {
                let line = raw.trim();
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }
                if let Some(k) = line.strip_prefix("@key ") {
                    key = k.trim().to_string();
                    continue;
                }
                if let Some(k) = line.strip_prefix("@with ") {
                    with.push((key.clone(), k.trim().to_string()));
                    continue;
                }
                if key.is_empty() {
                    continue;
                }
                let (text, tags) = match line.rsplit_once(" ~ ") {
                    Some((t, g)) => (t.trim(), g.trim()),
                    None => (line, ""),
                };
                let mut need = Vec::new();
                let mut not = Vec::new();
                let mut when = Vec::new();
                for t in tags.split_whitespace() {
                    let (eq, t) = match t.strip_prefix('!') {
                        Some(n) => (false, n),
                        None => (true, t),
                    };
                    if let Some((k, v)) = t.split_once('=') {
                        when.push((k.to_string(), v.to_string(), eq));
                    } else if eq {
                        need.push(t.to_string());
                    } else {
                        not.push(t.to_string());
                    }
                }
                let mut facts = Vec::new();
                let mut rest = text;
                while let Some(a) = rest.find('{') {
                    let Some(z) = rest[a..].find('}') else { break };
                    facts.push(rest[a + 1..a + z].to_string());
                    rest = &rest[a + z + 1..];
                }
                // The id is the version's own (its part and its words), so a
                // speaker's memory still points at the same version after
                // the bank grows, and the same words written for two kinds
                // of voice rest as one. A line written twice is kept once.
                let id = version_id(&key, text);
                if !seen.insert((key.clone(), version_id(&key, line))) {
                    b.repeats += 1;
                    continue;
                }
                b.count += 1;
                b.by_key.entry(key.clone()).or_default().push(Frag {
                    id,
                    text: text.to_string(),
                    need,
                    not,
                    facts,
                    when,
                });
            }
        }
        // A part's borrowed versions keep their own ids, so they rest as
        // one whichever part says them.
        for (to, from) in with {
            let add: Vec<Frag> = b.by_key.get(&from).cloned().unwrap_or_default();
            let here = b.by_key.entry(to).or_default();
            for f in add {
                if !here.iter().any(|x| x.id == f.id && x.need == f.need && x.not == f.not) {
                    here.push(f);
                }
            }
        }
        b
    }

    /// How many written parts the bank holds.
    pub fn size(&self) -> usize {
        self.count
    }

    pub fn has(&self, key: &str) -> bool {
        self.by_key.contains_key(key)
    }

    /// Every version of every part, counted out: how many different lines
    /// one part's alternatives can make.
    pub fn variants(&self) -> u64 {
        let mut n = 0u64;
        for frags in self.by_key.values() {
            for f in frags {
                let mut v = 1u64;
                let mut rest = f.text.as_str();
                while let Some(a) = rest.find('[') {
                    let Some(z) = rest[a..].find(']') else { break };
                    v *= rest[a + 1..a + z].split('|').count() as u64;
                    rest = &rest[a + z + 1..];
                }
                n += v;
            }
        }
        n
    }
}

/// A version's id: FNV-1a over its part's key and its words.
fn version_id(key: &str, line: &str) -> u32 {
    let mut h: u32 = 0x811C_9DC5;
    for b in key.bytes().chain([0u8]).chain(line.bytes()) {
        h ^= b as u32;
        h = h.wrapping_mul(0x0100_0193);
    }
    h
}

impl Bank {
    /// Lines written twice under one part (words and tags).
    pub fn repeats(&self) -> usize {
        self.repeats
    }

    /// How many different wordings the bank holds (the same words written
    /// for two kinds of voice count once).
    pub fn wordings(&self) -> usize {
        self.by_key.values().flatten().map(|x| x.id).collect::<std::collections::HashSet<_>>().len()
    }

    /// Every part's key with how many versions it has.
    pub fn keys(&self) -> Vec<(&str, usize)> {
        let mut v: Vec<(&str, usize)> = self.by_key.iter().map(|(k, f)| (k.as_str(), f.len())).collect();
        v.sort();
        v
    }

    /// The versions of one part as written.
    pub fn versions(&self, key: &str) -> Vec<Version<'_>> {
        self.by_key.get(key).map_or_else(Vec::new, |fr| {
            fr.iter()
                .map(|f| {
                    let when = f.when.iter().map(|w| w.0.as_str()).collect();
                    (f.text.as_str(), f.need.as_slice(), f.not.as_slice(), f.facts.as_slice(), when)
                })
                .collect()
        })
    }
}

/// One version as written: (text, tags who may say it, tags who may not,
/// the facts it needs, the facts it is said on).
pub type Version<'a> = (&'a str, &'a [String], &'a [String], &'a [String], Vec<&'a str>);

pub fn bank() -> &'static Bank {
    static B: OnceLock<Bank> = OnceLock::new();
    B.get_or_init(Bank::parse)
}

/// Who is talking: his memory key and the tags his voice allows.
#[derive(Clone, Debug, Default)]
pub struct Speaker {
    pub key: u64,
    pub tags: Vec<&'static str>,
}

impl Speaker {
    pub fn has(&self, t: &str) -> bool {
        self.tags.contains(&t)
    }

    pub(crate) fn era(tags: &mut Vec<&'static str>, year: i32) {
        tags.push(if year < 2000 {
            "pre2000"
        } else if year < 2010 {
            "pre2010"
        } else {
            "social"
        });
        if year >= 2010 {
            tags.push("modern");
        }
        if year >= 2021 {
            tags.push("nil");
        }
    }

    /// His position, so a line about routes, carries or the pass rush comes
    /// only from a man who has them: his spot (qb, rb, wr, te, ol, dl, lb,
    /// db, kicker, snapper), his side (offense, defense, special) and the
    /// groups lines share: skill (backs, receivers, tight ends), cover
    /// (linebackers and backs), rusher (edges and tackles) and lineman.
    fn position(tags: &mut Vec<&'static str>, pos: crate::positions::Position) {
        use crate::positions::Position as P;
        tags.push(match pos {
            P::QB => "qb",
            P::RB => "rb",
            P::WR => "wr",
            P::TE => "te",
            P::OT | P::IOL => "ol",
            P::EDGE | P::DT => "dl",
            P::LB => "lb",
            P::CB | P::S => "db",
            P::K | P::P => "kicker",
            P::LS => "snapper",
        });
        tags.push(match pos {
            P::QB | P::RB | P::WR | P::TE | P::OT | P::IOL => "offense",
            P::EDGE | P::DT | P::LB | P::CB | P::S => "defense",
            P::K | P::P | P::LS => "special",
        });
        if matches!(pos, P::RB | P::WR | P::TE) {
            tags.push("skill");
        }
        if matches!(pos, P::LB | P::CB | P::S) {
            tags.push("cover");
        }
        if matches!(pos, P::EDGE | P::DT) {
            tags.push("rusher");
        }
        if matches!(pos, P::OT | P::IOL | P::EDGE | P::DT) {
            tags.push("lineman");
        }
    }

    pub(crate) fn region(tags: &mut Vec<&'static str>, w: &World, state: Option<usize>) {
        let Some(s) = state else { return };
        let code = w.states.get(s).map_or("", |x| x.code.as_str());
        tags.push(match code {
            "TX" => "texas",
            "AL" | "GA" | "MS" | "LA" | "SC" | "TN" | "AR" | "FL" | "KY" | "NC" | "VA" | "WV" | "OK" => "south",
            "OH" | "MI" | "IN" | "IL" | "WI" | "MN" | "IA" | "MO" | "NE" | "KS" | "ND" | "SD" => "midwest",
            "NY" | "NJ" | "PA" | "MA" | "CT" | "RI" | "NH" | "VT" | "ME" | "MD" | "DE" | "DC" => "northeast",
            _ => "west",
        });
        if code == "TX" {
            tags.push("south");
        }
    }

    pub fn player(w: &World, i: usize) -> Speaker {
        let p = &w.people[i];
        let t = &p.player.personality.traits;
        let mut tags: Vec<&'static str> = vec!["player"];
        let age = p.player.age;
        tags.push(match p.stage {
            crate::world::Stage::Pro => "nfl",
            crate::world::Stage::College | crate::world::Stage::Declared => "college",
            _ => "hs",
        });
        if age < 23 {
            tags.push("young");
        }
        if age >= 28 {
            tags.push("vet");
        }
        let hi = |tr: Trait| t.get(tr) >= 14.0;
        let lo = |tr: Trait| t.get(tr) <= 7.0;
        if hi(Trait::Ego) {
            tags.push("ego");
        }
        if lo(Trait::Ego) {
            tags.push("humble");
        }
        if hi(Trait::Leadership) {
            tags.push("leader");
        }
        if t.get(Trait::Leadership) <= 8.0 && t.get(Trait::Ego) <= 11.0 {
            tags.push("quiet");
        }
        if hi(Trait::Temper) {
            tags.push("hot");
        }
        if lo(Trait::Temper) {
            tags.push("calm");
        }
        if hi(Trait::Loyalty) {
            tags.push("loyal");
        }
        if hi(Trait::Greed) {
            tags.push("greedy");
        }
        if hi(Trait::Professionalism) {
            tags.push("pro");
        }
        if lo(Trait::Professionalism) {
            tags.push("loose");
        }
        if p.bg.stability <= 7.0 {
            tags.push("shaky");
        }
        if p.bg.stability >= 14.0 {
            tags.push("steady");
        }
        if p.player.personality.competitiveness >= 14.0 {
            tags.push("competitor");
        }
        Self::position(&mut tags, p.player.position);
        // Where he is in his career, so a line about a senior year, a
        // redshirt or a rookie deal comes only from a man who has one.
        match p.stage {
            crate::world::Stage::Pro => {
                if let Some(c) = &p.pro {
                    if c.accrued == 0 {
                        tags.push("rookie");
                    }
                    if c.draft.is_none() {
                        tags.push("undrafted");
                    }
                    if c.draft.is_some_and(|d| d.1 == 1) {
                        tags.push("firstround");
                    }
                    if c.status == crate::world::pro::Status::PracticeSquad {
                        tags.push("psquad");
                    }
                }
            }
            crate::world::Stage::College | crate::world::Stage::Declared => {
                if p.college_seasons == 0 {
                    tags.push("freshman");
                }
                if p.college_seasons >= 3 {
                    tags.push("senior");
                }
                if p.redshirt_used {
                    tags.push("redshirt");
                }
                if p.transfers > 0 {
                    tags.push("transfer");
                }
                if p.walk_on {
                    tags.push("walkon");
                }
            }
            _ => {
                let fall = if w.today.month() >= 7 { w.today.year() } else { w.today.year() - 1 };
                tags.push(match p.class_of - fall {
                    ..=1 => "senior",
                    2 => "junior",
                    3 => "sophomore",
                    _ => "freshman",
                });
            }
        }
        Self::region(&mut tags, w, Some(p.home_state));
        Self::era(&mut tags, w.year);
        Speaker { key: 0x9_0000_0000 ^ p.player.id as u64, tags }
    }

    pub fn coach(w: &World, c: usize) -> Speaker {
        let co = &w.coaches[c];
        let mut tags: Vec<&'static str> = vec!["coach"];
        let pe = co.personality;
        if pe.temper >= 14.0 {
            tags.push("hot");
        }
        if pe.temper <= 7.0 {
            tags.push("calm");
        }
        if pe.ambition >= 14.0 {
            tags.push("climber");
        }
        if pe.loyalty >= 14.0 {
            tags.push("loyal");
        }
        if co.age >= 55 {
            tags.push("old");
        }
        if co.age < 35 {
            tags.push("young");
        }
        if co.ex_pro {
            tags.push("expro");
        }
        let a = co.ability();
        if a.discipline >= 14.0 {
            tags.push("strict");
        }
        if a.relationships >= 14.0 {
            tags.push("warm");
        }
        if a.relationships <= 7.0 {
            tags.push("cold");
        }
        match co.job.map(|j| j.employer) {
            Some(crate::world::coach::Employer::Nfl(_)) => tags.push("nfl"),
            Some(crate::world::coach::Employer::HighSchool(_)) => tags.push("hs"),
            _ => tags.push("college"),
        }
        Self::region(&mut tags, w, co.home_state);
        Self::era(&mut tags, w.year);
        Speaker { key: super::coach_key(c), tags }
    }

    pub fn cast(w: &World, c: &Cast) -> Speaker {
        let mut tags: Vec<&'static str> = vec![match c.r.role {
            Role::AthleticDirector | Role::SchoolAd => "ad",
            Role::President => "president",
            Role::Principal => "principal",
            Role::Superintendent => "superintendent",
            Role::Owner => "owner",
            Role::OwnerKin => "kin",
            Role::TeamPresident => "teampresident",
            Role::Gm => "gm",
            Role::Booster | Role::ClubPresident => "booster",
            Role::Reporter => "reporter",
            Role::Comms => "comms",
            Role::Ops => "ops",
            Role::Academic => "academic",
            Role::Trainer => "trainer",
            Role::Strength => "strength",
            Role::CapManager => "cap",
            Role::Personnel => "personnel",
            Role::Feeder | Role::Youth => "feeder",
            Role::ChiefOfStaff => "chief",
        }];
        tags.push(match c.r.team {
            super::Team::College(_) => "college",
            super::Team::Nfl(_) | super::Team::Other(_) => "nfl",
            super::Team::High(_) => "hs",
        });
        if c.she {
            tags.push("she");
        }
        if c.ego >= 14.0 {
            tags.push("ego");
        }
        if c.temper >= 14.0 {
            tags.push("hot");
        }
        if c.warmth >= 13.0 {
            tags.push("warm");
        }
        if c.warmth <= 7.0 {
            tags.push("cold");
        }
        if c.honesty >= 14.0 {
            tags.push("blunt");
        }
        if c.honesty <= 8.0 {
            tags.push("smooth");
        }
        if c.patience <= 7.0 {
            tags.push("impatient");
        }
        if c.patience >= 14.0 {
            tags.push("patient");
        }
        if c.meddling >= 14.0 {
            tags.push("meddler");
        }
        if c.tight >= 14.0 {
            tags.push("tight");
        }
        if c.age >= 60 {
            tags.push("old");
        }
        Self::region(&mut tags, w, c.r.team.state(w));
        Self::era(&mut tags, w.year);
        Speaker { key: c.key(), tags }
    }

    /// You: no hidden traits, your voice is your region and the tone you pick.
    pub fn you(w: &World) -> Speaker {
        let mut tags: Vec<&'static str> = vec!["you"];
        let state = w.career.as_ref().map(|c| c.you.home_state);
        Self::region(&mut tags, w, state);
        Self::era(&mut tags, w.year);
        if let Some(t) = super::Team::seat(w) {
            tags.push(match t {
                super::Team::College(_) => "college",
                super::Team::Nfl(_) | super::Team::Other(_) => "nfl",
                super::Team::High(_) => "hs",
            });
        }
        Speaker { key: 0x4000_0000_0000_0001, tags }
    }

    /// A parent of a player (people topic 5, the parents' style).
    pub fn parent(w: &World, i: usize) -> Speaker {
        let p = &w.people[i];
        let mut tags: Vec<&'static str> = vec!["parent"];
        tags.push(match p.family.parent_style {
            crate::world::background::ParentStyle::LoudAtGames => "loud",
            crate::world::background::ParentStyle::SocialMedia => "online",
            crate::world::background::ParentStyle::CalmPrivate => "calm",
            crate::world::background::ParentStyle::Absent => "absent",
        });
        Self::region(&mut tags, w, Some(p.home_state));
        Self::era(&mut tags, w.year);
        Speaker { key: 0x7A_0000_0000 ^ p.player.id as u64, tags }
    }
}

/// The facts one line can use.
#[derive(Clone, Debug, Default)]
pub struct Facts(pub Vec<(&'static str, String)>);

impl Facts {
    pub fn new() -> Facts {
        Facts(Vec::new())
    }
    pub fn set(&mut self, k: &'static str, v: impl Into<String>) -> &mut Facts {
        let v = v.into();
        if v.is_empty() {
            return self;
        }
        match self.0.iter_mut().find(|x| x.0 == k) {
            Some(x) => x.1 = v,
            None => self.0.push((k, v)),
        }
        self
    }
    pub fn with(mut self, k: &'static str, v: impl Into<String>) -> Facts {
        self.set(k, v);
        self
    }
    pub fn get(&self, k: &str) -> Option<&str> {
        self.0.iter().find(|x| x.0 == k).map(|x| x.1.as_str())
    }
}

/// Writes lines: the bank, a seeded stream and the speakers' memories.
/// It takes the memories out of the world while it writes (`take`) and puts
/// them back when done (`put`), so the world stays readable in between.
pub struct Pen {
    pub rng: Rng,
    today: Date,
    said: Vec<(u64, Vec<(u32, Date)>)>,
}

/// How long a version rests before the same speaker says it to you again:
/// a full season (D3, Cain 2026-10-27).
pub const REST_DAYS: i32 = 365;

impl Pen {
    pub fn take(w: &mut World, seed: u64) -> Pen {
        Pen {
            rng: Rng::new((seed ^ seed >> 32) as u32 ^ 0x5A1D_0000),
            today: w.today,
            said: std::mem::take(&mut w.folk.said_on),
        }
    }

    /// A pen with no memories (tests, and lines nobody hears twice).
    pub fn loose(seed: u64) -> Pen {
        Pen { rng: Rng::new((seed ^ seed >> 32) as u32 ^ 0x5A1D_0000), today: Date(0), said: Vec::new() }
    }

    /// A pen with its own memories on its own day (the repeat proof).
    pub fn on(seed: u64, today: Date, said: Vec<(u64, Vec<(u32, Date)>)>) -> Pen {
        Pen { rng: Rng::new((seed ^ seed >> 32) as u32 ^ 0x5A1D_0000), today, said }
    }

    /// Moves the pen's day on (the repeat proof writes a season with one pen).
    pub fn set_day(&mut self, d: Date) {
        self.today = d;
    }

    /// The memories, given back (the repeat proof).
    pub fn memories(self) -> Vec<(u64, Vec<(u32, Date)>)> {
        self.said
    }

    /// Puts the speakers' memories back in the world. Speakers who have
    /// said nothing to you for a season are forgotten.
    pub fn put(mut self, w: &mut World) {
        let today = self.today;
        self.said.retain(|x| x.1.iter().any(|&(_, d)| d.days_to(today) < REST_DAYS));
        w.folk.said_on = self.said;
    }

    /// When this speaker last said this version to you.
    fn last_said(&self, key: u64, id: u32) -> Option<Date> {
        self.said.iter().find(|x| x.0 == key).and_then(|x| x.1.iter().find(|e| e.0 == id)).map(|e| e.1)
    }

    fn remember(&mut self, key: u64, id: u32) {
        let today = self.today;
        match self.said.iter_mut().find(|x| x.0 == key) {
            Some(x) => {
                x.1.retain(|&(i, d)| i != id && d.days_to(today) < REST_DAYS);
                x.1.push((id, today));
            }
            None => self.said.push((key, vec![(id, today)])),
        }
    }

    /// How many versions of this part the speaker could say today that he
    /// hasn't said to you this season.
    pub fn fresh(&self, key: &str, who: &Speaker, facts: &Facts) -> usize {
        let Some(frags) = bank().by_key.get(key) else { return 0 };
        frags
            .iter()
            .filter(|f| f.fits(who, facts))
            .filter(|f| self.last_said(who.key, f.id).is_none_or(|d| d.days_to(self.today) >= REST_DAYS))
            .count()
    }

    /// One part from the bank for this speaker, or None when the bank has
    /// no version he can say with these facts. Which part is asked for, and
    /// so what he says, is the caller's; the bank only picks its wording.
    pub fn part(&mut self, key: &str, who: &Speaker, facts: &Facts) -> Option<String> {
        let frags = bank().by_key.get(key)?;
        let ok: Vec<&Frag> = frags.iter().filter(|f| f.fits(who, facts)).collect();
        if ok.is_empty() {
            return None;
        }
        let today = self.today;
        let last: Vec<Option<Date>> = ok.iter().map(|f| self.last_said(who.key, f.id)).collect();
        let fresh: Vec<&Frag> = ok
            .iter()
            .zip(&last)
            .filter(|(_, l)| l.is_none_or(|d| d.days_to(today) >= REST_DAYS))
            .map(|(f, _)| *f)
            .collect();
        let f = if fresh.is_empty() {
            // Everything he can say has been said this season: the version
            // he said longest ago.
            let k = (0..ok.len()).min_by_key(|&k| (last[k], k)).unwrap_or(0);
            ok[k]
        } else {
            // A version written for his kind of voice, or for this moment,
            // is likelier than a general one.
            let weights: Vec<(usize, f64)> =
                fresh.iter().enumerate().map(|(k, f)| (k, 1.0 + 0.8 * (f.need.len() + f.when.len()) as f64)).collect();
            fresh[self.rng.weighted(&weights)]
        };
        self.remember(who.key, f.id);
        Some(fill(&f.text, facts, &mut self.rng))
    }

    /// A whole line from several parts; a part starting with `?` is
    /// optional (said about half the time). Parts the speaker can't say
    /// are left out.
    pub fn line(&mut self, keys: &[&str], who: &Speaker, facts: &Facts) -> String {
        let mut out: Vec<String> = Vec::new();
        for k in keys {
            let (opt, k) = match k.strip_prefix('?') {
                Some(r) => (true, r),
                None => (false, *k),
            };
            if opt && !self.rng.chance(0.55) {
                continue;
            }
            if let Some(s) = self.part(k, who, facts) {
                out.push(s);
            }
        }
        tidy(&out.join(" "))
    }
}

/// Writes a crowd's lines (a town's fans, `fans.rs`): the memory is the
/// crowd's as one, over the year the chorus is walked through, and is
/// never saved. A version said once waits until every other version that
/// fits has had its turn, then the one said longest ago comes back.
pub struct Chorus {
    pub rng: Rng,
    /// (the crowd's key, the version) and when in the walk it was said.
    used: HashMap<(u64, u32), u32>,
    n: u32,
}

impl Chorus {
    pub fn new(seed: u64) -> Chorus {
        Chorus { rng: Rng::new((seed ^ seed >> 32) as u32 ^ 0xC40B_0000), used: HashMap::new(), n: 0 }
    }

    /// How many versions of this part the crowd could say that it hasn't.
    pub fn fresh(&self, key: &str, who: &Speaker, facts: &Facts) -> usize {
        let Some(frags) = bank().by_key.get(key) else { return 0 };
        frags.iter().filter(|f| f.fits(who, facts) && !self.used.contains_key(&(who.key, f.id))).count()
    }

    /// One part for this voice, or None when no version fits. Weighted like
    /// `Pen::part`: a version written for this voice or moment is likelier.
    pub fn part(&mut self, key: &str, who: &Speaker, facts: &Facts) -> Option<String> {
        let frags = bank().by_key.get(key)?;
        let ok: Vec<&Frag> = frags.iter().filter(|f| f.fits(who, facts)).collect();
        if ok.is_empty() {
            return None;
        }
        let fresh: Vec<&Frag> = ok.iter().copied().filter(|f| !self.used.contains_key(&(who.key, f.id))).collect();
        let f = if fresh.is_empty() {
            let k = (0..ok.len()).min_by_key(|&k| (self.used.get(&(who.key, ok[k].id)).copied(), k)).unwrap_or(0);
            ok[k]
        } else {
            let weights: Vec<(usize, f64)> =
                fresh.iter().enumerate().map(|(k, f)| (k, 1.0 + 0.8 * (f.need.len() + f.when.len()) as f64)).collect();
            fresh[self.rng.weighted(&weights)]
        };
        self.n += 1;
        self.used.insert((who.key, f.id), self.n);
        Some(fill(&f.text, facts, &mut self.rng))
    }

    /// A whole line from several parts, as `Pen::line` (`?` marks a part
    /// said about half the time).
    pub fn line(&mut self, keys: &[&str], who: &Speaker, facts: &Facts) -> String {
        let mut out: Vec<String> = Vec::new();
        for k in keys {
            let (opt, k) = match k.strip_prefix('?') {
                Some(r) => (true, r),
                None => (false, *k),
            };
            if opt && !self.rng.chance(0.55) {
                continue;
            }
            if let Some(s) = self.part(k, who, facts) {
                out.push(s);
            }
        }
        tidy(&out.join(" "))
    }
}

/// Fills a version: its choices and its facts.
fn fill(text: &str, facts: &Facts, rng: &mut Rng) -> String {
    let mut out = String::with_capacity(text.len() + 32);
    let mut rest = text;
    loop {
        let a = rest.find(['[', '{']);
        let Some(a) = a else {
            out.push_str(rest);
            break;
        };
        out.push_str(&rest[..a]);
        let open = rest.as_bytes()[a];
        let close = if open == b'[' { ']' } else { '}' };
        let Some(z) = rest[a..].find(close) else {
            out.push_str(&rest[a..]);
            break;
        };
        let inner = &rest[a + 1..a + z];
        if open == b'[' {
            let opts: Vec<&str> = inner.split('|').collect();
            out.push_str(opts[rng.int(0, opts.len() as i32 - 1) as usize]);
        } else {
            out.push_str(facts.get(inner).unwrap_or(""));
        }
        rest = &rest[a + z + 1..];
    }
    out
}

/// Spacing and the first letter of each sentence. A name ending in "Jr."
/// at the end of a sentence keeps one period; an ellipsis stays.
pub fn tidy(s: &str) -> String {
    let s = s.split_whitespace().collect::<Vec<_>>().join(" ");
    let s = s.replace(" ,", ",").replace(" .", ".").replace(" ?", "?").replace(" !", "!").replace(",.", ".");
    let mut out = String::with_capacity(s.len());
    let mut up = true;
    let mut in_link = false;
    let b = s.as_bytes();
    let mut k = 0;
    while k < b.len() {
        let ch = s[k..].chars().next().unwrap();
        let n = ch.len_utf8();
        if ch == '[' && b.get(k + 1) == Some(&b'[') {
            in_link = true;
            up = false;
        }
        if ch == ']' && b.get(k + 1) == Some(&b']') {
            in_link = false;
        }
        // Two dots become one; three stay an ellipsis.
        if ch == '.' && b.get(k + 1) == Some(&b'.') && b.get(k + 2) != Some(&b'.') && (k == 0 || b[k - 1] != b'.') {
            k += 1;
            continue;
        }
        if up && ch.is_alphabetic() && !in_link {
            out.extend(ch.to_uppercase());
            up = false;
            k += n;
            continue;
        }
        if ch.is_alphanumeric() {
            up = false;
        }
        out.push(ch);
        if matches!(ch, '.' | '?' | '!') && !in_link {
            // "Jr." or "St." inside a sentence: the next word is not a new sentence.
            let before = &s[..k];
            let abbr = ["Jr", "Sr", "St", "Mr", "Mrs", "Dr", "vs", "No"].iter().any(|a| before.ends_with(a))
                && b.get(k + 1) == Some(&b' ')
                && b.get(k + 2).is_some_and(|c| c.is_ascii_lowercase());
            up = !abbr;
        }
        k += n;
    }
    out
}
