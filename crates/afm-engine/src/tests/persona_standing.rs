//! Where he stands (Cain 2026-11-16: "the player/ai doesn't know what he
//! is ... this is a player with no eligibility left"): every talk reads a
//! man's own situation from the world (`persona::talk::standing`). These
//! prove that a player whose college career ends this year knows it, wants
//! the draft rather than a role, never opens on the portal and is never
//! asked to weigh a choice he doesn't have; that a boss knows he is the one
//! who judges you; that a decision is heard from the side of the man it's
//! on, with his name on the confirm; and that a recruit committed to you is
//! never told he hasn't decided.

use super::legacy::seat;
use crate::player::PlayingTime;
use crate::world::folk::Team;
use crate::world::folk::cast::{CastRef, Role};
use crate::world::folk::persona::ear::{Act, Reading};
use crate::world::folk::persona::gameday::group;
use crate::world::folk::persona::knows::{Answer, Knows, TellParts, Tells, Topic};
use crate::world::folk::persona::people::People;
use crate::world::folk::persona::room::{Channel, Place, Present, Room, Seat};
use crate::world::folk::persona::talk::stakes;
use crate::world::folk::persona::talk::standing::{self, Road};
use crate::world::folk::persona::talk::{self, channels};
use crate::world::folk::persona::terms::Decision;
use crate::world::folk::persona::wants::WantKind;
use crate::world::folk::persona::{Them, office, wanting};
use crate::world::folk::scene::{Kind as SceneKind, Scene};
use crate::world::folk::voice::{Facts, Pen, Speaker, bank};
use crate::world::{Stage, World};

/// Your college program with one player on it, a starter in his
/// `seasons`-th college season (counting the one being played).
fn player(seasons: u32) -> (World, usize) {
    let mut w = seat("college");
    let p = w.seat.and_then(|s| s.program).unwrap();
    w.add_class_aged(19);
    let year = w.year;
    let i = (0..w.people.len()).find(|&i| w.people[i].stage == Stage::HighSchool).unwrap();
    let q = &mut w.people[i];
    q.stage = Stage::College;
    q.college = Some(p);
    q.class_of = year - seasons as i32;
    q.college_seasons = seasons;
    q.last_role = Some(PlayingTime::Starter);
    q.trust = 0.6;
    w.programs[p].roster.push(i);
    (w, i)
}

/// A player with no college eligibility after this season, whatever the
/// day of the test world.
fn last_year() -> (World, usize) {
    let (mut w, i) = player(4);
    if standing::seasons_after(&w, i) > 0 {
        w.people[i].class_of -= 1;
        w.people[i].college_seasons += 1;
    }
    assert_eq!(standing::seasons_after(&w, i), 0);
    (w, i)
}

#[test]
fn a_player_whose_college_career_ends_knows_it_and_wants_the_draft() {
    let (w, i) = last_year();
    let road = standing::road(&w, i);
    assert!(road.college_ends(), "{road:?}");
    assert!(!road.has_choice());
    let id = w.people[i].player.id;
    let words = standing::words(&w, Them::Player(id)).join(". ");
    assert!(words.contains("no transfer and no coming back"), "{words}");
    assert!(words.contains("NFL draft"), "{words}");
    assert!(!words.contains("in the transfer portal"), "{words}");
    // His wants follow it: the draft first, a role far behind it.
    let raw = wanting::raw(&w, Them::Player(id));
    let weight = |k: WantKind| raw.iter().filter(|x| x.0 == k).map(|x| x.1).sum::<f32>();
    let top = raw.iter().copied().fold((WantKind::Role, f32::MIN), |a, b| if b.1 > a.1 { b } else { a });
    assert_eq!(top.0, WantKind::Draft, "{raw:?}");
    assert!(weight(WantKind::Draft) > 3.0 * weight(WantKind::Role), "{raw:?}");
    // His parent sees it the same way, about his son.
    let son = standing::words(&w, Them::Kin(id, 0)).join(". ");
    assert!(son.starts_with("Your son is"), "{son}");
}

#[test]
fn a_younger_player_has_his_seasons_and_no_draft_talk() {
    let (w, i) = player(2);
    let road = standing::road(&w, i);
    assert!(matches!(road, Road::College | Road::Deciding | Road::Returning), "{road:?}");
    assert!(!road.college_ends());
    let id = w.people[i].player.id;
    let words = standing::words(&w, Them::Player(id)).join(". ");
    assert!(words.contains("of college eligibility left after this one"), "{words}");
    assert!(!words.contains("no coming back"), "{words}");
    let raw = wanting::raw(&w, Them::Player(id));
    assert!(raw.iter().any(|x| x.0 == WantKind::Role), "{raw:?}");
}

#[test]
fn his_lines_keep_to_his_last_year() {
    let (w, i) = last_year();
    let who = Speaker::player(&w, i);
    assert!(who.has("lastyear"));
    let facts = Facts::new();
    // The bank has a draft opening for him, and none of his lines in any
    // key talks of the portal, a transfer or next season in college.
    assert!(bank().fitting("talk.open.draft", &who, &facts) > 0);
    let mut pen = Pen::loose(7);
    for key in ["talk.open.future", "talk.open.draft", "talk.open.role", "talk.open.good", "talk.open.bad"] {
        for _ in 0..40 {
            let line = pen.line(&[key], &who, &facts).to_lowercase();
            for not in ["portal", "transfer", "redshirt", "next season", "next year"] {
                assert!(!line.contains(not), "{key}: {line}");
            }
        }
    }
    // A younger player keeps them.
    let (w2, j) = player(2);
    let young = Speaker::player(&w2, j);
    assert!(!young.has("lastyear"));
    assert_eq!(bank().fitting("talk.open.draft", &young, &facts), 0);
}

#[test]
fn the_draft_question_asks_a_choice_only_of_a_man_who_has_one() {
    let (w, i) = last_year();
    let who = Speaker::player(&w, i);
    let no = Facts::new().with("choice", "no");
    let yes = Facts::new().with("choice", "yes");
    let mut pen = Pen::loose(11);
    for _ in 0..60 {
        let line = pen.line(&["talk.point.ask.draft"], &who, &no);
        assert!(!line.contains("ready to go") && !line.contains("Should I go"), "{line}");
    }
    assert!(bank().fitting("talk.point.ask.draft", &who, &yes) > bank().fitting("talk.point.ask.draft", &who, &no));
}

#[test]
fn a_boss_knows_he_is_the_one_who_judges_you() {
    let w = seat("college");
    let team = Team::seat(&w).unwrap();
    let ad = office::cast_them(&w, CastRef::new(team, Role::AthleticDirector, 0));
    let words = standing::words(&w, ad).join(". ");
    assert!(words.contains("You set his goals"), "{words}");
    assert!(words.contains("you're the one who judges him"), "{words}");
}

#[test]
fn a_decision_is_heard_from_his_side_and_confirmed_by_his_name() {
    assert_eq!(channels::decision_to(Decision::CoachLetGo, Them::Coach(3)), "he's letting you go: you're fired");
    assert_eq!(channels::decision_to(Decision::CoachLetGo, Them::Player(9)), "your coach is let go");
    assert_eq!(channels::decision_to(Decision::Benched, Them::Player(9)), "you're benched");
    assert_eq!(channels::decision_named(Decision::CoachLetGo, "Dale Hutto"), "Dale Hutto is let go");
    assert_eq!(channels::decision_named(Decision::Benched, "Ryan Wyatt"), "Ryan Wyatt is benched");
}

#[test]
fn a_recruit_committed_to_you_is_never_told_he_hasnt_decided() {
    let mut w = seat("college");
    let ours = w.seat.and_then(|s| s.program).unwrap();
    w.add_class_aged(17);
    let i = (0..w.people.len()).find(|&i| w.people[i].stage == Stage::HighSchool).unwrap();
    let today = w.today;
    w.people[i].cal.commit = Some((ours, today, false));
    let id = w.people[i].player.id;
    let s = Scene::new(SceneKind::RecruitCall, today, "A call");
    let all = stakes::stakes(&w, &s, Them::Player(id), &[]).join(". ");
    assert!(all.contains("You're committed to his school"), "{all}");
    assert!(!all.contains("you haven't decided"), "{all}");
    // Committed somewhere else: he's trying to change your mind.
    let other = (0..w.programs.len()).find(|&p| p != ours).unwrap();
    w.people[i].cal.commit = Some((other, today, false));
    let all = stakes::stakes(&w, &s, Them::Player(id), &[]).join(". ");
    assert!(all.contains("trying to change your mind"), "{all}");
}

#[test]
fn a_boss_tells_his_goals_and_the_budget_straight() {
    let w = seat("college");
    let team = Team::seat(&w).unwrap();
    let ad = office::cast_them(&w, CastRef::new(team, Role::AthleticDirector, 0));
    // The money is his budget, in words and never a figure.
    let money = People.knows(&w, ad, Topic::Money, None);
    assert!(money.iter().any(|k| k.words.contains("budget")), "{money:?}");
    assert!(!money.iter().any(|k| k.words.contains('$')), "{money:?}");
    // Asked what he's looking for, he means his goals for you.
    let wants = People.knows(&w, ad, Topic::Wants, None);
    assert!(!wants.is_empty() && wants.iter().all(|k| k.topic == Topic::Goals), "{wants:?}");
    // Even cold and guarded he tells both: they're his to tell.
    let cold = TellParts { trust: 0.0, professionalism: 1.0, ..Default::default() };
    for known in [&money, &wants] {
        let told = People.tell(&w, ad, known, &cold);
        assert_eq!(told.answer, Answer::Straight, "{told:?}");
    }
}

#[test]
fn your_support_staff_want_their_own_careers() {
    let w = seat("college");
    let team = Team::seat(&w).unwrap();
    let raw = |role| wanting::raw(&w, office::cast_them(&w, CastRef::new(team, role, 0)));
    let weight = |v: &[(WantKind, f32)], k: WantKind| v.iter().filter(|x| x.0 == k).map(|x| x.1).sum::<f32>();
    let trainer = raw(Role::Trainer);
    let comms = raw(Role::Comms);
    // The program's name is the communications director's job, a little
    // anyone else's.
    assert!(weight(&trainer, WantKind::Name) < 0.2, "{trainer:?}");
    assert!(weight(&comms, WantKind::Name) >= 0.1, "{comms:?}");
    assert!(weight(&trainer, WantKind::Career) > weight(&trainer, WantKind::Name), "{trainer:?}");
    assert!(weight(&trainer, WantKind::Winning) > 0.0, "{trainer:?}");
}

#[test]
fn a_family_that_does_the_talking_answers_for_him() {
    let kid = Them::Player(1);
    let mom = Them::Kin(1, 0);
    let room = |seat| Room {
        channel: Channel::InPerson(Place::Home),
        present: vec![Present { who: kid, seat: Seat::Across, speaks: true }, Present { who: mom, seat, speaks: true }],
    };
    let asked = |topic| Reading {
        acts: vec![Act::Question { topic, about: None, words: String::new() }],
        ..Default::default()
    };
    // His parents do the talking: they answer what matters in the
    // decision and the floor you give them; he answers his football.
    let decides = room(Seat::Decider);
    assert_eq!(channels::speaker(&decides, &asked(Topic::Wants)), Some((mom, Seat::Decider)));
    assert_eq!(channels::speaker(&decides, &asked(Topic::Nothing)), Some((mom, Seat::Decider)));
    assert_eq!(channels::speaker(&decides, &asked(Topic::Football)), None);
    // Asked about him in the third person ("how's his year going?"), it's
    // theirs to answer, his football too.
    let about_him = Reading {
        acts: vec![Act::Question { topic: Topic::Football, about: Some(kid), words: "How's his year going?".into() }],
        ..Default::default()
    };
    assert_eq!(channels::speaker(&decides, &about_him), Some((mom, Seat::Decider)));
    // A line that asks nothing ("he'll get every chance to compete") is
    // theirs to answer too.
    assert_eq!(channels::speaker(&decides, &Reading::default()), Some((mom, Seat::Decider)));
    // He decides himself: his family sits in, and he answers.
    let himself = room(Seat::Family);
    assert_eq!(channels::speaker(&himself, &asked(Topic::Wants)), None);
    assert_eq!(channels::speaker(&himself, &about_him), None);
}

#[test]
fn he_knows_his_place_from_last_season_until_this_one_shows_it() {
    // The talks audit, 2026-11-16: in the offseason a DT asked where he
    // stood on the depth chart, then "do I have a real role?" right after
    // "you're my first pick at DT". With nothing seen this season yet, he
    // knows what he did last season.
    let (mut w, i) = player(2);
    let id = w.people[i].player.id;
    assert_eq!(crate::world::folk::seen::first_now(&w, i), None, "nothing seen yet this season");
    let words = standing::words(&w, Them::Player(id)).join(". ");
    assert!(
        words.contains("Last season you started. Nobody's place for this season is set until practice starts"),
        "{words}"
    );
    w.people[i].last_role = Some(PlayingTime::Backup);
    let words = standing::words(&w, Them::Player(id)).join(". ");
    assert!(words.contains("Last season you were a backup"), "{words}");
    // A freshman has no college season to go on.
    w.people[i].college_seasons = 0;
    let words = standing::words(&w, Them::Player(id)).join(". ");
    assert!(words.contains("You haven't played a college snap yet"), "{words}");
}

#[test]
fn your_read_shows_when_it_catches_something_new() {
    // "He gives you nothing either way." stood under every reply of a talk
    // with a DT (Cain 2026-11-16): shown once, then again only when it
    // changes.
    let (mut w, i) = player(2);
    let mut s = Scene::new(SceneKind::OneOnOne, w.today, "A talk");
    s.a = w.people[i].player.id as u64;
    let sid = crate::world::folk::scene::add(&mut w, s);
    let nothing = "He gives you nothing either way.".to_string();
    talk::reply(&mut w, sid, "Okay, Coach.", None, Some(nothing.clone()), None);
    talk::reply(&mut w, sid, "Sure.", None, Some(nothing.clone()), None);
    talk::reply(&mut w, sid, "Fine.", None, Some("He sits up.".into()), None);
    talk::reply(&mut w, sid, "Yeah.", None, Some(nothing.clone()), None);
    let sc = w.folk.scenes.iter().find(|x| x.id == sid).unwrap();
    let tells: Vec<Option<String>> = sc.lines.iter().filter(|l| l.by == 2).map(|l| l.tell.clone()).collect();
    assert_eq!(tells, vec![Some(nothing.clone()), None, Some("He sits up.".into()), Some(nothing)]);
}

#[test]
fn a_senior_is_never_green_and_a_coach_counts_his_years() {
    // A college man in his fourth year or his last isn't "young", whatever
    // his age: "I know I'm green" was a senior's opening.
    let (mut w, i) = player(4);
    w.people[i].player.age = 21;
    assert!(!Speaker::player(&w, i).has("young"));
    let (mut w2, j) = player(1);
    w2.people[j].player.age = 19;
    assert!(Speaker::player(&w2, j).has("young"));
    // A coach of yours three seasons in, with no history kept: his years
    // counted as the opening counts them, never "less than a year".
    let me = w.career.as_ref().map(|c| c.coach);
    let team = Team::seat(&w).unwrap();
    let c = crate::world::folk::facts::assistants(&w, team)
        .into_iter()
        .map(|x| x.0)
        .find(|&c| Some(c) != me)
        .expect("a staff coach");
    let year = w.year;
    w.coaches[c].history.clear();
    w.coaches[c].born = crate::calendar::born::UNKNOWN;
    w.coaches[c].age = 50;
    if let Some(j) = w.coaches[c].job.as_mut() {
        j.first = 0;
        j.since = year - 3;
    }
    let room = Room {
        channel: Channel::InPerson(Place::Office),
        present: vec![Present { who: Them::Coach(c), seat: Seat::Across, speaks: true }],
    };
    let who = talk::who::who(&w, Them::Coach(c), &room);
    assert!(who.background.starts_with("50, coaching 3 years; came up as a"), "{}", who.background);
    assert!(!who.background.contains(" a o") && !who.background.contains(" a a"), "{}", who.background);
}

#[test]
fn your_boss_and_your_coordinator_say_how_its_going() {
    let w = seat("college");
    let team = Team::seat(&w).unwrap();
    // Your boss, asked how it's going: where you stand with him, straight.
    let ad = office::cast_them(&w, CastRef::new(team, Role::AthleticDirector, 0));
    let view = People.knows(&w, ad, Topic::Football, None);
    assert!(!view.is_empty(), "{view:?}");
    let cold = TellParts { trust: 0.0, professionalism: 1.0, ..Default::default() };
    assert_eq!(People.tell(&w, ad, &view, &cold).answer, Answer::Straight);
    // A coach on your staff, asked about his unit: never "he doesn't know".
    let me = w.career.as_ref().map(|c| c.coach);
    let c = crate::world::folk::facts::assistants(&w, team)
        .into_iter()
        .map(|x| x.0)
        .find(|&c| Some(c) != me)
        .expect("a staff coach");
    let unit = People.knows(&w, Them::Coach(c), Topic::Football, None);
    assert!(!unit.is_empty(), "{unit:?}");
    // The ear tags him on "the unit" (it's his own): the same answer.
    let own = People.knows(&w, Them::Coach(c), Topic::Football, Some(Them::Coach(c)));
    assert_eq!(own.len(), unit.len(), "{own:?}");
    let told =
        People.tell(&w, Them::Coach(c), &unit, &TellParts { trust: 0.6, professionalism: 10.0, ..Default::default() });
    assert!(!told.released.iter().any(|x| x == "He doesn't know."), "{told:?}");
}

#[test]
fn the_one_who_answers_speaks_with_no_label_in_his_words() {
    let (w, i) = player(2);
    let id = w.people[i].player.id;
    let (kid, mom) = (Them::Player(id), Them::Kin(id, 0));
    let room = Room {
        channel: Channel::InPerson(Place::Home),
        present: vec![
            Present { who: kid, seat: Seat::Across, speaks: true },
            Present { who: mom, seat: Seat::Decider, speaks: true },
        ],
    };
    // The voice wrote who speaks into the line (the talks audit,
    // 2026-11-16): the screen shows who speaks, so the label goes.
    let label = format!("{}:", channels::name(&w, mom));
    let said = format!("Coach, I appreciate it. {label} What we'd like to know is how you'll develop him.");
    assert_eq!(
        talk::unlabel(&w, &room, &said),
        "Coach, I appreciate it. What we'd like to know is how you'll develop him."
    );
    assert_eq!(talk::unlabel(&w, &room, &format!("{label} Money first.")), "Money first.");
    // A line with no label stays as said.
    assert_eq!(talk::unlabel(&w, &room, "Fine,  Coach."), "Fine,  Coach.");
}

#[test]
fn the_team_room_is_the_men_still_on_the_team() {
    let (mut w, i) = player(2);
    let team = Team::seat(&w).unwrap();
    let mut s = Scene::new(SceneKind::TeamTalk, w.today, "Talking to the team");
    s.team = Some(team);
    let id = Them::Player(w.people[i].player.id);
    let room = group::room(&w, &s).expect("a team room");
    assert!(room.present.iter().any(|p| p.who == id), "he's on the team");
    // His college career over (he declared), he's getting ready for the
    // draft: not in the team room, never the man who answers for it.
    w.people[i].stage = Stage::Declared;
    assert_eq!(standing::road(&w, i), Road::DraftBound);
    let room = group::room(&w, &s);
    assert!(room.as_ref().is_none_or(|r| !r.present.iter().any(|p| p.who == id)), "{room:?}");
}

/// Your whole history with him, read live on every line (Cain 2026-11-16:
/// "it should always check it's relation with me when speaking to me,
/// every aspect as much as it would itself"): a coach in his fifth week
/// has none, and the card says so in his terms, the right way round.
#[test]
fn a_new_coach_has_no_history_with_his_players_yet() {
    use crate::world::folk::persona::talk::between;
    let (mut w, i) = player(2);
    let id = w.people[i].player.id;
    w.folk.mine.since = Some(w.today.add(-35));
    let words = between::words(&w, Them::Player(id)).join(". ");
    assert!(words.contains("he's new here, and he has no history with this team yet"), "{words}");
    assert!(words.contains("about 5 weeks ago"), "{words}");
    assert!(words.contains("before the coach came"), "{words}");
    assert!(words.contains("You haven't practiced or played a game for the coach yet"), "{words}");
    assert!(words.contains("The coach hasn't made you a promise"), "{words}");
    assert!(words.contains("This is your first talk alone with the coach"), "{words}");
    assert!(words.contains("still making up your mind"), "{words}");
    // Second person: the player is "you", the coach is "the coach".
    assert!(!words.contains("you're new to him") && !words.contains("about you"), "{words}");
    assert!(!words.to_lowercase().contains("depth chart"), "{words}");
    // The voice's card carries it.
    let room = channels::one_on_one(&Scene::new(SceneKind::OneOnOne, w.today, "A talk"));
    let who = talk::who::who(&w, Them::Player(id), &room);
    assert_eq!(who.known_you, words);
}

/// A coach years into the job: the promise he made and kept, the talk you
/// had and what was said in it, your ruling on him, how long you've been
/// together, and never "new".
#[test]
fn your_history_with_him_grows_with_your_years_here() {
    use crate::world::folk::persona::talk::between;
    use crate::world::folk::rules::{Response, RuleKind, Ruling};
    use crate::world::folk::scene::{self, Said, Who};
    use crate::world::folk::talks::{Promise, PromiseKind, PromiseTo};
    let (mut w, i) = player(3);
    let id = w.people[i].player.id;
    let today = w.today;
    w.folk.mine.since = Some(today.add(-4 * 365 - 20));
    w.people[i].trust = 0.85;
    let mut p = Promise::blank(PromiseTo::Player(id), PromiseKind::Start);
    p.made = today.add(-40);
    p.due = today.add(-10);
    p.kept = Some(true);
    w.folk.promises.push(p);
    let mut s = Scene::new(SceneKind::OneOnOne, today.add(-9), "A talk");
    s.who = vec![Who::Player(id)];
    s.lines = vec![
        Said { by: 1, text: "You'll start Saturday.".into(), ..Default::default() },
        Said { by: 2, text: "I won't let you down, Coach.".into(), ..Default::default() },
    ];
    s.done = true;
    scene::add(&mut w, s);
    w.folk.rules.rulings.push(Ruling {
        date: today.add(-60),
        id,
        kind: RuleKind::Curfew,
        response: Response::Running,
        star: false,
    });
    let words = between::words(&w, Them::Player(id)).join(". ");
    assert!(words.contains("has been head coach here since") && words.contains("about 4 years ago"), "{words}");
    assert!(words.contains("the coach promised you a starting job: he kept it"), "{words}");
    assert!(words.contains("You've talked with the coach once before this one"), "{words}");
    assert!(
        words.contains("the coach said \"You'll start Saturday.\" and you said \"I won't let you down, Coach.\""),
        "{words}"
    );
    assert!(
        words.contains("you broke the team rule on curfew, and the coach gave you early-morning running"),
        "{words}"
    );
    assert!(words.contains("You trust the coach"), "{words}");
    assert!(!words.contains("new here") && !words.contains("first talk"), "{words}");
}

/// Your staff and your boss know their own history with you: a holdover
/// you kept, goals not set yet, and where the boss's patience is.
#[test]
fn your_staff_and_your_boss_know_your_history_with_them() {
    use crate::world::folk::persona::talk::between;
    let mut w = seat("college");
    let team = Team::seat(&w).unwrap();
    w.folk.mine.since = Some(w.today.add(-20));
    let me = w.career.as_ref().map(|c| c.coach);
    let c = crate::world::folk::facts::assistants(&w, team)
        .into_iter()
        .map(|x| x.0)
        .find(|&c| Some(c) != me)
        .expect("a staff coach");
    let t = w.folk.takeover.get_or_insert_with(Default::default);
    t.team = Some(team);
    t.kept.push(c);
    let staff = between::words(&w, Them::Coach(c)).join(". ");
    assert!(staff.contains("before the coach came, and he kept you on"), "{staff}");
    assert!(staff.contains("he's new here"), "{staff}");
    let ad = office::cast_them(&w, CastRef::new(team, Role::AthleticDirector, 0));
    let boss = between::words(&w, ad).join(". ");
    let season = crate::world::folk::boss::goal_season(&w, team);
    let has_goals = w.folk.goals.iter().any(|g| g.season == season);
    assert!(has_goals || boss.contains(&format!("You haven't set the coach's goals for {season} yet")), "{boss}");
    assert!(
        boss.contains("with the coach") || boss.contains("the coach,") || boss.contains("You want a change"),
        "{boss}"
    );
}

/// The day he came reads once ("today", never "today, today"), and a
/// player who came after the coach, out of high school, knows the coach
/// signed him even once his signing is off the books (it clears when he
/// enrolls).
#[test]
fn the_day_the_coach_came_and_how_a_player_got_here() {
    use crate::calendar::Date;
    use crate::world::folk::persona::talk::between;
    let (mut w, i) = player(1);
    let id = w.people[i].player.id;
    w.folk.mine.since = Some(w.today);
    let words = between::words(&w, Them::Player(id)).join(". ");
    assert!(words.contains("took the head coach job here today: he's new here"), "{words}");
    assert!(!words.contains("today, today"), "{words}");
    // The coach was here by the December before his first season.
    let first = w.people[i].class_of;
    w.folk.mine.since = Some(Date::ymd(first - 2, 12, 20));
    w.people[i].cal.signed = None;
    w.people[i].college_staff.clear();
    let words = between::words(&w, Them::Player(id)).join(". ");
    assert!(
        words.contains(&format!("The coach signed you, and you joined the team for the {first} season")),
        "{words}"
    );
    assert!(!words.contains("before the coach came"), "{words}");
    // A coach who came in the January of his first season: he may have
    // been on campus already as an early enrollee, so it says neither.
    w.folk.mine.since = Some(Date::ymd(first, 1, 10));
    let words = between::words(&w, Them::Player(id)).join(". ");
    assert!(words.contains(&format!("You came here for the {first} season, about when the coach did")), "{words}");
    // A coach who came after his first season: he was here before.
    w.folk.mine.since = Some(Date::ymd(first, 12, 10));
    let words = between::words(&w, Them::Player(id)).join(". ");
    assert!(words.contains("You were on the team before the coach came"), "{words}");
}

/// A line that looks back on this year's spring practice or fall camp
/// waits until it has happened (the talks audit of 2026-11-16: a February
/// opener asked "did spring settle it?").
#[test]
fn a_line_about_spring_waits_for_spring() {
    let (mut w, i) = player(2);
    let spring = w.dates.spring_practice.0;
    let camp = w.dates.fall_camp;
    let opener = "Is the starting job still up for grabs, or did spring settle it?";
    let version = bank().versions("talk.open.role").into_iter().find(|v| v.0 == opener).expect("the opener");
    assert!(version.2.iter().any(|t| t == "prespring"), "{:?}", version.2);
    // A line looking ahead to spring keeps no such tag.
    let ahead = bank().versions("talk.open.role").into_iter().find(|v| v.0.contains("this spring?"));
    assert!(ahead.is_none_or(|v| !v.2.iter().any(|t| t == "prespring")));
    w.today = spring.add(-10);
    let who = Speaker::player(&w, i);
    assert!(who.has("prespring") && who.has("precamp"));
    let facts = Facts::new();
    let mut pen = Pen::loose(5);
    for _ in 0..80 {
        let line = pen.line(&["talk.open.role"], &who, &facts).to_lowercase();
        assert!(
            !line.contains("did spring") && !line.contains("since spring") && !line.contains("after camp"),
            "{line}"
        );
    }
    w.today = camp.add(-10);
    let who = Speaker::player(&w, i);
    assert!(!who.has("prespring") && who.has("precamp"));
    w.today = camp.add(3);
    assert!(!Speaker::player(&w, i).has("precamp"));
}
