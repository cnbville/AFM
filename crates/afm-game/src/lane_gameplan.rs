//! Lane commands: the deeper game plan (update 2, lane 5: G1-G23). The
//! Plans page holds the program's decisions: the identity (your guidelines
//! for the coordinators, G1-G3), the standing game rules and the call
//! words (G4, G5), the self-scout (G6), the call sheet your coordinators
//! draft each week (G7-G13) and the installs (G14). The depth chart gets
//! its packages and the line rotation (G15). Game day's Game plan and
//! Report tabs hold the week against one opponent (G16-G22), and hiring,
//! recruiting and the draft board read fit against your guidelines (G23).
//!
//! Every rating here is your staff's read (a range or words), never a true
//! number. Everything works at every seat, high school, college and the
//! NFL, against each level's own baselines (Claude's picks, proposed, not
//! confirmed, in `docs/game.md`).

use afm_engine::calendar::Level as Cal;
use afm_engine::game::{GameTeam, PlayType};
use afm_engine::world::World;
use afm_engine::world::plans;
use afm_engine::world::seat::SeatGame;
use serde_json::Value;

use crate::views::{TeamRef, mine};
use crate::{Game, Reply};

pub mod depth;
pub mod fit;
pub mod identity;
pub mod installs;
pub mod likeus;
pub mod rules;
pub mod scout;
pub mod sheet;
pub mod week;

pub fn call(game: &mut Game, cmd: &str, args: &Value) -> Option<Reply> {
    let r = match cmd {
        "dp_identity" => (|| identity::view(game.w()?))(),
        "dp_guide" => (|| {
            let w = game.wm()?;
            identity::set(w, args)?;
            identity::view(w)
        })(),
        "dp_talk" => (|| {
            let w = game.wm()?;
            identity::talk(w)?;
            identity::view(w)
        })(),
        "dp_rules" => (|| rules::view(game.w()?))(),
        "dp_rules_set" => (|| {
            let w = game.wm()?;
            rules::set(w, args)?;
            rules::view(w)
        })(),
        "dp_words" => (|| {
            let w = game.wm()?;
            rules::words(w, args)?;
            rules::view(w)
        })(),
        "dp_scout" => (|| scout::view(game.w()?, args))(),
        "dp_study" => (|| {
            let w = game.wm()?;
            head(w)?;
            plans::study(w)?;
            scout::view(w, args)
        })(),
        "dp_sheet" => (|| sheet::view(game.wm()?, args))(),
        "dp_sheet_review" => (|| {
            let w = game.wm()?;
            sheet::review(w, args)?;
            sheet::view(w, args)
        })(),
        "dp_sheet_edit" => (|| {
            let w = game.wm()?;
            sheet::edit(w, args)?;
            sheet::view(w, args)
        })(),
        "dp_installs" => (|| installs::view(game.w()?, args))(),
        "dp_install_extra" => (|| {
            let w = game.wm()?;
            installs::extra(w, args)?;
            installs::view(w, args)
        })(),
        "dp_packages" => (|| depth::view(game.w()?, args))(),
        "dp_depth_set" => (|| {
            let w = game.wm()?;
            depth::set(w, args)?;
            depth::view(w, args)
        })(),
        "dp_rotation_set" => (|| {
            let w = game.wm()?;
            depth::rotation(w, args)?;
            depth::view(w, args)
        })(),
        "dp_week" => (|| week::view(game.w()?))(),
        "dp_week_set" => (|| {
            let w = game.wm()?;
            week::set(w, args)?;
            week::view(w)
        })(),
        "dp_like_us" => (|| likeus::view(game.w()?, args))(),
        "dp_like_us_add" => (|| {
            let w = game.wm()?;
            likeus::add(w, args)?;
            likeus::view(w, args)
        })(),
        "dp_fit_staff" => (|| fit::staff(game.w()?, args))(),
        "dp_fit_recruits" => (|| fit::recruits(game.w()?, args))(),
        _ => return None,
    };
    Some(r)
}

// ---------- shared ----------

/// Your team as the boards read it: its level and index, the screens'
/// reference and the game team the engine would dress.
pub(crate) struct Seat {
    pub level: Cal,
    pub t: usize,
    pub tr: TeamRef,
    pub team: GameTeam,
}

pub(crate) fn seat(w: &World) -> Result<Seat, String> {
    let (level, t) = w.seat_team().ok_or("You have no team.")?;
    let tr = mine(w).ok_or("You have no team.")?;
    let team = plans::our_team(w).ok_or("Your team can't dress for a game yet: the roster fills in first.")?;
    Ok(Seat { level, t, tr, team })
}

/// Only the head coach sets the program's decisions; an assistant sees
/// them.
pub(crate) fn head(w: &World) -> Result<(), String> {
    if w.seat.is_none() {
        return Err("You have no team.".into());
    }
    if !w.seat_head() {
        return Err("That's the head coach's call. You see his plan and coach to it.".into());
    }
    Ok(())
}

/// The level's key for the screens and its baselines.
pub(crate) fn lvl(level: Cal) -> &'static str {
    match level {
        Cal::Nfl | Cal::Other => "nfl",
        Cal::HighSchool => "hs",
        _ => "college",
    }
}

pub(crate) fn level_words(level: Cal) -> &'static str {
    match level {
        Cal::Nfl | Cal::Other => "NFL",
        Cal::HighSchool => "high school",
        _ => "college",
    }
}

/// Your staff's read of a coach's system knowledge (hidden: the true
/// values never reach a screen): a fixed lean per reader, coach and
/// system, `half` points wide either side on a 0-100 scale (about 8 for a
/// coach your staff works with or has studied, wider for a stranger).
/// Your own knowledge is yours.
pub(crate) fn know_read(w: &World, c: usize, half: f64) -> [f64; 11] {
    let k = w.coaches[c].system_know();
    if w.coaches[c].is_yours() {
        return k;
    }
    let reader = crate::views::reader_key(w);
    let mut out = [0.5; 11];
    for (s, v) in out.iter_mut().enumerate() {
        if let crate::reads::Read::Range(lo, hi) = crate::reads::range(100.0 * k[s], half, reader, c as u32, 300 + s) {
            *v = (lo as f64 + hi as f64) / 200.0;
        }
    }
    out
}

/// Your staff's read of a coach's hidden trait on its 1-20 scale (his
/// aggressiveness as a play caller, say), `half` points wide on 0-100.
pub(crate) fn trait_read(w: &World, c: usize, truth: f64, salt: usize, half: f64) -> f64 {
    if w.coaches[c].is_yours() {
        return truth;
    }
    match crate::reads::range(5.0 * truth, half, crate::views::reader_key(w), c as u32, 320 + salt) {
        crate::reads::Read::Range(lo, hi) => (lo as f64 + hi as f64) / 10.0,
        crate::reads::Read::NotSeen => 10.0,
    }
}

/// Your kept games of the season on the board (this one once it has
/// begun, else the last one), oldest first, with your side: the same
/// games the staff's self-scout reads (`plans::own_games`).
pub(crate) fn season_games<'a>(w: &'a World, _s: &Seat) -> Vec<(&'a SeatGame, usize)> {
    plans::own_games(w)
}

/// "This season" or "the 2026 season" (the season the boards read).
pub(crate) fn season_words(w: &World, _seat: &Seat) -> String {
    let s = plans::board_season(w);
    if s == w.today.year() && w.today.month() >= 8 { "this season".into() } else { format!("the {s} season") }
}

pub(crate) fn pct(x: f64) -> f64 {
    if x.is_finite() { (100.0 * x).round() } else { 0.0 }
}

/// A snap that is a run or a throw (sacks and scrambles were throws).
pub(crate) fn is_snap(k: PlayType) -> bool {
    matches!(k, PlayType::Run | PlayType::Pass | PlayType::Sack | PlayType::Scramble)
}

pub(crate) fn is_pass(k: PlayType) -> bool {
    matches!(k, PlayType::Pass | PlayType::Sack | PlayType::Scramble)
}

/// A team's short tag for a chart's foot: initials, or the first letters
/// of a one-word name; "@" for a road game.
pub(crate) fn abbr(name: &str, away: bool) -> String {
    let words: Vec<&str> =
        name.split_whitespace().filter(|x| !matches!(*x, "of" | "the" | "at" | "and" | "&")).collect();
    let mut s: String = if words.len() <= 1 {
        name.chars().filter(|c| c.is_alphanumeric()).take(4).collect::<String>().to_uppercase()
    } else {
        words.iter().filter_map(|x| x.chars().next()).take(4).collect::<String>().to_uppercase()
    };
    if s.is_empty() {
        s = "OPP".into();
    }
    if away { format!("@{s}") } else { s }
}

pub(crate) fn coach_link(w: &World, c: usize) -> String {
    format!("[[c:{c}|{}]]", w.coaches[c].name)
}

pub(crate) fn team_ref(level: Cal, i: usize) -> TeamRef {
    match level {
        Cal::Nfl => TeamRef::Nfl(i),
        Cal::HighSchool => TeamRef::HighSchool(i),
        Cal::Other => TeamRef::Other(i),
        _ => TeamRef::College(i),
    }
}

pub(crate) fn team_link(w: &World, level: Cal, i: usize) -> String {
    let r = team_ref(level, i).json();
    format!("[[t:{}:{}|{}]]", r["kind"].as_str().unwrap_or("college"), i, plans::team_name(w, level, i))
}

/// The opponent in a kept game.
pub(crate) fn opp_of(g: &SeatGame, side: usize) -> (usize, &str) {
    (g.teams[1 - side], g.names[1 - side].as_str())
}

/// A number's string without a trailing ".0".
pub(crate) fn num(x: f64, d: usize) -> String {
    let s = format!("{x:.d$}");
    if d > 0 && s.ends_with(&format!(".{}", "0".repeat(d))) { s[..s.len() - d - 1].to_string() } else { s }
}
