# Staff urgency: when to move on from a player

Designed 2026-11-21 with Cain (thread "Give staff urgency to cut underperformers"). Built 2026-11-21/22 in PR #611 (thread "Build staff urgency to move on"): `world/folk/roster_call.rs` and `world/growth.rs`; how it works in `docs/reference/how-it-works/people.md`, "Staff urgency: the roster calls", and `recruiting.md`, "Told he isn't in the plans". The world measure is at the end.

## Cain's words

> "create an urgency update, right now coordinator and position coordinators are constantly afraid to cut or be ruthless to players that are underperforming. Or RS Junior players way below FBS level. Also they have no sense of time, they're asking me to spend a winter on a sub-par player instead of cutting them and bringing in a portal player. Their evaluation is good too and it's been a consistent thing over a number of saves. It's a nuanced, dynamic fine line"

> "give them a sense of time too, they understand that on 31.12.2025, there is no time to wait for winterworkouts when portal starts on 2 january"

His answers to the three questions (2026-11-21): "1. Yeah they should give their input whenever it makes sense, like the first meetings, before portals etc / 2. yeah make sure it happens relative to realism / 3. sure".

## The cause

The reads are right; the advice built on them is wrong. The only keep-or-cut advice in the game is a coach's answer when you ask for his read (`world/folk/persona/evaluate.rs`, `read`, the `Lean`). Its rule was Claude's, never confirmed, and it has three faults:

1. It ranks the man only inside your own room. A starter is always "keep him", so on a weak room a RS junior far below FBS level is "one of our starters".
2. A man behind the two-deep is kept to develop when his ceiling as a pro reaches the last two-deep man's read today. The pro ceiling is the most he could ever be, so nearly every backup clears it: "keep him and develop him".
3. It reads no date, no eligibility left, no level marks, no portal, no roster limit and nothing about the coach.

The pieces it needed already exist: the level's backup, starter and elite marks (`world/norms.rs`, `Marks`), each man's read carried forward by how players his age grow in this world (the Needs map, `afm-game` `lane_squadboards/needs.rs`, `growth_table`, `carry`), the portal windows by era (`world/clock.rs`, `portal_windows`) and the roster limits (`world/ncaa.rs`). AI staffs never cut at all (`docs/reference/how-it-works/recruiting.md`, "Cuts"), so the same fault sits on the AI side.

## 1. His read of the man (what he sees)

Every item here is the staff's read (`readers`, the position coach's own looks when the man is in his room), never true ratings or potential.

- **Against the level.** He places the man against the level's marks at his spot (backup, starter, elite, for your level's pool), not only against your room. "Third of our five" stays as a fact; the judgment is against the level.
- **Where he'll be when he's out of time.** The man's read carried forward season by season to his last season of eligibility (college) or the end of his contract (NFL), by the world's growth at his age and tier, the same carry the Needs map uses. The middle of the growth and a good case (the table's upper band) both, so a late bloomer gets his chance. The pro ceiling no longer decides anything here.
- **The clock.** The date and the next decision point at his level: the portal window that opens next, when it closes and what comes after it. From the 2025 season one window, 2 January to 16 January: on 31 December 2025, waiting for winter workouts means the next chance to replace him is next January. In 2022-2024 a winter and a spring window; before 2018 the release after the season. NFL: free agency, the draft and the cutdown to 53. High school: none (see 6).
- **The market at his spot.** What the portal usually offers at that position at the level he'd need: how many men at or above the level's starter mark entered in the world's past windows, and how many your program landed against its targets (your appeal and money as the past windows showed them). Before the window opens, the players expected in (December's flight risks across the world, the on-hold entrants) count as the staff hears of them.
- **What he costs to keep.** His place under the roster limit (105 under the House limit, 85 scholarships before), his NIL or revenue share money, his snaps in winter workouts and spring ball that a newcomer would take, and what moving on costs: the room sees it (trust, the bonds and favoritism of `coach-bonds.md`), news when he's a 4-star or a starter, and a cut player's grudge.
- **The man himself.** Work ethic, coachability, leadership and his place in the room (a captain, a culture man), health (a man coming back from an injury is read on it), and a position move that could save him.

## 2. The bar he holds the man to (who he is)

The read in 1 is the same for every coach. His makeup and situation move only the bar.

- **Ruthless:** temper, ambition, a hot seat (`claims::program::hot_seat`), a contender's goals from his boss, a strong roster that can afford to lose a man.
- **Patient:** loyalty, a strong teacher (his teaching rating for the group), his own bond with the man (closeness, `folk/bonds.rs`), the man he recruited himself, a rebuilding program, a thin room where the portal rarely gives better.
- **Sizes (Cain "sure", 2026-11-21; proposed, not confirmed, then checked on the small world):** the average coach draws the line at the level's starter mark for a starter's job (the backup mark for a depth job). The most loyal coach keeps a man whose last-season projection is up to 4 points under it. The most ruthless moves on from a man projected up to 2 points over it when the portal clearly has better. Everyone else falls between by his makeup and situation.
- **He can be wrong, and says why.** A coach who keeps his own recruit against the numbers says the real reason ("I recruited him, I believe in him"); the read beside it stays honest.

## 3. What he tells you to do

One of three moves, each with his reason in plain words, no numbers on screen:

- **Move on now.** He won't reach the bar in the time he has, and the window gives a better chance. "He won't be a Power 4 starter before he's out of years. The window opens on the 2nd and we usually find two guards better than him there. Move on now."
- **Bring a man in over him.** He won't reach the bar for the job, but he's worth his place as depth (cheap, a culture man, a special teams player, room under the limit). "He won't start for us, but he holds the room together. Bring a man in over him and keep him."
- **Keep and develop.** He reaches the bar in time, or the market at his spot is too thin to do better. "He's a year away, and the portal rarely has better tackles than him. Keep him."

The answer to "should we cut him?" in a talk uses the same read and the same words (`evaluate::words`).

## 4. When the staff speaks up (Cain: "whenever it makes sense")

- **When you ask** (a talk's `Request::Evaluate`), any day.
- **The first meetings** of a new job: the state of the team (`Kind::StaffReport`, `world/folk/takeover.rs`) carries each coordinator's and position coach's list.
- **Before every window:** a roster meeting in the days before the portal opens, in the exit meeting week (college: after the last game, before 2 January from the 2025 season; before the spring window too in 2022-2024). One list, each coach's men marked with his move and reason, not a pile of messages. It sits with the exit meetings (`talks.rs`, `exit_meetings`), so a man marked "move on" can be told in his exit meeting.
- **Before the other real decision points:** the roster limit before fall camp under the House limit; at the NFL before free agency, before the draft and before the cutdown; at high school before the season and after it.
- A coach who changes his mind (a man who grows fast in spring, a transfer who didn't come) says so at the next of these, not every day.

## 5. AI staffs on the same rules (Cain: "make sure it happens relative to realism")

- AI head coaches take their own staff's moves on the same reads and bars, and act: a man marked "move on" is told before the window and enters it (a cut where real staffs cut; most are told they're not in the plans and enter themselves). "Bring a man in over him" puts his spot on the program's portal board.
- The world's portal total stays at its real size. These men are part of the real flow (many real entrants are encouraged out), not added on top: the world already takes 22-29 a program a year under the House limit against about 12 expected in December (`recruiting.md`), so the build measures the total and who leaves, by role and level, before and after.
- AI programs over the 105 limit trim to it before fall camp, the weakest projections first, by the same bars.
- NFL front offices already cut, waive and release on their own desk (`world/pro/`); they read the coaches' moves as input there. High school AI coaches move men between positions and levels (varsity, JV) the same way.

## 6. Each seat

- **College:** all of the above.
- **NFL:** the time is the contract and the cap, not eligibility: the cutdown to 53, the practice squad, waivers, dead money, the June 1 release, a vested veteran's guaranteed pay. The market is free agency and the waiver wire. The coaches advise; the GM decides where he decides today.
- **High school:** no portal and nobody is run off for a better player. The moves are playing time, moving him to another position or to JV, and telling a senior where he can play next. The clock is the season and his graduation.

## 7. The build (one lane)

- Engine: the read and the bar in one place shared by the talk, the meetings and the AI (`persona/evaluate.rs` grows into it, or a new `world/folk/roster_call.rs`); the growth carry moves to the engine so the Needs map and the advice read the same numbers.
- The meetings: the state of the team's list, the pre-window roster meeting, the other levels' points.
- The AI side and its measure on a small world (stand-in games are enough for the portal flow): portal entrants a program, by role and level, before and after, against real; rosters against the limit before fall camp.
- `docs/reference/how-it-works/` pages updated (live-talks "His read", recruiting "Cuts", people), `GAPS.md` cleared of what this builds.

## Proposed, not confirmed

- The sizes in 2 (4 points under for the most loyal, 2 over for the most ruthless, the average at the mark).
- The good-case band used for the late bloomer (the growth table's upper band).
- Which makeup traits weigh how much toward ruthless or patient.
- The market as a staff aims at a spot (the program's landing rate on an average spot's men, capped at all of them), half a man a window for thin and one for clear.
- The AI tells at most a quarter of a program's expected entrants (`TOLD_SHARE`), the men furthest under their bar first, and 85% of told men enter (`TOLD_GO`).
- The AI read's error (2 points for an average evaluator) and the divisions that act (FBS, FCS, Division II).

## World measure (2026-11-22)

`urgency_check` on one small world (scale 0.25, 6 years of history, stand-in games, seed 311), built twice on GitHub (`.github/workflows/urgency-check.yml`): `AFM_URGENCY=off` (no staff ever moves on from anyone) and on. `AFM_URGENCY_TRACE=1` prints every winter window's AI calls by reason and the market's spread. Four runs; the first three each found a fault, fixed in the same PR.

1. **Run 1.** The total held, but December calls counted the season just played as still to come (his seasons settle on January 2), so every December staff saw a year more than the man had. Fixed: `years_left` counts the season in play (`count_pending`), as the portal's own check does. This is Cain's own case: on December 31 a redshirt junior's season is already his.
2. **Run 2** (with the trace). "The portal rarely has better" decided about 45% of FBS calls (4,000 to 6,000 a year), the coaches' old fear in a new form: the market spread a program's landings evenly over every spot, a median of 0.1 to 0.24 men a spot. Fixed: a staff aims its landings at the spot it needs.
3. **Run 3.** The thin market was gone (FBS median 1.7 to 3.8 men a spot; under 5% of calls thin). But the room under the limit counted only the returners and the signed class. In December the seniors' places read as open, so almost nobody was moved on, and programs cut 550 men at camp instead. Fixed: the room is the limit less the larger of the roster now and the returners with their class.
4. **Run 4, the result.**

FBS entrants a program by cycle (the season just played; 2019 is a part cycle):

| Cycle | Off | On | Told (FBS) | Told (FCS) | Told (D2) |
|---|---|---|---|---|---|
| 2019 | 6.1 | 7.7 | 0.8 | 0.8 | 0.8 |
| 2020 | 16.6 | 18.2 | 1.6 | 1.9 | 1.2 |
| 2021 | 19.1 | 19.8 | 2.8 | 2.4 | 2.5 |
| 2022 | 18.3 | 20.2 | 1.9 | 1.7 | 1.8 |
| 2023 | 18.8 | 19.9 | 2.4 | 2.0 | 1.7 |
| 2024 | 24.4 | 24.4 | 0.0 | 1.2 | 1.4 |
| 2025 | 19.6 | 19.5 | 1.1 | 1.1 | 0.2 |

- **The total stays real.** Real FBS is about 17 to 21 scholarship entrants a program in the December 2024 window, and about 20 to 25 a cycle with spring and walk-ons. The world is 18 to 24 a full cycle in both arms. With urgency on it is about 1 a program higher a cycle (about 5%). The told men take the place of lean draws (the per-program factor holds), so the rise is the camp trims, which add to the flow: real cuts to the limit go to the portal too.
- **Why men leave (FBS, all cycles):** playing time 50.3% → 39.5%, not in the plans 0 → 8.2%, cut to the limit 0 → 8.3%, coaching change 18.0% → 14.9%, spring depth chart 22.0% → 19.9%, dismissed 5.3% → 5.0%. About one entrant in six is now a man his staff moved on from, where before none were.
- **Who is told** (all cycles): FBS 1,449 (908 backups, 525 who didn't play, 16 rotation or starters), FCS 1,440, D2 1,525. 56% of FBS told men landed, 48% a level down; FCS 51% landed; D2 17%. These are men under their level's backup mark with no time left, so many drop a level or don't land, as in real life.
- **2024's zero (FBS):** the 2025 season is the House limit's first. Walk-ons on a 2024-25 roster never count against the 105 (the designated athletes, real), so every FBS program had room. Its staffs brought men in over their depth (4,873 "over him" calls) instead of telling anyone. FCS and D2 kept telling.
- **2025's January window:** 4,273 FBS move-on calls, but the quarter-of-the-flow cap told 1.1 a program. The first five-season year (2026) gives every man a year more, so the calls that remain are the men a year more can't save.
- **The market** (run 4, FBS): a median of 2.5 men a spot a window; the 10th percentile 0.2 to 0.5 (low-prestige programs at scarce spots). "The portal rarely has better" is now 5 to 8% of FBS calls and almost none at FCS and D2.
- **Starters among FBS entrants** rose from about 2% to 4 to 7% in the early cycles. The told men are almost all backups, so the scaled draws fall more on backups.
- **Rosters:** 429 men cut to reach the limit before camp over all years (none with urgency off); the FBS median roster on February 9, 2026 was 112 in both arms.
- **On February 9, 2026** (the world's last day, the next window eleven months away), the FBS staffs would move on from 12.3 men a program (18.9 with urgency off, where nobody was told in January). That is the list a staff carries to its camp trim.

Not measured here: your own seat's talks and meetings (unit tests and the persona evaluate test cover the words and the call), the spring window of 2022-24 (the AI tells only before winter, `GAPS.md`), and a full-scale world.
