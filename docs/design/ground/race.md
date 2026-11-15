# Ground round 5: the race (college, 2026-11-15)

College only (Cain, 2026-11-15 16:16 UTC). Round 5 of the offseason game (`offseason-game.md`, section 14): rival recruiters as people, and the AI playing Ground. It builds on rule 2 (news goes in order), standing (GR17-GR29), rounds 1 to 4 (GR30-GR63) and what the game already has: how AI staffs recruit (`recruiting.rs`), coaches' recruiting ties that travel (`coach.rs`), the staff market and the carousels (`staff.rs`), commitments, flips and summer announcements, where you stand in a recruit's race and why he chose a rival (`front_office/college.rs`, `lane_help.rs`), and outside-the-rules picks in talks (`folk/finals.rs`). Cain said yes to the round 5 reply and its question on 2026-11-15 at 19:02 UTC (GR64, GR65). Built facts were read on main (cdfc3b1) on 2026-11-15.

## What it does

Every rival staff works the same ground by the same rules, and its recruiters are people you meet. Auburn's man in south Atlanta has a name, a face, a style, his own Home schools and a history with you. You see the race the way a real staff does, in signs: the sign-in sheet, the polo in the stands, the kid's visits and posts, the order the news reached you, and what a coach who trusts you tells you. You never see a rival's numbers. Both sides make the same moves (invade, defend, poach a man, outwork, flip a commit, run the other down to a family, spend money) and pay the same prices. The AI plays with the same rule book, money and people, and it only knows what its people know.

## Two races

**Georgia State against Auburn's man, May 2027.**
- Monday, 5 AM, the map: south Atlanta is in Auburn's colors. Its defensive line coach, Danner, has worked it for nine years and is Home with three coaches there. Columbus is yours, Trusted since April.
- Tuesday, 7:40 AM, a south Atlanta school: the sign-in sheet shows Danner was in yesterday. The coach is only Known with you, so you get his list. Danner got the truth about his junior tackle a day earlier.
- Thursday, a 7v7 tournament: Danner is in the stands in his Auburn polo, friendly. "Heard you've got Columbus." He heard it from the Columbus coach's defensive coordinator, his old teammate.
- That night, a warning: "Columbus: Auburn's been in twice since March, and Danner's telling families you'll be gone in a year." That's a claim about your job, and it settles in December. If you're still there, the families who heard it remember who said it.
- Monday at the map you choose: a second man into south Atlanta (two of your road days), or both men on Columbus.

**LSU against Texas for the Destrehan kid, June 2026.**
- Texas got in through his 7v7 director, who is Home with Texas because its camp put his team on film.
- The first weekend of June he takes his official visit to Texas, and his mother posts the photos. The race is public now.
- Texas answers your fence (built, from the takeover) with its collective: a bid over yours, inside its cap (built bidding).
- You host him the next weekend. Coleman and the collective match what NIL Go clears. You tell his mother the truth about the role.
- July: he picks Texas. The why: his mother weighed the money most, then the safety role. In August the paper holds you to what you said at media days (GR59).

## Rival recruiters

Every staff's men have areas, the way real staffs split states, and their standing there comes from the world's 30 years (coaches' ties, pipelines, footprint). Each one is a person (GR62) with:
- **A style from his makeup:** a grinder (work ethic: he's everywhere), a closer (people skill: families love him), a salesman (low straightness: he oversells and runs rivals down), a straight shooter (high straightness: coaches trust his word).
- **His ground:** where he's Home, and who he's tied to (his old school, his old teammates now coaching high school).
- **A history with you:** you played together, you beat him for a kid, he poached your man.
- **A career:** he moves up (coordinator, head coach) and moves jobs in the carousel, and his ties go with him.

The rivals page opens when a rival first shows up (round 1's `campaign.md`). You learn a rival's style the way you learn anyone's nature, from what he does.

## What you see

Signs, never numbers, and fogged by your standing (GR2):
1. The sign-in sheet at a school's front office: who came and when.
2. The polo in the stands: at camps, 7v7 tournaments and high school spring practices in the offseason, and on Friday nights in the fall.
3. A kid's visits: unofficials and officials, public when he posts them.
4. A kid's posts: offers, a top five, a commitment or a flip, by his habits and his following (GR43-GR46).
5. The order the news went, on every lead (rule 2): "Auburn: Monday. You: Tuesday."
6. What people tell you, by their standing. A Trusted coach says "Auburn's been in four times." A Home coach says "Auburn's about to offer him. Get in first."
7. The services: stars and rankings (built), and new predictions from the services' own sources, which can be wrong.
8. The map: owners' colors (GR8) and rivals' pipelines (GR6).

## Moves, both ways

- **Invade:** pour men and days into a rival's area, as Michigan's 2015 camps did in the South (`ground-design.md`).
- **Defend:** hold your Home schools with visits, calls and your own call by name (GR53).
- **Poach:** hire a rival's area man. His own ties come with him and his old program keeps its own (GR24, GR29, GR62). Rivals poach yours the same way.
- **Outwork:** more days and more calls, from the same road days and ledger (GR48, GR50).
- **Flip:** work a rival's commit until he signs. Commitments already hold by loyalty and trade up (built). Yours waver with signs (an unofficial to a rival's camp, his trainer's posts), you get a warning (GR25) and one scene to fight for him. Travis Hunter flipped from Florida State to Jackson State on early signing day 2021.
- **Run a rival down:** a claim about a rival to a family (his hot seat, his depth chart, his scheme) is a claim like any other. It settles on the facts, and a false one costs whoever said it with that family and their coach (GR62, the built lie check). Rivals do it to you.
- **Money:** legal NIL offers and bids, cleared by NIL Go (built). Outside the rules, a booster's or handler's price (built as outside-the-rules picks with a compliance risk in talks, and handlers as deciders). Rivals do it too, by their program's nature, and you hear it as rumors you can't prove. Found out, it's a case: Tennessee's 2023 case over cash and gifts to recruits' families brought a big fine, probation, and a six-year show-cause for its head coach (NCAA). Ground's own work still never buys a signature (rule 3). Only money can try, and that is a rules case.

## The AI's answers

By its staff's nature and its needs, a program losing an area sends a second man, sends its head coach when the rules allow (January, the fall, December), hires a local high school coach (the Malzahn move; from 2017 only to an on-field job), offers earlier, bids with its collective, or runs you down.

## The AI plays Ground

- The same rule book (road days, GR48), the same money (trips, GR49), the same people and memory (GR62) and the same favors (GR60-GR63).
- It plans its week by its needs (its class plan, built), its ground (its men's standing) and the threats it sees, through the same signs you see.
- It knows only what its people know. Its board comes from its men's tips and looks and from the services, not from stars and footprint (round 2's D11). That replaces today's board, and doesn't sit beside it.
- It plays as well as its people: their work ethic, recruiting skill, eye and way with people (built ratings).
- It changes how the whole world recruits, so its build checks the world's recruiting against real, like GR44 and GR45: where blue chips sign, flagships holding their state, how often low-star kids are drafted, and the FCS share of draft picks. It runs in GR61's trial worlds, so it adds no world runs.

## The race in public

July's announcements and flips (built summer weeks), the services' rankings and predictions, August's preseason lists, media days (GR59) and head coaches' public shots. In May 2022 Nick Saban said Texas A&M bought its players, and Jimbo Fisher called him a narcissist (ESPN). Families read it, and so do the kids.

## What's built and what's new

| Built | New |
|---|---|
| AI boards from reach, pipelines, footprint and stars; relationships from the head coach's and position coach's time; offers in rounds; NIL bids (`recruiting.rs`) | Boards from their men's ground (tips, looks, standing), like yours |
| Coaches' recruiting ties that travel (`coach.rs`) | Area men with standing, a style and a history with you |
| The staff market and the carousels (`staff.rs`) | Poaching a man for his ground |
| Commitments that hold by loyalty and trade up; summer announcements (`recruiting.rs`) | Warnings and a scene when your commit wavers; signs of a rival's flip |
| Where you stand and why he chose them (`front_office/college.rs`, `lane_help.rs`) | The signs: sign-in sheets, polos, visits and posts, the order of news |
| Outside-the-rules picks with a compliance risk (`folk/finals.rs`); handlers as deciders | Rivals' money outside the rules, rumors and cases |
| The services' stars and updates (`calendar/anchors.rs`) | The services' predictions from their own sources |
| Media days, the press and the fans (GR59) | Head coaches' public shots |

Taken as design, since real life and what's built settle them: the money side, running rivals down, poaching, and the AI on the same rules, checked on GR61's worlds. Only the question below changes an agreed rule (GR20).

## Question (agreed, Cain 2026-11-15 19:02 UTC: GR64)

**1. Within a level, a busy coach's nature decides who hears first: a loyal coach calls the staff he's closest to, a self-serving one the biggest program?**
Example: Georgia State and Auburn are both Home with the Columbus coach. He's loyal, and you've done more for him (the corner at Mercer, a clinic talk), so you hear about his new end on Monday and Auburn hears on Tuesday. If he were self-serving, Auburn would hear first, because a bigger program lifts his school (GR21). Today's rule (GR20) always puts the bigger program first within a level.
My pick: yes.
