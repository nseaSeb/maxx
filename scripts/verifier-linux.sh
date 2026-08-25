#!/usr/bin/env bash
# Rejoue localement la branche Linux de la CI, dans un conteneur.
#
# Ce que ça couvre et ce que ça ne couvre pas : Docker sur un Mac lance des
# conteneurs Linux, jamais Windows — un conteneur Windows exige un hôte
# Windows. Il n'y a donc pas d'équivalent local pour la branche Windows, et
# c'est bien pour ça qu'elle est dans la CI.
#
# Le cache de cargo est monté depuis un volume nommé : le premier passage
# compile 750 crates, les suivants non.
set -euo pipefail

racine="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
image="maxx-linux:1"

if ! docker image inspect "$image" >/dev/null 2>&1; then
  echo "Construction de l'image…"
  docker build -t "$image" -f - "$racine" <<'DOCKERFILE'
FROM rust:1-bookworm
# Les mêmes paquets que la CI installe : gpui dessine avec Vulkan, parle à
# Wayland comme à X11, et ses crates de fontes veulent fontconfig.
RUN apt-get update && apt-get install -y --no-install-recommends \
    libasound2-dev libfontconfig-dev libwayland-dev libxkbcommon-x11-dev \
    libx11-xcb-dev libssl-dev libvulkan-dev pkg-config cmake clang \
 && rm -rf /var/lib/apt/lists/*
RUN rustup component add clippy
DOCKERFILE
fi

docker run --rm -it \
  -v "$racine":/maxx \
  -v maxx-cargo-registry:/usr/local/cargo/registry \
  -v maxx-cible-linux:/maxx/target-linux \
  -e CARGO_TARGET_DIR=/maxx/target-linux \
  -w /maxx \
  "$image" \
  bash -c "cargo clippy --all-targets -- -D warnings && cargo test --profile ci"
