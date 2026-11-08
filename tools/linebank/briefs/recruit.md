# This family: recruiting

You recruit high school players to your college, and as an NFL coach you interview prospects at the combine. The speakers are a recruit's family in their living room, a college coach on the phone, and a draft prospect.

- `homevisit.*` is your home visit to a high school recruit, `{name}` (his first name). The speaker is whoever decides in his family: tag `himself` (the recruit speaks for himself, first person), `parent`, `grandparent`, or `handler` (a trainer or 7-on-7 coach who steers him); an untagged line fits all of them, so it speaks as "we" or about "him" without saying who the speaker is. `homevisit.ask.<pitch>` is their first question, about what they care about most: playing time, NIL money, the school, staying close to home, family, getting him to the next level, winning. `homevisit.tell` answers "tell me what matters most" with `{first}` then `{second}`, two of those in lower case ("playing time", "nil money", "the school", "close to home", "family", "getting him to the next level", "winning"), so the line must read with any of them. `homevisit.react.*` is how your pitch landed: good, flat or bad.
- A parent-tagged line may say "my son" or "my boy", a grandparent-tagged one "my grandson"; no other family member by role, and never a spouse.
- `hscall.ask.*` is a college coach phoning you, the high school coach, about your player `{name}`: his football, his grades, his character. The speaker is a college assistant or head coach, busy, friendly, fishing for the truth.
- `combine.press.*` is a draft prospect in an NFL team's interview room at the combine, after you press him hard (a bad game on tape, a drop in his last season, an off-field question, why he left a school): steady (he stays even and owns it), bristles (he gets defensive and edgy), shaken (he loses his footing). He is a college player about to be drafted, talking to NFL coaches.

## Voice

Families are real: careful with money talk, protective, proud, sometimes overwhelmed by attention; some have done this before with an older child (never say so). Handlers talk business. A recruit speaking for himself is a 17-year-old: plain, short, a little guarded. College coaches on the phone are quick and casual. A combine prospect is 21 or 22, rehearsed or raw. Never a real school, coach or player's name; never a ranking or star count.
