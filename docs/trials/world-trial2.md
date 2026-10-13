# World trial 2 (section 2, after the trial 1 list)

Seed 2026, the full-size world, 30 years of hidden history (1996-2025), built in about 145 seconds. Full output: `world-trial2-output.txt`. Rerun: `cargo run --release --bin world_trial -- 2026 30`. Player pages: `AFM_CARDS=cards.json cargo run --release --bin world_trial -- 2026 30`.

This trial checks Raichel's running list from trial 1 (items 1-8). Numbers marked "hidden" are debug-only; the game never shows them. Everything Claude tuned without Raichel's answer is listed in `docs/world-model.md` as proposed, not confirmed.

## Calibration (still holds)

| Check | Sim | Real | Why |
|---|---|---|---|
| Five-stars drafted | 66% | 63% (confirmed target) | A little high, as in trial 1. |
| Four / three / two-stars drafted | 23% / 4.8% / 1.1% | 24% / 6.0% / 2.1% | Unchanged from trial 1; the engine's star tuning sets these. |
| Draft picks from FCS / D2 | 6.5% / 0.2% | about 6% / under 1% | The new pipelines and overflow didn't upset the FCS share. |

## 1. Prove-it extensions

- **118 extensions in 30 years; 46 coaches were fired after the prove-it year.** Why: only coaches with 5+ years, loyalty and real success qualify (a title in the last 8 years, or 70% wins over 50+ games). About 6 in 10 get the year, more with an excuse (players lost to the draft, a young lineup).
- **The fired share is 39%.** Why: the prove-it year now ends in a firing only after a losing record, or a season well below expectation without a strong record. In the first run of this trial an 8-4 coach was fired; that rule was fixed.
- **Example: Liberty, 1997.** Brian Nowell got a vote of confidence after two tough seasons; the news reported it.

## 2. Ethnicity and the coaching gap

| FBS head coaches of color | Sim | Real (TIDES / CBS) |
|---|---|---|
| 2009 | 3 Black of 138 (4 of color) | 7 Black of 120 |
| 2016 | 7 Black of 138 (9 of color) | 13 Black of 128 |
| 2025 | 17 Black of 138 (21 of color) | 16 Black of 134 (2024) |

- **The gap is right at the end and a little too wide in the 2000s.** Why: the hidden bias is flat until 2005, then eases. Fewer coaches of color reach head jobs early, so fewer have head-coach records to be hired on later. That path effect is real, but here it runs a little strong.
- **Coordinators of color 22%, position coaches 26%.** Why: assistant hiring carries the bias too, but the pool coming into coaching is diverse (mostly former players). The gap opens at the top, as TIDES describes.
- **FBS players: 46% Black, 41% white, 10% Hispanic, 1.9% Polynesian, 1% Asian.** Real FBS 2023: 48.9% Black. Why: each state's child population, how often each group plays, and a lean toward who reaches FBS. Ethnicity never changes a rating.
- **High school players: 58% white, 24% Black, 16% Hispanic.** Why: the state populations, before the FBS lean.

## 3. Names

- **Jalen: 1,044 players, 3 white. Kobe: 462, none white.** Raichel's point from trial 1 is fixed. Why: first names now come from real birth data by ethnicity and decade.
- **Trey: 407 players, 371 white.** This may be wrong. Only one source (mortgage records) has Trey, and it says mostly white. Raichel's call.
- **DeShawn: none.** Neither source has the name.
- **Polynesian names** (Viliami Tupou, Siaosi Aiono, Keoni Palelei) come from a hand list: no open source exists.
- **47% of active players share a full name with another.** Why: 779,000 players in the world. Real high school football has about a million, and common names repeat (there are many Michael Smiths).

## 4. Pipelines

- **418 of 494 programs have a high school pipeline of 0.10+.** Why: pipelines now track single high schools and areas, credited only for impact players (all-conference, awards, draft round, judged inside their own conference).
- **Bowling Green: Ohio 0.67, Michigan 0.35.** A MAC school leaning on its region, plus a St. Thomas Aquinas (FL) high school pipeline from its impact players there. Why: Florida has more FBS-level talent than its big programs can sign, so it spills over to nearby and Group of 5 schools.
- **Georgia's pipeline to IMG Academy is 0.99.** Why: years of impact players from there.
- **North Dakota State: a Puyallup (WA) pipeline of 0.41.** Why (inferred, not traced): most likely a staff tie, since a coach who coached impact players from a school carries the tie with him.

## 5. The coaching ladder

- **The most common moves:** high school head coach to D2 position coach (1,387), FCS to FBS position coach (1,215). Why: each step up goes to the level just above.
- **Coaches also step back down** (489 FBS position coaches to high school head jobs), as happens in real life when a staff is let go.
- **3,548 college hires came straight from high school.** Most go to position coach jobs at JUCO and D2. A high school coach with a dominant record and recent state titles gets bigger looks.
- **From high school to an FBS head job: Adrian Rocca.** High schools in California, Mississippi and Texas, then Lafayette (FCS), 98-43, then FIU. Why: winning at each level built his buzz.

## 6. Academics

- **On record (18,541 drafted or blue-chip players):** 517 academic redshirts, 261 special admissions, 59 sent to JUCO for grades, 199 lifted by summer school, 43 seasons lost to ineligibility.
- **Matthew Smith (IOL, 4-star), core GPA 2.11.** That's between the NCAA's 2.0 and 2.3, so he signed with Auburn as an academic redshirt and sat year one. He won national honors at 20 and went 4th overall.
- **Adam Smith (IOL), core GPA 3.18,** was one of Notre Dame's few special admissions: below its bar, but trending up.
- **Only 4 of 691 five-stars went through JUCO.** Why: most borderline five-stars get an academic redshirt or summer school instead.

## 7. The draft by position

| | Sim | Real 2000-2025 |
|---|---|---|
| Number 1 pick | QB 58%, EDGE 15%, OT 8%, WR 8% | QB 73%, EDGE 19%, OT 8% |
| Picks 1-5 | QB 34%, WR 14%, EDGE 13%, CB 12%, OT 8% | QB 28%, EDGE 21%, OT 15%, WR 11%, CB 7% |
| Round 1 | QB 13%, WR 13%, OT 12%, CB 11%, IOL 10%, EDGE 10% | EDGE 17%, WR 13%, OT 12%, CB 12%, DT 10%, QB 9% |

- **QBs now go first most years.** Why: teams reach for a quarterback (his premium starts lower down the board), and a team with a young franchise QB doesn't take another.
- **Still off:** EDGE a little low in round 1, IOL a little high. 26 number 1 picks is a small sample.
- **The last number 1 pick was an LB (Matthew Simmons, grade 93.2).** Why: the best player by a wide margin. It happens rarely in real life too.

## 8. Measurables and combine

- **Every position but the kickers is within 0.5" of real height, 4 lb of real weight and 0.02s on the 40.** All run slightly small. Kickers are 8 lb heavy. Why: bodies are drawn from the real combine per position, and combine results come from attributes plus body, fitted to real averages.
- **Off:** bench press for skill players runs about 3 reps high (WR 17.8 vs 14.7). The 3-cone runs about 0.1s fast for big men.
- **Player pages** now show height, weight, arm, hand, wingspan, all six tests, level ratings each season, the coaches he had, his offers and why he was drafted.

## Offers

- **Five-stars on the player pages have 13-43 offers, four-stars 9-48, the unranked hidden gems 3-15.** Why: staffs stop offering a kid who already holds a much better offer. In the first run of this trial one recruit had 199 offers.

## Flags for Raichel

1. **Trey reads as mostly white** in the data (see 3).
2. **The coaching gap is a bit too wide in the 2000s** (see 2).
3. **Dynasties are still strong:** Alabama has 9 titles in 30 years (real: 6). This was trial 1's flag 1 and wasn't on the list.
4. **Bench reps run high for skill players** (see 8).
5. **The stat-record part of coach buzz waits for section 3,** as does the hidden-gem stat footprint (item 9).
