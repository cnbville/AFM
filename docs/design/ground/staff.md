# Ground round 7: your staff's ground (college, 2026-11-15)

College only (Cain, 2026-11-15 16:16 UTC). Round 7 of the offseason game (`offseason-game.md`, section 14): your men's areas and their ground, hiring for ground, losing a man, your men's careers, and the college assistant seat (SK76). It builds on standing (GR17-GR29, above all GR24 and GR29), the February meeting (GR30), the meeting at the map and the road days (GR47, GR48), one memory per person (GR62), the race (GR65) and finds (GR66, GR67). It also builds on what the game already has: recruiting areas with an area coach and a position coach (`world/scouting/dept.rs`), coaches' ties, careers and the carousels (`world/coach.rs`, `world/staff.rs`), the offer chain with overruling and each coach's record (`front_office/chain.rs`, SK25-SK31), hiring and its talks (`folk/staffing.rs`, `folk/hiring.rs`), personnel staffs and junior posts, and the assistant's job (`world/seat.rs`). Cain said yes to all of it on 2026-11-15 at 19:37 UTC (GR68, GR69). Built facts were read on main (87933f4) on 2026-11-15.

## What it does

Your men are your ground. Each one carries his own: the people who know him, the towns he grew up and coached in, his record and his finds. Where you send them is one of February's big calls. Hiring a man buys his ground, and losing one takes it with him, often in December, days before the early signing day. They're people with careers of their own: they climb, they get courted, they sulk when you overrule them, and the ones who leave on good terms become your coaching tree. As a college assistant you play it from the other side: you're the man in the car, with one area, one position group and one boss to convince. That also makes it the gentlest way into Ground for a new player.

## Two of them

**LSU: keeping Houston.**
- February 2027, the meeting at the map: your recruiting coordinator proposes the areas. Your backs coach grew up in Houston and coached eight years at a Houston high school. He's Home with four coaches there, so Houston is his. Your new safeties coach wants Houston too (he's from Katy), but he's only Known there. You give him the Golden Triangle, new ground he'll grow slowly, and he rides along on two of the backs coach's Houston trips in May.
- November 28, the carousel: Texas A&M's new head coach wants your backs coach as his offensive coordinator. The warning: "A&M has called him twice. College Station is 90 minutes from his mother's house, and he was passed over for coordinator last year." The early signing day is Wednesday.
- The scene in your office: a raise from your staff money, the assistant head coach title, or a promise of your next coordinator job. You promise, and he stays.
- If he'd gone: his own ties go with him, so Houston drops to what LSU itself has there (the coaches whose kids did well at LSU), his two Houston commits waver (one scene each to keep them), and the safeties coach starts Known with the coaches he met on the ride-alongs.
- December 2028: your offensive coordinator leaves, and the promise comes due.

**Georgia State: the D-line coach.**
- You start as Georgia State's D-line coach, under an AI head coach. Your area is Columbus and west Georgia, and your position is the D-line.
- May 2026: the Columbus coach tells you about his sophomore end who grew three inches (round 6's end, from the other chair). Your head coach's Monday plan gives you three road days this week, and you spend two in Columbus.
- June 13, your camp: he measures 6'4" and 245 and runs a 4.78. Monday you put him on your list. Your head coach says wait for his junior tape. He's a careful man, and you haven't been right for him yet.
- October: Auburn offers him, a level up. The next Monday your head coach signs your push on a Valdosta tackle without a word.
- 2029: the end makes all-SEC. He's Auburn's find, since Georgia State never offered (GR66), but the people who saw you push remember: your head coach and the Columbus coach, and word gets around through their ties. That December a Sun Belt head coach hiring a defensive coordinator calls you, with your record as one of his reasons. Your Columbus ground goes with you, and Georgia State keeps the program's.

## Your men and their areas

- **An area and a position.** Each on-field assistant works one or more areas on the map plus his position group, the way real staffs split it: the area man finds kids and builds the people, the position coach judges them and sells the role (built: the area coach and the position coach, scouting topic 15). Today an area is a whole state (`dept.rs`, `set_area`). Ground's areas are the map's (GR1), so a big state splits among several men.
- **Set in February, moved any Monday.** At the February meeting your recruiting coordinator proposes the split by each man's ground, and you change it at the map any week (GR47).
- **Your choices:** a man on his home ground (fast, since he's already Known or better there) or on new ground (slow: he starts a Stranger and climbs by GR28's times); one man on an area, or two on a big one (more road days there, fewer elsewhere); or an area left empty, which shows up as misses in the August ground report (GR67).
- **The ride-along.** A young coach rides with your veteran on his visits. It costs both men's road days (GR48), and the coaches he meets know him on his own, so he keeps that ground when the veteran leaves.
- **The staff room.** When your area man loves a kid and your position coach doesn't, the case comes to you as a scene, with both men in the room (a new scene, GR55).
- **Off-field staff stay on campus** by the rule book (built: the director of player personnel, the recruiting director and the analysts evaluate on campus, and only on-field coaches go out). They fill tape blanks (GR67; built staff film study uses no road days, SK72), keep the board and run the visits you host. If the 2027 model passed, up to 16 staff of any kind can go out (GR48's rule book).

## Each man's ground

- **His people:** his standing with each person in his area, with its why (GR18) and the moments they keep (GR62).
- **His ties:** his home town, his old schools and teammates, his coaching stops (built: coaches' ties that travel with them).
- **His record:** every call he made (built, SK30), his finds and their blooms (GR66, GR67), and the misses from his area.
- **His style,** from his makeup (GR65): a grinder, a closer, a salesman or a straight shooter. A salesman's claims to families are his to keep or break, and a false one costs him and your program with that family (the built lie check).
- **His week:** the road days you give him (GR48), his own hours and tiredness like yours (GR50), what he notices on his eye (GR52), and what a visit buys by his way with people (GR23).
- **His mood:** a find that blooms lifts him (GR67). Being overruled again and again costs his heart and makes him listen to other jobs (built, SK29), and so do an area he didn't want, a long spring away from home and being passed over for coordinator.

## Hiring for ground

- **Teaching, scheme or ground.** When a seat opens (built: the carousel the Sunday after rivalry week, or mid-year), the best man rarely has all three. The best D-line teacher on the market has never worked your state, and the man who owns Houston is an average teacher.
- **His ground shows in signs,** fogged by your network (GR2): his old jobs and where his signees came from (public), his finds if word reached you, and what your ties say ("he's the man in Houston", from a coach who knows him). The interview's recruiting and network reads (built, C25) are your read, never his true numbers.
- **What comes with him:** his own ties and his standing with people (GR29, GR62). His old program keeps its own. A man hired off a rival (GR65's poach) turns his old Home schools into contested ground.
- **A high school coach for his area:** his ties and his school's trust come with him. The built limits hold: while a kid of his is on your board, only for a five-star, and from 2017 only to an on-field job (C17, the NCAA's rule). The Malzahn move is the big version (`race.md`).

## Losing a man

- **December is the worst month.** The college carousel lands the Sunday after rivalry week, days before the early signing day (built: November 29 and December 2 in 2026).
- **Signs come first:** a rival's calls, his mood, what his ties tell you, and a warning with the reason and the fix (GR25's shape).
- **The counter scene:** a raise from your staff money, a title, or a promise (the next coordinator job). They're the tools his school already uses against you when you hire (built hiring talks: counters, titles and promises, C32, C46-C51), and a promise you break costs his trust and his network's (built, C48).
- **When he goes,** his own ties and standing go with him, and the program keeps its own (GR24, GR29). A man who rode along with him keeps what he built. Each commit's bond is partly with him, so his commits waver: a warning and one scene each (GR25, GR65), and some follow him.
- **His area doesn't go dark at once:** the program's ties still answer, but the next lead there reaches a rival first until your new man climbs.

## Your men's careers

- **They climb:** position coach to coordinator to head coach, with you or elsewhere, and their names grow from their finds and their teaching (built: the coach ladder and the carousels; GR67's name as a finder).
- **A man who leaves on good terms is your coaching tree:** Trusted ground at his new school, and Home if he's grateful (GR62).
- **A man who leaves angry** (overruled too often, a promise broken) runs you down to families in his new area (GR65).
- **Word travels through their ties both ways.** Your men hear what their old teammates hear, on high school staffs and rival ones, and rivals hear what they let slip, by each man's nature (GR62; in `race.md` Danner hears about Columbus from his old teammate there).

## The assistant seat (SK76)

- **What's yours:** your area and your position group (built: your group's depth chart and the players' plans). The rest is your head coach's: the program's board, the goals, the money and the final word on offers.
- **Your week:** his Monday plan gives you your road days and your film, and you plan your own route within them. Every visit, call, Friday night and camp on your route is yours (`week.md`): the week's key scenes come to you with the reason (GR51), and the rest play on your own ratings, the way game day plays the snaps you don't call.
- **Your list:** the built offer chain from the other side. You bring up to two kids a week, your side's coordinator watches the film first, and your head coach signs or says wait by his own read, the open spots and how far he trusts your eye. When you disagree, it's a scene in his office.
- **Your finds:** a kid your head coach signs is your find, credited to you (GR66). A kid he says wait on isn't, since your program never offered, but everyone who saw you push remembers: your head coach (his trust in your eye grows as the world catches up, in a bloom's steps: the built look-back from his side), the high school coach you told, and the people around them, through their ties, at about half its weight (GR62).
- **Your boss is a person** with one memory of you (GR62): your calls that held, the times you argued, the promises between you.
- **Your campaign:** at the February meeting your head coach gives you goals for your area and your position (GR30 at your seat), you pick your own ground goal, and the hunt is yours to try (GR33), with your head coach's yes on the offer. His August verdict counts when he reviews his staff (built: a kept head coach reshapes his staff).
- **Your career:** your record and your teaching bring offers (the built job market already says "how you recruit" among its reasons). Your ground goes with you (GR29). His fate is yours: if he's fired, a new head coach who knows you keeps you more readily (built). If a rival courts you, he can counter to keep you, the way the world's schools already counter for their men.
- **Your first offseason there is guided too** (GR31), by your program's recruiting coordinator.

## What's built and what's new

| Built | New |
|---|---|
| Recruiting areas by state, set by you or split by home state; the area coach and the position coach (`world/scouting/dept.rs`, topic 15) | Areas on the map (GR1), set at the February meeting; home or new ground, two men on a big area, the ride-along, the staff room |
| Coaches' ties that travel, careers, the coach ladder and the carousels (`world/coach.rs`, `world/staff.rs`) | Each man's ground: his people, his standing, his finds and record, his style, his week and his mood |
| The offer chain, overruling and each coach's recruiting record (`front_office/chain.rs`, SK25-SK31) | The chain from the assistant's side, with your head coach's trust in your eye |
| The hiring search, the interview's recruiting and network reads, hiring talks with counters, titles and tracked promises (`folk/staffing.rs`, `folk/hiring.rs`) | Hiring for ground, read in signs; your counter when a rival courts your man |
| A five-star's high school coach as a hire, on-field only from 2017 (`folk/backchannel.rs`, C17) | A high school coach hired for his area |
| Personnel staffs evaluating on campus, and junior posts: graduate assistants and analysts (`scouting/readers.rs`, `world/coach.rs`) | Off-field staff filling blanks, by each year's rule book |
| Commitments that hold by loyalty; bonds from the head coach's and position coach's time (`recruiting.rs`) | A commit's bond partly with his recruiter, so a man's leaving shakes his commits |
| The assistant's job: your group's depth chart and plans, and recruiting closed to you (`foIsHead`, SK76) | The assistant seat's whole Ground: your area, your route, your list, your campaign, your career |

The AI's staffs work by the same rules (GR65): their men have areas and careers, and they hire for ground, poach and lose men. Round 8 draws it: each man's area with his face on the map, and his page with his ground. The sizes (how much of a commit's bond is his recruiter's, how fast a head coach's trust grows, what a counter needs) are proposals, tuned in GR61's trial worlds and in play.

## No question this round (agreed, Cain 2026-11-15 19:37 UTC: GR68, GR69)

Everything here follows real life and the agreed rules, so it's all taken as design. The closest thing to a taste call: a kid your head coach says wait on isn't your find, because your program never offered (GR66), so he pays only through the people who saw you push. Making him a full find would pay the assistant seat more, but in real life only the people who were there know you pushed.
