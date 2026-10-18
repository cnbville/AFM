# The attribute list (67 visible, 5 hidden)

Raichel agreed on 2026-10-18 (thread "Full build and trial 7"): build this
list instead of the 38, then retune the engine on it; tendencies (scrambles,
gambles on routes, bounces runs, goes for the strip) become their own layer
after this build, with a short design round. The research behind it (the
engine moments, real scouting grids, other games, the gap and double-count
check) is in the project folder at `afm/attributes/` (`proposal.md`,
`research.json`).

Code: `crates/afm-engine/src/attributes.rs` (the list, labels and growth
profiles), `positions.rs` (what each position needs), `athlete.rs` (how a
new player gets them), `game/` (what each one drives on the field).

## The list

All 1-99, each with a hidden ceiling, growth window, coachability and decline
(see `docs/development-model.md`).

- **Physical (8):** speed, acceleration, agility, strength (raw strength; how
  strong he plays comes from strength, balance, bend, hand use and mass),
  balance, flexibility (bend), jumping, stamina.
- **Passing (11):** arm strength; short, medium and deep accuracy (0-9, 10-19
  and 20+ air yards, blended so there are no cliffs); touch; throwing on the
  move; release speed; footwork; pocket presence; ball handling (fakes,
  handoffs); eye manipulation (looking off the safety).
- **Running (4):** ball security, vision, elusiveness, leg drive (pad level,
  yards after contact).
- **Receiving (6):** catching, contested catches, ball tracking, body control
  (toe-taps, adjusting to a bad ball), route running, release off the line.
- **Line play (8):** pass blocking, run blocking, blocking in space, hand use,
  anchor (bull rush, double teams), power rush, finesse rush, block shedding.
- **Defense (8):** man coverage, zone coverage, press, disguise, tackling
  (with wrap range), pursuit, blitzing, moving through traffic.
- **Kicking and snapping (10):** kick power, short and long kick accuracy (one
  gift with a lean, blended by distance), kick trajectory, kickoff technique,
  punt power (hang comes from the flight), punt placement, get-off time,
  long snapping, shotgun snapping (centers).
- **Mental (12):** football IQ, play recognition, anticipation, decisions,
  composure, concentration, discipline, communication, toughness, motor,
  competitiveness, physicality.
- **Hidden (1-20):** consistency (game-to-game form), temperament (after the
  whistle), recovery (between games), injury proneness, longevity. The first
  three drift a little with age: consistency and temperament settle before
  26, recovery fades from 30 (proposed, not confirmed).
- **Not ratings:** throwing hand and kicking foot. About 10% throw left-handed;
  60% of them and 12% of right-handers kick left-footed (proposed, not
  confirmed). A left-handed quarterback's blind side is his right, so his best
  pass protector plays right tackle.

Cut on purpose: snap speed (part of long snapping), hang time (physics from
punt power), holding (the holder reads catching and concentration),
versatility and adaptability (football IQ and section 6 personality), clutch
(part of composure), hit power (mass times speed), leadership (its on-field
half is communication; the rest stays personality).

## How players get them

- Athlete-first creation (`athlete.rs`) draws the body and physical traits,
  then skill gifts that feed several attributes at once (an arm gift feeds arm
  strength; touch feeds the three accuracies and touch; a kicking leg feeds
  kick and punt power; football IQ feeds IQ, recognition, anticipation and
  decisions). Short and long kick accuracy share one gift with a split, so one
  kicker is lethal short and another long.
- Toughness, motor, competitiveness and physicality share one "competes" gift
  with the personality's competitiveness, so a player stays coherent (top
  motor with no competitiveness is rare).
- Background: hardship fuel raises the toughness ceiling (half as much
  competitiveness); a football bloodline raises football IQ.

## Trait-like ratings are read against the level

Discipline, toughness, motor, composure and concentration were hidden 1-20
traits with the same spread at every level. As attributes they grow with the
level, so the engine reads them against a typical player at that level
(`game/rules.rs`, `trait_norm`; `GamePlayer::trait20`): officials call each
level to its own standard, as real penalty rates are about the same in the
NFL and FBS (about 6 a team game). The norms were measured on the synthetic
rosters (proposed, not confirmed).

## What each attribute drives on the field

One main owner per engine moment, so nothing counts twice. Every new read is
centred at 60 (a 60-rated player plays as the engine did before) and bounded;
trait-like ratings are read against the level (above). All sizes are
proposed, not confirmed. Files: `sim.rs` (movement, blocks, tackles),
`scrimmage.rs` (the snap: passing, coverage, busts), `flow.rs` (pre-snap
fouls, units, rotation), `kicking.rs`.

- **Moving:** speed, acceleration (top speed and burst), agility (cuts, man
  coverage lag with flexibility), balance (staying up in contact, trips and
  arm tackles), stamina (fatigue), moving through traffic (how much bodies
  around him slow him).
- **Throwing:** arm strength (range, deep balls), short, medium and deep
  accuracy (placement, blended by air yards), touch (a share of placement on
  10-25 yard throws with a defender under the path, and softer short balls),
  throwing on the move, release speed (time to get it off; batted balls),
  footwork (time to set; a share of the release), pocket presence (his vision
  in the pocket), ball handling (play-action and jet fakes, the handoff),
  eye manipulation (looking off a deep defender on the first read),
  decisions (forcing a throw, throwing it away, intentional grounding).
- **Running and catching:** ball security (fumbles), vision (the carrier's
  vision cone), elusiveness, leg drive (yards after contact); catching (the
  catch), contested catches (the catch under contact; a defender's at the
  catch point), ball tracking (reading the flight, muffs), body control
  (catch radius and balls off his frame, toe-taps), route running (settling
  in zone holes with football IQ), release (off press and as a gunner).
- **Blocking and rushing:** pass and run blocking (the rep), blocking in
  space (getting hands on a man in space: climbs, screens, return walls,
  illegal blocks in the back), hand use (holding, sustaining a won block),
  anchor (double teams, the bull rush), power and finesse rush (the move),
  block shedding; a stalled rusher's counter move reads football IQ and
  motor.
- **Defending:** man and zone coverage (zone with play recognition),
  press (the jam, the vise on punts), disguise (the shell before the snap,
  what motion shows), tackling (the tackle and wrap range), pursuit,
  blitzing (beating the back's pickup), anticipation (jumping routes, the
  quarterback's eyes, batted balls, the get-off and offside).
- **Mental:** football IQ (assignments, blitz pickup, alignment fouls,
  ineligible downfield), play recognition (the vision cone, motion reads),
  communication (the caller's unit busts less: quarterback, center, the
  green-dot defender), concentration (crowd noise, false starts, late-drive
  lapses, the silent count), discipline (fouls, biting on fakes), composure
  (pressure, kicks), toughness, motor (effort through the whistle, letting
  go of a block), competitiveness (piles on short yardage), physicality
  (finishing blocks and pancakes, committing to the hit).
- **Kicking and snapping:** kick power (distance), short and long kick
  accuracy, kick trajectory (blocked kicks), kickoff technique (hang and
  landing spot, onside kicks), punt power, punt placement (pinning at the
  numbers), get-off (time to kick; blocked kicks and punts), long snapping
  (bad snaps, snap time), shotgun snapping (bad shotgun snaps).

Checked on the repo's match checks only (no trial): batted passes are 1.6% of
NFL throws (real about 1.5-2%); offside 0.56 a team game (real 0.52). High
school completion fell from 49.5% to 47.2%, mostly body control (high school
receivers adjust poorly to a ball off their frame). Not modelled: the ball's
height (touch only takes a share of placement) and a fumbled shotgun snap
(a bad one costs time only). Scouting reports grade the overall only, so
there is no per-attribute scouting noise yet.

## Proposed, not confirmed

Claude's picks inside the agreed list, for Raichel to adjust:
- the growth and decline profile of each new attribute, and the QB passing
  skills growing into the late 20s (so QBs still retire about two years after
  backs on the class checks: 30.5 against 28);
- the position weights (`positions.rs`): every position also weighs a little
  of the universal mental traits;
- the handedness rates above;
- the combine: the 40 reads game speed plus a stable track offset (sd 0.04 s;
  some play faster than they time) and 0.03 s of noise a run; the 3-cone also
  reads flexibility, as scouts use it as the bend test;
- the trait norms by level (NFL 67, measured on rosters like the world's);
- the hidden traits' drift with age;
- the retune on the new list (`docs/match-engine.md`, "The 67 attributes and
  the retune"): placement 0.5% a point of accuracy, footwork, release and
  anticipation at half their first size, holding and fumbles read against the
  man across from him, back committees (lead back about 65% of the backs'
  carries in an NFL game, a star about 82%), and synthetic NFL rosters shifted to look like the world's. NFL
  scoring is 26.5 points a team game on them (real 22.5); the agreed 20-25
  band was not reached, so the test band is 20-28 until the next trial
  measures the world.
