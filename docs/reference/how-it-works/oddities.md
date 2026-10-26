# Code oddities found while writing the reference

Checked against main at 94a6553 on 2026-10-25.

Places where the code does something its own comment, a design doc or the obvious intent says it shouldn't. Each is stated as what the code does. None is fixed: fixes wait for Cain's yes or no. Remove a line when it is fixed or decided.

1. A holdout's camp "dip" has the wrong sign: `adjustment -= 0.3`, and a negative adjustment raises form and performance in `develop_season` (`world/pro/mod.rs`, `camp`; `development.rs`).
2. Program `prestige` and `facilities` are stored 0-1, but `personnel` divides them by 100 and 20, so no FBS staff gets more than three personnel posts or a general manager (`world/scouting/readers.rs`, `personnel`). One public analyst's "big programs" lean checks `prestige >= 60.0` and never fires (`world/scouting/public.rs`).
3. Small-school players flagged by the stats footprint get a December read at accuracy 8 instead of 13; the comment says "truer read", but lower accuracy is a wider error (draft.md).
4. A cut creates a grudge only for waived players: for a veteran released before the deadline, the contract is cleared before the "deal ran out" check (`world/pro/mod.rs`, `release_with`). Trades never create one, though `grudge`'s comment says "cut or traded".
5. `InjuryKind::hampers` is never called: a hurt player takes a flat -3 on every rating (match-engine.md, injuries.md).
6. `injury_check`'s comment says fatigue matters; the formula doesn't read it (injuries.md).
7. In high school states with a heat rule, a note says practice was cut, but the week's work isn't reduced (injuries.md, training.md).
8. AI college and high school teams have no practice injuries (injuries.md).
9. In the world, wear only rises from major injuries; only the class sim adds wear per season (development.md).
10. RB physical decline starts 2.5 years early; the 1.5 cap only limits late decline (development.md).
11. Coaching a habit through your training plan resets the old habit (`Tendencies::set` also sets `was`), so there is nothing to slip back to under stress (tendencies.md).
12. Kick orders set ReturnRisk numbers that nothing reads (tendencies.md).
13. Nothing calls `team_fit` or `fit_note`; the design shows fit notes on the boards (tendencies.md).
14. Every world game uses today's rule book, so NFL history games play the 2025 kickoff, the 15-yard extra point and the 10-minute overtime (play-calling.md).
15. College overtime never reaches the two-point shootout (play-calling.md).
16. The game plan's run-pass lean is saved but never read (play-calling.md).
17. AI coordinators never call designed plays; `legal` ignores the level (play-calling.md).
18. Fourth-down math and in-game EPA use the NFL expected-points table at every level (play-calling.md).
19. `call_defense` centres `belief` twice, and the blitz quick-throw term counts prep snaps against this game's throws (play-calling.md).
20. AI teams have no team rules (`folk::rules::ai_book` has no callers); rule-breaking only runs for your team (people.md).
21. Several pre-draft choices are only words: the testing camp and "QB throws for some clubs" flags, and all department instructions except "focus"; of the scouting asks only "evaluate in person" becomes a look (draft.md, scouting.md).
22. Nothing in the AI draft reads injury history (injuries.md).
23. Turf (×1.12 injury risk) is only set in NFL games in closed stadiums (injuries.md).
24. `combine_bench_mean` has 13 values for 14 positions (no long snapper), and the combine CSV has no LS row, so snappers get the default size 73 in / 230 lb (body.md).
