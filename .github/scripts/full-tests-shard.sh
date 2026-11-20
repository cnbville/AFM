#!/usr/bin/env bash
# Runs one shard of Full tests (.github/workflows/full-tests.yml).
#
# Why shards: the tests that build worlds are CPU-bound on a 4-core runner and
# one job per part grew past its 4-hour limit (2026-11-20: units, screens and
# worlds each took 3.5 hours or more, from about an hour on 2026-11-15). Every
# shard is a share of the same tests, sized to a bit over an hour of tests.
#
# Nothing is dropped by a typo in a name: a shard that lists names (a test file,
# a module) fails when cargo cannot find the file, a shard that takes "the
# rest" is built from the other shards' lists (a new test file or test lands
# there), and a shard fails when it ran no test at all.
#
# `DRY_RUN=1 full-tests-shard.sh <shard>` prints the command and runs nothing.
# `full-tests-shard.sh list` prints every shard name.
set -euo pipefail

cd "$(dirname "$0")/../.."

# --- the lists (the only place that says which shard runs what) ---------------

# afm-game's lib unit tests that build their own small world, a few minutes
# each (module filters, substring of the test path). The rest of the lib tests
# run in units-core.
GAME_A=(lane_campaign:: lane_capital_)
GAME_B=(lane_find::)
GAME_C=(lane_leads:: lane_logos:: lane_training::)

# afm-game's screen tests (crates/afm-game/tests/*.rs), grouped by the small
# world each builds (one build is about 13 minutes; tests/common::build_world
# shares a world inside one job, so files on the same seed share a shard).
SCREENS_1=(autosave career depth_overview draftroom film find fresh_board scouting attr_view squadboards) # seeds 7, 17
SCREENS_2=(big_moments inbox_college inbox_commit_sync inbox_portal meeting_blocks stories boards gameplan handbook testing_history people) # seeds 12, 13 (+11 for people)
SCREENS_3=(gameday screens_core seats live_core) # seed 11, and 12 with the 2025 engine
SCREENS_4=(capital_campus_screen capital_donors_screen ground_person_screen ground_rival_screen ground_staff_screen ground_week_screen) # seeds 22, 23
# screens-5 is every other file, so a new file lands there.

# tools/tests/world.rs: three tests build worlds of their own, the other
# fourteen share one full-size world (built once inside the job).
WORLD_OWN_1=(a_seed_gives_the_same_world_on_any_core_count)
WORLD_OWN_2=(a_history_through_the_expansion_years_runs a_saved_world_plays_on_identically)

# --- helpers -------------------------------------------------------------------

test_files() { # <crate dir>: the stems of its integration test files
  local f
  for f in "$1"/tests/*.rs; do [ -e "$f" ] && basename "$f" .rs; done
  return 0
}

# --test <name> for every afm-game screen file not in the named lists.
screens_rest() {
  local taken=" ${SCREENS_1[*]} ${SCREENS_2[*]} ${SCREENS_3[*]} ${SCREENS_4[*]} " n
  for n in $(test_files crates/afm-game); do
    case "$taken" in *" $n "*) ;; *) printf '%s\n' "$n" ;; esac
  done
}

# --test <name> for every integration test file outside afm-game, world.rs left out.
tools_rest() {
  local d n
  for d in crates/*/; do
    d=${d%/}
    [ "$d" = crates/afm-game ] && continue
    for n in $(test_files "$d"); do
      [ "$d/$n" = crates/afm-tools/world ] || printf '%s\n' "$n"
    done
  done
}

CARGO=(cargo test --no-fail-fast)
args=()
doc=0

tests_of() { # --test <n> for each name given
  local n
  for n in "$@"; do args+=(--test "$n"); done
}

case "${1:-}" in
  list)
    printf '%s\n' units-core units-game-a units-game-b units-game-c \
      screens-1 screens-2 screens-3 screens-4 screens-5 worlds-misc worlds-own-1 worlds-own-2 worlds-shared
    exit 0
    ;;
  # The units: every lib and bin test in the workspace. The three heavy groups
  # of afm-game's lib tests have a shard each (and are skipped in units-core);
  # their shards leave out afm-engine's tests, which run in units-core.
  units-core)
    args=(--workspace --lib --bins --)
    for f in "${GAME_A[@]}" "${GAME_B[@]}" "${GAME_C[@]}"; do args+=(--skip "$f"); done
    doc=1
    ;;
  units-game-a) args=(--workspace --exclude afm-engine --lib -- "${GAME_A[@]}") ;;
  units-game-b) args=(--workspace --exclude afm-engine --lib -- "${GAME_B[@]}") ;;
  units-game-c) args=(--workspace --exclude afm-engine --lib -- "${GAME_C[@]}") ;;
  # The game's screen tests.
  screens-1) args=(-p afm-game); tests_of "${SCREENS_1[@]}" ;;
  screens-2) args=(-p afm-game); tests_of "${SCREENS_2[@]}" ;;
  screens-3) args=(-p afm-game); tests_of "${SCREENS_3[@]}" ;;
  screens-4) args=(-p afm-game); tests_of "${SCREENS_4[@]}" ;;
  screens-5) args=(-p afm-game); tests_of $(screens_rest) ;;
  # Every other integration test: the world reader's book, the national class,
  # the match engine checks, and the tools' world tests.
  worlds-misc)
    args=(--workspace --exclude afm-game); tests_of $(tools_rest)
    ;;
  worlds-own-1) args=(--workspace --exclude afm-game --test world -- "${WORLD_OWN_1[@]}") ;;
  worlds-own-2) args=(--workspace --exclude afm-game --test world -- "${WORLD_OWN_2[@]}") ;;
  worlds-shared)
    args=(--workspace --exclude afm-game --test world --)
    for f in "${WORLD_OWN_1[@]}" "${WORLD_OWN_2[@]}"; do args+=(--skip "$f"); done
    ;;
  *)
    echo "usage: $0 <shard> (see: $0 list)" >&2
    exit 2
    ;;
esac

if [ "${DRY_RUN:-}" = 1 ]; then
  echo "${CARGO[*]} ${args[*]}"
  [ "$doc" = 1 ] && echo "cargo test --workspace --doc --no-fail-fast"
  exit 0
fi

# Every test binary runs even when one fails, so a run names every failing
# test. The output is kept to check that the shard ran something.
log=$(mktemp)
status=0
"${CARGO[@]}" "${args[@]}" 2>&1 | tee "$log" || status=$?
if [ "$status" -eq 0 ] && ! grep -Eq 'test result: ok\. [1-9][0-9]* passed' "$log"; then
  echo "::error::shard $1 ran no test: a name or filter in .github/scripts/full-tests-shard.sh matches nothing"
  status=1
fi
if [ "$doc" = 1 ]; then
  cargo test --workspace --doc --no-fail-fast || status=$?
fi
exit "$status"
