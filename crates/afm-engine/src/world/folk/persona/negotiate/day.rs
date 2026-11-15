//! Your pros' agents calling back (lane N4's owed answers, this facet's
//! part): a contract offer you made your own man in a one-on-one, or one
//! an agent took to his client, is weighed again on its day by the desk's
//! market. A yes is a call you answer (sign or pass); anything else is
//! his counter, made on the desk, on a call that needs nothing from you.

use super::super::ear::Turn;
use super::super::memory::Ready;
use super::{BACK, END, PASS, SIGN, agent_of, contract};
use crate::world::World;
use crate::world::folk::scene::{self, Kind, Scene, Who};
use crate::world::folk::{self as folk};

/// Days a call back's deal waits on your answer before it lapses.
pub const READY_DAYS: i32 = 14;

/// The day's call backs, for your seat.
pub(crate) fn day(w: &mut World) {
    let today = w.today;
    let m = &mut w.folk.live.persona;
    if m.ready.is_empty() {
        return;
    }
    // A yes you never answered lapses; its call is over.
    let scenes = &w.folk.scenes;
    m.ready.retain(|r| {
        r.scene == 0 || r.by.days_to(today) < READY_DAYS && scenes.iter().any(|s| s.id == r.scene && !s.done)
    });
    let (due, rest): (Vec<Ready>, Vec<Ready>) =
        std::mem::take(&mut m.ready).into_iter().partition(|r| r.scene == 0 && r.by <= today);
    m.ready = rest;
    for r in due {
        call_back(w, r);
    }
}

fn call_back(w: &mut World, mut r: Ready) {
    let Some(i) = folk::person(w, r.player) else { return };
    let name = w.people[i].name.clone();
    let agent = agent_of(w, i);
    let who = agent.map_or_else(|| name.clone(), |a| w.market.agents[a].name.clone());
    let (wd, _) = contract::weigh(w, i, &r.offer);
    let mut s = Scene::new(Kind::Negotiation, w.today, format!("{who} calls back about {name}"));
    s.a = r.player as u64;
    s.b = BACK;
    s.c = agent.map_or(-1, |a| a as i64);
    s.place = "On the phone".into();
    s.who = vec![Who::Player(r.player)];
    s.opened = true;
    s.narrate(format!("{who} calls back about {name}."));
    let mut t = Turn::default();
    let mut notes = Vec::new();
    let yes = wd == super::super::terms::Weighed::Yes;
    let lines = contract::words(w, i, &r.offer, &mut t, &mut notes);
    if !yes {
        // His counter goes on the desk as a typed offer's would.
        contract::counter_on_desk(w, i, &r.offer);
    }
    for l in lines.iter().chain(notes.iter()) {
        s.narrate(folk::plain(l));
    }
    w.folk.live.persona.his_lies.extend(t.lies);
    if yes {
        s.pick(SIGN, "\"Let's get it done.\"", "Sign it", "warm", "It's signed through your front office.");
        s.pick(PASS, "\"Let me think about it.\"", "Pass for now", "calm", "Nothing is signed.");
        let id = scene::add(w, s);
        r.scene = id;
        w.folk.live.persona.ready.push(r);
    } else {
        s.stops = false;
        s.pick(END, "\"I'll get back to you.\"", "End the call", "calm", "His ask is on your desk.");
        scene::add(w, s);
    }
}
