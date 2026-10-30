#!/usr/bin/env bash
# The world reader's book (lane R4, docs/world-reader.md) for the world in
# ./out and the log beside it: every record as a table, the checks against
# real, the odd things and the coverage map. Its overview goes in the run's
# summary. It runs under a memory cap where systemd allows one, so running
# out of memory fails this step instead of killing the runner.
set -euo pipefail
f=$(realpath "$(ls out/*.afmworld | head -1)")
cargo build --release -p afm-reader --bin afm_read
cap=(sudo -E systemd-run --scope --quiet -p MemoryMax=13G -p MemorySwapMax=0 --uid="$(id -u)" --gid="$(id -g)" --)
if ! "${cap[@]}" /usr/bin/true; then
  cap=()
  echo "No memory cap on this runner"
fi
"${cap[@]}" "$PWD/target/release/afm_read" build "$f"
b="${f%.afmworld}.afmbook"
{ echo '```'; target/release/afm_read "$b" overview limit=20; echo '```'; } >> "$GITHUB_STEP_SUMMARY"
zstd -T0 -9 --rm "$b" -o "$b.zst"
ls -la out/
df -h / /mnt
