# Gaps

Gaps threads noticed and haven't built (the bug and improvement process, Cain 2026-11-17). Each: what's missing, why it matters, impact. Remove an entry once it is built. Building one outside its own task needs Cain's yes.

## Engine

- **Projects follow the player, not the staff that set them.** A skill or body project is stored on the player and nothing ends it when he changes team: a high school project rides into college, a portal transfer or a drafted man carries his old staff's project to the new one. Rooms then run past their slots (a 2026 test world, scale 0.03: your new college's rooms held 16 skill projects on 14 slots, RB 2 of 1, D-line 3 of 2, S 3 of 1, and a QB was in year 5 of his). The AI's own rooms are hit the same way (`ai_room` counts them as running, so it only stops starting new ones). Real staffs decide their own projects, so the fix is to end a project when he moves to a new staff and let that staff pick at its next checkpoint. It changes world results, so it waits for Cain's yes. High.
- **A project's minutes don't come out of the room's week.** Design item 8 says a project takes the position coach's one-on-one minutes, so the rest of the room gets a little less. Today the 60 minutes only set the project's pace (`development.rs`, `GrowthCtx::rate`); nobody else's individual time shrinks, for you or the AI. Without it a full room of projects is free. Medium.

## Logic

- **No word when a project clicks, stalls or is dropped.** A click (6-10 points on the ceiling), a dead end your staff would see, or a project dropped because he moved to a position that doesn't work the skill (`progression::project_year`) sends no note to the inbox or the staff notes. You only find out by opening his Development tab. Medium.
- **The knack clues in Reports.** The progression design puts the knack's vague clues on the Reports sub-tab; only the one-word read on the projects block exists. Medium.
- **Projects a player brings from his last staff run until you end them.** The AI's dead-end rule (`ai_keeps`) doesn't run on your team, so an inherited project keeps costing his other technique a quarter of its growth. The screen shows it as his last staff's with an End button, and the Training screen marks a room past its slots, but no note tells you it is there. Low (most of it goes with the engine gap above).

## UI

- **No project column on the Squad's Development view.** You see who is on a project only on the Training screen's room count or each player's own tab. Low.
