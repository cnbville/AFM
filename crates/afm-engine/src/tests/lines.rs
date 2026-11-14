//! The line bank (dialogue D2) and a version's season of rest (D3).

use std::collections::{HashMap, HashSet};
use std::time::Instant;

use crate::calendar::Date;
use crate::rng::Rng;
use crate::world::folk::voice::{CLAIM_FACTS, Facts, Pen, REST_DAYS, Speaker, bank};

/// Every tag a speaker can carry (`voice.rs`, the `Speaker` builders). A
/// version tagged with anything else could never be said.
const TAGS: &str = concat!(
    // who
    "player coach you parent ad president principal superintendent owner kin teampresident gm booster reporter ",
    "comms ops academic trainer strength cap personnel feeder chief ",
    // level
    "hs college nfl ",
    // players
    "young vet ego humble leader quiet hot calm loyal greedy pro loose shaky steady competitor brash ",
    // a player's position
    "qb rb wr te ol dl lb db kicker snapper offense defense special skill cover rusher lineman ",
    // where he is in his career
    "freshman sophomore junior senior redshirt transfer walkon rookie undrafted firstround psquad ",
    // coaches
    "climber old expro strict warm cold ",
    // your tone (game day talks)
    "fiery firm ",
    // who decides in a recruit's family (home visits)
    "himself grandparent handler ",
    // agents (honesty, agency size)
    "agent straight slick big small ",
    // the cast
    "she blunt smooth impatient patient meddler tight ",
    // parents
    "loud online absent ",
    // where and when
    "texas south midwest northeast west pre2000 pre2010 social modern nil ",
    // the fans (fans.rs): who they are and how they lean, the radio host, the crowd
    "fan crowd host ticket student alum local homer doomer ",
);

/// Every fact a caller hands the bank (`Facts::set` and `with` across
/// `world/folk` and `world/storylines/tell.rs`). A version that needs any
/// other fact would never be said.
const FACTS: &str = concat!(
    "amateur at_college at_hs at_pro boss boss_title came_up coach coach_last count count_next friend full games ",
    "gameday gone lead lead_last made mark met milestone money n n_words name not_hs nth_week old old_last ",
    "old_school opp other other_name other_short outlet player pos question rank rank_of record record_mark ",
    "result rival rival_last rivalry role scope score season series since since_year starters stars_now stars_was ",
    "stat subject team through tried to_go town unit want weeks word worry writer year years you young ",
    "last_record them captain moment last mistake us first second club ",
    // the fans (fans.rs)
    "defense margin upset streak run mood qb star goat cost next mascot color place at from station late class move draft time ",
    // why your player asked to see you before the portal (talks.rs)
    "reason ",
    // the recruit and portal conversation (pitch.rs): a point's own facts and
    // the rival school that told him the same
    "school spot when rival_school score ",
    // lines that deny or admit something about his own conduct: the
    // one-on-ones set them from his fact sheet (voice.rs, CLAIM_FACTS)
    "blameless at_fault ",
    // a one-on-one on a talking point (talk facts lane L4): the point, his
    // stance, how your move went, and every point's holes and checks
    "point stance went charge when step again outcome by rule punishment ruled why ahead ahead_class mistake ",
    "line award injury life known promise days gap hold trade asked round pc parent prep forty bench squat ",
    "weight forty_before bench_before weight_before test phase grade won lost battles shape pounds position ",
    "stage names arrived change how system grasp picks tie out sport event meet school offers tag what his ",
    "his_games finish margin ",
    // the staff and scouting rooms (claims/rooms.rs, lane L11)
    "yours strong weak worst pick_name pick_round pick_year form allowed scored pressure big ypc run_share ahead ",
    // the bosses and the outside on the fact sheet (lane L10): the case's
    // charge, day, step and whether it is his first; who gives and what a
    // booster asked; who holds the final say and the club's mode; the goal
    // in words and how the season review went
    "charge when step again giver power mode goal review ",
    // your own lines (lane L12): what your rules say comes next, the day it
    // comes, whether he starts, a teammate who looks out for him, and the
    // recruit pitch's holes
    "book when_due starter mate ahead_all best best_round building clubs conference conference_titles first_round ",
    "game guys home hours hs hs_coach leaving miles opens picks_words seasons seats state staying titles u ",
    // the states that pick a pitch's words (pitch.rs `claim_words`)
    "academics depth drive level new seat ",
);

/// The parts the code asks for (talks, press, boss, rules, staffing,
/// takeover, boosters, the parents and the story quotes' last fallbacks).
fn asked() -> Vec<String> {
    let mut v: Vec<String> = Vec::new();
    for m in ["sour", "good", "flat"] {
        v.push(format!("talk.open.{m}"));
        v.push(format!("talk.open.portal.{m}"));
    }
    for s in ["role", "usage", "money", "behavior", "life", "team", "future", "friend"] {
        v.push(format!("talk.open.{s}"));
        v.push(format!("talk.reveal.{s}"));
    }
    // His friend who just left (new bugs, Cain 2026-11-03).
    for o in ["good", "bad", "flat"] {
        v.push(format!("talk.reply.friend.{o}"));
        v.push(format!("talk.reply.friend.hand.{o}"));
    }
    for m in ["ask", "straight", "praise", "challenge", "criticize", "promise", "warn", "hand", "listen"] {
        for o in ["good", "bad", "flat"] {
            v.push(format!("talk.reply.{m}.{o}"));
        }
    }
    for o in ["good", "bad", "flat"] {
        v.push(format!("talk.close.{o}"));
    }
    v.push("talk.deflect".into());
    for q in
        ["vision", "oldcoach", "battle", "lastgame", "injury", "rival", "hotseat", "trouble", "recruiting", "player"]
    {
        v.push(format!("press.q.{q}"));
    }
    for k in [
        "boss.mid.open",
        "boss.goals.open",
        "boss.goals.point",
        "boss.review.good",
        "boss.review.mixed",
        "boss.review.bad",
        "rules.react.harsh",
        "rules.react.soft",
        "rules.react.fair",
        "staff.interview.open",
        "takeover.meeting.you_open",
        "takeover.meeting.react_compete",
        "takeover.meeting.react_honour",
        "takeover.meeting.react_winnow",
        "takeover.meeting.react_purge",
        "takeover.meeting.react_plain",
        "takeover.holdover.pitch",
        "takeover.holdover.detail",
        "takeover.leader.close_to_old",
        "takeover.leader.ego",
        "takeover.leader.steady",
        "takeover.leader.waiting",
        "takeover.oldcoach.greet",
        "booster.greet",
        "booster.small",
        "booster.want.rival",
        "booster.want.access",
        "booster.want.hires",
        "booster.want.player",
        "booster.want.nil",
        "booster.want.clean",
        "booster.want.building",
        "booster.want.oldschool",
        "booster.want.winnow",
        "parents.playing_time",
        "parents.call.open",
        "parents.call.point",
        "story.quote.any.lead",
        "story.quote.any.rival",
    ] {
        v.push(k.into());
    }
    // Game day talks (lane 14).
    for t in ["fiery", "warm", "firm", "calm"] {
        v.push(format!("gameday.open.{t}"));
        v.push(format!("gameday.close.{t}"));
    }
    for m in ["ask", "straight", "praise", "challenge", "criticize", "promise", "warn", "listen", "captain"] {
        v.push(format!("gameday.{m}"));
    }
    for m in ["straight", "praise", "challenge", "criticize", "warn", "listen"] {
        v.push(format!("gameday.word.{m}"));
    }
    for k in ["open", "own", "us", "job", "break"] {
        v.push(format!("gameday.captain.{k}"));
    }
    for k in ["good", "amped", "tight", "flat", "hollow", "captain"] {
        v.push(format!("gameday.staff.{k}"));
    }
    v.push("gameday.quote".into());
    // Talks off the game day (lane 14).
    for k in ["playing", "nil", "school", "distance", "family", "development", "winning"] {
        v.push(format!("homevisit.ask.{k}"));
    }
    v.push("homevisit.tell".into());
    for k in ["playing", "nil", "school", "distance", "family", "development", "winning"] {
        v.push(format!("portal.ask.{k}"));
    }
    v.push("portal.tell".into());
    for m in ["good", "flat", "bad"] {
        v.push(format!("portal.react.{m}"));
    }
    for k in ["knows", "believes", "doubts"] {
        v.push(format!("portal.promise.{k}"));
    }
    // The recruit and portal conversation (talk facts lane L5, pitch.rs): the
    // hand-written new parts and the recruiting lines' parts it asks for.
    for k in [
        "compare.any",
        "react.again.any",
        "pitch.straight.any",
        "react.straight.any",
        "tell.decides",
        "open.your_win",
        "open.your_loss",
        "open.his_game",
        "open.new_offer",
        "open.visit_elsewhere",
        "open.commit_at_spot",
        "open.coach_left",
        "open.entered_portal",
        "record.ask",
        "record.strong",
        "record.mixed",
        "record.weak",
        "you.new",
        "you.long",
        "want.second",
        "want.unsure",
        "want.dealbreaker",
        "history.first",
        "history.again",
        "history.silence",
        "team.good_year",
        "team.bad_year",
        "committed.you",
        "committed.elsewhere",
        "other.money",
        "other.closer",
        "other.start",
        "other.you_money",
    ] {
        v.push(format!("recruit.{k}"));
    }
    for s in ["call", "visit"] {
        for m in ["warm", "polite", "guarded"] {
            v.push(format!("recruit.open.{s}.{m}"));
        }
        for m in ["warm", "flat", "cool"] {
            v.push(format!("recruit.close.{s}.{m}"));
        }
    }
    for a in ["atmosphere", "brand", "campus", "conference", "facilities", "prestige", "tradition"] {
        for m in ["good", "cool", "weak"] {
            v.push(format!("recruit.aspect.{a}.{m}"));
        }
    }
    for b in [
        "ambitious",
        "bloodline_college",
        "bloodline_nfl",
        "brother_follow",
        "brother_own",
        "death",
        "family_move",
        "foster",
        "grades_low",
        "grandparents",
        "job_loss",
        "mentor",
        "money_fine",
        "money_tight",
        "new_baby",
        "nil_changed",
        "single_parent",
    ] {
        v.push(format!("recruit.bg.{b}"));
    }
    for k in ["choice", "coach", "grad", "money", "snaps", "up"] {
        v.push(format!("transfer.why.{k}"));
    }
    for k in ["starter", "backup"] {
        v.push(format!("transfer.role.{k}"));
    }
    for m in ["good", "flat", "bad"] {
        v.push(format!("homevisit.react.{m}"));
        v.push(format!("interview.react.{m}"));
    }
    for k in ["football", "grades", "character"] {
        v.push(format!("hscall.ask.{k}"));
    }
    for k in ["plan", "staff", "past"] {
        v.push(format!("interview.ask.{k}"));
    }
    for k in ["pressure", "zone", "run", "pass"] {
        v.push(format!("staffmeet.case.{k}"));
        v.push(format!("staffmeet.case.{k}.facts"));
    }
    // The rooms read the facts (lane L11).
    for k in ["plan", "past"] {
        v.push(format!("interview.ask.{k}.facts"));
    }
    for k in ["failure", "results", "people"] {
        v.push(format!("interview.react.past.{k}"));
        v.push(format!("interview.react.past.against.{k}"));
    }
    v.push("takeover.holdover.record".into());
    for k in ["role", "role.ahead", "winning.up", "winning.down", "city", "security", "home", "loyalty"] {
        v.push(format!("agent.wants.{k}"));
    }
    for k in ["yes", "half", "no"] {
        v.push(format!("staffmeet.reply.{k}"));
    }
    for k in ["steady", "bristles", "shaken"] {
        v.push(format!("combine.press.{k}"));
    }
    for k in ["yes", "range", "apart", "far", "near", "short", "insult", "done"] {
        v.push(format!("agent.{k}"));
    }
    for k in ["real", "bluff", "market", "stay"] {
        v.push(format!("agent.claim.{k}"));
    }
    // The bosses and the outside on the facts (lane L10, outside_facts.txt).
    for k in ["arrest", "charged", "court"] {
        v.push(format!("press.q.trouble.{k}"));
    }
    for k in ["deflect", "warn", "praise"] {
        v.push(format!("press.follow.trouble.{k}"));
    }
    for k in ["ahead", "on", "behind", "injury_true", "injury_thin"] {
        v.push(format!("boss.mid.reply.{k}"));
    }
    for k in ["higher", "fair", "lowered", "kept"] {
        v.push(format!("boss.goals.reply.{k}"));
    }
    for k in ["own", "point", "ask"] {
        v.push(format!("boss.review.reply.{k}"));
    }
    for k in ["promise", "straight", "business", "thanks"] {
        v.push(format!("booster.reply.{k}"));
    }
    for k in ["help", "respect", "mine"] {
        v.push(format!("takeover.oldcoach.reply.{k}"));
    }
    for k in ["say", "need", "never"] {
        v.push(format!("gm.reply.{k}"));
    }
    // The fans (fans.rs).
    v.extend(crate::world::folk::fans::parts());
    // One-on-ones on his talking points (talk facts lane L4): the parts
    // written by hand; the others fall back to today's lines until L12.
    for (point, stances) in [
        ("arrest", &["owns", "plays_down", "denies", "defiant", "shaken"][..]),
        ("lost_job", &["accepts", "pushes_back", "sulks", "fired_up"][..]),
        ("won_job", &["humble", "proud"][..]),
        ("big_game", &["humble", "proud"][..]),
        ("bad_game", &["accepts", "pushes_back", "sulks", "fired_up"][..]),
        ("hurt", &["opens_up", "guarded"][..]),
        ("good", &["humble", "proud"][..]),
    ] {
        for st in stances {
            v.push(format!("talk.point.open.{point}.{st}"));
        }
    }
    for q in ["lost_job", "buried", "promise_open", "draft", "your_job"] {
        v.push(format!("talk.point.ask.{q}"));
    }
    v.push("talk.point.again".into());
    // Your own lines (lane L12, own_lines_run.txt): the moves and answers on
    // a point, the moves on a subject and the pitch, each asked by the code.
    v.extend(YOUR_PARTS.iter().map(|k| k.to_string()));
    v
}

const YOUR_PARTS: &[&str] = &[
    "recruit.pitch.after_football.elite",
    "recruit.pitch.after_football.fair",
    "recruit.pitch.after_football.strong",
    "recruit.pitch.after_football.weak",
    "recruit.pitch.brand.elite",
    "recruit.pitch.brand.fair",
    "recruit.pitch.brand.strong",
    "recruit.pitch.brand.weak",
    "recruit.pitch.brother.elite",
    "recruit.pitch.brother.fair",
    "recruit.pitch.brother.strong",
    "recruit.pitch.brother.weak",
    "recruit.pitch.buildings.elite",
    "recruit.pitch.buildings.fair",
    "recruit.pitch.buildings.strong",
    "recruit.pitch.buildings.weak",
    "recruit.pitch.childhood_team.elite",
    "recruit.pitch.childhood_team.fair",
    "recruit.pitch.childhood_team.strong",
    "recruit.pitch.childhood_team.weak",
    "recruit.pitch.conference.elite",
    "recruit.pitch.conference.fair",
    "recruit.pitch.conference.strong",
    "recruit.pitch.conference.weak",
    "recruit.pitch.degree.elite",
    "recruit.pitch.degree.fair",
    "recruit.pitch.degree.strong",
    "recruit.pitch.degree.weak",
    "recruit.pitch.drive.elite",
    "recruit.pitch.drive.fair",
    "recruit.pitch.drive.strong",
    "recruit.pitch.drive.weak",
    "recruit.pitch.exposure.elite",
    "recruit.pitch.exposure.fair",
    "recruit.pitch.exposure.strong",
    "recruit.pitch.exposure.weak",
    "recruit.pitch.family_need.elite",
    "recruit.pitch.family_need.fair",
    "recruit.pitch.family_need.strong",
    "recruit.pitch.family_need.weak",
    "recruit.pitch.home_state.elite",
    "recruit.pitch.home_state.fair",
    "recruit.pitch.home_state.strong",
    "recruit.pitch.home_state.weak",
    "recruit.pitch.hs_coach_tie.elite",
    "recruit.pitch.hs_coach_tie.fair",
    "recruit.pitch.hs_coach_tie.strong",
    "recruit.pitch.hs_coach_tie.weak",
    "recruit.pitch.hs_mates.elite",
    "recruit.pitch.hs_mates.fair",
    "recruit.pitch.hs_mates.strong",
    "recruit.pitch.hs_mates.weak",
    "recruit.pitch.offer_others.elite",
    "recruit.pitch.offer_others.fair",
    "recruit.pitch.offer_others.strong",
    "recruit.pitch.offer_others.weak",
    "recruit.pitch.offer_rate.elite",
    "recruit.pitch.offer_rate.fair",
    "recruit.pitch.offer_rate.strong",
    "recruit.pitch.offer_rate.weak",
    "recruit.pitch.picks_all.elite",
    "recruit.pitch.picks_all.fair",
    "recruit.pitch.picks_all.strong",
    "recruit.pitch.picks_all.weak",
    "recruit.pitch.picks_position.elite",
    "recruit.pitch.picks_position.fair",
    "recruit.pitch.picks_position.strong",
    "recruit.pitch.picks_position.weak",
    "recruit.pitch.position_coach.elite",
    "recruit.pitch.position_coach.fair",
    "recruit.pitch.position_coach.strong",
    "recruit.pitch.position_coach.weak",
    "recruit.pitch.position_u.elite",
    "recruit.pitch.position_u.fair",
    "recruit.pitch.position_u.strong",
    "recruit.pitch.position_u.weak",
    "recruit.pitch.pro_day.elite",
    "recruit.pitch.pro_day.fair",
    "recruit.pitch.pro_day.strong",
    "recruit.pitch.pro_day.weak",
    "recruit.pitch.qb_rivals.elite",
    "recruit.pitch.qb_rivals.fair",
    "recruit.pitch.qb_rivals.strong",
    "recruit.pitch.qb_rivals.weak",
    "recruit.pitch.rival.elite",
    "recruit.pitch.rival.fair",
    "recruit.pitch.rival.strong",
    "recruit.pitch.rival.weak",
    "recruit.pitch.role.elite",
    "recruit.pitch.role.fair",
    "recruit.pitch.role.strong",
    "recruit.pitch.role.weak",
    "recruit.pitch.room.elite",
    "recruit.pitch.room.fair",
    "recruit.pitch.room.strong",
    "recruit.pitch.room.weak",
    "recruit.pitch.scheme.elite",
    "recruit.pitch.scheme.fair",
    "recruit.pitch.scheme.strong",
    "recruit.pitch.scheme.weak",
    "recruit.pitch.stadium.elite",
    "recruit.pitch.stadium.fair",
    "recruit.pitch.stadium.strong",
    "recruit.pitch.stadium.weak",
    "recruit.pitch.straight.any",
    "recruit.pitch.straight.home",
    "recruit.pitch.straight.league",
    "recruit.pitch.straight.money",
    "recruit.pitch.straight.people",
    "recruit.pitch.straight.playing_time",
    "recruit.pitch.straight.school",
    "recruit.pitch.straight.winning",
    "recruit.pitch.support.elite",
    "recruit.pitch.support.fair",
    "recruit.pitch.support.strong",
    "recruit.pitch.support.weak",
    "recruit.pitch.tenure.elite",
    "recruit.pitch.tenure.fair",
    "recruit.pitch.tenure.strong",
    "recruit.pitch.tenure.weak",
    "recruit.pitch.title_odds.elite",
    "recruit.pitch.title_odds.fair",
    "recruit.pitch.title_odds.strong",
    "recruit.pitch.title_odds.weak",
    "recruit.pitch.titles.elite",
    "recruit.pitch.titles.fair",
    "recruit.pitch.titles.strong",
    "recruit.pitch.titles.weak",
    "recruit.pitch.why_left.elite",
    "recruit.pitch.why_left.fair",
    "recruit.pitch.why_left.strong",
    "recruit.pitch.why_left.weak",
    "recruit.pitch.your_guys.elite",
    "recruit.pitch.your_guys.fair",
    "recruit.pitch.your_guys.strong",
    "recruit.pitch.your_guys.weak",
    "recruit.pitch.your_word.elite",
    "recruit.pitch.your_word.fair",
    "recruit.pitch.your_word.strong",
    "recruit.pitch.your_word.weak",
    "talk.point.answer.draft.dodge",
    "talk.point.answer.draft.soften",
    "talk.point.answer.draft.straight",
    "talk.point.answer.lost_job.dodge",
    "talk.point.answer.lost_job.promise",
    "talk.point.answer.lost_job.soften",
    "talk.point.answer.lost_job.straight",
    "talk.point.answer.promise_open.dodge",
    "talk.point.answer.promise_open.promise",
    "talk.point.answer.promise_open.soften",
    "talk.point.answer.promise_open.straight",
    "talk.point.answer.your_job.dodge",
    "talk.point.answer.your_job.promise",
    "talk.point.answer.your_job.soften",
    "talk.point.answer.your_job.straight",
    "talk.point.you.ahead.ask",
    "talk.point.you.ahead.challenge",
    "talk.point.you.ahead.straight",
    "talk.point.you.arrest.ask",
    "talk.point.you.arrest.hand",
    "talk.point.you.arrest.promise",
    "talk.point.you.arrest.straight",
    "talk.point.you.arrest.warn",
    "talk.point.you.award.ask",
    "talk.point.you.award.challenge",
    "talk.point.you.award.praise",
    "talk.point.you.back.ask",
    "talk.point.you.back.challenge",
    "talk.point.you.back.straight",
    "talk.point.you.bad_game.ask",
    "talk.point.you.bad_game.challenge",
    "talk.point.you.bad_game.criticize",
    "talk.point.you.bad_game.straight",
    "talk.point.you.big_game.ask",
    "talk.point.you.big_game.challenge",
    "talk.point.you.big_game.praise",
    "talk.point.you.birth.ask",
    "talk.point.you.birth.hand",
    "talk.point.you.birth.listen",
    "talk.point.you.buried.ask",
    "talk.point.you.buried.challenge",
    "talk.point.you.buried.promise",
    "talk.point.you.buried.straight",
    "talk.point.you.case_closed.ask",
    "talk.point.you.case_closed.challenge",
    "talk.point.you.case_closed.hand",
    "talk.point.you.case_closed.listen",
    "talk.point.you.case_closed.straight",
    "talk.point.you.case_closed.warn",
    "talk.point.you.chart_move.ask",
    "talk.point.you.chart_move.challenge",
    "talk.point.you.chart_move.praise",
    "talk.point.you.chart_move.straight",
    "talk.point.you.colleges.ask",
    "talk.point.you.colleges.praise",
    "talk.point.you.colleges.straight",
    "talk.point.you.death.ask",
    "talk.point.you.death.hand",
    "talk.point.you.death.listen",
    "talk.point.you.death.promise",
    "talk.point.you.divorce.ask",
    "talk.point.you.divorce.hand",
    "talk.point.you.divorce.listen",
    "talk.point.you.divorce.promise",
    "talk.point.you.draft.ask",
    "talk.point.you.draft.challenge",
    "talk.point.you.draft.listen",
    "talk.point.you.draft.straight",
    "talk.point.you.effort.ask",
    "talk.point.you.effort.challenge",
    "talk.point.you.effort.criticize",
    "talk.point.you.effort.straight",
    "talk.point.you.failed_term.ask",
    "talk.point.you.failed_term.hand",
    "talk.point.you.failed_term.straight",
    "talk.point.you.failed_term.warn",
    "talk.point.you.grades.ask",
    "talk.point.you.grades.hand",
    "talk.point.you.grades.straight",
    "talk.point.you.grades.warn",
    "talk.point.you.homesick.ask",
    "talk.point.you.homesick.hand",
    "talk.point.you.homesick.listen",
    "talk.point.you.homesick.promise",
    "talk.point.you.hurt.ask",
    "talk.point.you.hurt.listen",
    "talk.point.you.hurt.promise",
    "talk.point.you.hurt.straight",
    "talk.point.you.lost_job.ask",
    "talk.point.you.lost_job.challenge",
    "talk.point.you.lost_job.listen",
    "talk.point.you.lost_job.promise",
    "talk.point.you.lost_job.straight",
    "talk.point.you.money_lost.ask",
    "talk.point.you.money_lost.hand",
    "talk.point.you.money_lost.listen",
    "talk.point.you.money_lost.promise",
    "talk.point.you.new_coach.ask",
    "talk.point.you.new_coach.listen",
    "talk.point.you.new_coach.straight",
    "talk.point.you.new_position.ask",
    "talk.point.you.new_position.challenge",
    "talk.point.you.new_position.praise",
    "talk.point.you.new_position.straight",
    "talk.point.you.new_system.ask",
    "talk.point.you.new_system.challenge",
    "talk.point.you.new_system.praise",
    "talk.point.you.new_system.straight",
    "talk.point.you.other_sport.ask",
    "talk.point.you.other_sport.praise",
    "talk.point.you.other_sport.straight",
    "talk.point.you.parent_called.ask",
    "talk.point.you.parent_called.listen",
    "talk.point.you.parent_called.straight",
    "talk.point.you.pay.ask",
    "talk.point.you.pay.listen",
    "talk.point.you.pay.promise",
    "talk.point.you.pay.straight",
    "talk.point.you.portal.ask",
    "talk.point.you.portal.challenge",
    "talk.point.you.portal.promise",
    "talk.point.you.portal.straight",
    "talk.point.you.practice_week.ask",
    "talk.point.you.practice_week.challenge",
    "talk.point.you.practice_week.praise",
    "talk.point.you.practice_week.straight",
    "talk.point.you.pro_spring.ask",
    "talk.point.you.pro_spring.challenge",
    "talk.point.you.pro_spring.listen",
    "talk.point.you.pro_spring.straight",
    "talk.point.you.promise_broken.ask",
    "talk.point.you.promise_broken.listen",
    "talk.point.you.promise_broken.promise",
    "talk.point.you.promise_broken.straight",
    "talk.point.you.promise_open.ask",
    "talk.point.you.promise_open.listen",
    "talk.point.you.promise_open.straight",
    "talk.point.you.report_day.challenge",
    "talk.point.you.report_day.criticize",
    "talk.point.you.report_day.praise",
    "talk.point.you.report_day.straight",
    "talk.point.you.room_changed.ask",
    "talk.point.you.room_changed.challenge",
    "talk.point.you.room_changed.straight",
    "talk.point.you.rule_break.ask",
    "talk.point.you.rule_break.criticize",
    "talk.point.you.rule_break.straight",
    "talk.point.you.rule_break.warn",
    "talk.point.you.season_ended.ask",
    "talk.point.you.season_ended.challenge",
    "talk.point.you.season_ended.praise",
    "talk.point.you.season_ended.straight",
    "talk.point.you.sick_parent.ask",
    "talk.point.you.sick_parent.hand",
    "talk.point.you.sick_parent.listen",
    "talk.point.you.sick_parent.promise",
    "talk.point.you.slump.ask",
    "talk.point.you.slump.challenge",
    "talk.point.you.slump.listen",
    "talk.point.you.slump.straight",
    "talk.point.you.spring_step.ask",
    "talk.point.you.spring_step.challenge",
    "talk.point.you.spring_step.praise",
    "talk.point.you.spring_step.straight",
    "talk.point.you.suspended.challenge",
    "talk.point.you.suspended.listen",
    "talk.point.you.suspended.promise",
    "talk.point.you.suspended.straight",
    "talk.point.you.teammate_drafted.ask",
    "talk.point.you.teammate_drafted.challenge",
    "talk.point.you.teammate_drafted.straight",
    "talk.point.you.testing.ask",
    "talk.point.you.testing.challenge",
    "talk.point.you.testing.praise",
    "talk.point.you.testing.straight",
    "talk.point.you.wants_out.ask",
    "talk.point.you.wants_out.challenge",
    "talk.point.you.wants_out.promise",
    "talk.point.you.wants_out.straight",
    "talk.point.you.wedding.ask",
    "talk.point.you.wedding.hand",
    "talk.point.you.wedding.listen",
    "talk.point.you.won_job.ask",
    "talk.point.you.won_job.challenge",
    "talk.point.you.won_job.praise",
    "talk.you.behavior.ask",
    "talk.you.behavior.criticize",
    "talk.you.behavior.listen",
    "talk.you.behavior.praise",
    "talk.you.behavior.promise",
    "talk.you.behavior.warn",
    "talk.you.friend.ask",
    "talk.you.friend.hand",
    "talk.you.friend.listen",
    "talk.you.friend.praise",
    "talk.you.friend.promise",
    "talk.you.friend.straight",
    "talk.you.future.ask",
    "talk.you.future.challenge",
    "talk.you.future.listen",
    "talk.you.future.praise",
    "talk.you.future.promise",
    "talk.you.future.straight",
    "talk.you.life.ask",
    "talk.you.life.hand",
    "talk.you.life.listen",
    "talk.you.life.praise",
    "talk.you.life.promise",
    "talk.you.money.ask",
    "talk.you.money.listen",
    "talk.you.money.praise",
    "talk.you.money.promise",
    "talk.you.money.straight",
    "talk.you.role.ask",
    "talk.you.role.challenge",
    "talk.you.role.listen",
    "talk.you.role.praise",
    "talk.you.role.promise",
    "talk.you.role.straight",
    "talk.you.team.ask",
    "talk.you.team.hand",
    "talk.you.team.listen",
    "talk.you.team.praise",
    "talk.you.team.promise",
    "talk.you.usage.ask",
    "talk.you.usage.challenge",
    "talk.you.usage.listen",
    "talk.you.usage.praise",
    "talk.you.usage.promise",
    "talk.you.usage.straight",
];

#[test]
fn the_bank_reads_clean() {
    let b = bank();
    let tags: HashSet<&str> = TAGS.split_whitespace().collect();
    let facts: HashSet<&str> = FACTS.split_whitespace().collect();
    let mut bad: Vec<String> = Vec::new();
    for (key, _) in b.keys() {
        for (text, need, not, fs, when) in b.versions(key) {
            for f in when {
                if !facts.contains(f) {
                    bad.push(format!("{key}: said on an unknown fact {f}: {text}"));
                }
            }
            for t in need.iter().chain(not) {
                if !tags.contains(t.as_str()) {
                    bad.push(format!("{key}: unknown tag {t}: {text}"));
                }
            }
            for f in fs {
                if !facts.contains(f.as_str()) {
                    bad.push(format!("{key}: unknown fact {{{f}}}: {text}"));
                }
            }
            let mut depth = 0i32;
            for c in text.chars() {
                match c {
                    '[' | '{' => depth += 1,
                    ']' | '}' => depth -= 1,
                    '|' if depth == 0 => bad.push(format!("{key}: a choice bar outside brackets: {text}")),
                    _ => {}
                }
                if !(0..=1).contains(&depth) {
                    bad.push(format!("{key}: brackets inside brackets or closed twice: {text}"));
                    break;
                }
            }
            if depth != 0 {
                bad.push(format!("{key}: a bracket left open: {text}"));
            }
            if text.contains('"') {
                bad.push(format!("{key}: quote marks (the screen adds them): {text}"));
            }
        }
    }
    assert!(bad.is_empty(), "{} bad versions, first: {:#?}", bad.len(), &bad[..bad.len().min(20)]);
    assert_eq!(b.repeats(), 0, "a version written twice under one part");
}

#[test]
fn every_part_the_code_asks_for_has_lines() {
    let b = bank();
    let thin: Vec<String> = asked()
        .into_iter()
        .filter(|k| b.versions(k).len() < 6)
        .map(|k| format!("{k}: {}", b.versions(&k).len()))
        .collect();
    assert!(thin.is_empty(), "parts with fewer than 6 versions: {thin:?}");
}

/// Blatant ways a man denies or admits something about his own conduct
/// (lowercase, plain apostrophes). A one-on-one line that says one of them
/// must carry `blameless=yes` or `at_fault=yes`: the bank is fact-free, so
/// "I've done nothing wrong" would be said to a man with a fresh arrest, and
/// "I know I messed up" to one who did nothing (talk facts, lane L2). The
/// word scan and the reading that tagged the bank found more than these;
/// this list is the floor a later bank run can't slip under.
const DENIALS: &[&str] = &[
    "i haven't done anything wrong",
    "i've done nothing wrong",
    "i didn't do anything wrong",
    "i did nothing wrong",
    "i'm not in trouble",
    "i'm clean",
    "nothing to hide",
    "it wasn't me",
    "that wasn't me",
    "it's not my fault",
    "that's not my fault",
    "i didn't do it",
    "i'm innocent",
];
const ADMISSIONS: &[&str] = &[
    "i messed up",
    "i screwed up",
    "i blew it",
    "my fault",
    "i was wrong",
    "i let you down",
    "i let the team down",
    "i crossed a line",
    "i take the blame",
    "i'm to blame",
];

/// The blatant claim a line makes, if it makes one outright: not in a
/// question, an "if" ("If I messed up, tell me") or a hope ("I hope I'm not
/// in trouble"), and not about someone else ("I was wrong about you").
fn claim_in(text: &str) -> Option<&'static str> {
    let low = text.to_lowercase().replace('\u{2019}', "'");
    for p in DENIALS.iter().chain(ADMISSIONS) {
        let mut from = 0;
        while let Some(at) = low[from..].find(p) {
            let at = from + at;
            let (before, after) = (&low[..at], &low[at + p.len()..]);
            let word_start = before.chars().next_back().is_none_or(|c| !c.is_alphanumeric());
            let word_end = after.chars().next().is_none_or(|c| !c.is_alphanumeric());
            let asks = ["if ", "did ", "whether ", "in case ", "hope ", "because ", "glad "]
                .iter()
                .any(|g| before.ends_with(g));
            let hedged = after.starts_with(" about") || after.starts_with(" if");
            if word_start && word_end && !asks && !hedged {
                return Some(p);
            }
            from = at + p.len();
        }
    }
    None
}

#[test]
fn a_one_on_one_line_that_claims_his_conduct_waits_for_the_facts() {
    let b = bank();
    let mut loose: Vec<String> = Vec::new();
    let mut held = 0;
    for (key, _) in b.keys() {
        if !key.starts_with("talk.") {
            continue;
        }
        for (text, _, _, _, when) in b.versions(key) {
            if when.iter().any(|f| CLAIM_FACTS.contains(f)) {
                held += 1;
            } else if let Some(p) = claim_in(text) {
                loose.push(format!("{key}: \"{p}\": {text}"));
            }
        }
    }
    assert!(
        loose.is_empty(),
        "{} lines claim his conduct with no fact behind them, first: {:#?}",
        loose.len(),
        &loose[..loose.len().min(20)]
    );
    assert!(held >= 1_000, "the held lines are in the bank ({held})");
}

#[test]
fn held_lines_come_back_when_a_fact_backs_them() {
    // A held line is said only when blameless or at_fault is set; set them
    // and the lines are there (the one-on-ones set them from his fact
    // sheet, talk facts lane L4).
    let b = bank();
    let plain = Facts::new();
    let backed = Facts::new().with("blameless", "yes").with("at_fault", "yes");
    let mut back = 0;
    for level in ["hs", "college", "nfl"] {
        let who = Speaker { key: 1, tags: vec!["player", level, "social", "modern", "nil"] };
        for (key, _) in b.keys() {
            if !key.starts_with("talk.") {
                continue;
            }
            let (a, z) = (b.fitting(key, &who, &plain), b.fitting(key, &who, &backed));
            assert!(z >= a, "{level}: {key} says fewer lines when the facts are set");
            back += z - a;
        }
    }
    assert!(back >= 300, "setting the facts brings back the held lines ({back})");
}

fn player(key: u64, tags: &[&'static str]) -> Speaker {
    let mut t: Vec<&'static str> = vec!["player", "social", "modern", "nil"];
    t.extend_from_slice(tags);
    Speaker { key, tags: t }
}

#[test]
fn a_version_rests_a_full_season() {
    let who = player(7, &["college"]);
    let f = Facts::new().with("coach", "Coach Reed").with("name", "Marcus").with("you", "Reed");
    let key = "talk.reply.praise.good";
    let n = bank().versions(key).len();
    let can: usize = {
        // How many versions this speaker can say.
        let mut p = Pen::on(1, Date::ymd(2026, 8, 1), Vec::new());
        let mut seen = HashSet::new();
        for _ in 0..n * 40 {
            seen.insert(p.part(key, &who, &f).unwrap());
        }
        seen.len()
    };
    assert!(can >= 2);
    // A season's memory: every pick is new until he has said them all.
    let day0 = Date::ymd(2026, 8, 1);
    let mut p = Pen::on(2, day0, Vec::new());
    let mut said: Vec<u32> = Vec::new();
    for k in 0..can {
        p.set_day(Date(day0.0 + k as i32));
        p.part(key, &who, &f).unwrap();
        let mem = p.memories();
        let ids: Vec<u32> = mem.iter().find(|x| x.0 == 7).unwrap().1.iter().map(|e| e.0).collect();
        assert_eq!(ids.len(), k + 1, "a version came back inside the season");
        said = ids;
        p = Pen::on(2 + k as u64, Date(day0.0 + k as i32), mem);
    }
    // With all said, the next is the one said longest ago.
    p.set_day(Date(day0.0 + can as i32));
    p.part(key, &who, &f).unwrap();
    let mem = p.memories();
    let last = mem.iter().find(|x| x.0 == 7).unwrap().1.last().unwrap().0;
    assert_eq!(last, said[0], "the oldest version comes back first");
    // A season after the first was said, it is fresh again; another
    // speaker's memory is his own.
    let mut p = Pen::on(9, Date(day0.0 + REST_DAYS + 1), mem);
    let other = player(8, &["college"]);
    p.part(key, &other, &f).unwrap();
    let mem = p.memories();
    assert_eq!(mem.iter().find(|x| x.0 == 8).unwrap().1.len(), 1);
}

#[test]
fn the_rest_changes_the_words_only() {
    // The caller picks the part (what he says); the rest only picks among
    // that part's versions, so every line is still one of them.
    let who = player(11, &["nfl", "vet", "ego"]);
    let f = Facts::new().with("coach", "Coach Reed").with("name", "Dre").with("you", "Reed");
    for key in ["talk.reply.criticize.bad", "talk.reply.promise.good", "talk.deflect"] {
        let words: HashSet<String> = bank()
            .versions(key)
            .iter()
            .map(|v| v.0.split(['[', '{']).next().unwrap_or("").trim().to_string())
            .collect();
        let mut p = Pen::on(3, Date::ymd(2026, 9, 1), Vec::new());
        for k in 0..60 {
            p.set_day(Date(Date::ymd(2026, 9, 1).0 + k));
            let s = p.part(key, &who, &f).unwrap();
            assert!(
                words.iter().any(|w| w.is_empty() || s.starts_with(w.as_str())),
                "{key}: {s} is not one of its versions"
            );
        }
    }
    // A pen never touches any stream but its own: the same world stream
    // reads the same before and after a season of lines.
    let mut world_rng = Rng::new(42u32);
    let before = world_rng.clone().next();
    let mut p = Pen::on(4, Date::ymd(2026, 9, 1), Vec::new());
    for _ in 0..200 {
        p.part("talk.open.sour", &who, &f);
    }
    assert_eq!(world_rng.next(), before);
}

/// The proof the spec asks for (people topic 5): a season of one-on-ones
/// at full use, read part by part, with the repeats counted and timed.
/// `AFM_TALKS=path` writes the talks out to read.
#[test]
fn a_season_of_talks_counts_its_repeats() {
    let t0 = Instant::now();
    let mut r = Rng::new(2026u32);
    let levels = ["hs", "college", "nfl"];
    let traits = [
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
    ];
    let regions = ["texas", "south", "midwest", "northeast", "west"];
    let mut out = String::new();
    let mut forced = 0usize;
    let mut heard_again = 0usize;
    let mut parts = 0usize;
    let mut talks = 0usize;
    let mut thin: HashMap<String, usize> = HashMap::new();
    for (li, level) in levels.iter().enumerate() {
        // A roster: 85 high school, 105 college, 70 NFL players.
        let n = [85, 105, 70][li];
        let roster: Vec<Speaker> = (0..n)
            .map(|k| {
                let mut tags: Vec<&'static str> = vec![level];
                match *level {
                    "hs" => tags.push("young"),
                    "college" => {
                        if r.chance(0.85) {
                            tags.push("young")
                        }
                    }
                    _ => {
                        if r.chance(0.2) {
                            tags.push("young")
                        } else if r.chance(0.4) {
                            tags.push("vet")
                        }
                    }
                }
                for t in traits {
                    if r.chance(0.16) {
                        tags.push(t);
                    }
                }
                // Who talks back, as `Speaker::player` (a college player's
                // trust taken as low).
                let (ego, hot) = (tags.contains(&"ego"), tags.contains(&"hot"));
                if (*level == "nfl" && (ego || hot)) || (*level == "college" && ego && hot) {
                    tags.push("brash");
                }
                let region = regions[r.int(0, 4) as usize];
                tags.push(region);
                if region == "texas" {
                    tags.push("south");
                }
                player(1000 * li as u64 + k as u64, &tags)
            })
            .collect();
        let f = Facts::new()
            .with("coach", "Coach Reed")
            .with("you", "Reed")
            .with("team", "Iowa State")
            .with("friend", "Jalen")
            .with("pos", "WR")
            .with("subject", "his role")
            .with("worry", "his role");
        let mut pen = Pen::on(77 + li as u64, Date::ymd(2026, 7, 1), Vec::new());
        let mut heard: HashMap<(String, String), i32> = HashMap::new();
        let start = Date::ymd(2026, 7, 1).0;
        for day in 0..365 {
            let d = Date(start + day);
            pen.set_day(d);
            // 10 talk hours a week in season (Aug to Jan), 16 out of it,
            // all of them spent on one-on-ones: the most a coach can hear.
            let in_season = !(213..335).contains(&((day + 182) % 365));
            let per_week = if in_season { 10 } else { 16 };
            let today = per_week / 7 + usize::from(r.chance((per_week % 7) as f64 / 7.0));
            for _ in 0..today {
                talks += 1;
                // A few players get most of the talks.
                let who = if r.chance(0.6) { r.int(0, n / 5) } else { r.int(0, n - 1) } as usize;
                let who = &roster[who];
                let mood = ["sour", "good", "flat"][r.int(0, 2) as usize];
                let subject = ["role", "usage", "money", "behavior", "life", "team", "future"][r.int(0, 6) as usize];
                let mut keys = vec![format!("talk.open.{mood}"), format!("talk.open.{subject}")];
                let steps = r.int(1, 3);
                for _ in 0..steps {
                    let m =
                        ["ask", "straight", "praise", "challenge", "criticize", "promise", "warn", "hand", "listen"]
                            [r.int(0, 8) as usize];
                    let o = ["good", "bad", "flat"][r.int(0, 2) as usize];
                    if m == "ask" && r.chance(0.5) {
                        keys.push(if r.chance(0.55) {
                            format!("talk.reveal.{subject}")
                        } else {
                            "talk.deflect".into()
                        });
                    } else {
                        keys.push(format!("talk.reply.{m}.{o}"));
                        if r.chance(0.55) {
                            keys.push(format!("talk.close.{o}"));
                        }
                    }
                }
                let mut said = Vec::new();
                for k in &keys {
                    let before = pen.fresh(k, who, &f);
                    let Some(s) = pen.part(k, who, &f) else { continue };
                    parts += 1;
                    if before == 0 {
                        forced += 1;
                        *thin.entry(k.clone()).or_insert(0) += 1;
                    }
                    let e = heard.entry((k.clone(), s.clone())).or_insert(-REST_DAYS);
                    if day - *e < REST_DAYS {
                        heard_again += 1;
                    }
                    *e = day;
                    said.push(s);
                }
                if talks <= 300 {
                    out.push_str(&format!("{level} #{} {:?}: {}\n", who.key, who.tags, said.join(" / ")));
                }
            }
        }
    }
    let ms = t0.elapsed().as_millis();
    println!(
        "a season of one-on-ones at all three levels: {talks} talks, {parts} parts, {forced} said again inside the \
         season by the same man ({:.2}%), {heard_again} heard again from anyone inside the season ({:.1}%), {ms} ms",
        100.0 * forced as f64 / parts.max(1) as f64,
        100.0 * heard_again as f64 / parts.max(1) as f64
    );
    let mut thin: Vec<(String, usize)> = thin.into_iter().collect();
    thin.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    println!("parts that ran out of fresh versions for a man: {:?}", &thin[..thin.len().min(10)]);
    if let Ok(path) = std::env::var("AFM_TALKS") {
        std::fs::write(path, out).unwrap();
    }
    assert!(talks > 1500);
    // The rest holds for every man at full use.
    assert!(forced * 200 <= parts, "{forced} of {parts} parts came back to the same man inside a season");
}
