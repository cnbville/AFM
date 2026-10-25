//! Your moves (S3, S10): what you say and do about a story you are in.
//! Every move is a beat. Said at a press conference or in a story meeting,
//! it is public; a talk, a call or a meeting is private. How a move lands
//! comes from the situation and from you, never a flat score: backing the
//! starter the truth favours holds up, backing the one it doesn't comes
//! back on you; a denial is only as good as what is really going on; a
//! private talk is as good as your relationships; a public promise kept
//! pays and one broken costs double. Your record with each story (S10's
//! "handled") is the sum of your moves and how your words held up at the
//! ending.
//!
//! A story you are in that runs hot brings a story meeting (`Kind::Story`):
//! the hottest stop Continue, the rest wait for you a week and then go to
//! your staff. Reporters ask about your stories at your availabilities
//! (`press.rs`, `press_stories`). Under college rules a coach does not talk
//! about a recruit before he signs, so a recruiting story only has private
//! moves and "no comment" in public.
//!
//! Sizes are Claude's picks (proposed, not confirmed).

use super::{BeatKind, Family, K, Part, Phase, Rating, Story, Voice, Who, add_beat};
use crate::calendar::{Date, Level};
use crate::player::Trait;
use crate::world::World;
use crate::world::folk::cast::{self, Role};
use crate::world::folk::scene::{self, Kind, Pick, Scene};
use crate::world::folk::talks::{self, PromiseKind, PromiseTo, Reason};
use crate::world::pro::possessive;
use crate::world::stories::TeamKey;

/// What you can do about a story.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Move {
    /// Back the lead in public: the starter, the player in trouble, your
    /// coordinator.
    Back,
    /// Back the rival or the change in public.
    BackRival,
    /// It's open: let them compete, let it play out.
    Open,
    /// No comment.
    Quiet,
    /// Deal with it in private: a talk, a meeting, a call.
    Private,
    /// Deny it.
    Deny,
    /// Own it: take the blame.
    Own,
    /// Fire back at the critics or the rival.
    FireBack,
    /// Praise the player, the team, the opponent.
    Praise,
    /// Draw a line in public: a suspension, a warning.
    Warn,
    /// Let it out through a friendly writer or your agent.
    Leak,
    /// Promise in public: a result, a job, that you stay.
    Promise,
    /// Ask for more, in public.
    Challenge,
}

impl Move {
    pub const ALL: [Move; 13] = [
        Move::Back,
        Move::BackRival,
        Move::Open,
        Move::Quiet,
        Move::Private,
        Move::Deny,
        Move::Own,
        Move::FireBack,
        Move::Praise,
        Move::Warn,
        Move::Leak,
        Move::Promise,
        Move::Challenge,
    ];
    pub fn code(self) -> u32 {
        Move::ALL.iter().position(|&m| m == self).unwrap_or(3) as u32
    }
    pub fn from_code(c: u32) -> Move {
        Move::ALL.get(c as usize).copied().unwrap_or(Move::Quiet)
    }
    pub fn key(self) -> &'static str {
        match self {
            Move::Back => "back",
            Move::BackRival => "back_rival",
            Move::Open => "open",
            Move::Quiet => "quiet",
            Move::Private => "private",
            Move::Deny => "deny",
            Move::Own => "own",
            Move::FireBack => "fire_back",
            Move::Praise => "praise",
            Move::Warn => "warn",
            Move::Leak => "leak",
            Move::Promise => "promise",
            Move::Challenge => "challenge",
        }
    }
    /// The move as the talks name it.
    pub fn name(self) -> &'static str {
        match self {
            Move::Back | Move::BackRival => "Back him",
            Move::Open => "Challenge",
            Move::Quiet => "Deflect",
            Move::Private => "In private",
            Move::Deny => "Deny",
            Move::Own => "Take the blame",
            Move::FireBack => "Fire back",
            Move::Praise => "Praise",
            Move::Warn => "Warn",
            Move::Leak => "Leak",
            Move::Promise => "Promise",
            Move::Challenge => "Challenge",
        }
    }
    pub fn tone(self) -> &'static str {
        match self {
            Move::FireBack | Move::Promise => "fiery",
            Move::Back | Move::BackRival | Move::Warn | Move::Own | Move::Challenge | Move::Deny => "firm",
            Move::Praise | Move::Private => "warm",
            _ => "calm",
        }
    }
    /// Said where everyone hears it.
    pub fn public(self) -> bool {
        !matches!(self, Move::Private | Move::Leak)
    }
    /// The rating that decides how well you do it.
    fn rating(self, fam: Family) -> Rating {
        match self {
            Move::Back | Move::BackRival | Move::Open if fam == Family::Battle => Rating::Evaluation,
            Move::Back | Move::BackRival | Move::Private | Move::Own | Move::Praise => Rating::Relationships,
            Move::Open | Move::Challenge | Move::Promise | Move::FireBack => Rating::Motivation,
            Move::Warn | Move::Quiet => Rating::Discipline,
            Move::Deny => Rating::Negotiating,
            Move::Leak => Rating::Network,
        }
    }
    /// What a public word adds to the story's heat.
    fn heat(self) -> f32 {
        match self {
            Move::FireBack => 9.0,
            Move::Promise | Move::Warn => 6.0,
            Move::Leak => 8.0,
            Move::Back | Move::BackRival | Move::Open | Move::Challenge => 4.0,
            Move::Deny => 3.0,
            Move::Own | Move::Praise => 3.0,
            Move::Quiet | Move::Private => 0.0,
        }
    }
}

/// What your staff expects a move to do, as short tags with a tone (0
/// good, 1 plain, 2 bad), read off what `make` does to the story and the
/// people in it. How well it lands still depends on the truth and on you.
pub fn chips(w: &World, s: &Story, mv: Move) -> Vec<(String, u8)> {
    let f = super::tell::fill(w, s);
    let first = |k: &str| f.get(k).map(|n| n.split_whitespace().last().unwrap_or(n).to_string());
    let lead = first("lead");
    let rival = first("rival");
    let fam = s.family();
    let mut v: Vec<(String, u8)> = Vec::new();
    match mv {
        Move::Quiet => v.push(("cools it a little".into(), 0)),
        Move::Private => v.push(("stays private".into(), 1)),
        _ => {
            let h = mv.heat().round() as i32;
            v.push((format!("heat +{h}"), if h >= 6 { 2 } else { 1 }));
        }
    }
    match mv {
        Move::Back if fam == Family::Battle => {
            if let Some(l) = &lead {
                v.push((format!("{l} starts, said in public"), 0));
            }
            if let Some(r) = &rival {
                v.push((format!("{} mood down", possessive(r)), 2));
            }
        }
        Move::BackRival => {
            if let Some(r) = &rival {
                v.push((format!("{} mood up", possessive(r)), 0));
            }
            if let Some(l) = &lead {
                v.push((format!("{} mood down", possessive(l)), 2));
            }
        }
        Move::Back | Move::Praise => {
            v.push(("locker room +".into(), 0));
            if let Some(l) = &lead {
                v.push((format!("{} trust +", possessive(l)), 0));
            }
        }
        Move::Open | Move::Challenge => v.push(("competitors lift, egos bristle".into(), 1)),
        Move::Warn => {
            v.push(("boosters +".into(), 0));
            v.push(("locker room \u{2212}".into(), 2));
            if let Some(l) = &lead {
                v.push((format!("{} trust \u{2212}", possessive(l)), 2));
            }
        }
        Move::Private => {
            for n in [&lead, &rival].into_iter().flatten() {
                v.push((format!("{} trust +", possessive(n)), 0));
            }
        }
        Move::Own => {
            v.push(("fans +".into(), 0));
            v.push(("the room's trust +".into(), 0));
        }
        Move::FireBack => {
            v.push(("locker room +".into(), 0));
            v.push(("media \u{2212}".into(), 2));
        }
        Move::Deny => v.push(("on the record".into(), 1)),
        Move::Promise => {
            v.push(("fans +".into(), 0));
            v.push(("a promise to keep".into(), 1));
        }
        Move::Leak => v.push(("out by morning, unsigned".into(), 1)),
        Move::Quiet => v.push(("media \u{2212}".into(), 2)),
    }
    // A public word on a battle you walk back later costs.
    if s.stance != 0
        && matches!((s.stance, mv), (1, Move::BackRival | Move::Open | Move::Warn) | (-1, Move::Back | Move::Open))
    {
        v.push(("walks back your word".into(), 2));
    }
    v.truncate(3);
    v
}

/// What you said on the record, kept on the story (`Story::said`).
pub const SAID_PROMISE: u8 = 1;
pub const SAID_DENY: u8 = 2;
pub const SAID_LEAK: u8 = 4;
pub const SAID_OWN: u8 = 8;

/// One answer on the table.
#[derive(Clone, Debug)]
pub struct Opt {
    pub mv: Move,
    pub label: String,
    pub hint: String,
}

/// Where you stand in a story.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Sit {
    /// It is about you.
    Me,
    /// A player of yours leads it.
    Player,
    /// One of your coaches leads it.
    Coach,
    /// It is about your team.
    Team,
    /// A recruit you want.
    Recruit,
    /// You are the other side.
    Other,
}

fn sit(w: &World, s: &Story) -> Sit {
    let me = crate::world::stories::seat_coach(w).map(|c| c as u32);
    if me.is_some_and(|c| s.lead() == Some(Who::Coach(c))) {
        return Sit::Me;
    }
    if s.family() == Family::Recruiting {
        return Sit::Recruit;
    }
    if s.you == Some(Part::Rival) {
        return Sit::Other;
    }
    match s.lead() {
        Some(Who::Player(_)) => Sit::Player,
        Some(Who::Coach(_)) => Sit::Coach,
        _ => Sit::Team,
    }
}

fn kind(s: &Story) -> Option<K> {
    K::ALL.get(s.kind as usize).copied()
}

/// The answers a story allows you, said in public (`press`) or in a story
/// meeting where private moves are on the table too.
pub(crate) fn menu(w: &mut World, s: &Story, press: bool) -> Vec<Opt> {
    use Move::*;
    let f = super::tell::fill(w, s);
    let g = |k: &str, d: &str| f.get(k).unwrap_or(d).to_string();
    let (lead, rival, team, other) =
        (g("lead", "him"), g("rival", "the other one"), g("team", "us"), g("other", "them"));
    let k = kind(s);
    let fam = s.family();
    let college_recruit = s.level == Level::Recruiting || fam == Family::Recruiting;
    let mut v: Vec<(Move, String, &'static str)> = Vec::new();
    match (sit(w, s), fam) {
        (_, Family::Recruiting) | (Sit::Recruit, _) => {
            let your_player = s.lead_player().is_some_and(|id| {
                let Some(i) = super::pi(w, id) else { return false };
                matches!(crate::world::stories::seat_team(w), Some(TeamKey::Hs(h)) if w.people[i].stage == crate::world::Stage::HighSchool && w.people[i].hs == h as usize)
            });
            if your_player {
                v.push((
                    Praise,
                    format!("\"{lead} can play anywhere in the country.\""),
                    "His stock goes up; so does the noise around him.",
                ));
                v.push((
                    Private,
                    format!("Sit down with {lead} and his family about the choice."),
                    "Your read of the schools matters to them.",
                ));
                v.push((Quiet, "\"That's his decision and his family's.\"".into(), "Nothing for the insiders."));
            } else {
                v.push((
                    Quiet,
                    "\"I can't talk about recruits.\"".into(),
                    "The rules: not a word about a recruit before he signs.",
                ));
                if !press && college_recruit {
                    v.push((
                        Private,
                        format!("Call {lead} tonight yourself."),
                        "Your relationships decide how it lands.",
                    ));
                    v.push((
                        Leak,
                        format!("Let the insiders know {lead} is your top target."),
                        "His camp hears it; so do the other staffs.",
                    ));
                }
            }
        }
        (Sit::Me, Family::Seat) => match k {
            Some(K::VoteOfConfidence) => {
                v.push((
                    Praise,
                    format!("\"I appreciate {} support. Now we go win.\"", possessive(&g("boss", "the boss"))),
                    "Grateful in public; the writers read it as a coach who knows.",
                ));
                v.push((
                    Own,
                    "\"Support has to be earned on Saturdays. I know that.\"".into(),
                    "Honest; the boss respects it.",
                ));
                v.push((Quiet, "\"I'm worried about this week, not about me.\"".into(), "The standard answer."));
            }
            Some(K::InterimAudition) => {
                v.push((
                    Promise,
                    "\"These kids deserve a coach. I want to be him.\"".into(),
                    "Say it out loud: the room loves it, the search notices.",
                ));
                v.push((Praise, "\"This is about the players, not me.\"".into(), "Safe; the room hears it."));
                v.push((Quiet, "\"I'm not thinking about the job.\"".into(), "Nothing either way."));
            }
            Some(K::ExtensionTalks) => {
                v.push((
                    Promise,
                    format!("\"I want to be at {team} for a long time.\""),
                    "Fans love it; your agent less so.",
                ));
                v.push((Quiet, "\"That's between me and the athletic director.\"".into(), "Keeps the talks private."));
                if !press {
                    v.push((
                        Leak,
                        "Let your agent float the interest from other schools.".into(),
                        "Leverage at the table; the boss may resent it.",
                    ));
                }
            }
            _ => {
                v.push((
                    Own,
                    "\"The record is the record. I own it.\"".into(),
                    "Honest; your boss respects it more than the writers do.",
                ));
                v.push((
                    FireBack,
                    "\"The people writing that have never coached a down.\"".into(),
                    "Your players like the fight. The writers and your boss don't.",
                ));
                v.push((
                    Quiet,
                    "\"I'm not worried about my job. I'm worried about this week.\"".into(),
                    "The standard answer; nobody's convinced.",
                ));
                if !press {
                    v.push((
                        Private,
                        format!("Ask {} for a meeting, just the two of you.", g("boss", "the boss")),
                        "Straight talk in private; it lands by how he reads you.",
                    ));
                }
            }
        },
        (Sit::Me, Family::Wanted) | (Sit::Me, Family::You)
            if matches!(k, Some(K::JobRumors | K::NflWantsCoach | K::BigJobWantsCoach | K::OldSchoolCalls)) =>
        {
            v.push((
                Deny,
                format!("\"I'm not going anywhere. I'm the coach at {team}.\""),
                "Firm. If you do leave, people remember it.",
            ));
            v.push((Quiet, "\"I'm focused on this team. That's all I'll say.\"".into(), "The rumour keeps going."));
            v.push((
                Open,
                "\"I'll always listen. That's the business.\"".into(),
                "Honest; recruits and fans hear it too.",
            ));
            if !press {
                v.push((
                    Leak,
                    format!("Let your agent make sure {team} knows about {other}."),
                    "Leverage for a new deal; the boss may resent it.",
                ));
            }
        }
        (Sit::Me, _) => match k {
            Some(K::NewCoach) => {
                v.push((
                    Promise,
                    "\"We're here to win championships. Not someday. Now.\"".into(),
                    "The room loves it. The bar goes up.",
                ));
                v.push((
                    Own,
                    "\"It's a process. We'll build it the right way.\"".into(),
                    "Lowers what people expect of year one.",
                ));
                v.push((
                    Praise,
                    "\"The players here have been great. It starts with them.\"".into(),
                    "The holdovers warm to you.",
                ));
            }
            Some(K::BigOne | K::CantBeatRival) => {
                v.push((Own, "\"Until we win it, that question is fair.\"".into(), "Honest; takes the edge off."));
                v.push((
                    Promise,
                    "\"We're going to get it done. You'll see.\"".into(),
                    "Bold. Kept, it's legend; broken, it's a punchline.",
                ));
                v.push((Quiet, "\"I'm not going to talk about that.\"".into(), "The question stays."));
            }
            Some(K::OldTeam | K::CoachReturn) => {
                v.push((
                    Praise,
                    format!("\"I have nothing but respect for {other}.\""),
                    "Classy; nothing for their locker room.",
                ));
                v.push((
                    FireBack,
                    "\"I remember how it ended there. So do they.\"".into(),
                    "Your players love it; their fans will be ready.",
                ));
                v.push((Quiet, "\"It's just another game for us.\"".into(), "Nobody believes it, and that's fine."));
            }
            Some(K::YourContract) => {
                v.push((
                    Promise,
                    format!("\"I want to finish my career at {team}.\""),
                    "Fans love it; the talks lose an edge.",
                ));
                v.push((Quiet, "\"I'll let my agent handle that.\"".into(), "Neutral."));
                if !press {
                    v.push((
                        Leak,
                        "Let your agent float the interest from elsewhere.".into(),
                        "Leverage; the boss may resent it.",
                    ));
                }
            }
            _ => {
                v.push((
                    Praise,
                    "\"That's on the players and the staff. They earned it.\"".into(),
                    "The room hears you give them the credit.",
                ));
                v.push((Quiet, "\"I don't think about that during the season.\"".into(), "Nothing either way."));
            }
        },
        (Sit::Player, Family::Battle) => {
            let starter = if matches!(k, Some(K::QbControversy | K::KickerSlump)) { "our guy" } else { "our starter" };
            v.push((Back, format!("\"{lead} is {starter}.\""), "A public word: if it changes, it costs double."));
            if s.rival().is_some() {
                v.push((BackRival, format!("\"{rival} has earned his shot.\""), "The change, said out loud."));
            }
            v.push((
                Open,
                if s.rival().is_some() {
                    format!("\"It's open. {lead} and {rival} will compete.\"")
                } else {
                    "\"Everybody competes. Nobody owns a job.\"".into()
                },
                "Competitors lift; nobody is promised anything.",
            ));
            v.push((Quiet, "\"I'm not going to get into the depth chart.\"".into(), "Gives the writers nothing."));
            if !press {
                v.push((Private, format!("Talk to {lead} before anyone else does."), "He hears it from you first."));
            }
        }
        (Sit::Player, Family::OffField) => match k {
            Some(K::FamilyLoss) => {
                v.push((
                    Praise,
                    format!("\"{lead} has our full support. Football can wait.\""),
                    "The room sees how you treat people.",
                ));
                v.push((Quiet, "\"We'll respect the family's privacy.\"".into(), "Right and quiet."));
                if !press {
                    v.push((
                        Private,
                        format!("Go see {lead} and his family yourself."),
                        "A coach who shows up is remembered.",
                    ));
                }
            }
            Some(K::RoomRift | K::PublicFeud) => {
                v.push((
                    Own,
                    "\"Whatever's going on in that room is on me to fix.\"".into(),
                    "The room hears you take it on.",
                ));
                v.push((
                    Warn,
                    "\"Nobody is bigger than this team. Nobody.\"".into(),
                    "A line in public; egos take it personally.",
                ));
                v.push((Quiet, "\"We keep that in house.\"".into(), "The writers dig."));
                if !press {
                    v.push((
                        Private,
                        if s.rival().is_some() {
                            format!("Get {lead} and {rival} in a room together.")
                        } else {
                            "Call the leaders in.".into()
                        },
                        "Your relationships decide whether it heals.",
                    ));
                }
            }
            Some(K::NilDispute | K::Academics | K::SecondOpinion) => {
                v.push((
                    Back,
                    format!("\"{lead} is a big part of what we do. We'll work it out.\""),
                    "Loyal in public.",
                ));
                v.push((Quiet, "\"That's between him and us.\"".into(), "Neutral."));
                if !press {
                    v.push((Private, format!("Sit down with {lead}."), "He hears where he stands."));
                }
            }
            _ => {
                v.push((Quiet, "\"We're aware of it. It's being handled internally.\"".into(), "The standard line."));
                v.push((
                    Warn,
                    format!("\"{lead} is suspended indefinitely. We hold our players to a standard.\""),
                    "Firm in public; he hears it from the TV first.",
                ));
                v.push((
                    Back,
                    format!("\"{lead} is a good kid. Let the process play out.\""),
                    "Loyal; the boss would rather you said less.",
                ));
                if !press {
                    v.push((Private, format!("Call {lead} in before you say anything."), "You hear his side first."));
                }
            }
        },
        (Sit::Player, Family::Leaving) => {
            v.push((Back, format!("\"{lead} is a big part of what we do.\""), "Warm; he and his agent hear it."));
            v.push((
                Warn,
                "\"Nobody is bigger than the team.\"".into(),
                "A line in public; an ego takes it personally.",
            ));
            v.push((Quiet, "\"That's between him and us.\"".into(), "Neutral."));
            if !press {
                v.push((Private, format!("Sit down with {lead} yourself."), "Your relationships decide how it lands."));
            }
        }
        (Sit::Player, _) => {
            v.push((
                Praise,
                format!("\"{lead} is one of the best players I've coached.\""),
                "Public praise counts double; an ego grows.",
            ));
            v.push((
                Challenge,
                format!("\"{lead} knows there's more in him.\""),
                "A public challenge: competitors lift, egos bristle.",
            ));
            v.push((Quiet, "\"I'm not going to single anyone out.\"".into(), "Nothing either way."));
            if !press && matches!(fam, Family::Draft | Family::Leaving | Family::Comeback | Family::Ending) {
                v.push((
                    Private,
                    format!("Talk to {lead} about what's next."),
                    "Your read of him decides how it lands.",
                ));
            }
        }
        (Sit::Coach, _) => match k {
            Some(K::CoordinatorHeat) => {
                v.push((
                    Back,
                    format!("\"{lead} has my full confidence.\""),
                    "Loyal; if you make the change later, it costs you.",
                ));
                v.push((
                    Warn,
                    "\"Everything is on the table. I'll look at all of it.\"".into(),
                    "The room hears it; so does he.",
                ));
                v.push((Quiet, "\"We'll evaluate everything after the season.\"".into(), "The standard line."));
                if !press {
                    v.push((
                        Private,
                        format!("Sit down with {lead} and go over the tape."),
                        "Straight talk in private.",
                    ));
                }
            }
            _ => {
                v.push((
                    Praise,
                    format!("\"{lead} deserves every chance he gets.\""),
                    "Generous; the next assistant you hire hears it.",
                ));
                v.push((Quiet, "\"He's focused on us. So am I.\"".into(), "Neutral."));
                if !press {
                    v.push((
                        Private,
                        format!("Ask {lead} what it would take to stay."),
                        "A raise or a title might keep him.",
                    ));
                }
            }
        },
        (Sit::Other, _) => {
            v.push((
                Praise,
                format!("\"{team} is a good football team. It'll be a great game.\""),
                "Nothing for their locker room wall.",
            ));
            v.push((
                Promise,
                format!("\"We're going to go there and beat {team}.\""),
                "Bulletin-board material. Your fans love it; theirs will too.",
            ));
            v.push((Quiet, "\"We worry about ourselves.\"".into(), "Nothing either way."));
        }
        (Sit::Team, Family::Rivalry) => {
            v.push((
                Promise,
                format!("\"We're going to beat {other}.\""),
                "Bulletin-board material. Your fans love it; their locker room will too.",
            ));
            v.push((
                Praise,
                format!("\"{other} is a good football team. It'll be a great game.\""),
                "Nothing for their locker room wall.",
            ));
            v.push((Quiet, "\"It's one game. We prepare the same way.\"".into(), "Nobody believes it."));
            if matches!(k, Some(K::BulletinBoard)) {
                v.push((
                    FireBack,
                    "\"They can say what they want. We'll answer on the field.\"".into(),
                    "Heat on the week; your players love it.",
                ));
            }
        }
        (Sit::Team, _) if s.def().tone < 0 => {
            v.push((
                Own,
                "\"That's on me. I have to coach better.\"".into(),
                "Players respect it; your boss reads it either way.",
            ));
            v.push((
                Challenge,
                "\"Some guys need to look in the mirror.\"".into(),
                "Competitors respond; the fragile hear blame.",
            ));
            v.push((
                FireBack,
                "\"The people writing us off don't know this team.\"".into(),
                "The room likes the fight; the writers don't.",
            ));
            v.push((Quiet, "\"We're working. That's all I'll say.\"".into(), "Nothing either way."));
            if !press && matches!(k, Some(K::PortalExodus | K::FanRevolt | K::Graveyard | K::Decline)) {
                v.push((Private, "Call the leaders of the team in.".into(), "Your relationships decide how it lands."));
            }
        }
        (Sit::Team, _) => {
            v.push((
                Praise,
                "\"That's on the players. They earned it.\"".into(),
                "The room hears you give them the credit.",
            ));
            v.push((
                Challenge,
                "\"We haven't done anything yet.\"".into(),
                "Keeps them hungry; some feel it's never enough.",
            ));
            v.push((Quiet, "\"One week at a time.\"".into(), "The standard line."));
            if matches!(
                k,
                Some(K::Unbeaten | K::PlayoffPush | K::BowlChase | K::ConferenceRace | K::TitleDefense | K::Drought)
            ) {
                v.push((
                    Promise,
                    "\"We're going to finish this.\"".into(),
                    "Bold. Kept, it's remembered; broken, it's quoted back at you.",
                ));
            }
        }
    }
    let _ = &rival;
    // What you said on it this week isn't offered again: the same line
    // twice is no new move. With everything said, a no-comment is left.
    let said: Vec<&str> =
        s.beats.iter().filter(|b| b.by == Voice::You && b.date.days_to(w.today) < 7).map(|b| b.text.as_str()).collect();
    let (fresh, stale): (Vec<_>, Vec<_>) = v
        .into_iter()
        .filter(|(m, ..)| !press || m.public())
        .partition(|(_, label, _)| !said.iter().any(|t| t.ends_with(label.as_str())));
    let left = if fresh.is_empty() { stale.into_iter().filter(|(m, ..)| *m == Quiet).take(1).collect() } else { fresh };
    left.into_iter().take(4).map(|(mv, label, hint)| Opt { mv, label, hint: hint.to_string() }).collect()
}

/// Your rating for a move, centred: -0.9 at 1, 1 at 20.
fn skill(w: &World, r: Rating) -> f32 {
    let Some(c) = crate::world::stories::seat_coach(w) else { return 0.0 };
    let co = &w.coaches[c];
    ((r.get(&co.ability, co.yours.as_ref()) - 10.0) / 10.0) as f32
}

/// How well a move fits the situation, before your skill: the truth the
/// people inside know, the sides, the family.
fn fit(w: &World, s: &Story, mv: Move) -> f32 {
    use Move::*;
    let fam = s.family();
    let hot = s.heat >= 45.0;
    let t = s.truth;
    let you_lead = sit(w, s) == Sit::Me;
    match (fam, mv) {
        // A battle: the right call holds, the wrong one comes back.
        (Family::Battle, Back) => {
            if t < -0.2 {
                0.25
            } else if t > 0.3 {
                -0.15
            } else {
                0.05
            }
        }
        (Family::Battle, BackRival) => {
            if t > 0.3 {
                0.25
            } else if t < -0.2 {
                -0.2
            } else {
                0.0
            }
        }
        (Family::Battle, Open) => 0.15 - 0.2 * t.abs(),
        (_, Private) => 0.15,
        (_, Quiet) => {
            if hot {
                -0.04
            } else {
                0.03
            }
        }
        // Your job: owning a bad year plays; a denial is as good as the truth.
        (_, Own) if fam == Family::Seat || s.def().tone < 0 => 0.18,
        (_, Own) => 0.06,
        (_, Deny) => {
            if t > 0.3 {
                -0.3
            } else {
                0.15
            }
        }
        (_, FireBack) => {
            if you_lead {
                -0.12
            } else {
                -0.05
            }
        }
        (_, Open) if fam == Family::Wanted || fam == Family::You => -0.08,
        (_, Leak) => -0.02,
        // Off the field: discipline fits when he did it, loyalty when he didn't.
        (Family::OffField, Warn) => {
            if t > 0.0 {
                0.2
            } else {
                -0.1
            }
        }
        (Family::OffField, Back) => {
            if t < 0.0 {
                0.15
            } else {
                -0.22
            }
        }
        (_, Warn) => -0.05,
        (_, Back) => 0.08,
        (_, Praise) => {
            if s.def().tone < 0 {
                0.02
            } else {
                0.12
            }
        }
        (_, Challenge) => 0.06,
        (_, Promise) => 0.06,
        (_, BackRival) => 0.0,
        (_, Open) => 0.05,
    }
}

/// Makes a move on a story: its beat, what it does to the story and the
/// people, and the words for the outcome.
pub(crate) fn make(w: &mut World, id: u32, mv: Move, said: &str) -> Vec<String> {
    let Some(s) = w.storylines.get(id).cloned() else { return Vec::new() };
    if !s.phase.open() {
        return Vec::new();
    }
    // The same line twice in a week is no new move: nothing comes of it.
    if s.beats.iter().any(|b| b.by == Voice::You && b.date.days_to(w.today) < 7 && b.text.ends_with(said)) {
        return vec!["You said that this week already; nothing new comes of it.".into()];
    }
    // Said on the story before: the papers note it's the same line again.
    let again = s.beats.iter().any(|b| b.by == Voice::You && b.text.ends_with(said));
    let line = match (mv.public(), again) {
        (true, false) => format!("{}: {said}", scene::your_name(w)),
        (true, true) => format!("{}, again: {said}", scene::your_name(w)),
        (false, false) => said.to_string(),
        (false, true) => format!("Again: {said}"),
    };
    let seed = w.options.seed;
    let fam = s.family();
    let noise = (w.storylines.rng(seed).next() as f32 - 0.5) * 0.08;
    let q = (fit(w, &s, mv) + 0.12 * skill(w, mv.rating(fam)) + noise).clamp(-0.6, 0.6);
    let credible = 0.55 + 0.45 * w.folk.mine.straight.clamp(0.0, 1.0);
    let team = crate::world::folk::Team::seat(w);
    let mut out = Vec::new();
    // The story itself.
    if let Some(k) = w.storylines.idx(id) {
        let st = &mut w.storylines.open[k];
        // Walking back a public word costs: backing the lead and then the
        // change, or the other way.
        let walked_back = st.stance != 0
            && matches!(
                (st.stance, mv),
                (1, Move::BackRival | Move::Open | Move::Warn) | (-1, Move::Back | Move::Open)
            );
        st.handled = (st.handled + q - if walked_back { 0.12 } else { 0.0 }).clamp(-1.0, 1.0);
        st.moves = st.moves.saturating_add(1);
        match mv {
            Move::Back | Move::Deny | Move::Promise => st.stance = 1,
            Move::BackRival | Move::Warn => st.stance = -1,
            Move::Open => st.stance = 0,
            _ => {}
        }
        match mv {
            Move::Promise => st.said |= SAID_PROMISE,
            Move::Deny => st.said |= SAID_DENY,
            Move::Leak => st.said |= SAID_LEAK,
            Move::Own => st.said |= SAID_OWN,
            _ => {}
        }
        // The public's read: your word moves it by how far people believe you.
        match mv {
            Move::Back | Move::Deny => {
                st.belief += credible * 0.35 * (-1.0 - st.belief) * 0.6;
                st.sure = (st.sure + 0.12 * credible).min(0.95);
            }
            Move::BackRival | Move::Warn if fam != Family::OffField => {
                st.belief += credible * 0.35 * (1.0 - st.belief) * 0.6;
                st.sure = (st.sure + 0.12 * credible).min(0.95);
            }
            Move::Warn => st.belief = (st.belief + 0.25 * credible).min(1.0),
            Move::Open => st.belief *= 0.6,
            Move::Leak => {
                st.belief += 0.4 * (st.truth - st.belief);
                st.sure = (st.sure + 0.15).min(0.95);
            }
            _ => {}
        }
        st.belief = st.belief.clamp(-1.0, 1.0);
        // The sides: room, fans, media, boosters.
        let sides = &mut st.sides;
        let nudge = |v: &mut f32, by: f32| *v = (*v + by).clamp(0.05, 0.95);
        match mv {
            Move::Own => {
                nudge(&mut sides[1], 0.05);
                nudge(&mut sides[2], 0.04);
            }
            Move::FireBack => {
                nudge(&mut sides[0], 0.05);
                nudge(&mut sides[2], -0.08);
            }
            Move::Back | Move::Praise => nudge(&mut sides[0], 0.04),
            Move::Warn => {
                nudge(&mut sides[3], 0.03);
                nudge(&mut sides[0], -0.02);
            }
            Move::Quiet => nudge(&mut sides[2], -0.02),
            Move::Promise => nudge(&mut sides[1], 0.05),
            _ => {}
        }
        if mv == Move::Quiet {
            st.heat *= 0.92;
        }
    }
    // The beat.
    let text = line;
    let what = if mv.public() { BeatKind::Move } else { BeatKind::Private };
    let quiet_since = w.storylines.idx(id).map(|k| w.storylines.open[k].last_beat);
    add_beat(w, id, what, Voice::You, 255, mv.heat(), text);
    // A no-comment is no news: the story can still go quiet.
    if mv == Move::Quiet
        && let (Some(k), Some(was)) = (w.storylines.idx(id), quiet_since)
    {
        w.storylines.open[k].last_beat = was;
    }
    if mv == Move::Leak {
        // A leak goes out through a writer the next day.
        if let Some(k) = w.storylines.idx(id) {
            w.storylines.open[k].last_beat = w.today;
        }
        super::go_public(w, id, w.today);
    }
    if mv.public() && s.public {
        super::tell::post_beat(w, id);
    }
    // The people in it.
    let lead = s.lead_player();
    let rival = s.rival_player();
    let z = skill(w, Rating::Relationships);
    match mv {
        Move::Back if fam == Family::Battle => {
            if let Some(a) = lead {
                talks::promise(w, PromiseTo::Player(a), PromiseKind::Start, 0);
                if let Some(p) = w.folk.promises.last_mut() {
                    p.public = true;
                }
                talks::mood_event(w, a, Reason::Praise, 0.14);
            }
            if let Some(b) = rival {
                talks::mood_event(w, b, Reason::Benched, -0.12);
            }
            out.push(format!("You named {} the starter in public.", name(w, lead)));
        }
        Move::BackRival => {
            if let Some(b) = rival {
                talks::mood_event(w, b, Reason::Praise, 0.12);
            }
            if let Some(a) = lead {
                talks::mood_event(w, a, Reason::Benched, -0.14);
            }
            out.push(format!("You said in public that {} has earned his shot.", name(w, rival)));
        }
        Move::Back | Move::Praise => {
            if let Some(a) = lead {
                talks::mood_event(w, a, Reason::Praise, 0.1);
                bump_trust(w, a, 0.03);
                if mv == Move::Praise
                    && let Some(i) = super::pi(w, a)
                    && w.people[i].player.personality.traits.get(Trait::Ego) >= 13.0
                    && w.storylines.rng(seed).chance(0.15)
                {
                    w.people[i].player.personality.traits.shift(Trait::Ego, 0.4);
                }
            }
        }
        Move::Open | Move::Challenge => {
            for id in [lead, rival].into_iter().flatten() {
                if let Some(i) = super::pi(w, id) {
                    let comp = ((w.people[i].player.personality.competitiveness - 10.0) / 10.0) as f32;
                    let ego = w.people[i].player.personality.traits.z(Trait::Ego) as f32;
                    talks::mood_event(w, id, Reason::Noise, 0.05 * comp - 0.04 * ego.max(0.0));
                }
            }
        }
        Move::Warn => {
            if let Some(a) = lead {
                bump_trust(w, a, -0.08);
                talks::mood_event(w, a, Reason::Discipline, -0.1);
            }
            w.folk.mine.hard = (w.folk.mine.hard + 0.06).min(1.0);
        }
        Move::Private => {
            for id in [lead, rival].into_iter().flatten() {
                bump_trust(w, id, 0.04 * (1.0 + z).max(0.2));
                talks::mood_event(w, id, Reason::Talk, 0.04 * (1.0 + z).max(0.2));
            }
            out.push("You dealt with it in private.".into());
        }
        Move::Own => {
            if let Some(t) = team {
                for i in t.roster(w) {
                    w.people[i].trust = (w.people[i].trust + 0.006).min(1.0);
                }
            }
            w.folk.mine.straight = (w.folk.mine.straight + 0.03).min(1.0);
            out.push("You owned it in public.".into());
        }
        Move::FireBack => {
            if let Some(t) = team {
                for i in t.roster(w) {
                    w.people[i].mood = (w.people[i].mood + 0.015).min(1.0);
                }
            }
            out.push("You fired back in public.".into());
        }
        Move::Deny => out.push("You denied it on the record.".into()),
        Move::Promise => out.push("You made a promise in public.".into()),
        Move::Leak => out.push("It's out there now, and nobody can say it came from you.".into()),
        Move::Quiet => {}
    }
    // The boss and the writers.
    if let Some(t) = team {
        let boss = crate::world::folk::boss::boss(w, t);
        let by = match mv {
            Move::Own => {
                if boss.honesty >= 12.0 {
                    0.03
                } else {
                    0.01
                }
            }
            Move::FireBack => -0.04,
            Move::Warn if fam == Family::OffField => 0.03,
            Move::Back if fam == Family::OffField && s.truth > 0.0 => -0.03,
            Move::Leak | Move::Open if matches!(fam, Family::Wanted | Family::You) => -0.03,
            Move::Deny if matches!(fam, Family::Wanted | Family::You) => 0.015,
            _ => 0.0,
        };
        if by != 0.0 {
            w.folk.mine.nudge(boss.key(), by);
        }
        let writers = match mv {
            Move::Quiet => -0.015,
            Move::FireBack => -0.08,
            Move::Leak => 0.03,
            Move::Own | Move::Open => 0.01,
            _ => 0.005,
        };
        for r in cast::all(w, t, Role::Reporter) {
            w.folk.mine.nudge(r.key(), writers);
        }
    }
    out
}

fn name(w: &World, id: Option<u32>) -> String {
    id.and_then(|id| super::pi(w, id)).map_or_else(|| "him".to_string(), |i| w.people[i].name.clone())
}

fn bump_trust(w: &mut World, id: u32, by: f32) {
    if let Some(i) = super::pi(w, id) {
        w.people[i].trust = (w.people[i].trust + by).clamp(0.0, 1.0);
    }
}

/// At a story's ending: how your words held up. A public promise kept
/// pays, one broken costs double; a denial before you left is a lie
/// people remember; backing the one who won the battle was the right call.
pub(crate) fn settle(w: &mut World, id: u32) {
    let Some(k) = w.storylines.idx(id) else { return };
    let s = &mut w.storylines.open[k];
    if s.you.is_none() || s.moves == 0 {
        return;
    }
    let e = s.def().ending_key(s.ending);
    let v = if s.you == Some(Part::Rival) { -s.valence() } else { s.valence() };
    let mut by = 0.0;
    if s.said & SAID_PROMISE != 0 {
        by += if v > 0.2 { 0.25 } else { -0.35 };
    }
    if s.said & SAID_DENY != 0 {
        by += if matches!(e, "left" | "took_it" | "went_back" | "got_job") { -0.5 } else { 0.1 };
    }
    if s.family() == Family::Battle {
        let lead_won = matches!(e, "kept_job" | "held_job" | "won_back" | "fixed" | "arrived" | "stuck");
        let lead_lost = matches!(e, "won_job" | "benched" | "lost_job" | "gave_back");
        by += match s.stance {
            1 if lead_won => 0.15,
            1 if lead_lost => -0.2,
            -1 if lead_lost => 0.15,
            -1 if lead_won => -0.2,
            _ => 0.0,
        };
    }
    s.handled = (s.handled + by).clamp(-1.0, 1.0);
}

/// The stories a reporter asks you about: your open public ones, hottest
/// first, not asked about this week.
pub(crate) fn press_stories(w: &World, team: TeamKey) -> Vec<u32> {
    let d = w.today;
    let mut v: Vec<(f32, u32)> = w
        .storylines
        .open
        .iter()
        .filter(|s| s.public && s.phase.open() && s.you.is_some() && s.heat >= 20.0 && s.about(team))
        .filter(|s| !w.storylines.asked.iter().any(|a| a.0 == s.id && a.1.days_to(d) < 6))
        .map(|s| (s.heat, s.id))
        .collect();
    v.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
    v.into_iter().take(2).map(|x| x.1).collect()
}

/// What a story's family is, for the press to skip its own question on
/// the same thing.
pub(crate) fn press_family(w: &World, id: u32) -> Option<Family> {
    w.storylines.get(id).map(|s| s.family())
}

/// A reporter's question about a story.
pub(crate) fn press_question(w: &mut World, id: u32) -> String {
    let Some(s) = w.storylines.get(id).cloned() else { return String::new() };
    let f = super::tell::fill(w, &s);
    super::tell::line(w, &s, BeatKind::Quote, Some("ask"), &f).unwrap_or_else(|| {
        let q = super::tell::question(w, &s);
        format!("Coach, everyone is asking the same thing: {}", lower_first(&q))
    })
}

fn lower_first(s: &str) -> String {
    // Only a plain question word is lowered; names keep their capital.
    let first = s.split_whitespace().next().unwrap_or("");
    if !matches!(first, "Who" | "Will" | "Is" | "Can" | "How" | "Does" | "Did" | "When" | "What" | "Where" | "Has") {
        return s.to_string();
    }
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_lowercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

/// The answers on a press question about a story.
pub(crate) fn press_menu(w: &mut World, id: u32) -> Vec<Opt> {
    let Some(s) = w.storylines.get(id).cloned() else { return Vec::new() };
    let d = w.today;
    w.storylines.asked.retain(|a| a.1.days_to(d) < 14 && a.0 != id);
    w.storylines.asked.push((id, d));
    menu(w, &s, true)
}

/// Your answer to a press question about a story.
pub(crate) fn press_answer(w: &mut World, id: u32, opt: u32) -> Vec<String> {
    let Some(s) = w.storylines.get(id).cloned() else { return Vec::new() };
    let menu = menu(w, &s, true);
    let Some(o) = menu.get(opt as usize).cloned() else { return Vec::new() };
    make(w, id, o.mv, &o.label)
}

// --- Story meetings ---------------------------------------------------------

/// The most story meetings a week, and how often one story brings one.
const MEETINGS_A_WEEK: usize = 3;
const MEETING_GAP: i32 = 14;

/// A story you are in that runs hot brings a meeting.
pub(crate) fn opened(w: &mut World, id: u32) {
    consider(w, id);
}

/// Every day: your stories that are running hot get a meeting.
pub(crate) fn day(w: &mut World, d: Date) {
    if w.career.is_none() {
        return;
    }
    let mut ids: Vec<(f32, u32)> = w
        .storylines
        .open
        .iter()
        .filter(|s| s.you.is_some() && s.public && s.phase.open() && s.heat >= 30.0)
        .map(|s| (s.heat, s.id))
        .collect();
    ids.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
    for (_, id) in ids.into_iter().take(4) {
        consider(w, id);
    }
    w.storylines.met.retain(|m| m.1.days_to(d) < 60);
}

fn consider(w: &mut World, id: u32) {
    let d = w.today;
    let Some(s) = w.storylines.get(id).cloned() else { return };
    if w.career.is_none() || !s.public || !s.phase.open() || s.heat < 30.0 {
        return;
    }
    let Some(part) = s.you else { return };
    if !matches!(part, Part::Lead | Part::Decider | Part::Rival | Part::Suitor) {
        return;
    }
    if w.storylines.met.iter().any(|m| m.0 == id && m.1.days_to(d) < MEETING_GAP) {
        return;
    }
    if w.storylines.met.iter().filter(|m| m.1.days_to(d) < 7).count() >= MEETINGS_A_WEEK {
        return;
    }
    // A meeting already open on it.
    if w.folk.scenes.iter().any(|x| x.kind == Kind::Story && !x.done && x.a == id as u64) {
        return;
    }
    // Only when something new happened since the last meeting.
    let last_met = w.storylines.met.iter().filter(|m| m.0 == id).map(|m| m.1).max();
    if last_met.is_some_and(|m| !s.beats.iter().any(|b| b.date > m && b.what.public() && b.what != BeatKind::Move)) {
        return;
    }
    let stops = s.heat >= 55.0
        && matches!(part, Part::Lead | Part::Decider)
        && !w.folk.scenes.iter().any(|x| x.kind == Kind::Story && x.stops && x.date == d);
    let q = super::tell::question(w, &s);
    let mut sc = Scene::new(Kind::Story, d, q);
    sc.a = id as u64;
    sc.stops = stops;
    sc.place = place(&s);
    for m in s.cast.iter().take(4) {
        match m.who {
            Who::Player(p) => sc.who.push(scene::Who::Player(p)),
            Who::Coach(c) => sc.who.push(scene::Who::Coach(c as usize)),
            _ => {}
        }
    }
    scene::add(w, sc);
    w.storylines.met.push((id, d));
}

fn place(s: &Story) -> String {
    match s.family() {
        Family::Seat | Family::You | Family::Wanted => {
            "Your office, the door closed. Your phone hasn't stopped.".into()
        }
        Family::OffField => "Your office, early. The communications director is already waiting.".into(),
        Family::Battle => "The staff room, the depth chart on the whiteboard.".into(),
        Family::Recruiting => "The recruiting office, the board on the wall.".into(),
        _ => "Your office, the clips on the desk.".into(),
    }
}

/// Opens a story meeting: what has been said, the question, your answers.
pub(crate) fn open(w: &mut World, sc: &mut Scene) {
    let id = sc.a as u32;
    let Some(s) = w.storylines.get(id).cloned() else {
        sc.narrate("It has blown over.");
        sc.end();
        return;
    };
    if !s.phase.open() {
        sc.narrate("By the time you get to it, it's over.");
        sc.end();
        return;
    }
    let said: Vec<&super::Beat> = s.beats.iter().filter(|b| b.what.public() && !b.text.is_empty()).collect();
    let n = said.len();
    sc.narrate(format!(
        "{} The heat on it: {} of 100{}.",
        sc.place,
        s.heat.round(),
        match s.weeks.len() {
            0 | 1 => String::new(),
            k => format!(", {} weeks running", k),
        }
    ));
    for b in said.iter().skip(n.saturating_sub(3)) {
        sc.other(b.by.name(), b.text.clone());
    }
    sc.narrate(public_read(w, &s));
    // The clippings.
    let mut doc = scene::Doc {
        title: "What's been said".into(),
        cols: vec!["Date".into(), "Who".into(), "What".into()],
        ..Default::default()
    };
    for b in said.iter().rev().take(12) {
        doc.rows.push(scene::DocRow { pid: None, cells: vec![b.date.pretty(), b.by.name().into(), b.text.clone()] });
    }
    if !doc.rows.is_empty() {
        sc.docs.push(doc);
    }
    for o in menu(w, &s, false) {
        sc.pick(10 + o.mv.code(), o.label, o.mv.name(), o.mv.tone(), &o.hint);
    }
}

/// What the public thinks, in words.
pub fn public_read(w: &World, s: &Story) -> String {
    let sp = super::view::split(w, s);
    let (yes, no) = (s.belief > 0.35, s.belief < -0.35);
    // Which way people think it goes, in the question's own terms: the
    // insiders' picks for a recruit, the mock drafts for a prospect.
    let lean = match s.family() {
        Family::Recruiting => match sp.share {
            60.. => format!("Most insiders pick {}", sp.a),
            ..=40 if sp.b == "Elsewhere" => "Most insiders think he leaves".to_string(),
            ..=40 => format!("Most insiders pick {}", sp.b),
            _ => "The insiders' picks are split".to_string(),
        },
        Family::Draft => match sp.share {
            60.. => "Most mock drafts have him going high".to_string(),
            ..=40 => "Most mock drafts have him sliding".to_string(),
            _ => "The mock drafts are split on him".to_string(),
        },
        Family::Battle if yes => "Most expect a change at the top".to_string(),
        Family::Battle if no => "Most expect the starter to keep the job".to_string(),
        Family::Battle => "Nobody knows who starts".to_string(),
        Family::Seat | Family::Wanted | Family::Leaving if yes => "Most think he goes".to_string(),
        Family::Seat | Family::Wanted | Family::Leaving if no => "Most think he stays".to_string(),
        Family::Seat | Family::Wanted | Family::Leaving => "Nobody knows if he stays".to_string(),
        Family::OffField if yes => "Most expect the program to act".to_string(),
        Family::OffField if no => "Most expect it to blow over".to_string(),
        _ if yes => "Most expect it to happen".to_string(),
        _ if no => "Most doubt it".to_string(),
        _ => "Nobody knows which way it goes".to_string(),
    };
    let sure = if s.sure > 0.7 {
        "and they sound sure"
    } else if s.sure > 0.4 {
        "but it's far from settled"
    } else {
        "but most of it is talk"
    };
    format!("{lean}, {sure}.")
}

pub(crate) fn answer(w: &mut World, sc: &mut Scene, code: u32, picked: &Pick) {
    let id = sc.a as u32;
    let mv = Move::from_code(code.saturating_sub(10));
    if mv.public() {
        sc.you(&scene::your_name(w), picked.label.clone());
    } else {
        sc.narrate(picked.label.clone());
    }
    let out = make(w, id, mv, &picked.label);
    let tell = w.storylines.get(id).map(|s| how_it_lands(s, mv));
    if let Some(t) = tell {
        sc.narrate(t);
    }
    sc.outcome.extend(out);
    if sc.outcome.is_empty() {
        sc.outcome.push(format!("{}.", mv.name()));
    }
    sc.end();
}

/// A line on how the move went down, from the story's state after it.
fn how_it_lands(s: &Story, mv: Move) -> String {
    match mv {
        Move::Quiet => "The writers fill the space themselves.".into(),
        Move::Private => "Nothing reaches the papers. Inside the building, people noticed.".into(),
        Move::Leak => "By morning a writer has it, without your name on it.".into(),
        _ if s.handled > 0.25 => "It lands the way you meant it.".into(),
        _ if s.handled < -0.25 => "It doesn't land well. The clips the next morning say so.".into(),
        _ => "The clips the next morning are split.".into(),
    }
}

/// Handed to your staff: a no-comment from the communications office.
pub(crate) fn hand(w: &mut World, sc: &mut Scene) {
    let id = sc.a as u32;
    sc.handed = Some("the communications office".into());
    if let Some(s) = w.storylines.get(id).cloned()
        && s.phase.open()
    {
        let _ = make(w, id, Move::Quiet, "\"The program has no comment.\"");
        sc.outcome.push("Your staff put out a no-comment.".into());
    }
}

/// Why it matters to you, in one line, for the board.
pub fn stake_line(s: &Story) -> &'static str {
    match s.you {
        Some(Part::Lead) => "It's about you.",
        Some(Part::Decider) => "It's your call.",
        Some(Part::Rival) => "You're the other side.",
        Some(Part::Suitor) => "You want him; he decides.",
        Some(_) => "You're in it.",
        None => "",
    }
}

/// Whether a story has ended for the screens.
pub fn over(s: &Story) -> bool {
    !s.phase.open() || s.phase == Phase::Aftermath
}
