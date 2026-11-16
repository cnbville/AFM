# Ground: how it works today

Checked against main at e677a8d (lane G8 branch) on 2026-11-16.

Ground is the offseason game (`docs/design/ground.md`). It is built in lanes, one section of this page each. Paths are relative to `crates/afm-engine/src/` unless they start with `crates/` or `app/`.

## Quick answers

- Can a recruit have a famous father? Yes, from lane G8: a son or nephew of a world player with a Pro Bowl or more, or a Heisman, carries his name (`world/following.rs`, `names`, `man_fame`).
- How often? Per famous man, 45% have one son who plays and 13% two, each 14 when the man is 36 to 50; 30% have a nephew, 14 when the man is 30 to 55 (`kin`).
- Does it change how many boys have an NFL father? No. A son is one of the class's boys whose father already played in the NFL (1 in 200); now the father has a name (`names`; `world/background.rs`, `generate_family`).
- How much does the name add? To his fame, 0.35 × the man's fame for a son, 0.18 × for a nephew, in high school and college, half of it as a pro (`HYPE_SON`, `HYPE_NEPHEW`; `world/fame.rs`, `fame`).
- When is a commitment big news? When his following is one of his class's 10 biggest (and at least 10,000), huge for the class's biggest; otherwise his stars decide as before (`commit_news`; `world/recruiting.rs`, `commit_now`).
- What does the hype kid's commitment do for the program? Lifts its brand exposure grade for a year, by 3 × his share of the class's biggest following, all lifts together capped at 3 points (`committed`, `day`, `LIFT_TOP`).
- Does it stay if he flips or decommits? No. The lift is his while he is committed to, signed with or playing for that program; a flip moves it (`holds`, `committed_in`).
- Does a recruit's following change his own pick? Yes: brand exposure counts 1 + his share of the class's biggest following in the prestige part of a school's appeal, so up to twice (`world/recruiting.rs`, `stage_weight`, `utility_fit`, `six_parts`).
- Can a college player have a social media storm? Yes, a college player or recruit with 500,000 followers, the same weekly chance as a pro (`world/storylines/detect/weekly.rs`, `chances`; `STORM_FOLLOWERS`).

## A following and famous names (lane G8: GR42-GR46)

**Famous men.** A world player is famous from his career and books: a Hall of Famer 1.0; an MVP, Offensive or Defensive Player of the Year, Super Bowl MVP or three first-team All-Pros 0.85; a first-team All-Pro, four Pro Bowls or a Heisman 0.65; a Pro Bowl 0.4; anyone else 0 (`world/following.rs`, `man_fame`). It reads his alumni record's pro career when he has retired, his live career when he still plays (`World::alumni`, `Alumnus::pro`), the Hall (`nfl_book.hall`) and the Heisman (`stats.honors`). Only men in the world's books count: the recorded years from 1996 on in a 2026 world, so the first famous sons arrive around 2012.

**His kin.** Keyed on the man (his player id): 45% one son who plays, 13% two, 42% none; each son 14 when the man is 36 to 50 (even odds each year). A nephew for 30%, 14 when the man is 30 to 55 (`kin`). All proposed, not confirmed.

**The class day.** After the new class is in (`World::add_class`), each famous man's kin due this year joins it (`names`). A son is a boy of the new class whose father played in the NFL (`Bloodline::NflPlayer`), of the man's ethnicity, in the man's home state if one fits there, any state otherwise; a nephew a boy whose father didn't play (`Bloodline::None`). A boy already your former player's son is skipped, and your guys' sons skip a famous name (`world/legacy/guys.rs`, `sons`). The pick is keyed, so a seed always makes the same family. A son takes the man's surname; about one in eight is named for him ("Darnell Hayes Jr."). A nephew takes the surname two times in three (a brother's son), keeps his own otherwise (a sister's). Stored in `World::following.names` (`Name`: the boy, the man, son or nephew, the man in words, his fame, the hype).

**What the name does.**
- Fame: the hype adds to his fame like an honor, 0.35 × the man's fame for a son, 0.18 × for a nephew, in high school and college; half of it as a pro (`world/fame.rs`, `fame`). A Hall of Famer's son adds 0.35, about what a fifth star adds in high school (0.55 for 5 stars against 0.32 for 4). Followers, NIL brand deals, NIL Go's range and a storyline's weight all read fame, so all of them follow.
- Staffs notice him: his recruiting visibility takes 0.15 × the man's fame more (half for a nephew), beside the NFL bloodline's 0.15 (`world/recruiting.rs`, `recruit_visibility`).
- His file: "His father is Darnell Hayes, a four-time Pro Bowl receiver." leads the family notes, in place of "His father played in the NFL." (`world/scouting/lines.rs`, `family_notes`). The news says "son of four-time Pro Bowl receiver Darnell Hayes" (`news_name`, `Name::tag`).
- What it does not do: his ratings, ceilings and stars are the boy's own (a son keeps the NFL bloodline's small edge he already had).

**The class's biggest followings.** At February's book, after followers move, each high school class's biggest and tenth-biggest following is kept (`World::following.classes`, `class_tops`), and every high schooler's spotlight is set: his following ÷ his class's biggest, 0 to 1 (`Person::spotlight`; 0 out of high school). A world file from before this lane fills them on load (`after_load`).

**News (GR43).** A commitment, a flip and a signing-day flip post their news when he has 4 or 5 stars, as before, or a following in his class's 10 biggest (and at least 10,000, `NEWS_FLOOR`). Loudness is the louder of his stars' (5 stars medium, else small; a signing-day flip medium) and his following's: big for the 10 biggest, huge for the biggest. The news then adds "He has 412,000 followers." (`commit_news`, `followers_words`; `world/recruiting.rs`, `commit_now`, `run_signing_stage`). Big news gets its own screen and stops Continue; huge is the full show (`calendar/events.rs`, `Loudness`).

**Brand exposure (GR44).** A commitment (or a signature without one) lifts the program's brand exposure grade by 3 × his spotlight, kept when it is at least 0.05 (`committed`, `LIFT_FLOOR`). Each morning (5 AM, after the class day) a lift ends after 365 days or once he no longer holds it (committed to, signed with, or in college at the program), and each program's grade carries the sum of its lifts, at most 3 points and never past 99 (`day`, `set_lift`, `LIFT_TOP`, `LIFT_DAYS`; `Program::brand_lift` is what is in the grade now). At the year's grades the lift comes off first, so the grade moves from its own read, and goes back on after (`world/staff.rs`; `strip`). The cap on the sum is proposed, not confirmed.

**His pick (GR45).** In a school's appeal, the prestige part is (title chances + stadium + tradition + w × brand exposure) / 4 with w = 1 + his spotlight: 1 for a boy nobody follows, 2 for the class's biggest following (`world/recruiting.rs`, `stage_weight`, `utility_fit`, `six_parts`). A transfer's spotlight is 0, so the portal weighs it once, as before.

**Storms (GR46).** Each week a college player, high schooler or prep player with 500,000 followers or more rolls the pro's storm: 0.15% × (1 + his temper's z above 0), about 1 in 670 a week (`world/storylines/detect/weekly.rs`, `chances`; `STORM_FOLLOWERS`). The story ("A post that blew up") is open at every level. High school followers top out at 400,000 × reach, so a recruit reaches it only in a big-reach year or as a famous name with Aura.

## Where the knobs are

- `world/following.rs`: `HYPE_SON` 0.35, `HYPE_NEPHEW` 0.18, `FAMOUS` 0.4, the tiers in `man_fame`, the odds in `kin`, the Jr. 12% and the nephew's name 65% in `names`, `LIFT_TOP` 3, `LIFT_DAYS` 365, `LIFT_FLOOR` 0.05, `BIG_NEWS` 10, `NEWS_FLOOR` 10,000, `STORM_FOLLOWERS` 500,000.
- `world/recruiting.rs`: `stage_weight`; the visibility's 0.15 in `recruit_visibility`.

## Related pages

- [people.md](people.md): fame, Aura, followers and brand money.
- [recruiting.md](recruiting.md): what a recruit weighs, the program grades.
