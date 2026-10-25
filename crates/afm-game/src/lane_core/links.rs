//! Links in text (screens topic 17, L1): every noun with a page is a link.
//!
//! Text the screens draw carries a small markup the page turns into links:
//! `[[p:12|Jalen Carter]]` a player by id, `[[c:7|Mike Tillis]]` a coach,
//! `[[t:college:45|Iowa]]` a team, `[[g:3|the 31-17 win]]` one of your kept
//! games, `[[d:2026-03-14|Sat 14 Mar]]` a day on the calendar and
//! `[[s:standings?conference=SEC|9-3]]` a screen. Text written by the game
//! is linked when it is written (mail). The calendar's lines and the news
//! desk's stories store the ids behind their names (L2, `Ref`): those are
//! linked exactly, and any name still plain is found by its words when a
//! screen shows the line (`Linker`).

use std::collections::HashMap;

use afm_engine::calendar::{Date, Event, Level, Link, Ref};
use afm_engine::world::{Stage, World};

use crate::views::TeamRef;

/// Text that may hold `[[` or `|` is made safe to sit inside markup.
fn clean(s: &str) -> String {
    s.replace("[[", "[").replace("]]", "]").replace('|', "/")
}

pub fn player(id: u32, name: &str) -> String {
    format!("[[p:{id}|{}]]", clean(name))
}

pub fn coach(c: usize, name: &str) -> String {
    format!("[[c:{c}|{}]]", clean(name))
}

pub fn team_ref(t: TeamRef) -> String {
    match t {
        TeamRef::Nfl(i) => format!("nfl:{i}"),
        TeamRef::College(i) => format!("college:{i}"),
        TeamRef::HighSchool(i) => format!("hs:{i}"),
        TeamRef::Other(i) => format!("other:{i}"),
    }
}

pub fn team(w: &World, t: TeamRef) -> String {
    team_as(t, &crate::views::team_name(w, t))
}

pub fn team_as(t: TeamRef, text: &str) -> String {
    format!("[[t:{}|{}]]", team_ref(t), clean(text))
}

pub fn game(index: usize, text: &str) -> String {
    format!("[[g:{index}|{}]]", clean(text))
}

pub fn day(d: Date, text: &str) -> String {
    let (y, m, dd) = d.parts();
    format!("[[d:{y:04}-{m:02}-{dd:02}|{}]]", clean(text))
}

pub fn screen(target: &str, text: &str) -> String {
    format!("[[s:{target}|{}]]", clean(text))
}

/// Markup taken out: the words alone (for search and previews).
pub fn strip(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(i) = rest.find("[[") {
        out.push_str(&rest[..i]);
        let tail = &rest[i + 2..];
        match (tail.find('|'), tail.find("]]")) {
            (Some(bar), Some(end)) if bar < end => {
                out.push_str(&tail[bar + 1..end]);
                rest = &tail[end + 2..];
            }
            _ => {
                out.push_str("[[");
                rest = tail;
            }
        }
    }
    out.push_str(rest);
    out
}

fn fnv(s: &str) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in s.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    h
}

/// A word that can be part of a name: starts with a capital (O'Neil,
/// McKenzie), or a suffix (Jr., III).
fn name_word(w: &str) -> bool {
    let w = w.trim_end_matches([',', '.', ';', ':', ')', '!', '?']).trim_end_matches("'s");
    w.chars().next().is_some_and(char::is_uppercase) && w.chars().all(|c| c.is_alphabetic() || "'-.".contains(c))
}

/// Where each word starts and ends in a text.
fn words(text: &str) -> Vec<(usize, usize)> {
    let mut v = Vec::new();
    let mut start = None;
    for (i, ch) in text.char_indices() {
        let part = ch.is_alphanumeric() || ch == '\'' || ch == '-' || ch == '.' || ch == '&';
        match (part, start) {
            (true, None) => start = Some(i),
            (false, Some(s)) => {
                v.push((s, i));
                start = None;
            }
            _ => {}
        }
    }
    if let Some(s) = start {
        v.push((s, text.len()));
    }
    // A word's trailing period (end of a sentence) is not part of it,
    // unless it is an initial or a suffix (Jr., St.).
    for (s, e) in v.iter_mut() {
        let w = &text[*s..*e];
        if w.ends_with('.') && w.len() > 3 && !matches!(w, "Jr." | "Sr." | "St.") {
            *e -= 1;
        }
    }
    v
}

/// Finds the teams and people named in lines of text and links them.
/// Built once for a screen's lines: one pass over the world's names.
pub struct Linker<'a> {
    w: &'a World,
    /// Team names by their first word: (full name, team).
    teams: HashMap<&'a str, Vec<(String, TeamRef)>>,
    /// People and coaches by the hash of their full name.
    people: HashMap<u64, Vec<usize>>,
    coaches: HashMap<u64, Vec<usize>>,
}

impl<'a> Linker<'a> {
    /// A linker for these lines (their names are looked up once). High
    /// school names are linked only when `schools` is set: a high school
    /// called Central is not every "Central" in a college line.
    pub fn new(w: &'a World, lines: &[&str], schools: bool) -> Linker<'a> {
        let mut teams: HashMap<&'a str, Vec<(String, TeamRef)>> = HashMap::new();
        let mut add = |name: &'a str, full: String, t: TeamRef| {
            let first = name.split_whitespace().next().unwrap_or(name);
            teams.entry(first).or_default().push((full, t));
        };
        for p in &w.programs {
            let n = w.program_name(p.id);
            add(n, n.to_string(), TeamRef::College(p.id));
        }
        for (i, t) in w.nfl.iter().enumerate() {
            let full: &'a str = t.name;
            add(full, full.to_string(), TeamRef::Nfl(i));
            // "the Bears": the club by its nickname alone.
            if let Some(nick) = full.rsplit(' ').next()
                && nick != full
            {
                add(nick, nick.to_string(), TeamRef::Nfl(i));
            }
        }
        if schools {
            for s in &w.high_schools {
                let n = w.school_name(s.id);
                add(n, n.to_string(), TeamRef::HighSchool(s.id));
            }
        }
        // Each list longest name first, so "Texas A&M" wins over "Texas".
        for v in teams.values_mut() {
            v.sort_by_key(|x| std::cmp::Reverse(x.0.len()));
        }
        // Names in the lines: runs of two to four capitalised words.
        let mut wanted: std::collections::HashSet<u64> = std::collections::HashSet::new();
        for line in lines {
            let ws = words(line);
            for i in 0..ws.len() {
                for n in 2..=4 {
                    if i + n > ws.len() {
                        break;
                    }
                    let span = &ws[i..i + n];
                    if !span.iter().all(|&(s, e)| name_word(&line[s..e])) {
                        break;
                    }
                    wanted.insert(fnv(&line[span[0].0..span[n - 1].1]));
                }
            }
        }
        let mut people: HashMap<u64, Vec<usize>> = HashMap::new();
        let mut coaches: HashMap<u64, Vec<usize>> = HashMap::new();
        if !wanted.is_empty() {
            for (k, p) in w.people.iter().enumerate() {
                let h = fnv(&p.name);
                if wanted.contains(&h) {
                    people.entry(h).or_default().push(k);
                }
            }
            for (k, c) in w.coaches.iter().enumerate() {
                let h = fnv(&c.name);
                if wanted.contains(&h) {
                    coaches.entry(h).or_default().push(k);
                }
            }
        }
        Linker { w, teams, people, coaches }
    }

    /// Whether a person belongs to one of these teams now: on its roster,
    /// committed or signed to it, or at its high school.
    fn person_in(&self, k: usize, ctx: &[TeamRef]) -> bool {
        let p = &self.w.people[k];
        ctx.iter().any(|&t| match t {
            TeamRef::College(c) => {
                p.college == Some(c)
                    || p.cal.commit.is_some_and(|x| x.0 == c)
                    || p.cal.signed.is_some_and(|s| s.program == c)
                    || p.path.contains(&c)
            }
            TeamRef::Nfl(n) => p.pro.as_ref().is_some_and(|c| c.team == Some(n) || c.teams.contains(&n)),
            TeamRef::HighSchool(h) => p.hs == h,
            TeamRef::Other(f) => self.w.others.team(f).is_some_and(|t| t.roster.contains(&(k as u32))),
        })
    }

    fn coach_in(&self, c: usize, ctx: &[TeamRef]) -> bool {
        use afm_engine::world::coach::Employer;
        let co = &self.w.coaches[c];
        let at = |e: Employer| {
            ctx.iter().any(|&t| match (t, e) {
                (TeamRef::College(a), Employer::College(b)) => a == b,
                (TeamRef::Nfl(a), Employer::Nfl(b)) => a == b,
                (TeamRef::HighSchool(a), Employer::HighSchool(b)) => a == b,
                _ => false,
            })
        };
        co.job.is_some_and(|j| at(j.employer)) || co.history.iter().rev().take(3).any(|s| at(s.employer))
    }

    /// The one person or coach a name means here, if it is clear.
    fn who(&self, name: &str, ctx: &[TeamRef]) -> Option<String> {
        let h = fnv(name);
        let people: Vec<usize> = self.people.get(&h).cloned().unwrap_or_default();
        let coaches: Vec<usize> = self
            .coaches
            .get(&h)
            .map(|v| v.iter().copied().filter(|&c| !self.w.coaches[c].retired).collect())
            .unwrap_or_default();
        let pick = |v: &[usize], inside: &dyn Fn(usize) -> bool, active: &dyn Fn(usize) -> bool| -> Option<usize> {
            if v.len() == 1 {
                return Some(v[0]);
            }
            let near: Vec<usize> = v.iter().copied().filter(|&k| inside(k)).collect();
            if near.len() == 1 {
                return Some(near[0]);
            }
            let pool = if near.is_empty() { v.to_vec() } else { near };
            let live: Vec<usize> = pool.into_iter().filter(|&k| active(k)).collect();
            (live.len() == 1).then(|| live[0])
        };
        // A coach and a player with one name: the one tied to the line's teams.
        let p = pick(&people, &|k| self.person_in(k, ctx), &|k| self.w.people[k].stage != Stage::Done);
        let c = pick(&coaches, &|k| self.coach_in(k, ctx), &|k| self.w.coaches[k].job.is_some());
        match (p, c) {
            (Some(k), None) => Some(player(self.w.people[k].player.id, name)),
            (None, Some(c)) => Some(coach(c, name)),
            (Some(k), Some(c)) => {
                if self.coach_in(c, ctx) && !self.person_in(k, ctx) {
                    Some(coach(c, name))
                } else if self.person_in(k, ctx) && !self.coach_in(c, ctx) {
                    Some(player(self.w.people[k].player.id, name))
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// The line with its teams and people linked. `ctx` are teams the
    /// line is known to be about (the event's program or club).
    pub fn link(&self, text: &str, ctx: &[TeamRef]) -> String {
        self.link_refs(text, &[], ctx)
    }

    /// The line linked from the ids it stores first (exact), then by the
    /// names still plain.
    pub fn link_refs(&self, text: &str, refs: &[Ref], ctx: &[TeamRef]) -> String {
        let w = self.w;
        let mut spans: Vec<(usize, usize, String)> = Vec::new();
        let mut ctx: Vec<TeamRef> = ctx.to_vec();
        for r in refs {
            let (a, b) = (r.at as usize, r.at as usize + r.len as usize);
            if b > text.len() || !text.is_char_boundary(a) || !text.is_char_boundary(b) {
                continue;
            }
            if spans.iter().any(|x| a < x.1 && b > x.0) {
                continue;
            }
            let label = &text[a..b];
            let team = |t: TeamRef, ctx: &mut Vec<TeamRef>| {
                if !ctx.contains(&t) {
                    ctx.push(t);
                }
                team_as(t, label)
            };
            let m = match r.to {
                Link::Person(id) if crate::views::person_index(w, id).is_some() => player(id, label),
                Link::Coach(c) if (c as usize) < w.coaches.len() => coach(c as usize, label),
                Link::Program(p) if (p as usize) < w.programs.len() => team(TeamRef::College(p as usize), &mut ctx),
                Link::School(h) if (h as usize) < w.high_schools.len() => {
                    team(TeamRef::HighSchool(h as usize), &mut ctx)
                }
                Link::Club(t) if (t as usize) < w.nfl.len() => team(TeamRef::Nfl(t as usize), &mut ctx),
                Link::League(f) if (f as usize) < afm_engine::world::pro::other::FRANCHISES => {
                    team(TeamRef::Other(f as usize), &mut ctx)
                }
                Link::Game(date, level, home) => {
                    let home = home as usize;
                    // Your own kept game opens its game page; another
                    // team's game opens the home team's schedule.
                    let kept =
                        w.seat_games().iter().position(|g| g.date == date && g.level == level && g.teams[0] == home);
                    let kind = match level {
                        Level::Nfl if home < w.nfl.len() => Some("nfl"),
                        Level::College if home < w.programs.len() => Some("college"),
                        Level::HighSchool if home < w.high_schools.len() => Some("hs"),
                        _ => None,
                    };
                    match (kept, kind) {
                        (Some(k), _) => game(k, label),
                        // Another league's game opens the home club's page.
                        (None, None) if level == Level::Other && home < afm_engine::world::pro::other::FRANCHISES => {
                            team(TeamRef::Other(home), &mut ctx)
                        }
                        (None, Some(kind)) => screen(&format!("schedule?kind={kind}&id={home}"), label),
                        _ => continue,
                    }
                }
                _ => continue,
            };
            spans.push((a, b, m));
        }
        let ws = words(text);
        // A person's name holds its words: a school inside it ("Harrison
        // Lawrence", "Xavier Dopson") is no school, even when the name is
        // too common to link.
        let mut held: Vec<(usize, usize)> = Vec::new();
        let mut i = 0;
        while i < ws.len() {
            let mut took = 1;
            for n in (2..=4).rev() {
                if i + n > ws.len() {
                    continue;
                }
                let span = &ws[i..i + n];
                if !span.iter().all(|&(s, e)| name_word(&text[s..e])) {
                    continue;
                }
                let name = &text[span[0].0..span[n - 1].1];
                // A school named like a person ("James Madison") stays a school.
                let school =
                    self.teams.get(&text[span[0].0..span[0].1]).is_some_and(|v| v.iter().any(|(full, _)| full == name));
                let h = fnv(name);
                if !school && (self.people.contains_key(&h) || self.coaches.contains_key(&h)) {
                    held.push((span[0].0, span[n - 1].1));
                    took = n;
                    break;
                }
            }
            i += took;
        }
        let in_name = |a: usize, b: usize| held.iter().any(|x| a < x.1 && b > x.0);
        // Teams first: the longest name starting at each word.
        let covered = |a: usize, b: usize, spans: &[(usize, usize, String)]| spans.iter().any(|x| a < x.1 && b > x.0);
        let mut i = 0;
        while i < ws.len() {
            let (s, e) = ws[i];
            if covered(s, e, &spans) || in_name(s, e) {
                i += 1;
                continue;
            }
            let first = &text[s..e];
            let mut hit = None;
            if let Some(list) = self.teams.get(first) {
                for (full, t) in list {
                    if text[s..].starts_with(full.as_str()) {
                        let end = s + full.len();
                        let next = text[end..].chars().next();
                        if !next.is_some_and(|c| c.is_alphanumeric()) {
                            // One team per name: two schools sharing it are left plain.
                            let same = list.iter().filter(|x| x.0 == *full).count();
                            if same == 1 || list.iter().filter(|x| x.0 == *full).any(|x| ctx.contains(&x.1)) {
                                let t = if same == 1 {
                                    *t
                                } else {
                                    list.iter().find(|x| x.0 == *full && ctx.contains(&x.1)).map_or(*t, |x| x.1)
                                };
                                hit = Some((end, t));
                            }
                            break;
                        }
                    }
                }
            }
            if let Some((end, t)) = hit.filter(|&(end, _)| !covered(s, end, &spans) && !in_name(s, end)) {
                spans.push((s, end, team_as(t, &text[s..end])));
                if !ctx.contains(&t) {
                    ctx.push(t);
                }
                while i < ws.len() && ws[i].0 < end {
                    i += 1;
                }
            } else {
                i += 1;
            }
        }
        // Then people: the longest run of capitalised words that names one.
        let mut i = 0;
        while i < ws.len() {
            let mut done = false;
            for n in (2..=4).rev() {
                if i + n > ws.len() {
                    continue;
                }
                let span = &ws[i..i + n];
                if !span.iter().all(|&(s, e)| name_word(&text[s..e])) {
                    continue;
                }
                let (a, b) = (span[0].0, span[n - 1].1);
                if covered(a, b, &spans) {
                    continue;
                }
                if let Some(m) = self.who(&text[a..b], &ctx) {
                    spans.push((a, b, m));
                    i += n;
                    done = true;
                    break;
                }
            }
            if !done {
                i += 1;
            }
        }
        if spans.is_empty() {
            return clean_plain(text);
        }
        spans.sort_by_key(|x| x.0);
        let mut out = String::with_capacity(text.len() + spans.len() * 16);
        let mut at = 0;
        for (a, b, m) in spans {
            if a < at {
                continue;
            }
            out.push_str(&clean_plain(&text[at..a]));
            out.push_str(&m);
            at = b;
        }
        out.push_str(&clean_plain(&text[at..]));
        out
    }

    /// A calendar line linked, with its own program or club as context.
    pub fn event(&self, e: &Event) -> String {
        let mut ctx = Vec::new();
        if let Some(p) = e.program
            && p < self.w.programs.len()
        {
            ctx.push(TeamRef::College(p));
        }
        if let Some(t) = e.nfl_team
            && t < self.w.nfl.len()
        {
            ctx.push(TeamRef::Nfl(t));
        }
        self.link_refs(&e.text, &e.refs, &ctx)
    }
}

/// Plain text that must not be read as markup.
fn clean_plain(s: &str) -> String {
    if s.contains("[[") || s.contains("]]") { s.replace("[[", "[").replace("]]", "]") } else { s.to_string() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_keeps_the_words() {
        let s = format!("{} hires {} from {}", player(3, "Iowa"), coach(7, "Mike Tillis"), screen("standings", "9-3"));
        assert_eq!(strip(&s), "Iowa hires Mike Tillis from 9-3");
        assert_eq!(strip("plain [[ text"), "plain [[ text");
    }

    #[test]
    fn words_drop_a_sentence_end() {
        let t = "Jalen Carter commits to Texas.";
        let w: Vec<&str> = words(t).iter().map(|&(s, e)| &t[s..e]).collect();
        assert_eq!(w, ["Jalen", "Carter", "commits", "to", "Texas"]);
        let t = "Tom Smith Jr. signs.";
        let w: Vec<&str> = words(t).iter().map(|&(s, e)| &t[s..e]).collect();
        assert_eq!(w, ["Tom", "Smith", "Jr.", "signs"]);
    }
}
