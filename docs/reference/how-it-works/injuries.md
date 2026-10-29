# Injuries: how it works today

Checked against main at 8e145de with update 3 lanes 3 and 4 on 2026-10-28.

Paths are relative to `crates/afm-engine/src/` unless they start with `crates/` or `app/`.

There are four injury sources, and they do not talk to each other much:

1. The match engine: hits on each snap, at every level (`game/sim.rs`, `injury_check`).
2. NFL practice: every club, every Tuesday from veterans' report to the Super Bowl (`world/pro/games.rs`, `practice_injuries`).
3. Your own college or high school team's practices (`training/seat.rs`, `practice_injuries`, `heat_week`, `report_day`).
4. The yearly roll at each checkpoint, for everyone: the injury that costs growth, games on the season row and sometimes burst for good (`development.rs`, `develop_season`).

## Quick answers

- Does an engine game injury carry past the game? Yes, at every level unless he played on. In the NFL (and the other pro leagues) it sets `out_days`, counted down day by day, with `out_weeks` the weeks still left (lane 4, B1: `world/pro/games.rs`, `record`, `record_preseason`; `world/pro/other.rs`; `ProCareer::hurt_for`). In college and high school (JV games aside) he is out ceil(weeks × (1 − 0.12 × medical) × 7) days, at least 1, from the day after the game; medical = (the program's medical rating − 10) / 10 in college (a top staff 12% shorter, a poor one up to 11% longer), 0 in high school. Your players get a `Hurt` row ("game") and a trainers' note; everyone else goes in `training.others_hurt`, which `hurt_out` reads, so AI teams sit them too (`training/seat.rs`, `game_injuries`; `world/college.rs`, `play_day`; `world/hs_year.rs`).
- Then where do college and high school "games missed" come from? The yearly roll: minor 0-2 games, moderate 2-6, major half the season or more, written on the season row (`development.rs`, `develop_season`, `games_missed`). It does not take him out of engine games.
- Does toughness make injuries rarer? No. Toughness only decides whether he plays on hurt (`game/sim.rs`, `injury_check`, `plays_on`).
- Does fatigue raise injury odds? Yes: the chance × (1 + 0.8 × tired), tired 0-1 from his energy (`game/sim.rs`, `injury_check`). Fatigue also scales ratings (`game/sim.rs`, `Agent::new`).
- Does body weight matter? Yes, through the hit's force: half the sum of each man's mass times speed (`game/sim.rs`, `hit_effects`). Players of 260 lb or more (any position) also risk heat illness more in your high school practices, ×1.8 (`training/seat.rs`, `heat_week`).
- Does position matter? In the yearly roll: QB 0.7, RB 1.4, WR 1.0, TE 1.1, OT 0.9, IOL 0.9, EDGE 1.0, DT 1.0, LB 1.1, CB 1.0, S 1.0, K/P/LS 0.25 (`positions.rs`, `PositionProfile.injury_risk`). In the engine only through how often and how hard he is hit. In practice specialists are 0.25 (`world/pro/games.rs`, `practice_injuries`; `training/seat.rs`, `practice_injuries`).
- Does turf matter? ×1.12 on artificial turf, wherever the game is played (`game/sim.rs`, `injury_check`). NFL fields follow each club's real turf seasons, a future new stadium turf if domed, neutral sites turf if closed (`world/finance/data.rs`, `TURF`; `world/finance/nfl.rs`, `turf`; `world/pro/games.rs`). College and high school fields are drawn per program or school and year from real shares that rise over time (FBS, smaller colleges, high schools), richer programs more often (`world/season.rs`, `field_turf`; `world/college.rs`, `conditions`; `world/hs_year.rs`, `hs_turf`). Rain and cold don't touch injury odds (they raise fumbles) (`game/sim.rs`, `hit_effects`).
- What is injury proneness? A hidden 1-20 trait, normal(10, 3.5) at creation; +1 after each major injury, capped 20 (`player.rs`, `trait_1_20`, `generate_prospect`; `development.rs`, `develop_season`).
- Does the medical staff shorten injuries? NFL regular-season and playoff game injuries and practice injuries: up to 12% shorter (`world/pro/games.rs`, `record`, `practice_injuries`); preseason game injuries don't read it (`record_preseason`). The yearly roll: up to 15% fewer games for minor and moderate ones (`development.rs`, `develop_season`, `ctx.medical`). Your college or high school practice injuries: not read (`training/seat.rs`, `practice_injuries`, `med = 1.0`).
- Is there re-injury? Only for a player you rush back through a second-opinion scene: 8% a week for eight weeks, 2-6 weeks out (`world/folk/life.rs`, `opinion_answer`, `rushed_week`). Nothing else raises odds after an injury except +1 proneness after a major one.
- Can an injury end a career? Not directly. A major injury adds 1.5 wear, which lowers a pro's usual retirement age by 0.3 a point of wear above 6 (`development.rs`, `develop_season`; `world/pro/mod.rs`, end-of-season retirement). The class sim's own retirement (`career.rs`, `retires`) adds 12% after a major injury at 27 or older, but the world does not use it.
- Do AI draft boards read injuries? No reader found. The medical read is your staff's file only (`world/scouting/file.rs`, `medical_read`; shown by `crates/afm-game/src/lane_scouting.rs`).

## In games (the match engine)

- Where checked: every contact in `hit_effects` (the ball carrier and the tackler), and sacks (`game/sim.rs`, `hit_effects`; `game/scrimmage.rs`, the sack, share 0.8, or 1.3 when the quarterback didn't see him). Kick returns run through the same sim (`game/kicking.rs`).
- Force: (carrier mass × speed + tackler mass × speed) × 0.5 × (1 + 0.2 × share × his big-hit tendency lean) (`game/sim.rs`, `hit_effects`).
- Share: the carrier 1 + 0.4 × his contact tendency lean (min 0.2); the tackler 0.7 (`hit_effects`).
- Chance: `injury_scale × 0.022 × share × (force / 1700)^1.6 × prone × turf`, prone = 0.55 + proneness / 18 (1 → 0.61, 10 → 1.11, 20 → 1.66), turf 1.12 or 1.0, `injury_scale` 1.0 (`game/sim.rs`, `injury_check`; `game/flow.rs`).
- Type mix (weights): bruise 22, ankle sprain 13, high ankle 5, knee sprain 10, ACL 2.2, shoulder 9, concussion 8, ribs 5, hand 6, fracture 3, back 3, hamstring 6 (`injury_check`).
- Length: `injury_weeks` gives lo + (hi − lo) × u² (most are short) × (1.3 − 0.03 × recovery) (recovery 10 → ×1.0, 20 → ×0.7, 1 → ×1.27) (`game/sim.rs`, `injury_weeks`, `injury_check`).
- Ranges in weeks: bruise 0-1, ankle 0-3, high ankle 2-7, hamstring 1-5, knee 1-6, ACL 36-56, shoulder 0-8, concussion 0.5-3, ribs 0-4, hand 0-5, fracture 4-12, back 0-4 (`injury_weeks`).
- Playing on: never after a concussion, ACL or fracture; otherwise chance clamp(0.15 + 0.035 × toughness − 0.12 × weeks, 0, 0.95), toughness as a 1-20 read against the level (`injury_check`; `game/roster.rs`, `trait20`).
- Concussion protocol: caught 92% in the NFL, 80% in college, 60% in high school; one not caught plays on (`game/flow.rs`, `apply_injuries`).
- Playing hurt: the ratings his injury limits drop by its share each snap he plays, scrimmage and kicks alike (a hamstring speed and burst, a hand catching) (`game/sim.rs`, `hampered`, `InjuryKind::hampers`; `game/scrimmage.rs`, `run_snap`; `game/kicking.rs`). The rest are untouched.
- Out: removed for the rest of the game (`game/flow.rs`, `apply_injuries`, `out`).
- Practice snaps on the engine clear the engine's injuries every snap (`game/flow.rs`, `practice_snaps`).

## After an NFL game

- Each injury not played through: days = ceil(weeks × (1 − 0.12 × medical z) × 7), out that many days from the day after the game (lane 4, B1: `world/pro/games.rs`, `record`). The injury goes to the team he played for (`res.injuries` holds the team; it was read by snap side, so a kick or turnover snap's injuries landed on the wrong team, oddity 28). Preseason games the same without the medical staff. Practice injuries: ceil(weeks × 7) days.
- Injured reserve when weeks ≥ the minimum: 6 before 2020, 4 from 2020 (`world/pro/games.rs`, `ir_minimum`).
- Preseason games: 6 weeks or more puts a vested veteran (4+ accrued seasons) on reserve now; anyone else waits for the cutdown (`record_preseason`). At the cutdown a kept player still out 6+ weeks goes to reserve (`world/pro/mod.rs`, `cutdown`); one cut while hurt gets an injury settlement, his base pay for the weeks out, as dead money (`world/pro/squad.rs`, `settle_injury`). A camp injury of 6+ weeks costs a borderline player 2 points in the cut ranking (`world/pro/mod.rs`, `cut_scores`).
- Players with `out_weeks > 0` don't dress, nor do your own club's players on the non-football injury list from report day (`world/pro/games.rs`, `actives`, `preseason_side`; `training/seat.rs`, `SeatTraining::hurt_out`).

## NFL practice injuries

- Every Tuesday from veterans' report to the Super Bowl, every healthy roster or practice-squad player (`world/pro/mod.rs`, the daily step; `world/pro/games.rs`, `practice_injuries`).
- Weekly chance: base × prone × age × specialist × plan × load. Base 0.0155 before the cutdown, 0.009 after. Prone = 0.55 + proneness / 18. Age = 1 + 0.03 × max(age − 26, 0). Specialist 0.25. Plan = the club's week against an average staff's: clamp(sqrt((full-contact minutes + 10) / (neutral + 10)) × ((field minutes + 30) / (neutral + 30))^0.3, 0.5, 2.0) (`training/week.rs`, `team_week`, `risk`; stored in `training.club_risk`). Your own club's plan is the same read of your own week × (0.77 + 0.23 × limber), limber = clamp(1 − 0.1 × (mobility minutes − neutral) / (neutral + 15), 0.9, 1.1) (`training/seat.rs`, `week`, `my_risk`, `limber`; `training/week.rs`, `monday`). Load = 1 + 0.8 × (spike − 1.3) above a 1.3 spike (`training/mod.rs`, `Train::spike`).
- Mix: hamstring 22, ankle 15, knee 14, bruise 10, shoulder 7, back 6, hand 6, high ankle 4, concussion 4, ACL 2.5, ribs 2, fracture 2. Length: `injury_weeks` × (1.3 − 0.03 × recovery) × (1 − 0.12 × medical z), ceil; 0 weeks is no injury.
- Reserve: in camp 6+ weeks and vested; in season past `ir_minimum` and not on the practice squad.

## Your college or high school team's practices

- Run weekly on staff weeks, from the planned week (`training/seat.rs`, `week`, `practice_injuries`). Every player not already hurt and not absent from a voluntary week rolls; heat illness too (`practice_injuries`, `heat_week`, `absent_ids`). AI college and high school teams roll the same chance in the weekly world pass, from their own week's block, practices, contact and mobility work; the injury goes in `training.others_hurt` and he sits the games until back (`training/week.rs`, `monday`; `practice_kind`).
- Rate per practice: camp 0.0037, spring 0.0023, season and postseason 0.00115, other 0.0008; high school × 0.4 (`training/seat.rs`, `practice_rate`).
- Chance over the week: 1 − (1 − rate × contact × prone × exposure × sqrt(soft))^practices (`injury_chance`), then × (0.77 + 0.23 × limber) for the week's mobility work (`practice_injuries`, `limber`). Contact = ((full-contact minutes + 10) / (neutral + 10))^0.7. Prone = 0.42 + proneness / 18. Exposure 0.25 for specialists. Soft = 1 + 1.5 × (spike − 1.3) above 1.3 (`soft_tissue`).
- Mix as the NFL practice mix, with hamstring weight × soft × limber and concussion and shoulder × contact (`practice_injuries`).
- Length: days = round(`injury_weeks` × (1.3 − 0.03 × recovery) × 7), at least 1, starting on a random weekday of the week. Kept in `training.hurt` (`Hurt`), with a trainers' note.
- While hurt: his week keeps every off-field period except mat drills and testing (meetings, installs, film, the weight room, mobility, the rest of the classroom), plus 45 minutes of recovery on each day with any (`training/seat.rs`, `own_week`).
- He sits out every game until healed, at every level: `prepare` drops him and the game builders leave him out (`training/seat.rs`, `prepare`, `SeatTraining::hurt_out`; `world/college.rs`, `play_day`, `seat_setup`; `world/hs_year.rs`, `can_play`; `world/pro/games.rs`, `actives`, `preseason_side`).
- His status line shows the injury and the days or weeks left; an entry with no injury kind (a failed conditioning test, heat illness) shows as "Non-football injury list" (`crates/afm-game/src/views.rs`, `status_text`).
- His prep that week: limited (0.55 of the field work, all the classroom) when he is back within 3 days of Monday, else out (no field work, three quarters of the classroom). Out reads Unprepared or Underprepared for most players, and two poor weeks in a row start rust (`training/seat.rs`, `prep_detail`; `training/prep.rs`, `Status::there`; see [training.md](training.md)).
- Heat illness (your high school without a heat rule): see [training.md](training.md). Out 3 days, `kind: None` (`training/seat.rs`, `heat_week`).
- Failed conditioning test on report day: 3-10 days on the non-football injury list, `kind: None` (`training/seat.rs`, `report_day`; `training/offseason.rs`, `fail_chance`).
- An NFL seat team's practice injuries come from the league-wide NFL practice pass, not this one (`training/seat.rs`, `week`, `st.tier != Tier::Pro`).

## The yearly roll (every player, every checkpoint)

- Chance: level rate × position risk × (0.55 + 0.045 × proneness) × role (`development.rs`, `develop_season`). Level rate: high school 0.13, small college 0.16, FCS 0.17, FBS 0.19, NFL 0.24 (`level_injury_rate`). Role: 1.15 starter, 0.9 otherwise, 0.5 for a year without games or in trouble.
- Severity: 62% minor, 27% moderate, 11% major (`develop_season`).
- Games missed (`season_games`: HS 10, small college and FCS 11, FBS 12, NFL 17): minor 0-2, moderate 2-6, major from half to all. Minor and moderate × (1.3 − 0.03 × recovery) × (1 − 0.15 × medical), rounded. A year with no games records 0.
- Cost to the year: minor growth × 0.95 and snaps factor 0.9; moderate × 0.85 and 0.65; major × 0.5 and 0.2 (`develop_season`, `growth_multiplier`, `snaps_factor`). With weekly training, the growth multiplier is stored in the year's form and lands through the weeks (`training/mod.rs`, `Train::open`).
- Major injury, on the injury-sensitive attributes (speed, acceleration, agility, balance, flexibility, jumping; not strength or stamina) (`attributes.rs`, `injury_sensitive`):
  - 55% heal fully: −1.5 to −5 now, back next year.
  - 30%: the same, plus −1 to −3 on the ceilings for good.
  - 15% lose their burst (`LostBurst`): −3 to −7 now and on the ceilings, nothing back.
  - Every loss × the age hit: 1.0 under 24, 1.2 at 24-27, 1.6 at 28+ (`injury_age_hit`).
  - Proneness +1 (max 20), wear +1.5.
- The year back: a healed major injury returns min(room, 1.5-5 × age hit) on each sensitive attribute; the ceilings above them were kept from settling while he healed (`develop_season`, the year-back step and the tidy-up `rehab` rule).
- Recovery trait: normal(10, 3.5) from his id, −0.1 to −0.4 a year from 30 (`player.rs`, `GameTraits::for_id`, `age_one_year`).
- Wear: only major injuries add it in the world (no other `wear +=` outside `career.rs`). Wear above 3 speeds physical decline (`development.rs`, ageing, `wear_mult`) and above 6 lowers a pro's retirement age.

## Recovery and lists (NFL)

- `out_days` drops one every day, all year, after the day's games, so the day plays the game its morning showed (lane 4, B1: `world/pro/mod.rs`, `heal_day`, last in `nfl_year::day`), and `out_weeks` is the weeks still left, rounded up. The list of hurt pros is kept for the day and rebuilt from everyone each Tuesday and after a load; an older save's `out_weeks` turn into days the first morning. `heal_week` now counts only reserve and practice squad weeks.
- Back from reserve the night he heals, in season, if the club has returns left: none before 2012, 1 in 2012-16, 2 in 2017-19, unlimited 2020-21, 8 from 2022; else he sits out the season there (`world/pro/games.rs`, `ir_returns`; `world/pro/mod.rs`, `heal_day`). A full roster cuts its lowest-graded roster player to make room (health not checked).
- At the new league year a healed reserve player rejoins the roster; one still hurt moves to the reserve/PUP list and comes back the night he heals without using a return (`world/pro/mod.rs`, the league-year step, `pup`; `heal_day`).
- A club over the roster limit with too few healthy players to dress moves its longest injury to reserve (`world/pro/mod.rs`, `fill_roster`).
- Pros out 2+ weeks (other than yours, who train on your hurt week) train at a rehab ratio (strength 0.6, speed 0.4, conditioning 0.4, technique 0.1, contact 0, kicking 0.2, mental 0.5) at 70% attendance, load 150 units, and read as out for prep. An injury heals on Tuesday, after Monday's pass, so a pro with one week left practises and is read as full (`training/week.rs`, `monday`; `training/prep.rs`, `out_this_week`).
- Second opinions (your team): a player out 4+ weeks, or one with a major injury near fall camp, may ask; supporting it moves the estimate −1, 0 or +2 weeks; asking the trainer to rush him cuts it to three quarters unless the trainer's judgement is 14 or more (`world/folk/life.rs`, `opinions_week`, `opinion_answer`).

## How injuries carry over

- High school to college: through the season rows and the hidden traits (proneness, wear, ceilings lost). In the world, recruiting doesn't read a senior-year injury; only the class sim does (`career.rs`, `simulate_cohort`, the senior-film note).
- College eligibility: a season where a major injury hit or half the season's games were missed is "hurt most of the season" (`world/eligibility.rs`, `hurt_most_of_season`). With 3 games or 30% of the team's games played at most, it is a medical hardship and doesn't count; a hurt redshirt year counts toward the two lost seasons that earn a sixth year. From 2026-27 Division I counts every year (no hardships) (`world/college.rs`, `settle_eligibility`; `world/eligibility.rs`, `hardship_games`).
- College to the NFL: the medical read on your staff's file lists each past moderate or major injury with games missed and a lost burst, a count of minor knocks, and doctors' notes when examined (combine, a visit or a local day). Concern = 0.45 × (proneness − 10) / 5 + 0.8 × majors + 0.25 × moderates − 0.2 × longevity / 2 + 0.3 × wear, plus noise of (0.35 examined, 0.8 not) × the staff's sharpness. Grades: cleared, minor concern (0.6+), major concern (1.4+), failed (2.4+) (`world/scouting/file.rs`, `medical_read`, `medical_words`).
- A projected first-rounder may skip the all-star week to avoid injury risk (`world/recruiting.rs`, `all_star_invites`).

## Where the knobs are

- `game/sim.rs`: `injury_check` (0.022, force exponent 1.6, prone, turf 1.12, the mix, the play-on formula), `injury_weeks` (ranges), `InjuryKind`.
- `game/flow.rs`: `apply_injuries` (concussion catch rates), hurt penalty −3, `injury_scale` 1.0.
- `world/pro/games.rs`: `practice_injuries` (0.0155, 0.009, age, load), `ir_minimum`, `ir_returns`, medical 12%.
- `training/seat.rs`: `practice_rate`, `soft_tissue`, `injury_chance`, the practice mix, `heat_week`, `limber` (0.9-1.1, weight 0.23).
- `development.rs`: `level_injury_rate`, severity split, games missed, growth and snaps costs, the major injury outcomes, `injury_age_hit`.
- `positions.rs`: `injury_risk` per position. `attributes.rs`: `injury_sensitive`.
- `world/folk/life.rs`: second opinions and the 8% re-injury.
- `world/scouting/file.rs`: `medical_read` weights and grade lines.

## Related pages

- [training.md](training.md): load, the practice week, how the hurt train and their prep.
- [development.md](development.md): growth, ceilings, wear and ageing.
- [match-engine.md](match-engine.md): hits, force and contact.
- [nfl.md](nfl.md): rosters, reserve, cutdowns and settlements.
- [scouting.md](scouting.md): the medical read in context.
- [people.md](people.md): second opinions and trust.
