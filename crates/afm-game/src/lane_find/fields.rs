//! Every field the search can filter, sort and chart on (FS1-FS18, FR1-FR13),
//! each read from what your staff or the public knows.
//!
//! A field is a key (`attr:Speed`, `st:RushYds`, `svc:0`) and a value for
//! one player: a number, a word (kept as its place in an ordered list, so
//! "at least" works on words), a yes or no, a text, nothing (no number
//! known, as a kid with no stat line) or "not seen" (a read your staff
//! doesn't have, or one too wide for the search's "how sure" floor). The
//! catalog sent to the screen lists the fields by group, each with its kind
//! and words.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use afm_engine::attributes::Attr;
use afm_engine::calendar::{Date, YearDates};
use afm_engine::player::DevelopmentEvent as Ev;
use afm_engine::positions::Position;
use afm_engine::rng::Rng;
use afm_engine::tendencies::{self, Tend};
use afm_engine::world::front_office::college as desk;
use afm_engine::world::hs_year::{HsLine, HsStat};
use afm_engine::world::records::Stat;
use afm_engine::world::scouting::readers::{self, Staff};
use afm_engine::world::scouting::{Kind, file, services};
use afm_engine::world::{Person, Stage, Visit, World, geo, key_normal, key_uniform};
use serde_json::{Value, json};

use super::staff::Read;

/// A field's value for one player.
#[derive(Clone, Debug, PartialEq)]
pub enum Val {
    N(f64),
    T(String),
    /// Nothing known (no stat line, no test, not ranked).
    None,
    /// Your staff has no read (or none sure enough for the search).
    Unseen,
}

impl Val {
    pub fn num(&self) -> Option<f64> {
        if let Val::N(x) = self { Some(*x) } else { None }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FKind {
    Num,
    /// One of `opts`, worst to best: the value is its place.
    Word,
    /// Yes (1) or no (0).
    Flag,
    /// A name or code ("is" matches it, or any line of a list).
    Text,
}

impl FKind {
    fn key(self) -> &'static str {
        match self {
            FKind::Num => "num",
            FKind::Word => "word",
            FKind::Flag => "flag",
            FKind::Text => "text",
        }
    }
}

#[derive(Clone, Debug)]
pub struct Field {
    pub k: String,
    pub label: String,
    pub kind: FKind,
    pub opts: Vec<&'static str>,
    /// Lower is better (a 40 time, a rank).
    pub lower: bool,
    /// A read of your staff's ("not seen" applies).
    pub staff: bool,
    /// Decimals to show.
    pub dp: u8,
}

impl Field {
    fn new(k: impl Into<String>, label: impl Into<String>, kind: FKind) -> Field {
        Field { k: k.into(), label: label.into(), kind, opts: Vec::new(), lower: false, staff: false, dp: 0 }
    }
    fn lower(mut self) -> Field {
        self.lower = true;
        self
    }
    fn staff(mut self) -> Field {
        self.staff = true;
        self
    }
    fn dp(mut self, d: u8) -> Field {
        self.dp = d;
        self
    }
    fn opts(mut self, o: &[&'static str]) -> Field {
        self.opts = o.to_vec();
        self
    }
    pub fn json(&self) -> Value {
        json!({ "k": self.k, "label": self.label, "kind": self.kind.key(), "opts": self.opts, "lower": self.lower, "staff": self.staff, "dp": self.dp })
    }
}

/// A group of fields in the catalog, with the list item it answers.
pub struct Group {
    pub name: &'static str,
    pub id: &'static str,
    pub fields: Vec<Field>,
}

/// Whose search: a college staff (its program) or an NFL club.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Seat {
    College(usize),
    Nfl(usize),
}

/// How sure a read must be to count (FS2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sure {
    Any,
    Fair,
    Narrow,
}

impl Sure {
    pub fn parse(s: Option<&str>) -> Sure {
        match s {
            Some("fair") => Sure::Fair,
            Some("narrow") => Sure::Narrow,
            _ => Sure::Any,
        }
    }
    pub fn key(self) -> &'static str {
        match self {
            Sure::Any => "any",
            Sure::Fair => "fair",
            Sure::Narrow => "narrow",
        }
    }
    /// The widest spread (rating points) a read may have.
    fn max_sd(self) -> f64 {
        match self {
            Sure::Any => f64::MAX,
            Sure::Fair => 6.0,
            Sure::Narrow => 3.5,
        }
    }
}

/// The services' word on one class (F1, F4, FS15, FR9).
#[derive(Default)]
pub struct Ranks {
    pub date: Option<Date>,
    pub prev_date: Option<Date>,
    /// National rank and composite rating.
    pub comp: HashMap<u32, (u32, f64)>,
    pub prev: HashMap<u32, u32>,
    /// Rank at his position and in his state.
    pub pos_state: HashMap<u32, (u32, u32)>,
    /// Each active service's stars and rank (desk index, by player).
    pub boards: Vec<(usize, services::ServiceBoard)>,
}

impl Ranks {
    pub fn of_class(w: &World, class_of: i32) -> Ranks {
        let Some(c) = services::class_ranks(w, class_of) else { return Ranks::default() };
        let mut pos = [0u32; afm_engine::positions::POSITION_COUNT];
        let mut state: HashMap<usize, u32> = HashMap::new();
        let mut pos_state = HashMap::with_capacity(c.ids.len());
        for &id in &c.ids {
            if let Some(i) = crate::views::person_index(w, id) {
                let p = &w.people[i];
                let k = p.player.position.index();
                pos[k] += 1;
                let s = state.entry(p.home_state).or_insert(0);
                *s += 1;
                pos_state.insert(id, (pos[k], *s));
            }
        }
        Ranks {
            date: Some(c.date),
            prev_date: c.prev_date,
            comp: services::composite(w, c),
            prev: services::previous(c),
            pos_state,
            boards: services::service_boards(w, c),
        }
    }
}

/// The NFL club's side: its board, grades and the public desks (FR14).
#[derive(Default)]
pub struct Draft {
    /// Your board's order (rank from 0) and its grade tier.
    pub board: HashMap<u32, (usize, u8)>,
    pub public: HashMap<u32, usize>,
    /// Each analyst desk's big board: (desk index, rank by player from 1).
    pub desks: Vec<(usize, HashMap<u32, u32>)>,
    /// Each player's earliest and latest pick across the desks' latest
    /// mocks.
    pub mocks: HashMap<u32, (u16, u16)>,
    pub lists: HashMap<u32, u8>,
    pub year: i32,
}

/// A kid's race as he feels it: (program, appeal), best first.
type Race = Arc<Vec<(usize, f64)>>;

/// One search's context: the world, your seat and staff, and what the
/// public and your staff know, read once for the call.
pub struct Ctx<'a> {
    pub w: &'a World,
    pub seat: Seat,
    pub staff: Staff,
    pub reads: Arc<HashMap<u32, Read>>,
    pub ranks: Ranks,
    pub draft: Draft,
    /// The coming draft's dates (its combine, pro days and first night).
    pub dd: YearDates,
    pub board: HashSet<u32>,
    pub sure: Sure,
    pub unseen: bool,
    /// Your campus's state and its place on the map.
    pub home: usize,
    pub campus: (f64, f64),
    races: RefCell<HashMap<usize, Race>>,
    miles: RefCell<HashMap<usize, f64>>,
    /// High schools with a kid committed or signed to you (and the kids).
    mates: RefCell<Option<HashMap<usize, Vec<u32>>>>,
}

impl<'a> Ctx<'a> {
    pub fn new(w: &'a World, seat: Seat, staff: Staff, sure: Sure, unseen: bool) -> Ctx<'a> {
        let (home, campus) = match seat {
            Seat::College(j) => {
                let pr = &w.programs[j];
                (pr.state, geo::campus(pr.state, &pr.city))
            }
            Seat::Nfl(t) => {
                let st =
                    afm_engine::world::franchise::era(t, w.year).and_then(|e| geo::state_index(e.state)).unwrap_or(0);
                (st, geo::place(st, ""))
            }
        };
        let board: HashSet<u32> = match seat {
            Seat::College(_) => w.front_office.college.board.iter().map(|t| t.id).collect(),
            Seat::Nfl(_) => HashSet::new(),
        };
        Ctx {
            w,
            seat,
            staff,
            reads: Arc::new(HashMap::new()),
            ranks: Ranks::default(),
            draft: Draft::default(),
            dd: draft_dates(w),
            board,
            sure,
            unseen,
            home,
            campus,
            races: RefCell::new(HashMap::new()),
            miles: RefCell::new(HashMap::new()),
            mates: RefCell::new(None),
        }
    }

    pub fn p(&self, i: usize) -> &'a Person {
        &self.w.people[i]
    }

    pub fn read(&self, i: usize) -> Option<&Read> {
        self.reads.get(&self.w.people[i].player.id)
    }

    pub fn program(&self) -> Option<usize> {
        if let Seat::College(j) = self.seat { Some(j) } else { None }
    }

    /// A read's value if it is sure enough for the search.
    fn sure_val(&self, centre: f64, sd: f64) -> Val {
        if sd <= self.sure.max_sd() { Val::N(centre) } else { Val::Unseen }
    }

    /// His race as he feels it (schools that offered him, best first),
    /// read once per call.
    pub fn race(&self, i: usize) -> Arc<Vec<(usize, f64)>> {
        if let Some(r) = self.races.borrow().get(&i) {
            return r.clone();
        }
        let r = Arc::new(desk::race_at(self.w, i));
        self.races.borrow_mut().insert(i, r.clone());
        r
    }

    /// Whether a teammate of his at his high school is committed or signed
    /// to you (the schools are gathered once per call).
    pub fn teammate_committed(&self, i: usize) -> bool {
        let Some(j) = self.program() else { return false };
        let p = self.p(i);
        let mut m = self.mates.borrow_mut();
        let map = m.get_or_insert_with(|| {
            let mut map: HashMap<usize, Vec<u32>> = HashMap::new();
            for q in &self.w.people {
                if matches!(q.stage, Stage::HighSchool | Stage::PrepSchool)
                    && (q.cal.commit.is_some_and(|c| c.0 == j) || q.cal.signed.is_some_and(|s| s.program == j))
                {
                    map.entry(q.hs).or_default().push(q.player.id);
                }
            }
            map
        });
        map.get(&p.hs).is_some_and(|v| v.iter().any(|&id| id != p.player.id))
    }

    /// Miles from your campus to his high school's town (FR4).
    pub fn miles(&self, i: usize) -> f64 {
        let p = self.p(i);
        let at = if matches!(p.stage, Stage::HighSchool | Stage::PrepSchool) {
            p.hs
        } else {
            p.college.map_or(usize::MAX, |c| 1_000_000 + c)
        };
        if let Some(&m) = self.miles.borrow().get(&at) {
            return m;
        }
        let pt = if at >= 1_000_000 {
            let pr = &self.w.programs[at - 1_000_000];
            geo::campus(pr.state, &pr.city)
        } else {
            match self.w.high_schools.get(p.hs) {
                Some(h) => geo::place(h.state, &h.town),
                None => geo::place(p.home_state, ""),
            }
        };
        let m = geo::miles_between(self.campus, pt).max(1.0);
        self.miles.borrow_mut().insert(at, m);
        m
    }
}

// --- Words ------------------------------------------------------------------------------

pub const PROJ: [&str; 5] =
    ["a small-college player", "an FCS player", "an FBS starter", "a Power Four starter", "a future NFL player"];
pub const PROJ_NFL: [&str; 5] =
    ["a camp body", "an NFL backup", "an NFL starter", "a Pro Bowl player", "one of the best in the league"];
pub const SURE_WORDS: [&str; 3] = ["unsure", "fairly sure", "sure"];
pub const FIRM: [&str; 4] = ["Open", "Committed, listening", "Committed, firm", "Signed"];
pub const RISK: [&str; 3] = ["High", "Some", "None"];
pub const CORE: [&str; 2] = ["Behind", "On track"];
pub const CAMP: [&str; 3] = ["Not invited", "Invited", "Came to yours"];
pub const MEDICAL: [&str; 4] = ["Failed", "Major concern", "Minor concern", "Cleared"];
pub const FRAME: [&str; 4] = ["Filled out", "Some room", "Room to grow", "Lots of room"];
pub const REGIONS: [&str; 4] = ["Northeast", "Midwest", "South", "West"];
pub const SPORTS: [&str; 5] = ["None", "Baseball", "Basketball", "Wrestling", "Track"];
pub const DECIDER: [&str; 4] = ["He decides", "His parents", "His grandparents", "A trainer around him"];
pub const VS: [&str; 3] = ["You're lower", "Level", "You're higher"];
pub const STAFF_WORDS: [&str; 2] = ["Not seen", "Seen"];
pub const DIVISIONS: [&str; 3] = ["Division II and below", "FCS", "FBS"];

/// The stat lines the search reads: key, label, the high school stat and
/// the college one.
pub const STATS: [(&str, &str, HsStat, Stat); 18] = [
    ("PassYds", "Passing yards", HsStat::PassYds, Stat::PassYds),
    ("PassTd", "Passing TDs", HsStat::PassTd, Stat::PassTd),
    ("Int", "Interceptions thrown", HsStat::Int, Stat::Int),
    ("Rush", "Carries", HsStat::Rush, Stat::Rush),
    ("RushYds", "Rushing yards", HsStat::RushYds, Stat::RushYds),
    ("RushTd", "Rushing TDs", HsStat::RushTd, Stat::RushTd),
    ("Rec", "Catches", HsStat::Rec, Stat::Rec),
    ("RecYds", "Receiving yards", HsStat::RecYds, Stat::RecYds),
    ("RecTd", "Receiving TDs", HsStat::RecTd, Stat::RecTd),
    ("Tackles", "Tackles", HsStat::Tackles, Stat::Tackles),
    ("Tfl", "Tackles for loss", HsStat::Tfl, Stat::Tfl),
    ("Sacks", "Sacks", HsStat::Sacks, Stat::Sacks),
    ("Ints", "Interceptions", HsStat::Ints, Stat::Ints),
    ("Pbus", "Pass breakups", HsStat::Pbus, Stat::Pbus),
    ("FgMade", "Field goals made", HsStat::FgMade, Stat::FgMade),
    ("FgAtt", "Field goals tried", HsStat::FgAtt, Stat::FgAtt),
    ("Cmp", "Completions", HsStat::Cmp, Stat::Cmp),
    ("Att", "Pass attempts", HsStat::Att, Stat::Att),
];

/// The position reads (FS3): one read per job, from its attributes.
pub const POS_READS: [(&str, &str, &[Attr]); 14] = [
    ("throwing", "Throwing", &[Attr::ShortAccuracy, Attr::MediumAccuracy, Attr::DeepAccuracy, Attr::ArmStrength]),
    ("pocket", "Pocket play", &[Attr::PocketPresence, Attr::Decisions, Attr::QuickRelease]),
    ("carrying", "Ball carrying", &[Attr::BallCarrierVision, Attr::Elusiveness, Attr::BallSecurity]),
    ("routes", "Route running", &[Attr::RouteRunning, Attr::Release]),
    ("hands", "Hands", &[Attr::Catching, Attr::ContestedCatch]),
    ("passpro", "Pass protection", &[Attr::PassBlocking, Attr::Anchor, Attr::HandUse]),
    ("runblock", "Run blocking", &[Attr::RunBlocking, Attr::BlockingInSpace, Attr::Strength]),
    ("passrush", "Pass rush", &[Attr::PowerRush, Attr::FinesseRush, Attr::HandUse]),
    ("rundef", "Run defense", &[Attr::BlockShedding, Attr::Tackling, Attr::Pursuit, Attr::PlayRecognition]),
    ("coverage", "Coverage", &[Attr::ManCoverage, Attr::ZoneCoverage, Attr::Press]),
    ("ballskills", "Ball skills", &[Attr::BallTracking, Attr::ContestedCatch, Attr::Anticipation]),
    ("tackling", "Tackling", &[Attr::Tackling, Attr::Pursuit]),
    ("kicking", "Kicking", &[Attr::KickPower, Attr::ShortKickAccuracy, Attr::LongKickAccuracy]),
    ("burst", "Speed and burst", &[Attr::Speed, Attr::Acceleration, Attr::Agility]),
];

const TRAIT_KEYS: [usize; 4] = [0, 1, 9, 7];

// --- The catalog --------------------------------------------------------------------------

/// The fields for a seat's search, by group, in the order the screen
/// lists them.
pub fn catalog(seat: Seat, w: &World) -> Vec<Group> {
    let college = matches!(seat, Seat::College(_));
    let mut g: Vec<Group> = Vec::new();
    let mut reads = vec![
        Field::new("now", "Overall read", FKind::Num).staff(),
        Field::new("proj", if college { "Could play" } else { "Could be" }, FKind::Word)
            .opts(if college { &PROJ } else { &PROJ_NFL })
            .staff(),
        Field::new("sure", "How sure of the ceiling", FKind::Word).opts(&SURE_WORDS).staff(),
        Field::new("seen", "Your staff", FKind::Word).opts(&STAFF_WORDS),
        Field::new("looks", "Looks in your file", FKind::Num).staff(),
    ];
    if !college {
        reads.push(Field::new("board", "Your board", FKind::Num).lower());
        reads.push(Field::new("grade", "Your grade (round tier)", FKind::Num).lower());
    } else {
        reads.push(Field::new("vs", "You vs the services", FKind::Word).opts(&VS).staff());
    }
    for a in Attr::ALL {
        reads.push(Field::new(format!("attr:{}", a.key()), format!("{} read", a.label()), FKind::Num).staff());
    }
    g.push(Group { name: if college { "Your staff's reads" } else { "Your scouts' reads" }, id: "FS2", fields: reads });
    g.push(Group {
        name: "Position reads",
        id: "FS3",
        fields: POS_READS.iter().map(|(k, l, _)| Field::new(format!("pr:{k}"), *l, FKind::Num).staff()).collect(),
    });
    g.push(Group {
        name: "Measurables",
        id: "FS4",
        fields: vec![
            Field::new("ht_v", "Height, verified (in)", FKind::Num).dp(1),
            Field::new("wt_v", "Weight, verified", FKind::Num),
            Field::new("arm", "Arm length, verified (in)", FKind::Num).dp(1),
            Field::new("hand", "Hand size, verified (in)", FKind::Num).dp(1),
            Field::new("wing", "Wingspan, verified (in)", FKind::Num).dp(1),
            Field::new("ht", "Height as listed (in)", FKind::Num).dp(1),
            Field::new("wt", "Weight as listed", FKind::Num),
        ],
    });
    g.push(Group {
        name: "Testing",
        id: "FS5",
        fields: vec![
            Field::new("t40", "40", FKind::Num).lower().dp(2),
            Field::new("t10", "10-yard split", FKind::Num).lower().dp(2),
            Field::new("tsh", "Shuttle", FKind::Num).lower().dp(2),
            Field::new("t3c", "3-cone", FKind::Num).lower().dp(2),
            Field::new("tvj", "Vertical (in)", FKind::Num).dp(1),
            Field::new("tbj", "Broad jump (in)", FKind::Num),
            Field::new("tbp", "Bench (225 reps)", FKind::Num),
            Field::new("tver", "Testing verified", FKind::Flag),
        ],
    });
    let mut prod = vec![Field::new("games", "Games last season", FKind::Num)];
    for (k, l, _, _) in STATS {
        prod.push(Field::new(format!("st:{k}"), format!("{l}, last season"), FKind::Num));
    }
    prod.push(Field::new("rate:cmp", "Completion %", FKind::Num).dp(1));
    prod.push(Field::new("rate:ypc", "Yards a carry", FKind::Num).dp(1));
    prod.push(Field::new("rate:ypr", "Yards a catch", FKind::Num).dp(1));
    prod.push(Field::new("rate:fg", "Field goal %", FKind::Num).dp(1));
    for (k, l, _, _) in STATS {
        prod.push(Field::new(format!("pg:{k}"), format!("{l} a game"), FKind::Num).dp(1));
    }
    for (k, l, _, _) in STATS {
        prod.push(Field::new(format!("car:{k}"), format!("{l}, career"), FKind::Num));
    }
    if !college {
        prod.push(Field::new("starts", "Seasons as a starter", FKind::Num));
    }
    g.push(Group { name: "Production", id: "FS6", fields: prod });
    g.push(Group {
        name: "Competition",
        id: "FS7",
        fields: if college {
            vec![
                Field::new("class", "State classification (1 the smallest)", FKind::Num),
                Field::new("sos", "Schedule strength (1-10)", FKind::Num).dp(1),
                Field::new("culture", "State football culture (1-10)", FKind::Num).dp(1),
            ]
        } else {
            vec![
                Field::new("conf", "Conference", FKind::Text),
                Field::new("div", "Level", FKind::Word).opts(&DIVISIONS),
                Field::new("sos", "Schedule strength (points)", FKind::Num).dp(1),
            ]
        },
    });
    if college {
        g.push(Group {
            name: "Other sports",
            id: "FS8",
            fields: vec![
                Field::new("sport", "Other sport", FKind::Word).opts(&SPORTS),
                Field::new("m100", "100 m", FKind::Num).lower().dp(2),
                Field::new("m200", "200 m", FKind::Num).lower().dp(2),
                Field::new("hurdles", "110 m hurdles", FKind::Num).lower().dp(2),
                Field::new("shot", "Shot put (ft)", FKind::Num).dp(1),
                Field::new("hoops", "Basketball points a game", FKind::Num).dp(1),
                Field::new("wrestle", "Wrestling wins", FKind::Num),
            ],
        });
    }
    g.push(Group {
        name: "Scouted habits",
        id: "FS10",
        fields: Tend::ALL
            .iter()
            .map(|t| Field::new(format!("tend:{}", t.index()), t.label(), FKind::Num).staff().dp(1))
            .collect(),
    });
    let mut chr: Vec<Field> = file::TRAIT_NAMES
        .iter()
        .enumerate()
        .map(|(k, n)| Field::new(format!("chr:{k}"), *n, FKind::Num).staff().dp(1))
        .collect();
    chr.sort_by_key(|f| {
        let k: usize = f.k[4..].parse().unwrap_or(99);
        TRAIT_KEYS.iter().position(|&x| x == k).unwrap_or(9)
    });
    chr.push(Field::new("decider", "Who decides", FKind::Word).opts(&DECIDER).staff());
    chr.push(Field::new("trouble", "Trouble on his record", FKind::Flag));
    g.push(Group { name: "Character and family", id: "FS11", fields: chr });
    if college {
        g.push(Group {
            name: "Academics",
            id: "FS12",
            fields: vec![
                Field::new("gpa", "GPA", FKind::Num).dp(2),
                Field::new("test", "Test score (SAT scale)", FKind::Num),
                Field::new("core", "Core courses", FKind::Word).opts(&CORE),
                Field::new("risk", "Qualifier risk", FKind::Word).opts(&RISK),
            ],
        });
        g.push(Group {
            name: "Recruitment",
            id: "FS13",
            fields: vec![
                Field::new("committed", "Committed", FKind::Flag),
                Field::new("firm", "How firmly", FKind::Word).opts(&FIRM),
                Field::new("offers", "Offers", FKind::Num),
                Field::new("no_offers", "No offers yet", FKind::Flag),
                Field::new("offer_conf", "Has an offer from the conference", FKind::Text),
                Field::new("offer_from", "Has an offer from", FKind::Text),
                Field::new("leader", "Who leads", FKind::Text),
                Field::new("officials", "Officials used (of 5)", FKind::Num),
                Field::new("visits_you", "Visits to you", FKind::Num),
                Field::new("top_out", "Top schools announced", FKind::Flag),
                Field::new("insider", "Insider pick", FKind::Text),
                Field::new("insider_conf", "Insider confidence (%)", FKind::Num),
                Field::new("board_on", "On your board", FKind::Flag),
                Field::new("camp", "Your camp", FKind::Word).opts(&CAMP),
                Field::new("followers", "Followers", FKind::Num),
                Field::new("nil", "Public NIL value ($)", FKind::Num),
            ],
        });
        g.push(Group {
            name: "Geography and ties",
            id: "FS14",
            fields: vec![
                Field::new("state", "State", FKind::Text),
                Field::new("region", "Region", FKind::Word).opts(&REGIONS),
                Field::new("miles", "Miles from campus", FKind::Num).lower(),
                Field::new("instate", "In-state", FKind::Flag),
                Field::new("footprint", "Your footprint (0-100)", FKind::Num),
                Field::new("pipeline", "Your pipeline school", FKind::Flag),
                Field::new("area", "Area recruiter", FKind::Text),
                Field::new("ties", "Ties to you", FKind::Flag),
            ],
        });
        let mut svc = vec![
            Field::new("comp", "Composite", FKind::Num).dp(4),
            Field::new("cstars", "Composite stars", FKind::Num),
            Field::new("nat", "National rank", FKind::Num).lower(),
            Field::new("posrank", "Position rank", FKind::Num).lower(),
            Field::new("staterank", "State rank", FKind::Num).lower(),
            Field::new("move", "Rank movement (places up)", FKind::Num),
            Field::new("new", "Ranked for the first time", FKind::Flag),
            Field::new("camp_stars", "Kicking camps' stars", FKind::Num),
        ];
        for (k, d) in w.scouting.desks.iter().enumerate() {
            if d.kind == afm_engine::world::scouting::public::DeskKind::Service && d.active(w.year) {
                svc.push(Field::new(format!("svc:{k}"), format!("{} stars", d.name), FKind::Num));
            }
        }
        g.push(Group { name: "Services", id: "FS15", fields: svc });
    } else {
        let mut desks = vec![
            Field::new("cons", "Consensus rank", FKind::Num).lower(),
            Field::new("mock_hi", "Earliest mock pick", FKind::Num).lower(),
            Field::new("mock_lo", "Latest mock pick", FKind::Num).lower(),
            Field::new("vsd", "You vs the desks (places)", FKind::Num),
        ];
        for (k, d) in w.scouting.desks.iter().enumerate() {
            if d.kind == afm_engine::world::scouting::public::DeskKind::Analyst && d.active(w.year) {
                desks.push(Field::new(format!("desk:{k}"), format!("{} rank", d.outlet), FKind::Num).lower());
            }
        }
        g.push(Group { name: "Public desks", id: "FR14", fields: desks });
        g.push(Group {
            name: "Process",
            id: "FR14",
            fields: vec![
                Field::new("allstar", "All-star game", FKind::Text),
                Field::new("combine", "Combine invite", FKind::Flag),
                Field::new("t30", "On your top-30 list", FKind::Flag),
                Field::new("interview", "On your interview list", FKind::Flag),
            ],
        });
    }
    g.push(Group {
        name: if college { "Age for his class" } else { "Age and class" },
        id: "FS16",
        fields: if college {
            vec![
                Field::new("age_class", "Age on 1 September of his senior year", FKind::Num).dp(1).lower(),
                Field::new("young", "Young for his class", FKind::Flag),
                Field::new("reclass", "Held back or reclassified", FKind::Flag),
                Field::new("born", "Birth year", FKind::Num),
            ]
        } else {
            vec![
                Field::new("age_draft", "Age at the draft", FKind::Num).dp(1).lower(),
                Field::new("early", "Early entrant", FKind::Flag),
                Field::new("fifth", "Fifth-year senior", FKind::Flag),
            ]
        },
    });
    g.push(Group {
        name: "Frame",
        id: "FS17",
        fields: vec![
            Field::new("room", "Room to grow (lb)", FKind::Num).staff(),
            Field::new("frame", "Frame", FKind::Word).opts(&FRAME).staff(),
        ],
    });
    g.push(Group {
        name: "Injuries",
        id: "FS18",
        fields: vec![
            Field::new("missed", "Games missed hurt", FKind::Num).lower(),
            Field::new("major", "Major injuries", FKind::Num).lower(),
            Field::new("medical", "Medical read", FKind::Word).opts(&MEDICAL).staff(),
        ],
    });
    g
}

/// One field's definition by key.
pub fn field_of(groups: &[Group], k: &str) -> Option<Field> {
    groups.iter().flat_map(|g| g.fields.iter()).find(|f| f.k == k).cloned()
}

pub fn catalog_json(groups: &[Group]) -> Value {
    json!(
        groups
            .iter()
            .map(|g| json!({ "name": g.name, "id": g.id, "fields": g.fields.iter().map(Field::json).collect::<Vec<_>>() }))
            .collect::<Vec<_>>()
    )
}

// --- Values -----------------------------------------------------------------------------

fn flag(b: bool) -> Val {
    Val::N(if b { 1.0 } else { 0.0 })
}

/// One field's value for one player.
pub fn value(ctx: &Ctx, i: usize, k: &str) -> Val {
    let w = ctx.w;
    let p = ctx.p(i);
    let id = p.player.id;
    if let Some((head, tail)) = k.split_once(':') {
        return match head {
            "attr" => match (Attr::ALL.iter().find(|a| a.key() == tail), ctx.read(i)) {
                (Some(a), Some(r)) => r.attr(a.index()).map_or(Val::Unseen, |(c, sd)| ctx.sure_val(c, sd)),
                _ => Val::Unseen,
            },
            "pr" => pos_read(ctx, i, tail),
            "st" | "pg" | "car" => stat(ctx, i, head, tail),
            "rate" => rate(ctx, i, tail),
            "tend" => tail
                .parse::<usize>()
                .ok()
                .and_then(|t| Tend::ALL.get(t).copied())
                .map_or(Val::None, |t| tend(ctx, i, t)),
            "chr" => match (tail.parse::<usize>().ok(), ctx.read(i)) {
                (Some(t), Some(r)) => r
                    .traits
                    .get(t)
                    .filter(|x| x.1 > 0.0)
                    .map_or(Val::Unseen, |x| ctx.sure_val(x.0 as f64, x.1 as f64 * 1.6)),
                _ => Val::Unseen,
            },
            "svc" => {
                let d: usize = tail.parse().unwrap_or(usize::MAX);
                match ctx.ranks.boards.iter().find(|b| b.0 == d) {
                    Some((_, m)) => m.get(&id).map_or(Val::N(0.0), |x| Val::N(x.0 as f64)),
                    None => Val::None,
                }
            }
            "desk" => {
                let d: usize = tail.parse().unwrap_or(usize::MAX);
                ctx.draft
                    .desks
                    .iter()
                    .find(|b| b.0 == d)
                    .and_then(|(_, m)| m.get(&id))
                    .map_or(Val::None, |&r| Val::N(r as f64))
            }
            _ => Val::None,
        };
    }
    match k {
        "now" => match ctx.read(i) {
            Some(r) if r.now.1 > 0.0 || r.now.0 > 0.0 => ctx.sure_val(r.now.0 as f64, r.now.1 as f64),
            _ => Val::Unseen,
        },
        "proj" => match ctx.read(i) {
            Some(r) if r.ceiling.0 > 0.0 => ctx.sure_val(tier(ctx, r.ceiling.0 as f64) as f64, r.ceiling.1 as f64),
            _ => Val::Unseen,
        },
        "sure" => match ctx.read(i) {
            Some(r) if r.ceiling.0 > 0.0 => Val::N(match r.ceiling.1 {
                s if s <= 3.5 => 2.0,
                s if s <= 6.0 => 1.0,
                _ => 0.0,
            }),
            _ => Val::Unseen,
        },
        "seen" => flag(ctx.read(i).is_some()),
        "looks" => ctx.read(i).map_or(Val::Unseen, |r| Val::N(r.looks as f64)),
        "vs" => vs_services(ctx, i),
        "board" => ctx.draft.board.get(&id).map_or(Val::None, |b| Val::N(b.0 as f64 + 1.0)),
        "grade" => ctx.draft.board.get(&id).map_or(Val::None, |b| Val::N(b.1 as f64)),
        "ht_v" | "wt_v" | "arm" | "hand" | "wing" => measured(ctx, i, k),
        "ht" => Val::N(file::listed(&p.player).0),
        "wt" => Val::N(file::listed(&p.player).1),
        "t40" | "t10" | "tsh" | "t3c" | "tvj" | "tbj" | "tbp" => {
            testing(ctx, i).map_or(Val::None, |t| Val::N(t.get(k)))
        }
        "tver" => testing(ctx, i).map_or(Val::None, |t| flag(t.verified)),
        "games" => last_games(ctx, i).map_or(Val::None, |g| Val::N(g as f64)),
        "class" => hs_of(ctx, i).map_or(Val::None, |h| Val::N(w.high_schools[h].classification as f64 + 1.0)),
        "sos" => sos(ctx, i),
        "culture" => Val::N((w.states.get(p.home_state).map_or(0.5, |s| s.culture) * 10.0).clamp(1.0, 10.0)),
        "conf" => p.college.map_or(Val::None, |c| Val::T(w.programs[c].conference.clone())),
        "div" => p.college.map_or(Val::None, |c| {
            Val::N(match w.programs[c].division {
                afm_engine::world::data::Division::FBS => 2.0,
                afm_engine::world::data::Division::FCS => 1.0,
                _ => 0.0,
            })
        }),
        "sport" | "m100" | "m200" | "hurdles" | "shot" | "hoops" | "wrestle" => other_sport(ctx, i, k),
        "decider" => match ctx.read(i) {
            Some(r) if r.contact > 0 => Val::N(desk::family_read(w, i).decider as usize as f64),
            _ => Val::Unseen,
        },
        "trouble" => flag(trouble(p)),
        "gpa" => Val::N(gpa_shown(ctx, i)),
        "test" => p.cal.test_score.map_or(Val::None, |t| Val::N(t as f64)),
        "core" => Val::N(if core_behind(p) > 0 { 0.0 } else { 1.0 }),
        "risk" => Val::N(risk(ctx, i) as f64),
        "committed" => flag(p.cal.commit.is_some() || p.cal.signed.is_some()),
        "firm" => Val::N(firm(ctx, i) as f64),
        "offers" => Val::N(crate::views::offers_of(p).len() as f64),
        "no_offers" => flag(crate::views::offers_of(p).is_empty()),
        "offer_conf" => Val::T(
            crate::views::offers_of(p).iter().map(|&o| w.programs[o].conference.clone()).collect::<Vec<_>>().join("\n"),
        ),
        "offer_from" => Val::T(
            crate::views::offers_of(p).iter().map(|&o| w.program_name(o).to_string()).collect::<Vec<_>>().join("\n"),
        ),
        "leader" => leader(ctx, i).map_or(Val::None, |j| Val::T(w.program_name(j).to_string())),
        "officials" => Val::N(p.cal.visits.iter().filter(|v| v.2 == Visit::Official).count() as f64),
        "visits_you" => Val::N(ctx.program().map_or(0, |j| p.cal.visits.iter().filter(|v| v.0 == j).count()) as f64),
        "top_out" => flag(w.scouting.rankings.tops.iter().any(|t| t.id == id)),
        "insider" => insider(ctx, i).map_or(Val::None, |x| Val::T(w.program_name(x.program as usize).to_string())),
        "insider_conf" => insider(ctx, i).map_or(Val::None, |x| Val::N(x.conf as f64)),
        "board_on" => flag(ctx.board.contains(&id)),
        "camp" => Val::N(camp(ctx, i) as f64),
        "followers" => Val::N(p.followers as f64),
        "nil" => nil_value(ctx, i).map_or(Val::None, Val::N),
        "state" => Val::T(w.states.get(p.home_state).map_or(String::new(), |s| s.code.clone())),
        "region" => Val::N(region(w.states.get(p.home_state).map_or("", |s| s.code.as_str())) as f64),
        "miles" => Val::N(ctx.miles(i).round()),
        "instate" => flag(p.home_state == ctx.home),
        "footprint" => {
            ctx.program().map_or(Val::None, |j| Val::N((w.programs[j].footprint[p.home_state.min(50)] * 100.0).round()))
        }
        "pipeline" => ctx.program().map_or(Val::None, |j| flag(w.programs[j].hs_pipeline.contains_key(&p.hs))),
        "area" => area_coach(ctx, i).map_or(Val::None, |c| Val::T(w.coaches[c].name.clone())),
        "ties" => flag(!ties(ctx, i).is_empty()),
        "comp" => ctx.ranks.comp.get(&id).map_or(Val::None, |c| Val::N(c.1)),
        "cstars" => Val::N(p.stars as f64),
        "nat" => ctx.ranks.comp.get(&id).map_or(Val::None, |c| Val::N(c.0 as f64)),
        "posrank" => ctx.ranks.pos_state.get(&id).map_or(Val::None, |c| Val::N(c.0 as f64)),
        "staterank" => ctx.ranks.pos_state.get(&id).map_or(Val::None, |c| Val::N(c.1 as f64)),
        "move" => movement(ctx, i).map_or(Val::None, |m| Val::N(m as f64)),
        "new" => {
            flag(ctx.ranks.comp.contains_key(&id) && ctx.ranks.prev_date.is_some() && !ctx.ranks.prev.contains_key(&id))
        }
        "camp_stars" => Val::N(p.camp_stars as f64),
        "cons" => ctx.draft.public.get(&id).map_or(Val::None, |&r| Val::N(r as f64 + 1.0)),
        "mock_hi" => ctx.draft.mocks.get(&id).map_or(Val::None, |m| Val::N(m.0 as f64)),
        "mock_lo" => ctx.draft.mocks.get(&id).map_or(Val::None, |m| Val::N(m.1 as f64)),
        "vsd" => match (ctx.draft.board.get(&id), ctx.draft.public.get(&id)) {
            (Some(b), Some(&c)) => Val::N(c as f64 - b.0 as f64),
            _ => Val::None,
        },
        "allstar" => p.cal.all_star.as_ref().map_or(Val::None, |t| Val::T(t.to_string())),
        "combine" => flag(p.cal.combine_invite),
        "t30" => flag(ctx.draft.lists.get(&id).is_some_and(|l| l & 1 != 0)),
        "interview" => flag(ctx.draft.lists.get(&id).is_some_and(|l| l & 2 != 0)),
        "age_class" => age_for_class(p).map_or(Val::None, Val::N),
        "young" => age_for_class(p).map_or(Val::None, |a| flag(a < 17.25)),
        "reclass" => flag(reclassified(p)),
        "born" => {
            if p.born == afm_engine::calendar::born::UNKNOWN {
                Val::None
            } else {
                Val::N(p.born.year() as f64)
            }
        }
        "age_draft" => Val::N(age_at_draft(ctx, i)),
        "early" => flag(early_entrant(ctx, i)),
        "fifth" => flag(p.redshirt_used && p.college_seasons >= 4),
        "starts" => Val::N(
            p.player
                .history
                .iter()
                .filter(|h| h.level.is_college() && h.playing_time == afm_engine::player::PlayingTime::Starter)
                .count() as f64,
        ),
        "room" => frame_read(ctx, i).map_or(Val::Unseen, Val::N),
        "frame" => frame_read(ctx, i).map_or(Val::Unseen, |r| Val::N(frame_word(r) as f64)),
        "missed" => Val::N(p.player.history.iter().map(|h| h.games_missed as f64).sum()),
        "major" => Val::N(p.player.history.iter().filter(|h| h.events.has(Ev::MajorInjury)).count() as f64),
        "medical" => match ctx.read(i) {
            Some(r) if r.medical.1 => Val::N(3.0 - r.medical.0.min(3) as f64),
            _ => Val::Unseen,
        },
        _ => Val::None,
    }
}

/// The role tier of a ceiling read (0 the lowest, 4 the highest).
pub fn tier(ctx: &Ctx, ceiling: f64) -> usize {
    use afm_engine::career::NFL_LEVELS as L;
    match ctx.seat {
        Seat::College(_) => match ceiling {
            c if c >= L.starter => 4,
            c if c >= 68.0 => 3,
            c if c >= 61.0 => 2,
            c if c >= 54.0 => 1,
            _ => 0,
        },
        Seat::Nfl(_) => match ceiling {
            c if c >= L.elite => 4,
            c if c >= L.pro_bowl => 3,
            c if c >= L.starter => 2,
            c if c >= L.roster => 1,
            _ => 0,
        },
    }
}

/// Your staff's ceiling tier against the services' stars (FR10): 1 you're
/// higher (a sleeper to you), -1 lower, 0 level. Read as a word.
fn vs_services(ctx: &Ctx, i: usize) -> Val {
    let p = ctx.p(i);
    let Some(r) = ctx.read(i).filter(|r| r.ceiling.0 > 0.0) else { return Val::Unseen };
    if r.ceiling.1 as f64 > ctx.sure.max_sd() {
        return Val::Unseen;
    }
    let ours = tier(ctx, r.ceiling.0 as f64) as i32;
    let theirs = (rstars(p) as i32 - 1).clamp(0, 4);
    Val::N((ours - theirs).signum() as f64 + 1.0)
}

fn pos_read(ctx: &Ctx, i: usize, k: &str) -> Val {
    let Some((_, _, attrs)) = POS_READS.iter().find(|x| x.0 == k) else { return Val::None };
    let Some(r) = ctx.read(i) else { return Val::Unseen };
    let got: Vec<(f64, f64)> = attrs.iter().filter_map(|a| r.attr(a.index())).collect();
    if got.is_empty() {
        return Val::Unseen;
    }
    let n = got.len() as f64;
    let c = got.iter().map(|x| x.0).sum::<f64>() / n;
    let sd = got.iter().map(|x| x.1).sum::<f64>() / n;
    ctx.sure_val((c * 10.0).round() / 10.0, sd)
}

/// A tendency as your staff reads it (FS10): the middle of its read, on
/// the 1-20 scale, where your staff has seen him on film.
pub fn tend(ctx: &Ctx, i: usize, t: Tend) -> Val {
    let p = ctx.p(i);
    let pos = p.player.position;
    if !t.owned_by(pos) {
        return Val::None;
    }
    let Some(r) = ctx.read(i) else { return Val::Unseen };
    if r.film == 0 {
        return Val::Unseen;
    }
    let read = tend_read(ctx, i, t, r);
    let half = (read.high as f64 - read.low as f64) / 2.0;
    ctx.sure_val((read.low as f64 + read.high as f64) / 2.0, half * 1.5)
}

/// The scouted read the player's file shows (same key and film count).
pub fn tend_read(ctx: &Ctx, i: usize, t: Tend, r: &Read) -> tendencies::TendRead {
    let p = ctx.p(i);
    let key = readers::staff_key(ctx.staff);
    let mut rng = Rng::new(p.player.id ^ (t.index() as u32) << 24 ^ key as u32 ^ ctx.w.year as u32);
    let shown = tendencies::shown(t, p.player.tend.get(t), p.player.position);
    tendencies::scout(shown, r.film as f64 * 1.5, (r.judge as f64).max(6.0), &mut rng)
}

// --- Stats (FR2, FS6) ---------------------------------------------------------------------

fn hs_line_get(l: &HsLine, s: HsStat) -> f64 {
    l.get(s)
}

/// His last season's games (the last high school or college line).
pub fn last_games(ctx: &Ctx, i: usize) -> Option<u32> {
    let p = ctx.p(i);
    if p.college_seasons > 0 || matches!(p.stage, Stage::College | Stage::Declared) {
        ctx.w.stats.career(p.player.id).last().map(|r| r.games as u32)
    } else {
        p.hs_lines.last().map(|l| l.games as u32)
    }
}

fn college_side(p: &Person) -> bool {
    p.college_seasons > 0 || matches!(p.stage, Stage::College | Stage::Declared)
}

fn stat(ctx: &Ctx, i: usize, how: &str, k: &str) -> Val {
    let Some(&(_, _, hs, col)) = STATS.iter().find(|x| x.0 == k) else { return Val::None };
    let p = ctx.p(i);
    if college_side(p) {
        let career = ctx.w.stats.career(p.player.id);
        let Some(last) = career.last() else { return Val::None };
        return match how {
            "st" => Val::N(last.get(col)),
            "pg" => Val::N(last.get(col) / (last.games.max(1) as f64)),
            _ => Val::N(career.iter().map(|r| r.get(col)).sum()),
        };
    }
    let Some(last) = p.hs_lines.last() else { return Val::None };
    match how {
        "st" => Val::N(hs_line_get(last, hs)),
        "pg" => Val::N(hs_line_get(last, hs) / (last.games.max(1) as f64)),
        _ => Val::N(p.hs_lines.iter().map(|l| hs_line_get(l, hs)).sum()),
    }
}

fn rate(ctx: &Ctx, i: usize, k: &str) -> Val {
    let g = |s: &str| stat(ctx, i, "st", s).num().unwrap_or(0.0);
    let (num, den, min, scale) = match k {
        "cmp" => (g("Cmp"), g("Att"), 20.0, 100.0),
        "ypc" => (g("RushYds"), g("Rush"), 20.0, 1.0),
        "ypr" => (g("RecYds"), g("Rec"), 5.0, 1.0),
        "fg" => (g("FgMade"), g("FgAtt"), 3.0, 100.0),
        _ => return Val::None,
    };
    if den < min { Val::None } else { Val::N((num / den * scale * 10.0).round() / 10.0) }
}

/// His last season in a line (FR2), with his classification.
pub fn stat_line(ctx: &Ctx, i: usize) -> Option<String> {
    let p = ctx.p(i);
    let pos = p.player.position;
    if college_side(p) {
        let career = ctx.w.stats.career(p.player.id);
        let last = career.last()?;
        return Some(format!("{}: {}", last.year, crate::lane_core::mail::stat_line(last.pos, |s| last.get(s))));
    }
    let l = p.hs_lines.last()?;
    let g = |s: HsStat| l.get(s);
    use Position as P;
    let n = |s: HsStat| g(s).round() as i64;
    let text = match pos {
        P::QB => format!(
            "{}/{} · {} yds · {} TD · {} INT",
            n(HsStat::Cmp),
            n(HsStat::Att),
            n(HsStat::PassYds),
            n(HsStat::PassTd),
            n(HsStat::Int)
        ),
        P::RB => {
            let ypc = g(HsStat::RushYds) / g(HsStat::Rush).max(1.0);
            format!("{} yds · {:.1} a carry · {} TD", n(HsStat::RushYds), ypc, n(HsStat::RushTd))
        }
        P::WR | P::TE => format!("{} catches · {} yds · {} TD", n(HsStat::Rec), n(HsStat::RecYds), n(HsStat::RecTd)),
        P::OT | P::IOL => format!("{} games started up front", l.games),
        P::EDGE | P::DT | P::LB => {
            format!("{} tackles · {:.1} TFL · {:.1} sacks", n(HsStat::Tackles), g(HsStat::Tfl), g(HsStat::Sacks))
        }
        P::CB | P::S => format!("{} tackles · {} INT · {} PBU", n(HsStat::Tackles), n(HsStat::Ints), n(HsStat::Pbus)),
        P::K => format!("{}/{} FG · {} PAT", n(HsStat::FgMade), n(HsStat::FgAtt), n(HsStat::XpMade)),
        P::P | P::LS => format!("{} games", l.games),
    };
    Some(format!("{}: {text}", l.year))
}

// --- Measurables and testing (FR1, FS4, FS5) ----------------------------------------------

/// Whether a kid's numbers were taken by someone who measures (FR1): your
/// own camp's lasers, or a services' showcase (the ranked kids whose
/// families can travel go to one most years). An NFL prospect's are
/// verified once the combine or his pro day is done.
pub fn verified(ctx: &Ctx, i: usize) -> Option<Kind> {
    let w = ctx.w;
    let p = ctx.p(i);
    match ctx.seat {
        Seat::Nfl(_) => {
            if p.cal.combine_invite && w.today > ctx.dd.combine_end {
                return Some(Kind::Combine);
            }
            let (from, to) = ctx.dd.pro_days;
            let span = (from.days_to(to)).max(1);
            let day =
                from.add((key_uniform(0x9D4A ^ p.college.unwrap_or(0) as u64, w.year as u64) * span as f64) as i32);
            (w.today > day && w.today.month() <= 5).then_some(Kind::ProDay)
        }
        Seat::College(_) => {
            if ctx.read(i).is_some_and(|r| r.camp) {
                return Some(Kind::Camp);
            }
            let ranked = rstars(p) >= 3 || p.camp_stars > 0;
            let goes = key_uniform(0x5A0C ^ w.year as u64, p.player.id as u64) < 0.25 + 0.55 * p.family_money();
            (ranked && goes && matches!(p.stage, Stage::HighSchool | Stage::PrepSchool)).then_some(Kind::Showcase)
        }
    }
}

fn measured(ctx: &Ctx, i: usize, k: &str) -> Val {
    if verified(ctx, i).is_none() {
        return Val::None;
    }
    let b = afm_engine::body::body_now(&ctx.p(i).player);
    let eighth = |x: f64| (x * 8.0).round() / 8.0;
    Val::N(match k {
        "ht_v" => eighth(b.height),
        "wt_v" => b.weight.round(),
        "arm" => eighth(b.arm),
        "hand" => eighth(b.hand),
        _ => eighth(b.wingspan),
    })
}

/// His testing numbers as known: verified ones, else his own or his
/// school's (hand-timed, a little fast).
pub struct Tests {
    pub forty: f64,
    pub split: f64,
    pub shuttle: f64,
    pub cone: f64,
    pub vertical: f64,
    pub broad: f64,
    pub bench: f64,
    pub verified: bool,
    pub kind: Option<Kind>,
}

impl Tests {
    fn get(&self, k: &str) -> f64 {
        match k {
            "t40" => self.forty,
            "t10" => self.split,
            "tsh" => self.shuttle,
            "t3c" => self.cone,
            "tvj" => self.vertical,
            "tbj" => self.broad,
            _ => self.bench,
        }
    }
}

/// His testing numbers as a row shows them.
pub fn tests_json(t: &Tests) -> Value {
    json!({
        "t40": format!("{:.2}", t.forty), "t10": format!("{:.2}", t.split), "tsh": format!("{:.2}", t.shuttle),
        "t3c": format!("{:.2}", t.cone), "tvj": format!("{:.1}", t.vertical), "tbj": t.broad, "tbp": t.bench,
        "ver": t.verified, "how": t.kind.map(|k| k.name()),
    })
}

pub fn testing(ctx: &Ctx, i: usize) -> Option<Tests> {
    let p = ctx.p(i);
    let hs = matches!(p.stage, Stage::HighSchool | Stage::PrepSchool);
    // Most kids nobody ranks or watches have no numbers out.
    if hs && p.stars == 0 && p.camp_stars == 0 && ctx.read(i).is_none() && p.cal.offers.is_empty() {
        return None;
    }
    let year = ctx.w.year;
    let kind = verified(ctx, i);
    let c = file::tests(&p.player, kind.unwrap_or(Kind::Film), year);
    let fast = if kind.is_some() {
        0.0
    } else if hs {
        0.12
    } else {
        0.06
    };
    let r2 = |x: f64| (x * 100.0).round() / 100.0;
    let forty = r2(c.forty - fast);
    let split = r2(forty * 0.352 + 0.03 * key_normal(0x10A5 ^ year as u64, p.player.id as u64));
    Some(Tests {
        forty,
        split,
        shuttle: r2(c.shuttle - fast * 0.8),
        cone: r2(c.cone - fast),
        vertical: ((c.vertical + if kind.is_some() { 0.0 } else { 2.0 }) * 2.0).round() / 2.0,
        broad: c.broad.round(),
        bench: c.bench.round(),
        verified: kind.is_some(),
        kind,
    })
}

// --- Competition, other sports (FS7, FS8) ---------------------------------------------------

pub fn hs_of(ctx: &Ctx, i: usize) -> Option<usize> {
    let p = ctx.p(i);
    (p.hs < ctx.w.high_schools.len()).then_some(p.hs)
}

/// His schedule's strength (FS7): a high schooler's from his school's
/// classification within its state and the state's football; a college
/// player's from his last season's opponents (rating points against his
/// division's average).
fn sos(ctx: &Ctx, i: usize) -> Val {
    let w = ctx.w;
    let p = ctx.p(i);
    if college_side(p) {
        return w.stats.career(p.player.id).last().map_or(Val::None, |r| Val::N((r.sos as f64 * 10.0).round() / 10.0));
    }
    let Some(h) = hs_of(ctx, i) else { return Val::None };
    let s = &w.high_schools[h];
    let st = &w.states[s.state];
    let top = st.schools.iter().map(|&k| w.high_schools[k].classification).max().unwrap_or(0).max(1) as f64;
    let size = s.classification as f64 / top;
    Val::N(((2.0 + 5.0 * size + 3.0 * st.culture) * 10.0).round().clamp(10.0, 100.0) / 10.0)
}

/// His other sport and its public marks (FS8): track times from the
/// meets, basketball points and wrestling wins from the box scores, about
/// as his athletic tools would give them.
fn other_sport(ctx: &Ctx, i: usize, k: &str) -> Val {
    use afm_engine::training::offseason::Sport;
    let p = ctx.p(i);
    if !matches!(p.stage, Stage::HighSchool | Stage::PrepSchool) {
        return Val::None;
    }
    let s = afm_engine::training::offseason::other_sport(&p.player);
    if k == "sport" {
        return Val::N(match s {
            None => 0.0,
            Some(Sport::Baseball) => 1.0,
            Some(Sport::Basketball) => 2.0,
            Some(Sport::Wrestling) => 3.0,
            Some(Sport::Track) => 4.0,
        });
    }
    let id = p.player.id as u64;
    let a = &p.player.attributes;
    let n = |salt: u64| key_normal(0x0750 ^ salt, id);
    let forty = file::tests(&p.player, Kind::Film, ctx.w.year).forty;
    let big = p.player.body.weight >= 250.0;
    match (k, s) {
        ("m100", Some(Sport::Track)) if !big => {
            Val::N(((10.62 + (forty - 4.40) * 2.6 + 0.12 * n(1)) * 100.0).round() / 100.0)
        }
        ("m200", Some(Sport::Track)) if !big => {
            let t100 = 10.62 + (forty - 4.40) * 2.6 + 0.12 * n(1);
            Val::N(((t100 * 2.04 - 0.1 + 0.2 * n(2)) * 100.0).round() / 100.0)
        }
        ("hurdles", Some(Sport::Track)) if !big && p.player.body.height >= 71.0 => Val::N(
            ((14.3 + (forty - 4.40) * 3.0 - 0.02 * (a.get(Attr::Agility) - 70.0) + 0.25 * n(3)) * 100.0).round()
                / 100.0,
        ),
        ("shot", Some(Sport::Track)) if big => {
            Val::N(((38.0 + 0.45 * (a.get(Attr::Strength) - 50.0) + 3.0 * n(4)) * 10.0).round() / 10.0)
        }
        ("hoops", Some(Sport::Basketball)) => Val::N(
            ((6.0 + 0.25 * (a.get(Attr::Agility) - 50.0) + 0.15 * (a.get(Attr::Jumping) - 50.0) + 4.0 * n(5))
                .clamp(1.0, 32.0)
                * 10.0)
                .round()
                / 10.0,
        ),
        ("wrestle", Some(Sport::Wrestling)) => Val::N(
            (18.0 + 0.4 * (a.get(Attr::Strength) - 50.0) + 0.2 * (a.get(Attr::Balance) - 50.0) + 6.0 * n(6))
                .clamp(2.0, 52.0)
                .round(),
        ),
        _ => Val::None,
    }
}

// --- Character, academics, age (FS11, FS12, FS16) -----------------------------------------

pub fn trouble(p: &Person) -> bool {
    p.incident
        || p.cal.trouble
        || p.player.history.iter().any(|h| h.events.has(Ev::Suspended) || h.events.has(Ev::Dismissed))
}

/// His GPA as your staff has it (FR3): the transcript's once your academic
/// staff has it, otherwise what his recruiting profile says (a little high).
pub fn gpa_shown(ctx: &Ctx, i: usize) -> f64 {
    let p = ctx.p(i);
    let real = file::gpa(p);
    if ctx.read(i).is_some_and(|r| r.transcript) {
        return (real * 100.0).round() / 100.0;
    }
    let up = 0.1 + 0.15 * key_uniform(0x6FA1, p.player.id as u64);
    ((real + up).min(4.0) * 10.0).round() / 10.0
}

/// Core courses behind the NCAA's 16 (FS12): a struggling student is often
/// one or two short until summer school.
pub fn core_behind(p: &Person) -> u32 {
    let gpa = file::gpa(p);
    if gpa >= 2.6 {
        return 0;
    }
    let u = key_uniform(0xC0E5, p.player.id as u64);
    if gpa < 2.2 && u < 0.7 {
        2
    } else if u < 0.45 {
        1
    } else {
        0
    }
}

/// The academic staff's risk word (FR3): 0 high, 1 some, 2 none.
pub fn risk(ctx: &Ctx, i: usize) -> usize {
    let p = ctx.p(i);
    let line = match ctx.program() {
        Some(j) => desk::qualifies(ctx.w, j, i),
        None => "Qualifier",
    };
    match line {
        "Qualifier" if core_behind(p) == 0 && file::gpa(p) >= 2.6 => 2,
        "Qualifier" => 1,
        _ => 0,
    }
}

/// His age on 1 September of his senior year.
pub fn age_for_class(p: &Person) -> Option<f64> {
    if p.born == afm_engine::calendar::born::UNKNOWN {
        return None;
    }
    let start = Date::ymd(p.class_of - 1, 9, 1);
    Some(((start.0 - p.born.0) as f64 / 365.25 * 10.0).round() / 10.0)
}

pub fn reclassified(p: &Person) -> bool {
    p.born != afm_engine::calendar::born::UNKNOWN && afm_engine::calendar::born::class_of_birth(p.born) != p.class_of
}

fn age_at_draft(ctx: &Ctx, i: usize) -> f64 {
    let p = ctx.p(i);
    let draft = ctx.dd.draft[0].date;
    if p.born == afm_engine::calendar::born::UNKNOWN {
        return p.player.age as f64;
    }
    ((draft.0 - p.born.0) as f64 / 365.25 * 10.0).round() / 10.0
}

fn early_entrant(ctx: &Ctx, i: usize) -> bool {
    let p = ctx.p(i);
    p.stage == Stage::Declared && p.college_seasons < 4 && !p.cal.graduated
}

// --- Frame (FS17) ---------------------------------------------------------------------------

/// Your staff's read of how much weight his frame can still carry: the
/// frame he grew into and the years he has to fill it, read with the
/// judge's eye (a seen kid only).
pub fn frame_read(ctx: &Ctx, i: usize) -> Option<f64> {
    let r = ctx.read(i)?;
    let p = ctx.p(i);
    let b = &p.player.body;
    let years = (21.0 - p.player.age as f64).max(0.0);
    let size = afm_engine::body::combine_size(p.player.position);
    let truth = b.frame_weight + 6.0 * years + 0.4 * (size.2 - b.frame_weight).clamp(-10.0, 30.0) - b.weight;
    let sd = (12.0 - 0.45 * r.judge as f64).max(3.0);
    let read = truth + sd * key_normal(readers::staff_key(ctx.staff) ^ 0xF4A3, p.player.id as u64);
    if sd > ctx.sure.max_sd() * 1.5 {
        return None;
    }
    Some(read.round())
}

pub fn frame_word(room: f64) -> usize {
    match room {
        r if r >= 30.0 => 3,
        r if r >= 15.0 => 2,
        r if r >= 5.0 => 1,
        _ => 0,
    }
}

// --- Recruitment (FR5-FR8, FR11-FR13, FS13, FS14) -------------------------------------------

/// Who leads his race: the school he committed or signed with, else the
/// school ahead as he feels it.
pub fn leader(ctx: &Ctx, i: usize) -> Option<usize> {
    let p = ctx.p(i);
    if let Some(s) = p.cal.signed {
        return Some(s.program);
    }
    if let Some(c) = p.cal.commit {
        return Some(c.0);
    }
    ctx.race(i).first().map(|r| r.0)
}

/// How firmly he is taken (index into `FIRM`).
pub fn firm(ctx: &Ctx, i: usize) -> usize {
    let p = ctx.p(i);
    if p.cal.signed.is_some() {
        return 3;
    }
    let Some(c) = p.cal.commit else { return 0 };
    let race = ctx.race(i);
    let theirs = race.iter().find(|r| r.0 == c.0).map_or(0.0, |r| r.1);
    let best_other = race.iter().filter(|r| r.0 != c.0).map(|r| r.1).fold(f64::MIN, f64::max);
    let hold = desk::hold(ctx.w, i);
    if best_other > theirs - hold { 1 } else { 2 }
}

/// The insiders' pick on him: the most confident, then the latest.
pub fn insider(ctx: &Ctx, i: usize) -> Option<services::Pick> {
    let id = ctx.p(i).player.id;
    ctx.w
        .scouting
        .rankings
        .picks
        .iter()
        .filter(|x| x.id == id)
        .max_by(|a, b| a.conf.cmp(&b.conf).then(a.date.cmp(&b.date)))
        .copied()
}

/// Your camp (FR13): 2 came, 1 invited (on your board, near enough, the
/// camp still ahead), 0 neither.
pub fn camp(ctx: &Ctx, i: usize) -> usize {
    if ctx.read(i).is_some_and(|r| r.camp) {
        return 2;
    }
    let p = ctx.p(i);
    let Some(_) = ctx.program() else { return 0 };
    let season = ctx.w.dates.season;
    let day = Date::nth_weekday(season, 6, afm_engine::calendar::Weekday::Sat, 3);
    let invited = ctx.w.today <= day
        && p.stage == Stage::HighSchool
        && ctx.miles(i) <= 400.0
        && (ctx.board.contains(&p.player.id) || ctx.read(i).is_some());
    usize::from(invited)
}

/// His public NIL value (FS13): what the deal trackers print, from his
/// stars and following, in the NIL era only.
pub fn nil_value(ctx: &Ctx, i: usize) -> Option<f64> {
    let p = ctx.p(i);
    if !afm_engine::world::ncaa::nil_era(ctx.w.year) {
        return None;
    }
    // The public valuations (On3's kind) price a recruit by his rating, his
    // position and his reach: about $3,000 at a .70 composite, $16,000 at
    // the 3-star line, $76,000 at the 4-star line and $370,000 at the
    // 5-star line, quarterbacks well above that and linemen below.
    let base = match ctx.ranks.comp.get(&p.player.id) {
        Some(c) => 3_000.0 * ((c.1 - 0.70).clamp(0.0, 0.30) * 17.0).exp(),
        None => [3_000.0, 3_000.0, 8_000.0, 30_000.0, 120_000.0, 500_000.0][rstars(p).min(5) as usize],
    };
    let pos = match p.player.position {
        Position::QB => 1.8,
        Position::WR | Position::RB | Position::EDGE => 1.1,
        Position::CB => 1.0,
        Position::OT => 0.85,
        Position::IOL => 0.75,
        Position::K | Position::P | Position::LS => 0.4,
        _ => 0.9,
    };
    let v = base * pos + 1.2 * p.followers as f64;
    Some((v / 1000.0).round().max(1.0) * 1000.0)
}

/// The coming draft's dates: this football year's until its first night,
/// then next year's. The world's calendar holds a year's draft, combine and
/// pro days at its start and turns the year only after the Super Bowl, so
/// from the spring's draft to then they would be last spring's.
pub fn draft_dates(w: &World) -> YearDates {
    if afm_engine::world::scouting::public::draft_year(w) == w.year {
        w.dates.clone()
    } else {
        YearDates::new(w.year + 1)
    }
}

/// The stars that stand for a kid: the kicking or snapping camps' for a
/// specialist, the services' composite for the rest.
pub fn rstars(p: &Person) -> u8 {
    if p.player.position.is_specialist() && p.camp_stars > 0 { p.camp_stars } else { p.stars }
}

/// The four Census regions.
pub fn region(code: &str) -> usize {
    match code {
        "CT" | "ME" | "MA" | "NH" | "RI" | "VT" | "NJ" | "NY" | "PA" => 0,
        "IL" | "IN" | "MI" | "OH" | "WI" | "IA" | "KS" | "MN" | "MO" | "NE" | "ND" | "SD" => 1,
        "AZ" | "CO" | "ID" | "MT" | "NV" | "NM" | "UT" | "WY" | "AK" | "CA" | "HI" | "OR" | "WA" => 3,
        _ => 2,
    }
}

/// Your coach who recruits his state (FR12).
pub fn area_coach(ctx: &Ctx, i: usize) -> Option<usize> {
    let j = ctx.program()?;
    afm_engine::world::scouting::dept::area_coach(ctx.w, j, ctx.p(i).home_state as u8)
}

/// How well your area recruiter knows him (FR12), from your staff's calls,
/// visits and camp with him.
pub fn bond(ctx: &Ctx, i: usize) -> &'static str {
    let n = ctx.read(i).map_or(0, |r| r.contact);
    match n {
        0 => "None yet",
        1..=2 => "New",
        3..=5 => "Building",
        _ => "Strong",
    }
}

/// His ties to your program (FR11).
pub fn ties(ctx: &Ctx, i: usize) -> Vec<String> {
    let w = ctx.w;
    let p = ctx.p(i);
    let Some(j) = ctx.program() else { return Vec::new() };
    let mut v = Vec::new();
    if p.family().older_brother_at == Some(j) {
        v.push("His brother plays for you".to_string());
    } else if let Some(b) = p.family().older_brother_at {
        v.push(format!("Brother at {}", w.program_name(b)));
    }
    match p.family().bloodline {
        afm_engine::world::background::Bloodline::NflPlayer => v.push("Father played in the NFL".into()),
        afm_engine::world::background::Bloodline::CollegePlayer => v.push("Father played in college".into()),
        afm_engine::world::background::Bloodline::None => {}
    }
    if p.childhood_team == Some(j) {
        v.push("Grew up a fan of yours".into());
    }
    if w.programs[j].hs_pipeline.contains_key(&p.hs) {
        v.push("His coach's school is your pipeline".into());
    }
    if matches!(p.stage, Stage::HighSchool | Stage::PrepSchool) && ctx.teammate_committed(i) {
        v.push("A teammate committed to you".into());
    }
    v
}

/// Places up (positive) or down since the services' update before.
pub fn movement(ctx: &Ctx, i: usize) -> Option<i64> {
    let id = ctx.p(i).player.id;
    let now = ctx.ranks.comp.get(&id)?.0 as i64;
    ctx.ranks.prev_date?;
    let before = ctx.ranks.prev.get(&id)?;
    Some(*before as i64 - now)
}

/// A position from its short name ("QB", "EDGE").
pub fn position(name: &str) -> Option<Position> {
    Position::ALL.into_iter().find(|p| p.name() == name)
}
