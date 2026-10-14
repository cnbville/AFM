# 14 Match engine spec to GitHub

Thread "Match engine spec to GitHub" from the AFM project on the old account (thread id cmsg_015NJfb6NRUve4Wa2buMtZKs3vY3omfK4KUsMuzcyUMNNN). Every message, oldest first, copied word for word on 2026-10-14. 5 messages.

### 2026-10-13 18:13:47 UTC · Claude
<sub>id cmsg_015NJfb6NRUve4Wa2buMtZKs3vY3omfK4KUsMuzcyUMNNN · attachments: Section 3 spec: match engine and stats database <https://github.com/cnbville/AFM/pull/19></sub>

Section 3 design is fully answered, so I'm starting the write-up of the match engine spec into the repo now.

### 2026-10-13 18:13:57 UTC · Claude
<sub>id cmsg_015NJfb6NRUve4Wa2buMtZKs3RMhy5MRvQi73UoQtvZvxq</sub>

I'm reading the section 3 design thread first, so the spec keeps your own wording and examples.

### 2026-10-13 18:14:08 UTC · Claude
<sub>id cmsg_015NJfb6NRUve4Wa2buMtZKs8tH3DRTKhhQb6pqHwR9YUw</sub>

*[Progress checklist update]*

```text
Writing the section 3 spec

✱ Reading the design thread
○ Write docs/design/match-engine.md
○ Open the PR and merge to main
```

### 2026-10-13 18:16:13 UTC · Claude
<sub>id cmsg_015NJfb6NRUve4Wa2buMtZKs3CD2STjKjBYmd4h4495qjc</sub>

*[Progress checklist update]*

```text
Writing the section 3 spec

✓ Read the design thread
✓ Wrote docs/design/match-engine.md
**✓ Opened PR #19 and merged it to main**
```

### 2026-10-13 18:16:20 UTC · Claude
<sub>id cmsg_015NJfb6NRUve4Wa2buMtZKs4WNpYZLUpCGKx9rR5g2d1Z</sub>

The section 3 spec is now on main in `docs/design/match-engine.md` ([PR #19](https://github.com/cnbville/AFM/pull/19), merged). It covers all ten topics in your own words, including all your examples: the 4.2 safety, the 3-4 Okie, the kickers, Derrick Henry and play calls you accept or deny. It also records the build order (athlete-first player creation comes first), room for better visuals, and that the real-data checks only run during development.

Two items are marked "proposed, not confirmed" because you never answered them on their own. The first is how much detail to trim for high school players who never make college, since your rule is to log as much as possible. The second is whether beating expectations in the stats should raise a player's hidden ceilings.
