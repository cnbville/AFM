//! Lane commands: training and plans (section 6b, final build lane 11).
//! The practice week and its rules, each player's plan, the injury report
//! and the staff's notes; a player's Development tab; the plans (the game
//! plan and call sheet, the playbook and its installs, the play designer);
//! the squad's views; the depth chart as formation boards with your
//! position coaches' proposals. Every rating here is your staff's read
//! (a range or words), never a true number, and never a ceiling.

use std::collections::{HashMap, HashSet};

use afm_engine::attributes::Attr;
use afm_engine::calendar::Date;
use afm_engine::game::designer::{self, Design};
use afm_engine::game::playbook::{
    self as pb, Backfield, Drop, Family as PFamily, Formation, Motion, ORole, PASS_CONCEPTS, Personnel, Protection,
    RUN_CONCEPTS, Route, Set,
};
use afm_engine::positions::Position;
use afm_engine::tendencies::Tend;
use afm_engine::training::offseason;
use afm_engine::training::plan::{Contact, Day, Gear, Kind, Period, Slot, Tempo, Week, Who, available};
use afm_engine::training::rules::{Block, Tier, block_limits};
use afm_engine::training::seat::{self as ts, Board, InstallKey, SHEET, SavedDesign, SeatTeam, SheetKind, SheetPlay};
use afm_engine::training::week::block_of_week;
use afm_engine::training::work::Role;
use afm_engine::training::{FAMILIES, Family};
use afm_engine::world::{Stage, World};
use serde_json::{Value, json};

mod week;

use crate::reads;
use crate::views::{
    TeamRef, class_text, date_text, eye_for, mine, mood_words, person_index, reader_key, roster_people, row,
    status_text, team_brief,
};
use crate::{Game, Reply};

pub fn call(game: &mut Game, cmd: &str, args: &Value) -> Option<Reply> {
    let id = || args.get("id").and_then(Value::as_u64).map(|v| v as u32).ok_or_else(|| "Missing id.".to_string());
    let r = match cmd {
        "practice" => (|| practice(game.w()?))(),
        "practice_week" => (|| {
            let w = game.wm()?;
            save_week(w, args)?;
            practice(w)
        })(),
        "practice_week_reset" => (|| {
            let w = game.wm()?;
            mine_head(w)?;
            w.training.this_week = None;
            w.training.pushed = None;
            practice(w)
        })(),
        "practice_template" => (|| {
            let w = game.wm()?;
            save_template(w, args)?;
            practice(w)
        })(),
        "practice_template_reset" => (|| {
            let w = game.wm()?;
            mine_head(w)?;
            let slot = slot_arg(args)?;
            ts::reset_template(w, slot);
            practice(w)
        })(),
        "practice_plan" => (|| {
            let w = game.wm()?;
            set_plan(w, args)?;
            dev(w, id()?)
        })(),
        "player_dev" => (|| dev(game.w()?, id()?))(),
        "plans" => (|| plans(game.w()?))(),
        "plans_set" => (|| {
            let w = game.wm()?;
            plans_set(w, args)?;
            plans(w)
        })(),
        "plans_sheet" => (|| {
            let w = game.wm()?;
            sheet_set(w, args)?;
            plans(w)
        })(),
        "plans_install" => (|| {
            let w = game.wm()?;
            install_set(w, args)?;
            plans(w)
        })(),
        "designer" => (|| designer_view(game.w()?, args))(),
        "design_draw" => (|| design_draw(game.w()?, args))(),
        "design_save" => (|| {
            let w = game.wm()?;
            let k = design_save(w, args)?;
            designer_view(w, &json!({ "k": k }))
        })(),
        "design_delete" => (|| {
            let w = game.wm()?;
            design_delete(w, args)?;
            designer_view(w, &json!({}))
        })(),
        "squad" => (|| squad(game.w()?))(),
        "boards" => (|| boards(game.w()?, args))(),
        "board_set" => (|| {
            let w = game.wm()?;
            let note = board_set(w, args)?;
            let mut v = boards(w, args)?;
            v["note"] = json!(note);
            Ok(v)
        })(),
        _ => return None,
    };
    Some(r)
}

// ---------- shared ----------

fn team_of(w: &World) -> Result<(TeamRef, SeatTeam), String> {
    let t = mine(w).ok_or("You have no team.")?;
    let st = w.training_team().ok_or("You have no team.")?;
    Ok((t, st))
}

/// Your job runs the practice schedule only as the head coach (an
/// assistant plans his own players and proposes the rest).
fn mine_head(w: &World) -> Result<(), String> {
    if w.seat.is_none() {
        return Err("You have no team.".into());
    }
    if !w.seat_head() {
        return Err("The practice schedule is the head coach's. You plan your own players.".into());
    }
    Ok(())
}

fn tier_name(t: Tier) -> &'static str {
    match t {
        Tier::High => "hs",
        Tier::College => "college",
        Tier::Pro => "nfl",
    }
}

fn link_player(w: &World, id: u32) -> String {
    match person_index(w, id) {
        Some(i) => format!("[[p:{id}|{}]]", w.people[i].name),
        None => "a former player".into(),
    }
}

fn monday_of(d: Date) -> Date {
    let mut m = d;
    while afm_engine::training::plan::weekday_index(m) != 0 {
        m = m.add(-1);
    }
    m
}

const WEEKDAYS: [&str; 7] = ["Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday"];

/// A kickoff's minutes after midnight from its text ("7:30 pm").
fn kick_minutes(t: &str) -> Option<u32> {
    let (hm, ap) = t.trim().split_once(' ')?;
    let (h, m) = hm.split_once(':')?;
    let (h, m): (u32, u32) = (h.parse().ok()?, m.parse().ok()?);
    let h = match (h, ap) {
        (12, "am") => 0,
        (12, _) => 12,
        (h, "pm") => h + 12,
        (h, _) => h,
    };
    Some(h * 60 + m)
}

/// The other team in a game, at your team's level.
fn opp_ref(t: TeamRef, opp: usize) -> TeamRef {
    match t {
        TeamRef::College(_) => TeamRef::College(opp),
        TeamRef::Nfl(_) => TeamRef::Nfl(opp),
        TeamRef::HighSchool(_) => TeamRef::HighSchool(opp),
        TeamRef::Other(_) => TeamRef::Other(opp),
    }
}

fn period_json(p: &Period) -> Value {
    json!({
        "kind": p.kind.name(),
        "label": p.kind.label(),
        "name": p.name(),
        "group": p.kind.group(),
        "family": p.kind.shown_family(p.contact),
        "min": p.min,
        "gear": p.gear.name(),
        "contact": p.contact.name(),
        "tempo": p.tempo.name(),
        "who": p.who.name(),
        "what": p.what,
        "noise": p.noise,
        "field": p.kind.on_field(),
        "counted": p.counted(),
        "own": p.own(),
    })
}

fn day_json(d: &Day) -> Value {
    json!({
        "periods": d.periods.iter().map(period_json).collect::<Vec<_>>(),
        "game": d.game,
        "second": d.second,
        "minutes": d.minutes(),
        "field": d.field(),
        "padded": d.padded(),
        "rest": d.rest(),
        "practice": d.periods.iter().any(|p| p.kind.practice()),
        "full": d.periods.iter().filter(|p| p.contact.full()).map(|p| p.min as u32).sum::<u32>(),
    })
}

fn parse_period(v: &Value) -> Result<Period, String> {
    let s = |k: &str| v.get(k).and_then(Value::as_str).unwrap_or("");
    let kind = Kind::parse(s("kind")).ok_or("A period has no kind.")?;
    let min = v.get("min").and_then(Value::as_u64).unwrap_or(10).clamp(5, 180) as u8;
    let gear = Gear::parse(s("gear")).unwrap_or(Gear::None);
    let contact = Contact::parse(s("contact")).unwrap_or(Contact::Air);
    let mut p = Period::new(kind, min, gear, contact.min(gear.most_contact()));
    if let Some(t) = Tempo::parse(s("tempo")) {
        p.tempo = t;
    }
    if let Some(w) = Who::parse(s("who")) {
        p.who = w;
    }
    p.what = v.get("what").and_then(Value::as_u64).unwrap_or(0).min(kind.whats().len().max(1) as u64 - 1) as u8;
    p.noise = v.get("noise").and_then(Value::as_bool).unwrap_or(false) && kind.on_field();
    Ok(p)
}

fn parse_days(v: &Value) -> Result<[Day; 7], String> {
    let arr = v.get("days").and_then(Value::as_array).ok_or("Missing days.")?;
    if arr.len() != 7 {
        return Err("A week has seven days.".into());
    }
    let mut out: [Day; 7] = Default::default();
    for (k, d) in arr.iter().enumerate() {
        let periods = d.get("periods").and_then(Value::as_array).cloned().unwrap_or_default();
        out[k].periods = periods.iter().map(parse_period).collect::<Result<Vec<_>, _>>()?;
        out[k].second = d.get("second").and_then(Value::as_bool).unwrap_or(false);
        out[k].game = d.get("game").and_then(Value::as_bool).unwrap_or(false);
    }
    Ok(out)
}

fn slot_arg(args: &Value) -> Result<Slot, String> {
    Slot::parse(args.get("slot").and_then(Value::as_str).unwrap_or("")).ok_or_else(|| "No such week.".to_string())
}

fn options(st: &SeatTeam) -> Value {
    let kinds: Vec<Kind> =
        Kind::ALL.iter().copied().filter(|&k| available(k, st.tier, st.frame.season, st.frame.big)).collect();
    json!({
        "kinds": kinds.iter().map(|&k| json!({
            "kind": k.name(),
            "label": k.label(),
            "group": k.group(),
            "grows": kind_grows(k),
            "field": k.on_field(),
            "counted": k.counted(),
            "film": k.film(),
            "strength": k.strength_staff(),
            "slot": week::slot_of(k),
            "whats": k.whats(),
            "by": (0..k.whats().len().max(1)).map(|w| week::runs(k, w as u8)).collect::<Vec<_>>(),
            "family": k.shown_family(Contact::Air),
            "family_full": k.shown_family(Contact::Live),
        })).collect::<Vec<_>>(),
        "gears": Gear::ALL.iter().map(|g| json!({ "v": g.name(), "t": g.label() })).collect::<Vec<_>>(),
        "contacts": ([Contact::Air, Contact::Bags, Contact::Control, Contact::Thud, Contact::Live]).iter().map(|c| json!({ "v": c.name(), "t": c.label() })).collect::<Vec<_>>(),
        "tempos": [["walk", "Walkthrough pace"], ["normal", "Normal"], ["fast", "Fast (no huddle)"]],
        "whos": [["all", "Everyone, each at his station"], ["ones", "The two-deep (scout team for the rest)"], ["young", "The young players"]],
        "grows": week::SHOWN.iter().map(|f| json!([f, week::grows(f)])).collect::<Vec<_>>(),
    })
}

/// What a kind of period grows, in words (topic 1; update 2's new kinds).
fn kind_grows(k: Kind) -> &'static str {
    match k {
        Kind::WarmUp => "loosens up; a little movement and fitness",
        Kind::Individual => "technique with the position coach",
        Kind::InsideRun => "run blocking, shedding and tackling; the run game",
        Kind::SevenOnSeven => "routes, coverage and reads; no line contact",
        Kind::OneOnOnes => "the matchups: hands, release, press, pass rush",
        Kind::Team => "timing, reads and the install; contact by the gear",
        Kind::Scrimmage => "game-like reps: everything, and the most wear",
        Kind::Situations => "third down, red zone, two-minute: the call sheet's prep",
        Kind::SpecialTeams => "the kicking game's units",
        Kind::Walkthrough => "assignments at no speed: the mind, almost no wear",
        Kind::Young => "the players who don't play: reps of their own",
        Kind::Conditioning => "stamina; tired legs if overdone in season",
        Kind::Speed => "speed and agility work",
        Kind::WeightRoom => "strength and the body",
        Kind::Meeting => "the scheme and the plan: the team, each unit, each room",
        Kind::Film => "reading the opponent and yourself",
        Kind::Recovery => "treatment and rest: nothing counts",
        Kind::MatDrills => "conditioning and toughness at dawn; past two mornings a week it costs mood",
        Kind::Testing => "the strength staff's numbers: lifts, the 40, the shuttle, the vertical",
        Kind::Install => "the week's new calls on the board before they're repped",
        Kind::StudyHall => "grades for the flagged players: not counted",
        Kind::OnAir => "quarterback and receiver timing, routes without a defense",
        Kind::BallSecurity => "holding on to the ball: the gauntlet, the JUGS machine",
        Kind::KickStations => "snap, hold and kick; punts in the wind",
        Kind::Mobility => "hips, ankles and the back: a little less soft-tissue risk",
        Kind::Vision => "reaction and tracking with light boards and strobe glasses",
        Kind::VrReps => "mental reps in a headset, no wear",
        Kind::MentalSkills => "composure and routines with the sports psychologist",
        Kind::RulesClinic => "this year's points of emphasis: fewer penalties",
        Kind::PlanTest => "a quiz on the plan: your staff sees who knows it",
        Kind::Nutrition => "the body plan with the dietitian: weight targets move faster",
        Kind::Council => "elected players with the head coach: bonds and captains",
        Kind::TeamBuilding => "dinners and outings: bonds, leadership and mood",
        Kind::Signals => "the calls and checks, in noise: fewer busts",
        Kind::HomeStudy => "the plan on his own: the hard workers take more from it",
    }
}

/// Your staff's read of how a player's growth is coming (topic 13): words
/// from the week's real growth seen through the position coach's eye,
/// never a number.
fn growth_words(moved: f64, eye: f64, lean: f64) -> &'static str {
    let seen = moved + lean * (20.0 - eye.clamp(1.0, 20.0)) / 40.0;
    if seen >= 0.9 {
        "coming on fast"
    } else if seen >= 0.4 {
        "growing steadily"
    } else if seen >= 0.12 {
        "inching forward"
    } else if seen > -0.05 {
        "holding where he is"
    } else {
        "slipping"
    }
}

fn lean(reader: u64, id: u32) -> f64 {
    let mut z = reader ^ (id as u64) << 21 ^ 0x9A1E;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^= z >> 31;
    (z >> 11) as f64 / (1u64 << 53) as f64 * 2.0 - 1.0
}

fn load_words(acute: u8, chronic: u8) -> (&'static str, &'static str) {
    let spike = if chronic < 3 { if acute > 6 { 1.6 } else { 1.0 } } else { acute as f64 / chronic as f64 };
    // In tens of load units: a winter week is about 30, a game week about
    // 45, a camp week 90 or more.
    let level = match acute {
        0..=14 => "light",
        15..=34 => "moderate",
        35..=64 => "heavy",
        _ => "very heavy",
    };
    let trend = if spike > 1.45 {
        "a sharp jump: the trainers are watching him"
    } else if spike > 1.2 {
        "rising"
    } else if spike < 0.7 {
        "falling: fresh legs"
    } else {
        "steady"
    };
    (level, trend)
}

fn grade_words(g: u8) -> &'static str {
    match g {
        0 => "no practice grades yet",
        1..=49 => "losing most of his reps",
        50..=56 => "below the room",
        57..=63 => "holding his own",
        64..=70 => "winning his share",
        _ => "winning most of his reps",
    }
}

pub(crate) fn practice_status(w: &World, id: u32, i: usize) -> (&'static str, String) {
    let d = w.today;
    if let Some(h) = w.training.hurt_on(id, d) {
        let what = h.kind.map_or(h.how.clone(), |k| k.name().to_string());
        return ("out", format!("{what}, back about {}", date_text(h.until)));
    }
    let p = &w.people[i];
    if p.pro.as_ref().is_some_and(|c| c.out_weeks > 0) {
        return ("out", status_text(w, p));
    }
    if p.player.train.spike() > 1.45 {
        return ("limited", "load management".into());
    }
    if w.training.plan_of(id).rest > 0 {
        return ("limited", "a rest day each week".into());
    }
    ("full", String::new())
}

fn teaching_words(t: f64) -> &'static str {
    if t >= 15.5 {
        "one of the best teachers at his position anywhere"
    } else if t >= 12.5 {
        "a good teacher"
    } else if t >= 9.5 {
        "an average teacher"
    } else if t >= 7.0 {
        "a below-average teacher"
    } else {
        "a poor teacher"
    }
}

/// Who teaches a player (topic 6): his position coach, else the
/// coordinator, else the head coach; at a high school the head coach or
/// the assistant on his group.
pub(crate) fn teacher(w: &World, t: TeamRef, i: usize) -> Value {
    let p = &w.people[i];
    let g = afm_engine::world::coach::Group::of(p.player.position);
    let (coach, title) = match t {
        TeamRef::College(c) => {
            let pr = &w.programs[c];
            match pr.position_coaches[g as usize] {
                Some(x) => (Some(x), g.coach_title().to_string()),
                None => match if g.offense() { pr.oc } else { pr.dc } {
                    Some(x) => (
                        Some(x),
                        if g.offense() { "offensive coordinator" } else { "defensive coordinator" }.to_string(),
                    ),
                    None => (pr.head_coach, "head coach".to_string()),
                },
            }
        }
        TeamRef::Other(f) => {
            let tm = w.others.team(f).cloned().unwrap_or_default();
            match if g.offense() { tm.oc } else { tm.dc } {
                Some(x) => {
                    (Some(x), if g.offense() { "offensive coordinator" } else { "defensive coordinator" }.to_string())
                }
                None => (tm.head_coach, "head coach".to_string()),
            }
        }
        TeamRef::Nfl(c) => {
            let tm = &w.nfl[c];
            match if g.offense() { tm.oc } else { tm.dc } {
                Some(x) => {
                    (Some(x), if g.offense() { "offensive coordinator" } else { "defensive coordinator" }.to_string())
                }
                None => (tm.head_coach, "head coach".to_string()),
            }
        }
        TeamRef::HighSchool(h) => (w.high_schools[h].coach, "head coach".to_string()),
    };
    let teach = w.training_team().map_or(9.0, |st| ts::staff_teaching(w, &st, i));
    let you = w.career.as_ref().is_some_and(|c| Some(c.coach) == coach);
    json!({
        "id": coach,
        "name": coach.map(|c| w.coaches[c].name.clone()),
        "title": title,
        "words": teaching_words(teach),
        "you": you,
        "group": g.coach_title(),
    })
}

fn focus_options(pos: Position) -> Vec<Value> {
    let mut opts: Vec<(f64, Attr)> =
        Attr::ALL.iter().map(|&a| (afm_engine::positions::weight(pos, a), a)).filter(|x| x.0 > 0.0).collect();
    opts.sort_by(|a, b| b.0.total_cmp(&a.0));
    opts.iter().take(16).map(|(_, a)| json!({ "key": a.key(), "label": a.label() })).collect()
}

fn plan_json(w: &World, id: u32, pos: Position) -> Value {
    let plan = w.training.plan_of(id);
    let focus = w.focus_of(id);
    let staff = w.staff_focus(id);
    json!({
        "focus": focus.map(|a| a.key()),
        "focus_label": focus.or(staff).map(|a| a.label()),
        "focus_by": if focus.is_some() { "you" } else { "staff" },
        "staff_focus": staff.map(|a| a.label()),
        "habit": plan.habit.map(|(k, t)| json!({ "k": k, "label": Tend::ALL[k as usize % Tend::ALL.len()].label(), "target": t })),
        "body": plan.body.map(|(lbs, e)| json!({ "target": lbs, "emphasis": (["strength", "speed", "lean"][e.min(2) as usize]) })),
        "moving": plan.moving.map(|(to, prog)| json!({ "to": Position::ALL[to as usize % Position::ALL.len()].name(), "progress": prog })),
        "extra": { "film": plan.extra & 1 != 0, "work": plan.extra & 2 != 0, "lift": plan.extra & 4 != 0 },
        "rest": plan.rest,
        "options": focus_options(pos),
    })
}

// ---------- the practice week (topics 1-4, 7, 8, 11, 13) ----------

fn practice(w: &World) -> Reply {
    let (t, st) = team_of(w)?;
    let monday = monday_of(w.today);
    let pw = ts::planned(w, &st, monday);
    let reader = reader_key(w);
    let block = pw.block;
    let lim = block_limits(st.tier, block, st.frame.season);
    let counts = match w.training.counts {
        Some((b, s, c)) if b == block && s == st.frame.season => c,
        _ => Default::default(),
    };
    // Your next game and the games ahead (TR10), each game day's kickoff.
    let next = crate::lane_gameday::next_game(w);
    let ahead: Vec<(Date, String)> = crate::views::schedule_rows(w, t).into_iter().map(|(d, s, _)| (d, s)).collect();
    let this_game = next.as_ref().filter(|g| g.date >= monday && g.date < monday.add(7));
    let kick_of = |d: Date| -> u32 {
        let parsed = this_game.filter(|g| g.date == d).and_then(|g| g.kick.as_deref()).and_then(kick_minutes);
        parsed.unwrap_or(match st.tier {
            Tier::Pro => 780,
            Tier::College => 930,
            Tier::High => 1140,
        })
    };
    // Required days off (TR13): the week's last rest days, as many as the
    // rule asks; any others are the staff's own choice.
    let main = pw.books.iter().find(|b| b.coaches).unwrap_or(&pw.books[0]);
    let mut required = [false; 7];
    let mut need = main.days_off as usize;
    for k in (0..7).rev() {
        if need > 0 && pw.week.days[k].rest() {
            required[k] = true;
            need -= 1;
        }
    }
    let org = match st.tier {
        Tier::College => "NCAA",
        Tier::Pro => "CBA",
        Tier::High => "The state",
    };
    let in_season = matches!(block, Block::Season | Block::Postseason | Block::Camp);
    let days: Vec<Value> = pw
        .week
        .days
        .iter()
        .enumerate()
        .map(|(k, d)| {
            let bk = &pw.books[k];
            let date = monday.add(k as i32);
            let mut v = day_json(d);
            let kick = d.game.then(|| kick_of(date));
            v["clock"] = week::day_clock(st.tier, block, date, d, kick);
            v["weather"] = json!(pw.weather[k].map(|x| json!({ "temp": x.temp, "place": x.place, "why": x.why })));
            v["off"] =
                json!(if d.rest() && bk.coaches { Some(if required[k] { "req" } else { "chosen" }) } else { None });
            v["off_why"] = json!(format!(
                "{org}: {} a week{}",
                match main.days_off {
                    1 => "one",
                    2 => "two",
                    _ => "days off",
                },
                match (st.tier, block) {
                    (Tier::College, Block::Spring) => " in spring practice",
                    (Tier::College, _) if in_season => " in season",
                    (Tier::College, _) => " out of season",
                    _ => "",
                }
            ));
            v["game_text"] = json!(this_game.filter(|g| g.date == date).map(|g| {
                format!(
                    "{} {}",
                    if g.home || g.neutral { "vs" } else { "at" },
                    crate::views::team_name(w, opp_ref(t, g.opp))
                )
            }));
            v["kick"] = json!(kick.map(week::clock_text));
            v["date"] = json!(date_text(date));
            v["weekday"] = json!(WEEKDAYS[k]);
            v["today"] = json!(date == w.today);
            v["past"] = json!(date < w.today);
            v["rule"] = json!(bk.rule);
            v["allowed"] = json!({
                "coaches": bk.coaches, "field": bk.field, "team": bk.team,
                "gear": bk.max_gear.label(), "contact": bk.max_contact.label(),
                "day_min": bk.day_min, "field_min": bk.field_min, "full_session": bk.full_session,
                "voluntary": bk.voluntary, "two_a_days": bk.two_a_days,
            });
            v
        })
        .collect();
    let breaches: Vec<Value> = afm_engine::training::rules::check_week(&pw.books, &pw.week)
        .into_iter()
        .map(|b| json!({ "day": b.day.map(|d| WEEKDAYS[d as usize]), "text": b.text }))
        .collect();
    // The week's work against your staff's default, family by family.
    let mine_w = afm_engine::training::work::week_work(&pw.week, Role::Starter, block == Block::Camp, 1.0);
    let base_w = afm_engine::training::work::week_work(&pw.neutral, Role::Starter, block == Block::Camp, 1.0);
    let work: Vec<Value> = Family::ALL
        .iter()
        .map(|&f| {
            let (a, b) = (mine_w[f as usize], base_w[f as usize]);
            json!({ "family": f.label(), "mine": (a * 10.0).round() / 10.0, "staff": (b * 10.0).round() / 10.0, "lean": if b + a < 0.5 { "none" } else if a > b * 1.08 + 0.5 { "more" } else if a < b * 0.92 - 0.5 { "less" } else { "same" } })
        })
        .collect();
    let (_, full_load, field) = afm_engine::training::work::week_load(&pw.week, Role::Starter, block == Block::Camp);
    let (_, n_full, n_field) = afm_engine::training::work::week_load(&pw.neutral, Role::Starter, block == Block::Camp);
    let prep = afm_engine::training::work::prep_minutes(&pw.week);
    let n_prep = afm_engine::training::work::prep_minutes(&pw.neutral);
    let gd = pw.week.days.iter().position(|d| d.game);
    let fresh = gd.map(|g| {
        let a = afm_engine::training::work::late_load(&pw.week, g, Role::Starter);
        let b = afm_engine::training::work::late_load(&pw.neutral, g, Role::Starter);
        afm_engine::training::work::freshness(a, b)
    });
    let style = w.career.as_ref().map_or_else(Default::default, |c| w.coaches[c.coach].style());
    // Players and their plans.
    let roles = ts::roles(w, &st);
    let by_id: HashMap<u32, f32> = w.training.moved.iter().copied().collect();
    let live = w.today == monday;
    let mut preps: Vec<(usize, u8, afm_engine::training::prep::Prep)> = Vec::new();
    let mut players: Vec<(usize, Value)> = st
        .roster
        .iter()
        .map(|&i| {
            let p = &w.people[i];
            let id = p.player.id;
            let pos = p.player.position;
            let eye = eye_for(w, t, pos);
            let mut v = row(w, i, t, true, reader);
            let role = *roles.get(&i).unwrap_or(&Role::Young);
            let tr = &p.player.train;
            let (lvl, trend) = load_words(tr.load[0], tr.load[1]);
            let (status, why) = practice_status(w, id, i);
            v["role"] = json!(role.label());
            v["role_k"] = json!(role as u8);
            v["load"] = json!(lvl);
            v["load_trend"] = json!(trend);
            v["spike"] = json!(tr.spike() > 1.45);
            v["grade"] = json!(tr.grade);
            v["grade_words"] = json!(grade_words(tr.grade));
            v["practice"] = json!(status);
            v["practice_why"] = json!(why);
            v["growth"] = json!(by_id.get(&id).map(|&m| growth_words(m as f64, eye, lean(reader, id))));
            v["plan"] = plan_json(w, id, pos);
            v["may_plan"] = json!(w.may_plan(id));
            v["sport"] =
                json!((st.tier == Tier::High).then(|| offseason::other_sport(&p.player).map(|s| s.name())).flatten());
            let hurt_why = w.training.hurt_on(id, monday).and_then(|h| h.kind).map(|k| k.name().to_lowercase());
            let sport = (st.tier == Tier::High)
                .then(|| offseason::sport_ratio(&p.player, pw.block, monday).map(|x| x.0.name().to_lowercase()))
                .flatten();
            let why = hurt_why.or(sport);
            let (prep, word, pr) = week::prep_json(w, &st, &pw, i, role, live, why.as_deref().unwrap_or(""));
            v["prep"] = prep;
            preps.push((i, word, pr));
            (pos.index(), v)
        })
        .collect();
    players.sort_by_key(|x| x.0);
    // Prep and effort across the roster (TR11, TR43).
    use afm_engine::training::prep::Status as PS;
    let mut words = [0u32; 6];
    let (mut missed, mut days_in, mut scores) = (0u32, 0u32, Vec::new());
    let count = |f: &dyn Fn(PS) -> bool| preps.iter().filter(|x| f(x.2.status)).count();
    for (_, word, pr) in &preps {
        words[*word as usize] += 1;
        if matches!(pr.status, PS::Full | PS::New | PS::Limited | PS::Rested) && pr.days > 0 {
            missed += pr.missed as u32;
            days_in += pr.days as u32;
            scores.push(pr.score);
        }
    }
    let extra = st.roster.iter().filter(|&&i| w.training.plan_of(w.people[i].player.id).extra > 0).count();
    let effort = json!({
        "words": words,
        "attend": if days_in > 0 { Some((100.0 * (1.0 - missed as f64 / days_in as f64)).round()) } else { None },
        "missed": missed,
        "absent": count(&|s| s == PS::Absent),
        "limited": count(&|s| matches!(s, PS::Limited | PS::Rested)),
        "out": count(&|s| s == PS::Out),
        "away": count(&|s| s == PS::Away),
        "new": count(&|s| s == PS::New),
        "extra": extra,
        "word": (!scores.is_empty()).then(|| week::effort_word(scores.iter().sum::<f64>() / scores.len() as f64)),
        "voluntary": pw.books.iter().any(|b| b.voluntary),
        "none": preps.iter().all(|x| x.1 == 0),
    });
    // Your staff's reasons for this week (T1).
    let limited = preps
        .iter()
        .filter(|x| x.2.status == PS::Limited && x.2.game && roles.get(&x.0) == Some(&Role::Starter))
        .map(|x| &w.people[x.0])
        .max_by(|a, b| a.player.overall().total_cmp(&b.player.overall()))
        .map(|p| {
            format!("{} is limited, so he gets the install in the walkthrough and on film instead of reps.", p.name)
        });
    let first_year = afm_engine::world::staff_read::your_start(w).year() >= st.frame.season;
    let next_week_game = ahead.iter().any(|g| g.0 >= monday.add(7) && g.0 < monday.add(14));
    let reasons = week::reasons(
        w,
        &st,
        &pw,
        offseason::staff_reasons(&style, st.tier),
        this_game.map(|g| (g.date, crate::views::team_name(w, opp_ref(t, g.opp)), g.home || g.neutral)),
        next_week_game,
        limited,
        first_year,
    );
    // The opener, for an offseason week's game prep card.
    let opener = ahead.iter().find(|g| g.0 >= w.today).map(|g| format!("next game {} {}", date_text(g.0), g.1));
    // Across the league (TR42): the next opponent's rusty starters, and
    // the voluntary sessions' absences in the news.
    let opp_roster: Option<(String, Vec<usize>)> = this_game.map(|g| {
        let r = opp_ref(t, g.opp);
        (crate::views::team_name(w, r), roster_people(w, r))
    });
    let news: Vec<(String, String)> = w
        .calendar
        .events
        .range(w.today.add(-14)..=w.today)
        .rev()
        .flat_map(|(_, es)| es.iter())
        .filter(|e| e.text.contains("voluntary") && e.text.contains("is not at"))
        .map(|e| (e.nfl_team.map_or("The news".to_string(), |c| w.nfl_team_name(c)), e.text.clone()))
        .collect();
    let league = week::league(w, opp_roster.as_ref().map(|(n, r)| (n.as_str(), r.as_slice())), news);
    // The page's title and sub line.
    let weeks_in = {
        let mut a = monday;
        while a.add(-7) >= st.frame.start && block_of_week(&st.frame, a.add(-7)) == block {
            a = a.add(-7);
        }
        let mut b = monday;
        while b.add(7) < st.frame.end && block_of_week(&st.frame, b.add(7)) == block {
            b = b.add(7);
        }
        (a.days_to(monday) / 7 + 1, a.days_to(b) / 7 + 1)
    };
    let title = match this_game {
        Some(g) => format!(
            "Game week: {} {}",
            if g.home || g.neutral { "vs" } else { "at" },
            crate::views::team_name(w, opp_ref(t, g.opp))
        ),
        None if block == Block::Season => "Bye week".to_string(),
        None => block.label(st.tier).to_string(),
    };
    let sub = format!(
        "{} · {}{} · {} – {}{}",
        crate::views::team_name(w, t),
        block.label(st.tier),
        if weeks_in.1 > 1 { format!(", week {} of {}", weeks_in.0, weeks_in.1) } else { String::new() },
        date_text(monday).rsplit_once(' ').map_or_else(|| date_text(monday), |x| x.0.to_string()),
        date_text(monday.add(6)),
        this_game.and_then(|g| g.kick.clone()).map_or(String::new(), |k| format!(" · kickoff {k}"))
    );
    let notes: Vec<Value> = w
        .training
        .notes
        .iter()
        .rev()
        .take(60)
        .map(|n| json!({ "date": date_text(n.date), "id": n.id, "dept": n.dept, "text": n.text }))
        .collect();
    let log: Vec<Value> = w
        .training
        .log
        .iter()
        .rev()
        .take(8)
        .map(|l| {
            let named = |v: &Vec<(u32, u8)>| v.iter().map(|&(id, g)| json!({ "link": link_player(w, id), "grade": g })).collect::<Vec<_>>();
            json!({
                "monday": date_text(l.monday),
                "block": l.block.map(|b| b.label(st.tier)),
                "practices": l.week.days.iter().filter(|d| d.periods.iter().any(|p| p.kind.practice())).count(),
                "minutes": l.week.days.iter().map(|d| d.field()).sum::<u32>(),
                "snaps": l.snaps,
                "injuries": l.injuries,
                "fitted": l.fitted,
                "best": named(&l.best),
                "worst": named(&l.worst),
                "battles": l.battles.iter().take(8).map(|&(a, b, x, y)| json!({ "a": link_player(w, a), "b": link_player(w, b), "aw": x, "bw": y })).collect::<Vec<_>>(),
            })
        })
        .collect();
    let reports: Vec<Value> = w
        .training
        .reports
        .iter()
        .rev()
        .take(6)
        .map(|r| json!({ "date": date_text(r.date), "block": r.block.label(st.tier), "lines": r.lines.iter().map(|(a, b)| json!([a, b])).collect::<Vec<_>>() }))
        .collect();
    let hurt: Vec<Value> = w
        .training
        .hurt
        .iter()
        .filter(|h| h.until > w.today)
        .map(|h| json!({ "link": link_player(w, h.id), "id": h.id, "what": h.kind.map_or(h.how.clone(), |k| k.name().to_string()), "how": h.how, "from": date_text(h.from), "until": date_text(h.until), "football": h.kind.is_some() }))
        .collect();
    let cases: Vec<Value> = w
        .training
        .cases
        .iter()
        .rev()
        .map(|c| json!({ "date": date_text(c.date), "what": c.what, "how": c.how, "penalty": c.penalty }))
        .collect();
    let templates: Vec<Value> = ts::slots_for(st.tier)
        .into_iter()
        .map(|s| {
            let tpl = ts::default_template(&st, s);
            let yours = w.training.templates.iter().find(|(x, _)| *x == s).map(|(_, t)| t.clone());
            json!({
                "slot": s.name(), "label": s.label(), "yours": yours.is_some(),
                "days": yours.unwrap_or(tpl).iter().map(day_json).collect::<Vec<_>>(),
                "staff": ts::default_template(&st, s).iter().map(day_json).collect::<Vec<_>>(),
            })
        })
        .collect();
    // The year's blocks with their dates.
    let mut blocks: Vec<Value> = Vec::new();
    let mut m = monday_of(st.frame.start.add(1));
    let mut cur: Option<(Block, Date)> = None;
    while m < st.frame.end.add(7) {
        let b = block_of_week(&st.frame, m);
        match cur {
            Some((cb, _)) if cb == b => {}
            Some((cb, from)) => {
                blocks.push(json!({ "block": cb.name(), "label": cb.label(st.tier), "from": date_text(from), "to": date_text(m.add(-1)), "now": monday >= from && monday < m, "own": cb.own() }));
                cur = Some((b, m));
            }
            None => cur = Some((b, m)),
        }
        m = m.add(7);
    }
    if let Some((cb, from)) = cur {
        blocks.push(json!({ "block": cb.name(), "label": cb.label(st.tier), "from": date_text(from), "to": date_text(st.frame.end), "now": monday >= from, "own": cb.own() }));
    }
    let cards = week::cards(&pw, st.tier, &ts::counts_text(&st, block, &counts), opener);
    let mut out = json!({
        "team": team_brief(w, t),
        "tier": tier_name(st.tier),
        "head": w.seat_head(),
        "monday": date_text(monday),
        "block": block.name(),
        "block_label": block.label(st.tier),
        "own": block.own() || pw.books.iter().all(|b| !b.coaches),
        "slot": pw.slot.name(),
        "slot_label": pw.slot.label(),
        "yours": pw.yours,
        "pushed": pw.pushed,
        "days": days,
        "fitted": pw.fitted,
        "breaches": breaches,
        "counts": ts::counts_text(&st, block, &counts),
        "limits": { "practices": (lim.practices != u16::MAX).then_some(lim.practices), "contact": (lim.contact != u16::MAX).then_some(lim.contact), "padded": (lim.padded != u16::MAX).then_some(lim.padded), "scrimmages": (lim.scrimmages != u16::MAX).then_some(lim.scrimmages) },
        "work": work,
        "load": { "full": full_load.round(), "staff_full": n_full.round(), "field": field.round(), "staff_field": n_field.round() },
        "prep": { "mine": prep.round(), "staff": n_prep.round() },
        "fresh": fresh.map(|f| (f * 100.0).round()),
        "reasons": reasons,
        "players": players.into_iter().map(|x| x.1).collect::<Vec<_>>(),
        "notes": notes,
        "log": log,
        "reports": reports,
        "hurt": hurt,
        "cases": cases,
        "templates": templates,
        "blocks": blocks,
        "options": options(&st),
        "habits": Tend::ALL.iter().enumerate().map(|(k, t)| json!({ "k": k, "label": t.label() })).collect::<Vec<_>>(),
        "positions": Position::ALL.iter().map(|p| p.name()).collect::<Vec<_>>(),
    });
    // The week's frame, prep and staff (update 2, lane 4).
    let more = json!({
        "title": title,
        "sub": sub,
        "rule": main.rule,
        "live": live,
        "year": week::year(&st, monday, w.today, &ahead),
        "cards": cards,
        "effort": effort,
        "staff": week::staff(w, st.tier),
        "families": week::families(&pw),
        "groups": week::lift_groups(w, &st, &pw, &roles),
        "body": week::body_targets(w, &st),
        "numbers": week::numbers(w, &st, monday, w.today),
        "where": week::wheres(w, &st, &pw),
        "league": league,
    });
    if let (Some(a), Value::Object(b)) = (out.as_object_mut(), more) {
        a.extend(b);
    }
    Ok(out)
}

/// Your two-deep's prep for the game day report (TR43): how many of each
/// word this week, and the ones your staff worries about, the poorest
/// first, with what your staff says.
pub(crate) fn prep_report(w: &World) -> Value {
    let Ok((_, st)) = team_of(w) else { return Value::Null };
    let monday = monday_of(w.today);
    let pw = ts::planned(w, &st, monday);
    if pw.books.iter().all(|b| !b.coaches) {
        return Value::Null;
    }
    let roles = ts::roles(w, &st);
    let live = w.today == monday;
    let mut words = [0u32; 6];
    let mut rows: Vec<(u8, u32, Value)> = Vec::new();
    for &i in &st.roster {
        let role = *roles.get(&i).unwrap_or(&Role::Young);
        if role == Role::Young {
            continue;
        }
        let p = &w.people[i];
        let why = w.training.hurt_on(p.player.id, monday).and_then(|h| h.kind).map(|k| k.name().to_lowercase());
        let (v, word, _) = week::prep_json(w, &st, &pw, i, role, live, why.as_deref().unwrap_or(""));
        // A player out hurt doesn't play this game: the injury report has him.
        if v["status"] == "out" {
            continue;
        }
        words[word as usize] += 1;
        if (1..=3).contains(&word) {
            rows.push((
                word,
                (100.0 - p.player.overall()).max(0.0) as u32,
                json!({
                    "id": p.player.id,
                    "name": p.name,
                    "pos": p.player.position.name(),
                    "role": role.label(),
                    "word": word,
                    "word_text": v["word_text"],
                    "five": v["five"],
                    "rust": v["rust"],
                    "says": v["says"],
                }),
            ));
        }
    }
    rows.sort_by_key(|r| (r.0, r.1));
    json!({
        "words": words,
        "rows": rows.into_iter().take(10).map(|r| r.2).collect::<Vec<_>>(),
    })
}

fn save_week(w: &mut World, args: &Value) -> Result<(), String> {
    mine_head(w)?;
    let days = parse_days(args)?;
    let push = args.get("push").and_then(Value::as_bool).unwrap_or(false);
    let monday = monday_of(w.today);
    let mut wk = Week { days };
    // The game days stay where the schedule has them.
    let st = w.training_team().ok_or("You have no team.")?;
    for (k, d) in wk.days.iter_mut().enumerate() {
        d.game = st.games.contains(&monday.add(k as i32));
    }
    ts::save_this_week(w, monday, wk, push).map_err(|b| {
        let list: Vec<String> = b
            .iter()
            .map(|x| match x.day {
                Some(d) => format!("{}: {}", WEEKDAYS[d as usize], x.text),
                None => x.text.clone(),
            })
            .collect();
        format!("Outside the rules: {}", list.join(" "))
    })
}

fn save_template(w: &mut World, args: &Value) -> Result<(), String> {
    mine_head(w)?;
    let slot = slot_arg(args)?;
    let days = parse_days(args)?;
    ts::save_template(w, slot, days).map_err(|b| {
        let list: Vec<String> = b
            .iter()
            .map(|x| match x.day {
                Some(d) => format!("{}: {}", WEEKDAYS[d as usize], x.text),
                None => x.text.clone(),
            })
            .collect();
        format!("Outside the rules: {}", list.join(" "))
    })
}

fn set_plan(w: &mut World, args: &Value) -> Result<(), String> {
    let id = args.get("id").and_then(Value::as_u64).ok_or("Missing id.")? as u32;
    let t = mine(w).ok_or("You have no team.")?;
    if !roster_people(w, t).iter().any(|&k| w.people[k].player.id == id) {
        return Err("He isn't on your team.".into());
    }
    if !w.may_plan(id) {
        return Err(format!("His plan is the head coach's. Your job covers {}.", w.your_controls()));
    }
    let mut plan = w.training.plan_of(id);
    if let Some(f) = args.get("focus") {
        let focus = match f.as_str() {
            Some(k) if !k.is_empty() => Some(Attr::ALL.into_iter().find(|a| a.key() == k).ok_or("No such skill.")?),
            _ => None,
        };
        w.set_focus(id, focus);
    }
    if let Some(h) = args.get("habit") {
        plan.habit = match h.as_array() {
            Some(a) if a.len() == 2 => Some((
                a[0].as_u64().unwrap_or(0).min(Tend::ALL.len() as u64 - 1) as u8,
                a[1].as_u64().unwrap_or(10).clamp(1, 20) as u8,
            )),
            _ => None,
        };
    }
    if let Some(b) = args.get("body") {
        plan.body = match b.as_array() {
            Some(a) if a.len() == 2 => {
                Some((a[0].as_u64().unwrap_or(200).clamp(150, 380) as u16, a[1].as_u64().unwrap_or(0).min(2) as u8))
            }
            _ => None,
        };
    }
    if let Some(m) = args.get("moving") {
        plan.moving = match m.as_str() {
            Some(pos) if !pos.is_empty() => {
                let to = Position::ALL.iter().position(|p| p.name() == pos).ok_or("No such position.")?;
                let i = person_index(w, id).ok_or("No such player.")?;
                if Position::ALL[to] == w.people[i].player.position {
                    None
                } else {
                    Some((to as u8, plan.moving.filter(|x| x.0 as usize == to).map_or(0, |x| x.1)))
                }
            }
            _ => None,
        };
    }
    if let Some(e) = args.get("extra").and_then(Value::as_u64) {
        plan.extra = (e & 7) as u8;
    }
    if let Some(r) = args.get("rest").and_then(Value::as_u64) {
        plan.rest = r.min(3) as u8;
    }
    w.training.set_plan(plan);
    Ok(())
}

// ---------- the Development tab (L8) ----------

fn dev(w: &World, id: u32) -> Reply {
    let i = person_index(w, id).ok_or("That player is no longer in football.")?;
    let t = mine(w).ok_or("You have no team.")?;
    let p = &w.people[i];
    if !roster_people(w, t).contains(&i) {
        return Ok(json!({ "mine": false, "name": p.name }));
    }
    let st = w.training_team().ok_or("You have no team.")?;
    let reader = reader_key(w);
    let pos = p.player.position;
    let eye = eye_for(w, t, pos);
    let tr = &p.player.train;
    let (lvl, trend) = load_words(tr.load[0], tr.load[1]);
    let (status, why) = practice_status(w, id, i);
    let roles = ts::roles(w, &st);
    let role = *roles.get(&i).unwrap_or(&Role::Young);
    let focus = w.focus_of(id).or(w.staff_focus(id));
    let attrs = reads::attributes(&p.player, true, eye, reader);
    let read_of = |a: Attr| attrs.iter().find(|x| x.0 == a).map(|x| x.1.json());
    // The skills that matter most at his position, with your staff's read.
    let mut key: Vec<(f64, Attr)> =
        Attr::ALL.iter().map(|&a| (afm_engine::positions::weight(pos, a), a)).filter(|x| x.0 > 0.02).collect();
    key.sort_by(|a, b| b.0.total_cmp(&a.0));
    let key: Vec<Value> = key.iter().take(8).map(|&(_, a)| json!({ "key": a.key(), "label": a.label(), "read": read_of(a), "family": afm_engine::training::family_of(a).label(), "focus": Some(a) == focus })).collect();
    // How far into the year's work each family is (the calendar, not his growth).
    let landed: f64 =
        if tr.pending() { Family::ALL.iter().map(|&f| tr.landed(f)).sum::<f64>() / FAMILIES as f64 } else { 1.0 };
    let moved = w.training.moved.iter().find(|m| m.0 == id).map(|m| m.1 as f64);
    let notes: Vec<Value> = w
        .training
        .notes
        .iter()
        .rev()
        .filter(|n| n.id == id)
        .take(14)
        .map(|n| json!({ "date": date_text(n.date), "dept": n.dept, "text": n.text }))
        .collect();
    let plan = w.training.plan_of(id);
    let habit = plan.habit.map(|(k, target)| {
        let tend = Tend::ALL[k as usize % Tend::ALL.len()];
        let v = p.player.tend.get(tend);
        json!({ "label": tend.label(), "target": target, "read": reads::range(v * 99.0 / 20.0, 6.0, reader, id, 200 + tend.index()).json(), "target_read": (target as f64 * 99.0 / 20.0).round() })
    });
    // His body now, as every other screen shows it and the body plan reads
    // it (a boy still growing is smaller than the man he becomes).
    let body = afm_engine::body::body_now(&p.player);
    let sport = (st.tier == Tier::High).then(|| offseason::other_sport(&p.player)).flatten();
    let mut log_lines: Vec<Value> = Vec::new();
    for l in w.training.log.iter().rev().take(6) {
        let g = l.best.iter().chain(l.worst.iter()).find(|x| x.0 == id).map(|x| x.1);
        let b: Vec<String> = l
            .battles
            .iter()
            .filter(|x| x.0 == id || x.1 == id)
            .map(|&(a, bb, x, y)| {
                let (other, won, lost) = if a == id { (bb, x, y) } else { (a, y, x) };
                format!("{won}-{lost} against {}", link_player(w, other))
            })
            .collect();
        if g.is_some() || !b.is_empty() {
            log_lines.push(json!({ "monday": date_text(l.monday), "grade": g, "battles": b }));
        }
    }
    Ok(json!({
        "mine": true,
        "id": id,
        "name": p.name,
        "pos": pos.name(),
        "class": class_text(w, p),
        "age": p.age_on(w.today),
        "role": role.label(),
        "may_plan": w.may_plan(id),
        "plan": plan_json(w, id, pos),
        "teacher": teacher(w, t, i),
        "load": { "level": lvl, "trend": trend, "spike": tr.spike() > 1.45, "acute": tr.load[0], "chronic": tr.load[1] },
        "practice": { "status": status, "why": why },
        "grade": { "value": tr.grade, "words": grade_words(tr.grade) },
        "growth": moved.map(|m| growth_words(m, eye, lean(reader, id))),
        "year": offseason::year_words(landed),
        "key": key,
        "habit": habit,
        "weight": body.weight.round(),
        "height": crate::views::ft_in(body.height),
        "sport": sport.map(|s| json!({ "name": s.name(), "now": s.in_season(w.today) })),
        "mood": mood_words(p),
        "notes": notes,
        "practice_log": log_lines,
        "habits": Tend::ALL.iter().enumerate().map(|(k, t)| json!({ "k": k, "label": t.label() })).collect::<Vec<_>>(),
        "positions": Position::ALL.iter().map(|p| p.name()).collect::<Vec<_>>(),
        "stage": match p.stage { Stage::HighSchool | Stage::PrepSchool => "hs", Stage::Pro => "nfl", _ => "college" },
    }))
}

// ---------- the plans: game plan, call sheet, playbook, installs ----------

fn sheet_play_json(w: &World, sp: &SheetPlay) -> Value {
    let fam = |f: PFamily| {
        let i = w.training.installs.iter().find(|i| i.key == InstallKey::Family(f as u8));
        let (reps, l) = i.map_or((0, 0.0), |i| (i.reps, i.level));
        (l, family_words(reps, l))
    };
    let (kind, name, family, (install, words)) = match sp.play {
        SheetKind::Run(r) => ("run", r.name().to_string(), r.family(), fam(r.family())),
        SheetKind::Pass(k) => {
            let c = pb::pass_concept(k as usize);
            let f = pb::pass_family(c);
            ("pass", c.name.to_string(), f, fam(f))
        }
        SheetKind::Design(k) => {
            let d = w.training.designs.get(k as usize);
            let l = w.training.install_level(InstallKey::Design(k));
            let words = if l < 0.35 { "not called until they know it" } else { install_words(l) };
            (
                "design",
                d.map_or("A deleted design".to_string(), |d| d.design.name.clone()),
                PFamily::Dropback,
                (l, words),
            )
        }
    };
    let play = match sp.play {
        SheetKind::Run(r) => json!(r.name()),
        SheetKind::Pass(k) => json!(pb::pass_concept(k as usize).name),
        SheetKind::Design(k) => json!(k),
    };
    json!({
        "personnel": sp.personnel.name(),
        "backfield": backfield_name(sp.backfield),
        "set": sp.set.name(),
        "kind": kind,
        "play": play,
        "name": name,
        "family": family.name(),
        "install": (install * 100.0).round(),
        "words": words,
    })
}

/// A playbook family: the base playbook is known; reps on the card sharpen it.
fn family_words(reps: u32, l: f32) -> &'static str {
    if reps == 0 {
        "base playbook"
    } else if l < 0.33 {
        "a few extra reps"
    } else if l < 0.66 {
        "sharpening"
    } else if l < 0.9 {
        "sharp"
    } else {
        "polished"
    }
}

fn install_words(l: f32) -> &'static str {
    if l < 0.15 {
        "new: busts likely"
    } else if l < 0.33 {
        "learning it"
    } else if l < 0.66 {
        "they know their jobs"
    } else if l < 0.9 {
        "sharp"
    } else {
        "mastered"
    }
}

fn backfield_name(b: Backfield) -> &'static str {
    match b {
        Backfield::UnderCenter => "under center",
        Backfield::Shotgun => "shotgun",
        Backfield::Pistol => "pistol",
    }
}

pub(crate) fn personnel_of(s: &str) -> Personnel {
    match s {
        "12" => Personnel::P12,
        "21" => Personnel::P21,
        "10" => Personnel::P10,
        "22" => Personnel::P22,
        "13" => Personnel::P13,
        _ => Personnel::P11,
    }
}

pub(crate) fn backfield_of(s: &str) -> Backfield {
    match s {
        "under center" => Backfield::UnderCenter,
        "pistol" => Backfield::Pistol,
        _ => Backfield::Shotgun,
    }
}

pub(crate) fn set_of(s: &str) -> Set {
    match s {
        "trips" => Set::Trips,
        "bunch" => Set::Bunch,
        "stack" => Set::Stack,
        "empty" => Set::Empty,
        "wing" => Set::Wing,
        _ => Set::Doubles,
    }
}

fn motion_of(s: &str) -> Motion {
    match s {
        "jet" => Motion::Jet,
        "orbit" => Motion::Orbit,
        "across" => Motion::Across,
        "shift" => Motion::Shift,
        _ => Motion::None,
    }
}

fn plans(w: &World) -> Reply {
    let (t, st) = team_of(w)?;
    let tp = &w.training;
    // The next game the plan is for.
    let next = st.games.iter().copied().filter(|&g| g >= w.today).min();
    let opp = next.and_then(|g| crate::views::next_game(w, t).filter(|_| g >= w.today));
    let sheet: Vec<Value> = SHEET
        .iter()
        .enumerate()
        .map(|(k, name)| {
            let plays: Vec<Value> = tp
                .plan
                .sheet
                .iter()
                .find(|(s, _)| *s as usize == k)
                .map_or(Vec::new(), |(_, v)| v.iter().map(|sp| sheet_play_json(w, sp)).collect());
            json!({ "k": k, "name": name, "plays": plays })
        })
        .collect();
    // The playbook by family: runs, the library's pass concepts, your designs.
    let families: Vec<Value> = PFamily::ALL
        .iter()
        .filter(|f| {
            !matches!(f, PFamily::Front | PFamily::SingleHigh | PFamily::TwoHigh | PFamily::Man | PFamily::Pressure)
        })
        .map(|&f| {
            let runs: Vec<&str> = RUN_CONCEPTS.iter().filter(|r| r.family() == f).map(|r| r.name()).collect();
            let passes: Vec<&str> = PASS_CONCEPTS.iter().filter(|c| pb::pass_family(c) == f).map(|c| c.name).collect();
            let inst = tp.installs.iter().find(|i| i.key == InstallKey::Family(f as u8));
            json!({
                "family": f.name(), "k": f as u8,
                "runs": runs, "passes": passes,
                "install": inst.map_or(0.0, |i| (i.level * 100.0).round()),
                "reps": inst.map_or(0, |i| i.reps),
                "words": family_words(inst.map_or(0, |i| i.reps), inst.map_or(0.0, |i| i.level)),
            })
        })
        .collect();
    let defense: Vec<Value> = [PFamily::Front, PFamily::SingleHigh, PFamily::TwoHigh, PFamily::Man, PFamily::Pressure]
        .iter()
        .map(|&f| {
            let inst = tp.installs.iter().find(|i| i.key == InstallKey::Family(f as u8));
            json!({ "family": f.name(), "install": inst.map_or(0.0, |i| (i.level * 100.0).round()), "reps": inst.map_or(0, |i| i.reps), "words": family_words(inst.map_or(0, |i| i.reps), inst.map_or(0.0, |i| i.level)) })
        })
        .collect();
    let designs: Vec<Value> = tp.designs.iter().enumerate().map(|(k, d)| design_json(w, k, d)).collect();
    // How fast it is going in: this week's team and situation minutes.
    let monday = monday_of(w.today);
    let pw = ts::planned(w, &st, monday);
    let team_min: u32 = pw
        .week
        .days
        .iter()
        .flat_map(|d| d.periods.iter())
        .filter(|p| matches!(p.kind, Kind::Team | Kind::Situations | Kind::SevenOnSeven | Kind::Walkthrough))
        .map(|p| p.min as u32)
        .sum();
    let on_sheet = tp.plan.sheet.iter().flat_map(|(_, v)| v.iter()).count();
    let installing = tp.designs.iter().filter(|d| d.installing).count();
    let keys = {
        let mut f: Vec<u8> = Vec::new();
        for (_, plays) in &tp.plan.sheet {
            for sp in plays {
                let x = match sp.play {
                    SheetKind::Run(r) => r.family() as u8,
                    SheetKind::Pass(k) => pb::pass_family(pb::pass_concept(k as usize)) as u8,
                    SheetKind::Design(_) => continue,
                };
                if !f.contains(&x) {
                    f.push(x);
                }
            }
        }
        f.len() + installing
    };
    let reps_each = if keys > 0 { (team_min as f64 * 1.5 / keys as f64).round() } else { 0.0 };
    Ok(json!({
        "team": team_brief(w, t),
        "tier": tier_name(st.tier),
        "head": w.seat_head(),
        "next": opp,
        "next_date": next.map(date_text),
        "lean": tp.plan.lean,
        "stick": if tp.plan.sheet.is_empty() { 0.85 } else { tp.plan.stick },
        "sheet": sheet,
        "on_sheet": on_sheet,
        "families": families,
        "defense": defense,
        "designs": designs,
        "pace": { "team_minutes": team_min, "keys": keys, "reps_each": reps_each, "to_know": if reps_each > 0.0 { (26.0 / reps_each).ceil() } else { 0.0 } },
        "runs": RUN_CONCEPTS.iter().map(|r| json!({ "name": r.name(), "family": r.family().name() })).collect::<Vec<_>>(),
        "passes": PASS_CONCEPTS.iter().map(|c| json!({ "name": c.name, "family": pb::pass_family(c).name() })).collect::<Vec<_>>(),
        "personnels": ["11", "12", "21", "10", "22", "13"],
        "backfields": ["shotgun", "under center", "pistol"],
        "sets": ["doubles", "trips", "bunch", "stack", "empty", "wing"],
        "coordinator": oc_name(w),
    }))
}

fn plans_set(w: &mut World, args: &Value) -> Result<(), String> {
    if w.seat.is_none() {
        return Err("You have no team.".into());
    }
    if let Some(l) = args.get("lean").and_then(Value::as_f64) {
        w.training.plan.lean = l.clamp(-0.2, 0.2) as f32;
    }
    if let Some(s) = args.get("stick").and_then(Value::as_f64) {
        w.training.plan.stick = s.clamp(0.0, 1.0) as f32;
    }
    w.training.plan.game = w.training_team().and_then(|st| st.games.iter().copied().filter(|&g| g >= w.today).min());
    w.refresh_seat_team();
    Ok(())
}

fn sheet_set(w: &mut World, args: &Value) -> Result<(), String> {
    if w.seat.is_none() {
        return Err("You have no team.".into());
    }
    let k = args.get("sit").and_then(Value::as_u64).ok_or("Missing situation.")? as u8;
    if k as usize >= SHEET.len() {
        return Err("No such situation.".into());
    }
    let list = args.get("plays").and_then(Value::as_array).cloned().unwrap_or_default();
    if list.len() > 12 {
        return Err("Twelve plays a situation at most: a call sheet is one card.".into());
    }
    let mut plays = Vec::new();
    for v in &list {
        let s = |key: &str| v.get(key).and_then(Value::as_str).unwrap_or("").to_string();
        let play = match s("kind").as_str() {
            "run" => SheetKind::Run(RUN_CONCEPTS.iter().copied().find(|r| r.name() == s("play")).ok_or("Pick a run.")?),
            "design" => {
                let d = v.get("play").and_then(Value::as_u64).ok_or("Pick a design.")? as u16;
                if d as usize >= w.training.designs.len() {
                    return Err("No such design.".into());
                }
                SheetKind::Design(d)
            }
            _ => SheetKind::Pass(
                PASS_CONCEPTS.iter().position(|c| c.name == s("play")).ok_or("Pick a pass concept.")? as u16
            ),
        };
        plays.push(SheetPlay {
            personnel: personnel_of(&s("personnel")),
            backfield: backfield_of(&s("backfield")),
            set: set_of(&s("set")),
            play,
            hash: v.get("hash").and_then(Value::as_u64).unwrap_or(0).min(2) as u8,
        });
    }
    let sheet = &mut w.training.plan.sheet;
    sheet.retain(|(s, _)| *s != k);
    if !plays.is_empty() {
        sheet.push((k, plays));
        sheet.sort_by_key(|x| x.0);
    }
    if w.training.plan.stick == 0.0 {
        w.training.plan.stick = 0.85;
    }
    w.training.plan.game = w.training_team().and_then(|st| st.games.iter().copied().filter(|&g| g >= w.today).min());
    w.refresh_seat_team();
    Ok(())
}

fn install_set(w: &mut World, args: &Value) -> Result<(), String> {
    let k = args.get("design").and_then(Value::as_u64).ok_or("Missing design.")? as usize;
    let on = args.get("on").and_then(Value::as_bool).unwrap_or(true);
    let d = w.training.designs.get_mut(k).ok_or("No such design.")?;
    d.installing = on;
    Ok(())
}

// ---------- the play designer (plays and tactics topic 8) ----------

const ROLES: [ORole; 15] = [
    ORole::QB,
    ORole::RB,
    ORole::FB,
    ORole::X,
    ORole::Z,
    ORole::Slot,
    ORole::Slot2,
    ORole::TE,
    ORole::TE2,
    ORole::TE3,
    ORole::LT,
    ORole::LG,
    ORole::C,
    ORole::RG,
    ORole::RT,
];

fn role_name(r: ORole) -> String {
    format!("{r:?}")
}

fn role_of(s: &str) -> Option<ORole> {
    ROLES.iter().copied().find(|r| role_name(*r) == s)
}

fn role_label(r: ORole) -> &'static str {
    match r {
        ORole::X => "X (split end)",
        ORole::Z => "Z (flanker)",
        ORole::Slot => "Slot",
        ORole::Slot2 => "Second slot",
        ORole::TE => "Tight end",
        ORole::TE2 => "Second tight end",
        ORole::TE3 => "Third tight end",
        ORole::RB => "Running back",
        ORole::FB => "Fullback",
        ORole::QB => "Quarterback",
        _ => afm_engine::game::scrimmage::orole_label(r),
    }
}

/// The routes a designer offers, with the depths that real route trees use.
fn route_list() -> Vec<Value> {
    let d = |name: &str, label: &str, depths: &[u8]| json!({ "name": name, "label": label, "depths": depths });
    vec![
        d("block", "Block", &[]),
        d("check-release", "Check, then release", &[]),
        d("go", "Go", &[]),
        d("fade", "Fade", &[]),
        d("seam", "Seam", &[]),
        d("slant", "Slant", &[]),
        d("out", "Out", &[5, 8, 12, 15]),
        d("dig", "Dig (in)", &[6, 10, 12, 15]),
        d("curl", "Curl", &[8, 10, 12, 14]),
        d("hitch", "Hitch", &[5, 6, 7]),
        d("comeback", "Comeback", &[12, 15, 18]),
        d("post", "Post", &[8, 10, 12, 15]),
        d("corner", "Corner", &[8, 10, 12, 15]),
        d("flat", "Flat", &[]),
        d("wheel", "Wheel", &[]),
        d("drag", "Drag", &[]),
        d("sit", "Sit in the zone", &[4, 6, 8, 10]),
        d("swing", "Swing", &[]),
        d("screen", "Screen", &[]),
        d("bubble", "Bubble", &[]),
        d("angle", "Angle", &[]),
    ]
}

fn route_of(name: &str, depth: f64) -> Option<Route> {
    let d = if depth > 0.0 { depth.clamp(2.0, 25.0) } else { 10.0 };
    Some(match name {
        "block" => Route::Block,
        "check-release" => Route::CheckRelease,
        "go" => Route::Go,
        "fade" => Route::Fade,
        "seam" => Route::Seam,
        "slant" => Route::Slant,
        "out" => Route::Out(d),
        "dig" => Route::In(d),
        "curl" => Route::Curl(d),
        "hitch" => Route::Hitch(d.min(8.0)),
        "comeback" => Route::Comeback(d),
        "post" => Route::Post(d),
        "corner" => Route::Corner(d),
        "flat" => Route::Flat,
        "wheel" => Route::Wheel,
        "drag" => Route::Drag,
        "sit" => Route::Sit(d.min(12.0)),
        "swing" => Route::Swing,
        "screen" => Route::Screen,
        "bubble" => Route::Bubble,
        "angle" => Route::Angle,
        _ => return None,
    })
}

fn drop_name(d: Drop) -> &'static str {
    match d {
        Drop::Quick => "quick",
        Drop::Medium => "medium",
        Drop::Deep => "deep",
        Drop::Boot => "boot",
        Drop::Screen => "screen",
        Drop::Bubble => "bubble",
    }
}

fn protection_name(p: Protection) -> &'static str {
    match p {
        Protection::Standard => "standard",
        Protection::Six => "six",
        Protection::Max => "max",
    }
}

fn parse_design(v: &Value) -> Result<Design, String> {
    let d = v.get("design").ok_or("Missing design.")?;
    let s = |k: &str| d.get(k).and_then(Value::as_str).unwrap_or("").to_string();
    let formation = Formation {
        personnel: personnel_of(&s("personnel")),
        backfield: backfield_of(&s("backfield")),
        set: set_of(&s("set")),
        strength: if s("strength") == "right" { -1.0 } else { 1.0 },
        motion: motion_of(&s("motion")),
    };
    let mut routes = Vec::new();
    for r in d.get("routes").and_then(Value::as_array).cloned().unwrap_or_default() {
        let role = r.get("role").and_then(Value::as_str).and_then(role_of).ok_or("A route has no player.")?;
        let name = r.get("route").and_then(Value::as_str).unwrap_or("block");
        let depth = r.get("depth").and_then(Value::as_f64).unwrap_or(0.0);
        let route = route_of(name, depth).ok_or("No such route.")?;
        if formation.personnel.roles().contains(&role) {
            routes.push((role, route));
        }
    }
    let reads: Vec<ORole> = d
        .get("reads")
        .and_then(Value::as_array)
        .map(|a| a.iter().filter_map(|x| x.as_str().and_then(role_of)).collect())
        .unwrap_or_default();
    let name = s("name");
    Ok(Design {
        name: if name.trim().is_empty() { "Untitled".into() } else { name.trim().chars().take(40).collect() },
        formation,
        routes,
        reads,
        protection: match s("protection").as_str() {
            "six" => Protection::Six,
            "max" => Protection::Max,
            _ => Protection::Standard,
        },
        drop: match s("drop").as_str() {
            "quick" => Drop::Quick,
            "deep" => Drop::Deep,
            "boot" => Drop::Boot,
            "screen" => Drop::Screen,
            "bubble" => Drop::Bubble,
            _ => Drop::Medium,
        },
        play_action: d.get("play_action").and_then(Value::as_bool).unwrap_or(false),
    })
}

fn design_to_json(d: &Design) -> Value {
    json!({
        "name": d.name,
        "personnel": d.formation.personnel.name(),
        "backfield": backfield_name(d.formation.backfield),
        "set": d.formation.set.name(),
        "strength": if d.formation.strength < 0.0 { "right" } else { "left" },
        "motion": match d.formation.motion { Motion::None => "none", Motion::Jet => "jet", Motion::Orbit => "orbit", Motion::Across => "across", Motion::Shift => "shift" },
        "routes": d.routes.iter().map(|(r, rt)| json!({ "role": role_name(*r), "route": rt.name(), "depth": rt.depth() })).collect::<Vec<_>>(),
        "reads": d.reads.iter().map(|r| role_name(*r)).collect::<Vec<_>>(),
        "protection": protection_name(d.protection),
        "drop": drop_name(d.drop),
        "play_action": d.play_action,
    })
}

/// The field picture of a design: everyone's spot at the snap and each
/// route's path, in yards (x downfield from the line, y across).
fn geometry(d: &Design) -> Value {
    let los = 30.0;
    let ball = afm_engine::game::geom::WIDTH / 2.0;
    let mut roles = vec![ORole::QB, ORole::LT, ORole::LG, ORole::C, ORole::RG, ORole::RT];
    roles.extend_from_slice(d.formation.personnel.roles());
    let players: Vec<Value> = roles
        .iter()
        .map(|&r| {
            let at = pb::align(&d.formation, r, los, ball);
            let route = d.routes.iter().find(|x| x.0 == r).map(|x| x.1);
            let path: Vec<[f64; 2]> = match route {
                Some(rt) if !rt.blocks() => {
                    let out = if at.y >= ball { 1.0 } else { -1.0 };
                    let (pts, _) = rt.points(at, los, out);
                    std::iter::once([at.x - los, at.y - ball])
                        .chain(pts.iter().map(|p| [p.x - los, p.y - ball]))
                        .collect()
                }
                _ => Vec::new(),
            };
            json!({
                "role": role_name(r),
                "label": afm_engine::game::scrimmage::orole_label(r),
                "x": at.x - los,
                "y": at.y - ball,
                "route": route.map(|r| r.name()),
                "path": path,
                "read": d.reads.iter().position(|x| *x == r).map(|k| k + 1),
                "line": r.is_line(),
            })
        })
        .collect();
    json!({ "players": players })
}

fn design_json(w: &World, k: usize, d: &SavedDesign) -> Value {
    let inst = w.training.install_level(InstallKey::Design(k as u16));
    json!({
        "k": k,
        "design": design_to_json(&d.design),
        "saved": date_text(d.saved),
        "review": d.review,
        "read": d.read.as_ref().map(|r| json!({ "snaps": r.snaps, "yards": (r.yards * 10.0).round() / 10.0, "completions": r.completions })),
        "installing": d.installing,
        "install": (inst * 100.0).round(),
        "words": install_words(inst),
        "geometry": geometry(&d.design),
        "on_sheet": w.training.plan.sheet.iter().any(|(_, v)| v.iter().any(|sp| sp.play == SheetKind::Design(k as u16))),
    })
}

fn designer_view(w: &World, args: &Value) -> Reply {
    let (t, st) = team_of(w)?;
    let k = args.get("k").and_then(Value::as_u64).map(|v| v as usize);
    let designs: Vec<Value> = w.training.designs.iter().enumerate().map(|(k, d)| design_json(w, k, d)).collect();
    Ok(json!({
        "team": team_brief(w, t),
        "tier": tier_name(st.tier),
        "designs": designs,
        "open": k,
        "routes": route_list(),
        "personnels": (["11", "12", "21", "10", "22", "13"]).iter().map(|p| json!({ "v": p, "roles": personnel_of(p).roles().iter().map(|r| json!({ "role": role_name(*r), "label": role_label(*r) })).collect::<Vec<_>>() })).collect::<Vec<_>>(),
        "backfields": ["shotgun", "under center", "pistol"],
        "sets": ["doubles", "trips", "bunch", "stack", "empty", "wing"],
        "motions": [["none", "No motion"], ["jet", "Jet motion"], ["orbit", "Orbit motion"], ["across", "Motion across"], ["shift", "Shift"]],
        "protections": [["standard", "Five, the back checks first"], ["six", "Six: the back stays in"], ["max", "Seven: back and tight end stay in"]],
        "drops": [["quick", "Quick (three-step)"], ["medium", "Five-step"], ["deep", "Seven-step"], ["boot", "Boot"], ["screen", "Screen"], ["bubble", "Bubble"]],
        "coordinator": oc_name(w),
    }))
}

fn oc_name(w: &World) -> Option<String> {
    match mine(w)? {
        TeamRef::College(c) => w.programs[c].oc.or(w.programs[c].head_coach).map(|x| w.coaches[x].name.clone()),
        TeamRef::Nfl(c) => w.nfl[c].oc.or(w.nfl[c].head_coach).map(|x| w.coaches[x].name.clone()),
        TeamRef::HighSchool(h) => w.high_schools[h].coach.map(|x| w.coaches[x].name.clone()),
        TeamRef::Other(f) => {
            let tm = w.others.team(f)?;
            tm.oc.or(tm.head_coach).map(|x| w.coaches[x].name.clone())
        }
    }
}

/// The coordinator who reviews designs, as the engine's play caller.
fn reviewer(w: &World) -> Option<afm_engine::game::roster::Caller> {
    let (team, _) = w.seat_game_team()?;
    Some(team.staff.offense)
}

fn design_draw(w: &World, args: &Value) -> Reply {
    let d = parse_design(args)?;
    let level = w.seat_game_team().map_or(afm_engine::player::Level::FBS, |x| x.1);
    let issues: Vec<String> = designer::legal(&d, level).iter().map(|i| i.text()).collect();
    Ok(json!({ "geometry": geometry(&d), "issues": issues, "design": design_to_json(&d) }))
}

fn design_save(w: &mut World, args: &Value) -> Result<usize, String> {
    if w.seat.is_none() {
        return Err("You have no team.".into());
    }
    let d = parse_design(args)?;
    let level = w.seat_game_team().map_or(afm_engine::player::Level::FBS, |x| x.1);
    let issues = designer::legal(&d, level);
    if !issues.is_empty() {
        return Err(format!("Not legal: {}.", issues.iter().map(|i| i.text()).collect::<Vec<_>>().join("; ")));
    }
    let mut rng = afm_engine::rng::Rng::new(w.options.seed ^ w.today.0 as u32 ^ 0xDE5);
    let review = reviewer(w).map_or_else(Vec::new, |c| designer::review(&d, &c, &mut rng));
    let replace = args.get("k").and_then(Value::as_u64).map(|v| v as usize).filter(|&k| k < w.training.designs.len());
    let saved = SavedDesign { design: d, saved: w.today, review, read: None, installing: true };
    let k = match replace {
        Some(k) => {
            w.training.designs[k] = saved;
            // A redrawn play is learned again.
            w.training.installs.retain(|i| i.key != InstallKey::Design(k as u16));
            k
        }
        None => {
            w.training.designs.push(saved);
            w.training.designs.len() - 1
        }
    };
    w.training.register_designs();
    w.refresh_seat_team();
    Ok(k)
}

fn design_delete(w: &mut World, args: &Value) -> Result<(), String> {
    let k = args.get("k").and_then(Value::as_u64).ok_or("Missing design.")? as usize;
    if k >= w.training.designs.len() {
        return Err("No such design.".into());
    }
    w.training.designs.remove(k);
    // Later designs move down one; the sheet and the installs follow.
    for (_, plays) in w.training.plan.sheet.iter_mut() {
        plays.retain(|sp| sp.play != SheetKind::Design(k as u16));
        for sp in plays.iter_mut() {
            if let SheetKind::Design(x) = sp.play
                && x as usize > k
            {
                sp.play = SheetKind::Design(x - 1);
            }
        }
    }
    w.training.installs.retain(|i| i.key != InstallKey::Design(k as u16));
    for i in w.training.installs.iter_mut() {
        if let InstallKey::Design(x) = i.key
            && x as usize > k
        {
            i.key = InstallKey::Design(x - 1);
        }
    }
    // The drafted sheet follows too (update 2, G7).
    afm_engine::world::plans::design_removed(w, k as u16);
    w.training.register_designs();
    w.refresh_seat_team();
    Ok(())
}

// ---------- the squad (six views) ----------

pub(crate) fn squad(w: &World) -> Reply {
    let (t, st) = team_of(w)?;
    let reader = reader_key(w);
    let roles = ts::roles(w, &st);
    let by_id: HashMap<u32, f32> = w.training.moved.iter().copied().collect();
    let order = w.seat_game_depth().unwrap_or_default();
    let mut rows: Vec<(usize, Value)> = roster_people(w, t)
        .into_iter()
        .map(|i| {
            let p = &w.people[i];
            let id = p.player.id;
            let pos = p.player.position;
            let eye = eye_for(w, t, pos);
            let mut v = row(w, i, t, true, reader);
            let role = *roles.get(&i).unwrap_or(&Role::Young);
            let tr = &p.player.train;
            let (lvl, trend) = load_words(tr.load[0], tr.load[1]);
            let (status, why) = practice_status(w, id, i);
            let attrs = reads::attributes(&p.player, true, eye, reader);
            let mut key: Vec<(f64, Attr)> = Attr::ALL.iter().map(|&a| (afm_engine::positions::weight(pos, a), a)).filter(|x| x.0 > 0.02).collect();
            key.sort_by(|a, b| b.0.total_cmp(&a.0));
            v["keys"] = json!(key.iter().take(4).map(|&(_, a)| json!({ "label": a.label(), "read": attrs.iter().find(|x| x.0 == a).map(|x| x.1.json()) })).collect::<Vec<_>>());
            for (k, (_, a)) in key.iter().take(4).enumerate() {
                v[format!("k{k}")] = attrs.iter().find(|x| x.0 == *a).map_or(Value::Null, |x| x.1.json());
                v[format!("k{k}_label")] = json!(a.label());
                v[format!("k{k}_key")] = json!(a.key());
            }
            v["role"] = json!(role.label());
            v["depth"] = json!(order.iter().find(|(x, _)| *x == pos).and_then(|(_, ids)| ids.iter().position(|&x| x == id)).map(|k| k + 1));
            v["load"] = json!(lvl);
            v["load_trend"] = json!(trend);
            v["spike"] = json!(tr.spike() > 1.45);
            v["grade"] = json!(tr.grade);
            v["grade_words"] = json!(grade_words(tr.grade));
            v["practice"] = json!(status);
            v["practice_why"] = json!(why);
            v["growth"] = json!(by_id.get(&id).map(|&m| growth_words(m as f64, eye, lean(reader, id))));
            let focus = w.focus_of(id).or(w.staff_focus(id));
            v["focus"] = json!(focus.map(|a| a.label()));
            v["focus_by"] = json!(if w.focus_of(id).is_some() { "you" } else { "staff" });
            v["teacher"] = teacher(w, t, i);
            let plan = w.training.plan_of(id);
            v["moving"] = json!(plan.moving.map(|(to, prog)| format!("{} ({prog}%)", Position::ALL[to as usize % Position::ALL.len()].name())));
            v["body_target"] = json!(plan.body.map(|b| b.0));
            v["habit"] = json!(plan.habit.map(|(k, _)| Tend::ALL[k as usize % Tend::ALL.len()].label()));
            v["born"] = json!(crate::views::born_text(p.born));
            v["sport"] = json!((st.tier == Tier::High).then(|| offseason::other_sport(&p.player).map(|s| s.name())).flatten());
            v["last_role"] = json!(p.player.history.last().map(|h| h.playing_time.name()));
            v["missed"] = json!(p.player.history.last().map(|h| h.games_missed));
            if let Some(c) = &p.pro {
                let k = c.contract.as_ref();
                v["apy"] = json!(k.map(|k| k.apy()));
                v["through"] = json!(k.map(|k| k.last_year()));
                v["accrued"] = json!(c.accrued);
            }
            v["years_left"] = json!(match p.stage {
                Stage::College => Some((4 - p.college_seasons as i32).max(0)),
                Stage::HighSchool | Stage::PrepSchool => Some((p.class_of - w.year).max(0)),
                _ => None,
            });
            v["redshirt"] = json!(p.redshirt_used);
            v["offers"] = json!(crate::views::offers_of(p).len());
            (pos.index(), v)
        })
        .collect();
    rows.sort_by_key(|x| x.0);
    Ok(
        json!({ "team": team_brief(w, t), "tier": tier_name(st.tier), "rows": rows.into_iter().map(|x| x.1).collect::<Vec<_>>() }),
    )
}

// ---------- the depth chart as formation boards ----------

/// The base boards: the offense in 11 personnel, the defense in its base
/// front, and the specialists.
const BASE_O: [(&str, Position, f64, f64); 11] = [
    ("QB", Position::QB, -5.0, 0.0),
    ("RB", Position::RB, -7.5, 0.0),
    ("X", Position::WR, 0.0, -21.0),
    ("Z", Position::WR, -1.0, 21.0),
    ("SLOT", Position::WR, -1.8, 13.0),
    ("TE", Position::TE, 0.0, 8.0),
    ("LT", Position::OT, 0.0, -4.4),
    ("LG", Position::IOL, 0.0, -2.2),
    ("C", Position::IOL, 0.0, 0.0),
    ("RG", Position::IOL, 0.0, 2.2),
    ("RT", Position::OT, 0.0, 4.4),
];
const BASE_D: [(&str, Position, f64, f64); 11] = [
    ("LE", Position::EDGE, 1.0, -5.5),
    ("DT", Position::DT, 1.0, -1.5),
    ("DT", Position::DT, 1.0, 1.5),
    ("RE", Position::EDGE, 1.0, 5.5),
    ("SAM", Position::LB, 4.5, 5.0),
    ("MIKE", Position::LB, 5.0, 0.0),
    ("WILL", Position::LB, 4.5, -5.0),
    ("CB", Position::CB, 6.0, -19.0),
    ("CB", Position::CB, 6.0, 19.0),
    ("FS", Position::S, 13.0, -6.0),
    ("SS", Position::S, 10.0, 6.0),
];
const BASE_S: [(&str, Position, f64, f64); 3] =
    [("K", Position::K, -7.0, -3.0), ("P", Position::P, -14.0, 0.0), ("LS", Position::LS, 0.0, 0.0)];

/// Where each package slot stands on the board (yards: depth, across).
fn package_spots(b: Board) -> Vec<(f64, f64)> {
    match b {
        Board::Nickel => vec![
            (1.0, -5.5),
            (1.0, -1.5),
            (1.0, 1.5),
            (1.0, 5.5),
            (5.0, -2.5),
            (5.0, 2.5),
            (6.0, -19.0),
            (5.0, 10.0),
            (6.0, 19.0),
            (13.0, -6.0),
            (11.0, 6.0),
        ],
        Board::Dime => vec![
            (1.0, -5.5),
            (1.0, -1.5),
            (1.0, 1.5),
            (1.0, 5.5),
            (6.0, 0.0),
            (6.0, -19.0),
            (5.0, 10.0),
            (5.0, -10.0),
            (6.0, 19.0),
            (14.0, -6.0),
            (12.0, 6.0),
        ],
        Board::GoalLineD => vec![
            (0.8, -6.0),
            (0.8, -2.5),
            (0.8, 0.0),
            (0.8, 2.5),
            (0.8, 6.0),
            (3.0, 4.0),
            (3.0, 0.0),
            (3.0, -4.0),
            (2.0, -12.0),
            (2.0, 12.0),
            (6.0, 0.0),
        ],
        Board::Jumbo => vec![
            (-5.0, 0.0),
            (0.0, -4.4),
            (0.0, -2.2),
            (0.0, 0.0),
            (0.0, 2.2),
            (0.0, 4.4),
            (0.0, -7.6),
            (0.0, 7.6),
            (-1.0, 10.0),
            (-7.0, 0.0),
        ],
        Board::GoalLineO => vec![
            (-2.0, 0.0),
            (0.0, -4.4),
            (0.0, -2.2),
            (0.0, 0.0),
            (0.0, 2.2),
            (0.0, 4.4),
            (0.0, -7.6),
            (0.0, 7.6),
            (-2.0, 11.0),
            (-4.0, 0.0),
            (-7.5, 0.0),
        ],
        Board::KickReturn => vec![
            (-15.0, 0.0),
            (-10.0, 0.0),
            (-6.0, -4.0),
            (-6.5, 0.0),
            (-6.0, 4.0),
            (-1.0, -18.0),
            (-1.0, -10.0),
            (-1.0, -3.0),
            (-1.0, 3.0),
            (-1.0, 10.0),
            (-1.0, 18.0),
        ],
        Board::KickCover => vec![
            (-5.0, -1.0),
            (-1.0, -22.0),
            (-1.0, -17.0),
            (-1.0, -12.0),
            (-1.0, -7.0),
            (-1.0, -3.0),
            (-1.0, 3.0),
            (-1.0, 7.0),
            (-1.0, 12.0),
            (-1.0, 17.0),
            (-1.0, 22.0),
        ],
        Board::PuntReturn => vec![
            (11.0, 0.0),
            (1.0, -22.0),
            (1.0, 22.0),
            (1.0, -6.0),
            (1.0, -3.5),
            (1.0, -1.0),
            (1.0, 1.0),
            (1.0, 3.5),
            (1.0, 6.0),
            (4.0, -10.0),
            (4.0, 10.0),
        ],
        Board::PuntCover => vec![
            (-14.0, 0.0),
            (0.0, 0.0),
            (0.0, -22.0),
            (0.0, 22.0),
            (-6.0, 0.0),
            (-1.0, -6.5),
            (-1.0, 6.5),
            (0.0, -2.0),
            (0.0, 2.0),
            (0.0, -4.0),
            (0.0, 4.0),
        ],
        Board::FieldGoal => vec![
            (-8.0, -1.5),
            (-7.0, 0.0),
            (0.0, 0.0),
            (0.0, -4.0),
            (0.0, -2.0),
            (0.0, 2.0),
            (0.0, 4.0),
            (0.0, -5.5),
            (0.0, 5.5),
            (-1.5, -8.0),
            (-1.5, 8.0),
        ],
    }
}

/// A slot's read for the board: his staff-read overall at the slot's
/// position (another position's player pays the cross-training cost the
/// engine charges).
fn slot_read(w: &World, t: TeamRef, i: usize, pos: Position, reader: u64) -> f64 {
    let p = &w.people[i];
    let eye = eye_for(w, t, p.player.position);
    let r = reads::overall(&p.player, true, eye, reader);
    let mid = match r {
        reads::Read::Range(lo, hi) => (lo as f64 + hi as f64) / 2.0,
        reads::Read::NotSeen => 40.0,
    };
    if p.player.position == pos { mid } else { mid - 8.0 }
}

pub(crate) fn boards(w: &World, args: &Value) -> Reply {
    boards_with(w, args, w.seat_game_depth().unwrap_or_default())
}

/// The same with your staff's depth order in hand (a board that has built
/// your game team already).
pub(crate) fn boards_with(w: &World, args: &Value, staff_order: Vec<(Position, Vec<u32>)>) -> Reply {
    let (t, st) = team_of(w)?;
    let reader = reader_key(w);
    let roster: Vec<usize> = st.roster.clone();
    let roles = ts::roles(w, &st);
    let by_id: HashMap<u32, usize> = roster.iter().map(|&i| (w.people[i].player.id, i)).collect();
    let who = |i: usize| {
        let p = &w.people[i];
        let eye = eye_for(w, t, p.player.position);
        let (status, _) = practice_status(w, p.player.id, i);
        json!({
            "id": p.player.id, "name": p.name, "pos": p.player.position.name(),
            "class": class_text(w, p), "ovr": reads::overall(&p.player, true, eye, reader).json(),
            "practice": status, "role": roles.get(&i).map(|r| r.label()),
            "ht": crate::views::ft_in(afm_engine::body::body_now(&p.player).height), "wt": afm_engine::body::body_now(&p.player).weight.round(),
        })
    };
    // A base board: the depth order at each slot's position, yours where
    // set, else your staff's.
    let order_at = |pos: Position| -> Vec<u32> {
        if let Some((_, ids)) = w.seat_depth.iter().find(|(p, _)| *p == pos) {
            return ids.clone();
        }
        staff_order.iter().find(|(p, _)| *p == pos).map(|(_, v)| v.clone()).unwrap_or_default()
    };
    let staff_at = |pos: Position| -> Vec<u32> {
        staff_order.iter().find(|(p, _)| *p == pos).map(|(_, v)| v.clone()).unwrap_or_default()
    };
    // A spot the order leaves empty (a thin room in February) shows the man
    // the engine would send: the best free player in the position group,
    // else the best on that side of the ball, else anyone.
    let game_team = w.seat_game_team().map(|x| x.0);
    let next_up = |pos: Position, taken: &HashSet<u32>| -> Option<usize> {
        let gt = game_team.as_ref()?;
        let free = |p: &&afm_engine::game::roster::GamePlayer| !taken.contains(&p.id) && by_id.contains_key(&p.id);
        let best = |f: &dyn Fn(&afm_engine::game::roster::GamePlayer) -> bool| {
            gt.players
                .iter()
                .filter(free)
                .filter(|p| f(p))
                .max_by(|a, b| a.overall_at(pos).total_cmp(&b.overall_at(pos)))
        };
        let off = |q: Position| {
            matches!(q, Position::QB | Position::RB | Position::WR | Position::TE | Position::OT | Position::IOL)
        };
        let side = |q: Position| q.is_specialist() == pos.is_specialist() && off(q) == off(pos);
        best(&|p| afm_engine::spots::group(p.position) == afm_engine::spots::group(pos))
            .or_else(|| best(&|p| side(p.position)))
            .or_else(|| best(&|_| true))
            .and_then(|p| by_id.get(&p.id).copied())
    };
    let base = |slots: &[(&str, Position, f64, f64)], key: &str, label: &str| {
        let mut used: HashMap<Position, usize> = HashMap::new();
        let mut picks: Vec<(Option<usize>, Option<usize>)> = slots
            .iter()
            .map(|&(_, pos, _, _)| {
                let k = used.entry(pos).or_insert(0);
                let cur = order_at(pos).get(*k).and_then(|id| by_id.get(id)).copied();
                let prop = staff_at(pos).get(*k).and_then(|id| by_id.get(id)).copied();
                *k += 1;
                (cur, prop)
            })
            .collect();
        let mut filled = vec![false; slots.len()];
        let mut taken: HashSet<u32> = picks.iter().filter_map(|x| x.0).map(|i| w.people[i].player.id).collect();
        for (k, &(_, pos, _, _)) in slots.iter().enumerate() {
            if picks[k].0.is_none()
                && let Some(i) = next_up(pos, &taken)
            {
                taken.insert(w.people[i].player.id);
                picks[k] = (Some(i), picks[k].1.or(Some(i)));
                filled[k] = true;
            }
        }
        let list: Vec<Value> = slots
            .iter()
            .zip(picks)
            .zip(filled)
            .map(|((&(name, pos, x, y), (cur, prop)), fill)| {
                json!({ "slot": name, "pos": pos.name(), "x": x, "y": y, "player": cur.map(who), "proposal": prop.map(who), "controls": w.controls(pos), "fill": fill })
            })
            .collect();
        let set_by_you = slots.iter().any(|s| w.seat_depth.iter().any(|(p, _)| *p == s.1));
        json!({ "key": key, "label": label, "slots": list, "yours": set_by_you, "kind": "base" })
    };
    let mut out = vec![
        base(&BASE_O, "offense", "Offense"),
        base(&BASE_D, "defense", "Defense"),
        base(&BASE_S, "specialists", "Specialists"),
    ];
    for b in Board::ALL {
        let mine = w.training.package(b);
        let spots = package_spots(b);
        let prop = proposal(w, t, b, &roster, &roles, reader, &staff_order);
        let list: Vec<Value> = b
            .slots()
            .iter()
            .enumerate()
            .map(|(k, &(name, pos))| {
                // Until you set the unit, your staff's picks are the unit.
                let cur = if mine.is_empty() { prop.get(k).copied().flatten() } else { mine.get(k).and_then(|id| by_id.get(id)).copied() };
                let (x, y) = spots.get(k).copied().unwrap_or((0.0, 0.0));
                json!({ "slot": name, "pos": pos.name(), "x": x, "y": y, "player": cur.map(who), "proposal": prop.get(k).copied().flatten().map(who), "controls": w.controls(pos) })
            })
            .collect();
        out.push(
            json!({ "key": b.name(), "label": b.label(), "slots": list, "yours": !mine.is_empty(), "kind": "package" }),
        );
    }
    let open = args.get("board").and_then(Value::as_str).unwrap_or("offense");
    let mut players: Vec<(usize, Value)> =
        roster.iter().map(|&i| (w.people[i].player.position.index(), who(i))).collect();
    players.sort_by_key(|x| x.0);
    Ok(json!({
        "team": team_brief(w, t),
        "tier": tier_name(st.tier),
        "head": w.seat_head(),
        "boards": out,
        "open": open,
        "players": players.into_iter().map(|x| x.1).collect::<Vec<_>>(),
    }))
}

/// Your position coaches' proposal for a package (topic 8 of the screens):
/// the best read at each slot's position; special teams lean on backups,
/// as real units do, with the fastest returners.
fn proposal(
    w: &World,
    t: TeamRef,
    b: Board,
    roster: &[usize],
    roles: &HashMap<usize, Role>,
    reader: u64,
    staff: &[(Position, Vec<u32>)],
) -> Vec<Option<usize>> {
    let by_id: HashMap<u32, usize> = roster.iter().map(|&i| (w.people[i].player.id, i)).collect();
    let special =
        matches!(b, Board::KickReturn | Board::KickCover | Board::PuntReturn | Board::PuntCover | Board::FieldGoal);
    let mut used: Vec<usize> = Vec::new();
    b.slots()
        .iter()
        .map(|&(name, pos)| {
            let returner = matches!(name, "KR" | "PR");
            let best = roster
                .iter()
                .copied()
                .filter(|i| !used.contains(i))
                .filter(|&i| {
                    let p = w.people[i].player.position;
                    if pos.is_specialist() || matches!(name, "H") {
                        p == pos
                    } else if special {
                        // Quarterbacks and the specialists never cover or
                        // block on a kick; linemen only on the field goal.
                        p != Position::QB
                            && !p.is_specialist()
                            && (b == Board::FieldGoal
                                || !matches!(p, Position::OT | Position::IOL | Position::DT)
                                || p == pos)
                    } else {
                        p == pos
                    }
                })
                .map(|i| {
                    let p = &w.people[i].player;
                    let mut s = slot_read(w, t, i, pos, reader);
                    if returner {
                        s = p.attributes.get(Attr::Speed) * 0.6
                            + p.attributes.get(Attr::Agility) * 0.25
                            + p.attributes.get(Attr::BallSecurity) * 0.15;
                        if !matches!(p.position, Position::WR | Position::RB | Position::CB | Position::S) {
                            s -= 30.0;
                        }
                    } else if special && !pos.is_specialist() {
                        // Coverage and return units are backups and young
                        // starters at the skill spots: a starter costs his
                        // legs for his own job.
                        if roles.get(&i) == Some(&Role::Starter) {
                            s -= 18.0;
                        }
                        if p.position == pos {
                            s += 6.0;
                        }
                        s += (p.attributes.get(Attr::Speed) - 60.0) * 0.25;
                    }
                    (i, s)
                })
                .max_by(|a, b| a.1.total_cmp(&b.1))
                .map(|x| x.0);
            // No one listed at a kicking job: the engine's own pick, the
            // best leg for the kick and anyone for the snap (its staff
            // list); the holder is then the backup quarterback.
            let best = best.or_else(|| {
                if !pos.is_specialist() {
                    return None;
                }
                if name == "H" {
                    let qb = staff.iter().find(|(p, _)| *p == Position::QB)?;
                    return qb.1.iter().skip(1).filter_map(|id| by_id.get(id).copied()).find(|i| !used.contains(i));
                }
                let list = staff.iter().find(|(p, _)| *p == pos)?;
                list.1.iter().filter_map(|id| by_id.get(id).copied()).find(|i| !used.contains(i))
            });
            if let Some(i) = best {
                used.push(i);
            }
            best
        })
        .collect()
}

pub(crate) fn board_set(w: &mut World, args: &Value) -> Result<Option<String>, String> {
    let key = args.get("board").and_then(Value::as_str).ok_or("Missing board.")?;
    let ids: Vec<Option<u32>> = args
        .get("ids")
        .and_then(Value::as_array)
        .map(|a| a.iter().map(|v| v.as_u64().map(|x| x as u32)).collect())
        .unwrap_or_default();
    let reset = args.get("reset").and_then(Value::as_bool).unwrap_or(false);
    if let Some(b) = Board::parse(key) {
        if !w.seat_head() && !b.slots().iter().all(|s| w.controls(s.1)) {
            return Err(format!("That unit is the head coach's. Your job covers {}.", w.your_controls()));
        }
        w.training.packages.retain(|(x, _)| *x != b);
        if !reset {
            let list: Vec<u32> = ids.iter().map(|x| x.unwrap_or(0)).collect();
            if list.iter().any(|&x| x != 0) {
                w.training.packages.push((b, list));
            }
        }
        w.refresh_seat_team();
        return Ok(None);
    }
    let slots: &[(&str, Position, f64, f64)] = match key {
        "offense" => &BASE_O,
        "defense" => &BASE_D,
        "specialists" => &BASE_S,
        _ => return Err("No such board.".into()),
    };
    let positions: Vec<Position> = {
        let mut v: Vec<Position> = slots.iter().map(|s| s.1).collect();
        v.dedup();
        v.sort_by_key(|p| p.index());
        v.dedup();
        v
    };
    let mut notes = Vec::new();
    for pos in positions {
        if !w.controls(pos) {
            continue;
        }
        if reset {
            w.set_seat_depth(pos, Vec::new());
            continue;
        }
        // The slot order first (an emptied slot takes the next man in the
        // order who isn't on the board), then the rest of the order as it
        // stood. A player moved to another position's slot leaves this one.
        let on_board: Vec<u32> = ids.iter().filter_map(|x| *x).collect();
        let before = w
            .seat_depth
            .iter()
            .find(|(p, _)| *p == pos)
            .map(|(_, v)| v.clone())
            .or_else(|| w.seat_game_depth().and_then(|o| o.into_iter().find(|(p, _)| *p == pos).map(|x| x.1)))
            .unwrap_or_default();
        let mut rest = before.iter().copied().filter(|id| !on_board.contains(id));
        let mut list: Vec<u32> = Vec::new();
        for (_, id) in slots.iter().zip(ids.iter()).filter(|(s, _)| s.1 == pos) {
            if let Some(x) = id.or_else(|| rest.next()) {
                list.push(x);
            }
        }
        for id in rest {
            if !list.contains(&id) {
                list.push(id);
            }
        }
        if let Some(n) = w.propose_depth(pos, list)? {
            notes.push(n);
        }
    }
    w.refresh_seat_team();
    Ok((!notes.is_empty()).then(|| notes.join(" ")))
}
