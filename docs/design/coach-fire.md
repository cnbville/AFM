# Coach fire on the field: fire, temperament and the big play

Thread "Let coach fire carry onto the field", 2026-11-17. Cain, 01:01 UTC: "if i'm a fired up coach, playing with a fired up safety, it should cause a pivotal play. Like a strong safety taking the head of a tight end i.e Brian Branch". 01:02 UTC: "but also it could have a negative effect on a pocket passer, who'd be better of with an patient gentle coach".

Status: agreed. Cain said "yes" on 2026-11-17 01:11 UTC to the whole proposal and its three questions (section "Cain's answers"). Every size below is proposed, not confirmed, until the build measures it, except the caps of question 2. Built 2026-11-17 in one engine lane (thread "Build the coach fire engine lane"): how it works in `docs/reference/how-it-works/match-engine.md`, "Fire on the field", with every size the build picked (all proposed, not confirmed); what it measured in `docs/match-engine.md`, "Coach fire" (`fire_check`).

This extends `docs/design/coach-voice.md` section 5: still no flat bonus, but the built fire channel now works by the kind of work and through habits, so the coach's voice and the player's temperament reach the field.

## What is built today (not re-asked)

- Each player's game-day mind: fire, poise, focus (0-1), from the stakes, his temper, competitiveness, composure, concentration, years, mood and his practice week (`game/talk.rs`, `mind_now`). Pregame, halftime and sideline talks move it (`land`).
- Each position has one best pitch of fire (`ideal_fire`: kickers 0.34, QB 0.42, S 0.56, guards 0.62, edge 0.66). Moving toward it lifts **every** rating by up to 1.5 points; overshooting by 0.18 costs focus; past 0.72 fire a hot head adds late hits (`Pulse.heat`). It fades through the half.
- Momentum: per player only, -3 to +2 rating points, from interceptions, sacks, drops, fumbles, missed tackles and touchdown carries, softened by Composure (`game/flow.rs`, `edge_hit`, `decay_edges`). Big hits do not count. Teammates don't share it.
- Tendencies BigHit, Strip, Downhill, Contact, SnapTiming, JumpRoutes, each with its payoff and its risk already in the engine: BigHit x(1 + 0.2 lean) hit force, x(1 + 0.7 lean) miss chance, x(1 + 0.6 lean) personal fouls; Strip x(1 + 1.15 lean) fumbles; Downhill bites on fakes x(1 + 0.9 lean); SnapTiming +12 x lean get-off (offsides); the safety's hit at the catch point (breakups, unnecessary roughness on a defenseless receiver) (`game/sim.rs`, `game/scrimmage.rs`).
- Fumble chance rises with hit force^1.5 (`hit_effects`).
- Coach voice (agreed 2026-11-17, not built): your manner lands on players' trust, mood and game-day fire, poise and focus by their makeup and your relationship.
- Not in the engine: targeting or any ejection.

## The gap

Fire is a flat lift on everything. A fired-up safety gets +1.5 on his coverage reads the same as on his hits. It never changes how often he goes for the knockout, how hard he arrives or how hard he chases. And a fired-up pocket passer only pays through a small focus cost.

## Real basis

- Arousal and the kind of task: high arousal helps gross power, speed and effort tasks and hurts fine, complex and decision tasks (Oxendine, "Emotional Arousal and Motor Performance", Quest 13, 1970, the classic table in sport psychology texts; the inverted U).
- Each athlete has his own best zone (Hanin's individual zones of optimal functioning): some play best hot, some calm.
- Momentum: Fry and Shukairy (Journal of Quantitative Analysis in Sports, 2012) found no measurable momentum in the NFL after turnovers or scores allowed; the one possible exception was a 4th down conversion. Next-drive gains after a turnover come from field position.
- Contagion: small and short (Moll, Jordet and Pepping 2010, penalty shootouts: celebrating a goal linked to teammates scoring next; Totterdell 2000, mood linkage in cricket teams).
- Brian Branch (Lions): known for big hits, ejected Nov 3 2024 for a helmet-to-helmet hit on Bo Melton (plus an unsportsmanlike flag on the way out), fined in 2025 for unnecessary roughness on Tucker Kraft, Trey McBride (use of the helmet) and a Giants receiver (defenseless), and suspended for a post-game fight in 2025. The same fire that makes the hits brings the flags.
- College targeting: 15 yards and ejection since 2013; in the second half he also sits the first half of the next game.

## Proposal

1. **His own sweet spot.** Today's position pitch, moved by his makeup: hot temper, high competitiveness and Physicality play best hotter (up to +0.08); low Composure, high Concentration-dependent jobs cooler (down to -0.06). Branch-type safety 0.56 -> 0.63. A calm pocket passer 0.42 -> 0.38.
2. **Fire split by kind of work** (replaces the flat lift on everything). Power and effort parts rise with fire up to his sweet spot: Strength, Lower Body, hit power, Pursuit, Motor (effort when chasing), a little burst. Fine and thinking parts fall once fire passes his sweet spot: the accuracies, Touch, Decisions, Play Recognition, Anticipation, Discipline, Concentration, hands. Under his spot he loses effort, not thinking.
3. **Fire moves how often he tries**, not just how well: BigHit, Strip, Downhill, Contact, SnapTiming and JumpRoutes lean up with fire over his spot, down when cool. The built checks then give the payoff (harder hits, more fumbles at force^1.5, more breakups at the catch) and the cost (more misses, more personal fouls, biting on play action, offsides, beaten on a double move).
4. **Targeting and ejections** (real, so not asked): a high, helmet-led hit on a defenseless man draws targeting in college (15 yards, ejection, the next game's first half if in the second half) and unnecessary roughness in the NFL, with an ejection for the flagrant ones. Chance from his BigHit lean, fire, Discipline and temper, the target's state (defenseless at the catch, a sliding QB). Fines and suspensions follow for pros.
5. **The spark.** A big hit (top 3% of hit force at the level), a forced fumble or a 4th down stop counts on the built momentum scale for the hitter (+1), and spreads a small share to teammates on the field by their own fire (up to +0.4 points, a few snaps). The man hit "hears footsteps": his Contact lean drops for a stretch (short-arms over the middle more, a built check), sized by his Toughness and Composure.
6. **The pocket passer.** Through 1 and 2: a fiery room pushes a calm passer over his spot, so his accuracy and decisions drop; a talk aimed at him also costs poise, and poise feeds his panic under pressure (built). A patient coach keeps him at his spot with more poise. A gunslinger with a hot temper has a higher spot and minds less.
7. **Talent still wins.** Fire adds at most a few points and a few tendency points, so a 70-tackling safety at his best pitch still hits softer than an 85 at calm.
8. **Your side on screen.** The play log tags a fire-driven play ("he came downhill hot"), the man hit gets a "heard footsteps" tag, and the sideline reacts. Nothing shows hidden numbers.

## Cain's answers (2026-11-17 01:11 UTC, "yes")

1. Fire stops lifting every rating and works by the kind of work: power and effort up toward his own pitch, fine and thinking work down past it. Every team's games change, so worlds change.
2. The caps: fire adds at most 3 points of power and effort, at most 3 tendency points to the habits in item 3, and takes at most 3 points of fine and thinking work. The season effects (about +10% forced fumbles, +30% personal fouls for a Branch type under a Firebreather) are guesses the build measures.
3. The spark stays at the size the research allows (hitter +1 on the momentum scale, teammates up to +0.4 for a few snaps, the man hit flinches over the middle for a stretch) and shows clearly on screen (play log tags, "heard footsteps", the sideline reacting).

## Measuring

The same Branch-type safety and the same pocket passer under each prototype room, A/B on the same seeds (`ab_check` style, 272 NFL games each arm): forced fumbles per tackle, missed tackle rate, personal fouls, breakups; the passer's completion over expected, sacks and interceptions. League totals must stay at real (built real targets for forced fumbles per tackle, missed tackles, fouls).

## The lane

One engine lane after the coach-voice lane merges (it reads the room's fire that lane sets) and after the world rebuild, like the scheme-fit engine lanes. Opus max (engine lane). It updates `docs/reference/how-it-works/match-engine.md` and `tendencies.md`.
