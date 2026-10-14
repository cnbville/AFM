# 01 Map AFM repo toward the football sim goal

Thread "Map AFM repo toward the football sim goal" from the AFM project on the old account (thread id cmsg_015NJfb6NRUve4Wa2buMtZKsJZ2Xgaq8DDrvue1N3VJuwN). Every message, oldest first, copied word for word on 2026-10-14. 7 messages.

### 2026-10-10 19:43:13 UTC · Claude
<sub>id cmsg_015NJfb6NRUve4Wa2buMtZKsJZ2Xgaq8DDrvue1N3VJuwN</sub>

Map AFM repo toward the football sim goal

### 2026-10-10 19:43:25 UTC · Claude
<sub>id cmsg_015NJfb6NRUve4Wa2buMtZKsHDp5hje64Gd1iTniwHV2s9</sub>

*[Progress checklist update]*

```text
Mapping the AFM repo

✱ Reading repo structure and code
○ Check branches, PRs and issues
○ Compare against the game goal
```

### 2026-10-10 19:43:39 UTC · Claude
<sub>id cmsg_015NJfb6NRUve4Wa2buMtZKsKf8aPVZHmUGe6Fou4tSGk2</sub>

*[Progress checklist update]*

```text
Mapping the AFM repo

✓ Read repo structure and code
✓ Checked branches, PRs and issues
**✓ Compared against the game goal**
```

### 2026-10-10 19:43:44 UTC · Claude
<sub>id cmsg_015NJfb6NRUve4Wa2buMtZKs6FHLWH9F5Pk75m1brq6K8u</sub>

The AFM repo is completely empty right now. It has no commits, no files and no stack chosen, and GitHub shows no branches, pull requests or issues (checked with `git ls-remote` and the GitHub API).

That means nothing is in progress and every part of the goal is still a gap: the core sim engine (play-by-play game simulation), the player model (ratings, hidden potential, development curves), the high school to college to NFL pipeline, season flow, player cards and UI.

**One decision is yours: which stack should I build it on?**
- **TypeScript web app (recommended):** One language for the sim engine and the UI, it runs in any browser, and player cards and stat screens are easy to build.
- **Python engine first:** The simulation and data work comes quickly, but a UI has to be added later.
- **Godot or Unity:** Feels more like a real game, but iterating on deep stats is slower.

If you pick TypeScript, I'd start with the player model and development system. Potential, ageing and the high school to NFL progression are what everything else sits on.

### 2026-10-10 19:46:52 UTC · Raichel (member)
<sub>id cmsg_015NJfb6NRUve4Wa2buMtZKs98YdXoHoHeBycgaynYUya5</sub>

expand on what you mean about "real game" with godot/unityy

### 2026-10-10 19:47:00 UTC · Claude
<sub>id cmsg_015NJfb6NRUve4Wa2buMtZKsPjbc371UwNenVK7ns9MXKN · edited 2026-10-10T19:47:13Z</sub>

"Real game" was too vague of me. I meant what a game engine gives you that a web app doesn't by default:

- **A visual match engine.** FM shows matches as a 2D or 3D pitch with moving players. Godot and Unity are built for this: animation, camera, and 22 players running routes in real time. In AFM that would be watching plays unfold on a field instead of reading a play-by-play log.
- **A standalone desktop game.** You get an installable .exe or .app, ready for Steam, with fullscreen, sound and controller support.
- **Game feel.** Polished transitions, effects and audio are easier to add.

The catch is that FM is about 90% menus, tables, stat screens and player profiles, with the match view on top. Game engines are clumsy at dense tables and data screens, and a web stack is strongest at exactly that. A web app also isn't limited to text. It can draw a top-down 2D field with animated plays (like FM's classic 2D view), and later it can be packaged as a desktop app for Steam using Electron or Tauri.

So I still recommend TypeScript, with the sim engine kept separate from the visuals. If you later want a full 3D match view, the same engine can drive it.

[Edit: The player model is already being built this way in [Build the player potential and development model](#cmsg_015NJfb6NRUve4Wa2buMtZKs1pbtkM6b31NoTjktrjmMCU). Tell that thread if you'd rather use a different stack.]

### 2026-10-10 19:47:04 UTC · Claude
<sub>id cmsg_015NJfb6NRUve4Wa2buMtZKs2ihdV6wRgMXgxXiRRwvR2p</sub>

*[Internal note between Claude sessions: Received a message from the coordinator]*

> FYI: the player model is already being built (TypeScript default, engine-agnostic) in the thread "Build the player potential and development model". If Raichel says go, point them there instead of starting your own build, and stay read-only.
