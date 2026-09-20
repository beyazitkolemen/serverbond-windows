#!/usr/bin/env bash
# Idempotent Cloud Agent / Linux bootstrap. Does not start servers or run tests.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$root"

export PATH="${CARGO_HOME:-$HOME/.cargo}/bin:/usr/local/cargo/bin:${PATH}"
export DEBIAN_FRONTEND=noninteractive

apt_install() {
  if ! command -v apt-get >/dev/null 2>&1; then
    return 0
  fi
  local packages=(pkg-config libssl-dev)
  local missing=()
  for package in "${packages[@]}"; do
    if ! dpkg -s "$package" >/dev/null 2>&1; then
      missing+=("$package")
    fi
  done
  if ((${#missing[@]} == 0)); then
    return 0
  fi
  if [[ "$(id -u)" -eq 0 ]]; then
    apt-get update -qq
    apt-get install -y -qq "${missing[@]}"
  elif command -v sudo >/dev/null 2>&1; then
    sudo apt-get update -qq
    sudo apt-get install -y -qq "${missing[@]}"
  else
    echo "pkg-config and libssl-dev are required; install them before continuing." >&2
    return 1
  fi
}

ensure_rust() {
  if ! command -v rustup >/dev/null 2>&1; then
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable --profile minimal --component rustfmt,clippy
    # shellcheck disable=SC1091
    source "${CARGO_HOME:-$HOME/.cargo}/env"
  fi
  rustup toolchain install stable --component rustfmt,clippy --profile minimal
  rustup component add rustfmt clippy --toolchain stable
}

apt_install
ensure_rust
npm ci
cargo fetch --locked
