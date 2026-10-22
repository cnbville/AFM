//! Your game played live (section 9, `docs/design/game-day.md`). On the
//! morning of your game the game day screens build it exactly as the day
//! will (both teams as they dress, the venue, the seed) and you play it
//! snap by snap. When the day runs, the engine books your game from your
//! answers: the result you saw is the result the world keeps.

use crate::calendar::{Date, Level};
use crate::game::calling::Film;
use crate::game::live::{FourthChart, LiveGame, Params, PlanChange, Prep, Settings, Tempo, TwoChart};
use crate::game::{GameResult, GameTeam};

use super::World;

/// Your game today as the day will play it.
pub struct SeatSetup {
    pub level: Level,
    /// Home and away: program, club or school index.
    pub teams: [usize; 2],
    pub names: [String; 2],
    pub home: GameTeam,
    pub away: GameTeam,
    pub params: Params,
}

/// Your game being played live.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct LiveSeat {
    pub date: Date,
    pub level: Level,
    pub teams: [usize; 2],
    pub names: [String; 2],
    pub game: LiveGame,
}

/// Your game day settings and your film work, kept with the career.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct GameDay {
    /// Which snaps stop for you and the time for a call (Cain's two
    /// choices); used for every game you play live.
    pub settings: Settings,
    /// The opponent tapes you studied: (the date of your game they were
    /// studied for, the tape's game date, home, away).
    pub studied: Vec<(Date, Date, usize, usize)>,
    /// Tapes of the games your coming opponents played this season.
    pub tapes: Vec<super::tape::Tape>,
    /// Your game rules, kept from game to game (topic 2).
    #[serde(default)]
    pub rules: GameRules,
    /// This week's matchup plan and the hints you added to it.
    #[serde(default)]
    pub plan: WeekPlan,
}

/// Your game rules (topic 2): the 4th-down and two-point charts and your
/// offense's usual tempo.
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct GameRules {
    pub fourth: FourthChart,
    pub two: TwoChart,
    pub tempo: Tempo,
}

/// The matchup plan for one game (topic 2): players by id; None leaves it
/// to your coordinators.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct WeekPlan {
    /// The game it is for.
    pub date: Option<Date>,
    /// Attack this corner of theirs (Some(None): attack nobody).
    pub target_cb: Option<Option<u32>>,
    /// Bracket this receiver of theirs (Some(None): no bracket).
    pub bracket: Option<Option<u32>>,
    pub spy: Option<bool>,
    pub kick_away: Option<bool>,
    /// Hints you added: (tape date, home, away, the hint's key, its snaps).
    pub hints: Vec<(Date, usize, usize, String, Vec<usize>)>,
    /// Draws and screens against their upfield rush.
    #[serde(default)]
    pub draws: Option<bool>,
}

impl World {
    /// Your team's index in the world at its level, and the level.
    pub fn seat_team(&self) -> Option<(Level, usize)> {
        let s = self.seat?;
        let t = match s.level {
            Level::Nfl => s.nfl_team?,
            _ => s.program?,
        };
        Some((s.level, t))
    }

    /// Your game today as the day will play it, if you have one on the
    /// engine (section 9).
    pub fn seat_setup_today(&self) -> Option<SeatSetup> {
        let (level, t) = self.seat_team()?;
        match level {
            Level::College => super::college::seat_setup(self, t),
            Level::Nfl => {
                let f = super::nfl_year::seat_fixture(self, t)?;
                super::pro::games::seat_setup(self, &f)
            }
            Level::HighSchool => super::hs_year::seat_setup(self, t),
            _ => None,
        }
    }

    /// Kick off your game live: built as the day will play it, played to
    /// the first decision that waits for you.
    pub fn start_live(&mut self) -> Result<(), String> {
        if self.live.as_ref().is_some_and(|l| l.date == self.today) {
            return Ok(());
        }
        let s = self.seat_setup_today().ok_or("You have no game today.")?;
        let (_, t) = self.seat_team().ok_or("You have no team.")?;
        let side = if s.teams[0] == t { 0 } else { 1 };
        let opp = s.teams[1 - side];
        let them = if side == 0 { &s.away } else { &s.home };
        let prep = self.seat_prep(opp, them);
        let mut game = LiveGame::new(s.home, s.away, s.params, side, self.gameday.settings);
        game.ctl.prep = prep;
        self.live = Some(LiveSeat { date: self.today, level: s.level, teams: s.teams, names: s.names, game });
        Ok(())
    }

    /// Your week's work for today's game against `opp` (whose team as it
    /// dresses is `them`): the plan by their players' places on the field,
    /// the film you studied and the hints you added, your game rules.
    pub fn seat_prep(&self, opp: usize, them: &GameTeam) -> Prep {
        let gd = &self.gameday;
        let at = |id: Option<Option<u32>>| id.map(|x| x.and_then(|id| them.players.iter().position(|p| p.id == id)));
        let mine = gd.plan.date == Some(self.today);
        let plan = if mine {
            PlanChange {
                target_cb: at(gd.plan.target_cb),
                bracket: at(gd.plan.bracket),
                spy: gd.plan.spy,
                kick_away: gd.plan.kick_away,
                draws: gd.plan.draws,
            }
        } else {
            PlanChange::default()
        };
        // The tapes you studied for this game, and the hints you added
        // studied again.
        let mut film: Option<Film> = None;
        let mut add = |f: Film| film.get_or_insert_with(Film::default).add(&f);
        for t in gd.tapes.iter().filter(|t| t.teams.contains(&opp)) {
            let side = if t.teams[0] == opp { 0 } else { 1 };
            if gd.studied.iter().any(|s| s.0 == self.today && s.1 == t.date && [s.2, s.3] == t.teams) {
                add(t.film[side as usize].clone());
            }
            if mine {
                for h in gd.plan.hints.iter().filter(|h| h.0 == t.date && [h.1, h.2] == t.teams) {
                    add(t.film_of(side, &h.4));
                }
            }
        }
        Prep { plan, study: film, fourth: gd.rules.fourth, two: gd.rules.two, tempo: gd.rules.tempo }
    }

    /// The result of your live game for the day's books: your answers so
    /// far, the staff for the rest. Only today's game at this level.
    pub(crate) fn live_result(&self, level: Level) -> Option<([usize; 2], GameResult)> {
        if let Some(l) = self.live.as_ref() {
            if l.level != level || l.date != self.today {
                return None;
            }
            let mut g = l.game.clone();
            return Some((l.teams, g.finish()));
        }
        // Not played live: your week's work still goes in, and your staff
        // coaches the whole game with it.
        let gd = &self.gameday;
        let today = self.today;
        let worked =
            gd.rules != GameRules::default() || gd.plan.date == Some(today) || gd.studied.iter().any(|s| s.0 == today);
        if !worked || self.seat_team()?.0 != level {
            return None;
        }
        let s = self.seat_setup_today()?;
        let (_, t) = self.seat_team()?;
        let side = if s.teams[0] == t { 0 } else { 1 };
        let prep = self.seat_prep(s.teams[1 - side], if side == 0 { &s.away } else { &s.home });
        if prep.is_empty() {
            return None;
        }
        let settings = Settings { stop: crate::game::live::StopMode::Off, ..self.gameday.settings };
        let mut g = LiveGame::new(s.home, s.away, s.params, side, settings);
        g.ctl.prep = prep;
        Some((s.teams, g.finish()))
    }

    /// The day booked your game: the live game is done.
    pub(crate) fn live_booked(&mut self, level: Level, teams: [usize; 2]) {
        if self.live.as_ref().is_some_and(|l| l.level == level && l.teams == teams) {
            self.live = None;
        }
    }
}
