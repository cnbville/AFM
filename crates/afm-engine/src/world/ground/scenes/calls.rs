//! The meeting at the map (GR47, GR71), a coach's call (the NCAA doesn't
//! limit calls to coaches; GR53: yours counts by your name) and the night
//! calls from the road (GR13, GR14).

use super::{LEAVE, State, put_state, state};
use crate::attributes::Attr;
use crate::calendar::Date;
use crate::world::World;
use crate::world::folk::scene::{Scene, Who as SWho};
use crate::world::folk::voice::Facts;
use crate::world::ground::leads::{self, Blank};
use crate::world::ground::people::{self, Who};
use crate::world::ground::standing::{self, Level};
use crate::world::ground::week::{self, DoneKind, Slot, phone, plan};
use crate::world::scouting::Kind as Look;

// --- The meeting at the map -----------------------------------------------------------------

const GO: u32 = 1;
const REDRAW: u32 = 2;
const HAND: u32 = 3;

pub(super) fn setup_meeting(w: &mut World, prog: usize, s: &mut Scene) -> bool {
    let Some(c) = week::coordinator(w, prog) else { return false };
    s.place = "The staff room, the map on the wall, 8 a.m.".into();
    s.who = vec![SWho::Coach(c)];
    let st = State { who: Some(Who::Coach(c as u32)), man: Some(c as u32), ..State::default() };
    put_state(s, &st);
    true
}

pub(super) fn open_meeting(w: &mut World, prog: usize, s: &mut Scene) {
    plan::ensure(w, prog, s.date);
    let Some(c) = week::coordinator(w, prog) else {
        s.narrate("Nobody runs your recruiting this week.");
        s.end();
        return;
    };
    let name = w.coaches[c].name.clone();
    s.narrate(format!(
        "{name} has the week drafted on the map: each man's days drawn as routes, your calls, what you host."
    ));
    let monday = week::monday_of(s.date);
    let lines = plan::letter_for(w, monday).cloned().unwrap_or_default();
    for l in lines {
        s.them(&name, l, None);
    }
    s.pick(GO, "Go with his plan", "Agree", "calm", "What he drew is the week");
    s.pick(REDRAW, "Change it at the map", "Redraw", "firm", "Move any man to any day; he plays what you leave");
    s.pick(HAND, "Hand him the week", "Hand off", "", "He plays your scenes; only a call that matters stops you");
}

pub(super) fn answer_meeting(w: &mut World, prog: usize, s: &mut Scene, code: u32) {
    let me = crate::world::folk::scene::your_name(w);
    match code {
        REDRAW => {
            s.you(&me, "Leave it up. I want to move some things.");
            s.outcome = vec![week_line(w, "The week is yours to redraw at the map")];
        }
        HAND => {
            s.you(&me, "It's your week. Call me if something matters.");
            let _ = plan::hand(w, true);
            s.outcome = vec![week_line(w, "Your coordinator has the week")];
        }
        _ => {
            let _ = prog;
            s.you(&me, "Looks right. Go.");
            s.outcome = vec![week_line(w, "The plan stands")];
        }
    }
    s.end();
}

pub(super) fn hand_meeting(w: &mut World, _prog: usize, s: &mut Scene) {
    s.outcome = vec![week_line(w, "The meeting: your coordinator's plan stands")];
}

/// "The plan stands: 6 men out, 18 road days, 3 scenes for you."
fn week_line(w: &World, head: &str) -> String {
    let wk = &w.ground.week;
    let days: u32 = wk.roads.iter().map(|r| r.days.count_ones()).sum();
    let yours = wk.slots.iter().filter(|s| s.yours && s.kind != week::SlotKind::Meeting).count();
    format!(
        "{head}: {} {} out, {days} road {}, {yours} {} for you.",
        wk.roads.len(),
        if wk.roads.len() == 1 { "man" } else { "men" },
        if days == 1 { "day" } else { "days" },
        if yours == 1 { "scene" } else { "scenes" }
    )
}

// --- A coach's call ----------------------------------------------------------------------------

const ASK: u32 = 101;
const STRAIGHT: u32 = 102;
const CLINIC: u32 = 103;
const DONE: u32 = 109;

pub(super) fn setup_call(w: &mut World, prog: usize, s: &mut Scene, slot: &Slot) -> bool {
    let Some(who) = slot.who else { return false };
    let name = people::name(w, who);
    s.title = format!("Call {name}");
    // You call him at his hour, so he picks up (GR12).
    s.minute = Some(people::makeup(w, who).hour.max(slot.minute.min(13 * 60)));
    s.place = format!("Your office, on the phone with {}", super::last(&name));
    s.a = super::who_code(who);
    if let Who::Coach(c) = who {
        s.who = vec![SWho::Coach(c as usize)];
    }
    let st = State {
        who: Some(who),
        school: people::school(w, who).map(|h| h as u32),
        people: vec![who],
        ..State::default()
    };
    let _ = prog;
    put_state(s, &st);
    true
}

pub(super) fn open_call(w: &mut World, prog: usize, s: &mut Scene) {
    let mut st = state(s);
    let Some(who) = st.who else {
        s.end();
        return;
    };
    if w.ground.week.calls.iter().any(|p| p.who == who && p.state != 0) {
        s.narrate("You've talked with him already this week.");
        s.outcome = vec![format!("{}: called already this week.", people::name(w, who))];
        s.end();
        return;
    }
    let before = standing::level(w, prog, who);
    let said = match phone::call(w, who) {
        Ok(t) => t,
        Err(e) => {
            s.narrate(e.clone());
            s.outcome = vec![e];
            s.end();
            return;
        }
    };
    let d = w.today;
    let picked = w.ground.week.log.iter().rev().any(|x| x.kind == DoneKind::Call && x.who == Some(who) && x.date == d);
    if !picked {
        s.narrate(said.clone());
        s.outcome = vec![said];
        s.end();
        return;
    }
    super::moved(&mut st, who, before, standing::level(w, prog, who));
    let name = people::name(w, who);
    s.narrate(format!("You call {name} ({}).", phone::where_words(w, who)));
    let greet = greet(w, prog, s, who);
    s.them(&name, greet, None);
    if let Some(n) = his_news(w, who, d) {
        s.them(&name, n, None);
    }
    put_state(s, &st);
    call_picks(w, prog, s, &st);
}

fn greet(w: &mut World, prog: usize, s: &Scene, who: Who) -> String {
    let mut f = Facts::new();
    f.set("you", crate::world::folk::scene::your_last(w)).set("level", super::level_key(w, prog, who));
    let fallback = match standing::level(w, prog, who) {
        Level::Stranger => "This is Coach. Who's this? ... Oh. What can I do for you?",
        Level::Known => "Coach, good to hear from you.",
        _ => "I was going to call you this week.",
    };
    match who {
        Who::Coach(c) => super::coach_says(w, s, 1, "ground.call.greet", c as usize, &f, fallback),
        _ => fallback.into(),
    }
}

/// The newest news he carried to programs (lane G4), in his words.
fn his_news(w: &World, who: Who, d: Date) -> Option<String> {
    let n = w.ground.news.iter().rev().find(|n| n.source == Some(who) && n.date.days_to(d) <= 21)?;
    Some(format!("\"Did you hear? {}\"", n.line.trim_end_matches('.')))
}

fn call_picks(w: &World, prog: usize, s: &mut Scene, st: &State) {
    s.picks.clear();
    let said = |k: &str| st.stops.iter().any(|x| x.0 == k);
    if !said("ask") {
        s.pick(ASK, "Ask him about his kids", "Ask", "warm", "His list, by where you stand");
    }
    if !said("straight") {
        s.pick(STRAIGHT, "Tell him straight where his kids stand with you", "Straight", "calm", "He wants the truth");
    }
    if !said("clinic")
        && super::clinic_day(w, prog, w.today).is_some_and(|c| c > w.today)
        && st.who.is_some_and(|c| !w.ground.scenes.invited.iter().any(|x| x.0 == c))
    {
        s.pick(CLINIC, "Ask him to your clinic", "Invite", "warm", "A coach wants a clinic talk");
    }
    s.pick(DONE, "Wrap it up", "Close", "", "Fifteen minutes on your ledger");
}

pub(super) fn answer_call(w: &mut World, prog: usize, s: &mut Scene, code: u32) {
    let mut st = state(s);
    let Some(who) = st.who else {
        s.end();
        return;
    };
    let d = w.today;
    let name = people::name(w, who);
    let me = super::you(w);
    match code {
        ASK => {
            st.stops.push(("ask".into(), String::new()));
            let before: Vec<u32> = w.ground.leads.iter().map(|l| l.kid).collect();
            if let Some(h) = st.school {
                leads::coach_list(w, prog, me.map(|c| c as u32), h as usize, d);
            }
            let new: Vec<(u32, String)> = w
                .ground
                .leads
                .iter()
                .filter(|l| l.from == Some(who) && !before.contains(&l.kid))
                .map(|l| (l.kid, l.said.trim_matches('"').to_string()))
                .collect();
            if new.is_empty() {
                s.them(&name, "Nobody new since we talked.", None);
            } else {
                for (id, line) in &new {
                    st.leads.push(*id);
                    st.kids.push(*id);
                    s.them(&name, line.clone(), None);
                }
            }
            super::wanted(w, prog, who, "Called to ask about my kids.", d);
        }
        STRAIGHT => {
            st.stops.push(("straight".into(), String::new()));
            let you = crate::world::folk::scene::your_name(w);
            s.you(&you, "I'll tell you where we are on your guys, no runaround.");
            s.them(&name, "That's all I ask. Most of them tell me what my kids want to hear.", None);
            super::wanted(w, prog, who, "Told me straight where my kids stood.", d);
        }
        CLINIC => {
            st.stops.push(("clinic".into(), String::new()));
            let season = week::beat::dates_for(w, d).season;
            w.ground.scenes.invited.push((who, season));
            s.them(&name, "I'll bring my staff.", None);
            super::wanted(w, prog, who, "Asked me to his clinic.", d);
        }
        _ => {
            end_call(w, prog, s, &st, who);
            put_state(s, &st);
            return;
        }
    }
    st.talk += 1;
    if st.talk >= 2 {
        s.narrate("He has to get to practice.");
        end_call(w, prog, s, &st, who);
    } else {
        call_picks(w, prog, s, &st);
    }
    put_state(s, &st);
}

fn end_call(w: &World, prog: usize, s: &mut Scene, st: &State, who: Who) {
    let name = people::name(w, who);
    let n = st.leads.len();
    let mut line = format!("{name}: ");
    if n > 0 {
        line.push_str(&format!("{n} new {} from his list, ", if n == 1 { "name" } else { "names" }));
    }
    let up = st.levels.iter().find(|x| x.0 == who).is_some_and(|x| x.2 > x.1);
    line.push_str(&format!("{}{}.", if up { "now " } else { "" }, standing::level(w, prog, who).name()));
    s.outcome = vec![line];
    s.end();
}

pub(super) fn hand_call(w: &mut World, _prog: usize, s: &mut Scene) {
    let st = state(s);
    let name = st.who.map_or_else(|| "The call".into(), |c| people::name(w, c));
    s.outcome = vec![format!("{name}: left for your coordinator, who calls on Saturday as his own.")];
}

// --- The night calls ---------------------------------------------------------------------------

const BACK: u32 = 1;
const TAPE: u32 = 2;
const SAW: u32 = 3;

/// The night's calls from the road that matter, as a scene (GR13, GR14).
pub(super) fn night(w: &mut World, _prog: usize, d: Date) {
    let calls: Vec<week::Done> = w
        .ground
        .week
        .log
        .iter()
        .filter(|x| x.kind == DoneKind::NightCall && x.date == d && !w.ground.scenes.nights.contains(&x.id))
        .cloned()
        .collect();
    let Some(first) = calls.iter().find(|x| x.matters && !x.heard).cloned() else { return };
    let mut s = Scene::new(crate::world::folk::scene::Kind::Ground, d, "The night calls");
    s.b = super::Kind::NightCalls.code();
    s.minute = Some(first.minute);
    s.stops = true;
    s.place = "Home, the phone on the kitchen table".into();
    let man = first.man.unwrap_or(0);
    s.a = man as u64;
    if (man as usize) < w.coaches.len() {
        s.who = vec![SWho::Coach(man as usize)];
    }
    let mut st = State { man: first.man, ..State::default() };
    // The one that stops your evening first, then the rest.
    st.ids.push(first.id);
    st.ids.extend(calls.iter().filter(|x| x.id != first.id).map(|x| x.id));
    st.kids = first.ids.clone();
    put_state(&mut s, &st);
    for x in &calls {
        w.ground.scenes.nights.push(x.id);
    }
    if w.ground.scenes.nights.len() > 400 {
        let cut = w.ground.scenes.nights.len() - 400;
        w.ground.scenes.nights.drain(..cut);
    }
    crate::world::folk::scene::add(w, s);
}

fn log_of(w: &World, id: u32) -> Option<&week::Done> {
    w.ground.week.log.iter().find(|x| x.id == id)
}

pub(super) fn open_night(w: &mut World, _prog: usize, s: &mut Scene) {
    let st = state(s);
    for &id in &st.ids {
        let Some(x) = log_of(w, id).cloned() else { continue };
        let man = x.man.map_or_else(|| "Your man".into(), |m| week::last_name(w, m));
        s.them(&man, x.text.trim_start_matches(|c| c != '"').trim_matches('"').to_string(), None);
        if let Some(y) = w.ground.week.log.iter_mut().find(|y| y.id == id) {
            y.heard = true;
        }
    }
    night_picks(w, s, &st);
}

fn night_picks(w: &World, s: &mut Scene, st: &State) {
    s.picks.clear();
    let done = |k: &str| st.stops.iter().any(|x| x.0 == k);
    let man = st.man.map_or_else(|| "him".into(), |m| week::last_name(w, m));
    if !done("back") {
        s.pick(BACK, format!("Send {man} back later this week"), "Send", "firm", "One more road day off the window");
    }
    if !done("tape") && !st.kids.is_empty() {
        s.pick(TAPE, "Get me his tape", "Ask", "", "His film in your queue this week");
    }
    if !done("saw") && !st.kids.is_empty() {
        s.pick(SAW, "Ask what he saw himself", "Ask", "warm", "Your man's own eye");
    }
    s.pick(LEAVE, "Good work. Get some sleep", "Close", "", "");
}

pub(super) fn answer_night(w: &mut World, _prog: usize, s: &mut Scene, code: u32) {
    let mut st = state(s);
    let d = w.today;
    let man = st.man.unwrap_or(0) as usize;
    let mname = if man < w.coaches.len() { w.coaches[man].name.clone() } else { "Your man".into() };
    match code {
        BACK => {
            st.stops.push(("back".into(), String::new()));
            let line = send_back(w, man, d);
            s.them(&mname, line.clone(), None);
            st.stops.last_mut().unwrap().1 = line;
        }
        TAPE => {
            st.stops.push(("tape".into(), String::new()));
            let mut n = 0;
            for &id in &st.kids {
                if leads::lead(w, id).is_some() {
                    if leads::fill(w, id, Blank::Tape).is_ok() {
                        n += 1;
                    }
                } else if !w.scouting.dept.asks.iter().any(|a| a.0 == id && a.1 == Look::HsFilm) {
                    w.scouting.dept.asks.push((id, Look::HsFilm, d));
                    n += 1;
                }
            }
            s.them(
                &mname,
                if n > 0 { "I'll have it cut up for you by Thursday." } else { "You've got it already." },
                None,
            );
        }
        SAW => {
            st.stops.push(("saw".into(), String::new()));
            let line = his_eye(w, man, &st.kids, d);
            s.them(&mname, line, None);
        }
        _ => {
            let heard = st.ids.len();
            s.outcome = vec![format!(
                "The night calls: {heard} {} from the road{}.",
                if heard == 1 { "call" } else { "calls" },
                st.stops.iter().find(|x| x.0 == "back").map_or(String::new(), |x| format!("; {}", x.1.to_lowercase()))
            )];
            s.end();
            put_state(s, &st);
            return;
        }
    }
    if st.stops.len() >= 3 {
        s.outcome = vec![format!("The night calls: {} heard, your next moves made.", st.ids.len())];
        s.end();
    } else {
        night_picks(w, s, &st);
    }
    put_state(s, &st);
}

/// One more day for him this week in the same area, where the window and
/// the rules allow it.
fn send_back(w: &mut World, man: usize, d: Date) -> String {
    let Some(r) = w.ground.week.roads.iter().find(|r| r.man == man as u32).cloned() else {
        return "I'm off the road this week, Coach.".into();
    };
    let Some(monday) = w.ground.week.monday else { return "The week's plan isn't up.".into() };
    let from = (monday.days_to(d) + 1).max(0) as u32;
    let days: Vec<u32> = (0..7).filter(|k| r.days & (1 << k) != 0).collect();
    for k in from.max(1)..5 {
        if days.contains(&k) {
            continue;
        }
        let mut try_days = days.clone();
        try_days.push(k);
        if plan::set_road(w, man as u32, r.area, &try_days).is_ok() {
            let day = ["Monday", "Tuesday", "Wednesday", "Thursday", "Friday"][k as usize];
            return format!("I'll go back {day}.");
        }
    }
    "There's no day left this week the rules give me.".into()
}

/// What your man saw of the kids himself, on his own eye: the one thing
/// that stood out, if it did.
fn his_eye(w: &World, man: usize, kids: &[u32], d: Date) -> String {
    let eye = super::man_eye(w, man);
    let mut v = Vec::new();
    for &id in kids.iter().take(2) {
        let Some(i) = crate::world::scouting::place(w, id) else { continue };
        let p = &w.people[i];
        let last = super::last(&p.name);
        let a = [Attr::Speed, Attr::Strength, Attr::Agility]
            .into_iter()
            .max_by(|x, y| p.player.attributes.get(*x).total_cmp(&p.player.attributes.get(*y)))
            .unwrap_or(Attr::Speed);
        let subtle = crate::world::scouting::film::words::subtle(a);
        if super::notice(w, eye, subtle, 0.0, &[0x41E7, id as u64, man as u64, d.0 as u64]) {
            let mark = crate::world::scouting::film::words::level_mark(p.stage);
            let cue = crate::world::scouting::film::words::cue(
                a,
                crate::world::scouting::film::words::band(p.player.attributes.get(a), mark),
            );
            v.push(format!("{last}: {cue}"));
        } else {
            v.push(format!("{last}: only saw him in the weight room. Hard to say yet."));
        }
    }
    if v.is_empty() { "Nothing more than I told you.".into() } else { v.join(" ") }
}

pub(super) fn hand_night(w: &mut World, _prog: usize, s: &mut Scene) {
    let st = state(s);
    for &id in &st.ids {
        if let Some(y) = w.ground.week.log.iter_mut().find(|y| y.id == id) {
            y.heard = true;
        }
    }
    s.outcome = vec![format!("The night calls: {} from the road, taken by your coordinator.", st.ids.len())];
}
