//! The catalogue: every kind of talking point a talk can stand on (talk
//! facts design, sections 5, 6 and 9). One row a kind: its key (the
//! `<point>` in a line bank part such as `talk.point.open.lost_job.sulks`
//! or `recruit.pitch.stadium.elite`), whose fact it is, when the other
//! person can check it, how long it stays fresh and how big it is.
//!
//! Every size here is Claude's (proposed, not confirmed): the windows are
//! section 13's, the sizes of "how big it is" are picks.

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// A game is named for a week (rule 9: a talk is about today; build item
/// 30).
/// Proposed, not confirmed.
pub const GAME_DAYS: i32 = 7;
/// A return from injury, a parent's call, a week of practice, report day.
/// Proposed, not confirmed.
pub const FORTNIGHT_DAYS: i32 = 14;
/// A testing day, a college's move on a high school player.
/// Proposed, not confirmed.
pub const TEST_DAYS: i32 = 21;
/// A job lost or won, a move in his place (his practice group, his
/// starts), a friend who left.
/// Proposed, not confirmed.
pub const JOB_DAYS: i32 = 28;
/// A closed case, a ruling, an award, the season that ended, the room
/// changing, a teammate drafted, a failed term.
/// Proposed, not confirmed.
pub const NEWS_DAYS: i32 = 30;
/// A promise coming due, a new position coach.
/// Proposed, not confirmed.
pub const AHEAD_DAYS: i32 = 60;
/// A broken promise.
/// Proposed, not confirmed.
pub const BROKEN_DAYS: i32 = 120;

/// Whose fact it is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Whose {
    /// About the person across from you.
    His,
    /// About you, your staff or your program.
    Yours,
    /// Both sides lived it (a game, a season, a promise).
    Shared,
}

/// When the other person can check a claim about it (section 3; set once
/// per kind). A chip only states what is true, so chips never read it; a
/// typed line does (section 14).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum When {
    /// He can check it as you say it: a draft pick, a title, the miles,
    /// Saturday's game.
    Now,
    /// He can check it on a day: his place over his first two practice
    /// weeks and two games, the money at signing, his role on Saturday (the
    /// point's `due`).
    Day,
    /// He can never check it: what you told another recruit.
    Never,
}

/// The seven groups a recruit's points fall in (section 6): the six wants
/// and the people. A case counts each group at its best point.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Group {
    PlayingTime,
    Money,
    School,
    Home,
    People,
    League,
    Winning,
}

impl Group {
    pub const ALL: [Group; 7] =
        [Group::PlayingTime, Group::Money, Group::School, Group::Home, Group::People, Group::League, Group::Winning];
    pub fn key(self) -> &'static str {
        match self {
            Group::PlayingTime => "playing_time",
            Group::Money => "money",
            Group::School => "school",
            Group::Home => "home",
            Group::People => "people",
            Group::League => "league",
            Group::Winning => "winning",
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Group::PlayingTime => "Playing time",
            Group::Money => "Money",
            Group::School => "The school",
            Group::Home => "Close to home",
            Group::People => "The people",
            Group::League => "The league",
            Group::Winning => "Winning and the name",
        }
    }
    pub fn from_key(k: &str) -> Option<Group> {
        Group::ALL.into_iter().find(|g| g.key() == k)
    }
}

/// Where a kind belongs: which sheet lists it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Family {
    /// A player on your team, in season (section 5, "Topics on the table").
    Player,
    /// A player on your team, between seasons (section 5, "Out of season").
    Offseason,
    /// A point you can make to a recruit or a transfer (section 6, "The
    /// points").
    Pitch(Group),
    /// Something that changed since you last spoke: what a recruit opens
    /// on (section 6, "How a call flows" and "Out of season").
    Event,
    /// What is coming for him when nothing fresh has happened (rule 9).
    Next,
    /// Your team's own week and season (section 9: the team talk, the
    /// press, your boss).
    Team,
}

macro_rules! catalogue {
    ($( $(#[$doc:meta])* $name:ident $key:literal $family:expr, $whose:ident $when:ident $window:expr, $size:literal; )*) => {
        /// A kind of talking point.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub enum Kind {
            $( $(#[$doc])* $name, )*
        }

        impl Kind {
            /// Every kind, in the catalogue's order.
            pub const ALL: &'static [Kind] = &[ $( Kind::$name, )* ];

            /// Its key: the `<point>` of a line bank part, and what a
            /// pick carries in `Pick.point`.
            pub fn key(self) -> &'static str {
                match self { $( Kind::$name => $key, )* }
            }

            pub fn family(self) -> Family {
                match self { $( Kind::$name => $family, )* }
            }

            /// Whose fact it is.
            pub fn whose(self) -> Whose {
                match self { $( Kind::$name => Whose::$whose, )* }
            }

            /// When the other person can check a claim about it.
            pub fn when(self) -> When {
                match self { $( Kind::$name => When::$when, )* }
            }

            /// How many days it stays fresh; 0 for a point that lasts as
            /// long as it is true (an open case, an injury, his pay).
            pub fn window(self) -> i32 {
                match self { $( Kind::$name => $window, )* }
            }

            /// How big it is in a life, 0-1, before how much he cares and
            /// how fresh it is.
            pub fn size(self) -> f32 {
                match self { $( Kind::$name => $size, )* }
            }
        }
    };
}

use Family::{Event, Next, Offseason, Pitch, Player, Team};

catalogue! {
    // ----- A player on your team (section 5).
    /// A case is open: arrested, charged or in court.
    Arrest "arrest" Player, His Now 0, 1.0;
    /// A case closed in the last month.
    CaseClosed "case_closed" Player, His Now NEWS_DAYS, 0.6;
    /// A suspension is running.
    Suspended "suspended" Player, His Now 0, 0.8;
    /// A team-rule break you ruled on in the last month, or one waiting
    /// on your ruling.
    RuleBreak "rule_break" Player, Shared Now NEWS_DAYS, 0.5;
    /// Benched in the last four weeks.
    LostJob "lost_job" Player, His Now JOB_DAYS, 0.85;
    /// Third or lower on the chart after two seasons.
    Buried "buried" Player, His Now 0, 0.6;
    /// A new starter in the last four weeks.
    WonJob "won_job" Player, His Now JOB_DAYS, 0.7;
    /// His last game was among the best at his spot.
    BigGame "big_game" Player, Shared Now GAME_DAYS, 0.5;
    /// A costly mistake or a poor grade in his last game.
    BadGame "bad_game" Player, Shared Now GAME_DAYS, 0.55;
    /// Three or more games under his level.
    Slump "slump" Player, His Now GAME_DAYS, 0.6;
    /// An award in the last month.
    Award "award" Player, His Now NEWS_DAYS, 0.6;
    /// Out hurt now.
    Hurt "hurt" Player, His Now 0, 0.75;
    /// Back from an injury in the last two weeks.
    Back "back" Player, His Now FORTNIGHT_DAYS, 0.45;
    /// Life: a child was born.
    Birth "birth" Player, His Now 0, 0.6;
    /// Life: he got married.
    Wedding "wedding" Player, His Now 0, 0.5;
    /// Life: a parent is sick.
    SickParent "sick_parent" Player, His Now 0, 0.9;
    /// Life: a death in the family.
    Death "death" Player, His Now 0, 1.0;
    /// Life: a divorce.
    Divorce "divorce" Player, His Now 0, 0.8;
    /// Life: money lost.
    MoneyLost "money_lost" Player, His Now 0, 0.7;
    /// Life: homesick.
    Homesick "homesick" Player, His Now 0, 0.6;
    /// A promise of yours that comes due within two months.
    PromiseOpen "promise_open" Player, Shared Day AHEAD_DAYS, 0.5;
    /// A promise of yours that broke in the last four months.
    PromiseBroken "promise_broken" Player, Shared Now BROKEN_DAYS, 0.8;
    /// Paid under the going rate for his spot and role, or a pro who
    /// outplayed his deal.
    Pay "pay" Player, His Now 0, 0.6;
    /// A pro's holdout or trade wish.
    WantsOut "wants_out" Player, His Now 0, 0.9;
    /// He leans toward the portal, or asked to see you.
    Portal "portal" Player, His Now 0, 0.8;
    /// He can declare for the draft and has a read.
    Draft "draft" Player, His Now 0, 0.7;
    /// A close friend left the team.
    FriendLeft "friend_left" Player, His Now JOB_DAYS, 0.6;
    /// His position coach changed in the last two months.
    NewCoach "new_coach" Player, Shared Now AHEAD_DAYS, 0.4;
    /// His grades are under or near the eligibility line.
    Grades "grades" Player, His Now 0, 0.6;
    /// His parent called you in the last two weeks.
    ParentCalled "parent_called" Player, Shared Now FORTNIGHT_DAYS, 0.4;
    /// Your staff flagged his effort.
    Effort "effort" Player, His Now FORTNIGHT_DAYS, 0.4;

    // ----- A player on your team, out of season (section 5).
    /// The season that ended in the last month.
    SeasonEnded "season_ended" Offseason, Shared Now NEWS_DAYS, 0.5;
    /// Your strength staff tested him in the last three weeks.
    TestingDay "testing" Offseason, His Now TEST_DAYS, 0.35;
    /// His week of practice: his one-on-ones, his grade.
    PracticeWeek "practice_week" Offseason, His Now FORTNIGHT_DAYS, 0.4;
    /// Camp opened in the last two weeks: in shape or not.
    ReportDay "report_day" Offseason, His Now FORTNIGHT_DAYS, 0.4;
    /// His coaches tipped him for a step this spring, and whether it came.
    SpringStep "spring_step" Offseason, His Now 0, 0.5;
    /// His place moved in the last four weeks, as he saw it: up to the
    /// ones or down from them in practice, a start won or lost.
    ChartMove "chart_move" Offseason, Shared Now JOB_DAYS, 0.6;
    /// He is learning a new position.
    NewPosition "new_position" Offseason, His Now 0, 0.55;
    /// Someone arrived at his spot, or a man ahead of him left, in the
    /// last month.
    RoomChanged "room_changed" Offseason, Shared Now NEWS_DAYS, 0.55;
    /// A new system installed this spring.
    NewSystem "new_system" Offseason, Shared Now 0, 0.35;
    /// A teammate drafted in the last month.
    TeammateDrafted "teammate_drafted" Offseason, Shared Now NEWS_DAYS, 0.4;
    /// He failed a term at the last check.
    FailedTerm "failed_term" Offseason, His Now NEWS_DAYS, 0.7;
    /// His other sport, in its season (high school).
    OtherSport "other_sport" Offseason, His Now NEWS_DAYS, 0.3;
    /// A college moved on him in the last three weeks (your high school
    /// player).
    Colleges "colleges" Offseason, His Now TEST_DAYS, 0.6;
    /// A pro's spring: a tag, extension talks, a skipped voluntary week,
    /// a camp battle.
    ProSpring "pro_spring" Offseason, His Now 0, 0.6;

    // ----- Your points to a recruit or a transfer (section 6).
    /// The room at his spot, by class.
    Room "room" Pitch(Group::PlayingTime), Yours Now 0, 0.5;
    /// The role your staff plans for him.
    Role "role" Pitch(Group::PlayingTime), Yours Day 0, 0.5;
    /// A quarterback's rivals in the room.
    QbRivals "qb_rivals" Pitch(Group::PlayingTime), Yours Now 0, 0.5;
    /// The scheme and his position in it.
    Scheme "scheme" Pitch(Group::PlayingTime), Yours Now 0, 0.5;
    /// Your offer against the going rate at his spot.
    OfferRate "offer_rate" Pitch(Group::Money), Yours Day 0, 0.5;
    /// Your offer against his other offers.
    OfferOthers "offer_others" Pitch(Group::Money), Yours Day 0, 0.5;
    /// What the money does for a family in need.
    FamilyNeed "family_need" Pitch(Group::Money), Shared Day 0, 0.5;
    /// Brand deals for a kid with a name.
    Brand "brand" Pitch(Group::Money), Yours Now 0, 0.5;
    /// The degree.
    Degree "degree" Pitch(Group::School), Yours Now 0, 0.5;
    /// His own grades and your academic support.
    Support "support" Pitch(Group::School), Shared Now 0, 0.5;
    /// Life after football.
    AfterFootball "after_football" Pitch(Group::School), Yours Now 0, 0.5;
    /// The drive, and family at his games.
    Drive "drive" Pitch(Group::Home), Shared Now 0, 0.5;
    /// The team he grew up on.
    ChildhoodTeam "childhood_team" Pitch(Group::Home), Shared Now 0, 0.5;
    /// Staying in his state.
    HomeState "home_state" Pitch(Group::Home), Shared Now 0, 0.5;
    /// Players from his high school on your team.
    HsMates "hs_mates" Pitch(Group::Home), Shared Now 0, 0.5;
    /// A brother here.
    Brother "brother" Pitch(Group::Home), Shared Now 0, 0.5;
    /// Your years here and how safe your job is.
    Tenure "tenure" Pitch(Group::People), Yours Now 0, 0.5;
    /// The position coach he'd have and that man's record.
    PositionCoach "position_coach" Pitch(Group::People), Yours Now 0, 0.5;
    /// Your tie with his high school coach.
    HsCoachTie "hs_coach_tie" Pitch(Group::People), Shared Now 0, 0.5;
    /// Your name for keeping your word.
    YourWord "your_word" Pitch(Group::People), Yours Now 0, 0.5;
    /// Why a transfer left his old school.
    WhyLeft "why_left" Pitch(Group::People), His Now 0, 0.5;
    /// Draft picks at his position, by name.
    PicksAtPosition "picks_position" Pitch(Group::League), Yours Now 0, 0.5;
    /// Draft picks overall.
    PicksOverall "picks_all" Pitch(Group::League), Yours Now 0, 0.5;
    /// The players you coached yourself, anywhere.
    YourGuys "your_guys" Pitch(Group::League), Yours Now 0, 0.5;
    /// The position group's name ("DBU").
    PositionU "position_u" Pitch(Group::League), Yours Now 0, 0.5;
    /// Your pro day.
    ProDay "pro_day" Pitch(Group::League), Yours Now 0, 0.5;
    /// Titles and your last seasons.
    Titles "titles" Pitch(Group::Winning), Yours Now 0, 0.5;
    /// This year's title odds.
    TitleOdds "title_odds" Pitch(Group::Winning), Yours Now 0, 0.5;
    /// The stadium.
    Stadium "stadium" Pitch(Group::Winning), Yours Now 0, 0.5;
    /// Exposure.
    Exposure "exposure" Pitch(Group::Winning), Yours Now 0, 0.5;
    /// The conference.
    Conference "conference" Pitch(Group::Winning), Yours Now 0, 0.5;
    /// The rival.
    Rival "rival" Pitch(Group::Winning), Yours Now 0, 0.5;
    /// Buildings finished or under way.
    Buildings "buildings" Pitch(Group::Winning), Yours Now 0, 0.5;

    // ----- What changed since you last spoke (section 6).
    /// You won your last game.
    YourWin "your_win" Event, Yours Now GAME_DAYS, 0.5;
    /// You lost your last game.
    YourLoss "your_loss" Event, Yours Now GAME_DAYS, 0.5;
    /// His own last game.
    HisGame "his_game" Event, His Now GAME_DAYS, 0.5;
    /// A new offer from another school.
    NewOffer "new_offer" Event, His Now TEST_DAYS, 0.6;
    /// A visit he took elsewhere.
    VisitElsewhere "visit_elsewhere" Event, His Now TEST_DAYS, 0.5;
    /// His visit to you.
    YourVisit "your_visit" Event, Shared Now TEST_DAYS, 0.6;
    /// You took a commitment at his spot.
    CommitAtSpot "commit_at_spot" Event, Yours Now NEWS_DAYS, 0.7;
    /// A coach left your staff.
    CoachLeft "coach_left" Event, Yours Now AHEAD_DAYS, 0.7;
    /// Your draft day.
    YourDraft "your_draft" Event, Yours Now NEWS_DAYS, 0.5;
    /// He backed out of a commitment.
    Decommit "decommit" Event, His Now NEWS_DAYS, 0.7;
    /// A track time or the state meet.
    TrackTime "track_time" Event, His Now NEWS_DAYS, 0.4;
    /// A laser forty at a showcase or a camp.
    CampForty "camp_forty" Event, His Now NEWS_DAYS, 0.4;
    /// Your own camp, where your staff watched him.
    YourCamp "your_camp" Event, Shared Now NEWS_DAYS, 0.5;
    /// The services' ranking update and how far he moved.
    RankUpdate "rank_update" Event, His Now NEWS_DAYS, 0.5;
    /// The top schools he named.
    TopSchools "top_schools" Event, His Now NEWS_DAYS, 0.6;
    /// An insider's pick for him.
    InsiderPick "insider_pick" Event, His Now NEWS_DAYS, 0.4;
    /// All-state, or the January all-star game.
    AllState "all_state" Event, His Now AHEAD_DAYS, 0.5;
    /// His team's title run.
    TitleRun "title_run" Event, His Now AHEAD_DAYS, 0.5;
    /// His test score.
    TestScore "test_score" Event, His Now NEWS_DAYS, 0.4;
    /// How your season ended: the bowl, the record.
    YourBowl "your_bowl" Event, Yours Now NEWS_DAYS, 0.5;
    /// The final poll.
    FinalPoll "final_poll" Event, Yours Now NEWS_DAYS, 0.4;
    /// Signing day: who signed with you at his spot.
    SigneeAtSpot "signee_at_spot" Event, Yours Now NEWS_DAYS, 0.6;
    /// A transfer in at his spot.
    TransferIn "transfer_in" Event, Yours Now NEWS_DAYS, 0.6;
    /// Your spring: a new system, a player at his spot who broke out.
    YourSpring "your_spring" Event, Yours Now NEWS_DAYS, 0.4;
    /// Your pro day and how many clubs came.
    YourProDay "your_pro_day" Event, Yours Now NEWS_DAYS, 0.4;
    /// A coach you hired.
    CoachHired "coach_hired" Event, Yours Now AHEAD_DAYS, 0.5;
    /// A building announced or opened.
    Building "building" Event, Yours Now AHEAD_DAYS, 0.4;
    /// The August poll.
    AugustPoll "august_poll" Event, Yours Now NEWS_DAYS, 0.4;
    /// Next season's schedule: the opener, a game near his home.
    Schedule "schedule" Event, Yours Now 0, 0.3;
    /// Your job is in the news.
    YourJob "your_job" Event, Yours Now NEWS_DAYS, 0.7;
    /// A transfer: the day he entered the portal.
    EnteredPortal "entered_portal" Event, His Now NEWS_DAYS, 0.6;

    // ----- What is coming (rule 9).
    /// Official visits open.
    VisitsOpen "visits_open" Next, Shared Day 0, 0.3;
    /// A dead period ends.
    DeadEnds "dead_ends" Next, Shared Day 0, 0.3;
    /// His visit to you is set.
    HisVisit "his_visit" Next, Shared Day 0, 0.5;
    /// Your camp.
    CampDate "camp_date" Next, Yours Day 0, 0.3;
    /// His decision day.
    DecisionDay "decision_day" Next, His Day 0, 0.6;
    /// Signing day.
    SigningDay "signing_day" Next, Shared Day 0, 0.5;
    /// A player of yours: the next dated thing on his calendar (testing
    /// day, spring practice, report day, the opener).
    Ahead "ahead" Next, Shared Day 0, 0.2;

    // ----- Your team's week and season (section 9).
    /// The last game, while it is fresh.
    LastGame "last_game" Team, Shared Now GAME_DAYS, 0.6;
    /// A streak of three or more, winning or losing.
    Streak "streak" Team, Shared Now GAME_DAYS, 0.5;
    /// The next game.
    NextGame "next_game" Team, Shared Day 0, 0.4;
    /// Starters out hurt.
    StartersOut "starters_out" Team, Shared Now 0, 0.5;
}

impl Kind {
    pub fn from_key(k: &str) -> Option<Kind> {
        Kind::ALL.iter().copied().find(|x| x.key() == k)
    }

    /// The group of a recruit's point; none for every other kind.
    pub fn group(self) -> Option<Group> {
        match self.family() {
            Family::Pitch(g) => Some(g),
            _ => None,
        }
    }

    /// The points of one group, in the catalogue's order.
    pub fn of_group(g: Group) -> impl Iterator<Item = Kind> {
        Kind::ALL.iter().copied().filter(move |k| k.group() == Some(g))
    }

    /// One of the seven life events.
    pub fn is_life(self) -> bool {
        matches!(
            self,
            Kind::Birth
                | Kind::Wedding
                | Kind::SickParent
                | Kind::Death
                | Kind::Divorce
                | Kind::MoneyLost
                | Kind::Homesick
        )
    }

    /// Trouble: what a warning or a discipline talk can stand on.
    pub fn is_trouble(self) -> bool {
        matches!(self, Kind::Arrest | Kind::CaseClosed | Kind::Suspended | Kind::RuleBreak)
    }
}

/// Saved by its key, so the catalogue can grow or be reordered and old
/// saves still read (a key no longer in the catalogue fails the read of
/// that one row, which `Talked` drops).
impl Serialize for Kind {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(self.key())
    }
}

impl<'de> Deserialize<'de> for Kind {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Kind, D::Error> {
        let k = String::deserialize(d)?;
        Kind::from_key(&k).ok_or_else(|| serde::de::Error::custom(format!("unknown talking point: {k}")))
    }
}
