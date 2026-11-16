#!/usr/bin/env bash
# The test jobs' memory: the engine's unit tests take about 13 GB to compile
# at full speed, and cargo builds the other test programs beside them. On the
# 16 GB runner that overlap ran it out of memory and the machine shut down
# mid-build (three quick checks on 2026-11-16, "The runner has received a
# shutdown signal"). Swap carries the overlap: it goes on whichever disk has
# the most room, and is smaller when that disk is also the build's.
set -euo pipefail
free -m
dir=/mnt
if [ ! -d /mnt ] || [ "$(df --output=avail -B1G / | tail -1 | tr -d ' ')" -gt "$(df --output=avail -B1G /mnt | tail -1 | tr -d ' ')" ]; then
  dir=/
fi
avail=$(df --output=avail -B1G "$dir" | tail -1 | tr -d ' ')
if [ "$(stat -c %d "$dir")" != "$(stat -c %d "$GITHUB_WORKSPACE")" ] && [ "$avail" -ge 30 ]; then
  size=16
elif [ "$avail" -ge 40 ]; then
  size=12
else
  size=6
fi
file="${dir%/}/afm-swap"
sudo fallocate -l "${size}G" "$file"
sudo chmod 600 "$file"
sudo mkswap "$file" >/dev/null
sudo swapon "$file"
echo "swap: ${size} GB at $file (${avail} GB free there before)"
free -m
