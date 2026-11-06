#!/usr/bin/env bash
# Publishes the world in ./out on the worlds page (cnbville/afm-worlds), as
# a release tagged name-commit, then its tallies and its log: the "Build a
# world" workflow's world job runs it, or the finish job when the world was
# built over two machines (NB13). Needs GH_TOKEN (WORLDS_TOKEN), SEED, YEARS
# and SHA; writes tag=... to GITHUB_OUTPUT.
#
# The world goes up first, so a step that fails after it never costs the
# world. A release file may be at most 2 GB, so a bigger world goes up in
# parts (name.afmworld.001, .002, ...), which the game joins. The whole file
# stays for the star mix check. Then its tallies (lane NB1,
# name.afmdata.tar.001, ..., which the game downloads too), then the world
# log (world reader lane R2) as a tar file in parts (name.afmlog.tar.001,
# ...), packed as the folder is removed so the disk holds it once; the game
# does not download it, the book job does.
set -euo pipefail
repo=cnbville/afm-worlds
gh api "repos/$repo/commits?per_page=1" > /dev/null 2>&1 || gh api -X PUT "repos/$repo/contents/README.md" \
  -f message="Start the downloads page" \
  -f content="$(printf '# AFM downloads\n\nReady-built worlds and the newest AFM for Mac. The game downloads from here by itself.\n' | base64 | tr -d '\n')"
cd out
f=$(ls *.afmworld)
up="$f"
if [ "$(stat -c %s "$f")" -gt 2000000000 ]; then
  split -b 1900M -d -a 3 --numeric-suffixes=1 "$f" "$f."
  up=$(ls "$f".[0-9][0-9][0-9])
fi
short=$(echo "$SHA" | cut -c1-7)
tag="${f%.afmworld}-$short"
# shellcheck disable=SC2086
gh release create "$tag" $up --repo "$repo" \
  --title "Seed $SEED, $YEARS years of history (game $short)" \
  --notes "Built $(date -u +%Y-%m-%d) by the AFM game at commit $SHA. The world reader's book is the .afmbook.zst file (unzstd it, joining parts first)."
rm -f "$f".[0-9][0-9][0-9]
echo "tag=$tag" >> "$GITHUB_OUTPUT"
for d in *.afmdata; do
  [ -d "$d" ] || continue
  tar --remove-files -cf - "$d" | split -b 1900M -d -a 3 --numeric-suffixes=1 - "$d.tar."
  gh release upload "$tag" ./"$d".tar.* --repo "$repo"
  rm -f "$d".tar.*
done
for l in *.afmlog; do
  [ -d "$l" ] || continue
  tar --remove-files -cf - "$l" | split -b 1900M -d -a 3 --numeric-suffixes=1 - "$l.tar."
  gh release upload "$tag" ./"$l".tar.* --repo "$repo"
  rm -f "$l".tar.*
done
df -h / /mnt
