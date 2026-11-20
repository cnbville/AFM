//! The scheme-fit check's read-out: the audit's tables (`fit_pass.py` and
//! its helpers in `afm/scheme-fit/tools/`) from a run's block sums. Every
//! change is paired: an arm against its context's typical arm on the same
//! games, with its band from the 40 blocks. A star (*) is past the noise.
//!
//! It reads any run folder with a `schema.json`, `arms.jsonl` and the arms'
//! `.f32` sums, the audit's own runs included: slots are found by name, and a
//! measure whose slot a run lacks shows as a dash.

use std::collections::{BTreeMap, HashMap};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use afm_engine::attributes::Attr;
use afm_engine::positions::Position;
use afm_engine::world::systems::{FRONTS, OFFENSES, SHELLS};
use serde_json::Value;

use super::{Ctx, ROLES, bands};
use crate::ab::Part;

// ------------------------------------------------------------------ a run on disk

/// A watched man an arm changed, as the run wrote him.
#[derive(Clone, Debug, Default)]
pub struct ManData {
    pub role: String,
    pub weight: f64,
    pub height: f64,
    pub forty: f64,
    pub overall: f64,
    pub drawn: bool,
}

/// One arm's block sums.
pub struct ArmData {
    pub id: String,
    pub ctx: String,
    pub who: Vec<String>,
    pub label: String,
    pub men: Vec<ManData>,
    /// `blocks` rows of `width` floats.
    pub x: Vec<f64>,
}

impl ArmData {
    pub fn is_base(&self) -> bool {
        self.id.starts_with("base-")
    }
}

/// A run folder read back.
pub struct Run {
    pub dir: PathBuf,
    pub level: String,
    pub seed: u64,
    pub games: usize,
    pub blocks: usize,
    pub world: Option<String>,
    t: HashMap<String, usize>,
    p: HashMap<String, usize>,
    roles: Vec<String>,
    /// Each role's watched man's id.
    ids: Vec<u64>,
    width: usize,
    pub arms: Vec<ArmData>,
}

impl Run {
    pub fn load(dir: &Path) -> Result<Run, String> {
        let text = |name: &str| {
            std::fs::read_to_string(dir.join(name)).map_err(|e| format!("{}: {e}", dir.join(name).display()))
        };
        let s: Value = serde_json::from_str(&text("schema.json")?).map_err(|e| format!("schema.json: {e}"))?;
        let names = |k: &str| -> Vec<String> {
            s[k].as_array()
                .map(|v| v.iter().filter_map(|x| x.as_str().map(str::to_string)).collect())
                .unwrap_or_default()
        };
        let (tn, pn, roles) = (names("t"), names("p"), names("roles"));
        let width = tn.len() + roles.len() * pn.len();
        let blocks = s["blocks"].as_u64().unwrap_or(40) as usize;
        let who = s["who"].as_array().cloned().unwrap_or_default();
        let ids: Vec<u64> = roles
            .iter()
            .map(|r| who.iter().find(|w| w["role"].as_str() == Some(r)).and_then(|w| w["id"].as_u64()).unwrap_or(0))
            .collect();
        let mut arms: Vec<ArmData> = Vec::new();
        for line in text("arms.jsonl")?.lines().filter(|l| !l.trim().is_empty()) {
            let a: Value = serde_json::from_str(line).map_err(|e| format!("arms.jsonl: {e}"))?;
            let id = a["id"].as_str().unwrap_or_default().to_string();
            let path = dir.join(format!("{id}.f32"));
            let Ok(bytes) = std::fs::read(&path) else { continue };
            if arms.iter().any(|x| x.id == id) {
                continue;
            }
            if bytes.len() != blocks * width * 4 {
                return Err(format!(
                    "{}: {} bytes, the schema says {}",
                    path.display(),
                    bytes.len(),
                    blocks * width * 4
                ));
            }
            let x: Vec<f64> = bytes.as_chunks::<4>().0.iter().map(|c| f32::from_le_bytes(*c) as f64).collect();
            let men = a["men"]
                .as_array()
                .map(|v| {
                    v.iter()
                        .map(|m| ManData {
                            role: m["role"].as_str().unwrap_or_default().to_string(),
                            weight: m["weight"].as_f64().unwrap_or(f64::NAN),
                            height: m["height"].as_f64().unwrap_or(f64::NAN),
                            forty: m["forty"].as_f64().unwrap_or(f64::NAN),
                            overall: m["overall"].as_f64().unwrap_or(f64::NAN),
                            drawn: m["drawn"].as_bool().unwrap_or(true),
                        })
                        .collect()
                })
                .unwrap_or_default();
            arms.push(ArmData {
                id,
                ctx: a["ctx"].as_str().unwrap_or_default().to_string(),
                who: a["who"]
                    .as_array()
                    .map(|v| v.iter().filter_map(|x| x.as_str().map(str::to_string)).collect())
                    .unwrap_or_default(),
                label: a["label"].as_str().unwrap_or_default().to_string(),
                men,
                x,
            });
        }
        Ok(Run {
            dir: dir.to_path_buf(),
            level: s["level"].as_str().unwrap_or("FBS").to_string(),
            seed: s["seed"].as_u64().unwrap_or(0),
            games: s["games"].as_u64().unwrap_or(0) as usize,
            blocks,
            world: s["world"].as_str().map(str::to_string),
            t: tn.iter().enumerate().map(|(i, n)| (n.clone(), i)).collect(),
            p: pn.iter().enumerate().map(|(i, n)| (n.clone(), i)).collect(),
            ids,
            roles,
            width,
            arms,
        })
    }

    pub fn arm(&self, id: &str) -> Option<&ArmData> {
        self.arms.iter().find(|a| a.id == id)
    }

    pub fn base(&self, ctx: &str) -> Option<&ArmData> {
        self.arm(&format!("base-{ctx}"))
    }

    /// A season's games at the run's level.
    pub fn season(&self) -> f64 {
        match self.level.as_str() {
            "NFL" => 17.0,
            "HS" => 10.0,
            _ => 12.0,
        }
    }

    fn tcol(&self, n: &str) -> Option<usize> {
        self.t.get(n).copied()
    }

    fn pcol(&self, role: &str, n: &str) -> Option<usize> {
        let r = self.roles.iter().position(|x| x == role)?;
        Some(self.t.len() + r * self.p.len() + self.p.get(n)?)
    }

    /// The role whose slots hold a role's man: the last job with his id, as
    /// the run files him (the fullback is the back or tight end the engine
    /// seats there). The three corners' builds read the first corner.
    pub fn slot_role(&self, role: &str) -> String {
        if role == "CBS" {
            return "CB1".into();
        }
        let Some(r) = self.roles.iter().position(|x| x == role) else { return role.into() };
        let id = self.ids[r];
        let last = (0..self.roles.len()).rev().find(|&k| self.ids[k] == id && id != 0).unwrap_or(r);
        self.roles[last].clone()
    }

    fn header(&self) -> String {
        let world = if self.world.is_some() { ", the world's NFL rosters" } else { "" };
        format!("{}, seed {}, {} games an arm{world}", self.level, self.seed, self.games)
    }
}

/// The job a table files an arm under (three corners at once: CBS).
pub fn table_role(a: &ArmData) -> String {
    if a.who.len() == 1 { a.who[0].clone() } else { "CBS".into() }
}

// ------------------------------------------------------------------ measures

#[derive(Clone, Copy, Debug)]
enum Term {
    T(&'static str),
    P(&'static str),
    /// Games in a block.
    G,
}

/// A sum of slots.
#[derive(Clone, Debug)]
pub struct Lin(Vec<(f64, Term)>);

fn t(n: &'static str) -> Lin {
    Lin(vec![(1.0, Term::T(n))])
}

fn p(n: &'static str) -> Lin {
    Lin(vec![(1.0, Term::P(n))])
}

fn g() -> Lin {
    Lin(vec![(1.0, Term::G)])
}

impl std::ops::Add for Lin {
    type Output = Lin;
    fn add(mut self, o: Lin) -> Lin {
        self.0.extend(o.0);
        self
    }
}

impl std::ops::Sub for Lin {
    type Output = Lin;
    fn sub(mut self, o: Lin) -> Lin {
        self.0.extend(o.0.into_iter().map(|(c, x)| (-c, x)));
        self
    }
}

/// A measure: a ratio of slot sums, times `mult`, shown with `dec`
/// decimals.
#[derive(Clone, Debug)]
pub struct Metric {
    pub name: &'static str,
    num: Lin,
    den: Lin,
    mult: f64,
    pub dec: usize,
}

fn mt(name: &'static str, num: Lin, den: Lin, mult: f64, dec: usize) -> Metric {
    Metric { name, num, den, mult, dec }
}

impl Metric {
    /// Reads team slots only (no watched man's).
    fn team_only(&self) -> bool {
        self.num.0.iter().chain(&self.den.0).all(|(_, x)| !matches!(x, Term::P(_)))
    }
}

/// The measures for a job, with `s` games a season: the audit's columns in
/// its order (`fit_pass.py`, `metrics_for`), then the scheme-fit items' own.
pub fn metrics_for(role: &str, s: f64) -> Vec<Metric> {
    let team_off = vec![mt("pts/g", t("g_pf"), g(), 1.0, 2), mt("win%", t("g_win"), g(), 100.0, 1)];
    let team_def = vec![mt("pts allowed/g", t("g_pa"), g(), 1.0, 2), mt("win%", t("g_win"), g(), 100.0, 1)];
    let snap_o = mt("snap%", p("on"), t("o_snaps"), 100.0, 1);
    let snap_d = mt("snap%", p("on"), t("d_snaps"), 100.0, 1);
    let recv = vec![
        mt("tgt/g", p("tgt"), g(), 1.0, 2),
        mt("tgt/route", p("tgt"), p("on_db"), 100.0, 1),
        mt("yds/route", p("ryd"), p("on_db"), 1.0, 2),
        mt("catch%", p("rec"), p("tgt"), 100.0, 1),
        mt("yds/tgt", p("ryd"), p("tgt"), 1.0, 2),
        mt("sep", p("sep"), p("tgt"), 1.0, 2),
        mt("aDOT", p("air"), p("tgt"), 1.0, 2),
        mt("YAC/rec", p("yac"), p("rec"), 1.0, 2),
        mt("drop%", p("drop"), p("tgt"), 100.0, 1),
        mt("rec yds/season", p("ryd"), g(), s, 0),
        mt("rec TD/season", p("rtd"), g(), s, 2),
        mt("deep tgt/season", p("tgt20"), g(), s, 2),
    ];
    let block = vec![
        mt("run-block win%", p("rbw"), p("rbr"), 100.0, 1),
        mt("pass-block win%", p("pbw"), p("pbr"), 100.0, 1),
        mt("pressures allowed/season", p("pba"), g(), s, 1),
        mt("sacks allowed/season", p("sca"), g(), s, 2),
        mt("team ypc, him on", p("on_run_yds"), p("on_run"), 1.0, 2),
        mt("team run success%, him on", p("on_run_suc"), p("on_run"), 100.0, 1),
        mt("team sack%, him on", p("on_sack"), p("on_db"), 100.0, 2),
        mt("team pressure%, him on", p("on_prs"), p("on_db"), 100.0, 1),
    ];
    let rush = vec![
        mt("carries/g", p("car"), g(), 1.0, 2),
        mt("ypc", p("cyds"), p("car"), 1.0, 2),
        mt("success%", p("csuc"), p("car"), 100.0, 1),
        mt("10+%", p("c10"), p("car"), 100.0, 1),
        mt("yds before contact", p("cbc"), p("car"), 1.0, 2),
        mt("yds after contact", p("cyds") - p("cbc"), p("car"), 1.0, 2),
        mt("missed tkl forced/carry", p("cmt"), p("car"), 100.0, 1),
        mt("stuffed%", p("cneg"), p("car"), 100.0, 1),
        mt("fumbles/100 touches", p("cfum"), p("car") + p("rec"), 100.0, 2),
        mt("EPA/carry", p("cepa"), p("car"), 1.0, 3),
    ];
    let rusher = vec![
        mt("pressures/season", p("prs"), g(), s, 1),
        mt("sacks/season", p("sck"), g(), s, 2),
        mt("pressure/rush rep%", p("prs"), p("rrep"), 100.0, 1),
        mt("rush win/rep%", p("rwin"), p("rrep"), 100.0, 1),
        mt("beat pass block%", p("pbl"), p("pbf"), 100.0, 1),
        mt("hits/season", p("hit"), g(), s, 1),
    ];
    let run_d = vec![
        mt("tackles/g", p("tkl"), g(), 1.0, 2),
        mt("run stops/season", p("stop"), g(), s, 1),
        mt("TFL/season", p("tfl"), g(), s, 1),
        mt("missed tkl/season", p("mtk"), g(), s, 1),
        mt("shed%", p("shed"), p("rbf"), 100.0, 1),
    ];
    let cov = vec![
        mt("tgt on him/g", p("cov_t"), g(), 1.0, 2),
        mt("catch% on him", p("cov_c"), p("cov_t"), 100.0, 1),
        mt("yds/tgt on him", p("cov_y"), p("cov_t"), 1.0, 2),
        mt("INT/season", p("int"), g(), s, 2),
        mt("PBU/season", p("pbu"), g(), s, 1),
        mt("TD on him/season", p("cov_td"), g(), s, 2),
        mt("deep tgt on him/season", p("cov_t20"), g(), s, 1),
        mt("deep catch% on him", p("cov_c20"), p("cov_t20"), 100.0, 1),
        mt("jumps/season", p("jmp"), g(), s, 1),
        mt("busts/season", p("bust"), g(), s, 1),
    ];
    let tdef = vec![
        mt("team EPA/db allowed", t("d_db_epa"), t("d_db"), 1.0, 3),
        mt("team cmp% allowed", t("d_cmp"), t("d_att"), 100.0, 1),
        mt("team INT/season", t("d_int"), g(), s, 1),
        mt("S share of INT%", t("d_int_s"), t("d_int"), 100.0, 1),
        mt("team deep cmp% allowed", t("d_deep_cmp"), t("d_deep_att"), 100.0, 1),
        mt("team ypc allowed", t("d_ryds"), t("d_run"), 1.0, 2),
        mt("team run success% allowed", t("d_rsuc"), t("d_run"), 100.0, 1),
        mt("team sack%", t("d_sack"), t("d_db"), 100.0, 2),
        mt("team pressure%", t("d_prs"), t("d_db"), 100.0, 1),
        mt("blitz%", t("d_blitz"), t("d_snaps"), 100.0, 1),
        mt("man%", t("d_man"), t("d_snaps"), 100.0, 1),
    ];
    let tgts = || t("o_tgt_wr") + t("o_tgt_te") + t("o_tgt_rb");
    let toff = vec![
        mt("team EPA/db", t("o_db_epa"), t("o_db"), 1.0, 3),
        mt("team EPA/run", t("o_run_epa"), t("o_run"), 1.0, 3),
        mt("team ypc", t("o_ryds"), t("o_run"), 1.0, 2),
        mt("pass%", t("o_db"), t("o_snaps"), 100.0, 1),
        mt("WR tgt share%", t("o_tgt_wr"), tgts(), 100.0, 1),
        mt("TE tgt share%", t("o_tgt_te"), tgts(), 100.0, 1),
    ];
    let qb = vec![
        mt("cmp%", t("o_cmp"), t("o_att"), 100.0, 1),
        mt("Y/A", t("o_pyds"), t("o_att"), 1.0, 2),
        mt("INT%", t("o_int"), t("o_att"), 100.0, 2),
        mt("sack%", t("o_sack"), t("o_db"), 100.0, 2),
        mt("pressure%", t("o_prs"), t("o_db"), 100.0, 1),
        mt("aDOT", t("o_air"), t("o_att"), 1.0, 2),
        mt("TTT", t("o_ttt"), t("o_att"), 1.0, 3),
        mt("scrambles/g", t("o_scr"), g(), 1.0, 2),
        mt("his runs/g", p("car"), g(), 1.0, 2),
        mt("his ypc", p("cyds"), p("car"), 1.0, 2),
        mt("3rd down conv%", t("o_3conv"), t("o_3att"), 100.0, 1),
        mt("PA%", t("o_pa"), t("o_db"), 100.0, 1),
        mt("sep (all)", t("o_sep"), t("o_att"), 1.0, 2),
    ];
    // The items' own measures (scheme-fit.md), after the audit's.
    let pa = vec![
        mt("PA EPA/db", t("o_pa_epa"), t("o_pa"), 1.0, 3),
        mt("PA sack%", t("o_pa_sack"), t("o_pa"), 100.0, 1),
        mt("boot sack%", t("o_boot_sack"), t("o_boot"), 100.0, 1),
    ];
    let cover = vec![
        mt("sep vs man", p("sep_m"), p("tgt_m"), 1.0, 2),
        mt("sep vs zone", p("sep_z"), p("tgt_z"), 1.0, 2),
        mt("tgt vs man%", p("tgt_m"), p("tgt_m") + p("tgt_z"), 100.0, 1),
        mt("catch% vs man", p("rec_m"), p("tgt_m"), 100.0, 1),
        mt("catch% vs zone", p("rec") - p("rec_m"), p("tgt_z"), 100.0, 1),
        mt("tgt/route vs zone", p("tgt_z"), p("on_db_z"), 100.0, 1),
        mt("yds/route vs zone", p("ryd_z"), p("on_db_z"), 1.0, 2),
        mt("contested catch%", p("rec_c"), p("tgt_c"), 100.0, 1),
        mt("contested tgt%", p("tgt_c"), p("tgt"), 100.0, 1),
        mt("RZ TD/tgt%", p("td_rz"), p("tgt_rz"), 100.0, 1),
    ];
    let first = vec![
        mt("first tkl on runs/season", p("ftk"), g(), s, 1),
        mt("of them from a block%", p("ftk_blk"), p("ftk"), 100.0, 1),
    ];
    let blocked = mt("runs first-tackled from a block%", t("o_rtk_blk"), t("o_run"), 100.0, 1);
    let r = |v: &Vec<Metric>, i: std::ops::Range<usize>| v[i].to_vec();
    let mut out: Vec<Metric> = match role {
        "QB1" => [team_off, r(&toff, 0..4), qb, pa].concat(),
        "RB1" | "RB2" => [
            team_off,
            vec![snap_o],
            rush,
            r(&recv, 0..6),
            vec![block[2].clone(), toff[2].clone()],
            vec![
                mt("20+%", p("c20"), p("car"), 100.0, 1),
                mt("team short-yardage conv%", t("o_sy_rb_cv"), t("o_sy_rb"), 100.0, 1),
            ],
        ]
        .concat(),
        "FB" => [
            team_off,
            vec![snap_o],
            r(&block, 0..1),
            r(&block, 4..6),
            r(&rush, 0..2),
            r(&recv, 0..1),
            vec![toff[2].clone(), blocked],
        ]
        .concat(),
        "X" | "Z" | "SLOT" => {
            [team_off, vec![snap_o], recv, vec![toff[0].clone(), toff[4].clone(), block[0].clone()], cover].concat()
        }
        "TE1" | "TE2" | "TE3" => [
            team_off,
            vec![snap_o],
            recv,
            r(&block, 0..2),
            r(&block, 4..6),
            vec![toff[0].clone(), toff[2].clone(), toff[5].clone()],
            cover,
            vec![blocked],
        ]
        .concat(),
        "LT" | "LG" | "C" | "RG" | "RT" => {
            [team_off, vec![snap_o], block, vec![toff[0].clone(), toff[2].clone(), blocked]].concat()
        }
        "EDGE1" | "EDGE2" | "DT1" | "DT2" | "DT3" => [
            team_def,
            vec![snap_d],
            rusher,
            run_d,
            vec![tdef[5].clone(), tdef[6].clone(), tdef[7].clone(), tdef[8].clone(), tdef[0].clone()],
            first,
        ]
        .concat(),
        "SAM" | "MIKE" | "WILL" => [
            team_def,
            vec![snap_d],
            run_d,
            r(&cov, 0..5),
            r(&cov, 9..10),
            r(&rusher, 0..2),
            vec![tdef[5].clone(), tdef[6].clone(), tdef[0].clone()],
            vec![rusher[2].clone()],
            first,
        ]
        .concat(),
        "CB1" | "CB2" | "NCB" => [
            team_def,
            vec![snap_d],
            cov,
            r(&run_d, 0..1),
            r(&run_d, 3..4),
            vec![tdef[0].clone(), tdef[1].clone(), tdef[2].clone()],
        ]
        .concat(),
        "FS" | "SS" => [
            team_def,
            vec![snap_d],
            cov,
            r(&run_d, 0..4),
            vec![tdef[0].clone(), tdef[2].clone(), tdef[3].clone(), tdef[4].clone(), tdef[5].clone()],
            first,
        ]
        .concat(),
        _ => [team_off, team_def].concat(),
    };
    // A name once per job.
    let mut seen = Vec::new();
    out.retain(|m| {
        let new = !seen.contains(&m.name);
        seen.push(m.name);
        new
    });
    out
}

/// A measure on one arm: its pooled value and its value in each block.
pub fn eval(run: &Run, x: &[f64], role: &str, m: &Metric) -> (f64, Vec<f64>) {
    let gpb = run.games as f64 / run.blocks as f64;
    let cols = |l: &Lin| -> Option<Vec<(f64, Option<usize>)>> {
        l.0.iter()
            .map(|&(c, term)| match term {
                Term::T(n) => run.tcol(n).map(|i| (c, Some(i))),
                Term::P(n) => run.pcol(role, n).map(|i| (c, Some(i))),
                Term::G => Some((c, None)),
            })
            .collect()
    };
    let (Some(num), Some(den)) = (cols(&m.num), cols(&m.den)) else {
        return (f64::NAN, vec![f64::NAN; run.blocks]);
    };
    let at = |b: usize, l: &[(f64, Option<usize>)]| -> f64 {
        l.iter().map(|&(c, i)| c * i.map_or(gpb, |i| x[b * run.width + i])).sum()
    };
    let (mut sn, mut sd) = (0.0, 0.0);
    let blk: Vec<f64> = (0..run.blocks)
        .map(|b| {
            let (n, d) = (at(b, &num), at(b, &den));
            sn += n;
            sd += d;
            if d > 0.0 { n / d * m.mult } else { f64::NAN }
        })
        .collect();
    let tot = if sd > 0.0 { sn / sd * m.mult } else { f64::NAN };
    (tot, blk)
}

/// An arm against its typical arm on one measure.
#[derive(Clone, Copy, Debug)]
pub struct Eff {
    pub value: f64,
    pub base: f64,
    pub diff: f64,
    pub se: f64,
    pub z: f64,
}

fn sd(v: &[f64]) -> f64 {
    let n = v.len() as f64;
    let mean = v.iter().sum::<f64>() / n;
    (v.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (n - 1.0)).sqrt()
}

/// Paired over the blocks: the band is the spread of the block-by-block
/// differences.
pub fn diff(run: &Run, xa: &[f64], xb: &[f64], role: &str, m: &Metric) -> Eff {
    let (a, ba) = eval(run, xa, role, m);
    let (b, bb) = eval(run, xb, role, m);
    let d: Vec<f64> = ba.iter().zip(&bb).map(|(p, q)| p - q).filter(|v| !v.is_nan()).collect();
    let se = if d.len() > 3 { sd(&d) / (d.len() as f64).sqrt() } else { f64::NAN };
    let dv = a - b;
    let z = if se > 0.0 { dv / se } else { 0.0 };
    Eff { value: a, base: b, diff: dv, se, z }
}

fn fmt(v: f64, dec: usize) -> String {
    if v.is_nan() { "–".into() } else { format!("{v:.dec$}") }
}

fn fmt_signed(v: f64, dec: usize) -> String {
    if v.is_nan() { "–".into() } else { format!("{v:+.dec$}") }
}

fn star(e: &Eff) -> &'static str {
    if e.z.abs() > 1.96 { "*" } else { "" }
}

fn csv_num(v: f64) -> String {
    if v.is_nan() { "nan".into() } else { format!("{v}") }
}

/// The jobs in the report's order (defense first, the questions Cain asked).
pub const ROLE_ORDER: &[&str] = &[
    "FS", "SS", "CB1", "CB2", "NCB", "CBS", "SAM", "MIKE", "WILL", "EDGE1", "EDGE2", "DT1", "DT2", "DT3", "QB1", "RB1",
    "RB2", "FB", "X", "Z", "SLOT", "TE1", "TE2", "TE3", "LT", "LG", "C", "RG", "RT",
];

fn ctx_title(c: &str) -> String {
    Ctx::from_name(c).map(|x| format!("{c} ({})", x.title())).unwrap_or_else(|| c.to_string())
}

/// Every effect of a run: (arm, measure) to its effect, in table order.
pub struct Effects {
    pub rows: Vec<(String, String, String, String, &'static str, Eff)>,
}

impl Effects {
    pub fn get(&self, arm: &str, metric: &str) -> Option<&Eff> {
        self.rows.iter().find(|r| r.0 == arm && r.4 == metric).map(|r| &r.5)
    }
}

/// The arms by job, in table order, each job's arms in run order.
fn by_role(run: &Run) -> Vec<(String, Vec<&ArmData>)> {
    let mut m: BTreeMap<usize, (String, Vec<&ArmData>)> = BTreeMap::new();
    for a in run.arms.iter().filter(|a| !a.is_base()) {
        let r = table_role(a);
        let k = ROLE_ORDER.iter().position(|x| *x == r).unwrap_or(99);
        m.entry(k).or_insert_with(|| (r.clone(), Vec::new())).1.push(a);
    }
    m.into_values().collect()
}

pub fn effects(run: &Run) -> Effects {
    let s = run.season();
    let mut rows = Vec::new();
    for (role, arms) in by_role(run) {
        let rr = run.slot_role(&role);
        let ms = metrics_for(if role == "CBS" { "CB1" } else { &role }, s);
        for a in arms {
            let Some(base) = run.base(&a.ctx) else { continue };
            for m in &ms {
                let e = diff(run, &a.x, &base.x, &rr, m);
                rows.push((a.id.clone(), role.clone(), a.ctx.clone(), a.label.clone(), m.name, e));
            }
        }
    }
    Effects { rows }
}

// ------------------------------------------------------------------ the read-out

/// Writes the read-out into `<run>/report/` and returns what it wrote.
pub fn report(run: &Run) -> Result<Vec<String>, String> {
    let out = run.dir.join("report");
    std::fs::create_dir_all(&out).map_err(|e| e.to_string())?;
    let mut wrote = Vec::new();
    let mut put = |name: &str, body: String| -> Result<(), String> {
        std::fs::write(out.join(name), body).map_err(|e| e.to_string())?;
        wrote.push(name.to_string());
        Ok(())
    };
    let eff = effects(run);
    for (role, body) in role_tables(run, &eff) {
        put(&format!("{role}.md"), body)?;
    }
    put("effects.csv", effects_csv(&eff))?;
    put("_bases.md", bases(run))?;
    put("_compact.md", compact(run, &eff))?;
    put("_items.md", items(run))?;
    put("_recruited.md", recruited(run))?;
    put("_personnel.md", personnel(run))?;
    put("_alignment.md", alignment(run))?;
    put("_unread.md", unread(run))?;
    put("_weights.md", weights(run, &eff))?;
    Ok(wrote)
}

/// Each job's tables: per context, the typical value of every measure, then
/// each arm's change against it (`fit_pass.py`).
fn role_tables(run: &Run, eff: &Effects) -> Vec<(String, String)> {
    let s = run.season();
    let mut out = Vec::new();
    for (role, arms) in by_role(run) {
        let rr = run.slot_role(&role);
        let ms = metrics_for(if role == "CBS" { "CB1" } else { &role }, s);
        let mut ctxs: Vec<&str> = Vec::new();
        for a in &arms {
            if !ctxs.contains(&a.ctx.as_str()) {
                ctxs.push(&a.ctx);
            }
        }
        let mut md = format!("# {role} ({}, {s:.0}-game seasons)\n", run.header());
        for c in ctxs {
            let Some(base) = run.base(c) else { continue };
            let _ = writeln!(md, "\n## {role} in {}\n", ctx_title(c));
            let _ = writeln!(md, "| arm | {} |", ms.iter().map(|m| m.name).collect::<Vec<_>>().join(" | "));
            let _ = writeln!(md, "|---|{}", "---|".repeat(ms.len()));
            let cells: Vec<String> = ms.iter().map(|m| fmt(eval(run, &base.x, &rr, m).0, m.dec)).collect();
            let _ = writeln!(md, "| typical | {} |", cells.join(" | "));
            for a in arms.iter().filter(|a| a.ctx == c) {
                let cells: Vec<String> = ms
                    .iter()
                    .map(|m| match eff.get(&a.id, m.name) {
                        Some(e) => format!("{}{}", fmt_signed(e.diff, m.dec), star(e)),
                        None => "–".into(),
                    })
                    .collect();
                let _ = writeln!(md, "| {} | {} |", a.label, cells.join(" | "));
            }
        }
        md.push_str(
            "\nCells: the typical row is the value; other rows are the change against it. * = outside the 95% band (paired blocks).\n",
        );
        out.push((role, md));
    }
    out
}

fn effects_csv(eff: &Effects) -> String {
    let mut s = String::from("arm,role,ctx,label,metric,base,value,diff,se,z\n");
    for (arm, role, ctx, label, metric, e) in &eff.rows {
        let q = |x: &str| if x.contains(',') { format!("\"{x}\"") } else { x.to_string() };
        let _ = writeln!(
            s,
            "{arm},{role},{ctx},{},{},{},{},{},{},{}",
            q(label),
            q(metric),
            csv_num(e.base),
            csv_num(e.value),
            csv_num(e.diff),
            csv_num(e.se),
            csv_num(e.z)
        );
    }
    s
}

/// Team 0 by context: every team measure on each typical arm.
fn bases(run: &Run) -> String {
    let s = run.season();
    let mut ms: Vec<Metric> = Vec::new();
    for m in metrics_for("QB1", s).into_iter().chain(metrics_for("FS", s)) {
        if m.team_only() && !ms.iter().any(|x| x.name == m.name) {
            ms.push(m);
        }
    }
    let mut md = format!("# Team 0 by context ({})\n\n", run.header());
    let _ = writeln!(md, "| ctx | {} |", ms.iter().map(|m| m.name).collect::<Vec<_>>().join(" | "));
    let _ = writeln!(md, "|---|{}", "---|".repeat(ms.len()));
    for a in run.arms.iter().filter(|a| a.is_base()) {
        let cells: Vec<String> = ms.iter().map(|m| fmt(eval(run, &a.x, "QB1", m).0, m.dec)).collect();
        let _ = writeln!(md, "| {} | {} |", a.ctx, cells.join(" | "));
    }
    md
}

/// Each job group's key measures (`compact.py`).
fn key_measures(group: &str) -> &'static [&'static str] {
    match group {
        "FS" => &[
            "INT/season",
            "PBU/season",
            "catch% on him",
            "tgt on him/g",
            "deep tgt on him/season",
            "tackles/g",
            "run stops/season",
            "team EPA/db allowed",
            "S share of INT%",
            "pts allowed/g",
        ],
        "SS" => &[
            "INT/season",
            "PBU/season",
            "catch% on him",
            "yds/tgt on him",
            "tgt on him/g",
            "tackles/g",
            "run stops/season",
            "missed tkl/season",
            "team ypc allowed",
            "team EPA/db allowed",
            "pts allowed/g",
        ],
        "CB" => &[
            "tgt on him/g",
            "catch% on him",
            "yds/tgt on him",
            "INT/season",
            "PBU/season",
            "TD on him/season",
            "jumps/season",
            "tackles/g",
            "team EPA/db allowed",
            "pts allowed/g",
        ],
        "LB" => &[
            "tackles/g",
            "run stops/season",
            "TFL/season",
            "missed tkl/season",
            "shed%",
            "tgt on him/g",
            "catch% on him",
            "yds/tgt on him",
            "pressures/season",
            "busts/season",
            "team ypc allowed",
            "team EPA/db allowed",
            "pts allowed/g",
        ],
        "DL" => &[
            "snap%",
            "pressures/season",
            "sacks/season",
            "pressure/rush rep%",
            "rush win/rep%",
            "beat pass block%",
            "run stops/season",
            "shed%",
            "TFL/season",
            "team ypc allowed",
            "team pressure%",
            "pts allowed/g",
        ],
        "CBS" => &["team EPA/db allowed", "team cmp% allowed", "team INT/season", "pts allowed/g"],
        "QB" => &[
            "team EPA/db",
            "cmp%",
            "Y/A",
            "INT%",
            "sack%",
            "pressure%",
            "aDOT",
            "TTT",
            "scrambles/g",
            "his ypc",
            "3rd down conv%",
            "pts/g",
        ],
        "WR" => &[
            "snap%",
            "tgt/g",
            "tgt/route",
            "yds/route",
            "catch%",
            "yds/tgt",
            "sep",
            "YAC/rec",
            "drop%",
            "rec yds/season",
            "team EPA/db",
            "pts/g",
        ],
        "TE" => &[
            "snap%",
            "tgt/g",
            "tgt/route",
            "catch%",
            "yds/tgt",
            "sep",
            "rec yds/season",
            "run-block win%",
            "pass-block win%",
            "team ypc, him on",
            "team ypc",
            "pts/g",
        ],
        "RB" => &[
            "snap%",
            "carries/g",
            "ypc",
            "success%",
            "10+%",
            "yds before contact",
            "yds after contact",
            "missed tkl forced/carry",
            "stuffed%",
            "tgt/g",
            "rec yds/season",
            "pressures allowed/season",
            "team ypc",
            "pts/g",
        ],
        "FB" => &[
            "snap%",
            "run-block win%",
            "team ypc, him on",
            "team run success%, him on",
            "carries/g",
            "ypc",
            "tgt/g",
            "team ypc",
            "pts/g",
        ],
        _ => &[
            "pass-block win%",
            "pressures allowed/season",
            "sacks allowed/season",
            "run-block win%",
            "team ypc, him on",
            "team run success%, him on",
            "team sack%, him on",
            "team pressure%, him on",
            "team EPA/db",
            "team ypc",
            "pts/g",
        ],
    }
}

pub fn group_of(role: &str) -> &'static str {
    match role {
        "FS" => "FS",
        "SS" => "SS",
        "CB1" | "CB2" | "NCB" => "CB",
        "CBS" => "CBS",
        "SAM" | "MIKE" | "WILL" => "LB",
        "EDGE1" | "EDGE2" | "DT1" | "DT2" | "DT3" => "DL",
        "QB1" => "QB",
        "X" | "Z" | "SLOT" => "WR",
        "TE1" | "TE2" | "TE3" => "TE",
        "RB1" | "RB2" => "RB",
        "FB" => "FB",
        _ => "OL",
    }
}

/// The key measures per job, every arm against its typical arm (`compact.py`).
fn compact(run: &Run, eff: &Effects) -> String {
    let s = run.season();
    let mut md = format!("# Key measures by job ({})\n", run.header());
    for (role, arms) in by_role(run) {
        let rr = run.slot_role(&role);
        let all = metrics_for(if role == "CBS" { "CB1" } else { &role }, s);
        let ms: Vec<&Metric> =
            key_measures(group_of(&role)).iter().filter_map(|k| all.iter().find(|m| m.name == *k)).collect();
        let _ = writeln!(md, "\n## {role}\n");
        let _ = writeln!(md, "| context | arm | {} |", ms.iter().map(|m| m.name).collect::<Vec<_>>().join(" | "));
        let _ = writeln!(md, "|---|---|{}", "---|".repeat(ms.len()));
        let mut ctxs: Vec<&str> = Vec::new();
        for a in &arms {
            if !ctxs.contains(&a.ctx.as_str()) {
                ctxs.push(&a.ctx);
            }
        }
        for c in ctxs {
            let Some(base) = run.base(c) else { continue };
            let cells: Vec<String> = ms.iter().map(|m| fmt(eval(run, &base.x, &rr, m).0, m.dec)).collect();
            let _ = writeln!(md, "| {c} | typical | {} |", cells.join(" | "));
            for a in arms.iter().filter(|a| a.ctx == c) {
                let cells: Vec<String> = ms
                    .iter()
                    .map(|m| {
                        eff.get(&a.id, m.name)
                            .map_or("–".into(), |e| format!("{}{}", fmt_signed(e.diff, m.dec), star(e)))
                    })
                    .collect();
                let _ = writeln!(md, "| {c} | {} | {} |", a.label, cells.join(" | "));
            }
        }
    }
    md.push_str(
        "\nTypical rows are values; the others are changes against them. * = past the noise (paired blocks).\n",
    );
    md
}

/// One scheme-fit item's measure on team 0's games, against real.
struct Item {
    item: &'static str,
    name: &'static str,
    num: Lin,
    den: Lin,
    mult: f64,
    dec: usize,
    real: &'static str,
}

fn item(item: &'static str, name: &'static str, num: Lin, den: Lin, mult: f64, dec: usize, real: &'static str) -> Item {
    Item { item, name, num, den, mult, dec, real }
}

/// The items' measures (`docs/design/scheme-fit.md`, each item's target;
/// real is the 2024 NFL unless it says otherwise).
fn item_list() -> Vec<Item> {
    let tg = |g: &'static str, k: &'static str| -> Item {
        let (wr, te, rb) = match g {
            "11" => ("o_tg11_wr", "o_tg11_te", "o_tg11_rb"),
            "12" => ("o_tg12_wr", "o_tg12_te", "o_tg12_rb"),
            "13" => ("o_tg13_wr", "o_tg13_te", "o_tg13_rb"),
            _ => ("o_tg21_wr", "o_tg21_te", "o_tg21_rb"),
        };
        let (name, num, real) = match (g, k) {
            ("11", "te") => ("TE share of targets in 11 personnel", te, "19%"),
            ("12", "te") => ("TE share of targets in 12 personnel", te, "33%"),
            ("13", "te") => ("TE share of targets in 13 personnel", te, "56%"),
            ("21", "te") => ("TE share of targets in 21 personnel", te, "18%"),
            ("13", "wr") => ("WR share of targets in 13 personnel", wr, "27%"),
            _ => ("RB share of targets in 13 personnel", rb, "17%"),
        };
        item("2", name, t(num), t(wr) + t(te) + t(rb), 100.0, 1, real)
    };
    vec![
        item("1", "pass rate in 11 personnel", t("o_d11"), t("o_p11"), 100.0, 1, "65%"),
        item("1", "pass rate in 12 personnel", t("o_d12"), t("o_p12"), 100.0, 1, "46%"),
        item("1", "pass rate in 13 personnel", t("o_d13"), t("o_p13"), 100.0, 1, "36%"),
        item("1", "pass rate in 21 personnel", t("o_d21"), t("o_p21"), 100.0, 1, "40%"),
        tg("11", "te"),
        tg("12", "te"),
        tg("13", "te"),
        tg("21", "te"),
        tg("13", "wr"),
        tg("13", "rb"),
        item("3", "play action, share of dropbacks", t("o_pa"), t("o_db"), 100.0, 1, "21.2%"),
        item("3", "EPA a dropback, play action", t("o_pa_epa"), t("o_pa"), 1.0, 3, "+0.15"),
        item(
            "3",
            "EPA a dropback, no play action",
            t("o_db_epa") - t("o_pa_epa"),
            t("o_db") - t("o_pa"),
            1.0,
            3,
            "+0.03",
        ),
        item("3", "sack rate, play action", t("o_pa_sack"), t("o_pa"), 100.0, 1, "6.6%"),
        item("3", "sack rate, no play action", t("o_sack") - t("o_pa_sack"), t("o_db") - t("o_pa"), 100.0, 1, "–"),
        item("3", "boots, share of dropbacks", t("o_boot"), t("o_db"), 100.0, 1, "–"),
        item("3", "sack rate, boots", t("o_boot_sack"), t("o_boot"), 100.0, 1, "5.7% (boots and rollouts)"),
        item("6", "separation on targets against man", t("o_sep_man"), t("o_att_man"), 1.0, 2, "–"),
        item("6", "separation on targets against zone", t("o_sep_zone"), t("o_att_zone"), 1.0, 2, "–"),
        item("6", "catch rate against man", t("o_cmp_man"), t("o_att_man"), 100.0, 1, "–"),
        item(
            "7",
            // The play log's blocks: a block on record may have been put
            // down earlier or picked up after his hit, so this is not who
            // tackled from a block (`match_check` with `AFM_RUNDIAG` counts
            // that at the hit; lane SF3).
            "team 0's runs: first tackler with a run block on record he never shed",
            t("o_rtk_blk"),
            t("o_run"),
            100.0,
            1,
            "– (the audit's league: 34%)",
        ),
        item("7", "runs against team 0: the same", t("d_rtk_blk"), t("d_run"), 100.0, 1, "–"),
        item(
            "8",
            "3rd/4th and 2 or less, runs by backs and others, converted",
            t("o_sy_rb_cv"),
            t("o_sy_rb"),
            100.0,
            1,
            "67% (backs)",
        ),
        item("8", "3rd/4th and 2 or less, QB runs, converted", t("o_sy_qb_cv"), t("o_sy_qb"), 100.0, 1, "79%"),
        item("8", "against team 0: backs and others", t("d_sy_rb_cv"), t("d_sy_rb"), 100.0, 1, "67% (backs)"),
        item("8", "against team 0: QB runs", t("d_sy_qb_cv"), t("d_sy_qb"), 100.0, 1, "79%"),
        item("9", "backs' designed runs of 20+ yards", t("o_rb_r20"), t("o_rb_car"), 100.0, 2, "2.2% (2018-25: 2.4%)"),
        item("9", "backs' designed runs of 10+ yards", t("o_rb_r10"), t("o_rb_car"), 100.0, 1, "10.6%"),
        item("9", "against team 0: 20+", t("d_rb_r20"), t("d_rb_car"), 100.0, 2, "2.2%"),
        item("15", "30+ yard throws picked, against team 0", t("d_int30"), t("d_att30"), 100.0, 2, "6.7%"),
        item("14", "safeties' share of team 0's picks", t("d_int_s"), t("d_int"), 100.0, 1, "39%"),
        item("14", "safeties' share of team 0's picks behind man", t("d_int_s_man"), t("d_int_man"), 100.0, 1, "43%"),
        item(
            "16",
            "linemen's share of team 0's picks",
            t("d_int_dl"),
            t("d_int"),
            100.0,
            1,
            "4% (plus 5% by 3-4 OLBs)",
        ),
        item(
            "16",
            "of the linemen's picks, made while in a block",
            t("d_int_dl_blk"),
            t("d_int_dl"),
            100.0,
            1,
            "– (the audit's league: 175 of 195)",
        ),
        item(
            "17",
            "breakups by linebackers and backs, per 100 targeted throws",
            t("d_pbu_tgt"),
            t("d_tgt"),
            100.0,
            2,
            "8.2",
        ),
        item("19", "dropbacks against team 0 with a bust in its coverage", t("d_bdb"), t("d_db"), 100.0, 2, "–"),
        item(
            "19",
            "EPA a dropback against team 0, with such a bust",
            t("d_bdb_epa"),
            t("d_bdb"),
            1.0,
            3,
            "– (a man running free)",
        ),
        item(
            "19",
            "EPA a dropback against team 0, the others",
            t("d_db_epa") - t("d_bdb_epa"),
            t("d_db") - t("d_bdb"),
            1.0,
            3,
            "–",
        ),
    ]
}

/// The scheme-fit items' measures on each typical arm.
fn items(run: &Run) -> String {
    let bases: Vec<&ArmData> = run.arms.iter().filter(|a| a.is_base()).collect();
    let mut md = format!(
        "# The scheme-fit items' measures ({})\n\nTeam 0 (the typical team) in each context: its offense's numbers, or against its defense where it says. Real is the 2024 NFL (`docs/design/scheme-fit.md`) unless it says otherwise.\n\n",
        run.header()
    );
    let _ = writeln!(
        md,
        "| item | measure | {} | real |",
        bases.iter().map(|a| a.ctx.as_str()).collect::<Vec<_>>().join(" | ")
    );
    let _ = writeln!(md, "|---|---|{}---|", "---|".repeat(bases.len()));
    for it in item_list() {
        let m = mt(it.name, it.num, it.den, it.mult, it.dec);
        let cells: Vec<String> = bases.iter().map(|a| fmt(eval(run, &a.x, "QB1", &m).0, m.dec)).collect();
        let _ = writeln!(md, "| {} | {} | {} | {} |", it.item, it.name, cells.join(" | "), it.real);
    }
    md
}

/// As recruited: each band pair's drawn men against each other, four a band
/// (`draws.py`). Each side is the mean of its drawn men. "Games only" is the
/// band from the 40 blocks of the two sides' difference. "With the men" is
/// the audit's band, from the spread between the drawn men's own totals (each
/// already carries its own games' noise); the z and the star are on it.
fn recruited(run: &Run) -> String {
    let level = match run.level.as_str() {
        "NFL" => afm_engine::player::Level::NFL,
        "HS" => afm_engine::player::Level::HS,
        "FCS" => afm_engine::player::Level::FCS,
        _ => afm_engine::player::Level::FBS,
    };
    let s = run.season();
    let mut md = format!("# As recruited, four drawn men a band ({})\n", run.header());
    let mut any = false;
    for b in bands(level) {
        let c = b.ctx.name();
        let Some(base) = run.base(c) else { continue };
        let side = |k: usize| -> Vec<&ArmData> {
            (1..=4)
                .filter_map(|i| run.arm(&format!("{}-{c}-{}{i}", b.role, b.sides[k].0)))
                .filter(|a| a.men.first().is_none_or(|m| m.drawn))
                .collect()
        };
        let (sa, sb) = (side(0), side(1));
        if sa.is_empty() || sb.is_empty() {
            continue;
        }
        any = true;
        let (na, nb) = (b.sides[0].0, b.sides[1].0);
        let rr = run.slot_role(b.role);
        let unit = if b.part == Part::Forty { "s in the forty" } else { "lb" };
        let mean = |v: &[&ArmData], f: &dyn Fn(&ManData) -> f64| {
            let xs: Vec<f64> = v.iter().filter_map(|a| a.men.first().map(f)).collect();
            if xs.is_empty() { f64::NAN } else { xs.iter().sum::<f64>() / xs.len() as f64 }
        };
        let _ = writeln!(
            md,
            "\n## {} in {}: {na} against {nb} ({} and {} drawn men)\n",
            b.role,
            ctx_title(c),
            sa.len(),
            sb.len()
        );
        let _ = writeln!(
            md,
            "Mean weight {na} {:.0} lb, {nb} {:.0} lb; forty {na} {:.2}, {nb} {:.2}; overall {na} {:.1}, {nb} {:.1}. Bands ({unit}): {na} {:?}, {nb} {:?}.\n",
            mean(&sa, &|m| m.weight),
            mean(&sb, &|m| m.weight),
            mean(&sa, &|m| m.forty),
            mean(&sb, &|m| m.forty),
            mean(&sa, &|m| m.overall),
            mean(&sb, &|m| m.overall),
            b.sides[0].1,
            b.sides[1].1
        );
        let _ = writeln!(
            md,
            "| measure | typical | {na} | {nb} | {na} − {nb} | games only ± | with the men ± | z | % of {nb} |"
        );
        let _ = writeln!(md, "|---|---|---|---|---|---|---|---|---|");
        for m in metrics_for(b.role, s) {
            let ev =
                |v: &[&ArmData]| -> (Vec<f64>, Vec<Vec<f64>>) { v.iter().map(|a| eval(run, &a.x, &rr, &m)).unzip() };
            let (ta, ba) = ev(&sa);
            let (tb, bb) = ev(&sb);
            let avg = |v: &[f64]| v.iter().sum::<f64>() / v.len() as f64;
            let (ma, mb) = (avg(&ta), avg(&tb));
            let d = ma - mb;
            let per: Vec<f64> = (0..run.blocks)
                .map(|k| {
                    avg(&ba.iter().map(|x| x[k]).collect::<Vec<_>>())
                        - avg(&bb.iter().map(|x| x[k]).collect::<Vec<_>>())
                })
                .filter(|v| !v.is_nan())
                .collect();
            let se_g = if per.len() > 3 { sd(&per) / (per.len() as f64).sqrt() } else { f64::NAN };
            let var = |v: &[f64]| if v.len() > 1 { sd(v).powi(2) / v.len() as f64 } else { f64::NAN };
            let se_d = (var(&ta) + var(&tb)).sqrt();
            let z = d / se_d;
            let mark = if z.abs() >= 1.96 { "*" } else { "" };
            let _ = writeln!(
                md,
                "| {} | {} | {} | {} | {}{mark} | {} | {} | {} | {} |",
                m.name,
                fmt(eval(run, &base.x, &rr, &m).0, m.dec),
                fmt(ma, m.dec),
                fmt(mb, m.dec),
                fmt_signed(d, m.dec),
                fmt(1.96 * se_g, m.dec),
                fmt(1.96 * se_d, m.dec),
                fmt_signed(z, 2),
                fmt_signed(100.0 * d / mb, 1)
            );
        }
        let key = match group_of(b.role) {
            "DL" => "pressures/season",
            _ => "ypc",
        };
        let km = metrics_for(b.role, s).into_iter().find(|m| m.name == key).expect("key measure");
        let draw = |a: &ArmData| {
            let mm = a.men.first().cloned().unwrap_or_default();
            format!(
                "{} {:.0} lb {:.2} s ov {:.1}: {}",
                a.label,
                mm.weight,
                mm.forty,
                mm.overall,
                fmt(eval(run, &a.x, &rr, &km).0, km.dec)
            )
        };
        let all: Vec<String> = sa.iter().chain(&sb).map(|a| draw(a)).collect();
        let _ = writeln!(md, "\nThe drawn men ({key}): {}.", all.join("; "));
        let missing: Vec<String> = (0..2)
            .flat_map(|k| (1..=4).map(move |i| (k, i)))
            .filter_map(|(k, i)| {
                let id = format!("{}-{c}-{}{i}", b.role, b.sides[k].0);
                let a = run.arm(&id)?;
                (!a.men.first().is_none_or(|m| m.drawn)).then_some(id)
            })
            .collect();
        if !missing.is_empty() {
            let _ = writeln!(md, "\nNo man found in the band, left out: {}.", missing.join(", "));
        }
    }
    if !any {
        md.push_str("\nNo as-recruited arms in this run (`fit_check run ... draws`).\n");
    }
    md
}

/// Personnel on each typical arm (`pers.py`): the mix, how often each group
/// drops back, where the targets and yards go, play action.
fn personnel(run: &Run) -> String {
    let mut md = format!(
        "# Personnel by context ({})\n\n| ctx | snaps/g | 11/12/13/21/22/10/other % | dropback % in 11/12/13/21/22 | targets WR/TE/RB % | yards WR/TE/RB % | TE % of targets in 11/12/13/21 | PA % of dropbacks |\n|---|---|---|---|---|---|---|---|\n",
        run.header()
    );
    for a in run.arms.iter().filter(|a| a.is_base()) {
        let sum = |n: &str| -> f64 {
            run.tcol(n).map_or(f64::NAN, |i| (0..run.blocks).map(|b| a.x[b * run.width + i]).sum())
        };
        let pct = |x: f64, d: f64| if d > 0.0 { format!("{:.0}", 100.0 * x / d) } else { "–".into() };
        let groups = ["11", "12", "13", "21", "22", "10", "oth"];
        let p: Vec<f64> = groups.iter().map(|c| sum(&format!("o_p{c}"))).collect();
        let snaps: f64 = p.iter().sum();
        let d: Vec<f64> = groups[..5].iter().map(|c| sum(&format!("o_d{c}"))).collect();
        let tg = [sum("o_tgt_wr"), sum("o_tgt_te"), sum("o_tgt_rb")];
        let yd = [sum("o_yds_wr"), sum("o_yds_te"), sum("o_yds_rb")];
        let te: Vec<String> = ["11", "12", "13", "21"]
            .iter()
            .map(|g| {
                let (w, t_, r) =
                    (sum(&format!("o_tg{g}_wr")), sum(&format!("o_tg{g}_te")), sum(&format!("o_tg{g}_rb")));
                pct(t_, w + t_ + r)
            })
            .collect();
        let _ = writeln!(
            md,
            "| {} | {:.1} | {} | {} | {} | {} | {} | {} |",
            a.ctx,
            sum("o_snaps") / run.games as f64,
            p.iter().map(|&v| pct(v, snaps)).collect::<Vec<_>>().join("/"),
            (0..5).map(|i| pct(d[i], p[i])).collect::<Vec<_>>().join("/"),
            tg.iter().map(|&v| pct(v, tg.iter().sum())).collect::<Vec<_>>().join("/"),
            yd.iter().map(|&v| pct(v, yd.iter().sum())).collect::<Vec<_>>().join("/"),
            te.join("/"),
            if sum("o_db") > 0.0 { format!("{:.1}", 100.0 * sum("o_pa") / sum("o_db")) } else { "–".into() }
        );
    }
    md.push_str("\nReal 2024 NFL: dropbacks 65% in 11, 46% in 12, 36% in 13, 40% in 21; TE share of targets 19%, 33%, 56%, 18%; play action 21.2% of dropbacks.\n");
    md
}

/// The offense's spots (`game::playbook::ORole`, in its order) and the
/// defense's (`game::scrimmage::DSlot`), as the play log numbers them.
const ORN: [&str; 16] =
    ["QB", "RB", "FB", "X", "Z", "Slot", "Slot2", "TE", "TE2", "TE3", "LT", "LG", "C", "RG", "RT", "Slot3"];
const DSN: [&str; 14] = ["LE", "RE", "DTL", "DTR", "NT", "Sam", "Mike", "Will", "CBL", "CBR", "NCB", "DCB", "FS", "SS"];
const _: () = assert!(afm_engine::game::playbook::ORole::Slot3 as usize == 15);
const _: () = assert!(afm_engine::game::scrimmage::DSlot::SS as usize == 13);

/// Where each watched man lined up on each typical arm, and his share of
/// the snaps (`slots.py`): every man should play his own job.
fn alignment(run: &Run) -> String {
    let mut md = format!("# Where each watched man lined up ({})\n", run.header());
    for a in run.arms.iter().filter(|a| a.is_base()) {
        let _ = writeln!(md, "\n## {}\n\n| job | snap% | lined up (2% or more) |\n|---|---|---|", ctx_title(&a.ctx));
        let sum = |c: Option<usize>| c.map_or(0.0, |i| (0..run.blocks).map(|b| a.x[b * run.width + i]).sum::<f64>());
        for (ri, role) in ROLES.iter().enumerate() {
            if run.slot_role(role) != *role {
                continue;
            }
            let on = sum(run.pcol(role, "on"));
            if on == 0.0 {
                continue;
            }
            let offense = ri < 15;
            let snaps = sum(run.tcol(if offense { "o_snaps" } else { "d_snaps" }));
            let (names, pre): (&[&str], &str) = if offense { (&ORN, "or") } else { (&DSN, "ds") };
            let counts: Vec<f64> = (0..names.len()).map(|i| sum(run.pcol(role, &format!("{pre}{i}")))).collect();
            let tot: f64 = counts.iter().sum();
            let at: Vec<String> = counts
                .iter()
                .enumerate()
                .filter(|(_, v)| tot > 0.0 && **v / tot > 0.02)
                .map(|(i, v)| format!("{} {:.0}", names[i], 100.0 * v / tot))
                .collect();
            let _ = writeln!(md, "| {role} | {:.1} | {} |", 100.0 * on / snaps.max(1.0), at.join(", "));
        }
    }
    md
}

/// Arms that play every game exactly as their typical arm: the change is
/// never read (`unread.py`). A control should be here; a skill shouldn't.
fn unread(run: &Run) -> String {
    let arms: Vec<&ArmData> = run.arms.iter().filter(|a| !a.is_base()).collect();
    let same: Vec<&str> =
        arms.iter().filter(|a| run.base(&a.ctx).is_some_and(|b| b.x == a.x)).map(|a| a.id.as_str()).collect();
    let mut md = format!(
        "# Arms the engine never reads ({})\n\n{} of {} arms play every game exactly like their typical arm (the change is never read):\n\n",
        run.header(),
        same.len(),
        arms.len()
    );
    for id in same {
        let _ = writeln!(md, "- {id}");
    }
    md
}

// ------------------------------------------------------------------ fit weights

/// The world's fit weights (`world/systems.rs`, `score_by`): what each
/// system asks of a position, slot by slot (0-4 the offenses in `OFFENSES`
/// order, 5-6 the fronts, 7-10 the shells). The tool keeps its own copy;
/// `tests/fit_check.rs` checks it against the engine's `fits`. The two-gap
/// nose's body weight term is left out here (`score_by`'s `heavy`).
pub fn fit_weights(pos: Position, slot: usize) -> Vec<(Attr, f64)> {
    use Attr::*;
    use Position as P;
    let v: &[(Attr, f64)] = match (pos, slot) {
        (P::QB, 0) => {
            &[(ShortAccuracy, 0.3), (Decisions, 0.2), (Anticipation, 0.2), (QuickRelease, 0.15), (Footwork, 0.15)]
        }
        (P::QB, 1) => &[
            (ShortAccuracy, 0.2),
            (MediumAccuracy, 0.2),
            (DeepAccuracy, 0.15),
            (ArmStrength, 0.15),
            (QuickRelease, 0.15),
            (Decisions, 0.15),
        ],
        (P::QB, 2) => &[(ThrowOnMove, 0.3), (BallHandling, 0.25), (MediumAccuracy, 0.2), (Speed, 0.15), (Agility, 0.1)],
        (P::QB, 3) => {
            &[(ArmStrength, 0.3), (DeepAccuracy, 0.2), (PocketPresence, 0.2), (BallHandling, 0.15), (Footwork, 0.15)]
        }
        (P::QB, 4) => &[
            (Speed, 0.3),
            (Decisions, 0.2),
            (BallHandling, 0.15),
            (Elusiveness, 0.15),
            (ShortAccuracy, 0.1),
            (Agility, 0.1),
        ],
        (P::RB, 0) => &[(Catching, 0.35), (BallCarrierVision, 0.25), (RouteRunning, 0.2), (Agility, 0.2)],
        (P::RB, 1) => &[(Catching, 0.4), (PassBlocking, 0.35), (Speed, 0.25)],
        (P::RB, 2) => &[(BallCarrierVision, 0.45), (Acceleration, 0.3), (Agility, 0.25)],
        (P::RB, 3) => {
            &[(LegDrive, 0.3), (Balance, 0.2), (Acceleration, 0.2), (BallSecurity, 0.15), (BallCarrierVision, 0.15)]
        }
        (P::RB, 4) => &[(Speed, 0.4), (Acceleration, 0.3), (BallCarrierVision, 0.3)],
        (P::WR, 0) => &[(RouteRunning, 0.45), (Catching, 0.3), (Agility, 0.15), (FootballIq, 0.1)],
        (P::WR, 1) => &[(RouteRunning, 0.3), (Speed, 0.3), (Catching, 0.25), (FootballIq, 0.15)],
        (P::WR, 2) => &[(Speed, 0.4), (BlockingInSpace, 0.3), (RouteRunning, 0.3)],
        (P::WR, 3) => &[(BlockingInSpace, 0.35), (ContestedCatch, 0.3), (Catching, 0.2), (Strength, 0.15)],
        (P::WR, 4) => &[(Speed, 0.45), (BlockingInSpace, 0.3), (Catching, 0.25)],
        (P::TE, 0) => &[(Catching, 0.4), (RouteRunning, 0.4), (RunBlocking, 0.2)],
        (P::TE, 1) => &[(Catching, 0.5), (RouteRunning, 0.5)],
        (P::TE, 2) => &[(RunBlocking, 0.35), (Agility, 0.25), (Catching, 0.25), (BlockingInSpace, 0.15)],
        (P::TE, 3) => &[(RunBlocking, 0.45), (HandUse, 0.2), (Strength, 0.2), (Anchor, 0.15)],
        (P::TE, 4) => &[(BlockingInSpace, 0.4), (Catching, 0.3), (Speed, 0.3)],
        (P::OT | P::IOL, 0) => &[(PassBlocking, 0.55), (Agility, 0.2), (RunBlocking, 0.15), (HandUse, 0.1)],
        (P::OT | P::IOL, 1) => &[(PassBlocking, 0.7), (Anchor, 0.15), (Agility, 0.15)],
        (P::OT | P::IOL, 2) => &[(Agility, 0.3), (BlockingInSpace, 0.3), (RunBlocking, 0.2), (Speed, 0.2)],
        (P::OT | P::IOL, 3) => {
            &[(RunBlocking, 0.4), (Strength, 0.3), (Anchor, 0.1), (BlockingInSpace, 0.1), (HandUse, 0.1)]
        }
        (P::OT | P::IOL, 4) => &[(RunBlocking, 0.4), (Agility, 0.25), (BlockingInSpace, 0.2), (Strength, 0.15)],
        (P::DT, 5) => {
            &[(Acceleration, 0.35), (FinesseRush, 0.2), (PowerRush, 0.15), (Agility, 0.15), (Anticipation, 0.15)]
        }
        (P::DT, 6) => &[(Anchor, 0.25), (BlockShedding, 0.2), (HandUse, 0.2), (Strength, 0.15)],
        (P::EDGE, 5) => {
            &[(FinesseRush, 0.25), (PowerRush, 0.2), (Acceleration, 0.2), (BlockShedding, 0.2), (Anchor, 0.15)]
        }
        (P::EDGE, 6) => {
            &[(Speed, 0.25), (FinesseRush, 0.25), (Flexibility, 0.2), (ZoneCoverage, 0.15), (Pursuit, 0.15)]
        }
        (P::LB, 5) => &[(Speed, 0.3), (Pursuit, 0.25), (PlayRecognition, 0.25), (ZoneCoverage, 0.2)],
        (P::LB, 6) => &[(Tackling, 0.3), (BlockShedding, 0.25), (Traffic, 0.2), (Anchor, 0.15), (Strength, 0.1)],
        (P::CB, 7) => &[(Speed, 0.4), (ZoneCoverage, 0.35), (Anticipation, 0.15), (BallTracking, 0.1)],
        (P::CB, 8) => &[(ZoneCoverage, 0.35), (PlayRecognition, 0.25), (Tackling, 0.25), (ManCoverage, 0.15)],
        (P::CB, 9) => &[(ManCoverage, 0.45), (Speed, 0.35), (Press, 0.2)],
        (P::CB, 10) => &[(Press, 0.3), (ZoneCoverage, 0.3), (Tackling, 0.3), (Strength, 0.1)],
        (P::S, 7) => &[(Speed, 0.45), (ZoneCoverage, 0.25), (BallTracking, 0.15), (PlayRecognition, 0.15)],
        (P::S, 8) => &[(PlayRecognition, 0.35), (Tackling, 0.25), (ZoneCoverage, 0.25), (Anticipation, 0.15)],
        (P::S, 9) => &[(ManCoverage, 0.5), (Speed, 0.3), (Tackling, 0.2)],
        (P::S, 10) => &[(Speed, 0.35), (ZoneCoverage, 0.35), (BallTracking, 0.15), (PlayRecognition, 0.15)],
        (P::LB, 9) => &[(ManCoverage, 0.5), (Speed, 0.35), (Blitzing, 0.15)],
        (P::LB, 7 | 8 | 10) => &[(ZoneCoverage, 0.6), (PlayRecognition, 0.4)],
        (P::DT | P::EDGE, 9) => &[(Acceleration, 0.4), (FinesseRush, 0.3), (PowerRush, 0.3)],
        (P::DT | P::EDGE, 7 | 8 | 10) => &[(BlockShedding, 0.5), (PowerRush, 0.25), (FinesseRush, 0.25)],
        _ => &[],
    };
    v.to_vec()
}

/// The two-gap nose's body weight term in `score_by` (DT, slot 6): its share
/// of the score.
pub const HEAVY_WEIGHT: f64 = 0.2;

fn pos_of_role(role: &str) -> Option<Position> {
    Some(match role {
        "QB1" => Position::QB,
        "RB1" | "RB2" => Position::RB,
        "X" | "Z" | "SLOT" => Position::WR,
        "TE1" | "TE2" | "TE3" => Position::TE,
        "LT" | "RT" => Position::OT,
        "LG" | "C" | "RG" => Position::IOL,
        "DT1" | "DT2" | "DT3" => Position::DT,
        "EDGE1" | "EDGE2" => Position::EDGE,
        "SAM" | "MIKE" | "WILL" => Position::LB,
        "CB1" | "CB2" | "NCB" => Position::CB,
        "FS" | "SS" => Position::S,
        _ => return None,
    })
}

/// The system slots a job's fit reads in a context, as the audit counted
/// them (`hw.py`): the offense for the offense, the shell for corners and
/// safeties, the front for linemen, front and shell for linebackers. The
/// world also scores linemen in the shells; those slots come second.
fn slots_for(pos: Position, c: Ctx) -> (Vec<usize>, Vec<usize>) {
    let (off, front, shell) = c.schemes();
    let o = OFFENSES.iter().position(|&x| x == off).expect("offense slot");
    let f = 5 + FRONTS.iter().position(|&x| x == front).expect("front slot");
    let s = 7 + SHELLS.iter().position(|&x| x == shell).expect("shell slot");
    match pos {
        Position::CB | Position::S => (vec![s], vec![]),
        Position::DT | Position::EDGE => (vec![f], vec![s]),
        Position::LB => (vec![f, s], vec![]),
        _ => (vec![o], vec![]),
    }
}

/// One rating's +15 in a job and context: its name, the points it measured
/// (+ better for team 0) and its z.
type Measured = (String, f64, f64);

/// The world's fit weights against what +15 measured in the same scheme
/// (`hw.py`): points a game, the defense's as points saved, so + is better.
fn weights(run: &Run, eff: &Effects) -> String {
    let mut groups: BTreeMap<(usize, String), Vec<Measured>> = BTreeMap::new();
    for (arm, role, ctx, label, metric, e) in &eff.rows {
        if *metric != "pts/g" && *metric != "pts allowed/g" {
            continue;
        }
        let Some(att) = label.strip_suffix(" +15") else { continue };
        if pos_of_role(role).is_none() || run.arm(arm).is_none() {
            continue;
        }
        let sign = if *metric == "pts allowed/g" { -1.0 } else { 1.0 };
        let k = ROLE_ORDER.iter().position(|x| x == role).unwrap_or(99);
        groups.entry((k, format!("{role} {ctx}"))).or_default().push((att.to_string(), sign * e.diff, e.z));
    }
    let mut md = format!(
        "# The world's fit weights against what +15 measured ({})\n\nPoints a game, the defense's as points saved (+ is better); * past the noise. Weights from `world/systems.rs`, `score_by` (the tool's copy, checked against the engine by `tests/fit_check.rs`).\n\n| job and context | system slot | the weights, and what +15 measured | not weighted, measured |\n|---|---|---|---|\n",
        run.header()
    );
    let (mut tot, mut dead) = ([0.0f64; 2], [0.0f64; 2]);
    for ((_, key), arms) in &groups {
        let mut it = key.split(' ');
        let (role, ctx) = (it.next().unwrap_or(""), it.next().unwrap_or(""));
        let (Some(pos), Some(c)) = (pos_of_role(role), Ctx::from_name(ctx)) else { continue };
        let (main, extra) = slots_for(pos, c);
        for (k, sl) in main.iter().map(|&s| (0, s)).chain(extra.iter().map(|&s| (1, s))) {
            let mut w = fit_weights(pos, sl);
            if w.is_empty() {
                continue;
            }
            w.sort_by(|a, b| b.1.total_cmp(&a.1));
            let meas = |a: &str| arms.iter().find(|x| x.0 == a);
            let mut cells = Vec::new();
            for (a, wt) in &w {
                let name = format!("{a:?}");
                match meas(&name) {
                    Some((_, v, z)) => {
                        let is_dead = z.abs() < 1.96;
                        cells.push(format!("{name} {wt:.2}: {v:+.2}{}", if is_dead { "" } else { "*" }));
                        for j in k..2 {
                            tot[j] += wt;
                            dead[j] += wt * f64::from(u8::from(is_dead));
                        }
                    }
                    None => cells.push(format!("{name} {wt:.2}: not run")),
                }
            }
            if pos == Position::DT && sl == 6 {
                cells.push(format!("(body weight) {HEAVY_WEIGHT:.2}: not run"));
            }
            let mut extra_meas: Vec<&Measured> = arms
                .iter()
                .filter(|x| !w.iter().any(|(a, _)| format!("{a:?}") == x.0) && x.2.abs() >= 1.96 && x.1 > 0.0)
                .collect();
            extra_meas.sort_by(|a, b| b.1.total_cmp(&a.1));
            let _ = writeln!(
                md,
                "| {role} in {ctx} | {sl}{} | {} | {} |",
                if k == 1 { " (shell, not in the audit's count)" } else { "" },
                cells.join("; "),
                extra_meas.iter().map(|x| format!("{} {:+.2}*", x.0, x.1)).collect::<Vec<_>>().join(", ")
            );
        }
    }
    let pct = |d: f64, t: f64| if t > 0.0 { 100.0 * d / t } else { f64::NAN };
    let _ = writeln!(
        md,
        "\nWeight on measured ratings: {:.2}, of it on ratings with no measured effect: {:.2} ({:.0}%), as the audit counted. With the linemen's shell slots too: {:.2}, {:.2} ({:.0}%).",
        tot[0],
        dead[0],
        pct(dead[0], tot[0]),
        tot[1],
        dead[1],
        pct(dead[1], tot[1])
    );
    md
}

// ------------------------------------------------------------------ repeats

/// The report's repeat table (`rep2.py`): the claims behind the items, as
/// (label, arm, measure, measure in words).
pub const CLAIMS: &[(&str, &str, &str, &str)] = &[
    ("FS Zone Coverage +15, Cover 3", "FS-wc-ZoneCoverage+15", "INT/season", "picks a season"),
    ("FS Football IQ +15, Cover 3", "FS-wc-FootballIq+15", "INT/season", "picks"),
    ("FS Anticipation +15, Cover 3", "FS-wc-Anticipation+15", "INT/season", "picks"),
    ("FS Ball Tracking +15, Cover 3", "FS-wc-BallTracking+15", "INT/season", "picks"),
    ("FS all four +15, man pressure", "FS-mp-iq+15", "INT/season", "picks"),
    ("SS run support +15, Cover 3", "SS-wc-run+15", "pts allowed/g", "points allowed a game"),
    ("SS Man Coverage +15, Cover 3", "SS-wc-ManCoverage+15", "run stops/season", "run stops a season"),
    ("SS Man Coverage +15, man pressure", "SS-mp-ManCoverage+15", "pts allowed/g", "points allowed"),
    ("Corners built for man, Cover 3", "CBS-wc-man-built", "pts allowed/g", "points allowed"),
    ("Corners built for zone, Cover 3", "CBS-wc-zone-built", "pts allowed/g", "points allowed"),
    ("Corners built for man, man pressure", "CBS-mp-man-built", "pts allowed/g", "points allowed"),
    ("CB1 Zone Coverage +15, Cover 3", "CB1-wc-ZoneCoverage+15", "catch% on him", "catch rate on him"),
    ("CB1 Man Coverage +15, man pressure", "CB1-mp-ManCoverage+15", "catch% on him", "catch rate on him"),
    ("Mike Football IQ -15, Cover 3", "MIKE-wc-FootballIq-15", "busts/season", "busts a season"),
    ("Mike Football IQ -15, Cover 3", "MIKE-wc-FootballIq-15", "pts allowed/g", "points allowed"),
    ("Mike Blitzing +15, man pressure", "MIKE-mp-Blitzing+15", "pressures/season", "pressures a season"),
    ("Edge Finesse Rush +15", "EDGE1-wc-FinesseRush+15", "pressures/season", "pressures"),
    ("Tackle Finesse Rush +15", "DT1-wc-FinesseRush+15", "pressures/season", "pressures"),
    ("Tackle Power Rush +15", "DT1-wc-PowerRush+15", "pressures/season", "pressures"),
    ("Edge Get Off +15 (control)", "EDGE1-wc-GetOff+15", "pressures/season", "pressures"),
    ("QB Eye Manipulation +15", "QB1-wc-EyeManipulation+15", "pts/g", "points a game"),
    ("QB Decisions +15", "QB1-wc-Decisions+15", "INT%", "pick rate, points"),
    ("QB Decisions +15", "QB1-wc-Decisions+15", "aDOT", "air yards a throw"),
    ("QB Decisions +15", "QB1-wc-Decisions+15", "pts/g", "points a game"),
    ("QB Deep Accuracy +15", "QB1-wc-DeepAccuracy+15", "pts/g", "points a game"),
    ("X Route Running +15", "X-wc-RouteRunning+15", "rec yds/season", "his yards a season"),
    ("X Route Running +15", "X-wc-RouteRunning+15", "pts/g", "points a game"),
    ("TE3 Catching +15, 13 personnel", "TE3-pg13-Catching+15", "rec yds/season", "his yards a season"),
    ("LT Pass Blocking +15", "LT-wc-PassBlocking+15", "sacks allowed/season", "sacks a season"),
    ("LT Run Blocking +15, power gap", "LT-pg-RunBlocking+15", "run-block win%", "run blocks won, points"),
    ("LT Run Blocking +15, power gap", "LT-pg-RunBlocking+15", "team ypc", "team yards a carry"),
    ("Fullback Run Blocking +15, 21", "FB-wc21-RunBlocking+15", "team ypc", "team yards a carry"),
    ("RB1 Vision +15, power gap", "RB1-pg-BallCarrierVision+15", "ypc", "yards a carry"),
];

/// One claim's effect in a run.
pub fn claim(run: &Run, arm: &str, metric: &str) -> Option<Eff> {
    let a = run.arm(arm)?;
    let base = run.base(&a.ctx)?;
    let role = table_role(a);
    let m =
        metrics_for(if role == "CBS" { "CB1" } else { &role }, run.season()).into_iter().find(|m| m.name == metric)?;
    Some(diff(run, &a.x, &base.x, &run.slot_role(&role), &m))
}

/// Points a game for an arm, + better for team 0 (the defense's flipped).
fn points(run: &Run, a: &ArmData) -> Option<(f64, f64)> {
    let base = run.base(&a.ctx)?;
    let role = table_role(a);
    let off = ROLES.iter().position(|r| *r == run.slot_role(&role)).is_some_and(|i| i < 15);
    let (name, sign) = if off { ("pts/g", 1.0) } else { ("pts allowed/g", -1.0) };
    let m =
        metrics_for(if role == "CBS" { "CB1" } else { &role }, run.season()).into_iter().find(|m| m.name == name)?;
    let e = diff(run, &a.x, &base.x, &run.slot_role(&role), &m);
    Some((sign * e.diff, sign * e.z))
}

/// One arm in two runs: the larger |z| (to sort by), its id, then points a
/// game and z in the first run and in the later one.
type Paired = (f64, String, f64, f64, f64, f64);

/// The repeat table across runs, then each later run against the first on
/// points, arm by arm (`rep2.py`, `lvl.py`).
pub fn repeat(runs: &[(String, Run)]) -> String {
    let mut md = String::from("# Does it hold? The claims behind the items, run by run\n\n");
    for (name, r) in runs {
        let _ = writeln!(md, "- {name}: {} (`{}`)", r.header(), r.dir.display());
    }
    let _ =
        writeln!(md, "\n| test | measure | {} |", runs.iter().map(|(n, _)| n.as_str()).collect::<Vec<_>>().join(" | "));
    let _ = writeln!(md, "|---|---|{}", "---|".repeat(runs.len()));
    for (label, arm, metric, words) in CLAIMS {
        let cells: Vec<String> = runs
            .iter()
            .map(|(_, r)| match claim(r, arm, metric) {
                Some(e) if !e.diff.is_nan() => {
                    let v = if e.diff.abs() >= 20.0 { format!("{:+.0}", e.diff) } else { format!("{:+.2}", e.diff) };
                    format!("{v}{}", if e.z.abs() >= 1.96 { "" } else { " (n.s.)" })
                }
                _ => String::new(),
            })
            .collect();
        let _ = writeln!(md, "| {label} | {words} | {} |", cells.join(" | "));
    }
    md.push_str("\nEach cell is the change from the run's own typical arm; n.s. means inside the noise.\n");
    let Some((first, r0)) = runs.first() else { return md };
    for (name, r) in &runs[1..] {
        let mut out: BTreeMap<&str, Vec<Paired>> = BTreeMap::new();
        let mut both = 0;
        for a in r0.arms.iter().filter(|a| !a.is_base()) {
            let Some(b) = r.arm(&a.id) else { continue };
            let (Some((fv, fz)), Some((nv, nz))) = (points(r0, a), points(r, b)) else { continue };
            both += 1;
            let (fs, ns) = (fz.abs() >= 1.96, nz.abs() >= 1.96);
            let k = if fs && ns && (fz > 0.0) != (nz > 0.0) {
                "opposite"
            } else if fs && !ns {
                "first only"
            } else if ns && !fs {
                "second only"
            } else {
                "agree"
            };
            out.entry(k).or_default().push((fz.abs().max(nz.abs()), a.id.clone(), fv, fz, nv, nz));
        }
        let n = |k: &str| out.get(k).map_or(0, Vec::len);
        let _ = writeln!(
            md,
            "\n## {name} against {first}, on points (+ better for team 0)\n\nArms in both: {both}. Agree: {}. Past the noise in {first} only: {}. In {name} only: {}. Opposite: {}.\n",
            n("agree"),
            n("first only"),
            n("second only"),
            n("opposite")
        );
        for k in ["opposite", "second only", "first only"] {
            let Some(v) = out.get_mut(k) else { continue };
            v.sort_by(|a, b| b.0.total_cmp(&a.0));
            let _ = writeln!(md, "{k}:");
            for (_, id, fv, fz, nv, nz) in v.iter().take(25) {
                let _ = writeln!(md, "- {id}: {first} {fv:+.2} (z {fz:+.1}), {name} {nv:+.2} (z {nz:+.1})");
            }
        }
    }
    md
}
