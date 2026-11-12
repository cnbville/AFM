//! Every position's tests (`afm/ab-tests/plan.md`, "Every position").
//!
//! Body values at the NFL are the plan's: about 1.5 spreads under and over
//! the position's real combine mean (`data/real-world/combine_by_position.csv`);
//! the same inches and pounds are used at every level, so a 6'2 corner is
//! a 6'2 corner against high school receivers too. Ratings move 15 points;
//! tendencies go to 4 and 16. The exact numbers are Claude's picks,
//! proposed, not confirmed.

use afm_engine::attributes::Attr;
use afm_engine::links::Link;
use afm_engine::positions::Position;
use afm_engine::spots::Spot;
use afm_engine::tendencies::Tend;

use super::{Knob, Part, Who};

#[derive(Clone, Debug)]
pub enum Kind {
    /// A body number or drill: low, typical, high (pure), and optionally
    /// the generator's players split by it, and crossed with a habit.
    Body { part: Part, v: [f64; 3], recruit: bool, cross: Option<Tend> },
    /// Ratings moved together, 15 points down and up.
    Rating(Vec<Attr>),
    /// A tendency at 4 and 16.
    Tend(Tend),
    /// His hidden consistency at 4 and 16.
    Consistency,
    /// A rating crossed with a habit (2 × 2).
    RatingTend(Attr, Tend),
    /// A matchup grid: his body number against the man across from him (3 × 3).
    Grid { part: Part, v: [f64; 3], other: Who, opart: Part, ov: [f64; 3] },
    /// A Dynamic Link off and full.
    Link(Link),
    /// One team's man built for job `a` and man built for job `b`, named
    /// right ("lo") and swapped ("hi") on the board (linebacker L1: "if i
    /// put a will in sam the results reflect that?"). `knows`: both have
    /// played both jobs, so only the bodies and leanings differ.
    Swap { a: Spot, b: Spot, ka: Vec<Knob>, kb: Vec<Knob>, knows: bool },
}

#[derive(Clone, Debug)]
pub struct Test {
    pub id: &'static str,
    /// The report it belongs to.
    pub group: &'static str,
    pub name: &'static str,
    pub who: Who,
    pub kind: Kind,
    /// The stats from the snap to the result, the headline first.
    pub chain: Vec<&'static str>,
    /// (stat, real key in `ab_real.json`): a key with "__" is a slope, the
    /// rest are spreads across real starters.
    pub real: Vec<(&'static str, &'static str)>,
    /// The expected sign of high minus low on the headline (0: no call).
    pub sign: f64,
    /// The position's headline row: world rosters, the my team view, CI.
    pub headline: bool,
    /// What step 0 found the engine does not read (a gap by construction).
    pub note: &'static str,
    /// The test runs only in the my team view (returners: one team's
    /// returners against everyone's usual coverage).
    pub mine: bool,
}

/// A linebacker body test's rows against every real linebacker
/// (`afm/ab-tests/research-linebackers.md`).
const LB_BODY: &[&str] = &[
    "tk_run_snap",
    "tk_run_stop",
    "tk_run_long",
    "tk_rate",
    "tk_miss",
    "cv_tgt_snap",
    "cv_cmp",
    "cv_ypt",
    "cv_yac",
    "cv_adot",
    "dl_press_snap",
    "dl_sack_snap",
    "def_ypc",
    "def_out_ypc",
    "def_run_succ",
    "def_cmp",
    "def_epa_db",
    "def_blitz",
];
/// The same rows for each job, Mike, Sam and Will, with the body of the
/// man who plays it (`ab::lb_jobs`). The real jobs are in the research.
const LB_JOBS: &[&str] = &[
    "lj_mike_share",
    "lj_mike_rtk",
    "lj_mike_stop",
    "lj_mike_miss",
    "lj_mike_tgt",
    "lj_mike_cmp",
    "lj_mike_adot",
    "lj_mike_prs",
    "lj_mike_wt",
    "lj_mike_forty",
    "lj_sam_share",
    "lj_sam_rtk",
    "lj_sam_stop",
    "lj_sam_miss",
    "lj_sam_tgt",
    "lj_sam_cmp",
    "lj_sam_adot",
    "lj_sam_prs",
    "lj_sam_wt",
    "lj_sam_forty",
    "lj_will_share",
    "lj_will_rtk",
    "lj_will_stop",
    "lj_will_miss",
    "lj_will_tgt",
    "lj_will_cmp",
    "lj_will_adot",
    "lj_will_prs",
    "lj_will_wt",
    "lj_will_forty",
];
macro_rules! lb_real {
    ($x:literal) => {
        &[
            ("tk_run_snap", concat!("lb_", $x, "__rtk_per_snap")),
            ("tk_run_stop", concat!("lb_", $x, "__rstop_per_snap")),
            ("tk_run_long", concat!("lb_", $x, "__rlong_share")),
            ("tk_rate", concat!("lb_", $x, "__tkl_per_snap")),
            ("tk_miss", concat!("lb_", $x, "__mtkl_pct")),
            ("cv_tgt_snap", concat!("lb_", $x, "__tgt_per_snap")),
            ("cv_cmp", concat!("lb_", $x, "__cmp_allowed")),
            ("cv_ypt", concat!("lb_", $x, "__yds_per_tgt")),
            ("cv_yac", concat!("lb_", $x, "__yac_per_cmp")),
            ("cv_adot", concat!("lb_", $x, "__dadot")),
            ("dl_press_snap", concat!("lb_", $x, "__prss_per_snap")),
            ("dl_sack_snap", concat!("lb_", $x, "__sack_per_snap")),
        ]
    };
}

/// An edge rusher body, drill or bend test's rows against every real edge
/// (`afm/ab-tests/research-edge.md`): how he wins, how a pressure ends,
/// pressure by how fast the ball comes out, run support, flags, the team.
const ED_BODY: &[&str] = &[
    "dl_press_snap",
    "dl_sack_snap",
    "dl_hurry_snap",
    "dl_sack_prs",
    "dl_press",
    "dl_hit",
    "dl_sack",
    "dl_win",
    "dl_win_t",
    "dl_speed_win",
    "dl_power_win",
    "dl_finesse_win",
    "dl_contact",
    "dl_shed",
    "dl_press_quick",
    "dl_press_long",
    "dl_bat",
    "tk_run_snap",
    "tk_run_stop",
    "tk_rate",
    "tk_miss",
    "dl_tkl_run",
    "dl_stop_run",
    "dl_tfl",
    "dl_run_shed",
    "dl_off",
    "dl_rtp",
    "def_press",
    "def_sack",
    "def_ypc",
    "def_out_ypc",
    "def_run_succ",
    "def_epa_db",
    "def_scr_yds",
];
macro_rules! ed_real {
    ($x:literal) => {
        &[
            ("dl_press_snap", concat!("edge_", $x, "__prss_per_snap")),
            ("dl_sack_snap", concat!("edge_", $x, "__sack_per_snap")),
            ("dl_hurry_snap", concat!("edge_", $x, "__hrry_per_snap")),
            ("dl_sack_prs", concat!("edge_", $x, "__sk_per_prss")),
            ("tk_run_snap", concat!("edge_", $x, "__rtk_per_snap")),
            ("tk_run_stop", concat!("edge_", $x, "__rstop_per_snap")),
            ("tk_rate", concat!("edge_", $x, "__tkl_per_snap")),
            ("tk_miss", concat!("edge_", $x, "__mtkl_pct")),
            ("dl_off", concat!("edge_", $x, "__off_per_snap")),
            ("dl_rtp", concat!("edge_", $x, "__rtp_per_snap")),
        ]
    };
}

fn t(id: &'static str, group: &'static str, name: &'static str, pos: &[Position], kind: Kind) -> Test {
    Test {
        id,
        group,
        name,
        who: Who::at(pos),
        kind,
        chain: Vec::new(),
        real: Vec::new(),
        sign: 0.0,
        headline: false,
        note: "",
        mine: false,
    }
}

impl Test {
    /// Adds rows (each once; a second call adds to the first).
    fn chain(mut self, c: &[&'static str]) -> Test {
        for k in c {
            if !self.chain.contains(k) {
                self.chain.push(k);
            }
        }
        self
    }
    /// Adds real checks (each row once).
    fn real(mut self, r: &[(&'static str, &'static str)]) -> Test {
        for e in r {
            if !self.real.iter().any(|x| x.0 == e.0) {
                self.real.push(*e);
            }
        }
        self
    }
    fn sign(mut self, s: f64) -> Test {
        self.sign = s;
        self
    }
    fn headline(mut self) -> Test {
        self.headline = true;
        self
    }
    fn note(mut self, n: &'static str) -> Test {
        self.note = n;
        self
    }
    fn mine(mut self) -> Test {
        self.mine = true;
        self
    }
    fn spot(mut self, s: Spot) -> Test {
        self.who.spot = Some(s);
        self
    }
}

/// The team rows for the job swap.
const LB_SWAP: &[&str] =
    &["def_run_succ", "def_ypc", "def_out_ypc", "def_cmp", "def_epa_db", "cv_te_cmp", "def_bust", "lb_blitz_press"];

/// A Sam's build against a Will's: 248 lb and 4.68 against 228 lb and 4.56,
/// with Strength and Block Shedding 6 up and Zone Coverage 6 down, and the
/// other way round (Claude's picks, proposed, not confirmed).
fn swap(knows: bool) -> Kind {
    use Attr as A;
    let lean = |d: f64| {
        vec![Knob::Rating(A::Strength, d), Knob::Rating(A::BlockShedding, d), Knob::Rating(A::ZoneCoverage, -d)]
    };
    let mut ka = vec![Knob::Set(Part::Weight, 248.0), Knob::Set(Part::Forty, 4.68)];
    ka.extend(lean(6.0));
    let mut kb = vec![Knob::Set(Part::Weight, 228.0), Knob::Set(Part::Forty, 4.56)];
    kb.extend(lean(-6.0));
    Kind::Swap { a: Spot::SAM, b: Spot::OLB, ka, kb, knows }
}

fn body(part: Part, v: [f64; 3]) -> Kind {
    Kind::Body { part, v, recruit: false, cross: None }
}
fn drawn(part: Part, v: [f64; 3]) -> Kind {
    Kind::Body { part, v, recruit: true, cross: None }
}
fn r(a: &[Attr]) -> Kind {
    Kind::Rating(a.to_vec())
}

pub fn all() -> Vec<Test> {
    use Attr as A;
    use Part::*;
    use Position as P;
    let qb = &[P::QB];
    let rb = &[P::RB];
    let wr = &[P::WR];
    let te = &[P::TE];
    let ot = &[P::OT];
    let iol = &[P::IOL];
    let dt = &[P::DT];
    let ed = &[P::EDGE];
    let lb = &[P::LB];
    let cb = &[P::CB];
    let sf = &[P::S];
    let every = &[P::QB, P::RB, P::WR, P::TE, P::OT, P::IOL, P::EDGE, P::DT, P::LB, P::CB, P::S, P::K, P::P, P::LS];
    let ret = &[P::WR, P::RB, P::CB];
    let rush = ["dl_contact", "dl_shed", "dl_win", "dl_win_t", "dl_press", "dl_hit", "dl_sack", "def_sack"];
    let qb_reads = ["qb_ttt", "qb_check", "qb_away", "qb_int", "qb_sack", "qb_epa"];
    let mut v = vec![
        // ---------------------------------------------------- quarterback
        t(
            "qb_speed",
            "qb",
            "Speed (forty 5.05 / 4.82 / 4.57), and × the Scramble habit at 4 and 16",
            qb,
            Kind::Body { part: Forty, v: [5.05, 4.82, 4.57], recruit: true, cross: Some(Tend::Scramble) },
        )
        .chain(&["qb_scr", "qb_scr_yds", "qb_runs", "qb_spy", "qb_press", "qb_sack_press", "qb_sack", "qb_epa"])
        .real(&[
            ("qb_scr", "qb_forty__scramble_rate"),
            ("qb_scr_yds", "qb_forty__yds_per_scramble"),
            ("qb_sack", "qb_forty__sack_rate"),
        ])
        .sign(1.0)
        .headline()
        .note("the sack escape reads Elusiveness, Agility and Strength, not Speed"),
        t("qb_height", "qb", "Height (6'0 / 6'3 / 6'5)", qb, drawn(Height, [72.0, 75.0, 77.0]))
            .chain(&["qb_batted", "qb_mid_field", "qb_cmp", "qb_ypa", "qb_sack", "qb_ttt"])
            .real(&[("qb_batted", "qb_ht__batted_per_att"), ("qb_mid_field", "qb_ht__mid_cmp_pct")])
            .sign(-1.0)
            .note("seeing over the line is not read; height counts only in batted balls and the Pocket frame link"),
        t("qb_weight", "qb", "Weight (205 / 222 / 240 lb)", qb, body(Weight, [205.0, 222.0, 240.0]))
            .chain(&["qb_sack_press", "qb_scr", "qb_scr_yds", "qb_fum", "qb_inj"])
            .sign(-1.0),
        t("qb_arm", "qb", "Arm Strength", qb, r(&[A::ArmStrength]))
            .chain(&["qb_air", "qb_deep", "qb_deep_cmp", "qb_cmp20", "qb_deep_int", "qb_ypa"])
            .real(&[("qb_air", "qb_adot"), ("qb_deep", "qb_deep_share")])
            .sign(1.0),
        t("qb_short_acc", "qb", "Short Accuracy", qb, r(&[A::ShortAccuracy]))
            .chain(&["qb_short_cmp", "qb_cmp", "qb_int", "qb_ypa"])
            .real(&[("qb_cmp", "qb_cpoe")])
            .sign(1.0),
        t("qb_mid_acc", "qb", "Medium Accuracy", qb, r(&[A::MediumAccuracy]))
            .chain(&["qb_mid_cmp", "qb_cmp", "qb_int", "qb_ypa"])
            .real(&[("qb_cmp", "qb_cpoe")])
            .sign(1.0),
        t("qb_deep_acc", "qb", "Deep Accuracy", qb, r(&[A::DeepAccuracy]))
            .chain(&["qb_deep_cmp", "qb_cmp20", "qb_deep_int", "qb_ypa"])
            .real(&[("qb_cmp", "qb_cpoe")])
            .sign(1.0),
        t("qb_decisions", "qb", "Decisions", qb, r(&[A::Decisions]))
            .chain(&qb_reads)
            .real(&[("qb_int", "qb_int_rate"), ("qb_sack", "qb_sack_rate"), ("qb_ttt", "qb_ttt")]),
        t("qb_recognition", "qb", "Play Recognition", qb, r(&[A::PlayRecognition]))
            .chain(&qb_reads)
            .real(&[("qb_int", "qb_int_rate"), ("qb_sack", "qb_sack_rate"), ("qb_ttt", "qb_ttt")]),
        t("qb_iq", "qb", "Football IQ", qb, r(&[A::FootballIq]))
            .chain(&qb_reads)
            .real(&[("qb_int", "qb_int_rate"), ("qb_sack", "qb_sack_rate"), ("qb_ttt", "qb_ttt")]),
        t("qb_pocket", "qb", "Pocket Presence", qb, r(&[A::PocketPresence]))
            .chain(&["qb_sack_press", "qb_sack", "qb_ttt", "qb_scr"])
            .real(&[("qb_sack_press", "qb_sacks_per_pressure"), ("qb_sack", "qb_sack_rate")])
            .sign(-1.0),
        t("qb_release", "qb", "Quick Release", qb, r(&[A::QuickRelease]))
            .chain(&["qb_ttt", "qb_sack", "qb_batted", "qb_cmp"])
            .real(&[("qb_ttt", "qb_ttt")])
            .sign(-1.0),
        t("qb_footwork", "qb", "Footwork", qb, r(&[A::Footwork]))
            .chain(&["qb_ttt", "qb_sack", "qb_cmp", "qb_ypa"])
            .real(&[("qb_ttt", "qb_ttt")]),
        t("qb_composure", "qb", "Composure", qb, r(&[A::Composure]))
            .chain(&["qb_press_cmp", "qb_clean_cmp", "qb_press_int", "qb_late_cmp", "qb_int"])
            .sign(1.0),
        t("qb_on_move", "qb", "Throw on the Move", qb, r(&[A::ThrowOnMove]))
            .chain(&["qb_move_cmp", "qb_cmp", "qb_ypa"])
            .sign(1.0),
        t("qb_eyes", "qb", "Eye Manipulation", qb, r(&[A::EyeManipulation]))
            .chain(&["qb_deep_cmp", "qb_cmp20", "qb_int", "qb_ypa"])
            .sign(1.0),
        t("qb_handling", "qb", "Ball Handling", qb, r(&[A::BallHandling]))
            .chain(&["qb_pa_ypa", "off_ypc", "qb_fum"])
            .sign(1.0),
        t("qb_gun_dec", "qb", "Gunslinger habit × Decisions", qb, Kind::RatingTend(A::Decisions, Tend::Gunslinger))
            .chain(&["qb_int", "qb_deep", "qb_air", "qb_ypa", "qb_epa"])
            .real(&[("qb_int", "qb_int_rate")]),
        t("qb_gun_arm", "qb", "Gunslinger habit × Arm Strength", qb, Kind::RatingTend(A::ArmStrength, Tend::Gunslinger))
            .chain(&["qb_int", "qb_deep", "qb_air", "qb_ypa", "qb_epa"]),
        t("qb_hold_dec", "qb", "Hold habit × Decisions", qb, Kind::RatingTend(A::Decisions, Tend::Hold))
            .chain(&["qb_ttt", "qb_sack", "qb_cmp20", "qb_epa"]),
        t("qb_hold_arm", "qb", "Hold habit × Arm Strength", qb, Kind::RatingTend(A::ArmStrength, Tend::Hold))
            .chain(&["qb_ttt", "qb_sack", "qb_cmp20", "qb_epa"]),
        // ---------------------------------------------------- running back
        t("rb_weight", "rb", "Weight (192 / 217 / 242 lb)", rb, drawn(Weight, [192.0, 217.0, 242.0]))
            .chain(&["rb_yaco", "rb_mtf", "rb_ybc", "rb_short", "rb_20", "rb_ypc", "rb_fum", "rb_inj", "rb_epa"])
            .real(&[
                ("rb_yaco", "rb_wt__yaco_per_att"),
                ("rb_mtf", "rb_wt__brk_tkl_per_att"),
                ("rb_ybc", "rb_wt__ybc_per_att"),
                ("rb_short", "rb_wt__short_yd_conv"),
                ("rb_20", "rb_wt__run20_rate"),
                ("rb_fum", "rb_wt__fumble_per_touch"),
            ])
            .sign(1.0)
            .headline(),
        t(
            "rb_speed",
            "rb",
            "Speed (forty 4.78 / 4.58 / 4.38), and × the Bounce habit",
            rb,
            Kind::Body { part: Forty, v: [4.78, 4.58, 4.38], recruit: true, cross: Some(Tend::Bounce) },
        )
        .chain(&["rb_20", "rb_ybc", "rb_out", "rb_ypc", "rb_stuff", "rb_epa"])
        .real(&[
            ("rb_20", "rb_forty__run20_rate"),
            ("rb_ybc", "rb_forty__ybc_per_att"),
            ("rb_ypc", "rb_forty__ryoe_per_att"),
        ])
        .sign(1.0),
        t("rb_cone", "rb", "Acceleration and agility (3-cone 7.45 / 7.11 / 6.75)", rb, body(Cone, [7.45, 7.11, 6.75]))
            .chain(&["rb_ybc", "rb_mtf", "rb_ypc", "rb_stuff"])
            .real(&[("rb_ybc", "rb_cone__ybc_per_att"), ("rb_mtf", "rb_cone__brk_tkl_per_att")])
            .sign(1.0),
        t("rb_height", "rb", "Height (5'8 / 5'11 / 6'1)", rb, body(Height, [68.0, 71.0, 73.0]))
            .chain(&["rb_yaco", "rb_mtf", "rb_ypc", "rb_short"]),
        t("rb_vision", "rb", "Vision", rb, r(&[A::BallCarrierVision]))
            .chain(&["rb_ypc", "rb_out", "rb_ybc", "rb_20", "rb_stuff"])
            .real(&[("rb_ypc", "rb_ryoe_per_att")])
            .sign(1.0),
        t("rb_elusive", "rb", "Elusiveness", rb, r(&[A::Elusiveness]))
            .chain(&["rb_mtf", "rb_yaco", "rb_ypc"])
            .real(&[("rb_mtf", "rb_brk_tkl_per_att"), ("rb_yaco", "rb_yaco_per_att")])
            .sign(1.0),
        t("rb_strength", "rb", "Strength", rb, r(&[A::Strength]))
            .chain(&["rb_yaco", "rb_mtf", "rb_short", "rb_ypc"])
            .real(&[("rb_mtf", "rb_brk_tkl_per_att"), ("rb_yaco", "rb_yaco_per_att")])
            .sign(1.0),
        t("rb_legdrive", "rb", "Leg Drive", rb, r(&[A::LegDrive]))
            .chain(&["rb_yaco", "rb_mtf", "rb_short", "rb_ypc"])
            .real(&[("rb_mtf", "rb_brk_tkl_per_att"), ("rb_yaco", "rb_yaco_per_att")])
            .sign(1.0),
        t("rb_balance", "rb", "Balance", rb, r(&[A::Balance]))
            .chain(&["rb_mtf", "rb_yaco", "rb_ypc"])
            .real(&[("rb_mtf", "rb_brk_tkl_per_att"), ("rb_yaco", "rb_yaco_per_att")])
            .sign(1.0),
        t("rb_security", "rb", "Ball Security", rb, r(&[A::BallSecurity]))
            .chain(&["rb_fum"])
            .real(&[("rb_fum", "rb_fumble_per_touch")])
            .sign(-1.0),
        t("rb_passpro", "rb", "Pass Blocking", rb, r(&[A::PassBlocking]))
            .chain(&["rb_pp_lost", "blk_press", "off_sack"])
            .sign(-1.0),
        t("rb_iq", "rb", "Football IQ (blitz pickup)", rb, r(&[A::FootballIq]))
            .chain(&["rb_pp_lost", "blk_press", "off_sack", "off_bust"]),
        t("rb_patience_speed", "rb", "Patience habit × speed", rb, Kind::RatingTend(A::Speed, Tend::Patience))
            .chain(&["rb_ypc", "rb_20", "rb_stuff"]),
        t("rb_contact_speed", "rb", "Contact habit × speed", rb, Kind::RatingTend(A::Speed, Tend::Contact))
            .chain(&["rb_yaco", "rb_ypc", "rb_inj"]),
        t("fb_weight", "rb", "Fullback weight (228 / 245 / 262 lb)", rb, body(Weight, [228.0, 245.0, 262.0]))
            .spot(Spot::FB)
            .chain(&["fb_ypc", "blk_run_won", "off_ypc"])
            .sign(1.0),
        t("fb_runblock", "rb", "Fullback Run Blocking", rb, r(&[A::RunBlocking]))
            .spot(Spot::FB)
            .chain(&["fb_ypc", "blk_run_won", "off_ypc"])
            .sign(1.0),
        t("fb_space", "rb", "Fullback Blocking in Space", rb, r(&[A::BlockingInSpace]))
            .spot(Spot::FB)
            .chain(&["fb_ypc", "blk_run_won", "off_ypc"])
            .sign(1.0),
        // ---------------------------------------------------- wide receiver
        t("wr_height", "wr", "Height (5'9.5 / 6'1 / 6'4)", wr, drawn(Height, [69.5, 73.0, 76.0]))
            .chain(&[
                "rc_cont",
                "rc_tight",
                "rc_adot",
                "rc_deep_share",
                "rc_rz",
                "rc_rz_share",
                "rc_deep",
                "rc_catch",
                "rc_sep",
                "rc_ypt",
                "rc_int",
            ])
            .real(&[
                ("rc_rz", "wr_ht__rz_td_per_rz_tgt"),
                ("rc_deep", "wr_ht__deep_catch_rate"),
                ("rc_catch", "wr_ht__catch_rate"),
            ])
            .sign(1.0)
            .headline()
            .note("height is not read in the route or separation, only at the catch point"),
        t("wr_weight", "wr", "Weight (180 / 201 / 224 lb)", wr, body(Weight, [180.0, 201.0, 224.0]))
            .chain(&["rc_yac", "rc_mtf", "rc_sep", "rc_catch", "rc_cont"])
            .real(&[("rc_yac", "wr_wt__yac_per_rec"), ("rc_mtf", "wr_wt__brk_tkl_per_rec")])
            .sign(1.0),
        t("wr_speed", "wr", "Speed (forty 4.65 / 4.51 / 4.36)", wr, drawn(Forty, [4.65, 4.51, 4.36]))
            .chain(&["rc_adot", "rc_deep_share", "rc_sep", "rc_deep", "rc_yac", "rc_ypt"])
            .real(&[
                ("rc_adot", "wr_forty__adot"),
                ("rc_sep", "wr_forty__separation"),
                ("rc_deep_share", "wr_forty__deep_tgt_share"),
            ])
            .sign(1.0),
        t("wr_agility", "wr", "Agility", wr, r(&[A::Agility]))
            .chain(&["rc_sep", "rc_share", "rc_catch", "rc_ypt"])
            .real(&[("rc_sep", "wr_separation")])
            .sign(1.0),
        t("wr_route", "wr", "Route Running", wr, r(&[A::RouteRunning]))
            .chain(&["rc_sep", "rc_share", "rc_catch", "rc_ypt"])
            .real(&[("rc_sep", "wr_separation"), ("rc_catch", "wr_catch_rate")])
            .sign(1.0),
        t("wr_release", "wr", "Release", wr, r(&[A::Release]))
            .chain(&["rc_sep", "rc_share", "rc_catch"])
            .sign(1.0),
        t("wr_catching", "wr", "Catching", wr, r(&[A::Catching]))
            .chain(&["rc_drop", "rc_catch", "rc_cont"])
            .real(&[("rc_drop", "wr_drop_pct"), ("rc_catch", "wr_catch_rate")])
            .sign(-1.0),
        t("wr_contested", "wr", "Contested Catch", wr, r(&[A::ContestedCatch]))
            .chain(&["rc_cont", "rc_deep", "rc_catch", "rc_int"])
            .sign(1.0),
        t("wr_jumping", "wr", "Jumping", wr, r(&[A::Jumping]))
            .chain(&["rc_cont", "rc_deep", "rc_rz", "rc_catch"])
            .sign(1.0),
        t("wr_body", "wr", "Body Control", wr, r(&[A::BodyControl]))
            .chain(&["rc_cont", "rc_catch", "rc_deep"])
            .sign(1.0),
        t("wr_arms", "wr", "Arms (31 / 32.25 / 33.5 in)", wr, body(Arm, [31.0, 32.25, 33.5]))
            .chain(&["rc_cont", "rc_catch", "rc_deep"])
            .real(&[("rc_catch", "wr_arm__catch_rate"), ("rc_deep", "wr_arm__deep_catch_rate")])
            .sign(1.0),
        t("wr_tracking", "wr", "Ball Tracking", wr, r(&[A::BallTracking]))
            .chain(&["rc_deep", "rc_catch"])
            .sign(1.0),
        t("wr_contact", "wr", "Contact habit (4 / 16)", wr, Kind::Tend(Tend::Contact))
            .chain(&["rc_middle", "rc_catch", "rc_yac"])
            .sign(1.0),
        // ---------------------------------------------------- tight end
        t("te_height", "te", "Height (6'2 / 6'4.5 / 6'6.5)", te, drawn(Height, [74.0, 76.5, 78.5]))
            .chain(&["rc_cont", "rc_rz", "rc_rz_share", "rc_catch", "rc_ypt"])
            .real(&[("rc_rz", "te_ht__rz_td_per_rz_tgt"), ("rc_catch", "te_ht__catch_rate")])
            .sign(1.0)
            .headline(),
        t("te_weight", "te", "Weight (238 / 253 / 268 lb)", te, body(Weight, [238.0, 253.0, 268.0]))
            .chain(&["blk_run_won", "blk_pass_lost", "blk_press", "rc_yac", "rc_sep"])
            .sign(1.0),
        t("te_speed", "te", "Speed (forty 4.97 / 4.76 / 4.55)", te, body(Forty, [4.97, 4.76, 4.55]))
            .chain(&["rc_sep", "rc_adot", "rc_deep_share", "rc_yac"])
            .real(&[("rc_sep", "te_forty__separation"), ("rc_adot", "te_forty__adot")])
            .sign(1.0),
        t("te_runblock", "te", "Run Blocking", te, r(&[A::RunBlocking]))
            .chain(&["blk_run_won", "off_ypc"])
            .sign(1.0),
        t("te_passblock", "te", "Pass Blocking", te, r(&[A::PassBlocking]))
            .chain(&["blk_pass_lost", "blk_press", "off_sack"])
            .sign(-1.0),
        t("te_space", "te", "Blocking in Space", te, r(&[A::BlockingInSpace]))
            .chain(&["blk_run_won", "off_ypc"])
            .sign(1.0),
        t("te_route", "te", "Route Running", te, r(&[A::RouteRunning]))
            .chain(&["rc_sep", "rc_share", "rc_catch"])
            .real(&[("rc_sep", "te_separation"), ("rc_catch", "te_catch_rate")])
            .sign(1.0),
        t("te_catching", "te", "Catching", te, r(&[A::Catching]))
            .chain(&["rc_drop", "rc_catch"])
            .real(&[("rc_catch", "te_catch_rate")])
            .sign(-1.0),
        // ---------------------------------------------------- offensive tackle
        t("ot_arm", "ot", "Arms (32.5 / 34 / 35.5 in)", ot, drawn(Arm, [32.5, 34.0, 35.5]))
            .chain(&["blk_press", "blk_pass_lost", "blk_sack", "blk_held", "ol_hold", "off_sack"])
            .sign(-1.0)
            .headline()
            .note("arm inches are not read in the hand fight, only through the Length link (nothing below about the top 30%)"),
        t("ot_weight", "ot", "Weight (295 / 316 / 336 lb)", ot, body(Weight, [295.0, 316.0, 336.0]))
            .chain(&["blk_lost_power", "blk_lost_speed", "blk_pass_lost", "blk_press", "blk_run_won", "off_ypc"])
            .sign(-1.0),
        t("ot_feet", "ot", "Agility and footwork (shuttle 5.03 / 4.75 / 4.47)", ot, body(Shuttle, [5.03, 4.75, 4.47]))
            .chain(&["blk_lost_speed", "blk_pass_lost", "blk_press", "blk_held"])
            .sign(-1.0),
        t("ot_passblock", "ot", "Pass Blocking", ot, r(&[A::PassBlocking]))
            .chain(&["blk_pass_lost", "blk_press", "blk_lost_speed", "blk_lost_power", "off_sack"])
            .real(&[("off_sack", "team_sack_rate"), ("off_press", "team_pressure_rate")])
            .sign(-1.0),
        t("ot_hands", "ot", "Hand Use", ot, r(&[A::HandUse]))
            .chain(&["blk_pass_lost", "blk_press", "off_sack", "off_press"])
            .real(&[("off_sack", "team_sack_rate"), ("off_press", "team_pressure_rate")])
            .sign(-1.0),
        t("ot_anchor", "ot", "Anchor", ot, r(&[A::Anchor]))
            .chain(&["blk_lost_power", "blk_pass_lost", "blk_press", "off_sack"])
            .real(&[("off_sack", "team_sack_rate")])
            .sign(-1.0),
        t("ot_strength", "ot", "Strength", ot, r(&[A::Strength]))
            .chain(&["blk_lost_power", "blk_pass_lost", "blk_run_won", "off_sack"])
            .real(&[("off_sack", "team_sack_rate")])
            .sign(-1.0),
        t("ot_height", "ot", "Height (6'4 / 6'5.5 / 6'7)", ot, body(Height, [76.0, 77.5, 79.0]))
            .chain(&["blk_pass_lost", "blk_press"])
            .note("a lineman's height is not read anywhere in the blocks"),
        t("ot_discipline", "ot", "Discipline", ot, r(&[A::Discipline]))
            .chain(&["ol_hold", "ol_fs"])
            .real(&[("ol_hold", "ol_holding_per_snap"), ("ol_fs", "ol_false_start_per_snap")])
            .sign(-1.0),
        // ---------------------------------------------------- interior line
        t("iol_weight", "iol", "Weight (290 / 311 / 331 lb)", iol, drawn(Weight, [290.0, 311.0, 331.0]))
            .chain(&["blk_run_won", "off_in_ypc", "blk_lost_power", "blk_pass_lost", "blk_press", "off_ypc"])
            .sign(1.0)
            .headline(),
        t("iol_runblock", "iol", "Run Blocking", iol, r(&[A::RunBlocking]))
            .chain(&["blk_run_won", "off_in_ypc", "off_ypc"])
            .sign(1.0),
        t("iol_strength", "iol", "Strength", iol, r(&[A::Strength]))
            .chain(&["blk_run_won", "blk_lost_power", "off_in_ypc"])
            .sign(1.0),
        t("iol_anchor", "iol", "Anchor", iol, r(&[A::Anchor]))
            .chain(&["blk_lost_power", "blk_pass_lost", "off_sack"])
            .sign(-1.0),
        t("iol_space", "iol", "Blocking in Space", iol, r(&[A::BlockingInSpace]))
            .chain(&["blk_run_won", "off_ypc", "off_in_ypc"])
            .sign(1.0),
        t("iol_agility", "iol", "Agility", iol, r(&[A::Agility]))
            .chain(&["blk_run_won", "blk_pass_lost", "off_ypc"]),
        t("c_snap", "iol", "Center: Shotgun Snap", iol, r(&[A::ShotgunSnap]))
            .spot(Spot::C)
            .chain(&["bad_snap", "qb_fum"])
            .sign(-1.0),
        t("c_comm", "iol", "Center: Communication", iol, r(&[A::Communication]))
            .spot(Spot::C)
            .chain(&["off_sack", "off_press", "off_bust"])
            .sign(-1.0)
            .note("stunts passed off are not in the play record; the team's pressure and sacks stand in"),
        t("iol_discipline", "iol", "Discipline", iol, r(&[A::Discipline]))
            .chain(&["ol_hold", "ol_fs"])
            .real(&[("ol_hold", "ol_holding_per_snap"), ("ol_fs", "ol_false_start_per_snap")])
            .sign(-1.0),
        t("iol_focus", "iol", "Concentration", iol, r(&[A::Concentration]))
            .chain(&["ol_fs_road", "ol_fs_home", "ol_fs"])
            .sign(-1.0),
        // ---------------------------------------------------- defensive tackle
        t("dt_weight", "dt", "Weight (284 / 307 / 330 lb)", dt, drawn(Weight, [284.0, 307.0, 330.0]))
            .chain(&["dl_run_shed", "dl_tfl", "def_ypc", "dl_shed", "dl_power_win", "dl_win", "dl_press", "dl_sack"])
            .real(&[("dl_press_snap", "dt_wt__prss_per_snap"), ("dl_sack_snap", "dt_wt__sack_per_snap")])
            .sign(-1.0)
            .headline(),
        t("dt_getoff", "dt", "Get-off (10-yard split 1.84 / 1.76 / 1.68)", dt, drawn(Split, [1.84, 1.76, 1.68]))
            .chain(&rush)
            .real(&[("dl_press_snap", "dt_split__prss_per_snap"), ("dl_sack_snap", "dt_split__sack_per_snap")])
            .sign(-1.0),
        t("dt_snap", "dt", "Snap timing habit (4 / 16)", dt, Kind::Tend(Tend::SnapTiming))
            .chain(&["dl_contact", "dl_off", "dl_win", "dl_press"])
            .sign(-1.0),
        t("dt_arm", "dt", "Arms (32 / 33.5 / 35 in)", dt, body(Arm, [32.0, 33.5, 35.0]))
            .chain(&["dl_bat", "dl_shed", "dl_run_shed", "dl_press", "dl_sack"])
            .real(&[("dl_press_snap", "dt_arm__prss_per_snap"), ("dl_sack_snap", "dt_arm__sack_per_snap")])
            .sign(1.0),
        t("dt_height", "dt", "Height (6'1 / 6'3 / 6'5)", dt, body(Height, [73.0, 75.0, 77.0]))
            .chain(&["dl_bat", "dl_shed", "dl_press"])
            .sign(1.0),
        t("dt_power", "dt", "Power Rush", dt, r(&[A::PowerRush]))
            .chain(&["dl_power_win", "dl_win", "dl_press", "dl_sack"])
            .real(&[("dl_press_snap", "dt_prss_per_snap"), ("dl_sack_snap", "dt_sack_per_snap")])
            .sign(1.0),
        t("dt_shed", "dt", "Block Shedding", dt, r(&[A::BlockShedding]))
            .chain(&["dl_run_shed", "dl_tfl", "def_ypc"])
            .sign(1.0),
        t("dt_strength", "dt", "Strength", dt, r(&[A::Strength]))
            .chain(&["dl_run_shed", "dl_power_win", "dl_press", "def_ypc"])
            .real(&[("dl_press_snap", "dt_prss_per_snap")])
            .sign(1.0),
        t("dt_hands", "dt", "Hand Use", dt, r(&[A::HandUse]))
            .chain(&["dl_shed", "dl_win", "dl_press", "dl_run_shed"])
            .real(&[("dl_press_snap", "dt_prss_per_snap")])
            .sign(1.0),
        t("dt_recognition", "dt", "Play Recognition", dt, r(&[A::PlayRecognition]))
            .chain(&["dl_run_shed", "dl_tfl", "def_ypc", "dl_press"])
            .sign(1.0),
        t("dt_motor", "dt", "Motor", dt, r(&[A::Motor]))
            .chain(&["dl_win_t", "dl_win", "dl_press", "tk_rate"])
            .sign(1.0),
        // ---------------------------------------------------- edge rusher
        t("edge_arm", "edge", "Arms (32.1 / 33.75 / 35.4 in)", ed, drawn(Arm, [32.1, 33.75, 35.4]))
            .chain(&[
                "dl_press",
                "dl_contact",
                "dl_shed",
                "dl_speed_win",
                "dl_power_win",
                "dl_finesse_win",
                "dl_win",
                "dl_win_t",
                "dl_hit",
                "dl_sack",
                "dl_bat",
                "dl_off",
                "dl_tfl",
                "dl_run_shed",
                "def_ypc",
            ])
            .chain(ED_BODY)
            .real(ed_real!("arm"))
            .sign(1.0)
            .headline()
            .note("arm inches are not read in the hand fight, only through the Length link (nothing below about the top 30%) and batted balls"),
        t("edge_getoff", "edge", "Get-off (10-yard split 1.70 / 1.62 / 1.55)", ed, drawn(Split, [1.70, 1.62, 1.55]))
            .chain(&[
                "dl_contact",
                "dl_shed",
                "dl_speed_win",
                "dl_power_win",
                "dl_win",
                "dl_win_t",
                "dl_press",
                "dl_hit",
                "dl_sack",
                "dl_off",
                "dl_tfl",
                "def_ypc",
                "def_sack",
            ])
            .chain(ED_BODY)
            .real(ed_real!("split"))
            .sign(-1.0),
        t("edge_snap", "edge", "Snap timing habit (4 / 16)", ed, Kind::Tend(Tend::SnapTiming))
            .chain(&["dl_contact", "dl_off", "dl_win", "dl_press"])
            .sign(-1.0),
        t("edge_speed", "edge", "Speed (forty 5.03 / 4.79 / 4.55)", ed, drawn(Forty, [5.03, 4.79, 4.55]))
            .chain(&["dl_speed_win", "dl_win", "dl_press", "dl_sack", "def_scr_yds"])
            .chain(ED_BODY)
            .real(ed_real!("forty"))
            .sign(1.0),
        t("edge_bend", "edge", "Flexibility (bend)", ed, r(&[A::Flexibility]))
            .chain(&["dl_speed_win", "dl_win", "dl_press"])
            .chain(ED_BODY)
            .sign(1.0),
        t("edge_weight", "edge", "Weight (243 / 264 / 285 lb)", ed, drawn(Weight, [243.0, 264.0, 285.0]))
            .chain(&["dl_power_win", "dl_speed_win", "dl_press", "dl_run_shed", "def_ypc"])
            .chain(ED_BODY)
            .real(ed_real!("wt"))
            .sign(0.0),
        // The edge research (afm/ab-tests/research-edge.md): height, the
        // two agility drills (the 3-cone reads bend), Agility and Strength.
        t("edge_height", "edge", "Height (6-2 / 6-3¾ / 6-5½)", ed, drawn(Height, [74.0, 75.75, 77.5]))
            .chain(&["dl_press", "dl_speed_win", "dl_power_win", "dl_bat"])
            .chain(ED_BODY)
            .real(ed_real!("ht"))
            .sign(0.0),
        t("edge_cone", "edge", "3-cone (7.44 / 7.17 / 6.90)", ed, drawn(Cone, [7.44, 7.17, 6.90]))
            .chain(&["dl_press", "dl_sack_prs", "dl_finesse_win", "dl_speed_win"])
            .chain(ED_BODY)
            .real(ed_real!("cone"))
            .sign(1.0),
        t("edge_shuttle", "edge", "Short shuttle (4.53 / 4.36 / 4.18)", ed, drawn(Shuttle, [4.53, 4.36, 4.18]))
            .chain(&["dl_press", "dl_hurry_snap", "dl_tfl", "dl_finesse_win"])
            .chain(ED_BODY)
            .real(ed_real!("shuttle"))
            .sign(1.0),
        t("edge_agility", "edge", "Agility", ed, r(&[A::Agility]))
            .chain(&["dl_finesse_win", "dl_win", "dl_press", "dl_sack_prs"])
            .chain(ED_BODY)
            .sign(1.0),
        t("edge_strength", "edge", "Strength", ed, r(&[A::Strength]))
            .chain(&["dl_power_win", "dl_win", "dl_press", "dl_run_shed"])
            .chain(ED_BODY)
            .sign(1.0),
        t("edge_finesse", "edge", "Finesse Rush", ed, r(&[A::FinesseRush]))
            .chain(&["dl_finesse_win", "dl_win", "dl_press", "dl_sack"])
            .real(&[("dl_press_snap", "edge_prss_per_snap"), ("dl_sack_snap", "edge_sack_per_snap")])
            .sign(1.0),
        t("edge_power", "edge", "Power Rush", ed, r(&[A::PowerRush]))
            .chain(&["dl_power_win", "dl_win", "dl_press", "dl_sack"])
            .real(&[("dl_press_snap", "edge_prss_per_snap"), ("dl_sack_snap", "edge_sack_per_snap")])
            .sign(1.0),
        t("edge_hands", "edge", "Hand Use", ed, r(&[A::HandUse]))
            .chain(&["dl_shed", "dl_win", "dl_press", "dl_sack"])
            .real(&[("dl_press_snap", "edge_prss_per_snap")])
            .sign(1.0),
        t("edge_upfield", "edge", "Upfield habit (4 / 16)", ed, Kind::Tend(Tend::Upfield))
            .chain(&["dl_press", "def_scr_yds", "def_ypc"])
            .sign(1.0),
        // ---------------------------------------------------- linebacker
        t("lb_weight", "lb", "Weight (225 / 237 / 249 lb)", lb, drawn(Weight, [225.0, 237.0, 249.0]))
            .chain(&["tk_miss", "tk_yaco", "dl_run_shed", "cv_cmp", "cv_ypt", "tk_far"])
            .chain(LB_BODY)
            .chain(LB_JOBS)
            .real(lb_real!("wt"))
            .sign(-1.0),
        t("lb_speed", "lb", "Speed (forty 4.89 / 4.69 / 4.50)", lb, drawn(Forty, [4.89, 4.69, 4.50]))
            .chain(&["cv_cmp", "cv_ypt", "cv_back_cmp", "tk_far", "tk_rate", "lb_blitz_press", "def_ypc", "lb_tk_within"])
            .chain(LB_BODY)
            .chain(LB_JOBS)
            .real(lb_real!("forty"))
            .sign(-1.0)
            .headline(),
        t("lb_height", "lb", "Height (5'11 / 6'1.5 / 6'3.5)", lb, body(Height, [71.0, 73.5, 75.5]))
            .chain(&["cv_te_cmp", "cv_cmp", "cv_cont"])
            .chain(LB_BODY)
            .chain(LB_JOBS)
            .real(lb_real!("ht"))
            .sign(-1.0),
        t("lb_arm", "lb", "Arms (30.75 / 32.25 / 33.75 in)", lb, body(Arm, [30.75, 32.25, 33.75]))
            .chain(&["dl_run_shed", "cv_cmp", "cv_cont", "tk_miss"])
            .chain(LB_BODY)
            .chain(LB_JOBS)
            .real(lb_real!("arm"))
            .sign(1.0),
        // A Sam's build (heavy, strong, sheds) and a Will's (light, fast,
        // covers), right and swapped on every team's board (on one team's board
        // alone his Sam played under 1% of the snaps).
        t("lb_swap", "lb", "A Will at Sam and a Sam at Will (every team; new to the job)", lb, swap(false))
            .chain(LB_SWAP)
            .chain(LB_JOBS)
            .sign(1.0),
        t("lb_swap_known", "lb", "A Will at Sam and a Sam at Will (every team; both know both jobs)", lb, swap(true))
            .chain(LB_SWAP)
            .chain(LB_JOBS)
            .sign(1.0),
        t("lb_recognition", "lb", "Play Recognition", lb, r(&[A::PlayRecognition]))
            .chain(&["def_bust", "tk_rate", "def_ypc", "def_pa_ypa"])
            .chain(LB_JOBS)
            .sign(-1.0),
        t("lb_tackling", "lb", "Tackling", lb, r(&[A::Tackling]))
            .chain(&["tk_miss", "tk_rate", "def_ypc"])
            .real(&[("tk_miss", "lb_mtkl_pct")])
            .sign(-1.0),
        t("lb_pursuit", "lb", "Pursuit", lb, r(&[A::Pursuit]))
            .chain(&["tk_rate", "tk_miss", "def_ypc", "tk_far"])
            .real(&[("tk_miss", "lb_mtkl_pct")])
            .sign(1.0),
        t("lb_shed", "lb", "Block Shedding", lb, r(&[A::BlockShedding]))
            .chain(&["dl_run_shed", "tk_rate", "def_ypc"])
            .sign(1.0),
        t("lb_hands", "lb", "Hand Use", lb, r(&[A::HandUse]))
            .chain(&["dl_run_shed", "tk_rate", "def_ypc"])
            .sign(1.0),
        t("lb_zone", "lb", "Zone Coverage", lb, r(&[A::ZoneCoverage]))
            .chain(&["cv_cmp", "cv_ypt", "cv_int"])
            .real(&[("cv_cmp", "lb_cmp_allowed"), ("cv_ypt", "lb_yds_per_tgt")])
            .sign(-1.0),
        t("lb_man", "lb", "Man Coverage", lb, r(&[A::ManCoverage]))
            .chain(&["cv_cmp", "cv_ypt", "cv_back_cmp"])
            .real(&[("cv_cmp", "lb_cmp_allowed"), ("cv_ypt", "lb_yds_per_tgt")])
            .sign(-1.0),
        t("lb_blitz", "lb", "Blitzing", lb, r(&[A::Blitzing]))
            .chain(&["lb_blitz_press", "dl_sack", "def_sack"])
            .sign(1.0),
        t("lb_downhill", "lb", "Downhill habit (4 / 16)", lb, Kind::Tend(Tend::Downhill))
            .chain(&["dl_tfl", "def_pa_ypa", "def_ypc"])
            .sign(1.0),
        // ---------------------------------------------------- cornerback
        t("cb_height", "cb", "Height (5'9 / 5'11 / 6'2)", cb, drawn(Height, [69.0, 71.0, 74.0]))
            .chain(&[
                "cv_cmp", "cv_cont", "cv_tight", "cv_deep", "cv_his_int", "cv_int", "cv_pbu", "cv_rate", "cv_ypt",
                "cv_yac", "cv_sep", "cv_tgt", "cv_pi", "cv_dh", "cv_adot", "cv_ypc", "dl_press_snap", "dl_sack_snap",
                "tk_run_snap", "tk_run_stop", "tk_rate", "tk_miss",
            ])
            .real(&[
                ("cv_cmp", "cb_ht__cmp_allowed"),
                ("cv_int", "cb_ht__int_per_tgt"),
                ("cv_pbu", "cb_ht__pbu_per_tgt"),
                ("cv_ypt", "cb_ht__yds_per_tgt"),
                ("cv_rate", "cb_ht__rating_allowed"),
                ("cv_tgt_snap", "cb_ht__tgt_per_snap"),
                ("cv_yac", "cb_ht__yac_per_cmp"),
                ("cv_adot", "cb_ht__dadot"),
                ("cv_ypc", "cb_ht__yds_per_cmp"),
                ("cv_pi", "cb_ht__dpi_per_snap"),
                ("cv_dh", "cb_ht__dhold_per_snap"),
                ("dl_press_snap", "cb_ht__prss_per_snap"),
                ("dl_sack_snap", "cb_ht__sack_per_snap"),
                ("tk_run_snap", "cb_ht__rtk_per_snap"),
                ("tk_run_stop", "cb_ht__rstop_per_snap"),
                ("tk_rate", "cb_ht__tkl_per_snap"),
                ("tk_miss", "cb_ht__mtkl_pct"),
            ])
            .sign(-1.0)
            .headline()
            .note("a corner's height is read at the catch point only: not in separation, not in the jam"),
        t("cb_arm", "cb", "Arms (30.5 / 31.75 / 33 in)", cb, body(Arm, [30.5, 31.75, 33.0]))
            .chain(&[
                "cv_cont", "cv_pbu", "cv_his_int", "cv_cmp", "cv_rate", "cv_sep", "tk_miss", "cv_yac", "cv_adot", "cv_ypc",
                "cv_pi", "cv_dh", "dl_press_snap",
            ])
            .real(&[
                ("cv_his_int", "cb_arm__int_per_tgt"),
                ("cv_pbu", "cb_arm__pbu_per_tgt"),
                ("cv_cmp", "cb_arm__cmp_allowed"),
                ("cv_rate", "cb_arm__rating_allowed"),
                ("cv_yac", "cb_arm__yac_per_cmp"),
                ("cv_adot", "cb_arm__dadot"),
                ("cv_ypc", "cb_arm__yds_per_cmp"),
                ("cv_pi", "cb_arm__dpi_per_snap"),
                ("cv_dh", "cb_arm__dhold_per_snap"),
                ("dl_press_snap", "cb_arm__prss_per_snap"),
                ("tk_miss", "cb_arm__mtkl_pct"),
            ])
            .sign(-1.0),
        t("cb_weight", "cb", "Weight (179 / 193 / 206 lb)", cb, body(Weight, [179.0, 193.0, 206.0]))
            .chain(&[
                "cv_yac", "tk_miss", "cv_sep", "cv_cmp", "cv_ypt", "cv_pbu", "cv_pi", "cv_dh", "dl_press_snap", "tk_run_snap",
                "tk_run_stop", "tk_rate", "tk_ff", "def_ypc", "def_blitz", "lb_blitz_press",
            ])
            .real(&[
                ("cv_yac", "cb_wt__yac_per_cmp"),
                ("tk_miss", "cb_wt__mtkl_pct"),
                ("cv_cmp", "cb_wt__cmp_allowed"),
                ("cv_ypt", "cb_wt__yds_per_tgt"),
                ("cv_pbu", "cb_wt__pbu_per_tgt"),
                ("cv_pi", "cb_wt__dpi_per_snap"),
                ("cv_dh", "cb_wt__dhold_per_snap"),
                ("dl_press_snap", "cb_wt__prss_per_snap"),
                ("tk_run_snap", "cb_wt__rtk_per_snap"),
                ("tk_run_stop", "cb_wt__rstop_per_snap"),
                ("tk_rate", "cb_wt__tkl_per_snap"),
                ("tk_ff", "cb_wt__ff_per_tkl"),
            ])
            .sign(-1.0),
        t("cb_speed", "cb", "Speed (forty 4.63 / 4.49 / 4.36)", cb, drawn(Forty, [4.63, 4.49, 4.36]))
            .chain(&[
                "cv_deep", "cv_ypc", "cv_adot", "cv_adot_within", "cv_ypt", "cv_sep", "cv_yac", "cv_cmp", "cv_pbu",
                "tk_run_snap", "tk_miss", "tk_ff",
            ])
            .real(&[
                ("cv_ypc", "cb_forty__yds_per_cmp"),
                ("cv_adot", "cb_forty__dadot"),
                ("cv_yac", "cb_forty__yac_per_cmp"),
                ("cv_cmp", "cb_forty__cmp_allowed"),
                ("cv_pbu", "cb_forty__pbu_per_tgt"),
                ("cv_ypt", "cb_forty__yds_per_tgt"),
                ("tk_run_snap", "cb_forty__rtk_per_snap"),
                ("tk_miss", "cb_forty__mtkl_pct"),
                ("tk_ff", "cb_forty__ff_per_tkl"),
            ])
            .sign(-1.0),
        t("cb_cone", "cb", "Agility and flexibility (3-cone 7.25 / 6.94 / 6.63)", cb, body(Cone, [7.25, 6.94, 6.63]))
            .chain(&["cv_sep", "cv_cmp", "cv_ypt", "cv_pbu", "cv_yac", "tk_miss", "tk_run_snap"])
            .real(&[
                ("cv_cmp", "cb_cone__cmp_allowed"),
                ("cv_ypt", "cb_cone__yds_per_tgt"),
                ("cv_pbu", "cb_cone__pbu_per_tgt"),
                ("cv_yac", "cb_cone__yac_per_cmp"),
                ("tk_miss", "cb_cone__mtkl_pct"),
                ("tk_run_snap", "cb_cone__rtk_per_snap"),
            ])
            .sign(-1.0),
        t("cb_press", "cb", "Press", cb, r(&[A::Press])).chain(&["cv_sep", "cv_cmp", "cv_ypt"]).sign(-1.0),
        t("cb_man", "cb", "Man Coverage", cb, r(&[A::ManCoverage]))
            .chain(&["cv_sep", "cv_cmp", "cv_ypt"])
            .real(&[("cv_cmp", "cb_cmp_allowed"), ("cv_ypt", "cb_yds_per_tgt")])
            .sign(-1.0),
        t("cb_zone", "cb", "Zone Coverage", cb, r(&[A::ZoneCoverage]))
            .chain(&["cv_sep", "cv_cmp", "cv_ypt"])
            .real(&[("cv_cmp", "cb_cmp_allowed"), ("cv_ypt", "cb_yds_per_tgt")])
            .sign(-1.0),
        t("cb_anticipation", "cb", "Anticipation", cb, r(&[A::Anticipation]))
            .chain(&["cv_his_int", "cv_pbu", "cv_cmp"])
            .real(&[("cv_his_int", "cb_int_per_tgt"), ("cv_pbu", "cb_pbu_per_tgt")])
            .sign(1.0),
        t("cb_recognition", "cb", "Play Recognition", cb, r(&[A::PlayRecognition]))
            .chain(&["cv_his_int", "cv_pbu", "cv_cmp"])
            .real(&[("cv_his_int", "cb_int_per_tgt"), ("cv_pbu", "cb_pbu_per_tgt")])
            .sign(1.0),
        t("cb_hands", "cb", "Catching (hands)", cb, r(&[A::Catching]))
            .chain(&["cv_his_int", "cv_pbu"])
            .real(&[("cv_his_int", "cb_int_per_tgt")])
            .sign(1.0),
        t("cb_discipline", "cb", "Discipline", cb, r(&[A::Discipline]))
            .chain(&["cv_pi", "cv_dh"])
            .real(&[("cv_pi", "cb_dpi_per_snap"), ("cv_dh", "cb_holding_per_snap")])
            .sign(-1.0),
        t("cb_jump", "cb", "Jump routes habit (4 / 16)", cb, Kind::Tend(Tend::JumpRoutes))
            .chain(&["cv_his_int", "cv_deep", "cv_ypt"])
            .sign(1.0),
        // ---------------------------------------------------- safety
        t("s_weight", "s", "Weight (193 / 207 / 221 lb)", sf, drawn(Weight, [193.0, 207.0, 221.0]))
            .chain(&[
                "tk_miss", "tk_ff", "cv_yac", "tk_yaco", "cv_te_cmp", "tk_run_snap", "tk_run_stop", "tk_run_long", "tk_rate",
                "cv_cmp", "cv_pbu", "def_box", "def_ypc", "def_run_succ", "def_epa_db", "def_cmp",
            ])
            .real(&[
                ("tk_miss", "s_wt__mtkl_pct"),
                ("tk_ff", "s_wt__ff_per_tkl"),
                ("tk_run_snap", "s_wt__rtk_per_snap"),
                ("tk_run_stop", "s_wt__rstop_per_snap"),
                ("tk_run_long", "s_wt__rlong_share"),
                ("tk_rate", "s_wt__tkl_per_snap"),
                ("cv_cmp", "s_wt__cmp_allowed"),
                ("cv_pbu", "s_wt__pbu_per_tgt"),
            ])
            .sign(-1.0)
            .headline(),
        t("s_height", "s", "Height (5'10 / 6'0 / 6'2.5)", sf, body(Height, [70.0, 72.0, 74.5]))
            .chain(&["cv_te_cmp", "cv_cmp", "cv_cont", "cv_his_int"])
            .real(&[("cv_cmp", "s_ht__cmp_allowed")])
            .sign(-1.0),
        t("s_hitting", "s", "Hitting (Tackling, Strength, Physicality together)", sf, r(&[
            A::Tackling,
            A::Strength,
            A::Physicality,
        ]))
        .chain(&["tk_miss", "tk_ff", "cv_middle", "cv_yac"])
        .real(&[("tk_miss", "s_mtkl_pct"), ("tk_ff", "s_ff_per_tkl")])
        .sign(-1.0)
        .note("a hit at the catch point never jars the ball loose"),
        t("s_bighit", "s", "BigHit habit (4 / 16)", sf, Kind::Tend(Tend::BigHit)).chain(&["tk_miss", "tk_ff"]).sign(1.0),
        t("s_speed", "s", "Speed (forty 4.71 / 4.56 / 4.41)", sf, body(Forty, [4.71, 4.56, 4.41]))
            .chain(&["cv_deep", "cv_ypc", "def_20"])
            .real(&[("cv_ypc", "s_forty__yds_per_cmp")])
            .sign(-1.0),
        t("s_recognition", "s", "Play Recognition", sf, r(&[A::PlayRecognition]))
            .chain(&["cv_his_int", "cv_cmp", "cv_deep"])
            .real(&[("cv_cmp", "s_cmp_allowed")])
            .sign(1.0),
        t("s_anticipation", "s", "Anticipation", sf, r(&[A::Anticipation]))
            .chain(&["cv_his_int", "cv_cmp", "cv_deep"])
            .real(&[("cv_cmp", "s_cmp_allowed")])
            .sign(1.0),
        t("s_zone", "s", "Zone Coverage", sf, r(&[A::ZoneCoverage]))
            .chain(&["cv_cmp", "cv_deep", "cv_his_int"])
            .real(&[("cv_cmp", "s_cmp_allowed")])
            .sign(-1.0),
        t("s_comm", "s", "Communication", sf, r(&[A::Communication])).chain(&["def_bust", "def_20", "cv_deep"]).sign(-1.0),
        // ---------------------------------------------------- special teams
        t("k_power", "st", "Kick Power", &[P::K], r(&[A::KickPower]))
            .chain(&["k_ko_tb", "k_ko_dist", "k_fg50", "k_50_share", "k_fg_dist"])
            .real(&[("k_fg50", "k_fg_pct_50p"), ("k_ko_tb", "k_touchback_rate_2025")])
            .sign(1.0)
            .headline(),
        t("k_short", "st", "Short Kick Accuracy", &[P::K], r(&[A::ShortKickAccuracy]))
            .chain(&["k_fg_u40", "k_fg40", "k_xp"])
            .real(&[("k_fg40", "k_fg_pct_40_49")])
            .sign(1.0),
        t("k_long", "st", "Long Kick Accuracy", &[P::K], r(&[A::LongKickAccuracy]))
            .chain(&["k_fg50", "k_fg40", "k_fg_u40"])
            .real(&[("k_fg50", "k_fg_pct_50p"), ("k_fg40", "k_fg_pct_40_49")])
            .sign(1.0),
        t("k_consistency", "st", "Kicker consistency (hidden, 4 / 16)", &[P::K], Kind::Consistency)
            .chain(&["k_fg40", "k_fg50", "k_fg_u40"])
            .sign(1.0),
        t("k_composure", "st", "Kicker Composure", &[P::K], r(&[A::Composure])).chain(&["k_fg_late", "k_fg_rest"]).sign(1.0),
        t("k_trajectory", "st", "Kick Trajectory", &[P::K], r(&[A::KickTrajectory])).chain(&["k_blk"]).sign(-1.0),
        t("k_getoff", "st", "Kicker Get-off", &[P::K], r(&[A::GetOff])).chain(&["k_blk"]).sign(-1.0),
        t("p_power", "st", "Punt Power", &[P::P], r(&[A::PuntPower]))
            .chain(&["p_gross", "p_net", "p_tb", "p_in20"])
            .real(&[("p_gross", "p_gross"), ("p_net", "p_net")])
            .sign(1.0),
        t("p_place", "st", "Punt Placement", &[P::P], r(&[A::PuntPlacement]))
            .chain(&["p_in20", "p_tb", "p_net"])
            .real(&[("p_in20", "p_inside20_rate"), ("p_net", "p_net")])
            .sign(1.0),
        t("p_getoff", "st", "Punter Get-off", &[P::P], r(&[A::GetOff])).chain(&["p_blk"]).sign(-1.0),
        t("ls_snap", "st", "Long Snapping", &[P::LS], r(&[A::LongSnapping]))
            .chain(&["bad_snap", "st_blk"])
            .sign(-1.0),
        t("rt_speed", "st", "Returner Speed (one team's returners)", ret, r(&[A::Speed]))
            .chain(&["rt_kr", "rt_pr", "rt_20"])
            .real(&[("rt_kr", "kr_avg_2025"), ("rt_pr", "pr_avg")])
            .sign(1.0)
            .mine(),
        t("rt_elusive", "st", "Returner Elusiveness (one team's returners)", ret, r(&[A::Elusiveness]))
            .chain(&["rt_kr", "rt_pr", "rt_20"])
            .real(&[("rt_kr", "kr_avg_2025"), ("rt_pr", "pr_avg")])
            .sign(1.0)
            .mine(),
        t("rt_weight", "st", "Returner weight (180 / 200 / 220 lb; one team's)", ret, body(Weight, [180.0, 200.0, 220.0]))
            .chain(&["rt_kr", "rt_pr", "rt_20"])
            .mine(),
        t("rt_decisions", "st", "Returner Decisions (one team's)", ret, r(&[A::Decisions]))
            .chain(&["rt_out", "rt_fair", "rt_muff", "rt_kr", "rt_pr"])
            .mine(),
        t("rt_tracking", "st", "Returner Ball Tracking (one team's)", ret, r(&[A::BallTracking]))
            .chain(&["rt_muff", "rt_fair", "rt_pr"])
            .sign(-1.0)
            .mine(),
        // ---------------------------------------------------- every position
        t("all_discipline", "all", "Discipline", every, r(&[A::Discipline]))
            .chain(&["pen", "ol_hold", "ol_fs", "cv_pi", "dl_off"])
            .sign(-1.0),
        t("all_stamina", "all", "Stamina", every, r(&[A::Stamina]))
            .chain(&["st_snaps", "st_q4_press", "st_q1_press"])
            .sign(1.0),
        t("all_consistency", "all", "Consistency (hidden, 4 / 16)", every, Kind::Consistency)
            .chain(&["qb_ypa_sd", "qb_epa_form", "qb_cmp_form", "rb_ypc_sd"])
            .sign(-1.0),
        t("all_composure", "all", "Composure", every, r(&[A::Composure]))
            .chain(&["qb_late_cmp", "k_fg_late", "pen"])
            .sign(1.0),
        t("all_concentration", "all", "Concentration", every, r(&[A::Concentration]))
            .chain(&["ol_fs_road", "ol_fs_home", "off_bust", "rc_drop"])
            .sign(-1.0),
        t("all_iq", "all", "Football IQ", every, r(&[A::FootballIq])).chain(&["off_bust", "def_bust", "pen"]).sign(-1.0),
        t("all_toughness", "all", "Toughness", every, r(&[A::Toughness]))
            .chain(&["inj"])
            .sign(-1.0)
            .note("Toughness is read only in the in-game injury check"),
    ];
    // ---------------------------------------------------- matchup grids
    let grid = |id, group, name, me: &[Position], part, val, opp: &[Position], op, ov, chain: &[&'static str]| {
        t(id, group, name, me, Kind::Grid { part, v: val, other: Who::at(opp), opart: op, ov }).chain(chain)
    };
    v.push(grid(
        "grid_wr_cb",
        "wr",
        "Receiver height × corner height",
        wr,
        Height,
        [69.5, 73.0, 76.0],
        cb,
        Height,
        [69.0, 71.0, 74.0],
        &["rc_cont", "rc_catch", "rc_int", "rc_rz", "rc_ypt"],
    ));
    v.push(grid(
        "grid_edge_ot",
        "edge",
        "Edge arms × tackle arms",
        ed,
        Arm,
        [32.1, 33.75, 35.4],
        ot,
        Arm,
        [32.5, 34.0, 35.5],
        &["dl_press", "dl_shed", "dl_win", "dl_sack"],
    ));
    // The edge research's combinations: one edge's two numbers moved
    // together (body first, then the drill at that body).
    let ecombo: &[&'static str] =
        &["dl_press", "dl_sack", "dl_sack_prs", "dl_speed_win", "dl_power_win", "dl_win", "dl_stop_run", "def_ypc"];
    for (id, name, p, pv, q, qv) in [
        ("grid_edge_wt_split", "Edge weight × get-off", Weight, [243.0, 264.0, 285.0], Split, [1.70, 1.62, 1.55]),
        ("grid_edge_wt_forty", "Edge weight × speed", Weight, [243.0, 264.0, 285.0], Forty, [5.03, 4.79, 4.55]),
        ("grid_edge_wt_cone", "Edge weight × 3-cone", Weight, [243.0, 264.0, 285.0], Cone, [7.44, 7.17, 6.90]),
        ("grid_edge_ht_cone", "Edge height × 3-cone", Height, [74.0, 75.75, 77.5], Cone, [7.44, 7.17, 6.90]),
        ("grid_edge_arm_split", "Edge arms × get-off", Arm, [32.1, 33.75, 35.4], Split, [1.70, 1.62, 1.55]),
        ("grid_edge_arm_forty", "Edge arms × speed", Arm, [32.1, 33.75, 35.4], Forty, [5.03, 4.79, 4.55]),
        // E3: does strength pay with mass? (bench reps at his weight)
        ("grid_edge_wt_bench", "Edge weight × bench", Weight, [243.0, 264.0, 285.0], Bench, [16.0, 20.0, 24.0]),
        ("grid_edge_wt_arm", "Edge weight × arms", Weight, [243.0, 264.0, 285.0], Arm, [32.1, 33.75, 35.4]),
    ] {
        v.push(grid(id, "edge", name, ed, p, pv, ed, q, qv, ecombo));
    }
    v.push(grid(
        "grid_dt_iol",
        "dt",
        "Defensive tackle weight × guard weight",
        dt,
        Weight,
        [284.0, 307.0, 330.0],
        iol,
        Weight,
        [290.0, 311.0, 331.0],
        &["dl_run_shed", "dl_shed", "dl_power_win", "def_ypc"],
    ));
    v.push(grid(
        "grid_rb_lb",
        "rb",
        "Back weight × linebacker weight",
        rb,
        Weight,
        [192.0, 217.0, 242.0],
        lb,
        Weight,
        [225.0, 237.0, 249.0],
        &["rb_yaco", "rb_mtf", "rb_ypc"],
    ));
    v.push(grid(
        "grid_te_cover",
        "te",
        "Tight end height × the linebacker and safety heights",
        te,
        Height,
        [74.0, 76.5, 78.5],
        &[P::LB, P::S],
        Height,
        [70.5, 72.75, 75.0],
        &["rc_cont", "rc_catch", "rc_rz"],
    ));
    // ---------------------------------------------------- the 12 Dynamic Links
    let link = |id, l, me: &[Position], chain: &[&'static str]| {
        t(id, "links", l_name(l), me, Kind::Link(l)).chain(chain).sign(1.0)
    };
    v.push(link("link_highpointer", Link::Highpointer, &[P::WR, P::TE], &["rc_cont", "rc_deep", "rc_rz"]));
    v.push(link("link_burner", Link::Burner, &[P::RB, P::WR], &["rb_20", "rc_deep_share", "rc_sep", "rc_yac"]));
    v.push(link("link_sudden", Link::Sudden, &[P::RB, P::WR], &["rb_ybc", "rb_mtf", "rc_sep"]));
    v.push(link("link_lowcenter", Link::LowCenter, &[P::RB], &["rb_yaco", "rb_mtf"]));
    v.push(link("link_ram", Link::BatteringRam, &[P::RB], &["rb_yaco", "rb_short"]));
    v.push(link("link_length_edge", Link::Length, &[P::EDGE], &["dl_press", "dl_shed", "dl_bat"]));
    v.push(
        link("link_length_cb", Link::Length, &[P::CB], &["cv_pbu", "cv_cont", "cv_cmp"])
            .sign(-1.0)
            .chain(&["cv_cont", "cv_pbu", "cv_cmp"]),
    );
    v.push(link("link_bigmover", Link::BigMover, &[P::DT], &["dl_win", "dl_press", "dl_run_shed"]));
    v.push(link("link_anchor", Link::Anchor, &[P::IOL], &["blk_lost_power", "blk_pass_lost"]).sign(-1.0));
    v.push(link("link_getoff", Link::GetOff, &[P::EDGE], &["dl_contact", "dl_win", "dl_press"]).sign(-1.0));
    v.push(link("link_bend", Link::Bend, &[P::EDGE], &["dl_speed_win", "dl_press"]));
    v.push(link("link_lungs", Link::Lungs, &[P::EDGE, P::DT], &["st_snaps", "st_q4_press"]));
    v.push(link("link_pocket", Link::PocketFrame, &[P::QB], &["qb_batted", "qb_sack_press", "qb_ttt"]).sign(-1.0));
    // Every stat with a real number is measured, after the chain.
    for t in &mut v {
        for (s, _) in t.real.clone() {
            if !t.chain.contains(&s) {
                t.chain.push(s);
            }
        }
    }
    v
}

fn l_name(l: Link) -> &'static str {
    match l {
        Link::Highpointer => "Highpointer (receivers and tight ends)",
        Link::Burner => "Burner (backs and receivers)",
        Link::Sudden => "Sudden (backs and receivers)",
        Link::LowCenter => "Low center of gravity (backs)",
        Link::BatteringRam => "Battering ram (backs)",
        Link::Length => "Length",
        Link::BigMover => "Big man who moves (defensive tackles)",
        Link::Anchor => "Anchor (interior line)",
        Link::GetOff => "Get-off (edge rushers)",
        Link::Bend => "Bend (edge rushers)",
        Link::Lungs => "Lungs (defensive line)",
        Link::PocketFrame => "Pocket frame (quarterbacks)",
    }
}

/// Report names for each group.
pub fn group_name(g: &str) -> &'static str {
    match g {
        "qb" => "Quarterback",
        "rb" => "Running back and fullback",
        "wr" => "Wide receiver",
        "te" => "Tight end",
        "ot" => "Offensive tackle",
        "iol" => "Interior line",
        "dt" => "Defensive tackle",
        "edge" => "Edge rusher",
        "lb" => "Linebacker",
        "cb" => "Cornerback",
        "s" => "Safety",
        "st" => "Kicker, punter, snapper, returners",
        "all" => "Every position",
        "links" => "The 12 Dynamic Links",
        _ => "Other",
    }
}

pub const GROUPS: [&str; 14] =
    ["qb", "rb", "wr", "te", "ot", "iol", "dt", "edge", "lb", "cb", "s", "st", "all", "links"];
