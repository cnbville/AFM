//! The fact sheet (talk facts design, lane L3; `world/folk/claims/`): one
//! reader of the world for every talk. These prove the catalogue's keys,
//! your proof's grade against his other schools, the windows (nothing is
//! named past its window), what a point counts the second time, that no
//! hidden number reaches a value a line or a screen can read, and the
//! three names a scene shares with the live talks.

use super::legacy::seat;
use crate::calendar::Level;
use crate::game::stats::GameStats;
use crate::player::PlayingTime;
use crate::world::background::Decider;
use crate::world::coach::Group as Room;
use crate::world::folk::claims::{
    self, Alumni, Basis, Conduct, Ear, Family, Group, Kind, Point, Sheet, Talked, Tier, When, Whose,
};
use crate::world::folk::claims::{kinds, sheet};
use crate::world::folk::courts::{Case, Step};
use crate::world::folk::scene::{self, Pick, PickKind, Scene};
use crate::world::folk::{Team, voice};
use crate::world::seat::SeatGame;
use crate::world::{Stage, Visit, World};
use std::collections::HashSet;

/// Your college program with `n` players on it (sophomores), by index.
fn room(n: usize) -> (World, usize, Vec<usize>) {
    let mut w = seat("college");
    let p = w.seat.and_then(|s| s.program).unwrap();
    w.add_class_aged(19);
    let year = w.year;
    let v: Vec<usize> = (0..w.people.len()).filter(|&i| w.people[i].stage == Stage::HighSchool).take(n).collect();
    for &i in &v {
        let q = &mut w.people[i];
        q.stage = Stage::College;
        q.college = Some(p);
        q.class_of = year - 1;
        q.college_seasons = 1;
        q.last_role = Some(PlayingTime::Backup);
        w.programs[p].roster.push(i);
    }
    (w, p, v)
}

/// Your college program and `n` high school recruits who aren't
/// specialists, with three other schools on each one's list.
fn recruits(n: usize) -> (World, usize, Vec<usize>, Vec<usize>) {
    let mut w = seat("college");
    let p = w.seat.and_then(|s| s.program).unwrap();
    w.add_class_aged(17);
    let others: Vec<usize> = (0..w.programs.len())
        .filter(|&j| j != p && w.programs[j].division == crate::world::data::Division::FBS)
        .take(3)
        .collect();
    let v: Vec<usize> = (0..w.people.len())
        .filter(|&i| w.people[i].stage == Stage::HighSchool && !w.people[i].player.position.is_specialist())
        .take(n)
        .collect();
    for &i in &v {
        let mut list = vec![p];
        list.extend(&others);
        w.people[i].cal.offers = list;
    }
    (w, p, others, v)
}

/// Everything a line or a screen can read from a sheet: each point's
/// holes and checks, as `fill` hands them to the line bank.
fn readable(s: &Sheet) -> Vec<(Kind, &'static str, String)> {
    let mut out = Vec::new();
    for p in &s.points {
        let mut f = voice::Facts::new();
        p.fill(&mut f);
        for (k, v) in f.0 {
            out.push((p.kind, k, v));
        }
    }
    out
}

/// A number with a decimal point in it ("4.52").
fn has_decimal(v: &str) -> bool {
    let b = v.as_bytes();
    (1..b.len().saturating_sub(1)).any(|k| b[k] == b'.' && b[k - 1].is_ascii_digit() && b[k + 1].is_ascii_digit())
}

/// The holes a measured number may sit in (`Point::measured`): what a
/// stopwatch gives.
const MEASURED: [&str; 3] = ["forty", "forty_before", "time"];

/// A sheet is well formed: sorted by what is most on his mind, every
/// engine number in range, every value whole, decimals only where a
/// stopwatch put them.
fn well_formed(s: &Sheet) {
    for pair in s.points.windows(2) {
        assert!(pair[0].mind >= pair[1].mind, "{:?} before {:?}", pair[0].kind, pair[1].kind);
    }
    for p in &s.points {
        assert!((0.0..=1.0).contains(&p.mind) && p.care.is_finite() && p.care > 0.0, "{:?}", p.kind);
        if let Some(day) = p.day {
            assert!(sheet::in_window(p.kind, day, s.day), "{:?} is named past its window", p.kind);
        }
        if let Some(pr) = p.proof {
            assert!((0.0..=1.0).contains(&pr.strength), "{:?} {}", p.kind, pr.strength);
            assert_eq!(p.check("tier"), Some(pr.tier.key()));
        }
    }
    for (kind, hole, v) in readable(s) {
        assert!(!v.is_empty() && !v.contains(['{', '}']), "{kind:?} {hole}: {v:?}");
        assert!(!["NaN", "inf"].iter().any(|bad| v.split(|c: char| !c.is_alphanumeric()).any(|t| t == *bad)), "{v}");
        assert!(!has_decimal(&v) || MEASURED.contains(&hole), "{kind:?} {hole} carries a decimal: {v}");
    }
}

#[test]
fn the_catalogue_keys_are_unique_and_saved_by_key() {
    let mut seen = HashSet::new();
    for &k in Kind::ALL {
        let key = k.key();
        assert!(!key.is_empty() && key.chars().all(|c| c.is_ascii_lowercase() || c == '_'), "{key}");
        assert!(seen.insert(key), "two kinds share the key {key}");
        assert_eq!(Kind::from_key(key), Some(k));
        assert_eq!(serde_json::to_string(&k).unwrap(), format!("\"{key}\""));
        assert_eq!(serde_json::from_str::<Kind>(&format!("\"{key}\"")).unwrap(), k);
        assert!(k.size() > 0.0 && k.size() <= 1.0 && k.window() >= 0, "{key}");
        // A point of yours is proof: only a recruit's sheet and your
        // team's own carry one.
        match k.family() {
            Family::Pitch(g) => assert_eq!(k.group(), Some(g)),
            Family::Player | Family::Offseason => {
                assert_ne!(k.whose(), Whose::Yours, "{key}: a player's point is his or shared");
                assert_eq!(k.group(), None);
            }
            _ => assert_eq!(k.group(), None),
        }
        // Something still ahead is checked on its day.
        if k.family() == Family::Next {
            assert_eq!(k.when(), When::Day, "{key}");
        }
    }
    assert!(serde_json::from_str::<Kind>("\"no_such_point\"").is_err());
    for g in Group::ALL {
        assert!(Kind::of_group(g).count() >= 3, "{} has too few points", g.name());
        assert_eq!(Group::from_key(g.key()), Some(g));
    }
    assert_eq!(Group::ALL.iter().map(|&g| Kind::of_group(g).count()).sum::<usize>(), 33);
    assert!(Kind::Arrest.is_trouble() && !Kind::BigGame.is_trouble() && Kind::Death.is_life());
}

#[test]
fn your_proof_is_graded_against_his_other_schools() {
    // The best on his list by a clear lead is elite; the best by a hair,
    // or second, is strong; the middle is fair; below it is weak.
    assert_eq!(Tier::grade(0.9, &[0.5, 0.4, 0.3]), Tier::Elite);
    assert_eq!(Tier::grade(0.52, &[0.5, 0.4, 0.3]), Tier::Strong);
    assert_eq!(Tier::grade(0.45, &[0.5, 0.4, 0.3]), Tier::Strong);
    assert_eq!(Tier::grade(0.35, &[0.5, 0.4, 0.3, 0.2]), Tier::Fair);
    assert_eq!(Tier::grade(0.25, &[0.5, 0.4, 0.3, 0.2]), Tier::Weak);
    assert_eq!(Tier::grade(0.1, &[0.5, 0.4, 0.3]), Tier::Weak);
    // Level with every other school is not a lead.
    assert_eq!(Tier::grade(0.5, &[0.5, 0.5]), Tier::Strong);
    // A school that lacks the fact counts as nothing.
    assert_eq!(Tier::grade(0.3, &[0.0, 0.0]), Tier::Elite);
    // Nobody else on his list: the fact's own size is the grade.
    for x in [0.9, 0.7, 0.4, 0.1] {
        assert_eq!(Tier::grade(x, &[]), Tier::of(x));
    }
    let v: Vec<f32> = Tier::ALL.iter().map(|t| t.value()).collect();
    assert!(v.windows(2).all(|p| p[0] > p[1]), "elite, strong, fair, weak in order: {v:?}");
    assert_eq!(v, sheet::PROOF.to_vec());
    assert!(Tier::Elite > Tier::Strong && Tier::Strong.rank() == 3);
    // What a move rests on (rule 5).
    assert!(Basis::OnMind.lift() > Basis::True.lift() && Basis::True.lift() > Basis::Plain.lift());
    assert_eq!(Basis::Against.lift(), 0.0);
    assert_eq!(Basis::Against.trust_cost(-1.0), sheet::AGAINST_LEAST);
    assert_eq!(Basis::Against.trust_cost(2.0), sheet::AGAINST_MOST);
    assert_eq!(Basis::Plain.trust_cost(2.0), 0.0);
}

#[test]
fn a_hairs_lead_is_strong_and_only_your_program_has_your_word() {
    // Level schools, or one other: a hair ahead is not "clearly" ahead.
    assert_eq!(Tier::grade(0.51, &[0.5]), Tier::Strong);
    assert_eq!(Tier::grade(0.501, &[0.5, 0.5]), Tier::Strong);
    assert_eq!(Tier::grade(0.3, &[0.0, 0.0]), Tier::Elite);
    assert_eq!(Tier::grade(0.7, &[0.5]), Tier::Elite);
    // Your word is yours alone: graded on its own size, a poor record is
    // weak however the other schools stand.
    let (mut w, p, _, v) = recruits(1);
    let i = v[0];
    for k in 0..6 {
        w.folk.promises.push(crate::world::folk::talks::Promise {
            to: crate::world::folk::talks::PromiseTo::Player(900_000 + k),
            kind: crate::world::folk::talks::PromiseKind::Start,
            made: w.today.add(-200),
            due: w.today.add(-100),
            public: false,
            kept: Some(k == 0),
            who: String::new(),
        });
    }
    let s = claims::recruit(&w, i, p, Ear::Himself);
    let word = s.get(Kind::YourWord).expect("five broken promises are a record");
    assert!(word.is("record", "poor"));
    assert_eq!(word.tier(), Some(Tier::Weak), "{word:?}");
}

#[test]
fn conduct_waits_while_a_case_is_open() {
    let day = crate::calendar::Date::ymd(2026, 9, 1);
    let mut s = Sheet::new(day, Some(1), None);
    s.points = vec![Point::new(Kind::Arrest), Point::new(Kind::Suspended)];
    assert_eq!(s.conduct(), Conduct::Open, "suspended for the arrest: still open");
    s.points.remove(0);
    assert_eq!(s.conduct(), Conduct::AtFault);
    s.points = vec![Point::new(Kind::Arrest), Point::new(Kind::Effort)];
    assert_eq!(s.conduct(), Conduct::Open);
}

#[test]
fn report_day_reads_each_mans_own_pounds() {
    let (mut w, _, v) = room(3);
    let ids: Vec<u32> = v.iter().map(|&i| w.people[i].player.id).collect();
    let tag = |k: usize| format!("[[p:{}|{}]]", ids[k], w.people[v[k]].name);
    let text = format!(
        "Report day: 3 of 40 missed. Failed: {}, {}. Over their weight: {} (9 lb over).",
        tag(0),
        tag(1),
        tag(2)
    );
    w.training.notes.push(crate::training::seat::Note { date: w.today, id: 0, dept: "strength".into(), text });
    let failed = claims::player(&w, v[0]);
    assert!(failed.get(Kind::ReportDay).is_none(), "a man who failed is not over his weight");
    let heavy = claims::player(&w, v[2]);
    let pt = heavy.get(Kind::ReportDay).expect("over his weight");
    assert_eq!(pt.get("pounds"), Some("9"));
    assert!(pt.is("shape", "heavy"));
}

/// One of your games: each man's snaps on both sides.
fn game_on(w: &mut World, p: usize, day: i32, snaps: &[(u32, u32)]) {
    let opp = (0..w.programs.len()).find(|&j| j != p).unwrap();
    let mut stats = GameStats { score: [21, 14], ..Default::default() };
    for &(id, n) in snaps {
        stats.players[0].insert(id, crate::game::stats::StatLine { snaps_off: n, snaps_def: n, ..Default::default() });
    }
    w.seat_games.push(SeatGame {
        date: w.today.add(day),
        level: Level::College,
        teams: [p, opp],
        names: [w.program_name(p).to_string(), w.program_name(opp).to_string()],
        stats,
        replay: Vec::new(),
        marks: Vec::new(),
        gone: Vec::new(),
        spread: None,
        talks: Default::default(),
        whistle: None,
    });
}

/// One of your practice weeks: each man's group.
fn week_on(w: &mut World, day: i32, groups: &[(u32, u8)]) {
    let mut g = groups.to_vec();
    g.sort();
    w.training.groups.push((w.today.add(day), g));
}

#[test]
fn a_job_changes_hands_when_he_sees_it_twice_never_from_the_chart() {
    use crate::training::seat::{GROUP_ONES as ONES, GROUP_TWOS as TWOS};
    let (mut w, p, v) = room(2);
    for &i in &v {
        w.people[i].player.position = crate::positions::Position::WR;
    }
    let (a, b) = (w.people[v[0]].player.id, w.people[v[1]].player.id);
    let up = |s: &Sheet| s.has(Kind::WonJob) || s.get(Kind::ChartMove).is_some_and(|x| x.is("move", "up"));
    let down = |s: &Sheet| s.has(Kind::LostJob) || s.get(Kind::ChartMove).is_some_and(|x| x.is("move", "down"));
    // He started your game and worked with the ones after it.
    game_on(&mut w, p, -24, &[(a, 60), (b, 4)]);
    week_on(&mut w, -20, &[(a, ONES), (b, TWOS)]);
    // Your chart says the backup starts: nobody sees a chart.
    w.set_seat_depth(crate::positions::Position::WR, vec![b, a]);
    assert!(!up(&claims::player(&w, v[1])), "a chart he never saw wins him nothing");
    assert!(!down(&claims::player(&w, v[0])));
    // One week with the twos is a warning, not a lost job.
    week_on(&mut w, -13, &[(a, TWOS), (b, ONES)]);
    assert!(!down(&claims::player(&w, v[0])), "one week is a warning");
    assert!(!up(&claims::player(&w, v[1])));
    // The second week makes it.
    week_on(&mut w, -6, &[(a, TWOS), (b, ONES)]);
    let won = claims::player(&w, v[1]);
    assert!(up(&won));
    assert!(won.points.iter().any(|x| x.refs.contains(&claims::Ref::Player(a))), "he passed the starter");
    let lost = claims::player(&w, v[0]);
    assert!(down(&lost));
    assert!(lost.points.iter().any(|x| x.refs.contains(&claims::Ref::Player(b))), "the man with the ones took it");
}

#[test]
fn buried_is_two_weeks_away_from_the_ones_behind_the_men_who_play() {
    use crate::training::seat::{GROUP_APART as APART, GROUP_ONES as ONES, GROUP_TWOS as TWOS};
    let (mut w, p, v) = room(3);
    for &i in &v {
        w.people[i].player.position = crate::positions::Position::WR;
        w.people[i].college_seasons = 2;
    }
    let ids: Vec<u32> = v.iter().map(|&i| w.people[i].player.id).collect();
    let (a, b, c) = (ids[0], ids[1], ids[2]);
    // Two weeks apart from the two-deep, behind two men with the ones.
    week_on(&mut w, -13, &[(a, APART), (b, ONES), (c, ONES)]);
    assert!(!claims::player(&w, v[0]).has(Kind::Buried), "one week is not buried");
    week_on(&mut w, -6, &[(a, APART), (b, ONES), (c, ONES)]);
    game_on(&mut w, p, -3, &[(b, 60), (c, 50)]);
    let s = claims::player(&w, v[0]);
    let pt = s.get(Kind::Buried).expect("two weeks apart");
    assert!(pt.is("group", "apart"));
    assert_eq!(
        pt.get("ahead").map(String::from),
        Some(w.people[v[1]].name.clone()),
        "the man with the most snaps first"
    );
    assert!(pt.get("ahead_all").is_some_and(|x| x.contains(&w.people[v[2]].name)), "every man with the ones");
    // With the twos he is buried only if he doesn't play.
    let (mut w2, p2, v2) = room(2);
    for &i in &v2 {
        w2.people[i].player.position = crate::positions::Position::WR;
        w2.people[i].college_seasons = 2;
    }
    let (a, b) = (w2.people[v2[0]].player.id, w2.people[v2[1]].player.id);
    week_on(&mut w2, -13, &[(a, TWOS), (b, ONES)]);
    game_on(&mut w2, p2, -10, &[(a, 0), (b, 60)]);
    week_on(&mut w2, -6, &[(a, TWOS), (b, ONES)]);
    game_on(&mut w2, p2, -3, &[(a, 20), (b, 60)]);
    assert!(!claims::player(&w2, v2[0]).has(Kind::Buried), "he plays: not buried");
    w2.seat_games.last_mut().unwrap().stats.players[0].insert(a, Default::default());
    let s = claims::player(&w2, v2[0]);
    let pt = s.get(Kind::Buried).expect("with the twos and no snaps");
    assert!(pt.is("group", "twos"));
}

#[test]
fn a_dated_point_fades_and_then_leaves_its_window() {
    let (w, _, _) = room(1);
    let d = w.today;
    let win = Kind::BigGame.window();
    assert_eq!(win, kinds::GAME_DAYS);
    assert_eq!(sheet::fresh(Kind::BigGame, Some(d), d), Some(1.0));
    let last = sheet::fresh(Kind::BigGame, Some(d), d.add(win)).unwrap();
    assert!((last - sheet::FRESH_FLOOR).abs() < 1e-6, "{last}");
    assert_eq!(sheet::fresh(Kind::BigGame, Some(d), d.add(win + 1)), None, "a game is named for a week");
    assert_eq!(sheet::fresh(Kind::BigGame, Some(d.add(1)), d), None, "nothing is named before it happens");
    // A point that lasts as long as it is true never fades.
    assert_eq!(sheet::fresh(Kind::Hurt, Some(d), d.add(300)), Some(1.0));
    assert_eq!(sheet::fresh(Kind::NextGame, None, d), Some(1.0));
    assert!(sheet::in_window(Kind::LostJob, d, d.add(kinds::JOB_DAYS)));
    assert!(!sheet::in_window(Kind::LostJob, d, d.add(kinds::JOB_DAYS + 1)));
}

#[test]
fn a_point_counts_once_then_half_then_a_quarter() {
    let (w, _, _) = room(1);
    let d = w.today;
    let mut t = Talked::default();
    assert!(t.is_empty());
    let game = Point::new(Kind::BigGame).on(d);
    assert_eq!(t.again(d, 7, &game), 1.0, "the first time it counts in full");
    t.note(d, 7, &game, true, "praise");
    assert_eq!(t.again(d, 7, &game), 0.0, "once in a talk");
    assert_eq!(t.again(d.add(1), 7, &game), 0.5, "half in the next talk");
    assert_eq!(t.again(d.add(1), 8, &game), 1.0, "another man hasn't heard it");
    t.note(d.add(1), 7, &game, true, "praise");
    assert_eq!(t.again(d.add(2), 7, &game), 0.25, "then a quarter");
    assert_eq!(t.times(d.add(2), 7, &game), 2);
    assert!(t.remembers(d.add(2), 7, &game));
    // Another game is another point.
    let next = Point::new(Kind::BigGame).on(d.add(7));
    assert_eq!(t.again(d.add(8), 7, &next), 1.0);
    // A point that didn't land adds nothing the next time.
    t.note(d.add(8), 7, &next, false, "praise");
    assert_eq!(t.again(d.add(9), 7, &next), 0.0);
    // A recruit's point repeats only when your proof was strong or elite,
    // and counts in full again when the fact got better.
    let fair = Point::new(Kind::Stadium).proof(0.4, Tier::Fair, 3);
    t.note(d, 9, &fair, true, "pitch");
    assert_eq!(t.again(d.add(3), 9, &fair), 0.0);
    let strong = Point::new(Kind::Stadium).proof(0.7, Tier::Strong, 3);
    assert_eq!(t.again(d.add(3), 9, &strong), 1.0, "the fact got better");
    t.note(d.add(3), 9, &strong, true, "pitch");
    assert_eq!(t.again(d.add(5), 9, &strong), 0.25, "its second repeat");
    // Kept for a season.
    let later = d.add(claims::KEEP_DAYS + 9);
    assert_eq!(t.again(later, 7, &game), 1.0);
    assert!(!t.remembers(later, 7, &game));
    assert_eq!(t.times(later, 7, &game), 0);

    // It saves by key, and a key a build doesn't know drops that row only.
    let bytes = rmp_serde::encode::to_vec_named(&t).unwrap();
    let back: Talked = rmp_serde::from_slice(&bytes).unwrap();
    assert_eq!(back, t);
    let old: Talked = serde_json::from_str(
        r#"[{"who":1,"kind":"no_such_point","on":10},{"who":1,"kind":"big_game","on":10,"times":2,"landed":true}]"#,
    )
    .unwrap();
    assert_eq!(old.rows().len(), 1);
    assert_eq!(old.rows()[0].kind, Kind::BigGame);
    t.forget(7);
    assert!(t.with(7).next().is_none() && t.with(9).next().is_some());

    // A point still to come is the fact of its day: two promises due the
    // same day are two points, and another due day is another.
    let mut t = Talked::default();
    let start = Point::new(Kind::PromiseOpen).due(d.add(30)).topic("start");
    let snaps = Point::new(Kind::PromiseOpen).due(d.add(30)).topic("snaps");
    let later = Point::new(Kind::PromiseOpen).due(d.add(40)).topic("start");
    t.note(d, 7, &start, true, "remind");
    assert_eq!(t.again(d.add(1), 7, &start), 0.5);
    assert_eq!(t.again(d.add(1), 7, &snaps), 1.0);
    assert_eq!(t.again(d.add(1), 7, &later), 1.0);
    // Said twice in one talk, it landed if it landed either time.
    t.note(d, 7, &start, false, "remind");
    assert_eq!(t.again(d.add(1), 7, &start), 0.5);
    assert_eq!(t.times(d.add(1), 7, &start), 1);
    // The topic is saved with it.
    let bytes = rmp_serde::encode::to_vec_named(&t).unwrap();
    let back: Talked = rmp_serde::from_slice(&bytes).unwrap();
    assert_eq!(back, t);
}

/// Every read of a hidden number in the module, by the name it is read
/// under. A new one fails this test until it is a gate, carries its
/// `// hidden:` mark and is added here.
const HIDDEN_READS: [(&str, usize); 26] = [
    (".gpa", 2),
    ("gpa_trend", 0),
    (".lean", 1),
    ("lean_events", 0),
    ("service_read", 0),
    ("commit_timing", 0),
    ("decision_day(", 0),
    ("draft_read", 1),
    ("spring_gain", 0),
    ("spring_gain_ahead", 1),
    (".train", 1),
    ("overall()", 0),
    ("potential", 0),
    ("ceiling", 0),
    ("ceilings", 0),
    ("original_potential", 0),
    ("gpa_at_signing", 0),
    ("own_wants", 1),
    ("decider_wants", 1),
    ("stay_home", 0),
    (".ambition", 0),
    (".ability", 0),
    ("visibility", 0),
    (".stability", 0),
    ("home_talks", 0),
    ("goals(", 1),
];

#[test]
fn every_read_of_a_hidden_number_is_a_marked_gate() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/world/folk/claims");
    let ident = |c: char| c.is_alphanumeric() || c == '_';
    // Whether `tok` is in the line as a whole name (`.train`, not
    // `.training`; `spring_gain`, not `spring_gain_ahead`).
    let has = |line: &str, tok: &str| {
        line.match_indices(tok).any(|(at, _)| {
            let after = line[at + tok.len()..].chars().next();
            let before = line[..at].chars().next_back();
            let open = tok.ends_with('(') || tok.ends_with(')');
            (open || !after.is_some_and(ident)) && (tok.starts_with('.') || !before.is_some_and(ident))
        })
    };
    let mut found = vec![0usize; HIDDEN_READS.len()];
    let mut files = 0;
    for entry in std::fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_none_or(|e| e != "rs") {
            continue;
        }
        files += 1;
        let text = std::fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = text.lines().collect();
        for (n, line) in lines.iter().enumerate() {
            let code = line.trim_start();
            if code.starts_with("//") {
                continue;
            }
            for (k, (tok, _)) in HIDDEN_READS.iter().enumerate() {
                if !has(code, tok) {
                    continue;
                }
                found[k] += 1;
                let marked = lines[n.saturating_sub(8)..=n].iter().any(|l| l.contains("// hidden:"));
                assert!(marked, "{}:{}: `{tok}` is read without a `// hidden:` mark", path.display(), n + 1);
            }
        }
    }
    assert!(files >= 10, "the module's files were not found at {}", dir.display());
    for (k, (tok, budget)) in HIDDEN_READS.iter().enumerate() {
        assert_eq!(found[k], *budget, "`{tok}`: a hidden read was added or removed (rule 2 of `claims/mod.rs`)");
    }
}

#[test]
fn no_hidden_number_reaches_a_value_on_a_players_sheet() {
    let (mut w, p, v) = room(6);
    let team = Team::College(p);
    let today = w.today;
    // Numbers that would stand out in any value: his grades, his lean.
    for (n, &i) in v.iter().enumerate() {
        w.people[i].bg.gpa = 2.4173 - 0.3 * (n % 2) as f64;
        w.people[i].cal.lean = 0.7391;
    }
    let id = w.people[v[0]].player.id;
    w.folk.courts.open.push(Case {
        id,
        level: Level::College,
        what: "a DUI arrest".into(),
        opened: today.add(-2),
        next: today.add(19),
        step: Step::Arrest,
        guilty: true,
        outcome: None,
    });
    let sheets = claims::roster(&w, team);
    assert!(sheets.len() >= v.len());
    for s in &sheets {
        well_formed(s);
        assert_eq!(s.team, Some(team));
        for (kind, hole, val) in readable(s) {
            for secret in ["2.41", "2.11", "4173", "0.73", "7391"] {
                assert!(!val.contains(secret), "{kind:?} {hole} shows a hidden number: {val}");
            }
        }
    }
    // The grades point says which side of the line he is on, and no more.
    let s = claims::player(&w, v[1]);
    let g = s.get(Kind::Grades).expect("under the line");
    assert_eq!(g.check("line"), Some("under"));
    assert!(g.values().is_empty());
    // His lean puts the portal on his mind, and you don't know it until he
    // asks or a whisper is out.
    let portal = s.get(Kind::Portal).expect("he leans toward the portal");
    assert!(!portal.known && portal.is("asked", "no"));
    assert!(s.known().all(|x| x.kind != Kind::Portal));

    // The arrest: what he opens on, with what only he knows kept off the
    // line.
    let s = claims::player(&w, v[0]);
    let top = s.top().unwrap();
    assert_eq!(top.kind, Kind::Arrest);
    assert!(s.trouble());
    assert_eq!(top.get("charge"), Some("a DUI arrest"));
    assert_eq!(top.check("step"), Some("arrest"));
    assert_eq!(top.hid("did"), Some("yes"));
    assert_eq!(top.when(), (When::Now, Some(today.add(19))));
    let f = top.facts(&voice::Facts::new());
    assert_eq!(f.get("point"), Some("arrest"));
    assert!(f.get("did").is_none() && !f.0.iter().any(|x| x.1 == "yes" && x.0 != "again"), "{f:?}");
    assert_eq!(s.basis(Kind::Arrest), Basis::OnMind);
    assert_eq!(s.basis(Kind::BigGame), Basis::Plain, "praise with no game behind it");
    assert!(!claims::player(&w, v[2]).trouble(), "a warning to a man who did nothing has nothing behind it");

    // What the world backs about his conduct: with the case open no line
    // may deny or admit, whatever only he knows; a clean man may deny.
    assert_eq!(s.conduct(), Conduct::Open);
    w.folk.courts.open[0].guilty = false;
    let innocent = claims::player(&w, v[0]);
    assert_eq!(innocent.conduct(), Conduct::Open);
    let mut said = voice::Facts::new();
    innocent.fill_conduct(&mut said);
    assert!(said.get("at_fault").is_none() && said.get("blameless").is_none(), "{said:?}");
    let clean = claims::player(&w, v[2]);
    assert_eq!(clean.conduct(), Conduct::Blameless);
    clean.fill_conduct(&mut said);
    assert_eq!((said.get("blameless"), said.get("at_fault")), (Some("yes"), None));

    // Talked through once, it weighs half as much on a sheet for your side.
    let before = top.mind;
    let arrest = top.clone();
    w.folk.talked.note(today, id, &arrest, true, "ask");
    w.today = today.add(1);
    let after = claims::player(&w, v[0]).get(Kind::Arrest).unwrap().mind;
    assert!((after - before * sheet::TALKED_FADE).abs() < 1e-5, "{before} then {after}");

    // The court day brings it out: a conviction is his fault, an acquittal
    // is not held against him.
    let mut case = w.folk.courts.open.remove(0);
    case.next = w.today;
    case.outcome = Some("He was convicted of a DUI.".into());
    w.folk.courts.closed.push(case);
    let s = claims::player(&w, v[0]);
    assert!(s.get(Kind::CaseClosed).is_some_and(|c| c.is("outcome", "convicted")));
    assert_eq!(s.conduct(), Conduct::AtFault);
    let mut said = voice::Facts::new();
    s.fill_conduct(&mut said);
    assert_eq!((said.get("at_fault"), said.get("blameless")), (Some("yes"), None));
    w.folk.courts.closed.last_mut().unwrap().outcome = Some("He was acquitted.".into());
    assert_eq!(claims::player(&w, v[0]).conduct(), Conduct::Blameless);
}

#[test]
fn a_game_is_named_for_a_week_and_never_after() {
    let (mut w, p, _) = room(2);
    let team = Team::College(p);
    let opp = (0..w.programs.len()).find(|&j| j != p).unwrap();
    let day = w.today.add(-2);
    let stats = GameStats { score: [24, 17], ..Default::default() };
    w.seat_games.push(SeatGame {
        date: day,
        level: Level::College,
        teams: [p, opp],
        names: [w.program_name(p).to_string(), w.program_name(opp).to_string()],
        stats,
        replay: Vec::new(),
        marks: Vec::new(),
        gone: Vec::new(),
        spread: None,
        talks: Default::default(),
        whistle: None,
    });
    let s = claims::team(&w, team);
    well_formed(&s);
    let g = s.get(Kind::LastGame).expect("the game two days ago");
    assert_eq!(g.get("score"), Some("24-17"));
    assert_eq!(g.get("opp"), Some(w.program_name(opp)));
    assert!(g.is("result", "win") && g.is("margin", "close") && g.is("place", "home"));
    assert_eq!(g.day, Some(day));
    // A recruit you call hears of the same win.
    w.add_class_aged(17);
    let kid = (0..w.people.len()).find(|&i| w.people[i].stage == Stage::HighSchool).unwrap();
    let call = claims::recruit(&w, kid, p, Ear::Himself);
    assert_eq!(call.get(Kind::YourWin).and_then(|x| x.get("score")), Some("24-17"));
    assert!(!call.has(Kind::YourLoss));
    // Eight days on, nobody names it (rule 9).
    w.today = day.add(kinds::GAME_DAYS + 1);
    let s = claims::team(&w, team);
    assert!(!s.has(Kind::LastGame) && !s.has(Kind::Streak));
    assert!(!claims::recruit(&w, kid, p, Ear::Himself).has(Kind::YourWin));
}

#[test]
fn a_recruits_sheet_grades_your_points_and_names_what_changed() {
    let (mut w, p, others, v) = recruits(3);
    let today = w.today;
    let i = v[0];
    w.people[i].cal.early_offers.push((others[0], today.add(-3)));
    w.people[i].cal.early_offers.push((others[1], today.add(-40)));
    w.people[i].cal.visits.push((p, today.add(-5), Visit::Official));
    w.people[i].cal.visits.push((others[1], today.add(-6), Visit::Unofficial));
    w.people[i].cal.visits.push((p, today.add(10), Visit::Unofficial));
    let s = claims::recruit(&w, i, p, Ear::Himself);
    well_formed(&s);
    assert_eq!(s.about, Some(w.people[i].player.id));

    // What changed: the newest offer by name, his visits, the one ahead.
    let offer = s.get(Kind::NewOffer).expect("an offer three days ago");
    assert_eq!(offer.get("school"), Some(w.program_name(others[0])));
    assert_eq!(offer.get("offers"), Some("1"), "the offer from six weeks ago is no longer new");
    assert!(s.get(Kind::YourVisit).is_some_and(|x| x.is("visit", "official")));
    assert_eq!(s.get(Kind::VisitElsewhere).and_then(|x| x.get("school")), Some(w.program_name(others[1])));
    let ahead = s.get(Kind::HisVisit).expect("his visit to you is set");
    assert_eq!(ahead.when(), (When::Day, Some(today.add(10))));
    assert_eq!(ahead.get("days"), Some("10"));

    // Your points: every one of yours carries your proof, graded against
    // the other schools on his list, and a group counts at its best.
    let mut pitches = 0;
    for pt in s.points.iter().filter(|x| matches!(x.kind.family(), Family::Pitch(_))) {
        pitches += 1;
        if pt.whose == Whose::His {
            assert!(pt.proof.is_none());
            continue;
        }
        let proof = pt.proof.unwrap_or_else(|| panic!("{:?} has no proof", pt.kind));
        assert!(proof.against as usize <= crate::world::front_office::race::SCHOOLS);
        assert_eq!(pt.tier(), Some(proof.tier));
    }
    assert!(pitches >= 10, "only {pitches} points to make");
    for g in Group::ALL {
        let Some(best) = s.best(g) else { continue };
        assert!(s.group(g).filter_map(|x| x.tier()).all(|t| t <= best.tier().unwrap()), "{}", g.name());
    }
    assert!(s.best(Group::Winning).is_some() && s.best(Group::School).is_some());
    assert!(s.yours().all(|x| x.whose != Whose::His));
    // Your proof is not what is on his mind: he opens on something of his
    // own or shared.
    assert!(s.top().is_some_and(|x| x.whose != Whose::Yours));

    // Who listens changes how much a point weighs, never the fact.
    w.people[i].family.decider = Decider::Parent;
    w.people[i].family.own_wants.prestige = 0.02;
    w.people[i].family.decider_wants.prestige = 0.9;
    let own = claims::recruit(&w, i, p, Ear::Himself);
    let home = claims::recruit(&w, i, p, Ear::Decider);
    let (a, b) = (own.get(Kind::Stadium).unwrap(), home.get(Kind::Stadium).unwrap());
    assert!(b.care > a.care, "the parent cares more about the name: {} and {}", a.care, b.care);
    assert_eq!(a.proof.map(|x| x.tier), b.proof.map(|x| x.tier));
    assert_eq!(a.values(), b.values());

    // Three weeks on, the offer and the visits are old news.
    w.today = today.add(kinds::TEST_DAYS + 1);
    let later = claims::recruit(&w, i, p, Ear::Himself);
    well_formed(&later);
    assert!(!later.has(Kind::NewOffer) && !later.has(Kind::VisitElsewhere));
    // The visit that was ahead has been taken: it is the one to talk about.
    assert!(!later.has(Kind::HisVisit));
    assert!(later.get(Kind::YourVisit).is_some_and(|x| x.is("visit", "unofficial")));

    // A sheet for another school reads the same world: the same offer is
    // news there too, and its own proof is graded.
    let theirs = claims::recruit(&w, v[1], others[0], Ear::Himself);
    well_formed(&theirs);
    assert_eq!(theirs.team, Some(Team::College(others[0])));
}

#[test]
fn the_alumni_index_lists_a_programs_picks_by_position_and_coach() {
    let mut w = seat("nfl");
    let year = w.year;
    let prog = 3;
    let pros: Vec<usize> = (0..w.people.len()).filter(|&i| w.people[i].pro.is_some()).take(4).collect();
    assert_eq!(pros.len(), 4, "the club world has pros");
    for (k, &i) in pros.iter().enumerate() {
        let q = &mut w.people[i];
        q.path = vec![prog];
        q.college_staff = vec![(year - 3, prog, Some(7), Some(9))];
        q.pro.as_mut().unwrap().draft = Some((year - 1 - k as i32, 1 + k as u32, 5 + 32 * k as u32, 0));
    }
    let ix = Alumni::get(&w);
    assert!(std::sync::Arc::ptr_eq(&ix, &Alumni::get(&w)), "built once");
    let ids: Vec<u32> = pros.iter().map(|&i| w.people[i].player.id).collect();
    let mine: Vec<_> = ix.of(prog).filter(|a| a.id.is_some_and(|id| ids.contains(&id))).collect();
    assert_eq!(mine.len(), 4);
    assert!(mine.windows(2).all(|x| x[0].year >= x[1].year), "newest draft first");
    for a in &mine {
        assert!(a.active && a.coached_by(7) && a.coached_by(9) && !a.coached_by(8));
        assert!(ix.at(prog, Room::of(a.position)).any(|b| b.id == a.id));
        assert!(ix.recent(prog, a.position, year).any(|b| b.id == a.id));
        assert!(!ix.recent(prog, a.position, a.year - 1).any(|b| b.id == a.id), "not before he was drafted");
        assert!(ix.class(prog, a.year).any(|b| b.id == a.id));
        assert!(!a.club.is_empty() && a.round >= 1);
    }
    assert_eq!(ix.coached(9).filter(|a| a.id.is_some_and(|id| ids.contains(&id))).count(), 4);
    // Everyone still playing is listed once.
    let listed: Vec<u32> = ix.all().iter().filter_map(|a| a.id).collect();
    assert_eq!(listed.len(), listed.iter().collect::<HashSet<_>>().len());
    // A man who retired stays among the people until the world tidies
    // them: once his career is on the alumni list he is counted there
    // alone, as retired.
    let gone = pros[0];
    let (gone_year, gone_pick) = w.people[gone].pro.as_ref().and_then(|c| c.draft).map(|d| (d.0, d.2)).unwrap();
    let mut row = crate::world::Alumnus::from_person(&w.people[gone], gone_year, Some(gone_pick), true);
    row.colleges = vec![prog];
    row.pro = w.people[gone].pro.clone();
    w.alumni.push(row);
    let k = w.alumni.len() - 1;
    w.people[gone].pro.as_mut().unwrap().alumnus = Some(k);
    w.people[gone].stage = Stage::Done;
    let name = w.people[gone].name.clone();
    let ix = Alumni::get(&w);
    let his: Vec<_> = ix.of(prog).filter(|a| a.name == name && a.year == gone_year).collect();
    assert_eq!(his.len(), 1, "counted once");
    assert!(!his[0].active && his[0].id.is_none());
    assert_eq!(ix.coached(9).filter(|a| a.name == name).count(), 1);
    // A new year builds it again.
    w.year += 1;
    assert!(!std::sync::Arc::ptr_eq(&ix, &Alumni::get(&w)));
}

#[test]
fn the_shared_scene_names_leave_plain_picks_and_old_saves_as_they_were() {
    let (w, _, _) = room(1);
    let mut s = Scene::new(scene::Kind::OneOnOne, w.today, "A word");
    s.pick(1, "\"How are you holding up?\"", "Ask him", "warm", "He opens up.");
    // A plain pick and a scene with nothing behind it are written as before.
    let plain = serde_json::to_value(&s.picks[0]).unwrap();
    assert!(plain.get("point").is_none() && plain.get("kind").is_none(), "{plain}");
    assert!(serde_json::to_value(&s).unwrap().get("reach").is_none());
    // An old save's pick reads as a plain one.
    let old: Pick = serde_json::from_str(r#"{"label":"a","mv":"m","tone":"t","hint":"h","code":3}"#).unwrap();
    assert!(old.point.is_empty() && old.kind == PickKind::Plain && old.code == 3);
    // The three names.
    let on = s.picks[0].clone().on(Kind::Arrest.key()).of(PickKind::Final);
    let v = serde_json::to_value(&on).unwrap();
    assert_eq!(v["point"], "arrest");
    assert_eq!(v["kind"], "final");
    assert_eq!(Kind::from_key(&on.point), Some(Kind::Arrest));
    s.reach.push(
        Pick { label: "The stadium".into(), code: 9, ..Pick::default() }.on(Kind::Stadium.key()).of(PickKind::Free),
    );
    s.reach.push(Pick { label: "Something else".into(), code: 10, ..Pick::default() }.of(PickKind::Nav));
    let bytes = rmp_serde::encode::to_vec_named(&s).unwrap();
    let back: Scene = rmp_serde::from_slice(&bytes).unwrap();
    assert_eq!(back.reach.len(), 2);
    assert_eq!((back.reach[0].point.as_str(), back.reach[0].kind), ("stadium", PickKind::Free));
    assert_eq!(back.reach[1].kind, PickKind::Nav);
    assert_eq!(back.picks[0].kind, PickKind::Plain);
    // Ending a talk clears both lists.
    s.end();
    assert!(s.picks.is_empty() && s.reach.is_empty());
    // The store of what you have talked through writes nothing until used.
    let folk = serde_json::to_value(&w.folk).unwrap();
    assert!(folk.get("talked").is_none() && folk.get("alumni").is_none());
}
