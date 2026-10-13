# World trial 1 (section 2)

Seed 2026, the full-size world, 30 years of hidden history (1996-2025), built in about 100 seconds. Full output: `world-trial1-output.txt`. Rerun: `cargo run --release --bin world_trial -- 2026 30`.

Numbers marked "hidden" are debug-only; the game never shows them.

## Calibration

| Check | Sim | Real | Why |
|---|---|---|---|
| Five-stars drafted | 66% | 63% (confirmed target) | A little high. Five-stars land at the best programs, which develop and showcase them. |
| Four-stars drafted | 22% | 24% (ESPN classes 2013-2021, in the data) | Close. |
| Three-stars / two-stars | 4.7% / 1.1% | 6.0% / 2.1% (same source) | A little low. The engine's star tuning (section 1) sets these. |
| Draft picks from FCS | 6.2% | about 6% | The old gap (2-5%) is closed: FCS stars now get seen, and some move up through the portal. |
| Draft picks from D2 | 0.4% | under 1% | D2 players are rarely drafted, as decided. |
| FBS signees from TX / FL / GA | 12.6% / 7.9% / 7.3% | 14.2% / 10.6% / 9.1% | All three a little low. Ohio and Illinois run a little high. The state talent shares come from the data; the gap is in where FBS staffs look. |

## Programs

- **13 different FBS champions in 30 years.** Georgia won 9, Texas 5, Michigan 4. Why: Georgia's run came under two long-tenured head coaches with top recruiting, and titles feed prestige, which feeds recruiting. Real life: Alabama won 6 in the same 30 years. Georgia's 9 is above that, so dynasties are still a bit too strong (see the flags below).
- **Biggest riser: Bowling Green** (MAC), prestige 0.44 to 0.61. Why: a coach hired in 2018 went 56-28 with conference titles. His reputation (17.5) and the title-chances grade rose fast, but tradition (C) and facilities (D) barely moved, as the steady/fast split says they should.
- **Biggest faller: South Carolina** (SEC), 0.68 to 0.53. Why: ten head coaches in 30 years, nobody above .500 after 2007. The fast grades fell first (title chances F), the steady ones held (facilities B+).
- **Alabama fell** from 0.88 to 0.83 with 3 titles. Why: it fired coaches who went 45-21 and 36-12. A powerhouse's expectations are high, and recency bias punishes a few bad years.
- **Back-to-back conference titles** take a weak program's title-chances grade to A (Air Force D- to A, Bowling Green D+ to A). Why: the grade reads recent titles, record and roster strength, weighted by conference strength.
- **NIL warnings:** 150 programs got at least one. Temple got two (2000, 2014), each from a big booster leaving. The warning arrives three ways at once: an inbox message, a news story and a falling arrow on the NIL grade.

## Coaches

- **Top FBS coach: Jamal Roberts, Texas, 127-18, 5 titles.** Reputation 19.3, hidden head-coach value 17.9. Why: a long time as a defensive coordinator, then a top job. His recruiting (19.6) built the roster.
- **Most overrated: Kendrick Lewis, Eastern Michigan, reputation 16.3, hidden value 11.3.** Why: he was the offensive coordinator at Valdosta State during its title run, then won 2 FCS titles at New Hampshire. Recency bias and a "massive" title boost carried him past his true ability (Raichel's Cignetti rule working in reverse).
- **Most underrated: Hunter Price, Colorado State, reputation 5.9, hidden value 12.9.** Why: a first-time head coach, promoted from a position-coach career the game doesn't track yet. Nobody has seen him win.
- **Rise through the levels: Andre Johnson.** Arkansas high schools, then D2 titles at Sioux Falls, an FCS title at Southeast Missouri State, coordinator at Florida State, now Miami (FL) at 35-6 with 3 titles. 35 FCS title coaches moved up to FBS head jobs in 30 years.
- **Quick firings:** 53 FBS head coaches fired within two seasons (about 2 a year). Why: two bad seasons outweigh everything under recency bias, but first-year coaches are almost never fired.
- **High school coaches:** powerhouse coaches have a network of 14.7 and teaching of 12.5, small-school coaches 8.5 and 8.2. Why: powerhouses hire well, and 831 ex-pros coach at small schools with a strong network but average teaching.

## Players

- **Hidden gem: Micah Rogers**, an unranked EDGE from Los Angeles. He went to a California junior college, moved up to Tulane after a year, and went in round 2. Why: low visibility (a weak exposure network) hid him in high school. JUCO film got him a look. There are 33 such gems in 30 years.
- **JUCO to number 1 pick: Kendrick Lewis** (a different Kendrick Lewis; the name list is small). A five-star with a 2.6 GPA, he went to a Mississippi JUCO, moved up to Texas after one year, and was drafted first overall. Why: his grades kept him from signing out of high school, not his talent. That's the Cam Newton path.
- **Steered, then free: Austin Ward.** A family handler steered this five-star to a Texas JUCO. He moved up to Ohio State, then transferred to Georgia "free at last to choose for himself" and went 8th overall. Of 700 steered recruits, 251 transferred out once free.
- **Five-star busts:** Kendrick Henderson walked away from football as a freshman at Miami. Ivan Martin had a 1.8 GPA and never signed anywhere.
- **Powerhouse transfer: Noah Campbell** moved to King (TX) at 15, sat out part of the season under Texas's transfer rule, then went to Michigan and was picked 2nd.
- **Viral clip: Jalen Smith.** The clip got him a first look and a Michigan State offer, but his true potential was only 63 and he quit. A clip gets a look, not a career, as decided.

## What a scout sees

The same five-star, read by two scouts:
- Judging 6: "Raised by a single parent. Comfortable household. Has a mentor who keeps him on track. At 20, NIL money changed his family's life."
- Judging 16: adds "A parent is very active on social media about him", the new baby at 17, and gets the NIL year right (it started at 19).
- Truth: comfortable money (0.79), the parent is the decider and a social-media type. Why: a better scout sees more and makes fewer mistakes.

## Checker pass: flags for Raichel

1. **Dynasties are still a bit strong.** One program wins 6-11 FBS titles in 30 years, depending on the seed; the real-life maximum is about 6. Lower levels are worse: Valdosta State won 8 D2 titles (the real record is about 6 for Northwest Missouri State). Game results are a stand-in until the match engine (section 3), and part of this belongs there.
2. **The title-chances grade is generous** to Group of 5 and lower-league champions (Air Force to A).
3. **Odd background mix:** Kendrick Lewis "grew up in foster care" with money 0.86 and safety 1.00. That can happen with a good foster home, but it reads oddly.
4. **Repeated life event:** the scouted five-star has "NIL money changed his family's life" two years in a row.
5. **Ivan Martin's story doesn't say why** he never signed. JUCO had no grade floor for him, so his story needs a line saying why.
6. **Names repeat** (two Kendrick Lewises): the name list is 64 first names × 64 last names.

Items 3-6 are small. I'll fix them with Raichel's OK. Item 1 is a calibration question for section 3.
