//! His tape (update 3, lane 9; SK45, SK51, SK52): cut-ups of his own snaps,
//! each drawn as three stills from the match engine's tracking of the snap
//! (the snap, the moment that decides it, the end), like a TV breakdown.
//!
//! The world keeps no tracking of other teams' games, so a player's tape is
//! played on the match engine when you open it: his team as it dresses
//! today against real opponents of his level (his state's schools of his
//! size, his conference, the league), with him in his spot. The seed is his
//! id and the season, so the same tape comes back all week. Your own
//! players' tape is your kept games, played again from their seeds.
//! Nothing here changes the world: it only reads it.

use crate::attributes::Attr;
use crate::game::pose::{Act, BallMode};
use crate::game::sim::Frame;
use crate::game::stats::PlayType;
use crate::game::{GamePlayer, GameSetup, GameTeam, PlayRecord, play_game};
use crate::positions::Position;
use crate::world::season::key_uniform;
use crate::world::{Stage, World};

use super::words;

/// A man on a still: where (0-100 across, 0-100 down the frame), who (0 the
/// other side, 1 his side, 2 him) and a label.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Dot {
    pub x: u8,
    pub y: u8,
    pub k: u8,
    #[serde(default)]
    pub l: String,
}

/// A path on a still (0 his, 1 the man he faced, 2 the ball, 3 a
/// teammate's).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Line {
    pub k: u8,
    pub pts: Vec<(u8, u8)>,
}

/// One still of a snap.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Still {
    pub caption: String,
    pub dots: Vec<Dot>,
    pub lines: Vec<Line>,
    /// A circle on him (the moment).
    pub ring: Option<(u8, u8)>,
    /// The line of scrimmage, down the frame.
    pub los: u8,
}

/// What his job was on a snap.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Role {
    Passer,
    Target,
    Route,
    Carrier,
    PassPro,
    RunBlock,
    Rusher,
    RunStop,
    Cover,
    Zone,
    Kick,
    Coverage,
    Away,
}

/// The tape stops before the moment and asks you to call it (SK48).
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Call {
    pub q: String,
    pub opts: Vec<String>,
    pub right: u8,
    /// What happened, for the line after the call.
    pub truth: String,
}

/// The numbers of one snap, for the analyst (SK57).
#[derive(Clone, Copy, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Stat {
    pub targets: u8,
    pub catches: u8,
    pub yards: i16,
    pub carries: u8,
    pub att: u8,
    pub comp: u8,
    pub sacks: u8,
    pub pressures: u8,
    pub reps: u8,
    pub wins: u8,
    pub tackles: u8,
    pub missed: u8,
    pub pbu: u8,
    pub ints: u8,
    pub td: u8,
}

/// One of his snaps.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Snap {
    /// Which game of the tape.
    pub game: u8,
    /// "Q1 · 3rd & 6".
    pub sit: String,
    pub job: String,
    pub result: String,
    /// The charted line (down, distance, his job, what happened).
    pub chart: String,
    pub stills: Vec<Still>,
    pub role: Role,
    /// How the snap went for him: -1, 0, +1 (the engine's battles).
    pub grade: i8,
    /// What the tape shows of him on it: an attribute and his band (0-4)
    /// against the level, the question's first.
    pub cues: Vec<(Attr, u8)>,
    pub call: Option<Call>,
    pub after: Option<String>,
    /// A snap that jumps out (SK51): a cheap shot, a limp, a big hit on
    /// coverage.
    pub surprise: Option<String>,
    /// His top speed on the snap, yards a second (SK56).
    pub speed: f32,
    pub third: bool,
    pub red: bool,
    pub pressure: bool,
    pub special: bool,
    pub stat: Stat,
    /// Everyone else on the field (SK34: found on another kid's tape).
    pub others: Vec<u32>,
}

/// One game on the tape.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Game {
    pub label: String,
    /// The opponent's strength (its 22 best, rating points).
    pub strength: f32,
}

/// A player's whole tape: his games and his snaps in them.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Tape {
    pub games: Vec<Game>,
    pub snaps: Vec<Snap>,
}

/// The cut-ups (SK45): his snaps only, by situation. No whole-game tape.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Cut {
    ThirdDowns,
    RedZone,
    Pressure,
    BestOpponent,
    WorstGame,
    /// Your own practice and games (SK27): his last snaps for you.
    Ours,
}

impl Cut {
    pub const ALL: [Cut; 5] = [Cut::ThirdDowns, Cut::RedZone, Cut::Pressure, Cut::BestOpponent, Cut::WorstGame];
    pub fn key(self) -> &'static str {
        match self {
            Cut::ThirdDowns => "third",
            Cut::RedZone => "red",
            Cut::Pressure => "pressure",
            Cut::BestOpponent => "best",
            Cut::WorstGame => "worst",
            Cut::Ours => "ours",
        }
    }
    pub fn from_key(k: &str) -> Option<Cut> {
        Cut::ALL.into_iter().chain([Cut::Ours]).find(|c| c.key() == k)
    }
    /// The cut-up's name for a position ("against press" for a corner).
    pub fn words(self, pos: Position) -> &'static str {
        match self {
            Cut::ThirdDowns => "Third downs",
            Cut::RedZone => "Red zone",
            Cut::Pressure => match pos {
                Position::CB | Position::WR => "Against press and man",
                Position::QB | Position::OT | Position::IOL | Position::RB | Position::TE => "Against the blitz",
                Position::S | Position::LB => "In man coverage",
                Position::EDGE | Position::DT => "Against the pass",
                _ => "Under pressure",
            },
            Cut::BestOpponent => "Against his best opponent",
            Cut::WorstGame => "His worst game",
            Cut::Ours => "With us",
        }
    }
}

// --- Building the games ------------------------------------------------------------------------

fn college_level(w: &World, c: usize) -> crate::player::Level {
    use crate::world::data::Division;
    match w.programs[c].division {
        Division::FBS => crate::player::Level::FBS,
        Division::FCS => crate::player::Level::FCS,
        _ => crate::player::Level::SmallCollege,
    }
}

/// His team as it dresses today, with him in it and starting at his spot.
fn his_team(w: &World, i: usize) -> Option<(GameTeam, crate::player::Level, Kin)> {
    let p = &w.people[i];
    let id = p.player.id;
    let (mut team, level, kin) = match p.stage {
        Stage::HighSchool | Stage::PrepSchool => {
            let (t, _) = crate::world::hs_year::film_team(w, p.hs)?;
            (t, crate::player::Level::HS, Kin::School(p.hs))
        }
        Stage::College | Stage::Declared => {
            let c = p.college?;
            (crate::world::season::game_team(w, &w.programs[c])?, college_level(w, c), Kin::Program(c))
        }
        Stage::Pro => {
            let t = p.pro.as_ref().and_then(|c| c.team)?;
            if t >= w.nfl.len() {
                return None;
            }
            (crate::world::pro::games::game_side(w, t)?.team, crate::player::Level::NFL, Kin::Club(t))
        }
        _ => return None,
    };
    // He starts on his own tape: the men listed ahead of him at his spot
    // sit (his tape is his snaps).
    let pos = p.player.position;
    if !team.players.iter().any(|g| g.id == id) {
        let exp = (p.player.age as f64 - 14.0).clamp(0.0, 8.0) * 0.5;
        team.players.push(GamePlayer::from_player(&p.player, 0.8, exp));
    }
    let mine = team.players.iter().find(|g| g.id == id).map(|g| g.overall_at(pos)).unwrap_or(0.0);
    team.players.retain(|g| g.id == id || g.position != pos || g.overall_at(pos) < mine);
    Some((team, level, kin))
}

#[derive(Clone, Copy)]
enum Kin {
    School(usize),
    Program(usize),
    Club(usize),
}

/// Three real opponents of his level, each with its name and team.
fn opponents(w: &World, kin: Kin, salt: u64) -> Vec<(String, GameTeam)> {
    let mut picks: Vec<usize> = match kin {
        Kin::School(h) => {
            let me = &w.high_schools[h];
            (0..w.high_schools.len())
                .filter(|&o| {
                    o != h
                        && w.high_schools[o].state == me.state
                        && (w.high_schools[o].classification as i64 - me.classification as i64).abs() <= 1
                        && !w.high_schools[o].roster.is_empty()
                })
                .collect()
        }
        Kin::Program(c) => {
            let me = &w.programs[c];
            let conf: Vec<usize> =
                (0..w.programs.len()).filter(|&o| o != c && w.programs[o].conference == me.conference).collect();
            if conf.len() >= 3 {
                conf
            } else {
                (0..w.programs.len()).filter(|&o| o != c && w.programs[o].division == me.division).collect()
            }
        }
        Kin::Club(t) => (0..w.nfl.len()).filter(|&o| o != t).collect(),
    };
    // A fixed draw of three, by the season and him.
    picks.sort_by(|a, b| key_uniform(salt, *a as u64).total_cmp(&key_uniform(salt, *b as u64)));
    let mut out = Vec::new();
    for o in picks {
        if out.len() >= 3 {
            break;
        }
        let got = match kin {
            Kin::School(_) => crate::world::hs_year::film_team(w, o).map(|(t, _)| (w.school_name(o).to_string(), t)),
            Kin::Program(_) => {
                crate::world::season::game_team(w, &w.programs[o]).map(|t| (w.program_name(o).to_string(), t))
            }
            Kin::Club(_) => crate::world::pro::games::game_side(w, o).map(|s| (w.nfl[o].name.to_string(), s.team)),
        };
        if let Some(g) = got {
            out.push(g);
        }
    }
    out
}

/// A team's strength: its 22 best at their own spots.
fn strength(t: &GameTeam) -> f32 {
    let mut v: Vec<f64> = t.players.iter().map(|g| g.overall_at(g.position)).collect();
    v.sort_by(|a, b| b.total_cmp(a));
    let n = v.len().clamp(1, 22);
    (v.iter().take(n).sum::<f64>() / n as f64) as f32
}

/// A player's tape: three games on the engine with frames on, his snaps
/// in them. Err when there is no film of him.
pub fn player_tape(w: &World, id: u32) -> Result<Tape, String> {
    let i = super::super::place(w, id).ok_or("No such player.")?;
    let p = &w.people[i];
    let f = super::super::footage::footage(w, i);
    match p.stage {
        Stage::HighSchool | Stage::PrepSchool if f.snaps <= 0.0 && !f.reels => {
            return Err("There's no film of him yet: he hasn't played a high school game.".into());
        }
        Stage::College | Stage::Declared if f.snaps <= 0.0 => {
            return Err("He hasn't played a college snap: there's nothing to watch.".into());
        }
        Stage::HighSchool | Stage::PrepSchool | Stage::College | Stage::Declared | Stage::Pro => {}
        _ => return Err("There's no film of him.".into()),
    }
    let (team, level, kin) = his_team(w, i).ok_or("His team's film isn't to be had just now.")?;
    let season = w.dates.season;
    let salt = 0xF11A_0000 ^ (id as u64) << 20 ^ season as u64;
    let opps = opponents(w, kin, salt);
    if opps.is_empty() {
        return Err("His team's film isn't to be had just now.".into());
    }
    let mut tape = Tape::default();
    let week_word = |k: usize| match level {
        crate::player::Level::NFL => format!("Wk {}", 3 + 4 * k),
        _ => format!("Wk {}", 2 + 3 * k),
    };
    for (k, (name, opp)) in opps.iter().enumerate() {
        let home = k % 2 == 0;
        let (h, a) = if home { (&team, opp) } else { (opp, &team) };
        let mut s = GameSetup::new(h, a, level, salt.wrapping_mul(31).wrapping_add(k as u64));
        s.record = true;
        s.year = season;
        let res = play_game(&s);
        let side = if home { 0 } else { 1 };
        let label = format!("{} {} {}", week_word(k), if home { "vs" } else { "at" }, name);
        tape.games.push(Game { label, strength: strength(opp) });
        collect(&mut tape, k as u8, p, side, [h, a], &res, p.stage);
    }
    if tape.snaps.is_empty() {
        return Err("He didn't get on the field in the film there is.".into());
    }
    Ok(tape)
}

/// Your own player's tape (SK27): your kept games, played again from
/// their seeds with frames on.
pub fn our_tape(w: &World, id: u32) -> Result<Tape, String> {
    let i = super::super::place(w, id).ok_or("No such player.")?;
    let p = &w.people[i];
    let mut tape = Tape::default();
    let games: Vec<&crate::world::seat::SeatGame> =
        w.seat_games().iter().rev().filter(|g| !g.replay.is_empty()).take(3).collect();
    for (k, g) in games.iter().enumerate() {
        let Some(rs) = crate::game::ReplaySetup::unpack(&g.replay) else { continue };
        let side = if rs.home.players.iter().any(|x| x.id == id) {
            0
        } else if rs.away.players.iter().any(|x| x.id == id) {
            1
        } else {
            continue;
        };
        let res = rs.play(true);
        let opp = 1 - side;
        let label = format!("{} {} {}", g.date.pretty(), if side == 0 { "vs" } else { "at" }, g.names[opp]);
        let st = strength(if opp == 0 { &rs.home } else { &rs.away });
        tape.games.push(Game { label, strength: st });
        collect(&mut tape, k as u8, p, side, [&rs.home, &rs.away], &res, p.stage);
    }
    if tape.snaps.is_empty() {
        // Before your first kept game: the practice tape is his own tape.
        return player_tape(w, id);
    }
    Ok(tape)
}

// --- Reading the snaps --------------------------------------------------------------------

/// Every snap of his in one game.
fn collect(
    tape: &mut Tape,
    game: u8,
    p: &crate::world::Person,
    side: usize,
    teams: [&GameTeam; 2],
    res: &crate::game::GameResult,
    stage: Stage,
) {
    let me = p.player.id;
    let pos_of = |id: u32| -> Option<(usize, Position)> {
        for (s, t) in teams.iter().enumerate() {
            if let Some(g) = t.players.iter().find(|g| g.id == id) {
                return Some((s, g.position));
            }
        }
        None
    };
    let mark = words::level_mark(stage);
    for r in &res.stats.plays {
        if r.nullified || matches!(r.kind, PlayType::Kneel | PlayType::Spike | PlayType::NoPlay) {
            continue;
        }
        let Some((_, fr)) = res.frames.iter().find(|x| x.0 == r.n) else { continue };
        if fr.len() < 6 {
            continue;
        }
        let Some(mi) = fr[0].players.iter().position(|x| x.0 == me) else { continue };
        let snap = read_snap(r, fr, mi, me, side, &pos_of, p, mark, game);
        tape.snaps.push(snap);
    }
}

#[allow(clippy::too_many_arguments)]
fn read_snap(
    r: &PlayRecord,
    fr: &[Frame],
    mi: usize,
    me: u32,
    side: usize,
    pos_of: &dyn Fn(u32) -> Option<(usize, Position)>,
    p: &crate::world::Person,
    mark: f64,
    game: u8,
) -> Snap {
    let off = r.offense == side;
    let pos = p.player.position;
    let role = role_of(r, me, off, pos);
    let grade = grade_of(r, me, role);
    let m = moment(fr, role);
    // The man he faced: whom he blocked or was blocked by at the moment,
    // the man he covered or who covered him, else the nearest of theirs.
    let key = facing(r, fr, mi, m, me, role, side, pos_of);
    let dir = direction(fr, r.offense, side, pos_of);
    let los_x = fr[0].ball.x;
    let reach = |k: usize| -> f64 {
        let f = &fr[k.min(fr.len() - 1)];
        let mut d = (f.players[mi].1 as f64 - los_x) * dir;
        if let Some(ki) = key {
            d = d.max((f.players[ki].1 as f64 - los_x) * dir);
        }
        if matches!(role, Role::Passer | Role::Target | Role::Cover) {
            d = d.max((f.ball.x - los_x) * dir);
        }
        d
    };
    let far = reach(m).max(reach(fr.len() - 1)).max(0.0);
    let top = (far + 4.0).clamp(26.0, 38.0);
    let los = (2.0 + top * 2.4).round().clamp(10.0, 96.0) as u8;
    let place = |x: f32, y: f32| -> (u8, u8) {
        let down = (x as f64 - los_x) * dir;
        let vy = (los as f64 - down * 2.4).clamp(1.0, 99.0);
        let across = if dir > 0.0 { 1.0 - y as f64 / 53.33 } else { y as f64 / 53.33 };
        ((across * 100.0).clamp(1.0, 99.0).round() as u8, vy.round() as u8)
    };
    let label = |id: u32, idx: usize| -> String {
        if idx == mi {
            return pos.name().to_string();
        }
        if Some(idx) == key {
            return pos_of(id).map_or("X".into(), |x| x.1.name().to_string());
        }
        match pos_of(id) {
            Some((s, Position::QB)) if s == r.offense => "QB".into(),
            _ => String::new(),
        }
    };
    let dots = |k: usize| -> Vec<Dot> {
        let f = &fr[k];
        f.players
            .iter()
            .enumerate()
            .map(|(idx, &(id, x, y))| {
                let (px, py) = place(x, y);
                let kk = if idx == mi {
                    2
                } else if pos_of(id).is_some_and(|q| q.0 == side) {
                    1
                } else {
                    0
                };
                Dot { x: px, y: py, k: kk, l: label(id, idx) }
            })
            .collect()
    };
    let path = |who: usize, a: usize, b: usize, k: u8| -> Line {
        let mut pts = Vec::new();
        let step = ((b.saturating_sub(a)) / 6).max(1);
        let mut j = a;
        while j < b {
            let (x, y) = (fr[j].players[who].1, fr[j].players[who].2);
            pts.push(place(x, y));
            j += step;
        }
        let (x, y) = (fr[b].players[who].1, fr[b].players[who].2);
        pts.push(place(x, y));
        Line { k, pts }
    };
    let release = fr.iter().position(|f| f.ball_mode == BallMode::Air);
    let arrive = release.and_then(|s| fr[s..].iter().position(|f| f.ball_mode != BallMode::Air).map(|k| s + k));
    let ball_line = |a: usize, b: usize| -> Option<Line> {
        let s = release?;
        let e = arrive.unwrap_or(fr.len() - 1).min(b);
        if e <= s.max(a) {
            return None;
        }
        let from = place(fr[s].ball.x as f32, fr[s].ball.y as f32);
        let to = place(fr[e].ball.x as f32, fr[e].ball.y as f32);
        Some(Line { k: 2, pts: vec![from, to] })
    };
    let end = fr.len() - 1;
    let ring_at = |k: usize| Some(place(fr[k].players[mi].1, fr[k].players[mi].2));
    let mut s0 = Still { caption: caption_snap(r, role, off), dots: dots(0), los, ..Default::default() };
    s0.lines = Vec::new();
    let mut s1 =
        Still { caption: caption_moment(r, role, grade), dots: dots(m), los, ring: ring_at(m), ..Default::default() };
    s1.lines.push(path(mi, 0, m, 0));
    if let Some(ki) = key {
        s1.lines.push(path(ki, 0, m, 1));
    }
    if let Some(l) = ball_line(0, m) {
        s1.lines.push(l);
    }
    let mut s2 = Still { caption: caption_end(r, role, me), dots: dots(end), los, ..Default::default() };
    s2.lines.push(path(mi, m, end, 0));
    if let Some(ki) = key {
        s2.lines.push(path(ki, m, end, 1));
    }
    if let Some(l) = ball_line(m, end).or_else(|| ball_line(0, end)) {
        s2.lines.push(l);
    }
    // His top speed on the snap.
    let mut speed = 0.0f32;
    for w2 in fr.windows(2) {
        let (a, b) = (&w2[0].players[mi], &w2[1].players[mi]);
        let dt = (w2[1].t - w2[0].t).max(0.05) as f32;
        let v = ((b.1 - a.1).powi(2) + (b.2 - a.2).powi(2)).sqrt() / dt;
        if v < 13.0 {
            speed = speed.max(v);
        }
    }
    let salt = key_uniform(me as u64 ^ 0xAF7E, r.n as u64 ^ (game as u64) << 16);
    let went_well = grade > 0;
    let after = words::after(&p.player, went_well, salt).map(|s| {
        if went_well && words::helps_up(&p.player) && salt > 0.7 {
            "Helps the man he beat up off the turf.".to_string()
        } else {
            s.to_string()
        }
    });
    let surprise = surprise_of(r, me, role, p, salt);
    let special = matches!(r.kind, PlayType::Punt | PlayType::Kickoff | PlayType::FieldGoal | PlayType::ExtraPoint);
    let pressure = if off { r.blitz } else { r.coverage_man || r.pressured };
    let others: Vec<u32> = fr[0].players.iter().map(|x| x.0).filter(|&x| x != me).collect();
    let cues = cues_of(&p.player, role, r, mark);
    Snap {
        game,
        sit: situation(r),
        job: job_words(r, role, pos),
        result: result_words(r, role, me),
        chart: String::new(),
        stills: vec![s0, s1, s2],
        role,
        grade,
        cues,
        call: call_of(r, role, me, fr, side, pos_of, &place),
        after,
        surprise,
        speed,
        third: r.down == 3,
        red: r.yardline_100 <= 20.0,
        pressure,
        special,
        stat: stat_of(r, me, role),
        others,
    }
    .charted()
}

impl Snap {
    fn charted(mut self) -> Snap {
        self.chart = format!("{}. {}. {}.", self.sit.replace(" · ", ", "), self.job, self.result);
        self
    }
}

fn role_of(r: &PlayRecord, me: u32, off: bool, pos: Position) -> Role {
    let in_blocks_b = r.blocks.iter().any(|b| b.0 == me);
    let in_blocks_d = r.blocks.iter().any(|b| b.1 == me);
    match r.kind {
        PlayType::Pass | PlayType::Sack | PlayType::Scramble | PlayType::TwoPoint if off => {
            if r.passer == Some(me) || (r.kind == PlayType::Sack && pos == Position::QB) {
                Role::Passer
            } else if r.target == Some(me) {
                Role::Target
            } else if r.carrier == Some(me) {
                Role::Carrier
            } else if in_blocks_b || matches!(pos, Position::OT | Position::IOL) {
                Role::PassPro
            } else {
                Role::Route
            }
        }
        PlayType::Pass | PlayType::Sack | PlayType::Scramble | PlayType::TwoPoint => {
            if r.defender == Some(me) || r.pbu == Some(me) || r.interceptor == Some(me) {
                Role::Cover
            } else if r.sackers.contains(&me)
                || r.pressures.contains(&me)
                || r.hits.contains(&me)
                || r.rush_wins.iter().any(|x| x.0 == me)
                || in_blocks_d
                || matches!(pos, Position::EDGE | Position::DT)
            {
                Role::Rusher
            } else {
                Role::Zone
            }
        }
        PlayType::Run if off => {
            if r.carrier == Some(me) {
                Role::Carrier
            } else if in_blocks_b || matches!(pos, Position::OT | Position::IOL | Position::TE) {
                Role::RunBlock
            } else {
                Role::Away
            }
        }
        PlayType::Run => {
            if r.tacklers.contains(&me)
                || r.missed_tackles.iter().any(|x| x.0 == me)
                || in_blocks_d
                || matches!(pos, Position::EDGE | Position::DT | Position::LB)
            {
                Role::RunStop
            } else {
                Role::Away
            }
        }
        _ => {
            if r.kicker == Some(me) {
                Role::Kick
            } else if r.returner == Some(me) {
                Role::Carrier
            } else if r.tacklers.contains(&me) || r.missed_tackles.iter().any(|x| x.0 == me) {
                Role::Coverage
            } else {
                Role::Away
            }
        }
    }
}

fn grade_of(r: &PlayRecord, me: u32, role: Role) -> i8 {
    let missed = r.missed_tackles.iter().any(|x| x.0 == me);
    match role {
        Role::Passer => {
            if r.interception || r.kind == PlayType::Sack {
                -1
            } else if r.complete && (r.first_down || r.touchdown) {
                1
            } else if r.complete {
                0
            } else {
                -1
            }
        }
        Role::Target => {
            if r.drop {
                -1
            } else if r.complete {
                1
            } else {
                0
            }
        }
        Role::Carrier => {
            if r.fumble_lost {
                -1
            } else if r.yards >= 5.0 || r.touchdown || r.missed_tackles.iter().any(|x| x.1 == me) {
                1
            } else if r.yards <= 1.0 {
                -1
            } else {
                0
            }
        }
        Role::PassPro | Role::RunBlock => {
            let mine: Vec<_> = r.blocks.iter().filter(|b| b.0 == me).collect();
            if mine.iter().any(|b| !b.3 || b.5) {
                -1
            } else if !mine.is_empty() {
                1
            } else {
                0
            }
        }
        Role::Rusher => {
            if r.sackers.contains(&me) || r.pressures.contains(&me) || r.rush_wins.iter().any(|x| x.0 == me) {
                1
            } else {
                0
            }
        }
        Role::RunStop | Role::Coverage => {
            if missed {
                -1
            } else if r.tacklers.contains(&me) && r.yards <= 3.0 {
                1
            } else {
                0
            }
        }
        Role::Cover => {
            if r.pbu == Some(me) || r.interceptor == Some(me) {
                1
            } else if r.complete {
                -1
            } else {
                0
            }
        }
        Role::Zone | Role::Route | Role::Away => {
            if missed {
                -1
            } else {
                0
            }
        }
        Role::Kick => {
            if r.kick_good || r.inside20 {
                1
            } else if matches!(r.kind, PlayType::FieldGoal | PlayType::ExtraPoint) {
                -1
            } else {
                0
            }
        }
    }
}

/// The frame that decides the snap.
fn moment(fr: &[Frame], role: Role) -> usize {
    let n = fr.len();
    let release = fr.iter().position(|f| f.ball_mode == BallMode::Air);
    let arrive = release.and_then(|s| fr[s..].iter().position(|f| f.ball_mode != BallMode::Air).map(|k| s + k));
    let contact = fr.iter().position(|f| f.poses.iter().any(|p| matches!(p.act, Act::Tackle | Act::Wrapped)));
    let at_t = |t: f64| fr.iter().position(|f| f.t >= t).unwrap_or(n / 2);
    let k = match role {
        Role::Passer => release.or(contact).unwrap_or(n / 2),
        Role::Target | Role::Cover | Role::Zone | Role::Route => arrive.or(release).or(contact).unwrap_or(n / 2),
        Role::PassPro | Role::Rusher => release.unwrap_or(n).min(at_t(2.4)).min(contact.unwrap_or(n)),
        Role::Kick => arrive.unwrap_or(n * 2 / 5),
        _ => contact.unwrap_or(n * 11 / 20),
    };
    k.clamp(2.min(n - 1), n.saturating_sub(2).max(1))
}

/// Which way the offense goes (+1 toward x = 100).
fn direction(fr: &[Frame], offense: usize, _side: usize, pos_of: &dyn Fn(u32) -> Option<(usize, Position)>) -> f64 {
    let f = &fr[0];
    let (mut s, mut n) = (0.0, 0.0);
    for &(id, x, _) in &f.players {
        if pos_of(id).is_some_and(|q| q.0 == offense) {
            s += x as f64;
            n += 1.0;
        }
    }
    if n == 0.0 {
        return 1.0;
    }
    if s / n <= f.ball.x { 1.0 } else { -1.0 }
}

#[allow(clippy::too_many_arguments)]
fn facing(
    r: &PlayRecord,
    fr: &[Frame],
    mi: usize,
    m: usize,
    me: u32,
    role: Role,
    side: usize,
    pos_of: &dyn Fn(u32) -> Option<(usize, Position)>,
) -> Option<usize> {
    let f = &fr[m];
    let idx = |id: u32| f.players.iter().position(|x| x.0 == id);
    let with = f.poses.get(mi).map(|p| p.with).filter(|&w| (w as usize) < f.players.len());
    let by_role = match role {
        Role::Cover => r.target.and_then(idx),
        Role::Target => r.defender.and_then(idx),
        Role::Carrier => r.tacklers.first().copied().and_then(idx),
        Role::PassPro | Role::RunBlock => r.blocks.iter().find(|b| b.0 == me).and_then(|b| idx(b.1)),
        Role::Rusher | Role::RunStop => r.blocks.iter().find(|b| b.1 == me).and_then(|b| idx(b.0)),
        _ => None,
    };
    by_role.or(with.map(|w| w as usize)).or_else(|| {
        let (mx, my) = (f.players[mi].1, f.players[mi].2);
        f.players
            .iter()
            .enumerate()
            .filter(|(_, x)| pos_of(x.0).is_some_and(|q| q.0 != side))
            .min_by(|a, b| {
                let da = (a.1.1 - mx).powi(2) + (a.1.2 - my).powi(2);
                let db = (b.1.1 - mx).powi(2) + (b.1.2 - my).powi(2);
                da.total_cmp(&db)
            })
            .map(|x| x.0)
    })
}

fn ordinal(d: u8) -> &'static str {
    match d {
        1 => "1st",
        2 => "2nd",
        3 => "3rd",
        _ => "4th",
    }
}

fn situation(r: &PlayRecord) -> String {
    let q = if r.quarter > 4 { "OT".to_string() } else { format!("Q{}", r.quarter) };
    match r.kind {
        PlayType::Punt => format!("{q} · {} & {:.0}, punt", ordinal(r.down), r.togo.max(1.0)),
        PlayType::Kickoff => format!("{q} · kickoff"),
        PlayType::FieldGoal => format!("{q} · field goal from {:.0}", r.yardline_100 + 17.0),
        PlayType::ExtraPoint | PlayType::TwoPoint => format!("{q} · try"),
        _ => {
            let togo = if r.yardline_100 <= r.togo { "goal".to_string() } else { format!("{:.0}", r.togo.max(1.0)) };
            let clock = if (r.quarter == 2 || r.quarter == 4) && r.clock <= 120.0 {
                format!(", {}:{:02} left", (r.clock as i32) / 60, (r.clock as i32) % 60)
            } else {
                String::new()
            };
            format!("{q} · {} & {togo}{clock}", ordinal(r.down))
        }
    }
}

fn job_words(r: &PlayRecord, role: Role, pos: Position) -> String {
    match role {
        Role::Passer => {
            if r.play_action {
                "Play-action, read the coverage".into()
            } else {
                "Drop back, read the coverage".into()
            }
        }
        Role::Target => {
            let route = r.route;
            if route.is_empty() { "Thrown to".into() } else { format!("Route: {route}, thrown to") }
        }
        Role::Route => {
            let route = r.route;
            if route.is_empty() { "Run his route".into() } else { "Run his route, ball elsewhere".into() }
        }
        Role::Carrier => match r.kind {
            PlayType::Punt | PlayType::Kickoff => "Return the kick".into(),
            PlayType::Pass | PlayType::Scramble => "Run after the catch".into(),
            _ => "Carry".into(),
        },
        Role::PassPro => {
            if pos == Position::RB {
                "Stay in and protect".into()
            } else {
                "Pass protection".into()
            }
        }
        Role::RunBlock => "Run block".into(),
        Role::Rusher => {
            if pos == Position::LB || pos == Position::S || pos == Position::CB {
                "Blitz".into()
            } else {
                "Rush the passer".into()
            }
        }
        Role::RunStop => "Play the run".into(),
        Role::Cover => {
            if r.coverage_man {
                if pos == Position::CB { "Press, man on the receiver".into() } else { "Man coverage".into() }
            } else {
                "Zone, the ball comes his way".into()
            }
        }
        Role::Zone => {
            if r.coverage_man {
                "Man coverage, ball elsewhere".into()
            } else {
                "Zone, ball elsewhere".into()
            }
        }
        Role::Kick => match r.kind {
            PlayType::Punt => "Punt".into(),
            PlayType::Kickoff => "Kickoff".into(),
            _ => "The kick".into(),
        },
        Role::Coverage => "Kick coverage".into(),
        Role::Away => {
            if matches!(r.kind, PlayType::Run) {
                "Backside, ball away".into()
            } else {
                "His job, ball elsewhere".into()
            }
        }
    }
}

fn result_words(r: &PlayRecord, role: Role, me: u32) -> String {
    let y = r.yards.round() as i32;
    let yards = |n: i32| if n == 1 { "1 yard".to_string() } else { format!("{n} yards") };
    if r.touchdown && !r.defense_td {
        return match role {
            Role::Cover | Role::Zone | Role::RunStop | Role::Rusher => "Touchdown against".into(),
            _ => format!("Touchdown, {}", yards(y)),
        };
    }
    if r.interception {
        return if r.interceptor == Some(me) { "Intercepted it".into() } else { "Intercepted".into() };
    }
    match r.kind {
        PlayType::Sack => {
            if r.sackers.contains(&me) {
                "His sack".into()
            } else {
                format!("Sack, loss of {}", (-y).max(0))
            }
        }
        PlayType::Pass => {
            if r.complete {
                format!("Caught, {}", yards(y))
            } else if r.pbu == Some(me) {
                "Broken up".into()
            } else if r.drop {
                "Dropped".into()
            } else if r.throw_away {
                "Thrown away".into()
            } else {
                "Incomplete".into()
            }
        }
        PlayType::Run | PlayType::Scramble => {
            if r.fumble_lost {
                "Fumble, lost".into()
            } else if r.tacklers.contains(&me) {
                format!("His tackle, gain of {y}")
            } else if y < 0 {
                format!("Loss of {}", -y)
            } else {
                format!("Gain of {y}")
            }
        }
        PlayType::Punt => {
            if r.tacklers.contains(&me) {
                format!("His tackle, {} return", r.return_yards.round() as i32)
            } else if r.fair_catch {
                format!("{:.0}-yard punt, fair catch", r.kick_distance)
            } else {
                format!("{:.0}-yard punt", r.kick_distance)
            }
        }
        PlayType::Kickoff => {
            if r.touchback {
                "Touchback".into()
            } else if r.tacklers.contains(&me) {
                format!("His tackle at the {}", (100.0 - r.yardline_100).round() as i32)
            } else {
                format!("{}-yard return", r.return_yards.round() as i32)
            }
        }
        PlayType::FieldGoal | PlayType::ExtraPoint => {
            if r.kick_good {
                "Good".into()
            } else if r.blocked {
                "Blocked".into()
            } else {
                "No good".into()
            }
        }
        _ => format!("Gain of {y}"),
    }
}

fn caption_snap(r: &PlayRecord, role: Role, off: bool) -> String {
    let look = if off { r.personnel.clone() } else { format!("{} in the box", r.box_count) };
    match role {
        Role::Cover | Role::Zone => {
            format!("{}{}.", if r.coverage_man { "Man" } else { "Zone" }, if r.blitz { ", a blitz on" } else { "" })
        }
        Role::Rusher | Role::RunStop => format!("{look}{}.", if r.blitz { ", pressure called" } else { "" }),
        Role::Kick | Role::Coverage => "Special teams.".into(),
        _ => format!("{} personnel{}.", look, if r.play_action { ", play-action" } else { "" }),
    }
}

fn caption_moment(r: &PlayRecord, role: Role, grade: i8) -> String {
    let good = grade > 0;
    let bad = grade < 0;
    match role {
        Role::Passer => {
            if r.kind == PlayType::Sack {
                "The pocket closes before he can let it go.".into()
            } else if r.pressured {
                "Pressure in his face as he lets it go.".into()
            } else {
                "The ball comes out.".into()
            }
        }
        Role::Target | Role::Cover => {
            if good {
                "At the catch point, he wins it.".into()
            } else if bad {
                "At the catch point, he loses it.".into()
            } else {
                "The ball arrives.".into()
            }
        }
        Role::PassPro | Role::RunBlock => {
            if bad {
                "His man gets the edge on him.".into()
            } else {
                "Engaged, and holding.".into()
            }
        }
        Role::Rusher => {
            if good {
                "He's through the line.".into()
            } else {
                "Still engaged as the ball comes out.".into()
            }
        }
        Role::Carrier => {
            if good {
                "First contact, and he keeps going.".into()
            } else {
                "First contact.".into()
            }
        }
        Role::RunStop | Role::Coverage => {
            if good {
                "He fits the gap and meets the runner.".into()
            } else if bad {
                "He gets there and misses.".into()
            } else {
                "The ball carrier reaches his area.".into()
            }
        }
        Role::Kick => "The kick is away.".into(),
        _ => "The play goes the other way.".into(),
    }
}

fn caption_end(r: &PlayRecord, role: Role, me: u32) -> String {
    format!("{}.", result_words(r, role, me))
}

/// What the tape shows of him on a snap, by his job (SK43): the skills it
/// tests, and the effort when the ball goes away.
fn cues_of(p: &crate::player::Player, role: Role, r: &PlayRecord, mark: f64) -> Vec<(Attr, u8)> {
    use Attr::*;
    let pos = p.position;
    let list: Vec<Attr> = match role {
        Role::Passer => {
            let acc = if r.air_yards >= 20.0 {
                DeepAccuracy
            } else if r.air_yards >= 10.0 {
                MediumAccuracy
            } else {
                ShortAccuracy
            };
            if r.pressured || r.kind == PlayType::Sack {
                vec![PocketPresence, Decisions, acc]
            } else {
                vec![Decisions, acc, Anticipation]
            }
        }
        Role::Target => {
            if r.separation < 1.0 {
                vec![ContestedCatch, RouteRunning, Catching]
            } else if r.air_yards >= 20.0 {
                vec![BallTracking, Speed, RouteRunning]
            } else {
                vec![RouteRunning, Catching, Release]
            }
        }
        Role::Route => vec![RouteRunning, Release, Motor],
        Role::Carrier => vec![BallCarrierVision, Elusiveness, LegDrive],
        Role::PassPro => {
            if pos == Position::RB {
                vec![PassBlocking, PlayRecognition]
            } else {
                vec![PassBlocking, Footwork, HandUse, Anchor]
            }
        }
        Role::RunBlock => {
            if matches!(pos, Position::WR | Position::TE) && pos == Position::WR {
                vec![BlockingInSpace, Motor]
            } else {
                vec![RunBlocking, LegDrive, HandUse]
            }
        }
        Role::Rusher => {
            let mv = r.rush_wins.iter().find(|x| x.0 == p.id).map(|x| x.1);
            let rush = match mv.map(|m| format!("{m:?}")) {
                Some(s) if s.contains("Bull") || s.contains("Power") || s.contains("Long") => PowerRush,
                _ => FinesseRush,
            };
            if matches!(pos, Position::LB | Position::S | Position::CB) {
                vec![Blitzing, Acceleration]
            } else {
                vec![rush, HandUse, Acceleration]
            }
        }
        Role::RunStop => {
            if pos == Position::LB {
                vec![PlayRecognition, Traffic, Tackling]
            } else if matches!(pos, Position::CB | Position::S) {
                vec![Tackling, Pursuit]
            } else {
                vec![BlockShedding, Strength, Tackling]
            }
        }
        Role::Cover => {
            if r.coverage_man {
                if pos == Position::CB { vec![Press, ManCoverage, Speed] } else { vec![ManCoverage, Agility] }
            } else {
                vec![ZoneCoverage, PlayRecognition]
            }
        }
        Role::Zone => {
            if r.coverage_man {
                vec![ManCoverage, Motor]
            } else {
                vec![ZoneCoverage, PlayRecognition, Motor]
            }
        }
        Role::Kick => match r.kind {
            PlayType::Punt => vec![PuntPower, PuntPlacement],
            PlayType::FieldGoal if r.yardline_100 + 17.0 >= 45.0 => vec![KickPower, LongKickAccuracy],
            PlayType::FieldGoal | PlayType::ExtraPoint => vec![ShortKickAccuracy, Composure],
            _ => vec![KickPower],
        },
        Role::Coverage => vec![Tackling, Pursuit, Speed],
        Role::Away => vec![Motor, Pursuit],
    };
    list.into_iter().take(3).map(|a| (a, words::band(p.attributes.get(a), mark) as u8)).collect()
}

/// The call before the moment (SK48), with what really happened.
#[allow(clippy::too_many_arguments)]
fn call_of(
    r: &PlayRecord,
    role: Role,
    me: u32,
    fr: &[Frame],
    side: usize,
    pos_of: &dyn Fn(u32) -> Option<(usize, Position)>,
    place: &dyn Fn(f32, f32) -> (u8, u8),
) -> Option<Call> {
    let yn = |q: &str, yes: &str, no: &str, truth: bool, said: &str| Call {
        q: q.into(),
        opts: vec![yes.into(), no.into()],
        right: if truth { 0 } else { 1 },
        truth: said.into(),
    };
    match role {
        Role::Passer if r.kind == PlayType::Pass && !r.throw_away => {
            // Where should the ball go: the open man at the release, from
            // the tracking (the farthest from any defender).
            let s = fr.iter().position(|f| f.ball_mode == BallMode::Air)?;
            let f = &fr[s.saturating_sub(1)];
            let mut best: Option<(f32, u8)> = None;
            for &(id, x, y) in &f.players {
                let Some((sd, ps)) = pos_of(id) else { continue };
                if sd != side || !matches!(ps, Position::WR | Position::TE | Position::RB) {
                    continue;
                }
                let sep = f
                    .players
                    .iter()
                    .filter(|o| pos_of(o.0).is_some_and(|q| q.0 != side))
                    .map(|o| ((o.1 - x).powi(2) + (o.2 - y).powi(2)).sqrt())
                    .fold(99.0f32, f32::min);
                let third = third_of(place(x, y).0);
                if best.is_none_or(|b| sep > b.0) {
                    best = Some((sep, third));
                }
            }
            let (_, open) = best?;
            let thrown =
                r.target.and_then(|t| f.players.iter().find(|x| x.0 == t).map(|x| third_of(place(x.1, x.2).0)))?;
            let names = ["Left", "Middle", "Right"];
            let said = if thrown == open {
                format!("he went {}, to the open man.", names[open as usize].to_lowercase())
            } else {
                format!(
                    "the open man was {}; he went {}.",
                    names[open as usize].to_lowercase(),
                    names[thrown as usize].to_lowercase()
                )
            };
            Some(Call {
                q: "Top of his drop. Where should the ball go?".into(),
                opts: names.iter().map(|s| s.to_string()).collect(),
                right: open,
                truth: said,
            })
        }
        Role::Target => Some(yn(
            "The ball's coming to him. Does he win at the catch point?",
            "He wins it",
            "He loses it",
            r.complete,
            if r.complete { "he won it." } else { "he lost it." },
        )),
        Role::Cover => {
            let held = r.pbu == Some(me) || r.interceptor == Some(me) || !r.complete;
            Some(yn(
                "The break is coming. Does he stay with his man?",
                "He stays with him",
                "He gets beat",
                held,
                if held { "he stayed with him." } else { "he got beat." },
            ))
        }
        Role::PassPro | Role::RunBlock => {
            let b = r.blocks.iter().find(|b| b.0 == me)?;
            let held = b.3 && !b.5;
            Some(yn(
                "Contact. Does his block hold?",
                "It holds",
                "He gets beat",
                held,
                if held { "the block held." } else { "his man beat him." },
            ))
        }
        Role::Rusher => {
            let won = r.sackers.contains(&me) || r.pressures.contains(&me) || r.rush_wins.iter().any(|x| x.0 == me);
            Some(yn(
                "He's engaged. Does he win his rush?",
                "He wins",
                "He's stalemated",
                won,
                if won { "he won his rush." } else { "he was stalemated." },
            ))
        }
        Role::Carrier => {
            let broke = r.missed_tackles.iter().any(|x| x.1 == me) || r.yards >= 5.0;
            Some(yn(
                "First man coming. Does he get past him?",
                "He gets past",
                "He goes down",
                broke,
                if broke { "he got past him." } else { "he went down." },
            ))
        }
        Role::RunStop | Role::Coverage => {
            let made = r.tacklers.contains(&me) && !r.missed_tackles.iter().any(|x| x.0 == me);
            if !(r.tacklers.contains(&me) || r.missed_tackles.iter().any(|x| x.0 == me)) {
                return None;
            }
            Some(yn(
                "He's there. Does he make the tackle?",
                "He makes it",
                "He misses",
                made,
                if made { "he made it." } else { "he missed." },
            ))
        }
        Role::Kick if matches!(r.kind, PlayType::FieldGoal | PlayType::ExtraPoint) => Some(yn(
            "The kick is up. Is it good?",
            "Good",
            "No good",
            r.kick_good,
            if r.kick_good { "it was good." } else { "it missed." },
        )),
        _ => None,
    }
}

fn third_of(x: u8) -> u8 {
    if x < 38 {
        0
    } else if x > 62 {
        2
    } else {
        1
    }
}

fn surprise_of(r: &PlayRecord, me: u32, role: Role, p: &crate::world::Person, salt: f64) -> Option<String> {
    use crate::game::sim::FoulKind;
    if r.injuries.iter().any(|j| j.id == me) {
        return Some("Limps to the sideline after it.".into());
    }
    if r.penalties.iter().any(|x| {
        x.2 == me && matches!(x.0, FoulKind::UnnecessaryRoughness | FoulKind::FaceMask | FoulKind::RoughingPasser)
    }) {
        return Some("A late shot after the whistle. The flag comes out.".into());
    }
    if role == Role::Coverage && r.tacklers.contains(&me) {
        return Some("A big hit on kick coverage.".into());
    }
    let temper = p.player.personality.traits.temper;
    if temper >= 17 && salt > 0.985 {
        return Some("A shove after the whistle that the officials miss.".into());
    }
    None
}

fn stat_of(r: &PlayRecord, me: u32, role: Role) -> Stat {
    let mut s = Stat::default();
    let y = r.yards.round().clamp(-99.0, 99.0) as i16;
    match role {
        Role::Passer => {
            if r.kind == PlayType::Sack {
                s.sacks = 1;
            } else if r.kind == PlayType::Pass {
                s.att = 1;
                if r.complete {
                    s.comp = 1;
                    s.yards = y;
                }
                if r.interception {
                    s.ints = 1;
                }
            }
            if r.touchdown {
                s.td = 1;
            }
        }
        Role::Target => {
            s.targets = 1;
            if r.complete {
                s.catches = 1;
                s.yards = y;
            }
            if r.touchdown {
                s.td = 1;
            }
        }
        Role::Cover => {
            s.targets = 1;
            if r.complete {
                s.catches = 1;
                s.yards = y;
            }
            if r.pbu == Some(me) {
                s.pbu = 1;
            }
            if r.interceptor == Some(me) {
                s.ints = 1;
            }
        }
        Role::Carrier => {
            s.carries = 1;
            s.yards = y;
            if r.touchdown {
                s.td = 1;
            }
        }
        Role::PassPro | Role::RunBlock => {
            s.reps = 1;
            if r.blocks.iter().any(|b| b.0 == me && (!b.3 || b.5)) {
                s.pressures = 1;
            } else {
                s.wins = 1;
            }
        }
        Role::Rusher => {
            s.reps = 1;
            if r.sackers.contains(&me) {
                s.sacks = 1;
            }
            if r.pressures.contains(&me) || r.rush_wins.iter().any(|x| x.0 == me) {
                s.pressures = 1;
            }
        }
        _ => {}
    }
    if r.tacklers.contains(&me) {
        s.tackles = 1;
    }
    if r.missed_tackles.iter().any(|x| x.0 == me) {
        s.missed = 1;
    }
    s
}

/// The snaps of a cut-up (SK45), in game order, at most `n`; the snaps
/// that jump out ride along (SK51).
pub fn cut(tape: &Tape, c: Cut, n: usize) -> Vec<usize> {
    let snaps = &tape.snaps;
    let game_score = |g: u8| -> f64 {
        let v: Vec<f64> = snaps.iter().filter(|s| s.game == g && s.grade != 0).map(|s| s.grade as f64).collect();
        if v.is_empty() { 0.0 } else { v.iter().sum::<f64>() / v.len() as f64 }
    };
    let pick: Vec<usize> = match c {
        Cut::ThirdDowns => (0..snaps.len()).filter(|&k| snaps[k].third && !snaps[k].special).collect(),
        Cut::RedZone => (0..snaps.len()).filter(|&k| snaps[k].red && !snaps[k].special).collect(),
        Cut::Pressure => (0..snaps.len()).filter(|&k| snaps[k].pressure && !snaps[k].special).collect(),
        Cut::BestOpponent => {
            let g = (0..tape.games.len())
                .max_by(|a, b| tape.games[*a].strength.total_cmp(&tape.games[*b].strength))
                .unwrap_or(0) as u8;
            (0..snaps.len()).filter(|&k| snaps[k].game == g).collect()
        }
        Cut::WorstGame => {
            let g = (0..tape.games.len() as u8).min_by(|a, b| game_score(*a).total_cmp(&game_score(*b))).unwrap_or(0);
            (0..snaps.len()).filter(|&k| snaps[k].game == g).collect()
        }
        Cut::Ours => (0..snaps.len()).collect(),
    };
    // The snaps where he's in the play first, spread across the games.
    let mut busy: Vec<usize> = pick.iter().copied().filter(|&k| snaps[k].role != Role::Away).collect();
    let mut idle: Vec<usize> = pick.iter().copied().filter(|&k| snaps[k].role == Role::Away).collect();
    busy.truncate(n.saturating_sub(1).max(1));
    idle.truncate(n.saturating_sub(busy.len()).min(2));
    let mut out = busy;
    out.extend(idle);
    // A snap that jumps out, from anywhere on the tape.
    if let Some(k) = (0..snaps.len()).find(|&k| snaps[k].surprise.is_some() && !out.contains(&k)) {
        out.push(k);
    }
    out.sort_unstable();
    out
}
