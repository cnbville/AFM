//! The draft, live in your war room (topic 11): the AI clubs pick at the
//! pace you choose and the draft waits for your club's turn. On the clock
//! the phone rings (clubs below you offering to move up, priced on their
//! own charts), you can call a club above you, take a player from your
//! board, let the clock run out (the next club may jump in, as the Vikings
//! found in 2003) or hand the pick to your staff. After the last pick the
//! undrafted rush opens in the same room: your priority free agents get
//! your calls with the bonus and guarantee you set (topic 12).

use super::super::recruiting::{self, DraftRun};
use crate::world::World;
use crate::world::pro::scouts::{Extra, Offer, Swap};
use crate::world::season::key_uniform;

/// One line in the war room's feed.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Feed {
    pub pick: u32,
    pub round: u32,
    pub team: usize,
    /// The player taken (id), or None for a trade or a note.
    pub player: Option<u32>,
    pub text: String,
}

/// A club's offer for your pick while you're on the clock.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct TradeOffer {
    pub from: usize,
    /// The slot you'd get (index into the board).
    pub their_slot: usize,
    /// A later slot of theirs in this draft, or next year's pick by round.
    pub slot_extra: Option<usize>,
    pub future: Option<u32>,
    /// Who they want, as your staff guesses it.
    pub after: Option<u32>,
}

/// The draft while your war room is open.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct LiveDraft {
    pub run: DraftRun,
    pub team: usize,
    pub feed: Vec<Feed>,
    pub offers: Vec<TradeOffer>,
    /// Seconds used on your current pick (the real clock for the era runs
    /// in the room; the game counts what you've used when you act).
    pub used: u32,
    /// The rush is open: every pick is in.
    pub rush: bool,
    /// Your last pick, for the reveal: (pick, player id).
    pub revealed: Option<(u32, u32)>,
}

/// One pick on the record: pick, round, club, player id, name, position,
/// school, why.
pub type PickLine = (u32, u32, usize, u32, String, String, String, String);

/// The war room's last draft, kept for its page and the look-back.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct DraftRecord {
    pub year: i32,
    pub team: usize,
    /// Every pick: pick, round, club, player id, name, position, school, why.
    pub picks: Vec<PickLine>,
    pub trades: Vec<String>,
    /// The rush: player id, name, club, bonus, whether you'd called him.
    pub rush: Vec<(u32, String, usize, f64, bool)>,
}

/// The pick clock by era, minutes per round (real): 15 in round 1 and 10 in
/// round 2 until 2008; then 10, 7, 5 for rounds 3 to 6 and 4 in round 7;
/// round 1 down to 8 minutes from 2026.
pub fn clock_minutes(year: i32, round: u32) -> u32 {
    match (year, round) {
        (..=2007, 1) => 15,
        (..=2007, _) => 10,
        (2026.., 1) => 8,
        (_, 1) => 10,
        (_, 2) => 7,
        (_, 7) => 4,
        _ => 5,
    }
}

/// Opens the war room on draft day when your seat is at an NFL club that
/// drafts this year. The AI picks run up to your first turn. Returns false
/// (the draft runs whole) without such a seat.
pub(crate) fn start(w: &mut World, run: &mut DraftRun) -> bool {
    let Some(team) = w.seat.filter(|s| s.level == crate::calendar::Level::Nfl).and_then(|s| s.nfl_team) else {
        return false;
    };
    // A club without a pick this year watches the draft on television.
    if !run.board.iter().any(|s| s.0 == team) {
        return false;
    }
    let live = LiveDraft {
        run: std::mem::take(run),
        team,
        feed: Vec::new(),
        offers: Vec::new(),
        used: 0,
        rush: false,
        revealed: None,
    };
    w.scouting.live = Some(live);
    w.scouting.board.for_year(w.year);
    if let Some(l) = &w.scouting.live {
        let grades = grades(w, l);
        let public: std::collections::HashMap<u32, usize> =
            l.run.public_rank.iter().map(|(&i, &r)| (w.people[i].player.id, r)).collect();
        super::lookback::snapshot(w, &grades, &public);
    }
    let text =
        format!("The {} draft is on. Your war room is open: the picks run to your turn and wait for you.", w.year);
    let d = w.today;
    w.calendar.post(
        crate::calendar::Event::new(
            d,
            crate::calendar::Level::Nfl,
            crate::calendar::Kind::DraftPick,
            crate::calendar::Loudness::Huge,
            text,
        )
        .team(team)
        .decision()
        .rule("Your war room"),
    );
    advance(w, Until::Yours);
    true
}

/// How far the room runs on its own.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Until {
    /// One pick.
    Next,
    /// To your club's turn.
    Yours,
    /// To the end of this round (stops at yours on the way).
    Round,
}

/// Whether your club is on the clock now.
pub fn on_clock(l: &LiveDraft) -> bool {
    !l.rush && !l.run.done() && l.run.board[l.run.next as usize - 1].0 == l.team
}

/// The AI picks until `until` or your turn. A club that trades into a pick
/// shows in the feed before its pick.
pub fn advance(w: &mut World, until: Until) {
    let Some(mut l) = w.scouting.live.take() else { return };
    let start_round = l.run.board.get(l.run.next as usize - 1).map_or(0, |s| s.1);
    loop {
        if l.run.done() {
            l.rush = true;
            break;
        }
        if on_clock(&l) {
            if l.offers.is_empty() {
                l.offers = offers(w, &l);
            }
            break;
        }
        let round = l.run.board[l.run.next as usize - 1].1;
        if until == Until::Round && round != start_round {
            break;
        }
        let before = l.run.swaps.len();
        recruiting::draft_step(w, &mut l.run, Some(l.team), None);
        push_feed(w, &mut l, before);
        if until == Until::Next {
            if l.run.done() {
                l.rush = true;
            } else if on_clock(&l) && l.offers.is_empty() {
                l.offers = offers(w, &l);
            }
            break;
        }
    }
    w.scouting.live = Some(l);
}

fn push_feed(w: &World, l: &mut LiveDraft, swaps_before: usize) {
    for s in &l.run.swaps[swaps_before..] {
        let round = l.run.board[s.2 as usize - 1].1;
        l.feed.push(Feed {
            pick: s.2,
            round,
            team: s.0,
            player: None,
            text: format!(
                "Trade: the {} move up to pick {} with the {}, giving {}.",
                w.nfl[s.0].name, s.2, w.nfl[s.1].name, s.3
            ),
        });
    }
    if let Some(&(i, pick, t, ref why)) = l.run.picked.last()
        && l.feed.last().is_none_or(|f| f.pick != pick || f.player.is_none())
    {
        let p = &w.people[i];
        let school = p.college.map_or("", |c| w.program_name(c));
        let short = why.split(';').next().unwrap_or("").to_string();
        l.feed.push(Feed {
            pick,
            round: l.run.board[pick as usize - 1].1,
            team: t,
            player: Some(p.player.id),
            text: format!("The {} take {}, {}, {school}: {short}.", w.nfl[t].name, p.name, p.player.position),
        });
    }
}

/// Your pick: a player from the pool, by id.
pub fn pick(w: &mut World, id: u32) -> Result<(), String> {
    let mut l = w.scouting.live.take().ok_or("The draft isn't live.")?;
    if !on_clock(&l) {
        w.scouting.live = Some(l);
        return Err("You're not on the clock.".into());
    }
    let Some(i) = l.run.pool.iter().map(|x| x.0).find(|&i| w.people[i].player.id == id) else {
        w.scouting.live = Some(l);
        return Err("He's no longer on the board.".into());
    };
    let pk = l.run.next;
    recruiting::draft_step(w, &mut l.run, Some(l.team), Some(i));
    let n = l.run.swaps.len();
    push_feed(w, &mut l, n);
    l.revealed = Some((pk, id));
    l.offers.clear();
    l.used = 0;
    if l.run.done() {
        l.rush = true;
    }
    w.scouting.live = Some(l);
    Ok(())
}

/// Your staff's pick: the top of your board that is still there, unless the
/// position is one your club would never take (a kicker you don't need).
pub fn staff_choice(w: &World, l: &LiveDraft) -> Option<u32> {
    let grades = grades(w, l);
    let order = super::board::ordered(w, &grades);
    let pick = l.run.next;
    order.into_iter().filter(|id| !w.scouting.board.is_dnd(*id)).find(|id| {
        l.run
            .pool
            .iter()
            .find(|x| w.people[x.0].player.id == *id)
            .is_some_and(|x| recruiting::draft_value(w, &l.run, l.team, x.0, pick).is_some())
    })
}

/// Your club's grade of every player left (player id to the aligned grade).
pub fn grades(w: &World, l: &LiveDraft) -> std::collections::HashMap<u32, f64> {
    let b = l.run.boards.get(l.team);
    l.run
        .pool
        .iter()
        .map(|&(i, public)| {
            let g = b.and_then(|b| b.grade.get(&i).copied()).unwrap_or(public);
            (w.people[i].player.id, g)
        })
        .collect()
}

/// The clock ran out: the next club may pick first, and you stay on the
/// clock for the pick after (real: the Vikings in 2003, the Ravens and the
/// Chiefs in 2003 and 2011).
pub fn timeout(w: &mut World) -> Result<(), String> {
    let mut l = w.scouting.live.take().ok_or("The draft isn't live.")?;
    let slot = l.run.next as usize - 1;
    if !on_clock(&l) || slot + 1 >= l.run.board.len() || l.run.board[slot + 1].1 != l.run.board[slot].1 {
        // Nobody behind you in the round: the pick is made for you.
        w.scouting.live = Some(l);
        return auto_pick(w);
    }
    let other = l.run.board[slot + 1].0;
    l.run.board.swap(slot, slot + 1);
    l.feed.push(Feed {
        pick: slot as u32 + 1,
        round: l.run.board[slot].1,
        team: l.team,
        player: None,
        text: format!(
            "Time ran out on the {}. The {} jump in and turn in their card.",
            w.nfl[l.team].name, w.nfl[other].name
        ),
    });
    let before = l.run.swaps.len();
    recruiting::draft_step(w, &mut l.run, Some(l.team), None);
    push_feed(w, &mut l, before);
    l.offers.clear();
    l.used = 0;
    if !l.run.done() && on_clock(&l) {
        l.offers = offers(w, &l);
    }
    w.scouting.live = Some(l);
    Ok(())
}

/// Hands this pick to your staff.
pub fn auto_pick(w: &mut World) -> Result<(), String> {
    let l = w.scouting.live.as_ref().ok_or("The draft isn't live.")?;
    if !on_clock(l) {
        return Err("You're not on the clock.".into());
    }
    match staff_choice(w, l) {
        Some(id) => pick(w, id),
        None => {
            let mut l = w.scouting.live.take().unwrap();
            let before = l.run.swaps.len();
            recruiting::draft_step(w, &mut l.run, None, None);
            push_feed(w, &mut l, before);
            w.scouting.live = Some(l);
            Ok(())
        }
    }
}

/// The offers that ring while you're on the clock: clubs within the next
/// dozen places whose player is worth clearly more to them than what they
/// expect at their own pick, each priced on its own chart (a later pick
/// this year, or next year's), never more than three at once.
fn offers(w: &World, l: &LiveDraft) -> Vec<TradeOffer> {
    let run = &l.run;
    let slot = run.next as usize - 1;
    let pick = run.next;
    let left: Vec<usize> = run.pool.iter().take(300).map(|x| x.0).collect();
    let value = |t: usize, i: usize| recruiting::draft_value(w, run, t, i, pick).map(|x| x.0);
    let top_of = |t: usize, n: usize| -> Vec<(f64, usize)> {
        let mut v: Vec<(f64, usize)> = left.iter().filter_map(|&i| value(t, i).map(|x| (x, i))).collect();
        v.sort_by(|a, b| b.0.total_cmp(&a.0));
        v.truncate(n);
        v
    };
    let teams = crate::world::franchise::active_teams(run.year).len().max(1);
    let seed = w.options.seed as u64 ^ (run.year as u64) << 20 ^ 0x5EA7;
    let mut out = Vec::new();
    let mut seen = vec![l.team];
    for k in slot + 1..(slot + 13).min(run.board.len()) {
        let c = run.board[k].0;
        if seen.contains(&c) || c >= w.clubs.len() {
            continue;
        }
        seen.push(c);
        let theirs = top_of(c, k - slot + 1);
        let (Some(&(want, who)), Some(&(expect, _))) = (theirs.first(), theirs.last()) else { continue };
        let g = &w.clubs[c].office.gm;
        let eager = (want - expect) * (0.6 + 0.5 * g.impatience + 0.4 * g.risk);
        // Not every eager club calls: about half pick up the phone.
        if eager < 2.5 || key_uniform(seed ^ c as u64, slot as u64) > 0.55 + 0.1 * eager.min(3.0) {
            continue;
        }
        // Priced on the buyer's chart: they pay the gap and a little.
        let chart = |s: usize| crate::world::pro::trades::chart(g.classic_chart, s as f64 + 1.0);
        let gap = chart(slot) - chart(k);
        let later = (k + 1..run.board.len()).filter(|&q| run.board[q].0 == c).find(|&q| chart(q) >= 0.9 * gap);
        let (slot_extra, future) = match later {
            Some(q) => (Some(q), None),
            None => {
                let covers = |r: u32| {
                    let mid = ((r - 1) as usize * teams + teams / 2) as f64;
                    crate::world::pro::trades::chart(g.classic_chart, mid) * 0.85 >= 0.9 * gap
                };
                let r = (1..=7u32).rev().find(|&r| covers(r));
                match r {
                    Some(r) if w.market.picks.owner_of(run.year + 1, r, c) == c => (None, Some(r)),
                    _ => continue,
                }
            }
        };
        out.push(TradeOffer { from: c, their_slot: k, slot_extra, future, after: Some(w.people[who].player.id) });
        if out.len() >= 3 {
            break;
        }
    }
    out
}

/// You take an offer: you move down to their slot (and get what they add);
/// the room runs on to your new turn.
pub fn accept(w: &mut World, k: usize) -> Result<(), String> {
    let mut l = w.scouting.live.take().ok_or("The draft isn't live.")?;
    let Some(o) = l.offers.get(k).cloned() else {
        w.scouting.live = Some(l);
        return Err("That offer is gone.".into());
    };
    let pick = l.run.next;
    let sw = Swap {
        buyer: o.from,
        seller: l.team,
        their_slot: o.their_slot,
        extra: o.slot_extra.map(Extra::Slot).or(o.future.map(Extra::Future)),
    };
    let before = l.run.swaps.len();
    recruiting::apply_swap(&mut l.run, pick, &sw);
    push_feed(w, &mut l, before);
    l.offers.clear();
    l.used = 0;
    w.scouting.live = Some(l);
    Ok(())
}

/// What a club above you wants to give you its pick (you call it): your
/// next pick plus what covers the gap on its chart (a later pick of yours
/// this year, or next year's), or None if you have nothing to cover it.
pub fn quote(w: &World, l: &LiveDraft, their_slot: usize) -> Option<TradeOffer> {
    let run = &l.run;
    let at = run.next as usize - 1;
    if their_slot < at || their_slot >= run.board.len() {
        return None;
    }
    let seller = run.board[their_slot].0;
    if seller == l.team || seller >= w.clubs.len() {
        return None;
    }
    let mine = (their_slot + 1..run.board.len()).find(|&q| run.board[q].0 == l.team)?;
    let g = &w.clubs[seller].office.gm;
    let chart = |s: usize| crate::world::pro::trades::chart(g.classic_chart, s as f64 + 1.0);
    // A club asks a little over the chart to move down (about 5%).
    let gap = 1.05 * (chart(their_slot) - chart(mine));
    let teams = crate::world::franchise::active_teams(run.year).len().max(1);
    let later = (mine + 1..run.board.len()).filter(|&q| run.board[q].0 == l.team).find(|&q| chart(q) >= gap);
    let (slot_extra, future) = if gap <= 0.03 * chart(their_slot) {
        (None, None)
    } else {
        match later {
            Some(q) => (Some(q), None),
            None => {
                let covers = |r: u32| {
                    let mid = ((r - 1) as usize * teams + teams / 2) as f64;
                    crate::world::pro::trades::chart(g.classic_chart, mid) * 0.85 >= gap
                };
                let r = (1..=7u32).rev().find(|&r| covers(r))?;
                if w.market.picks.owner_of(run.year + 1, r, l.team) != l.team {
                    return None;
                }
                (None, Some(r))
            }
        }
    };
    Some(TradeOffer { from: seller, their_slot: mine, slot_extra, future, after: None })
}

/// You trade up to `their_slot` on the quote: your next pick and what was
/// added go to them, and their pick comes to you (taken when it comes up).
pub fn trade_up(w: &mut World, their_slot: usize) -> Result<(), String> {
    let mut l = w.scouting.live.take().ok_or("The draft isn't live.")?;
    let Some(q) = quote(w, &l, their_slot) else {
        w.scouting.live = Some(l);
        return Err("They won't move down for what you have.".into());
    };
    let seller = q.from;
    let mine = q.their_slot;
    l.run.board[their_slot].0 = l.team;
    l.run.board[mine].0 = seller;
    let mut what = format!("pick {}", mine + 1);
    let mut future = None;
    if let Some(s) = q.slot_extra {
        l.run.board[s].0 = seller;
        what += &format!(" and pick {}", s + 1);
    }
    if let Some(r) = q.future {
        future = Some(r);
        what += &format!(" and a {} round {r} pick", l.run.year + 1);
    }
    l.run.swaps.push((l.team, seller, their_slot as u32 + 1, what.clone(), future));
    l.run.traded_up.insert(their_slot as u32 + 1);
    l.feed.push(Feed {
        pick: their_slot as u32 + 1,
        round: l.run.board[their_slot].1,
        team: l.team,
        player: None,
        text: format!(
            "Trade: the {} move up to pick {} with the {}, giving {what}.",
            w.nfl[l.team].name,
            their_slot + 1,
            w.nfl[seller].name
        ),
    });
    l.offers.clear();
    w.scouting.live = Some(l);
    Ok(())
}

/// The undrafted rush (topic 12): your calls go out with the terms on your
/// priority free agent list (cut to your bonus pool), every club calls at
/// once, each player takes his best path, and the draft closes.
pub fn rush(w: &mut World) -> Result<DraftRecord, String> {
    let mut l = w.scouting.live.take().ok_or("The draft isn't live.")?;
    if !l.rush {
        // Anything left is picked by your staff first.
        w.scouting.live = Some(l);
        while w.scouting.live.as_ref().is_some_and(|l| !l.rush) {
            advance(w, Until::Yours);
            if w.scouting.live.as_ref().is_some_and(on_clock) {
                auto_pick(w)?;
            }
        }
        l = w.scouting.live.take().ok_or("The draft isn't live.")?;
    }
    let team = l.team;
    let pool_cap = crate::world::pro::scouts::udfa_pool(l.run.year);
    let mut spent = 0.0;
    let mut mine: Vec<(usize, Offer)> = Vec::new();
    let left: std::collections::HashMap<u32, usize> =
        l.run.pool.iter().map(|x| (w.people[x.0].player.id, x.0)).collect();
    let mut pfa = w.scouting.board.pfa.clone();
    if pfa.is_empty() {
        pfa = default_pfa(w, &l);
    }
    for (id, bonus, gtd) in pfa {
        let Some(&i) = left.get(&id) else { continue };
        let b = (bonus as f64).min((pool_cap - spent).max(0.0));
        spent += b;
        mine.push((i, Offer { team, bonus: b, guarantee: gtd as f64 }));
    }
    let called: std::collections::HashSet<usize> = mine.iter().map(|x| x.0).collect();
    let year = l.run.year;
    let picks: Vec<PickLine> = l
        .run
        .picked
        .iter()
        .map(|&(i, pk, t, ref why)| {
            let p = &w.people[i];
            (
                pk,
                l.run.board[pk as usize - 1].1,
                t,
                p.player.id,
                p.name.clone(),
                p.player.position.to_string(),
                p.college.map_or(String::new(), |c| w.program_name(c).to_string()),
                why.clone(),
            )
        })
        .collect();
    let trades: Vec<String> = l.feed.iter().filter(|f| f.player.is_none()).map(|f| f.text.clone()).collect();
    let signed = recruiting::draft_end(w, l.run, Some((team, mine)));
    let rush = signed
        .iter()
        .map(|&(i, o)| (w.people[i].player.id, w.people[i].name.clone(), o.team, o.bonus, called.contains(&i)))
        .collect();
    let rec = DraftRecord { year, team, picks, trades, rush };
    w.scouting.last_draft = Some(rec.clone());
    Ok(rec)
}

/// Your scouts' priority free agents when you set none: the best players
/// left on your board, a few more than you have room for, the bonus pool
/// split toward the top.
pub fn default_pfa(w: &World, l: &LiveDraft) -> Vec<(u32, u32, u32)> {
    let grades = grades(w, l);
    let order = super::board::ordered(w, &grades);
    let room = w.rules.nfl_offseason as usize;
    let have =
        w.clubs.get(l.team).map_or(0, |c| c.roster.len()) + l.run.picked.iter().filter(|x| x.2 == l.team).count();
    let n = room.saturating_sub(have).clamp(4, 20);
    let pool = crate::world::pro::scouts::udfa_pool(l.run.year);
    let min_pay = crate::world::pro::money::minimum(l.run.year, 0);
    let ids: Vec<u32> = order.into_iter().filter(|id| !w.scouting.board.is_dnd(*id)).take(n).collect();
    let k = ids.len();
    let total: f64 = (0..k).map(|j| ((k - j) as f64).powi(2)).sum::<f64>().max(1.0);
    ids.into_iter()
        .enumerate()
        .map(|(j, id)| {
            let bonus = pool * ((k - j) as f64).powi(2) / total;
            let gtd = match j {
                0 => 0.5,
                1 => 0.3,
                2 => 0.15,
                _ => 0.0,
            } * min_pay;
            (id, bonus.round() as u32, gtd.round() as u32)
        })
        .collect()
}

/// The war room closes on its own (Continue pressed elsewhere, a day played
/// from the debug tools): your staff makes every pick left and the rush
/// runs on your list.
pub(crate) fn finish(w: &mut World) {
    if w.scouting.live.is_some() {
        let _ = rush(w);
    }
}
