//! Talks replayed on the real model, for a thread with a key: each facet in
//! `AFM_REPLAY` (comma separated; every facet when empty) plays its script,
//! or the lines in `AFM_REPLAY_LINES` (split on `|`), and prints each line
//! with his answer and "Heard as". `AFM_REPLAY_CARDS=1` prints the voice's
//! and the ear's cards too, `AFM_REPLAY_EAR=1` the ear's reading. Ignored,
//! so CI never runs it:
//! `AFM_AI_KEY="$AFM_LINEBANK_KEY" cargo test --release -p afm-game replay_ -- --ignored --nocapture --test-threads 1`

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use afm_ai::{Failure, Key, Model, Real, Reply, Request};
use afm_engine::world::background::Decider;
use afm_engine::world::folk::cast::{CastRef, Role};
use afm_engine::world::folk::persona::talk::standing;
use afm_engine::world::folk::persona::{Them, office};
use afm_engine::world::folk::scene::{self, Kind, Scene};
use afm_engine::world::folk::{Team, facts, takeover};
use serde_json::{Value, json};

use super::recruit_tests as rt;
use super::tests::{game, ok, talk};
use crate::Game;

/// The candidate's ask in a hiring replay: the scripts' offers ("{a70}",
/// 70% of it) are sized to the man the tiny world draws, as Cain's 2.3 and
/// 2.6 million were to Donte's 3.1.
static ASK: Mutex<f64> = Mutex::new(0.0);

fn sized(line: &str) -> String {
    let ask = *ASK.lock().unwrap();
    let mut out = line.to_string();
    for (k, x) in [("{a70}", 0.7), ("{a75}", 0.75), ("{a85}", 0.85), ("{a90}", 0.9), ("{a95}", 0.95)] {
        let n = (ask * x / 10_000.0).round() * 10_000.0;
        out = out.replace(k, &afm_engine::world::folk::persona::terms::money_words(n).replace('$', ""));
    }
    out
}

/// The real model, every request and reply kept.
struct Kept {
    real: Real,
    log: Mutex<Vec<(String, String, String)>>,
}

impl Model for Kept {
    fn call(&self, req: &Request) -> Result<Reply, Failure> {
        let r = self.real.call(req);
        let said = match &r {
            Ok(x) => x.text.clone(),
            Err(e) => format!("{e:?}"),
        };
        self.log.lock().unwrap().push((req.label.clone(), req.user.clone(), said));
        r
    }
}

fn wait(g: &mut Game, ticket: u64) -> Value {
    let t = Instant::now();
    loop {
        let v = ok(g, "ai_poll", json!({ "ticket": ticket }));
        if v["state"] != "wait" || t.elapsed() > Duration::from_secs(120) {
            return v;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

/// A real room for a team talk: `n` more men on your roster, a position
/// group each in turn, a third of them starting, two captains (one a side).
fn fill_room(w: &mut afm_engine::world::World, n: usize) {
    use afm_engine::player::PlayingTime;
    use afm_engine::positions::Position;
    use afm_engine::world::Stage;
    let p = w.seat.and_then(|s| s.program).unwrap();
    w.add_class_aged(19);
    let year = w.year;
    let spots = [
        Position::QB,
        Position::RB,
        Position::WR,
        Position::TE,
        Position::OT,
        Position::IOL,
        Position::IOL,
        Position::EDGE,
        Position::DT,
        Position::LB,
        Position::CB,
        Position::S,
    ];
    let v: Vec<usize> = (0..w.people.len()).filter(|&i| w.people[i].stage == Stage::HighSchool).take(n).collect();
    for (k, &i) in v.iter().enumerate() {
        let q = &mut w.people[i];
        q.stage = Stage::College;
        q.college = Some(p);
        q.class_of = year - 1 + (k % 4) as i32;
        q.college_seasons = (3 - (k % 4)) as u32;
        q.last_role = Some(if k % 3 == 0 { PlayingTime::Starter } else { PlayingTime::Backup });
        q.player.position = spots[k % spots.len()];
        q.trust = 0.5;
        w.programs[p].roster.push(i);
    }
    w.programs[p].captains = vec![v[0], v[7]];
}

/// The scripts: what a coach would really say, in order.
fn script(facet: &str) -> &'static str {
    match facet {
        "ad_goals" => {
            "Good to meet you. What are you looking for from us this year?|I plan on using local recruiting to find unique body compositions, we'll churn out draft pick after draft pick. We're gonna go to a bowl game next year|We're gonna sign a top-6 class in the conference|What do you need from me on the budget side?|Sounds good. Anything else before I go?"
        }
        "ad" => {
            "Thanks for making time. How do you think we're doing so far?|I think the young guys are coming along. I'd like a bigger recruiting budget though.|What would it take for you to sign off on that?|Fair enough. We'll beat our rival this year, you can write that down.|Thanks, I'll let you get back to it."
        }
        "booster" => {
            "Appreciate you coming by. How's the family?|I wanted to say thanks for everything you do for this program.|We're going to beat our rival this year.|What would you like to see from us this season?|Alright, I'll see you at the game Saturday."
        }
        "player" => {
            "How are you doing, really?|I know you want more snaps. Tell me what you think you need to work on.|You'll get your shot, but you have to earn it in practice.|Anything going on at home I should know about?|Good talk. My door's always open."
        }
        "team" => {
            "Listen up. Last week wasn't good enough and you all know it.|We go back to basics this week: tackling, ball security, effort.|Any questions?|Alright, let's go to work."
        }
        "staff" => {
            "Got a minute? How do you feel about your guys right now?|What's the biggest thing holding the unit back?|I want you to spend more time with the young corners this week.|How's your family settling in?|Thanks, good work this week."
        }
        "doctor" => {
            "How are the guys holding up physically?|Anyone I should be worried about for Saturday?|Can we get the banged-up guys some extra treatment this week?|Thanks, keep me posted."
        }
        "recruit_home" => {
            "Thanks for having me in your home. How's his year going?|We see him as a big part of what we're building. What matters most to you in this decision?|He'll get every chance to compete for a starting job as a freshman.|What questions do you have for me?|I appreciate your time tonight."
        }
        "recruit_call" => {
            "Hey, it's Coach. How was your game Friday?|We loved your film. What are you looking for in a school?|We'd love to get you on campus for a visit.|Who else is recruiting you hardest right now?|Alright, talk soon."
        }
        // You are the high school coach: the college coach called you.
        "hs_call" => {
            "Hey Coach, good to hear from you. He'll qualify, his grades are fine.|He's a great kid. Shows up early, takes coaching, never a problem.|What's your staff thinking about him?|Thanks for calling, let me know if you need anything else."
        }
        "senior" => {
            "What's on your mind?|Honestly, I think you're a lottery pick.|What are you hearing from the scouts?|What do you want to get done before the draft?|I'll make some calls for you. Proud of you."
        }
        // Cain's talk with his DT (2026-11-16): told he's the first pick at
        // DT, he still asked whether he had a real role.
        "dt" => {
            "How are you feeling about where you're at right now?|How does being a captain sound?|You're already my first pick at DT, but i need more pressures from you. I need you to make the QB uncomfortable, scare the fuck out of him|Good. Keep that up and we'll be fine."
        }
        // The same DT in your fifth year with him: a promise you kept, a
        // talk he remembers, a ruling (Cain 2026-11-16: "it should always
        // check it's relation with me when speaking to me").
        "veteran" => {
            "How are you feeling about where you're at right now?|Remember what we talked about last time?|I need the same from you this year: lead that D-line."
        }
        "oc" => {
            "I've watched the tape from last season. The offense wasn't good enough.|So convince me to let you stay OC.|Why did the run game fall apart late in the year?|What would you change with this group?|I'm letting you go. I appreciate what you did here."
        }
        // Cain's five talks (2026-11-19, talk replies lane A): what each
        // should now get right.
        // A promise that his place wouldn't change, then "you'll be the
        // last": the promise breaks on the spot and he raises it (item 6).
        "sanders" => {
            "Hey kid, how's the family ?|I'll be straight, you haven't progressed as i expected. I've been watching the practice film and it's all decent, but there is nothing popping out to me. I still have trust in you but you'll be the last on the depth chart|since you're athletic numbers are sub-par. You needs to make huge strides as a pocket passer. To start we need your arm strength to improve ASAP|Your arm has to be rated 70, at least if you want a future here"
        }
        // Promises made in this talk stay this talk's (item 5), and he takes
        // them by how much he believes you (item 2).
        "lampkins" => {
            "How are you feeling about where you're at?|I'm not writing you off, but someone with elite hands like yours but not the fastest can be incredibly elite at slot position. So here is my thinking, i want you to be our starting slot, we'll move you from z and x so we can get burners there instead. You'll be the most consistent slot in college|You'll start on slot|You'll see in the winter workouts, you're going to be a star with us"
        }
        // He names his number (item 4) and a yes to it is your offer on his
        // terms (item 3).
        "donte" => {
            "Thanks for coming in. Tell me what you'd do with our offense.|Dynamic West Coast, we moved our best receiver from an outside guy to the slot, he has the most secure hands in college, we put him on a focus to improve his release speed. If you were to sign i'll find you a 6'2+ burner who runs at least a < 4.45|Let me know where your minds at|How does {a70} sound ?|Okay give me a ball-park|How does {a85} sound ?|Okay, {a95} and you're my coordinator. Deal?"
        }
        "donte_yes" => {
            "Thanks for coming in. What would it take to get you here?|How does {a70} sound ?|Okay give me a ball-park|Alright, you've got it. Your number works for me."
        }
        // A high school coach in an interview wants his players recruited as
        // part of the hire and names them (item 7); a yes counts (item 3).
        "merez" => {
            "Thanks for coming in. What are you looking for in this job?|Depends on your family, i see south carolina as a rebuild. A dynasty, i'm planning on being here a long time and winning championships. And South carolina is not a bad place to celebrate winning in hahaha|They'll get free scholarships if that's what you mean|Considere it done|which of the kids do you feel will imrpove our roster?"
        }
        // Talk replies lane B (items 13-15): what you say you'll do runs
        // after one confirm; his number comes down round by round; the read
        // of what he's after sharpens.
        "lampkins_work" => {
            "How are you feeling about where you're at?|I'm putting you on a release project this winter|And i want a focus on your route breaks this week|We'll move you to slot, you'll be our starting slot"
        }
        "donte_haggle" => {
            "Thanks for coming in. What would it take to get you here?|Okay give me a ball-park|How does {a75} sound ?|I can go to {a85}|{a90}, that's as far as I can go right now|Alright, {a95}. Deal?"
        }
        _ => "Hello.|How are you?",
    }
}

/// Opens the facet's talk: (the game, the talk's id).
fn open(facet: &str) -> Option<(Game, u32)> {
    let cast = |g: &mut Game, role: Role| -> Option<u32> {
        let r = {
            let w = g.world.as_mut().unwrap();
            let team = Team::seat(w).unwrap();
            office::cast_them(w, CastRef::new(team, role, 0))
        };
        let Them::Cast(r, _) = r else { return None };
        let v = ok(g, "people_sitdown", json!({ "cast": r.token() }));
        v["id"].as_u64().map(|x| x as u32)
    };
    match facet {
        "ad_goals" => {
            let (mut g, _, _) = game("replay-ad-goals", true);
            let w = g.world.as_mut().unwrap();
            let mut s = Scene::new(Kind::BossFirst, w.today, "Meeting the athletic director");
            s.team = Team::seat(w);
            s.stops = false;
            s.place = "His office".into();
            let sid = scene::add(w, s);
            ok(&mut g, "people_scene", json!({ "id": sid }));
            Some((g, sid))
        }
        "ad" | "booster" | "doctor" => {
            let (mut g, _, _) = game(&format!("replay-{facet}"), true);
            let role = match facet {
                "ad" => Role::AthleticDirector,
                "booster" => Role::Booster,
                _ => Role::Trainer,
            };
            let id = cast(&mut g, role)?;
            Some((g, id))
        }
        "player" => {
            let (mut g, _, him) = game("replay-player", true);
            let id = talk(&mut g, him, "role");
            Some((g, id))
        }
        "team" => {
            let (mut g, _, _) = game("replay-team", true);
            fill_room(g.world.as_mut().unwrap(), 44);
            let sid = ok(&mut g, "people_team_talk", json!({}))["scene"].as_u64().unwrap() as u32;
            ok(&mut g, "people_scene", json!({ "id": sid }));
            Some((g, sid))
        }
        "staff" => {
            let (mut g, _, _) = game("replay-staff", true);
            let c = {
                let w = g.world.as_ref().unwrap();
                let team = Team::seat(w).unwrap();
                (0..w.coaches.len()).find(|&k| {
                    w.coaches[k].job.is_some_and(|j| Team::from_employer(j.employer) == team)
                        && afm_engine::world::folk::persona::makeup::your_coach(w) != Some(k)
                })?
            };
            let v = ok(&mut g, "people_sitdown", json!({ "coach": c }));
            Some((g, v["id"].as_u64()? as u32))
        }
        "recruit_home" => {
            let (g, _, sid, ..) = rt::recruit_talk("replay-home", Kind::HomeVisit, Decider::Parent);
            Some((g, sid))
        }
        "recruit_call" => {
            let (g, _, sid, ..) = rt::recruit_talk("replay-call", Kind::RecruitCall, Decider::Himself);
            Some((g, sid))
        }
        "hs_call" => {
            let (g, _, sid, _) = rt::college_call("replay-hs", 2.5);
            Some((g, sid))
        }
        "senior" => {
            // A starter whose college career ends this year (Cain
            // 2026-11-16: Ryan Wyatt, no eligibility left, asked about the
            // transfer window and for a role).
            let (mut g, _, him) = game("replay-senior", true);
            {
                let w = g.world.as_mut().unwrap();
                let i = afm_engine::world::folk::person(w, him).unwrap();
                let year = w.year;
                let p = &mut w.people[i];
                p.class_of = year - 4;
                p.college_seasons = 4;
                p.last_role = Some(afm_engine::player::PlayingTime::Starter);
                if standing::seasons_after(w, i) > 0 {
                    w.people[i].class_of -= 1;
                    w.people[i].college_seasons += 1;
                }
                println!(
                    "  [setup] today {:?}, road {:?}, standing: {}",
                    w.today,
                    standing::road(w, i),
                    standing::words(w, Them::Player(him)).join(" | ")
                );
            }
            let id = talk(&mut g, him, "future");
            Some((g, id))
        }
        "dt" => {
            // A backup last season, in the offseason: nothing this season
            // shows him his place yet.
            let (mut g, _, him) = game("replay-dt", true);
            {
                let w = g.world.as_mut().unwrap();
                let i = afm_engine::world::folk::person(w, him).unwrap();
                w.people[i].last_role = Some(afm_engine::player::PlayingTime::Backup);
                println!("  [setup] standing: {}", standing::words(w, Them::Player(him)).join(" | "));
                println!(
                    "  [setup] you and the coach: {}",
                    afm_engine::world::folk::persona::talk::between::words(w, Them::Player(him)).join(" | ")
                );
            }
            let id = talk(&mut g, him, "role");
            Some((g, id))
        }
        "veteran" => {
            // Your fifth year: the seasons are yours, a starting job you
            // promised and kept, the last talk word for word, a curfew
            // ruling, and he trusts you.
            let (mut g, _, him) = game("replay-veteran", true);
            {
                use afm_engine::world::folk::rules::{Response, RuleKind, Ruling};
                use afm_engine::world::folk::scene::{Said, Who};
                use afm_engine::world::folk::talks::{Promise, PromiseKind, PromiseTo};
                let w = g.world.as_mut().unwrap();
                let today = w.today;
                let me = w.career.as_ref().unwrap().coach;
                w.folk.mine.since = Some(today.add(-4 * 365 - 30));
                if let Some(Team::College(p)) = Team::seat(w) {
                    let year = w.year;
                    for x in w.programs[p].seasons.iter_mut().filter(|x| x.year >= year - 4) {
                        x.coach = Some(me);
                    }
                }
                let i = afm_engine::world::folk::person(w, him).unwrap();
                w.people[i].trust = 0.8;
                w.people[i].last_role = Some(afm_engine::player::PlayingTime::Starter);
                let mut pr = Promise::blank(PromiseTo::Player(him), PromiseKind::Start);
                pr.made = today.add(-300);
                pr.due = today.add(-200);
                pr.kept = Some(true);
                w.folk.promises.push(pr);
                let mut s = Scene::new(Kind::OneOnOne, today.add(-40), "A talk");
                s.who = vec![Who::Player(him)];
                s.lines = vec![
                    Said {
                        by: 1,
                        text: "I need you to be a leader in that room this year.".into(),
                        ..Default::default()
                    },
                    Said { by: 2, text: "I hear you, Coach. I'll do it.".into(), ..Default::default() },
                ];
                s.done = true;
                scene::add(w, s);
                w.folk.rules.rulings.push(Ruling {
                    date: today.add(-200),
                    id: him,
                    kind: RuleKind::Curfew,
                    response: Response::Running,
                    star: false,
                });
                println!(
                    "  [setup] you and the coach: {}",
                    afm_engine::world::folk::persona::talk::between::words(w, Them::Player(him)).join(" | ")
                );
            }
            let id = talk(&mut g, him, "role");
            Some((g, id))
        }
        "oc" => {
            // Cain's holdover coordinator meeting (2026-11-16): the bar for
            // a direct, coherent talk.
            let (mut g, _, _) = game("replay-oc", true);
            let w = g.world.as_mut().unwrap();
            let team = Team::seat(w).unwrap();
            let me = w.career.as_ref().unwrap().coach;
            let staff: Vec<usize> = facts::assistants(w, team).into_iter().map(|x| x.0).filter(|&c| c != me).collect();
            let oc = staff
                .iter()
                .copied()
                .find(|&c| {
                    w.coaches[c].job.is_some_and(|j| j.role == afm_engine::world::coach::Role::OffensiveCoordinator)
                })
                .or_else(|| staff.first().copied())?;
            let old = (0..w.coaches.len()).find(|&k| k != me && k != oc && w.coaches[k].job.is_none()).unwrap_or(0);
            let today = w.today;
            let t = w.folk.takeover.get_or_insert_with(takeover::Takeover::default);
            t.team = Some(team);
            t.started = Some(today);
            t.old_coach = Some(old);
            t.kept.clear();
            t.let_go.clear();
            w.folk.staff.interviewed.retain(|x| x.0 != oc);
            let mut s = Scene::new(Kind::Holdover, today, "Meeting the offensive coordinator");
            s.a = oc as u64;
            s.stops = false;
            s.place = "Your office".into();
            let sid = scene::add(w, s);
            ok(&mut g, "people_scene", json!({ "id": sid }));
            Some((g, sid))
        }
        "sanders" | "lampkins" | "lampkins_work" => {
            use afm_engine::world::folk::talks::{Promise, PromiseKind, PromiseTo};
            let (mut g, _, him) = game(&format!("replay-{facet}"), true);
            {
                let w = g.world.as_mut().unwrap();
                let i = afm_engine::world::folk::person(w, him).unwrap();
                let today = w.today;
                let p = &mut w.people[i];
                if *facet == *"sanders" {
                    p.player.position = afm_engine::positions::Position::QB;
                    p.trust = 0.4;
                    // Earlier: his place wouldn't change, due next winter.
                    let mut pr = Promise::blank(PromiseTo::Player(him), PromiseKind::Place(2));
                    pr.made = today.add(-60);
                    pr.due = today.add(300);
                    pr.belief = 0.5;
                    w.folk.promises.push(pr);
                } else {
                    p.player.position = afm_engine::positions::Position::WR;
                }
                println!(
                    "  [setup] you and the coach: {}",
                    afm_engine::world::folk::persona::talk::between::words(w, Them::Player(him)).join(" | ")
                );
            }
            let id = talk(&mut g, him, "role");
            Some((g, id))
        }
        "donte" | "donte_yes" | "donte_haggle" | "merez" => {
            // A coordinator at another school, or a high school head coach
            // (with players of his when the world has one).
            let (mut g, _, _) = game(&format!("replay-{facet}"), true);
            {
                // The coordinator's chair open, as it was for Donte.
                let w = g.world.as_mut().unwrap();
                if let Some(Team::College(p)) = Team::seat(w) {
                    w.programs[p].oc = None;
                }
                let hs = (0..w.coaches.len())
                    .filter(|&c| {
                        matches!(afm_engine::world::folk::persona::makeup::coach_team(w, c), Some(Team::High(_)))
                    })
                    .count();
                println!(
                    "  [setup] openings {:?}; high school coaches {hs}",
                    afm_engine::world::folk::staffing::openings(w)
                );
            }
            let (c, role) = {
                let w = g.world.as_ref().unwrap();
                let team = Team::seat(w).unwrap();
                let theirs = |c: usize| {
                    let t = afm_engine::world::folk::persona::makeup::coach_team(w, c);
                    t.is_some() && t != Some(team)
                };
                let open = afm_engine::world::folk::staffing::openings(w);
                let oc = afm_engine::world::coach::Role::OffensiveCoordinator;
                let role = if open.contains(&oc) { oc } else { *open.first()? };
                if *facet == *"merez" {
                    let hs = |c: usize| {
                        matches!(afm_engine::world::folk::persona::makeup::coach_team(w, c), Some(Team::High(_)))
                    };
                    let roster = |c: usize| {
                        afm_engine::world::folk::persona::makeup::coach_team(w, c).map_or(0, |t| t.roster(w).len())
                    };
                    let with_kids = (0..w.coaches.len())
                        .find(|&c| hs(c) && !afm_engine::world::folk::persona::coaching::his_players(w, c).is_empty());
                    (
                        with_kids
                            .or_else(|| (0..w.coaches.len()).find(|&c| hs(c) && roster(c) >= 2))
                            .or_else(|| (0..w.coaches.len()).find(|&c| hs(c)))?,
                        role,
                    )
                } else {
                    let oc = (0..w.coaches.len()).find(|&c| {
                        theirs(c)
                            && w.coaches[c]
                                .job
                                .is_some_and(|j| j.role == afm_engine::world::coach::Role::OffensiveCoordinator)
                    });
                    (oc.or_else(|| (0..w.coaches.len()).find(|&c| theirs(c)))?, role)
                }
            };
            let w = g.world.as_mut().unwrap();
            // A high school coach's best players, starred so he has some to
            // sell (the tiny world's are mostly unstarred).
            if let Some(t @ Team::High(h)) = afm_engine::world::folk::persona::makeup::coach_team(w, c) {
                // The tiny world's schools can be empty: two of its high
                // schoolers play for him.
                if t.roster(w).len() < 2 {
                    let kids: Vec<usize> = (0..w.people.len())
                        .filter(|&i| w.people[i].stage == afm_engine::world::Stage::HighSchool)
                        .take(2)
                        .collect();
                    w.high_schools[h].roster.extend(kids);
                }
                for (k, i) in t.roster(w).into_iter().take(2).enumerate() {
                    w.people[i].stars = 3 - k as u8;
                }
            }
            *ASK.lock().unwrap() = afm_engine::world::folk::hiring::ask(w, c, role);
            println!(
                "  [setup] {} ({:?}), interviewing for {role:?}; ask {:.0}; his players: {}",
                w.coaches[c].name,
                afm_engine::world::folk::persona::makeup::coach_team(w, c),
                *ASK.lock().unwrap(),
                afm_engine::world::folk::persona::coaching::his_players(w, c).len()
            );
            let id = afm_engine::world::folk::staffing::start_interview(w, c, role).ok()?;
            ok(&mut g, "people_scene", json!({ "id": id }));
            Some((g, id))
        }
        _ => None,
    }
}

const FACETS: &[&str] = &[
    "ad_goals",
    "ad",
    "booster",
    "player",
    "dt",
    "veteran",
    "senior",
    "oc",
    "team",
    "staff",
    "doctor",
    "recruit_home",
    "recruit_call",
    "hs_call",
    "sanders",
    "lampkins",
    "donte",
    "donte_yes",
    "merez",
    "lampkins_work",
    "donte_haggle",
];

#[test]
#[ignore = "calls the real API; run alone with AFM_AI_KEY set"]
fn replay_talks_on_the_real_model() {
    let raw = std::env::var("AFM_AI_KEY").unwrap_or_default();
    if raw.trim().is_empty() {
        println!("skipped: AFM_AI_KEY is empty");
        return;
    }
    let real = Real::new(Key::parse(&raw).expect("an Anthropic key"))
        .with_timeout(Duration::from_secs(60))
        .with_proxy_from_env();
    let kept = Arc::new(Kept { real, log: Mutex::new(Vec::new()) });
    let only = std::env::var("AFM_REPLAY").unwrap_or_default();
    let cards = std::env::var("AFM_REPLAY_CARDS").is_ok_and(|v| v == "1");
    let ears = std::env::var("AFM_REPLAY_EAR").is_ok_and(|v| v == "1");
    let given = std::env::var("AFM_REPLAY_LINES").ok();
    for facet in FACETS {
        if !only.is_empty() && !only.split(',').any(|x| x == *facet) {
            continue;
        }
        let Some((mut g, id)) = open(facet) else {
            println!("\n===== {facet}: could not open");
            continue;
        };
        g.ai.use_real_for_tests(kept.clone());
        let v = ok(&mut g, "people_scene", json!({ "id": id }));
        println!("\n===== {facet}: {}", v["title"].as_str().unwrap_or_default());
        if let Some(lines) = v["lines"].as_array() {
            for l in lines {
                println!(
                    "  [opening] {}: {}",
                    l["who"].as_str().unwrap_or_default(),
                    l["text"].as_str().unwrap_or_default()
                );
            }
        }
        let mut seen = v["lines"].as_array().map_or(0, Vec::len);
        let lines = given.clone().unwrap_or_else(|| script(facet).to_string());
        for line in lines.split('|').map(sized) {
            let line = line.as_str();
            let before = kept.log.lock().unwrap().len();
            let r = g.call("people_say", &json!({ "id": id, "text": line }));
            let Ok(r) = r else {
                println!("COACH: {line}\n  (refused: {})", r.err().unwrap_or_default());
                break;
            };
            let Some(t) = r["ticket"].as_u64() else {
                println!("COACH: {line}\n  (no ticket: {r})");
                break;
            };
            let v = wait(&mut g, t);
            println!("\nCOACH: {line}");
            println!("  HEARD AS: {}", v["heard"].as_str().unwrap_or_default());
            let lines = v["reply"]["lines"].as_array().cloned().unwrap_or_default();
            for l in lines.iter().skip(seen) {
                let who = l["who"].as_str().unwrap_or_default();
                let text = l["text"].as_str().unwrap_or_default();
                if text != line {
                    println!("  {who}: {text}");
                }
            }
            seen = lines.len();
            if let Some(n) = v["notes"].as_array() {
                for x in n {
                    println!("  NOTE: {x}");
                }
            }
            if let Some(c) = v["confirm"].as_array() {
                for x in c {
                    println!("  CONFIRM: {}", x["text"].as_str().unwrap_or_default());
                }
                // Lane B: the actions said are confirmed, and what the
                // screens did is printed.
                if facet.ends_with("_work") {
                    for x in c {
                        let Ok(r) = g.call("talk_confirm", &x["args"]) else { continue };
                        let lines = r["lines"].as_array().cloned().unwrap_or_default();
                        for l in lines.iter().skip(seen) {
                            println!("  CONFIRMED: {}", l["text"].as_str().unwrap_or_default());
                        }
                        seen = lines.len();
                    }
                }
            }
            let left = g.call("people_scene", &json!({ "id": id })).ok();
            if let Some(after) = left.as_ref().and_then(|v| v["talk"]["left"].as_array().cloned()) {
                for s in after.iter().filter(|s| s["label"] == "What he's after") {
                    println!("  AFTER: {}", s["paras"]);
                }
            }
            let log = kept.log.lock().unwrap();
            for (label, user, said) in &log[before..] {
                if cards {
                    println!("--- {label} card ---\n{user}");
                }
                if ears || label != "ear" {
                    println!("  [{label}] {said}");
                }
            }
            if v["state"] != "done" {
                println!("  STATE: {v}");
            }
            if v["ended"].as_bool() == Some(true) {
                println!("  (the talk ended)");
                break;
            }
        }
    }
}
