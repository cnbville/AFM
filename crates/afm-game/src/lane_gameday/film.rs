//! The film on your next opponent (section 9, topic 1): your staff's
//! report, tagged from every tape of theirs with its sample and its
//! mistakes, and the tape's own hints when you study one yourself (Cain:
//! the same whatever the staff's rating, always true to that tape).

use std::collections::HashMap;

use afm_engine::rng::Rng;
use afm_engine::world::tape::{Snap, Tape};
use serde_json::{Value, json};

/// Names of the players on a tape: id to (name, position).
pub type Names = HashMap<u32, (String, String)>;

fn name(n: &Names, id: u32) -> String {
    n.get(&id).map_or(format!("No. {id}"), |x| x.0.clone())
}

fn pos(n: &Names, id: u32) -> &str {
    n.get(&id).map_or("", |x| x.1.as_str())
}

fn pct(k: u32, n: u32) -> u32 {
    if n == 0 { 0 } else { (100.0 * k as f64 / n as f64).round() as u32 }
}

/// "12 personnel under center, wing" from the film's "12 under center wing".
pub fn look_words(look: &str) -> String {
    let w: Vec<&str> = look.split_whitespace().collect();
    if w.len() < 3 {
        return look.to_string();
    }
    let (back, set) = if w[1] == "under" { ("under center", w.get(3)) } else { (w[1], w.get(2)) };
    let back = match back {
        "gun" => "shotgun",
        b => b,
    };
    format!("{} personnel, {back}, {}", w[0], set.copied().unwrap_or(""))
}

fn two_high(cov: &str) -> bool {
    matches!(cov, "Cover 2" | "Tampa 2" | "Cover 4" | "2-man" | "Cover 6")
}

fn scrimmage(s: &Snap) -> bool {
    !s.null && s.kind <= 1
}

/// What adding a hint to your plan does.
#[derive(Clone, Debug, PartialEq)]
pub enum Effect {
    /// Your coordinators build the week around it: its snaps are studied
    /// again, so your defenders key the tell and your caller counters the
    /// habit.
    Study,
    TargetCb(u32),
    Bracket(u32),
    Spy,
    KickAway,
    /// Draws and screens against an edge who wins upfield with speed.
    Draws,
    /// For your eyes: the battles decide it, nothing in the plan answers it.
    Note,
}

impl Effect {
    pub fn words(&self, n: &Names) -> String {
        match self {
            Effect::Study => "your coordinators study its snaps again, so your players key it a beat sooner".into(),
            Effect::TargetCb(id) => format!("your offense attacks {}", name(n, *id)),
            Effect::Bracket(id) => format!("your defense brackets {}", name(n, *id)),
            Effect::Spy => "a linebacker spies the quarterback".into(),
            Effect::KickAway => "your kicker keeps it away from the returner".into(),
            Effect::Draws => "your offense calls more draws and screens against his upfield rush".into(),
            Effect::Note => "for your eyes: the battles decide it".into(),
        }
    }
}

/// One thing a tape shows, with the count (topic 1, the tape's hints).
#[derive(Clone, Debug)]
pub struct Hint {
    /// The same habit on several tapes has the same key.
    pub key: String,
    /// "Tell", "Habit", "Battle", "Special teams", "Head coach".
    pub kind: &'static str,
    /// Whose unit: "their offense", "their defense", "their kicking game".
    pub unit: &'static str,
    pub text: String,
    pub tip: String,
    pub hit: u32,
    pub of: u32,
    pub strength: f64,
    /// The snaps that show it (indexes into the tape).
    pub snaps: Vec<usize>,
    pub effect: Effect,
}

/// The strongest hints of one tape on the team at `side` (0 home, 1 away):
/// three per tape, at most two of them notes the plan can't answer
/// (proposed, not confirmed; game-day.md topic 1).
pub fn hints(t: &Tape, side: u8, n: &Names) -> Vec<Hint> {
    let mut all = Vec::new();
    let off: Vec<usize> = (0..t.snaps.len()).filter(|&k| t.snaps[k].off == side && scrimmage(&t.snaps[k])).collect();
    let def: Vec<usize> = (0..t.snaps.len()).filter(|&k| t.snaps[k].off != side && scrimmage(&t.snaps[k])).collect();
    let s = |k: usize| &t.snaps[k];
    // Tells: a look they run from far more (or less) than their usual.
    let runs_all = off.iter().filter(|&&k| s(k).kind == 0).count() as f64;
    let base = runs_all / off.len().max(1) as f64;
    if off.len() >= 15 {
        let mut looks: HashMap<&str, Vec<usize>> = HashMap::new();
        for &k in &off {
            looks.entry(s(k).look.as_str()).or_default().push(k);
        }
        for (look, ks) in looks {
            let m = ks.len() as u32;
            if m < 5 || look.is_empty() {
                continue;
            }
            let r = ks.iter().filter(|&&k| s(k).kind == 0).count() as u32;
            let share = r as f64 / m as f64;
            let gap = share - base;
            if gap.abs() < 0.25 {
                continue;
            }
            let lw = look_words(look);
            let (text, tip, key) = if gap > 0.0 {
                (
                    format!("From {lw} they ran on {r} of {m} snaps ({}% overall).", (100.0 * base).round()),
                    format!(
                        "A run tell: when {lw} comes out, your front can play the run first. The danger is play action from it."
                    ),
                    format!("look:{look}:run"),
                )
            } else {
                (
                    format!(
                        "From {lw} they threw on {} of {m} snaps ({}% overall).",
                        m - r,
                        (100.0 * (1.0 - base)).round()
                    ),
                    format!(
                        "A pass tell: when {lw} comes out, drop into coverage or bring a pressure built for the pass."
                    ),
                    format!("look:{look}:pass"),
                )
            };
            all.push(Hint {
                key,
                kind: "Tell",
                unit: "their offense",
                text,
                tip,
                hit: if gap > 0.0 { r } else { m - r },
                of: m,
                strength: m as f64 * gap.abs() * 2.0,
                snaps: ks,
                effect: Effect::Study,
            });
        }
    }
    // Habits by situation, on offense.
    let pick = |f: &dyn Fn(&Snap) -> bool| -> Vec<usize> { off.iter().copied().filter(|&k| f(s(k))).collect() };
    let long3 = pick(&|x| x.down == 3 && x.togo >= 7);
    if long3.len() >= 4 {
        let m = long3.len() as u32;
        let p = long3.iter().filter(|&&k| s(k).kind == 1).count() as u32;
        let fav = favourite(&long3, t);
        if p as f64 / m as f64 >= 0.85 {
            let also =
                fav.as_ref().filter(|f| f.1 >= 3).map_or(String::new(), |f| format!(", {} of them {}", f.1, f.0));
            all.push(Hint {
                key: "off:3rd-long".into(),
                kind: "Habit",
                unit: "their offense",
                text: format!("On 3rd and 7 or longer they threw on {p} of {m} snaps{also}."),
                tip: "Long 3rd downs are pass downs for them: rush four, keep everything in front and rally to the sticks."
                    .into(),
                hit: p,
                of: m,
                strength: m as f64 * 0.6 + fav.map_or(0.0, |f| f.1 as f64 * 0.4),
                snaps: long3.clone(),
                effect: Effect::Study,
            });
        } else if (p as f64) / (m as f64) <= 0.5 {
            all.push(Hint {
                key: "off:3rd-long-run".into(),
                kind: "Habit",
                unit: "their offense",
                text: format!("On 3rd and 7 or longer they ran on {} of {m} snaps.", m - p),
                tip: "They give up on long 3rd downs with a draw or a run: keep your linebackers home and tackle short of the line to gain."
                    .into(),
                hit: m - p,
                of: m,
                strength: m as f64 * 0.9,
                snaps: long3.clone(),
                effect: Effect::Study,
            });
        }
    }
    let short = pick(&|x| (x.down == 3 || x.down == 4) && x.togo <= 2);
    if short.len() >= 4 {
        let m = short.len() as u32;
        let r = short.iter().filter(|&&k| s(k).kind == 0).count() as u32;
        let share = r as f64 / m as f64;
        if share >= 0.8 || share <= 0.35 {
            let (hit, text, tip) = if share >= 0.8 {
                let fav = favourite(&short, t)
                    .filter(|f| f.1 >= 2)
                    .map_or(String::new(), |f| format!(", {} of them {}", f.1, f.0));
                (
                    r,
                    format!("On 3rd or 4th and 2 or less they ran on {r} of {m} snaps{fav}."),
                    "Load the box on short yardage: they hand it off. Win the line of scrimmage and the down is yours.",
                )
            } else {
                (
                    m - r,
                    format!("On 3rd or 4th and 2 or less they threw on {} of {m} snaps.", m - r),
                    "Short yardage they throw: squeeze the quick game, jump the slant and the flat.",
                )
            };
            all.push(Hint {
                key: "off:short".into(),
                kind: "Habit",
                unit: "their offense",
                text,
                tip: tip.into(),
                hit,
                of: m,
                strength: m as f64 * 1.1,
                snaps: short.clone(),
                effect: Effect::Study,
            });
        }
    }
    let red = pick(&|x| x.yl <= 20);
    if red.len() >= 5
        && let Some((play, k, ks)) = favourite_snaps(&red, t)
        && k >= 3
    {
        all.push(Hint {
            key: format!("off:red:{play}"),
            kind: "Habit",
            unit: "their offense",
            text: format!("Inside your 20 they called {play} {k} times in {} snaps.", red.len()),
            tip: if t.snaps[ks[0]].kind == 0 {
                "Their go-to near the goal line is a run: fill fast and spill it to your pursuit.".into()
            } else {
                "Their go-to near the goal line is a throw: sit on the route and make the quarterback hold it.".into()
            },
            hit: k,
            of: red.len() as u32,
            strength: k as f64 * 1.6,
            snaps: ks,
            effect: Effect::Study,
        });
    }
    let first = pick(&|x| x.down == 1 && x.togo >= 10);
    if first.len() >= 12 {
        let m = first.len() as u32;
        let r = first.iter().filter(|&&k| s(k).kind == 0).count() as u32;
        let share = r as f64 / m as f64;
        if !(0.32..=0.68).contains(&share) {
            let run = share > 0.5;
            all.push(Hint {
                key: format!("off:1st:{}", if run { "run" } else { "pass" }),
                kind: "Habit",
                unit: "their offense",
                text: format!("On 1st and 10 they {} on {} of {m} snaps.", if run { "ran" } else { "threw" }, if run { r } else { m - r }),
                tip: if run {
                    "Early downs are run downs for them: an eighth man in the box on 1st down, and make them throw from behind the sticks.".into()
                } else {
                    "They throw early: play your early downs like passing downs and let your corners press.".into()
                },
                hit: if run { r } else { m - r },
                of: m,
                strength: m as f64 * (share - 0.5).abs() * 1.6,
                snaps: first.clone(),
                effect: Effect::Study,
            });
        }
    }
    // Their defense by situation.
    let dpick = |f: &dyn Fn(&Snap) -> bool| -> Vec<usize> { def.iter().copied().filter(|&k| f(s(k))).collect() };
    let d3l = dpick(&|x| x.down == 3 && x.togo >= 7);
    if d3l.len() >= 4 {
        let m = d3l.len() as u32;
        let two = d3l.iter().filter(|&&k| two_high(&s(k).coverage)).count() as u32;
        let share = two as f64 / m as f64;
        if share >= 0.75 {
            all.push(Hint {
                key: "def:3rd-long:two".into(),
                kind: "Habit",
                unit: "their defense",
                text: format!("On 3rd and 7 or longer they showed two deep safeties on {two} of {m} snaps."),
                tip: "Two deep on long 3rd downs leaves the middle under the safeties: dig and hook routes to the sticks, or the draw."
                    .into(),
                hit: two,
                of: m,
                strength: m as f64 * share,
                snaps: d3l.clone(),
                effect: Effect::Study,
            });
        } else if share <= 0.25 {
            all.push(Hint {
                key: "def:3rd-long:one".into(),
                kind: "Habit",
                unit: "their defense",
                text: format!("On 3rd and 7 or longer they played one deep safety on {} of {m} snaps.", m - two),
                tip: "One deep safety on long 3rd downs: attack the seams and the sideline, and beat the man under it with crossers."
                    .into(),
                hit: m - two,
                of: m,
                strength: m as f64 * (1.0 - share),
                snaps: d3l.clone(),
                effect: Effect::Study,
            });
        }
    }
    let d3 = dpick(&|x| x.down == 3);
    if d3.len() >= 5 {
        let m = d3.len() as u32;
        let man = d3.iter().filter(|&&k| s(k).man).count() as u32;
        if man as f64 / m as f64 >= 0.7 {
            all.push(Hint {
                key: "def:3rd:man".into(),
                kind: "Habit",
                unit: "their defense",
                text: format!("They played man coverage on {man} of {m} third downs."),
                tip: "Man on 3rd down: crossers, rub routes and bunch sets let your receivers run away from it.".into(),
                hit: man,
                of: m,
                strength: m as f64 * 0.9,
                snaps: d3.clone(),
                effect: Effect::Study,
            });
        }
        let bl = d3.iter().filter(|&&k| s(k).blitz).count() as u32;
        if bl as f64 / m as f64 >= 0.45 {
            all.push(Hint {
                key: "def:3rd:blitz".into(),
                kind: "Habit",
                unit: "their defense",
                text: format!("They sent extra rushers on {bl} of {m} third downs."),
                tip: "Expect pressure on 3rd down: quick throws and hot routes, and keep the back in to pick up the blitz."
                    .into(),
                hit: bl,
                of: m,
                strength: m as f64 * bl as f64 / m as f64 * 1.8,
                snaps: d3.iter().copied().filter(|&k| s(k).blitz).collect(),
                effect: Effect::Study,
            });
        }
    }
    let d1 = dpick(&|x| x.down == 1);
    if d1.len() >= 10 {
        let m = d1.len() as u32;
        let bl = d1.iter().filter(|&&k| s(k).blitz).count() as u32;
        if bl as f64 / m as f64 >= 0.35 {
            all.push(Hint {
                key: "def:1st:blitz".into(),
                kind: "Habit",
                unit: "their defense",
                text: format!("They blitzed on {bl} of {m} first downs."),
                tip: "Early-down blitzes: screens and play action punish them, and a quick run can hit behind the blitzer."
                    .into(),
                hit: bl,
                of: m,
                strength: bl as f64 * 1.4,
                snaps: d1.iter().copied().filter(|&k| s(k).blitz).collect(),
                effect: Effect::Study,
            });
        }
    }
    // Explosive plays against a coverage, and runs that gashed them.
    let mut by_cov: HashMap<&str, Vec<usize>> = HashMap::new();
    for &k in &def {
        let x = s(k);
        if x.kind == 1 && x.complete && x.yards >= 20 && !x.coverage.is_empty() {
            by_cov.entry(x.coverage.as_str()).or_default().push(k);
        }
    }
    for (cov, ks) in by_cov {
        let k = ks.len() as u32;
        if k < 2 {
            continue;
        }
        let of = def.iter().filter(|&&j| s(j).kind == 1 && s(j).coverage == cov).count() as u32;
        let tip = match cov {
            "Cover 3" | "Cover 3 match" => {
                "Four verticals and seams stretch Cover 3's deep thirds; a post over a dig splits it."
            }
            "Cover 2" | "Tampa 2" => {
                "The hole between the corner and the safety: smash and the deep corner route beat their Cover 2."
            }
            "Cover 1" | "Cover 0" | "Cover 1 robber" => {
                "Their man coverage gives up the deep ball: double moves and go routes on your best receiver."
            }
            "Cover 4" | "Cover 6" => "Quarters safeties bite on the run: play action and a post behind them.",
            _ => "Go back to what beat it: it worked more than once.",
        };
        all.push(Hint {
            key: format!("def:big:{cov}"),
            kind: "Battle",
            unit: "their defense",
            text: format!("They gave up {k} completions of 20 or more yards in {cov} (on {of} throws against it)."),
            tip: tip.into(),
            hit: k,
            of,
            strength: k as f64 * 2.2,
            snaps: ks,
            effect: Effect::Study,
        });
    }
    let mut by_run: HashMap<String, Vec<usize>> = HashMap::new();
    for &k in &def {
        let x = s(k);
        if x.kind == 0 && !x.play.is_empty() {
            let concept = x.play.trim_end_matches(" left").trim_end_matches(" right").to_string();
            by_run.entry(concept).or_default().push(k);
        }
    }
    for (concept, ks) in by_run {
        let m = ks.len() as u32;
        if m < 4 {
            continue;
        }
        let avg = ks.iter().map(|&k| s(k).yards as f64).sum::<f64>() / m as f64;
        if avg >= 6.0 {
            all.push(Hint {
                key: format!("def:run:{concept}"),
                kind: "Battle",
                unit: "their defense",
                text: format!("{} gained {avg:.1} yards a carry against them ({m} runs).", capital(&concept)),
                tip: format!("Their front doesn't fit {concept}: keep it on your call sheet."),
                hit: m,
                of: m,
                strength: m as f64 * (avg - 4.0) * 0.6,
                snaps: ks,
                effect: Effect::Study,
            });
        }
    }
    // Battles up front.
    for &(sd, id, reps, sp, pw, fi, pr) in &t.protection {
        if sd != side || reps < 8 {
            continue;
        }
        let lost = (sp + pw + fi) as u32;
        if lost < 3 || (lost as f64) < 0.22 * reps as f64 {
            continue;
        }
        let (mv, k) = [("speed", sp), ("power", pw), ("hand moves", fi)].into_iter().max_by_key(|x| x.1).unwrap();
        let tip = match mv {
            "speed" => "Line your fastest edge rusher up over him and win the corner.",
            "power" => "Bull rush him: he doesn't anchor.",
            _ => "Hands beat him: swim and rip moves.",
        };
        all.push(Hint {
            key: format!("pro:{id}"),
            kind: "Battle",
            unit: "their offense",
            text: format!(
                "{} ({}) lost {lost} of {reps} pass-protection reps, {k} of them to {mv}{}.",
                name(n, id),
                pos(n, id),
                if pr > 0 {
                    format!("; {} allowed", count(pr as usize, "pressure", "pressures"))
                } else {
                    String::new()
                }
            ),
            tip: tip.into(),
            hit: lost,
            of: reps as u32,
            strength: lost as f64 * 1.5,
            snaps: Vec::new(),
            effect: Effect::Note,
        });
    }
    // Their pass rushers, against the dropbacks they faced.
    let dropbacks = def.iter().filter(|&&k| s(k).kind == 1).count() as u32;
    for &(sd, id, sp, pw, fi) in &t.rush {
        if sd != side {
            continue;
        }
        let k = (sp + pw + fi) as u32;
        if k < 3 {
            continue;
        }
        let mv = [("speed", sp), ("power", pw), ("hand moves", fi)].into_iter().max_by_key(|x| x.1).unwrap().0;
        let speed = mv == "speed";
        all.push(Hint {
            key: format!("rush:{id}"),
            kind: "Battle",
            unit: "their defense",
            text: format!(
                "{} ({}) won quickly on {k} of {dropbacks} dropbacks, mostly with {mv}.",
                name(n, id),
                pos(n, id)
            ),
            tip: if speed {
                "He wins upfield with speed: draws and screens punish it, and a chip from the back slows him."
            } else {
                "Chip him with a back or a tight end, or slide the protection his way on passing downs."
            }
            .into(),
            hit: k,
            of: dropbacks.max(k),
            strength: k as f64 * 1.3,
            snaps: Vec::new(),
            effect: if speed { Effect::Draws } else { Effect::Note },
        });
    }
    // Their receivers and the corners who covered.
    let throws: Vec<usize> = off.iter().copied().filter(|&k| s(k).kind == 1 && s(k).target.is_some()).collect();
    if throws.len() >= 10 {
        let mut by: HashMap<u32, Vec<usize>> = HashMap::new();
        for &k in &throws {
            by.entry(s(k).target.unwrap()).or_default().push(k);
        }
        if let Some((&id, ks)) = by.iter().max_by_key(|(id, v)| (v.len(), **id)) {
            let share = ks.len() as f64 / throws.len() as f64;
            if share >= 0.28 && ks.len() >= 5 {
                let yds: i32 = ks.iter().filter(|&&k| s(k).complete).map(|&k| s(k).yards as i32).sum();
                all.push(Hint {
                    key: format!("tgt:{id}"),
                    kind: "Habit",
                    unit: "their offense",
                    text: format!("{} drew {} of {} targets for {yds} yards.", name(n, id), ks.len(), throws.len()),
                    tip:
                        "Their quarterback looks for him first: bracket him on 3rd down and make someone else beat you."
                            .into(),
                    hit: ks.len() as u32,
                    of: throws.len() as u32,
                    strength: ks.len() as f64 * 1.1,
                    snaps: ks.clone(),
                    effect: Effect::Bracket(id),
                });
            }
        }
    }
    let at: Vec<usize> = def.iter().copied().filter(|&k| s(k).kind == 1 && s(k).defender.is_some()).collect();
    let mut cov_by: HashMap<u32, Vec<usize>> = HashMap::new();
    for &k in &at {
        cov_by.entry(s(k).defender.unwrap()).or_default().push(k);
    }
    for (id, ks) in cov_by {
        let m = ks.len() as u32;
        if m < 4 {
            continue;
        }
        let c = ks.iter().filter(|&&k| s(k).complete).count() as u32;
        let y: i32 = ks.iter().filter(|&&k| s(k).complete).map(|&k| s(k).yards as i32).sum();
        if (c as f64) < 0.7 * m as f64 && y < 70 {
            continue;
        }
        let cb = pos(n, id) == "CB";
        all.push(Hint {
            key: format!("cov:{id}"),
            kind: "Battle",
            unit: "their defense",
            text: format!(
                "{} ({}) gave up {} on {m} throws at him for {y} yards.",
                name(n, id),
                pos(n, id),
                count(c as usize, "catch", "catches")
            ),
            tip: if cb {
                "Pick on him: put your best receiver on his side.".into()
            } else {
                "Get your tight end or back matched on him.".into()
            },
            hit: c,
            of: m,
            strength: c as f64 * 1.2 + y as f64 / 40.0,
            snaps: ks,
            effect: if cb { Effect::TargetCb(id) } else { Effect::Note },
        });
    }
    // A quarterback who runs.
    let qb = off.iter().filter_map(|&k| s(k).passer).fold(HashMap::new(), |mut m: HashMap<u32, u32>, id| {
        *m.entry(id).or_default() += 1;
        m
    });
    if let Some((&qid, _)) = qb.iter().max_by_key(|(id, c)| (**c, **id)) {
        let runs: Vec<usize> = off.iter().copied().filter(|&k| s(k).carrier == Some(qid) && !s(k).sack).collect();
        let yds: i32 = runs.iter().map(|&k| s(k).yards as i32).sum();
        let sc = runs.iter().filter(|&&k| s(k).scramble).count();
        if runs.len() >= 5 || yds >= 50 {
            all.push(Hint {
                key: format!("qbrun:{qid}"),
                kind: "Habit",
                unit: "their offense",
                text: format!(
                    "Their quarterback {} ran {} for {yds} yards ({}).",
                    name(n, qid),
                    count(runs.len(), "time", "times"),
                    count(sc, "scramble", "scrambles")
                ),
                tip: "Spy him on passing downs and keep your rush lanes: don't let him escape outside.".into(),
                hit: runs.len() as u32,
                of: runs.len() as u32,
                strength: runs.len() as f64 * 0.9 + yds as f64 / 30.0,
                snaps: runs,
                effect: Effect::Spy,
            });
        }
    }
    // Special teams: a returner who brings everything out.
    let kicks: Vec<usize> = (0..t.snaps.len())
        .filter(|&k| s(k).kind == 4 && s(k).off != side && s(k).returner.is_some_and(|r| n.contains_key(&r)))
        .collect();
    if kicks.len() >= 3 {
        let out: Vec<usize> = kicks.iter().copied().filter(|&k| !s(k).touchback && !s(k).fair).collect();
        let share = out.len() as f64 / kicks.len() as f64;
        if share >= 0.75 {
            let avg = out.iter().map(|&k| s(k).ret_yards as f64).sum::<f64>() / out.len().max(1) as f64;
            let rid = s(kicks[0]).returner.unwrap();
            all.push(Hint {
                key: format!("kr:{rid}"),
                kind: "Special teams",
                unit: "their kicking game",
                text: format!(
                    "Their kick returner {} brought out {} of {} kickoffs, {avg:.0} yards a return.",
                    name(n, rid),
                    out.len(),
                    kicks.len()
                ),
                tip: "Kick it high and short of the goal line: he brings everything out, and your coverage gets there first."
                    .into(),
                hit: out.len() as u32,
                of: kicks.len() as u32,
                strength: kicks.len() as f64 * share,
                snaps: kicks,
                effect: Effect::KickAway,
            });
        }
    }
    let fgs: Vec<usize> = (0..t.snaps.len()).filter(|&k| s(k).kind == 3 && s(k).off == side).collect();
    let missed = fgs.iter().filter(|&&k| !s(k).good).count() as u32;
    if missed >= 2 {
        all.push(Hint {
            key: "fg:miss".into(),
            kind: "Special teams",
            unit: "their kicking game",
            text: format!("Their kicker missed {missed} of {} field goals.", fgs.len()),
            tip: "Make them kick: a stop at your 30 is worth more against him than most.".into(),
            hit: missed,
            of: fgs.len() as u32,
            strength: missed as f64 * 1.5,
            snaps: fgs,
            effect: Effect::Note,
        });
    }
    // The head coach on 4th down.
    let fourth: Vec<usize> =
        (0..t.snaps.len()).filter(|&k| s(k).off == side && s(k).down == 4 && !s(k).null && s(k).kind <= 3).collect();
    if fourth.len() >= 3 {
        let go: Vec<usize> = fourth.iter().copied().filter(|&k| s(k).kind <= 1).collect();
        if go.len() >= 2 && go.len() * 2 >= fourth.len() {
            all.push(Hint {
                key: "hc:4th".into(),
                kind: "Head coach",
                unit: "their offense",
                text: format!("They went for it on {} of {} fourth downs.", go.len(), fourth.len()),
                tip: "Expect them to go from midfield in: keep your defense ready on 4th and short, don't sub in the punt-return team early."
                    .into(),
                hit: go.len() as u32,
                of: fourth.len() as u32,
                strength: go.len() as f64 * 1.2,
                snaps: go,
                effect: Effect::Note,
            });
        }
    }
    all.sort_by(|a, b| b.strength.total_cmp(&a.strength).then(a.key.cmp(&b.key)));
    // At most two "for your eyes" notes, so a tape whose strongest finds
    // are all lost battles still shows the best one the plan can answer.
    let mut notes = 0;
    all.retain(|h| {
        let note = h.effect == Effect::Note;
        notes += note as u32;
        !note || notes <= 2
    });
    all.truncate(3);
    all
}

fn capital(s: &str) -> String {
    let mut c = s.chars();
    c.next().map_or(String::new(), |f| f.to_uppercase().collect::<String>() + c.as_str())
}

/// The play they called most on these snaps, and how often.
fn favourite(ks: &[usize], t: &Tape) -> Option<(String, u32)> {
    favourite_snaps(ks, t).map(|x| (x.0, x.1))
}

fn favourite_snaps(ks: &[usize], t: &Tape) -> Option<(String, u32, Vec<usize>)> {
    let mut m: HashMap<&str, Vec<usize>> = HashMap::new();
    for &k in ks {
        let p = t.snaps[k].play.as_str();
        if !p.is_empty() {
            m.entry(p).or_default().push(k);
        }
    }
    m.into_iter()
        .max_by(|a, b| a.1.len().cmp(&b.1.len()).then(b.0.cmp(a.0)))
        .map(|(p, v)| (p.to_string(), v.len() as u32, v))
}

// ------------------------------------------------------------ the staff's report

/// One line of your staff's report, with its sample and how sure.
fn line(text: String, n: u32, tagged: f64) -> Value {
    let seen = n as f64 * tagged;
    let sure = if seen >= 40.0 {
        "sure"
    } else if seen >= 15.0 {
        "fairly sure"
    } else {
        "a small sample"
    };
    json!({ "text": text, "n": n, "sure": sure })
}

/// The staff's tagging: a share they mis-tag by their film study (a top
/// staff about 3%, a poor one about 20%; proposed, not confirmed), as a
/// count off by that much on each line.
struct Tagger {
    seed: u32,
    miss: f64,
}

impl Tagger {
    fn count(&self, key: &str, k: u32, n: u32) -> u32 {
        if n == 0 {
            return 0;
        }
        let h = key.bytes().fold(self.seed, |a, b| a.rotate_left(5) ^ b as u32).wrapping_mul(0x9E37_79B9);
        let mut r = Rng::new(h);
        let sd = (n as f64 * self.miss).sqrt();
        (k as f64 + r.normal(0.0, sd)).round().clamp(0.0, n as f64) as u32
    }
    fn pct(&self, key: &str, k: u32, n: u32) -> u32 {
        pct(self.count(key, k, n), n)
    }
}

/// Your staff's report on `team` from its tapes this season (newest
/// first, up to four), as good as the staff's film study (1-20).
pub fn report(tapes: &[&Tape], team: usize, film_study: f64, seed: u32, n: &Names, out: Vec<Value>) -> Value {
    let used: Vec<&Tape> = tapes.iter().take(4).copied().collect();
    let tag = Tagger { seed: seed ^ team as u32, miss: (0.25 - 0.011 * film_study).clamp(0.02, 0.22) };
    // How many snaps the staff tags well: a full staff all of them.
    let tagged = (0.55 + 0.025 * film_study).clamp(0.5, 1.0);
    let mut off: Vec<&Snap> = Vec::new();
    let mut def: Vec<&Snap> = Vec::new();
    let mut kicks: Vec<&Snap> = Vec::new();
    let mut all_own: Vec<&Snap> = Vec::new();
    for t in &used {
        let Some(side) = t.side_of(team) else { continue };
        for s in &t.snaps {
            if s.off == side {
                all_own.push(s);
            }
            if s.null {
                continue;
            }
            match (s.kind, s.off == side) {
                (0 | 1, true) => off.push(s),
                (0 | 1, false) => def.push(s),
                (4, false) => kicks.push(s),
                _ => {}
            }
        }
    }
    let mut sections = Vec::new();
    // Their offense: run and pass by down and distance.
    let mut lines = Vec::new();
    let split = |name: &str, f: &dyn Fn(&Snap) -> bool, lines: &mut Vec<Value>| {
        let v: Vec<&&Snap> = off.iter().filter(|s| f(s)).collect();
        let m = v.len() as u32;
        if m >= 3 {
            let p = v.iter().filter(|s| s.kind == 1).count() as u32;
            let q = tag.pct(name, p, m);
            lines.push(line(format!("{name}: pass {q}%, run {}%", 100 - q), m, tagged));
        }
    };
    split("1st and 10", &|s| s.down == 1 && s.togo >= 10, &mut lines);
    split("2nd and short (1-3)", &|s| s.down == 2 && s.togo <= 3, &mut lines);
    split("2nd and long (7+)", &|s| s.down == 2 && s.togo >= 7, &mut lines);
    split("3rd and short (1-3)", &|s| s.down == 3 && s.togo <= 3, &mut lines);
    split("3rd and medium (4-6)", &|s| s.down == 3 && (4..=6).contains(&s.togo), &mut lines);
    split("3rd and long (7+)", &|s| s.down == 3 && s.togo >= 7, &mut lines);
    if !lines.is_empty() {
        sections.push(json!({ "title": "Run and pass by down", "unit": "Their offense", "lines": lines }));
    }
    // By personnel.
    let mut pers: HashMap<&str, (u32, u32)> = HashMap::new();
    for s in &off {
        let p = s.look.split_whitespace().next().unwrap_or("");
        if !p.is_empty() {
            let e = pers.entry(p).or_default();
            e.0 += 1;
            e.1 += (s.kind == 0) as u32;
        }
    }
    let mut pv: Vec<(&str, (u32, u32))> = pers.into_iter().collect();
    pv.sort_by(|a, b| b.1.0.cmp(&a.1.0).then(a.0.cmp(b.0)));
    let total = off.len().max(1) as u32;
    let lines: Vec<Value> = pv
        .iter()
        .take(4)
        .map(|(p, (m, r))| {
            let q = tag.pct(&format!("pers{p}"), *r, *m);
            line(format!("{p} personnel on {}% of snaps: run {q}%, pass {}%", pct(*m, total), 100 - q), *m, tagged)
        })
        .collect();
    if !lines.is_empty() {
        sections.push(json!({ "title": "By personnel", "unit": "Their offense", "lines": lines }));
    }
    // Favourite plays by situation.
    let mut lines = Vec::new();
    let favs = |name: &str, f: &dyn Fn(&Snap) -> bool, lines: &mut Vec<Value>| {
        let v: Vec<&&Snap> = off.iter().filter(|s| f(s)).collect();
        let mut m: HashMap<&str, u32> = HashMap::new();
        for s in &v {
            if !s.play.is_empty() {
                *m.entry(s.play.as_str()).or_default() += 1;
            }
        }
        let mut top: Vec<(&str, u32)> = m.into_iter().filter(|x| x.1 >= 2).collect();
        top.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
        if !top.is_empty() {
            let t: Vec<String> = top.iter().take(3).map(|(p, k)| format!("{p} ({k})")).collect();
            lines.push(line(format!("{name}: {}", t.join(", ")), v.len() as u32, tagged));
        }
    };
    favs("On 3rd down", &|s| s.down == 3, &mut lines);
    favs("In the red zone", &|s| s.yl <= 20, &mut lines);
    favs("In the two-minute drill", &|s| (s.q == 2 || s.q == 4) && s.clock <= 120, &mut lines);
    favs("After a big play (1st down past midfield)", &|s| s.down == 1 && s.yl <= 50, &mut lines);
    if !lines.is_empty() {
        sections.push(json!({ "title": "Favourite calls", "unit": "Their offense", "lines": lines }));
    }
    // Explosive plays.
    let big: Vec<&&Snap> = off.iter().filter(|s| s.yards >= 20 && (s.kind == 0 || s.complete)).collect();
    if !off.is_empty() {
        let p = big.iter().filter(|s| s.kind == 1).count();
        let mut lines = vec![line(
            format!(
                "{} of 20+ yards in {} ({}, {})",
                count(big.len(), "play", "plays"),
                count(used.len(), "game", "games"),
                count(p, "pass", "passes"),
                count(big.len() - p, "run", "runs")
            ),
            off.len() as u32,
            tagged,
        )];
        let pa = off.iter().filter(|s| s.play_action).count() as u32;
        lines.push(line(
            format!("Play action on {}% of snaps", tag.pct("pa", pa, off.len() as u32)),
            off.len() as u32,
            tagged,
        ));
        sections.push(json!({ "title": "Big plays", "unit": "Their offense", "lines": lines }));
    }
    // Their defense.
    if !def.is_empty() {
        let m = def.len() as u32;
        let mut cov: HashMap<&str, u32> = HashMap::new();
        for s in &def {
            if s.kind == 1 && !s.coverage.is_empty() {
                *cov.entry(s.coverage.as_str()).or_default() += 1;
            }
        }
        let passes = def.iter().filter(|s| s.kind == 1).count() as u32;
        let mut cv: Vec<(&str, u32)> = cov.into_iter().collect();
        cv.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
        let mut lines: Vec<Value> = cv
            .iter()
            .take(5)
            .map(|(c, k)| line(format!("{c}: {}% of pass snaps", tag.pct(c, *k, passes)), passes, tagged))
            .collect();
        let man = def.iter().filter(|s| s.kind == 1 && s.man).count() as u32;
        lines.push(line(format!("Man coverage on {}% of pass snaps", tag.pct("man", man, passes)), passes, tagged));
        sections.push(json!({ "title": "Coverages", "unit": "Their defense", "lines": lines }));
        let mut lines = Vec::new();
        for (name, d) in [("1st down", 1u8), ("2nd down", 2), ("3rd down", 3)] {
            let v: Vec<&&Snap> = def.iter().filter(|s| s.down == d).collect();
            if v.len() >= 3 {
                let b = v.iter().filter(|s| s.blitz).count() as u32;
                let two = v.iter().filter(|s| s.kind == 1 && two_high(&s.coverage)).count() as u32;
                let vp = v.iter().filter(|s| s.kind == 1).count() as u32;
                lines.push(line(
                    format!(
                        "{name}: blitz {}%, two deep safeties on {}% of throws",
                        tag.pct(&format!("b{d}"), b, v.len() as u32),
                        tag.pct(&format!("t{d}"), two, vp)
                    ),
                    v.len() as u32,
                    tagged,
                ));
            }
        }
        let mut press: HashMap<&str, u32> = HashMap::new();
        for s in def.iter().filter(|s| s.blitz) {
            if !s.pressure.is_empty() {
                *press.entry(s.pressure.as_str()).or_default() += 1;
            }
        }
        let mut pv: Vec<(&str, u32)> = press.into_iter().collect();
        pv.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
        if let Some((p, k)) = pv.first() {
            lines.push(line(format!("Favourite pressure: {p} ({})", count(*k as usize, "time", "times")), m, tagged));
        }
        let runs: Vec<&&Snap> = def.iter().filter(|s| s.kind == 0).collect();
        if !runs.is_empty() {
            let avg = runs.iter().map(|s| s.yards as f64).sum::<f64>() / runs.len() as f64;
            lines.push(line(format!("Runs against them: {avg:.1} yards a carry"), runs.len() as u32, tagged));
        }
        sections.push(json!({ "title": "Pressure and run defense", "unit": "Their defense", "lines": lines }));
    }
    // The head coach.
    {
        let fourth: Vec<&&Snap> = all_own.iter().filter(|s| s.down == 4 && !s.null && s.kind <= 3).collect();
        let mut lines = Vec::new();
        if !fourth.is_empty() {
            let go = fourth.iter().filter(|s| s.kind <= 1).count();
            let punt = fourth.iter().filter(|s| s.kind == 2).count();
            lines.push(line(
                format!("4th downs: went for it {go}, punted {punt}, kicked {}", fourth.len() - go - punt),
                fourth.len() as u32,
                1.0,
            ));
        }
        let two = all_own.iter().filter(|s| s.kind == 6).count();
        if two > 0 {
            lines.push(line(format!("Two-point tries: {two}"), two as u32, 1.0));
        }
        if !lines.is_empty() {
            sections.push(json!({ "title": "The head coach", "unit": "Their sideline", "lines": lines }));
        }
    }
    // Special teams.
    {
        let mut lines = Vec::new();
        let mut ret: HashMap<u32, (u32, u32, i32)> = HashMap::new();
        for s in &kicks {
            if let Some(r) = s.returner.filter(|r| n.contains_key(r)) {
                let e = ret.entry(r).or_default();
                e.0 += 1;
                if !s.touchback && !s.fair {
                    e.1 += 1;
                    e.2 += s.ret_yards as i32;
                }
            }
        }
        if let Some((&id, &(k, o, y))) = ret.iter().max_by_key(|x| (x.1.0, *x.0)) {
            lines.push(line(
                format!(
                    "Kick returns: {} brought out {o} of {k}, {} yards a return",
                    name(n, id),
                    if o > 0 { y / o as i32 } else { 0 }
                ),
                k,
                1.0,
            ));
        }
        let fgs: Vec<&&Snap> = all_own.iter().filter(|s| s.kind == 3).collect();
        if !fgs.is_empty() {
            let good = fgs.iter().filter(|s| s.good).count();
            lines.push(line(format!("Field goals: {good} of {}", fgs.len()), fgs.len() as u32, 1.0));
        }
        if !lines.is_empty() {
            sections.push(json!({ "title": "Special teams", "unit": "Their kicking game", "lines": lines }));
        }
    }
    // Matchups: who lost up front, who won.
    {
        let mut pro: HashMap<u32, (u32, u32, u32)> = HashMap::new();
        let mut rush: HashMap<u32, u32> = HashMap::new();
        for t in &used {
            let Some(side) = t.side_of(team) else { continue };
            for &(sd, id, reps, a, b, c, pr) in &t.protection {
                if sd == side {
                    let e = pro.entry(id).or_default();
                    e.0 += reps as u32;
                    e.1 += (a + b + c) as u32;
                    e.2 += pr as u32;
                }
            }
            for &(sd, id, a, b, c) in &t.rush {
                if sd == side {
                    *rush.entry(id).or_default() += (a + b + c) as u32;
                }
            }
        }
        let mut lines = Vec::new();
        let mut pv: Vec<(u32, (u32, u32, u32))> = pro.into_iter().filter(|x| x.1.0 >= 10).collect();
        pv.sort_by(|a, b| (b.1.1 as f64 / b.1.0 as f64).total_cmp(&(a.1.1 as f64 / a.1.0 as f64)).then(a.0.cmp(&b.0)));
        for (id, (reps, lost, pr)) in pv.into_iter().take(2) {
            if lost >= 2 {
                lines.push(json!({
                    "text": format!("{} ({}) lost {lost} of {reps} pass-protection reps, {pr} pressures allowed", name(n, id), pos(n, id)),
                    "n": reps, "sure": if reps >= 60 { "sure" } else { "fairly sure" }, "id": id,
                }));
            }
        }
        let mut rv: Vec<(u32, u32)> = rush.into_iter().filter(|x| x.1 >= 2).collect();
        rv.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        for (id, k) in rv.into_iter().take(2) {
            lines.push(json!({ "text": format!("{} ({}) won {k} quick pass rushes", name(n, id), pos(n, id)), "n": k, "sure": "fairly sure", "id": id }));
        }
        if !lines.is_empty() {
            sections.push(json!({ "title": "Matchups", "unit": "Players", "lines": lines }));
        }
    }
    if !out.is_empty() {
        sections.push(json!({ "title": "Who is out", "unit": "Their injury report", "lines": out }));
    }
    let words = if film_study >= 15.0 {
        "a sharp quality-control staff: few tags wrong"
    } else if film_study >= 10.0 {
        "a solid staff: some tags wrong"
    } else {
        "a thin staff: tags often wrong, read the lines as rough"
    };
    json!({
        "games": used.len(),
        "snaps": off.len() + def.len(),
        "staff": words,
        "sections": sections,
        "thin": used.len() < 2,
    })
}

/// "1 game", "3 games".
fn count(k: usize, one: &str, many: &str) -> String {
    format!("{k} {}", if k == 1 { one } else { many })
}
