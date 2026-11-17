//! A school visit (GR72, board 3): your own day on the road (the January
//! contact period, the FCS head coach's spring, the fall's evaluation days
//! at FCS). Its stops in order: the sign-in sheet (who came before you),
//! the coach's office (his list, his ties, the talk), the weight room (what
//! your eye catches), the counselor (transcripts), practice when there is
//! one; the new names filling as the stops go; who heard first; and where
//! you stand with him when you leave. Four hours, like any visit in person
//! (`scouting::film::there`, `VISIT_MIN`).

use super::{LEAVE, State, put_doc, put_state, state};
use crate::attributes::Attr;
use crate::calendar::Date;
use crate::world::World;
use crate::world::folk::scene::{Scene, Who as SWho};
use crate::world::folk::voice::Facts;
use crate::world::ground::people::{self, Who};
use crate::world::ground::standing::{self, Contact, Level};
use crate::world::ground::week::{self, Slot, road};
use crate::world::ground::{areas, leads};
use crate::world::scouting::readers::COACH;
use crate::world::scouting::{Kind as Look, dept};
use crate::world::{Stage, Visit};

/// A visit's minutes (four hours at the school).
pub const MINUTES: u16 = 240;
const OFFICE: (u32, &str, u16) = (1, "office", 60);
const WEIGHT: (u32, &str, u16) = (2, "weight", 30);
const COUNSELOR: (u32, &str, u16) = (3, "counselor", 30);
const PRACTICE: (u32, &str, u16) = (4, "practice", 60);
/// The talk in his office: two exchanges.
const EXCHANGES: u8 = 2;
const ASK: u32 = 101;
const STRAIGHT: u32 = 102;
const CLINIC: u32 = 103;
const SELL: u32 = 104;
const DONE: u32 = 109;
/// Kids your eye can pick out in a weight room.
const LOOK_AT: usize = 6;

/// The scene for your road day's first school.
pub(super) fn setup(w: &mut World, prog: usize, s: &mut Scene, slot: &Slot) -> bool {
    let Some(me) = super::you(w) else { return false };
    let man = slot.man.map_or(me, |m| m as usize);
    let Some(r) = w.ground.week.roads.iter().find(|r| r.man == man as u32).cloned() else { return false };
    let Some(h) = road::first_school(w, prog, man, r.area, &r.focus, slot.date) else { return false };
    let coach = people::head_coach_of(w, h);
    let mut st =
        State { who: coach, school: Some(h as u32), man: Some(man as u32), minutes: MINUTES, ..State::default() };
    st.people = coach.into_iter().collect();
    s.title = format!("A visit: {}", w.school_name(h));
    s.place = format!("{}, {}, the front office", w.school_name(h), areas::area(r.area).name);
    s.a = coach.map_or(0, super::who_code);
    s.who = coach
        .and_then(|c| match c {
            Who::Coach(c) => Some(SWho::Coach(c as usize)),
            _ => None,
        })
        .into_iter()
        .collect();
    put_state(s, &st);
    true
}

/// Who was in before you (the sign-in sheet): every college whose staff
/// was at this school's kids in the last 60 days, by their offers and their
/// kids' visits to campus, newest first.
pub fn sheet(w: &World, prog: usize, h: usize, d: Date) -> Vec<(usize, Date, String)> {
    let mut v: Vec<(usize, Date, String)> = Vec::new();
    let from = d.add(-60);
    let mut put = |p: usize, day: Date, why: String| match v.iter_mut().find(|x| x.0 == p) {
        Some(x) if day > x.1 => {
            x.1 = day;
            x.2 = why;
        }
        Some(_) => {}
        None => v.push((p, day, why)),
    };
    for q in w.people.iter().filter(|q| q.hs == h && matches!(q.stage, Stage::HighSchool | Stage::PrepSchool)) {
        let last = crate::world::names::surname_of(&q.name).to_string();
        for &(p, day) in &q.cal.early_offers {
            if p != prog && day >= from && day <= d {
                put(p, day, format!("offered {last}"));
            }
        }
        for &(p, day, kind) in &q.cal.visits {
            if p != prog && day >= from && day <= d && kind != Visit::Home {
                put(p, day, format!("had {last} on campus"));
            }
        }
    }
    v.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    v.truncate(8);
    v
}

pub(super) fn open(w: &mut World, prog: usize, s: &mut Scene) {
    let st = state(s);
    let Some(h) = st.school.map(|h| h as usize) else {
        s.narrate("The school isn't on your road today.");
        s.end();
        return;
    };
    let d = w.today;
    let minute = s.minute.unwrap_or(9 * 60);
    s.narrate(format!("{}, {}. You sign in at the front office.", w.school_name(h), super::clock(minute)));
    let rows = sheet(w, prog, h, d);
    if rows.is_empty() {
        s.narrate("The sign-in sheet has no college names on it since spring. Nobody's been in.");
    } else {
        let first = &rows[0];
        let more = if rows.len() > 1 { format!(", and {} more before them", rows.len() - 1) } else { String::new() };
        s.narrate(format!(
            "The sign-in sheet: {} was here {} ({}){more}.",
            w.program_name(first.0),
            super::ago(first.1, d),
            first.2
        ));
    }
    let table: Vec<(Option<u32>, Vec<String>)> = rows
        .iter()
        .map(|(p, day, why)| (None, vec![w.program_name(*p).to_string(), day.pretty(), why.clone()]))
        .collect();
    put_doc(
        s,
        "The sign-in sheet",
        "Who was in before you, from their offers and visits",
        &["Program", "When", "What"],
        table,
    );
    if let Some(c) = st.who {
        s.narrate(format!(
            "{} is expecting you. You're {} with him.",
            people::name(w, c),
            super::level_words(w, prog, c)
        ));
    }
    stops(w, s, &st);
}

/// The stops still open, and leaving.
fn stops(w: &World, s: &mut Scene, st: &State) {
    s.picks.clear();
    let done = |k: &str| st.stops.iter().any(|x| x.0 == k);
    let coach = st.who.map(|c| super::last(&people::name(w, c))).unwrap_or_else(|| "the coach".into());
    if !done(OFFICE.1) && st.minutes >= OFFICE.2 {
        s.pick(
            OFFICE.0,
            format!("Sit down with Coach {coach} in his office"),
            "Talk",
            "",
            "His list, his ties, the talk",
        );
    }
    if practice_now(w) && !done(PRACTICE.1) && st.minutes >= PRACTICE.2 {
        s.pick(PRACTICE.0, "Watch practice", "Watch", "", "A live look at your board's kids here");
    }
    if !done(WEIGHT.1) && st.minutes >= WEIGHT.2 {
        s.pick(WEIGHT.0, "Walk through the weight room", "Watch", "", "What your eye catches");
    }
    if !done(COUNSELOR.1) && st.minutes >= COUNSELOR.2 {
        s.pick(COUNSELOR.0, "See the counselor", "Ask", "", "Transcripts: grades and core courses");
    }
    s.pick(LEAVE, "Thank him and head out", "Leave", "", "What you walk away with");
}

/// High schools practice in the spring (April and May, where states allow
/// it) and every fall day of the season.
fn practice_now(w: &World) -> bool {
    let m = w.today.parts().1;
    matches!(m, 4 | 5 | 8..=11) && super::weekday(w.today)
}

pub(super) fn answer(w: &mut World, prog: usize, s: &mut Scene, code: u32) {
    let mut st = state(s);
    let me = super::you(w);
    let d = w.today;
    if st.at == OFFICE.1 {
        office_talk(w, prog, s, &mut st, code, d);
        put_state(s, &st);
        return;
    }
    match code {
        c if c == OFFICE.0 => {
            st.minutes = st.minutes.saturating_sub(OFFICE.2);
            st.at = OFFICE.1.into();
            st.talk = 0;
            st.stops.push((OFFICE.1.into(), String::new()));
            office_open(w, prog, s, &st);
        }
        c if c == WEIGHT.0 => {
            st.minutes = st.minutes.saturating_sub(WEIGHT.2);
            let (eye, tired) = super::your_eye(w);
            let line = weight_room(w, prog, s, &mut st, me, eye, tired, d);
            s.narrate(line.clone());
            st.stops.push((WEIGHT.1.into(), line));
            stops(w, s, &st);
        }
        c if c == COUNSELOR.0 => {
            st.minutes = st.minutes.saturating_sub(COUNSELOR.2);
            let line = counselor(w, prog, &mut st, me, d);
            s.narrate(line.clone());
            st.stops.push((COUNSELOR.1.into(), line));
            stops(w, s, &st);
        }
        c if c == PRACTICE.0 => {
            st.minutes = st.minutes.saturating_sub(PRACTICE.2);
            let line = practice(w, prog, &mut st, me, d);
            s.narrate(line.clone());
            st.stops.push((PRACTICE.1.into(), line));
            stops(w, s, &st);
        }
        _ => {
            leave(w, prog, s, &mut st, me, d);
        }
    }
    if !s.done && st.minutes < 30 && st.at.is_empty() {
        s.narrate("The four hours are up.");
        leave(w, prog, s, &mut st, me, d);
    }
    put_state(s, &st);
}

fn office_open(w: &mut World, prog: usize, s: &mut Scene, st: &State) {
    let Some(coach) = st.who else {
        s.narrate("The head job is open: nobody's in the office.");
        return;
    };
    let name = people::name(w, coach);
    let mut f = Facts::new();
    f.set("name", super::last(&name)).set("you", crate::world::folk::scene::your_last(w));
    f.set("level", super::level_key(w, prog, coach));
    let greet = match coach {
        Who::Coach(c) => super::coach_says(
            w,
            s,
            1,
            "ground.visit.greet",
            c as usize,
            &f,
            match standing::level(w, prog, coach) {
                Level::Stranger => "Coach. Come on in. What can I do for you?",
                Level::Known => "Good to see you again. Sit down.",
                _ => "There you are. Shut the door, I've got some things for you.",
            },
        ),
        _ => "Come on in.".into(),
    };
    s.narrate(format!("{name}'s office: game film on the wall, a whiteboard full of the spring's lifts."));
    s.them(&name, greet, None);
    office_picks(w, prog, s, st);
}

fn office_picks(w: &World, prog: usize, s: &mut Scene, st: &State) {
    s.picks.clear();
    let said = |k: &str| st.stops.iter().any(|x| x.0 == k);
    if !said("ask") {
        s.pick(ASK, "Ask him about his kids", "Ask", "warm", "His list: what you get depends on where you stand");
    }
    if !said("straight") && top_kid(w, st).is_some() {
        s.pick(
            STRAIGHT,
            "Tell him straight where his best kid stands with you",
            "Straight",
            "calm",
            "He wants the truth",
        );
    }
    if !said("clinic") && clinic_ahead(w, prog) && st.who.is_some_and(|c| !invited(w, c)) {
        s.pick(CLINIC, "Ask him to your clinic", "Invite", "warm", "A coach wants a clinic talk");
    }
    if !said("sell") {
        s.pick(SELL, "Sell him on your program", "Sell", "firm", "Your program's pull, nothing more");
    }
    s.pick(DONE, "Thank him and move on", "Move on", "", "Back to the rest of the visit");
}

fn office_talk(w: &mut World, prog: usize, s: &mut Scene, st: &mut State, code: u32, d: Date) {
    let Some(coach) = st.who else {
        st.at.clear();
        stops(w, s, st);
        return;
    };
    let me = super::you(w);
    let name = people::name(w, coach);
    let h = st.school.map(|h| h as usize);
    match code {
        ASK => {
            st.stops.push(("ask".into(), String::new()));
            let line = coach_list(w, prog, s, st, me, coach, h, d);
            s.them(&name, line, None);
            super::wanted(w, prog, coach, "Asked about my kids before anything else.", d);
        }
        STRAIGHT => {
            st.stops.push(("straight".into(), String::new()));
            if let Some(i) = top_kid(w, st) {
                let (you_say, his) = straight_words(w, prog, i);
                let me_name = crate::world::folk::scene::your_name(w);
                s.you(&me_name, you_say);
                s.them(&name, his, None);
                let last = super::last(&w.people[i].name);
                super::wanted(w, prog, coach, &format!("Was straight with me about {last}."), d);
                if !st.kids.contains(&w.people[i].player.id) {
                    st.kids.push(w.people[i].player.id);
                }
            }
        }
        CLINIC => {
            st.stops.push(("clinic".into(), String::new()));
            let season = week::beat::dates_for(w, d).season;
            w.ground.scenes.invited.push((coach, season));
            s.them(&name, "I'll be there. Save me a seat up front.", None);
            super::wanted(w, prog, coach, "Asked me to his clinic.", d);
        }
        SELL => {
            st.stops.push(("sell".into(), String::new()));
            let p = &w.programs[prog];
            s.them(
                &name,
                format!(
                    "I know what {} is, Coach. My kids will want to hear it from you when it's their time.",
                    p.real_name
                ),
                None,
            );
        }
        _ => {
            st.at.clear();
            stops(w, s, st);
            return;
        }
    }
    st.talk += 1;
    if st.talk >= EXCHANGES {
        st.at.clear();
        s.narrate("He has a class at the top of the hour.");
        stops(w, s, st);
    } else {
        office_picks(w, prog, s, st);
    }
}

/// His best kid in the next two classes (by stars, then the public's
/// rank), for "tell him straight".
fn top_kid(w: &World, st: &State) -> Option<usize> {
    let h = st.school? as usize;
    let season = w.dates.season;
    w.high_schools
        .get(h)?
        .roster
        .iter()
        .copied()
        .filter(|&i| {
            w.people.get(i).is_some_and(|q| {
                matches!(q.stage, Stage::HighSchool)
                    && q.class_of > season
                    && q.class_of <= season + 2
                    && q.cal.signed.is_none()
            })
        })
        .max_by_key(|&i| (w.people[i].stars, std::cmp::Reverse(w.people[i].player.id)))
}

/// What you tell him about his best kid, and what he says: you've offered,
/// you're watching, or he isn't at your level and you say so.
fn straight_words(w: &World, prog: usize, i: usize) -> (String, String) {
    let p = &w.people[i];
    let first = p.name.split_whitespace().next().unwrap_or(&p.name).to_string();
    let board = dept::watch(w, prog);
    if p.cal.offers.contains(&prog) {
        (
            format!("{first}'s our guy. The offer stands, and I'll be at his games."),
            "He knows it. He talks about your place more than he lets on.".into(),
        )
    } else if board.contains(&i) {
        (
            format!(
                "We like {first}. We want to see his tape this fall before we offer, and I won't string him along."
            ),
            "That's fair. I'd rather hear that than a lot of what I hear.".into(),
        )
    } else {
        (
            format!("{first} can play, but he's not at our level. I'll make calls for him where he fits."),
            "Most guys won't say that to my face. I appreciate it.".into(),
        )
    }
}

/// His list (GR19) at his desk: the names by your standing, written into
/// your leads, and his words.
#[allow(clippy::too_many_arguments)]
fn coach_list(
    w: &mut World,
    prog: usize,
    s: &mut Scene,
    st: &mut State,
    me: Option<usize>,
    coach: Who,
    h: Option<usize>,
    d: Date,
) -> String {
    let Some(h) = h else { return "Nobody this year.".into() };
    let before: Vec<u32> = w.ground.leads.iter().map(|l| l.kid).collect();
    leads::coach_list(w, prog, me.map(|c| c as u32), h, d);
    let new: Vec<&leads::Lead> = w
        .ground
        .leads
        .iter()
        .filter(|l| l.from == Some(coach) && !before.contains(&l.kid) && l.how == leads::How::List)
        .collect();
    let ids: Vec<u32> = new.iter().map(|l| l.kid).collect();
    let rows: Vec<(Option<u32>, Vec<String>)> = new
        .iter()
        .filter_map(|l| {
            let i = crate::world::scouting::place(w, l.kid)?;
            let p = &w.people[i];
            Some((
                Some(l.kid),
                vec![
                    p.name.clone(),
                    p.player.position.name().to_string(),
                    leads::class_name(w, p.class_of).to_string(),
                    l.said.clone(),
                ],
            ))
        })
        .collect();
    let said: Vec<String> = new.iter().map(|l| l.said.trim_matches('"').to_string()).collect();
    if !rows.is_empty() {
        put_doc(
            s,
            "His list",
            "The names he gave you, in his words",
            &["Name", "Position", "Class", "What he said"],
            rows,
        );
    }
    for id in &ids {
        if !st.kids.contains(id) {
            st.kids.push(*id);
        }
        if !st.leads.contains(id) {
            st.leads.push(*id);
        }
    }
    if said.is_empty() {
        let given = w.ground.lists.iter().any(|x| x.0 == coach && x.1.days_to(d) < leads::LIST_EVERY && x.1 != d);
        if given {
            "You've got my list from last month. Nothing's changed.".into()
        } else {
            "Honestly? Nobody I'd put in front of you this year.".into()
        }
    } else if standing::level(w, prog, coach) == Level::Stranger {
        "The ones everybody's asking about, I'll get you their film.".into()
    } else {
        said.join(" ")
    }
}

/// The weight room: a kid nobody has (not your lead, not your offer) who
/// is worth noticing, if your eye catches him (GR52). Each is rolled the
/// way a film catch is: a plain thing for the eye (how strong he is, how
/// he's built), less for a tired week.
#[allow(clippy::too_many_arguments)]
fn weight_room(
    w: &mut World,
    prog: usize,
    _s: &mut Scene,
    st: &mut State,
    me: Option<usize>,
    eye: f64,
    tired: f64,
    d: Date,
) -> String {
    let Some(h) = st.school.map(|h| h as usize) else { return String::new() };
    let season = w.dates.season;
    let roster: Vec<usize> = w.high_schools.get(h).map(|s| s.roster.clone()).unwrap_or_default();
    let mut cand: Vec<(f64, usize)> = Vec::new();
    for i in roster {
        let Some(q) = w.people.get(i) else { continue };
        if !matches!(q.stage, Stage::HighSchool) || q.class_of <= season || q.class_of > season + 3 {
            continue;
        }
        if q.cal.offers.contains(&prog) || leads::lead(w, q.player.id).is_some() {
            continue;
        }
        if let Some(pct) = leads::worth(w, prog, i) {
            cand.push((pct, i));
        }
    }
    cand.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
    cand.truncate(LOOK_AT);
    let subtle = crate::world::scouting::film::words::subtle(Attr::Strength);
    let mut seen: Vec<String> = Vec::new();
    for (_, i) in cand {
        let id = w.people[i].player.id;
        if !super::notice(w, eye, subtle, tired, &[0x6E1A, id as u64, d.0 as u64]) {
            continue;
        }
        let b = crate::body::body_now(&w.people[i].player);
        let (name, class) = (w.people[i].name.clone(), leads::class_name(w, w.people[i].class_of));
        let what = format!(
            "{name}, a {class}, {}, {} lb, is moving more weight than the seniors around him.",
            crate::body::height_text(b.height.round()),
            b.weight.round()
        );
        crate::world::scouting::film::there::watch(w, id, &[Attr::Strength, Attr::LowerBody], 0x6E1B);
        if leads::seen(w, me.map(|c| c as u32), id, d, format!("You saw him yourself in the weight room: {what}")) {
            st.leads.push(id);
        }
        st.kids.push(id);
        seen.push(what);
        if seen.len() == 2 {
            break;
        }
    }
    if seen.is_empty() {
        "The weight room is loud and full. Nobody jumps out at you today.".into()
    } else {
        seen.join(" ")
    }
}

/// The counselor: the transcripts of your board's kids here (a call look
/// in his file, the grades blank), and her by name.
fn counselor(w: &mut World, prog: usize, st: &mut State, me: Option<usize>, d: Date) -> String {
    let Some(h) = st.school.map(|h| h as usize) else { return String::new() };
    let who = Who::Counselor(h as u32);
    let name = people::name(w, who);
    super::touch(w, prog, st, me, who, Contact::Visit, 1.0, d);
    let board = dept::watch(w, prog);
    let mut ids: Vec<u32> = board.iter().filter(|&&i| w.people[i].hs == h).map(|&i| w.people[i].player.id).collect();
    for l in w.ground.leads.iter().filter(|l| !l.dropped) {
        if crate::world::scouting::place(w, l.kid).is_some_and(|i| w.people[i].hs == h) && !ids.contains(&l.kid) {
            ids.push(l.kid);
        }
    }
    ids.truncate(6);
    let mut parts = Vec::new();
    for id in ids {
        let Some(i) = crate::world::scouting::place(w, id) else { continue };
        let code = me.map_or(crate::world::scouting::readers::PERSONNEL, |c| COACH + c as u32);
        dept::add(w, id, code, Look::Transcript, d);
        let last = super::last(&w.people[i].name);
        let gpa = w.people[i].bg.gpa;
        parts.push(format!("{last} {gpa:.1}{}", grade_words(gpa)));
        if !st.kids.contains(&id) {
            st.kids.push(id);
        }
    }
    if parts.is_empty() {
        format!("{name} has nobody you're recruiting here, but she gives you the school's profile.")
    } else {
        format!("{name} pulls the transcripts: {}.", parts.join("; "))
    }
}

fn grade_words(gpa: f64) -> &'static str {
    if gpa >= 3.3 {
        ", a good student"
    } else if gpa >= 2.6 {
        ", on track"
    } else if gpa >= 2.3 {
        ", needs his core courses watched"
    } else {
        ", short of the line right now"
    }
}

/// Practice: a live look at each of your board's kids here (the built
/// evaluation look, which brings his coach's time and list too).
fn practice(w: &mut World, prog: usize, st: &mut State, me: Option<usize>, d: Date) -> String {
    let Some(h) = st.school.map(|h| h as usize) else { return String::new() };
    let Some(me) = me else { return String::new() };
    let board = dept::watch(w, prog);
    let kids: Vec<usize> = board.into_iter().filter(|&i| w.people[i].hs == h).collect();
    let before = st.who.map(|c| standing::level(w, prog, c));
    let mut names = Vec::new();
    for i in kids {
        let id = w.people[i].player.id;
        if dept::add(w, id, COACH + me as u32, Look::Practice, d) {
            w.scouting.dept.days_log.push((d, COACH + me as u32, id));
        }
        names.push(super::last(&w.people[i].name));
        if !st.kids.contains(&id) {
            st.kids.push(id);
        }
    }
    if let (Some(c), Some(b)) = (st.who, before) {
        super::moved(st, c, b, standing::level(w, prog, c));
    }
    if names.is_empty() {
        "Practice runs crisp. Nobody on your board is out there, but you see how he coaches.".into()
    } else {
        format!("You watch {} through the team period.", road::names(w, &st.kids.clone()))
    }
}

/// Leaving: what you walk away with (rule 1): the visit with him (your name
/// counts, GR53), his list if you didn't sit down, and the day's line.
fn leave(w: &mut World, prog: usize, s: &mut Scene, st: &mut State, me: Option<usize>, d: Date) {
    let Some(h) = st.school.map(|h| h as usize) else {
        s.end();
        return;
    };
    if let Some(c) = st.who {
        super::touch(w, prog, st, me, c, Contact::Visit, 1.0, d);
        if !st.stops.iter().any(|x| x.0 == "ask") {
            // A visit always gets his list (rule 1): at the front desk on
            // your way out.
            let _ = coach_list(w, prog, s, st, me, c, Some(h), d);
        }
        // His coordinators sat in.
        let (oc, dc) = w.high_schools.get(h).map_or((None, None), |x| (x.oc, x.dc));
        for x in [oc, dc].into_iter().flatten() {
            super::touch(w, prog, st, me, Who::Coach(x as u32), Contact::Visit, standing::SAT_IN, d);
        }
    }
    log_visit(w, st, me, h, d);
    let line = summary(w, prog, st, h, d);
    s.narrate(line.clone());
    s.outcome = vec![line];
    s.end();
}

/// The visit on the week's log, so the road's day knows the school was
/// seen and the morning map counts it.
fn log_visit(w: &mut World, st: &State, me: Option<usize>, h: usize, d: Date) {
    let id = w.ground.week.next();
    let text = format!(
        "You at {}: {}",
        w.school_name(h),
        if st.kids.is_empty() { "the coach's time".to_string() } else { format!("saw {}", road::names(w, &st.kids)) }
    );
    w.ground.week.log.push(week::Done {
        id,
        date: d,
        minute: 13 * 60,
        kind: week::DoneKind::Visit,
        man: me.map(|c| c as u32),
        area: w.high_schools.get(h).map(|s| s.ground_area),
        school: Some(h as u32),
        who: st.who,
        ids: st.kids.clone(),
        text,
        matters: false,
        heard: true,
        level: st.levels.iter().map(|x| x.2).max().unwrap_or(0),
    });
}

/// "Carver: the coach's list (3 names), a name nobody else has (Hill),
/// Georgia was in last week; Coach Hill is Known with you."
fn summary(w: &World, prog: usize, st: &State, h: usize, d: Date) -> String {
    let mut parts = Vec::new();
    let listed = st.leads.iter().filter(|id| leads::lead(w, **id).is_some_and(|l| l.how == leads::How::List)).count();
    parts.push(if listed > 0 {
        format!("the coach's list ({listed} {})", if listed == 1 { "name" } else { "names" })
    } else {
        "the coach's time".to_string()
    });
    let seen: Vec<u32> =
        st.leads.iter().copied().filter(|id| leads::lead(w, *id).is_some_and(|l| l.how == leads::How::Seen)).collect();
    if !seen.is_empty() {
        parts.push(format!("a name nobody else has ({})", road::names(w, &seen)));
    }
    if let Some(first) = sheet(w, prog, h, d).first() {
        parts.push(format!("{} was in {}", w.program_name(first.0), super::ago(first.1, d)));
    }
    let mut line = format!("{}: {}", w.school_name(h), parts.join(", "));
    if let Some(c) = st.who {
        let lv = standing::level(w, prog, c);
        let up = st.levels.iter().find(|x| x.0 == c).is_some_and(|x| x.2 > x.1);
        line.push_str(&format!("; {} is {}{} with you.", people::name(w, c), if up { "now " } else { "" }, lv.name()));
    } else {
        line.push('.');
    }
    line
}

/// Skipped: you still made the stop, on the default route (practice when
/// there is one, his office for the list, the weight room on your eye),
/// and it comes to one line.
pub(super) fn hand(w: &mut World, prog: usize, s: &mut Scene) {
    let mut st = state(s);
    let me = super::you(w);
    let d = s.date.max(w.today.add(-1)).min(w.today);
    if st.school.is_none() {
        s.outcome = vec!["The visit fell through.".into()];
        return;
    }
    if practice_now(w) {
        let line = practice(w, prog, &mut st, me, d);
        st.stops.push((PRACTICE.1.into(), line));
    }
    if let Some(c) = st.who {
        let mut scratch = Scene::new(s.kind, s.date, "");
        let h = st.school.map(|h| h as usize);
        let line = coach_list(w, prog, &mut scratch, &mut st, me, c, h, d);
        st.stops.push(("ask".into(), line));
        s.docs.extend(scratch.docs.into_iter().filter(|x| !super::is_state(x)));
    }
    let eye = me.map_or(10.0, |c| super::man_eye(w, c));
    let line = weight_room(w, prog, s, &mut st, me, eye, 0.0, d);
    st.stops.push((WEIGHT.1.into(), line));
    let h = st.school.unwrap_or(0) as usize;
    if let Some(c) = st.who {
        super::touch(w, prog, &mut st, me, c, Contact::Visit, 1.0, d);
    }
    log_visit(w, &st, me, h, d);
    let line = summary(w, prog, &st, h, d);
    s.outcome = vec![line];
    put_state(s, &st);
}

/// Whether your clinic is still ahead this spring.
fn clinic_ahead(w: &World, prog: usize) -> bool {
    super::clinic_day(w, prog, w.today).is_some_and(|c| c > w.today)
}

fn invited(w: &World, c: Who) -> bool {
    let season = week::beat::dates_for(w, w.today).season;
    w.ground.scenes.invited.iter().any(|x| x.0 == c && x.1 == season)
}
