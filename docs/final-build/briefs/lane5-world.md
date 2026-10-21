# Lane 5: World fixes

Read first: `CLAUDE.md`, `../plan.md`, `../list.md` (A3, A6, A11, A12, A13; U-d, U-e; S11), `docs/trials/world-trial10.md`, `docs/world-model.md`, `docs/design/people.md` topic 9, `docs/research/hs-playoff-formats.md`, `docs/design/calendar.md`, `docs/design/calendar-checklist.md`. Work in progress: `claude/final-wip-lane5` (long college roads started).

- A3: the long college roads the world lacks (2020 COVID extra year, medical hardship, juco, D2/FCS start then moving up, missions, military, older walk-ons); add only the missing ones, at real rates.
- A6: dynasties (Georgia 18, Texas 10 national titles of 31): add the real levelers (coaching turnover and poaching, roster and scholarship limits, NIL spread, portal losses, schedule luck), from how real dynasties end.
- A11: Mater Dei won 248 straight: add the real HS leveler.
- A12: 5-stars drafted 87.6% (real about 62%): find and tune the part that runs above real.
- A13: FBS rosters median 86 (real about 105): more local walk-ons stay (real walk-on rules and the 2025 roster cap of 105).
- U-d: real HS playoff formats and private-school rules per state, filling the 10 states' gaps from each association's handbook; today each classification takes its best 2^rounds (`world/hs_year.rs`, the playoff field).
- U-e: weather from each town's real climate normals (NOAA 1991-2020), not one main town per state (`calendar/weather.rs`): a compact table for the towns the world uses (or nearest station).
- S11: 5-in-5 eligibility, real since June 2026 (on the agreed off-field change list, `calendar/rules.rs` `FiveInFive`): redshirts end when it starts; verify the date against a source.
- Real AP, Coaches and Heisman voters (people topic 9) replace the placeholders in `world/college.rs`, with their real habits (preseason anchoring, brand bias, loss timing), and the committee for the playoff.
- Check the NFL draft early-entry withdrawal date against a source and fix the calendar if wrong.
- Stale docs: `docs/handoff/open-questions.md` (mark what's since decided or built), the Dynamic Links lines saying "built later", the placeholder JUCO line, the GAN wording in `docs/design/faces.md` (faces are now 3D heads drawn by the game).

Agreed fixes may change results per seed; nothing else does. Measure each fix on a short world (e.g. `cargo run --release --bin world_trial -- 2026 12`, or the faster settings in CLAUDE.md) and report before/after against the real targets. Parked tuning (NFL scoring, ypa) is not yours. Update `docs/world-model.md`.
