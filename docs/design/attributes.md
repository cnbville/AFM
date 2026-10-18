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
centred at 60 (a 60-rated player plays as the engine did before) and bounded.
The full table is below once the engine wiring lands.

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
- the trait norms by level;
- the hidden traits' drift with age.
