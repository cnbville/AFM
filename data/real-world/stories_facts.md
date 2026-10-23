# Stories and culture facts (real-world data for the engine)

Research only, never decisions. Written 2026-10-22. Anything marked
"(from memory, to verify)" was not confirmed by a web source in this pass.
Web fetches of nfl.com, cbssports.com and wikipedia.org failed (DNS) in this
environment, so web checks came from search-result summaries only.

## Files

| File | What it holds |
|---|---|
| `storming_fines.csv` | field-storming fine policies by conference and era |
| `hard_knocks.csv` | every HBO Hard Knocks edition 2001-2025 |
| `college_docs.csv` | college, juco, high school and youth documentary series |
| `magazines.csv` | preseason annuals, college and NFL |
| `video_games.csv` | Madden, NFL 2K, NCAA Football, EA College Football by year |
| `intl_marketing_rights.csv` | NFL Global Markets Program, club by country |
| `celebration_rules.csv` | celebration and taunting rules, NFL, NCAA, NFHS |
| `retired_numbers_policy.csv` | number-retirement policy, 32 NFL clubs and 30 colleges |
| `one_day_contracts.csv` | one-day "retire with my team" contracts |
| `ring_sales.csv` | former players' title rings sold at auction |
| `parades.csv` | Super Bowl and college title parades with crowds |

## Hard Knocks: who can be forced

- 2013 rule: the league can force a club onto the training-camp edition
  unless it (a) has a first-year head coach, (b) made the playoffs in either
  of the last two seasons, or (c) appeared on Hard Knocks in the last 10
  years. Usually only 3-5 clubs are eligible a year.
  Source: https://awfulannouncing.com/2013/the-nfl-can-now-force-teams-to-do-hard-knocks.html ,
  https://www.realityblurred.com/realitytv/2013/10/hard-knocks-nfl-rule-few-teams/
- First real forcing: the Jets in 2023 (picked over three other eligible clubs).
  Source: https://awfulannouncing.com/hbo/nfl-media-hard-knocks-selection-process.html
- 2024 change (proposed by NFL Media, reported as adopted): the
  "appeared recently" window drops from 10 years to 8, the playoff exemption
  is removed, a club doing the in-season edition this season or next is
  exempt, and the new-coach exemption stays. The in-season edition then
  follows a whole division (AFC North 2024, NFC East 2025). An offseason
  edition (Giants) aired in 2024.
  Source: https://www.si.com/nfl/jaguars/how-new-hard-knocks-rules-impact-jacksonville
- Engine use: each spring pick one club for camp; volunteers first (new
  stadium, new star, owner who likes publicity); otherwise force one from
  the eligible list. Coaches hate it; players mostly shrug.

## Field storming (beyond the fines table)

- No clean per-season FBS count found. NCS4 (National Center for Spectator
  Sports Safety and Security) logged 90+ college field or court incursions
  (football and basketball together) from 2022 to late 2024.
  Source: https://www.athleticbusiness.com/operations/safety-security/article/15824740/the-dangers-of-court-and-field-storming-in-collegiate-athletics
- Pattern: storms follow a home upset of a ranked team or a rival.
  Same source.
- Base rate guess (from memory, to verify): about 15-30 FBS football field
  storms a season, rising since 2015; the SEC alone had 4-8 a year in
  2022-2024 (Tennessee-Alabama 2022, LSU-Alabama 2024, Ole Miss-Georgia
  2024, Vanderbilt-Alabama 2024, Oklahoma-Alabama 2024, Auburn-Texas A&M 2024).
  Source for the 2024 SEC fines: https://www.cbssports.com/college-football/news/sec-fines-oklahoma-auburn-after-fans-storm-field-in-upsets-over-alabama-texas-a-m/
- Injuries: 2023 a Toledo fan broke his neck; a Ball State fan was
  paralysed when a goalpost fell (Athletic Business, above). Goalposts
  are torn down after the biggest storms (Tennessee 2022 goalpost thrown
  into the Tennessee River; Vanderbilt 2024 goalpost carried out).
  (Goalpost details from memory, to verify.)
- The fine goes to the visiting school in the SEC. From 2025 the SEC fine
  is a flat $500,000 and can be waived if the visitors and officials are
  off the field first.

## College GameDay

- Began 1987 as a studio show. First road show 1993: Notre Dame vs Florida
  State at Notre Dame ("Game of the Century", Nov 13 1993).
  Source: https://app.cohorted.co.uk/news/college-gameday-history-impact-and (year verified; the game from memory, to verify)
- Lee Corso's headgear pick began 1996 (first: Brutus Buckeye head, Ohio
  State vs Penn State). (Year from search; the first head from memory, to verify.)
- Celebrity guest picker began 2008 (from memory, to verify; the search did
  not confirm it).
- Site choice: the biggest matchup with title stakes, rivalries, a good
  story and crowd; ESPN avoids repeating schools too often; it has gone to
  FCS, Division II and III sites for stories. "We go where the best story
  is" (Rece Davis).
  Source: https://awfulannouncing.com/espn/rece-davis-college-gameday-site-selection-process.html ,
  https://theanalyst.com/articles/college-gameday-fcs-football-games-espn-pregame-show-history
- Recruits: programs schedule official visits for GameDay weekends and
  coaches say it is the best recruiting weekend of the year (widely
  reported; no measured study found) (from memory, to verify).
- Corso retired after the 2025 season opener (Aug 30 2025, his 430th
  headgear pick) (from memory, to verify).

## Flex scheduling

| Window | From | Weeks | Limit | Notice | Source |
|---|---|---|---|---|---|
| Sunday Night | 2006 | 11-17 at first; 2014+ two flexes allowed in 5-10 | | ~12 days | (from memory, to verify) |
| Sunday Night | 2023 | up to three times in 5-14; any time in 15-17 | 3 in 5-14 | at least 12 days | https://www.nfl.com/news/flexible-scheduling-procedures-2023 |
| Monday Night | 2023 | 12-17 | league discretion | at least 12 days | same |
| Thursday Night | 2023 (trial) | 13-17 | at most 2 | at least 28 days | same |
| Week 18 | 2022 | all games | | 6 days; two Saturday games picked by the prior Monday | same |

- The 1pm and 4pm Sunday games can also be moved between windows.
- Short weeks: each club plays at most one Thursday game a year in the
  usual plan, and every Thursday game is played on 3 days' rest (from
  memory, to verify). Teams coming off a Monday game rarely play the next
  Thursday.

## International Series

- Since 2007 (Giants-Dolphins at Wembley). 2022 resolution with the 17-game
  season: every club plays abroad at least once every 8 years; the league
  can pick up to four clubs from the conference with nine home games that
  year to host a neutral-site game, so the "home" side gives up a home
  game. Clubs may still volunteer.
  Source: https://www.americanfootballinternational.com/nfl-expands-commitment-to-playing-international-games/
- Earlier: from 2014 a club had to play abroad to be eligible to host a
  Super Bowl (status after 2022 unclear).
  Source: https://www.cbssports.com/nfl/news/nfl-set-to-unveil-new-rotation-formula-for-international-games-once-17-game-schedule-is-implemented
- Jacksonville played a yearly London home game 2013-2023 (two in 2023).
  (from memory, to verify)
- Bye after a London trip: until about 2022 clubs nearly always got the bye
  the next week; since then it is offered and most take it (from memory,
  to verify).
- Body clock: studies on NFL time zones (e.g. Smith et al., Sleep 2013) find
  West Coast teams playing night games in the East gain an edge; for
  London games no clear win-rate effect is found; teams travel either
  Thursday night (arrive Friday) or early in the week (from memory, to verify).
- Sites so far: London (Wembley, Twickenham, Tottenham), Mexico City,
  Munich, Frankfurt, São Paulo (2024), Dublin (2025), Madrid (2025),
  Berlin (2025), Melbourne (2026) (from memory, to verify).

## International Player Pathway (IPP)

| Years | Rule | Source |
|---|---|---|
| 2017-2019 | Program starts; 4 players a year placed on one NFC South or AFC East division's clubs; an extra (11th) practice squad spot that cannot be promoted that season | (from memory, to verify) |
| 2020-2021 | Extra practice squad spot widened to more divisions; the 2020 practice squad grows to 16 | (from memory, to verify) |
| 2022-2023 | IPP player can be promoted; still one extra spot only for chosen divisions | (from memory, to verify) |
| 2024- | Every club may carry a 17th practice squad spot for an international player; that player can be elevated up to 3 times a season; one training camp roster exemption; eligible up to 3 seasons; at most 2 years of US high school; must have been draft-eligible | https://www.nfl.com/news/what-is-the-international-player-pathway |

## Jersey and likeness sales

- NFLPA "Top 50 player sales" list: published twice a year, a mid-year list
  (about late June, covering March-May) and a full-year list (about
  March-April, covering the prior league year) (from memory, to verify).
- College: from July 1 2021 (NCAA interim NIL policy) players can sell
  their names. Group licensing for jerseys came from OneTeam Partners and
  The Brandr Group (both 2021-22), with Fanatics, Homefield and others
  making the shirts; per-player jersey royalties are small (tens to
  hundreds of dollars a year for most, more for stars) (from memory, to verify).
- EA College Football 25 (July 2024) used about 11,000 opted-in players,
  each paid $600 plus a copy; College Football 26 raised it to $1,500 plus
  the Deluxe Edition.
  Source: https://thesideline.beehiiv.com/p/ea-sports-who-s-in-the-game ,
  https://www.si.com/college-football/ea-sports-double-nil-payments-appearing-college-football-video-game
- Madden ratings: released in "ratings week" before the August launch
  (late July to early August in recent years); the "99 Club" (players at
  99) is a yearly reveal from about Madden 17 (2016) (from memory, to verify).

## Fantasy football

- ESPN: 12 million players in 2023, 13 million in 2024 (both records).
  Source: https://espnpressroom.com/us/press-releases/2024/09/another-all-time-record-13-million-fans-playing-espn-fantasy-football-in-2024
- All US and Canada fantasy sports (FSGA): about 60-65 million players in
  recent years; football is the biggest share (from memory, to verify;
  the one aggregator found claims 81-86 million and is not reliable).
- Default scoring: standard (no point per catch) on ESPN and Yahoo until the
  late 2010s; ESPN switched its default to full PPR in 2019, Yahoo uses
  half-PPR by default (from memory, to verify).
- Waivers: most leagues process claims overnight Tuesday into Wednesday
  ("waiver Wednesday"); "waiver Tuesday" is when managers make claims
  (from memory, to verify).
- Engine use: a player's fantasy value is a public story (fans and media
  talk about "busts" and "sleepers"); it never changes how coaches play him.

## Fourth-down bots and public grading

- NYT 4th Down Bot: built by Brian Burke (Advanced NFL Stats) with The New
  York Times; live from late 2013, tweeted every 4th down; model based on
  expected points since 2000, switching to win probability late in games;
  upgraded 2015; retired about 2018 (end date from memory, to verify).
  Source: https://www.niemanlab.org/2013/12/yes-the-vikings-should-have-gone-for-it-on-4th-down-and-a-new-york-times-robot-knows-why/ ,
  https://www.niemanlab.org/2015/10/faster-stronger-and-with-better-kickers-the-new-york-times-4th-down-bot-gets-an-upgrade/
- EdjSports: sold a game-theory model to NFL clubs from about 2014; published
  the "Critical Call" and season coach rankings (from memory, to verify).
- Ben Baldwin's fourth-down bot (nflfastR data, 2020-): posts each close
  call during games on social media with the win-probability gain from
  going for it; at season's end people rank coaches by "go rate when the
  bot says go" (from memory, to verify).
- ESPN Analytics (Burke) shows a 4th-down decision graphic on broadcasts
  (from memory, to verify).
- How coaches are graded in public: after each game, media and fans
  quote the bot's call against the coach's call ("cost his team 3% win
  probability"). Since 2018 NFL coaches go for it far more often. The
  engine already has `nfl_fourth_go.csv` and `college_fourth_go.csv` for real
  go rates.

## Rings and gifts

- Super Bowl ring ceremony: held privately in late spring or early summer
  (May-June), at a team event (owner's home, a museum, a stadium club) with
  players, staff and families; staff receive smaller rings. The league pays
  up to about $7,000 a ring for up to 150 rings; the club pays the rest
  and for any extra rings (from memory, to verify).
- Ring size trend (from memory, to verify): Patriots XXXVIII (2004) about
  100 diamonds; Patriots LI (2017) 283 diamonds about 5 carats; Eagles LII
  (2018) 219 diamonds; Chiefs LIV (2020) 255 diamonds about 10.85 carats;
  Rams LVI (2022) about 20 carats, the biggest at the time; Chiefs LVIII
  (2024) 529 diamonds. Players' rings are worth about $30,000-50,000 each.
  Patriots LI diamonds count verified at
  https://sportscollectorsdigest.com/memorabilia/brady-family-super-bowl-ring
- Resale: rings reach the market through debts, divorce, estates and pawn.
  Auction records are about $230,000-345,000 (see `ring_sales.csv`).
- College title rings: schools buy them for players and staff; each
  conference champion and each bowl team also gives rings or gifts.
- NCAA gift limits (bowl or postseason), per player:
  bowl committee gifts up to $550 for up to 125 people per school (schools
  may buy more for others); plus up to $400 from the school and $400 from
  the conference for a bowl appearance; conference title awards about $375
  for taking part and $325 for winning.
  Source: https://www.sportsbusinessdaily.com/Journal/Issues/2016/12/05/Colleges/Bowl-gift-value.aspx ,
  https://www.wbur.org/onlyagame/2013/12/14/college-bowl-game-gift-bags
  (cap was $500 about 2009-2012 and $550 by 2013, from memory, to verify;
  the College Football Playoff gives its own gift package on top for
  semifinal and final teams.)

## Celebrations and fines (base rates)

- NFL excessive celebration and taunting flags: about 20-30 a season in
  2010-2016; fell after the 2017 loosening; taunting flags jumped in 2021
  (about 30-40 in that season after the point of emphasis), then eased
  (from memory, to verify).
- Famous fines (from memory, to verify): Chad Johnson's "Hall of Fame
  jacket" and Riverdance stunts (2005-2006, $10,000+ each); Terrell Owens'
  Sharpie (2002) and Cowboys star (2000, suspended a week); Joe Horn's cell
  phone (2003, $30,000); Randy Moss's fake moon (2005, $10,000); Jimmy
  Graham's goalpost dunk led to the 2014 ban; Odell Beckham Jr.'s "dog"
  celebration (2017, $12,154).
- First offense fines in the NFL schedule for unsportsmanlike conduct were
  about $10,000-15,000 in the 2020s (from memory, to verify).
- NCAA: since 2011 a live-ball taunt before the goal line costs the
  touchdown (15 yards from the spot); after the score it is a dead-ball
  foul enforced on the try or the kickoff. Two unsportsmanlike fouls in a
  game mean ejection. A dozen or so touchdowns a season have been taken off
  for live-ball taunting (from memory, to verify).
  Source: https://s3.amazonaws.com/ncaanewsarchive/2011/august/new-conduct-rule-ready-for-football-season.html
- Signature celebrations (from memory, to verify): Lambeau Leap (1993-),
  Ickey Shuffle (1988), Deion's high-step, the "Dirty Bird" (1998),
  Gronk spike, Cam Newton's "Superman" and dab, the Mile High Salute
  (Terrell Davis), the "Griddy" (2020s; from college, LSU's Justin
  Jefferson), Travis Kelce's group celebrations after 2017.

## Ring sales and parades: notes for the engine

- Former players sell rings rarely, perhaps a few a year that make news.
  Engine idea (proposed, not confirmed): a player who has lost his money
  (bankrupt, divorce) has a small chance a year to sell a title ring, a
  news story with a price from the auction table.
- Parades: NFL champions almost always hold a city parade within 3 days of
  the Super Bowl (exceptions: Packers' stadium rally 2011, Bucs' boat
  parade 2021). Crowd claims are usually city projections of about
  1 million; independent counts run lower (Eagles 2018: about 700,000 by
  crowd-safety researchers against "2 million" talk). College champions
  hold a stadium celebration on campus about 1-2 weeks after the title game,
  with 20,000-80,000 fans.
- The 2024 Chiefs rally ended with a shooting near Union Station: one woman
  killed, about 22 people shot or wounded (source in `parades.csv`).
