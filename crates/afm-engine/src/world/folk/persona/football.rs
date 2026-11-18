//! Football talk (lane N1, section 9, Cain's idea): what a man knows about
//! football and what he saw on the field, from the snaps the engine really
//! played. Every college and NFL snap is counted into his season's depth
//! counts (`tally::depth`, the players tally): pressures and sacks, time to
//! throw, the box, missed tackles, separation, blocks won and pressures
//! given up, rush wins by move, the blitz, coverage targets, kicks and
//! returns. He talks about his own spot, never a lumped group: a 3rd-down
//! back about pass pro, a slot about who's on him, a center about the A
//! gaps, an EDGE about his moves (9.2's table).
//!
//! What he can say follows his Football IQ (9.1): a freshman with a low
//! IQ knows his own job and one number, a sharp vet the whole picture. A
//! teammate or his coach saw the same tape; anyone else has the box score.
//! The line chart's protections, men kept in and stunt flag (your games
//! only) and doubles are for the terms lane (N2) on the play log; they
//! aren't counted here. High school games keep box scores, not snaps.

use super::Them;
use super::knows::{Known, Source, Topic};
use super::makeup::{self, Makeup};
use crate::positions::Position;
use crate::world::World;
use crate::world::tally::Level as L;
use crate::world::tally::depth::{self, ALIGN_O, Dc, FG, orole};

/// How many things a man can say about his football, by Football IQ: one at
/// a low IQ, up to five at the top (9.1).
pub fn reach(iq: f64) -> usize {
    (1.0 + (iq - 4.0).max(0.0) / 4.0).clamp(1.0, 5.0) as usize
}

/// His football IQ in words, with how long he's been at it (9.1).
pub fn iq_words(w: &World, i: usize) -> String {
    let m = makeup::makeup(w, Them::Player(w.people[i].player.id));
    let band = match m.iq {
        x if x >= 15.0 => "knows the whole call and why it was called",
        x if x >= 11.0 => "knows his job and the men around him",
        x if x >= 7.0 => "knows his own job",
        // "who is still learning": "a high school player who still
        // learning his own job" reached the voice (the talks audit,
        // 2026-11-16).
        _ => "is still learning his own job",
    };
    format!("{} who {band}", makeup::stage_words(w, i))
}

fn g(d: &depth::Depth, c: Dc) -> f64 {
    depth::get(d, c as u16)
}

fn per(n: f64, of: f64) -> f64 {
    if of > 0.0 { n / of } else { 0.0 }
}

fn pct(x: f64) -> String {
    format!("{:.0}%", 100.0 * x)
}

/// His spot as the snaps had it: a receiver in the slot or outside, a
/// lineman at center, guard or tackle (never lumped, 9.2).
fn spot(d: &depth::Depth, pos: Position) -> &'static str {
    let at = |r: u8| depth::get(d, ALIGN_O + r as u16);
    match pos {
        Position::WR => {
            let slot = at(orole::SLOT) + at(orole::SLOT2) + at(orole::SLOT3);
            let wide = at(orole::X) + at(orole::Z);
            if slot > wide { "slot" } else { "outside receiver" }
        }
        Position::IOL => {
            if at(orole::C) >= at(orole::LG) + at(orole::RG) {
                "center"
            } else {
                "guard"
            }
        }
        Position::OT => "tackle",
        Position::RB => {
            if g(d, Dc::PbSnaps) > g(d, Dc::RAtt) * 1.2 {
                "3rd-down back"
            } else {
                "every-down back"
            }
        }
        _ => pos.name(),
    }
}

/// What his season's snaps show, in words, the most telling first.
pub fn saw(d: &depth::Depth, pos: Position) -> Vec<String> {
    let mut v = Vec::new();
    match pos {
        Position::QB => {
            let db = g(d, Dc::Db);
            if db >= 10.0 {
                v.push(format!(
                    "pressured on {} of his dropbacks, sacked {} times",
                    pct(per(g(d, Dc::PrsDb), db)),
                    g(d, Dc::Sacked)
                ));
                v.push(format!("gets it out in {:.1} seconds on average", per(g(d, Dc::Ttt), g(d, Dc::Att))));
                v.push(format!("blitzed on {} of dropbacks", pct(per(g(d, Dc::BlzDb), db))));
                v.push(format!("throws {:.1} yards downfield on average", per(g(d, Dc::Air), g(d, Dc::Aimed))));
                v.push(format!("{} deep throws, {} caught", g(d, Dc::DeepAtt), g(d, Dc::DeepCmp)));
            }
        }
        Position::RB => {
            let c = g(d, Dc::RAtt);
            let third_down = g(d, Dc::PbSnaps) > c * 1.2;
            let carries = |v: &mut Vec<String>| {
                if c >= 5.0 {
                    v.push(format!("{} carries, {:.1} yards before contact a carry", c, per(g(d, Dc::RYbc), c)));
                    v.push(format!("8 or more in the box on {} of his carries", pct(per(g(d, Dc::RBox8), c))));
                    v.push(format!("{} missed tackles forced", g(d, Dc::RMtf)));
                }
            };
            let pro = |v: &mut Vec<String>| {
                let pb = g(d, Dc::PbSnaps);
                if pb >= 10.0 {
                    v.push(format!("{} snaps in pass protection, {} pressures given up", pb, g(d, Dc::PrsAllowed)));
                }
                if g(d, Dc::Tgt) >= 3.0 {
                    v.push(format!("{} targets, {} catches", g(d, Dc::Tgt), g(d, Dc::Rec)));
                }
            };
            if third_down {
                pro(&mut v);
                carries(&mut v);
            } else {
                carries(&mut v);
                pro(&mut v);
            }
        }
        Position::WR | Position::TE => {
            let t = g(d, Dc::Tgt);
            let routes = g(d, Dc::Routes);
            if pos == Position::TE {
                let blocks = g(d, Dc::PbSnaps) + g(d, Dc::RbSnaps);
                if blocks + routes >= 20.0 {
                    v.push(format!(
                        "blocked on {} of his snaps, ran {} routes",
                        pct(per(blocks, blocks + routes)),
                        routes
                    ));
                }
            }
            if routes >= 10.0 {
                v.push(format!("targeted {} times on {} routes", t, routes));
                if t >= 3.0 {
                    v.push(format!("{:.1} yards of separation when targeted", per(g(d, Dc::Sep), t)));
                    v.push(format!("{:.1} yards downfield a target", per(g(d, Dc::TgtAir), t)));
                    v.push(format!("{} contested targets, {} drops", g(d, Dc::ContTgt), g(d, Dc::Drop)));
                }
            }
        }
        Position::OT | Position::IOL => {
            let pb = g(d, Dc::PbSnaps);
            if pb >= 20.0 {
                v.push(format!(
                    "{} pressures and {} sacks given up in {} pass-block snaps",
                    g(d, Dc::PrsAllowed),
                    g(d, Dc::SacksAllowed),
                    pb
                ));
                v.push(format!("wins {} of his pass blocks", pct(per(g(d, Dc::PbWins), pb))));
            }
            let rb = g(d, Dc::RbSnaps);
            if rb >= 20.0 {
                v.push(format!("wins {} of his run blocks", pct(per(g(d, Dc::RbWins), rb))));
            }
            if g(d, Dc::Busts) >= 1.0 {
                v.push(format!("{} busted assignments", g(d, Dc::Busts)));
            }
        }
        Position::EDGE | Position::DT => {
            let pr = g(d, Dc::PrSnaps);
            if pr >= 20.0 {
                v.push(format!("{} pressures and {} sacks in {} rushes", g(d, Dc::Prs), g(d, Dc::Sacks), pr));
                let moves = [
                    (g(d, Dc::WinSpeed), "speed"),
                    (g(d, Dc::WinPower), "power"),
                    (g(d, Dc::WinFinesse), "finesse moves"),
                ];
                let wins = g(d, Dc::PrWins);
                if let Some(best) = moves.iter().max_by(|a, b| a.0.total_cmp(&b.0)).filter(|x| x.0 >= 2.0) {
                    v.push(format!("wins {} of his rushes, most of them on {}", pct(per(wins, pr)), best.1));
                }
                v.push(format!("{:.1} seconds to pressure on average", per(g(d, Dc::Ttp), g(d, Dc::TtpN))));
            }
            if g(d, Dc::DefRunSnaps) >= 20.0 {
                v.push(format!("{} run stops, {} missed tackles", g(d, Dc::RunStops), g(d, Dc::RunMiss)));
            }
        }
        Position::LB => {
            if g(d, Dc::Blitz) >= 3.0 {
                v.push(format!("blitzed {} times, {} pressures", g(d, Dc::Blitz), g(d, Dc::BlitzPrs)));
            }
            if g(d, Dc::Tkl) >= 5.0 {
                v.push(format!("{} tackles, {} missed, {} stops", g(d, Dc::Tkl), g(d, Dc::Miss), g(d, Dc::Stops)));
            }
            if g(d, Dc::TgtA) >= 3.0 {
                v.push(format!("targeted {} times in coverage, {} caught", g(d, Dc::TgtA), g(d, Dc::CmpA)));
            }
        }
        Position::CB | Position::S => {
            let t = g(d, Dc::TgtA);
            if t >= 3.0 {
                v.push(format!(
                    "targeted {} times in coverage, {} caught, {} broken up, {} picks",
                    t,
                    g(d, Dc::CmpA),
                    g(d, Dc::PbuA),
                    g(d, Dc::IntA)
                ));
                v.push(format!("{:.1} yards of separation allowed when targeted", per(g(d, Dc::SepA), t)));
            }
            if pos == Position::S && g(d, Dc::InBox) >= 5.0 {
                v.push(format!("rolled down into the box {} times", g(d, Dc::InBox)));
            }
            if g(d, Dc::Tkl) >= 5.0 {
                v.push(format!("{} tackles, {} missed", g(d, Dc::Tkl), g(d, Dc::Miss)));
            }
        }
        Position::K => {
            let bands = ["under 30", "30-39", "40-49", "50 and over"];
            let mut parts = Vec::new();
            for (b, name) in bands.iter().enumerate() {
                let tries = depth::get(d, FG + 2 * b as u16);
                if tries > 0.0 {
                    parts.push(format!("{} of {} from {name}", depth::get(d, FG + 2 * b as u16 + 1), tries));
                }
            }
            if !parts.is_empty() {
                v.push(format!("field goals: {}", parts.join(", ")));
            }
            if g(d, Dc::Ko) >= 5.0 {
                v.push(format!("touchbacks on {} of kickoffs", pct(per(g(d, Dc::KoTb), g(d, Dc::Ko)))));
            }
        }
        Position::P => {
            let n = g(d, Dc::Punts);
            if n >= 3.0 {
                v.push(format!("{:.1} yards a punt, {:.1} net", per(g(d, Dc::PuntYds), n), per(g(d, Dc::PuntNet), n)));
                v.push(format!("{} inside the 20, {} touchbacks", g(d, Dc::PuntIn20), g(d, Dc::PuntTb)));
            }
        }
        Position::LS => {}
    }
    // Returns, for whoever returns.
    if g(d, Dc::Kr) + g(d, Dc::Pr) >= 3.0 {
        v.push(format!(
            "{} kick returns for {} yards, {} punt returns for {}, {} fair catches",
            g(d, Dc::Kr),
            g(d, Dc::KrYds),
            g(d, Dc::Pr),
            g(d, Dc::PrYds),
            g(d, Dc::FairCatches)
        ));
    }
    v
}

/// His season's depth counts at his level (this season, or last when this
/// one hasn't started for him).
pub fn season(w: &World, i: usize) -> Option<(crate::world::tally::players::Season, i32)> {
    let p = &w.people[i];
    let level = match p.stage {
        crate::world::Stage::Pro => L::Nfl,
        crate::world::Stage::College | crate::world::Stage::Declared => L::College,
        _ => return None,
    };
    let id = p.player.id;
    let year = w.tally_open_season();
    for y in [year, year - 1] {
        if let Some(s) = w.tally_player(level, y, id).filter(|s| s.games > 0) {
            return Some((s, y));
        }
    }
    None
}

/// What `him` knows about the football of the man (or of himself, 9.1 and
/// 9.2). Himself and those who saw the tape (a teammate, his coach) get
/// the snaps, as far as their IQ reaches; anyone else gets the box score's
/// first line.
pub fn knows(w: &World, him: Them, about: Option<Them>, m: &Makeup) -> Vec<Known> {
    let subject = match about {
        Some(Them::Player(id)) => crate::world::folk::person(w, id),
        Some(_) => None,
        None => makeup::index(w, him).filter(|_| matches!(him, Them::Player(_))),
    };
    let Some(i) = subject else { return Vec::new() };
    let s = super::knowing::stand(w, him, i);
    let from = if s.own {
        Source::Himself
    } else if s.mate || s.coach {
        Source::Tape
    } else {
        Source::Public
    };
    let eye = super::knowing::eye(m, &s).min(if s.own { m.iq.max(1.0) } else { 20.0 });
    let mut v = Vec::new();
    if s.own {
        v.push(Known {
            topic: Topic::Football,
            about,
            from,
            eye,
            point: None,
            says: Vec::new(),
            words: iq_words(w, i),
            sure: 1.0,
            day: None,
        });
    }
    let Some((season, year)) = season(w, i) else { return v };
    let pos = w.people[i].player.position;
    let d = &season.depth;
    let n = match from {
        Source::Public => 1,
        _ => reach(m.iq),
    };
    let mut lines = saw(d, pos);
    lines.truncate(n);
    if lines.is_empty() {
        return v;
    }
    let mut words =
        format!("{} {}: {} games, {} starts; {}", year, spot(d, pos), season.games, season.starts, lines.join("; "));
    // His coaches know where his numbers stand among the conference's
    // starters at his spot (Cain 2026-11-17: "i want coaches to be aware
    // of team stats and player stats too, for in depth conversations").
    if s.coach
        && matches!(him, Them::Coach(_))
        && let Some(place) = super::review::player_place(w, i)
    {
        words.push_str(&format!("; {place}"));
    }
    v.push(Known {
        topic: Topic::Football,
        about,
        from,
        eye,
        point: None,
        says: Vec::new(),
        words,
        sure: super::knowing::sure_of(eye),
        day: None,
    });
    v
}
