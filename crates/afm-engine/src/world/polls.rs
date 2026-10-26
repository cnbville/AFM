//! How the voters rank teams (section 6 topic 9): each AP, Harris and
//! Coaches Poll voter fills a ballot of 25 from his own read of the
//! results, the BCS standings combine the polls with six computer
//! ratings, and the playoff committee ranks by its protocol.
//!
//! One shared read of the season is built per release (`Ev`): records,
//! who each team beat and lost to, a margin- and schedule-adjusted power
//! rating, the box score's eye test, the preseason view and the brand.
//! Each voter then weighs it with his own habits (`voters::Traits`) and
//! his last ballot. A release costs a few thousand small sums, so weekly
//! polls across a 30-year history take well under a second a season.

use std::collections::HashMap;

use super::World;
use super::college::PollKind;
use super::data::Division;
use super::season::key_normal;
use super::voters::{self, Traits};
use crate::calendar::{CfbPostseason, Date};

/// A poll's release: the top 25, each one's points and first-place votes.
#[derive(Clone, Debug, Default)]
pub(crate) struct Release {
    pub top: Vec<usize>,
    pub points: Vec<u32>,
    pub firsts: Vec<u16>,
}

/// One game from a team's side.
#[derive(Clone, Copy, Debug)]
pub(crate) struct GameEv {
    /// The opponent's index among the members, or `usize::MAX` below FBS.
    pub opp: usize,
    pub won: bool,
    pub margin: i32,
    pub date: Date,
    /// 1 at home, 0 neutral, -1 on the road.
    pub site: i8,
    /// The opponent's strength, 0-1.
    pub s: f64,
}

/// One team's season as the voters can read it.
#[derive(Clone, Debug, Default)]
pub(crate) struct TeamEv {
    pub team: usize,
    pub wins: u32,
    pub losses: u32,
    /// The résumé: wins weighted by who they came against, less losses
    /// weighted by how bad they were and how late (`r`), or with every
    /// week alike (`r0`, the committee's read).
    pub r: f64,
    pub r0: f64,
    /// The power rating (margin and schedule), the eye test (yards a play
    /// for and against), the average margin, strength of schedule, the
    /// preseason projection and the brand, each as a z-score.
    pub power: f64,
    pub eye: f64,
    pub margin: f64,
    pub sos: f64,
    pub pre: f64,
    pub brand: f64,
    pub g5: bool,
    pub champ: bool,
    pub conf: u16,
    pub state: usize,
}

/// The season so far, read once per release.
pub(crate) struct Ev {
    pub t: Vec<TeamEv>,
    pub games: Vec<Vec<GameEv>>,
    /// Program id to member index (`u16::MAX` for none).
    pub at: Vec<u16>,
    /// Latest meeting between two members: (lower, higher) index to winner.
    pub h2h: HashMap<(usize, usize), usize>,
}

impl Ev {
    pub fn idx(&self, prog: usize) -> Option<usize> {
        self.at.get(prog).copied().filter(|&k| k != u16::MAX).map(|k| k as usize)
    }
    fn beat(&self, a: usize, b: usize) -> bool {
        self.h2h.get(&(a.min(b), a.max(b))) == Some(&a)
    }
}

fn zscore(v: &mut [f64]) {
    let n = v.len().max(1) as f64;
    let m = v.iter().sum::<f64>() / n;
    let sd = (v.iter().map(|x| (x - m).powi(2)).sum::<f64>() / n).sqrt();
    // Capped at three standard deviations: one number far from the rest
    // (a lone big name, an early week when few teams have played) can't
    // swamp every other part of a ballot.
    for x in v.iter_mut() {
        *x = if sd > 1e-9 { ((*x - m) / sd).clamp(-3.0, 3.0) } else { 0.0 };
    }
}

/// How far into the season a date is, 0 at the opener and 1 at the last
/// regular-season Saturday (bowls a little past it).
fn season_frac(w: &World, d: Date) -> f64 {
    let weeks = &w.dates.cfb_weeks;
    let Some(first) = weeks.first() else { return 0.5 };
    let span = (weeks.len().max(2) - 1) as f64;
    (first.days_to(d) as f64 / 7.0 / span).clamp(0.0, 1.2)
}

/// A loss's weight: worse against a weak team, worse at home, worse by a
/// lot; `late` scales it by when it came (an early loss is forgiven more,
/// a late one hurts more: the voters' recency).
fn loss_weight(g: &GameEv, timing: f64) -> f64 {
    let m = g.margin.abs();
    let by = if m <= 7 {
        0.85
    } else if m >= 21 {
        1.15
    } else {
        1.0
    };
    let site = match g.site {
        1 => 1.06,
        0 => 0.96,
        _ => 0.92,
    };
    (0.9 + 1.1 * (1.0 - g.s)) * timing * by * site
}

fn win_weight(g: &GameEv) -> f64 {
    let by = if g.margin >= 17 {
        1.05
    } else if g.margin <= 3 {
        0.95
    } else {
        1.0
    };
    (0.35 + 0.65 * g.s) * by
}

/// The shared read of the FBS season so far.
pub(crate) fn evidence(w: &World) -> Ev {
    let cs = w.college.as_ref().expect("a college season");
    let year = cs.year;
    let members: Vec<usize> = w
        .programs
        .iter()
        .filter(|p| p.division == Division::FBS && !p.roster.is_empty() && p.plays(year))
        .map(|p| p.id)
        .collect();
    let n = members.len();
    let mut at = vec![u16::MAX; w.programs.len()];
    for (k, &p) in members.iter().enumerate() {
        at[p] = k as u16;
    }
    let mut games: Vec<Vec<GameEv>> = vec![Vec::new(); n];
    let mut h2h = HashMap::new();
    let mut played: Vec<&super::schedule::Game> = cs
        .games
        .iter()
        .filter(|g| g.played.is_some_and(|p| !p.cancelled))
        .filter(|g| at[g.home] != u16::MAX || at[g.away] != u16::MAX)
        .collect();
    played.sort_by_key(|g| g.date);
    for g in played {
        let p = g.played.unwrap();
        let (h, a) = (at[g.home], at[g.away]);
        let hm = p.score[0] as i32 - p.score[1] as i32;
        for (side, opp, m, site) in [(h, a, hm, 1i8), (a, h, -hm, -1i8)] {
            if side == u16::MAX {
                continue;
            }
            let won = if m == 0 { (site == 1) == p.home_won } else { m > 0 };
            games[side as usize].push(GameEv {
                opp: if opp == u16::MAX { usize::MAX } else { opp as usize },
                won,
                margin: m,
                date: g.date,
                site: if g.site.is_some() { 0 } else { site },
                s: 0.0,
            });
        }
        if h != u16::MAX && a != u16::MAX {
            let (h, a) = (h as usize, a as usize);
            let winner = if p.home_won { h } else { a };
            h2h.insert((h.min(a), h.max(a)), winner);
        }
    }
    // The power rating: margin (capped at 24) plus the opponent's rating,
    // averaged; a team below FBS counts 18 points worse than average.
    // Each step moves halfway to the new average, so a schedule that splits
    // into two sides (early weeks, a team whose foes played only it)
    // settles instead of swinging between two answers.
    let mut r = vec![0.0; n];
    for _ in 0..30 {
        let mut next: Vec<f64> = (0..n)
            .map(|i| {
                let l = &games[i];
                if l.is_empty() {
                    return 0.0;
                }
                l.iter()
                    .map(|g| g.margin.clamp(-24, 24) as f64 + if g.opp == usize::MAX { -18.0 } else { r[g.opp] })
                    .sum::<f64>()
                    / l.len() as f64
            })
            .collect();
        let mean = next.iter().sum::<f64>() / n.max(1) as f64;
        for v in next.iter_mut() {
            *v -= mean;
        }
        for (x, v) in r.iter_mut().zip(next) {
            *x = 0.5 * *x + 0.5 * v;
        }
    }
    // Who a team played, as voters judge it: the power rating once the
    // opponent has a few games, its August standing before (a week-one
    // loss to a preseason top-five team is a good loss).
    let mut pre_z: Vec<f64> = members
        .iter()
        .map(|&p| {
            let res = &cs.results[p];
            if res.preseason > 0.0 { res.preseason } else { super::college::projected_strength(w, p) }
        })
        .collect();
    let pre_raw = pre_z.clone();
    zscore(&mut pre_z);
    let strength = |o: usize| {
        if o == usize::MAX {
            return 0.06;
        }
        let g = games[o].len() as f64;
        let sure = g / (g + 2.0);
        sure / (1.0 + (-r[o] / 7.0).exp()) + (1.0 - sure) / (1.0 + (-1.2 * pre_z[o]).exp())
    };
    let s_of: Vec<Vec<f64>> = games.iter().map(|l| l.iter().map(|g| strength(g.opp)).collect()).collect();
    for (l, s) in games.iter_mut().zip(s_of) {
        for (g, v) in l.iter_mut().zip(s) {
            g.s = v;
        }
    }
    let avg_games = (games.iter().map(|l| l.len()).sum::<usize>() as f64 / n.max(1) as f64).max(1.0);
    let mut conf_names: Vec<String> = Vec::new();
    let mut t: Vec<TeamEv> = Vec::with_capacity(n);
    let (mut power, mut eye, mut margin, mut sos, mut pre, mut brand) =
        (Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new());
    for (i, &p) in members.iter().enumerate() {
        let l = &games[i];
        let res = &cs.results[p];
        let wins = l.iter().filter(|g| g.won).count() as u32;
        let losses = l.len() as u32 - wins;
        let quality: f64 = l.iter().filter(|g| g.won).map(win_weight).sum();
        let qn = if l.is_empty() { 0.0 } else { quality / l.len() as f64 * avg_games };
        let timed: f64 = l.iter().filter(|g| !g.won).map(|g| loss_weight(g, 0.75 + 0.5 * season_frac(w, g.date))).sum();
        let flat: f64 = l.iter().filter(|g| !g.won).map(|g| loss_weight(g, 1.0)).sum();
        let tot = &res.totals;
        let m = if l.is_empty() {
            0.0
        } else {
            // A win over a team from below counts as the power rating
            // reads it: 18 points are expected (voters shrug at cupcakes).
            l.iter().map(|g| g.margin.clamp(-35, 35) as f64 - if g.opp == usize::MAX { 18.0 } else { 0.0 }).sum::<f64>()
                / l.len() as f64
        };
        let e = if tot.plays > 0 && tot.plays_allowed > 0 {
            8.0 * (tot.yards / tot.plays as f64 - tot.yards_allowed / tot.plays_allowed as f64)
        } else {
            m
        };
        let prog = &w.programs[p];
        let conf = match conf_names.iter().position(|c| *c == prog.conference) {
            Some(k) => k,
            None => {
                conf_names.push(prog.conference.clone());
                conf_names.len() - 1
            }
        } as u16;
        power.push(r[i]);
        eye.push(e);
        margin.push(m);
        sos.push(if l.is_empty() { 0.5 } else { l.iter().map(|g| g.s).sum::<f64>() / l.len() as f64 });
        pre.push(pre_raw[i]);
        brand.push(prog.prestige);
        t.push(TeamEv {
            team: p,
            wins,
            losses,
            r: qn - timed,
            r0: qn - flat,
            g5: voters::group_of_five(w, p),
            champ: res.conference_title,
            conf,
            state: prog.state,
            ..Default::default()
        });
    }
    for v in [&mut power, &mut eye, &mut margin, &mut sos, &mut pre, &mut brand] {
        zscore(v);
    }
    for (i, x) in t.iter_mut().enumerate() {
        x.power = power[i];
        x.eye = eye[i];
        x.margin = margin[i];
        x.sos = sos[i];
        x.pre = pre[i];
        x.brand = brand[i];
    }
    Ev { t, games, at, h2h }
}

/// What each team's games since `since` did to a voter's view of it: a
/// win holds or nudges it up (more for a good win), a loss knocks it down
/// by the loss's weight.
fn shift_since(w: &World, ev: &Ev, since: Date) -> Vec<f64> {
    ev.games
        .iter()
        .map(|l| {
            l.iter()
                .filter(|g| g.date > since)
                .map(|g| {
                    if g.won {
                        0.12 + 0.5 * (g.s - 0.5).max(0.0)
                    } else {
                        -loss_weight(g, 0.75 + 0.5 * season_frac(w, g.date))
                    }
                })
                .sum()
        })
        .collect()
}

/// One voter as a ballot needs him.
pub(crate) struct Voter {
    pub traits: Traits,
    /// How much his own state and conference count (AP ballots public
    /// since 2007 halve it).
    pub era_homer: f64,
    pub state: Option<usize>,
    pub conf: Option<u16>,
    /// His own team, for a coach.
    pub own: Option<usize>,
    pub key: u64,
}

fn tf(v: u8) -> f64 {
    Traits::f(v)
}

fn homer_bonus(ev: &Ev, v: &Voter, i: usize) -> f64 {
    let h = (0.5 + tf(v.traits.homer)) * v.era_homer;
    let t = &ev.t[i];
    let mut b = 0.0;
    if v.state == Some(t.state) {
        b += 0.35 * h;
    }
    if v.conf == Some(t.conf) {
        b += 0.2 * h;
    }
    if v.own == Some(i) {
        b += 0.6 * (0.5 + tf(v.traits.homer));
    }
    b
}

/// A voter's fresh read of every team this week, before last week's
/// ballot holds him.
fn fresh(ev: &Ev, v: &Voter, week: u64) -> Vec<f64> {
    let tr = v.traits;
    let res = tf(tr.resume);
    let noise = 0.35 * (1.3 - tf(tr.care));
    (0..ev.t.len())
        .map(|i| {
            let t = &ev.t[i];
            let games = (t.wins + t.losses) as f64;
            // The ratings read from games say little after one or two of
            // them (one game, a third; eight, four fifths); what happened
            // (the résumé) counts in full.
            let sure = games / (games + 2.0);
            let mut f = t.r * (0.8 + 0.4 * res)
                + sure
                    * (t.power * (0.45 + 0.5 * (1.0 - res))
                        + t.eye * 0.35 * (1.3 - res)
                        + t.margin * 0.45 * tf(tr.margin))
                + t.pre * 1.2 * 0.8f64.powf(games)
                + t.brand * 0.35 * tf(tr.brand);
            if t.g5 {
                f -= 0.6 * tf(tr.g5);
            }
            f + homer_bonus(ev, v, i) + noise * key_normal(v.key ^ week, i as u64)
        })
        .collect()
}

/// A preseason read: last season's final ranking, what the roster brings
/// back and the program's name.
fn preseason(ev: &Ev, v: &Voter, last_final: &[f64], week: u64) -> Vec<f64> {
    let tr = v.traits;
    let noise = 0.45 * (1.3 - tf(tr.care));
    (0..ev.t.len())
        .map(|i| {
            let t = &ev.t[i];
            let mut f = 2.2 * last_final[i] * (0.6 + 0.8 * tf(tr.anchor))
                + 1.0 * t.pre
                + 0.45 * t.brand * (0.4 + 1.2 * tf(tr.brand));
            if t.g5 {
                f -= 0.3 * tf(tr.g5);
            }
            f + homer_bonus(ev, v, i) + noise * key_normal(v.key ^ week, i as u64)
        })
        .collect()
}

/// Puts a team that won the head-to-head above the team it beat when the
/// two sit close and the winner has no more losses ("teams rarely drop
/// below a team they beat").
fn head_to_head(ev: &Ev, order: &mut Vec<usize>) {
    for _ in 0..3 {
        let mut moved = false;
        let mut j = 1;
        while j < order.len() {
            let b = order[j];
            let from = j.saturating_sub(6);
            if let Some(k) = (from..j).find(|&k| ev.beat(b, order[k]) && ev.t[b].losses <= ev.t[order[k]].losses) {
                order.remove(j);
                order.insert(k, b);
                moved = true;
            }
            j += 1;
        }
        if !moved {
            break;
        }
    }
}

/// Places on a ballot per unit of a result's weight (`shift_since`): a
/// one-score road loss to a top-25 team weighs about 0.8, a win 0.12-0.37.
const PLACES: f64 = 8.0;

/// A voter's ballot of 25 (member indices): his fresh read held toward
/// last week's ballot after this week's results moved each team.
fn ballot(ev: &Ev, v: &Voter, fresh: &[f64], prev: &[usize], shift: &[f64]) -> Vec<usize> {
    // Early in the year a game or two says little, and voters hold their
    // August order more (real week-one polls barely move but for losses):
    // half the gap to a full hold after one game, fading as games pile up.
    let played = ev.games.iter().map(|l| l.len()).sum::<usize>() as f64 / ev.games.len().max(1) as f64;
    let early = 0.5 * 0.7f64.powf((played - 1.0).max(0.0));
    let lam = if prev.is_empty() {
        0.0
    } else {
        let base = 0.35 + 0.45 * tf(v.traits.anchor);
        base + (1.0 - base) * early
    };
    let mut sorted = fresh.to_vec();
    sorted.sort_by(|a, b| b.total_cmp(a));
    // A team new to his ballot has to push its way on.
    let mut s: Vec<f64> = fresh.iter().map(|f| f - if prev.is_empty() { 0.0 } else { 0.25 }).collect();
    // Voters move teams places on last week's ballot: a close road loss
    // to a good team costs about six or seven, a bad late loss ten or more,
    // a good win a place or two (`PLACES` a unit of `shift_since`).
    let last = sorted.len().saturating_sub(1) as f64;
    for (k, &i) in prev.iter().enumerate() {
        if i < s.len() && k < sorted.len() {
            let at = (k as f64 - PLACES * shift[i]).round().clamp(0.0, last) as usize;
            s[i] = lam * sorted[at] + (1.0 - lam) * fresh[i];
        }
    }
    let mut order: Vec<usize> = (0..s.len()).collect();
    order.sort_by(|&a, &b| s[b].total_cmp(&s[a]).then(a.cmp(&b)));
    order.truncate(32);
    head_to_head(ev, &mut order);
    order.truncate(25);
    order
}

/// Points 25 to 1 over the ballots: ties broken by first-place votes,
/// then by ballots listing the team, then by the earlier program id.
pub(crate) fn tally(n: usize, ballots: &[Vec<usize>]) -> (Vec<usize>, Vec<u32>, Vec<u16>, Vec<u32>) {
    let mut pts = vec![0u32; n];
    let mut firsts = vec![0u16; n];
    let mut listed = vec![0u16; n];
    for b in ballots {
        for (k, &i) in b.iter().enumerate() {
            pts[i] += 25 - k as u32;
            listed[i] += 1;
            if k == 0 {
                firsts[i] += 1;
            }
        }
    }
    let mut order: Vec<usize> = (0..n).filter(|&i| pts[i] > 0).collect();
    order.sort_by(|&a, &b| {
        pts[b].cmp(&pts[a]).then(firsts[b].cmp(&firsts[a])).then(listed[b].cmp(&listed[a])).then(a.cmp(&b))
    });
    order.truncate(25);
    let p = order.iter().map(|&i| pts[i]).collect();
    let f = order.iter().map(|&i| firsts[i]).collect();
    (order, p, f, pts)
}

/// Which family of releases a poll kind belongs to.
fn family(kind: PollKind) -> u8 {
    match kind {
        PollKind::ApPreseason | PollKind::Ap | PollKind::Final => 0,
        PollKind::CoachesPreseason | PollKind::Coaches | PollKind::CoachesFinal => 1,
        PollKind::Harris => 2,
        PollKind::Bcs => 3,
        PollKind::Cfp => 4,
    }
}

/// The last release of a family this season: its date and top 25.
fn last_release(w: &World, kind: PollKind) -> Option<(Date, Vec<usize>)> {
    let f = family(kind);
    w.college.as_ref()?.polls.iter().rev().find(|p| family(p.kind) == f).map(|p| (p.date, p.top.clone()))
}

fn final_poll(polls: &[super::college::Poll]) -> Option<&super::college::Poll> {
    polls.iter().rev().find(|p| p.kind == PollKind::Final)
}

/// Last season's final AP ranking, 1 for the champion down to 1/25 for
/// No. 25, as each member's preseason anchor.
fn last_final(w: &World, ev: &Ev) -> Vec<f64> {
    let mut v = vec![0.0; ev.t.len()];
    // Last season's polls are kept apart once the season turns (`prev_polls`).
    let fin = final_poll(&w.prev_polls).or_else(|| w.college_prev.as_ref().and_then(|c| final_poll(&c.polls)));
    if let Some(p) = fin {
        for (k, &t) in p.top.iter().enumerate() {
            if let Some(i) = ev.idx(t) {
                v[i] = (25 - k) as f64 / 25.0;
            }
        }
    } else {
        // No final poll on record (the world's first season): the
        // program's standing stands in.
        for (i, t) in ev.t.iter().enumerate() {
            v[i] = (w.programs[t.team].prestige - 0.5).max(0.0) * 1.6;
        }
    }
    v
}

/// The national champion and the title game's loser, once a title game
/// was played (the BCS and playoff eras).
fn title_pair(w: &World) -> Option<(usize, Option<usize>)> {
    if w.rules.cfb_postseason == CfbPostseason::PollsOnly {
        return None;
    }
    let cs = w.college.as_ref()?;
    let c = cs.champions[0]?;
    let r = cs
        .games
        .iter()
        .find(|g| g.kind == super::schedule::GameKind::Title && g.division == Division::FBS && g.involves(c))
        .and_then(|g| g.played.as_ref().map(|_| if g.home == c { g.away } else { g.home }));
    Some((c, r))
}

/// The champion first and the title game's loser second on a final
/// ballot (part D 25: voters rank them so; the Coaches Poll is bound to).
fn crown(ev: &Ev, b: &mut Vec<usize>, pair: Option<(usize, Option<usize>)>) {
    let Some((c, r)) = pair else { return };
    if let Some(i) = r.and_then(|r| ev.idx(r)) {
        b.retain(|&x| x != i);
        b.insert(0, i);
    }
    if let Some(i) = ev.idx(c) {
        b.retain(|&x| x != i);
        b.insert(0, i);
    }
    b.truncate(25);
}

fn week_key(w: &World, date: Date, salt: u64) -> u64 {
    (w.options.seed as u64) << 24 ^ (date.0 as u64).wrapping_mul(0x9E37_79B9) ^ salt
}

/// The conference each state's media follow most: its biggest FBS name's.
fn state_conf(ev: &Ev, w: &World) -> [Option<u16>; 51] {
    let mut best: [Option<(f64, u16)>; 51] = [None; 51];
    for t in &ev.t {
        let pr = w.programs[t.team].prestige;
        let s = t.state.min(50);
        if best[s].is_none_or(|(p, _)| pr > p) {
            best[s] = Some((pr, t.conf));
        }
    }
    best.map(|b| b.map(|x| x.1))
}

fn to_ids(ev: &Ev, b: &[usize]) -> Vec<u16> {
    b.iter().map(|&i| ev.t[i].team as u16).collect()
}

fn from_ids(ev: &Ev, b: &[u16]) -> Vec<usize> {
    b.iter().filter_map(|&p| ev.idx(p as usize)).collect()
}

fn release_of(ev: &Ev, ballots: &[Vec<usize>]) -> Release {
    let (order, points, firsts, _) = tally(ev.t.len(), ballots);
    Release { top: order.iter().map(|&i| ev.t[i].team).collect(), points, firsts }
}

/// An AP, Harris or (with `coaches`) Coaches Poll release: every voter's
/// ballot, kept as his latest, and the tally.
pub(crate) fn vote(w: &mut World, date: Date, kind: PollKind) -> Release {
    voters::ensure(w);
    let ev = evidence(w);
    if ev.t.is_empty() {
        return Release::default();
    }
    let pre = matches!(kind, PollKind::ApPreseason | PollKind::CoachesPreseason);
    let fam = family(kind);
    let (since, published) = match last_release(w, kind) {
        Some((d, top)) => (d, top),
        None => (Date(i32::MIN), Vec::new()),
    };
    let fallback: Vec<usize> = published.iter().filter_map(|&p| ev.idx(p)).collect();
    let shift = shift_since(w, &ev, since);
    let lf = if pre { last_final(w, &ev) } else { Vec::new() };
    let pair = if matches!(kind, PollKind::Final | PollKind::CoachesFinal) { title_pair(w) } else { None };
    let wk = week_key(w, date, fam as u64);
    let seed = w.options.seed;
    let sc = state_conf(&ev, w);
    // Who votes, as the ballot needs them, with each one's last ballot.
    let panel: Vec<(Voter, Vec<usize>)> = match fam {
        1 => w
            .voters
            .coaches
            .iter()
            .filter_map(|s| {
                let p = s.program as usize;
                let coach = w.programs.get(p)?.head_coach? as u32;
                let own = ev.idx(p);
                let v = Voter {
                    traits: voters::coach_traits(seed, coach),
                    era_homer: 1.0,
                    state: None,
                    conf: own.map(|i| ev.t[i].conf),
                    own,
                    key: (coach as u64) << 32 ^ 0xC0AC,
                };
                Some((v, from_ids(&ev, &s.ballot)))
            })
            .collect(),
        _ => {
            let list = if fam == 2 { &w.voters.harris } else { &w.voters.ap };
            let era_homer = if fam == 2 {
                0.7
            } else if w.year >= 2007 {
                0.5
            } else {
                1.0
            };
            list.iter()
                .map(|m| {
                    let s = m.state as usize;
                    let v = Voter {
                        traits: m.traits,
                        era_homer,
                        state: Some(s),
                        conf: sc.get(s).copied().flatten(),
                        own: None,
                        key: (m.id as u64) << 32 ^ fam as u64,
                    };
                    (v, from_ids(&ev, &m.ballot))
                })
                .collect()
        }
    };
    let ballots: Vec<Vec<usize>> = panel
        .iter()
        .map(|(v, prev)| {
            let mut b = if pre {
                let f = preseason(&ev, v, &lf, wk);
                ballot(&ev, v, &f, &[], &[])
            } else {
                let prev = if prev.is_empty() { &fallback } else { prev };
                let f = fresh(&ev, v, wk);
                ballot(&ev, v, &f, prev, &shift)
            };
            crown(&ev, &mut b, pair);
            b
        })
        .collect();
    // Keep each voter's latest ballot.
    match fam {
        1 => {
            let mut k = 0;
            for s in w.voters.coaches.iter_mut() {
                let p = s.program as usize;
                if w.programs.get(p).and_then(|p| p.head_coach).is_some() {
                    s.ballot = to_ids(&ev, &ballots[k]);
                    k += 1;
                } else {
                    s.ballot.clear();
                }
            }
        }
        _ => {
            let list = if fam == 2 { &mut w.voters.harris } else { &mut w.voters.ap };
            for (m, b) in list.iter_mut().zip(&ballots) {
                m.ballot = to_ids(&ev, b);
            }
        }
    }
    release_of(&ev, &ballots)
}

// --- The BCS standings (1998-2013) ----------------------------------------------------------

/// Each member's points in a panel's latest ballots.
fn panel_points(ev: &Ev, ballots: &[&Vec<u16>]) -> (Vec<f64>, usize) {
    let mut pts = vec![0.0; ev.t.len()];
    let mut n = 0;
    for b in ballots {
        if b.is_empty() {
            continue;
        }
        n += 1;
        for (k, &p) in b.iter().enumerate() {
            if let Some(i) = ev.idx(p as usize) {
                pts[i] += (25 - k) as f64;
            }
        }
    }
    (pts, n)
}

/// Ranks (1 = best) from scores, higher better.
fn ranks(v: &[f64]) -> Vec<usize> {
    let mut order: Vec<usize> = (0..v.len()).collect();
    order.sort_by(|&a, &b| v[b].total_cmp(&v[a]).then(a.cmp(&b)));
    let mut r = vec![0; v.len()];
    for (k, &i) in order.iter().enumerate() {
        r[i] = k + 1;
    }
    r
}

/// Poll ranks from points: teams with no votes share the next rank.
fn poll_ranks(pts: &[f64]) -> Vec<f64> {
    let r = ranks(pts);
    let voted = pts.iter().filter(|&&p| p > 0.0).count();
    pts.iter().zip(r).map(|(&p, k)| if p > 0.0 { k as f64 } else { (voted + 1) as f64 }).collect()
}

/// The six computer ratings the BCS averaged, rebuilt from their
/// published methods without margin of victory (barred from 2002): the
/// Colley Matrix, a Sagarin-style Elo-chess (wins-only Bradley-Terry),
/// Massey's wins-only least squares, Anderson & Hester's record and
/// schedule, Billingsley's game-by-game carry from last season, and
/// Wolfe's Bradley-Terry over every division's games. Higher is better.
fn computers(w: &World, ev: &Ev) -> [Vec<f64>; 6] {
    let n = ev.t.len();
    let fcs = n; // one node for every team below FBS
    let opp = |g: &GameEv| if g.opp == usize::MAX { fcs } else { g.opp };
    // Each node's games, the FCS node's from its opponents' side.
    let mut adj: Vec<Vec<(usize, bool)>> = vec![Vec::new(); n + 1];
    for (i, l) in ev.games.iter().enumerate() {
        for g in l {
            adj[i].push((opp(g), g.won));
            if g.opp == usize::MAX {
                adj[fcs].push((i, !g.won));
            }
        }
    }
    // Colley.
    let mut colley = vec![0.5; n + 1];
    for _ in 0..60 {
        for i in 0..=n {
            let (wn, ln) = adj[i].iter().fold((0.0, 0.0), |a, g| if g.1 { (a.0 + 1.0, a.1) } else { (a.0, a.1 + 1.0) });
            let sum: f64 = adj[i].iter().map(|g| colley[g.0]).sum();
            colley[i] = (1.0 + (wn - ln) / 2.0 + sum) / (2.0 + adj[i].len() as f64);
        }
    }
    // Bradley-Terry, wins only, with one tie against an average team.
    let bt = |adj: &Vec<Vec<(usize, bool)>>, nodes: usize| {
        let mut p = vec![1.0; nodes];
        for _ in 0..60 {
            let next: Vec<f64> = (0..nodes)
                .map(|i| {
                    let wins = adj[i].iter().filter(|g| g.1).count() as f64 + 0.5;
                    let den: f64 = adj[i].iter().map(|g| 1.0 / (p[i] + p[g.0])).sum::<f64>() + 1.0 / (p[i] + 1.0);
                    (wins / den).max(1e-6)
                })
                .collect();
            p = next;
        }
        p
    };
    let sagarin = bt(&adj, n + 1);
    // Massey, wins only (+1 a win, -1 a loss), with a ridge toward zero.
    let mut massey = vec![0.0; n + 1];
    for _ in 0..60 {
        let next: Vec<f64> = (0..=n)
            .map(|i| {
                let s: f64 = adj[i].iter().map(|g| massey[g.0] + if g.1 { 1.0 } else { -1.0 }).sum();
                s / (adj[i].len() as f64 + 1.0)
            })
            .collect();
        let m = next.iter().sum::<f64>() / (n + 1) as f64;
        massey = next.into_iter().map(|x| x - m).collect();
    }
    // Anderson & Hester: record against FBS teams and the schedule's
    // records two deep.
    let wp: Vec<f64> = (0..n)
        .map(|i| {
            let l: Vec<&(usize, bool)> = adj[i].iter().filter(|g| g.0 < n).collect();
            if l.is_empty() { 0.5 } else { l.iter().filter(|g| g.1).count() as f64 / l.len() as f64 }
        })
        .collect();
    let owp: Vec<f64> = (0..n)
        .map(|i| {
            let l: Vec<usize> = adj[i].iter().filter(|g| g.0 < n).map(|g| g.0).collect();
            if l.is_empty() { 0.5 } else { l.iter().map(|&o| wp[o]).sum::<f64>() / l.len() as f64 }
        })
        .collect();
    let anderson: Vec<f64> = (0..n)
        .map(|i| {
            let l: Vec<usize> = adj[i].iter().filter(|g| g.0 < n).map(|g| g.0).collect();
            let oowp = if l.is_empty() { 0.5 } else { l.iter().map(|&o| owp[o]).sum::<f64>() / l.len() as f64 };
            0.5 * wp[i] + 0.5 * (2.0 * owp[i] + oowp) / 3.0
        })
        .collect();
    // Billingsley: last season's finish carried in, then each game moves
    // rating from loser to winner, more for an upset.
    let lf = last_final(w, ev);
    let mut bill: Vec<f64> = lf.iter().map(|&x| 80.0 + 75.0 * x).collect();
    let mut seq: Vec<(Date, usize, usize, bool)> = Vec::new();
    for (i, l) in ev.games.iter().enumerate() {
        for g in l {
            if g.opp == usize::MAX || i < g.opp {
                seq.push((g.date, i, g.opp, g.won));
            }
        }
    }
    seq.sort_by_key(|x| (x.0, x.1));
    for (_, a, b, a_won) in seq {
        let ra = bill[a];
        let rb = if b == usize::MAX { 50.0 } else { bill[b] };
        let (wr, lr) = if a_won { (ra, rb) } else { (rb, ra) };
        let gain = 0.05 * lr * if wr < lr { 1.3 } else { 1.0 };
        let loss = 0.03 * wr;
        if a_won {
            bill[a] += gain;
            if b != usize::MAX {
                bill[b] -= loss;
            }
        } else {
            bill[a] -= loss;
            if b != usize::MAX {
                bill[b] += gain;
            }
        }
    }
    // Wolfe: every program a node, every division's games.
    let cs = w.college.as_ref().unwrap();
    let np = w.programs.len();
    let mut all: Vec<Vec<(usize, bool)>> = vec![Vec::new(); np];
    for g in cs.games.iter() {
        let Some(p) = g.played.filter(|p| !p.cancelled) else { continue };
        all[g.home].push((g.away, p.home_won));
        all[g.away].push((g.home, !p.home_won));
    }
    let wolfe_all = bt(&all, np);
    let wolfe: Vec<f64> = ev.t.iter().map(|t| wolfe_all[t.team]).collect();
    [colley[..n].to_vec(), sagarin[..n].to_vec(), massey[..n].to_vec(), anderson, bill, wolfe]
}

/// Strength of schedule as the 1998-2003 formula took it: two thirds the
/// opponents' record, one third their opponents'.
fn bcs_sos(ev: &Ev) -> Vec<f64> {
    let n = ev.t.len();
    let wp: Vec<f64> =
        ev.t.iter()
            .map(|t| if t.wins + t.losses == 0 { 0.5 } else { t.wins as f64 / (t.wins + t.losses) as f64 })
            .collect();
    let ow = |i: usize, f: &dyn Fn(usize) -> f64| {
        let l: Vec<usize> = ev.games[i].iter().filter(|g| g.opp != usize::MAX).map(|g| g.opp).collect();
        if l.is_empty() { 0.5 } else { l.iter().map(|&o| f(o)).sum::<f64>() / l.len() as f64 }
    };
    let owp: Vec<f64> = (0..n).map(|i| ow(i, &|o| wp[o])).collect();
    (0..n).map(|i| (2.0 * owp[i] + ow(i, &|o| owp[o])) / 3.0).collect()
}

/// The BCS standings by the season's formula: from 2004 the average of
/// the human polls' and the computers' shares (the AP poll through 2004,
/// the Harris poll after); before, poll and computer ranks plus schedule
/// rank, losses and (2001-2003) a quality-win deduction, lowest best.
pub(crate) fn bcs(w: &mut World) -> Release {
    voters::ensure(w);
    let ev = evidence(w);
    let n = ev.t.len();
    if n == 0 {
        return Release::default();
    }
    let year = w.year;
    let harris = year >= 2005 && !w.voters.harris.is_empty();
    let human: Vec<&Vec<u16>> = if harris {
        w.voters.harris.iter().map(|m| &m.ballot).collect()
    } else {
        w.voters.ap.iter().map(|m| &m.ballot).collect()
    };
    let (hp, hn) = panel_points(&ev, &human);
    let coach: Vec<&Vec<u16>> = w.voters.coaches.iter().map(|s| &s.ballot).collect();
    let (cp, cn) = panel_points(&ev, &coach);
    let comp = computers(w, &ev);
    let comp_ranks: Vec<Vec<usize>> = comp.iter().map(|c| ranks(c)).collect();
    let score: Vec<f64> = if year >= 2004 {
        (0..n)
            .map(|i| {
                let mut pts: Vec<f64> = comp_ranks.iter().map(|r| (26.0 - r[i] as f64).max(0.0)).collect();
                pts.sort_by(|a, b| a.total_cmp(b));
                let computer = pts[1..5].iter().sum::<f64>() / 100.0;
                let h = if hn > 0 { hp[i] / (25.0 * hn as f64) } else { 0.0 };
                let c = if cn > 0 { cp[i] / (25.0 * cn as f64) } else { 0.0 };
                (h + c + computer) / 3.0
            })
            .collect()
    } else {
        let hr = poll_ranks(&hp);
        let cr = poll_ranks(&cp);
        let sos = ranks(&bcs_sos(&ev));
        let base: Vec<f64> = (0..n)
            .map(|i| {
                let mut c: Vec<f64> = comp_ranks.iter().map(|r| r[i] as f64).collect();
                c.sort_by(|a, b| a.total_cmp(b));
                // From 1999 the worst computer ranking is dropped.
                let computer =
                    if year >= 1999 { c[..5].iter().sum::<f64>() / 5.0 } else { c.iter().sum::<f64>() / 6.0 };
                (hr[i] + cr[i]) / 2.0 + computer + sos[i] as f64 / 25.0 + ev.t[i].losses as f64
            })
            .collect();
        let quality = |pre: &[f64]| -> Vec<f64> {
            if year < 2001 {
                return pre.to_vec();
            }
            let r = ranks(&pre.iter().map(|x| -x).collect::<Vec<_>>());
            (0..n)
                .map(|i| {
                    let ded: f64 = ev.games[i]
                        .iter()
                        .filter(|g| g.won && g.opp != usize::MAX)
                        .map(|g| {
                            let k = r[g.opp] as f64;
                            if year == 2001 { (1.6 - 0.1 * k).max(0.0) } else { (1.1 - 0.1 * k).max(0.0) }
                        })
                        .sum();
                    pre[i] - ded
                })
                .collect()
        };
        quality(&base).into_iter().map(|x| -x).collect()
    };
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by(|&a, &b| score[b].total_cmp(&score[a]).then(a.cmp(&b)));
    order.truncate(25);
    Release {
        top: order.iter().map(|&i| ev.t[i].team).collect(),
        points: order.iter().map(|&i| (score[i].abs() * 10000.0).round() as u32).collect(),
        firsts: Vec::new(),
    }
}

// --- The playoff selection committee (2014 on) ----------------------------------------------

/// A member's read of a team by the protocol: the whole season's résumé
/// (every week alike), strength of schedule, game control and
/// championships won; no preseason view.
fn member_eval(ev: &Ev, tr: Traits, key: u64, i: usize) -> f64 {
    let t = &ev.t[i];
    let mut e = t.r0 * (0.8 + 0.4 * tf(tr.resume))
        + t.sos * 0.6 * tf(tr.resume)
        + (0.5 * t.eye + 0.5 * t.margin) * 0.6 * tf(tr.margin)
        + t.power * 0.4
        + t.brand * 0.1 * tf(tr.brand)
        + if t.champ { 0.8 } else { 0.0 };
    if t.g5 {
        e -= 0.2 * tf(tr.g5);
    }
    e + 0.25 * (1.2 - tf(tr.care)) * key_normal(key, i as u64)
}

/// Head-to-head and common opponents between two close teams, as the
/// protocol has the committee compare them.
fn compare(ev: &Ev, a: usize, b: usize) -> f64 {
    let mut adj = 0.0;
    if ev.beat(a, b) {
        adj += 0.6;
    } else if ev.beat(b, a) {
        adj -= 0.6;
    }
    let (mut diff, mut k) = (0.0, 0);
    for ga in ev.games[a].iter().filter(|g| g.opp != usize::MAX && g.opp != b) {
        if let Some(gb) = ev.games[b].iter().find(|g| g.opp == ga.opp) {
            diff += (ga.margin - gb.margin).clamp(-28, 28) as f64;
            k += 1;
        }
    }
    if k > 0 {
        adj += 0.25 * (diff / k as f64 / 21.0).clamp(-1.0, 1.0);
    }
    adj
}

/// The committee's ranking by its protocol: each member lists his best
/// 30 and teams on three lists stay under consideration; then, round by
/// round, members list the best six left, the six most listed are ranked
/// by each member, and the top three by total go onto the board while the
/// other three carry over, until 25 are placed. A member is out of the
/// room, and has no vote, for a school he is recused from.
pub(crate) fn committee(w: &mut World, date: Date) -> Release {
    voters::ensure(w);
    let ev = evidence(w);
    let n = ev.t.len();
    let members = &w.voters.committee;
    if n == 0 || members.is_empty() {
        return Release::default();
    }
    let wk = week_key(w, date, 0xCF9);
    let recused: Vec<Vec<bool>> =
        members.iter().map(|m| (0..n).map(|i| m.recused.contains(&(ev.t[i].team as u16))).collect()).collect();
    let evals: Vec<Vec<f64>> = members
        .iter()
        .enumerate()
        .map(|(k, m)| (0..n).map(|i| member_eval(&ev, m.traits, wk ^ (k as u64) << 40, i)).collect())
        .collect();
    // Each member's best `size` of a set (recusals left out).
    let best_of = |k: usize, set: &[usize], size: usize| -> Vec<usize> {
        let mut v: Vec<usize> = set.iter().copied().filter(|&i| !recused[k][i]).collect();
        v.sort_by(|&a, &b| evals[k][b].total_cmp(&evals[k][a]).then(a.cmp(&b)));
        v.truncate(size);
        v
    };
    let all: Vec<usize> = (0..n).collect();
    let mut listed = vec![0u32; n];
    for k in 0..members.len() {
        for i in best_of(k, &all, 30) {
            listed[i] += 1;
        }
    }
    // Teams listed by three or more members stay under consideration; a
    // team most members are recused from needs the same share of the
    // members free to list it, so the absent votes don't count against it.
    let free = |i: usize| recused.iter().filter(|r| !r[i]).count();
    let need = |i: usize| (3 * free(i)).div_ceil(members.len()).max(1) as u32;
    let mut left: Vec<usize> = (0..n).filter(|&i| listed[i] >= need(i)).collect();
    let mean_eval = |i: usize| evals.iter().map(|e| e[i]).sum::<f64>() / evals.len() as f64;
    let mut placed: Vec<usize> = Vec::new();
    let mut carry: Vec<usize> = Vec::new();
    while placed.len() < 25 && !(left.is_empty() && carry.is_empty()) {
        let need = 6usize.saturating_sub(carry.len()).min(left.len());
        let mut votes = vec![0u32; n];
        for k in 0..members.len() {
            for i in best_of(k, &left, 6) {
                votes[i] += 1;
            }
        }
        // Votes count as a share of the members free to vote for a team.
        let share = |i: usize| votes[i] as f64 / free(i).max(1) as f64;
        let mut pick = left.clone();
        pick.sort_by(|&a, &b| {
            share(b).total_cmp(&share(a)).then(mean_eval(b).total_cmp(&mean_eval(a))).then(a.cmp(&b))
        });
        pick.truncate(need);
        left.retain(|i| !pick.contains(i));
        let pool: Vec<usize> = carry.iter().copied().chain(pick).collect();
        // Each member ranks the pool; a close pair is settled by
        // head-to-head and common opponents.
        let mut total = vec![0.0; n];
        let mut voters = vec![0u32; n];
        for k in 0..members.len() {
            let mine: Vec<usize> = pool.iter().copied().filter(|&i| !recused[k][i]).collect();
            let score = |a: usize| -> f64 {
                evals[k][a]
                    + mine
                        .iter()
                        .filter(|&&b| b != a)
                        .map(|&b| compare(&ev, a, b) * (-(evals[k][a] - evals[k][b]).abs()).exp())
                        .sum::<f64>()
            };
            let mut order = mine.clone();
            let s: Vec<(usize, f64)> = order.iter().map(|&i| (i, score(i))).collect();
            order.sort_by(|&a, &b| {
                let sa = s.iter().find(|x| x.0 == a).unwrap().1;
                let sb = s.iter().find(|x| x.0 == b).unwrap().1;
                sb.total_cmp(&sa).then(a.cmp(&b))
            });
            for (r, &i) in order.iter().enumerate() {
                total[i] += (r + 1) as f64;
                voters[i] += 1;
            }
        }
        let mut ranked = pool.clone();
        let avg = |i: usize| if voters[i] == 0 { f64::MAX } else { total[i] / voters[i] as f64 };
        ranked.sort_by(|&a, &b| avg(a).total_cmp(&avg(b)).then(a.cmp(&b)));
        let take = 3.min(ranked.len()).min(25 - placed.len());
        placed.extend(&ranked[..take]);
        carry = ranked[take..].to_vec();
        if pool.is_empty() {
            break;
        }
    }
    // A small world can run out of teams under consideration: the rest
    // follow by the members' average read.
    if placed.len() < 25 {
        let mut rest: Vec<usize> = (0..n).filter(|i| !placed.contains(i)).collect();
        rest.sort_by(|&a, &b| mean_eval(b).total_cmp(&mean_eval(a)).then(a.cmp(&b)));
        placed.extend(rest.into_iter().take(25 - placed.len()));
    }
    Release { top: placed.iter().map(|&i| ev.t[i].team).collect(), points: Vec::new(), firsts: Vec::new() }
}
