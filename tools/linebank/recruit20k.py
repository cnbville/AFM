"""The recruiting talks line run (Cain 2026-11-12, "I want 20k lines, specifically for the recruiting personal
talks"; "go ahead with 20k in a fresh thread", API limit raised to $60): about 20,000 new versions for what a
recruit's side says to you, the college coach, on a call, at a home visit or in a sit-down.

Run it with `run.py --job recruit20k ...`. The parts are written against the facts and tags the game has today
(the lines tests reject any other); where a talk would be sharper with a fact the game does not hand over yet,
`docs/design/recruit-talk-lines.md` lists it. Nothing here is asked by the game's code yet except the portal
call's `portal.*` parts (14 parts that had 8 versions each): the rest wait for the talk design (F1) and the
talk screens (S1), which pick a part by the fact it is keyed on.

Two kinds of speaker, as the game builds them:
- the FAMILY SIDE of a high school recruit: `Speaker::parent` plus whoever decides (himself, parent,
  grandparent, handler), a loud, online or absent parent, a region;
- a TRANSFER in the portal: `Speaker::player` at college (position, personality, region, stage).
`portal.*` are his alone; every `recruit.*`, `transfer.*` part below is either his alone (`transfer.*`) or the
union speaker 'recruit': a version tagged for one side is said only by that side, an untagged one by both."""
from parts import PARTS, FILES, Part  # noqa: F401  (registering parts is the point)
import parts as P

FAMILY = 'recruittalks'
FILE = 'recruit_talks_run.txt'
FILES[FAMILY] = FILE

COLLEGE = {'college'}
# A transfer is a college player: his tags without the ones he cannot have.
TR_DROP = {'sophomore', 'junior', 'rookie', 'undrafted', 'firstround', 'psquad', 'brash'}

TARGETS = {}
# The first tally of the table below came to 20,920; this brings it to 20,000 (about 4.5% off every part).
SCALE = 0.94


def add(key, who, facts, note, target, **k):
    k.setdefault('levels', COLLEGE)
    PARTS[key] = Part(key, FAMILY, who, facts, note, **k)
    # The drive parts, the openings and the closes may repeat their own plain words (playing time, thanks);
    # every situation part may not repeat its key phrase.
    PARTS[key].phrase_cap = not key.startswith(('portal.', 'recruit.open.', 'recruit.close.'))
    TARGETS[key] = round(target * SCALE)


# ---------------------------------------------------------------------------
# P: the portal call (the game asks for these today; each had 8 versions).

DRIVES = {
    'playing': ("playing time: whether he would start, who is ahead of him at his spot, where he would sit on the "
                "depth chart, how many snaps", 280),
    'nil': ("the money: what the deal looks like, what the collective does for transfers, the number, whether "
            "it is real", 280),
    'school': ("the school itself: whether his credits and hours come with him, finishing his degree, what he "
               "would study, how class works for a player", 280),
    'distance': ("staying close to home: his family seeing him play, how far the drive is, being homesick or "
                 "far away last time", 280),
    'family': ("the staff and how they treat players: who has his back, whether he can trust the man he plays "
               "for, why he left his old coach, what the locker room is like", 280),
    'development': ("getting to the next level: coaching, film, the draft, what the position coach has done for "
                    "players like him", 280),
    'winning': ("winning: titles, the playoff, big games on TV, a team that plays for something", 280),
}
for k, (d, n) in DRIVES.items():
    add(f'portal.ask.{k}', 'player', set(),
        "A player who has entered the transfer portal is talking with you, the college coach who wants him (on "
        f"the phone, in his living room or at the end of his visit): his first question to you is about {d}. He "
        "decides for himself and opens on what he weighs most. He has already left one school, so 'last time' "
        "and 'where I was' are true; but never his role, his stats or why he left. Never say where the talk "
        "happens.", n,
        drop=TR_DROP, words=(5, 30))
add('portal.tell', 'player', {'first', 'second'},
    "You asked the transfer what he is looking for. He answers with {first}, then {second} (what matters most, "
    "then next, in lower case: playing time, nil money, the school, close to home, family, getting him to the "
    "next level, winning). The line must read with any of them. He has already left one school, so 'last "
    "time' and 'where I was' are true; his role, stats and why he left are not stated.", 300, drop=TR_DROP,
    words=(5, 30))
for k, d in {'good': "it landed well: it is what he wanted to hear, so he leans your way", 'flat': "it landed "
             "but it is not what he weighs most: polite, noncommittal", 'bad': "it landed badly: he does not "
             "believe you can back it up, or it is not what he cares about"}.items():
    add(f'portal.react.{k}', 'player', set(),
        f"A transfer in the portal reacts to the pitch you just made him, when {d}. Never names the pitch.", 330,
        drop=TR_DROP, words=(2, 25))
for k, d in {'knows': "he would start there anyway, so the promise adds little (polite, maybe a little amused, "
             "he knows what you are doing)", 'believes': "he believes you: your word is good with him",
             'doubts': "he has heard promises before and is not sure he believes yours"}.items():
    add(f'portal.promise.{k}', 'player', set(),
        f"You promised the transfer a starting job or a real role. His reaction: {d}. Never says which promise. "
        "He has already left one school, but never states a promise broken to him there.",
        170, drop=TR_DROP, words=(3, 25))

# ---------------------------------------------------------------------------
# Q: the beats of every talk (open and close), by the setting and by how he feels about you.

F = {'team', 'coach', 'you'}
FN = F | {'name'}       # {name} is the recruit's first name: only a family member says it, never the recruit
OPEN = {
    'warm': "He is keen on you: you are high on his list. He is glad to talk.",
    'polite': "He is courteous and noncommittal: you are one of several schools. Friendly, giving nothing away.",
    'guarded': "He is wary: other schools are ahead of you, or he has doubts about you. Short and careful.",
}
SETTING = {
    'call': ("A PHONE CALL (he picks up, or you have him on the line; it can be said that you called, that "
             "he can talk, that his phone has been busy). Never mentions a visit, a room or sitting."),
    'visit': ("IN PERSON: a family's living room when you arrive, or a sit-down in your office at the end of "
              "his campus visit (come in, have a seat, can I get you something, thanks for coming). Never "
              "mentions a phone call."),
}
for sk, sd in SETTING.items():
    for k, d in OPEN.items():
        add(f'recruit.open.{sk}.{k}', 'recruit', FN,
            f"The first words of a recruiting talk, from the recruit's side. THE TALK IS {sd} {d} No question "
            "about what he wants (that is another part), no mention of a subject.", 300, words=(2, 25))
CLOSE = {
    'warm': "It went well and he is warm: he thanks you, says he will think about it seriously, wants to talk "
            "again.",
    'flat': "It was fine, nothing more: he thanks you for your time and ends it politely.",
    'cool': "It did not go well: he ends it quickly and politely, maybe a little cold, leaves nothing open.",
}
for sk, sd in SETTING.items():
    for k, d in CLOSE.items():
        add(f'recruit.close.{sk}.{k}', 'recruit', FN,
            f"The last words of a recruiting talk, from the recruit's side. THE TALK IS {sd} {d} Never "
            "promises a decision date.", 250, words=(2, 25))

# ---------------------------------------------------------------------------
# R: your record at his position.

FR = FN | {'pos'}
add('recruit.record.ask', 'recruit', FR,
    "He asks you about your track record at his position: how your program has done with players at {pos}, who "
    "has played, been developed, been drafted. A league-minded recruit or his people checking if the claim is "
    "real. A question, not a statement; never states any result.", 450, words=(5, 30))
for k, d in {'strong': "your record at his position is strong: you have developed and sent players like him to "
             "the league, so he is impressed and says so",
             'mixed': "your record at his position is mixed: some success, some misses, so he is interested but "
                      "wants more",
             'weak': "your record at his position is weak: little to show for it, so he is unconvinced and "
                     "politely says so"}.items():
    add(f'recruit.record.{k}', 'recruit', FR,
        f"His reaction to your answer about your record at his position, when {d}. Never states a number or a "
        "name.", 330 if k != 'mixed' else 300, words=(3, 30))

# ---------------------------------------------------------------------------
# S: a pitch for one of your strengths.

ASPECTS = {
    'tradition': ("the program's tradition: its history, its name, the players who came through, what a jersey "
                  "means there"),
    'facilities': "the facilities: the weight room, the football building, the practice fields, the training "
                  "staff, the nutrition",
    'atmosphere': "the stadium and the game-day atmosphere: the crowd, the noise, the student section, a full "
                  "house on Saturdays",
    'brand': "the exposure: national TV games, the brand, being seen, the shoe and apparel deal",
    'conference': "the conference: playing the best teams every week, the schedule, the level of competition",
    'campus': "the campus and the town: life outside football, the college experience, the people, the place",
    'prestige': "the head coach's standing: his name, his reputation, who he has coached, who he knows",
}
for k, d in ASPECTS.items():
    add(f'recruit.aspect.{k}.good', 'recruit', FN,
        f"You pitched {d}. He CARES about it and your proof is strong: he is impressed and it moves him. "
        "He has been told about it, not shown: he has not visited, so no drive, no tour, no 'the building is "
        "nicer than I thought'. No stat, no name.", 160, words=(3, 30))
    add(f'recruit.aspect.{k}.cool', 'recruit', FN,
        f"You pitched {d}. It is NOT HIS THING: he does not care much about it, or he has heard it from every "
        "school. He has been told about it, not shown it: he has not visited. Polite, a little cool.", 90,
        words=(3, 30))
    add(f'recruit.aspect.{k}.weak', 'recruit', FN,
        f"You pitched {d}. He CARES about it, but your proof is WEAK: you oversold it, and he can tell. He has "
        "been told about it, not shown it: he has not visited. He wants more than the pitch, politely.", 110,
        words=(3, 30))

# ---------------------------------------------------------------------------
# B: his background and family situation, raised in his own way.

BG = {
    'money_tight': ("money is tight at home: the family worries about bills, cost, what a school would really "
                    "cost them, a job for a parent", 240, set()),
    'money_fine': ("money is not a worry at home: the family is comfortable, so money is not what moves "
                   "them, and they say so without bragging", 220, set()),
    'single_parent': ("he was raised by a single parent who has carried the family: a parent speaking is that "
                      "parent ('I raised him on my own', never a spouse); the recruit himself or a transfer may "
                      "say 'my mom'. Hard work, trust, someone who will look after him", 240, {'grandparent'}),
    'grandparents': ("his grandparents raised him: a grandparent speaking is one of them; the recruit himself or "
                     "a transfer may say 'my grandmother' or 'my grandparents'. Old-fashioned about trust, "
                     "church or family dinner, wary of strangers taking him away", 240, {'parent'}),
    'foster': ("he grew up in foster care and has a guardian: trust is earned slowly, he has moved before, "
               "stability matters more than anything", 220, {'grandparent'}),
    'job_loss': ("a parent lost a job this year: money got tight at home and the family is unsettled, so "
                 "security and cost weigh on the decision", 220, set()),
    'family_move': ("his family moved recently: a new town, a new school, a shaken routine, so he wants "
                    "something steady", 220, set()),
    'death': ("the family lost someone this year: grief is close, he wants to stay near home or at least be "
              "understood, the line carries the weight and never names who died", 220, set()),
    'new_baby': ("there is a new baby at home: the family has a lot on its plate and money is a little tighter, "
                 "so being near and supported matters", 220, set()),
    'mentor': ("a mentor, coach or older man outside the family keeps him straight and on track, and the "
               "family trusts him: they want to know you will work with that man", 220, set()),
    'nil_changed': ("NIL money has already changed the family's life, so a number has to be real and has to "
                    "last: they have done the math and know what a deal can do", 220, set()),
    'brother_follow': ("his older brother plays college football and he wants to follow him: a brother is named "
                       "by role (the recruit himself or a transfer says 'my brother', a parent, grandparent or "
                       "handler says 'his brother'). Same program, same stage, the brother's name opens doors "
                       "or weighs on him", 220, set()),
    'brother_own': ("his older brother plays college football somewhere, and he wants to make his own name "
                    "away from the brother's shadow: a brother is named by role (the recruit himself or a "
                    "transfer says 'my brother', a parent, grandparent or handler says 'his brother')", 220,
                    set()),
    'bloodline_nfl': ("football runs in the family and someone close played in the league: they have been "
                      "through this, they know what is real and what is a sales pitch (never says who)", 220,
                      set()),
    'bloodline_college': ("someone in the family played college football (never says who): they know what a "
                          "recruit visit is like and are hard to impress", 220, set()),
    'grades_low': ("his grades are a worry: will he qualify, will he keep up, what support there is for a player "
                   "who struggles in class", 220, set()),
    'ambitious': ("he is driven to reach the league and wants the fastest, best path there, not a comfortable "
                  "four years: he asks hard questions and measures everything against it", 220, set()),
}
for k, (d, n, drop) in BG.items():
    add(f'recruit.bg.{k}', 'recruit', FN,
        f"His side brings up something about their life, in their own way, unprompted or when asked how things "
        f"are at home: {d}. It is a fact of their life, said plainly and personally, never a speech. Never "
        "names a person, a school or an amount.", n, drop=drop, words=(4, 35))

# ---------------------------------------------------------------------------
# W: what he wants.

for k, d, n in [
    ('second', "after you answered his first question, he adds another thing he wants or needs to know", 280),
    ('dealbreaker', "he draws a line: one thing he will not accept (no real shot at playing, too far from "
                    "home, no honesty about the money, a staff that will not stay); firm, not rude", 280),
    ('unsure', "he does not know yet what matters most to him, or is still working it out; honest, a little "
               "young", 220),
]:
    add(f'recruit.want.{k}', 'recruit', FN, f"What he wants from his school: {d}.", n, words=(4, 30))

# ---------------------------------------------------------------------------
# E: what happened between you, and who else is in the picture.

EV = [
    ('history.first', "this is the first real talk you have had with him; he is meeting you fresh", 240),
    ('history.again', "you have talked before; he remembers it in general (never what was said) and picks up "
                      "from there", 240),
    ('history.silence', "it has been a while since you called him and he noticed, a little pointed or hurt, "
                        "never angry", 240),
    ('history.kept', "you kept your word to him before: he says so and it counts for you", 220),
    ('history.broke', "you did not keep your word to him before: he brings it up, hurt or wary, and wants a "
                      "straight answer", 220),
    ('other.money', "another school has put more money in front of him than you have; he says it, not to "
                    "bargain but because it is real", 250),
    ('other.you_money', "your money is the biggest he has seen, and he acknowledges it, though money is not "
                        "everything", 200),
    ('other.closer', "another school is much closer to home than you are, and the family has noticed", 220),
    ('other.start', "another school is promising him a starting job, and he asks how you compare", 220),
    ('committed.elsewhere', "he has already committed to another school, and you are calling anyway: he is "
                            "honest about it, polite, open or closed", 250),
    ('committed.you', "he has committed to you, and other schools keep calling him: he tells you where he "
                      "stands, loyal and a little pestered", 250),
    ('you.new', "you are new at your school: he wonders how long you will be there and what changes", 220),
    ('you.long', "you have been at your school for years: that steadiness counts with him", 200),
    ('team.good_year', "your team had a good year last season ({last_record} is the record): he mentions it "
                       "without stating the number", 220),
    ('team.bad_year', "your team had a rough year last season ({last_record} is the record): he brings it up "
                      "honestly, without stating the number", 220),
    ('visit.liked', "he has been to your campus on a visit and liked it: he talks about how it felt, never "
                    "about what he saw", 220),
    ('visit.meh', "he has been to your campus on a visit and it left him cool: he says so politely, never "
                  "about what he saw", 200),
]
for k, d, n in EV:
    f = FN | ({'last_record'} if 'year' in k else set())
    add(f'recruit.{k}', 'recruit', f, f"What happened, from his side: {d}.", n, words=(4, 35))

# ---------------------------------------------------------------------------
# T: a transfer in the portal explains himself (his own words, at college).

TWHY = {
    'snaps': "he left because he was not playing: the snaps were not there, he sat, and he wants a real shot",
    'coach': "he left because his coach left or the staff changed, and it was not the same place afterward",
    'money': "he left partly for the money: his old school could not or would not match what he is worth",
    'up': "he left to move up a level and play against better teams, to be seen by the league",
    'choice': "he left because he wanted a fresh start, a different place and a new set of people",
    'grad': "he finished his degree and has a year left to play, so he is a graduate transfer looking for the "
            "right place for it",
}
for k, d in TWHY.items():
    add(f'transfer.why.{k}', 'player', set(),
        f"You asked why he is in the portal and he explains himself plainly: {d}. About the move, never about "
        "your school. Never names his old school or coach.", 250, drop=TR_DROP, words=(5, 35))
for k, d in {'starter': "he was a starter at his old school and expects to compete for a job here",
             'backup': "he was a backup at his old school and wants to prove himself with real snaps"}.items():
    add(f'transfer.role.{k}', 'player', set(),
        f"He tells you where he stood at his old school: {d}. Plain, a little proud or a little hurt. Never "
        "names a school, a game or a stat.", 220, drop=TR_DROP, words=(4, 30))


# What a part can be about, so the requests of one part write different lines instead of the same few ideas many
# ways (the first trial: 73 of 100 money lines were checker-flagged as the same idea). Each request is handed
# three of them in turn.
FACETS = {
    'recruit.record.ask': ["how many of your players at his position have been drafted", "who has started "
        "early and who sat", "what your position coach does with a player like him", "whether the players you "
        "sell him on stayed or left", "a family checking a claim before believing it", "how your position "
        "players did against the best teams", "a plain 'prove it'", "what happened to the last one at his spot"],
    'recruit.record.strong': ["impressed but careful not to gush", "that is what he came to hear", "asks the "
        "next question, now serious", "tells you it puts you ahead of other schools", "wants to meet that "
        "position coach", "a parent relieved", "a transfer who has seen the other kind", "quiet respect"],
    'recruit.record.mixed': ["some good, some not, he wants the whole picture", "asks what was different about "
        "the ones who made it", "interested but not sold", "asks about the misses", "weighs it against another "
        "school", "a parent splitting the difference", "wants to hear it from a former player", "politely "
        "noncommittal"],
    'recruit.record.weak': ["unconvinced, says so politely", "asks what would be different for him", "points "
        "out how little there is to show", "worried he would be the experiment", "wants someone who has done "
        "it before", "a parent protective of his future", "a transfer who cannot afford another miss",
        "honest disappointment"],
    'recruit.bg.money_tight': ["the bills and the rent", "a parent working extra or two jobs", "what a trip, "
        "gear or a visit costs them", "what a scholarship covers and what it leaves", "a real number he could "
        "send home", "not wanting him in debt", "pride: they don't ask for help", "worry about being taken "
        "advantage of", "his plan to help his family", "small daily things money decides"],
    'recruit.bg.money_fine': ["money is not what decides it", "they can afford to be picky", "wanting him to "
        "earn what he gets", "no need for the collective's money", "caution about a school that leans on money",
        "a family that has seen money spoil kids", "they ask what else you have", "comfortable, says so without "
        "bragging"],
    'recruit.bg.single_parent': ["one person carrying everything", "a parent who rarely sits down", "trust "
        "is hard to give", "who will look after him when she cannot", "he has been the man of the house", "a "
        "parent wary of being sold to", "pride in how he turned out", "not wanting to be far away",
        "juggling work and his games"],
    'recruit.bg.grandparents': ["raised in an old house with old rules", "church and Sunday dinner", "a "
        "grandparent who does not travel", "wary of strangers taking him away", "health and age: how long they "
        "have", "being asked to trust a coach they met once", "pride and worry together", "a house where "
        "football is watched on TV every week"],
    'recruit.bg.foster': ["moved around before and does not unpack easily", "needs someone to stay put",
        "slow to trust adults", "a guardian who has fought for him", "wants a place that feels like a "
        "home", "afraid of being cut loose", "asks what happens if it does not work out",
        "stability over everything"],
    'recruit.bg.job_loss': ["a parent out of work", "hours cut and bills piling", "the family unsettled, "
        "trying not to show it", "security over glamour", "how the school helps with costs", "pride about not "
        "asking", "he feels responsible", "a steady paycheck for someone at home"],
    'recruit.bg.family_move': ["a new town and a new school", "friends left behind", "a routine that has not "
        "settled", "wants one steady thing", "does not want to move again for football", "adjusting, "
        "tired of starting over", "the house still in boxes", "home no longer feels like home"],
    'recruit.bg.death': ["a loss still fresh", "wants to stay close to home", "football as the thing that "
        "got him through", "asks you to understand, not pity", "the house is quieter now", "not ready to "
        "talk about it, but it is there", "a family leaning on each other", "wants someone who will be human "
        "with him"],
    'recruit.bg.new_baby': ["a new baby changed everything at home", "no sleep and a tight budget", "wants "
        "to be near to help", "a house full of new worries", "pride in the new addition", "visits are hard to "
        "plan around it", "money a little tighter", "the family's attention is split"],
    'recruit.bg.mentor': ["a man outside the family keeps him straight", "he'd have to be part of the "
        "decision", "a coach who has been there since he was small", "they want the mentor to keep in touch "
        "with you", "trust in him comes before trust in you", "he kept him out of trouble", "a mentor who "
        "gives a straight read", "what happens to that relationship if he leaves"],
    'recruit.bg.nil_changed': ["NIL money already changed how the family lives", "a number that has to be "
        "real and last", "they have done the math", "wary of a deal that disappears", "a family that grew up "
        "without it and knows its value", "they have a lawyer or an advisor look at it", "afraid it makes "
        "people treat him differently", "grateful and careful"],
    'recruit.bg.brother_follow': ["wants to follow his brother", "the brother's name opens doors", "the "
        "brother's advice and warnings", "playing on the same stage or program", "the family watched one son "
        "and wants to watch the other", "pressure of being the next one", "the brother told him what it is "
        "really like", "a brother who calls every week"],
    'recruit.bg.brother_own': ["tired of being compared to his brother", "wants his own name", "the "
        "brother's shadow", "somewhere the brother has not been", "love for the brother but wants space",
        "a parent who sees the difference between the two", "what he would prove", "the brother's "
        "opinion is not his"],
    'recruit.bg.bloodline_nfl': ["someone close played in the league and told them what is real", "knows "
        "what a recruiting pitch sounds like", "hard to impress", "football runs in the family", "the "
        "shadow of what someone else did", "has heard the horror stories and the good ones", "expects "
        "honesty about the business", "wants the real path, not the brochure"],
    'recruit.bg.bloodline_college': ["someone in the family played college ball", "they know how recruiting "
        "works", "they know what the weight room costs", "have seen the good and bad of it", "unmoved by "
        "the gloss", "asks the kind of question only an insider asks", "family pride, careful hope",
        "has seen a coach leave"],
    'recruit.bg.grades_low': ["worry about qualifying", "classes have always been a fight", "what tutoring "
        "and support exist", "a parent who has been on him about it", "he is embarrassed about it", "asks "
        "what happens if he falls behind", "wants someone who will not give up on him in class", "a plan to "
        "get it done"],
    'recruit.bg.ambitious': ["only the league matters", "four years is a means, not an end", "asks how fast "
        "he can get on the field", "compares every school to the best path", "has a plan for the draft",
        "impatient with comfortable answers", "works harder than people know", "asks what you do for the "
        "guys who make it"],
    'recruit.want.second': ["another thing he needs to know", "something he forgot to ask", "a smaller worry "
        "that matters to him", "a question about people, not football", "a practical question (housing, "
        "travel, food)", "something he has been told elsewhere", "a family concern", "what happens if it "
        "does not work out"],
    'recruit.want.dealbreaker': ["no real shot at playing", "too far from home", "no honesty about money",
        "a staff that will not stay", "not being treated like a person", "a school that ignores class",
        "promises that don't hold", "a program that does not develop players"],
    'recruit.want.unsure': ["still working it out", "everybody tells him something different", "wants to "
        "see more before he knows", "does not want to rush", "unsure what he should care about", "a parent "
        "who wants him to take his time", "afraid of choosing wrong", "honest and a little young"],
    'recruit.history.first': ["meeting you for the first time", "curious, sizing you up", "has heard of you "
        "but never talked", "polite and careful", "heard about your program from someone", "a parent who "
        "wants to see what kind of man you are", "has talked to other schools already", "first impression "
        "matters"],
    'recruit.history.again': ["picking up where you left off", "remembers it was a good talk", "a new "
        "question since last time", "wants to follow up", "has been thinking about it", "a parent who "
        "discussed it at home", "the second talk is where it gets real", "glad you kept in touch"],
    'recruit.history.silence': ["it's been a while and he noticed", "other schools called more", "hurt, "
        "not angry", "wonders if he's still a priority", "a parent keeping score", "wry and pointed",
        "gives you a chance to explain", "wants it to be different now"],
    'recruit.history.kept': ["you did what you said", "trust is built", "that mattered to the family",
        "it is rare, he says", "he tells other schools that", "a parent who noticed", "now he listens "
        "differently", "keep it up, he says"],
    'recruit.history.broke': ["you said one thing and it went another way", "he wants a straight answer", "hurt "
        "and wary", "a parent who remembers", "trust is hard to rebuild", "he doesn't want an excuse",
        "gives you one more chance", "other schools made the same promise"],
    'recruit.other.money': ["another school's number is bigger", "he says it plainly", "not bargaining, just "
        "honest", "wonders if yours can move", "money is not everything but it is something", "a parent "
        "doing the math", "he doesn't want a bidding war", "tells you what a bigger offer means"],
    'recruit.other.you_money': ["your number is the biggest he has seen", "he acknowledges it", "money is "
        "not everything though", "wonders if it lasts", "asks what is expected for it", "a parent surprised",
        "does not want to be bought", "grateful and cautious"],
    'recruit.other.closer': ["another school is a short drive from home", "family could see every game",
        "distance weighs on them", "a parent who does not like to fly", "homesick already", "asks about how "
        "you would get them there", "a closer school is hard to beat", "honest about the pull of home"],
    'recruit.other.start': ["another school promised him a starting job", "he asks how you compare", "has "
        "heard promises before", "wants it in writing, not words", "a parent wary of a promise", "not "
        "sure which school means it", "asks what you can honestly offer", "playing time is what counts"],
    'recruit.committed.elsewhere': ["he is committed and honest about it", "polite, still listening",
        "loyal to the school he picked", "open to hearing a pitch", "closing the door kindly", "a parent "
        "who respects the call", "tells you what would have to change", "has not told anyone he is wavering"],
    'recruit.committed.you': ["he is committed to you", "other schools keep calling", "tired of the calls",
        "wants to know you still want him", "reassures you", "wants you to keep in touch", "a parent who "
        "worries about a flip", "shares what the others are offering"],
    'recruit.you.new': ["wonders how long you'll be here", "what changes with a new coach", "who is on your "
        "staff", "did you bring your scheme", "a parent who's seen coaches leave", "wants a straight answer "
        "about your contract", "excited about a fresh start", "cautious about a new regime"],
    'recruit.you.long': ["you've been here a long time", "stability counts", "he knows what he's getting",
        "players you've coached are doing well", "a parent likes the continuity", "asks about your best "
        "memory", "a coach who has stayed through losing years", "trust earned over time"],
    'recruit.team.good_year': ["your team won and he noticed", "wants to be part of a winning team", "asks "
        "what makes it work", "excited but careful", "heard about the season", "a parent who watched "
        "your games", "wonders if you can keep it going", "congratulates you"],
    'recruit.team.bad_year': ["a rough season and he knows it", "asks what went wrong", "wants to hear the "
        "plan", "willing to be part of the turnaround", "a parent worried about losing", "honest, not "
        "rude", "he wants to win", "a bad year isn't a deal-breaker"],
    'recruit.visit.liked': ["the visit felt right", "met people he liked", "felt like family", "the staff "
        "were real", "a parent who came along and liked it", "he can picture himself there", "something "
        "small stuck with him", "the weekend left an impression"],
    'recruit.visit.meh': ["the visit felt flat", "it did not feel like home", "he wanted more time with the "
        "coaches", "a parent who had reservations", "nothing wrong, nothing special", "he expected more",
        "wasn't sure about the people", "politely honest"],
    'transfer.why.snaps': ["sat behind the same guy", "the depth chart never moved", "told he'd get a shot "
        "and didn't", "wants to play on Saturdays", "tired of scout team", "practice squad energy on a "
        "college team", "a coach who played favorites", "he's proven he can play"],
    'transfer.why.coach': ["the coach who recruited him left", "a new staff with new guys", "different "
        "scheme, no spot", "the relationship was gone", "loyalty was to a man, not a place", "the "
        "locker room changed", "the new coach never talked to him", "wants someone who wants him"],
    'transfer.why.money': ["the deal wasn't there", "other places offered more", "his old school couldn't "
        "match what he's worth", "he has a family to help", "money matters at his age", "honest about "
        "wanting it", "doesn't want it to be the only reason", "market value"],
    'transfer.why.up': ["wants to play on a bigger stage", "better competition", "scouts see him there",
        "the league is the goal", "outgrew his old level", "wants to see how good he is", "ambition, not "
        "bitterness", "a step up, nothing more"],
    'transfer.why.choice': ["fresh start", "just wanted a change", "the place wasn't for him", "no drama, "
        "a decision", "needed new people around him", "something small added up", "ready for something "
        "different", "grew apart from the place"],
    'transfer.why.grad': ["finished his degree", "one year left to play", "wants the right place for it",
        "knows what he's looking for", "a grad transfer with a clear plan", "wants to finish on his "
        "terms", "his old school is behind him", "plays hard, leaves clean"],
    'transfer.role.starter': ["started and wants to keep starting", "earned his spot", "proud of what he did "
        "there", "a captain or leader there", "expects to compete, not coast", "wonders what your depth "
        "chart looks like", "knows what he can do", "isn't taking a step back"],
    'transfer.role.backup': ["waited his turn", "wants real snaps", "practiced hard and never got his shot",
        "knows he's better than the depth chart", "some resentment, some humility", "ready for a chance",
        "stayed ready", "needs a place where he's seen"],
}
for _a in ASPECTS:
    FACETS[f'recruit.aspect.{_a}.good'] = [f"what that strength means to a player day to day", "a parent "
        "thinking about his future", "what other schools lack", "a transfer who has seen worse", "it "
        "matters more than he expected", "a quiet nod", "he repeats it to himself", "wants to see it"]
    FACETS[f'recruit.aspect.{_a}.weak'] = ["he cares and says the pitch fell short", "asks for proof, not words",
        "other schools have more of it", "a parent who sees through the pitch", "a transfer who has been "
        "oversold before", "polite but unconvinced", "tells you what would convince him", "it matters to "
        "him, and that's why he's picky"]
    FACETS[f'recruit.aspect.{_a}.cool'] = ["it's not what he cares about", "every school says the same", "a "
        "parent unmoved", "a transfer who has heard it before", "polite but flat", "he'd trade it for "
        "something real", "waiting for the next point", "unconvinced it's that good"]


# More facets for the top-up (Cain 2026-11-13): the first run's facets were moods; these are concrete things a
# real person in the spot would bring up, so a narrow part stops coming back as near copies. A request takes three
# of a part's facets, so more of them means more different requests.
ASPECT_DETAILS = {
    'tradition': ["the history and who played there before him", "alumni who still come back", "a name on the wall that "
        "means something", "old rivalries and what they mean to the town", "a program that has been through "
        "hard years and stayed itself", "tradition as pressure, not just pride"],
    'facilities': ["the weight room and what it does for a body in two years", "the training staff and getting hurt",
        "the nutrition and the food, day to day", "the practice fields and the film room", "the football building as "
        "a place he'd live in", "whether the staff keeps up what the building promises"],
    'atmosphere': ["game days and the noise", "the student section and the crowd", "a town that shows up",
        "playing in front of people who care", "what a loud place does to a visiting team", "the feeling on a Friday "
        "night or a Saturday afternoon"],
    'brand': ["being on TV every week", "the name on a jersey opening doors", "exposure for the draft", "the "
        "gear, the social media, the look of the program", "a brand that helps NIL deals", "a name people know "
        "when he goes home"],
    'conference': ["playing the best teams every week", "the level of competition he would face", "the league's "
        "reputation with scouts", "a schedule that tests him", "the travel and the road games", "a conference "
        "that wins the games that count"],
    'campus': ["the campus as a place to live, not just play", "the town around it", "housing and where he'd stay "
        "as a freshman", "being around regular students", "how far it feels from home", "a place he could finish a "
        "degree and stay"],
    'prestige': ["the head coach's name and who he has worked for", "players he has sent to the league", "what his "
        "word is worth", "whether he keeps his staff together", "his reputation with families", "a coach other "
        "coaches respect"],
}
for _a, _d in ASPECT_DETAILS.items():
    for _m in ('good', 'weak', 'cool'):
        FACETS[f'recruit.aspect.{_a}.{_m}'] = FACETS.get(f'recruit.aspect.{_a}.{_m}', []) + _d

MORE = {
    'recruit.record.ask': ["asks for the last few players at his position and where they are now", "how many "
        "left early and how many stayed four years", "whether freshmen at his position ever play", "how often "
        "the position coach makes the same player better in year two", "who coaches the position and how long "
        "he has been there", "what the school did when a player at his spot got hurt", "whether the best at the "
        "spot were developed or just recruited", "compares you to what another school told him about the position",
        "a parent who wants the honest numbers, not the sizzle", "asks what the room looks like behind and ahead "
        "of him"],
    'recruit.record.strong': ["relieved the answer is real", "now asks who coaches the room", "says it beats "
        "what the others told him", "a handler nodding at the pipeline", "a parent starting to picture it", "asks "
        "how a freshman gets into that line", "tells you he wants to be next", "keeps his guard up anyway"],
    'recruit.record.mixed': ["asks what the misses had in common", "wants to hear from a player, not a coach",
        "likes the honesty of a mixed answer", "weighs that against a cleaner pitch", "a parent asking if his "
        "son would be a hit or a miss", "asks what you changed", "says nobody has a perfect record", "interested "
        "but keeps asking"],
    'recruit.record.weak': ["tells you he was hoping for more", "asks what you would do differently for him",
        "says he does not want to be the one who turns it around", "a parent asking about the other schools' "
        "records at his spot", "politely says he has options", "asks if the staff there is changing", "a transfer "
        "who cannot afford another stalled year", "asks how you can fix the room"],
    'recruit.open.call.polite': ["a plain hello and he waits for you", "thanks you for calling without gushing",
        "a short pleasantry, then silence", "formal and respectful",
        "relaxed and chatty for a moment", "a parent who answers like a business call", "a transfer who gets "
        "straight to it", "a little dry humor about the phone ringing", "polite but giving you nothing yet",
        "asks how you are before anything else", "cheerful in a careful way"],
    'recruit.open.call.guarded': ["short answers, waiting to see what you want", "says he has a few minutes",
        "reminds you he has not decided anything", "a parent who screens calls and says so", "wary of being "
        "sold to", "has been burned by calls before", "polite wall", "says he will listen but not promise",
        "a transfer who has done this before", "keeps it brief and flat"],
    'recruit.open.call.warm': ["glad it is you", "has been waiting to hear from you", "laughs easily", "a parent "
        "who sounds genuinely pleased", "says the family likes you", "a quick joke before business", "a transfer "
        "who is open and friendly", "he sounds relaxed", "tells you he has good news to share", "sincere and "
        "unhurried"],
    'recruit.open.visit.polite': ["a handshake and a measured welcome", "offers you something to drink",
        "a parent who sets the tone at the door", "a handler who has a plan for the conversation", "seats you and "
        "waits for you to start", "a grandparent taking his time", "formal and respectful", "a quick, polite "
        "check on how your trip was", "keeps the room calm", "a family that rehearsed this"],
    'recruit.open.visit.guarded': ["arms crossed, but civil", "lets you do the talking first", "says they are "
        "hearing from other schools too", "a parent who watches how you carry yourself", "reminds you this is "
        "just a conversation", "polite wall", "tells you what they will and will not discuss", "cautious about "
        "who is in the room"],
    'recruit.open.visit.warm': ["makes you feel at home", "a hug or a big handshake", "offers a meal", "family "
        "all in the room and glad", "a parent who is clearly pleased", "jokes with you right away", "the "
        "grandparent has been waiting to meet you", "an easy start", "he sounds proud to have you there"],
    'recruit.close.call.flat': ["thanks you and moves on", "says he will think about it", "a parent says they "
        "will talk it over", "a short goodbye", "no promise either way", "polite, nothing more", "tells you to "
        "keep him posted", "a transfer who ends it quickly", "wishes you luck this weekend", "leaves the door "
        "open without pushing it"],
    'recruit.close.call.cool': ["a clipped goodbye", "tells you he has other calls", "ends it politely and "
        "quickly", "a parent who closes the call firmly", "says they will reach out if anything changes", "no "
        "warmth left", "a transfer who has heard enough", "thanks you for your time and hangs up", "does not "
        "ask you to call again"],
    'recruit.close.call.warm': ["tells you to call again soon", "says he enjoyed it", "a parent says the family "
        "will talk tonight", "thanks you genuinely", "wishes you luck", "a joke on the way out", "asks you to "
        "send him something", "says you are high on the list", "a transfer who says you are in the mix"],
    'recruit.close.visit.warm': ["walks you to the door", "says come back anytime", "a parent says thank you "
        "for coming", "hands you something to take with you", "the grandparent hugs you", "tells you the "
        "family will talk tonight", "jokes about the car ride", "a real goodbye, not a script", "says you made "
        "the day easy", "he shakes your hand and means it"],
    'recruit.close.visit.flat': ["thanks you for coming", "stays seated", "says they will be in touch", "a "
        "polite handshake", "says they have other visits", "a parent notes they will think it over", "nothing "
        "decided", "no follow-up question", "tells you it was fine", "keeps it formal on the way out"],
    'recruit.close.visit.cool': ["shows you out", "tells you not to expect an answer soon", "polite and "
        "distant", "says they have a lot to think about", "no invitation to come back", "a parent ends it "
        "quickly", "a flat thank you", "thanks you for stopping by and nothing else"],
    'recruit.history.again': ["picks up the thread from last time", "says he told his people about it", "a "
        "question that came to him after", "wonders if anything has changed", "a parent has an update from home",
        "says he kept thinking about one thing you said", "tells you what he heard from others since", "is "
        "friendlier the second time", "asks if you remembered", "starts right where the last talk ended"],
    'recruit.history.broke': ["reminds you what was said and what happened", "says he does not need an "
        "apology, just a straight answer", "a parent who kept the receipts in her head", "tells you it cost "
        "you with the family", "asks what is different now", "says he has seen this before", "cooler than the "
        "first time", "a transfer who has been lied to by coaches", "offers one more chance"],
    'recruit.bg.grandparents': ["the grandparents raised him since he was small", "a grandmother who prays for "
        "him before games", "a grandfather who does not trust phones", "they cannot travel far", "he lives in "
        "their house", "they have never met a coach before", "he wants them to see him play", "they worry about "
        "who will cook for him", "the house rules still apply"],
    'recruit.bg.brother_follow': ["he wants to wear what his brother wore", "the brother says the same thing "
        "about this school", "he watched his brother's games all his life", "his brother weighs in on the "
        "choice", "he grew up in the stands for his brother", "does not want to be second to him",
        "brother as a test: if it was good enough for him", "wants to be on the same field one day"],
    'recruit.bg.brother_own': ["he is tired of being called someone's little brother", "wants a different "
        "number than his brother", "the brother's shadow", "wants a school where nobody knows his brother",
        "proud of him but ready to be his own man", "people ask about the brother first", "he has his own "
        "style", "wants to be judged on his own film"],
    'recruit.bg.foster': ["moved houses more than once", "a guardian who is trying her best", "he keeps a "
        "small bag packed", "has learned to wait and watch", "not used to adults who stay", "wants a school "
        "that checks in", "a case worker in his past", "needs a place to be a kid", "afraid to let people in"],
    'recruit.bg.family_move': ["the family just moved for work", "a new school mid-year", "he has not made "
        "friends yet", "home is a rental now", "he left his old team", "boxes still in the hall", "the whole "
        "family is adjusting", "wants to put roots down", "does not want to uproot again"],
    'recruit.bg.money_tight': ["a trip to visit costs a week of groceries", "one car for the whole family",
        "a parent covering a double shift", "scholarship paperwork and what it leaves out", "the rent due "
        "before the season", "he wants to send money home", "cleats, gear and fees add up", "no cushion if "
        "something breaks", "a family that does not talk about it"],
    'recruit.bg.death': ["someone close passed away", "football got him through the winter", "the family "
        "is still grieving", "he plays for someone", "wants to stay close to the people left", "quiet about "
        "it, but it shows", "a parent who cannot say the name yet", "he needs a coach who can be gentle"],
    'recruit.bg.new_baby': ["a baby in the house", "a parent up all night", "diapers and bills", "he wants to "
        "be around for the baby", "a sibling just born", "the family is running on no sleep", "he rocks the baby "
        "to sleep after practice", "the new addition changed the schedule"],
    'recruit.bg.mentor': ["a coach who drove him to games when no one else could", "an older man at the gym "
        "who set him straight", "a pastor or a teacher who watches over him", "his mentor must meet you", "he "
        "has been at his side since eighth grade", "a voice who tells him no", "a mentor who knows the "
        "business of recruiting", "someone who spoke for him when he had no one"],
    'recruit.bg.ambitious': ["he writes his goals on the wall", "he trains before school", "asks what the "
        "fastest route is", "does not want to be comfortable", "compares every offer to the draft", "he reads "
        "about prospects", "has told everyone he is going pro", "impatient with the long road"],
    'recruit.bg.bloodline_college': ["his father played college ball", "an uncle who played in college", "an "
        "older cousin who played", "grew up around campus football", "they know how a weight room feels at "
        "five a.m.", "someone who knows both sides of recruiting", "has heard the stories about injuries"],
    'recruit.bg.bloodline_nfl': ["a relative played in the league", "grew up around a locker room", "knows "
        "how fast the career ends", "someone who tells him to protect his body", "football has paid their "
        "bills before", "expects real talk about the business", "knows what scouts look for"],
    'recruit.bg.grades_low': ["he hides his report card", "a teacher who believes in him", "summer school in "
        "his past", "a parent who checks his homework", "he is afraid of the qualifying test", "needs a tutor "
        "who is patient", "the first-generation college worry", "wants a plan, not a lecture"],
    'recruit.bg.job_loss': ["a parent laid off this year", "the plant closed", "benefits gone", "a parent job "
        "hunting every day", "the family cut back", "he took a job after practice", "does not want to add "
        "pressure at home", "wants the school to help with a job for a parent"],
    'recruit.bg.single_parent': ["one parent doing it alone", "she works two jobs", "he is the man of the "
        "house", "she rarely sat down at his games", "has never missed a game anyway", "trust is hard to give",
        "wants someone who will answer her calls", "she wants to meet every coach herself"],
    'recruit.bg.money_fine': ["the family is comfortable", "money will not decide it", "they want him to earn it",
        "do not need a handout", "suspicious of a school that talks money first", "could pay their own way",
        "asks what else the school offers", "a family that stays modest"],
    'recruit.bg.nil_changed': ["a deal changed how they live", "the family bought a house with it", "they know "
        "it can vanish", "a lawyer reads every page", "he does not want it to change him", "his friends "
        "treat him differently now", "they ask who handles the contract"],
    'recruit.want.unsure': ["has a list that keeps changing", "asks you what other recruits care about",
        "doesn't know if he wants big or small", "tells you he is overwhelmed",
        "wants a school that feels right, not looks right", "asks what you would do in his place", "has a "
        "different answer every day"],
    'recruit.want.second': ["asks about housing and roommates", "asks about class schedule and practice time",
        "asks what a normal week looks like", "wants to know about the team's food", "asks about family "
        "visits and trips home", "asks how coaches handle injuries", "asks how the team treats freshmen",
        "asks about the strength coach", "asks who he would be competing with"],
    'recruit.want.dealbreaker': ["a coach who lies", "being cut after one bad year", "a school that treats "
        "players like a number", "no path to a degree", "a staff that changes every year", "too much "
        "partying", "being asked to change positions", "no communication with the family"],
    'recruit.other.money': ["another number is bigger", "says it flat, not as a threat", "a parent comparing "
        "the numbers", "asks if you can match it", "says money is not why he is talking to you", "a "
        "collective offered more", "a handler negotiating", "wonders what the money means for him"],
    'recruit.other.closer': ["another school is an hour away", "his grandmother could come to every game",
        "his mother hates flying", "homesick before he leaves", "his friends are near the other school",
        "a short drive home on weekends", "the family can see him play in person", "asks how you would make "
        "up the distance"],
    'recruit.other.start': ["another coach said first team from day one", "he asks what you can say", "has "
        "been promised before", "a parent wants it in writing", "knows promises are cheap", "asks how often "
        "freshmen play for you", "says playing time is everything", "wants to compete, not be handed it"],
    'recruit.other.you_money': ["your offer is the biggest he has heard", "is stunned and says so", "wonders "
        "what it costs him", "asks what the catch is", "says he does not want to be bought", "a parent "
        "asking who pays", "a handler checking the terms", "grateful but guarded"],
    'recruit.team.good_year': ["he watched your games", "his friends talk about your season", "wants to be on "
        "a winner", "asks what changed", "a parent says the team looks like a program on the rise", "asks if "
        "the staff is staying together", "says it helps your case", "wonders if one good year is enough"],
    'recruit.team.bad_year': ["he saw the losses", "asks what went wrong", "a parent worried about wasting "
        "his years", "says one bad year does not scare him", "wants to hear the fix", "asks about the staff "
        "changes", "does not want to be part of a rebuild", "asks what the plan is to win"],
    'recruit.you.new': ["asks how long you plan to stay", "wonders what changed with your arrival", "a parent "
        "who has seen coaches leave", "asks if you kept the old staff", "wants to know your plan for the "
        "first year", "curious but cautious", "asks if your system is new to the players", "wants to meet the "
        "rest of the staff"],
    'recruit.you.long': ["you have been here since before he started high school", "stability matters to the "
        "family", "players speak well of you", "asks what keeps you at the same place", "a parent trusts a "
        "coach who stays", "he knows what he is getting", "a long run counts for something"],
    'recruit.committed.elsewhere': ["says he gave his word", "tells you it was a hard choice", "wants you to "
        "hear it from him", "says he will still listen", "asks you not to take it personally", "a parent "
        "respects his decision", "says he would only change for a big reason", "wishes you luck", "a transfer "
        "who already decided"],
    'recruit.committed.you': ["says he is yours", "tells you other coaches are still calling", "asks you to "
        "keep him close", "says he is not going anywhere", "a parent worried about a flip", "wants to hear "
        "you still want him", "has stopped taking other calls", "tells you what he is hearing from the others"],
    'recruit.visit.liked': ["the staff treated him like a person", "he met the players and liked them",
        "felt at home right away", "a parent liked the other families", "he can picture himself in the "
        "locker room", "it was better than he expected", "the food was a small thing that stuck", "he "
        "talked about it all the way home"],
    'recruit.visit.meh': ["it felt like every other visit", "he expected more time with the coaches",
        "nothing wrong, nothing special", "a parent was not sold", "he could not picture himself there",
        "it felt rushed", "he liked the town more than the program", "politely says it was fine"],
    'transfer.why.money': ["the deal at his old school was not there", "he found out others got more", "he "
        "helps support family", "a collective that did not pay what it promised", "wants a fair market price",
        "says it is business", "does not want money to be the only reason", "paid less than a teammate"],
    'transfer.why.grad': ["finished his degree early", "one year of eligibility left", "wants a different "
        "setting for his last year", "a graduate program he wants", "knows exactly what he needs", "ready to "
        "play out his last season", "the old school moved on", "wants to leave on his terms"],
    'transfer.why.coach': ["the coach who recruited him left", "a new staff did not know him", "a coach "
        "changed the scheme", "the position coach was fired", "his relationship with the head coach cooled",
        "new coaches brought their own guys", "loyalty was to a man", "he never got a call from the new staff"],
    'transfer.why.choice': ["it was just time", "the place did not fit", "no drama", "needed a new start",
        "his life changed", "a fresh set of people", "homesick for something else", "wanted to find his own place"],
    'transfer.why.snaps': ["same starter, same bench", "three years and no real snaps", "got promised a "
        "shot, never came", "tired of scout team", "the depth chart never moved", "wants Saturdays, not "
        "Sundays on the sideline", "a freshman got ahead", "knows he is better than where he sat"],
    'transfer.why.up': ["wants a higher level", "tougher competition", "more eyes from scouts", "outgrew his "
        "old league", "wants to test himself", "the league is the goal", "a bigger stage", "no bad blood"],
    'transfer.role.backup': ["waited his turn", "watched from the sideline", "played special teams", "the "
        "scout team all year", "got a few snaps late", "was one injury from starting", "practice player of "
        "the week", "knows he can play"],
    'transfer.role.starter': ["started most of his games", "was a captain", "earned the job", "led the room",
        "wants the same role here", "doesn't want to compete from scratch", "proud of his film", "asks "
        "about the depth chart"],
    'portal.react.good': ["that's what he wanted to hear", "a little relieved", "asks the next question",
        "tells you it moves you up", "sounds more interested", "says he'll talk about it with his people",
        "impressed but measured", "a real smile in the voice", "says it beats other pitches"],
    'portal.react.flat': ["nods, noncommittal", "says he has heard that", "asks a follow-up that tests it",
        "unmoved but polite", "says he'll think about it", "shrugs it off", "says everybody says that",
        "waits for something more"],
    'portal.react.bad': ["not what he hoped", "tells you it's a concern", "gets quiet", "says it's a "
        "problem for him", "asks if you can do better", "is blunt about it", "his tone cools"],
    'portal.ask.nil': ["asks what players at his position make", "asks how the collective works",
        "wants to know if it's guaranteed", "asks who handles the paperwork", "asks about bonuses for "
        "starting", "asks how NIL changes if he sits", "wants it in writing", "asks how it compares to "
        "other schools"],
    'portal.ask.distance': ["asks how far family would travel", "wants to be closer to home", "asks about "
        "flights", "family can't drive far", "he's far from home already", "asks if home games are "
        "reachable", "his grandmother can't fly", "tired of long trips"],
    'portal.tell': ["says what matters most, plain", "lists his priorities fast", "gives his first and "
        "second like a checklist", "says what he lost sleep over", "is blunt about what he wants", "says what "
        "he's been told by others", "hesitates, then says it", "says what the transfer taught him"],
    'portal.promise.believes': ["trusts you, with a warning", "says he'll hold you to it", "relieved", "wants "
        "it to be true", "thanks you for being straight", "says his word is good too", "tells you he'll work "
        "for it", "feels respected"],
    'portal.ask.family': ["asks how the staff treats the guys", "wants to know if coaches talk to the "
        "players' families", "asks about the locker room", "asks about team dinners", "wants a coach who "
        "answers the phone", "asks if guys look out for each other", "asks how coaches handle bad days"],
}
for _k, _v in MORE.items():
    FACETS[_k] = FACETS.get(_k, []) + _v


def total():
    return sum(TARGETS.values())


WHO = {
    'recruit': 'both sides: a high school recruit\'s family (parent, grandparent, handler, himself) and a '
               'transfer (a college player)',
    'player': 'a transfer only (a college player, decides for himself)',
}


def doc(done=None):
    """The part list as a table (afm/recruit20k/parts.md in the project folder): who says each key, what it takes
    as true, the facts its lines may name, its target and (once the run is merged) what it holds."""
    out = ['# The recruiting talks run: every part, who says it and what it takes as true', '',
           'Written 2026-11-12 for the talk design (F1) and the build lane that gates each key on its fact. '
           'Source: `tools/linebank/recruit20k.py`. Every key is said only when the game has checked the fact it is '
           'named for; a line asserts that fact and nothing else stored about him (no family shape, siblings, jobs, '
           'grades, offers or record unless the key is that fact). Untagged lines fit both sides; a line tagged for '
           'one side (`parent` `grandparent` `handler` `himself` `loud` `online` `absent` for a family, `college` '
           '+ position, personality or stage for a transfer) is said only by that side. No line names a school, a '
           'coach, a number or a day.', '',
           '| Key | Who says it | Takes as true | Facts its lines may name | Target |' +
           (' Written |' if done else ''), '|---|---|---|---|---|' + ('---|' if done else '')]
    for k in sorted(TARGETS):
        p = PARTS[k]
        facts = ' '.join('{' + f + '}' for f in sorted(p.facts) if not f.startswith('coach_')) or 'none'
        note = p.note.replace('|', '/')
        out.append(f'| `{k}` | {WHO[p.who]} | {note} | {facts} | {TARGETS[k]} |' +
                   (f' {done.get(k, 0)} |' if done else ''))
    out += ['', f'{len(TARGETS)} parts, {total():,} lines asked.', '']
    return '\n'.join(out)


if __name__ == '__main__':
    import sys
    if len(sys.argv) > 2 and sys.argv[1] == '--doc':
        open(sys.argv[2], 'w').write(doc())
        sys.exit(0)
    by = {}
    for k, n in TARGETS.items():
        by.setdefault(k.split('.')[0] + '.' + k.split('.')[1], 0)
        by[k.split('.')[0] + '.' + k.split('.')[1]] += n
    for k, n in sorted(by.items()):
        print(f'{k:24} {n:6}')
    print(f'{len(TARGETS)} parts, {total():,} versions')
