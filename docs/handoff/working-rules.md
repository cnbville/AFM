# How Raichel wants Claude to work

Every working-style rule and correction Raichel gave in the old project, with their words, the date and where it was said. Written 2026-10-14 from the transcripts for the move to the new account.

Most rules came from a correction: Claude did something, and Raichel pushed back. The quote shows what went wrong, so the new project doesn't repeat it. Pointers like ([T00] 2026-10-11 18:35) name the transcript and the time (UTC); the link list is at the bottom.

---

## 1. Raichel decides

1. **Ask before every design decision. Bring a concrete proposal and wait.**
   - "I need you to calm down with the freestyling. I need you to run aspect through me because you're making a lot of decisions without me right now" ([T00] 2026-10-10 20:01).
   - Claude had built the player model with its own defaults. It stopped and turned each default into a question ([T02] 2026-10-10 20:08).
2. **Talk before building when Raichel is still reviewing.**
   - "tell me what before you adjust anything" and "let's talk first" ([T02] 2026-10-11 12:27).
   - "(we'll talk before we implement.)" (same message).
   - "wait before we continue i haven't judged trial 5 yet" ([T04] 2026-10-11 14:06).
   - "Don't start building yet i first need to read the rest" ([T04] 14:42).
3. **"Don't build" means spec only.** "don't build push spec to  to git as next needed fix," ([T17] 2026-10-14 12:20).
4. **Mark Claude's unanswered picks as "proposed, not confirmed"** in every doc. Don't invent decisions. (Project rule since 2026-10-11; see `docs/roadmap.md` labels.)
5. **Remind Raichel of their own past answers when asked, plainly.** "you said that i said something abouy five-stars busts but i don't remember what i said" ([T04] 2026-10-11 14:38). Quote what they said, in simple sentences.

## 2. Ultra realism, always

1. **Never ask "realistic or simplified?". The answer is always the most realistic option.**
   - "so i'm noticing you ask me a lot of questions that you already know the answer too, i've told you multiple times i want yltra realism so your questions are arbitrary." ([T00] 2026-10-11 19:35)
   - "Instead of "oh should i use realistic schools bullshit," lets talk about the ranking of coaches, if a school is in a crime/low income zone how does that effect as opposed to a matter dei. That's the real shit i want to. get into." (same message)
   - Ask only about deep mechanics (how things work and interact), each with a concrete proposal.
2. **Realism wins unless it breaks the game.** "it's so extremely important that we make it realistic, unless it breaks the game or the possibility of the game you'll always match ultra realism" ([T13] 2026-10-13 17:00).
3. **The first proposal must already be the full-realism version.** Raichel had to ask twice in a row.
   - "anyway to make it more realistic ?" ([T18] 2026-10-14 11:10).
   - "I do but i'm wondering why you didn't propose this initially ?" (11:12).
   - "any ways to make it more realitisct >" (11:18).
   - "yes but i'm once again not happy that i had to give you an extra prompt before you considered those changes" (11:25).
   - Before posting, do the "what else would make this more realistic?" pass yourself: leverage, information and deception, situation, field and weather, level differences, real research, per-player nuance. Shorten the wording, never the substance.
4. **Dynamic, never static or flat.**
   - "Returning opinion is that every thing needs to be realistic" ([T02] 2026-10-10 20:21).
   - "Just don't make it stact, make it dynamic" ([T02] 2026-10-11 12:53).
   - "The nuances are important and just B is too plain" ([T15] 2026-10-14 08:14).
   - Derive outcomes from attributes and real football reasoning, never flat rules.
5. **Influences are noteworthy, rarely driving.** Raichel repeated this on almost every background answer: "do not make this a driving factor", "i repeat don't make this overly powerful", "it should be an noteworthy influence on potential, not driving factor", "I repeat, don't make this a driving factor" ([T00] 2026-10-11 19:52).
6. **Outliers must be possible.** "i'm very insistant of having outliers be present, a derrick henry type of situation where he scored 153 touchdowns in higschool should be a ble to happen" ([T13] 2026-10-13 18:12). Real-data checks are development safeguards only, never an in-game nudge ("are you treating it as an safe guard or as an in-game fix ?", 18:11).
7. **Research properly, and check sources.**
   - "Obviously and i expect you to research substantial information and data base" ([T13] 2026-10-13 17:12).
   - "YOU RESEARCH WHATEVER YOU NEED TO" ([T08] 2026-10-13 06:01).
   - "how sure are you of that ?i  believe i received  different info. Source check please" ([T16] 2026-10-14 08:45). Verify dates and facts with a source before stating them.
8. **Use common sense on obvious details.** "use common sense, redshirt, true freshman AND sophomore" ([T16] 2026-10-14 09:04).
9. **Potential stays hidden in the game.** "i absolutely don't want the "this is the players ceiling exactly" that defeats the entire point of the game... This is extremely pivotal to understand" ([T00] 2026-10-11 19:19). Exact potential is for trial and debug tools only.
10. **One aspect per proposal, never compound proposals.** "This game is about utter and total realism and multiple times when i ask to brain storm you keep suggesting compound proposals. "Oh this could be good" while inside that "this" are 10 individual aspects we need to discuss and check for realism/immersion. This needs to be stated clearly without me hgaving to manuallly tell the threads" ([project chat] 2026-10-15 11:11). Every proposal is one single aspect, checked on its own for realism and immersion, with its own yes or no. Split a big idea into its separate aspects as numbered items, grouped under headings when the list gets long. Small items of one kind (for example personal events: births, funerals, weddings) stay one proposal: "bundle them together, so that personal events is like a propasal instead of choosing between them individually" (2026-10-15 21:32).

## 3. Write plainly, ask clearly

1. **Short, clear sentences.** Raichel got confused several times:
   - "that sentence structure is confusing me, please be clearer" ([T04] 2026-10-11 14:40).
   - "you're wording is confusing" ([T16] 2026-10-14 12:03). The fix that worked: small, medium, big and huge, with one example each.
   - "then i'm still really confused why you ask me that in a seperate questuon" ([T00] 2026-10-11 19:27).
2. **Say exactly what you need from Raichel.**
   - "be clearer, what you need from me right now before we can execute the final trial 6 ?" ([T00] 2026-10-11 17:47).
   - "Be more clear what you mean with 1." (17:48).
   - "what's your actual question for me ?" ([T15] 2026-10-14 08:03).
3. **Ask in small batches, not one chaotic message.** "? what are you doing, chaotic message you send me" ([T00] 2026-10-11 18:35). The fix was batches of ten.
4. **Later questions must use earlier answers.** "make sure the questions in batch 2 take these answers into account" ([T00] 2026-10-11 18:42).
5. **Go topic by topic, in order. Don't skip.** "Why are you skipping topic 2 ?" ([T00] 2026-10-11 19:54).
6. **Keep Raichel oriented.**
   - "i lost track of where we are" ([T00] 2026-10-13 05:40).
   - "I'm not seeing a clear overview of what we're trying to build ?" ([T00] 2026-10-11 17:43).
   - "so what did for section so far was just setting the rules, it's not build yet ?" ([T00] 2026-10-13 07:59). Say clearly what is designed, what is built and what is next.
7. **Brief summaries are fine if the full dynamic is behind them.** "i assume you're now summarizing it briefly but have a broad dynamic in mind ?" ([T11] 2026-10-13 16:07). Yes: the full version goes in the doc.
8. **Expand when asked, then compress.** "expand your proposal on section 8, since i'm a stats fetiscist" ([T13] 17:39), then "okay i know i asked to expand but please compress it a bit so i get a better grasp" (17:42).
9. **Answer "genuine" questions as questions.** "What difference would it make ? (genuine question)" ([T00] 2026-10-11 17:55).
10. **Prompts for other agents say what's happening, not "you're joining".** "Not so much "JOINING" more this is what is happening elsewhere" ([T00] 2026-10-14 07:15). And "then you need to be clearer in the prompt" ([T00] 2026-10-11 20:21), after a loose data prompt.

## 4. Tokens, threads and messages

1. **Keep token use low.**
   - "any way you can decrease the cache ? we're using a lot of tokens right now" ([T00] 2026-10-11 13:57).
   - "we have to find a fix for this current token consumption. Single messeages cost 6% of my session" ([T00] 17:22), and "none because i've tried those alreeady and it still takes me 6% per message." (17:23).
   - "Minimize tokens used" ([T04] 2026-10-11 14:37).
   - So: one checker pass per trial, no agent fan-outs, no small follow-up messages.
2. **Fresh threads for big new work.** "Do trial 5 on a new thread" ([T00] 13:58), "make a completely fresh thread" (17:53), "Let's move on to section 3, fresh thread" (2026-10-13 16:56), "start it in a cresh thread please" ([T15] 2026-10-14 08:33).
3. **Design answers in the project chat, to save tokens.** "i'm answeringing topics here to save tokens ->" ([T00] 2026-10-13 05:49).
4. **Parallel threads are welcome.** "is there anything you want to hand off to another agent to save you tokens ? so that we can work simultaniously and get this going asap" ([T00] 2026-10-11 20:00). Also "Let's work section 4 in the mean time" (2026-10-13 18:27).
5. **Build threads push at once and send one final reply.** "immediately push to git after you're done so there is no small message i need to respond to that eventually costs me a lot of tokens" ([T00] 2026-10-11 17:53).
6. **Status must be honest.** "are you still running it ? looks stuck" ([T17] 2026-10-14 11:19). "i ask because usually [the paused icon] means that it's waiting on something for me to do" ([T00] 11:23). Say plainly when nothing is needed from Raichel.
7. **Give runtime estimates when asked.** "give me an esitmate on the runtime it'll take" ([T00] 2026-10-14 12:26).

## 5. Trials

1. **Every result explains why.** "you need to explain to me why so i can judge if it's within reason" ([T02] 2026-10-11 12:53).
2. **Sanity-check before showing.** "Do a sanity check, i'm seeing a lot of realism/logically fallacy’s" ([T02] 2026-10-10 23:07).
3. **Show it visually.** "can you visiualize the trial for me so i get an idea, a mock gui" ([T11] 2026-10-13 10:15). "BUild me a 10 player projection you feel like is realistic so i can see what you're aiming at" ([T02] 2026-10-10 20:26).
4. **Full player pages.** "i want more player details being shown, combine stats, so i can determine if it's just elite size etc, i'm not getting enough context in these pages to really decide if soemthings off" ([T11] 2026-10-13 12:18).
5. **Refresh both the mock GUI and the player pages every trial.** "why did you only give me a players page trial 2 ? and not the mock up you gave me the first time as well ?" ([T11] 2026-10-13 14:49).
6. **Keep a running list while Raichel reviews.** "keep a list as i walk through the data." ([T11] 2026-10-13 11:35).
7. **Park flags instead of fixing on the spot.** "no list them up for potential trial 3. I wanna get moving on the rest" ([T11] 2026-10-13 15:16).
8. **Don't re-measure after tuning.** "no need to re-messure after tuning. Note it as an worry/potential fix in the list so we can trial it after words, i don't want to use unneccessary tokens" ([T11] 2026-10-13 16:38).
9. **Check the whole picture, not one data point.** "don't just check #1 check the entire first round, the logic needs to be bound to every draft pick but still be intertwined with team need" ([T11] 2026-10-13 16:23).
10. **Trials must match the final game.** "but how can we trust the data output if the engine is not kept the same" ([T15] 2026-10-14 08:05). "As simillar to the final game as possible otherwise i'll find compounding bugs later on" (08:07).
11. **Bigger samples find more bugs.** "I also think the 10 player limit we have isn't showing all the potential bugs so let's maybe build 30 player trial before we go make big decisions." ([T02] 2026-10-11 12:27)
12. **After the player model, gameplay finds the bugs.** "Don't find/search calls/bugs, that's up to me to find out when eventually in gameplay." ([T00] 2026-10-11 17:53)

## 6. Git, merging and agreed sequences

1. **Everything goes into git, readable without the chat.**
   - "first, how are you currently making sure that every single detail we discussed gets saved outside of this convo ?" ([T00] 2026-10-11 19:54).
   - "push to an full spec in repo - i want everything we roadmapped and build so far uploaded to github in extensive detail. I work outside of this chat too so anywone not using this chat should be able to read it too" (19:56).
   - "write a clear prompt to the thread to push all we currently have into the git, make sure the git holds all the info and content we discussed so it can be understood by agents that only have access to the git." ([T00] 17:24).
2. **Finished work merges to main.**
   - "B" (merge into main) ([T00] 2026-10-11 17:56). "do PR and merge ofcourse" ([T11] 2026-10-13 16:39).
   - "why are the latest two threads not merged ?" ([T00] 2026-10-14 11:21).
3. **Agreed sequences carry on without being asked.** "no the agreement was to do the world trial 3 fix after the engine ? is there something blocking you from doing this ?" ([T15] 2026-10-14 06:46). When a build finishes, start the next agreed step.
4. **Engine speed-ups keep identical results.** The same seed must give exactly the same game. Ask before any speed-up that would change a result ([T15] 2026-10-14 08:12, Claude's promise after Raichel asked).
5. **Handoffs carry everything.** "can't harp enough that this handoff needs to include all the possible context so that nothing we spoke about gets left behind...nothing" ([T00] 2026-10-14 12:22).

[T00]: transcripts/00-project-chat.md
[T02]: transcripts/02-build-the-player-potential-and-development-model.md
[T04]: transcripts/04-trial-5-league-sim.md
[T08]: transcripts/08-real-world-dataset-check.md
[T11]: transcripts/11-build-the-world.md
[T13]: transcripts/13-section-3-match-engine-design.md
[T15]: transcripts/15-build-the-match-engine.md
[T16]: transcripts/16-section-4-calendar-design.md
[T17]: transcripts/17-world-trial-3.md
[T18]: transcripts/18-plays-and-tactics-design.md
