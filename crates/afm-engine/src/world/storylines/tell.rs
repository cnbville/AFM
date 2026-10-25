//! Who tells a story (S3): one story told by everyone, each from his own
//! angle and in his own voice. The facts come from the story's record; the
//! angle from who speaks and what he knows. Beat writers report from their
//! sources (a reporter's accuracy decides whether his report holds), the
//! national columnists take the big picture, the radio and the fans are
//! loud and split, players post, parents call in, rival coaches use it,
//! your staff tells you what is really going on. Lines come from the story
//! bank (`data/lines/stories/storylines.txt`), keyed by kind, family and
//! beat, and know the story's history ("for the third week running");
//! quotes come from the people bank (`data/lines/storylines.txt`) in the
//! speaker's own voice.

use super::{Beat, BeatKind, Family, K, Part, RULE, Story, Voice, Who, add_beat, go_public};
use crate::calendar::{Event, Kind, Level, Link, Loudness};
use crate::world::World;
use crate::world::folk::cast::{self, Role};
use crate::world::folk::voice::{Facts, Pen, Speaker};
use crate::world::stories::lines::Fill;
use crate::world::stories::{TeamKey, short_name, team_fill, team_in_text};

/// A name and its last word, and the player's position.
struct Name {
    full: String,
    last: String,
    first: String,
    pos: Option<&'static str>,
}

fn name_of(w: &World, who: Who) -> Option<Name> {
    let split = |full: String, pos: Option<&'static str>| {
        let parts: Vec<&str> = full.split_whitespace().collect();
        let last = match parts.as_slice() {
            [.., l, s] if matches!(*s, "Jr." | "Sr." | "II" | "III" | "IV") => format!("{l} {s}"),
            [.., l] => l.to_string(),
            _ => full.clone(),
        };
        let first = parts.first().map_or_else(String::new, |s| s.to_string());
        Name { full: full.clone(), last, first, pos }
    };
    match who {
        Who::Player(id) => {
            let i = super::pi(w, id)?;
            let p = &w.people[i];
            Some(split(p.name.clone(), Some(p.player.position.name())))
        }
        Who::Coach(c) => w.coaches.get(c as usize).map(|c| split(c.name.clone(), None)),
        Who::Team(t) => Some(split(team_in_text(w, t), None)),
        Who::Boss(t) => {
            let b = crate::world::folk::boss::boss(w, super::folk_team(t));
            Some(split(b.name.clone(), None))
        }
        Who::Parent(id) => {
            let i = super::pi(w, id)?;
            let p = &w.people[i];
            let last = p.name.split_whitespace().last().unwrap_or("").to_string();
            Some(Name { full: format!("{last}'s father"), last: last.clone(), first: String::new(), pos: None })
        }
    }
}

/// Position words for lines ("quarterback").
fn pos_word(p: &str) -> &'static str {
    match p {
        "QB" => "quarterback",
        "RB" => "running back",
        "WR" => "receiver",
        "TE" => "tight end",
        "OT" => "tackle",
        "IOL" => "guard",
        "EDGE" => "edge rusher",
        "DT" => "defensive tackle",
        "LB" => "linebacker",
        "CB" => "cornerback",
        "S" => "safety",
        "K" => "kicker",
        "P" => "punter",
        _ => "long snapper",
    }
}

const NUMBERS: [&str; 13] =
    ["no", "one", "two", "three", "four", "five", "six", "seven", "eight", "nine", "ten", "eleven", "twelve"];
const ORDINALS: [&str; 13] = [
    "", "first", "second", "third", "fourth", "fifth", "sixth", "seventh", "eighth", "ninth", "tenth", "eleventh",
    "twelfth",
];
const MONTHS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

/// Every value a story's lines may name.
pub(crate) fn fill(w: &World, s: &Story) -> Fill {
    let mut f = Fill::new();
    if let Some(t) = s.team {
        f = team_fill(w, t, f);
        if let Some(c) = head_coach(w, t)
            && let Some(n) = name_of(w, Who::Coach(c as u32))
        {
            f = f.set("coach", n.full).set("coach_last", n.last);
        }
    }
    if let Some(o) = s.other {
        f = f.set("other", team_in_text(w, o)).set("other_short", short_name(w, o));
    }
    for (part, key, last, first, pos) in [
        (Part::Lead, "lead", "lead_last", "lead_first", "lead_pos"),
        (Part::Rival, "rival", "rival_last", "rival_first", "rival_pos"),
    ] {
        if let Some(m) = s.cast.iter().find(|m| m.part == part)
            && let Some(n) = name_of(w, m.who)
        {
            f = f.set(key, n.full).set(last, n.last).set(first, n.first);
            if let Some(p) = n.pos {
                f = f.set(pos, pos_word(p));
            }
        }
    }
    if let Some(t) = s.team {
        let b = crate::world::folk::boss::boss(w, super::folk_team(t));
        f = f.set("boss", b.name.clone()).set("boss_title", b.title.to_lowercase());
        if s.kind == K::VoteOfConfidence.id() {
            f = f.set("other_name", b.name.clone());
        }
    }
    for (k, key) in ["n", "n2", "n3", "n4"].iter().enumerate() {
        if s.nums[k] != 0 {
            f = f.set(key, s.nums[k].to_string());
        }
    }
    if s.nums[0] > 0 && (s.nums[0] as usize) < NUMBERS.len() {
        f = f.set("n_words", NUMBERS[s.nums[0] as usize]);
    }
    f = numbers(w, s, f);
    // A Hall of Fame candidate long gone from the world's people.
    if f.get("lead").is_none() && !s.word.is_empty() && s.kind == K::HallCall.id() {
        let last = s.word.split_whitespace().last().unwrap_or("").to_string();
        f = f.set("lead", s.word.clone()).set("lead_last", last);
    }
    if let Some(t) = s.team {
        let (wn, l) = record_now(w, t);
        if wn + l > 0 {
            f = f.set("record", format!("{wn}-{l}"));
        }
    }
    let weeks = (s.age(w.today) / 7).max(0) as usize;
    if weeks >= 2 {
        f = f.set("weeks", if weeks < NUMBERS.len() { NUMBERS[weeks].to_string() } else { weeks.to_string() });
        if weeks < ORDINALS.len() {
            f = f.set("nth_week", ORDINALS[weeks]);
        }
    }
    if s.age(w.today) >= 20 {
        f = f.set("since", MONTHS[(s.opened.month() as usize).saturating_sub(1) % 12]);
    }
    if !s.word.is_empty() {
        let key = match K::ALL.get(s.kind as usize) {
            Some(K::TrophyGame) => "trophy",
            Some(K::ConferenceRace | K::Realignment) => "conf",
            Some(K::RecordChase | K::CareerRecord) => "holder",
            Some(K::PositionSwitch) => "pos",
            Some(K::AwardRace) => "award",
            _ => "word",
        };
        f = f.set(key, s.word.clone()).set("word", s.word.clone());
        if s.kind == K::PositionSwitch.id() {
            f = f.set("pos", pos_word(&s.word));
        }
    }
    f = f.set("season", s.season.to_string()).set("year", w.year.to_string());
    let q = question_with(s, &f);
    f.set("question", q)
}

/// A count in words up to twelve ("three"), digits above.
fn count(n: i32) -> String {
    if (0..NUMBERS.len() as i32).contains(&n) { NUMBERS[n as usize].to_string() } else { n.to_string() }
}

fn stars(n: i32) -> String {
    match n {
        1..=5 => format!("{}-star", NUMBERS[n as usize]),
        _ => "unrated".to_string(),
    }
}

/// Named numbers by kind, so lines read "$12 million", "8 of 13" and
/// "No. 4 in the country" instead of bare numbers.
fn numbers(w: &World, s: &Story, mut f: Fill) -> Fill {
    use crate::world::folk::facts::thousands;
    let n = s.nums;
    let Some(&kind) = K::ALL.get(s.kind as usize) else { return f };
    match kind {
        K::BuyoutWatch if n[0] > 0 => f = f.set("money", format!("${} million", n[0])),
        K::HotSeat if n[3] > 0 => f = f.set("money", format!("${} million", n[3])),
        _ => {}
    }
    match kind {
        K::HotSeat | K::ExtensionTalks | K::YourContract if n[2] > 2000 => f = f.set("through", n[2].to_string()),
        K::KickerSlump if n[1] > 0 => f = f.set("made", n[0].to_string()).set("tried", n[1].to_string()),
        K::CareerRecord | K::RecordChase if n[1] > 0 => {
            let stat = super::detect::weekly::stat_name(n[2]);
            if !stat.is_empty() {
                f = f
                    .set("stat", stat)
                    .set("mark", thousands(n[0].max(0) as u32))
                    .set("record_mark", thousands(n[1] as u32))
                    .set("to_go", thousands((n[1] - n[0]).max(0) as u32))
                    .set("scope", super::detect::weekly::record_scope(n[2]));
            }
        }
        K::Breakout | K::FreshmanStar | K::WalkOnStar | K::LateRoundGem if n[1] > 0 && n[1] < 99 => {
            const STATS: [&str; 5] = ["passing yards", "rushing yards", "receiving yards", "sacks", "interceptions"];
            f = f.set("rank", n[1].to_string());
            if let Some(st) = STATS.get(n[2].max(0) as usize) {
                f = f.set("stat", *st);
            }
        }
        K::Turnaround if n[2] + n[3] > 0 => f = f.set("last_record", format!("{}-{}", n[2], n[3])),
        K::Drought if n[0] > 1900 => {
            f = f.set("since_year", n[0].to_string()).set("years", count(w.year - n[0]));
        }
        K::LateRiser if n[1] > 0 => f = f.set("stars_was", stars(n[0])).set("stars_now", stars(n[1])),
        K::PortalExodus if n[0] > 0 => f = f.set("gone", count(n[0])).set("starters", count(n[1])),
        K::LeagueSuspension | K::SuspensionReturn if n[0] > 0 => {
            f = f.set("games", if n[0] == 1 { "one game".to_string() } else { format!("{} games", count(n[0])) });
        }
        K::CoordinatorHeat if n[1] > 0 => {
            f = f
                .set("unit", if n[2] == 1 { "offense" } else { "defense" })
                .set("rank_of", format!("{} of {}", n[0], n[1]));
        }
        K::Unbeaten | K::LosingStreak | K::RivalryStreak | K::Dynasty | K::Graveyard if n[0] > 0 => {
            f = f.set("count", count(n[0]));
        }
        K::CoachMilestone | K::YourMilestone if n[0] > 0 => f = f.set("milestone", n[0].to_string()),
        _ => {}
    }
    f
}

/// The story's question with its names filled.
pub fn question(w: &World, s: &Story) -> String {
    let f = fill(w, s);
    question_with(s, &f)
}

/// The question a story asks, as its fill allows: a coach wanted elsewhere
/// with no team of his own to leave is asked about the job alone.
fn asked(s: &Story, f: &Fill) -> &'static str {
    let q = s.def().question;
    if f.get("team").is_none() && q.contains("leave {team} for {other}") {
        return "Will {lead} take the {other_short} job?";
    }
    q
}

fn question_with(s: &Story, f: &Fill) -> String {
    let q = asked(s, f);
    let mut out = String::with_capacity(q.len() + 24);
    let mut rest = q;
    while let Some(a) = rest.find('{') {
        out.push_str(&rest[..a]);
        let after = &rest[a + 1..];
        let Some(z) = after.find('}') else { break };
        let key = &after[..z];
        let v = f.get(key).map(str::to_string).unwrap_or_else(|| match key {
            // Someone gone from the world: named by what he was in it.
            "lead" | "rival" => {
                let part = if key == "lead" { Part::Lead } else { Part::Rival };
                match s.cast.iter().find(|m| m.part == part).map(|m| m.who) {
                    Some(Who::Coach(_)) => "the coach",
                    Some(Who::Team(_)) => "the team",
                    _ if s.family() == Family::Recruiting => "the recruit",
                    _ => "the player",
                }
                .to_string()
            }
            "other" | "other_name" => "the other side".to_string(),
            "team" => "the team".to_string(),
            "n" => "the next one".to_string(),
            "money" => "the buyout".to_string(),
            _ => "it".to_string(),
        });
        out.push_str(&v);
        rest = &after[z + 1..];
    }
    out.push_str(rest);
    crate::world::stories::lines::tidy(&out)
}

/// The names a story's question asks for that its fill doesn't have (its
/// lead gone from the world, a team story asking for a person): the
/// scripted seasons list them.
pub fn question_gaps(w: &World, s: &Story) -> Vec<String> {
    let f = fill(w, s);
    let mut gaps = Vec::new();
    let mut rest = asked(s, &f);
    while let Some(a) = rest.find('{') {
        let after = &rest[a + 1..];
        let Some(z) = after.find('}') else { break };
        if f.get(&after[..z]).is_none() {
            gaps.push(after[..z].to_string());
        }
        rest = &after[z + 1..];
    }
    gaps
}

pub(crate) fn head_coach(w: &World, t: TeamKey) -> Option<usize> {
    crate::world::folk::Team::head_coach(super::folk_team(t), w)
}

/// A team's record this season so far.
pub(crate) fn record_now(w: &World, t: TeamKey) -> (u32, u32) {
    match t {
        TeamKey::College(p) => {
            w.college.as_ref().and_then(|c| c.results.get(p as usize)).map_or((0, 0), |r| (r.wins, r.losses))
        }
        TeamKey::Nfl(n) => {
            let (wn, l, _) = w.nfl_record(n as usize);
            (wn, l)
        }
        TeamKey::Hs(h) => {
            w.hs_season.as_ref().and_then(|s| s.results.get(h as usize)).map_or((0, 0), |r| (r.wins, r.losses))
        }
        TeamKey::Other(_) => (0, 0),
    }
}

/// The keys a beat's line is looked for under, most specific first.
fn keys(s: &Story, what: BeatKind, extra: Option<&str>) -> Vec<String> {
    let def = s.def();
    let fam = def.family.key();
    let b = what.key();
    let mut v = Vec::with_capacity(6);
    match what {
        BeatKind::Resolve => {
            let e = def.ending_key(s.ending);
            v.push(format!("sl.{}.end.{e}", def.key));
            v.push(format!("sl.{}.end", def.key));
            v.push(format!("sl.{fam}.end"));
            v.push("sl.any.end".to_string());
        }
        _ => {
            if let Some(x) = extra {
                v.push(format!("sl.{}.{b}.{x}", def.key));
                v.push(format!("sl.{fam}.{b}.{x}"));
                v.push(format!("sl.any.{b}.{x}"));
            }
            v.push(format!("sl.{}.{b}", def.key));
            v.push(format!("sl.{fam}.{b}"));
            v.push(format!("sl.any.{b}"));
        }
    }
    v
}

/// A beat's line from the bank, with the story's names and history.
pub(crate) fn line(w: &mut World, s: &Story, what: BeatKind, extra: Option<&str>, f: &Fill) -> Option<String> {
    let seed = w.options.seed;
    let year = w.year;
    // Never a line this story already told.
    let said: Vec<&str> = s.beats.iter().filter(|b| !b.text.is_empty()).map(|b| b.text.as_str()).collect();
    for k in keys(s, what, extra) {
        if let Some(t) = w.storylines.line(seed, &k, f, year, &said) {
            return Some(t);
        }
    }
    None
}

/// The text of a beat by a voice (a plain line when the bank has none).
pub(crate) fn text(w: &mut World, id: u32, what: BeatKind, by: Voice) -> String {
    let Some(s) = w.storylines.get(id).cloned() else { return String::new() };
    let f = fill(w, &s);
    if let Some(t) = line(w, &s, what, None, &f) {
        return t;
    }
    plain(&s, what, by, &f)
}

/// The fallback when the bank has nothing for a beat.
fn plain(s: &Story, what: BeatKind, _by: Voice, f: &Fill) -> String {
    let q = f.get("question").unwrap_or("").to_string();
    let team = f.get("team").unwrap_or("the team");
    match what {
        BeatKind::Spark | BeatKind::Seed => format!("{q} The question is out in the open now."),
        BeatKind::Peak => format!("It comes to a head this week: {}", lower_first(&q)),
        BeatKind::Resolve => {
            let e = s.def().ending_key(s.ending).replace('_', " ");
            format!("{} has its answer ({e}).", s.def().name)
        }
        BeatKind::Fizzle => format!("The talk around {team} has moved on."),
        _ => q,
    }
}

fn lower_first(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_lowercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

/// How loud a beat is on the calendar; None keeps it off the feed.
fn loudness(s: &Story, what: BeatKind) -> Option<Loudness> {
    let key = matches!(what, BeatKind::Peak | BeatKind::Resolve | BeatKind::Spark);
    if s.you.is_some() {
        Some(if key || s.heat >= 45.0 { Loudness::Medium } else { Loudness::Small })
    } else if s.near {
        Some(if key && s.heat >= 50.0 { Loudness::Medium } else { Loudness::Small })
    } else if s.heat >= 75.0 && key {
        Some(Loudness::Medium)
    } else if s.heat >= 50.0 || (key && s.heat >= 38.0) {
        Some(Loudness::Small)
    } else {
        None
    }
}

/// Posts the story's newest beat on the calendar, with its names linked.
pub(crate) fn post_beat(w: &mut World, id: u32) {
    let Some(s) = w.storylines.get(id).cloned() else { return };
    let Some(b) = s.beats.last().cloned() else { return };
    if b.text.is_empty() || !b.what.public() {
        return;
    }
    let Some(loud) = loudness(&s, b.what) else { return };
    let level =
        if s.level == Level::Recruiting || s.family() == Family::Recruiting { Level::Recruiting } else { s.level };
    let rumor =
        matches!(b.what, BeatKind::Leak) || (!s.def().public && matches!(b.what, BeatKind::Spark | BeatKind::Report));
    let mut e =
        Event::new(w.today, level, if rumor { Kind::Whisper } else { Kind::News }, loud, b.text.clone()).rule(RULE);
    e = link_names(w, &s, e);
    if b.what == BeatKind::Resolve && s.peak_heat >= 85.0 {
        e = e.memorable();
    }
    w.calendar.post(e);
}

/// Links the people and teams a line names.
fn link_names(w: &World, s: &Story, mut e: Event) -> Event {
    for m in &s.cast {
        match m.who {
            Who::Player(id) => {
                if let Some(i) = super::pi(w, id) {
                    let name = w.people[i].name.clone();
                    e = e.person(id, &name);
                }
            }
            Who::Coach(c) => {
                if let Some(co) = w.coaches.get(c as usize) {
                    let name = co.name.clone();
                    e = e.coach_link(c as usize, &name);
                }
            }
            _ => {}
        }
    }
    for t in [s.team, s.other].into_iter().flatten() {
        let name = short_name(w, t);
        e = match t {
            TeamKey::College(p) => e.school_link(p as usize, &name),
            TeamKey::Nfl(n) => e.club_link(n as usize, &name),
            TeamKey::Hs(h) => e.hs_link(h as usize, &name),
            TeamKey::Other(f) => e.link(Link::League(f as u32), &name),
        };
    }
    match s.team {
        Some(TeamKey::College(p)) => e.program(p as usize),
        Some(TeamKey::Nfl(n)) => e.team(n as usize),
        _ => e,
    }
}

/// The team's reporters, best known first (none drawn for far stories
/// that don't need a name).
fn reporters(w: &World, t: TeamKey) -> Vec<cast::Cast> {
    cast::all(w, super::folk_team(t), Role::Reporter)
}

/// A private story's daily chance to go public, and through whom: a
/// reporter with a source in the building, a player's post, a parent on
/// the radio, a rival coach. A leaky building sparks more.
pub(crate) fn maybe_spark(w: &mut World, id: u32) {
    let Some(s) = w.storylines.get(id).cloned() else { return };
    let seed = w.options.seed;
    // The building: its reporters' habits (only drawn when it matters).
    let (leaky, writer) = match s.team {
        Some(t) if s.near || s.stakes >= 50 || s.fame >= 0.5 => {
            let rs = reporters(w, t);
            let leaky = rs.iter().filter(|r| r.reporter.as_ref().is_some_and(|i| i.leaks)).count();
            (leaky as f32, rs.into_iter().next())
        }
        _ => (0.0, None),
    };
    let p = 0.009
        * (0.6 + s.stakes as f32 / 60.0)
        * (0.6 + 0.8 * s.fame)
        * (0.7 + 0.6 * s.market)
        * (1.0 + 0.45 * leaky)
        * super::slow_news(w.today);
    let r = w.storylines.rng(seed).next() as f32;
    if r >= p {
        return;
    }
    // Through whom: a reporter most often; a player with an ego or a loud
    // parent sometimes; a rival coach in a rivalry or recruiting story.
    let lead_i = s.lead_player().and_then(|id| super::pi(w, id));
    let ego = lead_i.map_or(0.0, |i| w.people[i].player.personality.traits.get(crate::player::Trait::Ego) as f32);
    let loud_parent = lead_i.is_some_and(|i| {
        matches!(
            w.people[i].family.parent_style,
            crate::world::background::ParentStyle::LoudAtGames | crate::world::background::ParentStyle::SocialMedia
        )
    });
    let social = w.year >= 2008;
    let mut choices: Vec<(u8, f64)> = vec![(0, 3.0 + leaky as f64)];
    if lead_i.is_some() && social {
        choices.push((1, ((ego - 8.0).max(0.0) / 4.0) as f64));
    }
    if loud_parent {
        choices.push((2, 0.8));
    }
    if s.other.is_some() && matches!(s.family(), Family::Rivalry | Family::Recruiting | Family::Wanted) {
        choices.push((3, 0.8));
    }
    let pick = w.storylines.rng(seed).weighted(&choices);
    let (by, extra) = match pick {
        1 => (Voice::Player, Some("post")),
        2 => (Voice::Parent, Some("parent")),
        3 => (Voice::Rival, Some("rival")),
        _ => (if s.family() == Family::Recruiting { Voice::Insider } else { Voice::Beat }, None),
    };
    go_public(w, id, w.today);
    let mut f = fill(w, &s);
    if let Some(r) = &writer {
        f = f.set("writer", r.name.clone());
        if let Some(info) = &r.reporter {
            f = f.set("outlet", info.outlet.clone());
        }
    }
    let t = line(w, &s, BeatKind::Spark, extra, &f).unwrap_or_else(|| plain(&s, BeatKind::Spark, by, &f));
    // A reporter's first report moves the public toward what he heard.
    if let Some(k) = w.storylines.idx(id) {
        let acc = writer.as_ref().and_then(|r| r.reporter.as_ref()).map_or(0.6, |i| i.accuracy) as f32;
        let st = &mut w.storylines.open[k];
        st.belief = if acc > 0.55 { st.truth * acc } else { -0.3 * st.truth };
        st.sure = 0.25 + 0.3 * acc;
    }
    add_beat(w, id, BeatKind::Spark, by, 255, BeatKind::Spark.heat(), t);
    post_beat(w, id);
}

/// The voices on a public story today: the beat writer, a leak, a column,
/// the radio, the fans, a rival's jab, a post from someone in it, the boss.
pub(crate) fn voice_beat(w: &mut World, id: u32, attended: bool) {
    let Some(s) = w.storylines.get(id).cloned() else { return };
    let seed = w.options.seed;
    let fam = s.family();
    let social = w.year >= 2008;
    let mut choices: Vec<(u8, f64)> = vec![(0, 3.0), (5, 2.0)];
    if matches!(fam, Family::Seat | Family::Wanted | Family::Leaving | Family::Recruiting | Family::Battle) {
        choices.push((1, 1.5));
    }
    if s.heat >= 55.0 && attended {
        choices.push((2, 2.0));
    }
    if matches!(s.level, Level::College | Level::Nfl) {
        choices.push((3, 1.2));
    }
    if s.other.is_some() && matches!(fam, Family::Rivalry | Family::Streak | Family::Recruiting) {
        choices.push((4, 1.0));
    }
    if s.lead_player().is_some()
        && matches!(
            fam,
            Family::Battle | Family::Leaving | Family::OffField | Family::Rise | Family::Comeback | Family::Recruiting
        )
    {
        choices.push((6, if social { 1.6 } else { 0.6 }));
    }
    if matches!(fam, Family::Seat | Family::You) && s.team.is_some() {
        choices.push((7, 0.8));
    }
    let pick = w.storylines.rng(seed).weighted(&choices);
    let (what, by, extra) = match pick {
        1 => (BeatKind::Leak, if fam == Family::Recruiting { Voice::Insider } else { Voice::Beat }, None),
        2 => (BeatKind::Column, Voice::National, None),
        3 => (BeatKind::Radio, Voice::Radio, None),
        4 => (BeatKind::Jab, Voice::Rival, None),
        5 => (BeatKind::Fans, Voice::Fans, Some(if s.sides[1] >= 0.5 { "for" } else { "against" })),
        6 => (
            BeatKind::Post,
            if fam == Family::Recruiting { Voice::Recruit } else { Voice::Player },
            if social { None } else { Some("paper") },
        ),
        7 => (BeatKind::Quote, Voice::Boss, Some("boss")),
        _ => (BeatKind::Report, if fam == Family::Recruiting { Voice::Insider } else { Voice::Beat }, None),
    };
    let writer = match (what, s.team) {
        (BeatKind::Report | BeatKind::Leak, Some(t)) if s.near || s.heat >= 50.0 => reporters(w, t).into_iter().next(),
        _ => None,
    };
    let mut f = fill(w, &s);
    if let Some(r) = &writer {
        f = f.set("writer", r.name.clone());
        if let Some(info) = &r.reporter {
            f = f.set("outlet", info.outlet.clone());
        }
    }
    let Some(mut t) = line(w, &s, what, extra, &f).or_else(|| line(w, &s, what, None, &f)) else { return };
    // A beat story carries a quote from the people in it.
    if what == BeatKind::Report && s.near && w.storylines.rng(seed).chance(0.55) {
        let pick = if s.rival().is_some() && w.storylines.rng(seed).chance(0.4) { Part::Rival } else { Part::Lead };
        if let Some(q) = quote(w, &s, pick) {
            t = format!("{t} {q}");
        }
    }
    // What it does to the public's belief and the sides.
    if let Some(k) = w.storylines.idx(id) {
        let acc = writer.as_ref().and_then(|r| r.reporter.as_ref()).map_or(0.62, |i| i.accuracy) as f32;
        let hit = w.storylines.rng(seed).next() < acc as f64;
        let st = &mut w.storylines.open[k];
        match what {
            BeatKind::Report | BeatKind::Leak => {
                let toward = if hit { st.truth } else { -st.truth * 0.5 };
                st.belief += 0.35 * (toward - st.belief);
                st.sure = (st.sure + if what == BeatKind::Leak { 0.1 } else { 0.06 }).min(0.95);
                st.sides[2] = (st.sides[2] + 0.08 * (0.5 - st.belief * 0.5 - st.sides[2])).clamp(0.05, 0.95);
            }
            BeatKind::Fans => {
                st.sides[1] = (st.sides[1] + if extra == Some("for") { 0.02 } else { -0.02 }).clamp(0.05, 0.95);
            }
            BeatKind::Jab => {
                st.sides[0] = (st.sides[0] + 0.02).clamp(0.05, 0.95);
            }
            _ => {}
        }
    }
    let heat = what.heat();
    add_beat(w, id, what, by, 255, heat, t);
    post_beat(w, id);
}

/// A quote from a cast member in his own voice, with who said it:
/// "\"We'll see,\" Smith said."
pub(crate) fn quote(w: &mut World, s: &Story, part: Part) -> Option<String> {
    let m = s.cast.iter().find(|m| m.part == part)?;
    let fam = s.family().key();
    let (speaker, who) = match m.who {
        Who::Player(id) => {
            let i = super::pi(w, id)?;
            (Speaker::player(w, i), name_of(w, m.who)?.last)
        }
        Who::Coach(c) => (Speaker::coach(w, c as usize), name_of(w, m.who)?.last),
        Who::Boss(t) => {
            let b = crate::world::folk::boss::boss(w, super::folk_team(t));
            (Speaker::cast(w, &b), b.last().to_string())
        }
        Who::Parent(id) => {
            let i = super::pi(w, id)?;
            (Speaker::parent(w, i), name_of(w, m.who)?.full)
        }
        Who::Team(_) => return None,
    };
    let f = fill(w, s);
    let mut facts = Facts::new();
    for k in ["team", "rival", "rival_last", "lead", "lead_last", "other", "coach", "n", "record", "since"] {
        if let Some(v) = f.get(k) {
            facts.set(k, v.to_string());
        }
    }
    let part_key = match part {
        Part::Lead => "lead",
        Part::Rival => "rival",
        Part::Boss => "boss",
        Part::Parent => "parent",
        _ => "decider",
    };
    let tone = if s.def().tone < 0 { "down" } else { "up" };
    let keys = [
        format!("story.quote.{fam}.{part_key}.{tone}"),
        format!("story.quote.{fam}.{part_key}"),
        format!("story.quote.any.{part_key}"),
    ];
    let seed = (w.folk.base as u64) << 16 ^ (s.id as u64) << 4 ^ s.beats.len() as u64;
    let mut pen = Pen::take(w, seed);
    let mut said = String::new();
    for k in &keys {
        if let Some(t) = pen.part(k, &speaker, &facts) {
            said = t;
            break;
        }
    }
    pen.put(w);
    if said.is_empty() {
        return None;
    }
    let said = said.trim_end_matches(['.', ',']).to_string();
    let end = if said.ends_with('?') || said.ends_with('!') { "" } else { "," };
    Some(format!("\"{said}{end}\" {who} said."))
}

/// Your staff's memo on one of your stories, about once a week: what is
/// really going on (the truth, against what the public believes) and what
/// they would do.
pub(crate) fn maybe_memo(w: &mut World, id: u32) {
    let d = w.today;
    if d.weekday() != crate::calendar::Weekday::Tue {
        return;
    }
    let Some(s) = w.storylines.get(id).cloned() else { return };
    if s.heat < 18.0 || s.beats.iter().any(|b| b.what == BeatKind::Memo && b.date.days_to(d) < 13) {
        return;
    }
    let Some(team) = s.team.or_else(|| crate::world::stories::seat_team(w)) else { return };
    // Two memos a week at most, hottest first.
    let this_week =
        w.storylines.open.iter().filter(|o| o.beats.iter().any(|b| b.what == BeatKind::Memo && b.date == d)).count();
    if this_week >= 2 {
        return;
    }
    let lean = if s.truth > 0.25 {
        "up"
    } else if s.truth < -0.25 {
        "down"
    } else {
        "even"
    };
    let f = fill(w, &s);
    let read = line(w, &s, BeatKind::Memo, Some(lean), &f);
    let gap = (s.truth - s.belief).abs() > 0.45 && s.public;
    let gap_line = gap.then(|| line(w, &s, BeatKind::Memo, Some("gap"), &f)).flatten();
    let Some(read) = read else { return };
    let mut body = vec![read];
    if let Some(g) = gap_line {
        body.push(g);
    }
    body.push(format!(
        "Where it stands: {}, heat {} of 100{}.",
        s.phase.name().to_lowercase(),
        s.heat.round(),
        if s.public { "" } else { ", still inside the building" }
    ));
    add_beat(w, id, BeatKind::Memo, Voice::Staff, 255, 0.0, body[0].clone());
    super::credit::memo(w, &s, team, body);
}

/// The callback line a story leaves behind ("the summer he lost the job").
pub(crate) fn legacy(w: &mut World, id: u32) {
    let Some(s) = w.storylines.get(id).cloned() else { return };
    let f = fill(w, &s);
    let def = s.def();
    let e = def.ending_key(s.ending);
    let seed = w.options.seed;
    let year = w.year;
    let mut out = None;
    for k in [
        format!("sl.{}.legacy.{e}", def.key),
        format!("sl.{}.legacy", def.key),
        format!("sl.{}.legacy", def.family.key()),
    ] {
        if let Some(t) = w.storylines.line(seed, &k, &f, year, &[]) {
            out = Some(t);
            break;
        }
    }
    if let Some(k) = w.storylines.idx(id) {
        w.storylines.open[k].legacy = out.unwrap_or_default();
    }
}

/// The last beat with text, for the screens.
pub fn last_said(s: &Story) -> Option<&Beat> {
    s.beats.iter().rev().find(|b| !b.text.is_empty() && b.what.public())
}
