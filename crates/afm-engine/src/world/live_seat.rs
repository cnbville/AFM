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
    /// Game rules changed for one game (update 2, G4): the game's date and
    /// its rules. Every other game starts from the standing rules
    /// (`rules`), set on the Plans page.
    #[serde(default)]
    pub week_rules: Option<(Date, GameRules)>,
    /// Your pregame talk for a game (dialogue D4): its date and your pick.
    #[serde(default)]
    pub pregame: Option<(Date, crate::game::talk::Pick)>,
    /// Your last talks (dialogue D4, D5), newest last: the next game's wear
    /// of your tones and your hand-offs to a captain read them.
    #[serde(default)]
    pub talks: Vec<super::folk::gametalk::Given>,
}

impl GameDay {
    /// The rules for a game: the week's when you changed them for it,
    /// else the standing rules.
    pub fn rules_for(&self, date: Date) -> GameRules {
        match self.week_rules {
            Some((d, r)) if d == date => r,
            _ => self.rules,
        }
    }
}

/// Your game rules (topic 2): the 4th-down and two-point charts and your
/// offense's usual tempo.
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct GameRules {
    pub fourth: FourthChart,
    pub two: TwoChart,
    pub tempo: Tempo,
    /// Update 2, G4: the timeout rule, the four-minute plan and your own
    /// cells on the 4th-down chart.
    #[serde(default)]
    pub timeouts: crate::game::live::TimeoutRule,
    #[serde(default)]
    pub four: crate::game::live::FourMinute,
    #[serde(default)]
    pub cells: [u32; 7],
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

/// An NFL club taken in season (`World::nfl_takeover`): the club, its
/// interim and its won-lost-tied record the day you took it.
pub(crate) type NflTakeover = (usize, Option<usize>, (u32, u32, u32));

impl World {
    /// Your team's index in the world at its level, and the level.
    pub fn seat_team(&self) -> Option<(Level, usize)> {
        let s = self.seat?;
        let t = match s.level {
            Level::Nfl => s.nfl_team?,
            Level::Other => s.other?,
            _ => s.program?,
        };
        Some((s.level, t))
    }

    /// The interim coaching your team's games still to play this season
    /// (the Jan 1 start, Cain 2026-11-08): the staff that got the team
    /// here prepares and calls them, the results go on his record, and you
    /// run everything else from day one. At a college he is the coach on
    /// the sideline (`Program::sideline`, set the day the man you replaced
    /// left); at an NFL club the coordinator the takeover named.
    pub fn seat_interim(&self) -> Option<usize> {
        let (level, t) = self.seat_team()?;
        let you = self.career.as_ref()?.coach;
        match level {
            Level::College => {
                let cs = self.college.as_ref()?;
                if !cs.started || cs.finished || !cs.games.iter().any(|g| g.involves(t) && g.played.is_none()) {
                    return None;
                }
                self.programs[t].sideline.filter(|&c| c != you)
            }
            Level::Nfl => {
                let (x, c, _) = self.nfl_takeover()?;
                if x != t || self.nfl[t].head_coach != Some(you) {
                    return None;
                }
                let s = self.nfl_season.as_ref()?;
                let alive = s.started
                    && s.champion.is_none()
                    && self.today <= self.dates.super_bowl
                    && (!s.booked || s.alive.iter().any(|a| a.contains(&t)));
                if alive { c } else { None }
            }
            _ => None,
        }
    }

    /// Your NFL club taken in season this football year (the Jan 1 start):
    /// the club, its interim (none when it had no coordinator) and its
    /// record the day you took it.
    pub(crate) fn nfl_takeover(&self) -> Option<NflTakeover> {
        let tk = self.folk.takeover.as_ref()?;
        let Some(super::folk::Team::Nfl(t)) = tk.team else { return None };
        if tk.started.is_none_or(|d| d < self.dates.start) {
            return None;
        }
        Some((t, tk.interim, tk.nfl_at?))
    }

    /// The coach on a club's sideline at the season's end: the interim at
    /// your club when he finished it (a Super Bowl run is his), else its
    /// head coach.
    pub(crate) fn nfl_sideline(&self, t: usize) -> Option<usize> {
        match self.nfl_takeover() {
            Some((x, Some(c), _)) if x == t => Some(c),
            _ => self.nfl[t].head_coach,
        }
    }

    /// The coach a club's season row goes to in the books: the coach of
    /// most of its games, as the record books credit it (minor bug 124's
    /// rule), so the man you replaced keeps a season he coached most of.
    pub(crate) fn nfl_book_coach(&self, t: usize) -> Option<usize> {
        let Some((x, c, at)) = self.nfl_takeover().filter(|x| x.0 == t) else { return self.nfl[t].head_coach };
        let s = self.nfl_season.as_ref();
        let games = s.map_or(0, |s| s.wins[x] + s.losses[x] + s.ties[x]);
        let before = at.0 + at.1 + at.2;
        let old = self.folk.takeover.as_ref().and_then(|tk| tk.old_coach);
        if games > 0 && 2 * before >= games { old.or(c) } else { c.or(self.nfl[t].head_coach) }
    }

    /// The head coach an NFL club plays under: the interim at yours while
    /// he has its games.
    pub(crate) fn nfl_head(&self, t: usize) -> Option<usize> {
        if self.seat_club(t)
            && let Some(c) = self.seat_interim()
        {
            return Some(c);
        }
        self.nfl[t].head_coach
    }

    /// Your game today as the day will play it, if you have one on the
    /// engine (section 9). Under an interim it is his game, not yours.
    pub fn seat_setup_today(&self) -> Option<SeatSetup> {
        // Booked already: today's game is over.
        if self.seat_kept_on(self.today).is_some() || self.seat_interim().is_some() {
            return None;
        }
        let (level, t) = self.seat_team()?;
        match level {
            Level::College => super::college::seat_setup(self, t),
            Level::Nfl => {
                let f = super::nfl_year::seat_fixture(self, t)?;
                super::pro::games::seat_setup(self, &f)
            }
            Level::HighSchool => super::hs_year::seat_setup(self, t),
            Level::Other => super::pro::other::seat_setup(self),
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
        // The game plays in its kickoff's section (the six-part day).
        if !self.kickoff_come() {
            return Err(match self.seat_kickoff(self.today) {
                Some(k) => format!("Your game kicks off at {}.", self.clock_of(k)),
                None => "You have no game today.".to_string(),
            });
        }
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
        let r = gd.rules_for(self.today);
        Prep {
            plan,
            study: film,
            fourth: r.fourth,
            two: r.two,
            tempo: r.tempo,
            timeouts: r.timeouts,
            four: r.four,
            cells: r.cells,
            talk: gd.pregame.filter(|p| p.0 == self.today).map(|p| p.1),
        }
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
        let worked = gd.rules_for(today) != GameRules::default()
            || gd.plan.date == Some(today)
            || gd.pregame.is_some_and(|p| p.0 == today)
            || gd.studied.iter().any(|s| s.0 == today);
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
