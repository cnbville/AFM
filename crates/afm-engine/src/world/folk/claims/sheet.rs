//! The sheet and its talking points (talk facts design, section 3), and
//! the sizes every talk shares (section 4's rules 5 and 6).

use super::kinds::{Group, Kind, When, Whose};
use crate::calendar::Date;
use crate::world::folk::Team;
use crate::world::folk::voice::Facts;

/// How fresh a dated point still is on the last day of its window: it
/// fades in a straight line from 1 on the day itself. Proposed, not
/// confirmed.
pub const FRESH_FLOOR: f32 = 0.4;
/// Each time you have talked a point through, it weighs this much less on
/// his mind. Proposed, not confirmed.
pub const TALKED_FADE: f32 = 0.5;
/// A point counts as on his mind when it is his top one, or this close
/// under it. Proposed, not confirmed.
pub const NEAR_TOP: f32 = 0.1;

/// Your proof by tier (section 6, "How much it counts"). Proposed, not
/// confirmed.
pub const PROOF: [f32; 4] = [1.0, 0.75, 0.45, 0.15];
/// A draft pick whose coaches have left counts this much (section 6: his
/// question is what you did for him). Proposed, not confirmed.
pub const ALUM_OTHERS: f32 = 0.6;
/// The best on his list "clearly": ahead of the next school by this share
/// of the spread on his list. Proposed, not confirmed.
pub const ELITE_LEAD: f32 = 0.15;

/// What a move rests on (section 5, "How much it counts"; rule 5).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Basis {
    /// A true fact that is on his mind.
    OnMind,
    /// A true fact that isn't on his mind.
    True,
    /// Nothing: the plain version.
    Plain,
    /// Something he knows is false or unfair: a warning to a man who did
    /// nothing, praise for nothing.
    Against,
}

/// The multipliers of `Basis`, each a size Cain can change. Proposed, not
/// confirmed.
pub const ON_MIND: f32 = 1.3;
/// A true fact that isn't on his mind. Proposed, not confirmed.
pub const TRUE_FACT: f32 = 1.0;
/// The plain version, with no fact behind it. Proposed, not confirmed.
pub const PLAIN: f32 = 0.7;
/// A move against the facts loses its lift and costs trust: this much,
/// up to `AGAINST_MOST` for a big ego.
/// Proposed, not confirmed.
pub const AGAINST_LEAST: f32 = 0.03;
/// The most a move against the facts costs. Proposed, not confirmed.
pub const AGAINST_MOST: f32 = 0.06;

impl Basis {
    /// What the move's lift is multiplied by.
    pub fn lift(self) -> f32 {
        match self {
            Basis::OnMind => ON_MIND,
            Basis::True => TRUE_FACT,
            Basis::Plain => PLAIN,
            Basis::Against => 0.0,
        }
    }

    /// The trust a move against the facts costs: 0.03, rising with his ego
    /// (z-score) to 0.06. Nothing for any other basis.
    pub fn trust_cost(self, ego_z: f64) -> f32 {
        match self {
            Basis::Against => {
                let t = (ego_z as f32 / 2.0).clamp(0.0, 1.0);
                AGAINST_LEAST + (AGAINST_MOST - AGAINST_LEAST) * t
            }
            _ => 0.0,
        }
    }
}

/// How strong your proof is next to his other top schools (section 6).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Tier {
    Weak,
    Fair,
    Strong,
    Elite,
}

impl Tier {
    pub const ALL: [Tier; 4] = [Tier::Elite, Tier::Strong, Tier::Fair, Tier::Weak];
    pub fn key(self) -> &'static str {
        match self {
            Tier::Elite => "elite",
            Tier::Strong => "strong",
            Tier::Fair => "fair",
            Tier::Weak => "weak",
        }
    }
    /// The proof value the landing formula reads.
    pub fn value(self) -> f32 {
        match self {
            Tier::Elite => PROOF[0],
            Tier::Strong => PROOF[1],
            Tier::Fair => PROOF[2],
            Tier::Weak => PROOF[3],
        }
    }
    /// A rank that can be stored: 4 elite to 1 weak (0 means none).
    pub fn rank(self) -> u8 {
        match self {
            Tier::Elite => 4,
            Tier::Strong => 3,
            Tier::Fair => 2,
            Tier::Weak => 1,
        }
    }

    /// Your fact graded against the same fact at his other top schools:
    /// elite when it is the best on his list by a clear lead, strong among
    /// the top two, fair in the middle, weak below the middle. With no
    /// other school on his list, the fact's own 0-1 size is read as the
    /// grade (as the pitch reads the program's grades today).
    pub fn grade(mine: f32, others: &[f32]) -> Tier {
        if others.is_empty() {
            return Tier::of(mine);
        }
        let above = others.iter().filter(|&&o| o > mine).count();
        let best = others.iter().copied().fold(f32::MIN, f32::max);
        let worst = others.iter().copied().fold(mine, f32::min);
        let spread = (best.max(mine) - worst).max(1e-6);
        // "Below the middle": more of his other schools beat you than not.
        if above == 0 {
            if (mine - best) / spread >= ELITE_LEAD { Tier::Elite } else { Tier::Strong }
        } else if above == 1 {
            Tier::Strong
        } else if above * 2 <= others.len() {
            Tier::Fair
        } else {
            Tier::Weak
        }
    }

    /// A 0-1 size read as a tier with nothing to compare against.
    pub fn of(x: f32) -> Tier {
        match x {
            x if x >= 0.85 => Tier::Elite,
            x if x >= 0.6 => Tier::Strong,
            x if x >= 0.35 => Tier::Fair,
            _ => Tier::Weak,
        }
    }
}

/// Your proof on one point to one recruit. Engine numbers: none of them
/// is shown. A screen shows your staff's read of the tier, and what the
/// file has learned of how much he cares (section 7).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Proof {
    /// The fact's own size, 0-1, the same reading at every school.
    pub strength: f32,
    pub tier: Tier,
    /// How many of his other top schools it was graded against.
    pub against: u8,
}

/// Someone or something a point names, for links and for code.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ref {
    Player(u32),
    Coach(usize),
    Team(Team),
}

/// One talking point: a fact that is true today, with what a line needs
/// to say it.
#[derive(Clone, Debug)]
pub struct Point {
    pub kind: Kind,
    pub whose: Whose,
    /// The day it happened; for something ahead, the day it comes.
    pub day: Option<Date>,
    /// The day the other person can check it, for a kind that is checked
    /// on a day (`Kind::when`).
    pub due: Option<Date>,
    /// The real values: the holes of a line (`{charge}`, `{rival}`).
    values: Vec<(&'static str, String)>,
    /// The state: the checks that pick the words (`step=charged`).
    state: Vec<(&'static str, &'static str)>,
    /// How much it is on his mind today, 0-1. An engine number: it picks
    /// what he opens on and what a move is worth, and is never shown.
    pub mind: f32,
    /// Whether you know it. A point you don't know is on his sheet and
    /// not on the table.
    pub known: bool,
    /// Who it names.
    pub refs: Vec<Ref>,
    /// Your proof, on a recruit's point.
    pub proof: Option<Proof>,
    /// How much he cares about this kind of thing, 1.0 for an average
    /// man: his traits on a player's point, the listener's wants on a
    /// recruit's. An engine number, already weighed into `mind`; the
    /// landing formula reads it (rule 5), and it is never shown.
    pub care: f32,
    /// What only he knows (whether he did it, what he plans): the engine
    /// reads it to play his side, and it is never a line's fact. `fill`
    /// leaves it out.
    hidden: Vec<(&'static str, &'static str)>,
}

impl Point {
    pub fn new(kind: Kind) -> Point {
        Point {
            kind,
            whose: kind.whose(),
            day: None,
            due: None,
            values: Vec::new(),
            state: Vec::new(),
            mind: kind.size(),
            known: true,
            refs: Vec::new(),
            proof: None,
            care: 1.0,
            hidden: Vec::new(),
        }
    }

    pub fn on(mut self, day: Date) -> Point {
        self.day = Some(day);
        self
    }
    pub fn due(mut self, day: Date) -> Point {
        self.due = Some(day);
        self
    }
    pub fn whose(mut self, w: Whose) -> Point {
        self.whose = w;
        self
    }
    pub fn unknown(mut self) -> Point {
        self.known = false;
        self
    }
    pub fn known_if(mut self, known: bool) -> Point {
        self.known = known;
        self
    }
    pub fn refer(mut self, r: Ref) -> Point {
        self.refs.push(r);
        self
    }
    pub fn proof(mut self, strength: f32, tier: Tier, against: usize) -> Point {
        self.proof = Some(Proof { strength, tier, against: against.min(255) as u8 });
        self.state.retain(|s| s.0 != "tier");
        self.state.push(("tier", tier.key()));
        self
    }

    /// A name, a word or a phrase for a hole. An empty one is left out, so
    /// a line that needs the hole is skipped (`voice`, a version whose
    /// fact is missing).
    pub fn text(mut self, hole: &'static str, v: impl Into<String>) -> Point {
        let v = v.into();
        if !v.is_empty() {
            self.values.retain(|x| x.0 != hole);
            self.values.push((hole, v));
        }
        self
    }
    /// A whole count: games, weeks, picks, seats, miles, years.
    pub fn count(self, hole: &'static str, n: i64) -> Point {
        self.text(hole, n.to_string())
    }
    /// A measured number, the kind a stopwatch, a bar or a scale gives (a
    /// forty, a bench, a weight): the only numbers with decimals a point
    /// may carry.
    pub fn measured(self, hole: &'static str, v: f64, decimals: usize) -> Point {
        self.text(hole, format!("{v:.decimals$}"))
    }
    /// A check that picks the words.
    pub fn state(mut self, check: &'static str, v: &'static str) -> Point {
        self.state.retain(|s| s.0 != check);
        self.state.push((check, v));
        self
    }
    /// Something only he knows. It never reaches a line (`fill` skips
    /// it); his side of a talk reads it with `hid`.
    pub fn hide(mut self, what: &'static str, v: &'static str) -> Point {
        self.hidden.retain(|s| s.0 != what);
        self.hidden.push((what, v));
        self
    }
    /// How much he cares about it (1.0: an average man), and with it how
    /// much it is on his mind.
    pub fn care(mut self, c: f32) -> Point {
        self.care *= c;
        self.weigh(c)
    }
    /// Scales how much it is on his mind (how big this one is, how fresh),
    /// kept in 0-1.
    pub fn weigh(mut self, by: f32) -> Point {
        self.mind = (self.mind * by).clamp(0.0, 1.0);
        self
    }

    /// The value of a hole.
    pub fn get(&self, hole: &str) -> Option<&str> {
        self.values.iter().find(|x| x.0 == hole).map(|x| x.1.as_str())
    }
    /// The value of a check.
    pub fn check(&self, check: &str) -> Option<&'static str> {
        self.state.iter().find(|x| x.0 == check).map(|x| x.1)
    }
    pub fn is(&self, check: &str, v: &str) -> bool {
        self.check(check) == Some(v)
    }
    pub fn values(&self) -> &[(&'static str, String)] {
        &self.values
    }
    pub fn states(&self) -> &[(&'static str, &'static str)] {
        &self.state
    }
    /// What only he knows, for the code that plays his side. Never put it
    /// in a line, a label or a hint.
    pub fn hid(&self, what: &str) -> Option<&'static str> {
        self.hidden.iter().find(|x| x.0 == what).map(|x| x.1)
    }
    pub fn tier(&self) -> Option<Tier> {
        self.proof.map(|p| p.tier)
    }
    /// When the other person can check a claim about it: the kind's rule,
    /// and the day for a kind checked on a day.
    pub fn when(&self) -> (When, Option<Date>) {
        (self.kind.when(), self.due)
    }

    /// Puts its holes and checks into a line's facts, so the bank says
    /// only versions whose facts are there and whose checks hold.
    pub fn fill(&self, f: &mut Facts) {
        f.set("point", self.kind.key());
        for (k, v) in &self.values {
            f.set(k, v.clone());
        }
        for (k, v) in &self.state {
            f.set(k, *v);
        }
    }

    /// A line's facts with this point on top of `base` (the speaker's
    /// name, your name, the team).
    pub fn facts(&self, base: &Facts) -> Facts {
        let mut f = base.clone();
        self.fill(&mut f);
        f
    }
}

/// What the world backs about a man's own conduct today: what a line that
/// denies or admits may stand on (the line bank's `blameless` and
/// `at_fault` facts).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Conduct {
    /// A rule he broke, a suspension, a conviction or a plea, or your
    /// staff's flag on his effort: a line may admit, none may deny.
    AtFault,
    /// Nothing against him: a line may deny, none may admit.
    Blameless,
    /// A case still open: whether he did it is his alone to know until
    /// the court day, so no line denies or admits.
    Open,
}

/// The fact sheet: every talking point that is true today for the person
/// across from you and for you, the one most on his mind first.
#[derive(Clone, Debug)]
pub struct Sheet {
    pub day: Date,
    /// The player, recruit or transfer it is about (player id); none for
    /// a team's sheet.
    pub about: Option<u32>,
    /// Your side: the team the talk is held for.
    pub team: Option<Team>,
    pub points: Vec<Point>,
}

impl Sheet {
    pub fn new(day: Date, about: Option<u32>, team: Option<Team>) -> Sheet {
        Sheet { day, about, team, points: Vec::new() }
    }

    pub(super) fn push(&mut self, p: Option<Point>) {
        if let Some(p) = p {
            self.points.push(p);
        }
    }

    /// Orders the points: most on his mind first, then the catalogue's
    /// order, so a sheet reads the same every time it is built.
    pub(super) fn settle(mut self) -> Sheet {
        self.points
            .sort_by(|a, b| b.mind.partial_cmp(&a.mind).unwrap_or(std::cmp::Ordering::Equal).then(a.kind.cmp(&b.kind)));
        self
    }

    /// The first point of a kind (the one most on his mind).
    pub fn get(&self, k: Kind) -> Option<&Point> {
        self.points.iter().find(|p| p.kind == k)
    }
    pub fn has(&self, k: Kind) -> bool {
        self.get(k).is_some()
    }
    /// Every point of a kind (two promises, two newcomers).
    pub fn all(&self, k: Kind) -> impl Iterator<Item = &Point> {
        self.points.iter().filter(move |p| p.kind == k)
    }
    /// What is most on his mind today, whether you know it or not (rule
    /// 3). His own and shared points only: your proof is not on his mind.
    pub fn top(&self) -> Option<&Point> {
        self.points.iter().find(|p| p.whose != Whose::Yours)
    }
    /// The points you know: what can be on the table (rule 1).
    pub fn known(&self) -> impl Iterator<Item = &Point> {
        self.points.iter().filter(|p| p.known)
    }
    /// Your side of the sheet: what your answers are built from (rule 4).
    pub fn yours(&self) -> impl Iterator<Item = &Point> {
        self.points.iter().filter(|p| p.whose != Whose::His)
    }
    /// A recruit's points in one group.
    pub fn group(&self, g: Group) -> impl Iterator<Item = &Point> {
        self.points.iter().filter(move |p| p.kind.group() == Some(g))
    }
    /// Your best point in a group: the strongest tier, then the bigger
    /// fact (a case counts each group at its best point).
    pub fn best(&self, g: Group) -> Option<&Point> {
        self.group(g).filter(|p| p.proof.is_some()).max_by(|a, b| {
            let (x, y) = (a.proof.unwrap(), b.proof.unwrap());
            x.tier.cmp(&y.tier).then(x.strength.partial_cmp(&y.strength).unwrap_or(std::cmp::Ordering::Equal))
        })
    }

    /// What a move about a kind of point rests on (rule 5): a fact on his
    /// mind, a fact, or nothing. `Basis::Against` is the caller's to say:
    /// only the tree knows a move claims something the sheet lacks (a
    /// warning with no trouble behind it).
    pub fn basis(&self, k: Kind) -> Basis {
        let Some(p) = self.get(k) else { return Basis::Plain };
        match self.top() {
            Some(t) if t.kind == k || p.mind >= t.mind - NEAR_TOP => Basis::OnMind,
            _ => Basis::True,
        }
    }

    /// Whether anything on the sheet backs a warning or a discipline talk.
    pub fn trouble(&self) -> bool {
        self.points.iter().any(|p| p.kind.is_trouble())
    }

    /// What the world backs about his conduct, on a person's sheet. A case
    /// that ended in an acquittal or with the charges dropped is not held
    /// against him. It reads what is public and never what only he knows
    /// (`Point::hid`): an open case answers `Conduct::Open` whether he did
    /// it or not.
    pub fn conduct(&self) -> Conduct {
        let fault = self.points.iter().any(|p| match p.kind {
            Kind::RuleBreak | Kind::Suspended | Kind::Effort => true,
            Kind::CaseClosed => p.is("outcome", "convicted") || p.is("outcome", "pleaded"),
            _ => false,
        });
        if fault {
            Conduct::AtFault
        } else if self.has(Kind::Arrest) {
            Conduct::Open
        } else {
            Conduct::Blameless
        }
    }

    /// Sets the line bank's conduct fact for a talk with him: `at_fault`
    /// or `blameless`, or neither while a case is open, so a line that
    /// denies or admits is said only when the world backs it.
    pub fn fill_conduct(&self, f: &mut Facts) {
        match self.conduct() {
            Conduct::AtFault => {
                f.set("at_fault", "yes");
            }
            Conduct::Blameless => {
                f.set("blameless", "yes");
            }
            Conduct::Open => {}
        }
    }
}

/// How fresh a dated point is: 1 on the day, `FRESH_FLOOR` on the last
/// day of its window, none after it. A point with no window is as fresh
/// as the day it became true.
pub fn fresh(kind: Kind, day: Option<Date>, today: Date) -> Option<f32> {
    let win = kind.window();
    let Some(day) = day else { return Some(1.0) };
    if win == 0 {
        return Some(1.0);
    }
    let age = day.days_to(today);
    if age < 0 || age > win {
        return None;
    }
    Some(1.0 - (1.0 - FRESH_FLOOR) * age as f32 / win as f32)
}

/// Whether a dated thing is still inside its kind's window today (rule 9:
/// nothing is named past its window).
pub fn in_window(kind: Kind, day: Date, today: Date) -> bool {
    let age = day.days_to(today);
    age >= 0 && (kind.window() == 0 || age <= kind.window())
}
