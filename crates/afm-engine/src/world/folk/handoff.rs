//! Hand-off projections (Cain 2026-11-15, "Decisions like delegating should
//! have clearer projections of what is gained and lost"): before you hand a
//! meeting to your staff, what each choice would do, side by side.
//!
//! Only read-only functions live here: nothing changes the world, and no
//! world result moves. The button names what happens (`label`): a man on
//! your staff does it (his rating sizes the result, shown as the range of
//! *your read* of him, never his true rating), your staff applies a fixed
//! standard, or nobody goes (a skip). Each kind says what you give up and
//! what you get by holding it. Fixed costs and the rule ladder are exact:
//! nothing in them is hidden.
//!
//! The handed one-on-one is worked from `talks::hand`'s own size at both
//! ends of your read of the coach's Relationships; yours from the talk
//! page's own staff reading (`talks::expect_parts`), on its five-word
//! scale. A recruit or transfer talk is worked on your file on him
//! (`pulls::file`), never his true wants: the handed pull against the pull
//! of your best talk, at both ends of your read of the man's ratings.
//!
//! Every size here repeats the code that does the thing; where the code
//! changes, change the projection with it (`docs/reference/how-it-works/
//! people.md`, "Hand-off projections").

use super::claims::Tier;
use super::scene::{Kind, Scene};
use super::{Team, arrests, boss, pitch, rules, takeover, talks};
use crate::calendar::{Date, Level};
use crate::world::World;
use serde::Serialize;

/// What kind of hand-off it is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Tone {
    /// A man on your staff does it, sized by his rating.
    Man,
    /// Your staff applies a fixed standard.
    Standard,
    /// Nobody goes.
    Skip,
}

/// One rating your read gives: a range, or your own exact one.
#[derive(Clone, Debug, Serialize)]
pub struct Rated {
    pub name: String,
    pub lo: u8,
    pub hi: u8,
    /// Your own rating (exact), as opposed to your read of his.
    pub yours: bool,
}

/// A bar of a projection. `centered` bars start at a tick (no change) and
/// run left (down) or right (up); the rest run from the left to `top`. The
/// solid part is the likely move, the faint part how far it can go.
#[derive(Clone, Debug, Serialize)]
pub struct Bar {
    pub name: String,
    pub centered: bool,
    pub solid: f32,
    pub faint: f32,
    /// The bar's full length, in the same unit.
    pub top: f32,
}

/// One side of the choice.
#[derive(Clone, Debug, Serialize)]
pub struct Side {
    pub title: String,
    pub aside: String,
    pub reads: Vec<Rated>,
    /// The small heading over the word ("Likely", "Likely, with your best
    /// move").
    pub head: String,
    pub word: String,
    pub text: String,
    pub bars: Vec<Bar>,
    /// "You give up" or "You get".
    pub list_head: String,
    pub list: Vec<String>,
    pub button: String,
}

/// The whole choice for one meeting.
#[derive(Clone, Debug, Serialize)]
pub struct Handoff {
    pub id: u32,
    pub tone: Tone,
    /// The hand-off button's words ("Hand to wide receivers coach Davis").
    pub label: String,
    pub title: String,
    pub hand: Side,
    pub hold: Side,
    /// Whether nothing in the projection is hidden (a fixed standard).
    pub exact: bool,
    /// The note under the two sides.
    pub note: String,
}

/// A meeting that falls inside a holiday, and what happens to it.
#[derive(Clone, Debug, Serialize)]
pub struct HolidayRow {
    pub id: u32,
    pub date: i32,
    pub title: String,
    pub tone: Tone,
    pub line: String,
}

/// The holiday preview: what falls inside, what can't be known yet and what
/// your staff holds.
#[derive(Clone, Debug, Serialize)]
pub struct HolidayPreview {
    pub until: i32,
    pub days: i32,
    pub rows: Vec<HolidayRow>,
    pub unknown: Vec<String>,
    pub duties: Vec<String>,
}

fn pts(x: f32) -> f32 {
    (x * 1000.0).round() / 10.0
}

fn up(s: &str) -> String {
    let mut c = s.chars();
    c.next().map_or_else(String::new, |f| f.to_uppercase().collect::<String>() + c.as_str())
}

fn last(name: &str) -> &str {
    let parts: Vec<&str> = name.split_whitespace().collect();
    match parts.as_slice() {
        [.., l, s] if matches!(*s, "Jr." | "II" | "III" | "IV") => l,
        [.., l] => l,
        _ => name,
    }
}

/// "Hand to wide receivers coach Davis" for a man on your staff.
fn coach_label(w: &World, c: usize) -> String {
    if w.career.as_ref().is_some_and(|k| k.coach == c) {
        return "Hand to your staff".to_string();
    }
    let co = &w.coaches[c];
    let role = co.job.map_or("coach", |j| j.role.name());
    format!("Hand to {role} {}", last(&co.name))
}

fn level_of(w: &World, s: &Scene) -> Level {
    s.team.or(Team::seat(w)).map_or(Level::College, |t| t.level())
}

/// The button's tone and words for a meeting: cheap, for every card.
pub fn label(w: &World, s: &Scene) -> (Tone, String) {
    match s.kind {
        Kind::OneOnOne => {
            let by = super::person(w, s.a as u32).and_then(|i| talks::position_coach_now(w, i));
            (Tone::Man, by.map_or_else(|| "Hand to your staff".to_string(), |c| coach_label(w, c)))
        }
        k if k.is_pitch() => {
            let by = super::person(w, s.a as u32)
                .filter(|_| (s.b as usize) < w.programs.len())
                .and_then(|i| super::dialogue::caller(w, i, s.b as usize));
            (Tone::Man, by.map_or_else(|| "Hand to your staff".to_string(), |c| coach_label(w, c)))
        }
        Kind::FlightRisks => (Tone::Man, "Hand to your position coaches".to_string()),
        _ => {
            let f = fixed(w, s);
            (f.tone, f.label)
        }
    }
}

/// A standard or a skip, worked out in words.
struct Fixed {
    tone: Tone,
    label: String,
    word: String,
    text: String,
    bars: Vec<(String, f32)>,
    gives: Vec<String>,
    gets: Vec<String>,
    hold_word: String,
    hold_text: String,
}

impl Fixed {
    fn new(tone: Tone, label: &str, word: &str, text: impl Into<String>) -> Fixed {
        Fixed {
            tone,
            label: label.to_string(),
            word: word.to_string(),
            text: text.into(),
            bars: Vec::new(),
            gives: Vec::new(),
            gets: Vec::new(),
            hold_word: "Your call".to_string(),
            hold_text: String::new(),
        }
    }
    fn bar(mut self, name: &str, delta: f32) -> Fixed {
        self.bars.push((name.to_string(), delta));
        self
    }
    fn gives(mut self, x: &[&str]) -> Fixed {
        self.gives = x.iter().map(|s| s.to_string()).collect();
        self
    }
    fn gets(mut self, x: &[&str]) -> Fixed {
        self.gets = x.iter().map(|s| s.to_string()).collect();
        self
    }
    fn hold(mut self, word: &str, text: &str) -> Fixed {
        self.hold_word = word.to_string();
        self.hold_text = text.to_string();
        self
    }
}

fn skip(label: &str, text: impl Into<String>) -> Fixed {
    Fixed::new(Tone::Skip, label, "Nobody goes", text)
}

/// What a kind does when you hand it on, read from the hand it runs
/// (`scene::dispatch_hand`): the standard it applies, or the skip.
fn fixed(w: &World, s: &Scene) -> Fixed {
    let level = level_of(w, s);
    let team = s.team.or(Team::seat(w));
    let who = || team.map_or_else(|| "your staff".to_string(), |t| takeover::staff_hand(w, t, s.kind));
    match s.kind {
        Kind::Discipline if s.b >= arrests::ARREST => match arrests::staff_ruling_for(w, s) {
            Some((resp, what)) => {
                let court = s.b == arrests::COURT;
                Fixed::new(
                    Tone::Standard,
                    "Let the staff rule",
                    &up(&resp.words(level)),
                    format!(
                        "By your staff's own code for {}: {}. The same however the day goes.",
                        if court { "a court day" } else { "an arrest" },
                        resp.words(level)
                    ),
                )
                .gives(&["Hearing him out, and ruling a game lighter or harder than the code.", "Anything you'd say to him before the room reads it."])
                .gets(&["Your own ruling, on what you know of him.", "The room reads your call, not the code's."])
                .hold("Your ruling", &format!("The charge: {what}. The staff would rule as above."))
            }
            None => skip("Let it go", "Nothing is left to rule."),
        },
        Kind::Discipline => {
            let kind = rules::RuleKind::ALL[s.b as usize % rules::RuleKind::ALL.len()];
            let strict = w.folk.rules.strict(kind);
            let n = strict + s.c.max(0) as u8;
            let resp = rules::staff_response(n, level);
            let earlier = s.c.max(0);
            Fixed::new(
                Tone::Standard,
                "Let the staff rule",
                &up(&resp.words(level)),
                format!(
                    "By your book on {} and {} earlier break{}. The same ladder every time.",
                    kind.name().to_lowercase(),
                    earlier,
                    if earlier == 1 { "" } else { "s" }
                ),
            )
            .gives(&["Hearing him out first.", "Going lighter or harder than your book."])
            .gets(&["His side, in his words, before you rule.", "A call you chose: the room reads that, not the ladder."])
            .hold("Your ruling", "Anything from a talk to the book's top; the ladder gives the above.")
        }
        Kind::Ritual => Fixed::new(
            Tone::Standard,
            "Leave it to the veterans",
            "Rituals run as they always have",
            "The veterans run it their way. Whether one crosses the line rides on who leads them: more ego than professionalism and it is likelier.",
        )
        .gives(&["Any limits: no money, no hands, nothing off campus."])
        .gets(&["Your limits on it, or none of it."])
        .hold("Your limits", "Set limits, leave it to the room, or ban it."),
        Kind::Press | Kind::PressIntro => {
            let mut f = Fixed::new(
                Tone::Standard,
                "Send a statement",
                "A statement is read",
                "The communications office reads one, and the writers fill the space themselves.",
            )
            .bar("Each beat writer's trust", -0.04)
            .gives(&["Your own answers, and what the writers print from them."])
            .gets(&["The questions, your answers and the next day's clips."])
            .hold("Your answers", "What you say is what runs.");
            if matches!(team, Some(Team::Nfl(_))) && s.kind == Kind::Press {
                f.text.push_str(" The league fines you for skipping your media obligations.");
                f.gives.push("A league fine.".into());
            }
            f
        }
        Kind::BossFirst | Kind::BossMeeting => {
            let name = team.map_or_else(|| "Your boss".to_string(), |t| boss::boss(w, t).name);
            skip("Skip it", format!("Nobody can go for you. {name} doesn't forget it."))
                .bar(&format!("{name}'s trust in you"), -0.06)
                .gets(&["Your goals and your standing settled face to face.", "What your boss makes of your season, in their own words."])
                .hold("Face to face", "Where your review and your goals are set.")
        }
        Kind::ReachOut => skip("Never call back", "You never call back. He cools on you a little.")
            .bar("His warmth toward you", -0.05)
            .gets(&["The tie kept warm, and what he knows."])
            .hold("A kept tie", "A call a month keeps a friend."),
        Kind::Trip => skip("Don't go", "You don't go. Nothing else moves.")
            .gets(&["What you'd see and who you'd meet there."])
            .hold("Your time", "A trip costs hours of your week."),
        Kind::JobCall => skip("Let the call go", "You let the call go. Nothing else moves.")
            .gets(&["What the caller had in mind."])
            .hold("Their offer", "You hear it before you decide."),
        Kind::Interview => skip("Skip it", "Nobody interviews for you. The candidate isn't seen and the opening stays open.")
            .gets(&["Your own read of the candidate, face to face."])
            .hold("Your read", "Hire on what you saw."),
        Kind::Firing => skip("Let it drop", "You let it drop. He stays.")
            .gets(&["The decision, made your way."])
            .hold("Your decision", "Keep him or let him go."),
        Kind::Proposal => skip("Let it drop", "You never answer; the staff keeps things as they were.")
            .bar("The coach's trust in you", -0.02)
            .gets(&["Your answer to what he asked for."])
            .hold("Your answer", "Take it, change it or turn it down."),
        Kind::StarPower => skip("Let it go", "You never get back to him. He takes that as an answer.")
            .bar("His mood", -0.12)
            .gets(&["His case, in his own words, and your answer."])
            .hold("Your answer", "What he gets to hear from you."),
        Kind::SecondOpinion => Fixed::new(
            Tone::Standard,
            "Let the doctors arrange it",
            "Arranged by the medical staff",
            "The medical staff arranges the second opinion. The timeline holds.",
        )
        .bar("His trust in you", 0.03)
        .gives(&["Choosing whether he waits or plays through it."])
        .gets(&["Your say on the opinion and what you do with it."])
        .hold("Your say", "Wait for the opinion, or push him back early."),
        Kind::Family => skip("Let it slide", "The week gets away from you. Nothing else moves.")
            .gets(&["Time at home, and what it settles."])
            .hold("Your time", "Costs hours of your week."),
        Kind::Story => Fixed::new(
            Tone::Standard,
            "Put out a no-comment",
            "A no-comment",
            "The communications office says: the program has no comment.",
        )
        .gives(&["Your own line on it."])
        .gets(&["Your own move on a story you are in."])
        .hold("Your move", "Answer, lean on it or stay quiet."),
        Kind::Backchannel => skip("Let the call go", "You let it go. Nothing is said back.")
            .gets(&["What the caller knows, and your answer."])
            .hold("Your answer", "Take the tip or not."),
        Kind::Negotiation => skip("Let it go", "Your staff hangs up politely. Nothing is signed.")
            .gets(&["Your own terms, and what they will give."])
            .hold("Your terms", "A deal is what you agree to."),
        Kind::DraftTalk => skip("Let it go", "You let it go for now.")
            .gets(&["What he tells you, and your answer."])
            .hold("Your read", "The talk goes into your board."),
        Kind::Sitdown | Kind::Reference => skip("Let it go", "You leave it to your staff, and nothing comes of it.")
            .gets(&["What they say to you, and your answer."])
            .hold("Your answer", "The sit-down is yours."),
        Kind::CollegeCall => skip("Let it go to voicemail", "You never call back. They go on the film.")
            .gets(&["What the school says about your player."])
            .hold("Your word", "Your word carries with the coach on the other end."),
        Kind::JobInterview => skip("Skip it", "You miss the interview. The committee notices.")
            .gets(&["Your case for the job, face to face."])
            .hold("Your case", "They hire on what they hear."),
        Kind::StaffMeeting => Fixed::new(
            Tone::Standard,
            "Let your assistant decide",
            "Half the change goes in",
            "Your assistant head coach runs the meeting. The coordinator gets half of what he asked.",
        )
        .gives(&["Taking all of it or none."])
        .gets(&["All, none or half of his change, your way."])
        .hold("Your call", "The plan moves exactly as far as you say."),
        Kind::CombineTalk => Fixed::new(
            Tone::Standard,
            "End it here",
            "What you asked stands",
            "The interview ends where it is: what you have already asked is in his file.",
        )
        .gives(&["The questions you hadn't asked yet."])
        .gets(&["The rest of the time, for the questions you choose."])
        .hold("Your questions", "The room's time is yours."),
        Kind::TeamTalk | Kind::StaffLine => skip("Let it pass", "Nobody talks to the room. Nothing moves.")
            .gets(&["Your words to them, in the tone you choose."])
            .hold("Your words", "A talk moves the room by what you say."),
        Kind::ParentCall => Fixed::new(
            Tone::Standard,
            "Let your staff return the call",
            "The call is returned",
            "Your staff returns it. The parent wanted you.",
        )
        .bar("His trust in you", -0.01)
        .gets(&["The parent hears you, and you hear them."])
        .hold("Your word", "What a family hears from the head coach."),
        Kind::TeamMeeting => Fixed::new(
            Tone::Standard,
            "Say a few plain words at practice",
            "No first impression either way",
            "Nobody else can hold your first team meeting: it waits as a short, plain word at the end of a workout.",
        )
        .gives(&["Your first message to the room: what you stand for."])
        .gets(&["Your first message, in the tone you choose."])
        .hold("Your message", "The room remembers its first meeting."),
        Kind::Holdover => Fixed::new(
            Tone::Standard,
            "Keep the holdover staff through spring",
            "Everyone stays for now",
            "Nobody is let go; you can still let anyone go from the staff screen.",
        )
        .gets(&["Your read of each man before you keep him."])
        .hold("Your read", "Keep or let go, man by man."),
        Kind::Booster => {
            let pass = super::backers::is_pass_test(s);
            let f = Fixed::new(
                Tone::Standard,
                &format!("Let {} take it", who()),
                "Taken without you",
                format!("{} takes the meeting instead of you, and the booster notices.", who()),
            )
            .gets(&["What the booster asks for, face to face."])
            .hold("Your word", "A booster remembers who showed up.");
            if pass {
                let mut f = f;
                f.text = "Nobody else can pull his pass: it stands, and the room saw it.".into();
                f
            } else {
                f.bar("The booster's trust in you", -0.08)
            }
        }
        Kind::Recruiting => Fixed::new(
            Tone::Standard,
            &format!("Let {} make the calls", who()),
            "Calls made without you",
            format!("{} makes the calls. The commits hear from the staff, not from you.", who()),
        )
        .gets(&["Your own voice on the commits' calls."])
        .hold("Your calls", "A commit remembers who called."),
        Kind::Parents => Fixed::new(
            Tone::Standard,
            &format!("Let {} run it", who()),
            "Run without you",
            format!("{} runs the parents' meeting. Parents wanted to see you; some say so.", who()),
        )
        .bar("Loud parents' sons: trust", -0.02)
        .gets(&["The parents, face to face."])
        .hold("Your word", "Parents judge the program by you."),
        Kind::OldCoach => skip("Skip the visit", "You never find the time to stop by his classroom. He notices, and so do the people who still call him Coach.")
            .bar("The old coach's trust in you", -0.1)
            .gets(&["His word, and his goodwill."])
            .hold("A visit", "A short visit costs little."),
        Kind::Gm => Fixed::new(
            Tone::Standard,
            "Let the GM run it",
            "Run on his terms",
            "The general manager runs the meeting on his own terms and leaves you his notes.",
        )
        .gets(&["Your say in the meeting."])
        .hold("Your say", "Both of you in the room."),
        Kind::SigneeRelease => Fixed::new(
            Tone::Standard,
            &format!("Let {} release him", who()),
            "He is let out",
            format!("{} takes the call and lets him out of his signing, as schools usually do.", who()),
        )
        .gives(&["Holding him to his letter, or talking him round."])
        .gets(&["Your answer to his ask: keep him, or let him go on your terms."])
        .hold("Your answer", "Keep him or let him go."),
        Kind::Trainer => Fixed::new(
            Tone::Standard,
            "Let the trainers run their plan",
            "Nobody is pushed",
            "Nobody is pushed back early.",
        )
        .gives(&["Pushing a man back early."])
        .gets(&["Your say on who comes back when."])
        .hold("Your say", "Pushing a man back costs him later."),
        _ => Fixed::new(
            Tone::Standard,
            &format!("Let {} take the notes", who()),
            "Notes left for you",
            "You get the notes afterward. Nothing is decided in the room without you.",
        )
        .gets(&["The meeting itself, in your words."])
        .hold("Your meeting", "What you say in the room is what stands."),
    }
}

fn side_fixed(f: &Fixed, s: &Scene, hours: Option<String>) -> (Side, Side) {
    let bars: Vec<Bar> = f
        .bars
        .iter()
        .map(|(n, d)| Bar { name: n.clone(), centered: true, solid: pts(*d), faint: pts(*d), top: 15.0 })
        .collect();
    let hand = Side {
        title: f.label.clone(),
        aside: match f.tone {
            Tone::Man => "your read".to_string(),
            Tone::Standard => "a fixed standard".to_string(),
            Tone::Skip => "nobody goes".to_string(),
        },
        reads: Vec::new(),
        head: "What happens".to_string(),
        word: f.word.clone(),
        text: f.text.clone(),
        bars,
        list_head: "You give up".to_string(),
        list: f.gives.clone(),
        button: f.label.clone(),
    };
    let hold = Side {
        title: "Hold it yourself".to_string(),
        aside: hours.unwrap_or_default(),
        reads: Vec::new(),
        head: "You decide".to_string(),
        word: f.hold_word.clone(),
        text: f.hold_text.clone(),
        bars: Vec::new(),
        list_head: "You get".to_string(),
        list: f.gets.clone(),
        button: if s.stops { "Hold it".to_string() } else { "Take it".to_string() },
    };
    (hand, hold)
}

/// The one range of two ends as bar numbers: the solid part is where the
/// two ends agree (nearest zero), the faint part how far the far end goes.
fn range_bar(name: &str, a: f32, b: f32, top: f32) -> Bar {
    let (a, b) = (pts(a), pts(b));
    let solid = if a * b > 0.0 { if a.abs() < b.abs() { a } else { b } } else { 0.0 };
    let faint = if a.abs() > b.abs() { a } else { b };
    Bar { name: name.to_string(), centered: true, solid, faint, top }
}

fn words_range(lo: f32, hi: f32) -> String {
    let (a, b) = (talks::word_of(lo.min(hi)), talks::word_of(lo.max(hi)));
    if a == b { a.to_string() } else { format!("{a} to {}", b.to_lowercase()) }
}

fn sure_words(q: f32) -> &'static str {
    match q {
        x if x >= 0.75 => "they're sure",
        x if x >= 0.55 => "fairly sure",
        x if x >= 0.4 => "a guess",
        _ => "a guess, and they don't know him well",
    }
}

fn one_on_one(w: &World, s: &Scene) -> Option<Handoff> {
    let h = talks::hand_one(w, s)?;
    let hold = talks::hold_one(w, s);
    let name = h.adviser.map(|c| super::coach_link(w, c));
    let label = h.adviser.map_or_else(|| "Hand to your staff".to_string(), |c| coach_label(w, c));
    let seen = [h.trust[0] + h.mood[0], h.trust[1] + h.mood[1]];
    let word = words_range(seen[0], seen[1]);
    let dir = match (seen[0] + seen[1]) / 2.0 {
        x if x > 0.005 => "a touch up",
        x if x < -0.005 => "down",
        _ => "where they were",
    };
    let by_whom = name.clone().unwrap_or_else(|| h.by.to_string());
    let text = format!(
        "Trust and mood each {dir}. The same however the talk goes{}.{}",
        if h.rel.is_some() { format!(": only your read of {by_whom} makes it a range") } else { String::new() },
        if h.spoke { " What you already said still lands." } else { "" }
    );
    let mut give = vec![
        "What he wants: his answer stays with him and his coach, and you hear it second-hand.".to_string(),
        "A promise: only you can make one.".to_string(),
    ];
    if h.asked {
        let (lo, hi) = h.rel.unwrap_or((9, 9));
        let pc = |r: u8| ((0.02 * (r as f32 - 8.0)) * 100.0).round() as i32;
        give.push(format!(
            "Keeping him: {by_whom} asks him to stay, and his lean to leave eases by {} to {} in 100, by your read.",
            pc(lo).max(0),
            pc(hi).max(0)
        ));
    }
    let hand = Side {
        title: label.clone(),
        aside: "your read".to_string(),
        reads: h
            .rel
            .map(|r| vec![Rated { name: "Relationships".into(), lo: r.0, hi: r.1, yours: false }])
            .unwrap_or_default(),
        head: "Likely".to_string(),
        word,
        text,
        bars: vec![range_bar("Trust", h.trust[0], h.trust[1], 15.0), range_bar("Mood", h.mood[0], h.mood[1], 15.0)],
        list_head: "You give up".to_string(),
        list: give,
        button: label.clone(),
    };
    let you = w.career.as_ref().map_or(10.0, |c| c.you.relationships);
    let hours = format!("{:.0} h of your week left", talks::hours_left(w));
    let hold_side = match hold {
        Some(x) => {
            let e = &x.best.2;
            let half = e.err / 2.0;
            let sure = sure_words(e.q);
            let by = name.clone().unwrap_or_else(|| "your staff".to_string());
            Side {
                title: "Hold it yourself".to_string(),
                aside: hours,
                reads: vec![Rated {
                    name: "Relationships".into(),
                    lo: you.round() as u8,
                    hi: you.round() as u8,
                    yours: true,
                }],
                head: "Likely, with your best move".to_string(),
                word: talks::word_of(e.seen).to_string(),
                text: format!(
                    "{}, {}. {by} reads it; {sure}. A wrong move can still cost you: {} reads \"{}\".",
                    x.best.0.name(),
                    x.best.1.name().to_lowercase(),
                    x.worst.0.name().to_lowercase(),
                    talks::word_of(x.worst.2).to_lowercase()
                ),
                bars: vec![
                    Bar {
                        name: "Trust".into(),
                        centered: true,
                        solid: pts(e.dt + half),
                        faint: pts(x.reach.0.max(e.dt + half)),
                        top: 15.0,
                    },
                    Bar {
                        name: "Mood".into(),
                        centered: true,
                        solid: pts(e.dm + half),
                        faint: pts(x.reach.1.max(e.dm + half)),
                        top: 15.0,
                    },
                ],
                list_head: "You get".to_string(),
                list: vec![
                    "His own words on what he wants, on his file.".to_string(),
                    "The chance to promise, warn or be straight with him.".to_string(),
                ],
                button: "Hold the meeting".to_string(),
            }
        }
        None => Side {
            title: "Hold it yourself".to_string(),
            aside: hours,
            reads: Vec::new(),
            head: "You decide".to_string(),
            word: "Your call".to_string(),
            text: String::new(),
            bars: Vec::new(),
            list_head: "You get".to_string(),
            list: vec!["His own words, and your answer.".to_string()],
            button: "Hold the meeting".to_string(),
        },
    };
    Some(Handoff {
        id: s.id,
        tone: Tone::Man,
        label,
        title: super::plain(&s.title),
        hand,
        hold: hold_side,
        exact: false,
        note: "Bars: the solid part is the likely move, the faint part how far it can go; the tick is no change. Your staff's read, never the true numbers.".to_string(),
    })
}

/// The five push words of a pull, against the most a talk can do.
fn push_word(pull: f32, scale: f32) -> &'static str {
    let f = pull / (super::case::CASE_TOP * scale).max(1e-6);
    match f {
        x if x >= 0.8 => "A strong push",
        x if x >= 0.5 => "A solid push",
        x if x >= 0.25 => "A fair push",
        x if x > 0.02 => "A small push",
        _ => "Next to nothing",
    }
}

fn tier_phrase(t: Tier) -> &'static str {
    match t {
        Tier::Elite => "the best on his list",
        Tier::Strong => "strong",
        Tier::Fair => "middling",
        Tier::Weak => "weak",
    }
}

/// Which way a recruit or transfer talk leans, from your read: your own
/// best talk pulls more than the handed one at both ends of your read of
/// the man, his pulls more at both, or it is close and could go either way.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Lean {
    Yours,
    His,
    Close,
}

pub(crate) fn lean(yours: f32, hand: [f32; 2]) -> Lean {
    if yours > hand[0].max(hand[1]) {
        Lean::Yours
    } else if yours < hand[0].min(hand[1]) {
        Lean::His
    } else {
        Lean::Close
    }
}

fn pitch_talk(w: &World, s: &Scene) -> Option<Handoff> {
    let h = pitch::hand_pitch(w, s)?;
    let you = &w.career.as_ref()?.you;
    let label = h.caller.map_or_else(|| "Hand to your staff".to_string(), |c| coach_label(w, c));
    let man = h.caller.filter(|_| !h.mine).map(|c| super::coach_link(w, c)).unwrap_or_else(|| "your staff".to_string());
    let scale = match s.kind {
        Kind::RecruitCall | Kind::PortalCall | Kind::FamilyCall => super::case::PHONE,
        Kind::RecruitMessage => super::case::MESSAGE,
        _ => 1.0,
    };
    let mut reads = Vec::new();
    if let Some(r) = h.rec {
        reads.push(Rated { name: "Recruiting".into(), lo: r.0, hi: r.1, yours: false });
    }
    if let Some(r) = h.rel {
        reads.push(Rated { name: "Relationships".into(), lo: r.0, hi: r.1, yours: false });
    }
    let ratio = |p: f32| if h.yours > 0.0005 { (p / h.yours).clamp(0.0, 1.2) } else { 0.0 };
    let (rl, rh) = (ratio(h.hand[0]).min(ratio(h.hand[1])), ratio(h.hand[0]).max(ratio(h.hand[1])));
    let five = |x: f32| ((x * 100.0 / 5.0).round() * 5.0) as i32;
    let leads: Vec<String> = h.top.iter().map(|(g, _)| super::pulls::word(*g).to_string()).collect();
    let lead = if leads.is_empty() { String::new() } else { format!(", if he leads with {}", leads.join(" and ")) };
    let loose = h.rec.is_some_and(|r| r.1 - r.0 >= 3);
    let mid = (h.hand[0] + h.hand[1]) / 2.0;
    let (word, text) = if h.over {
        (
            "Nothing left to say".to_string(),
            "He has made his choice, or signed: there is nothing left to talk about.".to_string(),
        )
    } else if h.stands {
        ("Your case stands".to_string(), "The talk is under way: nobody makes more of it for you.".to_string())
    } else {
        (
            push_word(mid, scale).to_string(),
            format!(
                "{} About {} to {} in 100 of yours{lead}.{}",
                match lean(h.yours, h.hand) {
                    Lean::Yours => "Your own talk should do more.",
                    Lean::His => "His talk should do more than yours.",
                    Lean::Close => "Close: either could come out ahead.",
                },
                five(rl),
                five(rh),
                if loose {
                    " His read of what they want is looser than your file, so he may spend an exchange on something they don't weigh."
                } else {
                    ""
                }
            ),
        )
    };
    let top = (super::case::CASE_TOP * scale).max(1e-6);
    let bars = if h.over || h.stands {
        Vec::new()
    } else {
        vec![Bar {
            name: "Pull".into(),
            centered: false,
            solid: pts(h.hand[0].min(h.hand[1]) / top * 10.0).max(0.0),
            faint: pts(h.hand[0].max(h.hand[1]) / top * 10.0).max(0.0),
            top: 100.0,
        }]
    };
    let mut give = Vec::new();
    if !h.stands && !h.over {
        give.push(
            "The free questions: who decides, what the other schools tell them, how things are at home.".to_string(),
        );
        give.push(format!(
            "The real reaction: {man} sees through a polite answer {} to {} times in 100. You, {}.",
            (h.catch[0] * 100.0).round() as i32,
            (h.catch[1] * 100.0).round() as i32,
            (h.yours_catch * 100.0).round() as i32
        ));
        if h.promise {
            give.push("Any promise to them.".to_string());
        }
    }
    let proof = if h.top.is_empty() {
        "Your file has no lean on him yet, so your staff goes in blind.".to_string()
    } else {
        let bits: Vec<String> =
            h.top.iter().map(|(g, t)| format!("{}: {} proof", g.name().to_lowercase(), tier_phrase(*t))).collect();
        format!("Your proof there: {}. Overselling where it is weak takes some of it back.", bits.join("; "))
    };
    let hand = Side {
        title: label.clone(),
        aside: "your read".to_string(),
        reads,
        head: "Likely".to_string(),
        word,
        text,
        bars,
        list_head: "You give up".to_string(),
        list: give,
        button: label.clone(),
    };
    let hold = Side {
        title: if matches!(s.kind, Kind::RecruitCall | Kind::PortalCall | Kind::FamilyCall) {
            "Take the call"
        } else {
            "Go yourself"
        }
        .to_string(),
        aside: format!("{} exchanges", h.exchanges),
        reads: vec![
            Rated {
                name: "Recruiting".into(),
                lo: you.recruiting.round() as u8,
                hi: you.recruiting.round() as u8,
                yours: true,
            },
            Rated {
                name: "Relationships".into(),
                lo: you.relationships.round() as u8,
                hi: you.relationships.round() as u8,
                yours: true,
            },
        ],
        head: "Likely, on his file's top two".to_string(),
        word: push_word(h.yours, scale).to_string(),
        text: proof,
        bars: vec![Bar {
            name: "Pull".into(),
            centered: false,
            solid: pts(h.yours / top * 10.0).max(0.0),
            faint: pts(h.yours / top * 10.0).max(0.0),
            top: 100.0,
        }],
        list_head: "You get".to_string(),
        list: vec![
            "Their answers on his file, and what your read catches.".to_string(),
            "The pick of answer each time they ask.".to_string(),
        ],
        button: if s.stops { "Go to it" } else { "Take it" }.to_string(),
    };
    Some(Handoff {
        id: s.id,
        tone: Tone::Man,
        label,
        title: super::plain(&s.title),
        hand,
        hold,
        exact: false,
        note: "The pull bar is scaled to what a talk can add. Where he stands in the race stays in words, as today: his wants and your case are never shown.".to_string(),
    })
}

fn flight_risks(w: &World, s: &Scene) -> Handoff {
    let n = w.folk.takeover.as_ref().map_or(0, |t| t.risks.len());
    let f = Fixed::new(
        Tone::Man,
        "Hand to your position coaches",
        "A word from each",
        format!(
            "Each of the {n} flight risks hears from his own position coach: the lean to leave eases by about 2 in 100 for each point of the coach's Relationships over 8. A word from you carries more."
        ),
    )
    .gives(&["Your own word to each of them."])
    .gets(&["Your own word to each of them, in the tone you choose."])
    .hold("Your word", "A word from the head coach carries more.");
    let (hand, hold) = side_fixed(&f, s, None);
    Handoff {
        id: s.id,
        tone: Tone::Man,
        label: f.label,
        title: super::plain(&s.title),
        hand,
        hold,
        exact: false,
        note: String::new(),
    }
}

/// Both sides of handing a meeting on, for the screen that asks before it
/// does.
pub fn preview(w: &World, id: u32) -> Result<Handoff, String> {
    let s = w.folk.scenes.iter().find(|s| s.id == id).ok_or("No such meeting.")?;
    if s.done {
        return Err("That meeting is over.".into());
    }
    let man = match s.kind {
        Kind::OneOnOne => one_on_one(w, s),
        k if k.is_pitch() => pitch_talk(w, s),
        Kind::FlightRisks => Some(flight_risks(w, s)),
        _ => None,
    };
    Ok(man.unwrap_or_else(|| {
        let f = fixed(w, s);
        let (hand, hold) = side_fixed(&f, s, None);
        Handoff {
            id: s.id,
            tone: f.tone,
            label: f.label,
            title: super::plain(&s.title),
            hand,
            hold,
            exact: true,
            note: "Nothing in this is hidden: the standard is the same every time.".to_string(),
        }
    }))
}

/// One line for a holiday row: who does it and what it comes to.
fn one_line(w: &World, s: &Scene) -> (Tone, String) {
    match preview(w, s.id) {
        Ok(h) if h.tone == Tone::Man => {
            let who = h.label.trim_start_matches("Hand to ").to_string();
            let you = h.hold.word.to_lowercase();
            let he = h.hand.word.to_lowercase();
            let line = if he == you {
                format!("{who}: {he}, as it would for you.")
            } else {
                format!("{who}: {he}. You'd likely get \"{you}\".")
            };
            (Tone::Man, line)
        }
        Ok(h) => (h.tone, format!("{}. {}", h.label, h.hand.text)),
        Err(e) => (Tone::Skip, e),
    }
}

/// What a holiday to `until` would do to the meetings already set: the
/// ones that fall inside, each with its line, what can't be known yet and
/// the duties your staff holds. The holiday hands every meeting dated
/// before the day it comes back (`scene::day`), a call that waits a week
/// lapses.
pub fn holiday(w: &World, until: Date) -> HolidayPreview {
    let mut rows: Vec<HolidayRow> = w
        .folk
        .scenes
        .iter()
        .filter(|s| !s.done)
        .filter(|s| if s.stops { s.date < until } else { s.date.add(7) < until })
        .map(|s| {
            let (tone, line) = one_line(w, s);
            HolidayRow { id: s.id, date: s.date.0, title: super::plain(&s.title), tone, line }
        })
        .collect();
    rows.sort_by_key(|r| (r.date, r.id));
    let mut unknown = vec![
        "A rule break: your staff rules by your book.".to_string(),
        "A player asking to talk: his position coach has it.".to_string(),
    ];
    if matches!(Team::seat(w), Some(Team::High(_))) {
        unknown.pop();
    }
    let duties = crate::world::front_office::staff_duty_lines(w);
    HolidayPreview { until: until.0, days: w.today.days_to(until), rows, unknown, duties }
}
