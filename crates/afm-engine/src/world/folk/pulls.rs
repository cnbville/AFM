//! What pulls him (talk facts design, section 7; Cain 2026-11-12, question
//! 4: "realistically it's the HC that determines NCAA signing. So tie that
//! into the dialogue update as much as you can"). You find out what a
//! recruit or a transfer weighs by talking to him, and his file keeps what
//! you have seen: what he brought up, how he took each point, what he told
//! you when you asked, what your read caught, and your area recruiter's
//! free guess before you spoke.
//!
//! The rows are evidence, never his true numbers: each says what he did or
//! said, where and when. `order` reads them into an order of the seven
//! groups that firms up as the evidence agrees, and marks where his words
//! and his reactions disagree ("says development, lights up at money").
//! The file's screen (lane L7) shows it.
//!
//! Every size here is Claude's (proposed, not confirmed).

use super::claims::{Ear, Group};
use crate::calendar::Date;
use crate::world::World;

/// Rows kept for one person. Proposed, not confirmed.
pub const ROWS_EACH: usize = 48;
/// Rows older than this go. Proposed, not confirmed.
pub const KEEP_DAYS: i32 = 730;
/// A good area recruiter (recruiting 16 or more) names a recruit's top
/// pull right this often before you have spoken to him (section 7's
/// working value: Cain left it open until the talks are played).
pub const GUESS_GOOD: f64 = 0.75;
/// A poor one (6 or less), this often.
pub const GUESS_POOR: f64 = 0.5;
/// The ratings a guess is read between (poor, good).
pub const GUESS_RATINGS: (f64, f64) = (6.0, 16.0);
/// What each kind of evidence says about a group, for the order. Proposed,
/// not confirmed.
pub const WEIGHT_BROUGHT_UP: f32 = 1.0;
/// A later question in the same talk: what he weighs after what he asked
/// first.
pub const WEIGHT_ASKED_LATER: f32 = 0.5;
pub const WEIGHT_TOLD_FIRST: f32 = 2.0;
pub const WEIGHT_TOLD_SECOND: f32 = 1.0;
pub const WEIGHT_TOOK_WELL: f32 = 1.0;
pub const WEIGHT_COOL: f32 = -1.0;
/// He cared and didn't believe you could back it up: it matters to him.
pub const WEIGHT_DOUBTED: f32 = 1.0;
/// What your read caught outweighs his words.
pub const WEIGHT_CAUGHT: f32 = 2.0;
pub const WEIGHT_GUESS: f32 = 0.75;

/// One piece of evidence about a group.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Ev {
    /// He opened on it or asked about it first.
    BroughtUp,
    /// He asked about it later in the talk.
    AskedLater,
    /// His words: it is what he came for.
    TookWell,
    /// His words: polite about it.
    Polite,
    /// His words: it isn't his thing.
    Cool,
    /// His words: it matters to him, and he doesn't believe you can back
    /// it up (you oversold it).
    Doubted,
    /// Your read caught it reaching him, whatever he said.
    CaughtLit,
    /// Your read caught the right words not reaching him.
    CaughtFlat,
    /// What he told you matters most, when you asked.
    ToldFirst,
    /// What he told you comes next.
    ToldSecond,
    /// Your area recruiter's guess, before you spoke.
    Guess,
    /// Who decides at his home, as he or they told you: the row's group
    /// holds `Decider` order (0 himself, 1 a parent, 2 a grandparent, 3 a
    /// handler). It speaks to no group.
    Decides,
}

impl Ev {
    fn weight(self) -> f32 {
        match self {
            Ev::BroughtUp => WEIGHT_BROUGHT_UP,
            Ev::AskedLater => WEIGHT_ASKED_LATER,
            Ev::TookWell => WEIGHT_TOOK_WELL,
            Ev::Polite => 0.0,
            Ev::Cool => WEIGHT_COOL,
            Ev::Doubted => WEIGHT_DOUBTED,
            Ev::CaughtLit => WEIGHT_CAUGHT,
            Ev::CaughtFlat => -WEIGHT_CAUGHT,
            Ev::ToldFirst => WEIGHT_TOLD_FIRST,
            Ev::ToldSecond => WEIGHT_TOLD_SECOND,
            Ev::Guess => WEIGHT_GUESS,
            Ev::Decides => 0.0,
        }
    }
    /// His words for it (what a sharp read can set against his reaction).
    fn words(self) -> Option<bool> {
        match self {
            Ev::BroughtUp | Ev::AskedLater | Ev::TookWell | Ev::Doubted | Ev::ToldFirst | Ev::ToldSecond => Some(true),
            Ev::Cool => Some(false),
            _ => None,
        }
    }
}

/// Where you saw it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Where {
    Call,
    Home,
    Campus,
    Message,
    /// A coach of yours: a talk you handed him, or his calls.
    Staff,
}

impl Where {
    pub fn words(self) -> &'static str {
        match self {
            Where::Call => "call",
            Where::Home => "home visit",
            Where::Campus => "his visit",
            Where::Message => "message",
            Where::Staff => "your staff",
        }
    }
}

/// A row of evidence on his file.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Pull {
    /// Whom it is about (player id).
    pub who: u32,
    /// The group (`Group` order).
    pub group: u8,
    pub ev: Ev,
    pub from: Where,
    pub on: Date,
    /// The coach who saw it, when it was your staff's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub by: Option<usize>,
    /// Whose wants it speaks for: the recruit himself, or whoever decides
    /// at his home.
    #[serde(default)]
    pub decider: bool,
}

impl Pull {
    pub fn group(&self) -> Group {
        Group::ALL[(self.group as usize).min(Group::ALL.len() - 1)]
    }
}

/// One group as his file reads it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Read {
    pub group: Group,
    /// How strongly the evidence puts it up (or down) his list. A file's
    /// number, read from what you saw, never his own.
    pub score: f32,
    /// How many rows speak to it.
    pub rows: u8,
    /// His words and his reactions disagree about it.
    pub conflict: bool,
}

/// The evidence on every recruit's and transfer's file.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Pulls {
    rows: Vec<Pull>,
}

impl Pulls {
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// A row seen today. Old rows go, and a person keeps his newest
    /// `ROWS_EACH`.
    pub fn note(&mut self, today: Date, row: Pull) {
        self.rows.retain(|r| r.on.days_to(today) <= KEEP_DAYS);
        let who = row.who;
        self.rows.push(row);
        let n = self.rows.iter().filter(|r| r.who == who).count();
        if n > ROWS_EACH {
            let mut drop = n - ROWS_EACH;
            self.rows.retain(|r| {
                if drop > 0 && r.who == who {
                    drop -= 1;
                    false
                } else {
                    true
                }
            });
        }
    }

    /// Everything on one file, oldest first.
    pub fn of(&self, who: u32) -> impl Iterator<Item = &Pull> {
        self.rows.iter().filter(move |r| r.who == who)
    }

    /// His file's order of the seven groups, strongest first. Each group
    /// reads: what he asked about (every talk's questions add up), the most
    /// he told you of it, and how he takes your points there, one reading
    /// however many points you made (the average of his words, or of what
    /// your read caught where it caught anything: a sharp read takes the
    /// place of his words). Groups with no evidence come last, in their
    /// own order. `decider` reads the rows that speak for whoever decides
    /// at his home (`false`: for the recruit himself).
    pub fn order(&self, who: u32, decider: Option<bool>) -> Vec<Read> {
        let mut out: Vec<Read> =
            Group::ALL.iter().map(|&g| Read { group: g, score: 0.0, rows: 0, conflict: false }).collect();
        let mut asked = [0.0f32; 7];
        let mut told = [0.0f32; 7];
        let mut words = [(0.0f32, 0u16); 7];
        let mut caught = [(0.0f32, 0u16); 7];
        let mut said = [(false, false); 7];
        let mut seen = [(false, false); 7];
        for r in self.of(who).filter(|r| r.ev != Ev::Decides && decider.is_none_or(|d| d == r.decider)) {
            let k = r.group as usize % 7;
            let x = r.ev.weight();
            out[k].rows = out[k].rows.saturating_add(1);
            match r.ev {
                Ev::BroughtUp | Ev::AskedLater | Ev::Guess => asked[k] += x,
                Ev::ToldFirst | Ev::ToldSecond => told[k] = told[k].max(x),
                Ev::CaughtLit | Ev::CaughtFlat => {
                    caught[k].0 += x;
                    caught[k].1 += 1;
                }
                Ev::TookWell | Ev::Polite | Ev::Cool | Ev::Doubted => {
                    words[k].0 += x;
                    words[k].1 += 1;
                }
                Ev::Decides => {}
            }
            match r.ev.words() {
                Some(true) => said[k].0 = true,
                Some(false) => said[k].1 = true,
                None => {}
            }
            match r.ev {
                Ev::CaughtLit => seen[k].0 = true,
                Ev::CaughtFlat => seen[k].1 = true,
                _ => {}
            }
        }
        for k in 0..7 {
            let took = if caught[k].1 > 0 {
                caught[k].0 / caught[k].1 as f32
            } else if words[k].1 > 0 {
                words[k].0 / words[k].1 as f32
            } else {
                0.0
            };
            out[k].score = asked[k] + told[k] + took;
            out[k].conflict = (said[k].0 && seen[k].1) || (said[k].1 && seen[k].0);
        }
        out.sort_by(|a, b| {
            (b.rows > 0).cmp(&(a.rows > 0)).then(b.score.total_cmp(&a.score)).then(a.group.cmp(&b.group))
        });
        out
    }

    /// The group his file puts first, once anything speaks to it.
    pub fn top(&self, who: u32, decider: Option<bool>) -> Option<Group> {
        self.order(who, decider).first().filter(|r| r.rows > 0 && r.score > 0.0).map(|r| r.group)
    }

    /// Who decides at his home, as you were last told: (`Decider` order,
    /// where, the day).
    pub fn decides(&self, who: u32) -> Option<(u8, Where, Date)> {
        self.of(who).filter(|r| r.ev == Ev::Decides).last().map(|r| (r.group, r.from, r.on))
    }

    /// Drops a person's rows.
    pub fn forget(&mut self, who: u32) {
        self.rows.retain(|r| r.who != who);
    }
}

/// Whose wants decide for him: whoever decides at home for a high school
/// recruit, himself in the portal (`Ear` for what pulls him).
pub fn deciding_ear(w: &World, i: usize) -> Ear {
    let p = &w.people[i];
    if p.cal.portal.is_some() || p.stage != crate::world::Stage::HighSchool { Ear::Himself } else { Ear::Decider }
}

/// Your area recruiter's free guess at what pulls him most (section 7,
/// "What others tell you"), from his calls to the kid's coaches, teachers
/// and counselor: once a call is on the file, one group, right `GUESS_GOOD`
/// of the time for a good recruiter and `GUESS_POOR` for a poor one. A
/// wrong guess names the group his wants put second, else another. The
/// recruiter who made the call, and the group.
pub fn guess(w: &World, i: usize) -> Option<(usize, Group)> {
    use crate::world::scouting::Kind as Look;
    use crate::world::scouting::readers::COACH;
    let p = &w.people[i];
    let id = p.player.id;
    let call = w
        .scouting
        .dept
        .looks
        .get(&id)?
        .iter()
        .rev()
        .find(|l| matches!(l.kind, Look::Call | Look::OldStaff) && l.who >= COACH && l.who - COACH < 1 << 16)?;
    let coach = (call.who - COACH) as usize;
    if coach >= w.coaches.len() {
        return None;
    }
    let care = super::claims::group_cares(p, deciding_ear(w, i));
    let mut rank: Vec<usize> = (0..7).collect();
    rank.sort_by(|&a, &b| care[b].total_cmp(&care[a]).then(a.cmp(&b)));
    let r = w.coaches[coach].ability().recruiting;
    let t = ((r - GUESS_RATINGS.0) / (GUESS_RATINGS.1 - GUESS_RATINGS.0)).clamp(0.0, 1.0);
    let right = GUESS_POOR + (GUESS_GOOD - GUESS_POOR) * t;
    let roll = super::roll(w, 0x9E55 ^ coach as u64, id as u64);
    let k = if roll < right {
        rank[0]
    } else {
        // Wrong: the second most of the time, else one of the rest.
        let rest = super::roll(w, 0x9E56 ^ coach as u64, id as u64);
        if rest < 0.6 { rank[1] } else { rank[2 + (rest * 1e6) as usize % 5] }
    };
    Some((coach, Group::ALL[k]))
}

/// Whether the rows that decide for him speak for whoever decides at his
/// home (`Pull.decider`): a high school recruit whose family decides.
pub fn decider_rows(w: &World, i: usize) -> bool {
    deciding_ear(w, i) == Ear::Decider && w.people[i].family.decider != crate::world::background::Decider::Himself
}

/// His file as it reads today (section 7): the evidence of your talks for
/// whoever decides for him, with your area recruiter's free guess counted
/// as one more row (`WEIGHT_GUESS`) until the talks outweigh it.
pub fn file(w: &World, i: usize) -> Vec<Read> {
    let id = w.people[i].player.id;
    let mut out = w.folk.pulls.order(id, Some(decider_rows(w, i)));
    if let Some((_, g)) = guess(w, i)
        && let Some(r) = out.iter_mut().find(|r| r.group == g)
    {
        r.score += WEIGHT_GUESS;
        r.rows = r.rows.saturating_add(1);
    }
    out.sort_by(|a, b| (b.rows > 0).cmp(&(a.rows > 0)).then(b.score.total_cmp(&a.score)).then(a.group.cmp(&b.group)));
    out
}
