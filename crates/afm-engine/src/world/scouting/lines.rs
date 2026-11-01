//! The words scouts, coaches, analysts and the media use (scouting topics
//! 5, 7, 18; people topic 5's line bank): trait words by level, what a look
//! saw for each attribute, character notes by source, analysts' takes and
//! draft-night lines. Each line is picked by a key from the player and the
//! writer, so the same report reads the same every time it's opened, and
//! two players rarely get the same words.

use crate::attributes::Attr;
use crate::world::season::key_uniform;

/// Picks one of `opts` by a key (stable for the same player and writer).
pub fn pick<'a>(opts: &[&'a str], key: u64, id: u64) -> &'a str {
    if opts.is_empty() {
        return "";
    }
    let k = (key_uniform(key ^ 0x11E5, id) * opts.len() as f64) as usize;
    opts[k.min(opts.len() - 1)]
}

/// A trait read in words (topic 5): never a number. `centre` is on 1-20.
pub fn trait_word(k: usize, c: f64) -> &'static str {
    let band = if c >= 16.0 {
        4
    } else if c >= 12.5 {
        3
    } else if c >= 8.0 {
        2
    } else if c >= 5.0 {
        1
    } else {
        0
    };
    const W: [[&str; 5]; 13] = [
        ["won't put the work in", "needs a push", "does what's asked", "a hard worker", "first in, last out"],
        ["tunes coaches out", "hard to coach", "takes coaching", "soaks up coaching", "a coach's dream"],
        [
            "shies from the fight",
            "competes in spurts",
            "competes",
            "a fierce competitor",
            "hates to lose more than he loves to win",
        ],
        [
            "football is a job to him",
            "questions about his love of it",
            "likes the game",
            "loves the game",
            "lives for football",
        ],
        ["content where he is", "modest goals", "wants to get better", "driven", "wants to be the best ever"],
        ["out the door at the first chance", "loyal to himself first", "loyal", "very loyal", "loyal to a fault"],
        [
            "money barely matters",
            "money isn't his driver",
            "wants to be paid fairly",
            "money matters a lot",
            "follows the money",
        ],
        ["never loses his cool", "even-tempered", "can flare up", "a short fuse", "explodes"],
        ["humble", "quiet confidence", "confident", "a big personality", "it's all about him"],
        ["a follower", "leads by example only", "leads when asked", "a leader", "a leader of men"],
        ["a red flag off the field", "loose habits", "handles himself", "a pro's pro", "a pro's pro from day one"],
        ["struggles with change", "slow to settle", "settles in", "adapts quickly", "fits in anywhere"],
        ["rattled by setbacks", "setbacks linger", "handles setbacks", "bounces back", "unflappable"],
    ];
    W[k.min(12)][band]
}

/// What a look saw in an attribute: a strength, then a worry.
pub fn attr_phrase(a: Attr, good: bool) -> &'static str {
    use Attr::*;
    let (g, b) = match a {
        Speed => ("plays fast; real long speed", "lacks long speed"),
        Acceleration => ("explosive first step", "slow to build up speed"),
        Agility => ("sudden change of direction", "stiff when he has to redirect"),
        Strength => ("strong at the point", "gets moved too easily"),
        Balance => ("stays on his feet through contact", "on the ground too much"),
        Flexibility => ("bends; loose hips", "tight hips"),
        Jumping => ("a lot of bounce", "plays below the rim"),
        Stamina => ("still going hard in the fourth quarter", "fades late in drives"),
        ArmStrength => ("drives the deep out with ease", "the arm runs out past 40 yards"),
        ShortAccuracy => ("puts the quick game on the numbers", "misses easy throws"),
        MediumAccuracy => ("fits it into tight windows", "late and off target over the middle"),
        DeepAccuracy => ("drops the deep ball in the bucket", "overthrows the vertical shots"),
        Touch => ("feathers it over linebackers", "everything is a fastball"),
        ThrowOnMove => ("accurate rolling out", "loses it off-platform"),
        QuickRelease => ("the ball is out in a blink", "a long wind-up"),
        Footwork => ("clean drops and a reset base", "sloppy feet in the pocket"),
        PocketPresence => ("feels the rush and climbs", "bails on clean pockets"),
        BallHandling => ("sells the fake", "the fakes fool nobody"),
        EyeManipulation => ("moves the safety with his eyes", "stares down his first read"),
        BallSecurity => ("high and tight; never puts it on the ground", "loose with the ball"),
        BallCarrierVision => ("sees the cutback before it opens", "runs into his own blockers"),
        Elusiveness => ("makes the first man miss", "goes down at first contact"),
        LegDrive => ("falls forward every time", "stops his feet on contact"),
        Catching => ("strong, natural hands", "body-catches and drops"),
        ContestedCatch => ("wins the 50-50 ball", "loses the contested catch"),
        BallTracking => ("tracks it over the shoulder", "misjudges the deep ball"),
        BodyControl => ("toe-taps on the sideline", "can't adjust to the ball"),
        RouteRunning => ("sinks his hips at the break", "rounds off his routes"),
        Release => ("beats press at the line", "gets jammed at the line"),
        PassBlocking => ("mirrors the rusher", "beaten around the edge"),
        RunBlocking => ("moves people in the run game", "can't sustain his run blocks"),
        BlockingInSpace => ("climbs to the second level", "whiffs on the move"),
        HandUse => ("violent, accurate hands", "late with the punch"),
        Anchor => ("anchors against the bull rush", "gets walked back into the quarterback"),
        PowerRush => ("converts speed to power", "no power in his rush"),
        FinesseRush => ("a full bag of rush moves", "one move and stuck"),
        BlockShedding => ("sheds blocks to make the tackle", "stays blocked"),
        ManCoverage => ("sticky in man coverage", "grabby and lost in man"),
        ZoneCoverage => ("reads the quarterback in zone", "lost in zone"),
        Press => ("re-routes at the line", "can't get hands on receivers"),
        Disguise => ("hides the coverage until the snap", "shows his hand early"),
        Tackling => ("a sure tackler", "misses tackles in space"),
        Pursuit => ("takes the right angles", "overruns the play"),
        Blitzing => ("times the blitz", "the blitz is always late"),
        Traffic => ("slips through the trash", "gets lost in traffic"),
        KickPower => ("a big leg", "short on the long ones"),
        ShortKickAccuracy => ("automatic inside 40", "shaky on short kicks"),
        LongKickAccuracy => ("accurate from distance", "sprays the long ones"),
        KickTrajectory => ("gets it up fast", "kicks low and gets blocked"),
        KickoffTechnique => ("places kickoffs in the corners", "kickoffs float to the middle"),
        PuntPower => ("booms it", "short punts"),
        PuntPlacement => ("pins it inside the 10", "outkicks his coverage"),
        GetOff => ("quick get-off", "slow to get the kick away"),
        LongSnapping => ("perfect, fast snaps", "the snaps wander"),
        ShotgunSnap => ("clean shotgun snaps", "shotgun snaps sail"),
        FootballIq => ("knows everyone's job", "slow to learn the playbook"),
        PlayRecognition => ("diagnoses plays fast", "fooled by play action"),
        Anticipation => ("plays a step ahead", "always a step late"),
        Decisions => ("makes the right call", "careless with his choices"),
        Composure => ("calm when it matters", "rattled when it matters"),
        Concentration => ("locked in every snap", "lapses late in drives"),
        Discipline => ("plays his gap and his eyes stay clean", "undisciplined: grabs and jumps early"),
        Communication => ("runs the calls at the line", "quiet before the snap"),
        Toughness => ("plays through pain", "comes off the field when it hurts"),
        Motor => ("relentless motor", "takes plays off"),
        Competitiveness => ("wins the one-on-ones", "loses the one-on-ones"),
        Physicality => ("strikes on contact", "avoids contact"),
    };
    if good { g } else { b }
}

/// Who a character note came from, at each level (topic 5).
pub fn note_source(kind: super::Kind, key: u64, id: u64) -> (&'static str, super::file::Shade) {
    use super::Kind as K;
    use super::file::Shade as S;
    match kind {
        K::School => {
            let opts: [(&str, S); 4] = [
                ("his strength coach", S::True),
                ("the head trainer", S::True),
                ("the equipment manager", S::True),
                ("his position coach", S::Warm),
            ];
            opts[(key_uniform(key ^ 0x50C, id) * 4.0) as usize % 4]
        }
        K::Interview | K::Visit | K::LocalDay => ("our interview", S::True),
        K::Background => ("club security", S::True),
        K::Social => ("his social media", S::True),
        K::Practice | K::Game => ("his high school coach", S::Warm),
        K::Camp => ("our camp coaches", S::True),
        K::Call => {
            let opts: [(&str, S); 3] =
                [("his high school coach", S::Warm), ("his counselor", S::True), ("a teacher", S::True)];
            opts[(key_uniform(key ^ 0xCA11, id) * 3.0) as usize % 3]
        }
        K::JuniorDay | K::Official | K::Unofficial | K::HomeVisit => ("his family", S::Defends),
        K::OldStaff => ("the staff he left", S::Cold),
        K::AllStar => ("the all-star coaches", S::True),
        K::Director | K::Gm | K::CrossCheck | K::Coach => ("people at his school", S::True),
        _ => ("his tape", S::True),
    }
}

/// A note on a trait from what the source saw, in the source's own
/// words, warmed or cooled by how the source runs (topic 5's "insane
/// detail": concrete things that happened).
pub fn trait_note(k: usize, centre: f64, shade: super::file::Shade, key: u64, id: u64) -> String {
    use super::file::Shade;
    let high = centre >= 13.0;
    let low = centre <= 7.5;
    let o: &[&str] = match (k, high, low) {
        (0, true, _) => &[
            "First one in the building at 5:40, last one out of the film room.",
            "Asked for extra reps after practice; the staff had to send him home.",
            "Came back from the summer in the best shape on the team.",
            "Lifts on his own on Sundays when the building is closed.",
        ],
        (0, _, true) => &[
            "Late to two lifts in the fall; cut corners in conditioning.",
            "Came back from the summer 14 pounds heavy.",
            "Coasts through the walk-throughs; the staff push him every day.",
            "Skipped summer workouts more than once.",
        ],
        (1, true, _) => &[
            "Fixes a mistake the next rep after one word from his coach.",
            "Takes notes in meetings and asks the second question.",
            "Changed his stance overnight when the line coach asked.",
        ],
        (1, _, true) => &[
            "Argues with his position coach on the field.",
            "Goes back to his old habits the moment the coach looks away.",
            "Rolled his eyes at a correction in front of the team.",
        ],
        (2, true, _) => &[
            "Wins every drill and wants a rematch when he doesn't.",
            "The one-on-ones get heated when he's in them.",
            "Plays the scout team like it's a playoff game.",
        ],
        (2, _, true) => &["Disappears when a game turns against his team.", "Lets the hard reps go to someone else."],
        (3, true, _) => &[
            "Watches film on the bus and talks football at dinner.",
            "Would play for nothing; he says so and means it.",
        ],
        (3, _, true) => &[
            "Talks about life after football more than about football.",
            "Seems to play because he's good at it, not because he loves it.",
        ],
        (4, true, _) => &[
            "Asked about our depth chart before he asked about our scheme.",
            "Talks about the Hall of Fame, and doesn't sound like he's joking.",
        ],
        (5, true, _) => {
            &["Turned down a bigger offer to stay with his teammates.", "Still close with his youth coach."]
        }
        (5, _, true) => &["Left his last school without telling the coaches.", "Always looking at the next place."],
        (6, true, _) => &[
            "Asked about NIL money in the first ten minutes.",
            "Has a marketing agent already; the money talk came early.",
        ],
        (7, true, _) => &[
            "Got into it with a teammate at practice; the coaches separated them.",
            "Two unsportsmanlike flags last season.",
            "Shouted at an official after the whistle and was benched for a series.",
        ],
        (7, _, true) => &["Never lost his cool, even when a teammate got hurt on a cheap shot."],
        (8, true, _) => &[
            "Talked about himself for 18 minutes.",
            "Called himself the best player in the class, twice.",
            "Celebrates his own tackles with the team down 20.",
        ],
        (8, _, true) => &["Gave the credit to his offensive line before anyone asked.", "Quiet, a little guarded."],
        (9, true, _) => &[
            "Teammates say he runs the locker room.",
            "Voted captain as a junior; the seniors followed him.",
            "Pulled the younger players together after a loss.",
        ],
        (9, _, true) => &["Teammates keep their distance.", "Never says a word in the huddle."],
        (10, true, _) => &[
            "Treats the equipment staff with respect; they'd do anything for him.",
            "Never late for a meeting in four years.",
            "Rehabbed his injury ahead of schedule, on his own time.",
        ],
        (10, _, true) => &[
            "Missed curfew on a road trip.",
            "Late to meetings; lost his starting spot for a game over it.",
            "The campus police know him by name.",
        ],
        (11, true, _) => &["Moved schools at 16 and was a captain by fall.", "Picked up a new position in a week."],
        (11, _, true) => &["Struggled after his position coach left.", "Homesick his first year away."],
        (12, true, _) => &[
            "Threw three interceptions and won the game on the next drive.",
            "Lost his starting job, kept his head down and took it back.",
        ],
        (12, _, true) => &["Pouted for a week after losing reps.", "A bad game stays with him for days."],
        _ => &[],
    };
    let line = pick(o, key ^ (k as u64) << 8, id);
    if line.is_empty() {
        return String::new();
    }
    let lead = match shade {
        Shade::Warm => pick(&["He'll tell you: ", "Glowing: ", "Sold hard: "], key ^ 0x3A, id),
        Shade::Defends => pick(&["The family says: ", "Defended: ", "His mother's word: "], key ^ 0x3B, id),
        Shade::Cold => pick(&["Their side: ", "Cool on him: ", "From the staff he left: "], key ^ 0x3C, id),
        Shade::True => "",
    };
    format!("{lead}{line}")
}

/// A family note (topic 5): the parents at games and online, who decides.
pub fn family_notes(p: &crate::world::Person, key: u64) -> Vec<String> {
    use crate::world::background::{Bloodline, Decider, ParentStyle};
    let id = p.player.id as u64;
    let f = &p.family;
    let mut v = Vec::new();
    v.push(format!("Home: {}.", f.structure.phrase()));
    v.push(
        match f.parent_style {
            ParentStyle::LoudAtGames => pick(
                &[
                    "His father yells at the officials every game.",
                    "A parent was asked to leave the stands last season.",
                    "The loudest voice in the bleachers is his dad's.",
                ],
                key ^ 0xFA1,
                id,
            ),
            ParentStyle::SocialMedia => pick(
                &[
                    "His mother runs his Instagram and posts every offer.",
                    "A parent tags every college coach in his highlights.",
                    "The family's social media is all about his recruitment.",
                ],
                key ^ 0xFA2,
                id,
            ),
            ParentStyle::CalmPrivate => pick(
                &[
                    "A calm, private family; they let the coaches coach.",
                    "Quiet parents who sit at the 30 every Friday.",
                ],
                key ^ 0xFA3,
                id,
            ),
            ParentStyle::Absent => pick(
                &["His parents aren't around his football.", "Nobody from home comes to his games."],
                key ^ 0xFA4,
                id,
            ),
        }
        .to_string(),
    );
    v.push(
        match f.decider {
            Decider::Himself => "He makes his own decisions.",
            Decider::Parent => "A parent makes the call on everything.",
            Decider::Grandparent => "His grandmother has the final word.",
            Decider::Handler => "A cousin handles everything; get to him first.",
        }
        .to_string(),
    );
    if f.bloodline == Bloodline::NflPlayer {
        v.push("His father played in the NFL.".to_string());
    } else if f.bloodline == Bloodline::CollegePlayer {
        v.push("His father played college football.".to_string());
    }
    v
}

/// An analyst's take on a pick (topic 18), in the desk's own voice.
pub fn pick_take(good: bool, reach: bool, key: u64, id: u64) -> &'static str {
    if reach {
        pick(
            &[
                "A reach. I had him two rounds later.",
                "They fell in love with the traits. Bold.",
                "I don't see it. He'll have to prove me wrong.",
                "That's a lot of pick for a project.",
            ],
            key,
            id,
        )
    } else if good {
        pick(
            &[
                "Value. He shouldn't have been there.",
                "A steal at this spot.",
                "Exactly the player they needed, and the best one left.",
                "Day-one starter. Love it.",
                "Great fit for what they run.",
            ],
            key,
            id,
        )
    } else {
        pick(
            &[
                "Fair value. Solid player.",
                "Right about where I had him.",
                "Fills a need. Fine pick.",
                "No complaints here.",
            ],
            key,
            id,
        )
    }
}

/// A class grade the morning after (topic 18).
pub fn class_grade(score: f64) -> &'static str {
    if score >= 2.0 {
        "A"
    } else if score >= 1.2 {
        "A-"
    } else if score >= 0.6 {
        "B+"
    } else if score >= 0.1 {
        "B"
    } else if score >= -0.4 {
        "B-"
    } else if score >= -0.9 {
        "C+"
    } else if score >= -1.5 {
        "C"
    } else {
        "D"
    }
}

/// One reader's own read of a player, for what he says in the room
/// (bug 18): his range, his looks, the tier it lands in, how the player
/// fits what we run, and where the room stands.
pub struct Voice<'a> {
    pub name: &'a str,
    /// His read of the player now, low and high.
    pub read: (u8, u8),
    /// The room's centre.
    pub room: f64,
    pub looks: usize,
    /// The tier his read lands in ("an NFL starter").
    pub tier: &'a str,
    /// His read of what our system asks at the spot against the rest of
    /// his read: above zero fits.
    pub fit: Option<f64>,
    /// He reads the numbers, not the tape.
    pub numbers: bool,
}

/// What a reader says in the room, built from his own read: where he
/// stands against the room, how sure his looks make him, the tier, the fit.
pub fn voice(v: &Voice, key: u64, id: u64) -> String {
    let (lo, hi) = v.read;
    let n = v.name;
    let gap = (lo as f64 + hi as f64) / 2.0 - v.room;
    let width = hi.saturating_sub(lo);
    let mut out = if gap >= 3.0 {
        pick(
            &[
                "I'm higher on {n} than the room. I have him {lo}-{hi}.",
                "I'll go higher than all of you on {n}: {lo}-{hi}.",
            ],
            key,
            id,
        )
    } else if gap <= -3.0 {
        pick(
            &[
                "I'm lower on {n} than the room. I have him {lo}-{hi}.",
                "I can't get as high as you on {n}. {lo}-{hi} for me.",
            ],
            key,
            id,
        )
    } else {
        pick(&["I'm with the room on {n}, {lo}-{hi}.", "{n} is {lo}-{hi} for me, about where you all are."], key, id)
    }
    .replace("{n}", n)
    .replace("{lo}", &lo.to_string())
    .replace("{hi}", &hi.to_string());
    let looks = v.looks;
    out.push(' ');
    out.push_str(&if v.numbers {
        "That's what his numbers say, not the tape.".to_string()
    } else if looks >= 6 && width <= 6 {
        format!("I've seen him {looks} times and I'm sure of it.")
    } else if width >= 12 {
        format!("On {looks} {} I can't pin him down yet.", crate::world::folk::noun(looks, "look"))
    } else {
        format!("That's on {looks} looks.")
    });
    out.push(' ');
    let tier = v.tier;
    let mut cap = tier.chars();
    let tier = cap.next().map_or(String::new(), |c| c.to_uppercase().collect::<String>() + cap.as_str());
    out.push_str(&format!("{tier} for me."));
    match v.fit {
        Some(f) if f >= 3.0 => out.push_str(" He fits what we run."),
        Some(f) if f <= -3.0 => out.push_str(" He isn't built for what we run."),
        _ => {}
    }
    out
}

/// A player's answer in an interview (SK15), in his own voice: band 0 is a
/// poor answer, 1 an ordinary one, 2 a strong one.
pub fn interview_answer(topic: &str, band: u8, key: u64, id: u64) -> String {
    let opts: &[&str] = match (topic, band.min(2)) {
        ("whiteboard", 2) => &[
            "He drew it back clean, then asked what the backside tackle does against a stunt.",
            "He had the protection and the hot answer before you finished the diagram.",
        ],
        ("whiteboard", 1) => &[
            "He got the base of it, and needed a nudge on the adjustment.",
            "He followed it, slowly, and wrote it down to keep it.",
        ],
        ("whiteboard", _) => &[
            "He nodded through it, and lost the thread when you changed the front.",
            "He couldn't draw it back. He said they ran it simpler at school.",
        ],
        ("worst_tape", 2) => &[
            "\"That's on me. I peeked in the backfield and he ran right past me. I fixed it the next week.\"",
            "\"I hate that play. Here's what I saw, and here's what I should have seen.\"",
        ],
        ("worst_tape", 1) => &[
            "\"Yeah, that one's bad. The call was late, but I should've been there.\"",
            "\"Not my best day. We had a lot going on that week.\"",
        ],
        ("worst_tape", _) => &[
            "\"The safety was supposed to be over the top. That's not my fault.\"",
            "He went quiet, then said the coaches never told him the check.",
        ],
        ("incident", 2) => &[
            "\"I was wrong. I made a bad choice and I let my team down. Here's what I changed.\"",
            "\"I won't make excuses. I talked to my coach the next morning and I've been clean since.\"",
        ],
        ("incident", 1) => &[
            "\"It got blown up bigger than it was, but I know how it looks.\"",
            "\"I was in the wrong place. I don't go out like that anymore.\"",
        ],
        ("incident", _) => {
            &["\"Everybody was doing it. I'm the one who got caught.\"", "He said he didn't want to talk about it."]
        }
        ("family", 2) => &[
            "\"My mom worked two jobs so I could play. Everything I do is for her and my little brothers.\"",
            "\"My dad coached me in peewee. He still calls after every game.\"",
        ],
        ("family", 1) => &[
            "\"It's me, my mom and my grandma. They're coming to the draft.\"",
            "\"We moved around a lot. My uncle kept me on track.\"",
        ],
        ("family", _) => {
            &["\"Some of them want a lot from me now. I'm handling it.\"", "He kept it short and looked at his agent."]
        }
        ("football", 2) => &[
            "\"I'd play for free. I'm in the building before the coaches.\"",
            "\"I want to be the best ever at my spot. I watch film on the bus.\"",
        ],
        ("football", 1) => &[
            "\"I love it. It's my way to take care of my family.\"",
            "\"I want to play a long time and get a second contract.\"",
        ],
        _ => &["\"It's a job. I'm good at it, so I do it.\"", "\"I want to get paid. After that, we'll see.\""],
    };
    pick(opts, key, id).to_string()
}
