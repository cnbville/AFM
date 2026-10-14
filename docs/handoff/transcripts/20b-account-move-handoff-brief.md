# 20b The coordinator's brief for the handoff thread

The instructions the coordinator session gave this thread when it started (2026-10-14 11:47:42 UTC), copied word for word from this thread's own session log. Threads never show these briefs in the chat, and the briefs for the other 19 threads could not be copied (see the README, "Not carried over"). The two later notes from the coordinator (12:23 and 12:27) and the one at 13:03 are in [20-account-move-handoff.md](20-account-move-handoff.md).

```text
<relay from="coordinator" session="session_018ttM73RpNgEsKygeTj4pUK" current-time="2026-10-14T11:47:42Z" reason="spawn">
  The messages below are picked by the coordinator session and not necessarily consecutive. User messages are copied verbatim by the server.
  <cited id="cmsg_015NJfb6NRUve4Wa2buMtZKsPbCSrRaekkg9NbhchzSmAz" author="user" name="Raichel" at="2026-10-14T11:43:00Z" where="timeline">
      I have two claude accounts, this one which has this project is going to close and i want to continue on a different one. Is there a best way to do this since i oviously already came pretty far on this account ?
  </cited>
  The message below was written by the coordinator session for context; it carries no approval from your user.
  <cited id="cmsg_015NJfb6NRUve4Wa2buMtZKsDfNEXVsx2mhvLTBycohFBw" author="coordinator" session="session_018ttM73RpNgEsKygeTj4pUK" at="2026-10-14T11:43:10Z" where="timeline">
      Yes. The safest way is to get everything into the GitHub repo before this account closes, then start a new project on the other account pointed at that same repo. I don't know of any way to move a project itself between accounts, so I'd assume it can't be done.
      
      **What carries over by itself:** the repo, which holds all merged code, specs and trial docs. On the new account you only need to connect GitHub as cnbville.
      
      **What's lost unless we copy it:**
      - Project memory (the running summary of all your decisions and preferences)
      - The shared design files (trial feedback, fix lists, calendar design, code backups)
      - Work still in progress: the World Trial 3 trace tools and Tennessee findings, the plays and tactics answers so far (topics 1 to 6), and the section 4 calendar design
      - The chat history itself, and the mock GUI and player page links
      
      **My plan:** one thread copies all of that into a docs/handoff folder in the repo and merges it. That includes a ready-to-paste starting prompt for the new project, so the new Claude picks up exactly where we are. Your answers in the open threads are already summarized in memory, so the handoff won't lose them.
      
      Should I start the handoff now? I'd do it soon, before the account closes.
  </cited>
  The note below was written by the coordinator session, a Claude session, not by your user.
  <note>
      Raichel is moving this project to another Claude account, and this one will close. Copy everything that only lives in this project into the GitHub repo https://github.com/cnbville/AFM under docs/handoff/, then merge it to main. That way the new project on the other account loses nothing.
      
      Copy:
      - The full project memory (read_memory), plus any memory files you can read.
      - Every file in /mnt/project-files (design/, trials/, code-backups/), copied as-is.
      - The design threads' decisions written out in full with Raichel's own wording. Read them with fetch_thread: "Plays and tactics design" (cmsg_015NJfb6NRUve4Wa2buMtZKsEx1hpV9u9vgyc9BbY1LLiX, still in progress), "Section 4 calendar design" (cmsg_015NJfb6NRUve4Wa2buMtZKsPTshjE8EffLpuChfRtDEcB, in progress), and the latest findings and the physics fix list from "World Trial 3" (cmsg_015NJfb6NRUve4Wa2buMtZKsKq4hQ8PrYACjiAt4Cefp8B). Skim the project timeline for decisions that aren't in memory yet.
      - A list of the published artifact links (list_project_artifacts) and every unmerged branch with what it holds. Branches stay on GitHub, so list them and don't merge them.
      - A README in docs/handoff that says where things stand, what is in progress, and the agreed next steps. Also include a ready-to-paste starting prompt for the new project, so the new Claude continues in the same way of working. The how-to-work rules in memory must be in that prompt.
      
      Don't touch engine code or other threads' branches. The other threads keep running here. Reply once, when it's merged, with the link to the handoff README.
  </note>
</relay>
```
