//! The scenes on your campus: junior day, an unofficial visit, the spring
//! game (hold it or skip it from 2027), an official visit weekend, your
//! camp, and the announcement (`ground/week.md`, "The scenes"). The visits
//! themselves are the built ones (`recruiting::recruiting_day`), with their
//! pull on each kid; what the scene adds is what you do with the hours:
//! the family you sit with (your name with them, GR53, and what they want,
//! in their words), the offer on the spot, the player host's word, the
//! coaches in the suite, and the call the night a kid decides.

use super::Kind;
use super::{LEAVE, State, put_doc, put_state, state};
use crate::attributes::Attr;
use crate::calendar::Date;
use crate::world::folk::scene::{Scene, Who as SWho};
use crate::world::folk::{self as folk, Team};
use crate::world::ground::people::{self, Who};
use crate::world::ground::standing::{self, Contact};
use crate::world::ground::week::Slot;
use crate::world::ground::{leads, week};
use crate::world::scouting::dept;
use crate::world::{Stage, Visit, World};

/// Families you can sit with in a day (an hour you split: the spring game's
/// is three 20-minute talks; a junior day's or an official's three of the
/// day's).
const FAMILIES: usize = 3;
const SIT: u32 = 200;
const OFFER: u32 = 300;
const HOST: u32 = 400;
const SUITE: u32 = 5;

/// The kids on campus for a slot, your board's first (your offer, then
/// stars): an official weekend from Friday to Sunday.
fn on_campus(w: &World, prog: usize, k: Kind, d: Date) -> Vec<usize> {
    let (from, to) = if k == Kind::Official { (d.add(-1), d.add(1)) } else { (d, d) };
    let mut v: Vec<usize> = w
        .people
        .iter()
        .enumerate()
        .filter(|(_, q)| matches!(q.stage, Stage::HighSchool | Stage::PrepSchool))
        .filter(|(_, q)| {
            q.cal.visits.iter().any(|x| {
                x.0 == prog
                    && x.1 >= from
                    && x.1 <= to
                    && match k {
                        Kind::Official => x.2 == Visit::Official,
                        _ => matches!(x.2, Visit::Unofficial | Visit::DropIn),
                    }
            })
        })
        .map(|(i, _)| i)
        .collect();
    v.sort_by_key(|&i| {
        let q = &w.people[i];
        (!q.cal.offers.contains(&prog), std::cmp::Reverse(q.stars), q.player.id)
    });
    v
}

pub(super) fn setup(w: &mut World, prog: usize, s: &mut Scene, k: Kind, slot: &Slot) -> bool {
    if k == Kind::SpringGame && !super::holds_spring_game(w, slot.date.year()) {
        return false;
    }
    let kids = on_campus(w, prog, k, slot.date);
    if kids.is_empty() && k != Kind::SpringGame {
        return false;
    }
    let ids: Vec<u32> = kids.iter().map(|&i| w.people[i].player.id).collect();
    s.who = ids.iter().take(4).map(|&id| SWho::Player(id)).collect();
    s.place = match k {
        Kind::JuniorDay => "Your football building, the team room".into(),
        Kind::Official => "Your campus, the official visit weekend".into(),
        Kind::SpringGame => "Your stadium, the spring game".into(),
        _ => "Your football building".into(),
    };
    let st = State { ids, kids: Vec::new(), ..State::default() };
    put_state(s, &st);
    true
}

pub(super) fn open(w: &mut World, prog: usize, s: &mut Scene, k: Kind) {
    let st = state(s);
    let n = st.ids.len();
    let names = week::road::names(w, &st.ids.iter().take(4).copied().collect::<Vec<_>>());
    let more = if n > 4 { format!(" and {} more", n - 4) } else { String::new() };
    match k {
        Kind::JuniorDay => {
            s.narrate(format!(
                "Junior day. {n} juniors and their families in the team room: {names}{more}. The tour, the position rooms, the photo in your jersey, then the families' questions."
            ));
        }
        Kind::Unofficial => {
            s.narrate(format!("Families on campus today on their own dime: {names}{more}. They'll see the D-line room and the weight room, and they want you."));
        }
        Kind::Official => {
            s.narrate(format!(
                "The official visit weekend: {names}{more}. Friday's arrival and dinner, Saturday's tours and the roster talk, each with a player host, and your sit-down with each on Sunday."
            ));
        }
        Kind::SpringGame => {
            let coaches = suite(w, prog, s.date);
            s.narrate(format!(
                "The spring game. {n} recruits in the stands on unofficial visits, {} high school coaches in the suite, and an hour between warmups and kickoff.",
                coaches.len()
            ));
            s.pick(
                SUITE,
                "Spend the hour in the coaches' suite",
                "Work the room",
                "warm",
                "Standing with the coaches who came",
            );
        }
        _ => {}
    }
    let rows: Vec<(Option<u32>, Vec<String>)> = st
        .ids
        .iter()
        .filter_map(|&id| {
            let i = folk::person(w, id)?;
            let p = &w.people[i];
            Some((
                Some(id),
                vec![
                    p.name.clone(),
                    p.player.position.name().to_string(),
                    w.school_name(p.hs).to_string(),
                    if p.cal.offers.contains(&prog) { "Your offer".into() } else { String::new() },
                ],
            ))
        })
        .collect();
    put_doc(s, "On campus", "", &["Name", "Position", "School", ""], rows);
    picks(w, prog, s, k, &st);
}

fn picks(w: &World, prog: usize, s: &mut Scene, k: Kind, st: &State) {
    s.picks.retain(|p| p.code == SUITE && !st.stops.iter().any(|x| x.0 == "suite"));
    let sat = st.stops.iter().filter(|x| x.0.starts_with("sit")).count();
    if sat < FAMILIES {
        for (n, &id) in st.ids.iter().take(6).enumerate() {
            if st.stops.iter().any(|x| x.0 == format!("sit{id}")) {
                continue;
            }
            let Some(i) = folk::person(w, id) else { continue };
            let fam = family_name(w, i);
            let what = match k {
                Kind::Official => format!("Sit with the {fam} family at Friday's dinner"),
                Kind::SpringGame => format!("Twenty minutes with the {fam} family"),
                _ => format!("Take the {fam} family yourself"),
            };
            s.pick(SIT + n as u32, what, "Sit down", "warm", "Your name with them, and what they want");
        }
    }
    if k == Kind::Official {
        for (n, &id) in st.ids.iter().take(4).enumerate() {
            if st.stops.iter().any(|x| x.0 == format!("host{id}")) {
                continue;
            }
            let Some(i) = folk::person(w, id) else { continue };
            s.pick(
                HOST + n as u32,
                format!("Hear from {}'s player host", first(&w.people[i].name)),
                "Ask",
                "",
                "What he was like off the clock",
            );
        }
    }
    if matches!(k, Kind::JuniorDay | Kind::Unofficial) {
        for (n, &id) in st.ids.iter().take(4).enumerate() {
            let Some(i) = folk::person(w, id) else { continue };
            if w.people[i].cal.offers.contains(&prog) || st.stops.iter().any(|x| x.0 == format!("offer{id}")) {
                continue;
            }
            s.pick(
                OFFER + n as u32,
                format!("Offer {} on the spot", w.people[i].name),
                "Offer",
                "firm",
                "Before the rivals do",
            );
        }
    }
    s.pick(LEAVE, "Let your staff run the rest", "Close", "", "");
}

pub(super) fn answer(w: &mut World, prog: usize, s: &mut Scene, k: Kind, code: u32) {
    let mut st = state(s);
    let d = w.today;
    let me = super::you(w);
    match code {
        SUITE => {
            st.stops.push(("suite".into(), String::new()));
            let coaches = suite(w, prog, d);
            for c in &coaches {
                super::touch(w, prog, &mut st, me, *c, Contact::Event(1.0), 1.0, d);
            }
            let line = format!("You work the suite: {} coaches, a handshake and a word with each.", coaches.len());
            s.narrate(line.clone());
            st.stops.last_mut().unwrap().1 = line;
        }
        c if (SIT..SIT + 6).contains(&c) => {
            let Some(&id) = st.ids.get((c - SIT) as usize) else { return };
            let line = sit(w, prog, s, &mut st, me, id, d);
            st.stops.push((format!("sit{id}"), line));
        }
        c if (HOST..HOST + 4).contains(&c) => {
            let Some(&id) = st.ids.get((c - HOST) as usize) else { return };
            let line = host(w, prog, id, d);
            s.narrate(line.clone());
            st.stops.push((format!("host{id}"), line));
            if !st.kids.contains(&id) {
                st.kids.push(id);
            }
        }
        c if (OFFER..OFFER + 4).contains(&c) => {
            let Some(&id) = st.ids.get((c - OFFER) as usize) else { return };
            let line = match crate::world::front_office::college::offer(w, id) {
                Ok(t) => format!("Offered on the spot. {t}"),
                Err(e) => e,
            };
            s.narrate(line.clone());
            st.stops.push((format!("offer{id}"), line));
            if !st.kids.contains(&id) {
                st.kids.push(id);
            }
        }
        _ => {
            finish(w, prog, s, k, &mut st);
            put_state(s, &st);
            return;
        }
    }
    let used = st.stops.len();
    if used >= FAMILIES + 2 {
        finish(w, prog, s, k, &mut st);
    } else {
        picks(w, prog, s, k, &st);
    }
    put_state(s, &st);
}

/// You sit with a family yourself (GR53: your name with them), hear their
/// question and what they want most, in their words.
fn sit(w: &mut World, prog: usize, s: &mut Scene, st: &mut State, me: Option<usize>, id: u32, d: Date) -> String {
    let Some(i) = folk::person(w, id) else { return String::new() };
    let fam = people::family_of(w, i);
    super::touch(w, prog, st, me, fam, Contact::Campus, 1.0, d);
    let wants = people::wants(w, fam);
    let top = wants.get(1).copied().unwrap_or("the truth");
    let who = folk::dialogue::decider_words(&w.people[i]).to_string();
    let q = question(top);
    let speaker = format!("His {who}");
    s.them(&speaker, q, None);
    let you = folk::scene::your_name(w);
    s.you(&you, "I'll answer that myself, and I'll tell you the same thing in a year.");
    super::wanted(w, prog, fam, "The head coach sat with us himself.", d);
    if !st.kids.contains(&id) {
        st.kids.push(id);
    }
    let line = format!("The {} family: what they want most is {top}.", family_name(w, i));
    s.narrate(line.clone());
    line
}

/// A family's first question, by what they want most.
fn question(top: &str) -> String {
    match top {
        "a degree" => "What happens to him in the classroom when football gets hard?",
        "playing time" => "How many are ahead of him at his spot, really?",
        "home close by" => "Can we get to his games? I need to see him play.",
        "money for the family" => "Let's not dance around it. What's real at your place?",
        "a path to the NFL" => "Who have you sent to the league from his room?",
        "a winner" => "Are you going to be here when he's a senior?",
        _ => "What happens if you leave for a bigger job?",
    }
    .to_string()
}

/// His player host's word on him off the clock: how he carried himself, how
/// he treated people, what he asked about (his makeup, shaded by what one
/// weekend shows).
fn host(w: &World, prog: usize, id: u32, d: Date) -> String {
    let Some(i) = folk::person(w, id) else { return String::new() };
    let p = &w.people[i];
    let host = Team::seat(w)
        .map(|t| t.roster(w))
        .unwrap_or_default()
        .into_iter()
        .filter(|&j| {
            crate::world::coach::Group::of(w.people[j].player.position)
                == crate::world::coach::Group::of(p.player.position)
        })
        .max_by(|&a, &b| {
            let la = w.people[a].player.personality.traits.leadership as f64;
            let lb = w.people[b].player.personality.traits.leadership as f64;
            la.total_cmp(&lb).then(b.cmp(&a))
        });
    let host = host.map_or_else(|| "His host".to_string(), |j| w.people[j].name.clone());
    let pe = p.player.personality;
    let t = pe.traits;
    let lean = |salt: u64| {
        crate::world::season::key_normal(w.options.seed as u64 ^ salt ^ (d.0 as u64) << 20, id as u64) * 2.5
    };
    let fst = first(&p.name);
    let _ = prog;
    let word = if t.temper as f64 + lean(0x7E39) >= 14.0 {
        format!("{fst} got into it with one of our guys at the bowling alley. Cooled off, but it got loud.")
    } else if t.professionalism as f64 + lean(0x9A0F) >= 14.0 {
        format!("{fst} said please and thank you to everybody and asked about study hall before the parties.")
    } else if pe.love_of_game + lean(0x10F6) <= 7.0 {
        format!("{fst} was on his phone all night. Asked more about the city than the playbook.")
    } else {
        format!("{fst} fit right in. Wanted to watch film with the guys Saturday morning.")
    };
    format!("{host}: \"{word}\"")
}

/// The coaches in the suite at the spring game: the high school head
/// coaches within a day's drive who know you (Known or better), the nearest
/// first, up to 20 plus 40 by your program's name.
fn suite(w: &World, prog: usize, _d: Date) -> Vec<Who> {
    let n = (20.0 + 40.0 * w.programs[prog].prestige) as usize;
    super::coaches::nearby(w, prog, 400.0, true).into_iter().take(n).collect()
}

fn family_name(w: &World, i: usize) -> String {
    crate::world::names::surname_of(&w.people[i].name).to_string()
}

fn first(n: &str) -> String {
    n.split_whitespace().next().unwrap_or(n).to_string()
}

fn finish(w: &mut World, prog: usize, s: &mut Scene, k: Kind, st: &mut State) {
    let n = st.ids.len();
    let sat: Vec<u32> =
        st.stops.iter().filter_map(|x| x.0.strip_prefix("sit").and_then(|v| v.parse::<u32>().ok())).collect();
    let offered = st.stops.iter().filter(|x| x.0.starts_with("offer") && x.1.starts_with("Offered")).count();
    let mut line = format!("{}: {n} on campus", k.name());
    if !sat.is_empty() {
        line.push_str(&format!(
            ", you sat with the {} {}",
            families_words(w, &sat),
            if sat.len() == 1 { "family" } else { "families" }
        ));
    }
    if offered > 0 {
        line.push_str(&format!(", {offered} offered on the spot"));
    }
    if st.stops.iter().any(|x| x.0 == "suite") {
        line.push_str(", the suite worked");
    }
    if k == Kind::Official {
        line.push_str("; your sit-downs are Sunday");
    }
    line.push('.');
    let _ = prog;
    s.outcome = vec![line];
    s.end();
}

fn families_words(w: &World, ids: &[u32]) -> String {
    week::road::names(w, ids)
}

/// Handed: your staff hosts; the coaches in the suite still came.
pub(super) fn hand(w: &mut World, prog: usize, s: &mut Scene, k: Kind) {
    let mut st = state(s);
    let d = s.date;
    if k == Kind::SpringGame {
        let man = week::coordinator(w, prog);
        for c in suite(w, prog, d) {
            super::touch(w, prog, &mut st, man, c, Contact::Event(1.0), 1.0, d);
        }
    }
    let n = st.ids.len();
    s.outcome = vec![format!("{}: {n} on campus; your staff hosted.", k.name())];
    put_state(s, &st);
}

// --- Your camp ------------------------------------------------------------------------------

const STATIONS: u32 = 1;
const COACH: u32 = 2;
const BROUGHT: u32 = 3;

pub(super) fn setup_camp(w: &mut World, prog: usize, s: &mut Scene) -> bool {
    let d = s.date;
    let kids = dept::campers(w, prog, d);
    if kids.is_empty() {
        return false;
    }
    let ids: Vec<u32> = kids.iter().map(|&i| w.people[i].player.id).collect();
    s.place = "Your practice fields, 8 a.m.: stations, lasers, the one-on-one pit".into();
    s.who = ids.iter().take(4).map(|&id| SWho::Player(id)).collect();
    let st = State { ids, ..State::default() };
    put_state(s, &st);
    true
}

pub(super) fn open_camp(w: &mut World, prog: usize, s: &mut Scene) {
    let st = state(s);
    if w.today == s.date {
        let _ = crate::world::scouting::film::there::camp_open(w);
    }
    let (coach, n) = brought(w, &st).unwrap_or((None, 0));
    s.narrate(format!(
        "Your camp. {} campers on the board's list, the lasers at the 40, your coaches at the stations.{}",
        st.ids.len(),
        coach.map_or(String::new(), |c| format!(
            " {} brought {n} of his kids and is watching how you treat them.",
            people::name(w, c)
        ))
    ));
    let _ = prog;
    camp_picks(w, s, &st);
}

/// The high school coach who brought the most kids.
fn brought(w: &World, st: &State) -> Option<(Option<Who>, usize)> {
    let mut by: Vec<(usize, usize)> = Vec::new();
    for &id in &st.ids {
        let Some(i) = folk::person(w, id) else { continue };
        let h = w.people[i].hs;
        match by.iter_mut().find(|x| x.0 == h) {
            Some(x) => x.1 += 1,
            None => by.push((h, 1)),
        }
    }
    let (h, n) = by.into_iter().filter(|x| x.1 >= 2).max_by_key(|x| (x.1, std::cmp::Reverse(x.0)))?;
    Some((people::head_coach_of(w, h), n))
}

fn camp_picks(w: &World, s: &mut Scene, st: &State) {
    s.picks.clear();
    let done = |k: &str| st.stops.iter().any(|x| x.0 == k);
    if !done("stations") {
        s.pick(STATIONS, "Walk the stations", "Watch", "", "The walk-up nobody listed, if your eye catches him");
    }
    if !done("coach") {
        s.pick(COACH, "Coach a group yourself", "Coach", "", "Up close: who stands out, the one-on-ones");
    }
    if !done("brought") && brought(w, st).is_some_and(|x| x.0.is_some()) {
        s.pick(
            BROUGHT,
            "Spend time with the coach who brought his kids",
            "Talk",
            "warm",
            "He watches how you treat them",
        );
    }
    for (n, &id) in st.kids.iter().take(3).enumerate() {
        if st.stops.iter().any(|x| x.0 == format!("offer{id}")) {
            continue;
        }
        let Some(i) = folk::person(w, id) else { continue };
        if w.people[i].cal.offers.is_empty()
            || !w.people[i].cal.offers.iter().any(|&p| Some(p) == crate::world::ground::standing::seat_program(w))
        {
            s.pick(
                OFFER + n as u32,
                format!("Offer {} on the spot", w.people[i].name),
                "Offer",
                "firm",
                "Before the rivals do",
            );
        }
    }
    s.pick(LEAVE, "Close the camp", "Close", "", "");
}

pub(super) fn answer_camp(w: &mut World, prog: usize, s: &mut Scene, code: u32) {
    let mut st = state(s);
    let d = w.today;
    let me = super::you(w);
    match code {
        STATIONS => {
            let (eye, tired) = super::your_eye(w);
            let line = stations(w, prog, &mut st, me, eye, tired, d);
            s.narrate(line.clone());
            st.stops.push(("stations".into(), line));
        }
        COACH => {
            let group = st
                .kids
                .first()
                .or(st.ids.first())
                .and_then(|&id| folk::person(w, id))
                .map_or(0, |i| crate::world::coach::Group::of(w.people[i].player.position) as u8);
            let line = crate::world::scouting::film::there::camp_coach(w, group).unwrap_or_else(|e| e);
            let stand = w
                .scouting
                .film
                .there
                .camp
                .as_ref()
                .map(|c| c.standouts.iter().map(|x| x.1.clone()).collect::<Vec<_>>().join(" "))
                .unwrap_or_default();
            s.narrate(format!("{line} {stand}").trim().to_string());
            st.stops.push(("coach".into(), line));
        }
        BROUGHT => {
            if let Some((Some(c), n)) = brought(w, &st) {
                super::touch(w, prog, &mut st, me, c, Contact::Event(1.0), 1.0, d);
                super::wanted(w, prog, c, "Treated my kids right at his camp.", d);
                s.them(&people::name(w, c), format!("My {n} had a good day. They'll remember who coached them."), None);
                st.stops.push(("brought".into(), format!("{}: {n} of his kids", people::name(w, c))));
            }
        }
        c if (OFFER..OFFER + 3).contains(&c) => {
            let Some(&id) = st.kids.get((c - OFFER) as usize) else { return };
            let line = crate::world::scouting::film::there::camp_offer(w, id)
                .or_else(|_| crate::world::front_office::college::offer(w, id))
                .unwrap_or_else(|e| e);
            s.narrate(line.clone());
            st.stops.push((format!("offer{id}"), line));
        }
        _ => {
            camp_finish(w, s, &mut st);
            put_state(s, &st);
            return;
        }
    }
    camp_picks(w, s, &st);
    put_state(s, &st);
}

/// The stations: a camper nobody has, worth noticing, if your eye catches
/// him (his speed at the lasers, how he moves), the way a film catch is
/// rolled. He becomes a lead you found yourself.
fn stations(w: &mut World, prog: usize, st: &mut State, me: Option<usize>, eye: f64, tired: f64, d: Date) -> String {
    let mut cand: Vec<(f64, usize)> = Vec::new();
    for &id in &st.ids {
        let Some(i) = folk::person(w, id) else { continue };
        if w.people[i].cal.offers.contains(&prog) || w.people[i].stars >= 4 {
            continue;
        }
        if let Some(pct) = leads::worth(w, prog, i) {
            cand.push((pct, i));
        }
    }
    cand.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
    let subtle = crate::world::scouting::film::words::subtle(Attr::Agility);
    let mut seen = Vec::new();
    for (_, i) in cand.into_iter().take(8) {
        let id = w.people[i].player.id;
        if !super::notice(w, eye, subtle, tired, &[0xCA5E, id as u64, d.0 as u64]) {
            continue;
        }
        let name = w.people[i].name.clone();
        let what = format!("{name} moves like nobody else in his group at the bag drills.");
        crate::world::scouting::film::there::watch(w, id, &[Attr::Agility, Attr::Acceleration, Attr::Speed], 0xCA5F);
        if leads::seen(w, me.map(|c| c as u32), id, d, format!("You saw him yourself at your camp: {what}")) {
            st.leads.push(id);
        }
        st.kids.push(id);
        seen.push(what);
        if seen.len() == 3 {
            break;
        }
    }
    if seen.is_empty() {
        "You walk the stations. Good work everywhere, nobody you'd stop the drill for.".into()
    } else {
        seen.join(" ")
    }
}

fn camp_finish(w: &mut World, s: &mut Scene, st: &mut State) {
    if w.scouting.film.there.camp.as_ref().is_some_and(|c| c.date == w.today) {
        let _ = crate::world::scouting::film::there::camp_close(w);
    }
    let offered = st.stops.iter().filter(|x| x.0.starts_with("offer") && x.1.starts_with("Offered")).count();
    let new = st.leads.len();
    let mut line = format!("Your camp: {} campers", st.ids.len());
    if new > 0 {
        line.push_str(&format!(", {new} {} you found yourself", if new == 1 { "name" } else { "names" }));
    }
    if offered > 0 {
        line.push_str(&format!(", {offered} offered on the spot"));
    }
    if let Some(x) = st.stops.iter().find(|x| x.0 == "brought") {
        line.push_str(&format!("; {}", x.1));
    }
    line.push('.');
    s.outcome = vec![line];
    s.end();
}

pub(super) fn hand_camp(w: &mut World, _prog: usize, s: &mut Scene) {
    let st = state(s);
    if w.scouting.film.there.camp.as_ref().is_some_and(|c| c.date == w.today) {
        let _ = crate::world::scouting::film::there::camp_close(w);
    }
    s.outcome = vec![format!("Your camp: {} campers; your staff ran the stations.", st.ids.len())];
}

// --- The announcement ---------------------------------------------------------------------------

const WELCOME: u32 = 1;
const GRACE: u32 = 2;
const DOOR: u32 = 3;

/// The kids your offer touched who decided: a commitment (to you or
/// elsewhere) or a flip away from you, each a call the night before he
/// posts it.
pub(super) fn announcements(w: &mut World, prog: usize, d: Date) {
    let mut now: Vec<(u32, Option<u32>)> = Vec::new();
    for q in w.people.iter().filter(|q| q.stage == Stage::HighSchool && q.cal.offers.contains(&prog)) {
        now.push((q.player.id, q.cal.commit.map(|c| c.0 as u32)));
    }
    now.sort_unstable();
    let before = std::mem::take(&mut w.ground.scenes.commits);
    let first = before.is_empty();
    w.ground.scenes.commits = now.clone();
    if first {
        return;
    }
    // The commitments (and flips) since yesterday of the kids who matter
    // to you: yours who flip, and the ones on your board. One call a
    // morning, the biggest: a flip first, then stars (the rest are the
    // news's).
    let board: Vec<usize> = dept::watch(w, prog);
    let mut calls: Vec<(bool, u8, u32, u32)> = Vec::new();
    for (id, to) in now {
        let was = before.binary_search_by_key(&id, |x| x.0).ok().map(|k| before[k].1);
        let Some(to) = to else { continue };
        if was == Some(Some(to)) {
            continue;
        }
        let flip = matches!(was, Some(Some(p)) if p as usize == prog);
        let Some(i) = folk::person(w, id) else { continue };
        if !flip && !board.contains(&i) {
            continue;
        }
        calls.push((flip, w.people[i].stars, id, to));
    }
    calls.sort_by(|a, b| b.0.cmp(&a.0).then(b.1.cmp(&a.1)).then(a.2.cmp(&b.2)));
    for (flip, _, id, to) in calls.into_iter().take(1) {
        let Some(i) = folk::person(w, id) else { continue };
        let name = w.people[i].name.clone();
        let mut s = Scene::new(crate::world::folk::scene::Kind::Ground, d, format!("The announcement: {name}"));
        s.b = Kind::Announcement.code();
        s.minute = Some(7 * 60 + 30);
        s.stops = true;
        s.place = "On the phone, early, before he posts it".into();
        s.a = id as u64;
        s.c = to as i64;
        s.f[0] = if flip { 1.0 } else { 0.0 };
        s.who = vec![SWho::Player(id)];
        let st = State { ids: vec![id], kids: vec![id], ..State::default() };
        put_state(&mut s, &st);
        crate::world::folk::scene::add(w, s);
    }
}

pub(super) fn open_announcement(w: &mut World, prog: usize, s: &mut Scene) {
    let id = s.a as u32;
    let Some(i) = folk::person(w, id) else {
        s.end();
        return;
    };
    let to = s.c as usize;
    let fst = first(&w.people[i].name);
    let flip = s.f[0] > 0.5;
    let school = w.program_name(to).to_string();
    if to == prog {
        s.them(&fst, format!("Coach, I wanted you to hear it from me before I post it. I'm coming to {school}."), None);
        s.pick(
            WELCOME,
            "Welcome him to the family",
            "Welcome",
            "warm",
            "His family and his coach hear how you took it",
        );
    } else if flip {
        s.them(&fst, format!("Coach, this is hard. I'm flipping to {school}. I wanted you to hear it from me."), None);
        s.pick(GRACE, "Wish him well", "Gracious", "calm", "His coach and family remember how you took it");
        s.pick(DOOR, "Tell him the door stays open until he signs", "Keep the door open", "firm", "You keep calling");
    } else {
        s.them(&fst, format!("Coach, I'm committing to {school} tonight. I wanted you to know first."), None);
        s.pick(GRACE, "Wish him well", "Gracious", "calm", "His coach and family remember how you took it");
        s.pick(DOOR, "Tell him the door stays open until he signs", "Keep the door open", "firm", "You keep calling");
    }
}

pub(super) fn answer_announcement(w: &mut World, prog: usize, s: &mut Scene, code: u32) {
    let id = s.a as u32;
    let Some(i) = folk::person(w, id) else {
        s.end();
        return;
    };
    let d = w.today;
    let fam = people::family_of(w, i);
    let coach = people::head_coach_of(w, w.people[i].hs);
    let me = folk::scene::your_name(w);
    let name = w.people[i].name.clone();
    let school = w.program_name(s.c as usize).to_string();
    match code {
        WELCOME => {
            s.you(&me, "Welcome home. Put your mom on, I want to tell her myself.");
            super::wanted(w, prog, fam, "He called the night our son committed and talked to us himself.", d);
            s.outcome = vec![format!("{name} committed to you; you welcomed him and his family yourself.")];
        }
        GRACE => {
            s.you(&me, "I'm proud of you. Go be great, and call me if you ever need anything.");
            super::wanted(w, prog, fam, "Took the news like a man.", d);
            if let Some(c) = coach {
                super::wanted(
                    w,
                    prog,
                    c,
                    &format!("Handled it with class when {} went elsewhere.", super::last(&name)),
                    d,
                );
            }
            s.outcome = vec![format!("{name} chose {school}; you wished him well, and his people noticed.")];
        }
        _ => {
            s.you(&me, "I respect it. Until you sign, my door's open and my phone's on.");
            s.outcome = vec![format!("{name} chose {school}; you'll keep calling until he signs.")];
        }
    }
    let _ = standing::level(w, prog, fam);
    s.end();
}

pub(super) fn hand_announcement(w: &mut World, prog: usize, s: &mut Scene) {
    let id = s.a as u32;
    let name = folk::person(w, id).map_or_else(|| "He".into(), |i| w.people[i].name.clone());
    let school = w.program_name(s.c as usize).to_string();
    s.outcome = vec![if s.c as usize == prog {
        format!("{name} committed to you; your position coach took the call.")
    } else {
        format!("{name} chose {school}; your staff took the call.")
    }];
}
