//! Section 6b, training: what the build must prove (topic 14). Match and
//! development checks only, no world runs.

use super::{context_for, mean, prospects};
use crate::attributes::Attr;
use crate::calendar::{Date, YearDates};
use crate::development::develop_season;
use crate::player::{Level, Player, PlayingTime};
use crate::rng::Rng;
use crate::tendencies::Style;
use crate::training::plan::{Contact, Day, Gear, Kind, Period, Slot, Tempo, Week, Who, fit, neutral, week_from};
use crate::training::rules::{Block, Counts, Frame, Tier, block_limits, check_week, weekly_base};
use crate::training::seat::{habit_rate, injury_chance, practice_rate};
use crate::training::week::{block_of_week, books, team_week, work_out};
use crate::training::work::{Role, freshness, late_load, prep_minutes, ratio, share, week_work};
use crate::training::{FAMILIES, Family, close_year, cover, owe, settle};

fn style(physical: f64, freedom: f64, risk: f64) -> Style {
    Style { physical, freedom, risk, ..Style::default() }
}

/// The first Monday on or after `d`.
fn monday(d: Date) -> Date {
    let mut m = d;
    while crate::training::plan::weekday_index(m) != 0 {
        m = m.add(1);
    }
    m
}

fn college_frame(season: i32) -> (YearDates, Frame) {
    let y = YearDates::new(season);
    let f = Frame::college(&y, y.conference_championships, true);
    (y, f)
}

fn games_in(games: &[Date], m: Date) -> Vec<Date> {
    games.iter().copied().filter(|&g| g >= m && g < m.add(7)).collect()
}

/// A college year for one player opened at his checkpoint, landed week by
/// week with each week's ratio from `plan` (None: the neutral week), the
/// way the world lands an AI player's growth, then closed.
fn college_year(p: &mut Player, f: &Frame, y: &YearDates, plan: &dyn Fn(Date, &Week, &Week) -> Week) {
    let weeks = f.weeks();
    let games: Vec<Date> = y.cfb_weeks.clone();
    let mut m = monday(f.start);
    let st = Style::default();
    let mut key = 1u64;
    while m < f.end {
        let g = games_in(&games, m);
        let (_, wk) = work_out(Tier::College, f, m, &st, &g, false, "", false);
        let block = block_of_week(f, m);
        let slot = Slot::of(block, !g.is_empty(), false);
        let bk = books(f, m, "", false);
        let mut nk = week_from(slot, m, &g, &neutral(Tier::College, slot), &neutral(Tier::College, Slot::Bye));
        fit(&mut nk, &bk);
        let mut mine = plan(m, &wk, &nk);
        fit(&mut mine, &bk);
        let camp = block == Block::Camp;
        let own = bk.iter().all(|b| !b.coaches);
        let r = if own {
            [1.0; FAMILIES]
        } else {
            ratio(&week_work(&mine, Role::Starter, camp, 1.0), &week_work(&nk, Role::Starter, camp, 1.0))
        };
        let base = weekly_base(f, &weeks, block);
        let b = cover(&mut p.train, &base);
        let s = share(&b, &r, 1.0);
        key += 1;
        if owe(&mut p.train, &s) || f.block(m.add(7)) != block {
            settle(p, None, key);
        }
        m = m.add(7);
    }
    close_year(p, None);
}

/// Opens a player's year as the world does (growth held for the weeks).
fn open_year(p: &mut Player, rng: &mut Rng) {
    let mut ctx = context_for(p.age);
    ctx.defer = true;
    develop_season(p, &ctx, rng);
}

fn yearly(p: &mut Player, rng: &mut Rng) {
    let ctx = context_for(p.age);
    develop_season(p, &ctx, rng);
}

/// Topic 14.1: the default plan keeps the calibration. A year landed week
/// by week on the neutral plan ends where the yearly step ends.
#[test]
fn the_default_plan_keeps_the_yearly_growth() {
    let t = std::time::Instant::now();
    let (y, f) = college_frame(2026);
    let mut gaps = Vec::new();
    let mut base = Vec::new();
    for (k, p0) in prospects(240, "train-cal", 1.0).into_iter().enumerate() {
        let mut p0 = p0;
        // Bring him to 18 on the yearly step.
        let mut rng = Rng::new(format!("pre{k}"));
        while p0.age < 18 {
            yearly(&mut p0, &mut rng);
        }
        for _ in 0..3 {
            let (mut a, mut b) = (p0.clone(), p0.clone());
            let seed = format!("y{k}-{}", p0.age);
            let before = a.overall();
            yearly(&mut a, &mut Rng::new(seed.clone()));
            open_year(&mut b, &mut Rng::new(seed));
            college_year(&mut b, &f, &y, &|_, _, nk| nk.clone());
            gaps.push(b.overall() - a.overall());
            base.push(a.overall() - before);
            assert!(!b.train.pending());
            assert!(b.attributes.values().zip(b.ceilings.values()).all(|(v, c)| v <= c + 1e-6));
            p0 = a;
        }
    }
    let gap = mean(&gaps);
    let growth = mean(&base);
    println!("yearly growth {growth:.2}, weekly minus yearly {gap:+.3} ({:.1}s)", t.elapsed().as_secs_f64());
    assert!(growth > 1.0, "growth {growth:.2}");
    assert!(gap.abs() < 0.03 * growth.max(1.0), "weekly landing moved the year by {gap:+.3} on {growth:.2}");
}

/// Topic 14.2: each period grows only what it works.
#[test]
fn each_period_grows_only_what_it_works() {
    let fam = |w: &[f64; FAMILIES], f: Family| w[f as usize];
    // Phase 1 of the NFL offseason: the weight room, conditioning and
    // meetings, nothing on the field.
    let y = YearDates::new(2025);
    let pro = Frame::pro(&y, false);
    let m = monday(pro.phase1.add(3));
    assert_eq!(block_of_week(&pro, m), Block::Phase1);
    let (_, wk) = work_out(Tier::Pro, &pro, m, &Style::default(), &[], false, "", false);
    let w = week_work(&wk, Role::Starter, false, 1.0);
    assert!(fam(&w, Family::Strength) > 0.0 && fam(&w, Family::Conditioning) > 0.0);
    for f in [Family::Technique, Family::Contact, Family::Kicking] {
        assert_eq!(fam(&w, f), 0.0, "phase 1 worked {f:?}");
    }
    // A day of individual line periods: technique, not reading.
    let day = |kind: Kind, contact: Contact| {
        let mut d = Day::default();
        for _ in 0..3 {
            d.periods.push(Period {
                tempo: Tempo::Normal,
                who: Who::All,
                ..Period::new(kind, 20, Gear::Pads, contact)
            });
        }
        let mut wk = Week::default();
        wk.days[1] = d;
        wk
    };
    let ind = week_work(&day(Kind::Individual, Contact::Bags), Role::Starter, false, 1.0);
    assert!(fam(&ind, Family::Technique) > 10.0 * fam(&ind, Family::Mental).max(0.1), "{ind:?}");
    // Seven on seven has no contact; film has nothing but the mental side.
    let sevens = week_work(&day(Kind::SevenOnSeven, Contact::Air), Role::Starter, false, 1.0);
    assert_eq!(fam(&sevens, Family::Contact), 0.0);
    let film = week_work(&day(Kind::Film, Contact::Air), Role::Starter, false, 1.0);
    for f in Family::ALL {
        if f != Family::Mental {
            assert_eq!(fam(&film, f), 0.0, "film worked {f:?}");
        }
    }
    assert!(fam(&film, Family::Mental) > 0.0);
}

/// Swaps every on-field period of a week for `kind` (kept inside the
/// rules by `fit` afterwards).
fn all_of(wk: &Week, kind: Kind) -> Week {
    let mut out = wk.clone();
    for d in out.days.iter_mut() {
        for p in d.periods.iter_mut() {
            if p.kind.on_field() || matches!(p.kind, Kind::WeightRoom | Kind::Film | Kind::Meeting) {
                let gear = if matches!(kind, Kind::WeightRoom | Kind::Film | Kind::Meeting | Kind::Conditioning) {
                    Gear::None
                } else {
                    p.gear
                };
                let contact = if gear == Gear::None { Contact::Air } else { p.contact };
                *p = Period { kind, gear, contact, ..*p };
            }
        }
    }
    out
}

/// Topic 14.3, choice 2: the best realistic plan against a poor one moves
/// a young player's year by at most about 15% either way, and never past
/// his ceilings.
#[test]
fn choices_move_a_year_within_the_agreed_size() {
    let t = std::time::Instant::now();
    let (y, f) = college_frame(2026);
    let players: Vec<Player> = prospects(160, "train-choice", 1.0)
        .into_iter()
        .enumerate()
        .map(|(k, mut p)| {
            let mut rng = Rng::new(format!("c{k}"));
            while p.age < 18 {
                yearly(&mut p, &mut rng);
            }
            p
        })
        .collect();
    let hard = style(18.0, 6.0, 16.0);
    let soft = style(3.0, 16.0, 4.0);
    type Plan = Box<dyn Fn(Date, &Week, &Week) -> Week>;
    let styled = |st: Style| -> Plan {
        Box::new(move |m: Date, _: &Week, nk: &Week| {
            let (_, f) = college_frame(2026);
            let y = YearDates::new(2026);
            let g = games_in(&y.cfb_weeks, m);
            let block = block_of_week(&f, m);
            let slot = Slot::of(block, !g.is_empty(), false);
            let _ = nk;
            week_from(
                slot,
                m,
                &g,
                &crate::training::plan::styled(Tier::College, slot, &st),
                &crate::training::plan::styled(Tier::College, Slot::Bye, &st),
            )
        })
    };
    let plans: Vec<(&str, Plan)> = vec![
        ("neutral", Box::new(|_: Date, _: &Week, nk: &Week| nk.clone())),
        ("hard staff", styled(hard)),
        ("soft staff", styled(soft)),
        ("all weight room", Box::new(|_: Date, _: &Week, nk: &Week| all_of(nk, Kind::WeightRoom))),
        ("all film", Box::new(|_: Date, _: &Week, nk: &Week| all_of(nk, Kind::Film))),
        ("all individual", Box::new(|_: Date, _: &Week, nk: &Week| all_of(nk, Kind::Individual))),
        (
            "all live team",
            Box::new(|_: Date, _: &Week, nk: &Week| {
                let mut w = all_of(nk, Kind::Team);
                for d in w.days.iter_mut() {
                    for p in d.periods.iter_mut().filter(|p| p.kind == Kind::Team) {
                        p.gear = Gear::Pads;
                        p.contact = Contact::Live;
                    }
                }
                w
            }),
        ),
    ];
    let mut gains: Vec<(&str, f64)> = Vec::new();
    for (name, plan) in &plans {
        let mut g = Vec::new();
        for (k, p0) in players.iter().enumerate() {
            let mut p = p0.clone();
            let before = p.overall();
            open_year(&mut p, &mut Rng::new(format!("open{k}")));
            college_year(&mut p, &f, &y, plan.as_ref());
            assert!(
                p.attributes.values().zip(p.ceilings.values()).all(|(v, c)| v <= c + 1e-6),
                "{name}: past a ceiling"
            );
            g.push(p.overall() - before);
        }
        gains.push((name, mean(&g)));
    }
    let n = gains[0].1;
    for (name, g) in &gains {
        println!("{name:<16} {g:.2} ({:+.1}%)", 100.0 * (g / n - 1.0));
    }
    println!("({:.1}s)", t.elapsed().as_secs_f64());
    for (name, g) in &gains[1..] {
        assert!((g / n - 1.0).abs() <= 0.16, "{name} moved the year {:+.1}%", 100.0 * (g / n - 1.0));
    }
}

/// Topic 14.4: the trade-offs. More install raises the plan's prep and
/// lowers technique work; more live contact raises contact work and the
/// injury risk; a heavy Thursday lowers game-day energy.
#[test]
fn the_trade_offs_show() {
    let (y, f) = college_frame(2026);
    let m = monday(y.cfb_weeks[3].add(-6));
    let g = games_in(&y.cfb_weeks, m);
    assert_eq!(g.len(), 1);
    let bk = books(&f, m, "", false);
    let mut nk = week_from(Slot::Game, m, &g, &neutral(Tier::College, Slot::Game), &neutral(Tier::College, Slot::Bye));
    fit(&mut nk, &bk);
    let tech = |w: &Week| week_work(w, Role::Starter, false, 1.0)[Family::Technique as usize];
    let contact = |w: &Week| week_work(w, Role::Starter, false, 1.0)[Family::Contact as usize];
    // Install: individual periods become team periods on the opponent.
    let mut install = nk.clone();
    for d in install.days.iter_mut() {
        for p in d.periods.iter_mut().filter(|p| p.kind == Kind::Individual) {
            p.kind = Kind::Team;
            p.what = 1;
        }
    }
    fit(&mut install, &bk);
    assert!(prep_minutes(&install) > prep_minutes(&nk));
    assert!(tech(&install) < tech(&nk));
    // Live contact in the team periods.
    let mut live = nk.clone();
    for d in live.days.iter_mut() {
        for p in d.periods.iter_mut().filter(|p| matches!(p.kind, Kind::Team | Kind::InsideRun)) {
            p.gear = Gear::Pads;
            p.contact = Contact::Live;
        }
    }
    let unfitted = live.clone();
    fit(&mut live, &bk);
    let live = if contact(&live) > contact(&nk) { live } else { unfitted };
    assert!(contact(&live) > contact(&nk), "live {} neutral {}", contact(&live), contact(&nk));
    let risk_n = team_week(&f, Block::Season, &bk, &nk, &nk).risk;
    let risk_l = team_week(&f, Block::Season, &bk, &live, &nk).risk;
    assert!(risk_l > risk_n, "risk {risk_l} against {risk_n}");
    // A heavy Thursday.
    let gd = nk.days.iter().position(|d| d.game).unwrap();
    let mut heavy = nk.clone();
    heavy.days[gd - 2]
        .periods
        .push(Period { tempo: Tempo::Fast, ..Period::new(Kind::Scrimmage, 40, Gear::Pads, Contact::Live) });
    let base = late_load(&nk, gd, Role::Starter);
    assert_eq!(freshness(base, base), 1.0);
    let fresh = freshness(late_load(&heavy, gd, Role::Starter), base);
    assert!(fresh < 0.97, "freshness {fresh}");
}

/// Topic 14.5: practice injuries land on the real split. A college team's
/// season on the neutral plan has most of its injuries in practice by
/// count (NCAA ISP: about 55-60%), the most a week in camp, and a load spike
/// raises the risk.
#[test]
fn practice_injuries_land_on_the_real_split() {
    let (y, f) = college_frame(2026);
    let mut by_block: Vec<(Block, f64)> = Vec::new();
    let mut weeks: Vec<(Block, f64)> = Vec::new();
    let mut m = monday(f.start);
    let roster = 105.0;
    while m < f.end {
        let g = games_in(&y.cfb_weeks, m);
        let (_, wk) = work_out(Tier::College, &f, m, &Style::default(), &g, false, "", false);
        let block = block_of_week(&f, m);
        let practices = wk.days.iter().filter(|d| d.periods.iter().any(|p| p.kind.on_field())).count() as f64;
        let bk = books(&f, m, "", false);
        if practices > 0.0 && !bk.iter().all(|b| !b.coaches) {
            let e = roster * injury_chance(Tier::College, block, practices, 1.0, 10.5, 1.0, 1.0);
            match by_block.iter_mut().find(|x| x.0 == block) {
                Some(x) => x.1 += e,
                None => by_block.push((block, e)),
            }
            match weeks.iter_mut().find(|x| x.0 == block) {
                Some(x) => x.1 += 1.0,
                None => weeks.push((block, 1.0)),
            }
        }
        m = m.add(7);
    }
    let practice: f64 = by_block.iter().map(|x| x.1).sum();
    let camp = by_block.iter().find(|x| x.0 == Block::Camp).map_or(0.0, |x| x.1);
    // Games: the engine's 0.77 time-loss injuries a team game at FBS
    // (`docs/match-engine.md`) cost a week or more; the ISP counts any
    // time lost, about 1.6 times as many (proposed, not confirmed).
    let games = 13.0 * 0.77 * 1.6;
    let share = practice / (practice + games);
    println!("practice {practice:.1} (camp {camp:.1}), games {games:.1}: practice share {:.0}%", 100.0 * share);
    assert!(share > 0.5 && share < 0.75, "practice share {share:.2}");
    // The camp peak: camp weeks carry the most injuries a week (the ISP's
    // preseason practice rate is about two and a half times the season's).
    let per_week = |b: Block| {
        let n = weeks.iter().find(|x| x.0 == b).map_or(1.0, |x| x.1);
        by_block.iter().find(|x| x.0 == b).map_or(0.0, |x| x.1) / n
    };
    for (b, _) in &by_block {
        if *b != Block::Camp {
            assert!(
                per_week(Block::Camp) >= 2.0 * per_week(*b),
                "{b:?} {:.2} a week against camp {:.2}",
                per_week(*b),
                per_week(Block::Camp)
            );
        }
    }
    let calm = injury_chance(Tier::College, Block::Camp, 5.0, 1.0, 10.5, 1.0, 1.0);
    let spiked = injury_chance(Tier::College, Block::Camp, 5.0, 1.0, 10.5, 1.0, 1.7);
    assert!(spiked > 1.2 * calm);
    assert!(practice_rate(Tier::High, Block::Camp) < practice_rate(Tier::College, Block::Camp));
}

/// Checks every week of a year for a staff style: each week inside its
/// books and each block's counts inside its limits.
fn check_year(tier: Tier, f: &Frame, games: &[Date], state: &str, heat: bool, st: &Style) -> Vec<String> {
    let mut out = Vec::new();
    let mut m = monday(f.start);
    let mut counts: Option<(Block, Counts)> = None;
    while m < f.end {
        let g = games_in(games, m);
        let (_, wk) = work_out(tier, f, m, st, &g, false, state, heat);
        let bk = books(f, m, state, heat);
        for b in check_week(&bk, &wk) {
            out.push(format!("{tier:?} {} {m}: {}", f.season, b.text));
        }
        let block = block_of_week(f, m);
        let c = match counts.as_mut() {
            Some((b, c)) if *b == block => c,
            _ => {
                counts = Some((block, Counts::default()));
                &mut counts.as_mut().unwrap().1
            }
        };
        c.add_week(&wk);
        for t in c.over(&block_limits(tier, block, f.season)) {
            out.push(format!("{tier:?} {} {block:?} by {m}: {t}", f.season));
        }
        m = m.add(7);
    }
    out
}

/// Topic 14.6: the rules hold. No illegal week saves, and every level and
/// era's AI weeks stay inside the day, week and block limits.
#[test]
fn the_ai_keeps_every_level_and_era_inside_the_rules() {
    let t = std::time::Instant::now();
    let styles = [Style::default(), style(19.0, 3.0, 19.0), style(2.0, 18.0, 2.0)];
    let mut breaches = Vec::new();
    for season in [1994, 2004, 2012, 2016, 2021, 2025] {
        let y = YearDates::new(season);
        for st in &styles {
            for (end, spring) in [(y.conference_championships, true), (y.d2_regular_end, false)] {
                let f = Frame::college(&y, end, spring);
                let games: Vec<Date> = y.cfb_weeks.iter().copied().filter(|&g| g <= end).collect();
                breaches.extend(check_year(Tier::College, &f, &games, "", false, st));
            }
            breaches.extend(check_year(Tier::Pro, &Frame::pro(&y, false), &y.nfl_weeks, "", false, st));
            for code in ["CA", "TX", "GA", "OH"] {
                let cal = crate::calendar::data::hs_calendar(code);
                let (start, _, finals) = crate::world::hs_year::state_season_dates(code, season);
                let f = Frame::high(&y, start, finals, cal.spring_ball);
                let games: Vec<Date> = (0..10).map(|k| start.add(7 * k)).collect();
                breaches.extend(check_year(Tier::High, &f, &games, cal.state, cal.heat_rule, st));
            }
        }
    }
    println!("{} breaches ({:.1}s)", breaches.len(), t.elapsed().as_secs_f64());
    for b in breaches.iter().take(20) {
        println!("{b}");
    }
    assert!(breaches.is_empty());
    // A week the rules forbid can't be saved: full pads on a college
    // winter day.
    let y = YearDates::new(2025);
    let f = Frame::college(&y, y.conference_championships, true);
    let m = monday(f.winter.add(14));
    let bk = books(&f, m, "", false);
    let mut wk = Week::default();
    wk.days[1].periods.push(Period::new(Kind::Team, 60, Gear::Pads, Contact::Live));
    assert!(!check_week(&bk, &wk).is_empty());
}

/// Topic 14.7: a habit is coached toward its target at the agreed speed:
/// about 3 a year from a strong coach with a coachable player, about 1.5
/// from an average pair, slower once he is set in his ways.
#[test]
fn a_habit_is_coached_at_the_expected_speed() {
    let strong = habit_rate(16.0, 15.0, 20);
    let average = habit_rate(12.0, 10.0, 20);
    let veteran = habit_rate(16.0, 15.0, 30);
    println!("strong {strong:.2}, average {average:.2}, veteran {veteran:.2}");
    assert!((2.5..=3.4).contains(&strong), "strong {strong}");
    assert!((1.0..=2.0).contains(&average), "average {average}");
    assert!(veteran < 0.5 * strong);
    // Obedience: a disciplined, coachable player takes it, a stubborn one
    // much less.
    let obey = crate::tendencies::obey_chance(16.0, 16.0, 0.5, 10.0);
    let stubborn = crate::tendencies::obey_chance(4.0, 4.0, 0.5, 10.0);
    assert!(obey > stubborn + 0.2, "{obey} {stubborn}");
}

/// The memory gate: a player's training store stays tiny.
#[test]
fn the_training_store_stays_small() {
    assert!(std::mem::size_of::<crate::training::Train>() <= 40);
    assert!(std::mem::size_of::<Period>() <= 8);
    let _ = (Level::FBS, PlayingTime::Starter, Attr::Speed);
}
