# 06 Rewrite engine in Rust

Thread "Rewrite engine in Rust" from the AFM project on the old account (thread id cmsg_015NJfb6NRUve4Wa2buMtZKs2sFCdLpmTWQPYRsf56GPMD). Every message, oldest first, copied word for word on 2026-10-14. 8 messages.

### 2026-10-11 19:26:55 UTC · Raichel (member)
<sub>id cmsg_015NJfb6NRUve4Wa2buMtZKs2sFCdLpmTWQPYRsf56GPMD</sub>

46. agreed.  49. Okay you can rewrite into rust, do this in a new thread and immeidately push to git to make sure everything is updated and we can continue with the actual roadmap and game. 50. Still don't understand why you need a different first season ? or are you suggesting to do that so that the rest of the world can set up ?

### 2026-10-11 19:27:12 UTC · Claude
<sub>id cmsg_015NJfb6NRUve4Wa2buMtZKs2g39MkxfjZ1znThGiz3mF4</sub>

I'm reading the repo and porting the player model to Rust, and I'll message you once it's on main.

### 2026-10-11 19:27:25 UTC · Claude
<sub>id cmsg_015NJfb6NRUve4Wa2buMtZKsLdcTQjehau7o2A2eugSNWo</sub>

*[Progress checklist update]*

```text
Rust engine port

✱ Reading the TypeScript model
○ Set up Cargo workspace, move TS to legacy-ts/
○ Port modules and tests
○ Check calibration numbers against TS
○ Update CLAUDE.md and docs
○ Open PR and merge
```

### 2026-10-11 19:33:14 UTC · Claude
<sub>id cmsg_015NJfb6NRUve4Wa2buMtZKs2xFpCQTMWVnDFDQgU4RY5A</sub>

*[Progress checklist update]*

```text
Rust engine port

✓ Read the TypeScript model
✓ Set up Cargo workspace, moved TS to legacy-ts/
✱ Porting modules (rng, attributes, positions, player, development, scouting, level ratings done; league and career next)
○ Port tests
○ Check calibration numbers against TS
○ Update CLAUDE.md and docs
○ Open PR and merge
```

### 2026-10-11 19:40:26 UTC · Claude
<sub>id cmsg_015NJfb6NRUve4Wa2buMtZKsV7cTmPHEcKhij63YtMAwbk</sub>

*[Progress checklist update]*

```text
Rust engine port

✓ Read the TypeScript model
✓ Set up Cargo workspace, moved TS to legacy-ts/
✓ Ported the engine modules
**✓ Ported unit tests (18 pass)**
✱ Porting the national-class summary and calibration tests
○ Check calibration numbers against TS
○ Update CLAUDE.md and docs
○ Open PR and merge
```

### 2026-10-11 19:45:36 UTC · Claude
<sub>id cmsg_015NJfb6NRUve4Wa2buMtZKsQ7MZXXQtyHqJa3kHEgBybE</sub>

*[Progress checklist update]*

```text
Rust engine port

✓ Read the TypeScript model
✓ Set up Cargo workspace, moved TS to legacy-ts/
✓ Ported the engine modules
✓ Ported unit tests (18 pass)
**✓ Ported the national-class tests and calibrate tool (250k class: 2s in Rust vs 57s in TS)**
✱ Comparing calibration numbers against TS on several seeds
○ Update CLAUDE.md and docs
○ Open PR and merge
```

### 2026-10-11 19:49:11 UTC · Claude
<sub>id cmsg_015NJfb6NRUve4Wa2buMtZKsJ1CkciU5G1GwnZ2v1wwhiV</sub>

*[Progress checklist update]*

```text
Rust engine port

✓ Read the TypeScript model
✓ Set up Cargo workspace, moved TS to legacy-ts/
✓ Ported the engine modules
✓ Ported unit tests (18 pass)
✓ Ported the national-class tests and calibrate tool
✓ Compared calibration against TS on 6 and 9 seeds
✓ Updated CLAUDE.md and docs
**✓ Opened and merged PR #3 into main**
```

### 2026-10-11 19:49:19 UTC · Claude
<sub>id cmsg_015NJfb6NRUve4Wa2buMtZKs19GYbjxwyJ3fEH4ooMNxFH</sub>

The Rust engine is built and merged into main ([PR #3](https://github.com/cnbville/AFM/pull/3)). Nothing is needed from you to keep going.

**What was ported:** everything in the player model. That covers attributes, positions, player generation, development, scouting, level ratings, the league, recruiting with film line C, college and the portal, the draft, pro careers, and all trial 6 fixes. The old TypeScript code is kept in `legacy-ts/` for reference.

**Speed:** a full class of 250,000 players now takes about 2 seconds. TypeScript took 57.

**Tests:** all 38 pass. That's the 35 old ones plus 3 new ones (the random generator, the potential outlook, and getting the same result on 1 core or many).

**Calibration:** I compared 6 TypeScript runs with 9 Rust runs, each with a different seed. Every number lands within normal run-to-run variation. For example, five-stars drafted came out at 61-91% in TypeScript and 76-88% in Rust. FCS players are still too rare among draft picks, at 2-5% against about 6% in reality.

I widened one test limit: a veteran's leftover upside at 33-35 may now be up to 1.5 points instead of 1. TypeScript itself went over 1 on one run.

**Hidden potential:** the game can no longer see a player's true ceiling, because the engine code itself blocks it. Only the test and trial tools can read it. The game gets a vague outlook in words, like "could become an NFL starter", plus how sure the scout is. Scouts rated 13 or higher also add hints, like "still growing into his body" or "a tireless worker", and better scouts give more of them and get them right more often. I built these tiers and hints as placeholders, so we can change them when we design scouting.

**Defaults I picked:**
- One engine library, plus a separate tools part for tests and calibration.
- No game screens or app shell yet.
- Rayon for using all processor cores. serde is used only in the tools.
- Each player gets his own random stream, so a seed gives a different class than it did in TypeScript, with the same overall numbers.
- The trial card script stays in TypeScript for now.

All of these are listed in `docs/decisions.md`.

**About 50:** you're right that the game doesn't need a separate first season. It was only a suggestion for the first point where you can open and play the game. The world (high school, college and the league) would already be simulated and running in the background, and you'd start your career straight away from mid-February 2026. The real question is just what we build first so you can start playing.
