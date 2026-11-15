//! Live talks v3, lane N5b: negotiating. A deal said in a talk is weighed
//! by the desk that prices it (the agent's market, the other GM's chart),
//! nothing changes until you confirm, a counter goes on the desk as a
//! typed offer's would, your own pro's agent calls back on lane N3's day,
//! and a confirmed deal signs or trades through the desk.

use crate::world::folk::persona::Them;
use crate::world::folk::persona::ear::Turn;
use crate::world::folk::persona::negotiate::{self, BACK, Facet, Market, SIGN, contract, trade};
use crate::world::folk::persona::terms::{DealKind, Give, Offer, PickIf, Valuing, Weighed};
use crate::world::folk::scene::Kind;
use crate::world::front_office::{deals, nfl};
use crate::world::pro::{Status, money};
use crate::world::seat::{Job, You};
use crate::world::{World, WorldOptions};

/// A tiny world in the 2025 league year with you as an NFL head coach and
/// general manager (as `front_office.rs`).
fn nfl_world() -> (World, usize) {
    let mut w = World::empty(WorldOptions { scale: 0.001, history_years: 0, start_year: 2037, ..Default::default() });
    let t = (0..w.clubs.len())
        .filter(|&t| crate::world::franchise::active_in(&w, t, w.year))
        .max_by_key(|&t| w.clubs[t].roster.len())
        .unwrap();
    let teaching = vec![12.0; crate::world::coach::GROUP_COUNT];
    let you: You = serde_json::from_value(serde_json::json!({
        "name": "Test Coach", "age": 45, "ethnicity": "White", "home_state": 0, "ex_pro": false, "reputation": 60.0,
        "teaching": teaching, "scheme": 12.0, "game_management": 12.0,
        "motivation": 12.0, "discipline": 12.0, "recruiting": 12.0, "network": 12.0, "relationships": 12.0,
        "style": { "risk": 10.0, "freedom": 10.0, "physical": 10.0, "ball_hunting": 10.0, "mould": 10.0 },
        "yours": { "play_calling": 12.0, "adaptation": 12.0, "evaluation": 12.0, "negotiating": 12.0 }
    }))
    .unwrap();
    w.start_career(&you, Job::Nfl(t), 7);
    (w, t)
}

/// A mid-priced veteran of yours whose deal just ran out, with an agent
/// who negotiates for him when `agent`.
fn expiring(w: &mut World, t: usize, agent: bool) -> usize {
    w.today = w.dates.tag_window_open;
    let i = *w.clubs[t]
        .roster
        .iter()
        .filter(|&&i| w.people[i].pro.as_ref().unwrap().status == Status::Roster)
        .filter(|&&i| contract::has_agent(w, i) == agent)
        .find(|&&i| (4.0e6..15.0e6).contains(&nfl::market_read(w, i).0))
        .expect("a mid-priced veteran");
    let year = w.year;
    let c = w.people[i].pro.as_mut().unwrap();
    c.accrued = 5;
    c.contract = Some(money::build(t, money::Kind::Veteran, year - 3, &money::Terms::flat(3, 6.0e6)));
    assert_eq!(nfl::negotiable(w, i), Some(nfl::TalkKind::ReSign));
    i
}

fn offer(gives: Vec<Give>) -> Offer {
    Offer { kind: DealKind::Contract, gives, ..Offer::default() }
}

#[test]
fn an_agent_in_a_talk_answers_with_the_desks_own_market() {
    let (mut w, t) = nfl_world();
    let i = expiring(&mut w, t, true);
    let id = w.people[i].player.id;
    let f = Facet::Agent { client: i, agent: negotiate::agent_of(&w, i) };
    let market = Market { facet: f, base: &crate::world::folk::persona::talk::DEALS };
    let him = Them::Agent(negotiate::agent_of(&w, i).unwrap());
    // The minimum: not a yes, and his number comes back as real terms.
    let min = money::minimum(w.year, w.people[i].pro.as_ref().unwrap().credited);
    let low = offer(vec![Give::Money(min), Give::Years(2), Give::Guaranteed(0.0)]);
    let wd = market.weigh(&w, him, &low);
    assert!(matches!(wd, Weighed::Counter(_) | Weighed::Lowball), "{wd:?}");
    let ask = nfl::asking(&w, id).unwrap();
    if let Weighed::Counter(c) = &wd {
        assert!(c.gives.contains(&Give::Money(ask.apy)), "his counter is his ask: {c:?}");
    }
    // Weighing changed nothing: no talk on the desk yet.
    assert!(w.front_office.nfl.talk(id).is_none());
    // The talk's verdict is the desk's for the same deal.
    let p = contract::proposal(&w, i, &low).unwrap();
    let j = nfl::judge(&w, id, &p).unwrap();
    // A counter goes on the desk as a typed offer would: the talk opens
    // with your offer and his ask.
    contract::counter_on_desk(&mut w, i, &low);
    let talk = w.front_office.nfl.talk(id).expect("the counter is on the desk");
    assert_eq!(talk.offer, Some(p));
    assert!((talk.patience - j.patience).abs() < 1e-6, "the desk's patience is the talk's read");
    // His own number, said back, is a yes; confirmed, he signs on it.
    let ask = nfl::asking(&w, id).unwrap();
    let yes = offer(contract::gives_of(&ask));
    assert_eq!(market.weigh(&w, him, &yes), Weighed::Yes);
    let lines = negotiate::make_real(&mut w, f, him, &yes).unwrap();
    let k = w.people[i].pro.as_ref().unwrap().contract.as_ref().unwrap();
    assert!(k.team == t && k.last_year() >= w.year, "he signed: {lines:?}");
    assert!((k.apy() - ask.apy).abs() < 0.02 * ask.apy, "on the deal said: {lines:?}");
}

#[test]
fn your_own_pros_agent_calls_back_on_his_day() {
    let (mut w, t) = nfl_world();
    let i = expiring(&mut w, t, true);
    let id = w.people[i].player.id;
    let ask = nfl::asking(&w, id).unwrap();
    let o = offer(contract::gives_of(&ask));
    // Said to him in a one-on-one, it goes to his agent: an answer in two days.
    let wd = contract::through_agent(&w, i, &o);
    let Weighed::Later(by) = wd else { panic!("{wd:?}") };
    assert_eq!(w.today.days_to(by), crate::world::folk::persona::deals::AGENT_DAYS);
    let t_ = Turn { weighed: vec![(o.clone(), wd)], ..Turn::default() };
    negotiate::applied(&mut w, Facet::Pro { i }, 0, &t_, &[]);
    assert_eq!(w.folk.live.persona.ready.len(), 1);
    // Nothing before the day.
    negotiate::day::day(&mut w);
    assert!(!w.folk.scenes.iter().any(|s| s.kind == Kind::Negotiation));
    // On the day his agent calls back with a yes: sign or pass.
    w.today = by;
    negotiate::day::day(&mut w);
    let s = w.folk.scenes.iter().find(|s| s.kind == Kind::Negotiation && s.b == BACK).expect("he calls back").clone();
    assert!(s.picks.iter().any(|p| p.code == SIGN), "{:?}", s.lines);
    assert!(w.people[i].pro.as_ref().unwrap().contract.as_ref().unwrap().last_year() < w.year, "nothing signed yet");
    crate::world::folk::scene::answer(&mut w, s.id, SIGN).unwrap();
    let k = w.people[i].pro.as_ref().unwrap().contract.as_ref().unwrap();
    assert!(k.team == t && k.last_year() >= w.year, "signed on the call back");
    assert!(w.folk.live.persona.ready.is_empty());
}

#[test]
fn a_trade_in_words_is_weighed_on_his_chart() {
    let (mut w, t) = nfl_world();
    w.today = w.dates.league_year.date.add(2);
    let o = (0..w.clubs.len()).find(|&o| o != t && crate::world::franchise::active_in(&w, o, w.year)).unwrap();
    let year = deals::first_draft(&w);
    let last = deals::picks_of(&w, o).iter().filter(|p| p.year == year).map(|p| p.round).max().unwrap();
    // Your first for his last: he takes it.
    let good =
        Offer { kind: DealKind::Trade, gives: vec![Give::Pick(1), Give::TheirPick(last as u8)], ..Offer::default() };
    let (wd, a) = trade::weigh(&w, o, &good);
    assert_eq!(wd, Weighed::Yes, "{:?}", a.map(|a| a.text));
    // Your last for his first: no.
    let mine = deals::picks_of(&w, t).iter().filter(|p| p.year == year).map(|p| p.round).max().unwrap();
    let bad =
        Offer { kind: DealKind::Trade, gives: vec![Give::Pick(mine as u8), Give::TheirPick(1)], ..Offer::default() };
    assert_ne!(trade::weigh(&w, o, &bad).0, Weighed::Yes);
    // A condition reads the first man you get, on your first pick past round 1.
    let man = w.clubs[o].roster[0];
    let mid = Offer {
        kind: DealKind::Trade,
        gives: vec![Give::Pick(3), Give::Player(w.people[man].player.id), Give::PickIf(PickIf::Snaps(60))],
        ..Offer::default()
    };
    let p = trade::proposal(&w, o, &mid).unwrap();
    assert_eq!(p.conditional.map(|c| (c.0.round, c.1)), Some((3, deals::Condition::Snaps(60))));
    // Made: the picks change hands, as the desk makes it.
    let p = trade::proposal(&w, o, &good).unwrap();
    trade::make(&mut w, o, &good);
    assert_eq!(w.market.picks.owner_of(p.give.picks[0].year, p.give.picks[0].round, p.give.picks[0].orig), o);
}

#[test]
fn a_bigger_snap_share_is_a_harder_condition() {
    let (w, t) = nfl_world();
    let i = w.clubs[t].roster[0];
    let easy = deals::met_chance(&w, t, i, deals::Condition::Snaps(20));
    let hard = deals::met_chance(&w, t, i, deals::Condition::Snaps(90));
    assert!((0.0..=1.0).contains(&easy) && (0.0..=1.0).contains(&hard) && easy >= hard, "{easy} {hard}");
    assert_eq!(deals::Condition::Snaps(60).words(), "if he plays 60% of the snaps");
}

#[test]
fn the_rush_takes_calls_only_while_the_draft_is_live() {
    let (w, t) = nfl_world();
    let i = w.clubs[t].roster[0];
    assert!(negotiate::rush::callable(&w, i).is_err());
    let (wd, _) = negotiate::rush::weigh(&w, i, &Offer::default());
    assert_eq!(wd, Weighed::No);
}
