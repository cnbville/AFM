//! Your seat in the world (section 8): what the game keeps for your own
//! team beyond what every AI team has. Your games' full box scores and play
//! logs, your depth chart, and plain reads of the season state the screens
//! need. Nothing here changes an AI team; a world without a seat plays
//! exactly as before.

use crate::calendar::Date;
use crate::game::GameStats;
use crate::positions::Position;

use super::front_office as fo;
use super::{World, pro};

/// One of your team's games, kept whole for its game page: the box score,
/// every play with the caller's reasons, and each side's name that day.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct SeatGame {
    pub date: Date,
    pub level: crate::calendar::Level,
    /// High school, college program or NFL club index of each side (home, away).
    pub teams: [usize; 2],
    pub names: [String; 2],
    pub stats: GameStats,
    /// Everything that decided the game, packed (`ReplaySetup::pack`, about
    /// 50 KB), so the film can be played again from its seed. Empty for a
    /// game kept before the views were built.
    #[serde(default)]
    pub replay: Vec<u8>,
    /// Your drawings on the film (film-room tool 1), saved with the clip.
    #[serde(default)]
    pub marks: Vec<FilmMark>,
    /// The players in it who have since left football, and so the world:
    /// their names for its pages (`seat_names::keep`).
    #[serde(default)]
    pub gone: Vec<super::seat_names::Gone>,
    /// What the home side was expected to win by before the game: the
    /// closing line where the books hung one, 0 at a neutral site without
    /// one. None for a home game with no line (and games kept before).
    #[serde(default)]
    pub spread: Option<f32>,
}

/// A drawing on a paused frame of the film: a line, an arrow, a circle or
/// a free hand, on the turf in field yards (x from the left end line of
/// the field as the view shows it, 0-120; y across, 0-53.3), so it sits
/// in the same spot in the 2D and 3D views and under every camera.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct FilmMark {
    /// The play's number in the game's play log and the moment, seconds
    /// into the snap.
    pub play: u32,
    pub t: f32,
    /// 0 line, 1 arrow, 2 circle, 3 free hand.
    pub kind: u8,
    /// 0 yellow, 1 red, 2 white, 3 blue.
    pub color: u8,
    pub pts: Vec<(f32, f32)>,
}

/// Drawings kept per game at most (a heavy film session).
pub const MARKS_PER_GAME: usize = 400;

/// How many of your games are kept whole (about two seasons and a
/// postseason); older ones keep their score in the season records.
const KEEP_GAMES: usize = 40;

impl World {
    /// Whether a game between these teams at this level is your team's.
    pub(crate) fn seat_plays(&self, level: crate::calendar::Level, teams: [usize; 2]) -> bool {
        let Some(seat) = self.seat else { return false };
        let mine = match level {
            crate::calendar::Level::Nfl => seat.nfl_team.is_some_and(|t| teams.contains(&t)),
            crate::calendar::Level::Other => seat.other.is_some_and(|t| teams.contains(&t)),
            _ => seat.program.is_some_and(|p| teams.contains(&p)),
        };
        mine && seat.level == level
    }

    /// Keeps a game whole when it is your team's, with its setup packed for
    /// watching it back.
    pub(crate) fn keep_seat_game(
        &mut self,
        level: crate::calendar::Level,
        teams: [usize; 2],
        stats: &GameStats,
        replay: Option<&crate::game::ReplaySetup>,
    ) {
        if !self.seat_plays(level, teams) {
            return;
        }
        // The day booked your game: a live game of it is over.
        self.live_booked(level, teams);
        let names = match level {
            crate::calendar::Level::Nfl => teams.map(|t| self.nfl_team_name(t)),
            crate::calendar::Level::HighSchool => teams.map(|h| self.school_name(h).to_string()),
            crate::calendar::Level::Other => teams.map(|f| super::pro::other::name(f, self.year).to_string()),
            _ => teams.map(|p| self.program_name(p).to_string()),
        };
        let replay = replay.map(|r| r.pack()).unwrap_or_default();
        self.seat_games.push(SeatGame {
            date: self.today,
            level,
            teams,
            names,
            stats: stats.clone(),
            replay,
            marks: Vec::new(),
            gone: Vec::new(),
            spread: None,
        });
        if self.seat_games.len() > KEEP_GAMES {
            let cut = self.seat_games.len() - KEEP_GAMES;
            self.seat_games.drain(..cut);
        }
    }

    /// The pregame line of the game just kept, when it is yours: `line` is
    /// the home side's spread (negative when favoured), and a neutral site
    /// without a line is even.
    pub(crate) fn seat_line(
        &mut self,
        level: crate::calendar::Level,
        teams: [usize; 2],
        line: Option<[f32; 2]>,
        neutral: bool,
    ) {
        let today = self.today;
        if let Some(g) = self.seat_games.last_mut()
            && g.level == level
            && g.teams == teams
            && g.date == today
            && g.spread.is_none()
        {
            g.spread = line.map(|l| -l[0]).or(neutral.then_some(0.0));
        }
    }

    /// An NFL club's name in this season (its franchise by era).
    pub fn nfl_team_name(&self, t: usize) -> String {
        self.nfl.get(t).map_or_else(|| format!("Team {t}"), |team| team.name.to_string())
    }

    /// A conference's name as this world shows it (fictional in a world
    /// built with the fictional league option).
    pub fn conference_name(&self, real: &str) -> String {
        if self.options.fictional_league { super::names::fictional_conference(real) } else { real.to_string() }
    }

    /// Your team as the engine would dress it today, with its level.
    pub fn seat_game_team(&self) -> Option<(crate::game::GameTeam, crate::player::Level)> {
        use crate::calendar::Level as Cal;
        let seat = self.seat?;
        Some(match seat.level {
            Cal::HighSchool => (super::hs_year::seat_team(self, seat.program?)?, crate::player::Level::HS),
            Cal::Nfl => (super::pro::games::game_side(self, seat.nfl_team?)?.team, crate::player::Level::NFL),
            Cal::Other => (super::pro::other::seat_side(self, seat.other?)?, crate::player::Level::NFL),
            _ => {
                let prog = self.programs.get(seat.program?)?;
                let mut team = super::season::game_team(self, prog)?;
                crate::training::seat::sit_hurt(self, &mut team);
                (team, super::division_level(prog.division))
            }
        })
    }

    /// Your depth chart order at a position (player ids, best first).
    pub fn set_seat_depth(&mut self, pos: Position, ids: Vec<u32>) {
        self.seat_depth.retain(|(p, _)| *p != pos);
        if !ids.is_empty() {
            self.seat_depth.push((pos, ids));
        }
    }

    /// The depth chart your team would take into a game today, built the
    /// way the engine builds it on game day: per position, player ids in
    /// order, with players from another spot where the staff would use
    /// them (a two-way high schooler, a corner at receiver) and your own
    /// order first where you set one. None while your team can't dress a
    /// game team.
    pub fn seat_game_depth(&self) -> Option<Vec<(Position, Vec<u32>)>> {
        let (team, level) = self.seat_game_team()?;
        let margin = crate::game::GameSetup::new(&team, &team, level, 0).two_way_margin[0];
        let d = crate::game::roster::depth_chart(&team, margin);
        Some(
            Position::ALL.iter().map(|&pos| (pos, d.list(pos).iter().map(|&i| team.players[i].id).collect())).collect(),
        )
    }

    /// Whether your NFL club's duty `d` is yours and one of its contracts
    /// passes `test` (lane 10: a morning stop only when it holds a choice).
    fn fo_holds(&self, d: fo::Duty, test: impl Fn(&pro::money::Contract, usize) -> bool) -> bool {
        let Some(t) = self.seat.and_then(|s| s.nfl_team) else { return false };
        fo::yours(self, t, d)
            && self.clubs[t].roster.iter().any(|&i| {
                self.people[i].pro.as_ref().and_then(|c| c.contract.as_ref()).is_some_and(|k| k.team == t && test(k, i))
            })
    }

    /// What waits for you on the morning of `d`, before the day plays: a
    /// meeting, your team's game, or a dated day at your level when you
    /// decide things (`your_days_on`: the combine, the tag window, free
    /// agency, the draft, cutdown, signing day, a new training block).
    /// Continue stops that morning, and the next Continue plays the day.
    pub fn morning_stop(&self, d: Date) -> Option<String> {
        use crate::calendar::Level;
        let s = self.seat?;
        let game = match s.level {
            Level::College => s.program.and_then(|p| {
                let f = self
                    .college_fixtures(Some(p))
                    .into_iter()
                    .find(|f| f.date == d && f.score.is_none() && !f.cancelled)?;
                let opp = if f.home == p { f.away } else { f.home };
                Some(format!(
                    "Game day: {} {}",
                    if f.home == p || f.neutral { "vs" } else { "at" },
                    self.program_name(opp)
                ))
            }),
            Level::Nfl => s.nfl_team.and_then(|t| {
                let f = self.nfl_fixtures(Some(t)).into_iter().find(|f| f.when == Some(d) && !f.played)?;
                let opp = if f.home == t { f.away } else { f.home };
                Some(format!("Game day: {} the {}", if f.home == t { "vs" } else { "at" }, self.nfl_team_name(opp)))
            }),
            Level::HighSchool => s.program.and_then(|h| {
                let f = self.hs_fixtures(h).into_iter().find(|f| f.date == d)?;
                let opp = if f.home == h { f.away } else { f.home };
                Some(format!("Game day: {} {}", if f.home == h { "vs" } else { "at" }, self.school_name(opp)))
            }),
            Level::Other => s.other.and_then(|fr| {
                let (_, h, a) = super::pro::other::seat_fixtures(self).into_iter().find(|f| f.0 == d)?;
                let opp = if h == fr { a } else { h };
                let name = super::pro::other::name(opp, d.year());
                Some(format!("Game day: {} the {name}", if h == fr { "vs" } else { "at" }))
            }),
            _ => None,
        };
        // Every reason the morning holds, your game first: a meeting no
        // longer hides the day's dates (the tag window behind February's
        // takeover meetings).
        let mut lines: Vec<String> = game.into_iter().collect();
        // A dated meeting waiting for you (people, takeover item 1).
        lines.extend(super::folk::scene::due(self, d));
        let frame = crate::training::seat::seat_frame(self, &self.dates);
        lines.extend(self.your_days_on(d, frame.as_ref()).into_iter().map(|x| x.1));
        (!lines.is_empty()).then(|| lines.join(" · "))
    }

    /// The days at your level you act on from `from` to `to`, by date, with
    /// a short name each (the Continue bar's "next").
    pub fn your_days(&self, from: Date, to: Date) -> Vec<(Date, String)> {
        let frame = crate::training::seat::seat_frame(self, &self.dates);
        let mut v = Vec::new();
        let mut d = from;
        while d <= to {
            v.extend(self.your_days_on(d, frame.as_ref()).into_iter().map(|x| (d, x.0)));
            d = d.add(1);
        }
        v
    }

    /// A dated day at your level when you decide things, as its short name
    /// and its morning line: every real date that opens or closes something
    /// you can do at your seat. A condition is read only on its own day.
    fn your_days_on(&self, d: Date, frame: Option<&crate::training::rules::Frame>) -> Vec<(String, String)> {
        use crate::calendar::Level;
        let Some(s) = self.seat else { return Vec::new() };
        let y = &self.dates;
        let mut v: Vec<(String, String)> = Vec::new();
        let mut add = |on: bool, name: &str, line: String| {
            if on {
                v.push((name.to_string(), line));
            }
        };
        // A new training block for your team opens: its weeks' work is
        // yours to set. The season stops on your games instead, and the
        // weeks the players are on their own hold nothing.
        if let Some(f) = frame {
            use crate::training::rules::{Block, Tier};
            let b = f.block(d);
            let staff_runs = match f.tier {
                Tier::Pro => matches!(b, Block::Phase1 | Block::Phase2 | Block::Otas | Block::Camp),
                _ => matches!(b, Block::Winter | Block::Spring | Block::Summer | Block::Camp),
            };
            if staff_runs && b != f.block(d.add(-1)) {
                let label = b.label(f.tier);
                // "Summer workouts" and "OTAs and minicamp" are many.
                let (open, its) =
                    if label.ends_with('s') || label.contains(" and ") { ("open", "their") } else { ("opens", "its") };
                add(true, label, format!("{label} {open} today: set {its} weeks' work on Training."));
            }
        }
        match s.level {
            Level::Nfl => {
                // All-star week: the rosters are named and your scouting
                // trips to see the seniors practise run to the end of
                // January (`scouting::own::trip_day`).
                let all_star = super::clock::all_star_roster_day(y);
                add(
                    d == all_star && d.month() == 1,
                    "All-star week",
                    "The all-star games name their rosters: a trip to see the seniors practise is open to the end of January.".into(),
                );
                add(
                    self.year >= 1993 && d == y.tag_window_open,
                    "The tag window opens",
                    "The tag window opens today: re-sign, tag or tender your expiring players.".into(),
                );
                // Your combine interviews are open only these days
                // (`scouting::own::rooms`).
                add(
                    d == y.combine_start,
                    "The combine opens",
                    format!(
                        "The combine opens in Indianapolis today: interview the prospects on your list by {}.",
                        y.combine_end.pretty()
                    ),
                );
                add(d == y.tag_deadline.date, "The tag deadline", "The franchise tag deadline is today.".into());
                if let Some(m) = y.tampering_open {
                    add(
                        d == m.date,
                        "The negotiating window opens",
                        "The negotiating window opens at noon: agree terms with other clubs' free agents.".into(),
                    );
                }
                add(
                    d == y.league_year.date,
                    "Free agency opens",
                    "The new league year opens today, and free agency with it.".into(),
                );
                // The last day to sign another club's tendered restricted
                // free agent to an offer sheet (`front_office::nfl`, offer
                // sheets), when one is still out there.
                add(
                    d == y.rfa_deadline && self.offer_sheet_open(),
                    "The offer sheet deadline",
                    "The offer sheet deadline is today: the last day to sign another club's restricted free agent."
                        .into(),
                );
                // Prospects on your visit list who haven't been in.
                if d == y.top30_deadline {
                    let n = self.visits_owed();
                    add(
                        n > 0,
                        "Top-30 visits close",
                        format!(
                            "Top-30 visits close today: {n} {} on your visit list {} been in.",
                            super::folk::noun(n, "prospect"),
                            if n == 1 { "hasn't" } else { "haven't" }
                        ),
                    );
                }
                add(d == y.draft[0].date, "The draft", "The draft opens tonight.".into());
                // From June 1 every release spreads its dead money over two
                // years (`front_office::nfl::release`): a day for whoever
                // runs the cap.
                add(
                    d == y.june_1
                        && self.rules.salary_cap.is_some()
                        && s.nfl_team.is_some_and(|t| fo::yours(self, t, fo::Duty::Cap)),
                    "June 1",
                    "June 1: from today a release spreads his dead money over two years.".into(),
                );
                add(d == y.cutdown.date, "Cutdown day", "Cutdown day: rosters go to 53 today.".into());
                add(d == y.trade_deadline.date, "The trade deadline", "The trade deadline is today.".into());
                // Your own front office's days (lane 10), only when they
                // hold a choice for you.
                add(
                    d == y.tag_long_term_deadline.date
                        && self.fo_holds(fo::Duty::Contracts, |k, i| {
                            k.signed == self.year
                                && matches!(k.kind, pro::money::Kind::Franchise | pro::money::Kind::Transition)
                                && self.people[i].pro.as_ref().is_some_and(|c| c.status == pro::Status::Roster)
                        }),
                    "The tagged players' deadline",
                    "The last day for a long-term deal with your tagged player: after 4pm ET he plays the year on the tag.".into(),
                );
                add(
                    d == y.fifth_year_option
                        && self.year >= 2014
                        && self.fo_holds(fo::Duty::Contracts, |k, i| {
                            k.kind == pro::money::Kind::Rookie
                                && k.option.is_none()
                                && self.people[i]
                                    .pro
                                    .as_ref()
                                    .and_then(|c| c.draft)
                                    .is_some_and(|d| d.1 == 1 && d.0 == self.year - 3)
                        }),
                    "The fifth-year option deadline",
                    "The fifth-year option deadline is today: pick up or decline your first-rounder's option.".into(),
                );
                add(
                    d == y.waiver_claims.date && self.fo_holds(fo::Duty::Waivers, |_, _| true),
                    "Waiver claims",
                    "Waiver claims are due at noon: the players cut yesterday are on the wire.".into(),
                );
                add(
                    d == y.practice_squads && self.fo_holds(fo::Duty::PracticeSquad, |_, _| true),
                    "Practice squads form",
                    "Practice squads form today: sign your squad from the players who cleared waivers.".into(),
                );
            }
            Level::College => {
                if let Some(e) = y.early_signing {
                    add(d == e, "The early signing period", "The early signing period opens today.".into());
                }
                // A signing day after the Super Bowl falls in the next
                // football year (part C 1).
                let late = self.prev_dates.as_ref().is_some_and(|p| p.signing_day == d && d > p.super_bowl);
                add(d == y.signing_day || late, "National signing day", "National signing day.".into());
                add(d == y.portal_open, "The portal opens", "The transfer portal opens today.".into());
                // Juniors' official visits open (2018 on): yours to set.
                add(
                    d == y.official_visits_from
                        && self.rules.junior_officials
                        && s.program.is_some_and(|c| fo::yours_program(self, c, fo::Duty::Visits)),
                    "Junior official visits open",
                    "Juniors may take official visits from today to June 30: set them on your recruiting desk.".into(),
                );
                // The NFL's advisory committee takes names to December 10
                // (`scouting::own::request_advisory`), and your
                // underclassmen decide by the declaration deadline after
                // your stay-or-go talks.
                let advisory = d == Date::ymd(y.season, 12, 10);
                let read = if advisory || d == y.declare_deadline { self.underclassmen_with_a_read() } else { 0 };
                let men = if read == 1 { "underclassman" } else { "underclassmen" };
                add(
                    advisory && read > 0,
                    "Advisory requests close",
                    format!(
                        "The NFL's advisory committee takes names until tonight: you have {read} {men} with an NFL read."
                    ),
                );
                add(
                    d == y.declare_deadline && read > 0,
                    "The declaration deadline",
                    format!(
                        "The deadline to declare for the draft is today: talk to your {read} {men} with an NFL read before {} {}.",
                        if read == 1 { "he" } else { "they" },
                        if read == 1 { "decides" } else { "decide" }
                    ),
                );
            }
            Level::HighSchool => {
                // Your seniors with offers sign: the last calls to colleges
                // and the ceremony.
                let signing = y.early_signing == Some(d) || d == y.signing_day;
                let n = if signing { self.seniors_to_sign() } else { 0 };
                let names =
                    |what: &str| format!("{what}: {n} {} with offers still to sign.", super::folk::noun(n, "senior"));
                add(
                    y.early_signing == Some(d) && n > 0,
                    "The early signing period",
                    names("The early signing period opens today"),
                );
                add(d == y.signing_day && n > 0, "National signing day", names("National signing day"));
            }
            _ => {}
        }
        v
    }

    /// Whether another club's tendered restricted free agent can still
    /// take your offer sheet, and offer sheets are yours.
    fn offer_sheet_open(&self) -> bool {
        let Some(t) = self.seat.and_then(|s| s.nfl_team) else { return false };
        fo::yours(self, t, fo::Duty::Tags)
            && self.clubs.iter().enumerate().filter(|&(o, _)| o != t).any(|(o, club)| {
                club.roster.iter().any(|&i| {
                    self.people[i].pro.as_ref().is_some_and(|c| {
                        c.team == Some(o)
                            && c.rfa.is_some()
                            && c.contract.as_ref().is_some_and(|k| k.kind == pro::money::Kind::Tender)
                    })
                })
            })
    }

    /// The declared prospects on your top-30 visit list who haven't been in
    /// this spring.
    fn visits_owed(&self) -> usize {
        use super::scouting::predraft;
        let Some(t) = self.seat.and_then(|s| s.nfl_team) else { return 0 };
        let year = self.today.year();
        predraft::seat_list(self, t, predraft::LIST_VISIT)
            .into_iter()
            .filter(|&id| {
                super::scouting::place(self, id).is_some_and(|i| self.people[i].stage == super::Stage::Declared)
                    && !self
                        .scouting
                        .own
                        .interviews
                        .iter()
                        .any(|x| x.id == id && x.room == "Top-30 visit" && x.date.year() == year)
            })
            .count()
    }

    /// Your underclassmen with an NFL read (your college program).
    fn underclassmen_with_a_read(&self) -> usize {
        let Some(c) = self.seat.and_then(|s| s.program) else { return 0 };
        self.programs.get(c).map_or(0, |p| {
            p.roster
                .iter()
                .filter(|&&i| {
                    let q = &self.people[i];
                    q.stage == super::Stage::College && q.college == Some(c) && q.cal.draft_read.is_some()
                })
                .count()
        })
    }

    /// Your high school's seniors who hold an offer and haven't signed.
    fn seniors_to_sign(&self) -> usize {
        let Some(h) = self.seat.and_then(|s| s.program) else { return 0 };
        let seniors = if self.today < self.dates.hs_school_start { self.year } else { self.year + 1 };
        self.high_schools.get(h).map_or(0, |school| {
            school
                .roster
                .iter()
                .filter(|&&i| {
                    let p = &self.people[i];
                    p.stage == super::Stage::HighSchool
                        && p.class_of == seniors
                        && !p.cal.offers.is_empty()
                        && p.cal.signed.is_none()
                })
                .count()
        })
    }

    /// A coach's record in his current job, with this season's games that
    /// aren't in it yet: a head coach's season goes into his record when it
    /// is booked at its end.
    pub fn stint_record(&self, c: usize) -> (u32, u32) {
        use super::coach::{Employer, Role};
        let Some(co) = self.coaches.get(c) else { return (0, 0) };
        let live = match co.job.filter(|j| j.role == Role::HeadCoach).map(|j| j.employer) {
            Some(Employer::HighSchool(h)) => {
                let filed =
                    self.high_schools.get(h).and_then(|s| s.seasons.last()).is_some_and(|s| s.year == self.year);
                if filed { (0, 0) } else { self.hs_record(h).unwrap_or((0, 0)) }
            }
            // Only this football year's season: after the Super Bowl the
            // season held is last year's, its bowl games his predecessor's.
            Some(Employer::College(p)) => self
                .college
                .as_ref()
                .filter(|cs| cs.year == self.year)
                .and_then(|cs| Some((cs.results.get(p)?, cs.credited.get(p)?)))
                .map_or((0, 0), |(r, k)| (r.wins.saturating_sub(k.0), r.losses.saturating_sub(k.1))),
            Some(Employer::Nfl(t)) => match self.nfl_season.as_ref() {
                Some(s) if !s.booked => {
                    let (wn, l, _) = self.nfl_record(t);
                    (wn, l)
                }
                _ => (0, 0),
            },
            // The other leagues' seasons are booked when they end.
            Some(Employer::Other(f)) => self
                .others
                .live
                .iter()
                .flat_map(|l| l.table.iter())
                .find(|r| r.fr as usize == f)
                .map_or((0, 0), |r| (r.w as u32, r.l as u32)),
            None => (0, 0),
        };
        (co.wins + live.0, co.losses + live.1)
    }

    /// His whole record as a head coach: every finished head coaching job
    /// and the one he holds.
    pub fn career_record(&self, c: usize) -> (u32, u32) {
        let Some(co) = self.coaches.get(c) else { return (0, 0) };
        let past = co.history.iter().filter(|s| s.role == super::coach::Role::HeadCoach);
        let (pw, pl) = past.fold((0, 0), |a, s| (a.0 + s.wins, a.1 + s.losses));
        let now =
            if co.job.is_some_and(|j| j.role == super::coach::Role::HeadCoach) { self.stint_record(c) } else { (0, 0) };
        (pw + now.0, pl + now.1)
    }

    /// Sets (or with None clears) a player's plan focus.
    pub fn set_focus(&mut self, player: u32, focus: Option<crate::attributes::Attr>) {
        self.seat_focus.retain(|(id, _)| *id != player);
        if let Some(a) = focus {
            self.seat_focus.push((player, a.index() as u8));
        }
    }

    pub fn focus_of(&self, player: u32) -> Option<crate::attributes::Attr> {
        self.seat_focus.iter().find(|(id, _)| *id == player).map(|&(_, a)| crate::attributes::Attr::ALL[a as usize])
    }

    /// The plan focus of each player on your roster now, by person index
    /// (a plan stays with your team: a player who leaves drops it).
    pub(crate) fn focus_by_person(&self) -> std::collections::HashMap<usize, crate::attributes::Attr> {
        let Some(seat) = self.seat else { return Default::default() };
        if self.seat_focus.is_empty() {
            return Default::default();
        }
        let roster: &[usize] = match seat.level {
            crate::calendar::Level::Nfl => seat.nfl_team.and_then(|t| self.clubs.get(t)).map_or(&[], |c| &c.roster),
            crate::calendar::Level::HighSchool => seat.program.map_or(&[], |h| &self.high_schools[h].roster),
            _ => seat.program.map_or(&[], |p| &self.programs[p].roster),
        };
        roster.iter().filter_map(|&i| self.focus_of(self.people[i].player.id).map(|a| (i, a))).collect()
    }

    /// Whether a college program is yours (its game team takes your order).
    pub(crate) fn seat_program(&self, prog: usize) -> bool {
        self.seat.is_some_and(|s| s.program == Some(prog) && s.level == crate::calendar::Level::College)
    }

    /// Your high school, when you coach one.
    pub(crate) fn seat_school(&self) -> Option<usize> {
        self.seat.filter(|s| s.level == crate::calendar::Level::HighSchool).and_then(|s| s.program)
    }

    pub(crate) fn seat_club(&self, t: usize) -> bool {
        self.seat.is_some_and(|s| s.nfl_team == Some(t) && s.level == crate::calendar::Level::Nfl)
    }
}

/// A college team's season so far, for the screens.
#[derive(Clone, Copy, Debug, Default)]
pub struct Record {
    pub wins: u32,
    pub losses: u32,
    pub conf_wins: u32,
    pub conf_losses: u32,
}

/// A college game on this season's schedule, for the screens.
#[derive(Clone, Debug)]
pub struct Fixture {
    pub date: Date,
    pub kickoff: (u16, u16),
    pub home: usize,
    pub away: usize,
    pub name: Option<&'static str>,
    pub conference: bool,
    pub neutral: bool,
    pub why: Vec<&'static str>,
    /// Home and away points, once played.
    pub score: Option<[u32; 2]>,
    /// Called off: a hurricane, or a team too short to dress.
    pub cancelled: bool,
    /// The betting line (C8): the home side's spread and the total. The
    /// books post a game's line in its own week; none below FCS.
    pub line: Option<[f32; 2]>,
    /// The crowd, once played (later extras topic 1).
    pub attendance: u32,
}

/// An NFL game on this season's schedule.
#[derive(Clone, Debug)]
pub struct NflFixture {
    pub week: usize,
    pub home: usize,
    pub away: usize,
    pub when: Option<Date>,
    /// Kickoff, minutes after midnight Eastern.
    pub minute: Option<u16>,
    pub played: bool,
    /// Home and away points, for a game played on the engine.
    pub score: Option<[u32; 2]>,
    /// The betting line (C8), once the week's lines are out.
    pub line: Option<[f32; 2]>,
    /// The crowd, once played (later extras topic 1).
    pub attendance: u32,
}

impl World {
    /// This college season's record for a program.
    pub fn college_record(&self, prog: usize) -> Record {
        self.college_record_in(prog, false)
    }

    /// A program's record this season, or last (`prev`).
    pub fn college_record_in(&self, prog: usize, prev: bool) -> Record {
        let cs = if prev { self.college_prev.as_ref() } else { self.college.as_ref() };
        cs.and_then(|cs| cs.results.get(prog)).map_or(Record::default(), |r| Record {
            wins: r.wins,
            losses: r.losses,
            conf_wins: r.conf_wins,
            conf_losses: r.conf_losses,
        })
    }

    /// This college season's games for a program (all programs if None).
    pub fn college_fixtures(&self, prog: Option<usize>) -> Vec<Fixture> {
        self.college_fixtures_in(prog, false)
    }

    /// This season's games, or last season's (`prev`).
    pub fn college_fixtures_in(&self, prog: Option<usize>, prev: bool) -> Vec<Fixture> {
        let cs = if prev { self.college_prev.as_ref() } else { self.college.as_ref() };
        let Some(cs) = cs else { return Vec::new() };
        cs.games
            .iter()
            .filter(|g| prog.is_none_or(|p| g.home == p || g.away == p))
            .map(|g| Fixture {
                date: g.date,
                kickoff: g.kickoff,
                home: g.home,
                away: g.away,
                name: g.name,
                conference: g.conference,
                neutral: g.site.is_some(),
                why: g.why.clone(),
                score: g.played.filter(|p| !p.cancelled).map(|p| p.score),
                cancelled: g.played.is_some_and(|p| p.cancelled),
                // The week's lines are out from the Sunday before.
                line: g.line.or_else(|| {
                    (g.played.is_none() && g.date >= self.today && g.date <= self.today.add(6))
                        .then(|| super::finance::lines::college(self, g.home, g.away, g.site.is_some()))
                        .flatten()
                }),
                attendance: g.attendance,
            })
            .collect()
    }

    /// The NFL season's year, if one is under way or just finished.
    pub fn nfl_season_year(&self) -> Option<i32> {
        self.nfl_season.as_ref().map(|s| s.year)
    }

    /// The college season's year, if one is under way or just finished.
    pub fn college_year(&self) -> Option<i32> {
        self.college.as_ref().map(|cs| cs.year)
    }

    /// This NFL season's record (wins, losses, ties) for a club.
    pub fn nfl_record(&self, t: usize) -> (u32, u32, u32) {
        self.nfl_season.as_ref().map_or((0, 0, 0), |s| {
            (
                s.wins.get(t).copied().unwrap_or(0),
                s.losses.get(t).copied().unwrap_or(0),
                s.ties.get(t).copied().unwrap_or(0),
            )
        })
    }

    /// Conference (0 AFC, 1 NFC) and division (0-7) of each club this season.
    pub fn nfl_alignment(&self) -> Option<(Vec<usize>, Vec<usize>)> {
        self.nfl_season.as_ref().map(|s| (s.conf.clone(), s.div.clone()))
    }

    /// This NFL season's games for a club (all if None).
    pub fn nfl_fixtures(&self, t: Option<usize>) -> Vec<NflFixture> {
        self.nfl_fixtures_in(t, false)
    }

    /// This NFL season's games, or last season's (`prev`).
    pub fn nfl_fixtures_in(&self, t: Option<usize>, prev: bool) -> Vec<NflFixture> {
        let s = if prev { self.nfl_prev.as_ref() } else { self.nfl_season.as_ref() };
        let Some(s) = s else { return Vec::new() };
        let mut out = Vec::new();
        for (k, week) in s.weeks.iter().enumerate() {
            for (g, &(h, a)) in week.iter().enumerate() {
                if t.is_some_and(|t| h != t && a != t) {
                    continue;
                }
                let slot = s.slots.get(k).and_then(|w| w.get(g));
                out.push(NflFixture {
                    week: k + 1,
                    home: h,
                    away: a,
                    when: slot.map(|x| x.at.date),
                    minute: slot.map(|x| x.at.minute),
                    played: slot.is_some_and(|x| x.played),
                    score: slot.and_then(|x| x.score),
                    line: slot.and_then(|x| x.line),
                    attendance: slot.map_or(0, |x| x.attendance),
                });
            }
        }
        out
    }

    /// Your team's games kept whole, oldest first.
    pub fn seat_games(&self) -> &[SeatGame] {
        &self.seat_games
    }

    /// Your drawings on one of your games' film, replaced whole.
    pub fn set_film_marks(&mut self, game: usize, marks: Vec<FilmMark>) -> bool {
        match self.seat_games.get_mut(game) {
            Some(g) => {
                g.marks = marks;
                g.marks.truncate(MARKS_PER_GAME);
                true
            }
            None => false,
        }
    }
}

/// You, as the setup builds you (section 8, topic 2; people topic 15).
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct You {
    pub name: String,
    pub age: u32,
    /// Your date of birth (A1), from the setup page.
    #[serde(default = "crate::calendar::born::unknown")]
    pub born: crate::calendar::Date,
    pub ethnicity: super::ethnicity::Ethnicity,
    pub home_state: usize,
    pub ex_pro: bool,
    pub reputation: f64,
    /// Your ratings, 1-20, on the same scale as every coach.
    pub teaching: [f64; super::coach::GROUP_COUNT],
    pub scheme: f64,
    pub game_management: f64,
    pub motivation: f64,
    pub discipline: f64,
    pub recruiting: f64,
    pub network: f64,
    pub relationships: f64,
    pub style: crate::tendencies::Style,
    /// Your road into coaching, in words ("Scout or personnel man"): what
    /// your coach page says you came up as.
    #[serde(default)]
    pub road: String,
    /// Play calling, in-game adaptation, evaluation and negotiating.
    #[serde(default)]
    pub yours: super::coach::Yours,
    /// Who you are away from the ratings (people topic 15, first-playable
    /// topic 2): where you're from, your school, family and looks.
    #[serde(default)]
    pub profile: Profile,
    /// Aura (final build F15): a hidden 1-20 presence, read by the world in
    /// interviews, with recruits and in the room. 0 in a file from before it:
    /// read from your road and your people ratings.
    #[serde(default)]
    pub aura: f64,
    /// The position you played or the group you came up coaching (coaching
    /// C5): the group your road's strength went to. None on a road with
    /// neither, and in a file from before it: the group you teach best.
    #[serde(default)]
    pub came_up: Option<super::coach::Group>,
    /// Your name (C10), 1-20: the road's agreed value. `reputation` is
    /// your standing. 0 in a file from before the split.
    #[serde(default)]
    pub renown: f64,
    /// How good a player you were (C11, `former::Played::grade`) on a
    /// playing road; None if you never played in college.
    #[serde(default)]
    pub played: Option<u8>,
}

impl You {
    /// Your Aura, hidden from your own screens like every person's.
    pub fn aura(&self) -> f64 {
        if self.aura > 0.0 {
            return self.aura;
        }
        let seed = self.name.bytes().fold(0x811C_9DC5u32, |h, b| (h ^ b as u32).wrapping_mul(0x0100_0193));
        super::front_office::market::your_aura(&self.road, self.relationships, self.motivation, seed)
    }
}

/// Your profile (people topics 13 and 15). Kept and shown on your page; the
/// world reads none of it yet beyond your home state.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Profile {
    /// Date of birth: year, month, day.
    pub born: Option<(i32, u32, u32)>,
    pub hometown: String,
    pub high_school: String,
    /// Where you went to college, played or not.
    pub alma_mater: String,
    pub college_years: String,
    pub degree: String,
    /// Married, single, divorced or widowed.
    pub status: String,
    pub spouse: String,
    pub roots: String,
    /// Each child's name and year of birth.
    pub children: Vec<(String, i32)>,
    pub height: String,
    pub build: String,
    pub hair: String,
    pub facial_hair: String,
    /// Your own photo as a JPEG data URL, small (about 320 by 400).
    pub photo: Option<String>,
}

/// Where you start (section 8, topic 2).
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub enum Job {
    HighSchool(usize),
    College(usize),
    Nfl(usize),
    /// Out of work at the start: you look for a job through the market.
    Unemployed,
    /// A club in another pro league (later extras topic 5), by franchise.
    Other(usize),
}

/// How you start (first-playable topic 2): a head job, an assistant's job
/// under an AI head coach, or no job at all at the level you'd work at.
#[derive(Clone, Copy, Debug)]
pub enum Start {
    Head(Job),
    Assistant(Job, super::coach::Role),
    Free(crate::calendar::Level),
}

impl World {
    /// Starts your career on this world (section 8, topics 1 and 2): you
    /// take the job, the clock stops for your team's moments, and the world
    /// gets fresh randomness from today, so two careers from one library
    /// world share its history and then play out differently. The career's
    /// seed makes it replay exactly. Returns your coach id.
    pub fn start_career(&mut self, you: &You, job: Job, career_seed: u32) -> usize {
        self.start_career_as(you, Start::Head(job), career_seed)
    }

    /// Starts your career in any seat (first-playable topic 2): a head job,
    /// an assistant's job (the coach who held it moves into the market) or
    /// none, with the people from the world's history who know you.
    pub fn start_career_as(&mut self, you: &You, start: Start, career_seed: u32) -> usize {
        self.start_with(you, start, career_seed, Vec::new())
    }

    /// The same, with the people who know you already found (the setup's
    /// preview reads them before the career starts).
    pub fn start_with(
        &mut self,
        you: &You,
        start: Start,
        career_seed: u32,
        known: Vec<super::front_office::market::Known>,
    ) -> usize {
        use super::coach::{Employer, Role};
        use crate::calendar::Level;
        // Fresh randomness: the world's own stream, the seed every keyed
        // draw reads, and every person's stream, each mixed with the career.
        self.options.seed ^= career_seed.rotate_left(13) ^ 0x5EA7_0001;
        self.rng = crate::rng::Rng::new(self.options.seed ^ 0xC0FF_EE11);
        let seed = self.options.seed;
        self.people.iter_mut().for_each(|p| p.rng = crate::rng::Rng::new(seed ^ p.player.id.wrapping_mul(0x9E37_79B9)));
        let (job, role) = match start {
            Start::Head(j) => (j, Role::HeadCoach),
            Start::Assistant(j, r) => (j, r),
            Start::Free(_) => (Job::Unemployed, Role::HeadCoach),
        };
        let employer = match job {
            Job::HighSchool(s) => Some(Employer::HighSchool(s)),
            Job::College(p) => Some(Employer::College(p)),
            Job::Nfl(t) => Some(Employer::Nfl(t)),
            Job::Other(f) => Some(Employer::Other(f)),
            Job::Unemployed => None,
        };
        let id = super::staff::take_seat_as(self, you, employer, role);
        self.seat = Some(match job {
            Job::HighSchool(s) => {
                super::Seat { level: Level::HighSchool, nfl_team: None, program: Some(s), other: None }
            }
            Job::College(p) => super::Seat { level: Level::College, nfl_team: None, program: Some(p), other: None },
            Job::Nfl(t) => super::Seat { level: Level::Nfl, nfl_team: Some(t), program: None, other: None },
            Job::Other(f) => super::Seat { level: Level::Other, nfl_team: None, program: None, other: Some(f) },
            Job::Unemployed => {
                let level = if let Start::Free(l) = start { l } else { Level::College };
                super::Seat { level, nfl_team: None, program: None, other: None }
            }
        });
        let mut you = you.clone();
        if you.aura <= 0.0 {
            you.aura = you.aura();
        }
        let today = self.today;
        self.career = Some(Career {
            you,
            coach: id,
            job,
            started: today,
            seed: career_seed,
            notes: Vec::new(),
            hs_staff: Vec::new(),
            desk: Default::default(),
            role: (role != Role::HeadCoach && employer.is_some()).then_some(role),
            market: Default::default(),
            known,
            past: None,
            family: Default::default(),
            staff_book: Default::default(),
            kept_reads: Vec::new(),
            kept_through: 0,
        });
        let home = employer.and_then(|e| super::front_office::market::state_of(self, e));
        if let Some(c) = self.career.as_mut() {
            c.family.state = home.or(Some(c.you.home_state));
            // A job in another state moved the family with it this winter;
            // in your home state they were already settled there.
            let moved = c.family.state != Some(c.you.home_state);
            c.family.since = if moved { self.year } else { self.year - 1 };
            c.family.mood = if moved { 0.5 } else { 0.62 };
            if employer.is_none() {
                c.market.out_since = Some(today);
            }
        }
        if let (Job::HighSchool(h), Role::HeadCoach) = (job, role) {
            let staff = self.hire_hs_staff(h);
            if let Some(c) = self.career.as_mut() {
                c.hs_staff = staff;
            }
            super::front_office::school::open(self, h);
        }
        // Your school's money desk is open from your first morning.
        super::finance::hs::open_seat(self);
        self.seat_depth.clear();
        self.seat_games.clear();
        self.seat_focus.clear();
        self.prune_tapes();
        // Your first two weeks: the meetings of a new job (takeover).
        super::folk::takeover::begin(self);
        id
    }
}

/// Your career (section 8): who you are, your coach in the world, where you
/// started and your own notes on people (screens topic 5: private, they
/// change nothing in the world and follow the person through his career).
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Career {
    pub you: You,
    pub coach: usize,
    pub job: Job,
    pub started: Date,
    pub seed: u32,
    /// Notes by player id, oldest first.
    pub notes: Vec<(u32, Vec<Note>)>,
    /// Your high school staff, when you coach a high school.
    #[serde(default)]
    pub hs_staff: Vec<HsAssistant>,
    /// Your desk: mail, notes on anything, "my players", watches (the
    /// final build's inbox and screens, `desk.rs`).
    #[serde(default)]
    pub desk: super::desk::Desk,
    /// Your role when you are an assistant (None: head coach, or out of work).
    #[serde(default)]
    pub role: Option<super::coach::Role>,
    /// Your side of the coaching market.
    #[serde(default)]
    pub market: super::front_office::market::Market,
    /// The people from the world's history who know you.
    #[serde(default)]
    pub known: Vec<super::front_office::market::Known>,
    /// Where you were in the world's history (your high school, college,
    /// stops): your old teammates and players come from it (C13).
    #[serde(default)]
    pub past: Option<super::front_office::market::Placement>,
    /// Your family's life where you live.
    #[serde(default)]
    pub family: super::front_office::market::FamilyLife,
    /// What each season showed of your staff's work (update 2, Q3).
    #[serde(default)]
    pub staff_book: super::staff_read::Book,
    /// Your staff's read of each player it follows, once a season (update
    /// 2, lane 9A: the career arc). Kept from the day your staff follows
    /// him; never rebuilt for the seasons before.
    #[serde(default)]
    pub kept_reads: Vec<KeptReads>,
    /// The last season whose reads were kept (0: none yet; a save from
    /// before the kept reads starts keeping at the next season's end).
    #[serde(default)]
    pub kept_through: i16,
}

/// One player's kept reads, oldest season first.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct KeptReads {
    pub id: u32,
    pub rows: Vec<KeptRead>,
}

/// Your staff's read of a player as one season ended: his overall and the
/// four ratings that matter most at his position, each with the rating's
/// `Attr::index`, low and high on 1-99 (0 and 0 when nobody saw it); the
/// middle of each of his position's eight skill families (0: not seen);
/// his weight then in pounds. About 30 bytes a player a season.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct KeptRead {
    pub season: i16,
    /// His position then (`Position::index`).
    pub pos: u8,
    pub ovr: [u8; 2],
    pub keys: [[u8; 3]; 4],
    pub fams: [u8; 8],
    pub wt: u16,
}

impl Career {
    /// The kept reads of one player, oldest first.
    pub fn kept_of(&self, id: u32) -> &[KeptRead] {
        self.kept_reads.iter().find(|k| k.id == id).map_or(&[], |k| k.rows.as_slice())
    }

    /// Keeps a season's read; a second read of the same season replaces
    /// the first.
    pub fn keep_read(&mut self, id: u32, row: KeptRead) {
        let at = match self.kept_reads.iter().position(|k| k.id == id) {
            Some(i) => i,
            None => {
                self.kept_reads.push(KeptReads { id, rows: Vec::new() });
                self.kept_reads.len() - 1
            }
        };
        let rows = &mut self.kept_reads[at].rows;
        rows.retain(|r| r.season != row.season);
        rows.push(row);
        rows.sort_by_key(|r| r.season);
    }
}

/// A high school assistant (people topic 10): most teach at the school and
/// coach for a stipend. Small schools have two to four, big Texas programs
/// fifteen or more across varsity, JV and freshmen.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct HsAssistant {
    pub name: String,
    pub age: u32,
    /// His date of birth (A1): the age the game shows counts from it.
    #[serde(default = "crate::calendar::born::unknown")]
    pub born: crate::calendar::Date,
    /// Offensive or defensive coordinator, or a position group's coach.
    pub coordinator: Option<bool>,
    /// The group he coaches (`coach::Group` as an index).
    pub group: u8,
    /// What he teaches in class, or None for a walk-on coach.
    pub subject: Option<String>,
    /// Coaching stipend a season, in dollars.
    pub stipend: u32,
    /// Hidden teaching ability, 1-20 on every coach's scale.
    pub teaching: f64,
    pub since: i32,
}

impl You {
    /// Your age on a day, from your date of birth (A1).
    pub fn age_on(&self, d: crate::calendar::Date) -> u32 {
        crate::calendar::born::age_or(self.born, d, self.age)
    }
}

impl HsAssistant {
    /// His age on a day, from his date of birth (A1).
    pub fn age_on(&self, d: crate::calendar::Date) -> u32 {
        crate::calendar::born::age_or(self.born, d, self.age)
    }
}

/// The key a high school assistant's birthday is drawn from: his name and
/// the year he was hired (keyed, no stream consumed).
pub(crate) fn assistant_born_key(name: &str, year: i32) -> u64 {
    name.bytes().fold(0xA551_57A0_u64 ^ year as u64, |k, b| k.wrapping_mul(0x100_0000_01B3) ^ b as u64)
}

/// Subjects high school coaches teach (proposed, not confirmed: weighted
/// toward physical education and history, as real staffs are).
const SUBJECTS: [&str; 10] = [
    "physical education",
    "physical education",
    "history",
    "history",
    "health",
    "math",
    "science",
    "English",
    "special education",
    "driver's education",
];

impl World {
    /// Your high school staff from the school's size and money (proposed,
    /// not confirmed: two plus one per 400 students, 3 to 15; ability
    /// around what the school's resources buy, the stand-in AI schools use).
    pub(crate) fn hire_hs_staff(&mut self, h: usize) -> Vec<HsAssistant> {
        let school = &self.high_schools[h];
        let n = (2 + school.enrollment / 400).clamp(3, 15) as usize;
        let mean = 4.0 + 12.0 * school.resources;
        let rich = school.resources;
        let year = self.year;
        let mut rng = crate::rng::Rng::new(self.options.seed ^ 0x45AF_F000 ^ h as u32);
        // Coordinators first, then the groups a staff fills in order.
        let order: [(Option<bool>, u8); 15] = [
            (Some(true), 0),
            (Some(false), 6),
            (None, 3),
            (None, 4),
            (None, 6),
            (None, 5),
            (None, 2),
            (None, 1),
            (None, 0),
            (None, 7),
            (None, 3),
            (None, 4),
            (None, 6),
            (None, 2),
            (None, 5),
        ];
        order[..n]
            .iter()
            .map(|&(coordinator, group)| {
                let eth = super::ethnicity::coach_ethnicity(super::ethnicity::CoachPool::HighSchool, year, &mut rng);
                let age = 24 + (rng.next() * 34.0) as u32;
                let name = super::names::person_name(eth, year - age as i32, &mut rng);
                // Some states allow walk-on coaches who don't teach.
                let subject = (rng.next() > 0.15).then(|| SUBJECTS[(rng.next() * 10.0) as usize % 10].to_string());
                let lead = if coordinator.is_some() { 1.0 } else { 0.0 };
                HsAssistant {
                    born: crate::calendar::born::birth_date(assistant_born_key(&name, year), age, year),
                    name,
                    age,
                    coordinator,
                    group,
                    subject,
                    stipend: (2500.0 + 5500.0 * rich + 1500.0 * lead + rng.normal(0.0, 800.0)).max(1000.0) as u32,
                    teaching: crate::rng::clamp(mean + lead + rng.normal(0.0, 2.5), 1.0, 20.0),
                    since: year - (rng.next() * 8.0) as i32,
                }
            })
            .collect()
    }

    /// What your high school staff teaches a position group: its best man
    /// for the group, coordinators covering their side (people topic 10;
    /// proposed, not confirmed).
    pub(crate) fn hs_staff_teaching(&self, h: usize, group: super::coach::Group) -> Option<f64> {
        if self.seat_school() != Some(h) {
            return None;
        }
        let staff = &self.career.as_ref()?.hs_staff;
        let g = group as u8;
        let offense = matches!(g, 0..=3);
        staff
            .iter()
            .filter(|a| a.group == g || a.coordinator == Some(offense) && g != 7)
            .map(|a| if a.group == g { a.teaching } else { a.teaching - 1.5 })
            .max_by(f64::total_cmp)
    }
}

/// One of your notes: the game date it was written and its text.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Note {
    pub date: Date,
    pub text: String,
    /// The page it was written on ("Player page · Jalen Carter", "Depth
    /// chart"); empty for notes from before the final build.
    #[serde(default)]
    pub page: String,
}

impl World {
    pub fn add_note(&mut self, player: u32, text: String) {
        self.add_note_from(player, text, String::new());
    }

    /// A note on a player, with the page it was written on.
    pub fn add_note_from(&mut self, player: u32, text: String, page: String) {
        let date = self.today;
        let Some(c) = self.career.as_mut() else { return };
        let note = Note { date, text, page };
        match c.notes.iter_mut().find(|(id, _)| *id == player) {
            Some((_, list)) => list.push(note),
            None => c.notes.push((player, vec![note])),
        }
    }

    pub fn delete_note(&mut self, player: u32, k: usize) {
        let Some(c) = self.career.as_mut() else { return };
        if let Some((_, list)) = c.notes.iter_mut().find(|(id, _)| *id == player)
            && k < list.len()
        {
            list.remove(k);
        }
    }

    pub fn notes_on(&self, player: u32) -> &[Note] {
        self.career
            .as_ref()
            .and_then(|c| c.notes.iter().find(|(id, _)| *id == player))
            .map_or(&[], |(_, l)| l.as_slice())
    }
}

impl World {
    /// Your own ratings: you see them (people topic 15); nobody else's.
    pub fn your_ability(&self) -> Option<&super::coach::Ability> {
        let c = self.career.as_ref()?;
        self.coaches.get(c.coach).map(|co| co.ability())
    }

    /// Your play calling, adaptation, evaluation and negotiating as they are
    /// now (they grow like your other ratings).
    pub fn your_extra(&self) -> Option<super::coach::Yours> {
        let c = self.career.as_ref()?;
        self.coaches.get(c.coach).and_then(|co| co.yours)
    }
}

impl World {
    /// Fills in, after a file loads, what the file's game didn't keep
    /// (section 8, the world importer): a world built before section 8 has
    /// no seat, no career and no kept games, which load as empty, and its
    /// people list gets the same room rule as a running world.
    pub fn after_load(&mut self) {
        let room = self.people.len() + self.people.len() / 3;
        self.people.shrink_to(room);
        self.seat_games.retain(|g| g.teams.iter().all(|&t| t < self.programs.len().max(self.nfl.len())));
        super::scouting::after_load(self);
        self.fill_final_build_fields();
        // Faces (F3, F4): looks numbers and the Asian groups for an older file.
        self.faces_after_load();
        // Your designed plays become concepts again (section 6b).
        self.training.register_designs();
        super::folk::after_load(self);
        super::storylines::after_load(self);
        // A world file from before the poll and award voters gets them now.
        super::voters::ensure(self);
        // Long snappers made before the final build at a lineman's weight
        // play at a snapper's (the 307-pound snapper).
        for p in self.people.iter_mut().filter(|p| p.player.position == crate::positions::Position::LS) {
            p.player.body.weight =
                p.player.body.weight.clamp(crate::athlete::LS_MIN_WEIGHT, crate::athlete::LS_MAX_WEIGHT);
        }
        // A career from before the lane-10 build: its school's people and
        // books are drawn now, its family settled where it is. The people
        // you answer to are the persons the meeting room knows.
        if let Some(h) = self.seat_school()
            && self.seat_head()
        {
            if self.front_office.school.school != Some(h) {
                super::front_office::school::open(self, h);
            } else {
                super::front_office::school::sync_people(self, h);
            }
        }
        let home = super::front_office::market::your_job(self)
            .and_then(|j| super::front_office::market::state_of(self, j.employer));
        let year = self.year;
        if let Some(c) = self.career.as_mut()
            && c.family.state.is_none()
        {
            c.family.state = home.or(Some(c.you.home_state));
            c.family.since = year - 1;
            c.family.mood = 0.62;
        }
        super::finance::after_load(self);
    }

    /// What a world file from before the final build lacks (lane 1): birth
    /// dates (A1), Aura (F15), followers and brand money. Every value is
    /// keyed by the world's seed and the person's id, so the same file
    /// always fills the same way. Old files get no held-back boys (A2): a
    /// birthday inside each person's own class keeps every grade as it was.
    pub(crate) fn fill_final_build_fields(&mut self) {
        use crate::calendar::born;
        let seed = self.options.seed;
        let year = self.year;
        let mut fresh = false;
        for p in self.people.iter_mut() {
            if p.born == born::UNKNOWN {
                p.born = born::for_class(super::born_key(seed, p.player.id), p.class_of);
            }
            let t = &mut p.player.personality.traits;
            if t.aura == 0 {
                t.aura = crate::player::Traits::draw_aura(
                    super::born_key(seed, p.player.id) ^ 0xA07A,
                    t.ego.max(1),
                    t.leadership.max(1),
                );
                fresh = true;
            }
        }
        for a in self.alumni.iter_mut().filter(|a| a.born == born::UNKNOWN) {
            let key = super::born_key(seed, a.year as u32) ^ a.name.len() as u64;
            a.born = born::for_class(key, a.class_of);
        }
        for c in self.coaches.iter_mut().filter(|c| c.born == born::UNKNOWN) {
            c.born = born::birth_date(super::staff::coach_born_key(seed, c.id), c.age, year);
        }
        if let Some(career) = self.career.as_mut() {
            let you = &mut career.you;
            if you.born == born::UNKNOWN {
                you.born = born::birth_date(seed as u64 ^ 0x70_0B0E, you.age, year);
            }
            for a in career.hs_staff.iter_mut().filter(|a| a.born == born::UNKNOWN) {
                a.born = born::birth_date(assistant_born_key(&a.name, a.since), a.age, a.since.max(year - 40));
            }
        }
        // Followers and brand money as the year's book would have left them.
        if fresh {
            super::fame::fill(self);
        }
    }
}
