# This family: the people you answer to

Your boss meets you about the season, and a panel interviews you for a job. The speaker is your boss, or a member of the panel.

- Your boss: at high school the principal, at college the athletic director, at the NFL the owner. Tag `principal`, `ad` or `owner` (with its level) when the words belong to one; an untagged line fits all three, so it can't name the school day, the conference office or the cap. `{boss}` is the boss's own first name, rarely useful.
- `boss.goals.open` opens the meeting where he sets your goals for the season, and `boss.goals.point` is an optional second line about one thing he expects. `boss.mid.open` opens the midseason check-in, `{record}` the record so far. `boss.review.good`, `mixed` and `bad` open the end-of-season review by how many goals you met (`{met}`, a number; `bad` has no `{met}`). A review never names last season's record.
- The base facts: `{team}`, `{town}`, `{year}` (the season), `{you}` and `{coach}` (you), `{old}` and `{old_last}` (the coach before you, when there was one), `{rival}`, `{rivalry}` (the rivalry game's name), `{series}` (the series record) and `{last_record}` (last season's record). A line with a fact is said only when the game has it.
- `interview.ask.*` is a panel member asking about your plan (win now, build it, develop), your staff (your own people, keep the good ones, the best available) or your past (owning a failure, pointing to results, the people). The panel: at high school the principal and the school's athletic director (tag `ad` with `hs`), at college the athletic director and the president, at the NFL the owner and the general manager (`gm`). `interview.react.*` is one of them reacting to your answer: good, flat or bad, as people on a hiring panel do (polite, guarded, sometimes cool).

## Voice

Principals care about the kids, the school day, grades, conduct and the community; athletic directors about the budget, compliance, donors, the conference and the brand; owners about winning, the city, the investment and the building. Cast tags are how they sound: ego, hot, warm, cold, blunt, smooth, impatient, patient, meddler (gets into football), tight (with money), old (60+), she. Boss talk is short and pointed, never a memo read aloud.
