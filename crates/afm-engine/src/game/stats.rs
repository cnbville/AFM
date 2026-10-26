//! The stats database (topic 8). One source: every play is logged with who
//! did what and why, and every stat is computed from that log. Game lines
//! roll up into season and career totals.

use super::sim::{FoulKind, Injury, Move};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum PlayType {
    Run,
    Pass,
    Sack,
    Scramble,
    Kneel,
    Spike,
    Punt,
    FieldGoal,
    ExtraPoint,
    TwoPoint,
    Kickoff,
    /// A pre-snap foul: no play.
    NoPlay,
}

/// One play in the log (8.1, 8.2).
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct PlayRecord {
    pub n: u32,
    pub quarter: u8,
    /// Seconds left in the quarter at the snap.
    pub clock: f64,
    /// Team index (0 home, 1 away) with the ball.
    pub offense: usize,
    pub down: u8,
    pub togo: f64,
    /// Yards from the opponent's goal line.
    pub yardline_100: f64,
    pub score: [i32; 2],
    pub kind: PlayType,
    pub call: String,
    /// The play caller's reason (plays and tactics topic 6.6).
    #[serde(deserialize_with = "crate::save::leaked")]
    pub call_why: crate::save::Text,
    pub defense_call: String,
    pub personnel: String,
    pub box_count: u8,
    pub blitz: bool,
    pub play_action: bool,
    pub coverage_man: bool,
    /// The route the target ran.
    #[serde(deserialize_with = "crate::save::leaked")]
    pub route: crate::save::Text,
    pub yards: f64,
    pub first_down: bool,
    pub touchdown: bool,
    /// The defense scored on the return (a pick six or a fumble return).
    pub defense_td: bool,
    pub turnover: bool,
    pub safety: bool,
    pub passer: Option<u32>,
    pub target: Option<u32>,
    pub complete: bool,
    pub interception: bool,
    pub carrier: Option<u32>,
    pub tacklers: Vec<u32>,
    pub missed_tackles: Vec<(u32, u32)>,
    pub sackers: Vec<u32>,
    pub hits: Vec<u32>,
    pub pressures: Vec<u32>,
    pub pbu: Option<u32>,
    pub defender: Option<u32>,
    pub interceptor: Option<u32>,
    pub forced_fumble: Option<u32>,
    pub fumbled: Option<u32>,
    pub fumble_lost: bool,
    pub air_yards: f64,
    pub yac: f64,
    pub before_contact: f64,
    pub time_to_throw: f64,
    pub separation: f64,
    pub throw_away: bool,
    pub drop: bool,
    pub pressured: bool,
    pub scramble: bool,
    pub out_of_bounds: bool,
    /// Wiped out by an accepted penalty: only its snaps and the penalty
    /// count, as in real box scores (nflverse calls it a no-play).
    pub nullified: bool,
    pub duration: f64,
    /// Special teams.
    pub kicker: Option<u32>,
    pub returner: Option<u32>,
    pub kick_distance: f64,
    pub return_yards: f64,
    pub kick_good: bool,
    pub touchback: bool,
    /// A single (the CFL's rouge) scored on this kick.
    #[serde(default)]
    pub single: bool,
    pub fair_catch: bool,
    pub inside20: bool,
    pub blocked: bool,
    pub kick_pressure: f64,
    /// (kind, team, player, yards, accepted)
    pub penalties: Vec<(FoulKind, usize, u32, f64, bool)>,
    /// Blocks: (blocker, defender, pass, won, seconds, gave up pressure).
    pub blocks: Vec<(u32, u32, bool, bool, f64, bool)>,
    pub rush_wins: Vec<(u32, Move, f64)>,
    pub busts: Vec<u32>,
    pub freelance: Vec<(u32, bool)>,
    pub injuries: Vec<Injury>,
    /// Players on the field: offense then defense (or kicking then return).
    pub on_field: [Vec<u32>; 2],
    /// Fastest player on the play: (id, mph).
    pub top_speed: (u32, f64),
    pub ep_before: f64,
    pub epa: f64,
    pub wp_before: f64,
    pub why: String,
    /// Context for splits (8.3): the drive started after a turnover by the
    /// other team, and the score margin for the offense.
    pub after_turnover: bool,
    /// Garbage time: each side's step at the snap (0 = everyone in; see
    /// `garbage`), home then away.
    pub garbage: [u8; 2],
    /// A man from another position group was on the field because his own
    /// group was out of bodies (fix list 3.6).
    pub emergency: bool,
    /// What the players' tendencies did (`docs/design/tendencies.md`).
    pub tend: super::sim::TendNotes,
    /// Game day (section 9): you called the offense (1) or the defense (2).
    #[serde(default)]
    pub yours: u8,
    /// Your call was off the week's sheet (less practiced).
    #[serde(default)]
    pub off_sheet: bool,
    /// The staff's call you replaced.
    #[serde(default)]
    pub staff_call: String,
    /// A close call by the officials, and any review of it.
    #[serde(default)]
    pub officiating: Option<super::live::Officiating>,
    /// Across the field: the snap and where the throw arrived (0 to 53.3,
    /// the offense's left high), for the pass chart.
    #[serde(default)]
    pub throw_y: Option<(f32, f32)>,
    /// A 4th down weighed by the model at the snap: the true win chance of
    /// going, punting and kicking (0 out of range), and the chance to
    /// convert.
    #[serde(default)]
    pub fourth: Option<[f32; 4]>,
    /// Where the ball sat at the snap (update 2, G6, the self-scout): 0 not
    /// kept, 1 the left hash, 2 the right, 3 the middle, as the offense
    /// looks downfield.
    #[serde(default)]
    pub hash: u8,
    /// Snapped without a huddle (G1's tempo, for the Identity tab).
    #[serde(default)]
    pub hurry: bool,
    /// The defense's package (G15): 0 not kept, 1 base, 2 nickel, 3 dime,
    /// 4 goal line.
    #[serde(default)]
    pub dpers: u8,
    /// A punt or a kickoff: the return team showed the look a fake or a
    /// surprise onside kick waits for (G18).
    #[serde(default)]
    pub look: bool,
}

impl PlayRecord {
    pub fn new(n: u32, quarter: u8, clock: f64, offense: usize, kind: PlayType) -> PlayRecord {
        PlayRecord {
            n,
            quarter,
            clock,
            offense,
            down: 0,
            togo: 0.0,
            yardline_100: 0.0,
            score: [0, 0],
            kind,
            call: String::new(),
            call_why: "",
            defense_call: String::new(),
            personnel: String::new(),
            box_count: 0,
            blitz: false,
            play_action: false,
            coverage_man: false,
            route: "",
            yards: 0.0,
            first_down: false,
            touchdown: false,
            defense_td: false,
            turnover: false,
            safety: false,
            passer: None,
            target: None,
            complete: false,
            interception: false,
            carrier: None,
            tacklers: Vec::new(),
            missed_tackles: Vec::new(),
            sackers: Vec::new(),
            hits: Vec::new(),
            pressures: Vec::new(),
            pbu: None,
            defender: None,
            interceptor: None,
            forced_fumble: None,
            fumbled: None,
            fumble_lost: false,
            air_yards: 0.0,
            yac: 0.0,
            before_contact: 0.0,
            time_to_throw: 0.0,
            separation: 0.0,
            throw_away: false,
            drop: false,
            pressured: false,
            scramble: false,
            out_of_bounds: false,
            nullified: false,
            duration: 0.0,
            kicker: None,
            returner: None,
            kick_distance: 0.0,
            return_yards: 0.0,
            kick_good: false,
            touchback: false,
            single: false,
            fair_catch: false,
            inside20: false,
            blocked: false,
            kick_pressure: 0.0,
            penalties: Vec::new(),
            blocks: Vec::new(),
            rush_wins: Vec::new(),
            busts: Vec::new(),
            freelance: Vec::new(),
            injuries: Vec::new(),
            on_field: [Vec::new(), Vec::new()],
            top_speed: (0, 0.0),
            ep_before: 0.0,
            epa: 0.0,
            wp_before: 0.5,
            why: String::new(),
            after_turnover: false,
            garbage: [0, 0],
            emergency: false,
            tend: Default::default(),
            yours: 0,
            off_sheet: false,
            staff_call: String::new(),
            officiating: None,
            throw_y: None,
            fourth: None,
            hash: 0,
            hurry: false,
            dpers: 0,
            look: false,
        }
    }

    pub fn success(&self) -> bool {
        self.epa > 0.0
    }

    /// The team (0 home, 1 away) a player on the field plays for, or None
    /// when he was not on the field.
    pub fn side_of(&self, id: u32) -> Option<usize> {
        if self.on_field[0].contains(&id) {
            Some(self.offense)
        } else if self.on_field[1].contains(&id) {
            Some(1 - self.offense)
        } else {
            None
        }
    }
}

/// One player's line for a game, a season or a career (8.3). Everything is
/// a count or a sum, so lines add up; rates are worked out when read.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct StatLine {
    pub games: u32,
    pub snaps_off: u32,
    pub snaps_def: u32,
    pub snaps_st: u32,
    // passing
    pub att: u32,
    pub cmp: u32,
    pub pass_yds: f64,
    pub pass_td: u32,
    pub int: u32,
    pub sacked: u32,
    pub sack_yds: f64,
    pub dropbacks: u32,
    pub pressured: u32,
    pub air_yds_att: f64,
    pub ttt: f64,
    pub pass_epa: f64,
    pub pass_success: u32,
    pub x_cmp: f64,
    pub big_time: u32,
    pub turnover_worthy: u32,
    pub pa_att: u32,
    pub pa_yds: f64,
    pub blitz_att: u32,
    pub blitz_yds: f64,
    pub deep_att: u32,
    pub deep_cmp: u32,
    pub throw_aways: u32,
    // rushing
    pub rush: u32,
    pub rush_yds: f64,
    pub rush_td: u32,
    pub rush_long: f64,
    pub ybc: f64,
    pub yaco: f64,
    pub forced_missed: u32,
    pub explosive_runs: u32,
    pub stuffs: u32,
    pub rush_success: u32,
    pub rush_first: u32,
    pub rush_epa: f64,
    pub scrambles: u32,
    pub scramble_yds: f64,
    pub fumbles: u32,
    pub fumbles_lost: u32,
    // receiving
    pub targets: u32,
    pub rec: u32,
    pub rec_yds: f64,
    pub rec_td: u32,
    pub rec_long: f64,
    pub drops: u32,
    pub tgt_air: f64,
    pub yac: f64,
    pub sep_sum: f64,
    pub routes: u32,
    pub contested_tgt: u32,
    pub contested_rec: u32,
    pub rec_first: u32,
    pub rec_epa: f64,
    // blocking
    pub pb_snaps: u32,
    pub pb_wins: u32,
    pub pressures_allowed: u32,
    pub sacks_allowed: f64,
    pub rb_snaps: u32,
    pub rb_wins: u32,
    // pass rush
    pub rush_snaps: u32,
    pub rush_wins: u32,
    pub sacks: f64,
    pub qb_hits: u32,
    pub hurries: u32,
    pub pressures: u32,
    pub speed_wins: u32,
    pub power_wins: u32,
    pub finesse_wins: u32,
    pub ttp_sum: f64,
    // run defense and tackling
    pub tackles: u32,
    pub assists: u32,
    pub tfl: f64,
    pub stops: u32,
    pub missed: u32,
    pub forced_fumbles: u32,
    pub fumble_rec: u32,
    // coverage
    pub cov_snaps: u32,
    pub tgt_allowed: u32,
    pub cmp_allowed: u32,
    pub yds_allowed: f64,
    pub td_allowed: u32,
    pub ints: u32,
    pub pbus: u32,
    pub sep_allowed: f64,
    pub freelance: u32,
    pub freelance_won: u32,
    // special teams
    pub fg_att: [u32; 4],
    pub fg_made: [u32; 4],
    pub fg_long: f64,
    pub clutch_att: u32,
    pub clutch_made: u32,
    pub xp_att: u32,
    pub xp_made: u32,
    pub punts: u32,
    pub punt_yds: f64,
    pub punt_net: f64,
    pub punt_in20: u32,
    pub punt_tb: u32,
    pub hang_sum: f64,
    pub kickoffs: u32,
    pub ko_tb: u32,
    pub kr: u32,
    pub kr_yds: f64,
    pub kr_td: u32,
    pub pr: u32,
    pub pr_yds: f64,
    pub pr_td: u32,
    pub st_tackles: u32,
    // discipline
    pub penalties: u32,
    pub penalty_yds: f64,
    pub busts: u32,
    // grade: sum of per-play grades and plays graded (PFF style, -2 to +2)
    pub grade_sum: f64,
    pub graded: u32,
    pub top_speed: f64,
}

impl StatLine {
    pub fn add(&mut self, o: &StatLine) {
        macro_rules! add {
            ($($f:ident),*) => { $( self.$f += o.$f; )* };
        }
        add!(
            games,
            snaps_off,
            snaps_def,
            snaps_st,
            att,
            cmp,
            pass_yds,
            pass_td,
            int,
            sacked,
            sack_yds,
            dropbacks,
            pressured,
            air_yds_att,
            ttt,
            pass_epa,
            pass_success,
            x_cmp,
            big_time,
            turnover_worthy,
            pa_att,
            pa_yds,
            blitz_att,
            blitz_yds,
            deep_att,
            deep_cmp,
            throw_aways,
            rush,
            rush_yds,
            rush_td,
            ybc,
            yaco,
            forced_missed,
            explosive_runs,
            stuffs,
            rush_success,
            rush_first,
            rush_epa,
            scrambles,
            scramble_yds,
            fumbles,
            fumbles_lost,
            targets,
            rec,
            rec_yds,
            rec_td,
            drops,
            tgt_air,
            yac,
            sep_sum,
            routes,
            contested_tgt,
            contested_rec,
            rec_first,
            rec_epa,
            pb_snaps,
            pb_wins,
            pressures_allowed,
            sacks_allowed,
            rb_snaps,
            rb_wins,
            rush_snaps,
            rush_wins,
            sacks,
            qb_hits,
            hurries,
            pressures,
            speed_wins,
            power_wins,
            finesse_wins,
            ttp_sum,
            tackles,
            assists,
            tfl,
            stops,
            missed,
            forced_fumbles,
            fumble_rec,
            cov_snaps,
            tgt_allowed,
            cmp_allowed,
            yds_allowed,
            td_allowed,
            ints,
            pbus,
            sep_allowed,
            freelance,
            freelance_won,
            clutch_att,
            clutch_made,
            xp_att,
            xp_made,
            punts,
            punt_yds,
            punt_net,
            punt_in20,
            punt_tb,
            hang_sum,
            kickoffs,
            ko_tb,
            kr,
            kr_yds,
            kr_td,
            pr,
            pr_yds,
            pr_td,
            st_tackles,
            penalties,
            penalty_yds,
            busts,
            grade_sum,
            graded
        );
        for k in 0..4 {
            self.fg_att[k] += o.fg_att[k];
            self.fg_made[k] += o.fg_made[k];
        }
        self.rush_long = self.rush_long.max(o.rush_long);
        self.rec_long = self.rec_long.max(o.rec_long);
        self.fg_long = self.fg_long.max(o.fg_long);
        self.top_speed = self.top_speed.max(o.top_speed);
    }

    // ---- rates read off the counts
    pub fn passer_rating(&self) -> f64 {
        if self.att == 0 {
            return 0.0;
        }
        let a = self.att as f64;
        let c = |x: f64| x.clamp(0.0, 2.375);
        let p1 = c((self.cmp as f64 / a - 0.3) * 5.0);
        let p2 = c((self.pass_yds / a - 3.0) * 0.25);
        let p3 = c(self.pass_td as f64 / a * 20.0);
        let p4 = c(2.375 - self.int as f64 / a * 25.0);
        (p1 + p2 + p3 + p4) / 6.0 * 100.0
    }
    pub fn cmp_pct(&self) -> f64 {
        self.cmp as f64 / self.att.max(1) as f64
    }
    pub fn cpoe(&self) -> f64 {
        (self.cmp as f64 - self.x_cmp) / self.att.max(1) as f64
    }
    pub fn ypa(&self) -> f64 {
        self.pass_yds / self.att.max(1) as f64
    }
    /// Adjusted net yards per attempt.
    pub fn anya(&self) -> f64 {
        (self.pass_yds + 20.0 * self.pass_td as f64 - 45.0 * self.int as f64 - self.sack_yds)
            / (self.att + self.sacked).max(1) as f64
    }
    pub fn ypc(&self) -> f64 {
        self.rush_yds / self.rush.max(1) as f64
    }
    pub fn yprr(&self) -> f64 {
        self.rec_yds / self.routes.max(1) as f64
    }
    pub fn pass_rush_win_rate(&self) -> f64 {
        self.rush_wins as f64 / self.rush_snaps.max(1) as f64
    }
    pub fn pass_block_win_rate(&self) -> f64 {
        self.pb_wins as f64 / self.pb_snaps.max(1) as f64
    }
    pub fn missed_tackle_rate(&self) -> f64 {
        self.missed as f64 / (self.missed + self.tackles + self.assists).max(1) as f64
    }
    /// 0-100 grade from the per-play grades, PFF style: 60 is average.
    pub fn grade(&self) -> f64 {
        if self.graded == 0 {
            return 0.0;
        }
        let avg = self.grade_sum / self.graded as f64;
        let conf = (self.graded as f64 / (self.graded as f64 + 25.0)).sqrt();
        (60.0 + 95.0 * avg * conf).clamp(20.0, 99.9)
    }
    pub fn fg_pct(&self) -> f64 {
        let (a, m): (u32, u32) = (self.fg_att.iter().sum(), self.fg_made.iter().sum());
        m as f64 / a.max(1) as f64
    }
}

/// Field goal distance bands: under 30, 30-39, 40-49, 50+.
pub fn fg_band(d: f64) -> usize {
    if d < 30.0 {
        0
    } else if d < 40.0 {
        1
    } else if d < 50.0 {
        2
    } else {
        3
    }
}

/// Expected completion for a throw, from depth, separation and pressure
/// (Claude's fit to real completion by air yards; see decisions.md).
pub fn expected_completion(air: f64, sep: f64, pressured: bool) -> f64 {
    let z = 1.35 - 0.055 * air + 0.55 * (sep - 1.5) - if pressured { 0.45 } else { 0.0 };
    1.0 / (1.0 + (-z).exp())
}

/// A team's game (8.3, team and unit).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct TeamStats {
    pub points: i32,
    pub plays: u32,
    pub yards: f64,
    pub pass_yards: f64,
    pub rush_yards: f64,
    pub first_downs: u32,
    pub third_att: u32,
    pub third_conv: u32,
    pub fourth_att: u32,
    pub fourth_conv: u32,
    pub penalties: u32,
    pub penalty_yds: f64,
    pub turnovers: u32,
    pub time_of_possession: f64,
    pub sacks: u32,
    pub red_zone_trips: u32,
    pub red_zone_tds: u32,
    pub epa: f64,
    pub successes: u32,
    pub explosive: u32,
    pub dropbacks: u32,
    pub pressures: u32,
    pub drives: u32,
    pub start_sum: f64,
    pub punts: u32,
    pub fg_att: u32,
    pub fg_made: u32,
    pub att: u32,
    pub cmp: u32,
    pub carries: u32,
    pub ints: u32,
    pub fumbles_lost: u32,
}

/// The whole game: final score, team totals, every player's line, and the
/// play log with its whys (8.8: box score, drive chart, win probability,
/// play by play).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct GameStats {
    pub score: [i32; 2],
    pub quarters: Vec<[i32; 2]>,
    pub team: [TeamStats; 2],
    /// Every player's line by id, in id order so walking the lines is the
    /// same in every run (stories draw on it).
    pub players: [std::collections::BTreeMap<u32, StatLine>; 2],
    pub plays: Vec<PlayRecord>,
    pub overtime: bool,
    /// Timeouts each team (home, away) still held when the first and the
    /// second half ended: what the morning-after grades read.
    #[serde(default)]
    pub timeouts_left: [[u8; 2]; 2],
}

impl GameStats {
    pub fn line(&mut self, team: usize, id: u32) -> &mut StatLine {
        self.players[team].entry(id).or_default()
    }

    /// Book a play into the player lines and team totals. Every number
    /// here comes off the play record.
    pub fn book(&mut self, r: &PlayRecord) {
        let o = r.offense;
        let d = 1 - o;
        // A role can fall to either team (an interception return: the
        // defense blocks, the offense tackles and forces fumbles), so a
        // player's line goes to the team he is on; `rule` is for a man
        // not on the field.
        let side = |id: u32, rule: usize| r.side_of(id).unwrap_or(rule);
        let grade = |s: &mut StatLine, g: f64| {
            s.grade_sum += g;
            s.graded += 1;
        };
        let scrimmage = matches!(r.kind, PlayType::Run | PlayType::Pass | PlayType::Sack | PlayType::Scramble);
        for &id in &r.on_field[0] {
            let l = self.line(o, id);
            if scrimmage {
                l.snaps_off += 1;
            } else {
                l.snaps_st += 1;
            }
        }
        for &id in &r.on_field[1] {
            let l = self.line(d, id);
            if scrimmage {
                l.snaps_def += 1;
            } else {
                l.snaps_st += 1;
            }
        }
        for &(kind, team, id, yds, accepted) in &r.penalties {
            if accepted {
                let l = self.line(team, id);
                l.penalties += 1;
                l.penalty_yds += yds;
                grade(l, -0.8);
                self.team[team].penalties += 1;
                self.team[team].penalty_yds += yds;
                let _ = kind;
            }
        }
        for &id in &r.busts {
            self.line(side(id, d), id).busts += 1;
        }
        if r.nullified {
            self.team[o].time_of_possession += r.duration;
            return;
        }
        let ts = &mut self.team[o];
        if scrimmage {
            ts.plays += 1;
            ts.epa += r.epa;
            ts.successes += r.success() as u32;
            ts.yards += r.yards;
            if r.yards >= 20.0 || (r.kind == PlayType::Run && r.yards >= 10.0) {
                ts.explosive += 1;
            }
            if r.first_down {
                ts.first_downs += 1;
            }
            if r.down == 3 {
                ts.third_att += 1;
                ts.third_conv += (r.first_down || r.touchdown) as u32;
            }
            if r.down == 4 {
                ts.fourth_att += 1;
                ts.fourth_conv += (r.first_down || r.touchdown) as u32;
            }
            ts.time_of_possession += r.duration;
        }
        match r.kind {
            PlayType::Pass | PlayType::Sack | PlayType::Scramble => {
                let ts = &mut self.team[o];
                ts.dropbacks += 1;
                ts.pressures += r.pressured as u32;
                if r.kind == PlayType::Pass && !r.throw_away || r.kind == PlayType::Pass {
                    // attempts below
                }
                if let Some(qb) = r.passer {
                    let l = self.line(o, qb);
                    l.dropbacks += 1;
                    l.pressured += r.pressured as u32;
                    match r.kind {
                        PlayType::Sack => {
                            l.sacked += 1;
                            l.sack_yds += -r.yards;
                            l.pass_epa += r.epa;
                            grade(l, -0.4);
                        }
                        PlayType::Scramble => {}
                        _ => {
                            l.att += 1;
                            l.ttt += r.time_to_throw;
                            l.pass_epa += r.epa;
                            l.pass_success += r.success() as u32;
                            l.air_yds_att += r.air_yards;
                            if !r.throw_away {
                                l.x_cmp += expected_completion(r.air_yards, r.separation, r.pressured);
                            } else {
                                l.throw_aways += 1;
                                l.x_cmp += 0.0;
                            }
                            if r.play_action {
                                l.pa_att += 1;
                                l.pa_yds += if r.complete { r.yards } else { 0.0 };
                            }
                            if r.blitz {
                                l.blitz_att += 1;
                                l.blitz_yds += if r.complete { r.yards } else { 0.0 };
                            }
                            if r.air_yards >= 20.0 {
                                l.deep_att += 1;
                                l.deep_cmp += r.complete as u32;
                            }
                            if r.complete {
                                l.cmp += 1;
                                l.pass_yds += r.yards;
                                l.pass_td += r.touchdown as u32;
                                if r.air_yards >= 15.0 && r.separation < 1.5 {
                                    l.big_time += 1;
                                }
                            }
                            if r.interception {
                                l.int += 1;
                                l.turnover_worthy += 1;
                            } else if r.pbu.is_some() && r.separation < 0.8 {
                                l.turnover_worthy += 1;
                            }
                            let g = if r.interception {
                                -1.6
                            } else if r.complete {
                                0.25 + (r.epa * 0.3).clamp(-0.5, 1.2)
                            } else if r.drop {
                                0.3
                            } else if r.throw_away {
                                0.0
                            } else {
                                -0.25
                            };
                            grade(l, g);
                        }
                    }
                    let ts = &mut self.team[o];
                    if r.kind == PlayType::Pass {
                        ts.att += 1;
                        ts.cmp += r.complete as u32;
                        ts.ints += r.interception as u32;
                        if r.complete {
                            ts.pass_yards += r.yards;
                        }
                    }
                    if r.kind == PlayType::Sack {
                        ts.sacks += 1;
                        ts.pass_yards += r.yards;
                    }
                }
                if r.kind == PlayType::Scramble
                    && let Some(c) = r.carrier
                {
                    let l = self.line(o, c);
                    l.scrambles += 1;
                    l.scramble_yds += r.yards;
                    l.rush += 1;
                    l.rush_yds += r.yards;
                    l.rush_td += r.touchdown as u32;
                    l.rush_long = l.rush_long.max(r.yards);
                    l.rush_epa += r.epa;
                    self.team[o].rush_yards += r.yards;
                    self.team[o].carries += 1;
                }
                // Routes run by everyone in a pattern.
                for &id in &r.on_field[0] {
                    if Some(id) != r.passer && !r.blocks.iter().any(|b| b.0 == id && b.2) {
                        self.line(o, id).routes += 1;
                    }
                }
                if let Some(t) = r.target {
                    let l = self.line(o, t);
                    l.targets += 1;
                    l.tgt_air += r.air_yards;
                    l.drops += r.drop as u32;
                    if r.separation < 1.0 {
                        l.contested_tgt += 1;
                        l.contested_rec += r.complete as u32;
                    }
                    if r.complete {
                        l.rec += 1;
                        l.rec_yds += r.yards;
                        l.rec_td += r.touchdown as u32;
                        l.rec_long = l.rec_long.max(r.yards);
                        l.yac += r.yac;
                        l.sep_sum += r.separation;
                        l.rec_first += r.first_down as u32;
                        l.rec_epa += r.epa;
                        grade(l, 0.3 + (r.yac / 10.0).min(1.0) + if r.separation < 1.0 { 0.4 } else { 0.0 });
                    } else if r.drop {
                        grade(l, -1.2);
                    }
                }
                if let Some(cb) = r.defender {
                    let l = self.line(d, cb);
                    l.tgt_allowed += 1;
                    l.sep_allowed += r.separation;
                    if r.complete {
                        l.cmp_allowed += 1;
                        l.yds_allowed += r.yards;
                        l.td_allowed += r.touchdown as u32;
                        grade(l, -0.4 - (r.yards / 25.0).min(1.0));
                    } else {
                        grade(l, 0.3);
                    }
                }
                if let Some(p) = r.pbu {
                    let l = self.line(d, p);
                    l.pbus += 1;
                    grade(l, 1.0);
                }
                if let Some(p) = r.interceptor {
                    let l = self.line(d, p);
                    l.ints += 1;
                    grade(l, 2.0);
                }
                for &id in &r.on_field[1] {
                    if !r.pressures.contains(&id)
                        && !r.sackers.contains(&id)
                        && !r.blocks.iter().any(|b| b.1 == id && b.2)
                    {
                        self.line(d, id).cov_snaps += 1;
                    }
                }
            }
            PlayType::Run => {
                if let Some(c) = r.carrier {
                    let l = self.line(o, c);
                    l.rush += 1;
                    l.rush_yds += r.yards;
                    l.rush_td += r.touchdown as u32;
                    l.rush_long = l.rush_long.max(r.yards);
                    l.ybc += r.before_contact;
                    l.yaco += (r.yards - r.before_contact).max(0.0);
                    l.explosive_runs += (r.yards >= 10.0) as u32;
                    l.stuffs += (r.yards <= 0.0) as u32;
                    l.rush_success += r.success() as u32;
                    l.rush_first += r.first_down as u32;
                    l.rush_epa += r.epa;
                    let g = (r.epa * 0.5).clamp(-0.8, 1.5)
                        + 0.25 * r.missed_tackles.iter().filter(|m| m.1 == c).count() as f64;
                    grade(l, g);
                }
                let ts = &mut self.team[o];
                ts.rush_yards += r.yards;
                ts.carries += 1;
            }
            _ => {}
        }
        // Pressure, sacks, hits.
        let share = 1.0 / r.sackers.len().max(1) as f64;
        for &id in &r.sackers {
            let l = self.line(d, id);
            l.sacks += share;
            l.tfl += share;
            grade(l, 1.6);
        }
        for &id in &r.hits {
            if !r.sackers.contains(&id) {
                let l = self.line(d, id);
                l.qb_hits += 1;
                grade(l, 0.6);
            }
        }
        let mut seen = Vec::new();
        for &id in &r.pressures {
            if seen.contains(&id) {
                continue;
            }
            seen.push(id);
            let l = self.line(d, id);
            l.pressures += 1;
            if !r.sackers.contains(&id) && !r.hits.contains(&id) {
                l.hurries += 1;
                grade(l, 0.5);
            }
            l.ttp_sum += r.time_to_throw;
        }
        // Blocks: wins, pressures and sacks allowed (the beaten blocker).
        // On an interception return the defense blocks.
        for &(b, def, pass, won, secs, _) in &r.blocks {
            let gave_sack = !won && r.sackers.contains(&def);
            let gave_pressure = !won && (r.pressures.contains(&def) || gave_sack);
            {
                let l = self.line(side(b, o), b);
                if pass {
                    l.pb_snaps += 1;
                    let w = won || secs >= 2.5;
                    l.pb_wins += w as u32;
                    if gave_pressure {
                        l.pressures_allowed += 1;
                    }
                    if gave_sack {
                        l.sacks_allowed += 1.0;
                    }
                    grade(
                        l,
                        if gave_sack {
                            -1.5
                        } else if gave_pressure {
                            -0.8
                        } else if w {
                            0.15
                        } else {
                            -0.3
                        },
                    );
                } else {
                    l.rb_snaps += 1;
                    l.rb_wins += won as u32;
                    grade(l, if won { 0.25 } else { -0.35 });
                }
            }
            let l = self.line(side(def, d), def);
            if pass {
                l.rush_snaps += 1;
            }
            if !won {
                if pass && secs < 2.5 {
                    l.rush_wins += 1;
                }
                grade(l, 0.3);
            } else {
                grade(l, -0.1);
            }
        }
        for &(id, mv, _) in &r.rush_wins {
            let l = self.line(d, id);
            match mv {
                Move::Speed => l.speed_wins += 1,
                Move::Power => l.power_wins += 1,
                Move::Finesse => l.finesse_wins += 1,
            }
        }
        // Tackles.
        let ret_team = |kind: PlayType| if matches!(kind, PlayType::Punt | PlayType::Kickoff) { o } else { d };
        let tteam = if r.interception || (r.fumble_lost && r.yards != 0.0 && false) { o } else { ret_team(r.kind) };
        let n = r.tacklers.len();
        for (k, &id) in r.tacklers.iter().enumerate() {
            let l = self.line(side(id, tteam), id);
            if n == 1 || k == 0 {
                l.tackles += 1;
            } else {
                l.assists += 1;
            }
            if matches!(r.kind, PlayType::Punt | PlayType::Kickoff) {
                l.st_tackles += 1;
            }
            if scrimmage && r.yards < 0.0 && r.kind != PlayType::Sack {
                l.tfl += 1.0 / n as f64;
            }
            if scrimmage && !r.success() {
                l.stops += 1;
                grade(l, 0.4);
            } else {
                grade(l, 0.1);
            }
        }
        for &(tk, runner) in &r.missed_tackles {
            let team = side(tk, o);
            let l = self.line(team, tk);
            l.missed += 1;
            grade(l, -0.7);
            let rteam = 1 - team;
            self.line(rteam, runner).forced_missed += 1;
        }
        if let Some(f) = r.forced_fumble {
            let l = self.line(side(f, d), f);
            l.forced_fumbles += 1;
            grade(l, 1.5);
        }
        if let Some(f) = r.fumbled {
            let team = side(f, d);
            let l = self.line(team, f);
            l.fumbles += 1;
            l.fumbles_lost += r.fumble_lost as u32;
            grade(l, -1.5);
            if r.fumble_lost {
                self.team[team].fumbles_lost += 1;
                self.team[team].turnovers += 1;
            }
        }
        // A turnover in the box score is an interception or a lost fumble
        // (above); a missed field goal, a turnover on downs or an onside
        // kick hands the ball over without being one.
        if r.interception {
            self.team[o].turnovers += 1;
        }
        for &(id, won) in &r.freelance {
            let l = self.line(side(id, d), id);
            l.freelance += 1;
            l.freelance_won += won as u32;
        }
        if r.top_speed.0 != 0 {
            let id = r.top_speed.0;
            let l = self.line(side(id, d), id);
            l.top_speed = l.top_speed.max(r.top_speed.1);
        }
        // Special teams.
        match r.kind {
            PlayType::FieldGoal => {
                if let Some(k) = r.kicker {
                    let l = self.line(o, k);
                    let b = fg_band(r.kick_distance);
                    l.fg_att[b] += 1;
                    l.fg_made[b] += r.kick_good as u32;
                    if r.kick_good {
                        l.fg_long = l.fg_long.max(r.kick_distance);
                    }
                    if r.kick_pressure > 0.6 {
                        l.clutch_att += 1;
                        l.clutch_made += r.kick_good as u32;
                    }
                    grade(l, if r.kick_good { 0.4 + 0.03 * (r.kick_distance - 40.0).max(0.0) } else { -1.0 });
                }
                self.team[o].fg_att += 1;
                self.team[o].fg_made += r.kick_good as u32;
            }
            PlayType::ExtraPoint => {
                if let Some(k) = r.kicker {
                    let l = self.line(o, k);
                    l.xp_att += 1;
                    l.xp_made += r.kick_good as u32;
                }
            }
            PlayType::Punt => {
                if let Some(k) = r.kicker {
                    let l = self.line(o, k);
                    l.punts += 1;
                    l.punt_yds += r.kick_distance;
                    l.punt_net += r.kick_distance - r.return_yards - if r.touchback { 20.0 } else { 0.0 };
                    l.punt_in20 += r.inside20 as u32;
                    l.punt_tb += r.touchback as u32;
                }
                if let Some(pr) = r.returner
                    && !r.fair_catch
                {
                    let l = self.line(d, pr);
                    l.pr += 1;
                    l.pr_yds += r.return_yards;
                    // A muff the punting team scores on isn't his touchdown.
                    l.pr_td += (r.touchdown && !r.turnover) as u32;
                }
                self.team[o].punts += 1;
            }
            PlayType::Kickoff => {
                if let Some(k) = r.kicker {
                    let l = self.line(o, k);
                    l.kickoffs += 1;
                    l.ko_tb += r.touchback as u32;
                }
                if let Some(kr) = r.returner {
                    let l = self.line(d, kr);
                    l.kr += 1;
                    l.kr_yds += r.return_yards;
                    l.kr_td += (r.touchdown && !r.turnover) as u32;
                }
            }
            _ => {}
        }
    }
}

/// High school stats as a MaxPreps-style site shows them (8.5). Most schools
/// report accurately; some report late or not at all; padding is rare and
/// small. Scouts' film and debug tools see the truth.
pub fn as_reported(line: &StatLine, reports: bool, pads: f64) -> Option<StatLine> {
    if !reports {
        return None;
    }
    let mut l = StatLine {
        games: line.games,
        att: line.att,
        cmp: line.cmp,
        pass_yds: line.pass_yds,
        pass_td: line.pass_td,
        int: line.int,
        rush: line.rush,
        rush_yds: line.rush_yds,
        rush_td: line.rush_td,
        rec: line.rec,
        rec_yds: line.rec_yds,
        rec_td: line.rec_td,
        tackles: line.tackles + line.assists,
        sacks: line.sacks,
        ints: line.ints,
        fg_att: line.fg_att,
        fg_made: line.fg_made,
        xp_att: line.xp_att,
        xp_made: line.xp_made,
        ..StatLine::default()
    };
    if pads > 0.0 {
        // A padding scheme inflates yards and tackles a little.
        l.pass_yds *= 1.0 + pads;
        l.rush_yds *= 1.0 + pads;
        l.rec_yds *= 1.0 + pads;
        l.tackles = (l.tackles as f64 * (1.0 + 2.0 * pads)).round() as u32;
    }
    Some(l)
}
