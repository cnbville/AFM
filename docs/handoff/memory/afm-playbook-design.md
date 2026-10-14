---
name: afm-playbook-design
description: Plays and tactics database design rounds with Raichel (started 2026-10-14), topic order and answers
metadata:
  type: project
  modified: 2026-10-14T11:09:37.887Z
---

Design-only thread "plays and tactics database" started 2026-10-14; no engine code here (World Trial 3 thread owns physics/coaching fix; send it decisions that change that fix). Spec goes to docs/design/ when done.

Engine today: 16 pass concepts, 9 run concepts, 8 coverages, 5 pressures, 4 def personnel (game/playbook.rs, calling.rs).

Proposed topic order (2026-10-14, not yet confirmed): 1 what a play is made of + library size, 2 counters, 3 pre-snap (motion/checks/disguise), 4 playbooks per scheme/level + learning, 5 weekly game plan/call sheet, 6 who calls + coach quality, 7 tendencies/self-scouting, 8 user-built plays, 9 ideas spreading over years.

Topic 1 proposed: plays built from real parts (personnel, formation, motion, concept, protection, tags; defense: personnel, front, techniques, coverage, pressure, stunts, disguise). ~40 formations, 25 runs, 60 pass concepts + PA/screens/RPO/trick; ~15 fronts, 20 coverages, 40 pressures. Raichel asked 'more realistic?'; added (CONFIRMED 2026-10-14, all five; Raichel asked why not proposed first: lead with full realism, shorten wording not substance): size from real playbooks (NFL several hundred + variations, ~150 weekly), rule-based reactive jobs (blocking rules, route branches/sight adjusts, pattern-match coverage), real techniques/gap fits (force/spill/cutback), play-calling languages (WCO/E-P/Air Raid, learn speed), per-level plays and rules (HS wing-T/flexbone/single wing; college 3-yd ineligible downfield -> RPOs).

Topic 2 counters proposed (waiting): no counter table; geometry/battles decide (box numbers, high-low on a defender); conflict defenders (IQ/discipline); players can beat the call both ways; researched counter knowledge steers coach calls only; real splits only as dev checks. Raichel asked 'more realistic?' again; added (waiting): leverage/alignment, plays set up plays (per-defender game memory), play-action per research (works regardless of run success; keys/discipline), personnel matchup hunting + tempo/no subs, field (hash widths per level, weather, red zone), crowd noise on visiting offense (silent count, simpler calls). Topic 2 CONFIRMED 2026-10-14 (all); Raichel unhappy to need an extra prompt again. Raichel probes for more realism each topic: give the full version first.

Topic 3 pre-snap CONFIRMED 2026-10-14: motion info + defenses passing it off, shifts, QB check freedom set by coach trust (none/kill call/full), protection calls (Mike ID, slide), disguise/rotation (IQ vs IQ), defensive checks vs trips/motion, tells + film, communication per level (helmet radio NFL + college since 2024, HS signals), play clock/tempo limits, hard counts.

Topic 4 playbooks/learning CONFIRMED 2026-10-14: learn parts not plays (transfer across formations/systems), 3 depths (job/why/mastered; thinking = slow), position load (QB/C/Mike/S know all), sources (reps by depth, meetings, film, home study; IQ, work ethic, position coach), real practice limits (CBA/NCAA/state), coach sizes the book (skill), weekly wrinkles risk, fade + new system resets, mid-season package, unit chemistry (OL continuity). UI: knowledge per concept family + install progress.

Topic 5 game plan CONFIRMED 2026-10-14: film breakdown limited by staff count/skill (errors), real practice week per level (short weeks cut install, bye small edge), situational call sheet (script 15, downs, RZ, GL, 2-min, 4-min, backed up, shots; off-sheet less practiced), matchup plans + injuries, scout team quality, plans wrong vs tendency-breaking coaches, weather/venue, user builds or delegates + HC philosophy.

RULE (Raichel 2026-10-14): good plans/calls must be clearly rewarded (not broken) through battles, so learning real football pays; sent to trial 3 thread. Topic 6 CONFIRMED 2026-10-14: user takes any snap else play-caller (OC or HC-caller), accept/reject; quality = reading D, situational, game mgmt, creativity, adjustment (aggressiveness = style); dev check good vs bad plan swing in a real play-caller band (to research); user and coach same rules; pressure by personality; coordinator explains 'why' + postgame breakdown teaches; opponents adapt to exploits by skill.

Topic 7 tendencies CONFIRMED 2026-10-14: play log trail by down/dist/personnel/formation/zone/hash/score/motion; predictability punished via defenders reading sooner (IQ, film), no flat penalty; tendency breaking costs, right mix = skill (Shanahan example); player tells (habits, fixable); defensive tendencies; self-scout by QC staff (bye week), tendency breakers; film builds over season, new coach = old team film; tendency report accuracy by staff + sample.

Topic 8 play designer CONFIRMED 2026-10-14: same parts both sides + edit tags; legality per level; never graded, geometry decides; coordinator feedback by skill; install/learn cost; noisy practice read vs scout team; opponents learn from film (trick plays best first time); playbook travels to new job; AI coaches invent concepts (feeds topic 9).

Topic 9 spread CONFIRMED 2026-10-14: real 2026 mix per level; innovation from creative coaches often small schools, needs players; spread via success/trees/visits/film, openness personality; both ways between levels; defenses answer (cycles); ideas die and return; schemes shape player pool over decades; rule changes + analytics spread same way; trend lines per level, real history checks pace only. DESIGN DONE: spec docs/design/plays-and-tactics.md merged to main (c5edfea). Open: research play-caller band etc; build placement in roadmap order to ask Raichel.

Related: [[afm-engine-physics-fixlist]], [[afm-section3-design]]
