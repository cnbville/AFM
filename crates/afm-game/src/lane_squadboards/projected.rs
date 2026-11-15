//! Projected test numbers (Cain 2026-11-14, yes to items 1-6 and 8): what
//! he'd measure and run at a coming testing day, from the read that judges
//! him (`reads::Seen::judging`: yours from your first own look, your
//! staff's before).
//!
//! The numbers are the combine's own drills on a projected body: his six
//! physical ratings and his weight and height walked forward, then run
//! through `Drills::raw`, so a projected forty is his projected speed,
//! acceleration and weight and can't drift away from them.
//!
//! - Each physical rating steps the way the road steps the overall: a drift
//!   plus a share of the gap between the ceiling read and the road's own
//!   middle that season (`road::mids`), fitted per rating by age and level
//!   on one year of the world's own development (`physical::physical_fit`).
//!   A part year (a combine in March) takes that share of the year's step,
//!   since a year's growth lands through it.
//! - Weight: his group's fill curve and the weight room's pull toward his
//!   position's weight (for the ceiling read's talent), as the world grows
//!   bodies; for your own players a body plan's target, with what each pound
//!   does to his ratings. Height on the median boy's curve. What he hides
//!   (his own fill offset, his bloom) widens the band instead of moving it.
//! - Anchored on what he actually ran or measured: the gap between his real
//!   test and what his read stands for is carried forward, shrunk by how
//!   much of it a test day's noise could explain and by the growth since an
//!   old test.
//! - The band (25th to 75th) draws the read's doubt, the ceiling read's,
//!   whole years of growth as the world's players had them, the hidden body
//!   and the target day's own noise.
//!
//! Display only: nothing here moves him or feeds a world step.

use std::sync::{Arc, Mutex};

use afm_engine::body::{self, DAY_SD, Drills, TRACK_SD};
use afm_engine::calendar::Date;
use afm_engine::development::{weight_room_pace, weight_room_room};
use afm_engine::positions::Position;
use afm_engine::rng::{Rng, clamp};
use afm_engine::world::scouting::Kind;
use afm_engine::world::scouting::physical::{self, Cell, PHYSICAL, PhysFit};
use afm_engine::world::{Stage, World};

use super::body::Known;
use crate::reads::{self, Judging};
use crate::road;
use crate::views;

/// What kind of day a known number came from.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Src {
    /// Your own staff measured him.
    Measured,
    /// An NFL team's listing (his real size).
    TeamListed,
    /// A school's listing (runs tall and heavy, `file::listed`).
    Listed,
    /// Your strength staff's testing day (`training::seat::test_row`).
    Testing,
    /// The combine or his pro day as the world records it.
    Combine,
    /// Laser gates at a camp or showcase (`world::times`).
    Laser,
    /// Your high school's hand-timed forty.
    Hand,
    /// A day your scouts saw (`file::tests`).
    Seen(Kind),
}

/// The drills in `Drills::raw`'s order, as rows of the Body tab.
pub(crate) const DRILL_ROW: [usize; 6] = [5, 7, 8, 11, 9, 10];

/// A coming testing day.
#[derive(Clone, Debug)]
pub(crate) struct Target {
    pub key: &'static str,
    pub label: &'static str,
    pub when: Date,
    /// Years from today.
    pub years: f64,
    /// Whether it is the combine (his last college development year, and
    /// a pro day forty for a declared man the combine didn't invite).
    pub combine: bool,
    pub note: String,
}

/// The coming days that apply to him: Senior year and At the combine for
/// a high schooler, At the combine in college, Next year in the pros.
pub(crate) fn targets(w: &World, i: usize) -> Vec<Target> {
    let Some(p) = w.people.get(i) else { return Vec::new() };
    let today = w.today;
    let (_, cm, cd) = w.dates.combine_start.parts();
    let years = |d: Date| (today.days_to(d).max(0) as f64 / 365.25 * 100.0).round() / 100.0;
    let combine_in = |y: i32| Date::ymd(y, cm, cd);
    // The next combine a man who hasn't declared can still go to: the
    // coming spring's until the declaration deadline in mid-January, the
    // spring after from then on.
    let next_combine = if today < Date::ymd(today.year(), 1, 15) { today.year() } else { today.year() + 1 };
    let combine = |earliest: i32| {
        let y = earliest.max(next_combine);
        let when = combine_in(y);
        Target {
            key: "combine",
            label: "At the combine",
            when,
            years: years(when),
            combine: true,
            note: format!("the {y} combine, the first he's eligible for"),
        }
    };
    let mut out = Vec::new();
    match p.stage {
        Stage::HighSchool | Stage::PrepSchool => {
            // His last high school testing day: the spring of his senior year.
            let when = Date::ymd(p.class_of, 4, 15);
            if when > today {
                out.push(Target {
                    key: "senior",
                    label: "Senior year",
                    when,
                    years: years(when),
                    combine: false,
                    note: format!("his last high school testing day, spring {}", p.class_of),
                });
            }
            out.push(combine(p.class_of + 3));
        }
        Stage::College => out.push(combine(p.class_of + 3)),
        Stage::Declared => {
            // This spring's combine: today once it has been run.
            let this = combine_in(today.year());
            let done = today > this.add(7);
            let when = if done { today } else { this };
            out.push(Target {
                key: "combine",
                label: "At the combine",
                when,
                years: years(when),
                combine: true,
                note: if done {
                    "this spring's combine and pro days".into()
                } else {
                    format!("the {} combine", when.year())
                },
            });
        }
        Stage::Pro if p.pro.as_ref().is_some_and(|c| c.team.is_some()) => {
            let when = today.add(365);
            out.push(Target {
                key: "next",
                label: "Next year",
                when,
                years: 1.0,
                combine: false,
                note: "next offseason's testing".into(),
            });
        }
        _ => {}
    }
    out
}

/// A projection: each Body tab row's middle and 25-75 band.
#[derive(Clone, Debug)]
pub(crate) struct Proj {
    pub vals: [Option<f64>; 12],
    pub band: [Option<(f64, f64)>; 12],
    /// Rows whose number leans on a real test of his.
    pub anchored: [bool; 12],
    /// His weight's middle at each coming season start and on the day.
    pub weights: Vec<(Date, f64)>,
    pub whose: &'static str,
}

/// The world's fit of a year's physical growth, kept for the season.
pub(crate) fn fit(w: &World) -> Arc<PhysFit> {
    type Cached = ((u32, i32), Arc<PhysFit>);
    static CACHE: Mutex<Option<Cached>> = Mutex::new(None);
    let key = (w.options.seed, w.year);
    let mut g = CACHE.lock().unwrap_or_else(|e| e.into_inner());
    if let Some((k, v)) = g.as_ref()
        && *k == key
    {
        return v.clone();
    }
    let v = Arc::new(physical::physical_fit(w));
    *g = Some((key, v.clone()));
    v
}

/// One draw of everything the projection doesn't know.
#[derive(Clone, Copy)]
struct Draw {
    ceiling: f64,
    /// His fill offset and bloom.
    offset: f64,
    bloom: f64,
}

/// One year of growth between today and the day: the age and level its
/// checkpoint fitted, the share of it that lands by the day, and whether
/// its checkpoint (the weight room's step) falls before the day.
#[derive(Clone, Copy, Debug)]
struct Year {
    age: u32,
    tier: u8,
    frac: f64,
    checkpoint: bool,
}

/// What the walk needs from today.
struct Start {
    pos: Position,
    age: f64,
    ratings: [f64; 6],
    weight: f64,
    height: f64,
    arm: f64,
    /// Your body plan's target for your own player.
    plan: Option<f64>,
    /// The road's coming seasons, for its middle.
    seasons: Vec<(i32, u32, u8)>,
    years: Vec<Year>,
}

/// The years of growth between today and a day. A year is decided at its
/// checkpoint (fall camp) and lands through the twelve months after it
/// (`development.md`): first what is left of the year under way (his last
/// checkpoint's age and his level now), then each coming camp's year (a
/// year older each time, at the level he'd be in that season, from
/// `road::seasons`), each by the share of it the day reaches.
fn years_to(w: &World, p: &afm_engine::world::Person, t: &Target) -> (Vec<(i32, u32, u8)>, Vec<Year>) {
    let (_, fm, fd) = w.dates.fall_camp.parts();
    let camp = |y: i32| Date::ymd(y, fm, fd);
    let first = if w.today < camp(w.year) { w.year } else { w.year + 1 };
    let last = t.when.year() + 1;
    let seasons = road::seasons(w, p, (last - w.year).max(0));
    let tier_now = super::tier_of(views::level_of(w, p));
    let cap = |tier: u8| match t.key {
        "senior" => 0,
        "combine" => tier.min(1),
        _ => tier,
    };
    let span = |a: Date, b: Date| clamp(a.days_to(b) as f64 / 365.0, 0.0, 1.0);
    let age = p.player.age.saturating_sub(1);
    let mut out =
        vec![Year { age, tier: cap(tier_now), frac: span(w.today, t.when.min(camp(first))), checkpoint: false }];
    for (m, y) in (first..last).enumerate() {
        let c = camp(y);
        if c >= t.when {
            break;
        }
        let tier =
            if y <= w.year { tier_now } else { seasons.get((y - w.year - 1) as usize).map_or(tier_now, |s| s.2) };
        out.push(Year { age: age + 1 + m as u32, tier: cap(tier), frac: span(c, t.when), checkpoint: true });
    }
    (seasons, out)
}

/// The body and ratings at the target for one draw (no rng: the middle).
fn walk(
    w: &World,
    st: &Start,
    j: &Judging,
    fit: &PhysFit,
    years: f64,
    d: Draw,
    rng: Option<&mut Rng>,
) -> ([f64; 6], f64, f64, f64, Vec<f64>) {
    let mut rng = rng;
    let mids = road::mids(w, j.now.0, d.ceiling, &st.seasons);
    let mut r = st.ratings;
    let fill0 = body::fill_with(st.pos, st.age, d.offset);
    let frame = st.weight / fill0;
    let mut adult = frame;
    let mut path = Vec::with_capacity(st.years.len());
    for (k, y) in st.years.iter().enumerate() {
        // The gap to the ceiling read from the road's middle at the year's
        // checkpoint (today's read for the year under way).
        let mid =
            if k == 0 { j.now.0 } else { mids.get(k - 1).copied().unwrap_or_else(|| *mids.last().unwrap_or(&j.now.0)) };
        let gap = (d.ceiling - mid).max(0.0);
        if let Some(c) = fit.cell(y.age, y.tier, st.pos) {
            let res = match rng.as_deref_mut() {
                Some(g) if !c.resid.is_empty() => {
                    let x = c.resid[(g.next_u32() as usize) % c.resid.len()];
                    x.map(f64::from)
                }
                _ => [0.0; 6],
            };
            for (n, v) in r.iter_mut().enumerate() {
                let [drift, share] = c.step[n];
                *v = clamp(*v + y.frac * (drift + share * gap + res[n]), 1.0, 99.0);
            }
        }
        // The weight room's step at a checkpoint before the day
        // (`develop_season`), at the age he turns that year.
        if y.checkpoint && (15..=25).contains(&y.age) && !st.pos.is_specialist() {
            let room = weight_room_room(st.pos, d.ceiling, frame);
            let pace = weight_room_pace(y.tier == 0, 10.0, 0.0);
            adult += (room - adult).max(0.0) * clamp(pace, 0.0, 1.0);
        }
        if k > 0 {
            path.push(adult * body::fill_with(st.pos, (st.age + k as f64).min(st.age + years), d.offset));
        }
    }
    let age_t = st.age + years;
    let mut weight = adult * body::fill_with(st.pos, age_t, d.offset);
    // Your body plan: 1.5 lb a week up, 1.2 down, every week of the year
    // (`training::seat::week`), and what each pound does to his ratings
    // against the weight he'd have reached without it (`body_change`).
    if let Some(t) = st.plan {
        let reach = |yrs: f64| {
            let most = if t > st.weight { 78.0 } else { 62.4 } * yrs;
            st.weight + (t - st.weight).clamp(-most, most)
        };
        let planned = reach(years);
        let lbs = planned - weight;
        for (n, per) in [(0usize, -0.03), (1, -0.02), (2, -0.03), (5, 0.05)] {
            r[n] = clamp(r[n] + per * lbs, 1.0, 99.0);
        }
        weight = planned;
        for (k, v) in path.iter_mut().enumerate() {
            *v = reach((k as f64 + 1.0).min(years));
        }
    }
    let grown = body::height_share(age_t, d.bloom) / body::height_share(st.age, d.bloom).max(0.5);
    let height = st.height * grown;
    let arm = st.arm * grown;
    (r, weight, height, arm, path)
}

fn drills(st: &Start, r: [f64; 6], weight: f64, arm: f64) -> [f64; 6] {
    Drills {
        speed: r[0],
        accel: r[1],
        agility: r[2],
        jumping: r[3],
        flexibility: r[4],
        strength: r[5],
        weight,
        arm,
        pos: st.pos,
    }
    .raw()
}

/// How far each drill moves on a point of each rating (`Drills::raw`'s
/// order by `PHYSICAL`'s).
fn slopes(st: &Start, r: [f64; 6], weight: f64, arm: f64) -> [[f64; 6]; 6] {
    let base = drills(st, r, weight, arm);
    let mut out = [[0.0; 6]; 6];
    for j in 0..6 {
        let mut up = r;
        up[j] += 1.0;
        let d = drills(st, up, weight, arm);
        for k in 0..6 {
            out[k][j] = d[k] - base[k];
        }
    }
    out
}

/// A drill's lasting knack as a sd (the track for the forty, the 3-cone
/// and shuttle knacks), 0 for the rest.
fn knack(pos: Position, k: usize) -> f64 {
    let (cone, shuttle) = body::knack_sd(pos);
    match k {
        0 => TRACK_SD,
        4 => cone,
        5 => shuttle,
        _ => 0.0,
    }
}

/// For a test from a kind of day: the forty's shift from the combine's,
/// the day's own sd, and whether the day carries his knack.
fn day_of(src: Src, k: usize) -> Option<(f64, f64, bool)> {
    let testing = [0.03, 0.8, 3.0, 1.5, 0.08, 0.04];
    match src {
        Src::Combine => Some((0.0, DAY_SD[k], true)),
        Src::Seen(kind) => {
            let shift = match (kind, k) {
                (Kind::ProDay, 0) => -0.05,
                (Kind::Camp, 0) => 0.04,
                _ => 0.0,
            };
            Some((shift, DAY_SD[k], true))
        }
        // A testing day reads his shuttle knack and none other.
        Src::Testing => Some((0.0, testing[k], k == 5)),
        Src::Laser if k == 0 => Some((0.0, 0.05, false)),
        _ => None,
    }
}

/// The share of a pound a listing adds, and of an inch (`file::listed`:
/// 3 to 9 lb and a quarter to an inch and a quarter heavy and tall).
const LISTED_EXTRA: (f64, f64) = (6.0, 0.75);
/// Draws behind the band.
const DRAWS: usize = 400;

/// His projection for a coming day, or why there is none.
pub(crate) fn project(w: &World, i: usize, k: &Known, t: &Target, arm_ratio: f64) -> Result<Proj, String> {
    let p = w.people.get(i).ok_or("That player is no longer in football.")?;
    let j = reads::judging(w, i).ok_or("Nobody of yours has read him yet, so there is nothing to project from.")?;
    let seen = reads::seen(w, i);
    let mut ratings = [0.0; 6];
    let mut sds = [0.0; 6];
    for (n, a) in PHYSICAL.iter().enumerate() {
        let mine = if j.yours { seen.mine_attrs.get(a.index()).copied().flatten() } else { None };
        let (lo, hi) = mine
            .or_else(|| seen.attrs.get(a.index()).copied().flatten())
            .ok_or("Your staff hasn't read all six of his athletic ratings yet.")?;
        ratings[n] = (lo as f64 + hi as f64) / 2.0;
        sds[n] = ((hi as f64 - lo as f64 - 1.0).max(0.6)) / 2.0;
    }
    let listed = |r: usize| matches!(k.src[r], Some((Src::Listed, _)));
    let weight = k.vals[1].ok_or("No weight is known for him.")? - if listed(1) { LISTED_EXTRA.0 } else { 0.0 };
    let height = k.vals[0].ok_or("No height is known for him.")? - if listed(0) { LISTED_EXTRA.1 } else { 0.0 };
    let arm = k.vals[2].unwrap_or(height * arm_ratio);
    let own = views::team_of(w, p).is_some_and(|tm| views::is_mine(w, tm));
    let plan = own.then(|| w.training.plan_of(p.player.id).body.map(|b| b.0 as f64)).flatten();
    let (seasons, years) = years_to(w, p, t);
    let st = Start {
        pos: p.player.position,
        age: p.player.age as f64 + f64::from(p.player.grow),
        ratings,
        weight,
        height,
        arm,
        plan,
        seasons,
        years,
    };
    let fit = fit(w);
    let mid_draw = Draw { ceiling: j.ceiling.0, offset: 0.0, bloom: 0.0 };
    let (r_t, w_t, h_t, arm_t, path) = walk(w, &st, &j, &fit, t.years, mid_draw, None);
    let mut mid = drills(&st, r_t, w_t, arm_t);
    if t.combine && p.stage == Stage::Declared && !p.cal.combine_invite && w.today >= w.dates.combine_invites {
        // Not invited: his forty is a pro day's.
        mid[0] -= 0.05;
    }
    // The read's doubt in each drill's units, and a year's growth's.
    let slope = slopes(&st, ratings, weight, arm);
    let v_read: [f64; 6] = std::array::from_fn(|d| (0..6).map(|n| (slope[d][n] * sds[n]).powi(2)).sum());
    let year_cell: Option<&Cell> = st.years.first().and_then(|y| fit.cell(y.age, y.tier, st.pos));
    let v_year: [f64; 6] = std::array::from_fn(|d| {
        year_cell.map_or(0.0, |c| {
            let n = c.resid.len().max(1) as f64;
            c.resid.iter().map(|x| (0..6).map(|q| slope[d][q] * f64::from(x[q])).sum::<f64>().powi(2)).sum::<f64>() / n
        })
    });
    // Anchor on his real tests.
    let mut offset = [0.0; 6];
    let mut static_var = [0.0; 6];
    let mut anchored = [false; 12];
    for d in 0..6 {
        let row = DRILL_ROW[d];
        let kn = knack(st.pos, d);
        let target_day = DAY_SD[d].powi(2);
        static_var[d] = v_read[d] + kn * kn + target_day;
        let (Some(v), Some((src, when))) = (k.vals[row], k.src[row]) else { continue };
        let Some((shift, day_sd, carries)) = day_of(src, d) else { continue };
        let since = when.map_or(0.0, |x| (x.days_to(w.today).max(0) as f64 / 365.25).min(6.0));
        // What his read stands for on that day: back along the fit by the
        // years since, at the weight he'd have had then.
        let then = if since > 0.05 {
            let mut r = ratings;
            let mut left = since;
            let (mut age, tier) = st.years.first().map_or((p.player.age, 0), |y| (y.age + 1, y.tier));
            while left > 0.0 && age > 14 {
                age -= 1;
                let f = left.min(1.0);
                if let Some(c) = fit.cell(age, tier, st.pos) {
                    let gap = (j.ceiling.0 - j.now.0).max(0.0);
                    for (n, x) in r.iter_mut().enumerate() {
                        *x -= f * (c.step[n][0] + c.step[n][1] * gap);
                    }
                }
                left -= f;
            }
            let wt = weight * body::fill_with(st.pos, st.age - since, 0.0) / body::fill_with(st.pos, st.age, 0.0);
            drills(&st, r, wt, arm)[d]
        } else {
            drills(&st, ratings, weight, arm)[d]
        };
        let gap = v - shift - then;
        let persist = v_read[d] + if carries { kn * kn } else { 0.0 };
        let share = persist / (persist + day_sd * day_sd + since * v_year[d]).max(1e-9);
        offset[d] = share * gap;
        static_var[d] = persist * (1.0 - share) + if carries { 0.0 } else { kn * kn } + target_day;
        anchored[row] = true;
    }
    for d in 0..6 {
        mid[d] += offset[d];
    }
    // The band: draws of the ceiling, whole years of growth, the hidden
    // body and the day.
    let mut rng = Rng::new(p.player.id ^ 0x0B0D_7E57 ^ (t.when.0 as u32).wrapping_mul(0x9E37_79B9));
    let mut cols: Vec<Vec<f64>> = (0..12).map(|_| Vec::with_capacity(DRAWS)).collect();
    for _ in 0..DRAWS {
        let ceiling = clamp(j.ceiling.0 + j.ceiling.1 * rng.normal(0.0, 1.0), 1.0, 99.0).max(j.now.0);
        let bloom = clamp(rng.normal(0.0, 1.1), -2.5, 3.0);
        let own = body::fill_own_draw(rng.next(), rng.normal(0.0, 1.0));
        let d = Draw { ceiling, offset: body::fill_offset_of(st.pos, bloom, own), bloom };
        let (r, wt, ht, a, _) = walk(w, &st, &j, &fit, t.years, d, Some(&mut rng));
        let dr = drills(&st, r, wt, a);
        for n in 0..6 {
            cols[DRILL_ROW[n]].push(dr[n] + static_var[n].sqrt() * rng.normal(0.0, 1.0));
        }
        cols[0].push(ht);
        cols[1].push(wt);
        cols[2].push(a);
    }
    let mut vals = [None; 12];
    let mut band = [None; 12];
    let mids_body = [(0usize, h_t), (1, w_t), (2, arm_t)];
    for (row, m) in mids_body {
        if row == 2 && k.vals[2].is_none() {
            continue;
        }
        vals[row] = Some(m);
    }
    let shown = body::rounded(mid);
    let show = |c: body::Combine| [c.forty, c.vertical, c.broad, c.bench, c.cone, c.shuttle];
    for (n, &row) in DRILL_ROW.iter().enumerate() {
        vals[row] = Some(show(shown)[n]);
    }
    for row in 0..12 {
        let (Some(m), false) = (vals[row], cols[row].is_empty()) else { continue };
        let c = &mut cols[row];
        let q50 = super::quantile(c, 0.5);
        let (q25, q75) = (super::quantile(c, 0.25), super::quantile(c, 0.75));
        let raw_mid = match DRILL_ROW.iter().position(|&x| x == row) {
            Some(n) => mid[n],
            None => m,
        };
        band[row] = Some((raw_mid + q25 - q50, raw_mid + q75 - q50));
    }
    // Round the drills' bands the way a test day writes them.
    let lo: [f64; 6] = std::array::from_fn(|n| band[DRILL_ROW[n]].map_or(mid[n], |b| b.0));
    let hi: [f64; 6] = std::array::from_fn(|n| band[DRILL_ROW[n]].map_or(mid[n], |b| b.1));
    let (lo, hi) = (show(body::rounded(lo)), show(body::rounded(hi)));
    for (n, &row) in DRILL_ROW.iter().enumerate() {
        band[row] = Some((lo[n].min(hi[n]), lo[n].max(hi[n])));
    }
    let mut weights: Vec<(Date, f64)> = Vec::new();
    for (k2, wt) in path.iter().enumerate() {
        let d = w.today.add(((k2 as f64 + 1.0) * 365.25).round() as i32);
        if d.days_to(t.when) > 60 {
            weights.push((d, *wt));
        }
    }
    weights.push((t.when, w_t));
    Ok(Proj { vals, band, anchored, weights, whose: j.whose() })
}
