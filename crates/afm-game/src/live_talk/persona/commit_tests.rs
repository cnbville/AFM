//! What you commit to the people you answer to (Cain 2026-11-16: "it also
//! says take on more, so why isn't he like 'Okay so you promise to recruit
//! the #1'"). A commitment your boss can judge goes on his goals for the
//! season, in place of his own when it asks more; one below his bar or
//! already on his list changes nothing, one he can't measure gets his ask
//! for a number. "Heard as" says what he did with it and how the line
//! went, on his own scale. A dodge needs a question of his on his last
//! line. An offer to your boss is never a player's deal. Asked about his
//! own job, a coach makes his case. A promise to a parent or a stay to
//! your staff is kept, not dropped.

use afm_engine::world::folk::boss::{self, GoalKind};
use afm_engine::world::folk::persona::Them;
use afm_engine::world::folk::persona::talk::stakes::{self, Holdover};
use afm_engine::world::folk::persona::talk::store;
use afm_engine::world::folk::scene::{self, Kind, Scene};
use afm_engine::world::folk::talks::{PromiseKind, PromiseTo};
use afm_engine::world::folk::{Team, commit, facts, takeover};
use afm_engine::world::ground::campaign::goals;
use serde_json::{Value, json};

use super::super::{ear::EAR, voice::VOICE};
use super::tests::{act, ear, game, ok, say, scene_of, voice};
use crate::Game;

/// Your first meeting with the athletic director: his goals for the
/// season, on the sheet.
fn goals_meeting(g: &mut Game) -> u32 {
    let w = g.world.as_mut().unwrap();
    let mut s = Scene::new(Kind::BossFirst, w.today, "Meeting the athletic director");
    s.team = Team::seat(w);
    s.stops = false;
    s.place = "His office".into();
    let sid = scene::add(w, s);
    ok(g, "people_scene", json!({ "id": sid }));
    sid
}

/// A commit act as the ear gives it.
fn commit_act(key: &str, n: &str, every: bool, words: &str) -> Value {
    let mut a = act("commit", key, "none", "");
    a["value"] = json!(n);
    a["when"] = json!(if every { "every_year" } else { "" });
    a["words"] = json!(words);
    a
}

/// The ear's reply with the dodge flag set.
fn dodging(acts: Vec<Value>) -> Value {
    let v = json!({ "acts": acts, "hits": [], "dodged": true, "not_talk": false });
    afm_ai::fake::body(&v.to_string(), 1200, 120)
}

/// The voice's card text for the last line said.
fn last_voice(f: &afm_ai::Fake) -> String {
    f.asked().into_iter().rev().find(|r| r.label == VOICE).map(|r| r.user).unwrap_or_default()
}

/// One line: the ear's reading, the voice's words, and the game's answer.
fn line(g: &mut Game, f: &afm_ai::Fake, id: u32, reading: Value, said: &str, text: &str) -> Value {
    f.push(EAR, Ok(reading));
    f.push(VOICE, Ok(voice(said, "")));
    let v = say(g, id, text);
    assert_eq!(v["state"], "done", "{v}");
    v
}

fn heard(v: &Value) -> String {
    v["heard"].as_str().unwrap_or_default().to_string()
}

/// His goals for the season his meeting sets.
fn goals_for(g: &Game) -> (i32, Vec<boss::Goal>) {
    let w = g.world.as_ref().unwrap();
    let season = boss::goal_season(w, Team::seat(w).unwrap());
    (season, w.folk.goals.iter().filter(|x| x.season == season).cloned().collect())
}

fn campaign_key(k: &GoalKind) -> Option<&'static str> {
    match k {
        GoalKind::Campaign(a) => Some(a.def().key),
        _ => None,
    }
}

#[test]
fn the_state_one_you_commit_to_goes_on_his_goals() {
    let (mut g, f, _) = game("commit-state-one", true);
    let f = f.unwrap();
    let id = goals_meeting(&mut g);
    let (season, before) = goals_for(&g);
    assert!(!before.is_empty(), "he set the season's goals");
    // He leads with his goals: the meeting is on his card.
    let v = line(
        &mut g,
        &f,
        id,
        ear(vec![act("question", "", "Goals", "")], vec![], false),
        "Here's the list.",
        "So what are the goals?",
    );
    assert!(!heard(&v).contains("you didn't answer"), "{v}");
    let card = last_voice(&f);
    assert!(card.contains("This is your goals meeting with"), "{card}");
    assert!(card.contains(&format!("Your goals for {season}")), "{card}");
    // "Keep the state's #1 away from Clemson, every year."
    let t0 = boss::trust(g.world.as_ref().unwrap(), Team::seat(g.world.as_ref().unwrap()).unwrap());
    let v = line(
        &mut g,
        &f,
        id,
        ear(
            vec![commit_act("state_one", "", true, "keep the #1 South Carolina player away from Clemson every year")],
            vec![],
            false,
        ),
        "Then that's on the sheet.",
        "We'll keep the #1 South Carolina player away from Clemson every year.",
    );
    assert!(heard(&v).contains("a commitment he wrote down"), "{v}");
    let (_, after) = goals_for(&g);
    let mine = after.iter().find(|x| campaign_key(&x.kind) == Some("state_one")).expect("the state's #1 is a goal now");
    assert!(mine.talk && mine.deal == 1, "{mine:?}");
    let card = last_voice(&f);
    assert!(card.contains("He writes it down: the goal is now to"), "{card}");
    assert!(
        card.contains("next year's is for next year's goals meeting"),
        "every year is held one season at a time: {card}"
    );
    let s = scene_of(&g, id);
    assert!(s.outcome.iter().any(|l| l.contains("wrote it down")), "{:?}", s.outcome);
    // The sheet on the table shows it as the bar you put up yourself.
    let sheet = s.docs.iter().find(|d| d.title.ends_with(&format!("goals for {season}"))).expect("the goals sheet");
    assert!(
        sheet.rows.iter().any(|r| r.cells.get(1).is_some_and(|c| c == "The bar you put up yourself.")),
        "{:?}",
        sheet.rows
    );
    // His trust moved by what he believes of it, nothing else.
    let w = g.world.as_ref().unwrap();
    let moved = boss::trust(w, Team::seat(w).unwrap()) - t0;
    assert!(moved.abs() <= commit::HELD_TRUST + 1e-4, "{moved}");
    // Saying it again changes nothing: it's already on his list.
    let v = line(
        &mut g,
        &f,
        id,
        ear(vec![commit_act("state_one", "", false, "the #1 kid stays home")], vec![], false),
        "You said that.",
        "The #1 kid stays home.",
    );
    assert!(heard(&v).contains("already on his list"), "{v}");
    assert_eq!(goals_for(&g).1.len(), after.len());
}

#[test]
fn wins_above_his_bar_replace_his_and_below_it_or_vague_change_nothing() {
    let (mut g, f, _) = game("commit-wins", true);
    let f = f.unwrap();
    let id = goals_meeting(&mut g);
    let his = goals_for(&g).1.iter().find_map(|x| if let GoalKind::Wins(n) = x.kind { Some(n) } else { None });
    let n = his.map_or(6, |n| (n + 1).min(17));
    // "We'll win at least 6 games" is a bar he writes down.
    let v = line(
        &mut g,
        &f,
        id,
        ear(vec![commit_act("wins", &n.to_string(), false, "win at least that many")], vec![], false),
        "I'll hold you to that.",
        &format!("We'll win at least {n} games."),
    );
    assert!(heard(&v).contains("a commitment he wrote down"), "{v}");
    let wins: Vec<GoalKind> =
        goals_for(&g).1.iter().map(|x| x.kind).filter(|k| matches!(k, GoalKind::Wins(_))).collect();
    assert_eq!(wins, vec![GoalKind::Wins(n)], "yours in place of his");
    // Less than the bar on his list: his stays.
    let v = line(
        &mut g,
        &f,
        id,
        ear(vec![commit_act("wins", &(n - 2).to_string(), false, "")], vec![], false),
        "That's less than we said.",
        &format!("We'll win {} at least.", n - 2),
    );
    assert!(heard(&v).contains("below his own bar"), "{v}");
    assert!(goals_for(&g).1.iter().any(|x| x.kind == GoalKind::Wins(n)));
    assert!(last_voice(&f).contains("asking for time is how a bar comes down"));
    // A slogan: he asks what he can measure it by, nothing is written.
    let count = goals_for(&g).1.len();
    let v = line(
        &mut g,
        &f,
        id,
        ear(vec![commit_act("none", "", true, "make them cry")], vec![], false),
        "What does that look like on paper?",
        "We'll make them cry, year after year.",
    );
    assert!(heard(&v).contains("a commitment he can't measure"), "{v}");
    assert_eq!(goals_for(&g).1.len(), count);
    assert!(last_voice(&f).contains("he asks what he can measure it by"));
}

#[test]
fn a_dodge_needs_his_own_question_on_his_last_line() {
    let (mut g, f, _) = game("commit-dodge", true);
    let f = f.unwrap();
    let id = goals_meeting(&mut g);
    let line_ok = |g: &mut Game, reading: Value, said: &str, text: &str| line(g, &f, id, reading, said, text);
    // His last line asked nothing: an answer the ear calls a dodge isn't one.
    // (A line that asks him about his goals would keep him from asking
    // about them himself, `talk::asked_about`.)
    line_ok(&mut g, ear(vec![], vec![], false), "Those are the goals.", "Good to meet you.");
    let w = g.world.as_ref().unwrap();
    let t0 = boss::trust(w, Team::seat(w).unwrap());
    // From here on his every line asks something, so whichever line the
    // game raises his ask on puts it to you as a question.
    let v = line_ok(&mut g, dodging(vec![]), "Fine. So how do you keep that kid home?", "Any other questions?");
    assert!(!heard(&v).contains("you didn't answer"), "{v}");
    assert!(!heard(&v).contains("doubted on what he told you"), "{v}");
    let w = g.world.as_ref().unwrap();
    assert!((boss::trust(w, Team::seat(w).unwrap()) - t0).abs() < 1e-6, "no cost for a dodge that wasn't");
    // He asks: the game raises his ask on a line of its own choosing, one
    // at a time (`store::Open::asking`), and the line after it that ignores
    // it is a dodge.
    let asking = |g: &Game| store::open(g.world.as_ref().unwrap(), id).is_some_and(|o| o.asking().is_some());
    for text in ["It's a big year.", "We'll see.", "Fair enough."] {
        if asking(&g) {
            break;
        }
        line_ok(&mut g, ear(vec![], vec![], false), "So how do you keep that kid home?", text);
    }
    assert!(asking(&g), "he raises an ask");
    let v = line_ok(&mut g, dodging(vec![]), "Okay.", "Let's talk about the stadium.");
    assert!(heard(&v).contains("you didn't answer his question"), "{v}");
}

/// An ask he only said he'd raise later ("once the season starts I'd
/// like to talk about the stadium") asks nothing yet: the next line that
/// moves on is no dodge (the last replay, 2026-11-17: a doctor who said
/// he'd like to talk about where he fits "once the staff is set").
#[test]
fn an_ask_put_off_for_later_is_no_question_to_dodge() {
    let (mut g, f, _) = game("commit-later", true);
    let f = f.unwrap();
    let id = goals_meeting(&mut g);
    let line_ok = |g: &mut Game, reading: Value, said: &str, text: &str| line(g, &f, id, reading, said, text);
    let later = "Once the season starts, I'd like to talk about that.";
    let asking = |g: &Game| store::open(g.world.as_ref().unwrap(), id).is_some_and(|o| o.asking().is_some());
    for text in ["Good to meet you.", "It's a big year.", "We'll see.", "Fair enough."] {
        line_ok(&mut g, ear(vec![], vec![], false), later, text);
        if asking(&g) {
            break;
        }
    }
    assert!(asking(&g), "he raises an ask");
    let v = line_ok(&mut g, dodging(vec![]), "Okay.", "Let's talk about the stadium.");
    assert!(!heard(&v).contains("you didn't answer"), "{v}");
}

#[test]
fn an_offer_to_your_boss_gets_no_players_counter() {
    let (mut g, f, _) = game("commit-offer", true);
    let f = f.unwrap();
    let id = goals_meeting(&mut g);
    let mut offer = act("offer", "", "", "");
    offer["deal"] = json!("Nil");
    offer["words"] = json!("more than Clemson");
    // Money in an NIL deal is the kid's, never your extension.
    offer["gives"] = json!([{ "give": "money", "number": 500000 }]);
    let v = line(
        &mut g,
        &f,
        id,
        ear(vec![offer], vec![], false),
        "That's your call.",
        "We're gonna offer him more than Clemson.",
    );
    assert!(!heard(&v).contains("an offer"), "{v}");
    let card = last_voice(&f);
    assert!(!card.to_lowercase().contains("leadership council"), "{card}");
    assert!(!card.contains("What you ask the coach yourself"), "{card}");
}

#[test]
fn a_money_ask_names_the_goal_it_puts_up() {
    let (g, _, _) = game("commit-ask", false);
    let w = g.world.as_ref().unwrap();
    let team = Team::seat(w).unwrap();
    use afm_engine::world::folk::asked::{Case, goal_in, strip_goal};
    let words = "NIL money, the state's number one signs, goal:state_one";
    assert_eq!(strip_goal(words), "NIL money, the state's number one signs");
    assert_eq!(goal_in(words).map(|c| c.key), Some("state_one".into()));
    let bar = Case::read_in(w, team, words).bar;
    assert_eq!(bar.as_ref().and_then(campaign_key), Some("state_one"), "{bar:?}");
    assert_eq!(Case::read_in(w, team, "staff money for nine wins, GOAL:wins 9").bar, Some(GoalKind::Wins(9)));
    // Any text, any case: never a slice inside a letter.
    assert_eq!(strip_goal("İstanbul money, goal:wins 8"), "İstanbul money");
    assert_eq!(Case::read_in(w, team, "money for the staff").bar, None);
}

#[test]
fn the_ears_goal_keys_are_the_games() {
    let rules = super::super::ear::EAR_RULES;
    let from = rules.find("Keys: ").expect("the commit keys") + 6;
    let to = from + rules[from..].find(". A commitment with nothing").expect("the list ends");
    let mut listed: Vec<String> = Vec::new();
    let mut depth = 0;
    let mut cur = String::new();
    for ch in rules[from..to].chars() {
        match ch {
            '(' => depth += 1,
            ')' => depth -= 1,
            ',' if depth == 0 => listed.push(std::mem::take(&mut cur).trim().to_string()),
            _ if depth == 0 => cur.push(ch),
            _ => {}
        }
    }
    listed.push(cur.trim().to_string());
    let mut game_keys: Vec<String> = commit::keys().into_iter().filter(|k| *k != "none").map(String::from).collect();
    game_keys.sort();
    listed.sort();
    assert_eq!(listed, game_keys);
    // Every class key is one class's goal that names nobody.
    for k in commit::class_keys() {
        let d = goals::by_key(k).unwrap();
        assert!(d.classes == 1 && !d.later, "{k}");
    }
}

/// A holdover coordinator's meeting (`goals_tests`).
fn holdover(g: &mut Game) -> (u32, usize) {
    let w = g.world.as_mut().unwrap();
    let team = Team::seat(w).unwrap();
    let me = w.career.as_ref().unwrap().coach;
    let c = facts::assistants(w, team).into_iter().map(|x| x.0).find(|&c| c != me).expect("a staff coach");
    let old = (0..w.coaches.len()).find(|&k| k != me && k != c && w.coaches[k].job.is_none()).unwrap_or(0);
    let today = w.today;
    let t = w.folk.takeover.get_or_insert_with(takeover::Takeover::default);
    t.team = Some(team);
    t.started = Some(today);
    t.old_coach = Some(old);
    t.kept.clear();
    t.let_go.clear();
    w.folk.staff.interviewed.retain(|x| x.0 != c);
    let mut s = Scene::new(Kind::Holdover, today, "Meeting the offensive coordinator");
    s.a = c as u64;
    s.stops = false;
    s.place = "Your office".into();
    let sid = scene::add(w, s);
    assert_eq!(stakes::holdover(w, c), Some(Holdover::OnTheLine));
    ok(g, "people_scene", json!({ "id": sid }));
    (sid, c)
}

#[test]
fn asked_why_you_should_keep_him_a_holdover_makes_his_case() {
    let (mut g, f, _) = game("commit-job", true);
    let f = f.unwrap();
    let (id, c) = holdover(&mut g);
    let v = line(
        &mut g,
        &f,
        id,
        ear(vec![act("question", "", "Job", "")], vec![], false),
        "Because I know these kids, Coach.",
        "Convince me why I should let you stay OC.",
    );
    assert!(heard(&v).contains("a question"), "{v}");
    let card = last_voice(&f);
    assert!(card.contains("he knows these players and the recruits"), "his case, straight: {card}");
    assert!(!card.contains("Holding back"), "{card}");
    // His own job is his: the same question about someone else's isn't.
    let w = g.world.as_ref().unwrap();
    let k = afm_engine::world::folk::persona::knowing::knows(
        w,
        Them::Coach(c),
        afm_engine::world::folk::persona::knows::Topic::Job,
        None,
    );
    assert!(k.iter().any(|x| x.words == "he knows these players and the recruits"), "{k:?}");
}

#[test]
fn a_stay_to_your_staff_and_a_promise_to_a_parent_are_kept() {
    let (mut g, f, him) = game("commit-promises", true);
    let f = f.unwrap();
    let (id, c) = holdover(&mut g);
    let n0 = g.world.as_ref().unwrap().folk.promises.len();
    let v = line(
        &mut g,
        &f,
        id,
        ear(vec![act("promise", "stay", "none", "")], vec![], false),
        "Good to hear.",
        "I'm not going anywhere.",
    );
    assert!(heard(&v).contains("a promise"), "{v}");
    let w = g.world.as_ref().unwrap();
    assert_eq!(w.folk.promises.len(), n0 + 1, "the stay is on your word");
    let p = w.folk.promises.last().unwrap();
    assert!(p.to == PromiseTo::Coach(c) && p.kind == PromiseKind::Stay, "{p:?}");
    // A parent's call: a start promised about the son is kept with him.
    let w = g.world.as_mut().unwrap();
    let mut s = Scene::new(Kind::ParentCall, w.today, "A call from his father");
    s.a = him as u64;
    s.stops = false;
    s.place = "On the phone".into();
    let pid = scene::add(w, s);
    ok(&mut g, "people_scene", json!({ "id": pid }));
    let n1 = g.world.as_ref().unwrap().folk.promises.len();
    let v = line(
        &mut g,
        &f,
        pid,
        ear(vec![act("promise", "start", "none", "")], vec![], false),
        "We'll hold you to that, Coach.",
        "He'll start by midseason.",
    );
    assert!(heard(&v).contains("a promise"), "{v}");
    let w = g.world.as_ref().unwrap();
    assert_eq!(w.folk.promises.len(), n1 + 1, "the promise is on his son's file");
    assert_eq!(w.folk.promises.last().unwrap().to, PromiseTo::Player(him));
}

#[test]
fn heard_as_says_how_it_went_on_his_own_scale() {
    use afm_engine::world::folk::persona::talk::hits::landed_bars;
    use afm_engine::world::folk::talk_points::LANDED;
    // A player moves on his own scale; someone in your building on his
    // meeting's (an office line moves trust at most 0.02).
    let (pu, pd) = landed_bars(Them::Player(1));
    assert!((pu - LANDED).abs() < 1e-6 && pd < 0.0);
    let (cu, cd) = landed_bars(Them::Coach(1));
    assert!(cu > 0.0 && cu < pu && cd < 0.0, "{cu} {cd}");
    // A commitment he writes down and believes lands with him.
    assert!(commit::HELD_TRUST > cu && commit::DOUBTED_TRUST < 0.0);
}

#[test]
fn a_booster_holds_you_to_what_he_wants() {
    use afm_engine::world::folk::cast::Want;
    use afm_engine::world::folk::commit::{Commitment, Took, booster};
    let c = |k: &str| Commitment { key: k.into(), ..Commitment::default() };
    assert_eq!(booster(&[Want::BeatRival], &c("beat_rival_class")), Took::Booster(Want::BeatRival));
    assert_eq!(booster(&[Want::WinNow], &c("wins")), Took::Booster(Want::WinNow));
    assert_eq!(booster(&[Want::WinNow], &c("grades")), Took::NotHisWant);
    // On his books, with no trust of his moved the moment you say it.
    assert!(Took::Booster(Want::Clean).held() && Took::Booster(Want::Clean).trust() == 0.0);
}
