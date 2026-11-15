//! Live talks v3, lane N1: people (`world::folk::persona`, the files
//! `makeup`, `knowing`, `telling`, `wanting`, `feel`, `football`,
//! `stores`, `coming` and `people`). These prove that each person is read
//! from the store that holds him, that what he knows follows where he
//! stands (himself, his family, a close friend, a teammate who was there, a
//! stranger), that how he tells it follows his trust, loyalty, stake and
//! honesty, that heat and patience follow his makeup, that the stores fill
//! and trim, and that a rule break your staff missed is kept with the
//! teammates who saw it, with no world number moved.

use super::legacy::seat;
use crate::player::{PlayingTime, Trait};
use crate::world::folk::Team;
use crate::world::folk::life::{Event, Happening};
use crate::world::folk::persona::Them;
use crate::world::folk::persona::feel::{self, Stir};
use crate::world::folk::persona::football;
use crate::world::folk::persona::knows::{Answer, Knows, Source, TellParts, Tells, Topic};
use crate::world::folk::persona::makeup::{self, Makeup};
use crate::world::folk::persona::memory::{Passed, Treated};
use crate::world::folk::persona::people::People;
use crate::world::folk::persona::stores;
use crate::world::folk::persona::telling;
use crate::world::folk::persona::wants::{Boil, Heat, Kind, Patience, WantKind, Wanting};
use crate::world::folk::rules::RuleKind;
use crate::world::folk::{talks, ties};
use crate::world::{Stage, World};

/// Your college program with `n` sophomores on it, who trust you.
fn room(n: usize) -> (World, usize, Vec<usize>) {
    let mut w = seat("college");
    let p = w.seat.and_then(|s| s.program).unwrap();
    w.add_class_aged(19);
    let year = w.year;
    let you = w.career.as_ref().unwrap().coach as u32;
    let v: Vec<usize> = (0..w.people.len()).filter(|&i| w.people[i].stage == Stage::HighSchool).take(n).collect();
    for &i in &v {
        let q = &mut w.people[i];
        q.stage = Stage::College;
        q.college = Some(p);
        q.class_of = year - 1;
        q.college_seasons = 1;
        q.last_role = Some(PlayingTime::Backup);
        q.trust_in = you;
        q.trust = 0.6;
        w.programs[p].roster.push(i);
    }
    (w, p, v)
}

fn id(w: &World, i: usize) -> u32 {
    w.people[i].player.id
}

#[test]
fn each_kind_is_read_from_where_the_world_holds_him() {
    let (mut w, p, v) = room(3);
    let me = Them::Player(id(&w, v[0]));
    assert_eq!(makeup::kind(&w, me), Kind::Player);
    // A high school kid is a recruit; a family member is family.
    w.add_class_aged(17);
    let kid = (0..w.people.len()).find(|&i| w.people[i].stage == Stage::HighSchool).unwrap();
    assert_eq!(makeup::kind(&w, Them::Player(id(&w, kid))), Kind::Recruit);
    assert_eq!(makeup::kind(&w, Them::Kin(id(&w, kid), 0)), Kind::Family);
    // Another school's player in the portal is a transfer.
    let other = (0..w.programs.len()).find(|&j| j != p).unwrap();
    let t = v[2];
    w.programs[p].roster.retain(|&x| x != t);
    w.people[t].college = Some(other);
    w.people[t].cal.portal = Some(crate::world::PortalEntry {
        entered: w.today,
        decide_by: w.today.add(30),
        why: "playing time".into(),
        held: false,
        looking: false,
    });
    assert_eq!(makeup::kind(&w, Them::Player(id(&w, t))), Kind::Transfer);
    // Your coordinator is staff; your athletic director is your boss.
    let team = Team::seat(&w).unwrap();
    if let Some(oc) = w.programs[p].oc {
        assert_eq!(makeup::kind(&w, Them::Coach(oc)), Kind::Staff);
    }
    let ad = crate::world::folk::cast::CastRef::new(team, crate::world::folk::cast::Role::AthleticDirector, 0);
    assert_eq!(makeup::kind(&w, Them::Cast(ad, 0)), Kind::Boss);
    if !w.market.agents.is_empty() {
        assert_eq!(makeup::kind(&w, Them::Agent(0)), Kind::Agent);
    }
}

#[test]
fn makeup_is_the_stores_own_traits() {
    let (w, p, v) = room(1);
    let i = v[0];
    let m = makeup::makeup(&w, Them::Player(id(&w, i)));
    let t = &w.people[i].player.personality.traits;
    assert_eq!(m.temper, t.get(Trait::Temper));
    assert_eq!(m.loyalty, t.get(Trait::Loyalty));
    assert_eq!(m.ego, t.get(Trait::Ego));
    let team = Team::College(p);
    let r = crate::world::folk::cast::CastRef::new(team, crate::world::folk::cast::Role::AthleticDirector, 0);
    let c = crate::world::folk::cast::get(&w, r);
    let mc = makeup::makeup(&w, Them::Cast(r, c.term));
    assert_eq!(mc.honesty, c.honesty);
    assert_eq!(mc.patience, c.patience);
    // Drawn traits are steady: the same man twice is the same man.
    assert_eq!(makeup::makeup(&w, Them::Kin(id(&w, i), 1)), makeup::makeup(&w, Them::Kin(id(&w, i), 1)));
    for x in [m, mc] {
        for v in [x.temper, x.honesty, x.patience, x.iq, x.judgement, x.warmth] {
            assert!((1.0..=20.0).contains(&v));
        }
    }
}

#[test]
fn a_life_event_reaches_himself_and_his_family_never_a_stranger() {
    let (mut w, _, v) = room(3);
    let (a, b) = (v[0], v[1]);
    let today = w.today;
    w.folk.life.events.push(Happening { id: id(&w, a), event: Event::ParentIll, date: today, told: false });
    let ppl = People;
    let own = ppl.knows(&w, Them::Player(id(&w, a)), Topic::Life, None);
    assert!(own.iter().any(|k| k.from == Source::Himself), "he knows his own: {own:?}");
    let kin = ppl.knows(&w, Them::Kin(id(&w, a), 0), Topic::Life, Some(Them::Player(id(&w, a))));
    assert!(kin.iter().any(|k| k.from == Source::Family));
    // A teammate with no tie doesn't know it.
    let stranger = ppl.knows(&w, Them::Player(id(&w, b)), Topic::Life, Some(Them::Player(id(&w, a))));
    assert!(stranger.is_empty(), "a stranger knows nothing of it: {stranger:?}");
}

#[test]
fn a_friend_sees_the_why_only_from_a_close_tie() {
    let (mut w, _, v) = room(3);
    let (a, b, c) = (v[0], v[1], v[2]);
    w.people[a].mood = -0.6;
    w.people[a].mood_why = crate::world::people::Why::Role as u8;
    let (ib, ia, ic) = (id(&w, b), id(&w, a), id(&w, c));
    ties::bond(&mut w, ib, ia, 60);
    ties::bond(&mut w, ic, ia, 12);
    let close = People.knows(&w, Them::Player(id(&w, b)), Topic::Mood, Some(Them::Player(id(&w, a))));
    assert!(close.iter().any(|k| k.from == Source::Friend && k.words.contains("mostly")), "{close:?}");
    // A weaker friend (37) sees the mood, not the why.
    let loose = People.knows(&w, Them::Player(id(&w, c)), Topic::Mood, Some(Them::Player(id(&w, a))));
    assert!(loose.iter().any(|k| k.from == Source::Friend && !k.words.contains("mostly")), "{loose:?}");
}

#[test]
fn how_he_tells_it_follows_trust_loyalty_and_honesty() {
    let m = Makeup::default();
    let warm = TellParts { trust: 0.85, professionalism: 15.0, ..Default::default() };
    let cold = TellParts { trust: 0.2, professionalism: 6.0, ..Default::default() };
    let a = telling::odds(&m, Topic::Mood, false, &warm);
    let b = telling::odds(&m, Topic::Mood, false, &cold);
    assert!(a[0] > b[0], "trust opens him: {a:?} {b:?}");
    for o in [a, b] {
        assert!((o.iter().sum::<f64>() - 1.0).abs() < 1e-9);
    }
    // About a friend, a loyal man says less and covers more.
    let loyal = TellParts { trust: 0.6, professionalism: 10.0, loyalty: 0.9, ..Default::default() };
    let none = TellParts { trust: 0.6, professionalism: 10.0, loyalty: 0.0, ..Default::default() };
    let l = telling::odds(&m, Topic::RuleBreak, true, &loyal);
    let n = telling::odds(&m, Topic::RuleBreak, true, &none);
    assert!(l[0] < n[0] && l[3] > n[3], "loyalty covers: {l:?} {n:?}");
    // His own stake makes a liar of a dishonest man, not an honest one.
    let stake = TellParts { trust: 0.5, professionalism: 10.0, interest: 0.8, ..Default::default() };
    let liar = Makeup { honesty: 3.0, ..Makeup::default() };
    let straight = Makeup { honesty: 18.0, ..Makeup::default() };
    assert!(
        telling::odds(&liar, Topic::Nil, false, &stake)[3] > telling::odds(&straight, Topic::Nil, false, &stake)[3]
    );
    // A room makes a touchy topic harder.
    let alone = TellParts { trust: 0.6, professionalism: 10.0, ..Default::default() };
    let crowd = TellParts { room: 1.0, ..alone };
    assert!(telling::odds(&m, Topic::Case, false, &crowd)[0] < telling::odds(&m, Topic::Case, false, &alone)[0]);
}

#[test]
fn a_lie_is_logged_with_the_day_it_can_show_and_never_tells_the_truth() {
    let (mut w, _, v) = room(1);
    let me = Them::Player(id(&w, v[0]));
    let today = w.today;
    w.folk.life.events.push(Happening { id: id(&w, v[0]), event: Event::Death, date: today, told: false });
    let known = People.knows(&w, me, Topic::Life, None);
    assert!(!known.is_empty());
    // Force a liar: no trust, a big stake, through the parts.
    let parts = TellParts { trust: 0.0, professionalism: 1.0, interest: 1.0, ..Default::default() };
    let mut lied = false;
    for d in 0..60 {
        w.today = today.add(d);
        let told = People.tell(&w, me, &known, &parts);
        assert!(told.cost.minutes > 0.0);
        match told.answer {
            Answer::Lies => {
                let lie = told.lie.expect("a lie is logged");
                assert_eq!(lie.by, me);
                assert!(!told.released.iter().any(|r| r.contains("lost someone")), "the voice gets the lie only");
                lied = true;
            }
            Answer::Partly | Answer::Deflects => assert!(told.holding_back),
            Answer::Straight => assert!(told.released.iter().any(|r| r.contains("at home"))),
        }
    }
    assert!(lied, "a man with nothing but a stake lies some days");
    // He doesn't know: he says so.
    let none = People.tell(&w, me, &[], &parts);
    assert_eq!(none.answer, Answer::Straight);
}

#[test]
fn an_unseen_break_is_known_to_the_men_who_were_there() {
    let (mut w, _, v) = room(12);
    let a = v[0];
    let (i1, ia) = (id(&w, v[1]), id(&w, a));
    ties::bond(&mut w, i1, ia, 60);
    let before = (w.people[a].mood, w.people[a].trust);
    let day = w.today;
    stores::record_unseen(&mut w, a, RuleKind::Curfew, day);
    assert_eq!((w.people[a].mood, w.people[a].trust), before, "keeping it moves nothing");
    let u = w.folk.live.persona.unseen[0].clone();
    assert!(!u.there.is_empty() && u.there.len() <= stores::THERE_MOST);
    assert!(u.there.contains(&id(&w, v[1])), "his close friend was with him");
    for x in &u.there {
        let k = People.knows(&w, Them::Player(*x), Topic::RuleBreak, Some(Them::Player(id(&w, a))));
        assert!(k.iter().any(|k| k.words.contains("never saw")), "{k:?}");
    }
    let own = People.knows(&w, Them::Player(id(&w, a)), Topic::RuleBreak, None);
    assert!(own.iter().any(|k| k.from == Source::Himself));
    // Someone who wasn't there and isn't close knows nothing.
    let out = v.iter().copied().find(|&j| j != a && !u.there.contains(&id(&w, j))).unwrap();
    let k = People.knows(&w, Them::Player(id(&w, out)), Topic::RuleBreak, Some(Them::Player(id(&w, a))));
    assert!(k.iter().all(|k| !k.words.contains("never saw")));
}

#[test]
fn the_grapevine_travels_by_ties_a_hop_at_a_time() {
    let (mut w, _, v) = room(8);
    for k in 0..6 {
        let (x, y) = (id(&w, v[k]), id(&w, v[k + 1]));
        ties::bond(&mut w, x, y, 70);
    }
    let first = Them::Player(id(&w, v[0]));
    let about = Some(Them::Player(id(&w, v[7])));
    let today = w.today;
    let rows = stores::spread(&mut w, first, Passed::YourWords, about, "Tre is soft", false);
    let mut seen = std::collections::HashSet::new();
    for h in &rows {
        assert!(h.hops >= 1 && h.hops <= stores::MAX_HOPS);
        assert!(h.day > today && h.day <= today.add(4 * h.hops as i32));
        assert!(seen.insert(h.to), "nobody hears it twice");
        assert_ne!(h.to, first);
    }
    // Keeping it quiet lowers every listener's odds, and a loyal man keeps
    // a secret better.
    let m = Makeup::default();
    assert!(stores::pass_odds(&m, Passed::YourWords, true, 0.5) < stores::pass_odds(&m, Passed::YourWords, false, 0.5));
    let loyal = Makeup { loyalty: 18.0, ..m };
    assert!(
        stores::pass_odds(&loyal, Passed::Secret(0), false, 0.5) < stores::pass_odds(&m, Passed::Secret(0), false, 0.5)
    );
    // What reached a man is his to know, dated.
    for h in rows.iter().filter(|h| h.day <= w.today) {
        assert!(!stores::heard_by(&w, h.to).is_empty());
    }
}

#[test]
fn heat_and_patience_follow_his_makeup() {
    let hot = Makeup { temper: 17.0, ego: 15.0, professionalism: 6.0, ..Makeup::default() };
    let calm = Makeup { temper: 4.0, ego: 7.0, professionalism: 15.0, ..Makeup::default() };
    let h = feel::stir(&hot, Heat(0.0), Stir::Insult);
    let c = feel::stir(&calm, Heat(0.0), Stir::Insult);
    assert!(h.0 > c.0, "a hothead takes an insult harder: {h:?} {c:?}");
    assert!(feel::point(&hot) < feel::point(&calm));
    // Pushed past his point, a proud hot man walks out or gets loud; a
    // calm man goes quiet.
    let mut x = Heat(0.0);
    for _ in 0..4 {
        x = feel::stir(&hot, x, Stir::Insult);
    }
    assert!(matches!(feel::boil(&hot, x), Some(Boil::WalksOut | Boil::Loud)));
    assert_eq!(feel::boil(&calm, Heat(feel::point(&calm) + 0.01)), Some(Boil::Quiet));
    assert!(feel::stir(&calm, Heat(0.5), Stir::Apology).0 < 0.5);
    assert!(feel::cool(&calm, Heat(0.5), 10.0).0 < 0.5);
    // A talk going nowhere wears an impatient man faster.
    let short = Makeup { patience: 3.0, ..Makeup::default() };
    let long = Makeup { patience: 17.0, ..Makeup::default() };
    assert!(feel::wear(&short, Patience(1.0), 10.0, false).0 < feel::wear(&long, Patience(1.0), 10.0, false).0);
    assert!(feel::asked_again(&long, Patience(1.0), 2).0 < 1.0);
}

#[test]
fn a_recruit_weighs_his_deciders_wants_and_a_met_want_is_marked() {
    let (mut w, p, _) = room(1);
    w.add_class_aged(17);
    let i = (0..w.people.len()).find(|&i| w.people[i].stage == Stage::HighSchool).unwrap();
    let kid = Them::Player(id(&w, i));
    {
        let f = &mut w.people[i].family;
        f.decider = crate::world::background::Decider::Parent;
        f.decider_wants = crate::world::background::Wants {
            money: 0.0,
            close_to_home: 0.0,
            playing_time: 0.0,
            academics: 1.0,
            development: 0.0,
            prestige: 0.0,
        };
        f.own_wants = crate::world::background::Wants {
            money: 1.0,
            close_to_home: 0.0,
            playing_time: 0.0,
            academics: 0.0,
            development: 0.0,
            prestige: 0.0,
        };
    }
    let wants = People.wants(&w, kid);
    assert_eq!(wants[0].kind, WantKind::Academics, "his decider decides: {wants:?}");
    assert!((wants.iter().map(|x| x.weight).sum::<f32>() - 1.0).abs() < 1e-4);
    // Your player promised a start has his role met.
    let (mut w2, _, v) = room(1);
    let me = Them::Player(id(&w2, v[0]));
    let today = w2.today;
    let mut pr = talks::Promise::blank(talks::PromiseTo::Player(id(&w2, v[0])), talks::PromiseKind::Start);
    pr.made = today;
    pr.due = today.add(60);
    w2.folk.promises.push(pr);
    assert!(People.wants(&w2, me).iter().any(|x| x.kind == WantKind::Role && x.met));
    let _ = p;
}

#[test]
fn football_talk_comes_from_his_own_snaps_by_his_spot() {
    use crate::positions::Position;
    use crate::world::tally::depth::{ALIGN_O, Dc, orole};
    let d: Vec<(u16, f32)> = {
        let mut v = vec![
            (Dc::PrSnaps as u16, 200.0),
            (Dc::PrWins as u16, 40.0),
            (Dc::Prs as u16, 22.0),
            (Dc::Sacks as u16, 6.0),
            (Dc::WinSpeed as u16, 30.0),
            (Dc::WinPower as u16, 6.0),
            (Dc::Ttp as u16, 55.0),
            (Dc::TtpN as u16, 22.0),
        ];
        v.sort_by_key(|x| x.0);
        v
    };
    let edge = football::saw(&d, Position::EDGE);
    assert!(edge[0].contains("22 pressures") && edge.iter().any(|x| x.contains("speed")), "{edge:?}");
    // A slot is a slot, not an outside receiver.
    let mut r = vec![
        (ALIGN_O + orole::SLOT as u16, 300.0),
        (ALIGN_O + orole::X as u16, 40.0),
        (Dc::Routes as u16, 250.0),
        (Dc::Tgt as u16, 40.0),
        (Dc::Sep as u16, 100.0),
    ];
    r.sort_by_key(|x| x.0);
    assert!(football::saw(&r, Position::WR).iter().any(|x| x.contains("separation")));
    // The sharper his football IQ, the more he can say.
    assert!(football::reach(4.0) < football::reach(12.0) && football::reach(12.0) < football::reach(20.0));
}

#[test]
fn the_stores_keep_their_rules() {
    let (mut w, _, v) = room(2);
    let me = Them::Player(id(&w, v[0]));
    for k in 0..20 {
        stores::note(&mut w, me, &format!("note {k}"));
    }
    stores::note(&mut w, me, "note 19");
    assert_eq!(stores::notes(&w, me).len(), stores::NOTES_EACH);
    assert_eq!(*stores::notes(&w, me).last().unwrap(), "note 19");
    // A secret he told you, used where he sees it, costs his trust.
    let before = w.people[v[0]].trust;
    let k = stores::confide(&mut w, me, Them::Player(0), false, "my knee's worse than I said", true);
    let cost = stores::used_secret(&mut w, k);
    assert!(cost > 0.0 && w.people[v[0]].trust < before);
    assert!(stores::treated(&w, me, 1).contains(&Treated::Dodged));
    // A year on, the weekly upkeep trims old rows.
    stores::treat(&mut w, me, Treated::Hot);
    w.today = w.today.add(stores::KEEP_DAYS + 7);
    stores::week(&mut w);
    assert!(w.folk.live.persona.treated.is_empty());
    assert_eq!(w.folk.live.persona.secrets.len(), 1, "secrets keep their places");
}

#[test]
fn a_family_crisis_he_hasnt_told_you_brings_him_to_your_door() {
    let (mut w, _, v) = room(1);
    let i = v[0];
    let today = w.today;
    w.folk.life.events.push(Happening { id: id(&w, i), event: Event::Death, date: today, told: false });
    let come = crate::world::folk::persona::coming::comes(&w);
    let a = come.iter().find(|a| a.who == Them::Player(id(&w, i))).expect("he comes to you");
    assert!(a.urgent);
    let trust = w.people[i].trust;
    crate::world::folk::persona::coming::ignored(&mut w, a);
    assert!(w.people[i].trust < trust);
    assert!(stores::treated(&w, a.who, 1).contains(&Treated::Ignored));
}
