//! V4, a player's shape: his position's eight skill families as a radar of
//! your staff's ranges, against the level's starters, against himself at
//! his last kept read, or against a teammate at his position; his ratings
//! listed the same way; his staff read against the level with the outlook
//! in words; his facts and what changed since the last kept read.

use afm_engine::attributes::{Attr, Category};
use afm_engine::positions::Position;
use afm_engine::world::norms::{self, Rated};
use afm_engine::world::{Stage, World};
use serde_json::{Value, json};

use super::{any_read, class_full, families, family_read, first_last, mid};
use crate::Reply;
use crate::reads::{self, Read};
use crate::views;

fn category_name(c: Category) -> &'static str {
    match c {
        Category::Physical => "Physical",
        Category::Passing => "Passing",
        Category::Running => "Running",
        Category::Receiving => "Receiving",
        Category::LinePlay => "Line play",
        Category::Defense => "Defense",
        Category::KickingAndSnapping => "Kicking and snapping",
        Category::Mental => "Mental",
    }
}

/// The ratings the page lists for a position: everything the position
/// uses, with the physical and mental ones always.
fn listed(pos: Position) -> Vec<Attr> {
    Attr::ALL
        .iter()
        .copied()
        .filter(|&a| {
            afm_engine::positions::weight(pos, a) > 0.0
                || matches!(a.category(), Category::Physical | Category::Mental)
                || families(pos).iter().any(|f| f.1.contains(&a))
        })
        .collect()
}

/// How sure the staff is, from the reader's eye and how much it has seen.
fn sure_words(own: bool, eye: f64, games: usize) -> &'static str {
    if !own {
        "not sure"
    } else if eye >= 14.0 && games >= 4 {
        "very sure"
    } else if eye >= 9.0 {
        "fairly sure"
    } else {
        "not sure"
    }
}

pub(crate) fn view(w: &World, id: u32) -> Reply {
    let i = views::person_index(w, id).ok_or("That player is no longer in football.")?;
    let p = &w.people[i];
    let pos = p.player.position;
    let (ovr, own, eye) = any_read(w, i);
    let reader = views::reader_key(w);
    let attrs = reads::attributes(&p.player, own, eye, reader);
    let pool = norms::pool_of(w, i);
    let level_mark = |rated: Rated| pool.and_then(|pl| crate::lane_ratings::marks_cached(w, pl, pos, rated));
    let team = views::team_of(w, p);
    // Who the level is, in words.
    let (one, many) = crate::lane_ratings::pos_words(pos);
    let level_words = match pool {
        Some(norms::Pool::Nfl) => format!("NFL starting {many}"),
        Some(norms::Pool::College(0)) => format!("FBS starting {many}"),
        Some(norms::Pool::College(1)) => format!("FCS starting {many}"),
        Some(norms::Pool::College(_)) => format!("small-college starting {many}"),
        Some(norms::Pool::Hs { state: Some(s), .. }) => {
            format!("{} high school starters at {one}", views::state_name(&w.states[s].code))
        }
        _ => format!("starting {many}"),
    };
    let short_level = match pool {
        Some(norms::Pool::Nfl) => "NFL".to_string(),
        Some(norms::Pool::College(0)) => "FBS".into(),
        Some(norms::Pool::College(1)) => "FCS".into(),
        Some(norms::Pool::College(_)) => "Small college".into(),
        Some(norms::Pool::Hs { state: Some(s), .. }) => views::state_name(&w.states[s].code).to_string(),
        _ => "Level".into(),
    };
    // The hero: his overall against the level's starters.
    let om = level_mark(Rated::Overall);
    let band = match (ovr, om.as_ref()) {
        (Read::Range(lo, hi), Some(m)) => {
            let (word, line) = crate::lane_ratings::band(m, lo, hi, &level_words);
            json!({ "word": word, "line": line, "starter": m.starter.round(), "backup": m.backup.round(), "elite": m.elite.round() })
        }
        _ => Value::Null,
    };
    let level = views::level_of(w, p);
    let outlook = reads::potential_words(&p.player, eye, level, reader, w.year);
    let games = super::my_games(w).iter().filter(|(g, side)| own && g.stats.players[*side].contains_key(&id)).count();
    let tape = if own {
        format!(
            "Every practice and {games} game{} of your tape · {}",
            if games == 1 { "" } else { "s" },
            sure_words(own, eye, games)
        )
    } else {
        "Public film and measurements only · not sure".to_string()
    };
    // The families, with each comparison.
    let fams = families(pos);
    let level_of_attr = |a: Attr| level_mark(Rated::Attr(a)).map(|m| m.starter);
    let last =
        w.career.as_ref().and_then(|c| c.kept_of(id).iter().rev().find(|r| r.pos as usize == pos.index()).copied());
    // Teammates at his position (your own roster only: you see them).
    let mates: Vec<usize> = match team {
        Some(t) if own => {
            let mut v: Vec<(f64, usize)> = views::roster_people(w, t)
                .into_iter()
                .filter(|&k| k != i && w.people[k].player.position == pos)
                .map(|k| (mid(super::own_read(w, t, k)).unwrap_or(0.0), k))
                .collect();
            v.sort_by(|a, b| b.0.total_cmp(&a.0));
            v.into_iter().take(8).map(|x| x.1).collect()
        }
        _ => Vec::new(),
    };
    let mate_reads: Vec<Vec<(Attr, Read)>> = mates
        .iter()
        .map(|&k| {
            let q = &w.people[k];
            let e = team.map_or(10.0, |t| views::eye_for(w, t, q.player.position));
            reads::attributes(&q.player, true, e, reader)
        })
        .collect();
    let fam_json: Vec<Value> = fams
        .iter()
        .enumerate()
        .map(|(k, f)| {
            let r = family_read(&attrs, f);
            let lv: Vec<f64> = f.1.iter().filter_map(|&a| level_of_attr(a)).collect();
            let lvl = (!lv.is_empty()).then(|| lv.iter().sum::<f64>() / lv.len() as f64);
            let then = last.map(|l| l.fams[k]).filter(|&x| x > 0);
            let mates: Vec<Value> = mate_reads
                .iter()
                .map(|mr| json!(family_read(mr, f).map(|(lo, hi)| ((lo + hi) / 2.0).round())))
                .collect();
            let from: Vec<Value> =
                f.1.iter()
                    .map(|&a| {
                        let rd = attrs.iter().find(|x| x.0 == a).map_or(Read::NotSeen, |x| x.1);
                        json!({ "label": a.label(), "key": a.key(), "read": rd.json() })
                    })
                    .collect();
            json!({
                "name": f.0, "lo": r.map(|x| x.0.round()), "hi": r.map(|x| x.1.round()),
                "level": lvl.map(f64::round), "last": then, "mates": mates, "from": from,
            })
        })
        .collect();
    // A note for each family, from what the numbers say.
    let mids: Vec<Option<f64>> =
        fam_json.iter().map(|f| f["lo"].as_f64().zip(f["hi"].as_f64()).map(|(a, b)| (a + b) / 2.0)).collect();
    let best =
        mids.iter().enumerate().filter_map(|(k, m)| m.map(|m| (k, m))).max_by(|a, b| a.1.total_cmp(&b.1)).map(|x| x.0);
    let widest = fam_json
        .iter()
        .enumerate()
        .filter_map(|(k, f)| f["hi"].as_f64().zip(f["lo"].as_f64()).map(|(h, l)| (k, h - l)))
        .max_by(|a, b| a.1.total_cmp(&b.1))
        .map(|x| x.0);
    let fam_json: Vec<Value> = fam_json
        .into_iter()
        .enumerate()
        .map(|(k, mut f)| {
            let mut note: Vec<String> = Vec::new();
            match (mids[k], f["level"].as_f64()) {
                (None, _) => note.push("Nobody on your staff has seen enough of it to read.".into()),
                (Some(m), Some(l)) if m - l >= 5.0 => {
                    note.push(format!("Ahead of {level_words} by about {:.0}.", m - l))
                }
                (Some(m), Some(l)) if l - m >= 5.0 => note.push(format!("Behind {level_words} by about {:.0}.", l - m)),
                (Some(_), Some(_)) => note.push(format!("About where {level_words} are.")),
                _ => {}
            }
            if best == Some(k) {
                note.push("His best family.".into());
            }
            if widest == Some(k) && own {
                note.push("The widest read on the page: the staff is least sure here.".into());
            }
            f["note"] = json!(note.join(" "));
            f
        })
        .collect();
    // The ratings list, by category.
    let mut groups: Vec<(Category, Vec<Value>)> = Vec::new();
    for a in listed(pos) {
        let rd = attrs.iter().find(|x| x.0 == a).map_or(Read::NotSeen, |x| x.1);
        let then = last.and_then(|l| {
            l.keys.iter().find(|x| x[0] as usize == a.index()).filter(|x| x[2] > 0).map(|x| [x[1], x[2]])
        });
        let mates: Vec<Value> = mate_reads
            .iter()
            .map(|mr| json!(mr.iter().find(|x| x.0 == a).and_then(|x| mid(x.1)).map(f64::round)))
            .collect();
        let row = json!({
            "label": a.label(), "key": a.key(), "read": rd.json(),
            "level": level_of_attr(a).map(f64::round), "last": then, "mates": mates,
            "fams": fams.iter().enumerate().filter(|(_, f)| f.1.contains(&a)).map(|(k, _)| k).collect::<Vec<_>>(),
        });
        match groups.iter_mut().find(|g| g.0 == a.category()) {
            Some(g) => g.1.push(row),
            None => groups.push((a.category(), vec![row])),
        }
    }
    // What changed since the last kept read.
    let since = last.map(|l| {
        let then = (l.ovr != [0, 0]).then(|| (l.ovr[0] as f64 + l.ovr[1] as f64) / 2.0);
        let mut gains: Vec<(String, f64)> = fams
            .iter()
            .enumerate()
            .filter_map(|(k, f)| {
                let a = mids[k]?;
                let b = l.fams[k];
                (b > 0).then(|| (f.0.to_string(), a - b as f64))
            })
            .collect();
        gains.sort_by(|a, b| b.1.total_cmp(&a.1));
        json!({
            "season": l.season,
            "then": super::pair_json(l.ovr),
            "change": then.zip(mid(ovr)).map(|(a, b)| (b - a).round()),
            "gains": gains.into_iter().take(3).map(|(n, d)| json!({ "name": n, "d": d.round() })).collect::<Vec<_>>(),
        })
    });
    // His facts.
    let b = afm_engine::body::body_now(&p.player);
    let measured = matches!(p.stage, Stage::College | Stage::Declared | Stage::Pro | Stage::Done);
    let forty = super::body::best_forty(w, i);
    let recruited = if p.stars > 0 {
        format!("{} {}, class of {}", p.stars, afm_engine::world::folk::noun(p.stars, "star"), p.class_of)
    } else if p.walk_on {
        format!("Walk-on, class of {}", p.class_of)
    } else {
        format!("Unrated, class of {}", p.class_of)
    };
    let facts = vec![
        json!({ "k": "Age", "v": p.age_on(w.today).to_string() }),
        json!({ "k": "Height", "v": views::ft_in(b.height) }),
        json!({ "k": "Weight", "v": format!("{:.0} lb", b.weight) }),
        json!({ "k": "Arms", "v": if measured { crate::lane_core::pages::eighths(b.arm) } else { "Not measured".into() } }),
        json!({ "k": "40 yards", "v": forty.unwrap_or_else(|| "Not timed".into()) }),
        json!({ "k": "Hometown", "v": views::hometown(w, p) }),
        json!({ "k": "High school", "v": w.high_schools.get(p.hs).map_or(String::new(), |_| w.school_name(p.hs).to_string()) }),
        json!({ "k": "Recruited", "v": recruited }),
    ];
    let (first, lastname) = first_last(&p.name);
    let prev_next = team.filter(|_| own).map(|t| {
        let mut v: Vec<(usize, u32)> = views::roster_people(w, t)
            .into_iter()
            .map(|k| (w.people[k].player.position.index(), w.people[k].player.id))
            .collect();
        v.sort();
        let at = v.iter().position(|x| x.1 == id).unwrap_or(0);
        let n = v.len();
        json!({ "prev": v[(at + n - 1) % n].1, "next": v[(at + 1) % n].1 })
    });
    let kept_season = last.map(|l| l.season);
    Ok(json!({
        "id": id, "name": p.name, "first": first, "last": lastname,
        "pos": pos.name(), "pos_words": one, "class_full": class_full(w, p),
        "own": own,
        "team": team.map(|t| views::team_brief(w, t)),
        "read": ovr.json(), "band": band, "level_words": level_words, "short_level": short_level,
        "outlook": outlook, "tape": tape,
        "families": fam_json,
        "groups": groups.into_iter().map(|(c, rows)| json!({ "name": category_name(c), "rows": rows })).collect::<Vec<_>>(),
        "compare": {
            "level": level_words,
            "level_short": format!("{short_level} starters"),
            "last": kept_season.map(|s| format!("Him at the end of {s}")),
            "last_short": kept_season.map(|s| format!("End of {s}")),
            "mates": mates.iter().map(|&k| json!({ "id": w.people[k].player.id, "name": w.people[k].name })).collect::<Vec<_>>(),
        },
        "since": since,
        "kept_note": if last.is_none() { Some("Your staff keeps a read of every player it follows when each season ends: this compares him with it from then on.") } else { None },
        "facts": facts,
        "nav": prev_next,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_position_lists_physical_and_mental_ratings() {
        for &pos in Position::ALL.iter() {
            let l = listed(pos);
            assert!(l.contains(&Attr::Speed));
            assert!(l.contains(&Attr::Composure));
            // Every family's ratings are on the list.
            for f in families(pos) {
                assert!(f.1.iter().all(|a| l.contains(a)), "{pos:?} {}", f.0);
            }
        }
    }
}
