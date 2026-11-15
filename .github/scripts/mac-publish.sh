#!/usr/bin/env bash
# Puts the newest Mac build on the "latest" link and on the public downloads
# page. Run by the "publish" job of mac-app.yml, one run at a time (its
# concurrency group), after a build has put its own release (app-<run>) on the
# AFM repository.
#
# Builds now run side by side and finish in any order, so this never publishes
# "the build that just finished". It looks at every finished build release,
# takes the highest run number, and only ever moves forward:
#   1. the AFM repository's "latest" release becomes the highest build, unless
#      a higher one is already latest;
#   2. the downloads page (afm-worlds, release app-latest, which holds the
#      dmg and the in-app update's latest.json) is replaced by the highest
#      build's own files, unless it already shows that build or a newer one.
# A job that was dropped from the queue costs nothing: the next one reads the
# same releases and ends in the same place.
#
# Needs: REPO (owner/name of the AFM repository), AFM_TOKEN (the job's token),
# WORLDS_TOKEN (may be empty: then only step 1 runs). DRY_RUN=1 prints the
# writes instead of doing them.
set -euo pipefail

repo=${REPO:?REPO is not set}
worlds=${WORLDS_REPO:-cnbville/afm-worlds}
dry=${DRY_RUN:-}

gha() { GH_TOKEN="$AFM_TOKEN" gh "$@"; }
ghw() { GH_TOKEN="$WORLDS_TOKEN" gh "$@"; }
write() { if [ -n "$dry" ]; then echo "DRY RUN: $*"; else "$@"; fi; }

# The highest finished build: a published release app-<n> whose dmg is fully
# uploaded (a release that broke half way through its upload doesn't count).
newest=$(gha api --paginate "repos/$repo/releases?per_page=100" \
  --jq '.[] | select(.draft | not) | select(.tag_name | test("^app-[0-9]+$")) | select(any(.assets[]; .name == "AFM_universal.dmg" and .state == "uploaded")) | .tag_name | ltrimstr("app-")' \
  | sort -n | tail -n 1)
if [ -z "$newest" ]; then
  echo "No finished build release yet: nothing to publish."
  exit 0
fi
echo "Highest finished build: $newest."

# 1. The AFM repository's latest release (github.com/<repo>/releases/latest).
current=$(gha api "repos/$repo/releases/latest" --jq .tag_name 2>/dev/null || true)
current_n=${current#app-}
if [[ "$current" =~ ^app-[0-9]+$ ]] && [ "$current_n" -ge "$newest" ]; then
  echo "Latest release is already $current."
else
  echo "Latest release is '${current:-none}': moving it to app-$newest."
  write gha release edit "app-$newest" --repo "$repo" --latest
fi

# 2. The public downloads page and the in-app update.
if [ -z "${WORLDS_TOKEN:-}" ]; then
  echo "No WORLDS_TOKEN secret yet: no public download or update."
  exit 0
fi
title=$(ghw release view app-latest --repo "$worlds" --json name --jq .name 2>/dev/null || true)
shown=$(printf '%s' "$title" | sed -n 's/.*0\.2\.\([0-9][0-9]*\).*/\1/p')
shown=${shown:-0}
if [ "$shown" -ge "$newest" ]; then
  echo "The downloads page shows build $shown: nothing to do."
  exit 0
fi
echo "The downloads page shows build $shown: putting build $newest on it."

dl=$(mktemp -d)
gha release download "app-$newest" --repo "$repo" --dir "$dl"
files=("$dl/AFM_universal.dmg")
[ -f "$dl/AFM.app.tar.gz" ] && [ -f "$dl/latest.json" ] && files+=("$dl/AFM.app.tar.gz" "$dl/latest.json")

# A brand-new repository has no commit, and a release needs one.
ghw api "repos/$worlds/commits?per_page=1" > /dev/null 2>&1 || write ghw api -X PUT "repos/$worlds/contents/README.md" \
  -f message="Start the downloads page" \
  -f content="$(printf '# AFM downloads\n\nReady-built worlds and the newest AFM for Mac. The game downloads from here by itself.\n' | base64 | tr -d '\n')"

write ghw release delete app-latest --repo "$worlds" --cleanup-tag --yes || true
write ghw release create app-latest "${files[@]}" --repo "$worlds" --latest=false \
  --title "AFM for Mac 0.2.$newest" \
  --notes "The newest AFM for Mac (one app for Apple silicon and Intel). First install: open the dmg, drag AFM to Applications, then right-click AFM and choose Open. After that it updates itself."
echo "Done: the downloads page shows build $newest."
