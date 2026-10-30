//! The fans on screen (people topic 9, "Fan voices"; `world/folk/fans.rs`):
//! the News page's Fans feed (the town after your last game, the call-in
//! show and the message board), a game page's Fans tab (the signs, the
//! crowd play by play and the town after it) and the crowd in the live
//! game. Every line is drawn when the screen opens; none is stored.

use afm_engine::calendar::{Date, Level as CalLevel};
use afm_engine::game::stats::PlayRecord;
use afm_engine::world::World;
use afm_engine::world::folk::Team;
use afm_engine::world::folk::fans::{self, Voice};
use serde_json::{Value, json};

use crate::views;

fn voice_json(v: &Voice) -> Value {
    json!({ "who": v.who, "place": v.place, "text": v.text })
}

/// The News page's Fans feed for your team.
pub(crate) fn feed(w: &World) -> Value {
    let Some(team) = Team::seat(w) else {
        return json!({ "kind": "fans", "rows": [], "fans": Value::Null });
    };
    let x = fans::week(w, team);
    let town = x.town.as_ref().map(|(k, voices)| {
        let g = &w.seat_games[*k];
        let (_, i) = team.parts();
        let side = usize::from(g.teams[0] != i);
        let (us, them) = (g.stats.score[side], g.stats.score[1 - side]);
        let word = match us.cmp(&them) {
            std::cmp::Ordering::Greater => "win",
            std::cmp::Ordering::Less => "loss",
            std::cmp::Ordering::Equal => "tie",
        };
        let at = if side == 0 { "against" } else { "at" };
        json!({
            "index": k,
            "label": format!("After the {us}-{them} {word} {at} {}", g.names[1 - side]),
            "date": views::date_text(g.date),
            "voices": voices.iter().map(voice_json).collect::<Vec<_>>(),
        })
    });
    let shows: Vec<Value> = x
        .shows
        .iter()
        .filter(|s| !s.calls.is_empty())
        .map(|s| {
            json!({
                "date": views::date_text(s.day),
                "name": s.name,
                "calls": s.calls.iter().map(|c| json!({ "caller": c.caller, "intro": c.intro, "text": c.text, "regular": c.regular })).collect::<Vec<_>>(),
            })
        })
        .collect();
    let threads: Vec<Value> = x
        .threads
        .iter()
        .map(|t| {
            json!({
                "date": views::date_text(t.day),
                "title": t.title,
                "subject": t.subject,
                "posts": t.posts.iter().map(|p| json!({ "by": p.by, "text": p.text })).collect::<Vec<_>>(),
            })
        })
        .collect();
    json!({
        "kind": "fans",
        "rows": [],
        "fans": {
            "team": views::team_brief(w, team_ref(team)),
            "mood": x.mood.words(),
            "mood_key": x.mood.key(),
            "board": x.board,
            "station": x.station,
            "town": town,
            "shows": shows,
            "threads": threads,
        },
    })
}

/// A kept game's Fans tab.
pub(crate) fn game(w: &World, k: usize) -> Result<Value, String> {
    let g = w.seat_games().get(k).ok_or("That game is not kept.")?;
    let v = fans::game(w, k).ok_or("No fans for that game.")?;
    let refs = [super::team_ref(g.level, g.teams[0]), super::team_ref(g.level, g.teams[1])];
    let names = crate::lane_boards::game_names(w, g, &refs);
    let name = |id: u32| names.get(&id).map_or(format!("No. {id}"), |x| x.0.clone());
    let crowd: Vec<Value> = v
        .crowd
        .iter()
        .map(|c| {
            let r = g.stats.plays.iter().find(|r| r.n == c.n);
            json!({
                "n": c.n,
                "q": r.map(|r| super::quarter_text(r.quarter, r.quarter > 4)),
                "clock": r.map(|r| super::clock_text(r.clock)),
                "play": r.map(|r| views::play_text(r, &name)),
                "who": r.map(|r| super::page::who(r, &names)),
                "text": c.text,
            })
        })
        .collect();
    Ok(json!({
        "team": views::team_brief(w, team_ref(v.team)),
        "crowd_team": views::team_brief(w, team_ref(v.crowd_team)),
        "mood_before": v.mood_before.words(),
        "mood_after": v.mood_after.words(),
        "signs": v.signs,
        "crowd": crowd,
        "town": v.town.iter().map(voice_json).collect::<Vec<_>>(),
    }))
}

/// The live game's signs and its crowd so far, by play number.
pub(crate) fn live(
    w: &World,
    date: Date,
    level: CalLevel,
    teams: [usize; 2],
    plays: &[PlayRecord],
    end: Option<[i32; 2]>,
) -> (Vec<String>, Vec<(u32, String)>) {
    let Some(team) = Team::seat(w) else { return (Vec::new(), Vec::new()) };
    let (signs, crowd) = fans::live(w, team, date, level, teams, plays, end);
    (signs, crowd.into_iter().map(|c| (c.n, c.text)).collect())
}

fn team_ref(t: Team) -> views::TeamRef {
    match t {
        Team::College(p) => views::TeamRef::College(p),
        Team::Nfl(n) => views::TeamRef::Nfl(n),
        Team::High(h) => views::TeamRef::HighSchool(h),
        Team::Other(f) => views::TeamRef::Other(f),
    }
}
