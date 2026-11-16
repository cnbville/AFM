//! Live talks v3, lane N5c: staff, bosses and money
//! (`world::folk::persona::office`). The proofs: the team doctor reads a
//! hurt man's chart and never clears him when the chip wouldn't; a
//! reference call puts his read on the coach's page and you owe him one;
//! a line to someone in your building moves the trust the world keeps for
//! him, held inside the chips; your boss weighs an ask in your words as
//! you said it (what you put up, his trust, his budget), never on his
//! meeting's chip, and says why; a coordinator moves the plan
//! by the staff meeting's own step; and a sit-down with no key plays as
//! chips on the same conversation and ends when you end it.

use super::legacy::seat;
use crate::calendar::{Date, Level};
use crate::player::PlayingTime;
use crate::training::seat::Hurt;
use crate::world::coach::Role;
use crate::world::folk::asked::{self, Case};
use crate::world::folk::boss::{self, AskKind, Goal, GoalKind};
use crate::world::folk::cast::{self, CastRef, Role as CastRole};
use crate::world::folk::coach_look;
use crate::world::folk::persona::Them;
use crate::world::folk::persona::ear::{Act, Hit, How, Item, Reading, Resolving, Sized, Turn};
use crate::world::folk::persona::knows::{Answer, Told, Topic};
use crate::world::folk::persona::office::{self, asks, doctor, refs, sit};
use crate::world::folk::persona::talk::{Conversation, Said, store};
use crate::world::folk::persona::terms::Request;
use crate::world::folk::persona::wants::Willing;
use crate::world::folk::{Team, scene};
use crate::world::{Stage, World};

/// A college world with your program and one of your players hurt for
/// three weeks.
fn hurt() -> (World, Team, u32) {
    let mut w = seat("college");
    w.add_class_aged(19);
    let team = Team::seat(&w).unwrap();
    let p = w.seat.and_then(|s| s.program).unwrap();
    let i = (0..w.people.len()).find(|&i| w.people[i].stage == Stage::HighSchool).unwrap();
    let year = w.year;
    let q = &mut w.people[i];
    q.stage = Stage::College;
    q.college = Some(p);
    q.class_of = year - 1;
    q.college_seasons = 1;
    q.last_role = Some(PlayingTime::Starter);
    w.programs[p].roster.push(i);
    let id = w.people[i].player.id;
    let today = w.today;
    w.training.hurt.push(Hurt { id, kind: None, from: today, until: Date(today.0 + 21), how: "practice".into() });
    (w, team, id)
}

fn trainer(w: &World, team: Team) -> Them {
    office::cast_them(w, CastRef::new(team, CastRole::Trainer, 0))
}

#[test]
fn the_doctor_reads_the_chart_and_never_clears_a_man_the_chip_wouldnt() {
    let (mut w, team, id) = hurt();
    let doc = trainer(&w, team);
    assert!(doctor::is_doctor(&w, doc));
    let k = doctor::knows(&w, doc, Topic::Injury, Some(Them::Player(id)));
    assert_eq!(k.len(), 1, "he knows the man's chart");
    assert!(k[0].words.contains("out"), "{}", k[0].words);
    assert!(k[0].day.is_some(), "the day he's back is on it");
    // Words never clear a man the chip wouldn't: a staff with good
    // judgment never rushes him.
    let clear = asks::willing(&w, doc, &Request::Clear(id)).unwrap();
    if !crate::world::folk::life::would_rush(&w, team) {
        assert_eq!(clear, Willing::No);
        assert_eq!(doctor::clear_score(&w, doc, team), 0.0);
    } else {
        let yes = doctor::clear_score(&w, doc, team) >= doctor::CLEARS;
        assert_eq!(clear == Willing::Yes, yes);
    }
    // A second opinion runs the meeting's own: his trust in you grows.
    assert_eq!(asks::willing(&w, doc, &Request::SecondOpinion(id)), Some(Willing::Yes));
    let i = crate::world::folk::person(&w, id).unwrap();
    let before = w.people[i].trust;
    let ran = office::parts().acting.run(&mut w, doc, &Request::SecondOpinion(id));
    assert!(!ran.done.is_empty());
    assert!(w.people[i].trust > before);
}

#[test]
fn a_reference_call_puts_his_read_on_the_coachs_page_and_you_owe_him() {
    let mut w = seat("college");
    let found = (0..w.coaches.len()).find_map(|c| coach_look::sources(&w, c).into_iter().next().map(|s| (c, s.1)));
    let Some((c, src)) = found else { return };
    let owed = w.folk.network.ledger.len();
    let id = refs::start(&mut w, c, &src.code()).unwrap();
    assert_eq!(w.folk.network.ledger.len(), owed + 1, "you owe him for the call");
    let him = refs::them(&w, src);
    let s = scene::open(&mut w, id).unwrap();
    assert_eq!(office::across(&w, &s), Some(him));
    assert!(s.picks.iter().any(|p| p.code == sit::REF));
    // A straight answer about his football: one read per facet, each
    // inside 1-20, kept on the coach's page with its facet.
    let reading = Reading {
        acts: vec![Act::Question { topic: Topic::Football, about: Some(Them::Coach(c)), words: String::new() }],
        ..Reading::default()
    };
    let told = Told { answer: Answer::Straight, released: vec!["he's good".into()], ..Told::default() };
    let t = Turn { told: vec![told], ..Turn::default() };
    if refs::agenda(&w, him, c).is_none() {
        return;
    }
    let had = coach_look::look_mut(&mut w, c).asked.len();
    let out = refs::after(&mut w, id, him, &reading, &t);
    let n = refs::facets(&w, c, Topic::Football).len();
    let asked = &coach_look::look_mut(&mut w, c).asked;
    assert_eq!(asked.len() - had, n);
    assert!(asked.iter().skip(had).all(|a| a.facet.is_some()));
    assert_eq!(out.len(), 1);
    for f in refs::facets(&w, c, Topic::Football) {
        let r = refs::read(&w, him, c, f).unwrap();
        assert!((1.0..=20.0).contains(&r));
    }
}

#[test]
fn a_line_to_someone_in_your_building_is_held_inside_the_chips() {
    let (w, team, _) = hurt();
    let doc = trainer(&w, team);
    let room = office::room(&w, &scene::Scene::new(scene::Kind::Trainer, w.today, "x")).unwrap();
    assert_eq!(room.present[0].who, doc);
    let hit = |trust: f32| Sized {
        hit: Hit { on: doc, item: Item::HisWord, how: How::Respected },
        trust,
        mood: 0.1,
        faded: false,
    };
    let mut up = vec![hit(0.08), hit(0.04)];
    office::cap(&w, &room, &mut up);
    let sum: f32 = up.iter().map(|h| h.trust).sum();
    assert!((sum - office::CAP_UP).abs() < 1e-6, "{sum}");
    assert!(up.iter().all(|h| h.mood == 0.0), "no mood for someone in your building");
    let mut down = vec![hit(-0.3)];
    office::cap(&w, &room, &mut down);
    assert!((down[0].trust - office::CAP_DOWN).abs() < 1e-6);
}

#[test]
fn trust_moves_where_the_world_keeps_it() {
    let (mut w, team, _) = hurt();
    let doc = trainer(&w, team);
    let before = office::trust(&w, doc);
    assert!(office::nudge(&mut w, doc, 0.02));
    assert!((office::trust(&w, doc) - before - 0.02).abs() < 1e-5);
    let r = CastRef::new(team, CastRole::Trainer, 0);
    let key = cast::get(&w, r).key();
    assert!(w.folk.mine.trust_of(key).is_some(), "the store his meeting's chips move");
}

#[test]
fn your_boss_weighs_a_typed_money_ask_as_said_never_on_the_meetings_chip() {
    let w = seat("college");
    let team = Team::seat(&w).unwrap();
    let bossman = office::cast_them(&w, CastRef::new(team, cast::boss_role(team), 0));
    // What you put up, read from what you said the money is for.
    let line = "more NIL, and I'll be able to get a top 3 class in the conference";
    let case = Case::read(line, Level::College);
    assert_eq!(case.bar, Some(GoalKind::Recruiting(3)));
    assert_eq!(Case::read("give me nine wins", Level::College).bar, Some(GoalKind::Wins(9)));
    assert_eq!(Case::read("a 10-win season", Level::Nfl).bar, Some(GoalKind::Wins(10)));
    assert_eq!(Case::read("more for the staff", Level::College).bar, None);
    // The first number that is a bar, and only a bar said as one.
    assert_eq!(Case::read("from a top 40 class to a top 3 class", Level::College).bar, Some(GoalKind::Recruiting(3)));
    assert_eq!(Case::read("win 1 more game, get us to 8 wins", Level::College).bar, Some(GoalKind::Wins(8)));
    assert_eq!(Case::read("we lost 5 games last year", Level::College).bar, None);
    assert_eq!(Case::read("on arrival", Level::College).bar, None);
    // What the money is for is named first; a class you put up is a bar.
    assert_eq!(asks::money_kind(line), AskKind::NilMoney);
    assert_eq!(asks::money_kind("more for my staff, and a top-3 recruiting class"), AskKind::StaffMoney);
    assert_eq!(asks::money_kind("more NIL money, and we'll fill the stadium"), AskKind::NilMoney);
    assert_eq!(asks::money_kind("a bigger recruiting budget"), AskKind::RecruitingMoney);
    assert_eq!(asks::money_kind("vanilla"), AskKind::StaffMoney);
    // His answer in words is the weighed one, with its reasons, never the
    // meeting's canned line.
    let ask = Request::Money(line.into());
    let a = asked::money(&w, team, AskKind::NilMoney, &case);
    let wl = asks::willing(&w, bossman, &ask);
    assert_eq!(wl, Some(if a.yes { Willing::Yes } else { Willing::No }));
    let said = asks::why(&w, bossman, &ask);
    assert_eq!(said[0], format!("{} to more NIL and revenue-sharing money.", if a.yes { "Yes" } else { "No" }));
    assert!(a.yes || said.len() > 1, "a no comes with its reason: {said:?}");
    assert!(!said.iter().any(|l| l.contains("right now.\"") || l.contains("I can't do it")), "{said:?}");
}

#[test]
fn a_yes_holds_you_to_your_bar_and_a_no_is_kept_with_his_reasons() {
    let mut w = seat("college");
    let team = Team::seat(&w).unwrap();
    // A class's rank is held for the class being signed now: the season
    // after this one is the one the boss judges it by.
    let season = w.year + 1;
    let today = w.today;
    w.folk.goals.push(Goal { kind: GoalKind::Recruiting(6), season, deal: 0, met: None, set_on: today, talk: false });
    let p = w.seat.and_then(|s| s.program).unwrap();
    let pool = w.programs[p].collective.base;
    let lines = asked::take(&mut w, team, AskKind::NilMoney, &Case { bar: Some(GoalKind::Recruiting(3)) }, false);
    assert!(w.programs[p].collective.base > pool, "the money moves");
    let ours: Vec<&Goal> = w.folk.goals.iter().filter(|g| g.season == season).collect();
    assert!(
        ours.iter().any(|g| g.kind == GoalKind::Recruiting(3) && g.talk)
            && !ours.iter().any(|g| g.kind == GoalKind::Recruiting(6)),
        "{ours:?}"
    );
    assert!(lines.iter().any(|l| l.contains("hold you to it")), "{lines:?}");
    assert!(w.folk.asks.last().is_some_and(|x| x.granted && x.kind == AskKind::NilMoney));
    // The same money isn't found twice in a year: asked again it's a no
    // with its reason, and a second confirm moves nothing.
    let again = asked::money(&w, team, AskKind::NilMoney, &Case::default());
    assert_eq!(again, asked::Answer { yes: false, why: vec![asked::Why::Stretched(0)] });
    let base = w.programs[p].collective.base;
    let twice = asked::take(&mut w, team, AskKind::NilMoney, &Case::default(), false);
    assert_eq!(w.programs[p].collective.base, base, "{twice:?}");
    // A bar that asks no more than his own is nothing on the table.
    assert_eq!(
        asked::money(&w, team, AskKind::StaffMoney, &Case { bar: Some(GoalKind::Recruiting(5)) }),
        asked::money(&w, team, AskKind::StaffMoney, &Case::default())
    );
    // A no, with his reason on record; each no weighs on the next.
    let no = asked::Answer { yes: false, why: vec![asked::Why::Budget] };
    let line = asked::refuse(&mut w, team, AskKind::Facility, &Case::default(), &no, false);
    assert!(line.starts_with("You asked for a facility project: no.") && line.contains("budget"), "{line}");
    assert!(w.folk.asks.last().is_some_and(|x| !x.granted && x.words == line));
    let after_no = asked::money(&w, team, AskKind::Facility, &Case::default());
    assert!(after_no.yes || after_no.why.contains(&asked::Why::AskedNo(0)), "{after_no:?}");
}

#[test]
fn the_goals_meeting_keeps_a_bar_you_put_up_and_one_below_his_is_nothing() {
    let mut w = seat("college");
    let team = Team::seat(&w).unwrap();
    // Before he sets the season's goals, a bar is weighed against the one
    // he would set.
    let season = boss::goal_season(&w, team);
    w.folk.goals.retain(|g| g.season != season);
    let Some(n) =
        boss::draft_goals(&w, team).iter().find_map(|g| if let GoalKind::Wins(n) = g { Some(*n) } else { None })
    else {
        return;
    };
    assert_eq!(asked::on_table(&w, team, &Case { bar: Some(GoalKind::Wins(n)) }).bar, None);
    let more = GoalKind::Wins(n + 1);
    assert_eq!(asked::on_table(&w, team, &Case { bar: Some(more) }).bar, Some(more));
    // A yes holds it, and the goals meeting for that season keeps it in
    // place of his own of that kind.
    asked::take(&mut w, team, AskKind::StaffMoney, &Case { bar: Some(more) }, false);
    let mut s = scene::Scene::new(scene::Kind::BossMeeting, w.today, "The season's goals");
    s.c = 0;
    s.team = Some(team);
    boss::open(&mut w, &mut s);
    let wins: Vec<&Goal> =
        w.folk.goals.iter().filter(|g| g.season == season && matches!(g.kind, GoalKind::Wins(_))).collect();
    assert_eq!(wins.len(), 1, "{wins:?}");
    assert!(wins[0].talk && wins[0].kind == more, "{wins:?}");
    assert!(s.docs.iter().any(|d| d.rows.iter().any(|r| r.cells.iter().any(|c| c.contains("put up yourself")))));
}

#[test]
fn time_reads_the_goals_in_force() {
    let mut w = seat("college");
    let team = Team::seat(&w).unwrap();
    // Before he sets any goals there is nothing to settle.
    w.folk.goals.clear();
    assert_eq!(asked::time(&w, team), asked::Answer { yes: false, why: vec![asked::Why::NoBar] });
    // This season's goals, past the season's first month: time reads them,
    // not next season's (none yet).
    let season = w.year;
    w.today = w.dates.fall_camp.add(45);
    let today = w.today;
    w.folk.goals.push(Goal { kind: GoalKind::Wins(9), season, deal: 0, met: None, set_on: today, talk: false });
    assert_eq!(boss::goal_season(&w, team), w.year + 1);
    assert_eq!(asked::in_force(&w, team), Some(season));
    let line = asked::settle_time(&mut w, team, &asked::Answer { yes: true, why: Vec::new() }, false);
    assert!(w.folk.goals.iter().any(|g| g.season == season && g.kind == GoalKind::Wins(7) && g.deal == -1), "{line}");
    // Asked again, it was already answered.
    assert_eq!(asked::time(&w, team).why, vec![asked::Why::TimeAsked]);
}

#[test]
fn a_booster_puts_money_in_once_a_year() {
    let mut w = seat("college");
    let team = Team::seat(&w).unwrap();
    let Some(r) = cast::all(&w, team, CastRole::Booster).first().map(|c| c.r) else { return };
    let him = office::cast_them(&w, r);
    let c = cast::get(&w, r);
    let p = w.seat.and_then(|s| s.program).unwrap();
    assert_eq!(asked::gave_lately(&w, &c), None);
    let base = w.programs[p].collective.base;
    let ran = office::parts().acting.run(&mut w, him, &Request::Fund);
    assert!(w.programs[p].collective.base > base, "{:?}", ran.done);
    assert_eq!(asked::gave_lately(&w, &c), Some(0));
    assert_eq!(asked::fund(&w, team, &c, 0.9), asked::Answer { yes: false, why: vec![asked::Why::Gave(0)] });
    // A second yes puts nothing more in, and your boss's own NIL record
    // doesn't read the booster's gift as his.
    let once = w.programs[p].collective.base;
    office::parts().acting.run(&mut w, him, &Request::Fund);
    assert_eq!(w.programs[p].collective.base, once);
    assert_eq!(asked::granted(&w, AskKind::NilMoney), None);
}

#[test]
fn a_sharp_boss_doubts_a_bar_your_record_doesnt_back() {
    let w0 = seat("college");
    let team = Team::seat(&w0).unwrap();
    let Team::College(p) = team else { return };
    let mut w = w0;
    // Your last class ranked last in the conference.
    let conf = w.programs[p].conference.clone();
    let year = w.year;
    let mut n = 0;
    for (k, q) in w.programs.iter_mut().enumerate() {
        if q.conference == conf {
            q.class_scores.retain(|x| x.0 != year);
            q.class_scores.push((year, if k == p { 0.0 } else { 100.0 + k as f64 }));
            n += 1;
        }
    }
    let b = boss::boss(&w, team);
    if n < 8 || b.judgement < 6.0 {
        return;
    }
    assert_eq!(boss::class_rank(&w, p, year), Some(n));
    let a = asked::money(&w, team, AskKind::NilMoney, &Case { bar: Some(GoalKind::Recruiting(1)) });
    if a.yes {
        // He may still say yes, and then he holds you to what you put up.
        assert_eq!(a.why, vec![asked::Why::Bar(GoalKind::Recruiting(1))]);
    } else {
        assert!(matches!(a.why[0], asked::Why::Doubts(GoalKind::Recruiting(1), Some(r)) if r == n), "{:?}", a.why);
    }
}

#[test]
fn a_coordinator_moves_the_plan_by_the_staff_meetings_step() {
    let mut w = seat("college");
    let oc = (0..w.coaches.len()).find(|&c| {
        asks::on_your_staff(&w, c) && w.coaches[c].job.is_some_and(|j| j.role == Role::OffensiveCoordinator)
    });
    let Some(oc) = oc else { return };
    let him = Them::Coach(oc);
    assert_eq!(asks::willing(&w, him, &Request::Plan("run the ball more".into())), Some(Willing::Yes));
    assert_eq!(asks::willing(&w, him, &Request::Plan("bring more pressure".into())), Some(Willing::No));
    let mut by_chip = seat("college");
    crate::world::folk::dialogue::shift_plan(&mut by_chip, 2, 1.0);
    office::parts().acting.run(&mut w, him, &Request::Plan("run the ball more".into()));
    assert_eq!(w.plans.guide.run.to, by_chip.plans.guide.run.to);
}

#[test]
fn a_sit_down_with_no_key_plays_as_chips_and_ends_when_you_end_it() {
    let (mut w, team, id) = hurt();
    let doc = trainer(&w, team);
    let sid = sit::start(&mut w, doc, None).unwrap();
    let s = scene::open(&mut w, sid).unwrap();
    assert!(s.picks.iter().any(|p| p.code == sit::INJURY), "a chip for the hurt man");
    assert_eq!(sit::hurt_men(&w, &s), vec![id]);
    let lines = s.lines.len();
    let s = scene::answer(&mut w, sid, sit::INJURY).unwrap();
    assert!(s.lines.len() > lines, "he answers");
    assert!(!s.done);
    assert!(!s.picks.is_empty(), "the chips are back");
    assert!(store::open(&w, sid).is_some(), "the talk is open");
    let s = scene::answer(&mut w, sid, sit::END).unwrap();
    assert!(s.done);
    assert!(store::open(&w, sid).is_none());
}

#[test]
fn a_typed_line_to_the_doctor_plays_on_the_office_parts() {
    let (mut w, team, id) = hurt();
    let doc = trainer(&w, team);
    let sid = sit::start(&mut w, doc, None).unwrap();
    let s = scene::open(&mut w, sid).unwrap();
    let room = office::room(&w, &s).unwrap();
    store::begin(&mut w, sid, room.clone());
    let reading = Reading {
        acts: vec![Act::Question { topic: Topic::Injury, about: Some(Them::Player(id)), words: String::new() }],
        ..Reading::default()
    };
    let conv = Conversation {
        parts: office::parts(),
        said: Said { scene: sid, words: "Where's he at?".into(), reading, promises: Vec::new() },
    };
    let t = conv.turn(&w, &room, &conv.said.reading);
    assert_eq!(t.told.len(), 1);
    conv.apply(&mut w, &t);
    assert!(store::open(&w, sid).is_some_and(|o| o.lines == 1));
}

#[test]
fn a_goal_in_your_assistants_deal_is_a_clause_on_the_season() {
    use crate::world::folk::persona::terms::{DealKind, Offer};
    use crate::world::folk::terms::On;
    let mut w = seat("college");
    let c =
        (0..w.coaches.len()).find(|&c| asks::on_your_staff(&w, c) && w.folk.staff.deals.iter().any(|d| d.coach == c));
    let Some(c) = c else { return };
    let wins = crate::world::folk::terms::read(None, "wins", Some(9.0), false).unwrap();
    assert!(wins.season);
    let o = Offer { kind: DealKind::Staff, terms: vec![wins], ..Offer::default() };
    let out = office::deals::agree(&mut w, 0, Them::Coach(c), &o).unwrap();
    assert!(!out.is_empty());
    let cl = w.folk.clauses.last().expect("the goal is on the books");
    assert_eq!(cl.on, On::Staff);
    assert_eq!(cl.coach, Some(c));
    assert!(cl.pays > 0.0);
}

/// His patience after each of `n` lines asking the doctor the same thing,
/// with your Relationships at `rel`.
fn grind(rel: f64, n: usize) -> Vec<f32> {
    let (mut w, team, id) = hurt();
    let you = w.career.as_ref().unwrap().coach;
    w.coaches[you].ability.relationships = rel;
    let doc = trainer(&w, team);
    let sid = sit::start(&mut w, doc, None).unwrap();
    let s = scene::open(&mut w, sid).unwrap();
    let room = office::room(&w, &s).unwrap();
    store::begin(&mut w, sid, room.clone());
    let mut out = Vec::new();
    for _ in 0..n {
        let reading = Reading {
            acts: vec![Act::Question { topic: Topic::Injury, about: Some(Them::Player(id)), words: String::new() }],
            ..Reading::default()
        };
        let conv = Conversation {
            parts: office::parts(),
            said: Said { scene: sid, words: "Where's he at?".into(), reading, promises: Vec::new() },
        };
        let t = conv.turn(&w, &room, &conv.said.reading);
        conv.apply(&mut w, &t);
        out.push(t.patience.0);
        if store::open(&w, sid).is_none() {
            break;
        }
    }
    out
}

#[test]
fn asking_the_same_thing_again_wears_his_patience_by_your_relationships() {
    use crate::world::folk::persona::talk::feel::repeat_wear;
    assert!((repeat_wear(20.0) - 0.5).abs() < 1e-4);
    assert!((repeat_wear(1.0) - 2.0).abs() < 1e-4);
    assert!((repeat_wear(10.5) - 1.0).abs() < 1e-4);
    assert!(repeat_wear(15.0) < 1.0 && repeat_wear(5.0) > 1.0);
    let mid = grind(10.5, 4);
    eprintln!("the same question four times, his patience after each line: {mid:?}");
    // The first answer gets somewhere; every repeat wears him down.
    assert!(mid.len() >= 3, "{mid:?}");
    for k in 2..mid.len() {
        assert!(mid[k] < mid[k - 1], "a repeat wears his patience: {mid:?}");
    }
    let (top, bottom) = (grind(20.0, 4), grind(1.0, 4));
    let fall = |p: &[f32]| p[0] - p[p.len().min(3) - 1];
    eprintln!("Relationships 20: {top:?}; 1: {bottom:?}");
    assert!(fall(&top) < fall(&mid) && fall(&mid) < fall(&bottom), "{top:?} {mid:?} {bottom:?}");
    // Half as fast at the top, twice as fast at the bottom.
    assert!((fall(&top) / fall(&mid) - 0.5).abs() < 0.05, "{top:?} {mid:?}");
    assert!((fall(&bottom) / fall(&mid) - 2.0).abs() < 0.1 || bottom.len() < 3, "{bottom:?} {mid:?}");
}
