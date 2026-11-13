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
