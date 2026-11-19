//! What he wants (lane N1, section 3.3): his wants today, the heaviest
//! first, each from the store that already holds it. A recruit weighs his
//! decider's wants (`decider_wants`), a college player his own
//! (`own_wants`), a pro and a prospect the market's goals
//! (`market::goals`), a booster his `cast::Want`s, a boss his goals for you
//! (`boss.rs`); a coach, an agent and a GM want what their makeup says,
//! and a coach on your staff wants to keep his job (`talk::stakes`).
//! A want you've already met (an open or kept promise, a deal agreed) is
//! marked, so what he asks next is what he wants most that you haven't
//! given (question 3).
//!
//! Weights are 0-1 and sum to 1 for each man. Every blend here is Claude's
//! (proposed, not confirmed).

use super::Them;
use super::makeup::{self, Makeup};
use super::terms::DealKind;
use super::wants::{Kind, Want, WantKind};
use crate::world::World;
use crate::world::background::{Decider, Wants};
use crate::world::folk::talks::PromiseKind;

/// How much of a recruit's wants are his decider's when someone else
/// decides (3.3: "his decider's wants"; he still says some of his own).
pub const DECIDER_SHARE: f32 = 0.7;

/// The six family wants, spread onto `WantKind`s: prestige is winning and
/// the brand, half each.
fn from_wants(x: &Wants, by: f32, out: &mut Vec<(WantKind, f32)>) {
    let add = |out: &mut Vec<(WantKind, f32)>, k: WantKind, v: f64| out.push((k, by * v as f32));
    add(out, WantKind::Money, x.money);
    add(out, WantKind::Place, x.close_to_home);
    add(out, WantKind::Role, x.playing_time);
    add(out, WantKind::Academics, x.academics);
    add(out, WantKind::Development, x.development);
    add(out, WantKind::Winning, 0.5 * x.prestige);
    add(out, WantKind::Name, 0.5 * x.prestige);
}

/// Folds repeats, drops the dust (under 2% of the whole), sorts heaviest
/// first and sums to 1.
fn finish(mut raw: Vec<(WantKind, f32)>) -> Vec<(WantKind, f32)> {
    let mut v: Vec<(WantKind, f32)> = Vec::new();
    for (k, x) in raw.drain(..) {
        match v.iter_mut().find(|y| y.0 == k) {
            Some(y) => y.1 += x.max(0.0),
            None => v.push((k, x.max(0.0))),
        }
    }
    let all: f32 = v.iter().map(|x| x.1).sum();
    v.retain(|x| all > 0.0 && x.1 / all >= 0.02);
    let s: f32 = v.iter().map(|x| x.1).sum();
    for x in v.iter_mut() {
        x.1 /= s;
    }
    v.sort_by(|a, b| b.1.total_cmp(&a.1));
    v
}

fn z(v: f64) -> f32 {
    Makeup::z(v) as f32
}

/// His wants today, with their weights, before what you've met.
pub fn raw(w: &World, him: Them) -> Vec<(WantKind, f32)> {
    let kind = makeup::kind(w, him);
    let m = makeup::makeup(w, him);
    let mut out = Vec::new();
    match him {
        Them::Player(_) => {
            let Some(i) = makeup::index(w, him) else { return out };
            let p = &w.people[i];
            match kind {
                Kind::Recruit => {
                    let f = &p.family;
                    if matches!(f.decider, Decider::Himself) {
                        from_wants(&f.own_wants, 1.0, &mut out);
                    } else {
                        from_wants(&f.decider_wants, DECIDER_SHARE, &mut out);
                        from_wants(&f.own_wants, 1.0 - DECIDER_SHARE, &mut out);
                    }
                }
                Kind::Prospect | Kind::Player if p.pro.is_some() || kind == Kind::Prospect => {
                    let g = crate::world::pro::market::goals(w, i);
                    out.push((WantKind::Money, g.money as f32));
                    out.push((WantKind::Security, g.security as f32));
                    out.push((WantKind::Winning, g.winning as f32));
                    out.push((WantKind::Role, g.role as f32));
                    out.push((WantKind::Place, (0.5 * g.city + 0.5 * g.home) as f32));
                    out.push((WantKind::Loyalty, (0.6 * g.loyalty) as f32));
                }
                _ => {
                    use super::talk::standing::{self, Road};
                    let road = standing::road(w, i);
                    let friends = crate::world::folk::ties::friends(w, p.player.id).len().min(4) as f32;
                    if road.college_ends() {
                        // His college career ends this year (Cain
                        // 2026-11-16: a senior with no eligibility asked
                        // for a role): the draft is in front of him, not a
                        // role next season or a school near home. What his
                        // family weighed still colours it: the money, being
                        // ready for the league, his degree.
                        let o = &p.family.own_wants;
                        out.push((WantKind::Draft, 0.45 + 0.15 * z(m.ambition)));
                        out.push((WantKind::Money, 0.5 * o.money as f32));
                        out.push((WantKind::Development, 0.05 + 0.5 * o.development as f32));
                        out.push((WantKind::Academics, 0.5 * o.academics as f32));
                        if road == Road::LastSeason {
                            // Still playing: his last snaps are his tape,
                            // and one more run with his teammates.
                            out.push((WantKind::Role, 0.08 * (1.0 + z(m.ambition)).max(0.0)));
                            out.push((WantKind::Winning, 0.08 + 0.3 * o.prestige as f32));
                        }
                        out.push((WantKind::Loyalty, 0.04 * (1.0 + z(m.loyalty)).max(0.0)));
                        out.push((WantKind::Friends, 0.015 * friends * (1.0 + z(m.loyalty)).max(0.0)));
                    } else {
                        // A college or high school player: his own wants,
                        // his role weighed by his ambition, and his friends.
                        from_wants(&p.family.own_wants, 1.0, &mut out);
                        out.push((WantKind::Role, 0.08 * (1.0 + z(m.ambition)).max(0.0)));
                        out.push((WantKind::Loyalty, 0.06 * (1.0 + z(m.loyalty)).max(0.0)));
                        out.push((WantKind::Friends, 0.025 * friends * (1.0 + z(m.loyalty)).max(0.0)));
                        if kind == Kind::Transfer {
                            out.push((WantKind::Role, 0.15));
                        }
                        // Free to declare: the draft weighs by his drive,
                        // and more once the scouts' word says early.
                        if road == Road::Deciding {
                            let early =
                                standing::scouts_word(w, i).is_some() && p.cal.draft_read.is_some_and(|r| r.round <= 3);
                            out.push((WantKind::Draft, 0.12 + 0.08 * z(m.ambition) + if early { 0.2 } else { 0.0 }));
                        }
                    }
                }
            }
        }
        Them::Kin(_, slot) => {
            let Some(i) = makeup::index(w, him) else { return out };
            let f = &w.people[i].family;
            // The decider weighs what the family weighs; the others the
            // same, and their son's good above all.
            from_wants(&f.decider_wants, if slot == 0 { 1.0 } else { 0.7 }, &mut out);
            out.push((WantKind::Family, if slot == 0 { 0.12 } else { 0.3 }));
        }
        // A man interviewing for your staff wants what a candidate wants,
        // whatever his job now (talk replies item 7: a high school coach in
        // a staff interview kept a feeder coach's wants and asked for
        // "clinic and camp invites"): the role and its path, the money, his
        // say in the scheme, a winner, a job that lasts, his family's move;
        // a high school coach also his best players recruited with him.
        Them::Coach(c) if super::coaching::interviewing(w, c) && kind != Kind::Staff => {
            out.push((WantKind::Career, 0.3 + 0.2 * z(m.ambition)));
            out.push((WantKind::Money, 0.2 + 0.15 * z(m.greed)));
            out.push((WantKind::Scheme, 0.12 + 0.1 * z(m.ego)));
            out.push((WantKind::Winning, 0.15));
            out.push((WantKind::Security, 0.1 + 0.08 * z(m.loyalty)));
            out.push((WantKind::Family, 0.08));
            if kind == Kind::HsCoach && !super::coaching::his_players(w, c).is_empty() {
                out.push((WantKind::Players, 0.15 + 0.1 * z(m.warmth) + 0.05 * z(m.ego)));
            }
        }
        Them::Coach(c) => match kind {
            Kind::HsCoach => {
                out.push((WantKind::Kids, 0.45 + 0.1 * z(m.warmth)));
                out.push((WantKind::Name, 0.2 + 0.1 * z(m.ego)));
                out.push((WantKind::Career, 0.15 + 0.15 * z(m.ambition)));
                out.push((WantKind::Winning, 0.15));
            }
            _ => {
                out.push((WantKind::Career, 0.3 + 0.2 * z(m.ambition)));
                out.push((WantKind::Money, 0.15 + 0.15 * z(m.greed)));
                out.push((WantKind::Scheme, 0.15 + 0.1 * z(m.ego)));
                out.push((WantKind::Winning, 0.2));
                out.push((WantKind::Loyalty, 0.1 + 0.1 * z(m.loyalty)));
                out.push((WantKind::Family, 0.08));
                // A man on your staff wants to keep his job: most of all a
                // holdover you haven't decided on (`talk::stakes`).
                if kind == Kind::Staff {
                    out.push((WantKind::Security, super::talk::stakes::job_weight(w, c)));
                }
            }
        },
        Them::Cast(r, _) => match kind {
            Kind::Booster => {
                let c = crate::world::folk::cast::get(w, r);
                let wants = c.booster.map(|b| b.wants).unwrap_or_default();
                let n = wants.len().max(1) as f32;
                for (k, x) in wants.iter().enumerate() {
                    // The first he names weighs most.
                    out.push((WantKind::Booster(*x), (n - k as f32) / n));
                }
                out.push((WantKind::Winning, 0.3));
            }
            Kind::Boss => {
                out.push((WantKind::Goals, 0.5));
                out.push((WantKind::Money, 0.2 + 0.15 * z(m.greed)));
                out.push((WantKind::Name, 0.2 + 0.1 * z(m.ego)));
                out.push((WantKind::Winning, 0.2));
            }
            Kind::Gm => {
                out.push((WantKind::Club, 0.5));
                out.push((WantKind::Winning, 0.25));
                out.push((WantKind::Career, 0.15 + 0.1 * z(m.ambition)));
            }
            // Your support staff (the trainer, the strength coach,
            // academics, operations, the cap manager, personnel) and the
            // outside: his own career and winning; the program's name is
            // the communications director's job, a little anyone else's
            // (the talks audit, 2026-11-16: the head trainer asked the
            // head coach how the program is seen from the outside).
            _ => {
                use crate::world::folk::cast::Role as CastRole;
                let name = if r.role == CastRole::Comms { 0.3 + 0.1 * z(m.ego) } else { 0.05 + 0.05 * z(m.ego) };
                out.push((WantKind::Career, 0.35 + 0.2 * z(m.ambition)));
                out.push((WantKind::Name, name.max(0.0)));
                out.push((WantKind::Winning, 0.25));
                if kind == Kind::Staff {
                    out.push((WantKind::Money, 0.1 + 0.1 * z(m.greed)));
                    out.push((WantKind::Loyalty, 0.1 + 0.1 * z(m.loyalty)));
                }
            }
        },
        Them::Agent(_) => {
            // His client's money, his fee, his name (3.3: fee and name are
            // new wants).
            out.push((WantKind::Money, 0.45));
            out.push((WantKind::Fee, 0.25 + 0.15 * z(m.greed)));
            out.push((WantKind::Name, 0.2 + 0.1 * z(m.ego)));
            out.push((WantKind::Security, 0.1));
        }
        Them::ProScout(_) => {
            out.push((WantKind::Career, 0.5 + 0.2 * z(m.ambition)));
            out.push((WantKind::Name, 0.3));
            out.push((WantKind::Club, 0.2));
        }
        Them::Drawn(..) => {
            out.push((WantKind::Career, 0.4 + 0.2 * z(m.ambition)));
            out.push((WantKind::Name, 0.3 + 0.1 * z(m.ego)));
            out.push((WantKind::Money, 0.15 + 0.1 * z(m.greed)));
        }
    }
    finish(out)
}

/// Whether you've already given a want (question 3): an open or kept P1
/// promise to him that covers it, or a deal agreed in a talk.
pub fn met(w: &World, him: Them, k: WantKind) -> bool {
    let to = him.promise_to();
    let promised = w.folk.promises.iter().any(|p| {
        Some(p.to) == to
            && p.kept != Some(false)
            && match k {
                WantKind::Role => matches!(
                    p.kind,
                    PromiseKind::Start
                        | PromiseKind::Snaps
                        | PromiseKind::Touches
                        | PromiseKind::Join(_)
                        | PromiseKind::Position(_)
                ),
                WantKind::Money => matches!(p.kind, PromiseKind::Contract),
                WantKind::Security | WantKind::Loyalty => matches!(p.kind, PromiseKind::Stay),
                WantKind::Career => matches!(p.kind, PromiseKind::Raise),
                WantKind::Booster(x) => p.kind == PromiseKind::Booster(x),
                _ => false,
            }
    });
    if promised {
        return true;
    }
    w.folk.live.persona.agreed.iter().any(|a| {
        a.with == him
            && match a.offer.kind {
                DealKind::Nil | DealKind::Contract | DealKind::Restructure => k == WantKind::Money,
                DealKind::Scholarship => matches!(k, WantKind::Role | WantKind::Academics | WantKind::Players),
                DealKind::Staff => matches!(k, WantKind::Career | WantKind::Money),
                DealKind::Booster => matches!(k, WantKind::Booster(_)),
                DealKind::Favour => k == WantKind::Kids,
                _ => false,
            }
    })
}

/// His wants today, the heaviest first, with what you've met marked.
pub fn wants(w: &World, him: Them) -> Vec<Want> {
    raw(w, him).into_iter().map(|(kind, weight)| Want { kind, weight, met: met(w, him, kind) }).collect()
}

/// What he wants most that you haven't given (question 3's seed for his
/// asks and counters).
pub fn top_unmet(w: &World, him: Them) -> Option<Want> {
    wants(w, him).into_iter().find(|x| !x.met)
}

/// A want in words, as he'd put it.
pub fn want_words(k: WantKind) -> &'static str {
    match k {
        WantKind::Role => "playing time, a real role",
        WantKind::Money => "the money",
        WantKind::Security => "security: guarantees, years",
        WantKind::Winning => "winning",
        WantKind::Place => "being close to home",
        WantKind::Development => "coaching that gets him to the next level",
        WantKind::Academics => "school and a degree",
        WantKind::Name => "the name: the brand, being seen",
        WantKind::Loyalty => "staying loyal to where he is",
        WantKind::Family => "what's best for the family",
        WantKind::Friends => "his friends and teammates",
        WantKind::Fee => "his fee",
        WantKind::Career => "his own career: the next job, a title",
        WantKind::Scheme => "his scheme and his say in it",
        WantKind::Club => "his club: its value, its cap, his own job",
        WantKind::Goals => "his goals for you, the budget, the program's name",
        WantKind::Booster(x) => x.short(),
        WantKind::Kids => "his kids' futures",
        WantKind::Draft => "the NFL draft: going as high as he can",
        WantKind::Players => "his best players recruited as part of the hire",
    }
}
