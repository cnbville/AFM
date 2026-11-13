//! The bosses and the outside talk from the fact sheet (talk facts lane L10,
//! items 13, 15, 16 and 22): the press asks at the step a case has reached,
//! the boss answers for where the season stands and how the review went, a
//! GM's words follow who holds the final say, and a backchannel call names
//! what your chart says. Words and options only, apart from the two sizes
//! Cain said yes to on 2026-11-13: the review answers' trust and the press's
//! week before a case's next step.

use std::collections::HashSet;

use super::legacy::seat;
use crate::calendar::{Date, Level};
use crate::game::stats::GameStats;
use crate::player::PlayingTime;
use crate::positions::Position;
use crate::world::data::Division;
use crate::world::folk::boss::{self, Goal, GoalKind};
use crate::world::folk::courts::{Case, Step};
use crate::world::folk::scene::{self, Kind, Scene};
use crate::world::folk::voice::{Facts, Pen, Speaker, bank};
use crate::world::folk::{Team, backchannel, press};
use crate::world::pro::office::Power;
use crate::world::program::ProgramSeason;
use crate::world::seat::SeatGame;
use crate::world::{Stage, World};

/// Opens a scene the way the calendar does, held now and not stopping.
fn open_scene(w: &mut World, kind: Kind, c: i64, b: u64, team: Team, title: &str) -> (u32, Scene) {
    let mut s = Scene::new(kind, w.today, title);
    s.c = c;
    s.b = b;
    s.team = Some(team);
    s.stops = false;
    let id = scene::add(w, s);
    (id, scene::open(w, id).expect("the scene opens"))
}

fn text_of(s: &Scene) -> String {
    s.lines.iter().map(|l| l.text.clone()).collect::<Vec<_>>().join("\n")
}

/// What he said last in the scene.
fn him(s: &Scene) -> String {
    s.lines.iter().rev().find(|l| l.by == 2).map(|l| l.text.clone()).unwrap_or_default()
}

/// Whether `line` is one of the bank's versions of `key`, with its holes
/// filled the way the tree fills them.
fn in_bank(key: &str, line: &str, holes: &[(&str, &str)]) -> bool {
    bank().versions(key).into_iter().any(|v| {
        let mut t = v.0.to_string();
        for (hole, value) in holes {
            t = t.replace(hole, value);
        }
        t == line
    })
}

// ---------------------------------------------------------------------------
// Press (item 13).

/// Your college program with three players on it (the test world's seat
/// starts with an empty roster): their indexes.
fn with_players(w: &mut World) -> Vec<usize> {
    let p = w.seat.and_then(|s| s.program).expect("your program");
    w.add_class_aged(19);
    let year = w.year;
    let v: Vec<usize> = (0..w.people.len()).filter(|&i| w.people[i].stage == Stage::HighSchool).take(3).collect();
    for &i in &v {
        let q = &mut w.people[i];
        q.stage = Stage::College;
        q.college = Some(p);
        q.class_of = year - 1;
        q.college_seasons = 1;
        q.last_role = Some(PlayingTime::Backup);
        w.programs[p].roster.push(i);
    }
    v
}

/// A college seat world with one of your players at `step` of a case, and
/// the weekly press availability open on it: (world, scene id, scene, his
/// name, his index).
fn press_with_case(step: Step, again: bool) -> (World, u32, Scene, String, usize) {
    press_with_case_at(step, again, 0, 20)
}

/// As `press_with_case`, with the case opened `ago` days before today and
/// its next step `next_in` days after it.
fn press_with_case_at(step: Step, again: bool, ago: i32, next_in: i32) -> (World, u32, Scene, String, usize) {
    let mut w = seat("college");
    let i = with_players(&mut w)[0];
    let team = Team::seat(&w).expect("your team");
    let (id, name) = (w.people[i].player.id, w.people[i].name.clone());
    let case = |step, outcome: Option<String>| Case {
        id,
        level: Level::College,
        what: "a DUI".into(),
        opened: w.today.add(-ago),
        next: w.today.add(next_in),
        step,
        guilty: false,
        outcome,
    };
    if again {
        // An earlier case of his that ended in a conviction.
        let old = case(Step::Court, Some(format!("{name} was convicted.")));
        w.folk.courts.closed.push(old);
    }
    let now = case(step, None);
    w.folk.courts.open.push(now);
    let (sid, s) = open_scene(&mut w, Kind::Press, 1, 0, team, "Monday press availability");
    (w, sid, s, name, i)
}

#[test]
fn the_reporter_asks_at_the_step_the_case_has_reached() {
    for step in [Step::Arrest, Step::Charged, Step::Court] {
        let (_, _, s, name, _) = press_with_case(step, false);
        let ask: String = s
            .lines
            .iter()
            .filter(|l| l.text.contains(&name))
            .map(|l| l.text.to_lowercase())
            .collect::<Vec<_>>()
            .join(" ");
        assert!(!ask.is_empty(), "{step:?}: the reporter names {name}: {}", text_of(&s));
        if step == Step::Arrest {
            assert!(!ask.contains("court") && !ask.contains("judge"), "arrest: a court the case hasn't reached: {ask}");
        }
        if step == Step::Court {
            assert!(
                ["court", "judge", "case", "ruling"].iter().any(|k| ask.contains(k)),
                "court: nothing about the court: {ask}"
            );
        }
        // Your standard answer says where the case is, never "internally".
        let deflect = s.picks.iter().find(|p| p.mv == "Deflect").expect("the standard answer is on the table");
        let want = match step {
            Step::Arrest => "gathering the facts",
            Step::Charged => "his day in court",
            _ => "in front of a judge",
        };
        assert!(deflect.label.contains(want), "{step:?}: {}", deflect.label);
        assert!(!deflect.label.contains("internally"), "{step:?}: {}", deflect.label);
    }
}

#[test]
fn a_man_already_out_cant_be_suspended_again() {
    let (_, _, s, ..) = press_with_case(Step::Charged, false);
    assert!(s.picks.iter().any(|p| p.mv == "Warn"), "a man still playing can be suspended");
    // The same case with him already sitting out for the season.
    let mut w = seat("college");
    let i = with_players(&mut w)[0];
    let team = Team::seat(&w).expect("your team");
    w.people[i].cal.trouble = true;
    w.people[i].on_notice = true;
    let id = w.people[i].player.id;
    w.folk.courts.open.push(Case {
        id,
        level: Level::College,
        what: "a DUI".into(),
        opened: w.today,
        next: w.today.add(20),
        step: Step::Charged,
        guilty: false,
        outcome: None,
    });
    let (_, s) = open_scene(&mut w, Kind::Press, 1, 0, team, "Monday press availability");
    assert!(s.picks.iter().any(|p| p.mv == "Deflect"), "{}", text_of(&s));
    assert!(!s.picks.iter().any(|p| p.mv == "Warn"), "he is already out, yet you may suspend him");
}

#[test]
fn a_man_in_trouble_before_is_not_simply_a_good_kid() {
    let (_, _, first, ..) = press_with_case(Step::Charged, false);
    let (_, _, again, ..) = press_with_case(Step::Charged, true);
    let praise =
        |s: &Scene| s.picks.iter().find(|p| p.mv == "Praise").map(|p| p.label.clone()).expect("a loyal answer");
    assert!(praise(&first).contains("good kid"), "{}", praise(&first));
    assert!(!praise(&again).contains("good kid"), "{}", praise(&again));
}

#[test]
fn the_reporter_follows_up_on_what_you_said() {
    let (mut w, id, s, ..) = press_with_case(Step::Charged, false);
    let deflect = s.picks.iter().find(|p| p.mv == "Deflect").expect("the standard answer").code;
    let s = scene::answer(&mut w, id, deflect).expect("an answer");
    let b = bank();
    let follow: HashSet<String> =
        b.versions("press.follow.trouble.deflect").into_iter().map(|v| v.0.to_string()).collect();
    assert!(!follow.is_empty());
    assert!(s.lines.iter().any(|l| follow.contains(&l.text)), "no follow-up: {}", text_of(&s));
}

#[test]
fn a_repeat_case_brings_its_own_follow_ups() {
    // A reporter asks "how many chances" of a man who has been through a
    // case before, and never calls him "a good kid" after you didn't.
    let who = Speaker::default();
    let key = "press.follow.trouble.praise";
    let said = |again: &str| -> HashSet<String> {
        let f = Facts::new().with("again", again);
        (0..120).filter_map(|seed| Pen::on(seed, Date::ymd(2026, 9, 1), Vec::new()).part(key, &who, &f)).collect()
    };
    let (first, again) = (said("no"), said("yes"));
    assert!(again.iter().any(|l| l.contains("been through this before")), "{again:?}");
    assert!(!first.iter().any(|l| l.contains("been through this before")), "{first:?}");
    assert!(first.iter().any(|l| l.contains("good kid")), "{first:?}");
    assert!(!again.iter().any(|l| l.contains("good kid")), "{again:?}");
}

/// Whether the reporters ask about his case today.
fn asked_about_case(w: &World, i: usize) -> bool {
    press::in_trouble(w, Team::seat(w).expect("your team")) == Some(i)
}

#[test]
fn the_reporter_asks_again_the_week_before_a_case_moves() {
    use press::{TROUBLE_NEW_DAYS, TROUBLE_NEXT_STEP_DAYS};
    // A new case is asked about whatever its next step.
    let (w, _, s, _, i) = press_with_case_at(Step::Arrest, false, TROUBLE_NEW_DAYS - 1, 30);
    assert!(asked_about_case(&w, i), "a case opened {} days ago: {}", TROUBLE_NEW_DAYS - 1, text_of(&s));
    for step in [Step::Charged, Step::Court] {
        // An old case comes back the week before its court date or ruling,
        // in the part for its step.
        let (w, _, s, name, i) = press_with_case_at(step, false, 60, TROUBLE_NEXT_STEP_DAYS);
        assert!(asked_about_case(&w, i), "{step:?}: the next step in a week: {}", text_of(&s));
        let want = if step == Step::Charged { "his day in court" } else { "in front of a judge" };
        let deflect = s.picks.iter().find(|p| p.mv == "Deflect").expect("the standard answer is on the table");
        assert!(deflect.label.contains(want), "{step:?}: {}", deflect.label);
        assert!(s.lines.iter().any(|l| l.text.contains(&name)), "{step:?}: the reporter names {name}");
        // Not before.
        let (w, _, s, _, i) = press_with_case_at(step, false, 60, TROUBLE_NEXT_STEP_DAYS + 1);
        assert!(!asked_about_case(&w, i), "{step:?}: the next step more than a week away: {}", text_of(&s));
        assert!(!s.picks.iter().any(|p| p.label.contains(want)), "{step:?}: asked anyway: {}", text_of(&s));
    }
    // A league appeal is not a court case: only its first three weeks.
    let (w, _, s, _, i) = press_with_case_at(Step::Appeal { games: 2 }, false, 60, 3);
    assert!(!asked_about_case(&w, i), "an old appeal: {}", text_of(&s));
}

// ---------------------------------------------------------------------------
// The boss (item 15).

/// Your team's results this season, after camp opened.
fn results(w: &mut World, wins: u32, losses: u32) {
    let team = Team::seat(w).expect("your team");
    let (_, p) = team.parts();
    let opp = (0..w.programs.len()).find(|&q| q != p && w.programs[q].division == Division::FBS).unwrap();
    for k in 0..wins + losses {
        let score = if k < wins { [31, 10] } else { [10, 31] };
        w.seat_games.push(SeatGame {
            date: w.dates.fall_camp.add(k as i32),
            level: Level::College,
            teams: [p, opp],
            names: ["Home".into(), "Away".into()],
            stats: GameStats { score, ..Default::default() },
            replay: Vec::new(),
            marks: Vec::new(),
            gone: Vec::new(),
            spread: None,
            talks: Default::default(),
            whistle: None,
        });
    }
    w.folk.goals.clear();
}

#[test]
fn the_midseason_check_in_follows_where_the_season_stands() {
    for (wins, losses, pace, text) in [
        (6, 0, boss::Pace::Ahead, "ahead of where you thought"),
        (3, 3, boss::Pace::On, "about where we should be"),
        (0, 6, boss::Pace::Behind, "isn't good enough"),
    ] {
        let mut w = seat("college");
        results(&mut w, wins, losses);
        let team = Team::seat(&w).expect("your team");
        assert_eq!(boss::pace(&w, team), Some(pace), "{wins}-{losses}");
        let (id, s) = open_scene(&mut w, Kind::BossMeeting, 1, 0, team, "The midseason check-in");
        let tell = s.picks.iter().find(|p| p.code == 30).expect("tell him straight");
        assert!(tell.label.contains(text), "{wins}-{losses}: {}", tell.label);
        // Owning a bad year is on the table when the year is bad, and only
        // then.
        let blame = s.picks.iter().any(|p| p.code == 31);
        assert_eq!(blame, pace == boss::Pace::Behind, "{wins}-{losses}: the take-the-blame option");
        let s = scene::answer(&mut w, id, 30).expect("an answer");
        let line = him(&s);
        let rec = format!("{wins}-{losses}");
        let part = |p: boss::Pace| format!("boss.mid.reply.{}", p.key());
        assert!(in_bank(&part(pace), &line, &[("{record}", &rec)]), "{rec}: {line:?} is not from {}", part(pace));
        for other in [boss::Pace::Ahead, boss::Pace::Behind] {
            if other != pace {
                assert!(
                    !in_bank(&part(other), &line, &[("{record}", &rec)]),
                    "{rec}: {line:?} is for a {other:?} year"
                );
            }
        }
    }
}

#[test]
fn he_answers_a_good_year_and_a_bad_one_differently() {
    let b = bank();
    let spk = Speaker::default();
    let f = Facts::new().with("record", "6-0").with("boss", "Pat");
    let texts = |key: &str| -> HashSet<String> { b.versions(key).into_iter().map(|v| v.0.to_string()).collect() };
    let (ahead, behind) = (texts("boss.mid.reply.ahead"), texts("boss.mid.reply.behind"));
    assert!(ahead.is_disjoint(&behind), "one reply for a good year and a bad one");
    for k in ["boss.mid.reply.ahead", "boss.mid.reply.on", "boss.mid.reply.behind"] {
        assert!(b.fitting(k, &spk, &f) >= 3, "{k}");
    }
}

/// A college seat world after an 8-4 year with the season's goals set and
/// the review open.
fn review_with(goals: &[GoalKind]) -> (World, u32, Scene, Team) {
    let mut w = seat("college");
    let team = Team::seat(&w).expect("your team");
    let (year, today) = (w.year, w.today);
    let p = w.seat.and_then(|s| s.program).expect("your program");
    w.programs[p].seasons.push(ProgramSeason {
        year,
        wins: 8,
        losses: 4,
        conference_title: false,
        national_title: false,
        coach: None,
        strength: 0.0,
        totals: Default::default(),
        conference_won: None,
        conference: None,
        division: None,
    });
    w.folk.goals.clear();
    for kind in goals {
        w.folk.goals.push(Goal { kind: *kind, season: year, deal: 0, met: None, set_on: today });
    }
    let (id, s) = open_scene(&mut w, Kind::BossMeeting, 2, year as u64, team, "The season review");
    (w, id, s, team)
}

#[test]
fn a_review_with_misses_can_be_answered() {
    for (goals, codes) in [
        // A miss (ten wins) and a goal met (a bowl): own it, point to the bowl,
        // or ask.
        (vec![GoalKind::Wins(10), GoalKind::Postseason], vec![50, 51, 52]),
        // Nothing met: own it or ask; nothing to point to.
        (vec![GoalKind::Wins(10), GoalKind::Title], vec![50, 52]),
    ] {
        let (_, _, s, _) = review_with(&goals);
        let got: Vec<u32> = s.picks.iter().map(|p| p.code).filter(|c| (50..=52).contains(c)).collect();
        assert_eq!(got, codes, "{goals:?}: {}", text_of(&s));
        for code in codes {
            let (mut w, id, _, team) = review_with(&goals);
            // How the season ended moved his trust when the review opened;
            // the answer adds a little on top, more for a goal met.
            let key = boss::boss(&w, team).key();
            let before = w.folk.mine.trust_of(key).unwrap_or(0.5);
            let s = scene::answer(&mut w, id, code).expect("an answer");
            let by = if code == 51 { boss::REVIEW_POINT_TRUST } else { boss::REVIEW_OWN_TRUST };
            let after = w.folk.mine.trust_of(key).unwrap_or(0.5);
            assert!((after - before - by).abs() < 1e-6, "{goals:?}: {code} moved his trust {before} to {after}");
            let line = him(&s);
            // 50 and 52 name the goal he missed, 51 the one he met.
            let missed = GoalKind::Wins(10).words(Level::College);
            let met = GoalKind::Postseason.words(Level::College);
            let (part, goal) = match code {
                50 => ("boss.review.reply.own", &missed),
                51 => ("boss.review.reply.point", &met),
                _ => ("boss.review.reply.ask", &missed),
            };
            assert!(in_bank(part, &line, &[("{goal}", goal)]), "{goals:?}: {code}: {line:?} is not from {part}");
            assert!(s.outcome.iter().any(|o| o.starts_with("You ")), "{goals:?}: {code}: {:?}", s.outcome);
        }
    }
}

// ---------------------------------------------------------------------------
// Boosters, the old coach and the GM (item 16).

#[test]
fn a_big_giver_is_answered_as_one() {
    let b = bank();
    let spk = Speaker { key: 1, tags: vec!["booster", "college", "ego"] };
    let with = |giver: &str| Facts::new().with("name", "Hal").with("giver", giver);
    for key in ["booster.reply.promise", "booster.reply.straight", "booster.reply.thanks"] {
        let (big, small) = (b.fitting(key, &spk, &with("big")), b.fitting(key, &spk, &with("small")));
        assert!(big > small, "{key}: a big giver hears {big} and a small one {small}");
    }
}

#[test]
fn the_gm_offers_what_you_dont_already_hold() {
    for (power, offer) in
        [(Power::Gm, "want a say"), (Power::Coach, "final call on the 53 is yours"), (Power::Owner, "owner's room")]
    {
        let mut w = seat("nfl");
        let team = Team::seat(&w).expect("your team");
        let Team::Nfl(t) = team else { panic!("an NFL seat") };
        w.clubs[t].office.power = power;
        let (id, s) = open_scene(&mut w, Kind::Gm, 0, 0, team, "The general manager");
        let say = s.picks.iter().find(|p| p.code == 170).expect("the say option");
        assert!(say.label.contains(offer), "{power:?}: {}", say.label);
        // And his answer never says the other way.
        let s = scene::answer(&mut w, id, 170).expect("an answer");
        let line = him(&s);
        assert!(in_bank("gm.reply.say", &line, &[]), "{power:?}: {line:?} is not from the bank");
        if power != Power::Gm {
            assert!(!line.contains("The decisions are mine"), "{power:?}: {line}");
        }
    }
}

#[test]
fn the_gm_speaks_for_who_holds_the_final_say() {
    let who = Speaker { key: 3, tags: vec!["gm", "nfl"] };
    for (power, never) in [
        ("gm", &["final call is yours", "hold the pen", "owner has the last word", "owner decides"][..]),
        ("coach", &["decisions are mine", "won't get a vote", "arrangement", "owner decides"][..]),
        ("owner", &["decisions are mine", "won't get a vote", "arrangement", "hold the pen"][..]),
    ] {
        let f = Facts::new().with("power", power).with("mode", "middle");
        let mut said = HashSet::new();
        for seed in 0..60 {
            let mut pen = Pen::on(seed, Date::ymd(2026, 9, 1), Vec::new());
            let line = pen.part("gm.reply.say", &who, &f).expect("a reply he can say");
            let low = line.to_lowercase();
            assert!(!never.iter().any(|n| low.contains(n)), "{power}: says what isn't so: {line}");
            said.insert(line);
        }
        assert!(said.len() >= 2, "{power}: one reply only");
    }
}

#[test]
fn the_gm_names_the_club_s_mode_only_when_it_is_so() {
    let who = Speaker { key: 3, tags: vec!["gm", "nfl"] };
    // A line that only fits one kind of year, and the years it must not be
    // said in.
    let only = [
        ("in a rebuild, nobody is untouchable", ["contend", "middle"]),
        ("the guys who got us here", ["rebuild", "middle"]),
        ("not many. ask me again once we know what kind of year it is", ["contend", "rebuild"]),
    ];
    let mut hits = 0;
    for (phrase, wrong) in only {
        for mode in wrong {
            let f = Facts::new().with("power", "gm").with("mode", mode);
            for seed in 0..80 {
                let mut pen = Pen::on(seed, Date::ymd(2026, 9, 1), Vec::new());
                let line = pen.part("gm.reply.never", &who, &f).expect("a reply he can say").to_lowercase();
                assert!(!line.contains(phrase), "{mode}: {line}");
            }
        }
        // And it is said in its own year.
        let right = if phrase.starts_with("in a rebuild") {
            "rebuild"
        } else if phrase.starts_with("the guys") {
            "contend"
        } else {
            "middle"
        };
        let f = Facts::new().with("power", "gm").with("mode", right);
        for seed in 0..80 {
            let mut pen = Pen::on(seed, Date::ymd(2026, 9, 1), Vec::new());
            let line = pen.part("gm.reply.never", &who, &f).expect("a reply he can say").to_lowercase();
            hits += usize::from(line.contains(phrase));
        }
    }
    assert!(hits >= 3, "each year has a line of its own ({hits})");
}

#[test]
fn a_deal_you_cant_pay_is_not_on_the_table() {
    for (cost, offered) in [(1.0e12, false), (1.0, false), (0.0, true)] {
        let mut w = seat("college");
        let i = with_players(&mut w)[0];
        let team = Team::seat(&w).expect("your team");
        let (_, p) = team.parts();
        let mut s = Scene::new(Kind::Backchannel, w.today, "A friend calls");
        s.a = w.people[i].player.id as u64;
        s.b = 15;
        s.c = p as i64;
        s.f = [cost, 0.0, 0.0, 0.0];
        s.stops = false;
        s.team = Some(team);
        let sid = scene::add(&mut w, s);
        let s = scene::open(&mut w, sid).expect("the call opens");
        let deal = s.picks.iter().any(|x| x.code == 151);
        assert_eq!(deal, offered, "{cost}: {}", text_of(&s));
        assert!(s.picks.iter().any(|x| x.code == 152), "you can always talk to the kid first");
        let says = text_of(&s).contains("Your collective has");
        assert_eq!(says, !offered, "{cost}: the call says why the deal is gone: {}", text_of(&s));
    }
}

// ---------------------------------------------------------------------------
// Backchannel (item 22).

#[test]
fn a_deal_for_a_player_names_what_your_chart_says() {
    let w = seat("nfl");
    let depth = w.seat_game_depth().expect("your team can dress a game team");
    let mut named = 0;
    for (pos, ids) in depth {
        let words = backchannel::chart_words(&w, pos).expect("there is a chart");
        match ids.first() {
            Some(id) => {
                assert!(words.contains(&format!("[[p:{id}|")), "{pos:?}: {words}");
                named += 1;
            }
            None => assert!(words.contains("Nobody is listed"), "{pos:?}: {words}"),
        }
    }
    assert!(named >= 11, "a chart starts at {named} spots");
    assert!(backchannel::chart_words(&w, Position::QB).expect("a chart").contains("quarterback"));
    // The interior line is one spot on the chart, not "guard".
    assert!(backchannel::chart_words(&w, Position::IOL).expect("a chart").contains("interior line"));
    // A team that can't dress a game team has no chart to speak of.
    let empty = seat("college");
    assert!(backchannel::chart_words(&empty, Position::QB).is_none());
}
