//! Promises on how you'll use him (agreed with Cain 2026-11-14: the 9
//! football promise items, "as an baseline for every position with their
//! own promises"; the per-position reading in `docs/design/
//! live-talks-persona.md`, section 8.4, proposed, not confirmed).
//!
//! - **A term** is a measure on the play log (his snaps, carries, targets,
//!   routes, rushes, stunts, blitzes, man snaps, kicks, returns...), a
//!   number from your words, a count a game or a share of his chances, at
//!   least or at most, with the situations you named (a down, the red zone,
//!   20+ air yards, a package, a look, a spot he lines up at).
//! - **Judged game by game** on your own games' plays (`World::seat_games`),
//!   through the promises design's rules (`promise.rs`): kept in 3 of every
//!   4 healthy games by its date. A term that says "never" (the kicker's
//!   field goal line: no punt from inside it) breaks on the first game that
//!   breaks it.
//! - **Only what you control** is judged: your calls, where you line him
//!   up, his assignment, his touches. Chips and doubles are the other
//!   team's call: kept and shown, never promised (agreed item 4).
//! - **A plan setting keeps it** (`roster::Keep`): set from your open
//!   promises before each game your team plays (`keep`), never for an AI
//!   team. Measures with no setting are kept by your depth chart and your
//!   calls.
//! - **Belief**: a number past what his football says is real is believed
//!   less by a sharp man and more by a big ego (agreed item 8; `likely`).
//!
//! Every size here is Claude's (proposed, not confirmed).

use crate::attributes::Attr;
use crate::game::chart::Protection;
use crate::game::playbook::{Coverage, PASS_CONCEPTS, PassConcept, PassDepth, RUN_CONCEPTS, RunConcept};
use crate::game::roster::Keep;
use crate::game::stats::{PlayRecord, PlayType, StatLine};
use crate::player::Trait;
use crate::positions::Position;
use crate::world::World;
use crate::world::tally::depth::{dslot, orole};

/// What a term counts.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum What {
    /// His snaps on his side of the ball (a share: of his team's snaps).
    #[default]
    Snaps,
    Carries,
    Targets,
    /// Carries and targets.
    Touches,
    /// Pass plays he went out on a route (not kept in to block).
    Routes,
    /// His throws (a quarterback).
    Throws,
    /// Runs called for him (a quarterback's, not scrambles).
    Designed,
    /// Screens thrown to him.
    Screens,
    /// His dropbacks with seven or more kept in to block (your games).
    Protected,
    /// His pass snaps with help: a second blocker on his man, the slide to
    /// his side or a chip on his man (a lineman; your games).
    Help,
    /// Pass plays he rushed (your games).
    Rushes,
    /// His rushes in a stunt (your games).
    Stunts,
    /// Dropbacks he blitzed: rushed from off the line (your games).
    Blitzes,
    /// His pass snaps in man coverage.
    Man,
    /// His tries at a field goal.
    FieldGoals,
    /// Your team's punts (the kicker's field goal line).
    TeamPunts,
    /// His punts.
    Punts,
    Kickoffs,
    Returns,
    /// His snaps on kicks.
    Teams,
    /// His team's snaps after his first turnover or bust in a game: not
    /// pulled after a mistake.
    AfterMistake,
    /// Chips on his rushes: shown, never promised (the other team's call).
    Chips,
    /// Double teams on him: shown, never promised.
    Doubles,
    // Lane N2: the six readers kept on every snap.
    /// His snaps at quarterback with at least this freedom at the line
    /// (`kind`: 2 a kill call, 3 full audibles).
    Freedom,
    /// His carries in a run family (`kind`: 1 inside zone, 2 outside zone,
    /// 3 gap, 4 option); a man who carried none, the runs he was on for (a
    /// guard's gap scheme).
    RunFamily,
    /// Targets on quick-game and RPO throws.
    QuickTargets,
    /// His outside corner snaps on a side by the hash (`kind`: 1 the
    /// boundary, 2 the field).
    Side,
    /// His pass snaps with two deep: help over the top.
    TwoHigh,
    /// His pass snaps as the post safety: single-high, at free safety.
    Post,
    /// His pass snaps as the robber: Cover 1 robber, at strong safety.
    Robber,
    /// His team's field goal and extra point tries he kicked (the only
    /// kicker).
    PlaceKicks,
    /// His pass snaps spying the quarterback (a spy call, at Mike).
    Spy,
    // Lane N2: season terms for incentives, NIL clauses, conditional picks
    // and staff and boss goals (`terms.rs`; `Usage::season`).
    /// A season stat line number (`kind`: the index in `terms::LINES`).
    Stat,
    /// His starts, his games played.
    Starts,
    Games,
    /// An honor (`kind`: `terms::HONORS`).
    Honor,
    /// His team's wins, a postseason, a title.
    Wins,
    Postseason,
    Title,
    /// His team's rank in its division (`kind`: `terms::RANKS`), at most
    /// `n`.
    Rank,
}

/// Where he lined up (`PlayRecord::roles`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Spot {
    #[default]
    Any,
    /// Outside: the X or the Z.
    Wide,
    X,
    Z,
    Slot,
    /// A tight end's spot on the line.
    Inline,
    Backfield,
    LeftTackle,
    RightTackle,
    Guard,
    Center,
    /// An end (LE or RE).
    End,
    LeftEnd,
    RightEnd,
    /// A three-technique or a one (DT, not the nose).
    Tackle,
    Nose,
    Mike,
    /// Sam or Will, off the ball.
    Outside,
    /// An outside corner (not the slot).
    Corner,
    Nickel,
    Free,
    Strong,
    // One key for each role a snap keeps (the unions above stay).
    Tailback,
    Fullback,
    LeftGuard,
    RightGuard,
    /// The defensive tackles by side (DTL, DTR).
    LeftDt,
    RightDt,
    Sam,
    Will,
    LeftCorner,
    RightCorner,
    Dime,
    /// The nickel or the dime: a corner inside.
    InsideCorner,
}

impl Spot {
    pub const KEYS: [(&'static str, Spot); 34] = [
        ("any", Spot::Any),
        ("wide", Spot::Wide),
        ("x", Spot::X),
        ("z", Spot::Z),
        ("slot", Spot::Slot),
        ("inline", Spot::Inline),
        ("backfield", Spot::Backfield),
        ("left_tackle", Spot::LeftTackle),
        ("right_tackle", Spot::RightTackle),
        ("guard", Spot::Guard),
        ("center", Spot::Center),
        ("end", Spot::End),
        ("left_end", Spot::LeftEnd),
        ("right_end", Spot::RightEnd),
        ("tackle", Spot::Tackle),
        ("nose", Spot::Nose),
        ("mike", Spot::Mike),
        ("outside_lb", Spot::Outside),
        ("corner", Spot::Corner),
        ("nickel", Spot::Nickel),
        ("free", Spot::Free),
        ("strong", Spot::Strong),
        ("tailback", Spot::Tailback),
        ("fullback", Spot::Fullback),
        ("left_guard", Spot::LeftGuard),
        ("right_guard", Spot::RightGuard),
        ("left_dt", Spot::LeftDt),
        ("right_dt", Spot::RightDt),
        ("sam", Spot::Sam),
        ("will", Spot::Will),
        ("left_corner", Spot::LeftCorner),
        ("right_corner", Spot::RightCorner),
        ("dime", Spot::Dime),
        ("inside_corner", Spot::InsideCorner),
    ];

    /// Offense spots (the rest are the defense's).
    fn offense(self) -> bool {
        matches!(
            self,
            Spot::Wide
                | Spot::X
                | Spot::Z
                | Spot::Slot
                | Spot::Inline
                | Spot::Backfield
                | Spot::LeftTackle
                | Spot::RightTackle
                | Spot::Guard
                | Spot::Center
                | Spot::Tailback
                | Spot::Fullback
                | Spot::LeftGuard
                | Spot::RightGuard
        )
    }

    /// He lined up here: his role's number on his side.
    fn holds(self, code: u8, offense: bool) -> bool {
        if self == Spot::Any {
            return true;
        }
        if offense != self.offense() {
            return false;
        }
        use dslot as d;
        use orole as o;
        match self {
            Spot::Any => true,
            Spot::Wide => code == o::X || code == o::Z,
            Spot::X => code == o::X,
            Spot::Z => code == o::Z,
            Spot::Slot => matches!(code, o::SLOT | o::SLOT2 | o::SLOT3),
            Spot::Inline => matches!(code, o::TE | o::TE2 | o::TE3),
            Spot::Backfield => code == o::RB || code == o::FB,
            Spot::LeftTackle => code == o::LT,
            Spot::RightTackle => code == o::RT,
            Spot::Guard => code == o::LG || code == o::RG,
            Spot::Center => code == o::C,
            Spot::End => code == d::LE || code == d::RE,
            Spot::LeftEnd => code == d::LE,
            Spot::RightEnd => code == d::RE,
            Spot::Tackle => code == d::DTL || code == d::DTR,
            Spot::Nose => code == d::NT,
            Spot::Mike => code == d::MIKE,
            Spot::Outside => code == d::SAM || code == d::WILL,
            Spot::Corner => code == d::CBL || code == d::CBR,
            Spot::Nickel => code == d::NCB,
            Spot::Free => code == d::FS,
            Spot::Strong => code == d::SS,
            Spot::Tailback => code == o::RB,
            Spot::Fullback => code == o::FB,
            Spot::LeftGuard => code == o::LG,
            Spot::RightGuard => code == o::RG,
            Spot::LeftDt => code == d::DTL,
            Spot::RightDt => code == d::DTR,
            Spot::Sam => code == d::SAM,
            Spot::Will => code == d::WILL,
            Spot::LeftCorner => code == d::CBL,
            Spot::RightCorner => code == d::CBR,
            Spot::Dime => code == d::DCB,
            Spot::InsideCorner => code == d::NCB || code == d::DCB,
        }
    }

    fn words(self) -> &'static str {
        match self {
            Spot::Any => "",
            Spot::Wide => "outside",
            Spot::X => "at the X",
            Spot::Z => "at the Z",
            Spot::Slot => "in the slot",
            Spot::Inline => "inline at tight end",
            Spot::Backfield => "in the backfield",
            Spot::LeftTackle => "at left tackle",
            Spot::RightTackle => "at right tackle",
            Spot::Guard => "at guard",
            Spot::Center => "at center",
            Spot::End => "at end",
            Spot::LeftEnd => "at left end",
            Spot::RightEnd => "at right end",
            Spot::Tackle => "at tackle, not the nose",
            Spot::Nose => "at the nose",
            Spot::Mike => "at Mike",
            Spot::Outside => "at Sam or Will",
            Spot::Corner => "at outside corner",
            Spot::Nickel => "at nickel",
            Spot::Free => "at free safety",
            Spot::Strong => "at strong safety",
            Spot::Tailback => "at tailback",
            Spot::Fullback => "at fullback",
            Spot::LeftGuard => "at left guard",
            Spot::RightGuard => "at right guard",
            Spot::LeftDt => "at the left tackle spot on defense",
            Spot::RightDt => "at the right tackle spot on defense",
            Spot::Sam => "at Sam",
            Spot::Will => "at Will",
            Spot::LeftCorner => "at left corner",
            Spot::RightCorner => "at right corner",
            Spot::Dime => "at dime",
            Spot::InsideCorner => "inside at nickel or dime",
        }
    }
}

/// The look filter's bits (`Usage::look`).
pub const PLAY_ACTION: u8 = 1;
pub const TEMPO: u8 = 2;
pub const GUN: u8 = 4;
pub const UNDER: u8 = 8;
pub const TWO_MINUTE: u8 = 16;

/// A usage term.
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Usage {
    pub what: What,
    /// Only snaps where he lined up here.
    pub spot: Spot,
    /// Only on this down (0 any).
    pub down: u8,
    /// Only inside this yard line (yards to their goal; 0 anywhere): 20
    /// the red zone, 5 the goal line.
    pub inside: u8,
    /// At least this far: air yards on a throw, the try's length on a
    /// field goal (0 any).
    pub far: u8,
    /// Only in this package: the defense's 1 base, 2 nickel, 3 dime, 4 goal
    /// line, 5 nickel or dime; the offense's personnel 10 to 23 (0 any).
    pub pkg: u8,
    /// Only plays with this look (`PLAY_ACTION`, `TEMPO`, `GUN`, `UNDER`,
    /// `TWO_MINUTE`; 0 any).
    pub look: u8,
    /// The number: a count a game, or a share of his chances.
    pub n: f32,
    pub share: bool,
    /// At most (a ceiling) rather than at least.
    pub most: bool,
    /// Which one, for a measure with kinds (lane N2): the freedom, the run
    /// family, the side, the stat, the honor, the rank (0 none).
    pub kind: u8,
    /// Judged on the season, not game by game (lane N2): an incentive, an
    /// NIL clause, a conditional pick, a staff or boss goal.
    pub season: bool,
}

// Usage keeps no NaN (every number comes from words, clamped), so it is
// Eq for the types that hold one (a pick's condition, a boss's goal).
impl Eq for Usage {}

/// One game's reading of a term: what counted and his chances.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Count {
    pub n: f64,
    pub of: f64,
}

impl Usage {
    /// A ceiling of nothing: one game that breaks it breaks the promise
    /// (the kicker's field goal line, agreed item 1).
    pub fn never(&self) -> bool {
        self.most && !self.share && self.n <= 0.0
    }

    /// About his team, not him (judged even in a game he sat out).
    pub fn team(&self) -> bool {
        matches!(self.what, What::TeamPunts | What::Wins | What::Postseason | What::Title | What::Rank)
    }

    /// A measure read off the season only (an outcome, an honor, the
    /// team's record): never a game-by-game promise.
    pub fn season_only(&self) -> bool {
        matches!(
            self.what,
            What::Stat
                | What::Starts
                | What::Games
                | What::Honor
                | What::Wins
                | What::Postseason
                | What::Title
                | What::Rank
        )
    }

    /// Kept only on your games' line chart (lane 9).
    pub fn charted(&self) -> bool {
        matches!(
            self.what,
            What::Protected | What::Help | What::Rushes | What::Stunts | What::Blitzes | What::Chips | What::Doubles
        )
    }

    /// The other team's call: shown, never promised (agreed item 4).
    pub fn shown_only(&self) -> bool {
        matches!(self.what, What::Chips | What::Doubles)
    }

    /// The kicker's field goal line: no punt from inside it (agreed item
    /// 1), with the try's length (the yard line plus 17).
    pub fn fg_line(yards: u8) -> Usage {
        Usage { what: What::TeamPunts, inside: yards.saturating_sub(SNAP_HOLD), most: true, n: 0.0, ..Usage::default() }
    }

    /// Whether a play passes the term's situations.
    fn fits(&self, r: &PlayRecord) -> bool {
        if self.down > 0 && r.down != self.down {
            return false;
        }
        if self.inside > 0 && r.yardline_100 > self.inside as f64 {
            return false;
        }
        if self.pkg > 0 {
            let ok = match self.pkg {
                1..=4 => r.dpers == self.pkg,
                5 => r.dpers == 2 || r.dpers == 3,
                p => r.personnel == p.to_string(),
            };
            if !ok {
                return false;
            }
        }
        if self.look & PLAY_ACTION != 0 && !r.play_action {
            return false;
        }
        if self.look & TEMPO != 0 && !r.hurry {
            return false;
        }
        if self.look & GUN != 0 && !r.call.contains(" gun ") {
            return false;
        }
        if self.look & UNDER != 0 && !r.call.contains("under center") {
            return false;
        }
        if self.look & TWO_MINUTE != 0 && !(matches!(r.quarter, 2 | 4) && r.clock <= 120.0) {
            return false;
        }
        true
    }

    /// His spot on a play, when he was on it on `offense`'s side.
    fn spot_ok(&self, r: &PlayRecord, id: u32, offense: bool) -> bool {
        if self.spot == Spot::Any {
            return true;
        }
        let k = if offense { 0 } else { 1 };
        r.on_field[k]
            .iter()
            .position(|&x| x == id)
            .and_then(|i| r.roles[k].get(i))
            .is_some_and(|&c| self.spot.holds(c, offense))
    }

    /// What one game showed: `side` his team's index in the game (0 home).
    pub fn count(&self, plays: &[PlayRecord], side: usize, id: u32) -> Count {
        let mut c = Count::default();
        let live = |r: &&PlayRecord| !r.nullified && r.kind != PlayType::NoPlay;
        let ours = |r: &PlayRecord| r.offense == side;
        let on = |r: &PlayRecord, k: usize| r.on_field[k].contains(&id);
        let dropback = |r: &PlayRecord| matches!(r.kind, PlayType::Pass | PlayType::Sack | PlayType::Scramble);
        let scrimmage = |r: &PlayRecord| {
            matches!(r.kind, PlayType::Run | PlayType::Pass | PlayType::Sack | PlayType::Scramble | PlayType::TwoPoint)
        };
        let kick = |r: &PlayRecord| {
            matches!(r.kind, PlayType::Punt | PlayType::FieldGoal | PlayType::ExtraPoint | PlayType::Kickoff)
        };
        let mut add = |hit: bool, chance: bool| {
            if chance {
                c.of += 1.0;
                if hit {
                    c.n += 1.0;
                }
            }
        };
        match self.what {
            What::AfterMistake => {
                // From his first turnover or bust on, the share of his
                // side's snaps he stayed on for.
                let mut since = false;
                for r in plays.iter().filter(live).filter(|r| scrimmage(r)) {
                    let off = ours(r);
                    let k = if off { 0 } else { 1 };
                    if since && (off == offense_side(plays, side, id)) {
                        add(on(r, k), true);
                    }
                    let his = r.busts.contains(&id)
                        || (r.interception && r.passer == Some(id))
                        || (r.fumble_lost && r.fumbled == Some(id));
                    since |= his;
                }
            }
            _ => {
                // A back's run family is read off his own carries; a man
                // who carried none (a guard) reads the runs he was on for.
                let carried = self.what == What::RunFamily
                    && plays.iter().any(|r| r.offense == side && r.kind == PlayType::Run && r.carrier == Some(id));
                for r in plays.iter().filter(live).filter(|r| self.fits(r)) {
                    let off = ours(r);
                    let chart = r.line.as_deref();
                    match self.what {
                        What::Snaps => {
                            if scrimmage(r) {
                                let me_off = offense_side(plays, side, id);
                                if off == me_off {
                                    let k = if off { 0 } else { 1 };
                                    add(on(r, k) && self.spot_ok(r, id, off), true);
                                }
                            }
                        }
                        What::Carries => {
                            if off && r.kind == PlayType::Run {
                                add(r.carrier == Some(id), true);
                            }
                        }
                        What::Targets => {
                            if off && r.kind == PlayType::Pass && r.passer.is_some() {
                                let far = self.far == 0 || r.air_yards >= self.far as f64;
                                add(r.target == Some(id) && far && self.spot_ok(r, id, true), true);
                            }
                        }
                        What::Touches => {
                            if off && matches!(r.kind, PlayType::Run | PlayType::Pass) {
                                add(r.carrier == Some(id) && r.kind == PlayType::Run || r.target == Some(id), true);
                            }
                        }
                        What::Routes => {
                            if off && dropback(r) && on(r, 0) {
                                let blocked = r.blocks.iter().any(|b| b.0 == id && b.2);
                                let kept = chart.is_some_and(|c| c.reps.iter().any(|x| x.blocker == id && x.pass()));
                                add(!blocked && !kept, true);
                            }
                        }
                        What::Throws => {
                            if off && dropback(r) {
                                let far = self.far == 0 || r.air_yards >= self.far as f64;
                                add(r.kind == PlayType::Pass && r.passer == Some(id) && far, true);
                            }
                        }
                        What::Designed => {
                            if off && r.kind == PlayType::Run {
                                add(r.carrier == Some(id) && !r.scramble, true);
                            }
                        }
                        What::Screens => {
                            if off && r.kind == PlayType::Pass {
                                add(r.target == Some(id) && r.call.to_lowercase().contains("screen"), true);
                            }
                        }
                        What::Protected => {
                            if off
                                && dropback(r)
                                && r.passer == Some(id)
                                && let Some(ch) = chart
                            {
                                add(ch.blockers >= 7, true);
                            }
                        }
                        What::Help => {
                            if off
                                && dropback(r)
                                && let Some(ch) = chart
                            {
                                let mine: Vec<_> = ch.reps.iter().filter(|x| x.blocker == id && x.pass()).collect();
                                if !mine.is_empty() {
                                    let double = mine.iter().any(|x| x.double());
                                    let chipped =
                                        mine.iter().any(|x| ch.rush.iter().any(|q| q.id == x.defender && q.chipped()));
                                    let code = r.on_field[0]
                                        .iter()
                                        .position(|&x| x == id)
                                        .and_then(|i| r.roles[0].get(i))
                                        .copied();
                                    let left = matches!(code, Some(orole::LT | orole::LG));
                                    let right = matches!(code, Some(orole::RT | orole::RG));
                                    let slide = (ch.protection == Protection::SlideLeft && left)
                                        || (ch.protection == Protection::SlideRight && right)
                                        || ch.protection == Protection::Max;
                                    add(double || chipped || slide, true);
                                }
                            }
                        }
                        What::Rushes | What::Blitzes => {
                            if !off
                                && dropback(r)
                                && on(r, 1)
                                && let Some(ch) = chart
                            {
                                let me = ch.rush.iter().find(|q| q.id == id);
                                let hit = match self.what {
                                    What::Rushes => me.is_some(),
                                    _ => me.is_some_and(|q| !q.lineman()),
                                };
                                add(hit, true);
                            }
                        }
                        What::Stunts | What::Chips => {
                            if !off
                                && dropback(r)
                                && let Some(q) = chart.and_then(|ch| ch.rush.iter().find(|q| q.id == id))
                            {
                                add(if self.what == What::Stunts { q.stunt() } else { q.chipped() }, true);
                            }
                        }
                        What::Doubles => {
                            if !off
                                && dropback(r)
                                && let Some(ch) = chart.filter(|ch| ch.reps.iter().any(|x| x.defender == id))
                            {
                                add(ch.reps.iter().any(|x| x.defender == id && x.double()), true);
                            }
                        }
                        What::Man => {
                            if !off && dropback(r) && on(r, 1) && self.spot_ok(r, id, false) {
                                add(r.coverage_man, true);
                            }
                        }
                        What::FieldGoals => {
                            if off && r.kind == PlayType::FieldGoal {
                                let len = r.yardline_100 + SNAP_HOLD as f64;
                                add(r.kicker == Some(id) && (self.far == 0 || len >= self.far as f64), true);
                            }
                        }
                        What::TeamPunts => {
                            if off && r.kind == PlayType::Punt {
                                add(true, true);
                            }
                        }
                        What::Punts => {
                            if off && r.kind == PlayType::Punt {
                                add(r.kicker == Some(id), true);
                            }
                        }
                        What::Kickoffs => {
                            if off && r.kind == PlayType::Kickoff {
                                add(r.kicker == Some(id), true);
                            }
                        }
                        What::Returns => {
                            if !off && matches!(r.kind, PlayType::Punt | PlayType::Kickoff) {
                                add(r.returner == Some(id), true);
                            }
                        }
                        What::Teams => {
                            if kick(r) {
                                add(r.on_field[0].contains(&id) || r.on_field[1].contains(&id), true);
                            }
                        }
                        What::Freedom => {
                            if off && scrimmage(r) && r.freedom > 0 && role(r, id, 0) == Some(orole::QB) {
                                add(r.freedom >= self.kind.max(2), true);
                            }
                        }
                        What::RunFamily => {
                            if off && r.kind == PlayType::Run && !r.scramble {
                                let chance = if carried { r.carrier == Some(id) } else { on(r, 0) };
                                let fam = run_of(&r.call).and_then(crate::game::calling::run_family);
                                add(fam.is_some_and(|f| f + 1 == self.kind as usize), chance);
                            }
                        }
                        What::QuickTargets => {
                            if off && r.kind == PlayType::Pass && r.passer.is_some() {
                                add(r.target == Some(id) && quick_call(&r.call) && self.spot_ok(r, id, true), true);
                            }
                        }
                        What::Side => {
                            if !off && scrimmage(r) && matches!(r.hash, 1 | 2) {
                                let me = role(r, id, 1);
                                if matches!(me, Some(dslot::CBL | dslot::CBR)) {
                                    let boundary = (r.hash == 1) == (me == Some(dslot::CBL));
                                    add(boundary == (self.kind != 2), true);
                                }
                            }
                        }
                        What::TwoHigh | What::Post | What::Robber | What::Spy => {
                            if !off && dropback(r) && on(r, 1) && self.spot_ok(r, id, false) {
                                let cov = coverage_of(&r.defense_call);
                                let me = role(r, id, 1);
                                let hit = match self.what {
                                    What::TwoHigh => cov.is_some_and(|c| c.high() == 2),
                                    What::Post => cov.is_some_and(|c| c.high() == 1) && me == Some(dslot::FS),
                                    What::Robber => cov == Some(Coverage::Cover1Robber) && me == Some(dslot::SS),
                                    _ => r.defense_call.contains("QB spy") && me == Some(dslot::MIKE),
                                };
                                add(hit, true);
                            }
                        }
                        What::PlaceKicks => {
                            if off && matches!(r.kind, PlayType::FieldGoal | PlayType::ExtraPoint) {
                                add(r.kicker == Some(id), true);
                            }
                        }
                        What::AfterMistake
                        | What::Stat
                        | What::Starts
                        | What::Games
                        | What::Honor
                        | What::Wins
                        | What::Postseason
                        | What::Title
                        | What::Rank => {}
                    }
                }
            }
        }
        c
    }

    /// One game against the term: Some(kept), or None when the game says
    /// nothing (a share with no chances: no dropbacks while he was on).
    pub fn game(&self, plays: &[PlayRecord], side: usize, id: u32, played: bool) -> Option<bool> {
        if self.season_only() {
            return None;
        }
        if !played && !self.team() {
            // He didn't play: no snaps, nothing kept, and a ceiling is safe.
            return Some(self.most);
        }
        let c = self.count(plays, side, id);
        let v = if self.share {
            if c.of <= 0.0 {
                return None;
            }
            c.n / c.of
        } else {
            if self.what == What::AfterMistake && c.of <= 0.0 {
                return None;
            }
            c.n
        };
        let bar = self.n as f64;
        Some(if self.most { v <= bar + 1e-9 } else { v + 1e-9 >= bar })
    }

    /// One game kept as a box score only (high school, lane N2): what its
    /// lines can say, his count and his chances (for a share, his team's
    /// count: its carries, targets, kicks; for snaps, the most any man on
    /// his side played). None for a term with a situation (a down, a zone,
    /// a spot, a package, a look, air yards), one read off the line chart,
    /// a team term, or a game whose box keeps no lines.
    pub fn box_count(&self, mates: &std::collections::BTreeMap<u32, StatLine>, id: u32) -> Option<Count> {
        if mates.is_empty()
            || self.down > 0
            || self.inside > 0
            || self.far > 0
            || self.pkg > 0
            || self.look > 0
            || self.spot != Spot::Any
            || self.charted()
            || self.team()
            || self.season_only()
        {
            return None;
        }
        let off = mates.get(&id).is_none_or(|l| l.snaps_off >= l.snaps_def);
        let get = |l: &StatLine| -> Option<f64> {
            Some(match self.what {
                What::Snaps => (if off { l.snaps_off } else { l.snaps_def }) as f64,
                What::Carries => l.rush as f64,
                What::Targets => l.targets as f64,
                What::Touches => (l.rush + l.targets) as f64,
                What::Throws => l.att as f64,
                What::FieldGoals => l.fg_att.iter().sum::<u32>() as f64,
                What::PlaceKicks => (l.fg_att.iter().sum::<u32>() + l.xp_att) as f64,
                What::Punts => l.punts as f64,
                What::Kickoffs => l.kickoffs as f64,
                What::Returns => l.kr as f64,
                What::Teams => l.snaps_st as f64,
                _ => return None,
            })
        };
        let n = mates.get(&id).map_or(Some(0.0), get)?;
        let of = if !self.share {
            1.0
        } else if matches!(self.what, What::Snaps | What::Teams) {
            mates.values().filter_map(get).fold(0.0, f64::max)
        } else {
            mates.values().filter_map(get).sum()
        };
        Some(Count { n, of })
    }

    /// A game read as a count against the term: Some(kept), or None when
    /// it says nothing.
    pub fn judge(&self, c: Count) -> Option<bool> {
        let v = if self.share {
            if c.of <= 0.0 {
                return None;
            }
            c.n / c.of
        } else {
            c.n
        };
        let bar = self.n as f64;
        Some(if self.most { v <= bar + 1e-9 } else { v + 1e-9 >= bar })
    }

    /// The term in your words, as he heard it.
    pub fn words(&self) -> String {
        let n = |x: f32| if (x - x.round()).abs() < 0.05 { format!("{}", x.round() as i32) } else { format!("{x:.1}") };
        if self.what == What::TeamPunts && self.never() {
            return if self.inside > 0 {
                format!(
                    "no punts inside the {} (a {}-yard try): the field goal unit goes on",
                    self.inside,
                    self.inside as u32 + SNAP_HOLD as u32
                )
            } else {
                "no punts".into()
            };
        }
        let thing = match self.what {
            What::Snaps => "snaps",
            What::Carries => "carries",
            What::Targets if self.far > 0 => "deep targets",
            What::Targets => "targets",
            What::Touches => "touches",
            What::Routes => "pass plays out on a route",
            What::Throws => "throws",
            What::Designed => "designed runs",
            What::Screens => "screens to him",
            What::Protected => "dropbacks with seven kept in to block",
            What::Help => "pass snaps with help",
            What::Rushes => "pass rushes",
            What::Stunts => "stunts",
            What::Blitzes => "blitzes",
            What::Man => "snaps in man coverage",
            What::FieldGoals => "field goal tries",
            What::TeamPunts => "punts",
            What::Punts => "punts",
            What::Kickoffs => "kickoffs",
            What::Returns => "returns",
            What::Teams => "special teams snaps",
            What::AfterMistake => "snaps after a mistake",
            What::Chips => "chips",
            What::Doubles => "double teams",
            What::Freedom => "snaps",
            What::RunFamily => "carries",
            What::QuickTargets => "quick-game and RPO targets",
            What::Side => "snaps",
            What::TwoHigh => "pass snaps with help over the top",
            What::Post => "pass snaps as the post safety",
            What::Robber => "pass snaps as the robber",
            What::PlaceKicks => "kicks",
            What::Spy => "pass snaps spying the quarterback",
            What::Stat
            | What::Starts
            | What::Games
            | What::Honor
            | What::Wins
            | What::Postseason
            | What::Title
            | What::Rank => {
                return super::terms::words(self);
            }
        };
        let of = match self.what {
            What::Snaps => "of the snaps",
            What::Carries => "of the carries",
            What::Targets | What::Screens => "of the targets",
            What::Touches => "of the plays",
            What::Routes => "of his pass snaps",
            What::Throws | What::Protected => "of the dropbacks",
            What::Designed => "of the runs",
            What::Help => "of his pass snaps",
            What::Rushes | What::Blitzes => "of the dropbacks he's on for",
            What::Stunts | What::Chips | What::Doubles => "of his pass rushes",
            What::Man => "of his pass snaps",
            What::AfterMistake => "of the snaps after a mistake",
            What::Freedom => "of his snaps at quarterback",
            What::RunFamily => "of his carries",
            What::Side => "of his snaps at outside corner",
            What::TwoHigh | What::Post | What::Robber | What::Spy => "of his pass snaps",
            What::PlaceKicks => "of the field goals and extra points",
            _ => "of the chances",
        };
        let mut out = if self.share {
            let pct = (self.n * 100.0).round() as i32;
            let how = if self.most { "at most" } else { "at least" };
            match self.what {
                What::Stunts => format!("{how} {pct}% of his pass rushes in a stunt"),
                What::Snaps if self.spot != Spot::Any => format!("{how} {pct}% of his snaps {}", self.spot.words()),
                What::AfterMistake => format!("{how} {pct}% of the snaps after a mistake: not pulled"),
                What::Freedom => format!(
                    "{} on {how} {pct}% of his snaps at quarterback",
                    if self.kind >= 3 { "full audibles at the line" } else { "a kill call at the line" }
                ),
                What::RunFamily => format!(
                    "{how} {pct}% of the runs with him in {}",
                    ["the run game", "inside zone", "outside zone", "the gap scheme", "the option"]
                        [self.kind.min(4) as usize]
                ),
                What::Side => format!(
                    "{how} {pct}% of his outside corner snaps on the {} side",
                    if self.kind == 2 { "field" } else { "boundary" }
                ),
                _ => format!("{thing} on {how} {pct}% {of}"),
            }
        } else if self.most {
            if self.n <= 0.0 { format!("no {thing}") } else { format!("at most {} {thing} a game", n(self.n)) }
        } else {
            format!("at least {} {thing} a game", n(self.n))
        };
        if self.far > 0 {
            out.push_str(&match self.what {
                What::FieldGoals => format!(", from {} yards or more", self.far),
                _ => format!(", {}+ air yards", self.far),
            });
        }
        if self.spot != Spot::Any && !(self.what == What::Snaps && self.share) {
            out.push(' ');
            out.push_str(self.spot.words());
        }
        if self.down > 0 {
            out.push_str(&format!(" on {}", ["", "first", "second", "third", "fourth"][self.down.min(4) as usize]));
            out.push_str(" down");
        }
        match self.inside {
            0 => {}
            5 => out.push_str(" at the goal line"),
            20 => out.push_str(" in the red zone"),
            y => out.push_str(&format!(" inside the {y}")),
        }
        match self.pkg {
            0 => {}
            1 => out.push_str(" in base"),
            2 => out.push_str(" in nickel"),
            3 => out.push_str(" in dime"),
            4 => out.push_str(" in the goal line package"),
            5 => out.push_str(" in nickel and dime"),
            p => out.push_str(&format!(" in {p} personnel")),
        }
        for (bit, w) in [
            (PLAY_ACTION, " off play action"),
            (TEMPO, " at tempo"),
            (GUN, " from the gun"),
            (UNDER, " under center"),
            (TWO_MINUTE, " in the two-minute drill"),
        ] {
            if self.look & bit != 0 {
                out.push_str(w);
            }
        }
        out
    }
}

/// A field goal try is the line of scrimmage plus 17 yards (the end zone
/// and the hold).
pub const SNAP_HOLD: u8 = 17;

/// Whether he plays on offense in this game: from his own snaps (a two-way
/// man counts where he played most).
fn offense_side(plays: &[PlayRecord], side: usize, id: u32) -> bool {
    let (mut o, mut d) = (0u32, 0u32);
    for r in
        plays.iter().filter(|r| matches!(r.kind, PlayType::Run | PlayType::Pass | PlayType::Sack | PlayType::Scramble))
    {
        if r.offense == side && r.on_field[0].contains(&id) {
            o += 1;
        } else if r.offense != side && r.on_field[1].contains(&id) {
            d += 1;
        }
    }
    o >= d
}

/// His spot's number on a play on side `k` (0 offense), when he was on.
fn role(r: &PlayRecord, id: u32, k: usize) -> Option<u8> {
    r.on_field[k].iter().position(|&x| x == id).and_then(|i| r.roles[k].get(i)).copied()
}

/// The run concept a call names (the longest name in it: "QB power", not
/// "power").
pub fn run_of(call: &str) -> Option<RunConcept> {
    RUN_CONCEPTS.iter().copied().filter(|c| call.contains(c.name())).max_by_key(|c| c.name().len())
}

/// The pass concept a call names (the longest name in it).
pub fn pass_of(call: &str) -> Option<&'static PassConcept> {
    PASS_CONCEPTS.iter().filter(|c| call.contains(c.name)).max_by_key(|c| c.name.len())
}

/// A quick-game or RPO throw: the quick concepts and every RPO.
pub fn quick_call(call: &str) -> bool {
    call.contains(" with ") || call.contains("RPO") || pass_of(call).is_some_and(|c| c.depth == PassDepth::Quick)
}

/// The coverage a defensive call names (it leads the call's words).
pub fn coverage_of(call: &str) -> Option<Coverage> {
    let head = call.split(',').next()?.trim();
    Coverage::ALL.iter().copied().find(|c| c.name() == head)
}

// ---------------------------------------------------------------------------
// The catalogue: what each spot can be promised (section 8.4's table).

/// A measure the reader may name, with its key, what it counts, its usual
/// number (a share or a count a game) and the spots it fits.
pub struct Measure {
    pub key: &'static str,
    pub what: What,
    /// The number when you named none, and whether it is a share.
    pub n: f32,
    pub share: bool,
    pub most: bool,
    pub spots: &'static [Position],
    /// What it means, for the reader's rules.
    pub means: &'static str,
}

use Position as P;

const OFF: &[Position] = &[P::QB, P::RB, P::WR, P::TE, P::OT, P::IOL];
const DEF: &[Position] = &[P::EDGE, P::DT, P::LB, P::CB, P::S];
const ALL: &[Position] =
    &[P::QB, P::RB, P::WR, P::TE, P::OT, P::IOL, P::EDGE, P::DT, P::LB, P::CB, P::S, P::K, P::P, P::LS];

/// Every measure a usage promise can be made on. The numbers are Claude's
/// (proposed, not confirmed).
pub const MEASURES: &[Measure] = &[
    Measure {
        key: "snaps",
        what: What::Snaps,
        n: 0.5,
        share: true,
        most: false,
        spots: ALL,
        means: "his share of the snaps (with a spot: lined up there; with a down, a zone or a package: in that situation)",
    },
    Measure {
        key: "carries",
        what: What::Carries,
        n: 12.0,
        share: false,
        most: false,
        spots: &[P::RB, P::WR, P::QB],
        means: "carries a game (a back's, a receiver's jet sweeps)",
    },
    Measure {
        key: "targets",
        what: What::Targets,
        n: 5.0,
        share: false,
        most: false,
        spots: &[P::RB, P::WR, P::TE],
        means: "targets a game (with far: 20+ air yards, deep shots; with inside 20: red zone)",
    },
    Measure {
        key: "touches",
        what: What::Touches,
        n: 15.0,
        share: false,
        most: false,
        spots: &[P::RB, P::WR, P::TE],
        means: "carries plus targets a game",
    },
    Measure {
        key: "routes",
        what: What::Routes,
        n: 0.8,
        share: true,
        most: false,
        spots: &[P::RB, P::TE],
        means: "let out on a route, not kept in to block, on a share of his pass snaps",
    },
    Measure {
        key: "throws",
        what: What::Throws,
        n: 30.0,
        share: false,
        most: false,
        spots: &[P::QB],
        means: "a quarterback's throws a game (with far: deep throws; with look: off play action)",
    },
    Measure {
        key: "designed_runs",
        what: What::Designed,
        n: 5.0,
        share: false,
        most: false,
        spots: &[P::QB],
        means: "runs called for the quarterback a game, not scrambles (most with 0: no designed runs)",
    },
    Measure {
        key: "screens",
        what: What::Screens,
        n: 2.0,
        share: false,
        most: false,
        spots: &[P::RB, P::WR, P::TE],
        means: "screens thrown to him a game",
    },
    Measure {
        key: "protection",
        what: What::Protected,
        n: 0.3,
        share: true,
        most: false,
        spots: &[P::QB],
        means: "seven or more kept in to block on a share of his dropbacks",
    },
    Measure {
        key: "help",
        what: What::Help,
        n: 0.4,
        share: true,
        most: false,
        spots: &[P::OT, P::IOL],
        means: "help on his man (a double, the slide his way or a chip) on a share of his pass snaps",
    },
    Measure {
        key: "rushes",
        what: What::Rushes,
        n: 0.9,
        share: true,
        most: false,
        spots: &[P::EDGE, P::DT, P::LB],
        means: "he rushes, not drops, on a share of dropbacks",
    },
    Measure {
        key: "stunts",
        what: What::Stunts,
        n: 0.25,
        share: true,
        most: false,
        spots: &[P::EDGE, P::DT],
        means: "stunts or twists to free him up, a share of his pass rushes (agreed: at least 1 in 4)",
    },
    Measure {
        key: "blitzes",
        what: What::Blitzes,
        n: 0.25,
        share: true,
        most: false,
        spots: &[P::LB, P::S, P::CB],
        means: "he blitzes on a share of dropbacks (agreed: at least 1 in 4)",
    },
    Measure {
        key: "man",
        what: What::Man,
        n: 0.5,
        share: true,
        most: false,
        spots: &[P::CB, P::S],
        means: "man coverage on a share of his pass snaps",
    },
    Measure {
        key: "field_goals",
        what: What::FieldGoals,
        n: 2.0,
        share: false,
        most: false,
        spots: &[P::K],
        means: "his field goal tries a game (with far: the long ones)",
    },
    Measure {
        key: "fg_line",
        what: What::TeamPunts,
        n: 0.0,
        share: false,
        most: true,
        spots: &[P::K],
        means: "the kicker's field goal line: no punt from inside it (give distance, the try's length: the yard line plus 17)",
    },
    Measure {
        key: "punts",
        what: What::Punts,
        n: 0.9,
        share: true,
        most: false,
        spots: &[P::P, P::K],
        means: "he punts, a share of the team's punts",
    },
    Measure {
        key: "kickoffs",
        what: What::Kickoffs,
        n: 0.9,
        share: true,
        most: false,
        spots: &[P::K, P::P],
        means: "he kicks off, a share of the kickoffs",
    },
    Measure {
        key: "returns",
        what: What::Returns,
        n: 0.6,
        share: true,
        most: false,
        spots: &[P::RB, P::WR, P::CB, P::S],
        means: "he returns kicks and punts, a share of them",
    },
    Measure {
        key: "special_teams",
        what: What::Teams,
        n: 0.5,
        share: true,
        most: false,
        spots: ALL,
        means: "on the field for kicks, a share (most with 0: off special teams)",
    },
    Measure {
        key: "not_pulled",
        what: What::AfterMistake,
        n: 0.9,
        share: true,
        most: false,
        spots: ALL,
        means: "stays on the field after his own turnover or bust",
    }, // Lane N2: the six readers kept on every snap.
    Measure {
        key: "kill_call",
        what: What::Freedom,
        n: 0.9,
        share: true,
        most: false,
        spots: &[P::QB],
        means: "a kill call at the line (two plays in the huddle, he picks), a share of his snaps",
    },
    Measure {
        key: "audibles",
        what: What::Freedom,
        n: 0.9,
        share: true,
        most: false,
        spots: &[P::QB],
        means: "full audibles at the line, a share of his snaps",
    },
    Measure {
        key: "inside_zone",
        what: What::RunFamily,
        n: 0.4,
        share: true,
        most: false,
        spots: &[P::RB, P::IOL, P::OT, P::TE],
        means: "a share of his carries in inside zone (a lineman's or tight end's: of the runs he's on for)",
    },
    Measure {
        key: "outside_zone",
        what: What::RunFamily,
        n: 0.4,
        share: true,
        most: false,
        spots: &[P::RB, P::IOL, P::OT, P::TE],
        means: "a share of his carries in outside zone and the toss",
    },
    Measure {
        key: "gap_scheme",
        what: What::RunFamily,
        n: 0.4,
        share: true,
        most: false,
        spots: &[P::RB, P::IOL, P::OT, P::TE],
        means: "a share of his carries in the gap scheme: power, counter, duo, trap, iso",
    },
    Measure {
        key: "option",
        what: What::RunFamily,
        n: 0.2,
        share: true,
        most: false,
        spots: &[P::RB, P::QB],
        means: "a share of his carries in the option",
    },
    Measure {
        key: "quick_targets",
        what: What::QuickTargets,
        n: 3.0,
        share: false,
        most: false,
        spots: &[P::WR, P::TE, P::RB],
        means: "targets a game on quick-game and RPO throws",
    },
    Measure {
        key: "boundary",
        what: What::Side,
        n: 0.8,
        share: true,
        most: false,
        spots: &[P::CB],
        means: "the boundary (short side) corner, a share of his outside corner snaps by the hash",
    },
    Measure {
        key: "field_side",
        what: What::Side,
        n: 0.8,
        share: true,
        most: false,
        spots: &[P::CB],
        means: "the field (wide side) corner, a share of his outside corner snaps by the hash",
    },
    Measure {
        key: "help_over_top",
        what: What::TwoHigh,
        n: 0.5,
        share: true,
        most: false,
        spots: &[P::CB],
        means: "two deep behind him (help over the top), a share of his pass snaps",
    },
    Measure {
        key: "post_safety",
        what: What::Post,
        n: 0.4,
        share: true,
        most: false,
        spots: &[P::S],
        means: "the post safety in single-high, a share of his pass snaps",
    },
    Measure {
        key: "robber",
        what: What::Robber,
        n: 0.15,
        share: true,
        most: false,
        spots: &[P::S],
        means: "the robber in Cover 1 robber, a share of his pass snaps",
    },
    Measure {
        key: "only_kicker",
        what: What::PlaceKicks,
        n: 0.95,
        share: true,
        most: false,
        spots: &[P::K],
        means: "he kicks the field goals and extra points, a share of them",
    },
    Measure {
        key: "spy",
        what: What::Spy,
        n: 0.1,
        share: true,
        most: false,
        spots: &[P::LB],
        means: "spies the quarterback, a share of his pass snaps",
    },
];

/// Measures never promised (the other team's call): the reader leaves
/// these as talking points (agreed item 4).
pub const SHOWN: &[&str] = &["chips", "doubles"];

/// The catalogue entry for a key.
pub fn measure(key: &str) -> Option<&'static Measure> {
    MEASURES.iter().find(|m| m.key == key)
}

/// A spot's offense or defense side, for a term that names a spot.
fn on_side(pos: Position) -> Option<bool> {
    if OFF.contains(&pos) {
        Some(true)
    } else if DEF.contains(&pos) {
        Some(false)
    } else {
        None
    }
}

/// What a reader's labels make of a term for a man at `pos`: the measure's
/// key, the number you said (None: the measure's usual), a share or a
/// count, at most, the spot, the down, the zone (yard line), how far (air
/// yards or the try's length), the package and the look. None when the
/// measure doesn't fit his spot or isn't one the game can hold you to.
#[allow(clippy::too_many_arguments)]
pub fn read(
    pos: Position,
    key: &str,
    number: Option<f32>,
    share: Option<bool>,
    most: bool,
    spot: &str,
    down: u8,
    inside: u8,
    far: u8,
    pkg: u8,
    look: u8,
) -> Option<Usage> {
    let m = measure(key)?;
    if !m.spots.contains(&pos) {
        return None;
    }
    let kind = kind_of(key);
    if m.what == What::TeamPunts {
        let yards = far.max(number.map_or(0, |x| x.round().clamp(0.0, 80.0) as u8));
        if yards <= SNAP_HOLD {
            return None;
        }
        return Some(Usage::fg_line(yards));
    }
    let spot = Spot::KEYS.iter().find(|x| x.0 == spot).map_or(Spot::Any, |x| x.1);
    let spot = if spot != Spot::Any && on_side(pos) != Some(spot.offense()) { Spot::Any } else { spot };
    let share = share.unwrap_or(m.share);
    let mut n = number.unwrap_or(if share == m.share {
        m.n
    } else if share {
        0.5
    } else {
        1.0
    });
    if share && n > 1.0 {
        // "60 percent" said as 60.
        n /= 100.0;
    }
    let n = if share { n.clamp(0.0, 1.0) } else { n.clamp(0.0, 99.0) };
    Some(Usage {
        what: m.what,
        spot,
        down: down.min(4),
        inside: inside.min(99),
        far,
        pkg,
        look,
        n,
        share,
        most: most || m.most,
        kind,
        season: false,
    })
}

/// The kind a reader's key names (lane N2): the freedom, the run family,
/// the corner's side (0 for the rest).
pub fn kind_of(key: &str) -> u8 {
    match key {
        "kill_call" => 2,
        "audibles" => 3,
        "inside_zone" | "boundary" => 1,
        "outside_zone" | "field_side" => 2,
        "gap_scheme" => 3,
        "option" => 4,
        _ => 0,
    }
}

// ---------------------------------------------------------------------------
// What he believes is real (agreed item 8).

/// What your football has given his spot lately, as he reads it: the best
/// game reading among your men at his position this season and last, for a
/// count the most any of them got a game, for a share the highest share.
/// None with no games to read.
pub fn shown(w: &World, i: usize, u: &Usage) -> Option<f64> {
    let team = super::Team::seat(w)?;
    let pos = w.people[i].player.position;
    let since = w.today.add(-420);
    let mates: Vec<u32> = team
        .roster(w)
        .iter()
        .filter(|&&j| w.people[j].player.position == pos)
        .map(|&j| w.people[j].player.id)
        .collect();
    let mut best: Option<f64> = None;
    let mut games = 0;
    for g in w.seat_games.iter().filter(|g| g.date >= since && g.date <= w.today) {
        let Some(side) = super::claims::side_of(g, team) else { continue };
        let boxed = g.stats.plays.is_empty();
        if boxed && u.box_count(&g.stats.players[side], 0).is_none() {
            continue;
        }
        games += 1;
        for &id in &mates {
            if !g.stats.players[side].contains_key(&id) {
                continue;
            }
            let c = if boxed {
                u.box_count(&g.stats.players[side], id).unwrap_or_default()
            } else {
                u.count(&g.stats.plays, side, id)
            };
            let v = if u.share {
                if c.of < 4.0 {
                    continue;
                }
                c.n / c.of
            } else {
                c.n
            };
            best = Some(best.map_or(v, |b: f64| b.max(v)));
        }
    }
    if games == 0 { None } else { best.or(Some(0.0)) }
}

/// How likely the term looks to him (P1's "likely" part, 0.3-1): a number
/// your football hasn't come near is a pitch to a sharp man; a big ego
/// takes it as his due. A kicker's field goal line past his range reads
/// from his Kick Power (his range as he sees it), stretched by ego and
/// tightened by Football IQ.
pub fn likely(w: &World, i: usize, u: &Usage) -> f64 {
    let p = &w.people[i];
    let iq = (p.player.attributes.get(Attr::FootballIq) - 60.0) / 20.0;
    let ego = p.player.personality.traits.z(Trait::Ego);
    // How much of the gap he sees: a sharp man all of it, a big ego less.
    let sees = (0.75 + 0.2 * iq - 0.15 * ego).clamp(0.3, 1.2);
    if u.what == What::TeamPunts && u.never() {
        // His range as he sees it: the engine's range (38 + 0.3 x Kick
        // Power - 4), stretched a yard per point of ego.
        let range = 34.0 + 0.3 * p.player.attributes.get(Attr::KickPower) + 2.0 * ego;
        let line = u.inside as f64 + SNAP_HOLD as f64;
        let past = (line - range).max(0.0);
        return (1.0 - 0.04 * past * sees).clamp(0.3, 1.0);
    }
    if u.most {
        return 0.95;
    }
    let Some(seen) = shown(w, i, u) else { return 0.85 };
    let bar = u.n as f64;
    if bar <= seen || bar <= 0.0 {
        return 1.0;
    }
    let over = if seen <= 0.0 { 2.0 } else { (bar / seen - 1.0).min(2.0) };
    (1.0 - 0.35 * over * sees).clamp(0.3, 1.0)
}

// ---------------------------------------------------------------------------
// The plan that keeps them.

/// Every open usage promise to a man on your team, and the recruits you
/// signed with one: (his id, the term).
pub fn open(w: &World) -> Vec<(u32, Usage)> {
    use super::talks::{PromiseKind, PromiseTo};
    w.folk
        .promises
        .iter()
        .filter(|p| p.kept.is_none() && !p.terms.off)
        .filter(|p| matches!(p.kind, PromiseKind::Usage | PromiseKind::Recruit(super::pledge::USAGE, _)))
        .filter_map(|p| match (p.to, p.terms.usage) {
            (PromiseTo::Player(id), Some(u)) => Some((id, u)),
            _ => None,
        })
        .collect()
}

/// The plan settings your staff sets from your open promises on usage,
/// before each game your team plays (`plans::prepare`). A promise whose
/// setting you turned off (`Terms::off`) sets nothing.
pub fn keep(w: &World) -> Keep {
    let mut k = Keep::default();
    let push = |v: &mut Vec<u32>, id: u32| {
        if !v.contains(&id) {
            v.push(id);
        }
    };
    for (id, u) in open(w) {
        let pos = super::person(w, id).map(|i| w.people[i].player.position);
        // "No designed runs" is a ceiling the plan keeps.
        if u.what == What::Designed && u.never() {
            k.qb_runs = -1;
            continue;
        }
        if u.most && u.what != What::TeamPunts {
            continue;
        }
        match u.what {
            What::TeamPunts if u.never() && u.inside > 0 => k.fg_line = k.fg_line.max(u.inside),
            What::Stunts => push(&mut k.free_up, id),
            What::Blitzes => push(&mut k.send, id),
            What::Targets if u.far >= 15 => push(&mut k.shots, id),
            What::Throws if u.far >= 15 => {}
            What::Throws => {
                k.pass = true;
                k.play_action |= u.look & PLAY_ACTION != 0;
                if u.look & GUN != 0 {
                    k.backfield = 1;
                } else if u.look & UNDER != 0 {
                    k.backfield = -1;
                }
            }
            What::Designed if k.qb_runs == 0 => k.qb_runs = 1,
            What::Protected => k.protect = true,
            What::Targets | What::Touches => push(&mut k.feature, id),
            What::Screens => {
                push(&mut k.feature, id);
                push(&mut k.screens, id);
            }
            What::QuickTargets => {
                push(&mut k.feature, id);
                push(&mut k.quick, id);
            }
            What::Carries if pos == Some(Position::WR) => push(&mut k.gadget, id),
            What::Carries | What::AfterMistake => push(&mut k.on, id),
            What::Snaps => {
                push(&mut k.on, id);
                if u.pkg >= 10 {
                    k.personnel = u.pkg;
                }
            }
            What::Routes => push(&mut k.release, id),
            What::Help => push(&mut k.help, id),
            What::Rushes => push(&mut k.rush, id),
            What::Man => k.man = k.man.max(if u.share { u.n } else { 0.5 }),
            What::Freedom => match k.freedom.iter_mut().find(|x| x.0 == id) {
                Some(x) => x.1 = x.1.max(u.kind.max(2)),
                None => k.freedom.push((id, u.kind.max(2))),
            },
            What::RunFamily if (1..=4).contains(&u.kind) => k.family[u.kind as usize - 1] = 1.0,
            What::Side if !k.side.iter().any(|x| x.0 == id) => k.side.push((id, u.kind.max(1))),
            What::TwoHigh => k.two_high = k.two_high.max(if u.share { u.n } else { 0.5 }),
            What::Post => {
                push(&mut k.post, id);
                k.one_high = k.one_high.max(if u.share { u.n } else { 0.4 });
            }
            What::Robber => {
                push(&mut k.robber, id);
                k.rob = k.rob.max(if u.share { u.n } else { 0.15 });
            }
            What::Returns => push(&mut k.returner, id),
            _ => {}
        }
    }
    k
}

/// What keeps a term in games your staff runs, in words: a plan setting,
/// or your depth chart and your calls.
pub fn kept_by(u: &Usage) -> &'static str {
    match u.what {
        What::TeamPunts if u.never() => "The field goal line on the game plan: inside it, the staff kicks.",
        What::Designed if u.never() => "No designed runs on the game plan: the staff takes his runs off the sheet.",
        _ if u.most => "Your depth chart and your calls keep it: the game plan has no setting for it.",
        What::Stunts => "Free him up on the game plan: stunts with him in them.",
        What::Blitzes => "Send him on the game plan: more blitzes, with him the man sent.",
        What::Targets if u.far >= 15 => {
            "Take shots to him on the game plan: more deep calls, his deep routes read first."
        }
        What::Targets | What::Touches => "Feature him on the game plan: the quarterback looks to him first more often.",
        What::Screens => "Screens to him on the game plan: more screens, and he's read first.",
        What::QuickTargets => {
            "The quick game to him on the game plan: more quick throws and RPOs, and he's read first."
        }
        What::Help => "Help him on the game plan: the slide, a double or a chip on his man.",
        What::Rushes => "Rush him on the game plan: he never drops.",
        What::Man => "More man on the game plan.",
        What::Throws if u.far >= 15 => "Your calls keep it: the game plan has no setting for his deep throws.",
        What::Throws => "Throw more on the game plan, with play action or the backfield you named.",
        What::Designed => "Designed runs on the game plan: his runs called more often.",
        What::Protected => "Max protect on the game plan: more seven-man protections.",
        What::Carries | What::Snaps | What::AfterMistake => {
            "Keep him on, on the game plan: never spelled, never kept to base downs. Your depth chart still has to start him."
        }
        What::Routes => "Release him on the game plan: he runs a route where the call keeps him in.",
        What::Freedom => "Freedom at the line on the game plan: his kill calls or full audibles.",
        What::RunFamily => "Run family weights on the game plan: that family called more.",
        What::Side => "His side on the game plan: he trades sides with the other corner by the hash.",
        What::TwoHigh => "Two-high on the game plan: at least that much two deep.",
        What::Post => "Single-high with him deep on the game plan: he plays free safety in it.",
        What::Robber => "Rob with him on the game plan: Cover 1 robber with him the strong safety.",
        What::Returns => "He returns on the game plan: first on kickoffs, the punt returner.",
        _ => "Your depth chart and your calls keep it: the game plan has no setting for it.",
    }
}

// ---------------------------------------------------------------------------
// Turning a setting off, and the staff's warning before a game (lane N2).

/// Turn the plan setting that keeps promise `k` (an index in
/// `Folk.promises`) off, or back on. Your staff stops keeping it and warns
/// you before each game while it is off. Err for a promise that isn't an
/// open usage promise.
pub fn set_off(w: &mut World, k: usize, off: bool) -> Result<(), String> {
    use super::talks::PromiseKind;
    let p = w.folk.promises.get(k).ok_or("No such promise.")?;
    if p.kept.is_some()
        || p.terms.usage.is_none()
        || !matches!(p.kind, PromiseKind::Usage | PromiseKind::Recruit(super::pledge::USAGE, _))
    {
        return Err("That promise has no plan setting.".into());
    }
    w.folk.promises[k].terms.off = off;
    Ok(())
}

/// Your staff's warnings before a game: each open promise on usage the
/// plan won't keep, and why (its setting turned off, him not with the ones
/// for a term that needs him on the field, a field goal line past his
/// kicker's range, a term with no setting that fell behind). Nothing for a
/// man out hurt: his promise waits.
pub fn warnings(w: &World) -> Vec<String> {
    use super::promise::State;
    use super::talks::{PromiseKind, PromiseTo};
    let mut out = Vec::new();
    for p in w.folk.promises.iter().filter(|p| p.kept.is_none()) {
        if !matches!(p.kind, PromiseKind::Usage | PromiseKind::Recruit(super::pledge::USAGE, _)) {
            continue;
        }
        let (PromiseTo::Player(id), Some(u)) = (p.to, p.terms.usage) else { continue };
        let Some(i) = super::person(w, id) else { continue };
        if !u.team() && super::seen::hurt_on(w, id, w.today) {
            continue;
        }
        let said = u.words();
        let who = &p.who;
        let specialist = w.people[i].player.position.is_specialist();
        let field = !u.team()
            && !u.most
            && !specialist
            && !matches!(u.what, What::Returns | What::Teams | What::PlaceKicks | What::Kickoffs | What::Punts);
        let line = if p.terms.off {
            Some(format!("You turned off the plan setting for {who} ({said}): we won't keep it."))
        } else if field && !super::seen::with_the_ones(w, id) {
            Some(format!(
                "{who} isn't with the ones, so the plan alone won't keep {said}: your depth chart has to put him there."
            ))
        } else if u.what == What::TeamPunts && u.never() && likely(w, i, &u) < 0.7 {
            Some(format!(
                "The field goal line for {who} ({said}) is past his range: we'll kick from there, and he may miss."
            ))
        } else if super::promise::State::of(p.state) == State::Behind {
            Some(format!(
                "{who} is behind on {said}. {}",
                if kept_by(&u).starts_with("Your depth chart") {
                    "The game plan has no setting for it: your calls and your depth chart keep it."
                } else {
                    "The setting is on, but the game didn't give it to him last week."
                }
            ))
        } else {
            None
        };
        out.extend(line);
    }
    out
}

/// The day before each of your games, your staff's warnings land in your
/// inbox (lane N2).
pub(crate) fn warn(w: &mut World, d: crate::calendar::Date) {
    if w.seat.is_none() || !w.folk.promises.iter().any(|p| p.kept.is_none() && p.terms.usage.is_some()) {
        return;
    }
    if crate::world::plans::next_game(w).is_none_or(|g| g.date != d.add(1)) {
        return;
    }
    for text in warnings(w) {
        super::note(w, "talk", "Your staff, before the game".into(), text, None);
    }
}
