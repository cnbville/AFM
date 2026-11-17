//! The scheme-fit check (`docs/design/scheme-fit.md`, lane SF0): does a
//! player's skill pay inside his team's scheme? This is the audit's own tool
//! (`afm/scheme-fit/full-pass.md` and `tools/` in the project folder),
//! committed so every scheme-fit lane measures the same way. Measurement
//! only: the game changes nothing.
//!
//! Team 0 of a 32-team league (the synthetic league, or a world's NFL clubs
//! with `AFM_AB_WORLD`) is set to the level's typical team: every starter at
//! the typical starter's ratings and body, backups at the typical player's,
//! habits at the middle, no flaws or tools, the same knowledge, and a depth
//! order that names every job. Each arm changes one thing on team 0: its
//! scheme (the context), one man's ratings or body, or a man the generator
//! draws in a body band at his overall ("as recruited", four drawn men a
//! band). Every arm replays the same games on the same seeds, so two arms
//! differ only in their change. An arm writes the block sums (40 blocks) of
//! every team slot and every watched man's slots; `read` turns them into the
//! report's tables.

pub mod read;

use std::collections::HashMap;
use std::io::Write as _;
use std::path::Path;
use std::time::Instant;

use afm_engine::attributes::Attr;
use afm_engine::game::guide::{Firm, Guidelines, Want};
use afm_engine::game::playbook::Personnel;
use afm_engine::game::roster::{Front, GameTeam, OffScheme, Shell};
use afm_engine::game::stats::PlayType;
use afm_engine::game::synthetic::{typical, typical_body, typical_starter};
use afm_engine::game::{Conditions, Venue, quick_game_in};
use afm_engine::player::Level;
use afm_engine::positions::{Position, overall_for};
use afm_engine::spots::{self, Spot};
use rayon::prelude::*;
use serde_json::{Value, json};

use crate::ab::{Knob, Part, Squad, change_one, synthetic, world_nfl};

macro_rules! slots {
    ($en:ident, $names:ident, $($n:ident),* $(,)?) => {
        #[allow(non_camel_case_types, dead_code, clippy::upper_case_acronyms)]
        #[derive(Clone, Copy)]
        pub enum $en { $($n),* , COUNT }
        pub const $names: &[&str] = &[$(stringify!($n)),*];
    };
}

// Team 0's slots. The audit's first, in its order; the scheme-fit items'
// own measures after them (each lane's targets, `docs/design/scheme-fit.md`).
slots!(
    T,
    T_NAMES, // offense
    o_snaps,
    o_db,
    o_att,
    o_cmp,
    o_pyds,
    o_air,
    o_int,
    o_sack,
    o_scr,
    o_scr_yds,
    o_run,
    o_ryds,
    o_rsuc,
    o_r10,
    o_p20,
    o_epa,
    o_db_epa,
    o_run_epa,
    o_prs,
    o_ttt,
    o_pa,
    o_3att,
    o_3conv,
    o_td,
    o_to,
    o_sep,
    o_drop,
    o_yac,
    o_p11,
    o_p12,
    o_p13,
    o_p21,
    o_p22,
    o_p10,
    o_poth,
    o_d11,
    o_d12,
    o_d13,
    o_d21,
    o_d22,
    o_d10,
    o_tgt_wr,
    o_tgt_te,
    o_tgt_rb,
    o_yds_wr,
    o_yds_te,
    o_yds_rb, // defense
    d_snaps,
    d_db,
    d_att,
    d_cmp,
    d_pyds,
    d_air,
    d_int,
    d_sack,
    d_prs,
    d_run,
    d_ryds,
    d_rsuc,
    d_r10,
    d_p20,
    d_epa,
    d_db_epa,
    d_run_epa,
    d_blitz,
    d_man,
    d_3att,
    d_3conv,
    d_td,
    d_to,
    d_deep_att,
    d_deep_cmp,
    d_ttt,
    d_box,
    d_base,
    d_nickel,
    d_dime,
    d_int_s,
    d_int_cb,
    d_int_lb,
    d_int_dl,
    d_pbu_s,
    d_pbu_cb,
    d_pbu_lb,
    d_pbu_dl,
    d_sack_dl,
    d_sack_lb,
    d_sack_db, // game
    g_pf,
    g_pa,
    g_win, // play action and the boot (item 3)
    o_pa_epa,
    o_pa_sack,
    o_boot,
    o_boot_sack,
    o_boot_epa, // targets in each personnel group (items 1 and 2)
    o_tg11_wr,
    o_tg11_te,
    o_tg11_rb,
    o_tg12_wr,
    o_tg12_te,
    o_tg12_rb,
    o_tg13_wr,
    o_tg13_te,
    o_tg13_rb,
    o_tg21_wr,
    o_tg21_te,
    o_tg21_rb, // the catch against man (item 6)
    o_att_man,
    o_sep_man,
    o_cmp_man,
    o_att_zone,
    o_sep_zone, // the first tackle on runs (item 7)
    o_rtk,
    o_rtk_blk,
    d_rtk,
    d_rtk_blk, // 3rd and 4th down, 2 or less to go (item 8)
    o_sy_rb,
    o_sy_rb_cv,
    o_sy_qb,
    o_sy_qb_cv,
    d_sy_rb,
    d_sy_rb_cv,
    d_sy_qb,
    d_sy_qb_cv, // backs' long runs (item 9)
    o_rb_car,
    o_rb_r10,
    o_rb_r20,
    d_rb_car,
    d_rb_r10,
    d_rb_r20, // picks and breakups (items 14 to 17)
    o_att30,
    o_int30,
    d_tgt,
    d_att30,
    d_int30,
    d_att_man,
    d_int_man,
    d_int_s_man,
    d_int_dl_blk,
    d_pbu_tgt,
);

// Each watched man's slots: the audit's first, then the items' own.
slots!(
    P, P_NAMES, // on the field
    on, on3, on_db, on_run, on_run_yds, on_run_suc, on_db_epa, on_run_epa, on_sack, on_prs, // receiving
    tgt, rec, ryd, air, yac, sep, drop, tgt20, rec20, tgt_epa, rtd, r1d, rmt, // rushing
    car, cyds, csuc, c10, cbc, cmt, cfum, cepa, ctd, cneg, // passing
    pdb, patt, pcmp, pyds, pint, psack, pepa, pair, pscr, pscr_yds, // blocking
    pbr, pbw, pba, sca, rbr, rbw, rbs, // defense
    tkl, rtkl, stop, tfl, mtk, prs, sck, hit, rwin, rrep, shed, rbf, pbf, pbl, cov_t, cov_c, cov_y, cov_air, cov_epa,
    cov_td, cov_t20, cov_c20, int, pbu, bust, jmp, jmpok, ff, // where he lined up
    or0, or1, or2, or3, or4, or5, or6, or7, or8, or9, or10, or11, or12, or13, or14, or15, ds0, ds1, ds2, ds3, ds4, ds5,
    ds6, ds7, ds8, ds9, ds10, ds11, ds12, ds13, // targets against man and zone (item 6)
    tgt_m, rec_m, sep_m, tgt_z, sep_z, // his long runs (item 9)
    c20,   // first tackles on runs, and those made from a block he never shed (item 7)
    ftk, ftk_blk,
);

/// The watched men: one per job on the typical team.
pub const ROLES: &[&str] = &[
    "QB1", "RB1", "RB2", "FB", "X", "Z", "SLOT", "TE1", "TE2", "TE3", "LT", "LG", "C", "RG", "RT", "EDGE1", "EDGE2",
    "DT1", "DT2", "DT3", "SAM", "MIKE", "WILL", "CB1", "CB2", "NCB", "FS", "SS",
];

/// Blocks of games: an arm's band comes from its 40 block sums, paired with
/// the same blocks of its context's typical arm.
pub const NB: usize = 40;

pub fn role(name: &str) -> usize {
    ROLES.iter().position(|r| *r == name).unwrap_or_else(|| panic!("role {name}"))
}

/// Floats an arm writes for one block.
pub fn width() -> usize {
    T::COUNT as usize + ROLES.len() * P::COUNT as usize
}

/// A successful play: 40% of the distance on first down, half on second,
/// all of it on third and fourth.
fn success(down: u8, togo: f64, gain: f64) -> bool {
    let need = match down {
        1 => 0.4 * togo,
        2 => 0.5 * togo,
        _ => togo,
    };
    gain >= need
}

// ------------------------------------------------------------------ contexts

/// The scheme team 0 plays in an arm. Offense contexts keep the Cover 3 4-3
/// defense; defense contexts keep the West Coast offense.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Ctx {
    Wc,
    Ar,
    Wz,
    Pg,
    So,
    Pg12,
    Pg13,
    Wc21,
    Q,
    Mp,
    C34,
    C2,
}

impl Ctx {
    pub const ALL: [Ctx; 12] = [
        Ctx::Wc,
        Ctx::Ar,
        Ctx::Wz,
        Ctx::Pg,
        Ctx::So,
        Ctx::Pg12,
        Ctx::Pg13,
        Ctx::Wc21,
        Ctx::Q,
        Ctx::Mp,
        Ctx::C34,
        Ctx::C2,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Ctx::Wc => "wc",
            Ctx::Ar => "ar",
            Ctx::Wz => "wz",
            Ctx::Pg => "pg",
            Ctx::So => "so",
            Ctx::Pg12 => "pg12",
            Ctx::Pg13 => "pg13",
            Ctx::Wc21 => "wc21",
            Ctx::Q => "q",
            Ctx::Mp => "mp",
            Ctx::C34 => "c34",
            Ctx::C2 => "c2",
        }
    }

    pub fn from_name(s: &str) -> Option<Ctx> {
        Ctx::ALL.into_iter().find(|c| c.name() == s)
    }

    /// In words, for the tables.
    pub fn title(self) -> &'static str {
        match self {
            Ctx::Wc => "West Coast, Cover 3 4-3",
            Ctx::Ar => "air raid",
            Ctx::Wz => "wide zone",
            Ctx::Pg => "power gap",
            Ctx::So => "spread option",
            Ctx::Pg12 => "power gap in 12 personnel",
            Ctx::Pg13 => "power gap in 13 personnel",
            Ctx::Wc21 => "West Coast in 21 personnel",
            Ctx::Q => "Quarters",
            Ctx::Mp => "man pressure",
            Ctx::C34 => "a 3-4",
            Ctx::C2 => "Cover 2",
        }
    }

    /// Team 0's offense, front and coverage shell in this context.
    pub fn schemes(self) -> (OffScheme, Front, Shell) {
        let off = match self {
            Ctx::Ar => OffScheme::AirRaid,
            Ctx::Wz => OffScheme::WideZone,
            Ctx::Pg | Ctx::Pg12 | Ctx::Pg13 => OffScheme::PowerGap,
            Ctx::So => OffScheme::SpreadOption,
            _ => OffScheme::WestCoast,
        };
        let front = if self == Ctx::C34 { Front::ThreeFour } else { Front::FourThree };
        let shell = match self {
            Ctx::Q => Shell::Quarters,
            Ctx::Mp => Shell::ManPressure,
            Ctx::C2 => Shell::Cover2,
            _ => Shell::Cover3,
        };
        (off, front, shell)
    }
}

/// Team 0's staff for a context.
pub fn apply_ctx(t: &mut GameTeam, c: Ctx) {
    let s = &mut t.staff;
    s.guide = Guidelines::default();
    let (off, front, shell) = c.schemes();
    s.off_scheme = off;
    s.front = front;
    s.shell = shell;
    s.defense.aggressiveness = 8.0;
    s.guide.man = Want::new(0.15, Firm::Firm);
    s.guide.pressure = Want::new(0.18, Firm::Firm);
    match c {
        Ctx::Pg12 => s.guide.personnel = Want::new(Personnel::P12, Firm::Firm),
        Ctx::Pg13 => s.guide.personnel = Want::new(Personnel::P13, Firm::Firm),
        Ctx::Wc21 => s.guide.personnel = Want::new(Personnel::P21, Firm::Firm),
        Ctx::Q => {
            s.defense.aggressiveness = 10.0;
            s.guide.man = Want::new(0.2, Firm::Firm);
            s.guide.pressure = Want::new(0.2, Firm::Firm);
        }
        Ctx::Mp => {
            s.defense.aggressiveness = 18.0;
            s.guide.man = Want::new(0.5, Firm::Firm);
            s.guide.pressure = Want::new(0.45, Firm::Firm);
        }
        _ => {}
    }
}

// ------------------------------------------------------------------ the typical team

pub fn ranked(t: &GameTeam, pos: Position) -> Vec<usize> {
    let mut v: Vec<usize> = (0..t.players.len()).filter(|&k| t.players[k].position == pos).collect();
    v.sort_by(|&a, &b| overall_for(pos, &t.players[b].attrs).total_cmp(&overall_for(pos, &t.players[a].attrs)));
    v
}

/// Starters at a position on the typical team.
pub fn starters(pos: Position) -> usize {
    match pos {
        Position::RB | Position::OT | Position::EDGE | Position::S => 2,
        Position::WR | Position::TE | Position::IOL | Position::DT | Position::LB | Position::CB => 3,
        _ => 1,
    }
}

/// How many men each watched job needs at his position.
const NEED: [(Position, usize); 11] = [
    (Position::QB, 1),
    (Position::RB, 2),
    (Position::WR, 3),
    (Position::TE, 3),
    (Position::OT, 2),
    (Position::IOL, 3),
    (Position::EDGE, 2),
    (Position::DT, 3),
    (Position::LB, 3),
    (Position::CB, 3),
    (Position::S, 2),
];

/// Team 0 set to the typical team, and its player index for each role.
pub struct Base {
    pub team: GameTeam,
    pub at: Vec<usize>,
}

pub fn base_team(sq: &Squad, level: Level) -> Base {
    let mut t = sq.team.clone();
    t.order.clear();
    t.ranked.clear();
    let n = t.players.len();
    let know = t.players.iter().map(|p| p.knowledge).sum::<f64>() / n as f64;
    let exp = t.players.iter().map(|p| p.experience).sum::<f64>() / n as f64;
    let mut lists: HashMap<Position, Vec<usize>> = HashMap::new();
    for pos in Position::ALL {
        let list = ranked(&t, pos);
        for (r, &k) in list.iter().enumerate() {
            let start = r < starters(pos);
            let (h, w, arm) = typical_body(level, pos);
            let mut ks = vec![Knob::Set(Part::Height, h), Knob::Set(Part::Weight, w), Knob::Set(Part::Arm, arm)];
            for a in Attr::ALL {
                let to = if start { typical_starter(level, pos, a) } else { typical(level, pos, a) };
                ks.push(Knob::Rating(a, to - t.players[k].attrs.get(a)));
            }
            change_one(&mut t.players[k], &sq.people[k], &ks, level);
            let gp = &mut t.players[k];
            gp.craft.clear();
            gp.tend = afm_engine::tendencies::Tendencies::default();
            gp.traits.consistency = 10.0;
            gp.traits.temperament = 10.0;
            gp.traits.recovery = 10.0;
            gp.knowledge = know;
            gp.experience = exp;
            for &sp in spots::starter_spots(pos) {
                gp.spots_played |= sp.bit();
            }
            gp.spots_played |= gp.spot.bit();
        }
        let ids: Vec<u32> = list.iter().map(|&k| t.players[k].id).collect();
        if !ids.is_empty() {
            t.order.push((pos, ids));
        }
        lists.insert(pos, list);
    }
    let l = |p: Position, i: usize| lists[&p][i];
    // The fullback: a back or tight end past the first, the one rated best
    // there (as `pick_fb` seats him when all are fresh); each knows the spot.
    let cands: Vec<usize> = lists[&Position::RB][1..].iter().chain(&lists[&Position::TE][1..]).copied().collect();
    for &k in &cands {
        t.players[k].spots_played |= Spot::FB.bit();
    }
    let fb = *cands
        .iter()
        .max_by(|&&a, &&b| t.players[a].spot_r[Spot::FB.index()].total_cmp(&t.players[b].spot_r[Spot::FB.index()]))
        .expect("a fullback");
    use Position as Ps;
    let at = vec![
        l(Ps::QB, 0),
        l(Ps::RB, 0),
        l(Ps::RB, 1),
        fb,
        l(Ps::WR, 0),
        l(Ps::WR, 1),
        l(Ps::WR, 2),
        l(Ps::TE, 0),
        l(Ps::TE, 1),
        l(Ps::TE, 2),
        l(Ps::OT, 0),
        l(Ps::IOL, 0),
        l(Ps::IOL, 1),
        l(Ps::IOL, 2),
        l(Ps::OT, 1),
        l(Ps::EDGE, 0),
        l(Ps::EDGE, 1),
        l(Ps::DT, 0),
        l(Ps::DT, 1),
        l(Ps::DT, 2),
        l(Ps::LB, 0),
        l(Ps::LB, 1),
        l(Ps::LB, 2),
        l(Ps::CB, 0),
        l(Ps::CB, 1),
        l(Ps::CB, 2),
        l(Ps::S, 0),
        l(Ps::S, 1),
    ];
    assert_eq!(at.len(), ROLES.len());
    Base { team: t, at }
}

/// The league an arm plays in. With `AFM_AB_WORLD` at the NFL, a world's 32
/// clubs as they would dress today; team 0 is the first of them with a man
/// for every watched job (a club may carry two tight ends), still set to the
/// typical team, and the other 31 play as they are. Otherwise the synthetic
/// league. Returns the world file's path when it is used.
pub fn league(level: Level, seed: u32) -> (Vec<Squad>, Option<String>) {
    match std::env::var("AFM_AB_WORLD") {
        Ok(path) if level == Level::NFL => {
            let mut base = world_nfl(&path);
            let full = base.iter().position(|sq| NEED.iter().all(|&(p, n)| ranked(&sq.team, p).len() >= n));
            let i = full.expect("a world club with every watched job");
            base.swap(0, i);
            eprintln!("world rosters: {} NFL clubs, team 0 is club {i}", base.len());
            (base, Some(path))
        }
        Ok(_) => {
            eprintln!("AFM_AB_WORLD is NFL only: this run plays the synthetic league");
            (synthetic(level, seed), None)
        }
        _ => (synthetic(level, seed), None),
    }
}

// ------------------------------------------------------------------ arms

/// One change to team 0, replayed on the same games as its context's
/// typical arm (`base-<ctx>`).
#[derive(Clone)]
pub struct Arm {
    pub id: String,
    pub ctx: Ctx,
    pub who: Vec<usize>,
    pub knobs: Vec<Knob>,
    pub label: String,
}

impl Arm {
    pub fn is_base(&self) -> bool {
        self.who.is_empty()
    }
}

fn aname(a: Attr) -> String {
    format!("{a:?}")
}

/// An as-recruited pair: four men drawn from one band of a body number
/// against four from another, each at the typical man's overall (one draw is
/// one man's mix of ratings, so a single draw says little).
pub struct Bands {
    pub role: &'static str,
    pub ctx: Ctx,
    pub part: Part,
    /// Each side's name ("heavy", "light"; "fast", "slow") and its four
    /// sub-bands, one drawn man each.
    pub sides: [(&'static str, [(f64, f64); 4]); 2],
}

/// The audit's bands (`fit_draws` and `rb_forty`, full pass 2026-11-16):
/// backs by weight (the same at every level) and by the forty, edges and
/// tackles by weight about each level's typical man. High school and FCS
/// use the FBS bands (the audit measured FBS and the NFL only).
pub fn bands(level: Level) -> Vec<Bands> {
    use Ctx::*;
    let nfl = level == Level::NFL;
    let rb_heavy = [(225.0, 231.0), (231.0, 237.0), (237.0, 243.0), (243.0, 250.0)];
    let rb_light = [(180.0, 186.0), (186.0, 192.0), (192.0, 198.0), (198.0, 205.0)];
    let (fast, slow) = if nfl {
        (
            [(4.38, 4.4), (4.4, 4.42), (4.42, 4.44), (4.44, 4.46)],
            [(4.6, 4.62), (4.62, 4.64), (4.64, 4.66), (4.66, 4.68)],
        )
    } else {
        ([(4.48, 4.5), (4.5, 4.52), (4.52, 4.54), (4.54, 4.56)], [(4.78, 4.8), (4.8, 4.82), (4.82, 4.84), (4.84, 4.86)])
    };
    let (edge_heavy, edge_light) = if nfl {
        (
            [(280.0, 285.0), (285.0, 290.0), (290.0, 295.0), (295.0, 305.0)],
            [(245.0, 250.0), (250.0, 255.0), (255.0, 260.0), (260.0, 265.0)],
        )
    } else {
        (
            [(255.0, 260.0), (260.0, 265.0), (265.0, 270.0), (270.0, 280.0)],
            [(225.0, 229.0), (229.0, 233.0), (233.0, 237.0), (237.0, 241.0)],
        )
    };
    let (dt_heavy, dt_light) = if nfl {
        (
            [(320.0, 325.0), (325.0, 330.0), (330.0, 335.0), (335.0, 345.0)],
            [(285.0, 290.0), (290.0, 295.0), (295.0, 300.0), (300.0, 305.0)],
        )
    } else {
        (
            [(295.0, 300.0), (300.0, 305.0), (305.0, 310.0), (310.0, 320.0)],
            [(262.0, 266.0), (266.0, 270.0), (270.0, 274.0), (274.0, 278.0)],
        )
    };
    let mut v = Vec::new();
    for ctx in [Pg, Wz] {
        v.push(Bands { role: "RB1", ctx, part: Part::Weight, sides: [("heavy", rb_heavy), ("light", rb_light)] });
    }
    for ctx in [Pg, Wz] {
        v.push(Bands { role: "RB1", ctx, part: Part::Forty, sides: [("fast", fast), ("slow", slow)] });
    }
    v.push(Bands { role: "EDGE1", ctx: Wc, part: Part::Weight, sides: [("heavy", edge_heavy), ("light", edge_light)] });
    v.push(Bands { role: "DT1", ctx: Wc, part: Part::Weight, sides: [("heavy", dt_heavy), ("light", dt_light)] });
    v
}

/// Every arm at a level: each context's typical arm, the audit's 377 arms on
/// one man's ratings or body (or a room's), then the as-recruited draws.
pub fn arms(level: Level) -> Vec<Arm> {
    use Attr as A;
    use Ctx::*;
    let mut v: Vec<Arm> = Vec::new();
    for c in Ctx::ALL {
        v.push(Arm { id: format!("base-{}", c.name()), ctx: c, who: vec![], knobs: vec![], label: "typical".into() });
    }
    let one = |v: &mut Vec<Arm>, r: &str, c: Ctx, attrs: &[A]| {
        for &a in attrs {
            v.push(Arm {
                id: format!("{r}-{}-{}+15", c.name(), aname(a)),
                ctx: c,
                who: vec![role(r)],
                knobs: vec![Knob::Rating(a, 15.0)],
                label: format!("{} +15", aname(a)),
            });
        }
    };
    let bundle = |v: &mut Vec<Arm>, r: &str, c: Ctx, name: &str, ks: &[(A, f64)]| {
        v.push(Arm {
            id: format!("{r}-{}-{name}", c.name()),
            ctx: c,
            who: vec![role(r)],
            knobs: ks.iter().map(|&(a, d)| Knob::Rating(a, d)).collect(),
            label: name.into(),
        });
    };
    let body = |v: &mut Vec<Arm>, r: &str, c: Ctx, part: Part, to: f64, name: &str| {
        v.push(Arm {
            id: format!("{r}-{}-{name}", c.name()),
            ctx: c,
            who: vec![role(r)],
            knobs: vec![Knob::Set(part, to)],
            label: name.into(),
        });
    };
    let iq = [(A::ZoneCoverage, 15.0), (A::PlayRecognition, 15.0), (A::FootballIq, 15.0), (A::Anticipation, 15.0)];
    let iq_dn = iq.map(|(a, d)| (a, -d));
    let run_sup = [(A::Tackling, 15.0), (A::Pursuit, 15.0), (A::BlockShedding, 15.0), (A::Strength, 15.0)];

    // ---- defense first: the questions Cain asked
    // Free safety
    one(
        &mut v,
        "FS",
        C2,
        &[A::ZoneCoverage, A::PlayRecognition, A::FootballIq, A::Anticipation, A::BallTracking, A::Speed],
    );
    for c in [Wc, Mp, Q, C2] {
        bundle(&mut v, "FS", c, "iq+15", &iq);
    }
    one(
        &mut v,
        "FS",
        Wc,
        &[
            A::ZoneCoverage,
            A::PlayRecognition,
            A::FootballIq,
            A::Anticipation,
            A::BallTracking,
            A::Catching,
            A::Speed,
            A::Acceleration,
            A::Jumping,
            A::Tackling,
            A::ManCoverage,
            A::Pursuit,
            A::Disguise,
            A::Communication,
            A::Decisions,
        ],
    );
    bundle(&mut v, "FS", Wc, "iq-15", &iq_dn);
    bundle(&mut v, "FS", Wc, "FootballIq-15", &[(A::FootballIq, -15.0)]);
    one(
        &mut v,
        "FS",
        Mp,
        &[A::ZoneCoverage, A::PlayRecognition, A::FootballIq, A::Anticipation, A::Speed, A::ManCoverage],
    );
    one(&mut v, "FS", Q, &[A::ZoneCoverage, A::PlayRecognition, A::Anticipation, A::FootballIq, A::Speed]);
    // Strong safety
    for c in [Wc, Mp, Q] {
        bundle(&mut v, "SS", c, "run+15", &run_sup);
    }
    one(
        &mut v,
        "SS",
        Wc,
        &[
            A::Tackling,
            A::Pursuit,
            A::BlockShedding,
            A::Strength,
            A::ManCoverage,
            A::ZoneCoverage,
            A::PlayRecognition,
            A::Speed,
            A::Blitzing,
            A::Physicality,
        ],
    );
    one(&mut v, "SS", Mp, &[A::ManCoverage, A::Blitzing, A::Tackling, A::Press]);
    one(&mut v, "SS", Q, &[A::ZoneCoverage, A::PlayRecognition]);
    // Corners
    one(
        &mut v,
        "CB1",
        Wc,
        &[
            A::ManCoverage,
            A::ZoneCoverage,
            A::Press,
            A::Speed,
            A::Acceleration,
            A::Agility,
            A::Jumping,
            A::BallTracking,
            A::Catching,
            A::PlayRecognition,
            A::Anticipation,
            A::FootballIq,
            A::Tackling,
        ],
    );
    body(&mut v, "CB1", Wc, Part::Height, 74.0, "height74");
    body(&mut v, "CB1", Wc, Part::Height, 70.0, "height70");
    one(&mut v, "CB1", Q, &[A::ZoneCoverage, A::ManCoverage, A::PlayRecognition]);
    one(&mut v, "CB1", Mp, &[A::ManCoverage, A::Press, A::Speed, A::Anticipation]);
    one(&mut v, "CB2", Wc, &[A::ManCoverage, A::ZoneCoverage]);
    one(&mut v, "CB2", Mp, &[A::ManCoverage, A::Press]);
    one(
        &mut v,
        "NCB",
        Wc,
        &[A::ManCoverage, A::ZoneCoverage, A::Agility, A::Tackling, A::Blitzing, A::PlayRecognition, A::Acceleration],
    );
    one(&mut v, "NCB", Mp, &[A::ManCoverage, A::Blitzing]);
    for c in [Wc, Mp, Q] {
        for (name, d) in [("man-built", 12.0), ("zone-built", -12.0)] {
            v.push(Arm {
                id: format!("CBS-{}-{name}", c.name()),
                ctx: c,
                who: vec![role("CB1"), role("CB2"), role("NCB")],
                knobs: vec![Knob::Rating(A::ManCoverage, d), Knob::Rating(A::ZoneCoverage, -d)],
                label: name.into(),
            });
        }
    }
    // Linebackers
    one(
        &mut v,
        "MIKE",
        Wc,
        &[
            A::Tackling,
            A::Pursuit,
            A::PlayRecognition,
            A::FootballIq,
            A::ZoneCoverage,
            A::ManCoverage,
            A::BlockShedding,
            A::Blitzing,
            A::Speed,
            A::Anticipation,
            A::Communication,
            A::Decisions,
            A::Traffic,
        ],
    );
    bundle(&mut v, "MIKE", Wc, "FootballIq-15", &[(A::FootballIq, -15.0)]);
    one(&mut v, "MIKE", Q, &[A::ZoneCoverage, A::PlayRecognition]);
    one(&mut v, "MIKE", Mp, &[A::Blitzing, A::ManCoverage, A::PlayRecognition, A::Speed]);
    one(&mut v, "MIKE", C34, &[A::Tackling, A::BlockShedding, A::PlayRecognition]);
    one(&mut v, "MIKE", C2, &[A::ZoneCoverage, A::Speed]);
    one(
        &mut v,
        "WILL",
        Wc,
        &[
            A::Pursuit,
            A::Speed,
            A::Tackling,
            A::ZoneCoverage,
            A::ManCoverage,
            A::PlayRecognition,
            A::Blitzing,
            A::Traffic,
        ],
    );
    one(&mut v, "WILL", Mp, &[A::Blitzing, A::ManCoverage]);
    one(
        &mut v,
        "SAM",
        Wc,
        &[A::BlockShedding, A::Tackling, A::PlayRecognition, A::ManCoverage, A::ZoneCoverage, A::Blitzing, A::Strength],
    );
    one(&mut v, "SAM", Mp, &[A::Blitzing, A::ManCoverage]);
    // Edges
    one(
        &mut v,
        "EDGE1",
        Wc,
        &[
            A::PowerRush,
            A::FinesseRush,
            A::GetOff,
            A::HandUse,
            A::Acceleration,
            A::Flexibility,
            A::Speed,
            A::Strength,
            A::Motor,
            A::PlayRecognition,
            A::BlockShedding,
            A::Tackling,
            A::Pursuit,
            A::Discipline,
        ],
    );
    body(&mut v, "EDGE1", Wc, Part::Weight, 270.0, "weight270");
    body(&mut v, "EDGE1", Wc, Part::Weight, 240.0, "weight240");
    one(&mut v, "EDGE1", C34, &[A::PowerRush, A::FinesseRush, A::GetOff, A::BlockShedding]);
    one(&mut v, "EDGE1", Mp, &[A::FinesseRush, A::PowerRush, A::GetOff, A::Speed]);
    one(&mut v, "EDGE2", Wc, &[A::FinesseRush, A::PowerRush, A::BlockShedding]);
    // Interior line
    one(
        &mut v,
        "DT1",
        Wc,
        &[
            A::PowerRush,
            A::FinesseRush,
            A::GetOff,
            A::HandUse,
            A::Strength,
            A::BlockShedding,
            A::Tackling,
            A::PlayRecognition,
            A::Acceleration,
            A::Anchor,
        ],
    );
    body(&mut v, "DT1", Wc, Part::Weight, 320.0, "weight320");
    body(&mut v, "DT1", Wc, Part::Weight, 290.0, "weight290");
    one(&mut v, "DT1", C34, &[A::Strength, A::BlockShedding, A::PowerRush]);
    body(&mut v, "DT1", C34, Part::Weight, 330.0, "weight330");
    one(&mut v, "DT1", Mp, &[A::PowerRush, A::FinesseRush, A::GetOff]);
    one(&mut v, "DT2", Wc, &[A::PowerRush, A::BlockShedding, A::Strength]);
    one(&mut v, "DT2", C34, &[A::Strength, A::BlockShedding]);
    one(&mut v, "DT3", C34, &[A::Strength, A::BlockShedding]);

    // ---- offense
    // Quarterback
    one(
        &mut v,
        "QB1",
        Wc,
        &[
            A::ShortAccuracy,
            A::MediumAccuracy,
            A::DeepAccuracy,
            A::ArmStrength,
            A::Touch,
            A::QuickRelease,
            A::PocketPresence,
            A::Decisions,
            A::FootballIq,
            A::Anticipation,
            A::EyeManipulation,
            A::ThrowOnMove,
            A::Composure,
            A::BallHandling,
            A::Speed,
            A::Footwork,
        ],
    );
    one(
        &mut v,
        "QB1",
        Ar,
        &[A::DeepAccuracy, A::ArmStrength, A::QuickRelease, A::Decisions, A::Anticipation, A::ShortAccuracy],
    );
    one(&mut v, "QB1", Wz, &[A::BallHandling, A::ThrowOnMove, A::MediumAccuracy, A::Decisions, A::Speed]);
    one(&mut v, "QB1", So, &[A::Speed, A::BallHandling, A::Decisions, A::Elusiveness]);
    // Receivers
    let star = [(A::RouteRunning, 15.0), (A::Catching, 15.0), (A::Release, 15.0), (A::Agility, 15.0), (A::Speed, 10.0)];
    one(
        &mut v,
        "X",
        Wc,
        &[
            A::RouteRunning,
            A::Release,
            A::Catching,
            A::ContestedCatch,
            A::BallTracking,
            A::BodyControl,
            A::Speed,
            A::Acceleration,
            A::Agility,
            A::Jumping,
            A::Elusiveness,
            A::Concentration,
            A::RunBlocking,
        ],
    );
    body(&mut v, "X", Wc, Part::Height, 76.0, "height76");
    body(&mut v, "X", Wc, Part::Height, 70.0, "height70");
    one(&mut v, "X", Ar, &[A::RouteRunning, A::Release, A::Speed, A::Catching, A::ContestedCatch, A::BallTracking]);
    for c in [Wc, Pg, Pg13] {
        bundle(&mut v, "X", c, "star", &star);
    }
    one(&mut v, "Z", Wc, &[A::RouteRunning, A::Release, A::Catching, A::Speed, A::ContestedCatch, A::Agility]);
    one(&mut v, "Z", Wz, &[A::RouteRunning, A::Speed, A::RunBlocking]);
    one(
        &mut v,
        "SLOT",
        Wc,
        &[
            A::RouteRunning,
            A::Catching,
            A::Agility,
            A::Acceleration,
            A::Elusiveness,
            A::Release,
            A::Speed,
            A::Concentration,
        ],
    );
    one(&mut v, "SLOT", Ar, &[A::RouteRunning, A::Agility, A::Catching, A::Elusiveness]);
    // Tight ends
    one(
        &mut v,
        "TE1",
        Wc,
        &[
            A::Catching,
            A::RouteRunning,
            A::Release,
            A::ContestedCatch,
            A::RunBlocking,
            A::PassBlocking,
            A::Speed,
            A::Strength,
        ],
    );
    one(&mut v, "TE1", Pg12, &[A::Catching, A::RouteRunning, A::RunBlocking, A::BlockingInSpace]);
    one(&mut v, "TE1", Pg13, &[A::Catching, A::RouteRunning, A::RunBlocking]);
    one(&mut v, "TE2", Pg12, &[A::Catching, A::RouteRunning, A::RunBlocking]);
    one(&mut v, "TE2", Pg13, &[A::Catching, A::RunBlocking]);
    one(&mut v, "TE3", Pg13, &[A::Catching, A::RunBlocking]);
    // Backs
    one(
        &mut v,
        "RB1",
        Pg,
        &[
            A::BallCarrierVision,
            A::Elusiveness,
            A::LegDrive,
            A::Balance,
            A::Strength,
            A::Speed,
            A::Acceleration,
            A::Agility,
            A::BallSecurity,
            A::Catching,
            A::PassBlocking,
        ],
    );
    one(&mut v, "RB1", Wz, &[A::BallCarrierVision, A::Elusiveness, A::LegDrive, A::Speed, A::Agility, A::Acceleration]);
    one(&mut v, "RB1", Ar, &[A::Catching, A::RouteRunning, A::PassBlocking, A::Elusiveness, A::BallCarrierVision]);
    for c in [Pg, Wz, Wc21] {
        body(&mut v, "RB1", c, Part::Weight, 235.0, "weight235");
        body(&mut v, "RB1", c, Part::Weight, 195.0, "weight195");
    }
    one(&mut v, "RB2", Wc, &[A::Catching, A::RouteRunning, A::PassBlocking, A::Elusiveness]);
    one(&mut v, "FB", Wc21, &[A::RunBlocking, A::BlockingInSpace, A::Strength, A::LegDrive, A::Catching]);
    // Offensive line
    one(
        &mut v,
        "LT",
        Wc,
        &[
            A::PassBlocking,
            A::RunBlocking,
            A::HandUse,
            A::Anchor,
            A::Agility,
            A::Strength,
            A::BlockingInSpace,
            A::Footwork,
        ],
    );
    one(&mut v, "LT", Ar, &[A::PassBlocking, A::Anchor]);
    one(&mut v, "LT", Wz, &[A::RunBlocking, A::Agility, A::BlockingInSpace]);
    one(&mut v, "LT", Pg, &[A::RunBlocking, A::Strength, A::Anchor]);
    one(&mut v, "RT", Wc, &[A::PassBlocking, A::RunBlocking, A::Anchor]);
    one(&mut v, "RT", Pg, &[A::RunBlocking, A::Strength]);
    one(
        &mut v,
        "C",
        Wc,
        &[A::PassBlocking, A::RunBlocking, A::Anchor, A::FootballIq, A::Communication, A::ShotgunSnap],
    );
    one(&mut v, "C", Wz, &[A::RunBlocking, A::Agility]);
    one(&mut v, "C", Pg, &[A::RunBlocking, A::Strength]);
    one(&mut v, "LG", Wc, &[A::PassBlocking, A::RunBlocking, A::Anchor, A::Strength]);
    one(&mut v, "LG", Wz, &[A::RunBlocking, A::Agility]);
    one(&mut v, "LG", Pg, &[A::RunBlocking, A::Strength, A::LegDrive]);
    one(&mut v, "RG", Wc, &[A::PassBlocking, A::RunBlocking]);
    one(&mut v, "RG", Pg, &[A::RunBlocking]);

    // ---- as recruited: four drawn men a band
    for b in bands(level) {
        for (side, subs) in &b.sides {
            for (i, &(lo, hi)) in subs.iter().enumerate() {
                v.push(Arm {
                    id: format!("{}-{}-{side}{}", b.role, b.ctx.name(), i + 1),
                    ctx: b.ctx,
                    who: vec![role(b.role)],
                    knobs: vec![Knob::Recruit(b.part, lo, hi)],
                    label: format!("{side} {}", i + 1),
                });
            }
        }
    }
    v
}

/// The audit's repeat set: the arms it played again on FBS seed 171 and on
/// world seed 1's NFL rosters, every arm behind the report's repeat table
/// among them (`read::CLAIMS`). Its two single drawn backs are the power-gap
/// band's eight drawn men here.
pub const REPEAT: &[&str] = &[
    "QB1-wc-Decisions+15",
    "QB1-wc-DeepAccuracy+15",
    "QB1-wc-EyeManipulation+15",
    "QB1-wc-PocketPresence+15",
    "QB1-wc-ShortAccuracy+15",
    "RB1-pg-BallCarrierVision+15",
    "RB1-wz-BallCarrierVision+15",
    "RB1-pg-weight195",
    "RB1-pg-weight235",
    "RB1-wz-weight195",
    "RB1-wz-weight235",
    "RB1-wc21-weight195",
    "RB1-wc21-weight235",
    "RB1-pg-heavy1",
    "RB1-pg-heavy2",
    "RB1-pg-heavy3",
    "RB1-pg-heavy4",
    "RB1-pg-light1",
    "RB1-pg-light2",
    "RB1-pg-light3",
    "RB1-pg-light4",
    "FB-wc21-RunBlocking+15",
    "FB-wc21-BlockingInSpace+15",
    "FB-wc21-Strength+15",
    "FB-wc21-LegDrive+15",
    "FB-wc21-Catching+15",
    "X-wc-RouteRunning+15",
    "X-wc-star",
    "X-pg-star",
    "X-pg13-star",
    "X-wc-height70",
    "X-wc-height76",
    "TE1-wc-Catching+15",
    "TE1-pg13-Catching+15",
    "TE1-pg13-RouteRunning+15",
    "TE3-pg13-Catching+15",
    "LT-wc-PassBlocking+15",
    "LT-pg-RunBlocking+15",
    "EDGE1-wc-FinesseRush+15",
    "EDGE1-wc-PowerRush+15",
    "EDGE1-wc-GetOff+15",
    "EDGE1-wc-weight240",
    "EDGE1-wc-weight270",
    "DT1-wc-FinesseRush+15",
    "DT1-wc-PowerRush+15",
    "DT1-wc-weight290",
    "DT1-wc-weight320",
    "MIKE-wc-FootballIq+15",
    "MIKE-wc-FootballIq-15",
    "MIKE-wc-PlayRecognition+15",
    "MIKE-wc-Tackling+15",
    "MIKE-mp-Blitzing+15",
    "WILL-mp-Blitzing+15",
    "CB1-wc-ZoneCoverage+15",
    "CB1-wc-ManCoverage+15",
    "CB1-mp-ManCoverage+15",
    "CBS-wc-man-built",
    "CBS-wc-zone-built",
    "CBS-mp-man-built",
    "CBS-mp-zone-built",
    "CBS-q-man-built",
    "CBS-q-zone-built",
    "FS-wc-ZoneCoverage+15",
    "FS-wc-FootballIq+15",
    "FS-wc-FootballIq-15",
    "FS-wc-Anticipation+15",
    "FS-wc-BallTracking+15",
    "FS-wc-PlayRecognition+15",
    "FS-wc-iq+15",
    "FS-mp-ZoneCoverage+15",
    "FS-mp-Anticipation+15",
    "FS-mp-iq+15",
    "SS-wc-run+15",
    "SS-wc-ManCoverage+15",
    "SS-mp-run+15",
    "SS-mp-ManCoverage+15",
    "SS-mp-Blitzing+15",
];

/// The arms a filter picks: every arm with none; else the arms whose id
/// contains any of the words (`bases` picks the typical arms, `draws` the
/// as-recruited ones, `repeat` the audit's repeat set, `REPEAT`). Each
/// picked arm's typical arm comes too, first.
pub fn pick(all: &[Arm], filter: &[String]) -> Vec<Arm> {
    if filter.is_empty() {
        return all.to_vec();
    }
    let draw = |a: &Arm| a.knobs.iter().any(|k| matches!(k, Knob::Recruit(..)));
    let chosen = |a: &Arm| {
        filter.iter().any(|f| match f.as_str() {
            "bases" => a.is_base(),
            "draws" => draw(a),
            "repeat" => REPEAT.contains(&a.id.as_str()),
            w => a.id.contains(w),
        })
    };
    let picked: Vec<&Arm> = all.iter().filter(|a| chosen(a)).collect();
    let ctxs: Vec<Ctx> = picked.iter().map(|a| a.ctx).collect();
    let mut out: Vec<Arm> = all.iter().filter(|a| a.is_base() && ctxs.contains(&a.ctx)).cloned().collect();
    out.extend(picked.into_iter().filter(|a| !a.is_base()).cloned());
    out
}

// ------------------------------------------------------------------ one game

struct Games<'a> {
    teams: Vec<&'a GameTeam>,
    level: Level,
    seed: u32,
    role_of: HashMap<u32, usize>,
    pos_of: HashMap<u32, Position>,
}

/// The game seed of game `g`: the same in every arm.
pub fn game_seed(seed: u32, g: usize) -> u64 {
    seed as u64 * 1_000_003 + g as u64
}

/// One game of team 0's, read into its slots. Team 0 meets each other team
/// in turn, at home on even games.
fn one(cx: &Games, g: usize) -> Vec<f64> {
    use PlayType as K;
    let n = cx.teams.len();
    let opp = 1 + (g / 2) % (n - 1);
    let (h, a) = if g.is_multiple_of(2) { (0, opp) } else { (opp, 0) };
    let venue =
        Venue { cond: Conditions { crowd: 0.65, ..Conditions::default() }, wear: [0.0, 0.012], ..Venue::default() };
    let r = quick_game_in(cx.teams[h], cx.teams[a], cx.level, game_seed(cx.seed, g), Default::default(), venue);
    let me = if h == 0 { 0usize } else { 1 };
    let mut m = vec![0.0f64; width()];
    let tb = T::COUNT as usize;
    let pw = P::COUNT as usize;
    let pi = |ri: usize, s: P| tb + ri * pw + s as usize;
    let ro = |id: &u32| cx.role_of.get(id).copied();
    for p in &r.stats.plays {
        if p.nullified || !matches!(p.kind, K::Run | K::Pass | K::Sack | K::Scramble) {
            continue;
        }
        let db = matches!(p.kind, K::Pass | K::Sack | K::Scramble);
        let run = p.kind == K::Run;
        let pass = p.kind == K::Pass;
        let att = pass && !p.throw_away;
        let suc = success(p.down, p.togo, p.yards);
        let off = p.offense == me;
        let side = if off { 0 } else { 1 };
        for (j, id) in p.on_field[side].iter().enumerate() {
            let Some(ri) = ro(id) else { continue };
            m[pi(ri, P::on)] += 1.0;
            if p.down >= 3 {
                m[pi(ri, P::on3)] += 1.0;
            }
            if db {
                m[pi(ri, P::on_db)] += 1.0;
                m[pi(ri, P::on_db_epa)] += p.epa;
                if p.kind == K::Sack {
                    m[pi(ri, P::on_sack)] += 1.0;
                }
                if p.pressured {
                    m[pi(ri, P::on_prs)] += 1.0;
                }
            }
            if run {
                m[pi(ri, P::on_run)] += 1.0;
                m[pi(ri, P::on_run_yds)] += p.yards;
                m[pi(ri, P::on_run_suc)] += f64::from(suc);
                m[pi(ri, P::on_run_epa)] += p.epa;
            }
            if let Some(&rv) = p.roles[side].get(j) {
                let rv = rv as usize;
                if off && rv < 16 {
                    m[pi(ri, P::or0) + rv] += 1.0;
                } else if !off && rv < 14 {
                    m[pi(ri, P::ds0) + rv] += 1.0;
                }
            }
        }
        if off {
            let mut t = |s: T, x: f64| m[s as usize] += x;
            t(T::o_snaps, 1.0);
            t(T::o_epa, p.epa);
            if db {
                t(T::o_db, 1.0);
                t(T::o_db_epa, p.epa);
                t(T::o_prs, f64::from(p.pressured));
                t(T::o_pa, f64::from(p.play_action));
            }
            if pass {
                t(T::o_att, 1.0);
                t(T::o_ttt, p.time_to_throw);
                if p.complete {
                    t(T::o_cmp, 1.0);
                    t(T::o_pyds, p.yards);
                    t(T::o_yac, p.yac);
                    t(T::o_p20, f64::from(p.yards >= 20.0));
                }
                if att {
                    t(T::o_air, p.air_yards);
                    t(T::o_sep, p.separation);
                }
                t(T::o_int, f64::from(p.interception));
                t(T::o_drop, f64::from(p.drop));
            }
            if p.kind == K::Sack {
                t(T::o_sack, 1.0);
            }
            if p.kind == K::Scramble {
                t(T::o_scr, 1.0);
                t(T::o_scr_yds, p.yards);
            }
            if run {
                t(T::o_run, 1.0);
                t(T::o_ryds, p.yards);
                t(T::o_rsuc, f64::from(suc));
                t(T::o_r10, f64::from(p.yards >= 10.0));
                t(T::o_run_epa, p.epa);
            }
            if p.down >= 3 {
                t(T::o_3att, 1.0);
                t(T::o_3conv, f64::from(p.first_down || (p.touchdown && !p.defense_td)));
            }
            t(T::o_td, f64::from(p.touchdown && !p.defense_td));
            t(T::o_to, f64::from(p.turnover));
            let (ps, pd) = match p.personnel.as_str() {
                "11" => (T::o_p11, Some(T::o_d11)),
                "12" => (T::o_p12, Some(T::o_d12)),
                "13" => (T::o_p13, Some(T::o_d13)),
                "21" => (T::o_p21, Some(T::o_d21)),
                "22" => (T::o_p22, Some(T::o_d22)),
                "10" => (T::o_p10, Some(T::o_d10)),
                _ => (T::o_poth, None),
            };
            t(ps, 1.0);
            if db && let Some(pd) = pd {
                t(pd, 1.0);
            }
            if pass && let Some(tg) = p.target {
                let y = if p.complete { p.yards } else { 0.0 };
                match cx.pos_of.get(&tg) {
                    Some(Position::WR) => {
                        t(T::o_tgt_wr, 1.0);
                        t(T::o_yds_wr, y);
                    }
                    Some(Position::TE) => {
                        t(T::o_tgt_te, 1.0);
                        t(T::o_yds_te, y);
                    }
                    Some(Position::RB) => {
                        t(T::o_tgt_rb, 1.0);
                        t(T::o_yds_rb, y);
                    }
                    _ => {}
                }
            }
            // the watched men on offense
            if pass && let Some(ri) = p.target.as_ref().and_then(ro) {
                m[pi(ri, P::tgt)] += 1.0;
                m[pi(ri, P::tgt_epa)] += p.epa;
                if att {
                    m[pi(ri, P::air)] += p.air_yards;
                    m[pi(ri, P::sep)] += p.separation;
                    m[pi(ri, P::tgt20)] += f64::from(p.air_yards >= 20.0);
                }
                m[pi(ri, P::drop)] += f64::from(p.drop);
                if p.complete {
                    m[pi(ri, P::rec)] += 1.0;
                    m[pi(ri, P::ryd)] += p.yards;
                    m[pi(ri, P::yac)] += p.yac;
                    m[pi(ri, P::rec20)] += f64::from(p.air_yards >= 20.0);
                    m[pi(ri, P::rtd)] += f64::from(p.touchdown && !p.defense_td);
                    m[pi(ri, P::r1d)] += f64::from(p.first_down);
                }
            }
            for (_, runner) in &p.missed_tackles {
                if let Some(ri) = ro(runner) {
                    if run {
                        m[pi(ri, P::cmt)] += 1.0;
                    } else if pass {
                        m[pi(ri, P::rmt)] += 1.0;
                    }
                }
            }
            if run && let Some(ri) = p.carrier.as_ref().and_then(ro) {
                m[pi(ri, P::car)] += 1.0;
                m[pi(ri, P::cyds)] += p.yards;
                m[pi(ri, P::csuc)] += f64::from(suc);
                m[pi(ri, P::c10)] += f64::from(p.yards >= 10.0);
                m[pi(ri, P::cbc)] += p.before_contact;
                m[pi(ri, P::cepa)] += p.epa;
                m[pi(ri, P::ctd)] += f64::from(p.touchdown && !p.defense_td);
                m[pi(ri, P::cneg)] += f64::from(p.yards < 0.0);
            }
            if let Some(ri) = p.fumbled.as_ref().and_then(ro) {
                m[pi(ri, P::cfum)] += 1.0;
            }
            if db {
                let qb = if p.kind == K::Scramble { p.carrier.or(p.passer) } else { p.passer.or(p.carrier) };
                if let Some(ri) = qb.as_ref().and_then(ro) {
                    m[pi(ri, P::pdb)] += 1.0;
                    m[pi(ri, P::pepa)] += p.epa;
                    if pass {
                        m[pi(ri, P::patt)] += 1.0;
                        m[pi(ri, P::pint)] += f64::from(p.interception);
                        if p.complete {
                            m[pi(ri, P::pcmp)] += 1.0;
                            m[pi(ri, P::pyds)] += p.yards;
                        }
                        if att {
                            m[pi(ri, P::pair)] += p.air_yards;
                        }
                    }
                    if p.kind == K::Sack {
                        m[pi(ri, P::psack)] += 1.0;
                    }
                    if p.kind == K::Scramble {
                        m[pi(ri, P::pscr)] += 1.0;
                        m[pi(ri, P::pscr_yds)] += p.yards;
                    }
                }
            }
            for &(b, d, bp, won, secs, _) in &p.blocks {
                let Some(ri) = ro(&b) else { continue };
                if bp {
                    m[pi(ri, P::pbr)] += 1.0;
                    if won {
                        m[pi(ri, P::pbw)] += 1.0;
                    } else {
                        if p.pressures.contains(&d) || p.sackers.contains(&d) || p.hits.contains(&d) {
                            m[pi(ri, P::pba)] += 1.0;
                        }
                        if p.sackers.contains(&d) {
                            m[pi(ri, P::sca)] += 1.0;
                        }
                    }
                } else {
                    m[pi(ri, P::rbr)] += 1.0;
                    m[pi(ri, P::rbw)] += f64::from(won);
                    m[pi(ri, P::rbs)] += secs;
                }
            }
        } else {
            let mut t = |s: T, x: f64| m[s as usize] += x;
            t(T::d_snaps, 1.0);
            t(T::d_epa, p.epa);
            t(T::d_blitz, f64::from(p.blitz));
            t(T::d_man, f64::from(p.coverage_man));
            t(T::d_box, f64::from(p.box_count));
            t(T::d_base, f64::from(p.dpers == 1));
            t(T::d_nickel, f64::from(p.dpers == 2));
            t(T::d_dime, f64::from(p.dpers == 3));
            if db {
                t(T::d_db, 1.0);
                t(T::d_db_epa, p.epa);
                t(T::d_prs, f64::from(p.pressured));
            }
            if pass {
                t(T::d_att, 1.0);
                t(T::d_ttt, p.time_to_throw);
                t(T::d_int, f64::from(p.interception));
                if p.complete {
                    t(T::d_cmp, 1.0);
                    t(T::d_pyds, p.yards);
                    t(T::d_p20, f64::from(p.yards >= 20.0));
                }
                if att {
                    t(T::d_air, p.air_yards);
                    if p.air_yards >= 20.0 {
                        t(T::d_deep_att, 1.0);
                        t(T::d_deep_cmp, f64::from(p.complete));
                    }
                }
            }
            if p.kind == K::Sack {
                t(T::d_sack, 1.0);
            }
            if run {
                t(T::d_run, 1.0);
                t(T::d_ryds, p.yards);
                t(T::d_rsuc, f64::from(suc));
                t(T::d_r10, f64::from(p.yards >= 10.0));
                t(T::d_run_epa, p.epa);
            }
            if p.down >= 3 {
                t(T::d_3att, 1.0);
                t(T::d_3conv, f64::from(p.first_down || (p.touchdown && !p.defense_td)));
            }
            t(T::d_td, f64::from(p.touchdown && !p.defense_td));
            t(T::d_to, f64::from(p.turnover));
            let group = |id: Option<u32>| id.and_then(|i| cx.pos_of.get(&i).copied());
            match group(p.interceptor.filter(|_| p.interception)) {
                Some(Position::S) => t(T::d_int_s, 1.0),
                Some(Position::CB) => t(T::d_int_cb, 1.0),
                Some(Position::LB) => t(T::d_int_lb, 1.0),
                Some(Position::EDGE | Position::DT) => t(T::d_int_dl, 1.0),
                _ => {}
            }
            if pass {
                match group(p.pbu) {
                    Some(Position::S) => t(T::d_pbu_s, 1.0),
                    Some(Position::CB) => t(T::d_pbu_cb, 1.0),
                    Some(Position::LB) => t(T::d_pbu_lb, 1.0),
                    Some(Position::EDGE | Position::DT) => t(T::d_pbu_dl, 1.0),
                    _ => {}
                }
            }
            let ns = p.sackers.len().max(1) as f64;
            for s in &p.sackers {
                match cx.pos_of.get(s) {
                    Some(Position::EDGE | Position::DT) => t(T::d_sack_dl, 1.0 / ns),
                    Some(Position::LB) => t(T::d_sack_lb, 1.0 / ns),
                    Some(Position::CB | Position::S) => t(T::d_sack_db, 1.0 / ns),
                    _ => {}
                }
            }
            // the watched men on defense
            for tk in &p.tacklers {
                let Some(ri) = ro(tk) else { continue };
                m[pi(ri, P::tkl)] += 1.0;
                if run {
                    m[pi(ri, P::rtkl)] += 1.0;
                    m[pi(ri, P::stop)] += f64::from(!suc);
                    m[pi(ri, P::tfl)] += f64::from(p.yards < 0.0);
                }
            }
            for (tk, _) in &p.missed_tackles {
                if let Some(ri) = ro(tk) {
                    m[pi(ri, P::mtk)] += 1.0;
                }
            }
            for x in &p.pressures {
                if let Some(ri) = ro(x) {
                    m[pi(ri, P::prs)] += 1.0;
                }
            }
            for x in &p.sackers {
                if let Some(ri) = ro(x) {
                    m[pi(ri, P::sck)] += 1.0 / ns;
                }
            }
            for x in &p.hits {
                if let Some(ri) = ro(x) {
                    m[pi(ri, P::hit)] += 1.0;
                }
            }
            for (x, _, _) in &p.rush_wins {
                if let Some(ri) = ro(x) {
                    m[pi(ri, P::rwin)] += 1.0;
                }
            }
            for (x, _) in &p.contact {
                if let Some(ri) = ro(x) {
                    m[pi(ri, P::rrep)] += 1.0;
                }
            }
            for &(_, d, bp, won, _, _) in &p.blocks {
                let Some(ri) = ro(&d) else { continue };
                if bp {
                    m[pi(ri, P::pbf)] += 1.0;
                    m[pi(ri, P::pbl)] += f64::from(!won);
                } else {
                    m[pi(ri, P::rbf)] += 1.0;
                    m[pi(ri, P::shed)] += f64::from(!won);
                }
            }
            if att && let Some(ri) = p.defender.as_ref().and_then(ro) {
                m[pi(ri, P::cov_t)] += 1.0;
                m[pi(ri, P::cov_air)] += p.air_yards;
                m[pi(ri, P::cov_epa)] += p.epa;
                if p.complete {
                    m[pi(ri, P::cov_c)] += 1.0;
                    m[pi(ri, P::cov_y)] += p.yards;
                    m[pi(ri, P::cov_td)] += f64::from(p.touchdown && !p.defense_td);
                }
                if p.air_yards >= 20.0 {
                    m[pi(ri, P::cov_t20)] += 1.0;
                    m[pi(ri, P::cov_c20)] += f64::from(p.complete);
                }
            }
            if p.interception
                && let Some(ri) = p.interceptor.as_ref().and_then(ro)
            {
                m[pi(ri, P::int)] += 1.0;
            }
            if pass && let Some(ri) = p.pbu.as_ref().and_then(ro) {
                m[pi(ri, P::pbu)] += 1.0;
            }
            for x in &p.busts {
                if let Some(ri) = ro(x) {
                    m[pi(ri, P::bust)] += 1.0;
                }
            }
            if let Some((x, ok)) = p.tend.jump
                && let Some(ri) = ro(&x)
            {
                m[pi(ri, P::jmp)] += 1.0;
                m[pi(ri, P::jmpok)] += f64::from(ok);
            }
            if let Some(ri) = p.forced_fumble.as_ref().and_then(ro) {
                m[pi(ri, P::ff)] += 1.0;
            }
        }
        items(cx, p, off, &mut m);
    }
    let (pf, pa) = (f64::from(r.stats.score[me]), f64::from(r.stats.score[1 - me]));
    m[T::g_pf as usize] = pf;
    m[T::g_pa as usize] = pa;
    m[T::g_win as usize] = if pf > pa {
        1.0
    } else if pf == pa {
        0.5
    } else {
        0.0
    };
    m
}

/// The scheme-fit items' own measures on one snap (each lane's targets,
/// `docs/design/scheme-fit.md`), read the way the audit's traces read them
/// (`run_where` and `int_where` in `afm/scheme-fit/tools/`).
fn items(cx: &Games, p: &afm_engine::game::stats::PlayRecord, off: bool, m: &mut [f64]) {
    use PlayType as K;
    let db = matches!(p.kind, K::Pass | K::Sack | K::Scramble);
    let run = p.kind == K::Run;
    let pass = p.kind == K::Pass;
    // Targeted throws, as `int_where` counts them: no throwaways.
    let targeted = pass && !p.throw_away && p.target.is_some();
    let tb = T::COUNT as usize;
    let pw = P::COUNT as usize;
    let pi = |ri: usize, s: P| tb + ri * pw + s as usize;
    let ro = |id: &u32| cx.role_of.get(id).copied();
    let pos = |id: Option<u32>| id.and_then(|i| cx.pos_of.get(&i).copied());
    let add = |m: &mut [f64], s: T, x: f64| m[s as usize] += x;
    // The first tackle on a designed run, and whether its man was still in
    // a run block he never shed (`run_where`).
    let first = p.tacklers.first().copied();
    let first_blk = run && first.is_some_and(|d| p.blocks.iter().any(|b| b.1 == d && !b.2 && b.3));
    // 3rd and 4th down with 2 or less to go: designed runs by the QB, and
    // by everyone else (`run_where`).
    let short = run && p.down >= 3 && p.togo <= 2.0;
    let qb_run = pos(p.carrier) == Some(Position::QB);
    let conv = f64::from(p.first_down || p.touchdown);
    let rb_run = run && pos(p.carrier) == Some(Position::RB);
    let (sy, sy_cv) = match (off, qb_run) {
        (true, true) => (T::o_sy_qb, T::o_sy_qb_cv),
        (true, false) => (T::o_sy_rb, T::o_sy_rb_cv),
        (false, true) => (T::d_sy_qb, T::d_sy_qb_cv),
        (false, false) => (T::d_sy_rb, T::d_sy_rb_cv),
    };
    if short {
        add(m, sy, 1.0);
        add(m, sy_cv, conv);
    }
    let (rtk, rtk_blk, rb_car, rb_r10, rb_r20) = if off {
        (T::o_rtk, T::o_rtk_blk, T::o_rb_car, T::o_rb_r10, T::o_rb_r20)
    } else {
        (T::d_rtk, T::d_rtk_blk, T::d_rb_car, T::d_rb_r10, T::d_rb_r20)
    };
    if run && first.is_some() {
        add(m, rtk, 1.0);
        add(m, rtk_blk, f64::from(first_blk));
    }
    if rb_run {
        add(m, rb_car, 1.0);
        add(m, rb_r10, f64::from(p.yards >= 10.0));
        add(m, rb_r20, f64::from(p.yards >= 20.0));
    }
    if off {
        if db && p.play_action {
            add(m, T::o_pa_epa, p.epa);
            add(m, T::o_pa_sack, f64::from(p.kind == K::Sack));
        }
        if db && p.call.contains("boot") {
            add(m, T::o_boot, 1.0);
            add(m, T::o_boot_sack, f64::from(p.kind == K::Sack));
            add(m, T::o_boot_epa, p.epa);
        }
        if pass && let Some(tg) = p.target {
            let g = match p.personnel.as_str() {
                "11" => Some(0),
                "12" => Some(1),
                "13" => Some(2),
                "21" => Some(3),
                _ => None,
            };
            let k = match cx.pos_of.get(&tg) {
                Some(Position::WR) => Some(0),
                Some(Position::TE) => Some(1),
                Some(Position::RB) => Some(2),
                _ => None,
            };
            if let (Some(g), Some(k)) = (g, k) {
                m[T::o_tg11_wr as usize + 3 * g + k] += 1.0;
            }
        }
        if targeted && p.coverage_man {
            add(m, T::o_att_man, 1.0);
            add(m, T::o_sep_man, p.separation);
            add(m, T::o_cmp_man, f64::from(p.complete));
        } else if targeted {
            add(m, T::o_att_zone, 1.0);
            add(m, T::o_sep_zone, p.separation);
        }
        if targeted && p.air_yards >= 30.0 {
            add(m, T::o_att30, 1.0);
            add(m, T::o_int30, f64::from(p.interception));
        }
        // the watched men
        if targeted && let Some(ri) = p.target.as_ref().and_then(ro) {
            if p.coverage_man {
                m[pi(ri, P::tgt_m)] += 1.0;
                m[pi(ri, P::rec_m)] += f64::from(p.complete);
                m[pi(ri, P::sep_m)] += p.separation;
            } else {
                m[pi(ri, P::tgt_z)] += 1.0;
                m[pi(ri, P::sep_z)] += p.separation;
            }
        }
        if run
            && p.yards >= 20.0
            && let Some(ri) = p.carrier.as_ref().and_then(ro)
        {
            m[pi(ri, P::c20)] += 1.0;
        }
    } else {
        if targeted {
            add(m, T::d_tgt, 1.0);
            if matches!(pos(p.pbu), Some(Position::S | Position::CB | Position::LB)) {
                add(m, T::d_pbu_tgt, 1.0);
            }
            if p.air_yards >= 30.0 {
                add(m, T::d_att30, 1.0);
                add(m, T::d_int30, f64::from(p.interception));
            }
            if p.coverage_man {
                add(m, T::d_att_man, 1.0);
                add(m, T::d_int_man, f64::from(p.interception));
                add(m, T::d_int_s_man, f64::from(p.interception && pos(p.interceptor) == Some(Position::S)));
            }
        }
        if p.interception
            && let Some(id) = p.interceptor
            && matches!(pos(Some(id)), Some(Position::EDGE | Position::DT))
            && p.blocks.iter().any(|x| x.1 == id)
        {
            add(m, T::d_int_dl_blk, 1.0);
        }
        // the watched men
        if run
            && let Some(d) = first
            && let Some(ri) = ro(&d)
        {
            m[pi(ri, P::ftk)] += 1.0;
            m[pi(ri, P::ftk_blk)] += f64::from(first_blk);
        }
    }
}

// ------------------------------------------------------------------ an arm

/// Who an arm changed: each watched man after his change.
#[derive(Clone, Debug)]
pub struct Man {
    pub role: &'static str,
    pub weight: f64,
    pub height: f64,
    pub forty: f64,
    pub overall: f64,
    /// An as-recruited draw found a man in its band (false: none was found,
    /// and the typical man played).
    pub drawn: bool,
}

/// Team 0 for an arm, and the men it changed.
pub fn arm_team(base: &[Squad], b: &Base, arm: &Arm, level: Level) -> (GameTeam, Vec<Man>) {
    let mut t0 = b.team.clone();
    apply_ctx(&mut t0, arm.ctx);
    let mut men = Vec::new();
    for &ri in &arm.who {
        let k = b.at[ri];
        let before = t0.players[k].attrs;
        change_one(&mut t0.players[k], &base[0].people[k], &arm.knobs, level);
        let gp = &t0.players[k];
        let recruit = arm.knobs.iter().any(|k| matches!(k, Knob::Recruit(..)));
        men.push(Man {
            role: ROLES[ri],
            weight: gp.weight,
            height: gp.height,
            forty: gp.forty(),
            overall: overall_for(gp.position, &gp.attrs),
            drawn: !recruit || Attr::ALL.iter().any(|&a| gp.attrs.get(a) != before.get(a)),
        });
    }
    (t0, men)
}

/// Plays an arm's games and returns its block sums (`NB` blocks of
/// `width()` floats, row by row).
pub fn run_arm(base: &[Squad], b: &Base, arm: &Arm, level: Level, games: usize, seed: u32) -> (Vec<f64>, Vec<Man>) {
    let (t0, men) = arm_team(base, b, arm, level);
    let mut teams: Vec<&GameTeam> = base.iter().map(|s| &s.team).collect();
    teams[0] = &t0;
    let role_of: HashMap<u32, usize> = b.at.iter().enumerate().map(|(ri, &k)| (t0.players[k].id, ri)).collect();
    let pos_of: HashMap<u32, Position> =
        teams.iter().flat_map(|t| t.players.iter().map(|p| (p.id, p.position))).collect();
    let cx = Games { teams, level, seed, role_of, pos_of };
    let w = width();
    let per: Vec<(usize, Vec<f64>)> = (0..games).into_par_iter().map(|g| (g, one(&cx, g))).collect();
    let mut blocks = vec![0.0f64; NB * w];
    for (g, m) in per {
        let bi = g * NB / games;
        for (x, y) in blocks[bi * w..(bi + 1) * w].iter_mut().zip(&m) {
            *x += y;
        }
    }
    (blocks, men)
}

// ------------------------------------------------------------------ a run

/// A run's settings, kept beside its arms (`schema.json`).
pub struct Setup {
    pub level: Level,
    pub games: usize,
    pub seed: u32,
}

pub fn level_name(level: Level) -> &'static str {
    match level {
        Level::NFL => "NFL",
        Level::HS => "HS",
        Level::FCS => "FCS",
        _ => "FBS",
    }
}

/// Plays the arms into `out`: each arm's block sums (`<id>.f32`), its line
/// in `arms.jsonl`, and the run's `schema.json`, `spread.csv` (each rating's
/// spread among the league's starters) and `typical.csv`. An arm already in
/// the folder is kept, so a stopped run picks up where it was; a folder
/// holding another run's settings is refused.
pub fn run(setup: &Setup, out: &Path, list: &[Arm]) -> Result<(), String> {
    let Setup { level, games, seed } = *setup;
    // Each block holds as many games, so a block's per-game measures and the
    // bands paired on them read every block alike (`read::eval`).
    if games < NB || games % NB != 0 {
        return Err(format!("games an arm must be a multiple of {NB} (one or more a block), not {games}"));
    }
    std::fs::create_dir_all(out).map_err(|e| format!("{}: {e}", out.display()))?;
    let (base, world) = league(level, seed);
    // Ids must be unique across the league, or a man is mistaken for another.
    {
        let mut seen = std::collections::HashSet::new();
        for s in &base {
            for p in &s.team.players {
                assert!(seen.insert(p.id), "id {} on two teams", p.id);
            }
        }
    }
    let b = base_team(&base[0], level);
    let schema = schema(setup, &b, world.as_deref());
    let sp = out.join("schema.json");
    if let Ok(old) = std::fs::read_to_string(&sp) {
        let old: Value = serde_json::from_str(&old).map_err(|e| format!("{}: {e}", sp.display()))?;
        for k in ["level", "seed", "games", "blocks", "world", "t", "p", "roles"] {
            if old[k] != schema[k] {
                return Err(format!("{} holds a run with another {k}: use a new folder", out.display()));
            }
        }
    }
    std::fs::write(&sp, schema.to_string()).map_err(|e| e.to_string())?;
    write_spread(&base, &out.join("spread.csv"));
    write_typical(level, &out.join("typical.csv"));
    eprintln!("{} arms, {games} games each, {} seed {seed}", list.len(), level_name(level));
    let t0 = Instant::now();
    let mut done = 0usize;
    for arm in list {
        let path = out.join(format!("{}.f32", arm.id));
        if path.exists() {
            continue;
        }
        let t1 = Instant::now();
        let (blocks, men) = run_arm(&base, &b, arm, level, games, seed);
        let bytes: Vec<u8> = blocks.iter().flat_map(|&x| (x as f32).to_le_bytes()).collect();
        let tmp = out.join(format!("{}.tmp", arm.id));
        std::fs::write(&tmp, bytes).map_err(|e| e.to_string())?;
        // Its line first, then its sums: a run stopped between the two
        // plays the arm again and `read` keeps one of its lines.
        let mut f = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(out.join("arms.jsonl"))
            .map_err(|e| e.to_string())?;
        let band = arm.knobs.iter().find_map(|k| match k {
            Knob::Recruit(part, lo, hi) => Some(json!({"part": format!("{part:?}"), "lo": lo, "hi": hi})),
            _ => None,
        });
        let line = json!({
            "id": arm.id,
            "ctx": arm.ctx.name(),
            "who": arm.who.iter().map(|&ri| ROLES[ri]).collect::<Vec<_>>(),
            "label": arm.label,
            "games": games,
            "band": band,
            "men": men.iter().map(|m| json!({
                "role": m.role, "weight": m.weight, "height": m.height, "forty": m.forty,
                "overall": m.overall, "drawn": m.drawn,
            })).collect::<Vec<_>>(),
        });
        writeln!(f, "{line}").map_err(|e| e.to_string())?;
        std::fs::rename(&tmp, &path).map_err(|e| e.to_string())?;
        done += 1;
        let el = t0.elapsed().as_secs_f64();
        eprintln!(
            "{} [{:.0} s, {done}/{} in {:.0} min, eta {:.0} min]",
            arm.id,
            t1.elapsed().as_secs_f64(),
            list.len(),
            el / 60.0,
            el / done as f64 * (list.len() - done) as f64 / 60.0
        );
    }
    eprintln!("done [{:.0} s]", t0.elapsed().as_secs_f64());
    Ok(())
}

/// The run's settings, its slots, and who each watched man is.
fn schema(setup: &Setup, b: &Base, world: Option<&str>) -> Value {
    let who: Vec<Value> =
        b.at.iter()
            .enumerate()
            .map(|(ri, &k)| {
                let p = &b.team.players[k];
                json!({
                    "role": ROLES[ri],
                    "id": p.id,
                    "pos": format!("{:?}", p.position),
                    "spot": format!("{:?}", p.spot),
                    "ov": (overall_for(p.position, &p.attrs) * 10.0).round() / 10.0,
                    "h": (p.height * 10.0).round() / 10.0,
                    "w": p.weight.round(),
                })
            })
            .collect();
    json!({
        "tool": "fit_check",
        "level": level_name(setup.level),
        "seed": setup.seed,
        "games": setup.games,
        "blocks": NB,
        "world": world,
        "t": T_NAMES,
        "p": P_NAMES,
        "roles": ROLES,
        "who": who,
    })
}

/// The spread of each rating among the league's starters (teams 1-31 and
/// team 0 before it is set typical): the yardstick for a +15.
fn write_spread(base: &[Squad], path: &Path) {
    let mut s = String::from("pos,attr,n,mean,sd\n");
    for pos in Position::ALL {
        let mut vals: Vec<Vec<f64>> = vec![Vec::new(); Attr::ALL.len()];
        for sq in base {
            for &k in ranked(&sq.team, pos).iter().take(starters(pos)) {
                for (i, a) in Attr::ALL.iter().enumerate() {
                    vals[i].push(sq.team.players[k].attrs.get(*a));
                }
            }
        }
        for (i, a) in Attr::ALL.iter().enumerate() {
            let v = &vals[i];
            let n = v.len().max(1) as f64;
            let mean = v.iter().sum::<f64>() / n;
            let sd = (v.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (n - 1.0).max(1.0)).sqrt();
            s.push_str(&format!("{pos:?},{a:?},{},{mean:.2},{sd:.2}\n", v.len()));
        }
    }
    std::fs::write(path, s).expect("write spread.csv");
}

/// The level's typical starter and typical player, rating by rating.
fn write_typical(level: Level, path: &Path) {
    let mut s = String::from("pos,attr,typical_starter,typical\n");
    for pos in Position::ALL {
        for a in Attr::ALL {
            s.push_str(&format!("{pos:?},{a:?},{:.2},{:.2}\n", typical_starter(level, pos, a), typical(level, pos, a)));
        }
    }
    std::fs::write(path, s).expect("write typical.csv");
}
