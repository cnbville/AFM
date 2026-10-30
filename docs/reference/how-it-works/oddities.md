# Code oddities found while writing the reference

Checked against main at 81c9f0c with update 3 lane 3 on 2026-10-28.

Places where the code does something its own comment, a design doc or the obvious intent says it shouldn't. Each is stated as what the code does. None is fixed: fixes wait for Cain's yes or no. Remove a line when it is fixed or decided.

2. Program `prestige` and `facilities` are stored 0-1, but `personnel` divides them by 100 and 20, so no FBS staff gets more than three personnel posts or a general manager (`world/scouting/readers.rs`, `personnel`). One public analyst's "big programs" lean checks `prestige >= 60.0` and never fires (`world/scouting/public.rs`).
3. Small-school players flagged by the stats footprint get a December read at accuracy 8 instead of 13; the comment says "truer read", but lower accuracy is a wider error (draft.md).
9. In the world, wear only rises from major injuries; only the class sim adds wear per season (development.md).
10. RB physical decline starts 2.5 years early; the 1.5 cap only limits late decline (development.md).
13. Nothing calls `team_fit` or `fit_note`; the design shows fit notes on the boards (tendencies.md).
16. The game plan's run-pass lean is saved but never read (play-calling.md).
21. Several pre-draft choices are only words: the testing camp and "QB throws for some clubs" flags, and all department instructions except "focus"; of the scouting asks only "evaluate in person" becomes a look (draft.md, scouting.md).
22. Nothing in the AI draft reads injury history (injuries.md).
24. `combine_bench_mean` has 13 values for 14 positions (no long snapper), and the combine CSV has no LS row, so snappers get the default size 73 in / 230 lb (body.md).
28. NFL and other-league game injuries are carried by `sides[inj.side]`, but `Injury::side` is the snap side (0 offense, 1 defense); the game team is the list's first element. A man hurt while his team had the other side of the ball is looked up on the wrong roster and never carries, about half of pro game injuries (`world/pro/games.rs`, `record`, `record_preseason`; `world/pro/other.rs`) (injuries.md). College and high school read the game team.
