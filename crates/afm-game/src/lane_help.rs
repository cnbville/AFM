//! Lane commands: help on every screen (update 3, lane 6, H1-H7). The game
//! teaches through your own staff and your own players, never with pop-ups:
//! hovers, the depth chart's side panel and lines in your mail.
//!
//! - H1 `help_attr`: what a rating is, what it does on the field and how it
//!   grows (the rating's own growth profile), with where it matters most.
//! - H2 `last_year`: how your staff's read of a rating moved since the last
//!   season it kept (`Career::kept_reads`) and the biggest reasons it can
//!   see. `lane_ratings::card` adds it to the range's hover card.
//! - H3 `help_level`: what a level rating ("76 among FBS players") ranks
//!   against, its five-season scale and how many rated above him.
//! - H4 `why_here`: who put a man on a depth chart spot, why, and who else
//!   could play it (board 37, WhyHerePB; `lane_squadboards::depth`).
//! - H5 `weak_spots`: the lineup choices of yours your staff thinks are
//!   clearly weak, for your coordinator's note before kickoff (mail).
//! - H6 `pick_why`, `mood_dips`: the why-lines on mail and news that touch
//!   your team (recruits' choices, portal entries, morale, your boss).
//! - H7: the football terms that explain themselves are the page's own
//!   glossary (`app/ui/screens/help.js`).
//!
//! Every read here is your staff's: ranges, words and the public record.
//! Nothing shows a true rating or a potential.

use afm_engine::attributes::{Attr, AttributeGroup};
use afm_engine::calendar::Date;
use afm_engine::level_rating::{LevelGroup, level_group};
use afm_engine::player::{DevelopmentEvent as E, PlayingTime};
use afm_engine::positions::Position;
use afm_engine::world::program::GradeCat;
use afm_engine::world::staff_read::{self as sr, Job};
use afm_engine::world::{Stage, World};
use serde_json::{Value, json};

use crate::lane_squadboards::{cap, families, first_last, health, key_attrs, mid};
use crate::reads::{self, Read};
use crate::views::{self, TeamRef};
use crate::{Game, Reply};

const COMMANDS: [&str; 3] = ["help_attr", "help_attrs", "help_level"];

pub fn call(game: &mut Game, cmd: &str, args: &Value) -> Option<Reply> {
    if !COMMANDS.contains(&cmd) {
        return None;
    }
    Some(match cmd {
        "help_attr" => (|| {
            let k = args.get("k").and_then(Value::as_str).ok_or("Missing k.")?;
            let a = Attr::ALL.iter().copied().find(|a| a.key() == k).ok_or("No such rating.")?;
            Ok(attr_help(a))
        })(),
        // The labels the page looks for in its text, with their keys.
        "help_attrs" => {
            Ok(json!(Attr::ALL.iter().map(|a| json!({ "key": a.key(), "label": a.label() })).collect::<Vec<_>>()))
        }
        "help_level" => (|| {
            let w = game.world.as_ref().ok_or("No world is loaded.")?;
            let id = args.get("p").and_then(Value::as_u64).ok_or("Missing p.")? as u32;
            level_help(w, id)
        })(),
        _ => Err(format!("Unknown command: {cmd}")),
    })
}

// ---------- H1: what a rating is ----------

/// What a rating is, and what it does on the field (`docs/design/attributes.md`,
/// "What each attribute drives on the field").
fn words(a: Attr) -> (&'static str, &'static str) {
    use Attr::*;
    match a {
        Speed => (
            "Top speed for his size: how fast he runs once he is going.",
            "Wins the race to the edge and down the field, closes on the ball and outruns pursuit. With his weight it sets his forty.",
        ),
        Acceleration => (
            "Burst: how quickly he reaches his top speed.",
            "The first steps off the snap, out of a cut and through a gap. It sets his 10-yard split.",
        ),
        Agility => (
            "Change of direction: how sharply he cuts without slowing.",
            "Cuts in the open field, breaks at the top of routes and mirrors a receiver in man coverage. It shows in the shuttle and the 3-cone.",
        ),
        Strength => (
            "Raw strength, as the weight room measures it.",
            "With balance, bend, hand use and his mass it decides how strong he plays: holding a block, shedding one, finishing a tackle, breaking an arm tackle. It shows in the bench press.",
        ),
        Balance => (
            "Staying on his feet through contact.",
            "Keeps a runner up through arm tackles and a lineman square in his stance; fewer trips and falls.",
        ),
        Flexibility => (
            "Bend: how low and loose he can get.",
            "A rusher bending the edge, a corner turning to run with a receiver, a lineman sinking his hips.",
        ),
        Jumping => (
            "How high and far he jumps.",
            "High-points the ball, wins jump balls and gets a hand on passes at the line. It shows in the vertical and the broad jump.",
        ),
        Stamina => (
            "How long he keeps his level through a drive and a game.",
            "A tired player is slower and weaker late in long drives and late in games. Low stamina needs a rotation.",
        ),
        ArmStrength => (
            "How far and how hard he can throw.",
            "Range on deep balls, throws into tight windows and the out route from the far hash.",
        ),
        ShortAccuracy => (
            "Placement on throws under 10 yards in the air.",
            "Quick game, screens and checkdowns in stride, so the catch comes with yards after it.",
        ),
        MediumAccuracy => (
            "Placement on throws of 10 to 19 yards in the air.",
            "Digs, curls and outs against zone: the throws that move the chains.",
        ),
        DeepAccuracy => (
            "Placement on throws of 20 yards and more in the air.",
            "Turns a step of separation downfield into a completion instead of an overthrow.",
        ),
        Touch => (
            "Putting air under the ball when the throw needs it.",
            "Drops the ball over a linebacker and in front of a safety on 10 to 25 yard throws, and takes speed off short ones.",
        ),
        ThrowOnMove => {
            ("Accuracy when he throws on the run.", "Bootlegs, rollouts and scrambles that keep a play alive.")
        }
        QuickRelease => (
            "How fast the ball comes out once he decides.",
            "Beats the rush on quick throws and gets fewer passes batted at the line.",
        ),
        Footwork => (
            "His drop and his setup in the pocket.",
            "The time it takes to set his feet. Clean feet speed his release and steady his accuracy.",
        ),
        PocketPresence => (
            "Feeling the rush and moving inside the pocket.",
            "Steps up or slides away from pressure with his eyes still downfield; fewer sacks.",
        ),
        BallHandling => (
            "Fakes and handoffs.",
            "Play-action and jet fakes that hold linebackers and safeties, and clean exchanges in the backfield.",
        ),
        EyeManipulation => (
            "Looking defenders off.",
            "Moves a deep safety with his eyes on the first read to open the throw he wants.",
        ),
        BallSecurity => {
            ("Protecting the ball.", "Fewer fumbles, above all in traffic and when he is hit from the side.")
        }
        BallCarrierVision => (
            "Seeing the hole and the field with the ball in his hands.",
            "Finds the cutback lane and the open grass; how much of the field he takes in.",
        ),
        Elusiveness => ("Making tacklers miss.", "Jukes, spins and side-steps in space: missed tackles forced."),
        LegDrive => {
            ("Pad level and drive through contact.", "Yards after contact and falling forward on short yardage.")
        }
        Catching => ("Hands: catching the ball cleanly.", "Fewer drops on catchable balls."),
        ContestedCatch => (
            "Catching through contact.",
            "Wins 50-50 balls with a defender on him; for a defender, the breakups at the catch point.",
        ),
        BallTracking => {
            ("Reading the ball in the air.", "Adjusts to deep balls over his shoulder; a returner muffs fewer punts.")
        }
        BodyControl => (
            "Adjusting his body to the ball.",
            "Toe-taps on the sideline and catches off his frame and on bad throws: a bigger catch radius.",
        ),
        RouteRunning => {
            ("Running crisp, timed routes.", "Separation at the break, and settling in the hole of a zone.")
        }
        Release => (
            "Getting off the line against press.",
            "Beats the jam so the route stays on time; a gunner gets free on punts.",
        ),
        PassBlocking => (
            "Protecting the passer one on one.",
            "Wins his rep against the rush and buys the quarterback time to throw.",
        ),
        RunBlocking => {
            ("Moving people in the run game.", "Makes the hole: drive blocks, down blocks and double teams.")
        }
        BlockingInSpace => (
            "Blocking on the move.",
            "Climbs to linebackers, leads on screens and walls off on returns, without blocks in the back.",
        ),
        HandUse => (
            "Where his hands land and how he grips.",
            "Keeps a won block without holding; for a rusher, the hands that free him.",
        ),
        Anchor => ("Holding his ground against power.", "Stands up to the bull rush and to double teams."),
        PowerRush => ("Rushing through the blocker.", "Bull rushes and long arms that collapse the pocket."),
        FinesseRush => ("Rushing around the blocker.", "Speed rushes, swims, rips and spins that win the edge."),
        BlockShedding => ("Getting off a block.", "Frees him to make the tackle on runs and screens."),
        ManCoverage => (
            "Covering a receiver one on one.",
            "Mirrors routes and stays in phase; with agility and bend, how much he lags at the break.",
        ),
        ZoneCoverage => (
            "Covering an area of the field.",
            "Reads the quarterback and the routes around him and, with play recognition, breaks on the throw.",
        ),
        Press => {
            ("Jamming a receiver at the line.", "Disrupts the release so the route runs late; holds the vise on punts.")
        }
        Disguise => (
            "Hiding the coverage before the snap.",
            "Keeps the quarterback from reading the shell, and motion from giving it away.",
        ),
        Tackling => ("Bringing the ball carrier down.", "Form and wrap range: fewer missed and broken tackles."),
        Pursuit => (
            "Taking good angles to the ball.",
            "Runs plays down from the back side and keeps a gain from becoming a big one.",
        ),
        Blitzing => ("Rushing from off the line.", "Times the blitz and beats the back's pickup."),
        Traffic => (
            "Moving through bodies.",
            "How much a crowd slows him: a linebacker through the trash, a back through the line.",
        ),
        KickPower => ("Leg strength on place kicks.", "Field goal range and kickoff distance."),
        ShortKickAccuracy => ("Accuracy on extra points and short field goals.", "Makes from inside about 40 yards."),
        LongKickAccuracy => ("Accuracy on long field goals.", "Makes from 40 yards and beyond."),
        KickTrajectory => {
            ("How quickly his kick gets up.", "A low kick gets blocked; for a punter, the flight that gives hang time.")
        }
        KickoffTechnique => ("Placing kickoffs.", "Hang time and landing spot on kickoffs, and onside kicks."),
        PuntPower => ("Leg strength on punts.", "Punt distance; the hang comes from the flight."),
        PuntPlacement => ("Directional punting.", "Pins returners at the numbers and inside the 20."),
        GetOff => ("Time from the snap to the kick.", "A quick get-off means fewer blocked kicks and punts."),
        LongSnapping => ("Snaps on punts and kicks.", "The speed and aim of the long snap; fewer bad snaps."),
        ShotgunSnap => ("A center's shotgun snap.", "Fewer bad snaps in the shotgun, so the play starts on time."),
        FootballIq => (
            "Knowing the game and his job in it.",
            "Assignments, blitz pickup and alignment: fewer busts and pre-snap fouls.",
        ),
        PlayRecognition => (
            "Reading a play as it unfolds.",
            "Sees run or pass sooner and reads motion; with zone coverage, breaks on the ball.",
        ),
        Anticipation => (
            "Acting before the play shows itself.",
            "Jumps routes, reads the quarterback's eyes, bats balls and gets off on the snap.",
        ),
        Decisions => (
            "His choices with the ball.",
            "Fewer forced throws and bad risks: throws it away instead of taking a sack or an interception.",
        ),
        Composure => ("Calm under pressure.", "Holds up in a collapsing pocket, on big kicks and in big moments."),
        Concentration => {
            ("Focus through noise and fatigue.", "Fewer false starts in loud stadiums and fewer lapses late in drives.")
        }
        Discipline => ("Doing his job and nothing else.", "Fewer penalties; he doesn't bite on fakes."),
        Communication => (
            "Getting everyone on the same page.",
            "The quarterback, the center and the defense's signal caller keep their unit from busting.",
        ),
        Toughness => ("Playing through pain and contact.", "Keeps playing hurt and doesn't shy from collisions."),
        Motor => (
            "Effort to the whistle.",
            "Chases plays down and doesn't let go of a block; with football IQ, a rusher's counter move.",
        ),
        Competitiveness => ("Wanting it more.", "Wins the piles on short yardage and the close battles."),
        Physicality => ("Playing with violence.", "Finishes blocks, puts men on the ground and commits to the hit."),
    }
}

/// How a rating grows, from its growth profile, in plain words.
fn grows(a: Attr) -> String {
    let pr = a.profile();
    let (lo, hi) = pr.growth_window;
    let how = match pr.group {
        AttributeGroup::Physical => "It grows with his body and the weight room",
        AttributeGroup::Technical => "It grows with coaching and reps",
        AttributeGroup::Mental => "It grows with games, film and time",
    };
    let coach = if pr.coachability >= 0.6 {
        "good teaching moves it a lot"
    } else if pr.coachability >= 0.35 {
        "teaching moves it some"
    } else {
        "teaching moves it little: much of it is born"
    };
    let snaps = if pr.experience >= 0.4 { " Game snaps teach it more than practice does." } else { "" };
    let injury = if pr.injury_sensitive { " A major injury can take some of it for good." } else { "" };
    format!(
        "{how}, fastest from about {lo:.0} to {hi:.0}; {coach}.{snaps} It starts to fade around {:.0}.{injury}",
        pr.decline_age
    )
}

/// The positions where a rating weighs most in the overall, best first.
fn matters(a: Attr) -> Vec<String> {
    let mut v: Vec<(f64, Position)> =
        Position::ALL.iter().map(|&p| (afm_engine::positions::weight(p, a), p)).filter(|x| x.0 >= 0.03).collect();
    v.sort_by(|x, y| y.0.total_cmp(&x.0));
    v.into_iter().take(4).map(|(_, p)| crate::lane_ratings::pos_words(p).1.to_string()).collect()
}

fn attr_help(a: Attr) -> Value {
    let (what, field) = words(a);
    let m = matters(a);
    json!({
        "key": a.key(),
        "label": a.label(),
        "what": what,
        "field": field,
        "grows": grows(a),
        "matters": if m.is_empty() { "Little in anyone's overall: it shows in the moments it decides.".to_string() } else { format!("Weighs most for {}.", join_and(&m)) },
    })
}

// ---------- H2: how his read moved since last season ----------

/// How your staff's read of rating `a` moved since the last season it kept,
/// with the reasons it can see. `now` is the range shown today.
pub(crate) fn last_year(w: &World, i: usize, a: Attr, now: (u8, u8)) -> Option<Value> {
    let p = &w.people[i];
    let c = w.career.as_ref()?;
    let id = p.player.id;
    let last = c.kept_of(id).last()?;
    let then_pos = Position::ALL.get(last.pos as usize).copied().unwrap_or(p.player.position);
    let was_key = last.keys.iter().find(|k| k[0] as usize == a.index() && k[1] > 0).map(|k| (k[1] as f64, k[2] as f64));
    let fams = families(then_pos);
    let was_fam =
        fams.iter().position(|f| f.1.contains(&a)).filter(|&k| last.fams[k] > 0).map(|k| (fams[k].0, last.fams[k]));
    let season = last.season as i32;
    let nm = (now.0 as f64 + now.1 as f64) / 2.0;
    let (was, line, delta) = if let Some((lo, hi)) = was_key {
        let d = nm - (lo + hi) / 2.0;
        (
            Some([lo as u8, hi as u8]),
            format!("At the end of the {season} season your staff read it {lo:.0}-{hi:.0}; today {}-{}.", now.0, now.1),
            d,
        )
    } else if let Some((name, m)) = was_fam {
        // Only the family's middle was kept: the move is the family's.
        let fam_now = fams.iter().find(|f| f.0 == name).and_then(|f| {
            let reads = reads::attributes(&p.player, own(w, i), eye(w, i), views::reader_key(w));
            crate::lane_squadboards::family_read(&reads, f).map(|(lo, hi)| (lo + hi) / 2.0)
        });
        let d = fam_now.map_or(0.0, |x| x - m as f64);
        (
            None,
            format!(
                "Your staff kept his {} work at {m} at the end of the {season} season; it reads {:.0} today.",
                name.to_lowercase(),
                fam_now.unwrap_or(m as f64)
            ),
            d,
        )
    } else {
        return Some(json!({
            "word": "No read kept",
            "line": format!("Your staff kept no read of his {} at the end of the {season} season.", a.label().to_lowercase()),
            "why": reasons(w, i, a, last.wt),
            "season": season,
        }));
    };
    let word = if delta >= 3.0 {
        "Up clearly"
    } else if delta >= 1.5 {
        "Up a little"
    } else if delta > -1.5 {
        "About the same"
    } else if delta > -3.0 {
        "Down a little"
    } else {
        "Down clearly"
    };
    Some(json!({ "word": word, "was": was, "line": line, "why": reasons(w, i, a, last.wt), "season": season }))
}

fn own(w: &World, i: usize) -> bool {
    views::team_of(w, &w.people[i]).is_some_and(|t| views::is_mine(w, t))
}

fn eye(w: &World, i: usize) -> f64 {
    let p = &w.people[i];
    views::team_of(w, p).filter(|_| own(w, i)).map_or(10.0, |t| views::eye_for(w, t, p.player.position))
}

/// The biggest reasons a rating moved that your staff can see: his last
/// season's events and role, his body, his age, your plan and his teacher.
fn reasons(w: &World, i: usize, a: Attr, kept_wt: u16) -> Vec<String> {
    let p = &w.people[i];
    let pr = a.profile();
    let mut v: Vec<String> = Vec::new();
    let row = p.player.history.last();
    if let Some(h) = row {
        for e in h.events.iter() {
            let s = match e {
                E::Breakout => Some("A breakout season: everything he worked on clicked at once".to_string()),
                E::SpringBreakout => Some("A spring breakout".to_string()),
                E::Slump => Some("A slump: his game went backward for a season".to_string()),
                E::Stagnation => Some("A stalled season: he barely moved".to_string()),
                E::LostSeason => Some("A lost season".to_string()),
                E::GrowthSpurt if pr.group == AttributeGroup::Physical => Some("A growth spurt".to_string()),
                E::BodyStalled if pr.group == AttributeGroup::Physical => {
                    Some("His body has stopped developing".to_string())
                }
                E::MajorInjury => Some(format!("A major injury: he missed {} games", h.games_missed)),
                E::ModerateInjury => Some(format!("An injury cost him {} games", h.games_missed)),
                E::LostBurst if pr.injury_sensitive => {
                    Some("He never got all of his burst back after the injury".to_string())
                }
                E::FirstYear => Some("His first year at a new level".to_string()),
                E::PoorCoaching => Some("Poor coaching where he was".to_string()),
                E::Complacent => Some("He lost some of his drive".to_string()),
                E::WeightGain => Some("He put on bad weight".to_string()),
                E::Suspended => Some("He sat out the season".to_string()),
                _ => None,
            };
            v.extend(s);
        }
    }
    let id = p.player.id;
    if own(w, i) {
        if w.focus_of(id) == Some(a) {
            v.push("Your plan for him focuses on it".into());
        } else if w.staff_focus(id) == Some(a) {
            v.push("His position coach's plan focuses on it".into());
        }
    }
    let wt = afm_engine::body::body_now(&p.player).weight.round();
    let dw = wt - kept_wt as f64;
    if pr.group == AttributeGroup::Physical && kept_wt > 0 && dw.abs() >= 4.0 {
        v.push(format!("{} {:.0} lb since then ({wt:.0} lb now)", if dw > 0.0 { "Up" } else { "Down" }, dw.abs()));
    }
    let age = p.age_on(w.today);
    if age as f64 >= pr.decline_age {
        v.push(format!("He is {age}: past about {:.0} this rating fades a little each year", pr.decline_age));
    } else if (age as f64) < pr.growth_window.1 {
        v.push(format!("He is {age}, still in the years it grows fastest"));
    }
    if let Some(h) = row {
        let role = views::role_words(h.playing_time).to_lowercase();
        if pr.experience >= 0.3 || matches!(h.playing_time, PlayingTime::Redshirt | PlayingTime::Backup) {
            v.push(format!("A {role}'s snaps last season"));
        }
    }
    if let Some(t) = views::team_of(w, p).filter(|&t| views::is_mine(w, t))
        && pr.coachability >= 0.35
    {
        let tch = crate::lane_training::teacher(w, t, i);
        if let (Some(n), Some(words)) = (tch["name"].as_str(), tch["words"].as_str()) {
            let who = if tch["you"].as_bool().unwrap_or(false) {
                "You teach him".to_string()
            } else {
                format!("Taught by {n}")
            };
            v.push(format!("{who}: {words}"));
        }
    }
    v.truncate(3);
    v
}

// ---------- H3: the level rating ----------

fn level_help(w: &World, id: u32) -> Reply {
    let i = views::person_index(w, id).ok_or("That player is no longer in football.")?;
    let p = &w.people[i];
    let h = p.player.history.last().ok_or("He has no rated season yet.")?;
    let r = h.level_rating_end.or(h.level_rating).ok_or("He has no level rating.")?;
    let group = level_group(h.level, h.age).ok_or("Pros carry no level rating.")?;
    let rating = |s: &afm_engine::player::SeasonRecord| s.level_rating_end.or(s.level_rating);
    // This season's rated rows, as the world rates them (`attach_level_ratings`).
    let (mut above, mut total) = (0u32, 0u32);
    for q in &w.people {
        let Some(s) = q.player.history.last() else { continue };
        if s.age + 1 != q.player.age || level_group(s.level, s.age) != Some(group) {
            continue;
        }
        if let Some(x) = rating(s) {
            total += 1;
            if x > r {
                above += 1;
            }
        }
    }
    let fresh = h.age + 1 == p.player.age;
    let (title, pool, scale) = match group {
        LevelGroup::College => (
            format!("{r} among FBS players"),
            "Every college player's last season on one scale. FBS seasons set it; an FCS season counts one overall point lower and a smaller college's five.".to_string(),
            "Pooled over the last five seasons: a median player rates about 72, the No. 700 about 85, the No. 100 about 90 and the No. 8 about 95.",
        ),
        LevelGroup::HS(_) => {
            let peers = match p.class_of - w.year.min(p.class_of) {
                0 => "seniors",
                1 => "juniors",
                2 => "sophomores",
                _ => "freshmen",
            };
            (
                format!("{r} among {peers}"),
                format!("Every high school player his age in the country, {peers} by last season's class."),
                "Pooled over the last five seasons: a median player rates about 50, the No. 2,000 about 80, the No. 100 about 90 and the No. 8 about 95.",
            )
        }
    };
    let start = h.level_rating.filter(|&s| Some(s) != h.level_rating_end);
    Ok(json!({
        "title": title,
        "rating": r,
        "what": "His overall from last season, set against his own position so a 76 means the same at every spot, then ranked.",
        "pool": pool,
        "scale": scale,
        "above": if fresh {
            Some(format!("{} rated above him last season, of {}.", count_words(above), thousands(total)))
        } else {
            None
        },
        "start": start.map(|s| format!("He began that season at {s}.")),
        "public": "It is the public rating of his season, the same for every staff. Your staff's read of him is the range.",
    }))
}

fn count_words(n: u32) -> String {
    match n {
        0 => "Nobody".into(),
        1 => "One player".into(),
        _ => format!("{} players", thousands(n)),
    }
}

fn thousands(n: u32) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (k, c) in s.chars().enumerate() {
        if k > 0 && (s.len() - k).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

fn join_and(v: &[String]) -> String {
    match v.len() {
        0 => String::new(),
        1 => v[0].clone(),
        n => format!("{} and {}", v[..n - 1].join(", "), v[n - 1]),
    }
}

// ---------- H4: why is he here ----------

/// Your staff's read of one of your men at a position: his own position's
/// overall read there, else the "where else he could play" read.
pub(crate) fn read_at(w: &World, t: TeamRef, i: usize, pos: Position) -> Read {
    let p = &w.people[i];
    let e = views::eye_for(w, t, p.player.position);
    let reader = views::reader_key(w);
    if p.player.position == pos {
        return reads::overall(&p.player, true, e, reader);
    }
    let v = afm_engine::positions::overall_for(pos, &p.player.attributes);
    let half = 2.0 + (20.0 - e.clamp(1.0, 20.0)) * 0.12;
    reads::range(v, half + 1.0, reader, p.player.id, 300 + pos.index())
}

fn rr(r: Read) -> String {
    match r {
        Read::Range(lo, hi) => format!("{lo}-{hi}"),
        Read::NotSeen => "not seen".into(),
    }
}

fn last_name(w: &World, i: usize) -> String {
    first_last(&w.people[i].name).1
}

/// The staff member who keeps a side's depth chart: its coordinator, else a
/// coach of the group, else the head coach. Yours when it is your job.
fn chart_keeper(w: &World, pos: Position) -> Option<(String, String)> {
    use afm_engine::world::coach::{Group, Role};
    let g = Group::of(pos);
    let ms = sr::members(w);
    let coord = if g == Group::Specialists {
        None
    } else if g.offense() {
        Some(Role::OffensiveCoordinator)
    } else {
        Some(Role::DefensiveCoordinator)
    };
    let pick = coord
        .and_then(|r| ms.iter().find(|m| m.job == Job::Coach(r)))
        .or_else(|| {
            ms.iter().find(|m| {
                m.group == Some(g) && matches!(m.job, Job::Coach(Role::PositionCoach(_)) | Job::Room(_) | Job::HsCoach)
            })
        })
        .or_else(|| ms.iter().find(|m| m.job == Job::Coach(Role::HeadCoach)))?;
    Some((pick.name.clone(), pick.title.to_lowercase()))
}

/// "Why is he here?" for one depth chart spot (board 37): who put the
/// first man there, the biggest reasons your staff sees, who else could
/// play the spot and their practice this block. `men` are the spot's three.
#[allow(clippy::too_many_arguments)]
pub(crate) fn why_here(
    w: &World,
    t: TeamRef,
    roster: &[usize],
    pos: Position,
    spot: &str,
    controls: bool,
    yours: bool,
    men: &[Option<usize>; 3],
) -> Value {
    let Some(a) = men[0] else { return Value::Null };
    let pa = &w.people[a];
    let (one, _) = crate::lane_ratings::pos_words(pos);
    let ra = read_at(w, t, a, pos);
    // Who put him there.
    let (by, by_name, when) = if !controls {
        let hc = w.your_boss().map(|c| w.coaches[c].name.clone()).unwrap_or_else(|| "The head coach".into());
        ("C", format!("{hc}, the head coach"), format!("The head coach's call. Your job covers {}.", w.your_controls()))
    } else if yours {
        (
            "Y",
            "You".to_string(),
            "You set this order, so it stays until you change it or give it back to the staff.".to_string(),
        )
    } else if let Some((n, title)) = chart_keeper(w, pos).filter(|(_, title)| !title.is_empty()) {
        let you = w.career.as_ref().is_some_and(|c| w.coaches[c.coach].name == n);
        if you {
            (
                "A",
                "The auto chart".to_string(),
                "Your staff's chart from their reads: it moves with injuries every week until you set this spot."
                    .to_string(),
            )
        } else {
            (
                "C",
                format!("{n}, your {title}"),
                "Set from his reads in the staff meeting; it moves with injuries every week until you set this spot."
                    .to_string(),
            )
        }
    } else {
        (
            "A",
            "The auto chart".to_string(),
            "Your staff's chart from their reads: it moves with injuries every week until you set this spot."
                .to_string(),
        )
    };
    // The men at this position the chart chose between, by your read here.
    let mut pool: Vec<(f64, usize)> = roster
        .iter()
        .copied()
        .filter(|&k| k != a && (w.people[k].player.position == pos || men.contains(&Some(k))))
        .map(|k| (mid(read_at(w, t, k, pos)).unwrap_or(0.0), k))
        .collect();
    pool.sort_by(|x, y| y.0.total_cmp(&x.0));
    let healthy = |k: usize| health(w, k).0 != "out";
    let next = men[1].filter(|&k| healthy(k)).or_else(|| pool.iter().map(|x| x.1).find(|&k| healthy(k)));
    let mut why: Vec<(String, String)> = Vec::new();
    // 1. Someone better is hurt.
    let ma = mid(ra).unwrap_or(0.0);
    if let Some(&(_, k)) = pool.iter().find(|&&(m, k)| m > ma + 1.0 && !healthy(k)) {
        let (_, words) = health(w, k);
        why.push((
            format!("{} is out", w.people[k].name),
            format!("{}. The chart skips a player who is out.", cap(&words)),
        ));
    } else if let Some(&(_, k)) = pool.iter().find(|&&(m, k)| m > ma + 1.0 && health(w, k).0 == "limited") {
        let (_, words) = health(w, k);
        why.push((format!("{} is limited", w.people[k].name), format!("{}.", cap(&words))));
    }
    // 2. The read at the spot.
    match next {
        Some(b) => {
            let rb = read_at(w, t, b, pos);
            let gap = ma - mid(rb).unwrap_or(0.0);
            let title = if gap >= 4.0 {
                format!("Your staff's best read at {one}")
            } else if gap > -1.0 {
                "A close call at the spot".to_string()
            } else if by == "Y" {
                "Your choice over a higher read".to_string()
            } else {
                "The staff's choice over a higher read".to_string()
            };
            why.push((
                title,
                format!("{} {} here; {}, the next healthy man, {}.", last_name(w, a), rr(ra), w.people[b].name, rr(rb)),
            ));
            // 3. Where he beats the next man most.
            let reader = views::reader_key(w);
            let ea = reads::attributes(&pa.player, true, views::eye_for(w, t, pa.player.position), reader);
            let pb = &w.people[b];
            let eb = reads::attributes(&pb.player, true, views::eye_for(w, t, pb.player.position), reader);
            let get = |v: &[(Attr, Read)], x: Attr| v.iter().find(|y| y.0 == x).map_or(Read::NotSeen, |y| y.1);
            let best = key_attrs(pos)
                .into_iter()
                .filter_map(|x| Some((mid(get(&ea, x))? - mid(get(&eb, x))?, x)))
                .max_by(|p, q| p.0.total_cmp(&q.0));
            if let Some((d, x)) = best.filter(|b| b.0 >= 3.0) {
                let _ = d;
                why.push((
                    format!("Ahead on {}", x.label().to_lowercase()),
                    format!("{} {} to {}'s {}.", x.label(), rr(get(&ea, x)), last_name(w, b), rr(get(&eb, x))),
                ));
            }
        }
        None => why.push((
            "Nobody else is ready".to_string(),
            format!("Your staff reads him {} at {one}, with nobody healthy behind him.", rr(ra)),
        )),
    }
    // 4. A man from another spot.
    if pa.player.position != pos {
        let at = roster.iter().filter(|&&k| w.people[k].player.position == pos && healthy(k)).count();
        let n = afm_engine::game::roster::starters(pos);
        let s = if at < n {
            format!(
                "Only {at} healthy {} on the roster, so the staff slides him over.",
                crate::lane_ratings::pos_words(pos).1
            )
        } else {
            format!("He reads {} here against the room; the staff would rather start him than the next {one}.", rr(ra))
        };
        why.push((format!("He plays {}", crate::lane_ratings::pos_words(pa.player.position).0), s));
    }
    // 5. Practice and last season.
    let g = pa.player.train.grade;
    if g > 0 {
        why.push((
            "In practice".to_string(),
            format!("{} this block, on his position coach's chart.", cap(practice_words(g))),
        ));
    }
    if let Some(PlayingTime::Starter) = pa.last_role {
        why.push(("Started last season".to_string(), "He comes in with a starter's snaps behind him.".to_string()));
    }
    why.truncate(3);
    // Who else could play the spot: the men behind him, the best of the
    // rest at the position and the best from another spot.
    let mut alts: Vec<usize> = men[1..].iter().flatten().copied().collect();
    if let Some(&(_, k)) = pool.iter().find(|&&(_, k)| !alts.contains(&k) && w.people[k].player.position == pos) {
        alts.push(k);
    }
    let cross = roster
        .iter()
        .copied()
        .filter(|&k| {
            k != a
                && !alts.contains(&k)
                && w.people[k].player.position != pos
                && !w.people[k].player.position.is_specialist()
        })
        .map(|k| (mid(read_at(w, t, k, pos)).unwrap_or(0.0), k))
        .max_by(|x, y| x.0.total_cmp(&y.0));
    if let Some((_, k)) = cross {
        alts.push(k);
    }
    alts.truncate(4);
    let alt_rows: Vec<Value> = alts
        .iter()
        .map(|&k| {
            let q = &w.people[k];
            let r = read_at(w, t, k, pos);
            let (h, hw) = health(w, k);
            let d = mid(r).unwrap_or(0.0) - ma;
            let note = if h == "out" {
                format!("Out: {hw}.")
            } else if h == "limited" {
                format!("Limited: {hw}.")
            } else if q.player.position != pos {
                if d > 1.0 {
                    format!("Reads higher here, but your staff keeps him at {}.", q.player.position.name())
                } else {
                    format!("Plays {}: he would learn the spot.", q.player.position.name())
                }
            } else if Some(k) == men[1] {
                if d > -2.0 { "Next up, and pushing him.".to_string() } else { "Next up.".to_string() }
            } else if d > 1.0 {
                "Reads higher, and is behind him on the chart.".to_string()
            } else {
                "Further down the chart.".to_string()
            };
            json!({
                "id": q.player.id, "name": q.name, "pos": q.player.position.name(),
                "class": views::class_text(w, q), "read": r.json(), "note": note, "hurt": h != "full",
            })
        })
        .collect();
    let practice: Vec<Value> = std::iter::once(a)
        .chain(alts.iter().copied())
        .map(|k| {
            let q = &w.people[k];
            let g = q.player.train.grade;
            json!({ "id": q.player.id, "name": q.name, "grade": g, "words": practice_words(g) })
        })
        .collect();
    json!({
        "title": format!("{spot} · {}", pa.name),
        "sub": format!("{} · {}", views::class_text(w, pa), if pa.player.position == pos { format!("{} read {}", pos.name(), rr(ra)) } else { format!("a {} at {}, read {}", pa.player.position.name(), pos.name(), rr(ra)) }),
        "read": ra.json(),
        "by": by, "by_name": by_name, "when": when,
        "why": why.iter().map(|(t, s)| json!({ "t": t, "s": s })).collect::<Vec<_>>(),
        "else_title": format!("Who else could play {one}"),
        "alts": alt_rows,
        "practice": practice,
    })
}

fn practice_words(g: u8) -> &'static str {
    match g {
        0 => "no practice grades yet",
        1..=49 => "losing most of his reps",
        50..=56 => "below the room",
        57..=63 => "holding his own",
        64..=70 => "winning his share",
        _ => "winning most of his reps",
    }
}

// ---------- H5: a weak lineup before kickoff ----------

/// The lineup choices of yours your staff reads as clearly weak: a starter
/// at a spot you set with a healthy man on the bench whose read there is
/// clearly higher (8 points between the middles and no overlap). Each
/// comes with its position and the line your coordinator writes.
pub(crate) fn weak_spots(w: &World, t: TeamRef) -> Vec<(Position, String)> {
    let Some(game) = w.seat_game_depth() else { return Vec::new() };
    let roster = views::roster_people(w, t);
    let idx = |id: u32| roster.iter().copied().find(|&k| w.people[k].player.id == id);
    let mut out = Vec::new();
    for (pos, ids) in &game {
        let pos = *pos;
        if !w.controls(pos) || !w.seat_depth.iter().any(|(p, _)| *p == pos) {
            continue;
        }
        let n = afm_engine::game::roster::starters(pos);
        let starters: Vec<usize> = ids.iter().take(n).filter_map(|&id| idx(id)).collect();
        let bench: Vec<(Read, usize)> = roster
            .iter()
            .copied()
            .filter(|&k| w.people[k].player.position == pos && !starters.contains(&k) && health(w, k).0 != "out")
            .map(|k| (read_at(w, t, k, pos), k))
            .collect();
        for &s in &starters {
            let rs = read_at(w, t, s, pos);
            let (Read::Range(_, shi), Some(sm)) = (rs, mid(rs)) else { continue };
            let best = bench
                .iter()
                .filter(|(r, _)| matches!(r, Read::Range(lo, _) if *lo > shi) && mid(*r).is_some_and(|m| m - sm >= 8.0))
                .max_by(|x, y| mid(x.0).unwrap_or(0.0).total_cmp(&mid(y.0).unwrap_or(0.0)));
            if let Some(&(rb, b)) = best {
                let (one, _) = crate::lane_ratings::pos_words(pos);
                let ps = &w.people[s];
                let at = if ps.player.position == pos {
                    String::new()
                } else {
                    format!(" (he plays {})", ps.player.position.name())
                };
                out.push((
                    pos,
                    format!(
                        "{} starts at {one}{at}: we read him {} there. {}, {}, is on the bench.",
                        crate::lane_core::links::player(ps.player.id, &ps.name),
                        rr(rs),
                        crate::lane_core::links::player(w.people[b].player.id, &w.people[b].name),
                        rr(rb)
                    ),
                ));
                break;
            }
        }
    }
    out
}

/// Who writes the lineup note: the coordinator of the side, else a coach of
/// the group, else the head coach; never you.
pub(crate) fn note_writer(w: &World, pos: Position) -> Option<(String, String)> {
    let (n, title) = chart_keeper(w, pos)?;
    let you = w.career.as_ref().map(|c| w.coaches[c.coach].name.clone());
    (Some(&n) != you.as_ref()).then(|| (n, cap(&title)))
}

// ---------- H6: why a recruit chose, why a mood fell ----------

/// What a recruit's decider reads in a program, 0-1 per want: closeness,
/// the name, the path to the field, academics, development and money.
fn program_reads(w: &World, home: usize, prog: usize) -> [f64; 6] {
    let pr = &w.programs[prog];
    let g = |c: GradeCat| pr.grades[c as usize] / 100.0;
    let miles = afm_engine::world::geo::miles(home, pr.state);
    [
        1.0 / (1.0 + (miles / 300.0).powi(2)),
        (g(GradeCat::TitleChances)
            + g(GradeCat::StadiumAtmosphere)
            + g(GradeCat::Tradition)
            + g(GradeCat::BrandExposure))
            / 4.0,
        g(GradeCat::PlayingTime),
        g(GradeCat::Academics),
        (g(GradeCat::ProPipeline) + g(GradeCat::CoachPrestige)) / 2.0,
        g(GradeCat::Nil),
    ]
}

/// The biggest reasons a recruit picked `chosen`, as your staff reads his
/// family's wants (`front_office::college::family_read`): against `versus`
/// (your program) when given, else what the school offered that he wanted.
pub(crate) fn pick_why(w: &World, i: usize, chosen: usize, versus: Option<usize>) -> Vec<String> {
    use afm_engine::world::background::Decider;
    let p = &w.people[i];
    let f = afm_engine::world::front_office::college::family_read(w, i);
    let wants = [f.close_to_home, f.prestige, f.playing_time, f.academics, f.development, f.money];
    let total: f64 = wants.iter().sum::<f64>().max(1e-9);
    let a = program_reads(w, p.home_state, chosen);
    let b = versus.map(|v| program_reads(w, p.home_state, v));
    let name = w.program_name(chosen).to_string();
    let miles = |prog: usize| afm_engine::world::geo::miles(p.home_state, w.programs[prog].state);
    let mut scored: Vec<(f64, String)> = (0..6)
        .map(|k| {
            let edge = match b {
                Some(b) => a[k] - b[k],
                None => a[k] - 0.5,
            };
            let text = match k {
                0 => match versus {
                    Some(v) => format!("Closer to home: {:.0} miles against our {:.0}", miles(chosen), miles(v)),
                    None => format!("Close to home: {:.0} miles", miles(chosen)),
                },
                1 => format!("{name}'s name: titles, the stadium and television"),
                2 => format!("A clearer path to the field at {name}"),
                3 => format!("{name}'s academics"),
                4 => format!("{name}'s record of developing pros"),
                _ => format!("The NIL money at {name}"),
            };
            (wants[k] / total * edge, text)
        })
        .collect();
    scored.sort_by(|x, y| y.0.total_cmp(&x.0));
    let decider = match f.decider {
        Decider::Himself => "He decided himself",
        Decider::Parent => "A parent made the call",
        Decider::Grandparent => "His grandparents made the call",
        Decider::Handler => "A trainer close to the family steered it",
    };
    let top = (0..6).max_by(|&x, &y| wants[x].total_cmp(&wants[y])).unwrap_or(0);
    let want =
        ["staying close to home", "the biggest name", "playing time", "academics", "development", "the money"][top];
    let mut v = vec![format!("{decider}; the family weighs {want} most")];
    v.extend(scored.into_iter().filter(|x| x.0 > 0.004).take(2).map(|x| x.1));
    if v.len() == 1 {
        v.push(format!("No single edge: {name} was simply the better fit to the family"));
    }
    v
}

/// Your players whose mood fell in the week before `d`, with the dated
/// reasons behind the fall (newest first, at most three each).
pub(crate) fn mood_dips(w: &World, t: TeamRef, d: Date) -> Vec<(usize, Vec<String>)> {
    let mut out = Vec::new();
    for k in views::roster_people(w, t) {
        let id = w.people[k].player.id;
        let rs = afm_engine::world::folk::talks::reasons(w, id);
        let week: Vec<&(Date, String, f32)> = rs.iter().filter(|x| x.0 < d && x.0 >= d.add(-7)).collect();
        let fell: f32 = week.iter().map(|x| x.2).sum();
        if fell <= -0.08 {
            let why: Vec<String> = week.iter().filter(|x| x.2 < 0.0).take(3).map(|x| cap(&x.1)).collect();
            out.push((k, why));
        }
    }
    out.sort_by_key(|x| std::cmp::Reverse(w.people[x.0].stars));
    out
}

/// The why-line of a news item about one of your people (a recruit's
/// choice, a portal entry), as your staff sees it.
pub(crate) fn news_why(w: &World, me: TeamRef, kind: afm_engine::calendar::Kind, id: u32) -> Option<Vec<String>> {
    use afm_engine::calendar::Kind;
    let i = views::person_index(w, id)?;
    let p = &w.people[i];
    match (kind, me) {
        (Kind::Commitment, TeamRef::College(pi)) => {
            let (j, _, _) = p.cal.commit?;
            if j == pi {
                Some(pick_why(w, i, j, None))
            } else if p.cal.offers.contains(&pi) || p.cal.early_offers.iter().any(|o| o.0 == pi) {
                Some(pick_why(w, i, j, Some(pi)))
            } else {
                None
            }
        }
        (Kind::Commitment, TeamRef::HighSchool(h)) if p.hs == h => {
            let (j, _, _) = p.cal.commit?;
            Some(pick_why(w, i, j, None))
        }
        (Kind::Transfer, TeamRef::College(pi)) if p.college == Some(pi) => {
            let e = p.cal.portal?;
            Some(portal_why(w, i, e.why))
        }
        _ => None,
    }
}

/// Why one of your players entered the portal: what he told the staff, his
/// role and his mood's dated reasons.
pub(crate) fn portal_why(w: &World, i: usize, said: &str) -> Vec<String> {
    let p = &w.people[i];
    let mut v = Vec::new();
    if !said.is_empty() {
        v.push(format!("He told the staff: {}", said.trim_end_matches('.')));
    }
    if let Some(r) = p.last_role {
        v.push(format!("{} last season", views::role_words(r)));
    }
    let rs = afm_engine::world::folk::talks::reasons(w, p.player.id);
    v.extend(rs.iter().filter(|x| x.2 < 0.0).take(2).map(|x| cap(&x.1)));
    if p.stage == Stage::College && p.college_seasons >= 3 {
        v.push(format!("{} seasons in: a graduate's last chance to play somewhere", p.college_seasons));
    }
    v.truncate(3);
    v
}

/// Your boss's letter after a losing streak: the losses, the record and
/// what he hired you to do.
pub(crate) fn streak_why(losses: &[String], record: &str, expect: &str) -> Vec<String> {
    let mut v = Vec::new();
    if !losses.is_empty() {
        v.push(format!("Three straight losses: {}", join_and(losses)));
    }
    if !record.is_empty() {
        v.push(format!("The record now: {record}"));
    }
    if !expect.is_empty() {
        v.push(format!("What he hired you for: {}", expect.trim_end_matches('.')));
    }
    v
}
