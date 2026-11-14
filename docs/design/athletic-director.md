# The athletic director layer

Designed 2026-11-14 in the thread "Design the athletic director layer" (Cain's ask, 20:05 UTC: model the real conflict between an athletic director and his head coach, the money walls between the school's capital and the boosters', the AD's verbs, and the failure states and feedback loops, with no god-mode sliders and no chore lists). Design only, no code. Real life is the default. Every size here is Claude's, **proposed, not confirmed**, unless a real source gives it. The five questions real life can't settle are at the end.

Sources for the real facts are named in each topic. Facts marked "from memory" were not checked against a primary source this session.

---

## What it does in the game

- **Your AD is a person with his own problem.** He runs 20-odd sports on one budget, owes the bank for the stadium, answers to a president and a board, and courts the same donors you do. When you ask for $1.5 million more for your staff, he says yes or no from his real books, and a yes names where it came from: "We'll take it out of reserves," or "Rowing's travel gets cut," or "Coleman will cover it, but he wants a word with you about the quarterback."
- **Boosters split their money, and you compete for it.** A big donor gives a set amount a year. If the AD opens a $180 million stadium campaign with naming rights, that donor's money goes to the stadium, and your collective has less for the portal in December. You can fight for it (call him yourself), and the AD will know.
- **Losing costs twice when there's debt.** A school that borrowed for a new football building pays the same debt service whether it goes 10-2 or 3-9. Three bad years in a row means seat donations fall, the debt stays, and your staff pool is what gets squeezed. Your recruiting money dries up just when you need it.
- **Firing a coach is a money question.** A $50 million buyout needs donors to cover it (LSU, October 2025), or reserves, or a university loan. An AD who can't raise it keeps a coach he would otherwise fire. A booster group that wants a coach gone can offer to pay, and that pressure is part of your hot seat.
- **The president and the state can step in.** A public school's board approves big contracts and debt. When a firing goes badly, the governor can take the next hire away from the AD (Louisiana, 2025).
- **Title IX ties football's growth to everyone else.** An AD who wants to grow football's revenue share and cut a men's sport to pay for it can do that. Cutting a women's sport invites a lawsuit and a reversal (Iowa, 2020-21).
- **Realignment is a cliff or a ladder.** A school left behind when its conference breaks up loses most of its media money in a year. A school invited up pays an exit fee, takes a part share for its first years, and still owes revenue share at the new level's rate.
- **The world's other 130-odd FBS departments run the same way,** so other programs rise on a rich donor's money, overbuild, go into debt drag, cut sports, and fire coaches they can't afford.

None of it shows as a meter (section 5's rule, people topic 14). You hear it in his words, in what he grants, in the news, and in the public numbers each winter.

---

## What exists

- **Built:**
  - The AD as a person: name, judgement, patience, since when (`world/program.rs`, `AthleticDirector`). He reviews his head coach against the roster he was given, with a honeymoon, prove-it years, extensions and a buyout factor, clamp(1.2 − buyout / (0.2 × athletic budget + boosters), 0.15, 1) (`world/staff.rs`, `college_review`). He leaves at 6% a year, up to 18% after a four-year slide, +4% after ten years (`ad_turnover`).
  - A full athletic department book with 24 lines, July to June (`world/finance/college.rs`): conference money, postseason, tickets, seat donations, gifts, NCAA, licensing, guarantees, other sports, student fees, university support; coaches, buyouts, staff, recruiting, scholarships, revenue share, House back damages, debt, travel, guarantees out, bowl trips, other costs, admin. Reserves carry year to year; the university fills the gap up to the floor its other sports carry (Title IX's women's sports and the NCAA's minimums).
  - Facility projects decided each July, two or three years out, with pledged and borrowed parts; debt is 7.5% a year of the borrowed part for 22 years (`open_projects`, `open_year`). Real projects before 2026 (`real_groundbreaks`).
  - Media deals by conference, and realignment by media money after 2026, with exit fees and newcomers' part shares (`realign`, `open_year`).
  - The collective: a wider booster base and one big booster who can walk away, drifting toward a target from fans and results (`program.rs`, `update_collective`, `nil_target`). Two pots from July 2025: the school's revenue share and the collective's money, with NIL Go reviewing the collective's deals (`ncaa.rs`, `two_pots`, `nil_go_reviews`).
  - Every NIL dollar is yours (Cain, 2026-11-07 and 11-08): you set each player's revenue share and collective deal; the game never sets one for you.
  - Your boss's trust, goals and season meetings; six asks (staff money, a facility, recruiting money, NIL money, roster spots, an extension) (`world/folk/boss.rs`, `ask`). Big donors' strings on your collective (L9, `world/legacy/college.rs`, `string_letter`). Going around your boss costs trust (people topic 12).
  - Your AD's recruiting money (missed feature 5, PR #481): a yes adds a tenth to the recruiting line, paid out of what the other sports would get (`raise_recruiting`).
- **Decided:** every boss is a person with topic 1's traits, the president and board included; goals are negotiated; bosses meddle; bosses change (people topic 12). The hot seat is public, firings follow the real calendar, the fired coach is paid (people topic 14). Full club finances, and what each seat sees: a college head coach sees football's lines, attendance and donations, and the department's public numbers (later extras topic 1). Prices are set by the business side, never by you.
- **What this design changes in the built code:**
  - `boss.rs`, `ask`: today a dice roll (0.15 + 0.6 × (trust − 0.4) + 0.2 × money + ...) with flat outcomes (+10% staff pool, +3% facilities, ×1.08 collective). It reads no real book. It becomes the AD's answer from his books (topic 5).
  - `finance/college.rs`, `open_year`: football's staff and recruiting lines come from one formula, spend = 0.35 + 0.45 × prestige + 0.2 × fans + reserves. It becomes the AD's own allocation (topic 3).
  - `program.rs`, `update_collective`: one number drifting to a target. The big booster becomes named donors with wallets (topic 2); the wider base stays a mass.
  - `college_review`'s buyout factor becomes who pays (topic 7). `ad_turnover`'s flat rate becomes the AD's standing with his president (topic 6).
  - Two facility asks today (`boss.rs` Facility and `ask_facility`) become one: the campaign (topic 4).
  - All of this changes results for every program, so it needs a world rebuild and a trial (question 5).

---

## Topic 1. The people around the money

### How real life works
- **The AD** runs every sport, not just football: the budget, hiring and firing head coaches with the president's sign-off, scheduling, facilities, fundraising through the athletic foundation, ticketing and seating policy, and conference business beside the president. ADs come up through fundraising, business or coaching (Barry Alvarez was Wisconsin's head coach and AD at once in 2004-05; Bear Bryant held both jobs at Alabama, from memory).
- **The president** decides what athletics is for at his school: a brand engine for applications and the legislature, or a cost to keep in line. He signs off on head coach hires, debt and conference moves, and he is the one who fires an AD.
- **The board** (regents, trustees, curators) approves big contracts and borrowing at public schools. Utah's board approved the first private equity deal for an athletic department on December 9, 2025 (Otro Capital, about $500 million with donor capital; finalized July 1, 2026) (CBS Sports, KSL, Front Office Sports).
- **The state** can step in at public schools. After LSU fired Brian Kelly on October 26, 2025, owing about $54 million that unnamed donors pledged to cover, Governor Jeff Landry said AD Scott Woodward would not hire the next coach; Woodward resigned four days after the firing (NBC Sports, Fox Sports, theScore).
- **The athletic foundation** (Texas A&M's 12th Man Foundation, Clemson's IPTAY, the Gator Boosters, all from memory) is a separate nonprofit that raises the department's gifts and runs priority seating. Its board is the biggest donors.
- **The collective** is a third party, legally not the school. Since July 2025 the College Sports Commission's NIL Go reviews its deals; its July 10 memo said a collective paying a school's athletes had no valid business purpose, and the revised guidance of July 31 let collectives pay athletes directly when a deal meets business-purpose benchmarks (Bond Schoeneck & King, AP).
- **The faculty senate and the student government** push back on subsidies and fees (Berkeley's academic senate reports on the stadium debt, from the search above).

### My proposal
- **The AD keeps judgement and patience (built) and gains six traits on the 1-20 scale:** fundraising (how much a donor gives when he asks), stewardship (how much risk he takes with debt and reserves), ambition (how hard he chases facilities, a bigger conference, a bigger name), football first (how much of the department he will bend to football against the other sports), politics (how well he keeps the president and board with him), and media touch (how a cut or a firing plays in the press). Plus topic 1's personality (ego, loyalty, temper). Proposed, not confirmed: drawn N(10, 3.5) like the built two, with fundraising and politics a little higher at Power 4 schools, where ADs are hired for them.
- **The president becomes a person** (now only a name on the interview panel): athletics view (brand engine to academic first), risk aversion, political exposure (high at a flagship public, low at a private school), patience with his AD, and his ties to the big donors. He turns over at real rates (from memory: university presidents serve about six years on average).
- **The board** is a body, not a person: an approval bar for contracts and debt by school type, and a politics read from the state (public flagships answer to the legislature; private boards answer to donors).
- **The state steps in** only at public schools, only on loud events (a firing with a huge buyout, a scandal, a sport cut that makes national news), and reads the governor's own politics. Rare: proposed, not confirmed, about one case a decade across FBS, as the record shows.
- **The athletic foundation** is the AD's fundraising arm, with a CEO (a staff person) and a board made of the program's named donors (topic 2).
- **The collective** stays a third party with a head (a cast person) and its own money. It answers to its donors, not the AD.

---

## Topic 2. Three kinds of money, and the walls between them

### How real life works
- **Department money** is the university's. The AD spends it inside a budget the president approves. Football and men's basketball carry most of the other sports.
- **Restricted gifts** go only where the donor said: a building, an endowed scholarship, a coach's buyout fund. The AD can't move them.
- **The revenue-share pot** (from July 1, 2025) is department money paid straight to athletes, up to the cap ($20.5 million in 2025-26, rising about 4% a year). Up to $2.5 million of new scholarships counts toward the cap. The settlement doesn't say how the pot is split across sports; most schools planned about 75% football, 15% men's basketball, 5% women's basketball and 5% everything else (Front Office Sports, CBS Sports). The House back damages (about $2.8 billion over ten years) are split by the same 75/15/5/5 formula and are on appeal at the Ninth Circuit over Title IX.
- **Collective money** is the boosters', paid by a third party under NIL Go.
- **The walls leak in only a few real ways:** the university's subsidy and loans (president, board), student fees (student votes, the board, and in some states the legislature), borrowing (the board), private capital (the board, from 2025), and donors choosing where to give.
- **The donor's dollar is the same dollar.** ADs have said since 2022 that the department and the collective compete for it, worst below the Power 4 (On3, "donor fatigue"). Taxes push it: gifts to the school are deductible, and the IRS said in 2023 that most collectives' gifts aren't. Since 2018 the 80% deduction for seat-priority donations is gone (the 2017 tax law, from memory). From 2025 a donor can give to the school for revenue share and deduct it.

### My proposal
- **Football's money lives in four ponds**, each with its owner:
  1. **The department's operating money** (the built book). The AD's, under the president.
  2. **Restricted gifts**, held by purpose: a project's pledges (built), endowed scholarships, a buyout fund raised for one firing. They pay only that purpose.
  3. **The school's revenue-share pot** (built as `Collective::school`). Its size and its split across sports are the AD's call in spring (question 1). You set every football player's number inside football's share (decided).
  4. **The collective's money** (built, external). The donors' and the collective's. NIL Go reviews it (built). You set every deal (decided).
- **Each big donor is a named person with a wallet** (replacing the single `big_booster` number). Proposed, not confirmed: 6-10 named donors at a Power 4 school, 3-5 in the Group of 5, 1-2 in FCS, about 1,500 people in all (a few hundred kilobytes). Each has:
  - **capacity:** what he can give a year, from his wealth;
  - **affinity:** how much he loves the school, which wins and slights move slowly;
  - **wants:** a weight on each of five things a gift can buy: winning now, a legacy (his name on a building), access (the sideline, the locker room, the coach's phone), the whole school, and a say (a voice in hires and the depth chart);
  - **ego, meddling and patience** (topic 1's traits; meddling is built for the cast's donors);
  - **tax weight:** how much the deduction matters to him.
- **Each year a donor splits his giving** across the places that ask him: the annual fund (tied to his seats, so sticky), the AD's campaign, the collective, and a buyout fund when one is raised. Each place's pull is how well it serves his wants right now, times who asked and how well (the AD's fundraising, your recruiting and relationships through the talk system, the collective head's), times the deduction for those who care. So:
  - a campaign with naming rights pulls legacy donors away from the collective;
  - a portal window with a named target ("we lose the quarterback without $1.2 million") pulls win-now donors toward it;
  - a losing streak turns affinity down and access-hungry donors toward a say (strings, built in L9);
  - a cut sport costs the "whole school" donors and a scandal costs everyone.
- **The wider base stays a mass** (season ticket donors by priority level and annual fund members), moved by fan mood as built. The collective's wider base keeps the built drift.
- **The leaks are the AD's verbs** (topic 3): asking the university for subsidy or a loan, asking for a student fee raise, a bond issue, and private capital from 2025, each needing the president or the board.

---

## Topic 3. The AD's verbs

### How real life works
- **The budget** is built in spring and approved by the board in June for a July 1 start (the built money year).
- **The revenue share** total and split are set before July 1.
- **Coaching:** reviews, extensions, firings and searches, with buyouts paid from reserves, donors or loans.
- **Capital projects** start with a feasibility study and a quiet phase of big pledges before a public campaign and the board's approval of the debt.
- **Sport sponsorship:** adding or cutting sports. About 41 Division I programs were cut by July 2025, ahead of the settlement's costs and roster limits; Cal Poly cut swimming in March 2025 citing $450,000 a year (KBTX, KSTP). Title IX's three-part test (proportionality, a history of adding women's sports, or meeting women's interest) governs cuts; Iowa reinstated women's swimming after a Title IX suit (Athletic Business).
- **Scheduling**, ticket and seat policy, the multimedia rights partner, and conference business beside the president.

### My proposal
The AD acts on the real calendar, and every verb is a choice his traits make against his books. No slider sets any of it.

- **Spring: the budget.** He sets each line for the year from what came in, what's owed (debt, contracts, the House damages), the floor for the other sports, and his priorities. Football's lines (staff pool, recruiting, support, travel) replace today's formula: football first and ambition push them up, stewardship and a thin reserve hold them down, the president's view sets how much subsidy he can count on. A deficit he can't cover goes to the president as a loan ask (topic 6).
- **Spring: the revenue share.** How much of the cap the school funds (not every school funds it all: G5 schools fund a fraction, built as `opt`), and the split across sports. Football first moves football's share from the usual 75% (proposed, not confirmed: 65-85%); politics and Title IX exposure hold the women's sports' share. You hear football's number and can argue it in your spring meeting (question 1).
- **Any time: coaching.** The built review, now paying the buyout from a real pond (topic 7).
- **July: capital projects become campaigns.** He decides whether to start one from ambition, the facility gap against his conference, and his donors. A project goes: idea, quiet phase (he asks his named donors one by one for lead gifts), the board's vote on the debt, public phase, groundbreaking, opening, then 22 years of debt service (built). A quiet phase that falls short gets shelved, shrunk or borrowed for anyway, by stewardship.
- **Spring: sports.** A department in structural deficit (topic 6) looks at cuts. He picks what to cut from cost, Title IX room (participation against enrollment by sex) and the donors who back each sport. Cutting a men's sport is legal and costs the "whole school" donors; cutting a women's sport without room invites a lawsuit with a real chance of reversal. Other sports aren't played as games (decided), so each one is a roster count, a budget and a set of backers.
- **Fall: fundraising focus.** Which campaign he pushes and which donors he courts. This is where he and you collide (topic 5).
- **Winter: his stance on the collective:** arm's length, a partner (shared donor calendar, the collective in the foundation's events), or folding it into the school's marketing partner. His politics, the president's risk aversion and NIL Go's era decide.
- **Rare: the leaks.** A university loan, a student fee raise, a bond issue, private capital (from 2025; Utah's model). Each goes to the president or board, who read their own traits and the school's politics.
- **Scheduling** (built in L10) and **conference business** (built realignment), now with his ambition and the president's view in the call to move.

---

## Topic 4. Debt and the arms race

### How real life works
- Cal renovated Memorial Stadium for about $445 million and planned to pay it with Endowment Seats. Sales stalled at 1,857 of 2,902 by 2013, and even a sellout would have left about $134 million short. The stadium payment took about $18 million of an $89 million budget, about 20%, and drove most of a $22 million deficit in 2016. The campus later took on much of the debt (Sports Business Daily, CBS San Francisco).
- Debt service is fixed. Seat donations and gate move with winning. A losing cycle on a big debt squeezes everything else.
- Clemson's $55 million football building (2017) and the indoor field race are the arms race (later extras topic 1).

### My proposal
- **Debt service stays as built** (7.5% a year of the borrowed part for 22 years), and now the borrowed part is what the quiet phase didn't raise. An ambitious AD with low stewardship borrows a gap a careful one would shrink.
- **Pledges are paid over five years** (proposed, not confirmed), and a donor whose affinity falls can stop paying. A pledge that dies after groundbreaking becomes debt. That is Cal's road.
- **Debt drag** is just the books: when gate and seat donations fall in a losing cycle and debt doesn't, the next spring's budget has less for football's lines, unless the AD draws reserves, asks the president, or cuts other sports.
- **A new building still moves facilities (built),** so the arms race is a bet: it helps recruiting for years and costs every year for 22.

---

## Topic 5. You and the AD

### How real life works
- Head coaches ask for staff money, facilities, NIL, scheduling, an extension, and argue with the conference's numbers: "Georgia pays its staff $9 million more than we do."
- ADs say no, or yes with a condition. Coaches go around them: to donors, the president, or the press.
- Coaches raise money too: booster club tours in spring, donor dinners, calls to the collective's biggest givers.

### My proposal
- **An ask becomes a case, like a recruit's.** You argue with facts from the fact sheet (`claims::team`), graded against the schools you're measured against (your conference, your class rank rivals): "our staff pool is 11th of 16", "the three schools ahead of us in the class rankings pay more", "we lose our left tackle without collective money". He weighs it by his priorities, his trust in you and his books. The built dice roll goes.
- **His answer reads his real money.** Can he afford it after the floor, the debt and the reserves? Where would it come from? A yes names its source, and the cost lands where the world can see it: reserves drawn, a sport's travel cut (with a news line when it's big), a donor's gift with his string attached, or a fee ask to the president. A no gives the real reason ("the stadium payment", "the board won't see more subsidy this year", "Title IX won't let me take it from the women's side").
- **A yes is a debt.** Each grant becomes one of his goals for you (people topic 12, built goals), and missing it costs more trust than a missed goal you never asked for.
- **You can go around him:**
  - **To a donor**, for the collective: the donor's split moves your way, and if it moves money out of the AD's campaign, the AD learns (his fundraising staff talk to the same donors) and his trust in you falls. A donor you talk to a lot wants more say (L9 strings, built).
  - **To the president:** works when the president's view is brand-first and he likes you; costs the AD's trust either way; and a president who backs you over his AD twice starts reading the AD's standing down.
  - **To the press:** a public ask ("we need an indoor facility to compete") moves fan mood and donors, and costs the AD's trust; his media touch decides how it plays.
- **You fundraise yourself:** the spring booster tour (a dated event), donor dinners and calls through the talk system. Your relationships and Aura (built) move what a donor gives, and where. You don't choose which pond gets it: the donor does, from his wants and your pitch.
- **You never set the AD's budget, the revenue-share split or any other sport's money.** You argue, and he decides (question 1 covers football's share).
- **What you see:** his words, what he grants, the news (cuts, campaigns, a donor's gift), your own lines, and the department's public numbers each winter (decided). No meters.

---

## Topic 6. The states that matter

Each one is read from the world each month or at its real moment, never rolled on its own.

### The department's money
- **Healthy:** reserves growing. The AD can grant asks and start campaigns.
- **Tight:** spending equals income. Asks need a named source.
- **Deficit:** reserves falling, or the university filling a gap past its usual share. Asks mostly get no; he looks at fees and loans.
- **Structural deficit:** two or more years of deficit with reserves gone (proposed, not confirmed). He has to cut: staff pools first at a careful AD, other sports first at a football-first one; the president and faculty start asking questions.
- **Rescue:** a loan, a fee raise, private capital, the campus taking the debt (Cal 2018), or a conference move. Or, at the bottom, dropping a level or football (question 3).

### The AD's standing with his president
- **Secure, questioned, on notice, gone.** It moves with the department's money, football and basketball against expectations, scandals and how they played (media touch), donors' complaints (big donors have the president's ear), and Title IX trouble. This replaces the flat 6% turnover. A new AD reads you fresh (built).

### Each big donor
- **Engaged, pledged, restricting, cooling, lapsed, revolting.** Affinity and wants move it: losing, a slight (you ignored his string), a cut sport he backs, being outbid for the naming. A revolting donor stops paying his pledge, takes his money to the collective or away, recruits other donors, talks to the press, and offers to pay a buyout to get the coach or the AD out.

### The collective
- **Healthy, drying (built trend), merged or folded.** It folds into the school's partner when the AD's stance and NIL Go's era make it the better home for donors.

### A project
- **Idea, quiet phase, board vote, public phase, building, open, paying off** (topic 4).

### The conference
- **Stable, courted (an invitation or a rumor), leaving (exit fee), new member (part shares), or left behind** (built realignment). The ACC's 2025 settlement with Clemson and Florida State is the real model for a fight: the exit fee starts at $165 million in 2025-26 and falls to $75 million, and 60% of TV money is split by brand value (CBS Sports, On3).

### Your seat
- Built (people topics 12 and 14), now with the AD's standing and the donors' moods in it.

---

## Topic 7. How things go wrong (none of them scripted)

- **Donor revolt.** A few big donors cool after losing and a slight, then one offers to fund the buyout. The AD can take the money (and owe the donor), refuse (and lose him), or stall. Real: LSU's donors pledged Kelly's $54 million in 2025.
- **The buyout trap.** A coach the AD wants gone costs more than any pond can pay. He keeps him, and the program slides further. The built buyout factor is this, now read from real ponds: reserves, a donor fund, a university loan.
- **Debt drag.** Topic 4's chain: fixed debt, falling seat money, squeezed football lines, worse recruiting, more losing.
- **Overbuilding.** An ambitious AD borrows the gap of a campaign that fell short, and pledges die later (Cal).
- **Cannibalized giving.** A strong collective drive drains the annual fund and the campaign, so the department runs short and cuts the staff pool you were trying to protect. Worst below the Power 4, as the AD surveys found.
- **Left behind.** Realignment takes the media money away in a year. Oregon State and Washington State after 2024 are the real case (from memory).
- **The Title IX trap.** A department that funds football's revenue share by cutting a women's sport loses the suit or the press, and the sport comes back.
- **The president's turn.** A new president with an academic-first view stops the subsidy; a scandal makes him take over a search.
- **The state's turn.** The governor takes the hire from the AD at a public school after a loud firing.
- **An AD out from under you.** He's fired or leaves; the new one didn't hire you (built).

---

## Topic 8. The loops

- **The win loop (more money from winning):** wins → fan mood and gate → seat donations and donors' affinity → staff, recruiting and collective money → wins. Today it saturates at the top (`afm/dynasty-layers/diagnosis.md`: boosters near the $20 million ceiling and brand at 1.0 keep their pull through bad years). Its real brakes come from this design: donor wallets that are finite and split, debt taken on in good years that is still owed in bad ones, the revenue-share cap that every power school hits, and Title IX's floor under the other sports.
- **The losing spiral:** losses → affinity and seat money fall → debt doesn't → football's lines shrink → recruiting falls → losses. It breaks on a coaching change, a new big donor (built arrival), a conference move, a rescue, or a few lucky years.
- **Donor capture:** the more the AD (or you) leans on one big donor, the more he asks for say. A diversified donor list is slower but safer. An ambitious AD with low stewardship builds on one man (T. Boone Pickens at Oklahoma State, people topic 12).
- **The trust loop:** what he grants you becomes a goal you owe; hitting it earns the next yes; missing it costs more.
- **The Title IX coupling:** football's growth is held by the other sports' floor and the women's sports' share. The Education Department rescinded the guidance treating NIL as athletic aid on February 12, 2025 (McGuireWoods, National Law Review), so today revenue share sits outside Title IX's aid rule, but cutting women's sports doesn't. The world can change that after 2026 (question 4).
- **The arms race:** a rival's new building moves its facilities (built), which raises your AD's facility gap, which pushes an ambitious AD to start his own campaign, which pulls donors from your collective.

---

## Topic 9. AI programs and other levels

- **Every AI department runs this exactly as yours,** with its own AD, president, donors and collective. AI head coaches ask their ADs by the same case, with their staffs' reads, so the carousel and every department's money come from the same people.
- **The NFL** has no AD: the owner and his books are built (section 5, later extras topic 1). Nothing here changes it.
- **High school** keeps the built side: the district, the principal and AD, the booster club, town support and bond votes (later extras topic 1, L11). Nothing here changes it.

---

## Topic 10. What a trial must show

Against real numbers, after a world rebuild (proposed, not confirmed as the list):
- the share of FBS departments in deficit before university support, by conference tier (NCAA financial reports, Knight-Newhouse);
- sports cut a year across Division I (about 41 by July 2025);
- head coach firings with donor-funded buyouts, and buyouts that kept a coach;
- debt service as a share of budget at the heaviest-debt schools (Cal's 20% is the high end);
- AD turnover a year (built 6% as the starting check);
- the win loop's lock-in against today's (`afm/dynasty-layers/diagnosis.md`), named programs that rose or fell on money, with why;
- every result explained, as every trial is.

---

## Questions for Cain

1. **Football's revenue-share pot.** The AD decides how much of the cap the school funds and football's share of it; you argue for more in your spring meeting; you still set every player's number inside it. My pick: yes.
2. **An AD seat for you.** You can take an AD job yourself as a career step after coaching (Alvarez, Bryant). My pick: yes, built after the AI ADs are in and measured.
3. **Your school can drop.** Your own school can step down a level or drop football while you coach there (UAB cut football in December 2014 and brought it back in 2017; Idaho went from FBS to FCS in 2018, both from memory), with a year or more of warning signs, and your career goes on. My pick: yes.
4. **Title IX can change.** After 2026 the world can put revenue share under Title IX (a court ruling on the House appeals or new guidance), at a small chance a year. My pick: yes.
5. **When it's built.** The AD layer goes into the world after the coming three-world rebuild, not into it (it changes every department's results and would hold that rebuild back). My pick: yes.
