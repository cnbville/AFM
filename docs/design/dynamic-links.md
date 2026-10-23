# Dynamic Links: physical gifts that stack

Designed 2026-10-21 in the thread "Section 10: UI and screens", agreed by Cain the same evening (20:47, "agreed on all"). Cain first wanted it built later ("dynamic links is an new feature i want to implement later", 20:26); on 2026-10-21 (20:57) he put every designed feature into the final build, so it is built there (engine lane 3, PR #97).

How it started, in Cain's words:
- 20:26: "I want to make Geneticall traits important in this game, it's what could give a 2* a serious scholarship to a SEC Team because they see a physical freak. my idea was to have players that has (speed/acc above 90, size >6'3, reach = elite) earn a extreme highpoint catcher. Like those traits link so dynamically that it creates a monster."
- 20:32: "I just want to add a realistic dynamic to the gamne where people who are genetically gifted do get an edge." It is an edge, not a big stat boost.
- 20:36: "you're not thinking of placing limits right ?" No limits are added. The hidden ceilings that move every year (section 1, `docs/development-model.md`) stay as they are.
- 20:40: "instead having one genetic freak trait, i want to have multiple smaller ones, which could be stacked up on each other. Like i said, size, reach, jumping could get like a Highpointer trait, and then we can find other dynamic links that suiot eachohter".

---

## What exists

- **Built: the body.** Height, weight, arm length, hand size, wingspan and frame weight, drawn from the real combine by position (`body.rs`, `athlete.rs`).
- **Built: the physical attributes.** Speed, acceleration, agility, strength, balance, flexibility (bend), jumping and stamina; recovery is a hidden 1-20 trait (`docs/design/attributes.md`). Each has a hidden ceiling that moves with work ethic, coaching, coachability and playing time, and fades with age: speed and agility first, from about 27 (`docs/development-model.md`).
- **Built: each trait already does its own job in the engine.** A receiver's catch radius grows with height, arm length, jumping and body control. A defender's reach at the catch point grows with height and jumping. A rusher's arms and height bat passes, and a tall quarterback gets fewer batted. Arm length adds tackling reach. Contact uses mass times speed. (`game/scrimmage.rs`, `game/sim.rs`.)
- **Decided: how staffs see bodies** (`docs/design/scouting.md`): measured against listed numbers, the athletic read rating (how well a scout turns tests and movement into attributes), "tests measure the test", and each scout's taste for a body type.
- **Found 2026-10-21, gaps in the physics underneath** (real life settles each, so they are bugs, fixed in the same build):
  1. In a contested ball the fight ignores both men's size: a 6'5" receiver beats a 5'9" corner no more often than a 5'9" receiver does. The defender's reach counts his height and jump but not his arms.
  2. The quarterback's read counts only separation (`window`), so a covered big receiver is never open for a jump ball or a back-shoulder throw, though real quarterbacks throw those to him every week.
  3. A player's value is a straight sum of his attributes (`positions.rs`, `overall_for`). Height and arms count only as some scouts' taste (`world/pro/scouts.rs`), and the recruiting services' stars are 0.25 film and 0.75 projection. Nothing pays for traits that can't be coached, which is what real staffs pay for first.

## How a link works (agreed)

- **A link is 2 to 4 body traits,** measured against his position on the real combine numbers the game already has. The weakest part sets its strength, so one elite trait alone makes no link.
- **It grows smoothly, with no cliff.** It starts once every part is in about the top 30% at his position and is full near the top 5%. A receiver at 6'2.9" and one at 6'3.1" play the same. Because it is measured against the combine, a link is rare in high school and common at the top of the NFL, as real freaks are.
- **It is an edge, not a boost.** A full link is worth about what 5 rating points would add (question 13), and only in its own situations. Each trait keeps doing its own job in the engine; the link adds only what the traits give together.
- **Body only** (question 14). Skills are never part of a link. The skill decides how well he uses the gift: a Highpointer with poor hands still loses balls he should win.
- **Links stack.** Links that act in different situations each count in full, so more links make a more complete player, not one huge number. Two links in the same situation: the stronger counts in full and the other at half (question 15), so it can't run away.
- **Links change over a career.** A weight program can build Anchor or Battering ram. Age takes Burner first (speed fades from about 27), while Highpointer lasts longer because height and arms don't age (jumping does). A major injury, which already costs speed, acceleration, agility and jumping, can cost a link: "he lost a step".
- **No limits.** Links add nothing that caps anyone. A player without links can still become great through his skills (Cooper Kupp ran a 4.62 and won the receiving triple crown in 2021), and a player with links can bust without them (Darrius Heyward-Bey ran a 4.30 and went 7th overall in 2009).

## The 12 links (agreed)

| # | Link | Body traits | Where it gives the edge | Real example |
|---|---|---|---|---|
| 1 | Highpointer | Height, arm length, jumping | Balls in the air: jump balls, back-shoulder throws, fades in the red zone. A tall corner or safety uses the same gift to play the ball at its highest point. | Calvin Johnson: 6'5", 239 lb, 4.35, 42.5" vertical |
| 2 | Burner | Speed, acceleration | Separation on deep routes, breakaway runs and returns, a beaten defender's recovery, pursuit | Tyreek Hill, DeSean Jackson |
| 3 | Sudden | Acceleration, agility, balance | Changing direction without slowing: separation at the break, making the first man miss, mirroring a route | The 3-cone and short shuttle measure it |
| 4 | Low center of gravity | Short for his position, heavy for his height, balance, strength | Hard to knock over: staying up through contact, the low man winning leverage | Barry Sanders (5'8"), Maurice Jones-Drew (5'7") |
| 5 | Battering ram | Weight, speed, strength | Contact at speed: breaking arm tackles, wearing a defense down late; on defense, the thumper's hits. The engine already does part of this through mass times speed. | Derrick Henry: 6'3", 247 lb, 4.54 |
| 6 | Length | Arm length, wingspan, hand size | First contact in blocking and rushing, press at the line, batted passes, tackling reach, the catch window | Scouts want tackles with arms near 34 inches |
| 7 | Big man who moves | Weight, acceleration, agility, judged among linemen | Pass sets against speed, pulling and reaching the second level; on defense, shooting gaps and splitting double teams | Tristan Wirfs: 4.85 at 320 lb; Aaron Donald: 4.68 at 285 lb |
| 8 | Anchor | Weight, strength, balance | The bull rush and double teams, on both sides; short-yardage push | A 340 lb nose tackle |
| 9 | Get-off | Acceleration, jumping power | The first step off the snap | Scouts time the 10-yard split and the broad jump |
| 10 | Bend | Flexibility, balance, speed | Dipping under the tackle's hands around the corner without slowing; a tight turn at speed | Von Miller |
| 11 | Lungs | Stamina, recovery | Staying fresh late in drives and games, fewer breaks, the fourth quarter | Every-down players |
| 12 | Pocket frame | A quarterback's height, weight, hand size | Seeing over the line, fewer batted passes, shrugging off arm tackles in the pocket | Ben Roethlisberger, Josh Allen |

Stacks: Highpointer plus Burner is the Calvin Johnson type. Length plus Burner is a rangy safety or a press corner. Get-off plus Bend plus Length is an elite edge rusher. Low center of gravity plus Sudden is the Barry Sanders type. Big man who moves plus Anchor is an elite tackle.

## What the world does with links (agreed)

- **Your staff names them** in words with the measured numbers: "Highpointer: 6'4", 33¾" arms, 38" vertical". Before he is measured it is a guess from film ("plays above the rim") and can be wrong; evaluators with a better athletic read see links earlier and truer. Listed numbers run tall, as real school listings do.
- **College staffs value them,** because traits can't be coached. The services rank mostly from film and fall behind, so a little-known kid who tests at a June camp gets the offer before his stars catch up: the 2-star with the SEC scholarship.
- **NFL clubs value them** in their grades, and the combine and pro days reveal them. A link without skill is the "workout warrior" risk scouts argue about in the draft meeting.
- **Opposing coaches plan for them** when they know them: a safety over the Burner, a bracket on the Highpointer in the red zone, a chip on the Bend rusher. That help opens up his teammates.
- **Staffs use them for position moves:** a Highpointer tight end may be a receiver, and a Big man who moves on the defensive line may be a tackle.
- **The news uses them:** "a physical freak", "a workout warrior", "he lost a step".

## Build order (later)

1. Fix the three physics gaps above, so each trait does its real job: at the catch point each man's highest reach is his height plus arm length plus vertical, less what running full speed costs his jump, against how high the ball is when it arrives; the quarterback sees a size mismatch on jump balls and back-shoulder throws; staffs' projections pay for measured traits that can't be coached.
2. Add the 12 links on top, each adding only its together-extra in its own situations.
3. Show them on screens: the player page, a Links column in the squad and board tables, chips on the recruiting board and the draft board.

## What the build must prove

Match checks and unit tests only, each timed (no world runs):
1. How common each link is at each level (high school, FBS, NFL), rising with the level.
2. Each link wins its own situations more often, by about the agreed size, and changes nothing else.
3. The stacking rules hold: links in different situations add in full; in the same situation the second counts half.
4. Every level's averages stay in their bands (`match_check` for the NFL, FBS and high school, before and after).
5. The physics: a 6'5" receiver wins a contested ball against a 5'9" corner more often than against a 6'2" one, and quarterbacks throw jump balls to a covered big receiver.
6. Recruiting: of two 2-stars alike in everything else, the one with strong links draws more and better offers.

## Proposed, not confirmed

- The Dynamic Links button on the player page (`screens.md` topic 5, a placeholder since 2026-10-20) opens the player's links with their numbers and what each does.
- How much the physics fixes weigh (the jump lost at full speed, how much a size mismatch moves the quarterback's read), set by the match checks.
- How strongly AI game plans react to a link (help coverage, brackets, chips), from the coordinator's own scouting.
- The situations listed for each link beyond the ones posted in the thread (Low center of gravity for blockers, Pocket frame's grip in rain and cold).
