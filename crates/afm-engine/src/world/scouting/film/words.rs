//! The film room's words (update 3, lane 9): the questions a session can
//! settle at each position (SK47), what your eye catches on a snap for each
//! attribute (SK43), how hard each is to see, and what he does after the
//! whistle (SK51). Every line is chosen by what he really is, so the tape
//! never lies; whether you see it is your eye's part. Sizes are Claude's,
//! proposed, not confirmed.

use crate::attributes::Attr;
use crate::positions::Position;

/// One question a session can settle (SK47): the attributes it rests on
/// and its words.
#[derive(Clone, Copy, Debug)]
pub struct Ask {
    pub key: &'static str,
    pub text: &'static str,
    pub attrs: &'static [Attr],
}

const fn ask(key: &'static str, text: &'static str, attrs: &'static [Attr]) -> Ask {
    Ask { key, text, attrs }
}

/// The questions a position's tape can answer, the first the usual one.
pub fn asks(pos: Position) -> &'static [Ask] {
    use Attr::*;
    use Position::*;
    match pos {
        QB => {
            const A: &[Ask] = &[
                ask(
                    "qb_read",
                    "Does he read the field fast enough for our level?",
                    &[Decisions, PlayRecognition, Anticipation],
                ),
                ask(
                    "qb_throws",
                    "Can he make the throws our offense asks for?",
                    &[MediumAccuracy, DeepAccuracy, ArmStrength],
                ),
                ask("qb_pocket", "Does he hang in the pocket under pressure?", &[PocketPresence, Composure]),
                ask("qb_broken", "Can he make plays when it breaks down?", &[ThrowOnMove, Agility]),
            ];
            A
        }
        RB => {
            const A: &[Ask] = &[
                ask(
                    "rb_vision",
                    "Does he see the hole and make the first man miss?",
                    &[BallCarrierVision, Elusiveness],
                ),
                ask("rb_protect", "Can he pick up the blitz?", &[PassBlocking, PlayRecognition]),
                ask("rb_hands", "Is he a threat out of the backfield?", &[Catching, RouteRunning]),
                ask("rb_ball", "Does he protect the ball in traffic?", &[BallSecurity, LegDrive]),
            ];
            A
        }
        WR => {
            const A: &[Ask] = &[
                ask("wr_press", "Can he beat press at our level?", &[Release, RouteRunning]),
                ask("wr_air", "Does he win the ball in the air?", &[ContestedCatch, BallTracking]),
                ask("wr_deep", "Does he separate deep?", &[Speed, Acceleration, RouteRunning]),
                ask("wr_hands", "Are his hands reliable?", &[Catching, Concentration]),
            ];
            A
        }
        TE => {
            const A: &[Ask] = &[
                ask("te_block", "Can he block at the point of attack?", &[RunBlocking, HandUse]),
                ask("te_routes", "Is he a real receiving threat?", &[RouteRunning, Catching]),
                ask("te_air", "Does he win in traffic?", &[ContestedCatch, BodyControl]),
            ];
            A
        }
        OT => {
            const A: &[Ask] = &[
                ask("ot_speed", "Can he hold up against speed off the edge?", &[PassBlocking, Footwork, Agility]),
                ask("ot_power", "Can he anchor against power?", &[Anchor, Strength]),
                ask("ot_run", "Does he move people in the run game?", &[RunBlocking, LegDrive]),
                ask("ot_hands", "Is his hand work clean?", &[HandUse, Discipline]),
            ];
            A
        }
        IOL => {
            const A: &[Ask] = &[
                ask("iol_power", "Can he anchor against power?", &[Anchor, Strength]),
                ask("iol_run", "Does he move people in the run game?", &[RunBlocking, LegDrive]),
                ask("iol_pass", "Does he pass off stunts and twists?", &[PassBlocking, PlayRecognition]),
                ask("iol_hands", "Is his hand work clean?", &[HandUse, Discipline]),
            ];
            A
        }
        EDGE => {
            const A: &[Ask] = &[
                ask("edge_speed", "Can he win with speed off the edge at our level?", &[FinesseRush, Acceleration]),
                ask("edge_power", "Does he have a power move to go with it?", &[PowerRush, Strength]),
                ask("edge_run", "Does he set the edge against the run?", &[BlockShedding, Tackling]),
                ask("edge_motor", "Does he chase when the ball goes away?", &[Motor, Pursuit]),
            ];
            A
        }
        DT => {
            const A: &[Ask] = &[
                ask("dt_push", "Can he push the pocket?", &[PowerRush, Strength]),
                ask("dt_double", "Does he hold up against double teams?", &[BlockShedding, Strength]),
                ask("dt_motor", "Does he play hard every snap?", &[Motor, Pursuit]),
                ask("dt_hands", "Does he win with his hands?", &[HandUse, FinesseRush]),
            ];
            A
        }
        LB => {
            const A: &[Ask] = &[
                ask("lb_keys", "Does he read his keys and fit the run?", &[PlayRecognition, Traffic]),
                ask("lb_cover", "Can he cover in space?", &[ZoneCoverage, ManCoverage]),
                ask("lb_tackle", "Is he a sure tackler in space?", &[Tackling, Pursuit]),
                ask("lb_blitz", "Can he get home on the blitz?", &[Blitzing, Acceleration]),
            ];
            A
        }
        CB => {
            const A: &[Ask] = &[
                ask("cb_press", "Can he play press at our level?", &[Press, ManCoverage]),
                ask("cb_motor", "Does he play hard when the ball goes away?", &[Motor, Pursuit]),
                ask("cb_outside", "Is he quick enough to stay outside?", &[Speed, Agility]),
                ask("cb_zone", "Does he read routes in zone?", &[ZoneCoverage, PlayRecognition]),
            ];
            A
        }
        S => {
            const A: &[Ask] = &[
                ask("s_deep", "Can he play the deep half?", &[ZoneCoverage, PlayRecognition, Speed]),
                ask("s_support", "Is he a reliable last line against the run?", &[Tackling, Pursuit]),
                ask("s_man", "Can he cover a slot or a tight end in man?", &[ManCoverage, Agility]),
            ];
            A
        }
        K => {
            const A: &[Ask] = &[
                ask("k_leg", "Does he have the leg for long field goals?", &[KickPower, LongKickAccuracy]),
                ask("k_clutch", "Is he steady when it counts?", &[Composure, ShortKickAccuracy]),
            ];
            A
        }
        P => {
            const A: &[Ask] = &[ask("p_flip", "Can he flip the field?", &[PuntPower, PuntPlacement])];
            A
        }
        LS => {
            const A: &[Ask] = &[ask("ls_snap", "Are his snaps on the money every time?", &[LongSnapping, Composure])];
            A
        }
    }
}

/// How hard an attribute is to see on tape, 0-1: technique and effort are
/// subtle (a false step, eyes dropping before contact, a lazy backside
/// chase); results anyone sees (he caught it, he outran them).
pub fn subtle(a: Attr) -> f64 {
    use Attr::*;
    match a {
        Footwork | HandUse | Press | Release | RouteRunning | Decisions | PlayRecognition | Anticipation
        | Discipline | Motor | BlockShedding | EyeManipulation | PocketPresence | Disguise | Traffic | Anchor
        | Communication | Concentration | Balance | Flexibility => 0.7,
        ManCoverage | ZoneCoverage | PassBlocking | RunBlocking | BlockingInSpace | FinesseRush | PowerRush
        | BallCarrierVision | Elusiveness | BodyControl | ThrowOnMove | Touch | QuickRelease | Pursuit | Blitzing
        | LegDrive | Composure | Toughness | Competitiveness | Physicality | FootballIq => 0.45,
        _ => 0.2,
    }
}

/// The level's usual mark for an attribute that matters (rating points):
/// a high school starter's, a college starter's, an NFL starter's.
pub fn level_mark(stage: crate::world::Stage) -> f64 {
    use crate::world::Stage;
    match stage {
        Stage::Pro => 70.0,
        Stage::College | Stage::Declared => 60.0,
        _ => 48.0,
    }
}

/// 0 a liability, 1 poor, 2 ordinary, 3 good, 4 a standout, against the
/// level's mark.
pub fn band(truth: f64, mark: f64) -> usize {
    let z = (truth - mark) / 7.0;
    if z >= 1.2 {
        4
    } else if z >= 0.4 {
        3
    } else if z >= -0.4 {
        2
    } else if z >= -1.2 {
        1
    } else {
        0
    }
}

/// What a sharp eye sees of an attribute on one snap, by his band (0-4):
/// the line the tape gives, true to what he is.
pub fn cue(a: Attr, band: usize) -> &'static str {
    use Attr::*;
    let b = band.min(4);
    let lines: [&'static str; 5] = match a {
        Press => [
            "Lunges at the jam and whiffs; the receiver is past him before he turns.",
            "Chest over his toes at the jam. He reaches and the receiver swipes his hands away.",
            "Gets a hand on him at the line, nothing more.",
            "Patient feet, a late jab with the off hand, then he mirrors.",
            "Two hands land inside the numbers and he rides the receiver to the sideline. Textbook.",
        ],
        ManCoverage => [
            "Opens his hips early and gets turned around at the break.",
            "A step late out of the break, trailing the whole way.",
            "Stays in phase to the break, a step off at the catch.",
            "In phase all the way, hip to hip at the top of the route.",
            "Mirrors every move and is sitting on the route before the receiver breaks.",
        ],
        ZoneCoverage => [
            "Loses his zone looking at the quarterback and a crosser runs free behind him.",
            "Drops to a spot and waits; the throw is in before he reacts.",
            "Gets to his landmark and drives on the ball once it's thrown.",
            "Reads the quarterback's eyes and breaks as the ball comes out.",
            "Sinks under the route and closes before the ball arrives.",
        ],
        PlayRecognition => [
            "Bites on the play-action and is out of the play.",
            "His eyes go to the backfield at the top of the route. That's why he's late.",
            "Reads it, a beat late.",
            "Keys his man and is moving before the play declares.",
            "Diagnoses it at the snap and beats the blocker to the spot.",
        ],
        Speed => [
            "Gets run down from behind.",
            "Can't open up the distance he needs.",
            "Enough speed for the job.",
            "Pulls away in the open field.",
            "Nobody on the field closes on him.",
        ],
        Acceleration => [
            "Takes three steps to get going.",
            "Slow off the mark.",
            "Normal burst.",
            "Explodes out of his stance.",
            "Top speed in two steps.",
        ],
        Agility => [
            "Stiff in the hips; he rounds every turn.",
            "Needs a gather step to change direction.",
            "Changes direction without losing much.",
            "Sudden in and out of cuts.",
            "Plants and goes without slowing down at all.",
        ],
        Strength => [
            "Gets moved off his spot.",
            "Gives ground when he's hit square.",
            "Holds his own.",
            "Rocks the man in front of him.",
            "Throws grown men aside.",
        ],
        Tackling => [
            "Ducks his head and slides off the ball carrier.",
            "Arm tackle; the runner falls forward for two more.",
            "Wraps up and gets him down.",
            "Squares up, wraps and drives through him.",
            "Runs through the ball carrier and puts him on his back.",
        ],
        Pursuit => [
            "Jogs when the ball goes away.",
            "Takes a bad angle and never gets back into it.",
            "Gets to the pile.",
            "Takes the right angle and closes from the backside.",
            "Runs it down from across the field.",
        ],
        Motor => [
            "Quits on the play once it goes the other way.",
            "Stops at the whistle's first hint.",
            "Plays to the whistle.",
            "Still chasing 30 yards downfield.",
            "Never takes a snap off; he's in on a tackle from the backside.",
        ],
        Decisions => [
            "Forces it into double coverage.",
            "Holds the ball, then throws late into a closing window.",
            "Takes what's there.",
            "Gets to his second read and hits the open man.",
            "Knows where he's going before the snap and puts it there.",
        ],
        Anticipation => [
            "Waits until the receiver is open, and by then he isn't.",
            "Throws to open, never to where he'll be.",
            "On time.",
            "Lets it go before the break.",
            "Throws his man open before he turns his head.",
        ],
        ShortAccuracy | MediumAccuracy => [
            "Sails it over the receiver's head.",
            "Behind the receiver; he has to stop and reach back.",
            "Catchable.",
            "Hits him in stride.",
            "Right on the numbers, away from the defender.",
        ],
        DeepAccuracy => [
            "Underthrows it by ten yards.",
            "Out of bounds; the receiver had a step.",
            "In the area.",
            "Drops it over the shoulder.",
            "Puts it where only his man can catch it, 50 yards downfield.",
        ],
        ArmStrength => [
            "The ball floats; the safety has all day.",
            "Needs a full wind-up to get it out there.",
            "Enough arm.",
            "Drives it into the window on a line.",
            "Flicks it 50 yards off his back foot.",
        ],
        PocketPresence => [
            "Bails out of a clean pocket.",
            "Feels phantom pressure and drops his eyes.",
            "Steps up when he has to.",
            "Slides away from the rush with his eyes downfield.",
            "Hangs in, takes the hit and delivers.",
        ],
        Footwork => [
            "Feet all over the place; he's off balance at the throw.",
            "Late with his set and has to reach.",
            "Clean enough.",
            "Quick, balanced feet; always in position.",
            "His feet are never wrong.",
        ],
        HandUse => [
            "Hands wide, chest exposed.",
            "Late with his hands and gets jolted.",
            "Gets his hands on him.",
            "Inside hands, and he resets them when he loses them.",
            "Violent, precise hands; the other man never gets into his chest.",
        ],
        PassBlocking => [
            "Beaten clean around the edge.",
            "Gives up the inside move and the pocket collapses.",
            "Holds long enough.",
            "Mirrors the rusher and walls him off.",
            "Erases the man across from him.",
        ],
        RunBlocking => [
            "Whiffs at the point of attack and the play dies.",
            "Gets a piece of him but no movement.",
            "Seals his man.",
            "Drives his man two yards off the ball.",
            "Moves the man off the ball and buries him.",
        ],
        Anchor => [
            "Gets driven into the quarterback's lap.",
            "Gives ground against the bull rush.",
            "Sits down and holds.",
            "Drops his hips and stops the bull rush cold.",
            "Like hitting a wall.",
        ],
        LegDrive => [
            "Stops his feet at contact.",
            "Feet stall at contact.",
            "Keeps his feet moving.",
            "Churns through contact.",
            "Carries tacklers for five more.",
        ],
        FinesseRush => [
            "No plan; he runs into the tackle.",
            "One move, and when it doesn't work he stops.",
            "Wins now and then with his get-off.",
            "Dips and rips under the tackle's hands.",
            "Bends the edge at full speed and is in the backfield before the set.",
        ],
        PowerRush => [
            "Gets stoned at the line.",
            "Bull rush with nothing behind it.",
            "Pushes the pocket a little.",
            "Long arm drives the guard back into the quarterback.",
            "Walks the blocker straight back into the pocket.",
        ],
        BlockShedding => [
            "Stays blocked the whole play.",
            "Gets off the block too late to matter.",
            "Sheds and gets in on the tackle.",
            "Stacks, sheds and makes the play.",
            "Throws the blocker and makes the tackle at the line.",
        ],
        Blitzing => [
            "Telegraphs the blitz and walks into the back.",
            "Comes late and the ball is gone.",
            "Gets there.",
            "Times the snap and shoots the gap.",
            "Untouched; the quarterback never sees him.",
        ],
        Traffic => [
            "Gets lost in the trash.",
            "Picked off by the guard.",
            "Works through it.",
            "Slips blocks and finds the ball.",
            "Navigates the wash like it isn't there.",
        ],
        RouteRunning => [
            "Rounds his cut; the corner sits on it.",
            "Telegraphs the break with his eyes.",
            "Runs it the way it's drawn.",
            "Sells vertical, snaps it off at the break.",
            "Separation at the break nobody else on the field gets.",
        ],
        Release => [
            "Stuck at the line; the jam ruins the timing.",
            "Slow off press, the route is late.",
            "Gets off the line.",
            "A quick swipe and he's clean off the line.",
            "Beats the jam before the corner gets his hands up.",
        ],
        Catching => [
            "Drops it, right in his hands.",
            "Body-catches it and juggles.",
            "Catches it.",
            "Plucks it away from his body.",
            "Snatches it out of the air without breaking stride.",
        ],
        ContestedCatch => [
            "Loses it when the defender gets a hand in.",
            "Gives up the ball in traffic.",
            "Holds on through contact.",
            "High-points it over the corner.",
            "Wins it over two defenders.",
        ],
        BallTracking => [
            "Misjudges it and stops.",
            "Slows down to find it.",
            "Finds it late.",
            "Tracks it over the shoulder in stride.",
            "Adjusts to a bad ball at full speed.",
        ],
        BallCarrierVision => [
            "Runs into the back of his own linemen.",
            "Misses the cutback lane.",
            "Hits the hole that's there.",
            "Presses the hole and cuts back off the linebacker's flow.",
            "Sees the lane before it opens.",
        ],
        Elusiveness => [
            "Goes down to the first man.",
            "Straight-line runner; no make-you-miss.",
            "Makes one man miss now and then.",
            "Jump-cuts the safety in a phone booth.",
            "Leaves three defenders grabbing air.",
        ],
        BallSecurity => [
            "Ball away from his body in traffic.",
            "Loose carry when he's hit.",
            "Two hands in traffic.",
            "High and tight through contact.",
            "Never gives them a chance at it.",
        ],
        Discipline => [
            "Holds when he's beaten.",
            "Grabs when he's late.",
            "Clean.",
            "Never reaches when he's beaten; he recovers instead.",
            "Disciplined to the end of the play.",
        ],
        Composure => [
            "Rushes everything once it gets tight.",
            "Gets jumpy when it matters.",
            "Same player in the fourth quarter.",
            "Calm at the biggest moment.",
            "Ice in his veins.",
        ],
        KickPower => [
            "Barely reaches the uprights from 40.",
            "Short leg; he needs the wind.",
            "Enough leg.",
            "Clears it from 50 with room.",
            "Plenty left from 55.",
        ],
        LongKickAccuracy | ShortKickAccuracy => [
            "Hooks it wide.",
            "Starts outside the post and fades back in, barely.",
            "Through.",
            "Right down the middle.",
            "Splits it every time.",
        ],
        PuntPower => [
            "Shanks it 30 yards.",
            "Low line drive; the returner has room.",
            "A decent punt.",
            "High and long, plenty of hang.",
            "A boomer; the coverage is waiting for it.",
        ],
        PuntPlacement => [
            "Into the end zone for a touchback.",
            "Middle of the field, returnable.",
            "Toward the sideline.",
            "Pinned inside the 10.",
            "Out at the 3.",
        ],
        _ => [
            "Poor on this one.",
            "Below the level on this one.",
            "Ordinary on this one.",
            "Good on this one.",
            "Outstanding on this one.",
        ],
    };
    lines[b]
}

/// A half-seen moment's prompt (SK49): what caught your eye without your
/// seeing it clearly.
pub fn hint_of(a: Attr) -> &'static str {
    use Attr::*;
    match a {
        Press | Footwork | Agility | Release => "Something about his feet on that one…",
        PlayRecognition | Decisions | Anticipation | ZoneCoverage => "Something about his eyes on that one…",
        HandUse | PassBlocking | RunBlocking | Anchor | BlockShedding | FinesseRush | PowerRush => {
            "Something about his hands at contact…"
        }
        Motor | Pursuit => "Something about him after the ball went away…",
        Tackling => "Something about the way he came to balance…",
        RouteRunning | ManCoverage => "Something about the break on that one…",
        _ => "Something about that snap…",
    }
}

/// The attribute's words for a sheet or a hint ("press").
pub fn short(a: Attr) -> String {
    a.label().to_lowercase()
}

/// After the whistle (SK51): what he does in the seconds after the play,
/// from who he is. `went_well` is how the snap went for him.
pub fn after(p: &crate::player::Player, went_well: bool, salt: f64) -> Option<&'static str> {
    let pe = &p.personality;
    let t = &pe.traits;
    if salt < 0.45 {
        return None;
    }
    let lead = t.leadership as f64;
    let temper = t.temper as f64;
    let ego = t.ego as f64;
    Some(if went_well {
        if ego >= 15.0 && salt > 0.8 {
            "Stands over the man he beat and lets him hear it."
        } else if lead >= 14.0 {
            "Jogs back and points out the next call to the others."
        } else if pe.competitiveness >= 14.0 {
            "Claps it off and is first back to the huddle."
        } else {
            "Jogs back to the huddle."
        }
    } else if temper >= 15.0 {
        "Slaps the turf and has words with the official."
    } else if pe.love_of_game <= 7.0 || pe.work_ethic <= 7.0 {
        "Walks back slowly, head down."
    } else if lead >= 14.0 {
        "Gathers the others and takes the blame himself."
    } else {
        "Shakes it off and gets set for the next one."
    })
}

/// A teammate helped up, the opponent too (good sportsmanship reads on
/// tape from professionalism and temper).
pub fn helps_up(p: &crate::player::Player) -> bool {
    let t = &p.personality.traits;
    t.professionalism >= 13 && t.temper <= 12
}
