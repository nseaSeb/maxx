#!/usr/bin/env bash
# Replays the CI's Linux branch locally, in a container.
#
# What it covers and what it does not: Docker on a Mac runs Linux containers,
# never Windows ones — a Windows container needs a Windows host. There is
# therefore no local equivalent for the Windows branch, and that is precisely
# why it lives in the CI.
#
# Cargo's cache is mounted from a named volume: the first pass compiles 750
# crates, the ones after it do not.
#
# That volume was called `maxx-cible-linux` until the repository moved to
# English. If you ran the script before that, the first run after it rebuilds
# the cache once, and `docker volume rm maxx-cible-linux` reclaims the old one.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
image="maxx-linux:1"

if ! docker image inspect "$image" >/dev/null 2>&1; then
  echo "Building the image…"
  docker build -t "$image" -f - "$root" <<'DOCKERFILE'
FROM rust:1-bookworm
# The same packages the CI installs: gpui draws with Vulkan, speaks Wayland as
# well as X11, and its font crates want fontconfig.
RUN apt-get update && apt-get install -y --no-install-recommends \
    libasound2-dev libfontconfig-dev libwayland-dev libxkbcommon-x11-dev \
    libx11-xcb-dev libssl-dev libvulkan-dev pkg-config cmake clang \
 && rm -rf /var/lib/apt/lists/*
RUN rustup component add clippy
DOCKERFILE
fi

docker run --rm -it \
  -v "$root":/maxx \
  -v maxx-cargo-registry:/usr/local/cargo/registry \
  -v maxx-target-linux:/maxx/target-linux \
  -e CARGO_TARGET_DIR=/maxx/target-linux \
  -w /maxx \
  "$image" \
  bash -c "cargo clippy --all-targets -- -D warnings && cargo test --profile ci"
