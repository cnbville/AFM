//! Your team's rooms read the facts (talk facts lane L9, items 18, 20, 21
//! and 26; `world/folk/claims/team_rooms.rs`): the team talk opens on the
//! real week and words each option for it, its reaction names who really
//! moved; a parent calls about the real complaint with what the family can
//! see; an exit meeting opens on why he is on your list; and the stakes, a
//! starter out, the playoffs and seniors' night reach your game day words.
//! Words and options only: every code and every effect is the same.

use super::legacy::seat;
use crate::calendar::Level;
use crate::game::stats::{GameStats, StatLine};
use crate::game::talk::{Move as GMove, Pick as GPick, Subject as GSubject, Tone as GTone};
use crate::player::{PlayingTime, Trait};
use crate::positions::Position;
use crate::training::seat::{GROUP_ONES, GROUP_TWOS};
use crate::world::folk::claims::team_rooms::{self, Complaint, Leaving};
use crate::world::folk::claims::{self, Kind as K};
use crate::world::folk::gametalk::{self, LineCx};
use crate::world::folk::rules::{Response, RuleKind, Ruling};
use crate::world::folk::scene::{self, Kind, Scene};
use crate::world::folk::talk_points::Topic;
use crate::world::folk::talks::{self, Subject};
use crate::world::folk::voice::bank;
use crate::world::folk::{Team, takeover};
use crate::world::seat::SeatGame;
use crate::world::{Stage, World};

/// A name no other school has, so a line that names the game is found.
const OPP: &str = "Zzyzx Tech";

/// Your college program with `n` level-headed sophomore backups on it.
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
        q.player.position = Position::WR;
        q.trust = 0.5;
        q.mood = 0.0;
        q.mood_why = 0;
        q.cal.lean = 0.0;
        q.bg.stability = 10.0;
        for t in Trait::ALL {
            q.player.personality.traits.set(t, 10.0);
        }
        w.programs[p].roster.push(i);
    }
    let opp = (0..w.programs.len()).find(|&q| q != p).unwrap();
    w.programs[opp].real_name = OPP.into();
    w.programs[opp].fictional_name = OPP.into();
    (w, p, v)
}

/// One of your games `ago` days back against `OPP`: the score, and each
/// man's snaps on offense.
fn game(w: &mut World, p: usize, ago: i32, score: [u32; 2], snaps: &[(u32, u32)]) {
    let opp = (0..w.programs.len()).find(|&q| q != p).unwrap();
    let mut stats = GameStats { score: [score[0] as i32, score[1] as i32], ..Default::default() };
    for &(id, n) in snaps {
        stats.players[0].insert(id, StatLine { snaps_off: n, ..Default::default() });
    }
    w.seat_games.push(SeatGame {
        date: w.today.add(-ago),
        level: Level::College,
        teams: [p, opp],
        names: [w.program_name(p).to_string(), OPP.to_string()],
        stats,
        replay: Vec::new(),
        marks: Vec::new(),
        gone: Vec::new(),
        spread: None,
        talks: Default::default(),
        whistle: None,
    });
    w.seat_games.sort_by_key(|g| g.date);
}

fn id(w: &World, i: usize) -> u32 {
    w.people[i].player.id
}

fn text(s: &Scene) -> String {
    s.lines.iter().map(|l| l.text.clone()).collect::<Vec<_>>().join("\n")
}

fn label(s: &Scene, code: u32) -> String {
    s.picks.iter().find(|p| p.code == code).map(|p| p.label.clone()).unwrap_or_default()
}

fn codes(s: &Scene) -> Vec<u32> {
    s.picks.iter().map(|p| p.code).collect()
}

/// Whether `line` holds one of the bank's versions of `key`, with its
/// holes filled the way the room fills them.
fn has_version(key: &str, line: &str, holes: &[(&str, &str)]) -> bool {
    bank().versions(key).into_iter().any(|v| {
        let mut t = v.0.to_string();
        for (hole, value) in holes {
            t = t.replace(hole, value);
        }
        let t = t.trim_end_matches(['.', '!', '?']).to_lowercase();
        !t.is_empty() && line.to_lowercase().contains(&t)
    })
}

fn team_talk(w: &mut World) -> Scene {
    w.folk.mine.hours = 0.0;
    let sid = talks::start_team_talk(w).expect("a team talk");
    scene::open(w, sid).expect("it opens")
}

const WEEKDAYS: [&str; 7] = ["Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday"];

// ---------------------------------------------------------------------------
// The team talk (item 18).

#[test]
fn the_team_talk_opens_on_the_real_day_and_words_its_options_for_the_win() {
    let (mut w, p, _) = room(3);
    game(&mut w, p, 2, [31, 17], &[]);
    let s = team_talk(&mut w);
    let t = text(&s);
    let day = WEEKDAYS[w.today.weekday().index() as usize];
    assert!(t.starts_with(&format!("{day}, two days after the 31-17 win over")), "{t}");
    assert!(t.contains(OPP) && t.contains("somebody has music on"), "{t}");
    assert_eq!(codes(&s), [200, 201, 202, 203, 204], "every code is on the table");
    assert!(label(&s, 204).contains(&format!("That 31-17 win over {OPP} was yours")), "{}", label(&s, 204));
    assert!(label(&s, 203).contains(&format!("We beat {OPP}")), "{}", label(&s, 203));
}

#[test]
fn a_close_loss_and_a_losing_streak_reach_the_room() {
    let (mut w, p, _) = room(3);
    for ago in [16, 9, 2] {
        game(&mut w, p, ago, [17, 24], &[]);
    }
    let s = team_talk(&mut w);
    let t = text(&s);
    assert!(t.contains("two days after the 17-24 loss to"), "{t}");
    assert!(t.contains("still replaying the last drive"), "a close loss: {t}");
    assert!(t.contains("That's three straight losses."), "{t}");
    assert!(label(&s, 203).contains(&format!("17-24 against {OPP}")), "{}", label(&s, 203));
    assert!(label(&s, 201).contains("Three straight doesn't define us"), "{}", label(&s, 201));
    let praise = s.picks.iter().find(|p| p.code == 204).unwrap();
    assert!(praise.label.contains("17-24 or not"), "{}", praise.label);
    assert!(praise.hint.contains("hollow"), "the hint says praise after a loss rings hollow: {}", praise.hint);
}

#[test]
fn starters_out_hurt_are_named_and_the_calm_option_says_who() {
    let (mut w, p, v) = room(3);
    game(&mut w, p, 2, [17, 24], &[]);
    let today = w.today;
    w.people[v[0]].last_role = Some(PlayingTime::Starter);
    w.training.hurt.push(crate::training::seat::Hurt {
        id: id(&w, v[0]),
        kind: None,
        from: today.add(-3),
        until: today.add(20),
        how: "game".into(),
    });
    let name = w.people[v[0]].name.clone();
    let s = team_talk(&mut w);
    assert!(text(&s).contains(&format!("Out hurt: {name}.")), "{}", text(&s));
    assert!(label(&s, 201).contains(&format!("We're without {name}")), "{}", label(&s, 201));
}

#[test]
fn with_no_game_near_the_room_names_no_score() {
    let (mut w, p, _) = room(2);
    game(&mut w, p, 200, [17, 52], &[]);
    let s = team_talk(&mut w);
    let t = text(&s);
    assert!(t.contains("No game to talk about."), "{t}");
    assert!(!t.contains("17-52") && !t.contains("after the"), "{t}");
    assert!(!label(&s, 203).contains("17-52") && !label(&s, 204).contains("17-52"));
    assert_eq!(codes(&s), [200, 201, 202, 203, 204]);
}

#[test]
fn the_reaction_names_the_men_who_really_moved() {
    for code in [200, 201, 202, 203, 204] {
        let (mut w, p, v) = room(8);
        game(&mut w, p, 1, [28, 10], &[]);
        // Mixed men, so the room splits.
        for (k, &i) in v.iter().enumerate() {
            let t = &mut w.people[i].player.personality.traits;
            t.set(Trait::Ego, 4.0 + 2.0 * k as f64);
            t.set(Trait::Ambition, 18.0 - 2.0 * k as f64);
            if k % 2 == 0 {
                w.people[i].last_role = Some(PlayingTime::Starter);
            }
        }
        let s = team_talk(&mut w);
        let before: Vec<(usize, f32)> = v.iter().map(|&i| (i, w.people[i].trust)).collect();
        let s = scene::answer(&mut w, s.id, code).unwrap();
        let t = text(&s);
        let gain = |i: usize| w.people[i].trust - before.iter().find(|x| x.0 == i).unwrap().1;
        let named: Vec<usize> = v.iter().copied().filter(|&i| t.contains(&format!("[[p:{}|", id(&w, i)))).collect();
        for i in named {
            let g = gain(i);
            assert!(g.abs() > 0.02, "{code}: a man named in the reaction moved ({g}): {t}");
            if g > 0.0 {
                assert!(v.iter().all(|&j| gain(j) <= g), "{code}: the man named first up rose most: {t}");
            } else {
                assert!(v.iter().all(|&j| gain(j) >= g), "{code}: the man named cold fell most: {t}");
            }
        }
    }
}

// ---------------------------------------------------------------------------
// The first team meeting (item 18).

#[test]
fn the_first_team_meeting_names_the_season_before_you() {
    let mut w = seat("nfl");
    let team = Team::seat(&w).unwrap();
    let mut s = Scene::new(Kind::TeamMeeting, w.today, "Your first team meeting");
    s.stops = false;
    s.team = Some(team);
    let sid = scene::add(&mut w, s);
    let s = scene::open(&mut w, sid).unwrap();
    assert_eq!(codes(&s), [10, 11, 12, 13, 14], "every message is on the table");
    let arr = team_rooms::arrival(&w, team);
    if let (Some(y), Some(r)) = (arr.year, &arr.record) {
        assert!(text(&s).contains(&format!("The {y} team went {r}")), "{}", text(&s));
        assert!(label(&s, 12).contains(r.as_str()), "{}", label(&s, 12));
    }
    if let Some(what) = arr.ahead.as_ref().and_then(|a| a.get("what")) {
        assert!(label(&s, 14).contains(what), "{}", label(&s, 14));
    }
    let before: Vec<(usize, f32)> = team.roster(&w).into_iter().map(|i| (i, w.people[i].trust)).collect();
    let s = scene::answer(&mut w, sid, 10).unwrap();
    let (best, _) = talks::movers(&w, &before);
    if let Some(b) = best {
        assert!(text(&s).contains(&format!("[[p:{}|", id(&w, b))), "the man it moved most is named: {}", text(&s));
    }
}

// ---------------------------------------------------------------------------
// A parent's call (item 20).

fn parent_call(w: &mut World, who: u32) -> Scene {
    let mut s = Scene::new(Kind::ParentCall, w.today, "His family calls");
    s.a = who as u64;
    s.stops = false;
    s.team = Team::seat(w);
    let sid = scene::add(w, s);
    scene::open(w, sid).unwrap()
}

fn rule_broken(w: &mut World, i: usize, response: Response) {
    let today = w.today;
    let who = id(w, i);
    w.folk.rules.rulings.push(Ruling { date: today.add(-3), id: who, kind: RuleKind::Curfew, response, star: false });
}

#[test]
fn a_family_calls_about_a_punishment_and_never_about_snaps() {
    let (mut w, _, v) = room(1);
    w.people[v[0]].family.parent_style = crate::world::background::ParentStyle::LoudAtGames;
    rule_broken(&mut w, v[0], Response::Running);
    let fam = team_rooms::family(&w, v[0]).expect("a complaint");
    assert_eq!(fam.complaint, Complaint::Punishment);
    assert_eq!(fam.penalty.as_deref(), Some("the early-morning running"));
    let pid = id(&w, v[0]);
    let s = parent_call(&mut w, pid);
    let call = s.lines.iter().find(|l| l.by == 2).map(|l| l.text.to_lowercase()).unwrap();
    for word in ["snaps", "playing time", "on the field", "isn't playing", "bench"] {
        assert!(!call.contains(word), "a punishment call says nothing of {word}: {call}");
    }
    assert!(label(&s, 300).contains("our rule on curfew, and the early-morning running stands"), "{}", label(&s, 300));
    assert!(label(&s, 303).contains("Team discipline"), "{}", label(&s, 303));
    assert_eq!(codes(&s), [300, 301, 303], "no promise of snaps to a man who never started");
    // His answer is to what you said about the punishment.
    let s = scene::answer(&mut w, s.id, 301).unwrap();
    let reply = s.lines.iter().rev().find(|l| l.by == 2).unwrap().text.clone();
    let name = w.people[v[0]].name.split_whitespace().next().unwrap().to_string();
    assert!(
        has_version("parents.reply.listen", &reply, &[("{name}", &name)]),
        "the reply is the bank's listen part: {reply}"
    );
    assert!(!reply.contains("position coach"), "{reply}");
}

#[test]
fn a_warning_is_nothing_a_family_calls_about() {
    let (mut w, _, v) = room(1);
    rule_broken(&mut w, v[0], Response::Talk);
    assert!(team_rooms::family(&w, v[0]).is_none());
}

#[test]
fn a_role_call_says_what_the_family_can_see() {
    let (mut w, p, v) = room(2);
    let (a, b) = (id(&w, v[0]), id(&w, v[1]));
    w.people[v[0]].mood_why = crate::world::people::Why::Role as u8;
    let today = w.today;
    for day in [-10, -3] {
        w.training.groups.push((today.add(day), vec![(a, GROUP_TWOS), (b, GROUP_ONES)]));
    }
    game(&mut w, p, 2, [21, 14], &[(a, 5), (b, 60)]);
    let fam = team_rooms::family(&w, v[0]).expect("his role");
    assert_eq!(fam.complaint, Complaint::Role);
    assert_eq!(
        fam.sees().as_deref(),
        Some("with the second team in practice, as he tells it; five snaps and no starts in his last game.")
    );
    let s = parent_call(&mut w, a);
    assert!(text(&s).contains("What the family can see: with the second team"), "{}", text(&s));
    assert_eq!(codes(&s), [300, 301, 302, 303]);
    assert!(label(&s, 300).contains("He's working with the second team"), "{}", label(&s, 300));
}

#[test]
fn a_position_move_is_its_own_complaint() {
    let (mut w, _, v) = room(1);
    let who = id(&w, v[0]);
    let mut plan = w.training.plan_of(who);
    plan.moving = Some((Position::ALL.iter().position(|&x| x == Position::CB).unwrap() as u8, 10));
    w.training.set_plan(plan);
    let fam = team_rooms::family(&w, v[0]).expect("the move");
    assert_eq!(fam.complaint, Complaint::Position);
    let s = parent_call(&mut w, who);
    assert!(label(&s, 300).contains("We moved him to corner"), "{}", label(&s, 300));
    assert!(label(&s, 302).contains("snaps at corner"), "{}", label(&s, 302));
}

#[test]
fn a_loud_family_calls_about_a_punishment_on_the_mondays() {
    // A man sore about a ruling: no role complaint behind his mood.
    let calls = |ruled: bool| {
        let (mut w, _, v) = room(1);
        w.people[v[0]].family.parent_style = crate::world::background::ParentStyle::LoudAtGames;
        let mut monday = w.today;
        while monday.weekday() != crate::calendar::Weekday::Mon {
            monday = monday.add(1);
        }
        for k in 0..20 {
            let d = monday.add(7 * k);
            w.today = d;
            w.people[v[0]].mood = -0.5;
            w.people[v[0]].mood_why = 0;
            if ruled {
                w.folk.rules.rulings.retain(|_| false);
                rule_broken(&mut w, v[0], Response::Running);
            }
            talks::day(&mut w, d);
        }
        w.folk.scenes.iter().filter(|s| s.kind == Kind::ParentCall).count()
    };
    assert!(calls(true) > 0, "a family calls about the running");
    assert_eq!(calls(false), 0, "nothing the family can see: no call");
}

// ---------------------------------------------------------------------------
// Exit meetings (item 21).

fn exit_meeting(w: &mut World, i: usize, staff: Option<&str>) -> Scene {
    let mut s = Scene::new(Kind::OneOnOne, w.today, format!("Exit meeting: {}", w.people[i].name));
    s.a = id(w, i) as u64;
    s.stops = false;
    s.place = "Your office".into();
    talks::exit_reason(w, &mut s, i, staff);
    let sid = scene::add(w, s);
    scene::open(w, sid).unwrap()
}

#[test]
fn a_takeover_exit_meeting_opens_on_the_old_coach() {
    let (mut w, _, v) = room(1);
    let old = (0..w.coaches.len()).find(|&c| w.coaches[c].name.split_whitespace().count() >= 2).unwrap();
    let why = "close to the old coach, other programs are already calling";
    w.folk.takeover = Some(takeover::Takeover {
        old_coach: Some(old),
        started: Some(w.today),
        risks: vec![(id(&w, v[0]), why.into())],
        ..Default::default()
    });
    assert_eq!(team_rooms::leaving(&w, v[0], Some(why)), Leaving::OldCoach);
    let s = exit_meeting(&mut w, v[0], Some(why));
    let t = text(&s);
    assert!(t.contains(&format!("Your staff's note on him: {why}.")), "{t}");
    let last = w.coaches[old].name.split_whitespace().last().unwrap().to_string();
    let his = s.lines.iter().find(|l| l.by == 2).unwrap().text.clone();
    assert!(
        has_version("talk.exit.open.oldcoach", &his, &[("{old_last}", &last)]),
        "his opening is about the old coach: {his}"
    );
}

#[test]
fn an_unhappy_man_opens_on_it_and_a_draft_read_opens_on_the_draft() {
    let (mut w, _, v) = room(2);
    w.people[v[0]].mood = -0.5;
    assert_eq!(team_rooms::leaving(&w, v[0], None), Leaving::Unhappy);
    let s = exit_meeting(&mut w, v[0], None);
    assert!(text(&s).contains("Your staff's note on him: hasn't been happy for a while."), "{}", text(&s));
    let his = s.lines.iter().find(|l| l.by == 2).unwrap().text.clone();
    assert!(has_version("talk.exit.open.unhappy", &his, &[]), "{his}");

    w.people[v[1]].cal.draft_read = Some(crate::world::DraftRead { grade: 70.0, score: 70.0, round: 2 });
    let sheet = claims::player(&w, v[1]);
    if sheet.known().any(|p| p.kind == K::Draft) {
        assert_eq!(team_rooms::leaving(&w, v[1], None), Leaving::Point(K::Draft));
        let s = exit_meeting(&mut w, v[1], None);
        assert_eq!(Topic::of(s.b), Topic::Point(K::Draft), "the talk is about the draft");
        assert!(text(&s).contains("Your staff's note on him: the draft is on his mind."), "{}", text(&s));
    }
}

#[test]
fn a_plain_exit_meeting_stays_on_his_future() {
    let (mut w, _, v) = room(1);
    assert_eq!(team_rooms::leaving(&w, v[0], None), Leaving::Plain);
    let s = exit_meeting(&mut w, v[0], None);
    assert_eq!(Topic::of(s.b), Topic::Subject(Subject::Future));
    assert!(text(&s).contains("his plans for next year are open"), "{}", text(&s));
}

// ---------------------------------------------------------------------------
// Game day words (item 26).

fn cx(seed: u64) -> LineCx {
    LineCx { them: OPP.into(), seed, ..Default::default() }
}

#[test]
fn the_day_reaches_your_game_day_words() {
    let (w, _, _) = room(1);
    let pick = GPick::new(GMove::Straight, GTone::Firm, GSubject::Us);
    for seed in 0..12 {
        // A starter out.
        let c = LineCx { hurt: Some("Marcus".into()), ..cx(seed) };
        assert_eq!(c.day_part(), Some("gameday.day.hurt"));
        let line = gametalk::talk_line(&w, &pick, &c);
        assert!(line.contains("Marcus"), "{seed}: {line}");
        // Seniors' night before the game, not at halftime.
        let c = LineCx { seniors: true, ..cx(seed) };
        let line = gametalk::talk_line(&w, &pick, &c);
        assert!(has_version("gameday.day.seniors", &line, &[("{them}", OPP)]), "{seed}: {line}");
        let half = LineCx { seniors: true, halftime: Some(-3), ..cx(seed) };
        assert_eq!(half.day_part(), None, "seniors' night is a pregame word");
        // The playoffs come first.
        let c = LineCx { playoff: true, seniors: true, hurt: Some("Marcus".into()), ..cx(seed) };
        assert_eq!(c.day_part(), Some("gameday.day.playoff"));
        let line = gametalk::talk_line(&w, &pick, &c);
        assert!(has_version("gameday.day.playoff", &line, &[]), "{seed}: {line}");
        // Big stakes.
        let c = LineCx { stakes: 0.9, ..cx(seed) };
        let line = gametalk::talk_line(&w, &pick, &c);
        assert!(has_version("gameday.day.big", &line, &[("{them}", OPP)]), "{seed}: {line}");
        // A regular week says none of it.
        assert_eq!(cx(seed).day_part(), None);
        assert_eq!(LineCx { stakes: 0.4, ..cx(seed) }.day_part(), None);
    }
    // A talk about the man who is out names him once, not twice.
    let c = LineCx { hurt: Some("Marcus".into()), player: Some("Marcus".into()), ..cx(1) };
    assert_eq!(c.day_part(), None);
}

#[test]
fn a_captain_names_the_starter_out() {
    let (w, _, v) = room(1);
    let c = LineCx { hurt: Some("Marcus".into()), ..cx(7) };
    let said = gametalk::captain_lines(&w, v[0], &c);
    assert!(said.iter().any(|l| l.contains("Marcus")), "{said:?}");
    let plain = gametalk::captain_lines(&w, v[0], &cx(7));
    assert!(plain.iter().all(|l| !has_version("gameday.captain.day", l, &[])), "{plain:?}");
}
