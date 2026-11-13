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
//!
//! A one-on-one line that denies or admits something about his own conduct
//! ("I've done nothing wrong", "I know I messed up") carries `blameless=yes`
//! or `at_fault=yes` (`CLAIM_FACTS`). No caller sets those facts yet, so
//! those lines are not said at all: a man with a fresh arrest never says he
//! has done nothing wrong, and a clean one never owns a mistake he didn't
//! make. The fact sheet of the talks (talk facts, item 5 and 6) sets a fact
//! when the world backs the line.

use std::collections::HashMap;
use std::sync::OnceLock;

use super::cast::{Cast, Role};
use crate::calendar::Date;
use crate::player::Trait;
use crate::rng::Rng;
use crate::world::World;
use crate::world::pronoun::Pro;

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
    // The line bank run (Cain 2026-11-07): one file per family, written
    // through the Batches API and checked by tools/linebank.
    include_str!("../../../data/lines/talks_run.txt"),
    include_str!("../../../data/lines/gameday_run.txt"),
    include_str!("../../../data/lines/staff_run.txt"),
    include_str!("../../../data/lines/recruit_run.txt"),
    include_str!("../../../data/lines/ad_run.txt"),
    include_str!("../../../data/lines/agents_run.txt"),
    include_str!("../../../data/lines/room_run.txt"),
    include_str!("../../../data/lines/boosters_run.txt"),
];

/// The facts that gate a line which denies (`blameless`) or admits
/// (`at_fault`) something about the speaker's own conduct. Nothing sets them
/// today, so those lines are held back until the talks can check the facts.
pub const CLAIM_FACTS: [&str; 2] = ["blameless", "at_fault"];

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
                    // A pronoun is always there: he unless the facts or the
                    // pen say she.
                    if !is_pronoun(&rest[a + 1..a + z]) {
                        facts.push(rest[a + 1..a + z].to_string());
                    }
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

    /// How many versions of a part this speaker can say with these facts.
    pub fn fitting(&self, key: &str, who: &Speaker, facts: &Facts) -> usize {
        self.by_key.get(key).map_or(0, |fr| fr.iter().filter(|f| f.fits(who, facts)).count())
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
        // A man out of football (retired, or away on a mission or a
        // military term) speaks from the last level he played (minor bug 92).
        let last = || p.player.history.last().map(|h| h.level);
        tags.push(match p.stage {
            crate::world::Stage::Pro => "nfl",
            crate::world::Stage::College | crate::world::Stage::Declared => "college",
            crate::world::Stage::Done | crate::world::Stage::Away => match last() {
                Some(crate::player::Level::NFL) => "nfl",
                Some(l) if l.is_college() => "college",
                _ if p.pro.is_some() => "nfl",
                _ if p.college_seasons > 0 => "college",
                _ => "hs",
            },
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
        // Who talks back in your office (Cain 2026-11-05, "Yes, tag them"):
        // a high school player never; a college player only with a big ego,
        // a hot temper and little trust in his coach; an NFL ego or temper
        // keeps its edge. Low trust is under 0.4, below where a new coach
        // starts (about 0.5; proposed, not confirmed).
        const BRASH_TRUST: f32 = 0.4;
        let brash = match tags[1] {
            "nfl" => hi(Trait::Ego) || hi(Trait::Temper),
            "college" => hi(Trait::Ego) && hi(Trait::Temper) && p.trust < BRASH_TRUST,
            _ => false,
        };
        if brash {
            tags.push("brash");
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
            // No class year for a man whose playing days are behind him.
            crate::world::Stage::Done | crate::world::Stage::Away => {}
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
        // Your own coach speaks as you set it up: no "my wife" from her.
        if crate::world::pronoun::coach(w, c).she {
            tags.push("she");
        }
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
    /// The coach's pronouns, {coach_he} {coach_him} {coach_his}
    /// {coach_himself} {coach_man} and {coach_sir} (sir or ma'am to his
    /// face): whoever {coach} names.
    pub fn coach(&mut self, pro: Pro) -> &mut Facts {
        self.pronouns("coach", pro)
    }
    /// A story lead's, {lead_he} and the rest.
    pub fn lead(&mut self, pro: Pro) -> &mut Facts {
        self.pronouns("lead", pro)
    }
    fn pronouns(&mut self, who: &'static str, pro: Pro) -> &mut Facts {
        let coach = who == "coach";
        self.set(if coach { "coach_he" } else { "lead_he" }, pro.he());
        self.set(if coach { "coach_him" } else { "lead_him" }, pro.him());
        self.set(if coach { "coach_his" } else { "lead_his" }, pro.his());
        self.set(if coach { "coach_himself" } else { "lead_himself" }, pro.himself());
        self.set(if coach { "coach_man" } else { "lead_man" }, pro.man());
        self.set(if coach { "coach_sir" } else { "lead_sir" }, pro.sir())
    }
}

/// How many times each version has been said to you, from anyone (the
/// listener-wide rest, line bank run question 4, Cain 2026-11-07): a
/// wording waits until every other wording of its part the speaker can say
/// has been heard as often. Kept for the whole career, one entry a version
/// you have heard (the bank's size at most).
pub type Heard = std::collections::BTreeMap<u32, u16>;

/// Writes lines: the bank, a seeded stream and the memories (what each
/// speaker has said to you, and what you've heard from anyone).
/// It takes the memories out of the world while it writes (`take`) and puts
/// them back when done (`put`), so the world stays readable in between.
pub struct Pen {
    pub rng: Rng,
    today: Date,
    said: Vec<(u64, Vec<(u32, Date)>)>,
    heard: Heard,
    /// Your words: what {coach_he} and the like say when the facts don't
    /// name the coach (a line said to you speaks of you).
    you: Pro,
}

/// How long a version rests before the same speaker says it to you again:
/// a full season (D3, Cain 2026-10-27).
pub const REST_DAYS: i32 = 365;

/// The pick (D3 and the listener-wide rest): among the versions that fit,
/// the ones this speaker hasn't said to you in a season come first; of
/// those, the ones you've heard least often from anyone; and among them a
/// version written for his kind of voice, or for this moment, is likelier
/// than a general one. If he has said every version that fits this season,
/// the one he said longest ago.
fn choose<'a>(
    ok: &[&'a Frag],
    last: impl Fn(u32) -> Option<Date>,
    times: impl Fn(u32) -> u32,
    today: Date,
    rng: &mut Rng,
) -> &'a Frag {
    let last: Vec<Option<Date>> = ok.iter().map(|f| last(f.id)).collect();
    let fresh: Vec<usize> = (0..ok.len()).filter(|&k| last[k].is_none_or(|d| d.days_to(today) >= REST_DAYS)).collect();
    if fresh.is_empty() {
        let k = (0..ok.len()).min_by_key(|&k| (last[k], k)).unwrap_or(0);
        return ok[k];
    }
    let n: Vec<u32> = fresh.iter().map(|&k| times(ok[k].id)).collect();
    let least = n.iter().copied().min().unwrap_or(0);
    let tier: Vec<&Frag> = fresh.iter().zip(&n).filter(|x| *x.1 == least).map(|x| ok[*x.0]).collect();
    let weights: Vec<(usize, f64)> =
        tier.iter().enumerate().map(|(k, f)| (k, 1.0 + 0.8 * (f.need.len() + f.when.len()) as f64)).collect();
    tier[rng.weighted(&weights)]
}

/// When this speaker last said this version to you, in a memory.
fn said_last(said: &[(u64, Vec<(u32, Date)>)], key: u64, id: u32) -> Option<Date> {
    said.iter().find(|x| x.0 == key).and_then(|x| x.1.iter().find(|e| e.0 == id)).map(|e| e.1)
}

/// Remembers a version said to you today: by its speaker (for a season)
/// and once more heard from anyone (for good).
fn remember_in(said: &mut Vec<(u64, Vec<(u32, Date)>)>, heard: &mut Heard, key: u64, id: u32, today: Date) {
    match said.iter_mut().find(|x| x.0 == key) {
        Some(x) => {
            x.1.retain(|&(i, d)| i != id && d.days_to(today) < REST_DAYS);
            x.1.push((id, today));
        }
        None => said.push((key, vec![(id, today)])),
    }
    let n = heard.entry(id).or_insert(0);
    *n = n.saturating_add(1);
}

impl Pen {
    pub fn take(w: &mut World, seed: u64) -> Pen {
        let you = crate::world::pronoun::you(w);
        let today = w.today;
        Pen { you, ..Pen::take_from(&mut w.folk, today, seed) }
    }

    /// `take` from a people store of its own on its own day (the proofs).
    pub fn take_from(f: &mut super::Folk, today: Date, seed: u64) -> Pen {
        Pen {
            rng: Rng::new((seed ^ seed >> 32) as u32 ^ 0x5A1D_0000),
            today,
            said: std::mem::take(&mut f.said_on),
            heard: std::mem::take(&mut f.heard),
            you: Pro::HE,
        }
    }

    /// A pen with no memories (tests, and lines nobody hears twice).
    pub fn loose(seed: u64) -> Pen {
        Pen {
            rng: Rng::new((seed ^ seed >> 32) as u32 ^ 0x5A1D_0000),
            today: Date(0),
            said: Vec::new(),
            heard: Heard::new(),
            you: Pro::HE,
        }
    }

    /// A pen with its own memories on its own day (the repeat proof).
    pub fn on(seed: u64, today: Date, said: Vec<(u64, Vec<(u32, Date)>)>) -> Pen {
        Pen { rng: Rng::new((seed ^ seed >> 32) as u32 ^ 0x5A1D_0000), today, said, heard: Heard::new(), you: Pro::HE }
    }

    /// Moves the pen's day on (the repeat proof writes a season with one pen).
    pub fn set_day(&mut self, d: Date) {
        self.today = d;
    }

    /// The memories, given back (the repeat proof).
    pub fn memories(self) -> Vec<(u64, Vec<(u32, Date)>)> {
        self.said
    }

    /// How often you've heard each version, from anyone (the proofs).
    pub fn heard(&self) -> &Heard {
        &self.heard
    }

    /// Puts the memories back in the world. Speakers who have said nothing
    /// to you for a season are forgotten; what you've heard is kept.
    pub fn put(self, w: &mut World) {
        self.put_into(&mut w.folk);
    }

    /// `put` into a people store of its own (the proofs).
    pub fn put_into(mut self, f: &mut super::Folk) {
        let today = self.today;
        self.said.retain(|x| x.1.iter().any(|&(_, d)| d.days_to(today) < REST_DAYS));
        f.said_on = self.said;
        f.heard = self.heard;
    }

    /// How many versions of this part the speaker could say today that he
    /// hasn't said to you this season.
    pub fn fresh(&self, key: &str, who: &Speaker, facts: &Facts) -> usize {
        let Some(frags) = bank().by_key.get(key) else { return 0 };
        frags
            .iter()
            .filter(|f| f.fits(who, facts))
            .filter(|f| said_last(&self.said, who.key, f.id).is_none_or(|d| d.days_to(self.today) >= REST_DAYS))
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
        let (said, heard) = (&self.said, &self.heard);
        let f = choose(
            &ok,
            |id| said_last(said, who.key, id),
            |id| heard.get(&id).copied().unwrap_or(0) as u32,
            self.today,
            &mut self.rng,
        );
        remember_in(&mut self.said, &mut self.heard, who.key, f.id, self.today);
        Some(fill(&f.text, facts, &mut self.rng, self.you))
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

/// One line a screen showed you: (its seed, the speaker, the part, the
/// version).
type ShownLine = (u64, u64, u32, u32);

/// The lines screens showed you today (minor bug 314): the game day room
/// and its preview, the sideline words and the paper's quote read the
/// world without changing it, so what they pick waits in `waiting` until
/// the game's call ends; `remember_shown` then keeps it in `lines` and
/// remembers it as said to you. The same moment shows the same words all
/// day; a new day starts the list again.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Shown {
    pub day: Date,
    pub lines: Vec<ShownLine>,
    #[serde(skip)]
    waiting: Waiting,
}

/// Lines picked by a screen and not yet remembered.
#[derive(Debug, Default)]
struct Waiting(std::sync::Mutex<Vec<ShownLine>>);

impl Clone for Waiting {
    fn clone(&self) -> Waiting {
        Waiting(std::sync::Mutex::new(self.0.lock().map(|v| v.clone()).unwrap_or_default()))
    }
}

impl Shown {
    fn waiting(&self) -> Vec<ShownLine> {
        self.waiting.0.lock().map(|v| v.clone()).unwrap_or_default()
    }

    fn wait(&self, l: ShownLine) {
        if let Ok(mut v) = self.waiting.0.lock() {
            // A screen nobody remembers (a test, a tool) never grows it
            // past a day's worth.
            if v.len() >= 4096 {
                v.remove(0);
            }
            v.push(l);
        }
    }
}

/// Remembers the lines the screens showed you since the last call (the
/// game runs it after every command): each as said to you today by its
/// speaker, and heard once more. A line you read on a screen counts as
/// heard.
pub fn remember_shown(w: &mut World) {
    let today = w.today;
    remember_shown_in(&mut w.folk, today);
}

/// `remember_shown` over a people store of its own (the proofs).
pub fn remember_shown_in(f: &mut super::Folk, today: Date) {
    let waiting = std::mem::take(f.shown.waiting.0.get_mut().unwrap_or_else(|e| e.into_inner()));
    if f.shown.day != today {
        f.shown.day = today;
        f.shown.lines.clear();
    }
    for l in waiting {
        match f.shown.lines.iter_mut().find(|x| (x.0, x.1, x.2) == (l.0, l.1, l.2)) {
            Some(x) if x.3 == l.3 => continue,
            // The version shown earlier no longer fits (its facts moved):
            // the new one is what the screen shows from now on.
            Some(x) => x.3 = l.3,
            None => f.shown.lines.push(l),
        }
        remember_in(&mut f.said_on, &mut f.heard, l.1, l.3, today);
    }
}

/// A part's key as a number, for the lines a screen showed.
fn part_id(key: &str) -> u32 {
    version_id(key, "")
}

/// A pen for a screen that reads the world without changing it (the game
/// day room and its preview, the sideline, the paper's quote; minor bug
/// 314). It picks the way `Pen` does, with the world's memories and with
/// what the screens showed earlier today, and the same seed gives the same
/// words all day. What it picks is remembered after the game's call
/// (`remember_shown`).
pub struct Look<'w> {
    folk: &'w super::Folk,
    today: Date,
    seed: u64,
    you: Pro,
}

impl<'w> Look<'w> {
    pub fn new(w: &'w World, seed: u64) -> Look<'w> {
        Look { folk: &w.folk, today: w.today, seed, you: crate::world::pronoun::you(w) }
    }

    /// A look over a people store of its own (the proofs).
    pub fn over(folk: &'w super::Folk, today: Date, seed: u64) -> Look<'w> {
        Look { folk, today, seed, you: Pro::HE }
    }

    /// Its own stream for one part, so a part's words don't depend on which
    /// parts came before it.
    fn rng(&self, who: &Speaker, part: u32, salt: u32) -> Rng {
        let s = self.seed ^ who.key.rotate_left(17) ^ (part as u64) << 11 ^ salt as u64;
        Rng::new((s ^ s >> 32) as u32 ^ 0x100C_0000)
    }

    /// How many versions of this part the speaker could say today that he
    /// hasn't said to you this season (a line shown since the last call
    /// counts as said).
    pub fn fresh(&self, key: &str, who: &Speaker, facts: &Facts) -> usize {
        let Some(frags) = bank().by_key.get(key) else { return 0 };
        let waiting = self.folk.shown.waiting();
        frags
            .iter()
            .filter(|f| f.fits(who, facts))
            .filter(|f| !waiting.iter().any(|l| l.1 == who.key && l.3 == f.id))
            .filter(|f| said_last(&self.folk.said_on, who.key, f.id).is_none_or(|d| d.days_to(self.today) >= REST_DAYS))
            .count()
    }

    /// One part for this speaker, as `Pen::part`.
    pub fn part(&self, key: &str, who: &Speaker, facts: &Facts) -> Option<String> {
        let frags = bank().by_key.get(key)?;
        let ok: Vec<&Frag> = frags.iter().filter(|f| f.fits(who, facts)).collect();
        if ok.is_empty() {
            return None;
        }
        let part = part_id(key);
        let shown = &self.folk.shown;
        let waiting = shown.waiting();
        let today = shown.day == self.today;
        let same = |l: &&ShownLine| (l.0, l.1, l.2) == (self.seed, who.key, part);
        let before =
            waiting.iter().find(same).or_else(|| if today { shown.lines.iter().find(same) } else { None }).map(|l| l.3);
        let f = match before.and_then(|id| ok.iter().find(|f| f.id == id)) {
            Some(f) => *f,
            None => {
                // Lines shown since the last call count as said already.
                let said = &self.folk.said_on;
                let heard = &self.folk.heard;
                let f = choose(
                    &ok,
                    |id| {
                        if waiting.iter().any(|l| l.1 == who.key && l.3 == id) {
                            Some(self.today)
                        } else {
                            said_last(said, who.key, id)
                        }
                    },
                    |id| {
                        heard.get(&id).copied().unwrap_or(0) as u32
                            + waiting.iter().filter(|l| l.3 == id).count() as u32
                    },
                    self.today,
                    &mut self.rng(who, part, 0),
                );
                shown.wait((self.seed, who.key, part, f.id));
                f
            }
        };
        Some(fill(&f.text, facts, &mut self.rng(who, part, f.id), self.you))
    }

    /// A whole line from several parts, as `Pen::line` (`?` marks a part
    /// said about half the time, the same half all day).
    pub fn line(&self, keys: &[&str], who: &Speaker, facts: &Facts) -> String {
        let mut out: Vec<String> = Vec::new();
        for k in keys {
            let (opt, k) = match k.strip_prefix('?') {
                Some(r) => (true, r),
                None => (false, *k),
            };
            if opt && !self.rng(who, part_id(k), 0x0BE7).chance(0.55) {
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
        #[cfg(test)]
        again::note(key, fresh.is_empty());
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
        Some(fill(&f.text, facts, &mut self.rng, Pro::HE))
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

/// The pronoun facts a line may name about the coach ({coach_he}) or a
/// story's lead ({lead_he}). Always there: set by the caller (`Facts::coach`,
/// `Facts::lead`), else your own for the coach on a pen's line, else he.
const PRONOUNS: [&str; 6] = ["he", "him", "his", "himself", "man", "sir"];

fn is_pronoun(k: &str) -> bool {
    k.strip_prefix("coach_").or_else(|| k.strip_prefix("lead_")).is_some_and(|w| PRONOUNS.contains(&w))
}

fn pronoun(k: &str, coach: Pro) -> Option<&'static str> {
    let (pro, w) = match (k.strip_prefix("coach_"), k.strip_prefix("lead_")) {
        (Some(w), _) => (coach, w),
        (_, Some(w)) => (Pro::HE, w),
        _ => return None,
    };
    Some(match w {
        "he" => pro.he(),
        "him" => pro.him(),
        "his" => pro.his(),
        "himself" => pro.himself(),
        "man" => pro.man(),
        "sir" => pro.sir(),
        _ => return None,
    })
}

/// Fills a version: its choices and its facts.
pub(crate) fn fill(text: &str, facts: &Facts, rng: &mut Rng, you: Pro) -> String {
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
            let v = facts.get(inner).or_else(|| pronoun(inner, you)).unwrap_or("");
            // A pronoun that opens a sentence takes its capital here, for a
            // part said on its own.
            let opens = out.trim_end().is_empty() || out.trim_end().ends_with(['.', '!', '?', '"']);
            match v.chars().next() {
                Some(c) if is_pronoun(inner) && opens => {
                    out.extend(c.to_uppercase());
                    out.push_str(&v[c.len_utf8()..]);
                }
                _ => out.push_str(v),
            }
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

/// How often a chorus said a version again for want of a fresh one, by
/// part: (said, said again). Tests only (the fans' season, `tests/fans.rs`).
#[cfg(test)]
pub(crate) mod again {
    use std::cell::RefCell;
    use std::collections::HashMap;

    thread_local! {
        static SAID: RefCell<HashMap<String, (u32, u32)>> = RefCell::new(HashMap::new());
    }

    pub(crate) fn note(key: &str, again: bool) {
        SAID.with(|m| {
            let mut m = m.borrow_mut();
            let e = m.entry(key.to_string()).or_default();
            e.0 += 1;
            e.1 += u32::from(again);
        });
    }

    /// Everything noted on this thread since the last take.
    pub(crate) fn take() -> HashMap<String, (u32, u32)> {
        SAID.with(|m| std::mem::take(&mut *m.borrow_mut()))
    }
}
