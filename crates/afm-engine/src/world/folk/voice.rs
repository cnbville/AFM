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

use std::collections::HashMap;
use std::sync::OnceLock;

use super::cast::{Cast, Role};
use crate::player::Trait;
use crate::rng::Rng;
use crate::world::World;

const BANKS: [&str; 9] = [
    include_str!("../../../data/lines/takeover.txt"),
    include_str!("../../../data/lines/players.txt"),
    include_str!("../../../data/lines/staff.txt"),
    include_str!("../../../data/lines/bosses.txt"),
    include_str!("../../../data/lines/boosters.txt"),
    include_str!("../../../data/lines/press.txt"),
    include_str!("../../../data/lines/room.txt"),
    include_str!("../../../data/lines/network.txt"),
    include_str!("../../../data/lines/life.txt"),
];

#[derive(Debug)]
struct Frag {
    id: u32,
    text: String,
    need: Vec<String>,
    not: Vec<String>,
    facts: Vec<String>,
}

#[derive(Default, Debug)]
pub struct Bank {
    by_key: HashMap<String, Vec<Frag>>,
    count: usize,
}

impl Bank {
    fn parse() -> Bank {
        let mut b = Bank::default();
        let mut key = String::new();
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
                if key.is_empty() {
                    continue;
                }
                let (text, tags) = match line.rsplit_once(" ~ ") {
                    Some((t, g)) => (t.trim(), g.trim()),
                    None => (line, ""),
                };
                let mut need = Vec::new();
                let mut not = Vec::new();
                for t in tags.split_whitespace() {
                    match t.strip_prefix('!') {
                        Some(n) => not.push(n.to_string()),
                        None => need.push(t.to_string()),
                    }
                }
                let mut facts = Vec::new();
                let mut rest = text;
                while let Some(a) = rest.find('{') {
                    let Some(z) = rest[a..].find('}') else { break };
                    facts.push(rest[a + 1..a + z].to_string());
                    rest = &rest[a + z + 1..];
                }
                let id = b.count as u32;
                b.count += 1;
                b.by_key.entry(key.clone()).or_default().push(Frag { id, text: text.to_string(), need, not, facts });
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

    fn era(tags: &mut Vec<&'static str>, year: i32) {
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

    fn region(tags: &mut Vec<&'static str>, w: &World, state: Option<usize>) {
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
    said: Vec<(u64, Vec<u32>)>,
}

/// How many recent parts each speaker keeps (topic 5: a pattern rests
/// about a season before the same person reaches for it again; proposed,
/// not confirmed).
const MEMORY: usize = 48;

impl Pen {
    pub fn take(w: &mut World, seed: u64) -> Pen {
        Pen { rng: Rng::new((seed ^ seed >> 32) as u32 ^ 0x5A1D_0000), said: std::mem::take(&mut w.folk.said) }
    }

    /// A pen with no memories (tests, and lines nobody hears twice).
    pub fn loose(seed: u64) -> Pen {
        Pen { rng: Rng::new((seed ^ seed >> 32) as u32 ^ 0x5A1D_0000), said: Vec::new() }
    }

    /// Puts the speakers' memories back in the world.
    pub fn put(self, w: &mut World) {
        w.folk.said = self.said;
    }

    fn memory(&self, key: u64) -> &[u32] {
        self.said.iter().find(|x| x.0 == key).map_or(&[], |x| x.1.as_slice())
    }

    fn remember(&mut self, key: u64, id: u32) {
        match self.said.iter_mut().find(|x| x.0 == key) {
            Some(x) => {
                x.1.push(id);
                if x.1.len() > MEMORY {
                    x.1.remove(0);
                }
            }
            None => {
                self.said.push((key, vec![id]));
                if self.said.len() > 900 {
                    self.said.remove(0);
                }
            }
        }
    }

    /// One part from the bank for this speaker, or None when the bank has
    /// no version he can say with these facts.
    pub fn part(&mut self, key: &str, who: &Speaker, facts: &Facts) -> Option<String> {
        let frags = bank().by_key.get(key)?;
        let seen = self.memory(who.key).to_vec();
        let ok: Vec<&Frag> = frags
            .iter()
            .filter(|f| f.need.iter().all(|t| who.has(t)))
            .filter(|f| !f.not.iter().any(|t| who.has(t)))
            .filter(|f| f.facts.iter().all(|k| facts.get(k).is_some()))
            .collect();
        if ok.is_empty() {
            return None;
        }
        let fresh: Vec<&Frag> = ok.iter().copied().filter(|f| !seen.contains(&f.id)).collect();
        let pool = if fresh.is_empty() { ok } else { fresh };
        // A version written for his kind of voice is likelier than a
        // general one.
        let weights: Vec<(usize, f64)> =
            pool.iter().enumerate().map(|(k, f)| (k, 1.0 + 0.8 * f.need.len() as f64)).collect();
        let k = self.rng.weighted(&weights);
        let f = pool[k];
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
