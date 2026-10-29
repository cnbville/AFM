//! The moves that fit a player, a coach or a team for your seat (minor
//! bugs 62-64): one list, by your seat and by who he is to you, shared by
//! the right-click menu, the player page's hero, the hover card and the
//! rows' ⋯ button. Each move is `{k, t, main?, ...}`: `k` names what the
//! screens do with it, `t` is its words, `main` puts it among the hero's
//! buttons, and the rest is what the move needs (an argument, a confirm
//! line, the numbers a dialog shows).

use afm_engine::world::front_office::college::{self as fc, Standing};
use afm_engine::world::front_office::{Duty, Owner, nfl, redshirt, reserve, school};
use afm_engine::world::pro::Status;
use afm_engine::world::scouting::{predraft, public};
use afm_engine::world::{Stage, World};
use serde_json::{Value, json};

use crate::views::{self, TeamRef};

fn mv(k: &str, t: impl Into<String>) -> Value {
    json!({ "k": k, "t": t.into() })
}

fn main(mut v: Value) -> Value {
    v["main"] = json!(true);
    v
}

/// Every move that fits player `id` for your seat, the main ones first.
pub fn player(w: &World, id: u32) -> Vec<Value> {
    let Some(k) = views::person_index(w, id) else { return Vec::new() };
    let p = &w.people[k];
    let seat = views::mine(w);
    let team = views::team_of(w, p);
    // A club in another league keeps its roster on the league's own list.
    let other_own =
        matches!(seat, Some(TeamRef::Other(f)) if w.others.team(f).is_some_and(|t| t.roster.contains(&(k as u32))));
    let own = team.is_some_and(|t| views::is_mine(w, t)) || other_own;
    let mut out: Vec<Value> = Vec::new();
    if own && p.stage != Stage::Done {
        out.push(main(mv("talk", "Talk to him")));
    }
    match seat {
        Some(TeamRef::Nfl(_)) => nfl_moves(w, k, own, &mut out),
        Some(TeamRef::College(j)) => college_moves(w, k, j, own, &mut out),
        Some(TeamRef::HighSchool(h)) => school_moves(w, k, h, own, &mut out),
        _ => {}
    }
    if own {
        out.push(mv("dev", "Development"));
    }
    // Your staff's looks: the file, and a new look where the rules allow.
    if p.stage != Stage::Done && afm_engine::world::scouting::readers::seat_staff(w).is_some() {
        let kinds = crate::lane_scouting::ask_kinds(w, p);
        if !kinds.is_empty() {
            let mut v = mv("ask", "Ask for a new look");
            v["kinds"] = json!(kinds);
            out.push(v);
        }
        out.push(mv("scout", "Scouting file"));
        // Your own eye on his tape, in the film room (lane 9, bug 82).
        if afm_engine::world::scouting::own::film_cost(w, id).is_ok() {
            out.push(mv("film", "Watch his film"));
        }
    }
    out.push(if crate::lane_core::desk::is_listed(w, id) {
        mv("list_remove", "Take off my players")
    } else {
        mv("list_add", "Add to my players")
    });
    out.push(mv("note", "Write a note"));
    out.push(mv("compare", "Compare"));
    out
}

fn nfl_moves(w: &World, k: usize, own: bool, out: &mut Vec<Value>) {
    let p = &w.people[k];
    let id = p.player.id;
    let talk = nfl::negotiable(w, k);
    let words = match talk {
        Some(nfl::TalkKind::ReSign) => Some("Re-sign him"),
        Some(nfl::TalkKind::Extension) => Some("Offer an extension"),
        Some(nfl::TalkKind::FreeAgent) => Some("Make an offer"),
        Some(nfl::TalkKind::OfferSheet) => Some("Offer sheet"),
        None => None,
    };
    if let Some(t) = words {
        out.push(main(mv("deal", t)));
    }
    let c = p.pro.as_ref();
    // The front office's moves are the head coach's (he is the GM).
    let head = w.seat_head();
    if own {
        if let Some(cap) = crate::lane_frontoffice::nfl::cap_moves(w, k).filter(|_| head) {
            if cap["re"].as_bool() == Some(true) {
                let mut v = mv("restructure", "Restructure");
                v["cap"] = cap.clone();
                out.push(v);
            }
            let mut v = mv("release", "Release him");
            v["cap"] = cap;
            out.push(v);
        }
        if head {
            let on = w.front_office.trades.block.contains(&id);
            let mut v = mv("block", if on { "Take off the trading block" } else { "Put on the trading block" });
            v["on"] = json!(!on);
            out.push(v);
        }
        // Injured reserve: your call, with your medical staff's word (bug 73).
        if let Ok(words) = reserve::place_words(w, id) {
            let mut v = mv("ir", "Place on injured reserve");
            v["words"] = json!(words);
            out.push(main(v));
        } else {
            match reserve::activate_words(w, id) {
                Ok(words) => {
                    let mut v = mv("ir_back", "Bring him back from injured reserve");
                    v["words"] = json!(words);
                    out.push(main(v));
                }
                // Healed but blocked (a full roster, no returns left): the
                // move shows with why, so it isn't a silent dead end. A
                // season-ending list (before the cutdown, or before 2012)
                // has no move at all.
                Err(why)
                    if head
                        && afm_engine::world::pro::games::ir_returns(w.year) != 0
                        && c.is_some_and(|c| c.status == Status::Reserve && c.out_weeks == 0 && !c.ir_out) =>
                {
                    let mut v = mv("ir_back", "Bring him back from injured reserve");
                    v["why"] = json!(why);
                    out.push(v);
                }
                Err(_) => {}
            }
        }
        return;
    }
    // Another club's player: a trade, and a look.
    if let Some(t) = c.and_then(|c| c.team).filter(|_| p.stage == Stage::Pro && head)
        && c.is_some_and(|c| matches!(c.status, Status::Roster | Status::Reserve))
    {
        let mut v = mv("trade", "Trade for him");
        v["team"] = json!(t);
        out.push(main(v));
    }
    // A free agent you can bring in.
    if talk == Some(nfl::TalkKind::FreeAgent)
        && w.today >= w.dates.league_year.date
        && !afm_engine::world::front_office::nfl::visited_recently(w, id)
    {
        out.push(mv("visit", "Bring him in for a visit"));
    }
    // A prospect of the coming draft: the four pre-draft lists.
    let dy = public::draft_year(w);
    let prospect = p.cal.draft_read.is_some()
        && (p.stage == Stage::Declared
            || (p.stage == Stage::College && dy - p.class_of >= 4 && w.today.parts().1 >= 8));
    if prospect {
        for (n, name) in predraft::LIST_NAMES.iter().enumerate() {
            let on = w.scouting.predraft.lists.iter().any(|x| x.0 == id && x.1 as usize == n);
            let mut v = mv(
                "predraft",
                if on {
                    format!("Take off {}", name.to_lowercase())
                } else {
                    format!("Add to {}", name.to_lowercase())
                },
            );
            v["list"] = json!(n);
            v["on"] = json!(!on);
            out.push(v);
        }
    }
}

fn college_moves(w: &World, k: usize, j: usize, own: bool, out: &mut Vec<Value>) {
    // The program's moves are the head coach's.
    if !w.seat_head() {
        return;
    }
    let p = &w.people[k];
    let id = p.player.id;
    if own {
        // Talk about staying (the portal) and stay or go (the draft).
        if p.cal.portal.is_none() && !w.front_office.college.talks.iter().any(|t| t.id == id) {
            out.push(mv("keep", "Talk about staying"));
        }
        if p.stage == Stage::College && p.cal.draft_read.is_some() {
            out.push(mv("stay_go", "Stay or go for the draft"));
        }
        if let Ok(r) = redshirt::read(w, id)
            && r.can.is_ok()
        {
            let mut v = mv("redshirt", if r.redshirt { "Play him" } else { "Redshirt him" });
            v["on"] = json!(!r.redshirt);
            v["games"] = json!(r.games);
            v["four"] = json!(r.four);
            out.push(v);
        }
        if let Ok(words) = fc::cut_words(w, id) {
            let mut v = mv("cut", "Cut him");
            v["words"] = json!(words);
            out.push(v);
        }
        return;
    }
    // A transfer in the portal: an offer, with NIL in its era.
    if p.cal.portal.is_some() && p.stage == Stage::College {
        out.push(main(mv("portal", "Offer him a spot")));
        return;
    }
    // A recruit.
    if fc::recruitable(w, k).is_err() {
        return;
    }
    let st = fc::standing(p, j);
    if matches!(st, Standing::SignedHere | Standing::SignedElsewhere | Standing::Enrolled | Standing::Gone) {
        out.push(mv("file", "His recruit file"));
        return;
    }
    let board = w.front_office.college.board.iter().find(|t| t.id == id);
    let offered = board.is_some_and(|t| t.offered.is_some() && t.pulled.is_none());
    // Offers and pulls wait on the board's owner: your coordinator's while
    // he runs it.
    if w.front_office.owner(Duty::Board) == Owner::You {
        out.push(main(if offered { mv("pull", "Pull the offer") } else { mv("offer", "Offer a scholarship") }));
    }
    if board.is_none() {
        out.push(main(mv("board", "Add to the board")));
    } else if !offered {
        out.push(mv("unboard", "Take off the board"));
    }
    out.push(mv("visit_plan", "Plan a visit"));
    if afm_engine::world::ncaa::nil_era(w.year) {
        out.push(mv("nil", "NIL promise"));
    }
    out.push(main(mv("file", "His recruit file")));
}

fn school_moves(w: &World, k: usize, h: usize, own: bool, out: &mut Vec<Value>) {
    if !own || !w.seat_head() {
        return;
    }
    let p = &w.people[k];
    let (jv, fr) = school::fields(w, h);
    let grade = school::grade(w, p.class_of);
    let now = school::squads(w).into_iter().find(|x| x.0 == k).map(|x| x.1);
    for to in school::Squad::ALL {
        let fits = match to {
            school::Squad::Varsity => true,
            school::Squad::Jv => jv && grade < 12,
            school::Squad::Freshman => fr && grade == 9,
        };
        if fits && Some(to) != now {
            let mut v = mv(
                "squad",
                format!("Move to {}", if to == school::Squad::Jv { "JV".into() } else { to.name().to_lowercase() }),
            );
            v["squad"] = json!(to.key());
            out.push(v);
        }
    }
    if p.stage == Stage::HighSchool && grade >= 10 {
        out.push(main(mv("call", "Call a college")));
    }
}

/// The moves for a coach (your assistants and the candidates for your openings).
pub fn coach(w: &World, c: usize) -> Vec<Value> {
    crate::lane_people::coach_moves(w, c)
}

/// The moves for a team: a trade with another NFL club.
pub fn team(w: &World, t: TeamRef) -> Vec<Value> {
    match (views::mine(w), t) {
        (Some(TeamRef::Nfl(me)), TeamRef::Nfl(o)) if me != o && w.seat_head() => {
            let mut v = mv("trade_with", "Trade with them");
            v["team"] = json!(o);
            vec![v]
        }
        _ => Vec::new(),
    }
}
