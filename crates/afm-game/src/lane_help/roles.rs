//! What each rating does at each position (Cain, 2026-11-14): the hover on a
//! skill's name says what it does for the position of the player being
//! viewed and, in a short second line, what it does elsewhere.
//!
//! Every line here traces to what the match engine reads
//! (`docs/reference/how-it-works/match-engine.md`, the file and function in
//! each section). Where the engine does not read a skill for a position the
//! skill still counts toward his rating, and `UNREAD` says so: the hover
//! tells the player plainly instead of inventing a use.
//!
//! A position with no role listed for a skill keeps the skill's general
//! words (`words` in `lane_help.rs`); a skill that is no part of the
//! position's job says so.

use afm_engine::attributes::Attr;
use afm_engine::positions::Position;

/// One use of a skill: the positions it applies to, what it does there, and
/// the few words for the "elsewhere" line on another position's hover.
pub(super) struct Role {
    pub at: &'static [Position],
    pub line: &'static str,
    pub short: &'static str,
}

const fn r(at: &'static [Position], line: &'static str, short: &'static str) -> Role {
    Role { at, line, short }
}

use Position::*;

const QBS: &[Position] = &[QB];
const RBS: &[Position] = &[RB];
const RECS: &[Position] = &[WR, TE];
const OL: &[Position] = &[OT, IOL];
const RUSH: &[Position] = &[EDGE, DT];
const BACKERS: &[Position] = &[LB];
const DBS: &[Position] = &[CB, S];
const SEVEN: &[Position] = &[LB, CB, S];
const CARRY: &[Position] = &[QB, RB, WR, TE];
const HANDS: &[Position] = &[RB, WR, TE];

/// The skills the engine never reads for a position: they still count toward
/// his rating and show on scouting reads, but nothing in a game uses them
/// there yet (`docs/reference/how-it-works/ratings.md`, the in-play lists).
pub(super) const UNREAD: &[(Attr, &[Position])] = &[
    (Attr::LegDrive, &[OT, IOL, EDGE, DT]),
    (Attr::BallTracking, &[CB, S]),
    (Attr::Decisions, &[LB, S]),
    (Attr::Motor, &[OT, IOL]),
    (Attr::PlayRecognition, &[OT, IOL]),
    (Attr::Communication, &[OT, CB]),
    (Attr::Anticipation, &[WR, TE]),
    (Attr::Physicality, &[RB]),
    (Attr::KickTrajectory, &[P]),
    (Attr::BallHandling, &[P]),
    (Attr::KickPower, &[P]),
    (Attr::ShortKickAccuracy, &[P]),
    (Attr::ShotgunSnap, &[LS]),
    (Attr::FootballIq, &[LS]),
    (Attr::Concentration, &[K, LS]),
];

pub(super) fn unread(a: Attr, p: Position) -> bool {
    UNREAD.iter().any(|(x, ps)| *x == a && ps.contains(&p))
}

/// The roles of one skill, in the order the second line lists them.
pub(super) fn roles(a: Attr) -> Vec<Role> {
    use Attr::*;
    match a {
        Speed => vec![
            r(
                QBS,
                "Buys time and runs: his clock gets 0.2 seconds longer for every 10 points over the level's typical passer, and he takes off more often when nobody is open.",
                "buys time and scrambles",
            ),
            r(
                HANDS,
                "How fast he gets down the field and away from pursuit: routes run at about 90% of top speed, a carrier at 95%. Past 9 yards deep a fast man also stretches the defense. With his weight it sets his forty.",
                "gets down the field, away from pursuit",
            ),
            r(
                OL,
                "Only how fast he gets to the second level on climbs and pulls; the block itself is decided by other skills.",
                "climbs and pulls",
            ),
            r(
                RUSH,
                "Runs the play down from the back side and chases a scrambler. A speed rush reads a fifth of it; power rushes and the get-off read other skills.",
                "chases from behind",
            ),
            r(
                BACKERS,
                "Range: how much field he covers to the ball, on runs to the sideline and on drops. With his weight it sets his forty.",
                "range to the ball",
            ),
            r(
                DBS,
                "Runs with receivers and covers the deep part of the field; it is what lets a defender recover when a receiver gets a step.",
                "runs with receivers",
            ),
            r(&[K, P, LS], "Only on coverage: getting down the field to the returner.", "kick coverage"),
        ],
        Acceleration => vec![
            r(QBS, "Burst out of the pocket and on a scramble.", "burst on a scramble"),
            r(
                HANDS,
                "Burst off the snap, out of a cut and through a gap; the release off the line and the break at the top of a route. It sets his 10-yard split.",
                "off the snap and out of cuts",
            ),
            r(
                OL,
                "The get-off at the snap: the run start reads 0.3 of it against the defender's get-off, and block power a tenth of it on every block.",
                "the get-off at the snap",
            ),
            r(
                RUSH,
                "The get-off: 0.6 of his first step is burst, the rest is Anticipation on a run and his rush skills on a pass rush. A heavy tackle gives some of it back to his weight.",
                "the first step",
            ),
            r(
                SEVEN,
                "Closing burst: how fast he breaks on a throw, fills a gap or gets home on a blitz.",
                "closing burst",
            ),
        ],
        Agility => vec![
            r(QBS, "Escaping the rush: a third of his sack-escape roll.", "escaping the rush"),
            r(
                RBS,
                "How sharply he cuts at speed: his cut sets how little speed he loses in a turn. The dodge itself is Elusiveness.",
                "sharp cuts at speed",
            ),
            r(
                RECS,
                "Breaks at the top of routes (the cut sets how much speed he loses) and a quarter of beating the jam at the line.",
                "breaks and the release",
            ),
            r(
                OL,
                "A third of his pass set against speed and finesse, and half of blocking in space. A quick-footed blocker takes longer to be beaten again.",
                "the pass set, blocks in space",
            ),
            r(
                RUSH,
                "A quarter of his finesse rush: spins, swims and the change of direction that beats a set.",
                "spins and swims",
            ),
            r(
                BACKERS,
                "Mirrors backs and tight ends in man coverage and breaks down in the open field to make the tackle (half of that break-down).",
                "mirrors, breaks down to tackle",
            ),
            r(
                DBS,
                "Mirrors receivers: how well his hips match the route, half of the break-down that makes a tackle, and a fifth of the jam at the line.",
                "mirrors receivers",
            ),
        ],
        Strength => vec![
            r(QBS, "Only through a sack: it is a third of his escape roll.", "escaping a sack"),
            r(
                RBS,
                "Breaking tackles and moving the pile: half of his broken-tackle score, and a part of his resistance once he is wrapped up.",
                "breaks tackles, moves the pile",
            ),
            r(
                RECS,
                "Beating the jam (a fifth) and a quarter of the fight at the catch point. A tight end's block power reads a fifth of it.",
                "the jam, the catch point, blocks",
            ),
            r(
                OL,
                "A fifth of block power and of the run set, and a fifth of his anchor against power. With his mass it decides who wins the hand fight.",
                "blocking and anchoring",
            ),
            r(
                RUSH,
                "A quarter of a power rush, paid in full with mass (an edge up to 20 lb over the typical edge). It also helps shed blocks and hold a tackle.",
                "power rush, shedding",
            ),
            r(
                SEVEN,
                "Taking on blockers and finishing tackles; a corner's jam at the line reads a quarter of it, and a safety's hit at the catch point a fifth.",
                "blockers, tackles, the jam",
            ),
        ],
        Balance => vec![
            r(
                RBS,
                "Staying up through arm tackles: he runs through more of them and keeps more speed, and it adds to his resistance in the wrap-up.",
                "runs through arm tackles",
            ),
            r(
                RECS,
                "Staying up after the catch, the way a back runs through arm tackles. A tight end's block power reads a tenth of it.",
                "stays up after the catch",
            ),
            r(OL, "A tenth of block power and a fifth of his anchor against a bull rush.", "holding the block"),
            r(
                RUSH,
                "A tenth of a power rush and a fifth of his anchor against double teams; a balanced defender is also pancaked less on run blocks.",
                "power rush, anchor",
            ),
            r(
                SEVEN,
                "After a missed tackle he loses less of his speed and stays in the play; a balanced defender is pancaked less on run blocks.",
                "recovers from a miss",
            ),
        ],
        Flexibility => vec![
            r(
                RUSH,
                "Bend: the speed rush reads 0.15 of it, and an edge who bends the corner flattens to the quarterback, so more of his pressures become sacks. A tall edge's bend counts for more.",
                "bending the corner",
            ),
            r(OL, "Sinking his hips: 0.15 of his pass set against a speed rush.", "sinking his hips"),
            r(
                SEVEN,
                "Turns his hips with the man he covers: part of the lag behind a route reads his hips.",
                "turns his hips with the receiver",
            ),
        ],
        Jumping => vec![
            r(
                RECS,
                "Winning at the high point: his vertical, his catch radius and a quarter of the fight on a jump ball.",
                "high-pointing the ball",
            ),
            r(
                DBS,
                "Getting a hand on the ball at the catch point: his reach there and part of the breakup score.",
                "breakups at the catch point",
            ),
            r(
                RUSH,
                "Bats passes at the line (his reach counts 0.015 a point) and adds to the rush on field goal blocks.",
                "bats passes",
            ),
        ],
        Stamina => vec![
            r(
                RBS,
                "Staying fresh carry after carry: a back comes out below 60% energy if a fresher one is close.",
                "stays fresh carry after carry",
            ),
            r(
                RECS,
                "Keeping his legs late in drives: a receiver or tight end comes out below 74% energy.",
                "late-drive legs",
            ),
            r(
                OL,
                "Run blocks cost 0.9 of work each and pass blocks half a unit a second; a tired lineman loses strength and burst.",
                "tires on blocks",
            ),
            r(
                RUSH,
                "A rush is the heaviest work on the field: a tackle comes out below 84% energy and an edge below 90%, so stamina decides how many snaps he plays.",
                "decides his snaps",
            ),
            r(
                SEVEN,
                "Holds his speed late in drives and games: he comes out below 68% energy if a fresher man is close.",
                "holds his speed late",
            ),
        ],
        ArmStrength => vec![r(
            QBS,
            "How fast the ball gets there (16.5 yards a second plus 0.13 a point): a spot past 25 plus 0.45 × his arm in yards is never open. He also trusts deep throws more with an arm over the level's typical.",
            "ball speed and range",
        )],
        ShortAccuracy => vec![r(
            QBS,
            "Misses on throws up to 5 air yards, blending into medium by 15. Quick game, screens and checkdowns.",
            "throws up to 5 air yards",
        )],
        MediumAccuracy => vec![r(
            QBS,
            "Misses on throws around 15 air yards, blending in from short and out to deep by 25. Digs, curls and outs.",
            "throws around 15 air yards",
        )],
        DeepAccuracy => vec![r(
            QBS,
            "Misses on throws from 25 air yards. A step of separation downfield becomes a completion or an overthrow.",
            "throws past 25 yards",
        )],
        Touch => vec![r(
            QBS,
            "On a medium throw over a defender it takes up to a third of the miss; a hard, fast ball is caught less cleanly when his touch is low.",
            "throws over a defender",
        )],
        ThrowOnMove => vec![r(
            QBS,
            "Cuts the miss when he throws moving faster than 3 yards a second: bootlegs, rollouts and scrambles.",
            "throws on the run",
        )],
        QuickRelease => vec![r(
            QBS,
            "Three quarters of his release time (with footwork), which is how long a rusher has to reach him; it also cuts passes batted at the line.",
            "release time",
        )],
        Footwork => vec![r(QBS, "How long his drop takes, and a quarter of his release time.", "drop and release")],
        PocketPresence => vec![r(
            QBS,
            "Feels rushers he can't see, sees pressure sooner, adds to his clock and to his sack escape.",
            "feeling the rush",
        )],
        BallHandling => vec![
            r(
                QBS,
                "Sells play-action and jet fakes (the defense reads a sold fake later) and clean exchanges: 35% of the risk of a bad handoff.",
                "fakes and exchanges",
            ),
            r(&[P], "Not read for a punter; the hold uses Catching and Concentration.", "not read"),
        ],
        EyeManipulation => vec![r(
            QBS,
            "Looking a deep defender off on his first read: the safety is pulled away from the throw he wants.",
            "looking safeties off",
        )],
        BallSecurity => vec![
            r(QBS, "Strip sacks: the chance a sack knocks the ball loose falls as it rises.", "strip sacks"),
            r(RBS, "Fumbles when hit, and clean handoffs: 65% of the exchange risk is his.", "fumbles and exchanges"),
            r(RECS, "Fumbles when hit after the catch.", "fumbles after the catch"),
        ],
        BallCarrierVision => vec![
            r(
                RBS,
                "Reads the hole and the cutback: the noise in his read falls with it, and he sees threats from farther away (7 yards plus 0.12 a point).",
                "reads the hole and the cutback",
            ),
            r(
                &[QB, WR, TE],
                "With the ball he picks his path the way a back does: how far he sees and how well he reads the tacklers.",
                "picks a path with the ball",
            ),
        ],
        Elusiveness => vec![
            r(
                RBS,
                "Making the first tackler miss (0.8 of the check, with speed) and breaking out of a tackle (half of the broken-tackle score).",
                "makes tacklers miss",
            ),
            r(RECS, "The same after the catch: the first miss and breaking out of tackles.", "after the catch"),
            r(QBS, "Escaping a sack (40% of the roll) and the moves on a scramble.", "escaping sacks"),
        ],
        LegDrive => vec![
            r(
                CARRY,
                "Yards after contact: once a tackler has him wrapped up it helps him keep driving, fall forward and move the pile.",
                "yards after contact",
            ),
            r(OL, "In real football the push behind a drive block.", "the push behind a drive block"),
            r(RUSH, "In real football the legs behind a bull rush.", "the legs behind a bull rush"),
        ],
        Catching => vec![
            r(
                HANDS,
                "Hands: the base chance of a miss is 15%, falling 0.095 points for each point of rating down to 1.2%; every drop on a catchable ball comes from it.",
                "hands",
            ),
            r(SEVEN, "His hands on a pass in reach: tips and off-target balls become picks more often.", "picks"),
            r(
                &[P],
                "On kicks the holder's hands (with Concentration, half each) set how often a hold goes bad.",
                "holding on kicks",
            ),
        ],
        ContestedCatch => vec![
            r(
                RECS,
                "Winning at the catch point with a defender on him: half of the fight there. The passer also trusts him more on throws into traffic.",
                "50-50 balls",
            ),
            r(DBS, "A share of the breakup score at the catch point.", "breakups at the catch point"),
        ],
        BallTracking => vec![
            r(RECS, "Reacts to the throw sooner and closes faster on long balls.", "tracks the ball"),
            r(&[RB], "Fielding punts and kicks when he returns them: most of the muff chance.", "fielding returns"),
            r(
                DBS,
                "Reading the ball in the air. A defender's catch point is played with Catching and Jumping instead.",
                "reading the ball in the air",
            ),
        ],
        BodyControl => vec![r(
            RECS,
            "Adjusts to a ball away from his frame (it softens the penalty for a ball off his body) and keeps his feet at the sideline.",
            "bad throws and the sideline",
        )],
        RouteRunning => vec![r(
            RECS,
            "How little speed a break costs him, how early he moves to the next point, and how he settles in a zone hole; it also sets the lag a man defender must make up.",
            "separation at the break",
        )],
        Release => vec![
            r(
                RECS,
                "Beating the jam: over half of it, and the footwork before the route starts (a bad release runs late).",
                "beating the jam",
            ),
            r(DBS, "A gunner on punts uses it to get free from the jam.", "gunner on punts"),
        ],
        PassBlocking => vec![
            r(
                OL,
                "Half of his pass set against the rush: he wins or loses his rep, and the quarterback gets his time.",
                "his rep against the rush",
            ),
            r(
                &[RB, TE],
                "Picking up a blitzer (a back's chance rises 0.4% a point) and chipping a rusher.",
                "blitz pickup and chips",
            ),
        ],
        RunBlocking => vec![
            r(
                OL,
                "Makes the hole: 45% of block power in the run game, and half the start against the defender's get-off.",
                "drive and down blocks",
            ),
            r(
                RECS,
                "The same block power as a lineman's, for the blocks he holds on run plays.",
                "his blocks on run plays",
            ),
        ],
        BlockingInSpace => vec![
            r(
                OL,
                "Climbs, pulls and screens: half of it, with Agility, decides whether he hits the man he goes after.",
                "climbs, pulls and screens",
            ),
            r(RECS, "Blocks on returns and screens, and fewer illegal blocks in the back.", "returns and screens"),
        ],
        HandUse => vec![
            r(
                OL,
                "Where his hands land: 15% of block power and a major part of the hand fight against a rusher; a blocker with poor hands holds more.",
                "holding a block",
            ),
            r(
                RUSH,
                "Frees him from the blocker: a tenth to a fifth of every move, and a fifth of shedding.",
                "the hands that free him",
            ),
            r(&[TE, WR], "Keeps a won block without holding.", "keeps blocks clean"),
            r(BACKERS, "Part of shedding a block (a fifth of shed power).", "shedding"),
        ],
        Anchor => vec![
            r(
                OL,
                "Holding his ground against a bull rush: 30% of his pass set against power.",
                "stands up to a bull rush",
            ),
            r(RUSH, "Holding the point against a double team, and 15% of shedding.", "holds against double teams"),
            r(
                BACKERS,
                "Part of shedding a block and standing up to a lead blocker (15% of shed power).",
                "stands up to lead blockers",
            ),
        ],
        PowerRush => vec![r(
            RUSH,
            "40% of the bull rush and long arm, against the blocker's pass set; a bull rush moves the pocket 1.4 times as fast. It also adds to field goal blocks.",
            "bull rushes and long arms",
        )],
        FinesseRush => vec![
            r(
                RUSH,
                "45% of the finesse rush and 40% of the speed rush: swims, rips and the turn of the corner.",
                "swims, rips and spins",
            ),
            r(BACKERS, "Rushing from off the line on blitzes, with the same moves the rushers use.", "blitz moves"),
        ],
        BlockShedding => vec![
            r(
                RUSH,
                "Getting off the block: 40% of shed power on run plays, and a tenth of the power rush.",
                "getting off the block",
            ),
            r(
                SEVEN,
                "Frees him from blockers to make the tackle on runs and screens; a corner works off the receiver's block.",
                "getting off blocks",
            ),
        ],
        ManCoverage => vec![
            r(
                DBS,
                "Staying with his man: the lag behind the route falls with it, and it is read in the breakup at the catch point.",
                "stays with his man",
            ),
            r(
                BACKERS,
                "Covering a back or tight end in man; the Sam takes the tight end.",
                "covers backs and tight ends",
            ),
        ],
        ZoneCoverage => vec![
            r(
                SEVEN,
                "Where he drops and how he plays the area: 60% of his zone read, with Play recognition.",
                "plays the area",
            ),
            r(&[EDGE], "Dropping into the flat on zone blitzes.", "flat on zone blitzes"),
        ],
        Press => vec![
            r(
                &[CB],
                "The jam at the line: over half of the contest against the receiver's release, which can make him late or add lag to the route.",
                "the jam",
            ),
            r(&[S, LB], "Presses only when the call puts him on a man at the line.", "occasional jams"),
        ],
        Disguise => vec![r(
            DBS,
            "Hides the coverage before the snap: the secondary's average against the quarterback's read decides whether he is fooled.",
            "hides the coverage",
        )],
        Tackling => vec![
            r(
                SEVEN,
                "Brings the runner down: 60% of his tackle score against the carrier's evade, and it reaches farther (0.01 yard a point).",
                "makes the tackle",
            ),
            r(
                RUSH,
                "Finishing the sack and bringing the runner down: with Pursuit it sets how many pressures become sacks.",
                "finishing sacks",
            ),
            r(&[K, P, LS], "Tackling on kick coverage.", "kick coverage"),
        ],
        Pursuit => vec![
            r(
                &[LB, CB, S, EDGE, DT],
                "The angle to the ball: half of how well he leads the runner, with a share of the tackle score and the sack finish.",
                "angles to the ball",
            ),
            r(&[K, P, LS], "Running down returners on coverage.", "kick coverage"),
        ],
        Blitzing => vec![
            r(
                BACKERS,
                "Picks which linebacker comes, and how hard he is to pick up: a back's chance of picking him up falls 0.8% a point.",
                "times the blitz, beats the pickup",
            ),
            r(
                &[S, CB],
                "Matters only in how hard the back finds him; which back comes is chosen by size.",
                "rare, beats the pickup",
            ),
        ],
        Traffic => vec![r(
            &[LB, S, CB, EDGE, DT, OT, IOL, WR, TE, RB],
            "Bodies push apart less when his rating is high, so he slips through a crowd. A ball carrier is skipped, so a back never uses it while he runs with the ball.",
            "slips through traffic",
        )],
        KickPower => vec![
            r(&[K], "How far the field goal and the kickoff go.", "field goal and kickoff distance"),
            r(&[P], "Read only when the club has no kicker.", "only without a kicker"),
        ],
        ShortKickAccuracy => {
            vec![r(
                &[K],
                "Accuracy on extra points and field goals up to 30 yards.",
                "extra points and short field goals",
            )]
        }
        LongKickAccuracy => {
            vec![r(&[K], "Accuracy on field goals from 50 yards, blending in from 30.", "long field goals")]
        }
        KickTrajectory => vec![
            r(
                &[K],
                "How quickly the kick gets up: a low kick is blocked more, and it adds hang on kickoffs.",
                "blocks and hang",
            ),
            r(&[P], "Not read for punts; hang comes from Punt power and Punt placement.", "not read"),
        ],
        KickoffTechnique => vec![r(
            &[K],
            "Placing kickoffs: where it lands, how long it hangs and the out-of-bounds risk, and onside kicks.",
            "kickoffs and onside kicks",
        )],
        PuntPower => vec![r(&[P], "Distance: it sets the leg, and with Placement the hang.", "punt distance")],
        PuntPlacement => vec![r(
            &[P],
            "Directional punting: pinning returners near the numbers and inside the 20; it adds hang.",
            "pinning returners",
        )],
        GetOff => {
            vec![r(
                &[K, P],
                "Time from the snap to the kick: a quick get-off means fewer blocks.",
                "fewer blocked kicks",
            )]
        }
        LongSnapping => vec![r(&[LS], "Speed and aim of the snap: bad snaps and the time to the kick.", "long snaps")],
        ShotgunSnap => vec![
            r(
                &[IOL],
                "A bad snap costs 0.3 to 0.6 seconds and the play starts late. Only the man at center reads it; a guard's is not used.",
                "center's snap",
            ),
            r(&[LS], "A long snapper only snaps on kicks; the shotgun snap is a center's.", "not read"),
        ],
        FootballIq => vec![
            r(
                QBS,
                "Hard counts, delay of game and illegal formations read it, and it is half of his sense of when to give up on a play.",
                "counts and checks",
            ),
            r(
                OL,
                "A fifth of his pass set, and fewer busts and fouls: his bust chance rises below 60.",
                "assignments, pickups",
            ),
            r(
                RUSH,
                "Which move he picks (the noise shrinks), the counter, and his assignments.",
                "picks his rush, assignments",
            ),
            r(SEVEN, "His assignments and run fits: fewer busts, which rise below 60.", "assignments and busts"),
            r(
                HANDS,
                "A back's pickup of a blitzer and a receiver's settle in a zone hole read it.",
                "pickups and zone holes",
            ),
        ],
        PlayRecognition => vec![
            r(
                QBS,
                "Reading the defense: half of his time on each read with Decisions, and 30% of how open a man looks.",
                "reading the defense",
            ),
            r(
                SEVEN,
                "Sees the play sooner: his read time falls with it, he breaks on a throw sooner, and it is 40% of his zone read.",
                "sees the play sooner",
            ),
            r(RUSH, "A sooner read on run plays and 15% of shedding blocks.", "sooner run reads"),
        ],
        Anticipation => vec![
            r(QBS, "Lets the ball go on timing: the wait after he decides falls with it.", "throwing on timing"),
            r(RUSH, "Two fifths of the get-off: reading the snap and the movement.", "reads the snap"),
            r(
                SEVEN,
                "Jumping routes (the chance he guesses right) and a better break on the throw; it keeps him from biting on a look-off.",
                "jumps routes",
            ),
            r(RECS, "Not read yet for receivers.", "not read"),
        ],
        Decisions => vec![
            r(
                QBS,
                "His choices: half of his time on each read, how open he thinks a man is, when to throw, check down or throw it away.",
                "reads and throws",
            ),
            r(
                &[RB, WR, CB],
                "As a returner: whether he fields a punt or fair catches, and when to kneel.",
                "returners' fair-catch choices",
            ),
            r(&[LB, S], "Not read for a defender yet; it only counts toward his rating.", "not read"),
        ],
        Composure => vec![
            r(
                QBS,
                "Holds up under pressure: the miss grows less with a rusher on him, he waits longer in a collapsing pocket, and a bad play costs him less momentum.",
                "throwing under pressure",
            ),
            r(
                &[K, P],
                "Pressure on a kick: the miss grows when a kick decides the game, and punt shanks.",
                "kicks that decide the game",
            ),
            r(
                &[RB, WR, TE, LB, CB, S, EDGE, DT, OT, IOL, LS],
                "Softens momentum swings: a bad play costs him less over the next snaps.",
                "momentum",
            ),
        ],
        Concentration => vec![
            r(
                &[WR, TE, OT, IOL, RB],
                "The silent count and the crowd: fewer false starts and late releases on the road, and fewer lapses late in drives.",
                "noise and late-drive lapses",
            ),
            r(
                &[LB, CB, S, EDGE, DT],
                "Fewer lapses late in drives, when a tired, unfocused player busts more.",
                "late-drive lapses",
            ),
            r(&[P], "Half of the hold on kicks (with Catching).", "the hold"),
        ],
        Discipline => vec![
            r(
                &[OT, IOL, TE, WR, RB],
                "Fewer holding calls, false starts and illegal blocks.",
                "holds and false starts",
            ),
            r(
                DBS,
                "Fewer holds and pass interference calls, and he doesn't bite on fakes.",
                "holds and pass interference",
            ),
            r(
                &[LB, EDGE, DT],
                "Fewer offside calls and face masks, and he stays on assignment against fakes.",
                "offside, fakes",
            ),
        ],
        Communication => vec![
            r(
                &[QB],
                "Keeps the offense from busting: his rating sets the call on a bust chance for every offensive player.",
                "keeps the offense together",
            ),
            r(
                &[IOL],
                "The center's line calls set the bust chance across the line; only the man at center reads it, a guard's is not used.",
                "center's line calls",
            ),
            r(
                &[LB, S],
                "The best caller among linebackers and safeties sets the defense's bust chance.",
                "calls for the defense",
            ),
        ],
        Toughness => vec![r(
            &[QB, RB, WR, TE, OT, IOL, EDGE, DT, LB, CB, S, K, P, LS],
            "Only decides whether he plays on through an injury: the chance rises with it and falls with how long the injury will last. It is the same at every position.",
            "plays through injury",
        )],
        Motor => vec![
            r(RUSH, "His counter move after a first block loss.", "counter moves"),
            r(
                SEVEN,
                "Effort to the whistle: a defender chasing from behind keeps his effort up instead of giving up on the play.",
                "chases to the whistle",
            ),
        ],
        Competitiveness => vec![
            r(
                &[RB, WR, TE, QB],
                "Fights for extra yards after contact: a share of his resistance in the wrap-up.",
                "yards after contact",
            ),
            r(&[LB, CB, S, EDGE, DT], "Wins the fight at the tackle and the loose ball.", "wins close ones"),
        ],
        Physicality => vec![
            r(&[OT, IOL, TE, WR], "Finishing blocks: a pancake chance each tick rises with it.", "finishes blocks"),
            r(
                &[LB, CB, S, EDGE, DT],
                "Finishing tackles: runners break his arm tackles less (a fortieth a point), and a safety's hit at the catch point reads a quarter of it.",
                "the hit",
            ),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every skill has roles, no position is named twice for one skill, and
    /// every line is a sentence short enough for a hover card.
    #[test]
    fn roles_are_clean() {
        for a in Attr::ALL {
            let all = roles(a);
            assert!(!all.is_empty(), "{:?} has no role", a);
            for (i, x) in all.iter().enumerate() {
                assert!(x.line.len() > 12 && x.line.len() <= 260, "{:?}: {}", a, x.line);
                assert!(x.line.ends_with('.'), "{:?}: {}", a, x.line);
                assert!(!x.short.is_empty() && x.short.len() <= 48, "{:?}: {}", a, x.short);
                for y in &all[i + 1..] {
                    for p in x.at {
                        assert!(!y.at.contains(p), "{:?} names {} twice", a, p.name());
                    }
                }
            }
        }
    }

    /// The skills the engine never reads for a position are the ones the
    /// in-play lists already say are read little; a position must be real.
    #[test]
    fn unread_skills_are_known() {
        for (a, ps) in UNREAD {
            assert!(!ps.is_empty(), "{:?}", a);
        }
        assert!(unread(Attr::LegDrive, Position::OT));
        assert!(!unread(Attr::LegDrive, Position::RB));
    }

    /// Cain's case: leg drive reads differently for a back and a lineman.
    #[test]
    fn leg_drive_differs_by_position() {
        let at = |p: Position| roles(Attr::LegDrive).iter().find(|r| r.at.contains(&p)).map(|r| r.line);
        assert_ne!(at(Position::RB), at(Position::OT));
        assert!(at(Position::RB).unwrap().contains("after contact"));
        assert!(at(Position::OT).unwrap().contains("drive block"));
    }
}
