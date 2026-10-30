#!/usr/bin/env bash
# The "Build a world" jobs' first step: removes the runner's toolsets these
# jobs never use, then points ./out (the world, its log and its book) at
# whichever disk has the most room, and prints the disks.
set -euo pipefail
df -h / /mnt || true
sudo rm -rf /usr/share/dotnet /usr/local/lib/android /opt/ghc /usr/local/.ghcup \
  /opt/hostedtoolcache/CodeQL /usr/share/swift /usr/local/share/powershell
free -g
root=$(df --output=avail -B1G / | tail -1 | tr -d ' ')
mnt=$(df --output=avail -B1G /mnt | tail -1 | tr -d ' ')
if [ "$(stat -c %d /mnt)" != "$(stat -c %d /)" ] && [ "$mnt" -gt "$root" ]; then
  sudo mkdir -p /mnt/afm-out
  sudo chown "$(id -u)" /mnt/afm-out
  ln -sfn /mnt/afm-out out
  echo "out: /mnt, a disk of its own (${mnt} GB free; the runner's disk ${root} GB)"
else
  mkdir -p out
  echo "out: the runner's disk (${root} GB free; /mnt is no disk of its own or has less)"
fi
df -h / /mnt
