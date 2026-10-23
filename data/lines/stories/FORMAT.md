# Story line banks (final build lane 8)

Plain text banks the engine reads (`crates/afm-engine/src/world/stories/lines.rs`).

## Format

```
# a comment
[key]
one template per line
another template
[key@2010-]
lines only used in seasons 2010 and later
[key@1996-2009]
lines only used in seasons 1996-2009
```

- A key is dotted lowercase (`nick.speed`). The same key may appear several times with different year ranges; lines with no range are used in every year.
- One template per line. Blank lines are ignored. No line may start with `#` or `[`.
- Placeholders are in braces, e.g. `{last}`. Only the placeholders listed for a key may be used. A template that needs a value the caller doesn't have is skipped.
- No em-dashes. Plain American sportswriting: specific, concrete, varied in rhythm and length. No two lines in a key should read alike.
- Never use a legendary real nickname or named play (see `blocklist` in nicknames.txt).
