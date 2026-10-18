# Media, social media and fans around football

Research for section 6, topic "media, social media and fans". Cain asked for it on 2026-10-16 ("a full research dedeicted to creating realistic social media/news aspect") and agreed on 2026-10-18 that it runs in its own thread. Done 2026-10-18.

**Research only.** Nothing here is a decision. The section 6 design thread turns it into proposals for Cain. Where this file says what a game "needs", that is Claude's reading of real life, not a design pick.

Labels, as in the rest of `docs/research/`:
- **[published]**: a study, an official rule or a named report, linked in the source list at the end.
- **[known]**: a widely recorded public fact (a launch date, a rule everyone in the sport knows), not separately linked.
- **[inferred]**: Claude's reading or estimate. No published number backs it.

---

## 1. What AFM has today

Read from the code on 2026-10-18.

- **A plain news feed** (`world/news.rs`). A `NewsItem` has a date, a kind, a channel (inbox, public news or the NIL grade arrow), an optional program and one line of text. There are 11 kinds: NIL warning (3 levels), coach hired, fired, left, extended, the yearly diversity report, national title, NCAA penalty, conference title, state title and realignment. Record-book entries and honors also write lines. Trials export it as `raw/news.jsonl.gz`.
- **Loudness on the calendar** (`calendar/events.rs`, section 4 topic 8): small is a news line, medium an inbox message, big its own screen, huge a full show.
- **Things already built that the media side will touch:**
  - Coaches have a hidden ability and a public **reputation (1-20)**, "what schools and the media think of him". It follows results over about three seasons (`world/coach.rs`).
  - NFL **fan mood** from -1 to 1 (results, hype, star signings, renewals) feeds the owner and the contend-or-rebuild call (`pro/office.rs`, section 5 topic 7).
  - **Viral clips**: a kid can go viral (2% a year, 6% with a social-media parent). It gets him a first look only (`recruiting.rs`, section 2 topic 4).
  - **Parent style**: one parent style is "very active on social media about him". Scouts note it, and it shapes recruiting (`background.rs`).
  - Calendar items already dated: conference media days (A26), state media polls (A33), the polls, awards, signing days and the portal windows.
- **Missing:** no reporters, no outlets, no rumors or leaks, no social media accounts or followers, no press conferences, no fan voices, and no feedback from coverage into morale, recruiting, NIL or job security beyond the few links above.

---

## 2. The landscape changes by era

AFM runs 30 years of hidden history before 2026, and a career runs on into the future. The media world in 1996 is not the one in 2026. Dates are **[known]** unless marked.

| Year | What changed |
|---|---|
| 1979 | ESPN starts. Highlights become national. |
| Mid-1990s | Team message boards and fan sites. Recruiting is covered by newsletters and phone-line services. |
| 1998-2000 | Rivals.com starts (sources give 1998 or 2000) **[published]**. Premium team sites with message boards. Yahoo buys it in 2007. |
| 2001 | HBO *Hard Knocks* starts (NFL training camp documentary). |
| 2002 | MaxPreps starts (high school scores and computer rankings); CBS buys it in 2015 **[published, low-quality source]**. |
| 2004-2006 | Facebook (open to all from 2006). Twitter starts 2006. |
| 2008-2012 | Twitter becomes where news breaks. Coaches start banning players from it (section 4.6). |
| 2009 | NFL social media rule: players, coaches and staff stop 90 minutes before kickoff until after postgame interviews **[published]**. |
| 2010 | Instagram. 247Sports starts (sold to CBS 2015) **[published]**. |
| 2011 | Snapchat. |
| 2013 | ESPN and NFL Network agree not to tip draft picks before the podium **[published]**. NFL legal negotiating window before free agency starts (2 days). |
| 2014-2015 | Players' Tribune (athlete first-person media). |
| 2016 | NCAA "click, don't type": coaches may like and retweet recruits' posts from 1 August 2016, but not comment **[published]**. *The Athletic* starts (New York Times buys it in 2022). |
| 2018 | TikTok reaches the US. |
| 2021 | NIL opens on 1 July. On3 starts (also by Shannon Terry, who founded Rivals and 247) **[published]**. |
| 2023 | Twitter renamed X. Threads. *The Pat McAfee Show* licensed to ESPN (about $85M) **[published]**. |
| 2024 | SEC requires public availability (injury) reports before conference games, with fines **[published]**. *New Heights* (the Kelces) signs with Wondery (about $100M) **[published]**. NCAA study of online abuse **[published]**. |
| 2025 | On3 buys Rivals from Yahoo; two big recruiting databases remain (Rivals/On3 and 247) **[published]**. Revenue sharing in college (House settlement, see `rules-and-money-2026.md`). |

What this means for a world that runs 1996-2026 and beyond **[inferred]**:
- News speed rises by era: next morning's paper (1990s), same day online (2000s), minutes on X (2010s on).
- Who breaks news shifts: local beat writers, then national TV insiders, then a few national insiders on X, then also the players themselves.
- Recruits gain a public voice only from about 2008-2010. Before then, a recruit is a name in a newsletter.
- Player income from social media exists in college only from July 2021.

---

## 3. NFL media

### 3.1 Who reports

- **National insiders** (Adam Schefter at ESPN, Ian Rapoport at NFL Network, and a few more). Their sources are agents, executives, coaches and league staff. They break trades, signings, firings and injuries, and they corroborate with a second source before posting **[published, weak sources; the pattern itself is known]**. Opening day of free agency is their busiest day: Schefter reported dozens of signings on one 2015 day **[published]**.
- **Team beat writers**: local papers, *The Athletic*, local TV and radio. They are at practice and in the locker room, and they write the daily story.
- **Team-employed media**: the club's own site, social accounts and reporters. Friendly coverage, never the first with bad news **[inferred]**.
- **Local sports talk radio and TV.** Radio draws the most passionate fans: 82% of NFL fans who listen on AM/FM call themselves extremely or very passionate, against 67% of TV viewers **[published, broadcaster's own research]**.
- **National shows and ex-player analysts**, and **athlete-owned media** (podcasts, YouTube). Athlete-owned shows have more than 7 billion YouTube views together **[published, USC Annenberg]**; a 2025 count found 33 athlete-owned production companies making over 370 shows **[published, secondary]**.
- **Draft media**: mock drafts, big boards, and anonymous scout quotes. Bob McGinn has polled NFL personnel since the mid-1980s; scouts asked to stay anonymous from 2015 **[published]**. The quotes can be brutal ("mom is a problem", "selfish, diva wide receiver", "mentally frail") and some scouts shade them to move a player up or down a board **[published]**.

### 3.2 The league's rules on media

- **Media access policy** (league rule, updated each year) **[published]**:
  - After a game, the locker room opens 10-15 minutes after players enter it (earlier versions said 10-12).
  - The head coach, the quarterback and at least one other player with an impact on the game must come to the interview area.
  - Every player must be available after every game and regularly during the week. Players in heavy demand must also meet the beat media at least once in the week.
  - The 2014 policy opened locker rooms Monday, Wednesday, Thursday and Friday for at least 45 minutes (Sunday game). By 2024-2025 the policy says several clubs open before and after practice every day.
  - Head coach and coordinator times are set by each club (Baltimore: head coach Monday, Wednesday and Friday).
- **Fines for not talking**: Marshawn Lynch was fined $50,000 in 2014 plus a held-over $50,000 from 2013, $100,000 total **[published]**. Buffalo coach Marv Levy was fined $5,000 for missing 1991 Super Bowl media day **[published]**.
- **Injury reports** **[published]**: three practice reports in the week before a game. Every player with a reportable injury is listed as did not participate, limited or full, even if he takes every rep. "Limited" is less than 100% of his normal reps. Wrong reports bring league fines (the Steelers and Mike Tomlin, 2019, over Ben Roethlisberger). Game-status labels (out, doubtful, questionable) are **[known]**.
- **Social media in game windows**: off from 90 minutes before kickoff until after postgame interviews (2009 rule). Officials may never post **[published]**. Some coaches add their own rules (Sean Payton planned a signal scrambler in the Saints locker room) **[published]**.
- **The draft**: the networks agreed in 2013 not to tip picks early, banned their reporters from tipping round 1 from 2014, and NFL Network covered two nights from 2016. It held through at least 2019. Schefter kept the right to report "bold moves" **[published]**.
- **Minicamp and camp absences are public**: fines of $16,953, $33,908 and $50,855 for the three minicamp days (2024); $50,000 a day in training camp **[published]**. Holdouts are a media story in themselves **[inferred]**.

### 3.3 When NFL news happens

The NFL year is already in the calendar (`world/nfl_year.rs`). The news peaks are **[known]**:
- Black Monday (the day after the regular season: firings).
- The combine (interviews, medical rumors), the legal tampering window and the first days of free agency (most "agrees to terms" stories of the year), the draft.
- Contract holdouts in summer, the cutdown to 53, the trade deadline.
- Weekly: injury reports Wednesday to Friday, game-day inactives 90 minutes before kickoff, Monday press conferences.

---

## 4. College media

### 4.1 Who reports

- **Beat writers and the school's own content studio.** Big programs now run in-house studios of about 20 full-time staff (USC and Florida; 6 at Florida work only for football). Florida made over 18,000 posts in seven months of 2025. Nebraska's social lead runs 7 full-time and 50 part-time staff **[published]**. Their posts serve recruiting first: graphics, edits and hype videos **[published]**.
- **Recruiting services**: Rivals/On3 and 247Sports, each with a team site per school, a premium message board and paid insiders **[published]**. They rank recruits, report offers, visits and commitments, and publish **predictions**:
  - 247Sports Crystal Ball: top analysts are right 80-92% of the time (2018: 85.2% and 80.8% for two analysts; Steve Wiltfong 434 of 474, 91.6%) **[published]**. No audited system-wide figure exists.
  - On3's Recruiting Prediction Machine blends expert picks, visits, "social sentiment" and history; no published accuracy **[published]**.
- **Transfer portal reporters.** Entries usually break through the player's representatives to one national reporter (Pete Nakos at On3 is the common name), then spread on X **[published]**.
- **Hot-seat media.** Firings are season-long stories. In 2024, 15 FBS head coaches were fired by early December, with over $36M in buyouts. In 2025 nine were gone by 19 October, the first season since 1978 with seven or more jobs open before November, and buyouts reached about $168M with LSU's Brian Kelly (about $54M). The record before that was $118M in 2023 (Jimbo Fisher's $76.8M at Texas A&M) **[published]**.
- **Polls are media products.** The AP poll is voted by about 60 sportswriters and broadcasters; the coaches' poll by about 65 head coaches; the playoff committee is 13 people **[known]**. Studies find voters favour teams from their own state and conference, though less than in 2007 (Coleman et al. 2009; Hensley 2020) **[published]**. Preseason rank tends to persist into the postseason **[published, blog analysis]**.

### 4.2 Rules and habits

- **No conference-wide rule** makes a college head coach hold weekly press conferences. Schools set it (Wisconsin under Barry Alvarez: Mondays, plus a 10-minute Big Ten call-in slot) **[published]**. Writers' groups push norms: Steve Spurrier apologized in 2014 after a short postgame statement drew an FWAA complaint **[published]**.
- **The SEC injury report** from 2024: public availability reports before conference games, fines for late or wrong ones **[published]**. Big Ten and ACC followed with their own versions **[known]**.
- **Coaches banning players from Twitter**: Boise State (Petersen), Kansas, South Carolina (Spurrier, after a false tweet about a teammate's arrest), Florida State (Fisher), Texas Tech and Washington State (Leach, suspension for any post), North Carolina (Butch Davis), Towson, and Clemson in season (Swinney, 2018) **[published]**. Critics called it costly to players' self-marketing **[published]**. Since NIL (2021), a ban costs players money **[inferred]**.
- **Coaches and recruits**: since August 2016 coaches may like and retweet recruits' posts but not comment ("click, don't type"). Direct messages were already allowed within contact rules **[published]**.

### 4.3 NIL is a social media market

- On3's NIL valuation starts from **followers per platform** (Instagram, X, TikTok, each weighted by what brands spend there), adjusts for **engagement**, adds **performance and exposure** (national TV games, honors) and **deals and collectives**. It moves week to week **[published]**. One 2022 example: 16,900 followers gave about $293 per post **[published]**.
- Opendorse (its own marketplace only): 86.1% of deals came from posting content, but posts were only 28.6% of the money **[published, undated]**. In 2022 the median social media deal was $50 and the mean $905 (INFLCR) **[published]**. So: many tiny posting deals, and the big money elsewhere **[inferred]**.
- Collectives paid about 81.6% of all NIL money; football took 72.2% of collective dollars and 76.6% of commercial dollars **[published, Opendorse via Front Office Sports]**.
- **Followers are not all real.** One 2025 analysis claimed most of a star NFL back's Instagram followers were fake **[published, single outlet]**.
- **Big moments move followers.** Draft-night gains: Ashton Jeanty +35,000 Instagram followers on one day (2025); Caleb Downs +62,634, the most in round 1 of 2026 **[published]**.

---

## 5. High school media

- **Local papers are shrinking fast, and prep sports are cut first.** The LA Times cut about two-thirds of its prep reporters; the St. Louis Post-Dispatch cut 4 of 7 in its high school department; the Chicago Sun-Times scaled back prep coverage in 2015; small papers drop sports when a writer retires **[published]**. No national headcount exists.
- **What filled the gap**: MaxPreps (scores, stats, computer rankings: it polls no one), Hudl (about 29,000 of the 37,000 high school programs use it), team and player accounts, and local TV highlight shows **[published / known]**. State media polls are already in the calendar (A33).
- **Recruits are their own media outlets** from about 2010 **[known]**:
  - Offers are posted on X ("Blessed to receive an offer from ..."); highlight reels are shared there **[known; no study found]**.
  - College coaches do find players through reels, and the first plays on a reel decide whether a coach keeps watching **[published, Hudl]**.
  - Commitment announcements: hat ceremonies on live TV (ESPN, signing day specials, the All-American Bowl), then pre-produced videos and stunts (a live puppy, fake-outs). Some stunts backfire and bring apologies **[published]**.
- **What recruits' social media reveals.** A University of Iowa study of recruits with two or more offers and public X accounts **[published]**:
  - Posting a school's hashtag made a recruit about 3 times as likely to sign there.
  - Each coach on a staff he followed made him 47% more likely to sign there.
  - The model, which also used distance, academics and team success, picked the right school 70% of the time.
  - This is why On3's prediction machine uses "social sentiment" **[inferred]**.
- **What recruits say.** In ESPN's 2014 survey of top-300 recruits (150+ answered), 81% said fan messages don't sway them, 83% wanted coaches allowed to message without limit, and 62% had been harassed by a fan **[published]**.
- **Decommitments are news.** About 15-17% of top-100 recruits (2007-2011 classes) decommitted at least once; 12.4% signed elsewhere **[published, SI]**. In the 2023 cycle, 12 of the top 40 decommitted or flipped at least once, 8 of them five-stars **[published]**. 247 listed 50 decommitments in the 2025 class (3 five-stars, 4 four-stars, 41 three-stars) **[published]**.

---

## 6. How a story spreads

The usual chain **[known; no data on timings]**:

1. **Source.** Someone with a reason to talk: an agent (leverage in a negotiation), a team (a smokescreen, or softening fans up for a cut), a player's camp, a parent, a booster, a rival staff, a scout.
2. **First report.** A national insider for big NFL news; a beat writer for team news; a recruiting insider or the recruit himself for recruiting; the player's representatives for portal entries.
3. **Aggregation.** Other outlets repeat it "per [reporter]" within minutes (the RotoWire and SI pattern in the sources is exactly this).
4. **Reaction.** Fans, ex-players and talk shows. Volume can be huge: over 40,000 tweets after Tennessee fired Jeremy Pruitt, with opinion split down the middle **[published]**.
5. **Response or correction.** The team confirms, denies or says nothing. Insiders occasionally correct **[published]**.

What decides how far a story goes **[inferred, consistent with the sources]**:
- **The name.** Media attention follows fame more than performance: 2015 coverage of 53 NFL quarterbacks gave "name brand" passers far more words per minute played than their passer rating earned **[published, MIT Civic Media]**.
- **The market and the fan base.** The most intense fan bases are not the biggest markets: Nielsen's NFL Fandom Index ranks Buffalo, Kansas City and Green Bay top **[published]**.
- **The moment.** A Tuesday story in March competes with free agency; a Monday story in a bye week does not.
- **Rumor versus report.** "Is expected to", "per sources", "is exploring" are steps on a ladder that the real reader learns to weigh.

---

## 7. Fans

- **Where fans follow**: live TV 69%, social media 37% (second), per YouGov **[published]**. Social media is now the main source of highlights, clips and news outside live games **[published, Hub Entertainment 2024]**. Under 30, 43% regularly get news on TikTok (Pew, all news) **[published]**.
- **Fan anger online is real and measurable:**
  - NCAA pilot study (2024): 72,000+ flagged messages, 5,000+ confirmed abusive or threatening, aimed at 3,164 athletes, 489 coaches and 197 officials. About 18% sexual, 12% betting-related, 10% racist, 9% homophobic or transphobic, 6% violent. FBS football was among the most violent. One athlete got 1,400+ abusive messages in two weeks **[published]**.
  - NFL: in The Athletic's anonymous 2025 player survey (75-84 players), 70% had been messaged by fans about betting or fantasy losses, mostly on Instagram and X, peaking at the fantasy playoffs (week 14). Most never reply **[published]**.
- **Fans move job security.** "Forced crowdsourcing": social media lets fans push a school to act fast. Rutgers went from suspending basketball coach Mike Rice to firing him in about a day **[published]**. Fan reaction to a coach leaving runs through rallying, blaming, insulting and threats (Cincinnati after Brian Kelly left, 717 posts) **[published]**.
- **Fans move money.**
  - Attendance rises with this season's wins and the last one or two seasons' wins, bowl and playoff history, and rivalries. Last season matters most early in a season and fades as this season goes on. Habits persist **[published, FBS, D2 and D3 studies]**. Teams coming off winning seasons filled about 88% of capacity against an 80% average (2008-2010) **[published, descriptive]**.
  - Donations: a surprise win raises alumni athletic giving by about $134,000; a five-win jump in a year (8% of FBS teams do it) raises athletic donations about 28% ($682,000), applications 5% and SAT scores slightly **[published, Anderson, NBER]**. At one selective school, a conference title raised men's giving about 7%, while plain win-loss records barely mattered **[published, Meer and Rosen]**.

---

## 8. Feedback into the rest of the game

What is measured, and what is only inferred.

| Link | Evidence |
|---|---|
| Social media → player performance | **[published]** NBA players who tweeted late at night scored and rebounded less next day and shot 1.7 points worse (112 players, 2009-2016). It is a proxy for lost sleep, not proof of cause. No football study found. |
| Online abuse → mental health | **[published]** Among 85 D1 athletes, online harassment went with depression (not anxiety). Few reported it (9.4% via posts). The NCAA calls the risk real but has not measured it. |
| Coverage → morale | **[inferred]** No measurement found. Real signs: "bulletin board material", players saying they don't read it, coaches like Saban calling praise "rat poison" (**[known]**), and the bans in 4.2. |
| Social media → recruiting | **[published]** Recruits' posts and follows predict their choice (Iowa study). Recruits say fan messages don't sway them (ESPN 2014). Schools spend heavily on recruiting content (4.1). |
| Social media → NIL | **[published]** Followers and engagement are the base of NIL valuations; performance and TV exposure add to it (On3). Most deals are posts, most money is collectives (Opendorse). |
| Media and fans → job security | **[published]** Case studies only (Rutgers, Tennessee). No large study links sentiment to firings. Money (donations, attendance) does follow wins (section 7). |
| Media → reputation | **[published]** Attention follows fame more than performance (QB study). Poll voters favour their own state and conference. AFM's coach reputation already lags results by about three seasons. |
| Media → draft stock | **[published]** Anonymous scout quotes and character leaks are public and sometimes self-serving; no study measures their effect on where a player goes. |
| Leaks → negotiations | **[inferred]** Agents leak to insiders to create a market; teams leak to lower a price or prepare fans. No data. |

---

## 9. Gaps nobody publishes

The design will have to model these without data **[inferred list]**:
- How fast a story spreads by era, level and size (minutes to first aggregation, share that goes national).
- How often reports are wrong, and how often insiders correct.
- Follower counts for typical players at each level (only the very top and draft-night gains are published).
- How much coverage changes a player's morale or form in football.
- The share of firings that public pressure brought forward.
- How many beat reporters cover a typical NFL team, FBS team or high school (only cuts are reported).
- Each outlet's audience and reach.

---

## 10. Running on an 8 GB M1

Estimates for the design thread, not measurements **[inferred]**:
- **Store events, not text.** An event (who, what, when, source, how sure, which outlet) fits in about 32-64 bytes. Lines of text are rendered from templates only when a screen asks for them, so the world never holds millions of strings.
- **Rough volume.** 32 NFL teams, about 680 college programs and the high schools near the player's job, each with a few items a day, plus recruits' posts. That is about 2-5 million events a year. At 48 bytes, 100-250 MB a year if all were kept, so old small items must be summarized or dropped (the calendar already keeps "only the loud ones" for past years).
- **Fan voices** (posts, callers, comments) do not need to exist as stored objects for the whole world. They can be drawn when a screen opens, from a seed and the facts (result, mood, the name involved), the same way player streams already work.
- **The 30-year history** needs only the headline events (titles, firings, big trades, records), which already exist.
- These sizes should be timed and measured in a proof before anything is promised.

---

## Sources

NFL rules and media:
- Marshawn Lynch $100,000 fine: https://nbcsports.com/nfl/profootballtalk/rumor-mill/news/league-fines-marshawn-lynch-100000 and https://abc7news.com/post/silence-costs-marshawn-lynch-$100k/402569/
- NFL media access policies (2014-2026 versions): https://www.profootballwriters.org/nfl-media-access-policy/ and https://www.profootballwriters.org/2014/05/23/2014-nfl-media-relations-policy/
- Baltimore club media policy: https://static.clubs.nfl.com/image/upload/ravens/di42quh468m4ctmclhh6
- NFL 2009 social media rule: https://www.prdaily.com/have-you-seen-the-details-of-the-nfls-social-media-ban/ and https://www.si.com/more-sports/2009/09/04/twitter ; Payton's scrambler: https://www.complex.com/sports/a/aaron-mansfield/nfl-coach-scrambler-block-social-media-locker-room
- Injury reports: https://www.profootballnetwork.com/?p=456093 ; minicamp fines: https://www.cbssports.com/nfl/news/aaron-rodgers-skipping-rest-of-jets-mandatory-minicamp-this-week-per-report-absence-unexcused-per-hc/ and https://www.wxow.com/sports/aaron-rodgers-didn-t-show-for-mandatory-minicamp-risks-93-000-fine/article_bf8a7b33-e268-5ad3-9a39-37ff1995d032.html
- Draft no-tipping: https://www.nfl.com/news/networks-agree-not-to-tip-picks-during-2013-nfl-draft-0ap1000000162244 and https://awfulannouncing.com/nfl/espn-nfl-network-agree-not-tip-draft-picks-year.html
- Insiders on free agency day: https://awfulannouncing.com/?p=36718
- Anonymous scouts: https://www.acmepackingcompany.com/2025/3/24/24393201/what-are-we-really-learning-from-bob-mcginns-anonymous-scouting-report and https://www.phillyvoice.com/carson-wentz-receives-rare-effusive-praise-anonymous-scouts/
- Athlete-owned media: https://annenberg.usc.edu/node/75881 , https://www.semafor.com/article/08/08/2024/cam-heyward-is-leaving-espn-to-launch-a-podcast-with-vox-media , https://digiday.com/publishers/players-tribune-gets-podcasts/
- Radio audience: https://www.westwoodone.com/?p=22609 and https://audacyinc.com/insights/the-most-avid-engaged-nfl-fans-theyre-on-sports-audio/
- Nielsen NFL Fandom Index: https://www.fantasynerds.com/news/story/2026/08/18/nielsen-study-buffalo-bills-boast-americas-most-passionate-nfl-fan-base-1599690
- QB coverage study: https://civic.mit.edu/index.html%3Fp=1614.html
- Betting harassment of NFL players: https://www.gamblinginsider.com/news/107864/sports-bettors-harassment-nfl-players-venmo-requests-death-threats-survey and https://casinobeats.com/2026/02/02/nfl-survey-shows-bettors-regularly-target-players-over-lost-bets/
- Follower figures: https://www.rotowire.com/football/article/2026-nfl-draft-social-media-growth-112264 , https://googlenews.rotowire.com/football/article/most-social-media-growth-of-2025-nfl-draft-prospects-92303 , https://www.covers.com/nfl/top-20-fake-instagram-follower-list

College:
- NCAA "click, don't type" (2016): https://www.elevenwarriors.com/college-football/2016/08/72626/thanks-to-new-ncaa-rules-big-ten-coaches-are-allowed-to-retweet-recruits-most-havent and https://saturdayblitz.com/2016/08/01/college-football-recruiting-new-ncaa-social-media-rules-take-effect/
- Recruiting services history: https://en.wikipedia.org/wiki/On3.com , https://awfulannouncing.com/college-football/rivals-on3-247-sports-recruiting-service-feud.html , https://awfulannouncing.com/college-football/on3-purchases-rivals-forges-partnership-with-yahoo.html
- Crystal Ball accuracy: https://www.elevenwarriors.com/11w/2018/02/90951/eleven-warriors-andrew-lind-andrew-ellis-finish-fourth-sixth-in-2018-247sports-crystal-ball-rankings ; On3 RPM: https://www.on3.com/news/on3-releases-recruiting-prediction-machine-rpm
- Portal reporting: https://rotowire.com/cfootball/headlines/decoldest-crawford-injury-enters-transfer-portal-207913 , https://awfulannouncing.com/college-football/on3-contract-reporter-nico-iamaleava-father-pete-nakos.html
- Content studios: https://frontofficesports.com/article/college-athletic-department-social-media-content-studios/ and https://tallo.com/talent/job/design-and-media/media-or-talent-director/ne/lincoln/assistant-ad-social-strategy-a1346c25
- Coach media norms: https://www.si.com/college/2014/11/10/south-carolina-steve-spurrier-apologizes , https://the5thdown.com/2018/08/15/fwaa-issues-reminder-of-media-access-standards-as-season-approaches/ ; SEC availability reports: https://www.theadvocate.com/baton_rouge/sports/lsu/sec-player-availability-injury-reports-lsu/article_cfe0adc2-53ba-5e69-aa3d-00f3fedee914.html
- Twitter bans: https://www.nbcsports.com/college-football/news/chris-petersen-to-boise-players-no-tweets-for-you , https://bleacherreport.com/articles/1723829-dabo-swinney-makes-right-call-in-banning-clemson-players-from-twitter , https://news.wfsu.org/post/fsus-fisher-bans-football-team-twitter , https://www.knkx.org/post/wsu-coach-leach-bans-players-twitter , https://www.cnbc.com/2011/08/08/coaches-ban-of-twitter-proves-college-sports-isnt-about-education.html
- Firings and buyouts: https://6abc.com/post/168m-buyouts-owed-fired-fbs-head-football-coaches/18080986/ , https://frontofficesports.com/article/college-football-coach-firings-nearing-100m-in-contract-buyouts/ , https://frontofficesports.com/newsletter/cfb-coach-buyouts-hit-116m/
- Poll bias: https://econ.berkeley.edu/sites/default/files/Brent%20Hensley%20Honors%20Thesis.pdf , https://works.bepress.com/jonathan_santo/30 , https://blog.minitab.com/en/blog/the-statistics-game/the-influence-of-the-ap-preseason-college-football-poll
- NIL valuation and deals: https://www.on3.com/nil/news/how-social-impacts-your-nil-valuation , https://boardroom.tv/opendorse-nil-ncaa-compensation-data/ , https://www.nbcdfw.com/news/sports/one-year-of-nil-how-much-have-athletes-made/3008831/ , https://frontofficesports.com/as-nil-turns-three-collectives-and-football-still-control-the-industry/

High school and recruiting:
- Prep coverage cuts: https://www.riverfronttimes.com/news/post-dispatch-lays-off-more-than-half-its-prep-sports-staff-39381510 , https://robertfeder.dailyherald.com/2015/05/03/sun-times-trims-prep-sports-coverage/ , https://www.thewrap.com/media-platforms/journalism/sports-journalism-must-evolve-to-survive-washington-post/ , https://lbpost.com/sports/?p=5722
- MaxPreps: https://www.cbssports.com/high-school/football/news/high-school-football-rankings-mater-dei-finishes-no-1-in-final-maxpreps-top-25-of-2024
- Hudl: https://www.inquirer.com/college-sports/hudl-national-signing-day-recruiting-college-football-high-school-public-league-highlights-scouting-20181229.html , https://hudl.com/blog/join-the-club-high-school-highlights-arent-the-only-way-to-get-recruited
- Iowa social media and commitments study: https://www.informs.org/News-Room/INFORMS-Releases/News-Releases/University-of-Iowa-researchers-develop-new-model-to-predict-which-universities-student-athletes-will-commit-to and https://www.saturdaydownsouth.com/?p=195888
- ESPN 2014 recruit survey: https://abc7chicago.com/post/recruits-support-athletes-stipends/204726/
- Decommitments: https://www.si.com/more-sports/2012/01/20/commitment-project , https://www.on3.com/news/decommitments-that-shaped-the-2023-recruiting-cycle , https://www.yardbarker.com/college_football/articles/de_commitments_of_football_recruits_a_recurring_phenomenon/s1_17394_40509724
- Commitment ceremonies: https://sportsday.dallasnews.com/high-school/high-schools/2017/01/29/signing-day-pick-hat-tradition-started-dallas-recruits-kept-pushing-envelope , https://africa.espn.com/college-football/story/_/id/28291590/from-bulldogs-snoop-dogg-best-props-college-football-national-signing-week-history , https://awfulannouncing.com/2015/iman-marshalls-bleacher-report-video-takes-signing-day-whole-new-level.html

Fans, money and wellbeing:
- YouGov: https://yougov.com/articles/46648-how-do-american-sports-fans-engage-with-sports-on-social-media ; Hub 2024: https://www.newscaststudio.com/2025/03/06/cable-broadcast-and-streaming-now-neck-and-neck-for-live-sports
- NCAA online abuse study: https://abcnews.go.com/Sports/wireStory/ncaa-pilot-study-finds-widespread-social-media-harassment-114703594 , https://www.athleticbusiness.com/operations/governing-bodies/article/15705423/ncaa-study-college-athletes-suffering-abuse-from-angry-bettors
- Harassment and depression (MAC athletes): https://scholarcommons.sc.edu/jiia/vol19/iss1/14
- Late-night tweeting: https://researchconnect.stonybrook.edu/en/publications/association-between-late-night-tweeting-and-next-day-game-perform/
- Forced crowdsourcing: https://news.ku.edu/news/article/2014/05/07/social-media-forced-crowdsourcing-have-given-fans-unprecedented-power-decisions-fire ; Pruitt tweets: https://wbir.com/article/sports/ut-social-media-research-center-tracks-online-conversation-of-coach-pruitt-firing/51-c0cb04c9-f8b4-40ec-ae5a-45b82578267c ; Cincinnati fans: https://researchgate.net/profile/Jimmy_Sanderson/publication/271569551_From_Loving_the_Hero_to_Despising_the_Villain_Sports_Fans_Facebook_and_Social_Identity_Threats/links/5c2d3dbe92851c22a3563128/From-Loving-the-Hero-to-Despising-the-Villain-Sports-Fans-Facebook-and-Social-Identity-Threats.pdf
- Attendance: https://www.scinapse.io/papers/2130300257 , https://ideas.repec.org/a/kap/atlecj/v53y2025i3d10.1007_s11293-025-09830-6.html , https://harvardsportsanalysis.org/2011/01/what-factors-contribute-to-attendance-in-college-football
- Donations: https://www.nber.org/digest/nov12/w18196.html , https://swh.princeton.edu/~ceps/workingpapers/162rosen.pdf

Caveats on sources: several figures come from news coverage of a study rather than the study itself, a few from low-quality pages (marked where used), and the NFL media access policy PDFs could not be opened from this container, so their content here comes from search summaries of the 2014-2025 versions.
