//! Ground's people (lane G7): the commands the person screen (lane S5)
//! reads. A person's page in words (`ground::card`): who he is, his face,
//! what he wants, his ties, your standing and its why, the moments he
//! keeps, his kids with you, your word and his, favors either way. Never a
//! hidden number.

use afm_engine::calendar::Date;
use afm_engine::world::World;
use afm_engine::world::coach::Employer;
use afm_engine::world::folk::persona::Them;
use afm_engine::world::ground::people::{Holder, Kind};
use afm_engine::world::ground::{Level, Memory, Who, card, people, standing};
use serde_json::{Value, json};

use crate::views::{TeamRef, team_colors};
use crate::{Game, Reply};

pub fn call(game: &mut Game, cmd: &str, args: &Value) -> Option<Reply> {
    Some(match cmd {
        "ground_person" | "ground_family" => {
            let family = cmd == "ground_family";
            let who = match game.w().and_then(|w| resolve(w, args, family)) {
                Ok(x) => x,
                Err(e) => return Some(Err(e)),
            };
            let (fid, fkey) = face_ident(who);
            let drawn = crate::lane_faces::drawn_on(game);
            let saved = if drawn { crate::lane_faces::saved_drawn(game, fid as u32, &fkey) } else { None };
            game.w().and_then(|w| {
                let prog = prog(w)?;
                Ok(page(w, prog, who, drawn, saved))
            })
        }
        "ground_school_people" => game.w().and_then(|w| school(w, args)),
        "ground_people" => game.w().and_then(people_list),
        _ => return None,
    })
}

fn prog(w: &World) -> Result<usize, String> {
    standing::seat_program(w).ok_or_else(|| "Ground's people are for a college staff.".to_string())
}

/// The person an argument names: `kind` (coach, trainer, seven, counselor,
/// family, prep) and `id`.
pub fn who(args: &Value) -> Result<Who, String> {
    let id = args.get("id").and_then(Value::as_u64).ok_or("Missing id.")? as u32;
    Ok(match args.get("kind").and_then(Value::as_str).unwrap_or("coach") {
        "coach" => Who::Coach(id),
        "trainer" => Who::Trainer(id),
        "seven" => Who::SevenOnSeven(id),
        "counselor" => Who::Counselor(id),
        "family" => Who::Family(id),
        "prep" => Who::Prep(id as u16),
        k => return Err(format!("No such kind of person: {k}.")),
    })
}

/// A person's kind and id as the commands take them.
pub fn key(who: Who) -> Value {
    let (k, id) = match who {
        Who::Coach(c) => ("coach", c),
        Who::Trainer(f) => ("trainer", f),
        Who::SevenOnSeven(f) => ("seven", f),
        Who::Counselor(h) => ("counselor", h),
        Who::Family(p) => ("family", p),
        Who::Prep(k) => ("prep", k as u32),
    };
    json!({ "kind": k, "id": id })
}

fn page(w: &World, prog: usize, who: Who, drawn: bool, saved: Option<String>) -> Value {
    let c = card::card(w, prog, who);
    let mut v = serde_json::to_value(&c).unwrap_or(Value::Null);
    v["key"] = key(who);
    if let Some(ties) = v.get_mut("ties").and_then(Value::as_array_mut) {
        for (t, x) in c.ties.iter().zip(ties.iter_mut()) {
            x["key"] = key(t.who);
            x["lv"] = lv_json(Level::ALL.iter().position(|l| l.name() == t.level).unwrap_or(0));
        }
    }
    extras(w, prog, who, &c, &mut v);
    v["face"] = face(w, who, drawn, saved);
    v
}

/// The person an argument names: a person by `kind` and `id`, or a
/// recruit's family by the recruit's player id.
fn resolve(w: &World, args: &Value, family: bool) -> Result<Who, String> {
    if !family {
        return who(args);
    }
    let id = args.get("id").and_then(Value::as_u64).ok_or("Missing id.")? as u32;
    let i = afm_engine::world::folk::person(w, id).ok_or("No such player.")?;
    Ok(people::family_of(w, i))
}

/// Everyone around a high school: its coaches and counselor, then the
/// families, trainers and 7v7 men around its best kids.
fn school(w: &World, args: &Value) -> Reply {
    let prog = prog(w)?;
    let h = args.get("id").and_then(Value::as_u64).ok_or("Missing id.")? as usize;
    if h >= w.high_schools.len() {
        return Err("No such school.".into());
    }
    let mut list: Vec<Who> = people::at_school(w, h);
    for i in people::kids_at(w, h, 6) {
        list.push(people::family_of(w, i));
        list.extend(people::trainer_of(w, i));
        list.extend(people::seven_of(w, i));
    }
    let mut seen: Vec<Who> = Vec::new();
    let rows: Vec<Value> = list
        .into_iter()
        .filter(|x| {
            let new = !seen.contains(x);
            seen.push(*x);
            new
        })
        .map(|x| {
            json!({
                "key": key(x),
                "name": people::name(w, x),
                "role": people::role(w, x),
                "kind": people::kind(w, x).words(),
                "level": standing::level(w, prog, x).name(),
                "why": standing::why(w, prog, x),
            })
        })
        .collect();
    Ok(json!({ "school": w.school_name(h), "people": rows }))
}

// ---- What the person screen draws on top of the card (lane S5, GR74) ------

const MONTHS: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];

fn dmy(d: Date) -> String {
    let (y, m, day) = d.parts();
    format!("{} {}, {}", MONTHS[m as usize - 1], day, y)
}

fn my(d: Date) -> String {
    let (y, m, _) = d.parts();
    format!("{} {}", MONTHS[m as usize - 1], y)
}

fn lv_json(n: usize) -> Value {
    json!({ "n": n, "name": Level::ALL[n.min(3)].name() })
}

/// Days ago, in words.
fn ago(today: Date, d: Date) -> String {
    match d.days_to(today) {
        i32::MIN..=0 => "Today".to_string(),
        1 => "Yesterday".to_string(),
        n @ 2..=13 => format!("{n} days ago"),
        n @ 14..=59 => format!("{} weeks ago", n / 7),
        n @ 60..=729 => format!("{} months ago", n / 30),
        n => format!("{} years ago", n / 365),
    }
}

/// His age, where the world keeps it.
fn age_of(w: &World, who: Who) -> Option<i32> {
    match who {
        Who::Coach(c) => w.coaches.get(c as usize).map(|c| c.age as i32),
        Who::Trainer(f) | Who::SevenOnSeven(f) => w.formers.get(f).map(|x| w.year - x.born.year()),
        Who::Prep(_) => Some(48),
        _ => None,
    }
}

/// The place's colors on his jersey: his school's, his program's, else grey.
fn colors_of(w: &World, who: Who) -> [String; 2] {
    match who {
        Who::Coach(c) => match w.coaches.get(c as usize).and_then(|c| c.job).map(|j| j.employer) {
            Some(Employer::HighSchool(h)) => team_colors(w, TeamRef::HighSchool(h)),
            Some(Employer::College(p)) => team_colors(w, TeamRef::College(p)),
            _ => ["#3a3f4b".into(), "#9aa3b2".into()],
        },
        Who::Counselor(h) => team_colors(w, TeamRef::HighSchool(h as usize)),
        _ => ["#3a3f4b".into(), "#9aa3b2".into()],
    }
}

/// Where a person's drawn head is kept: an id in a range of its own (so a
/// person never takes a player's head) and a key that turns over each
/// render version.
fn face_ident(who: Who) -> (u64, String) {
    let k = key(who);
    let (kind, id) = (k["kind"].as_str().unwrap_or(""), k["id"].as_u64().unwrap_or(0));
    let slot = ["coach", "trainer", "seven", "counselor", "family", "prep"].iter().position(|x| *x == kind).unwrap_or(0)
        as u64;
    (0x8000_0000u64 + slot * 0x0100_0000 + (id & 0x00FF_FFFF), "gd".to_string())
}

/// The head drawn from his recipe: men only, the way the faces are drawn
/// (a counselor and a family get initials). The id is in a range of its
/// own, so a person's head never takes a player's.
fn face(w: &World, who: Who, drawn: bool, saved: Option<String>) -> Value {
    let looks = people::looks(w, who);
    let heritage = people::heritage(w, who);
    let f = match who {
        Who::Coach(c) => w.coach_face(c as usize),
        Who::Trainer(f) | Who::SevenOnSeven(f) => w
            .formers
            .get(f)
            .map(|x| w.adult_face(looks, heritage, (w.year - x.born.year()).max(22), !x.clubs.is_empty())),
        Who::Prep(_) => Some(w.adult_face(looks, heritage, 48, false)),
        _ => None,
    };
    let [c1, c2] = colors_of(w, who);
    let name = people::name(w, who);
    let (fid, fkey) = face_ident(who);
    let initials: String =
        name.split_whitespace().filter_map(|p| p.chars().next()).take(2).collect::<String>().to_uppercase();
    match f {
        Some(f) => json!({
            "id": fid, "initials": initials, "colors": [c1, c2], "drawn_on": drawn,
            "heritage_words": crate::lane_faces::heritage_words(&name, &f),
            "spec": crate::lane_faces::spec(w, &f, &[c1.clone(), c2.clone()]),
            "key": fkey, "drawn": saved,
        }),
        None => json!({ "id": fid, "initials": initials, "colors": [c1, c2], "drawn_on": false }),
    }
}

/// Your staff's read of him, in words (GR74): his eye and how straight he
/// is from his record with you, his nature once he is Known. Never a number.
fn staff_read(w: &World, who: Who, c: &card::Card, level: Level) -> Value {
    let mk = people::makeup(w, who);
    let rec = w
        .ground
        .with(who)
        .iter()
        .map(|m| m.his_word)
        .fold((0u8, 0u8), |a, b| (a.0.saturating_add(b.0), a.1.saturating_add(b.1)));
    let (n, held) = rec;
    let eye = if n >= 3 {
        let r = held as f32 / n as f32;
        let how = if r >= 0.75 {
            "Good"
        } else if r >= 0.5 {
            "Fair"
        } else {
            "Poor"
        };
        Some(format!("{how}. {held} of his {n} kids held up."))
    } else if level >= Level::Known {
        Some("Too early to say. Nothing he has told you has settled yet.".to_string())
    } else {
        None
    };
    let straight = match (&c.read, n) {
        (Some(r), _) => Some(match r.as_str() {
            "his word holds up" => "Straight with you so far.".to_string(),
            "he oversells a little" => "He oversells a little.".to_string(),
            _ => "He oversells his kids.".to_string(),
        }),
        (None, _) if level >= Level::Known => Some("Too early to say.".to_string()),
        _ => None,
    };
    let nature = if level >= Level::Known {
        let loyal = if mk.loyal >= 0.65 {
            "Loyal."
        } else if mk.loyal <= 0.35 {
            "Looks out for himself."
        } else {
            "Fair to those who are fair to him."
        };
        let long = if mk.grudge >= 4.5 {
            " Holds a debt a long time, a grudge longer."
        } else if mk.grudge <= 3.5 {
            " Forgives and forgets fast."
        } else {
            ""
        };
        Some(format!("{loyal}{long}"))
    } else {
        None
    };
    json!({ "eye": eye, "straight": straight, "nature": nature })
}

/// What a level with him gets you, by what he is (people.md, "What he can
/// tell you"; the words are the build's own, proposed, not confirmed).
fn gets(kind: Kind, level: Level) -> &'static str {
    use Kind::*;
    match (kind, level) {
        (_, Level::Stranger) => "Only what anyone sees: rosters, stars and box scores.",
        (HsHead | HsCoordinator | HsCoach, Level::Known) => {
            "His real list of prospects, the same day. His word on a kid is the public version."
        }
        (HsHead | HsCoordinator | HsCoach, Level::Trusted) => {
            "The truth about a kid: his work, his family, the injury he plays through. Tape before you ask."
        }
        (HsHead | HsCoordinator | HsCoach, Level::Home) => {
            "He calls you first, before anyone, and tells you what he would tell his own staff."
        }
        (Juco | Prep, Level::Known) => "His late bloomers and academic cases, when he has them.",
        (Juco | Prep, Level::Trusted) => "Who is ready, who is not, and what a kid needs to clear.",
        (Juco | Prep, Level::Home) => "Your no's get a home with him first, and he calls you before anyone.",
        (College, Level::Known) => "His leavers, once their names are public.",
        (College, Level::Trusted) => "Who he is about to lose, and who he is passing on.",
        (College, Level::Home) => "His no's come to you first.",
        (Trainer, Level::Known) => "Which of his clients are rising, and where they train.",
        (Trainer, Level::Trusted) => "His clients' real work and speed, with no sales pitch.",
        (Trainer, Level::Home) => "His next client comes to you before anyone else sees him.",
        (SevenOnSeven, Level::Known) => "Who is rising on the circuit.",
        (SevenOnSeven, Level::Trusted) => "Who he is bringing to the next tournament, and who is not ready.",
        (SevenOnSeven, Level::Home) => "You see his players before the circuit does.",
        (Counselor, Level::Known) => "A kid's grades, in the open.",
        (Counselor, Level::Trusted) => "The transcript, the core courses, and what stands in the way.",
        (Counselor, Level::Home) => "She warns you before a kid's eligibility slips.",
        (Family, Level::Known) => "What the kid says he wants.",
        (Family, Level::Trusted) => "What the family really wants, and what they will not say in front of him.",
        (Family, Level::Home) => "They tell you where else the kid is looking.",
    }
}

/// When the next level can come, in words, from the gates (GR28).
fn next_words(m: Option<&Memory>, level: Level, today: Date) -> String {
    match level {
        Level::Stranger => "Known, with two contacts".to_string(),
        Level::Known => match m.and_then(|m| m.known_since.map(|k| (k, m.broken))) {
            Some((k, broken)) => {
                let from = broken.map_or(k, |b| b.max(k));
                let open = from.add(365);
                if today.days_to(open) > 0 {
                    format!("Trusted, not before {}", my(open))
                } else {
                    "Trusted, when you've earned it".to_string()
                }
            }
            None => "Trusted, a year after Known".to_string(),
        },
        Level::Trusted => match m {
            Some(m) => {
                let years = if m.did_well { 3 } else { 5 };
                let open = m.met.add(years * 365);
                if today.days_to(open) > 0 {
                    format!("Home, not before {}", my(open))
                } else {
                    "Home, when you've earned it".to_string()
                }
            }
            None => "Home, in years".to_string(),
        },
        Level::Home => "The top. Keep it.".to_string(),
    }
}

/// The moments he keeps in full: each with its day, the newest marked, the
/// old fading (a big one never fades while it holds).
fn moments_full(w: &World, who: Who) -> Vec<Value> {
    let mut all: Vec<&afm_engine::world::ground::Moment> =
        w.ground.with(who).iter().flat_map(|m| m.moments.iter()).collect();
    all.sort_by_key(|m| std::cmp::Reverse(m.date));
    let mut out: Vec<Value> = Vec::new();
    let mut seen: Vec<&str> = Vec::new();
    for m in all {
        if seen.contains(&m.words.as_str()) {
            continue;
        }
        seen.push(&m.words);
        let age = m.date.days_to(w.today);
        let note = if m.big && m.good {
            "A big one. He keeps it at full weight for years.".to_string()
        } else if m.big {
            match m.holds_to {
                Some(d) if d.days_to(w.today) < 0 => format!("A big one. He holds it against you until {}.", d.year()),
                _ => "A big one.".to_string(),
            }
        } else if age <= 7 {
            ago(w.today, m.date)
        } else {
            String::new()
        };
        out.push(json!({
            "date": dmy(m.date), "words": m.words, "good": m.good, "big": m.big,
            "new": age <= 7 && out.is_empty(), "fade": !m.big && age > 365, "note": note,
        }));
        if out.len() == 5 {
            break;
        }
    }
    out
}

/// Where he stands with the other staffs: the ones he knows, the best
/// first, and yours (GR17: every staff has its own memory of him).
fn staffs(w: &World, prog: usize, who: Who, you: (f32, Holder)) -> Vec<Value> {
    if matches!(who, Who::Family(_) | Who::Counselor(_)) {
        return Vec::new();
    }
    // Kids of his school by the program they went to first.
    let school = people::school(w, who);
    let mut took: std::collections::BTreeMap<usize, u32> = Default::default();
    if let Some(h) = school {
        for a in w.alumni.iter().filter(|a| a.high_school == h) {
            if let Some(&c) = a.colleges.first() {
                *took.entry(c).or_default() += 1;
            }
        }
        for p in w.people.iter().filter(|p| p.hs == h && p.stage != afm_engine::world::Stage::HighSchool) {
            if let Some(&c) = p.path.first() {
                *took.entry(c).or_default() += 1;
            }
        }
    }
    let alma: Vec<usize> = match who {
        Who::Trainer(f) | Who::SevenOnSeven(f) => {
            w.formers.get(f).map(|x| x.colleges.iter().map(|c| c.0 as usize).collect()).unwrap_or_default()
        }
        _ => Vec::new(),
    };
    let mut rows: Vec<(f32, usize)> = (0..w.programs.len())
        .filter(|&p| p != prog)
        .map(|p| (standing::points(w, who, Holder::Program(p as u32)), p))
        .filter(|x| x.0 >= 25.0)
        .collect();
    rows.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
    let mut out: Vec<Value> = rows
        .iter()
        .take(5)
        .map(|&(pts, p)| {
            let n = took.get(&p).copied().unwrap_or(0);
            let note = if n > 0 {
                format!("took {n} of his kids")
            } else if alma.contains(&p) {
                "his old school".to_string()
            } else {
                String::new()
            };
            json!({
                "program": p, "name": w.program_name(p), "color": team_colors(w, TeamRef::College(p))[0],
                "note": note, "level": lv_json(Level::of(pts) as usize), "you": false,
            })
        })
        .collect();
    let met = w
        .ground
        .with(who)
        .iter()
        .filter(|m| m.holder == you.1 || matches!(m.holder, Holder::Program(_)))
        .map(|m| m.met)
        .min();
    out.push(json!({
        "program": prog, "name": w.program_name(prog), "color": team_colors(w, TeamRef::College(prog))[0],
        "note": match met { Some(d) => format!("you, since {}", dmy(d).rsplit_once(", ").map_or(dmy(d), |x| x.0.to_string())), None => "you".to_string() },
        "level": lv_json(Level::of(you.0) as usize), "you": true,
    }));
    out
}

/// The talks you have had with him: how many, the last day, and (for a
/// family) what they were about.
fn talks(w: &World, who: Who) -> (u32, Option<Date>, Vec<String>) {
    let them = match who {
        Who::Coach(c) => Some(Them::Coach(c as usize)),
        Who::Family(k) => Some(Them::Kin(k, 0)),
        _ => None,
    };
    let mut n = 0u32;
    let mut last: Option<Date> = None;
    if let Some(t) = them {
        for s in w.folk.live.persona.talks.iter().filter(|s| s.with == t) {
            n += 1;
            last = Some(last.map_or(s.day, |d| d.max(s.day)));
        }
    }
    let mut about: Vec<String> = Vec::new();
    if let Who::Family(k) = who {
        for r in w.folk.talked.with(k) {
            if let Some(g) = r.kind.group() {
                let name = g.name().to_string();
                if !about.contains(&name) {
                    about.push(name);
                }
            }
        }
    }
    (n, last, about)
}

fn extras(w: &World, prog: usize, who: Who, c: &card::Card, v: &mut Value) {
    let you = standing::staff(w, prog, who);
    let level = Level::of(you.0);
    let mem = w.ground.get(who, you.1);
    let kind = people::kind(w, who);
    let (talked, last_talk, about) = talks(w, who);
    let last = [mem.map(|m| m.last), last_talk].into_iter().flatten().max();
    v["age"] = json!(age_of(w, who));
    v["school"] = json!(people::school(w, who));
    v["lv"] = lv_json(level as usize);
    v["moments"] = json!(moments_full(w, who));
    v["staff_read"] = staff_read(w, who, c, level);
    v["next"] = json!(next_words(mem, level, w.today));
    v["gets"] = json!({
        "now": { "level": level.name(), "words": gets(kind, level) },
        "next": if level == Level::Home { Value::Null } else {
            let nx = Level::ALL[level as usize + 1];
            json!({ "level": nx.name(), "words": gets(kind, nx) })
        },
    });
    v["staffs"] = json!(staffs(w, prog, who, you));
    v["talked"] = json!({ "times": talked, "about": about });
    v["last"] = json!(last.map(|d| ago(w.today, d)));
    v["met"] = json!(mem.map(|m| dmy(m.met)));
    v["call"] = json!(crate::lane_groundweek::who_key(who));
    v["today"] = json!(dmy(w.today));
}

/// Everyone your staff has dealt with, the last in touch first: where the
/// People page starts.
fn people_list(w: &World) -> Reply {
    let prog = prog(w)?;
    let holders = standing::holders(w, prog);
    let mut seen: Vec<(Date, Who)> = Vec::new();
    for m in w.ground.memories.iter().filter(|m| holders.contains(&m.holder)) {
        match seen.iter_mut().find(|x| x.1 == m.who) {
            Some(x) => x.0 = x.0.max(m.last),
            None => seen.push((m.last, m.who)),
        }
    }
    seen.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    let total = seen.len();
    let rows: Vec<Value> = seen
        .into_iter()
        .take(60)
        .map(|(d, x)| {
            let (pts, _) = standing::staff(w, prog, x);
            json!({
                "key": key(x), "name": people::name(w, x), "role": people::role(w, x),
                "kind": people::kind(w, x).words(),
                "lv": lv_json(Level::of(pts) as usize), "last": ago(w.today, d),
            })
        })
        .collect();
    Ok(json!({ "people": rows, "total": total, "today": dmy(w.today) }))
}
