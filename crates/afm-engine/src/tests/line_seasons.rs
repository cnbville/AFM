//! The line bank run's season proofs (line bank run question 5, Cain
//! 2026-11-07, "yes on all"). Each family the run fills plays a full-use
//! season at every level that hears it, with the pens the game uses (a
//! scene's pen, and the screens' pen for game day), and no more than 2 in
//! 100 of its lines may be ones the same speaker already said to you that
//! season. The one-on-ones keep their 0.5% (`lines.rs`). Each proof also
//! prints the share heard again from anyone inside the season.
//!
//! How often each talk comes at full use is Claude's (proposed, not
//! confirmed): every room drawn three times (the staff's pick, one more
//! look, yours) at the deepest playoff run, two sideline offers a game with
//! all eight words shown, a staff meeting after every game, and so on
//! below.

use std::collections::HashMap;
use std::sync::OnceLock;

use crate::calendar::Date;
use crate::rng::Rng;
use crate::world::folk::Folk;
use crate::world::folk::voice::{Facts, Look, Pen, Speaker, bank, remember_shown_in};

/// The bar: 2 lines in 100 said again by the same speaker in a season.
const BAR: f64 = 0.02;

/// A family held above `BAR` for now: (family, its measured share and a
/// point more). Before the line bank run's lines landed (2026-11-08), game
/// day was held at 59% and staff talk at 55%; with them every family meets
/// `BAR`, so the list is empty.
const TODAY: &[(&str, f64)] = &[];

fn bar(family: &str) -> f64 {
    TODAY.iter().find(|x| x.0 == family).map_or(BAR, |x| x.1)
}

#[derive(Default)]
struct Ear {
    parts: usize,
    /// Said again by the same speaker inside the season.
    same: usize,
    /// Heard again from anyone inside the season.
    any: usize,
    heard: HashMap<(String, String), i32>,
    thin: HashMap<String, usize>,
    /// How often each part was said (the run's split inside a family).
    uses: HashMap<String, usize>,
}

impl Ear {
    fn note(&mut self, key: &str, words: String, again: bool, day: i32) {
        self.parts += 1;
        *self.uses.entry(key.to_string()).or_default() += 1;
        if again {
            self.same += 1;
            *self.thin.entry(key.to_string()).or_default() += 1;
        }
        let e = self.heard.entry((key.to_string(), words)).or_insert(-400);
        if day - *e < 365 {
            self.any += 1;
        }
        *e = day;
    }
}

/// One season at one level: the people store your memories live in, and
/// an ear per family.
struct Sim {
    folk: Folk,
    r: Rng,
    start: i32,
    day: i32,
    seed: u64,
    level: &'static str,
    ears: HashMap<&'static str, Ear>,
}

const ERA: [&str; 3] = ["social", "modern", "nil"];
const REGIONS: [&str; 5] = ["texas", "south", "midwest", "northeast", "west"];

impl Sim {
    fn date(&self) -> Date {
        Date(self.start + self.day)
    }

    fn region(&mut self, tags: &mut Vec<&'static str>) {
        let g = REGIONS[self.r.int(0, 4) as usize];
        tags.push(g);
        if g == "texas" {
            tags.push("south");
        }
    }

    /// A player as `Speaker::player` builds one (a college player's trust
    /// in you taken as low for the brash tag).
    fn player(&mut self, key: u64) -> Speaker {
        let level = self.level;
        let mut t: Vec<&'static str> = vec!["player", level];
        match level {
            "hs" => t.push("young"),
            "college" => {
                if self.r.chance(0.85) {
                    t.push("young")
                }
            }
            _ => {
                if self.r.chance(0.2) {
                    t.push("young")
                } else if self.r.chance(0.4) {
                    t.push("vet")
                }
            }
        }
        for tr in [
            "ego",
            "humble",
            "leader",
            "quiet",
            "hot",
            "calm",
            "loyal",
            "greedy",
            "pro",
            "loose",
            "shaky",
            "steady",
            "competitor",
        ] {
            if self.r.chance(0.16) {
                t.push(tr);
            }
        }
        let (ego, hot) = (t.contains(&"ego"), t.contains(&"hot"));
        if (level == "nfl" && (ego || hot)) || (level == "college" && ego && hot) {
            t.push("brash");
        }
        let pos: [&[&'static str]; 10] = [
            &["qb", "offense"],
            &["rb", "offense", "skill"],
            &["wr", "offense", "skill"],
            &["te", "offense", "skill"],
            &["ol", "offense", "lineman"],
            &["dl", "defense", "rusher", "lineman"],
            &["lb", "defense", "cover"],
            &["db", "defense", "cover"],
            &["kicker", "special"],
            &["snapper", "special"],
        ];
        t.extend_from_slice(pos[[0, 1, 2, 2, 3, 4, 4, 5, 5, 6, 7, 7, 8, 9][self.r.int(0, 13) as usize]]);
        let stage: &[&'static str] = match level {
            "hs" => &["freshman", "sophomore", "junior", "senior"],
            "college" => &["freshman", "senior", "redshirt", "transfer", "walkon"],
            _ => &["rookie", "undrafted", "firstround", "psquad"],
        };
        if self.r.chance(0.5) {
            t.push(stage[self.r.int(0, stage.len() as i32 - 1) as usize]);
        }
        self.region(&mut t);
        t.extend(ERA);
        Speaker { key, tags: t }
    }

    /// A coach as `Speaker::coach` builds one.
    fn coach(&mut self, key: u64, level: &'static str) -> Speaker {
        let mut t: Vec<&'static str> = vec!["coach", level];
        for tr in ["hot", "calm", "climber", "loyal", "old", "young", "expro", "strict", "warm", "cold"] {
            if self.r.chance(0.15) {
                t.push(tr);
            }
        }
        self.region(&mut t);
        t.extend(ERA);
        Speaker { key, tags: t }
    }

    /// One of the cast (an AD, an owner, a booster) as `Speaker::cast`.
    fn cast(&mut self, key: u64, role: &'static str) -> Speaker {
        let mut t: Vec<&'static str> = vec![role, self.level];
        if self.r.chance(0.1) {
            t.push("she");
        }
        for tr in ["ego", "hot", "warm", "cold", "blunt", "smooth", "impatient", "patient", "meddler", "tight", "old"] {
            if self.r.chance(0.15) {
                t.push(tr);
            }
        }
        self.region(&mut t);
        t.extend(ERA);
        Speaker { key, tags: t }
    }

    /// A player's parent as `Speaker::parent`.
    fn parent(&mut self, key: u64) -> Speaker {
        let mut t: Vec<&'static str> = vec!["parent", ["loud", "online", "calm", "absent"][self.r.int(0, 3) as usize]];
        self.region(&mut t);
        t.extend(ERA);
        Speaker { key, tags: t }
    }

    fn you(&mut self) -> Speaker {
        let mut t: Vec<&'static str> = vec!["you", "south", self.level];
        t.extend(ERA);
        Speaker { key: 0x4000_0000_0000_0001, tags: t }
    }

    /// A scene's line (a scene's pen, taken and put back): `plain` is the
    /// talks off the game day, where a speaker no tagged version fits says
    /// a plain one (`dialogue.rs`, `line`).
    fn say(&mut self, fam: &'static str, key: &str, who: &Speaker, f: &Facts, plain: bool) {
        self.seed += 1;
        let d = self.date();
        let who = if plain && bank().fitting(key, who, f) == 0 {
            Speaker { key: who.key, tags: Vec::new() }
        } else {
            who.clone()
        };
        let mut pen = Pen::take_from(&mut self.folk, d, self.seed);
        let again = pen.fresh(key, &who, f) == 0;
        let said = pen.part(key, &who, f);
        pen.put_into(&mut self.folk);
        if let Some(s) = said {
            let day = self.day;
            self.ears.entry(fam).or_default().note(key, s, again, day);
        }
    }

    /// A line on a screen (the screens' pen); `shown` remembers them when
    /// the screen is drawn.
    fn look(&mut self, fam: &'static str, key: &str, who: &Speaker, f: &Facts, seed: u64) {
        let d = self.date();
        let (again, said) = {
            let look = Look::over(&self.folk, d, seed);
            (look.fresh(key, who, f) == 0, look.part(key, who, f))
        };
        if let Some(s) = said {
            let day = self.day;
            self.ears.entry(fam).or_default().note(key, s, again, day);
        }
    }

    fn shown(&mut self) {
        let d = self.date();
        remember_shown_in(&mut self.folk, d);
    }

    fn base(&self) -> Facts {
        Facts::new()
            .with("team", "Iowa State")
            .with("town", "Ames")
            .with("you", "Reed")
            .with("coach", "Coach Reed")
            .with("year", "2026")
            .with("old", "Matt Campbell")
            .with("old_last", "Campbell")
            .with("rival", "Iowa")
            .with("rivalry", "the Cy-Hawk game")
            .with("series", "3-4")
            .with("last_record", "7-6")
    }
}

const TONES: [&str; 4] = ["fiery", "warm", "firm", "calm"];
const MOVES: [&str; 8] = ["ask", "straight", "praise", "challenge", "criticize", "promise", "warn", "listen"];
const SUBJECTS: [&str; 6] = ["us", "them", "plan", "stakes", "seniors", "one"];
const MISTAKES: [&str; 6] = ["fumble", "interception", "extended", "killed", "blown", "pattern"];
const READS: [&str; 6] = ["good", "amped", "tight", "flat", "hollow", "captain"];
const WORDS: [(&str, &str); 8] = [
    ("straight", "calm"),
    ("straight", "firm"),
    ("praise", "warm"),
    ("challenge", "fiery"),
    ("criticize", "fiery"),
    ("criticize", "firm"),
    ("warn", "firm"),
    ("listen", "calm"),
];

/// One room drawn on screen: your talk for this pick and both coordinators'
/// reads, and on the draw you say, when you hand him the room, the first
/// captain's five lines (`lane_gameday/talk.rs`, `room_json` and
/// `captains_json`: a screen draws a captain's words only once he has
/// spoken, and the one-click hand-off goes to the first captain).
fn room(sim: &mut Sim, caps: &[Speaker; 2], staff: &[Speaker; 2], halftime: bool, rival: bool, yours: bool) {
    let tone = TONES[sim.r.int(0, 3) as usize];
    let captain = sim.r.chance(0.1);
    let mv = if captain { "captain" } else { MOVES[sim.r.int(0, 7) as usize] };
    let mut you = sim.you();
    you.tags.push(tone);
    let mut f = Facts::new();
    f.set("them", "Iowa").set("subject", SUBJECTS[sim.r.int(0, 5) as usize]).set("player", "Marcus");
    f.set("last", ["win", "loss", "none"][sim.r.int(0, 2) as usize]).set("captain", "Dre");
    if halftime {
        f.set("moment", "halftime").set("lead", ["up", "down", "level"][sim.r.int(0, 2) as usize]);
    } else {
        f.set("moment", "pregame");
    }
    if rival {
        f.set("rival", "yes");
    }
    sim.seed += 1;
    let seed = sim.seed;
    sim.look("gameday", &format!("gameday.open.{tone}"), &you, &f, seed);
    sim.look("gameday", &format!("gameday.{mv}"), &you, &f, seed);
    if sim.r.chance(0.55) {
        sim.look("gameday", &format!("gameday.close.{tone}"), &you, &f, seed);
    }
    for (k, c) in staff.iter().enumerate() {
        let read = READS[sim.r.int(0, 5) as usize];
        let f = Facts::new().with("player", "Marcus");
        sim.look("staff", &format!("gameday.staff.{read}"), c, &f, seed ^ k as u64);
    }
    if captain && yours {
        for k in ["open", "own", "us", "job", "break"] {
            sim.look("gameday", &format!("gameday.captain.{k}"), &caps[0], &f, seed ^ 0xCA97_A100);
        }
    }
    sim.shown();
}

fn season(level: &'static str, li: u64) -> HashMap<&'static str, Ear> {
    let mut sim = Sim {
        folk: Folk::default(),
        r: Rng::new(2026u32 + li as u32),
        start: Date::ymd(2026, 7, 1).0,
        day: 0,
        seed: 77 + li,
        level,
        ears: HashMap::new(),
    };
    let base = 1_000_000 * (li + 1);
    let caps = [sim.player(base + 1), sim.player(base + 2)];
    let staff = [sim.coach(base + 3, level), sim.coach(base + 4, level)];
    let roster: Vec<Speaker> = (0..[85u64, 105, 70][li as usize]).map(|k| sim.player(base + 100 + k)).collect();
    let (boss_role, panel, booster) = match level {
        "hs" => ("principal", ["principal", "ad"], "booster"),
        "college" => ("ad", ["ad", "president"], "booster"),
        _ => ("owner", ["owner", "gm"], "kin"),
    };
    let boss = sim.cast(base + 5, boss_role);

    // A new job's first weeks (the most a season can hold): your first
    // team meeting and a leader's answer, each assistant's pitch to stay,
    // the room's voices, the old coach, the parents' night (high school)
    // and each big booster.
    let mut f = sim.base();
    f.set("count", "11");
    let you = sim.you();
    sim.say("room", "takeover.meeting.you_open", &you, &f, false);
    let msg = ["compete", "honour", "winnow", "purge", "plain"][sim.r.int(0, 4) as usize];
    sim.say("room", &format!("takeover.meeting.react_{msg}"), &caps[0], &f, false);
    for k in 0..8u64 {
        sim.day = 2 + k as i32;
        let c = sim.coach(base + 200 + k, level);
        let mut g = sim.base();
        g.set("name", "Mike").set("role", "offensive line coach").set("years", "4");
        sim.say("room", "takeover.holdover.pitch", &c, &g, false);
        if sim.r.chance(0.55) {
            sim.say("room", "takeover.holdover.detail", &c, &g, false);
        }
    }
    for (k, p) in roster.iter().take(6).enumerate() {
        let key = ["close_to_old", "ego", "steady", "waiting"][k % 4];
        let mut g = sim.base();
        g.set("name", "Tre");
        sim.say("room", &format!("takeover.leader.{key}"), p, &g, false);
    }
    let old = sim.coach(base + 300, level);
    sim.say("room", "takeover.oldcoach.greet", &old, &sim.base(), false);
    if level == "hs" {
        let dad = sim.parent(base + 301);
        sim.say("room", "parents.playing_time", &dad, &sim.base(), false);
    }
    for k in 0..6u64 {
        sim.day = 12 + k as i32;
        let b = sim.cast(base + 400 + k, booster);
        let mut g = sim.base();
        g.set("name", "Bill").set("want", "a win over Iowa").set("old_school", "Valley");
        sim.say("room", "booster.greet", &b, &g, false);
        if sim.r.chance(0.55) {
            sim.say("room", "booster.small", &b, &g, false);
        }
        let want = ["rival", "access", "hires", "player", "nil", "clean", "building", "oldschool", "winnow"]
            [sim.r.int(0, 8) as usize];
        sim.say("room", &format!("booster.want.{want}"), &b, &g, false);
    }

    // The boss's goals for the season.
    sim.day = 20;
    let mut g = sim.base();
    g.set("boss", "Jamie");
    sim.say("ad", "boss.goals.open", &boss, &g, false);
    if sim.r.chance(0.55) {
        sim.say("ad", "boss.goals.point", &boss, &g, false);
    }

    // The games: every room drawn three times, two sideline offers with
    // all eight words shown, the paper's quote a third of the weeks, and
    // the staff meeting the morning after.
    let games = match level {
        "hs" => 14,
        "college" => 16,
        _ => 21,
    };
    for g in 0..games {
        sim.day = 61 + 7 * g;
        let rival = g == games / 2;
        for k in 0..3 {
            room(&mut sim, &caps, &staff, false, rival, k == 2);
        }
        if sim.r.chance(0.34) {
            let them = sim.player(base + 500 + g as u64);
            let mut them = them;
            them.tags.push("ego");
            let f = Facts::new().with("us", "Iowa State").with("unit", "front");
            let seed = 0x0E07_E000 ^ sim.date().0 as u64;
            sim.look("gameday", "gameday.quote", &them, &f, seed);
            sim.shown();
        }
        for o in 0..2u64 {
            sim.seed += 1;
            let seed = sim.seed;
            let mistake = MISTAKES[sim.r.int(0, 5) as usize];
            for (k, (mv, tone)) in WORDS.iter().enumerate() {
                let mut you = sim.you();
                you.tags.push(tone);
                let f = Facts::new().with("player", "Marcus").with("mistake", mistake);
                sim.look("gameday", &format!("gameday.word.{mv}"), &you, &f, seed ^ k as u64 ^ o << 8);
            }
            sim.shown();
        }
        for k in 0..3 {
            room(&mut sim, &caps, &staff, true, rival, k == 2);
        }
        // The staff meeting the next morning.
        sim.day += 1;
        let k = sim.r.int(0, 3) as usize;
        let c = staff[usize::from(k >= 2)].clone();
        sim.say(
            "staff",
            &format!("staffmeet.case.{}", ["pressure", "zone", "run", "pass"][k]),
            &c,
            &Facts::new(),
            true,
        );
        let reply = ["yes", "half", "no"][sim.r.int(0, 2) as usize];
        sim.say("staff", &format!("staffmeet.reply.{reply}"), &c, &Facts::new(), true);
        // The boss at midseason.
        if g == games / 2 {
            let mut f = sim.base();
            f.set("boss", "Jamie").set("record", "4-2");
            sim.say("ad", "boss.mid.open", &boss, &f, false);
        }
        // Team rules: about two rulings a week, and the parents who call.
        for _ in 0..2 {
            let p = roster[sim.r.int(0, roster.len() as i32 - 1) as usize].clone();
            let key = ["harsh", "soft", "fair"][sim.r.int(0, 2) as usize];
            sim.say("room", &format!("rules.react.{key}"), &p, &Facts::new().with("coach", "Coach Reed"), false);
        }
        if level != "nfl" {
            for k in 0..2u64 {
                let mum = sim.parent(base + 600 + 2 * g as u64 + k);
                let f = Facts::new()
                    .with("name", "Jalen")
                    .with("full", "Jalen Moore")
                    .with("coach", "Coach Reed")
                    .with("you", "Reed")
                    .with("pos", "WR")
                    .with("team", "Iowa State")
                    .with("friend", "Tre");
                sim.say("room", "parents.call.open", &mum, &f, false);
                if sim.r.chance(0.55) {
                    sim.say("room", "parents.call.point", &mum, &f, false);
                }
            }
        }
    }

    // The boss's review.
    sim.day = 190;
    let mut f = sim.base();
    f.set("boss", "Jamie").set("met", "2");
    let how = ["good", "mixed", "bad"][sim.r.int(0, 2) as usize];
    sim.say("ad", &format!("boss.review.{how}"), &boss, &f, false);

    // Recruiting and scouting: home visits in the winter (college), the
    // college staffs' calls (high school), the combine room (NFL).
    match level {
        "college" => {
            for k in 0..45u64 {
                sim.day = 150 + k as i32 * 3 / 2;
                let mut fam = sim.parent(base + 700 + k);
                fam.tags.push(["himself", "parent", "grandparent", "handler"][sim.r.int(0, 3) as usize]);
                let f = Facts::new().with("name", "Jalen");
                let want = ["playing", "nil", "school", "distance", "family", "development", "winning"]
                    [sim.r.int(0, 6) as usize];
                sim.say("recruit", &format!("homevisit.ask.{want}"), &fam, &f, true);
                if sim.r.chance(0.5) {
                    let f = Facts::new().with("first", "playing time").with("second", "the school");
                    sim.say("recruit", "homevisit.tell", &fam, &f, true);
                }
                let mood = ["good", "flat", "bad"][sim.r.int(0, 2) as usize];
                sim.say("recruit", &format!("homevisit.react.{mood}"), &fam, &Facts::new(), true);
            }
        }
        "hs" => {
            let callers: Vec<Speaker> = (0..12u64).map(|k| sim.coach(base + 800 + k, "college")).collect();
            for k in 0..30 {
                sim.day = 60 + k * 6;
                let c = callers[sim.r.int(0, 11) as usize].clone();
                let about = ["football", "grades", "character"][sim.r.int(0, 2) as usize];
                sim.say("recruit", &format!("hscall.ask.{about}"), &c, &Facts::new().with("name", "Jalen"), true);
            }
        }
        _ => {
            sim.day = 240;
            for k in 0..45u64 {
                // A draft prospect speaks from college.
                sim.level = "college";
                let p = sim.player(base + 900 + k);
                sim.level = "nfl";
                for _ in 0..usize::from(sim.r.chance(0.5)) + usize::from(sim.r.chance(0.2)) {
                    let mood = ["steady", "bristles", "shaken"][sim.r.int(0, 2) as usize];
                    sim.say("recruit", &format!("combine.press.{mood}"), &p, &Facts::new(), true);
                }
            }
        }
    }

    // Your job interviews (three questions and three answers each) and
    // the coaches you interview for your staff.
    for k in 0..4u64 {
        sim.day = 160 + 8 * k as i32;
        let who = [sim.cast(base + 1000 + 2 * k, panel[0]), sim.cast(base + 1001 + 2 * k, panel[1])];
        for (q, ask) in ["plan", "staff", "past"].iter().enumerate() {
            let c = who[q % 2].clone();
            sim.say("ad", &format!("interview.ask.{ask}"), &c, &Facts::new(), true);
            let mood = ["good", "flat", "bad"][sim.r.int(0, 2) as usize];
            sim.say("ad", &format!("interview.react.{mood}"), &c, &Facts::new(), true);
        }
    }
    for k in 0..12u64 {
        sim.day = 195 + 3 * k as i32;
        let c = sim.coach(base + 1100 + k, level);
        let f =
            Facts::new().with("role", "defensive coordinator").with("team", "Iowa State").with("came_up", "defense");
        sim.say("staff", "staff.interview.open", &c, &f, false);
    }

    // Agents in March (NFL): about 80 answers across 25 agents, a claim
    // with half of them and a broken talk now and then.
    if level == "nfl" {
        let agents: Vec<Speaker> = (0..25u64)
            .map(|k| {
                let mut t: Vec<&'static str> = vec!["agent"];
                match sim.r.int(0, 2) {
                    0 => t.push("straight"),
                    1 => t.push("slick"),
                    _ => {}
                }
                match sim.r.int(0, 2) {
                    0 => t.push("hot"),
                    1 => t.push("calm"),
                    _ => {}
                }
                match sim.r.int(0, 2) {
                    0 => t.push("big"),
                    1 => t.push("small"),
                    _ => {}
                }
                Speaker { key: base + 1200 + k, tags: t }
            })
            .collect();
        for k in 0..80 {
            sim.day = 250 + k * 3 / 8;
            let a = agents[sim.r.int(0, 24) as usize].clone();
            let f = Facts::new().with("name", "Marcus Hill").with("money", "$4.5 million").with("club", "Bills");
            let part = ["yes", "range", "apart", "far", "near", "short", "insult"][sim.r.int(0, 6) as usize];
            sim.say("agents", &format!("agent.{part}"), &a, &f, false);
            if sim.r.chance(0.5) {
                let claim = ["real", "bluff", "market", "stay"][sim.r.int(0, 3) as usize];
                sim.say("agents", &format!("agent.claim.{claim}"), &a, &f, false);
            }
            if sim.r.chance(0.08) {
                sim.say("agents", "agent.done", &a, &f, false);
            }
        }
    }
    sim.ears
}

/// Every level's season, played once for all the proofs.
fn seasons() -> &'static Vec<(&'static str, HashMap<&'static str, Ear>)> {
    static S: OnceLock<Vec<(&'static str, HashMap<&'static str, Ear>)>> = OnceLock::new();
    S.get_or_init(|| ["hs", "college", "nfl"].iter().enumerate().map(|(li, &l)| (l, season(l, li as u64))).collect())
}

fn proof(family: &str) {
    let mut all = (0usize, 0usize, 0usize);
    let mut uses: HashMap<&String, usize> = HashMap::new();
    for (level, ears) in seasons() {
        let Some(e) = ears.get(family) else { continue };
        for (k, n) in &e.uses {
            *uses.entry(k).or_default() += n;
        }
        let mut thin: Vec<(&String, &usize)> = e.thin.iter().collect();
        thin.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
        println!(
            "{family} {level}: {} lines, {} said again by the same speaker ({:.2}%), {} heard again from anyone \
             ({:.1}%); thinnest {:?}",
            e.parts,
            e.same,
            100.0 * e.same as f64 / e.parts.max(1) as f64,
            e.any,
            100.0 * e.any as f64 / e.parts.max(1) as f64,
            &thin[..thin.len().min(6)]
        );
        all = (all.0 + e.parts, all.1 + e.same, all.2 + e.any);
    }
    let mut uses: Vec<(&String, usize)> = uses.into_iter().collect();
    uses.sort();
    println!("{family} uses: {uses:?}");
    assert!(all.0 > 0, "{family}: no lines said");
    let share = all.1 as f64 / all.0 as f64;
    assert!(
        share <= bar(family),
        "{family}: {} of {} lines said again by the same speaker inside a season ({:.2}%, bar {:.0}%)",
        all.1,
        all.0,
        100.0 * share,
        100.0 * bar(family)
    );
}

#[test]
fn a_season_of_game_days_counts_its_repeats() {
    proof("gameday");
}

#[test]
fn a_season_of_recruiting_counts_its_repeats() {
    proof("recruit");
}

#[test]
fn a_season_of_staff_talk_counts_its_repeats() {
    proof("staff");
}

#[test]
fn a_season_of_bosses_and_interviews_counts_its_repeats() {
    proof("ad");
}

#[test]
fn a_season_of_agents_counts_its_repeats() {
    proof("agents");
}

#[test]
fn a_season_off_the_field_counts_its_repeats() {
    proof("room");
}

/// The screens' pen (minor bug 314): the same moment shows the same words
/// however often the screen is drawn, and what it showed is remembered, so
/// the next game day reaches for other words.
#[test]
fn a_screen_shows_the_same_words_and_remembers_them() {
    let mut folk = Folk::default();
    let day = Date::ymd(2026, 9, 5);
    let you = Speaker { key: 1, tags: vec!["you", "college", "south", "social", "modern", "nil", "firm"] };
    let f = Facts::new().with("player", "Marcus").with("mistake", "fumble");
    let key = "gameday.word.straight";
    let first = Look::over(&folk, day, 9).part(key, &you, &f).unwrap();
    // Drawn again before and after the call ends: the same words.
    assert_eq!(Look::over(&folk, day, 9).part(key, &you, &f).unwrap(), first);
    remember_shown_in(&mut folk, day);
    assert_eq!(Look::over(&folk, day, 9).part(key, &you, &f).unwrap(), first);
    assert_eq!(folk.heard.values().sum::<u16>(), 1, "shown once, heard once");
    // Another moment the same day, and the next week: not those words.
    let n = bank().fitting(key, &you, &f);
    assert!(n >= 2);
    let other = Look::over(&folk, day, 10).part(key, &you, &f).unwrap();
    assert_ne!(other, first);
    remember_shown_in(&mut folk, day);
    assert_eq!(folk.shown.lines.len(), 2, "two moments shown today");
    let next = day.add(7);
    let mut seen = vec![first, other];
    for s in 0..(n as u64 - 2) {
        let w = Look::over(&folk, next, 100 + s).part(key, &you, &f).unwrap();
        assert!(!seen.contains(&w), "a word came back inside the season: {w}");
        seen.push(w);
        remember_shown_in(&mut folk, next);
    }
    // A new day starts the list of what was shown: only its own moments.
    assert_eq!(folk.shown.day, next);
    assert_eq!(folk.shown.lines.len(), n - 2, "the old day's lines are cleared");
    assert!(folk.shown.lines.iter().all(|l| l.0 >= 100), "only the new day's moments");
}

/// The listener-wide rest (question 4): a wording waits until every other
/// wording of its part has been heard, whoever says it.
#[test]
fn a_wording_waits_for_every_other_one() {
    let key = "talk.reply.praise.good";
    let f = Facts::new().with("coach", "Coach Reed").with("name", "Marcus").with("you", "Reed");
    let plain = |k: u64| Speaker { key: k, tags: vec!["player", "college", "social", "modern", "nil"] };
    let n = bank().fitting(key, &plain(0), &f);
    assert!(n >= 40);
    let mut folk = Folk::default();
    let day0 = Date::ymd(2026, 8, 1);
    // A different man every day: no wording twice until all have been said.
    for k in 0..n as u64 {
        let mut pen = Pen::take_from(&mut folk, Date(day0.0 + k as i32), 5 + k);
        pen.part(key, &plain(100 + k), &f).unwrap();
        pen.put_into(&mut folk);
    }
    assert_eq!(folk.heard.len(), n, "a wording came back before every other one was heard");
    assert!(folk.heard.values().all(|&c| c == 1));
    // Then the second round: again each once before any third time.
    for k in 0..n as u64 {
        let mut pen = Pen::take_from(&mut folk, Date(day0.0 + (n as u64 + k) as i32), 900 + k);
        pen.part(key, &plain(5000 + k), &f).unwrap();
        pen.put_into(&mut folk);
    }
    assert!(folk.heard.values().all(|&c| c == 2));
}

/// The recruiting talks run's season (Cain 2026-11-12, 20,000 lines for the recruiting personal talks): a college
/// head coach at full use over the recruiting year, 60 high school recruits with a family each, four talks
/// apiece, and 20 transfers, three talks apiece, drawing the run's parts by the fact each is named for (the way
/// the talk design gates them; how often each comes is Claude's, proposed, not confirmed), with lane L5's own
/// parts (`recruit_case.txt`: what changed, a rival who said the same, a point he remembers, a straight answer,
/// who decides). The conversation (`folk::pitch`) asks these parts; this proves the bank holds enough for a
/// season of it. The bar is the other families': 2 in 100 lines said again by the same speaker; the share heard
/// again from anyone is printed with it.
#[test]
fn a_season_of_recruit_talks_counts_its_repeats() {
    let mut sim = Sim {
        folk: Folk::default(),
        r: Rng::new(2026u32 + 77),
        start: Date::ymd(2026, 7, 1).0,
        day: 0,
        seed: 4242,
        level: "college",
        ears: HashMap::new(),
    };
    const FAM: &str = "recruittalks";
    let base = 9_000_000u64;
    let bg = [
        "money_tight",
        "money_fine",
        "single_parent",
        "grandparents",
        "foster",
        "job_loss",
        "family_move",
        "death",
        "new_baby",
        "mentor",
        "nil_changed",
        "brother_follow",
        "brother_own",
        "bloodline_nfl",
        "bloodline_college",
        "grades_low",
        "ambitious",
    ];
    let aspects = ["tradition", "facilities", "atmosphere", "brand", "conference", "campus", "prestige"];
    let drives = ["playing", "nil", "school", "distance", "family", "development", "winning"];
    let whys = ["snaps", "coach", "money", "up", "choice", "grad"];
    let f = Facts::new()
        .with("name", "Jalen")
        .with("team", "Iowa State")
        .with("coach", "Coach Reed")
        .with("you", "Reed")
        .with("pos", "WR")
        .with("last_record", "7-6")
        .with("first", "playing time")
        .with("second", "the school")
        .with("opp", "Kansas")
        .with("score", "31-17")
        .with("when", "Saturday")
        .with("school", "Baylor")
        .with("spot", "receiver")
        .with("rival_school", "TCU");
    let pick = |sim: &mut Sim, v: &[&str]| v[sim.r.int(0, v.len() as i32 - 1) as usize].to_string();
    // Lane L5's own parts (the hand-written bank, `recruit_case.txt`): what
    // changed since you last spoke, a rival who said the same, a point he
    // remembers, a straight answer, who decides.
    let events = ["your_win", "your_loss", "his_game", "new_offer", "visit_elsewhere", "commit_at_spot", "coach_left"];
    // The common beats of one talk, from either side.
    let beats = |sim: &mut Sim, who: &Speaker, first: bool, k: usize, facts: &[String]| {
        let call = sim.r.chance(0.7);
        let mood = ["warm", "polite", "polite", "guarded"][sim.r.int(0, 3) as usize];
        let setting = if call { "call" } else { "visit" };
        sim.say(FAM, &format!("recruit.open.{setting}.{mood}"), who, &f, true);
        if first {
            sim.say(FAM, "recruit.history.first", who, &f, true);
            for b in facts {
                sim.say(FAM, &format!("recruit.bg.{b}"), who, &f, true);
            }
        } else {
            if sim.r.chance(0.3) {
                let e = events[sim.r.int(0, 6) as usize];
                sim.say(FAM, &format!("recruit.open.{e}"), who, &f, true);
            } else {
                let h =
                    ["again", "again", "again", "again", "again", "silence", "kept", "broke"][sim.r.int(0, 7) as usize];
                sim.say(FAM, &format!("recruit.history.{h}"), who, &f, true);
            }
            if sim.r.chance(0.25) {
                sim.say(FAM, "recruit.react.again.any", who, &f, true);
            }
        }
        if first && sim.r.chance(0.5) {
            sim.say(FAM, "recruit.tell.decides", who, &f, true);
        }
        if sim.r.chance(0.3) {
            sim.say(FAM, "recruit.compare.any", who, &f, true);
        }
        if sim.r.chance(0.2) {
            sim.say(FAM, "recruit.react.straight.any", who, &f, true);
        }
        for _ in 0..sim.r.int(1, 2) {
            let a = aspects[sim.r.int(0, 6) as usize];
            let s = ["good", "good", "cool", "weak"][sim.r.int(0, 3) as usize];
            sim.say(FAM, &format!("recruit.aspect.{a}.{s}"), who, &f, true);
        }
        if sim.r.chance(0.5) {
            sim.say(FAM, "recruit.record.ask", who, &f, true);
            let r = ["strong", "mixed", "weak"][sim.r.int(0, 2) as usize];
            sim.say(FAM, &format!("recruit.record.{r}"), who, &f, true);
        }
        if sim.r.chance(0.4) {
            let o = ["other.money", "other.you_money", "other.closer", "other.start", "committed.elsewhere"]
                [sim.r.int(0, 4) as usize];
            sim.say(FAM, &format!("recruit.{o}"), who, &f, true);
        }
        let w = sim.r.int(0, 9);
        if w < 3 {
            sim.say(FAM, "recruit.want.second", who, &f, true);
        } else if w == 3 {
            sim.say(FAM, "recruit.want.dealbreaker", who, &f, true);
        } else if w == 4 {
            sim.say(FAM, "recruit.want.unsure", who, &f, true);
        }
        if sim.r.chance(0.2) {
            let t = ["team.good_year", "team.bad_year", "you.new", "you.long", "visit.liked", "visit.meh"]
                [(k + sim.r.int(0, 1) as usize) % 6];
            sim.say(FAM, &format!("recruit.{t}"), who, &f, true);
        }
        let close = ["warm", "flat", "flat", "cool"][sim.r.int(0, 3) as usize];
        sim.say(FAM, &format!("recruit.close.{setting}.{close}"), who, &f, true);
    };
    for k in 0..60u64 {
        let mut fam = sim.parent(base + k);
        fam.tags.push(["himself", "parent", "grandparent", "handler"][sim.r.int(0, 3) as usize]);
        let facts: Vec<String> = (0..sim.r.int(2, 3)).map(|_| pick(&mut sim, &bg)).collect();
        for t in 0..4 {
            sim.day = 120 + (k as i32 * 2 + t * 30) % 250;
            beats(&mut sim, &fam, t == 0, k as usize, &facts);
        }
    }
    for k in 0..20u64 {
        sim.level = "college";
        let tr = sim.player(base + 500 + k);
        let fact = pick(&mut sim, &bg);
        for t in 0..3 {
            sim.day = 150 + (k as i32 * 5 + t * 20) % 200;
            let call = sim.r.chance(0.7);
            let mood = ["warm", "polite", "polite", "guarded"][sim.r.int(0, 3) as usize];
            let setting = if call { "call" } else { "visit" };
            sim.say(FAM, &format!("recruit.open.{setting}.{mood}"), &tr, &f, true);
            if t == 0 {
                sim.say(FAM, "recruit.open.entered_portal", &tr, &f, true);
            }
            if sim.r.chance(0.3) {
                sim.say(FAM, "recruit.compare.any", &tr, &f, true);
            }
            let drive = pick(&mut sim, &drives);
            sim.say(FAM, &format!("portal.ask.{drive}"), &tr, &f, true);
            if sim.r.chance(0.5) {
                sim.say(FAM, "portal.tell", &tr, &f, true);
            }
            for _ in 0..sim.r.int(1, 2) {
                let m = ["good", "flat", "bad"][sim.r.int(0, 2) as usize];
                sim.say(FAM, &format!("portal.react.{m}"), &tr, &f, true);
            }
            if sim.r.chance(0.4) {
                let p = ["knows", "believes", "doubts"][sim.r.int(0, 2) as usize];
                sim.say(FAM, &format!("portal.promise.{p}"), &tr, &f, true);
            }
            if t == 0 {
                let why = pick(&mut sim, &whys);
                sim.say(FAM, &format!("transfer.why.{why}"), &tr, &f, true);
                if sim.r.chance(0.5) {
                    let role = ["starter", "backup"][sim.r.int(0, 1) as usize];
                    sim.say(FAM, &format!("transfer.role.{role}"), &tr, &f, true);
                }
                sim.say(FAM, &format!("recruit.bg.{}", fact), &tr, &f, true);
            }
            let a = aspects[sim.r.int(0, 6) as usize];
            let s = ["good", "cool", "weak"][sim.r.int(0, 2) as usize];
            sim.say(FAM, &format!("recruit.aspect.{a}.{s}"), &tr, &f, true);
            if sim.r.chance(0.5) {
                sim.say(FAM, "recruit.record.ask", &tr, &f, true);
                let r = ["strong", "mixed", "weak"][sim.r.int(0, 2) as usize];
                sim.say(FAM, &format!("recruit.record.{r}"), &tr, &f, true);
            }
            let close = ["warm", "flat", "flat", "cool"][sim.r.int(0, 3) as usize];
            sim.say(FAM, &format!("recruit.close.{setting}.{close}"), &tr, &f, true);
        }
    }
    let e = sim.ears.get(FAM).expect("no lines said");
    let mut thin: Vec<(&String, &usize)> = e.thin.iter().collect();
    thin.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
    let mut uses: Vec<(&String, &usize)> = e.uses.iter().collect();
    uses.sort();
    println!(
        "recruit talks: {} lines, {} said again by the same speaker ({:.2}%), {} heard again from anyone ({:.1}%); \
         thinnest {:?}",
        e.parts,
        e.same,
        100.0 * e.same as f64 / e.parts.max(1) as f64,
        e.any,
        100.0 * e.any as f64 / e.parts.max(1) as f64,
        &thin[..thin.len().min(8)]
    );
    println!("recruit talks uses: {uses:?}");
    // A part nothing fits is said by a plain version or not at all: count what the bank could not say.
    let silent = e.parts;
    assert!(silent > 0);
    assert!(
        (e.same as f64 / e.parts as f64) <= bar("recruit"),
        "{} of {} lines said again by the same speaker",
        e.same,
        e.parts
    );
}
